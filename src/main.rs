#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

mod capture;
mod engine;
mod guide;
mod listener_stats;
mod output;
mod recording_dialog;
mod settings;
mod ui;

use engine::Engine;
use settings::{Destination, Recording, Settings, Source, settings_load, settings_save};

fn app_icon() -> Option<eframe::egui::IconData> {
    eframe::icon_data::from_png_bytes(include_bytes!("../assets/icon-256.png")).ok()
}

fn main() -> eframe::Result {
    let mut viewport = eframe::egui::ViewportBuilder::default()
        .with_inner_size([820.0, 720.0])
        .with_min_inner_size([550.0, 600.0]);
    if let Some(icon) = app_icon() {
        viewport = viewport.with_icon(std::sync::Arc::new(icon));
    }
    let options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };
    eframe::run_native(
        "Mötessändare",
        options,
        Box::new(|cc| Ok(Box::new(ui::Ui::new(&cc.egui_ctx)))),
    )
}

#[cfg(test)]
mod tests {
    use super::app_icon;

    #[test]
    fn app_icon_decodes_cleanly() {
        let icon = app_icon().expect("embedded app icon PNG should decode");
        assert_eq!(icon.width, 256);
        assert_eq!(icon.height, 256);
        assert_eq!(icon.rgba.len(), 256 * 256 * 4);
    }
}
