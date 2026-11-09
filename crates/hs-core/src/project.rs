//! Project files (.harp) and offline WAV export.

use crate::engine::Engine;
use crate::song::Song;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

pub fn projects_dir() -> PathBuf {
    let home = std::env::var("USERPROFILE")
        .or_else(|_| std::env::var("HOME"))
        .unwrap_or_else(|_| ".".to_string());
    let mut p = PathBuf::from(home);
    p.push("Documents");
    p.push("Harpsistudio");
    let _ = fs::create_dir_all(&p);
    p
}

pub fn sanitize(name: &str) -> String {
    let mut s = String::new();
    for c in name.chars() {
        if c.is_alphanumeric() || c == ' ' || c == '-' || c == '_' {
            s.push(c);
        }
    }
    let s = s.trim().to_string();
    if s.is_empty() {
        "Untitled".to_string()
    } else {
        s
    }
}

pub fn save_song(song: &Song) -> Result<PathBuf, String> {
    let dir = projects_dir();
    let path = dir.join(format!("{}.harp", sanitize(&song.name)));
    save_song_to(song, &path)?;
    Ok(path)
}

pub fn save_song_as(song: &Song, name: &str) -> Result<PathBuf, String> {
    let dir = projects_dir();
    let path = dir.join(format!("{}.harp", sanitize(name)));
    save_song_to(song, &path)?;
    Ok(path)
}

/// Serialize a project to an explicit path. `save_song` uses this after choosing
/// the Documents path; tests use it to write isolated project fixtures.
pub fn save_song_to(song: &Song, path: &Path) -> Result<(), String> {
    let json = serde_json::to_string_pretty(song).map_err(|e| e.to_string())?;
    fs::write(path, json).map_err(|e| e.to_string())?;
    Ok(())
}

pub fn load_song(path: &Path) -> Result<Song, String> {
    let data = fs::read_to_string(path).map_err(|e| e.to_string())?;
    let mut song: Song = serde_json::from_str(&data).map_err(|e| e.to_string())?;
    song.normalize();
    if song.return_fx.kind != crate::params::DeviceKind::Reverb {
        song.return_fx = crate::params::Device::new(crate::params::DeviceKind::Reverb);
        song.return_fx.set(3, 1.0);
    }
    Ok(song)
}

#[derive(Clone)]
pub struct FileEntry {
    pub name: String,
    pub path: PathBuf,
    pub modified: u64,
}

pub fn list_projects() -> Vec<FileEntry> {
    let dir = projects_dir();
    let mut out = Vec::new();
    if let Ok(rd) = fs::read_dir(&dir) {
        for e in rd.flatten() {
            let path = e.path();
            if path
                .extension()
                .and_then(|x| x.to_str())
                .map(|s| s.eq_ignore_ascii_case("harp"))
                == Some(true)
            {
                let name = path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("Project")
                    .to_string();
                let modified = e
                    .metadata()
                    .ok()
                    .and_then(|m| m.modified().ok())
                    .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                    .map(|d| d.as_secs())
                    .unwrap_or(0);
                out.push(FileEntry {
                    name,
                    path,
                    modified,
                });
            }
        }
    }
    out.sort_by_key(|a| std::cmp::Reverse(a.modified));
    out
}

fn write_wav(path: &Path, samples: &[f32], sample_rate: u32) -> Result<(), String> {
    let channels: u16 = 2;
    let bits: u16 = 16;
    let data_len = (samples.len() * 2) as u32;
    let mut f = fs::File::create(path).map_err(|e| e.to_string())?;
    let byte_rate = sample_rate * channels as u32 * bits as u32 / 8;
    let block_align = channels * bits / 8;
    let mut h: Vec<u8> = Vec::new();
    h.extend_from_slice(b"RIFF");
    h.extend_from_slice(&(36 + data_len).to_le_bytes());
    h.extend_from_slice(b"WAVE");
    h.extend_from_slice(b"fmt ");
    h.extend_from_slice(&16u32.to_le_bytes());
    h.extend_from_slice(&1u16.to_le_bytes()); // PCM
    h.extend_from_slice(&channels.to_le_bytes());
    h.extend_from_slice(&sample_rate.to_le_bytes());
    h.extend_from_slice(&byte_rate.to_le_bytes());
    h.extend_from_slice(&block_align.to_le_bytes());
    h.extend_from_slice(&bits.to_le_bytes());
    h.extend_from_slice(b"data");
    h.extend_from_slice(&data_len.to_le_bytes());
    f.write_all(&h).map_err(|e| e.to_string())?;
    let mut buf = Vec::with_capacity(8192);
    for s in samples {
        let v = (s.clamp(-1.0, 1.0) * 32767.0) as i16;
        buf.extend_from_slice(&v.to_le_bytes());
        if buf.len() >= 8192 {
            f.write_all(&buf).map_err(|e| e.to_string())?;
            buf.clear();
        }
    }
    f.write_all(&buf).map_err(|e| e.to_string())?;
    Ok(())
}

pub fn export_wav(song: &Song) -> Result<PathBuf, String> {
    let dir = projects_dir();
    let path = dir.join(format!("{} [mixdown].wav", sanitize(&song.name)));
    export_wav_to(song, &path)?;
    Ok(path)
}

/// Render the same offline mixdown the UI's Mixdown action uses, to an
/// explicit path. This keeps the renderer and WAV writer shared with tests.
pub fn export_wav_to(song: &Song, path: &Path) -> Result<(), String> {
    let sr = 48000u32;
    let samples = Engine::render_song(song, sr as f32);
    write_wav(path, &samples, sr)
}

/// Write the arrangement as a Standard MIDI File next to saved projects.
pub fn export_midi(song: &Song) -> Result<PathBuf, String> {
    let dir = projects_dir();
    let path = dir.join(format!("{} [MIDI].mid", sanitize(&song.name)));
    export_midi_to(song, &path)?;
    Ok(path)
}

/// Export arrangement clips to an explicit Standard MIDI File path.
pub fn export_midi_to(song: &Song, path: &Path) -> Result<(), String> {
    crate::midi_export::write(song, path)
}
