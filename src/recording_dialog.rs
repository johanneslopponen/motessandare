use crate::feedback;
use eframe::egui;
use std::{
    io,
    path::{Path, PathBuf},
    process::Command,
};

pub struct SavedRecording {
    source: String,
    path: PathBuf,
    name: String,
    error: Option<String>,
}

impl SavedRecording {
    pub fn new(source: String, path: PathBuf) -> Self {
        let name = path
            .file_stem()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned();
        Self {
            source,
            path,
            name,
            error: None,
        }
    }

    pub fn show(&mut self, ctx: &egui::Context) -> bool {
        let mut close = false;
        egui::Modal::new(egui::Id::new("saved-recording")).show(ctx, |ui| {
            ui.set_width(390.0);
            ui.heading("Inspelningen är sparad");
            ui.label(&self.source);
            ui.add_space(8.0);
            let label = ui.label("Filnamn").id;
            let extension = self.path.extension().unwrap_or_default().to_string_lossy();
            ui.horizontal(|ui| {
                if ui
                    .add(
                        egui::TextEdit::singleline(&mut self.name)
                            .margin(egui::Margin::symmetric(6, 4))
                            .desired_width(305.0),
                    )
                    .labelled_by(label)
                    .changed()
                {
                    self.error = None;
                }
                if !extension.is_empty() {
                    ui.label(format!(".{extension}"));
                }
            });
            if let Some(folder) = self.path.parent() {
                ui.weak(folder.display().to_string());
            }
            if let Some(error) = &self.error
                && feedback::message(ui, error)
            {
                self.error = None;
            }
            ui.add_space(8.0);
            ui.horizontal_wrapped(|ui| {
                let changed = self
                    .path
                    .file_stem()
                    .is_some_and(|stem| stem != self.name.as_str());
                if changed && ui.button("Spara namn").clicked() {
                    self.error = self.apply_name().err();
                }
                let reveal = if cfg!(windows) {
                    "Visa i Utforskaren"
                } else if cfg!(target_os = "macos") {
                    "Visa i Finder"
                } else {
                    "Visa mapp"
                };
                if ui
                    .button(reveal)
                    .on_hover_text("Spara filnamnet och visa inspelningen")
                    .clicked()
                {
                    self.error = self
                        .apply_name()
                        .and_then(|_| reveal_file(&self.path))
                        .err();
                }
                if changed && ui.button("Behåll namnet").clicked() {
                    self.name = self
                        .path
                        .file_stem()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .into_owned();
                    self.error = None;
                }
                if ui
                    .button("Klar")
                    .on_hover_text("Stäng med det senast sparade filnamnet")
                    .clicked()
                {
                    close = true;
                }
            });
        });
        close
    }

    fn apply_name(&mut self) -> Result<(), String> {
        let stem = self.name.trim();
        if self.path.file_stem().is_some_and(|current| current == stem) {
            self.name = stem.to_owned();
            return Ok(());
        }
        if stem.is_empty()
            || stem.ends_with('.')
            || crate::output::is_windows_reserved_name(stem)
            || stem.contains(['/', '\\', ':', '*', '?', '"', '<', '>', '|'])
            || stem.chars().any(char::is_control)
        {
            return Err("Ange ett filnamn, till exempel Kvällsmöte. Undvik snedstreck, specialtecken, reserverade namn och punkt på slutet.".into());
        }
        let ext = self.path.extension().unwrap_or_default().to_string_lossy();
        let filename = if ext.is_empty() {
            stem.to_owned()
        } else {
            format!("{stem}.{ext}")
        };
        let target = self.path.with_file_name(filename);
        rename_without_overwrite(&self.path, &target).map_err(|error| {
            if error.kind() == io::ErrorKind::AlreadyExists {
                "Namnet används redan. Välj ett annat filnamn.".into()
            } else {
                format!(
                    "Filnamnet kunde inte ändras. {}",
                    feedback::file_hint(&error)
                )
            }
        })?;
        self.path = target;
        self.name = stem.to_owned();
        Ok(())
    }
}

#[cfg(windows)]
fn rename_without_overwrite(from: &Path, to: &Path) -> io::Result<()> {
    use std::os::windows::ffi::OsStrExt;
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn MoveFileW(existing: *const u16, new_name: *const u16) -> i32;
    }
    let wide = |path: &Path| {
        path.as_os_str()
            .encode_wide()
            .chain(Some(0))
            .collect::<Vec<_>>()
    };
    let from = wide(from);
    let to = wide(to);
    // Both arguments are NUL-terminated and live until MoveFileW returns.
    if unsafe { MoveFileW(from.as_ptr(), to.as_ptr()) } == 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

#[cfg(not(windows))]
fn rename_without_overwrite(from: &Path, to: &Path) -> io::Result<()> {
    let mut original = tempfile::TempPath::try_from_path(from)?;
    // On collision the original recording must survive dropping the error.
    original.disable_cleanup(true);
    original.persist_noclobber(to).map_err(|e| e.error)
}

fn reveal_file(path: &Path) -> Result<(), String> {
    std::fs::metadata(path).map_err(|err| {
        format!(
            "Inspelningen kunde inte visas. {}",
            feedback::file_hint(&err)
        )
    })?;
    let open_error = |_| "Filhanteraren kunde inte öppnas. Öppna mappen som visas ovan.".to_owned();
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        Command::new("explorer.exe")
            .raw_arg(format!("/select,\"{}\"", path.display()))
            .spawn()
            .map_err(open_error)?;
    }
    #[cfg(target_os = "macos")]
    Command::new("open")
        .arg("-R")
        .arg(path)
        .spawn()
        .map_err(open_error)?;
    #[cfg(all(unix, not(target_os = "macos")))]
    Command::new("xdg-open")
        .arg(path.parent().unwrap_or(path))
        .spawn()
        .map_err(open_error)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_ui::click_button;

    #[test]
    fn save_name_keeps_dialog_open_and_done_keeps_the_last_saved_name() {
        let dir = tempfile::tempdir().unwrap();
        let original = dir.path().join("Möte.wav");
        let saved = dir.path().join("Kvällsmöte.wav");
        std::fs::write(&original, b"audio").unwrap();
        let mut dialog = SavedRecording::new("Sal".into(), original.clone());
        let ctx = egui::Context::default();
        dialog.name = " Kvällsmöte ".into();
        let mut closed = false;
        click_button(&ctx, "Spara namn", |ctx| closed |= dialog.show(ctx));
        assert!(!closed);
        assert_eq!(dialog.path, saved);
        assert_eq!(dialog.name, "Kvällsmöte");
        assert!(!original.exists());

        dialog.name = "Utkast".into();
        click_button(&ctx, "Klar", |ctx| closed |= dialog.show(ctx));
        assert!(closed);
        assert_eq!(std::fs::read(saved).unwrap(), b"audio");
        assert!(!dir.path().join("Utkast.wav").exists());
    }

    #[test]
    fn keep_name_discards_a_conflicting_edit_and_clears_the_error() {
        let dir = tempfile::tempdir().unwrap();
        let original = dir.path().join("Möte.flac");
        let existing = dir.path().join("Arkiv.flac");
        std::fs::write(&original, b"audio").unwrap();
        std::fs::write(&existing, b"existing").unwrap();
        let mut dialog = SavedRecording::new("Sal".into(), original.clone());
        let ctx = egui::Context::default();
        dialog.name = "Arkiv".into();
        click_button(&ctx, "Spara namn", |ctx| assert!(!dialog.show(ctx)));
        assert!(dialog.error.is_some());
        click_button(&ctx, "Behåll namnet", |ctx| assert!(!dialog.show(ctx)));
        assert!(dialog.error.is_none());
        assert_eq!(dialog.name, "Möte");
        assert_eq!(std::fs::read(original).unwrap(), b"audio");
        assert_eq!(std::fs::read(existing).unwrap(), b"existing");
    }

    #[test]
    fn rename_preserves_extension_and_protects_existing_files() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("Möte.wav");
        std::fs::write(&path, b"audio").unwrap();
        std::fs::write(dir.path().join("Annat.wav"), b"existing").unwrap();
        let mut recording = SavedRecording::new("Sal".into(), path.clone());
        for invalid in ["Annat", "../utanför", ""] {
            recording.name = invalid.into();
            assert!(recording.apply_name().is_err());
            assert_eq!(std::fs::read(&path).unwrap(), b"audio");
        }
        assert_eq!(
            std::fs::read(dir.path().join("Annat.wav")).unwrap(),
            b"existing"
        );
        recording.name = "Kvällsmöte 1.2".into();
        recording.apply_name().unwrap();
        assert_eq!(recording.path, dir.path().join("Kvällsmöte 1.2.wav"));
        assert_eq!(std::fs::read(&recording.path).unwrap(), b"audio");
        assert!(!path.exists());
    }

    #[test]
    fn rename_rejects_windows_reserved_names_and_avoids_trailing_dot_without_extension() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("no_ext_recording");
        std::fs::write(&path, b"audio").unwrap();
        let mut recording = SavedRecording::new("Sal".into(), path.clone());
        for reserved in ["CON", "NUL", "COM1", "con.extra"] {
            recording.name = reserved.into();
            assert!(recording.apply_name().is_err(), "{reserved}");
            assert!(path.exists());
        }
        recording.name = "renamed_no_ext".into();
        recording.apply_name().unwrap();
        assert_eq!(recording.path, dir.path().join("renamed_no_ext"));
        assert_eq!(std::fs::read(&recording.path).unwrap(), b"audio");
    }
}
