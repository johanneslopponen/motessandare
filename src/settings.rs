use serde::{Deserialize, Serialize};
use std::{env, fs, io::Write, path::PathBuf};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Destination {
    pub id: u64,
    pub name: String,
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
            id: 0,
            name: String::new(),
            server: "http://localhost:8000".into(),
            mount: "/live.opus".into(),
            user: "source".into(),
            password: String::new(),
            codec: "Opus".into(),
            bitrate: 96,
        }
    }
}

impl Destination {
    pub fn display_name(&self) -> &str {
        if self.name.trim().is_empty() {
            "Namnlös destination"
        } else {
            self.name.trim()
        }
    }

    fn same_endpoint(&self, other: &Self) -> bool {
        self.server == other.server
            && self.mount == other.mount
            && self.user == other.user
            && self.password == other.password
            && self.codec == other.codec
            && self.bitrate == other.bitrate
    }
}

pub const DEFAULT_FILENAME_FORMAT: &str = "{namn}-{datum}-{tid}";

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Recording {
    pub folder: String,
    pub filename_format: String,
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
            filename_format: DEFAULT_FILENAME_FORMAT.into(),
            format: "FLAC".into(),
            quality: 24,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Source {
    pub id: u64,
    pub name: String,
    pub device: String,
    pub device_name: String,
    pub left: usize,
    pub right: Option<usize>,
    pub destination_ids: Vec<u64>,
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
            destination_ids: Vec::new(),
            recording: Recording::default(),
        }
    }
}

impl Source {
    pub fn resolve_destinations<'a>(&self, destinations: &'a [Destination]) -> Vec<&'a Destination> {
        self.destination_ids
            .iter()
            .filter_map(|id| destinations.iter().find(|dest| dest.id == *id))
            .collect()
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub compact_overview: Option<bool>,
    pub destinations: Vec<Destination>,
    pub sources: Vec<Source>,
}

impl Settings {
    pub fn next_source_id(&self) -> u64 {
        let mut id = 1;
        while self.sources.iter().any(|source| source.id == id) {
            id += 1;
        }
        id
    }

    pub fn next_destination_id(&self) -> u64 {
        let mut id = 1;
        while self.destinations.iter().any(|dest| dest.id == id) {
            id += 1;
        }
        id
    }

    fn sanitize(&mut self) {
        let mut used_ids = Vec::new();
        for dest in &mut self.destinations {
            if dest.id == 0 || used_ids.contains(&dest.id) {
                let mut id = 1;
                while used_ids.contains(&id) {
                    id += 1;
                }
                dest.id = id;
            }
            used_ids.push(dest.id);
        }
        for source in &mut self.sources {
            let mut seen = Vec::new();
            source.destination_ids.retain(|id| {
                let valid = self.destinations.iter().any(|dest| dest.id == *id)
                    && !seen.contains(id);
                if valid {
                    seen.push(*id);
                }
                valid
            });
        }
    }
}

#[derive(Clone, Deserialize)]
#[serde(default)]
struct RawSource {
    id: u64,
    name: String,
    device: String,
    device_name: String,
    left: usize,
    right: Option<usize>,
    destination_ids: Vec<u64>,
    destinations: Vec<Destination>,
    recording: Recording,
}

impl Default for RawSource {
    fn default() -> Self {
        let defaults = Source::default();
        Self {
            id: defaults.id,
            name: defaults.name,
            device: defaults.device,
            device_name: defaults.device_name,
            left: defaults.left,
            right: defaults.right,
            destination_ids: Vec::new(),
            destinations: Vec::new(),
            recording: defaults.recording,
        }
    }
}

#[derive(Deserialize)]
struct RawSettingsObject {
    compact_overview: Option<bool>,
    #[serde(default)]
    destinations: Vec<Destination>,
    #[serde(default)]
    sources: Vec<RawSource>,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum StoredSettings {
    Object(RawSettingsObject),
    LegacyArray(Vec<RawSource>),
}

fn migrate_raw(destinations: Vec<Destination>, raw_sources: Vec<RawSource>, compact_overview: Option<bool>) -> Settings {
    let mut settings = Settings {
        compact_overview,
        destinations,
        sources: Vec::with_capacity(raw_sources.len()),
    };
    settings.sanitize();

    for raw in raw_sources {
        let mut destination_ids = raw.destination_ids;
        for mut inline_dest in raw.destinations {
            let id = if let Some(existing) = settings
                .destinations
                .iter()
                .find(|dest| dest.same_endpoint(&inline_dest))
            {
                existing.id
            } else {
                let id = settings.next_destination_id();
                inline_dest.id = id;
                if inline_dest.name.trim().is_empty() {
                    inline_dest.name = format!("Destination {id}");
                }
                settings.destinations.push(inline_dest);
                id
            };
            if !destination_ids.contains(&id) {
                destination_ids.push(id);
            }
        }
        settings.sources.push(Source {
            id: raw.id,
            name: raw.name,
            device: raw.device,
            device_name: raw.device_name,
            left: raw.left,
            right: raw.right,
            destination_ids,
            recording: raw.recording,
        });
    }
    settings.sanitize();
    settings
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

pub fn settings_load() -> Result<Settings, String> {
    let path = path();
    #[cfg(target_os = "macos")]
    return keychain::load(&path);
    #[cfg(not(target_os = "macos"))]
    load_plain(&path)
}

pub fn settings_save(settings: &Settings) -> Result<(), String> {
    let path = path();
    #[cfg(target_os = "macos")]
    return keychain::save(&path, settings);
    #[cfg(not(target_os = "macos"))]
    save_plain(&path, settings)
}

#[cfg(any(not(target_os = "macos"), test))]
fn load_plain(path: &std::path::Path) -> Result<Settings, String> {
    match read_settings::<StoredSettings>(path)? {
        None => Ok(Settings::default()),
        Some(StoredSettings::Object(raw)) => Ok(migrate_raw(raw.destinations, raw.sources, raw.compact_overview)),
        Some(StoredSettings::LegacyArray(sources)) => Ok(migrate_raw(Vec::new(), sources, None)),
    }
}

#[cfg(any(not(target_os = "macos"), test))]
fn save_plain(path: &std::path::Path, settings: &Settings) -> Result<(), String> {
    // Never overwrite a configuration we cannot read.
    read_settings::<StoredSettings>(path)?;
    write_settings(path, settings)
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
    use super::{
        Deserialize, Destination, RawSource, Serialize, Settings, Source, fs, migrate_raw,
        read_settings, write_settings,
    };
    use security_framework::passwords::{
        delete_generic_password, generic_password, set_generic_password,
    };
    use std::path::Path;

    const SERVICE: &str = "se.motessandare.app.credentials";

    #[derive(Deserialize)]
    #[serde(untagged)]
    enum StoredPasswords {
        Global(Vec<String>),
        Legacy(Vec<Vec<String>>),
    }

    #[derive(Deserialize)]
    #[serde(untagged)]
    enum StoredSettings {
        Protected(RawProtectedSettings),
        Legacy(Vec<RawSource>),
    }

    #[derive(Deserialize)]
    struct RawProtectedSettings {
        compact_overview: Option<bool>,
        credential_account: Option<String>,
        #[serde(default)]
        destinations: Vec<Destination>,
        #[serde(default)]
        sources: Vec<RawSource>,
    }

    #[derive(Serialize)]
    struct ProtectedSettings {
        compact_overview: Option<bool>,
        credential_account: Option<String>,
        destinations: Vec<Destination>,
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

    pub fn load(path: &Path) -> Result<Settings, String> {
        let _lock = lock(path)?;
        match read_settings(path)? {
            None => Ok(Settings::default()),
            Some(StoredSettings::Legacy(raw_sources)) => {
                let settings = migrate_raw(Vec::new(), raw_sources, None);
                if settings
                    .destinations
                    .iter()
                    .any(|dest| !dest.password.is_empty())
                {
                    commit(path, &settings, None)?;
                }
                Ok(settings)
            }
            Some(StoredSettings::Protected(raw)) => hydrate(raw),
        }
    }

    pub fn save(path: &Path, settings: &Settings) -> Result<(), String> {
        let _lock = lock(path)?;
        let previous_account = match read_settings(path)? {
            Some(StoredSettings::Protected(raw)) => {
                let account = raw.credential_account.clone();
                hydrate(raw)?;
                account
            }
            Some(StoredSettings::Legacy(_)) | None => None,
        };
        commit(path, settings, previous_account.as_deref())
    }

    fn hydrate(mut raw: RawProtectedSettings) -> Result<Settings, String> {
        if let Some(account) = raw.credential_account {
            let bytes = generic_password(SERVICE, &account).map_err(keychain_error)?;
            let passwords: StoredPasswords = serde_json::from_slice(&bytes)
                .map_err(|_| "Nyckelringen innehåller ogiltiga lösenordsuppgifter.".to_owned())?;
            match passwords {
                StoredPasswords::Global(passwords) => {
                    if passwords.len() != raw.destinations.len() {
                        return Err(
                            "Nyckelringens lösenordsuppgifter matchar inte konfigurationen."
                                .to_owned(),
                        );
                    }
                    for (destination, password) in raw.destinations.iter_mut().zip(passwords) {
                        destination.password = password;
                    }
                }
                StoredPasswords::Legacy(passwords) => {
                    if passwords.len() != raw.sources.len()
                        || raw
                            .sources
                            .iter()
                            .zip(&passwords)
                            .any(|(source, row)| source.destinations.len() != row.len())
                    {
                        return Err(
                            "Nyckelringens lösenordsuppgifter matchar inte konfigurationen."
                                .to_owned(),
                        );
                    }
                    for (source, row) in raw.sources.iter_mut().zip(passwords) {
                        for (destination, password) in source.destinations.iter_mut().zip(row) {
                            destination.password = password;
                        }
                    }
                }
            }
        }
        Ok(migrate_raw(raw.destinations, raw.sources, raw.compact_overview))
    }

    fn commit(
        path: &Path,
        settings: &Settings,
        previous_account: Option<&str>,
    ) -> Result<(), String> {
        let mut safe_destinations = settings.destinations.clone();
        let passwords: Vec<String> = safe_destinations
            .iter_mut()
            .map(|destination| std::mem::take(&mut destination.password))
            .collect();
        let new_account = if passwords.iter().any(|password| !password.is_empty()) {
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
        let protected = ProtectedSettings {
            compact_overview: settings.compact_overview,
            credential_account: new_account,
            destinations: safe_destinations,
            sources: settings.sources.clone(),
        };
        if let Err(err) = write_settings(path, &protected) {
            if let Some(account) = &protected.credential_account {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn saves_and_loads_global_destinations_and_source_references() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("sources.json");
        let settings = Settings {
            compact_overview: Some(false),
            destinations: vec![
                Destination {
                    id: 1,
                    name: "Huvudserver".into(),
                    server: "https://stream.example.org".into(),
                    mount: "/main.opus".into(),
                    user: "source".into(),
                    password: "secret".into(),
                    codec: "Opus".into(),
                    bitrate: 96,
                },
                Destination {
                    id: 2,
                    name: "Reserv".into(),
                    server: "https://backup.example.org".into(),
                    mount: "/backup.mp3".into(),
                    user: "source".into(),
                    password: String::new(),
                    codec: "MP3".into(),
                    bitrate: 128,
                },
            ],
            sources: vec![Source {
                id: 1,
                name: "Sal A".into(),
                destination_ids: vec![1, 2],
                ..Source::default()
            }],
        };

        save_plain(&file, &settings).unwrap();
        let loaded = load_plain(&file).unwrap();
        assert_eq!(loaded, settings);
    }

    #[test]
    fn migrates_legacy_source_array_and_deduplicates_destinations() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("sources.json");
        let legacy_json = r#"[
            {
                "id": 1,
                "name": "Sal A",
                "device": "dev1",
                "device_name": "Mic 1",
                "left": 0,
                "right": null,
                "destinations": [
                    {
                        "server": "http://localhost:8000",
                        "mount": "/shared.opus",
                        "user": "source",
                        "password": "pw",
                        "codec": "Opus",
                        "bitrate": 96
                    },
                    {
                        "server": "http://localhost:8000",
                        "mount": "/only-a.mp3",
                        "user": "source",
                        "codec": "MP3",
                        "bitrate": 128
                    }
                ]
            },
            {
                "id": 2,
                "name": "Sal B",
                "device": "dev2",
                "device_name": "Mic 2",
                "left": 0,
                "right": 1,
                "destinations": [
                    {
                        "server": "http://localhost:8000",
                        "mount": "/shared.opus",
                        "user": "source",
                        "password": "pw",
                        "codec": "Opus",
                        "bitrate": 96
                    }
                ]
            }
        ]"#;
        fs::write(&file, legacy_json).unwrap();

        let loaded = load_plain(&file).unwrap();
        assert_eq!(loaded.destinations.len(), 2);
        assert_eq!(loaded.destinations[0].id, 1);
        assert_eq!(loaded.destinations[0].name, "Destination 1");
        assert_eq!(loaded.destinations[0].mount, "/shared.opus");
        assert_eq!(loaded.destinations[0].password, "pw");
        assert_eq!(loaded.destinations[1].id, 2);
        assert_eq!(loaded.destinations[1].mount, "/only-a.mp3");

        assert_eq!(loaded.sources.len(), 2);
        assert_eq!(loaded.sources[0].destination_ids, vec![1, 2]);
        assert_eq!(loaded.sources[1].destination_ids, vec![1]);
    }

    #[test]
    fn sanitizes_dangling_and_duplicate_destination_ids() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("sources.json");
        let json = r#"{
            "destinations": [
                { "id": 5, "name": "Huvud", "server": "http://localhost:8000", "mount": "/a.opus", "user": "source", "codec": "Opus", "bitrate": 96 }
            ],
            "sources": [
                { "id": 1, "name": "Källa", "destination_ids": [5, 99, 5] }
            ]
        }"#;
        fs::write(&file, json).unwrap();

        let loaded = load_plain(&file).unwrap();
        assert_eq!(loaded.sources[0].destination_ids, vec![5]);
    }

    #[test]
    fn refuses_to_overwrite_unreadable_config() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("sources.json");
        fs::write(&file, "{ not valid json").unwrap();

        let result = save_plain(&file, &Settings::default());
        assert!(result.is_err());
        assert_eq!(fs::read_to_string(&file).unwrap(), "{ not valid json");
    }
}
