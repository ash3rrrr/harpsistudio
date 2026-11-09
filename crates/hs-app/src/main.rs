#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

mod app;
mod audio;
mod midi;
mod theme;
mod view_arrangement;
mod view_device;
mod view_mixer;
mod view_piano;
mod view_session;
mod widgets;

use eframe::egui;

fn main() -> eframe::Result {
    let capture = std::env::var_os("HARPSI_CAPTURE_DIR").is_some();
    let mut viewport = egui::ViewportBuilder::default()
        .with_inner_size([1600.0, 980.0])
        .with_min_inner_size([1180.0, 720.0]);
    if !capture {
        viewport = viewport.with_maximized(true);
    }
    let options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };
    eframe::run_native(
        "Harpsistudio",
        options,
        Box::new(|cc| Ok(Box::new(app::HarpsiApp::new(cc)))),
    )
}
