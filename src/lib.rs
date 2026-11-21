mod capture;
mod engine;
mod feedback;
#[cfg(feature = "fuzzing")]
#[doc(hidden)]
pub mod fuzzing;
mod guide;
mod listener_stats;
mod output;
mod recording_dialog;
mod settings;
#[cfg(test)]
mod test_ui;
mod ui;

use engine::Engine;
use settings::{Destination, Recording, Settings, Source, settings_load, settings_save};

fn app_icon() -> Option<eframe::egui::IconData> {
    eframe::icon_data::from_png_bytes(include_bytes!("../assets/icon-256.png")).ok()
}

pub fn run() -> eframe::Result {
    let mut viewport = eframe::egui::ViewportBuilder::default()
        .with_inner_size([680.0, 620.0])
        .with_min_inner_size([420.0, 400.0]);
    if let Some(icon) = app_icon() {
        viewport = viewport.with_icon(std::sync::Arc::new(icon));
    }
    let options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };
    let result = eframe::run_native(
        "Mötessändare",
        options,
        Box::new(|cc| Ok(Box::new(ui::Ui::new(&cc.egui_ctx)))),
    );
    if result.is_err() {
        rfd::MessageDialog::new()
            .set_level(rfd::MessageLevel::Error)
            .set_title("Mötessändare kunde inte öppna fönstret")
            .set_description("Starta om appen. Om problemet kvarstår, kontrollera datorns skärm- och grafikinställningar.")
            .show();
    }
    result
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
