use serde::{Deserialize, Serialize};
use std::{env, fs, io::Write, path::PathBuf};

#[derive(Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Destination {
    pub server: String,
    pub mount: String,
    pub user: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub password: String,
    pub codec: String,
    pub bitrate: u32,
}

impl Default for Destination {
    fn default() -> Self {
        Self {
            server: "http://localhost:8000".into(),
            mount: "/live.opus".into(),
            user: "source".into(),
            password: String::new(),
            codec: "Opus".into(),
            bitrate: 96,
        }
    }
}

#[derive(Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Recording {
    pub folder: String,
    pub format: String,
    pub quality: u32,
}

fn home() -> PathBuf {
    env::var_os(if cfg!(windows) { "USERPROFILE" } else { "HOME" })
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
}

impl Default for Recording {
    fn default() -> Self {
        Self {
            folder: home()
                .join("Music")
                .join("Mötessändare")
                .to_string_lossy()
                .into_owned(),
            format: "FLAC".into(),
            quality: 24,
        }
    }
}

#[derive(Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Source {
    pub id: u64,
    pub name: String,
    pub device: String,
    pub device_name: String,
    pub left: usize,
    pub right: Option<usize>,
    pub destinations: Vec<Destination>,
    pub recording: Recording,
}

impl Default for Source {
    fn default() -> Self {
        Self {
            id: 1,
            name: "Ny källa".into(),
            device: String::new(),
            device_name: String::new(),
            left: 0,
            right: None,
            destinations: vec![Destination::default()],
            recording: Recording::default(),
        }
    }
}

fn path() -> PathBuf {
    let base = if cfg!(target_os = "macos") {
        home().join("Library/Application Support")
    } else if cfg!(windows) {
        env::var_os("APPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(home)
    } else {
        env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| home().join(".config"))
    };
    base.join("Mötessändare").join("sources.json")
}

pub fn settings_load() -> Result<Vec<Source>, String> {
    let path = path();
    #[cfg(target_os = "macos")]
    return keychain::load(&path);
    #[cfg(not(target_os = "macos"))]
    Ok(read_settings(&path)?.unwrap_or_default())
}

pub fn settings_save(sources: &[Source]) -> Result<(), String> {
    let path = path();
    #[cfg(target_os = "macos")]
    return keychain::save(&path, sources);
    #[cfg(not(target_os = "macos"))]
    {
        // Never overwrite a configuration we cannot read.
        read_settings::<Vec<Source>>(&path)?;
        write_settings(&path, &sources)
    }
}

fn read_settings<T: serde::de::DeserializeOwned>(
    path: &std::path::Path,
) -> Result<Option<T>, String> {
    let bytes = match fs::read(path) {
        Ok(bytes) => bytes,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(err) => return Err(err.to_string()),
    };
    serde_json::from_slice(&bytes)
        .map(Some)
        .map_err(|_| format!("Ogiltig konfiguration. Kontrollera {}", path.display()))
}

fn write_settings(path: &std::path::Path, value: &impl Serialize) -> Result<(), String> {
    let parent = path.parent().unwrap();
    fs::create_dir_all(parent).map_err(|err| err.to_string())?;
    let mut file = tempfile::NamedTempFile::new_in(parent).map_err(|err| err.to_string())?;
    serde_json::to_writer_pretty(&mut file, value).map_err(|err| err.to_string())?;
    file.flush()
        .and_then(|_| file.as_file().sync_all())
        .map_err(|err| err.to_string())?;
    file.persist(path).map_err(|err| err.error.to_string())?;
    Ok(())
}

#[cfg(target_os = "macos")]
mod keychain {
    use super::{Deserialize, Serialize, Source, fs, read_settings, write_settings};
    use security_framework::passwords::{
        delete_generic_password, generic_password, set_generic_password,
    };
    use std::path::Path;

    const SERVICE: &str = "se.motessandare.app.credentials";
    type Passwords = Vec<Vec<String>>;

    #[derive(Deserialize)]
    #[serde(untagged)]
    enum StoredSettings {
        Protected(ProtectedSettings),
        Legacy(Vec<Source>),
    }

    #[derive(Serialize, Deserialize)]
    struct ProtectedSettings {
        credential_account: Option<String>,
        sources: Vec<Source>,
    }

    // Keep the same lock file across renames of sources.json. This keeps a reader
    // from using the previous reference after a writer deletes its entry.
    fn lock(path: &Path) -> Result<fs::File, String> {
        fs::create_dir_all(path.parent().unwrap()).map_err(|err| err.to_string())?;
        let file = fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(path.with_extension("lock"))
            .map_err(|err| err.to_string())?;
        file.lock().map_err(|err| err.to_string())?;
        Ok(file)
    }

    pub fn load(path: &Path) -> Result<Vec<Source>, String> {
        let _lock = lock(path)?;
        match read_settings(path)? {
            None => Ok(Vec::new()),
            Some(StoredSettings::Legacy(sources)) => {
                if sources
                    .iter()
                    .flat_map(|source| &source.destinations)
                    .any(|dest| !dest.password.is_empty())
                {
                    commit(path, &sources, None)?;
                }
                Ok(sources)
            }
            Some(StoredSettings::Protected(settings)) => hydrate(settings),
        }
    }

    pub fn save(path: &Path, sources: &[Source]) -> Result<(), String> {
        let _lock = lock(path)?;
        let previous_account = match read_settings(path)? {
            Some(StoredSettings::Protected(settings)) => {
                let account = settings.credential_account.clone();
                hydrate(settings)?;
                account
            }
            Some(StoredSettings::Legacy(_)) | None => None,
        };
        commit(path, sources, previous_account.as_deref())
    }

    fn hydrate(mut settings: ProtectedSettings) -> Result<Vec<Source>, String> {
        if let Some(account) = settings.credential_account {
            let bytes = generic_password(SERVICE, &account).map_err(keychain_error)?;
            let passwords: Passwords = serde_json::from_slice(&bytes)
                .map_err(|_| "Nyckelringen innehåller ogiltiga lösenordsuppgifter.".to_owned())?;
            if passwords.len() != settings.sources.len()
                || settings
                    .sources
                    .iter()
                    .zip(&passwords)
                    .any(|(source, row)| source.destinations.len() != row.len())
            {
                return Err(
                    "Nyckelringens lösenordsuppgifter matchar inte konfigurationen.".to_owned(),
                );
            }
            for (source, row) in settings.sources.iter_mut().zip(passwords) {
                for (destination, password) in source.destinations.iter_mut().zip(row) {
                    destination.password = password;
                }
            }
        }
        Ok(settings.sources)
    }

    fn commit(
        path: &Path,
        sources: &[Source],
        previous_account: Option<&str>,
    ) -> Result<(), String> {
        let mut safe_sources = sources.to_vec();
        let passwords: Passwords = safe_sources
            .iter_mut()
            .map(|source| {
                source
                    .destinations
                    .iter_mut()
                    .map(|destination| std::mem::take(&mut destination.password))
                    .collect()
            })
            .collect();
        let new_account = if passwords
            .iter()
            .flatten()
            .any(|password| !password.is_empty())
        {
            // Use a fresh Keychain item so a failure writing JSON leaves the
            // current settings and their existing Keychain entry intact.
            let unique = tempfile::Builder::new()
                .prefix("settings-")
                .rand_bytes(24)
                .tempfile_in(path.parent().unwrap())
                .map_err(|err| err.to_string())?;
            let account = unique
                .path()
                .file_name()
                .unwrap()
                .to_string_lossy()
                .into_owned();
            let data = serde_json::to_vec(&passwords).map_err(|err| err.to_string())?;
            set_generic_password(SERVICE, &account, &data).map_err(keychain_error)?;
            Some(account)
        } else {
            None
        };
        let settings = ProtectedSettings {
            credential_account: new_account,
            sources: safe_sources,
        };
        if let Err(err) = write_settings(path, &settings) {
            if let Some(account) = &settings.credential_account {
                let _ = delete_generic_password(SERVICE, account);
            }
            return Err(err);
        }
        if let Some(account) = previous_account {
            let _ = delete_generic_password(SERVICE, account);
        }
        Ok(())
    }

    fn keychain_error(error: security_framework::base::Error) -> String {
        format!(
            "Det gick inte att komma åt Nyckelringen ({error}). Kontrollera att den är upplåst och att appen har åtkomst."
        )
    }
}
