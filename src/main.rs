#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

mod capture;
mod engine;
mod output;
mod settings;
mod ui;

use engine::Engine;
use settings::{Destination, Recording, Source, settings_load, settings_save};

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([1000.0, 720.0])
            .with_min_inner_size([730.0, 500.0]),
        ..Default::default()
    };
    eframe::run_native(
        "Mötessändare",
        options,
        Box::new(|_| Ok(Box::new(ui::Ui::new()))),
    )
}
