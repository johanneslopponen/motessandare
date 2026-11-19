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
                ui.add(
                    egui::TextEdit::singleline(&mut self.name)
                        .margin(egui::Margin::symmetric(6, 4))
                        .desired_width(305.0),
                )
                .labelled_by(label);
                ui.label(format!(".{extension}"));
            });
            if let Some(folder) = self.path.parent() {
                ui.weak(folder.display().to_string());
            }
            if let Some(error) = &self.error {
                ui.colored_label(ui.visuals().warn_fg_color, error);
            }
            ui.add_space(8.0);
            ui.horizontal_wrapped(|ui| {
                let changed = self
                    .path
                    .file_stem()
                    .is_some_and(|stem| stem != self.name.as_str());
                if ui
                    .button(if changed { "Spara namn" } else { "Klar" })
                    .clicked()
                {
                    if changed {
                        self.error = self.apply_name().err().map(|error| error.to_string());
                    } else {
                        close = true;
                    }
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
                        .err()
                        .map(|error| error.to_string());
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
            });
        });
        close
    }

    fn apply_name(&mut self) -> io::Result<()> {
        let stem = self.name.trim();
        if self.path.file_stem().is_some_and(|current| current == stem) {
            self.name = stem.to_owned();
            return Ok(());
        }
        if stem.is_empty()
            || stem.ends_with('.')
            || stem.contains(['/', '\\', ':', '*', '?', '"', '<', '>', '|'])
            || stem.chars().any(char::is_control)
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Ange ett giltigt filnamn utan sökväg.",
            ));
        }
        let ext = self.path.extension().unwrap_or_default().to_string_lossy();
        let target = self.path.with_file_name(format!("{stem}.{ext}"));
        rename_without_overwrite(&self.path, &target).map_err(|error| {
            if error.kind() == io::ErrorKind::AlreadyExists {
                io::Error::new(
                    io::ErrorKind::AlreadyExists,
                    "Namnet används redan. Välj ett annat namn.",
                )
            } else {
                error
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

fn reveal_file(path: &Path) -> io::Result<()> {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        Command::new("explorer.exe")
            .raw_arg(format!("/select,\"{}\"", path.display()))
            .spawn()?;
    }
    #[cfg(target_os = "macos")]
    Command::new("open").arg("-R").arg(path).spawn()?;
    #[cfg(all(unix, not(target_os = "macos")))]
    Command::new("xdg-open")
        .arg(path.parent().unwrap_or(path))
        .spawn()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
