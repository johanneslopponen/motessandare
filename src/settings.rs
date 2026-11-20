use serde::{Deserialize, Serialize};
use std::{collections::HashSet, env, fs, io::Write, path::PathBuf};

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

pub const DEFAULT_FILENAME_FORMAT: &str = "{datum} {tid} - {namn}";

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
    pub fn display_name(&self) -> &str {
        if self.name.trim().is_empty() {
            "Namnlös ljudkälla"
        } else {
            &self.name
        }
    }

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
}

fn repair_ids<T>(items: &mut [T], id_of: impl Fn(&mut T) -> &mut u64) {
    let mut reserved_ids: HashSet<u64> = items.iter_mut().map(|item| *id_of(item)).collect();
    let mut seen_ids = HashSet::new();
    let mut next_id = 1;
    for item in items {
        let id = id_of(item);
        if *id == 0 || !seen_ids.insert(*id) {
            while !reserved_ids.insert(next_id) {
                next_id += 1;
            }
            *id = next_id;
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
            id: 0,
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

fn migrate_raw(
    destinations: Vec<Destination>,
    raw_sources: Vec<RawSource>,
    compact_overview: Option<bool>,
) -> Settings {
    let original_ids: HashSet<u64> = destinations
        .iter()
        .map(|dest| dest.id)
        .filter(|id| *id != 0)
        .collect();
    let mut settings = Settings {
        compact_overview,
        destinations,
        sources: Vec::with_capacity(raw_sources.len()),
    };
    repair_ids(&mut settings.destinations, |destination| &mut destination.id);

    for raw in raw_sources {
        let mut seen = HashSet::new();
        let mut destination_ids = raw.destination_ids;
        destination_ids.retain(|id| original_ids.contains(id) && seen.insert(*id));
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
    repair_ids(&mut settings.sources, |source| &mut source.id);
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
        Err(err) => return Err(settings_file_error(path, err)),
    };
    serde_json::from_slice(&bytes)
        .map(Some)
        .map_err(|_| format!("Inställningsfilen har ett ogiltigt format. Kontrollera filen ”{}” eller återställ den från en säkerhetskopia.", path.display()))
}

fn settings_file_error(path: &std::path::Path, error: std::io::Error) -> String {
    format!(
        "{} Inställningsfil: {}",
        crate::feedback::file_hint(&error),
        path.display()
    )
}

fn write_settings(path: &std::path::Path, value: &impl Serialize) -> Result<(), String> {
    let parent = path.parent().unwrap();
    let file_error = |err| settings_file_error(path, err);
    fs::create_dir_all(parent).map_err(file_error)?;
    let mut file = tempfile::NamedTempFile::new_in(parent).map_err(file_error)?;
    serde_json::to_writer_pretty(&mut file, value).map_err(|err| file_error(err.into()))?;
    file.flush()
        .and_then(|_| file.as_file().sync_all())
        .map_err(file_error)?;
    file.persist(path).map_err(|err| file_error(err.error))?;
    Ok(())
}

#[cfg(any(target_os = "macos", test))]
fn restore_passwords(
    bytes: &[u8],
    destinations: &mut [Destination],
    sources: &mut [RawSource],
) -> Result<(), String> {
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum StoredPasswords {
        Global(Vec<String>),
        Legacy(Vec<Vec<String>>),
    }

    const MISMATCH: &str = "De sparade lösenorden i Nyckelringen hör inte ihop med destinationerna i inställningsfilen. Kontrollera om filen har bytts ut.";
    let passwords = serde_json::from_slice(bytes)
        .map_err(|_| "De sparade lösenorden i Nyckelringen kunde inte läsas. Kontrollera appens uppgifter i Nyckelhanterare.")?;
    match passwords {
        StoredPasswords::Global(passwords) => {
            if passwords.len() != destinations.len() {
                return Err(MISMATCH.to_owned());
            }
            for (destination, password) in destinations.iter_mut().zip(passwords) {
                destination.password = password;
            }
        }
        StoredPasswords::Legacy(passwords) => {
            if passwords.len() != sources.len()
                || sources
                    .iter()
                    .zip(&passwords)
                    .any(|(source, row)| source.destinations.len() != row.len())
            {
                return Err(MISMATCH.to_owned());
            }
            for (source, row) in sources.iter_mut().zip(passwords) {
                for (destination, password) in source.destinations.iter_mut().zip(row) {
                    destination.password = password;
                }
            }
        }
    }
    Ok(())
}

#[cfg(target_os = "macos")]
mod keychain {
    use super::{
        Deserialize, Destination, RawSource, Serialize, Settings, Source, fs, migrate_raw,
        read_settings, restore_passwords, settings_file_error, write_settings,
    };
    use security_framework::passwords::{
        delete_generic_password, generic_password, set_generic_password,
    };
    use std::path::Path;

    const SERVICE: &str = "se.motessandare.app.credentials";

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
        let file_error = |err| settings_file_error(path, err);
        fs::create_dir_all(path.parent().unwrap()).map_err(file_error)?;
        let file = fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(path.with_extension("lock"))
            .map_err(file_error)?;
        file.lock().map_err(file_error)?;
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
            restore_passwords(&bytes, &mut raw.destinations, &mut raw.sources)?;
        }
        Ok(migrate_raw(
            raw.destinations,
            raw.sources,
            raw.compact_overview,
        ))
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
                .map_err(|err| settings_file_error(path, err))?;
            let account = unique
                .path()
                .file_name()
                .unwrap()
                .to_string_lossy()
                .into_owned();
            let data = serde_json::to_vec(&passwords)
                .map_err(|_| "Lösenorden kunde inte förberedas för Nyckelringen.")?;
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

    fn keychain_error(_error: security_framework::base::Error) -> String {
        "Det gick inte att komma åt lösenorden i Nyckelringen. Kontrollera att den är upplåst och att Mötessändare har åtkomst.".into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn restores_global_passwords_including_empty_slots_in_destination_order() {
        let mut destinations = vec![Destination::default(); 3];
        restore_passwords(br#"["first","","third"]"#, &mut destinations, &mut []).unwrap();
        assert_eq!(
            destinations
                .iter()
                .map(|d| d.password.as_str())
                .collect::<Vec<_>>(),
            ["first", "", "third"]
        );
    }

    #[test]
    fn restores_legacy_passwords_before_deduplicating_source_routes() {
        let mut sources: Vec<RawSource> =
            serde_json::from_str(r#"[{"destinations":[{},{}]},{"destinations":[{}]}]"#).unwrap();
        restore_passwords(br#"[["first","shared"],["shared"]]"#, &mut [], &mut sources).unwrap();
        let settings = migrate_raw(Vec::new(), sources, None);
        assert_eq!(settings.destinations.len(), 2);
        assert_eq!(settings.destinations[0].password, "first");
        assert_eq!(settings.destinations[1].password, "shared");
        let first_id = settings.destinations[0].id;
        let shared_id = settings.destinations[1].id;
        assert_eq!(settings.sources[0].destination_ids, [first_id, shared_id]);
        assert_eq!(settings.sources[1].destination_ids, [shared_id]);
    }

    #[test]
    fn rejects_corrupt_or_mismatched_keychain_payloads_without_partial_restoration() {
        let mut destinations = vec![Destination::default(); 2];
        let mut sources: Vec<RawSource> =
            serde_json::from_str(r#"[{"destinations":[{}]},{"destinations":[{}]}]"#).unwrap();
        for payload in [
            "not json",
            r#"["only one global entry"]"#,
            r#"[["only one source"]]"#,
            r#"[["first"],[]]"#,
        ] {
            assert!(
                restore_passwords(payload.as_bytes(), &mut destinations, &mut sources).is_err(),
                "{payload}"
            );
            assert!(destinations.iter().all(|dest| dest.password.is_empty()));
            assert!(
                sources
                    .iter()
                    .flat_map(|s| &s.destinations)
                    .all(|dest| dest.password.is_empty())
            );
        }
    }

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
    fn merges_inline_destinations_with_existing_routes_in_configured_order() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("sources.json");
        fs::write(
            &file,
            r#"{
            "destinations": [
                {"id":3,"name":"Huvud","mount":"/main","password":"shared"},
                {"id":9,"mount":"/backup"}
            ],
            "sources": [{
                "destination_ids":[9],
                "destinations":[
                    {"mount":"/backup"},
                    {"name":"Old name","mount":"/main","password":"shared"},
                    {"mount":"/main","password":"different"}
                ]
            }]
        }"#,
        )
        .unwrap();

        let settings = load_plain(&file).unwrap();
        assert_eq!(settings.destinations.len(), 3);
        assert_eq!(settings.destinations[0].name, "Huvud");
        let routes = settings.sources[0].resolve_destinations(&settings.destinations);
        let endpoints: Vec<_> = routes
            .iter()
            .map(|d| (d.mount.as_str(), d.password.as_str()))
            .collect();
        assert_eq!(
            endpoints,
            [("/backup", ""), ("/main", "shared"), ("/main", "different")]
        );
    }

    #[test]
    fn repairing_invalid_destination_ids_preserves_existing_source_routes() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("sources.json");
        let json = r#"{
            "destinations": [
                { "name": "Saknar id", "mount": "/unassigned.opus" },
                { "id": 1, "name": "Huvud", "mount": "/main.opus" },
                { "id": 1, "name": "Dubblett", "mount": "/duplicate.opus" },
                { "id": 2, "name": "Reserv", "mount": "/backup.opus" }
            ],
            "sources": [
                { "id": 1, "name": "Källa", "destination_ids": [1, 2, 0, 3, 4] }
            ]
        }"#;
        fs::write(&file, json).unwrap();

        let loaded = load_plain(&file).unwrap();
        let mounts: Vec<_> = loaded.sources[0]
            .resolve_destinations(&loaded.destinations)
            .into_iter()
            .map(|dest| dest.mount.as_str())
            .collect();
        assert_eq!(mounts, ["/main.opus", "/backup.opus"]);
        assert_eq!(
            loaded
                .destinations
                .iter()
                .map(|dest| dest.id)
                .collect::<Vec<_>>(),
            [3, 1, 4, 2]
        );
    }

    #[test]
    fn repairs_missing_and_duplicate_source_ids_in_a_legacy_object() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("sources.json");
        fs::write(
            &file,
            r#"{
                "sources": [
                    { "name": "Utan id", "device": "mic-a" },
                    { "id": 1, "name": "Första", "device": "mic-b" },
                    { "id": 1, "name": "Dubblett", "device": "mic-c" },
                    { "id": 2, "name": "Andra", "device": "mic-d" },
                    { "name": "Också utan id", "device": "mic-e" }
                ]
            }"#,
        )
        .unwrap();

        let loaded = load_plain(&file).unwrap();
        let source_devices: Vec<_> = loaded
            .sources
            .iter()
            .map(|source| (source.id, source.device.as_str()))
            .collect();
        assert_eq!(
            source_devices,
            [
                (3, "mic-a"),
                (1, "mic-b"),
                (4, "mic-c"),
                (2, "mic-d"),
                (5, "mic-e")
            ]
        );
        assert_eq!(loaded.next_source_id(), 6);
        save_plain(&file, &loaded).unwrap();
        assert_eq!(load_plain(&file).unwrap(), loaded);
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
