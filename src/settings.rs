mod credentials;

use serde::{Deserialize, Serialize};
use std::{
    collections::HashSet,
    env, fs,
    io::{Read, Write},
    path::{Path, PathBuf},
};

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
    // Publishing uses only the HTTP origin. Drop pasted URL credentials and
    // tokens before the server field is persisted or shown in a summary.
    pub fn server_without_secrets(&self) -> Option<String> {
        let trimmed = self.server.trim();
        let parsed = url::Url::parse(trimmed);
        if let Ok(mut url) = parsed {
            if matches!(url.scheme(), "http" | "https") {
                return (!url.username().is_empty()
                    || url.password().is_some()
                    || url.query().is_some()
                    || url.fragment().is_some())
                .then(|| url.origin().ascii_serialization());
            }
            if !url.cannot_be_a_base() {
                let had_secrets = !url.username().is_empty()
                    || url.password().is_some()
                    || url.query().is_some()
                    || url.fragment().is_some();
                if !had_secrets {
                    return None;
                }
                let _ = url.set_username("");
                let _ = url.set_password(None);
                url.set_query(None);
                url.set_fragment(None);
                let cleaned = url.to_string();
                return (cleaned != trimmed).then_some(cleaned);
            }
            let prefix = format!("{}:", url.scheme());
            let rest = &trimmed[prefix.len()..];
            let without_qf = rest.split(['?', '#']).next().unwrap_or("");
            let clean_rest = match without_qf.rsplit_once('@') {
                Some((_, after)) => after,
                None => without_qf,
            };
            let cleaned = format!("{prefix}{clean_rest}");
            return (cleaned != trimmed).then_some(cleaned);
        }
        let (prefix, rest) = if let Some((scheme, rest)) = trimmed.split_once("://") {
            (format!("{scheme}://"), rest)
        } else {
            (String::new(), trimmed)
        };
        let without_qf = rest.split(['?', '#']).next().unwrap_or("");
        let clean_rest = match without_qf.rsplit_once('@') {
            Some((_, after)) => after,
            None => without_qf,
        };
        let cleaned = format!("{prefix}{clean_rest}");
        (cleaned != trimmed).then_some(cleaned)
    }

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

    pub fn resolve_destinations<'a>(
        &self,
        destinations: &'a [Destination],
    ) -> Vec<&'a Destination> {
        self.destination_ids
            .iter()
            .filter_map(|id| destinations.iter().find(|dest| dest.id == *id))
            .collect()
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ThemePreference {
    #[default]
    System,
    Light,
    Dark,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub compact_overview: Option<bool>,
    pub theme: ThemePreference,
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
    credential_account: Option<String>,
    compact_overview: Option<bool>,
    #[serde(default)]
    theme: ThemePreference,
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
    theme: ThemePreference,
) -> Settings {
    let original_ids: HashSet<u64> = destinations
        .iter()
        .map(|dest| dest.id)
        .filter(|id| *id != 0)
        .collect();
    let mut settings = Settings {
        compact_overview,
        theme,
        destinations,
        sources: Vec::with_capacity(raw_sources.len()),
    };
    repair_ids(&mut settings.destinations, |destination| {
        &mut destination.id
    });

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
    credentials::load(&path(), &credentials::SystemStore)
}

pub fn settings_save(settings: &Settings) -> Result<(), String> {
    credentials::save(&path(), settings, &credentials::SystemStore)
}

const MAX_SETTINGS_BYTES: u64 = 16 * 1024 * 1024;

fn read_settings<T: serde::de::DeserializeOwned>(
    path: &std::path::Path,
) -> Result<Option<T>, String> {
    let file = match fs::File::open(path) {
        Ok(file) => file,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(err) => return Err(settings_file_error(path, err)),
    };
    let mut bytes = Vec::new();
    file.take(MAX_SETTINGS_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|err| settings_file_error(path, err))?;
    if bytes.len() as u64 > MAX_SETTINGS_BYTES {
        return Err("Inställningsfilen är för stor för att öppna.".into());
    }
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

fn settings_dir(path: &Path) -> Result<&Path, String> {
    path.parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .ok_or_else(|| {
            settings_file_error(
                path,
                std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    "missing parent directory",
                ),
            )
        })
}

fn write_settings(path: &std::path::Path, value: &impl Serialize) -> Result<(), String> {
    let parent = settings_dir(path)?;
    let file_error = |err| settings_file_error(path, err);
    fs::create_dir_all(parent).map_err(file_error)?;
    let mut file = tempfile::NamedTempFile::new_in(parent).map_err(file_error)?;
    serde_json::to_writer_pretty(&mut file, value).map_err(|err| file_error(err.into()))?;
    file.flush()
        .and_then(|_| file.as_file().sync_all())
        .map_err(file_error)?;
    if file.as_file().metadata().map_err(file_error)?.len() > MAX_SETTINGS_BYTES {
        return Err("Inställningarna är för stora för att spara.".into());
    }
    file.persist(path).map_err(|err| file_error(err.error))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use credentials::{Store, TestStore, restore_passwords};

    fn load_test_settings(path: &Path) -> Result<Settings, String> {
        credentials::load(path, &TestStore)
    }

    fn save_test_settings(path: &Path, settings: &Settings) -> Result<(), String> {
        credentials::save(path, settings, &TestStore)
    }

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
        let settings = migrate_raw(Vec::new(), sources, None, ThemePreference::System);
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
            theme: ThemePreference::Dark,
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

        save_test_settings(&file, &settings).unwrap();
        let loaded = load_test_settings(&file).unwrap();
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

        let loaded = load_test_settings(&file).unwrap();
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

        let loaded = load_test_settings(&file).unwrap();
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

        let settings = load_test_settings(&file).unwrap();
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

        let loaded = load_test_settings(&file).unwrap();
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

        let loaded = load_test_settings(&file).unwrap();
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
        save_test_settings(&file, &loaded).unwrap();
        assert_eq!(load_test_settings(&file).unwrap(), loaded);
    }

    #[test]
    fn refuses_to_overwrite_unreadable_config() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("sources.json");
        fs::write(&file, "{ not valid json").unwrap();

        let result = save_test_settings(&file, &Settings::default());
        assert!(result.is_err());
        assert_eq!(fs::read_to_string(&file).unwrap(), "{ not valid json");
    }

    #[test]
    fn migrates_plaintext_object_and_binds_secrets_to_destinations() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("sources.json");
        fs::write(&file, r#"{"destinations":[{"id":1,"server":"https://user:url-secret@first.example/unused?token=query-secret","password":"first secret"},{"id":2,"server":"https://second.example","password":"second secret"}]}"#).unwrap();
        let original = load_test_settings(&file).unwrap();
        assert_eq!(original.destinations[0].server, "https://first.example");
        let saved = fs::read_to_string(&file).unwrap();
        assert!(!saved.contains("secret") && !saved.contains("\"password\""));
        let mut json: serde_json::Value = serde_json::from_str(&saved).unwrap();
        json["destinations"].as_array_mut().unwrap().reverse();
        fs::write(&file, json.to_string()).unwrap();
        let loaded = load_test_settings(&file).unwrap();
        assert_eq!(loaded.destinations[0], original.destinations[1]);
        assert_eq!(loaded.destinations[1], original.destinations[0]);
        json["destinations"][0]["server"] = "https://other.example".into();
        let changed = json.to_string();
        fs::write(&file, &changed).unwrap();
        assert!(load_test_settings(&file).is_err());
        assert!(save_test_settings(&file, &original).is_err());
        assert_eq!(fs::read_to_string(&file).unwrap(), changed);
    }

    #[test]
    fn upgrades_old_keychain_references_only_after_saving_the_new_settings() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("sources.json");
        let old_account = TestStore.put(dir.path(), br#"["old secret"]"#).unwrap();
        let old =
            serde_json::json!({"credential_account": old_account, "destinations":[{"id": 7}]});
        fs::write(&file, old.to_string()).unwrap();
        let settings = load_test_settings(&file).unwrap();
        assert_eq!(settings.destinations[0].password, "old secret");
        assert!(TestStore.get(&old_account).is_err());
        assert_eq!(load_test_settings(&file).unwrap(), settings);
    }

    #[test]
    fn credential_store_failure_preserves_the_original_file() {
        struct Unavailable;
        impl Store for Unavailable {
            fn put(&self, _: &Path, _: &[u8]) -> Result<String, String> {
                Err("Unavailable".into())
            }
            fn get(&self, _: &str) -> Result<Vec<u8>, String> {
                Err("Unavailable".into())
            }
            fn remove(&self, _: &str) {
                panic!("no credentials should be removed");
            }
        }
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("sources.json");
        let legacy = r#"[{"destinations":[{"password":"old secret"}]}]"#;
        fs::write(&file, legacy).unwrap();
        assert!(credentials::load(&file, &Unavailable).is_err());
        assert_eq!(fs::read_to_string(&file).unwrap(), legacy);
    }

    #[test]
    #[cfg_attr(
        not(windows),
        ignore = "requires access to an unlocked desktop credential store"
    )]
    fn round_trips_and_removes_credentials_with_the_system_store() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("sources.json");
        let store = credentials::SystemStore;
        fs::write(
            &file,
            r#"{"destinations":[{"id":1,"password":"system-store test secret"}]}"#,
        )
        .unwrap();
        let settings = credentials::load(&file, &store).unwrap();
        assert_eq!(
            settings.destinations[0].password,
            "system-store test secret"
        );
        assert!(
            !fs::read_to_string(&file)
                .unwrap()
                .contains("system-store test secret")
        );
        assert_eq!(credentials::load(&file, &store).unwrap(), settings);
        credentials::save(&file, &Settings::default(), &store).unwrap();
        assert_eq!(
            credentials::load(&file, &store).unwrap(),
            Settings::default()
        );
    }

    #[test]
    fn server_without_secrets_strips_malformed_and_non_http_urls_idempotently() {
        for (input, expected) in [
            ("https://stream.example.org:8443", None),
            ("ftp://example.org:21/live", None),
            (
                "http://user:secret@example.org:99999/live?token=1#frag",
                Some("http://example.org:99999/live"),
            ),
            (
                "ftp://user:secret@example.org:21/live?token=1#frag",
                Some("ftp://example.org/live"),
            ),
            (
                "javascript:user:secret@example.org:8000?token=1",
                Some("javascript:example.org:8000"),
            ),
            (
                "http://example.org:https://user:secret@example.org:8443/p?token=1#f",
                Some("http://example.org:8443/p"),
            ),
        ] {
            let dest = Destination {
                server: input.into(),
                ..Destination::default()
            };
            assert_eq!(dest.server_without_secrets().as_deref(), expected, "{input}");
            if let Some(cleaned) = expected {
                let rerun = Destination {
                    server: cleaned.into(),
                    ..Destination::default()
                };
                assert_eq!(rerun.server_without_secrets(), None, "{cleaned}");
            }
        }
    }

    #[test]
    fn save_repairs_duplicate_destination_ids_and_rejects_rootless_paths() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("sources.json");
        let duplicate = Settings {
            destinations: vec![
                Destination {
                    id: 1,
                    name: "A".into(),
                    password: "p1".into(),
                    ..Destination::default()
                },
                Destination {
                    id: 1,
                    name: "B".into(),
                    password: "p2".into(),
                    ..Destination::default()
                },
            ],
            ..Settings::default()
        };
        save_test_settings(&file, &duplicate).unwrap();
        let loaded = load_test_settings(&file).unwrap();
        assert_eq!(
            loaded
                .destinations
                .iter()
                .map(|d| (d.id, d.password.as_str()))
                .collect::<Vec<_>>(),
            [(1, "p1"), (2, "p2")]
        );

        for bad in [Path::new(""), Path::new("/"), Path::new("sources.json")] {
            assert!(load_test_settings(bad).is_err(), "{bad:?}");
            assert!(save_test_settings(bad, &Settings::default()).is_err(), "{bad:?}");
            assert!(write_settings(bad, &serde_json::json!({})).is_err(), "{bad:?}");
        }
    }
}
