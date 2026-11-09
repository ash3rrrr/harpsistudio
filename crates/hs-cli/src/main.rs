//! Headless access to Harpsistudio's core renderer.
//!
//! This is deliberately separate from the Windows GUI: it makes the project
//! useful in CI, scripts and other frontends while exercising exactly the same
//! `hs-core` song and export paths as the desktop app.

use hs_core::presets::{self, DEMOS};
use hs_core::project;
use hs_core::song::Song;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

const USAGE: &str = "\
Harpsistudio headless CLI

Usage:
  harpsistudio-cli <command> [arguments]

Commands:
  list-demos                         List the built-in factory projects.
  demo <index|name> <output.wav>     Render a factory project.
  render <input.harp> <output.wav>   Render a saved project.
  midi <input.harp> <output.mid>     Export arrangement clips as MIDI.
  import-midi <input.mid> <output.harp>
                                     Import a Standard MIDI File.
  inspect <input.harp>               Print song, track and clip counts.
  help, --help, -h                   Show this help.
  version, --version, -V             Print the CLI version.
";

#[derive(Debug, PartialEq)]
enum Command {
    Help,
    Version,
    ListDemos,
    Demo { query: String, output: PathBuf },
    Render { input: PathBuf, output: PathBuf },
    Midi { input: PathBuf, output: PathBuf },
    ImportMidi { input: PathBuf, output: PathBuf },
    Inspect { input: PathBuf },
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();
    let command = match parse_args(&args) {
        Ok(command) => command,
        Err(error) => {
            eprintln!("error: {error}\n\n{USAGE}");
            return ExitCode::from(2);
        }
    };

    match run(command) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::FAILURE
        }
    }
}

fn parse_args(args: &[String]) -> Result<Command, String> {
    let Some(command) = args.first().map(String::as_str) else {
        return Ok(Command::Help);
    };

    match command {
        "help" | "--help" | "-h" => Ok(Command::Help),
        "version" | "--version" | "-V" => Ok(Command::Version),
        "list-demos" | "demos" | "list" => Ok(Command::ListDemos),
        "demo" => {
            if args.len() != 3 {
                return Err("demo expects a demo name/index and an output path".to_string());
            }
            Ok(Command::Demo {
                query: args[1].clone(),
                output: PathBuf::from(&args[2]),
            })
        }
        "render" => {
            if args.len() != 3 {
                return Err("render expects an input .harp path and an output path".to_string());
            }
            Ok(Command::Render {
                input: PathBuf::from(&args[1]),
                output: PathBuf::from(&args[2]),
            })
        }
        "midi" | "export-midi" => {
            if args.len() != 3 {
                return Err("midi expects an input .harp path and an output .mid path".to_string());
            }
            Ok(Command::Midi {
                input: PathBuf::from(&args[1]),
                output: PathBuf::from(&args[2]),
            })
        }
        "import-midi" => {
            if args.len() != 3 {
                return Err(
                    "import-midi expects an input .mid path and an output .harp path".to_string(),
                );
            }
            Ok(Command::ImportMidi {
                input: PathBuf::from(&args[1]),
                output: PathBuf::from(&args[2]),
            })
        }
        "inspect" => {
            let Some(input) = args.get(1) else {
                return Err("inspect expects an input .harp path".to_string());
            };
            if args.len() != 2 {
                return Err("inspect expects exactly one input .harp path".to_string());
            }
            Ok(Command::Inspect {
                input: PathBuf::from(input),
            })
        }
        other => Err(format!("unknown command `{other}`")),
    }
}

fn run(command: Command) -> Result<(), String> {
    match command {
        Command::Help => {
            print!("{USAGE}");
            Ok(())
        }
        Command::Version => {
            println!("harpsistudio-cli {}", env!("CARGO_PKG_VERSION"));
            Ok(())
        }
        Command::ListDemos => {
            list_demos();
            Ok(())
        }
        Command::Demo { query, output } => {
            let index = resolve_demo(&query)?;
            let song = presets::demo_song(index);
            println!("demo: {} ({} BPM)", song.name, song.bpm);
            render_song(&song, &output)
        }
        Command::Render { input, output } => {
            let song = load(&input)?;
            println!("project: {} ({} BPM)", song.name, song.bpm);
            render_song(&song, &output)
        }
        Command::Midi { input, output } => {
            let song = load(&input)?;
            println!("project: {} ({} BPM)", song.name, song.bpm);
            write_midi(&song, &output)
        }
        Command::ImportMidi { input, output } => {
            let song = hs_core::midi_import::read(&input)?;
            println!(
                "imported: {} ({} BPM, {} tracks, {} notes)",
                song.name,
                song.bpm,
                song.tracks.len(),
                song.tracks
                    .iter()
                    .flat_map(|track| track.clips.iter())
                    .map(|clip| clip.notes.len())
                    .sum::<usize>()
            );
            write_project(&song, &output)
        }
        Command::Inspect { input } => {
            let song = load(&input)?;
            print_song(&song);
            Ok(())
        }
    }
}

fn list_demos() {
    for (index, demo) in DEMOS.iter().enumerate() {
        println!(
            "{index:>2}  {:<18} {:<12} {:>3} BPM  {}",
            demo.name, demo.tag, demo.bpm, demo.blurb
        );
    }
}

fn resolve_demo(query: &str) -> Result<usize, String> {
    if let Ok(index) = query.parse::<usize>() {
        if index < DEMOS.len() {
            return Ok(index);
        }
        return Err(format!(
            "demo index {index} is out of range (0..{})",
            DEMOS.len()
        ));
    }

    let needle = query.to_lowercase();
    let exact = DEMOS
        .iter()
        .position(|demo| demo.name.to_lowercase() == needle);
    if let Some(index) = exact {
        return Ok(index);
    }

    let partial: Vec<usize> = DEMOS
        .iter()
        .enumerate()
        .filter(|(_, demo)| {
            demo.name.to_lowercase().contains(&needle) || demo.tag.to_lowercase().contains(&needle)
        })
        .map(|(index, _)| index)
        .collect();
    match partial.as_slice() {
        [index] => Ok(*index),
        [] => Err(format!(
            "no demo matches `{query}`; run `harpsistudio-cli list-demos`"
        )),
        _ => Err(format!(
            "`{query}` matches more than one demo; use an index or exact name"
        )),
    }
}

fn load(path: &Path) -> Result<Song, String> {
    if !path.exists() {
        return Err(format!("project not found: {}", path.display()));
    }
    project::load_song(path).map_err(|error| format!("could not load {}: {error}", path.display()))
}

fn render_song(song: &Song, output: &Path) -> Result<(), String> {
    ensure_parent(output)?;
    project::export_wav_to(song, output)
        .map_err(|error| format!("could not render {}: {error}", output.display()))?;
    let bytes = fs::metadata(output)
        .map(|metadata| metadata.len())
        .unwrap_or(0);
    println!("rendered: {} ({bytes} bytes)", output.display());
    Ok(())
}

fn write_midi(song: &Song, output: &Path) -> Result<(), String> {
    ensure_parent(output)?;
    project::export_midi_to(song, output)
        .map_err(|error| format!("could not write {}: {error}", output.display()))?;
    let bytes = fs::metadata(output)
        .map(|metadata| metadata.len())
        .unwrap_or(0);
    println!("wrote MIDI: {} ({bytes} bytes)", output.display());
    Ok(())
}

fn write_project(song: &Song, output: &Path) -> Result<(), String> {
    ensure_parent(output)?;
    project::save_song_to(song, output)
        .map_err(|error| format!("could not write {}: {error}", output.display()))?;
    let bytes = fs::metadata(output)
        .map(|metadata| metadata.len())
        .unwrap_or(0);
    println!("wrote project: {} ({bytes} bytes)", output.display());
    Ok(())
}

fn ensure_parent(path: &Path) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)
                .map_err(|error| format!("could not create {}: {error}", parent.display()))?;
        }
    }
    Ok(())
}

fn print_song(song: &Song) {
    let arrangement_clips: usize = song
        .tracks
        .iter()
        .map(|track| track.arrangement_clips().count())
        .sum();
    let notes: usize = song
        .tracks
        .iter()
        .flat_map(|track| track.clips.iter())
        .map(|clip| clip.notes.len())
        .sum();
    println!("name: {}", song.name);
    println!("tempo: {:.1} BPM", song.bpm);
    println!("tracks: {}", song.tracks.len());
    println!("scenes: {}", song.scene_names().len());
    println!("session clips: {}", song.all_session_clips().len());
    println!("arrangement clips: {arrangement_clips}");
    println!("notes: {notes}");
    println!("arrangement bars: {:.2}", song.length_beats() / 4.0);
    println!("tracks:");
    for (index, track) in song.tracks.iter().enumerate() {
        let clips = track.clips.len();
        println!(
            "  {:>2}. {:<24} {} clip{}",
            index + 1,
            track.name,
            clips,
            if clips == 1 { "" } else { "s" }
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| value.to_string()).collect()
    }

    #[test]
    fn parses_the_headless_commands() {
        assert_eq!(parse_args(&[]), Ok(Command::Help));
        assert_eq!(parse_args(&args(&["list-demos"])), Ok(Command::ListDemos));
        assert_eq!(
            parse_args(&args(&["render", "song.harp", "mix.wav"])),
            Ok(Command::Render {
                input: PathBuf::from("song.harp"),
                output: PathBuf::from("mix.wav"),
            })
        );
        assert_eq!(
            parse_args(&args(&["demo", "Midnight House", "mix.wav"])),
            Ok(Command::Demo {
                query: "Midnight House".to_string(),
                output: PathBuf::from("mix.wav"),
            })
        );
        assert_eq!(
            parse_args(&args(&["midi", "song.harp", "song.mid"])),
            Ok(Command::Midi {
                input: PathBuf::from("song.harp"),
                output: PathBuf::from("song.mid"),
            })
        );
        assert_eq!(
            parse_args(&args(&["import-midi", "song.mid", "song.harp"])),
            Ok(Command::ImportMidi {
                input: PathBuf::from("song.mid"),
                output: PathBuf::from("song.harp"),
            })
        );
        assert!(parse_args(&args(&["render", "only-one-path"])).is_err());
        assert!(parse_args(&args(&["midi", "only-one-path"])).is_err());
        assert!(parse_args(&args(&["import-midi", "only-one-path"])).is_err());
        assert!(parse_args(&args(&["wat"])).is_err());
    }

    #[test]
    fn resolves_demos_by_index_name_and_unique_tag() {
        assert_eq!(resolve_demo("0"), Ok(0));
        assert_eq!(resolve_demo("Midnight House"), Ok(0));
        assert_eq!(resolve_demo("midnight house"), Ok(0));
        assert_eq!(resolve_demo("RAP"), Ok(7));
        assert!(resolve_demo("not-a-demo").is_err());
        assert!(resolve_demo("11").is_err());
    }

    #[test]
    fn prints_a_stable_summary_for_a_demo() {
        let song = presets::demo_song(0);
        assert_eq!(song.name, "Midnight House");
        assert_eq!(song.tracks.len(), 6);
        assert_eq!(song.scene_names().len(), 8);
        assert!(song.length_beats() > 0.0);
    }
}
