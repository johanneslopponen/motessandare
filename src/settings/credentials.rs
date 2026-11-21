use super::*;
use std::collections::HashMap;

pub(super) trait Store {
    fn put(&self, dir: &Path, data: &[u8]) -> Result<String, String>;
    fn get(&self, account: &str) -> Result<Vec<u8>, String>;
    fn remove(&self, account: &str);
}

pub(super) struct SystemStore;

#[cfg(unix)]
const SERVICE: &str = "se.motessandare.app.credentials";

#[cfg(any(unix, test))]
fn new_account(dir: &Path) -> Result<String, String> {
    let unique = tempfile::Builder::new()
        .prefix("settings-")
        .rand_bytes(24)
        .tempfile_in(dir)
        .map_err(|err| settings_file_error(dir, err))?;
    Ok(unique
        .path()
        .file_name()
        .unwrap()
        .to_string_lossy()
        .into_owned())
}

#[cfg(target_os = "macos")]
impl Store for SystemStore {
    fn put(&self, dir: &Path, data: &[u8]) -> Result<String, String> {
        let account = new_account(dir)?;
        security_framework::passwords::set_generic_password(SERVICE, &account, data).map_err(
            |_| "Kunde inte spara lösenord i Nyckelringen. Kontrollera att den är upplåst.",
        )?;
        Ok(account)
    }
    fn get(&self, account: &str) -> Result<Vec<u8>, String> {
        use security_framework::passwords::{PasswordOptions, generic_password};
        generic_password(PasswordOptions::new_generic_password(SERVICE, account))
            .map_err(|_| "Kunde inte läsa lösenord från Nyckelringen. Kontrollera att den är upplåst och att appen har åtkomst.".into())
    }
    fn remove(&self, account: &str) {
        let _ = security_framework::passwords::delete_generic_password(SERVICE, account);
    }
}

#[cfg(all(unix, not(target_os = "macos")))]
mod secret_service {
    use super::*;
    use dbus_secret_service::{EncryptionType, Item, SecretService};

    const UNAVAILABLE: &str = "Kunde inte använda skrivbordets lösenordsförvaring. Kontrollera att Secret Service är tillgänglig och upplåst.";

    fn service() -> Result<SecretService, String> {
        SecretService::connect(EncryptionType::Dh).map_err(|_| UNAVAILABLE.into())
    }

    fn item<'a>(service: &'a SecretService, account: &str) -> Result<Item<'a>, String> {
        let matches = service
            .search_items(HashMap::from([
                ("application", SERVICE),
                ("account", account),
            ]))
            .map_err(|_| UNAVAILABLE)?;
        let mut items = matches.unlocked.into_iter().chain(matches.locked);
        let item = items
            .next()
            .ok_or("Det sparade lösenordet saknas i Secret Service.")?;
        if items.next().is_some() {
            return Err("Flera lösenordsposter har samma identitet.".into());
        }
        if item.is_locked().map_err(|_| UNAVAILABLE)? {
            item.unlock().map_err(|_| UNAVAILABLE)?;
        }
        Ok(item)
    }

    impl Store for SystemStore {
        fn put(&self, dir: &Path, data: &[u8]) -> Result<String, String> {
            let account = new_account(dir)?;
            let service = service()?;
            let collection = service.get_default_collection().map_err(|_| UNAVAILABLE)?;
            if collection.is_locked().map_err(|_| UNAVAILABLE)? {
                collection.unlock().map_err(|_| UNAVAILABLE)?;
            }
            collection
                .create_item(
                    "Mötessändare",
                    HashMap::from([("application", SERVICE), ("account", account.as_str())]),
                    data,
                    false,
                    "application/json",
                )
                .map_err(|_| UNAVAILABLE)?;
            Ok(account)
        }
        fn get(&self, account: &str) -> Result<Vec<u8>, String> {
            item(&service()?, account)?
                .get_secret()
                .map_err(|_| UNAVAILABLE.into())
        }
        fn remove(&self, account: &str) {
            if let Ok(service) = service()
                && let Ok(old) = item(&service, account)
            {
                let _ = old.delete();
            }
        }
    }
}

#[cfg(windows)]
mod windows {
    use super::*;
    use base64::{Engine, engine::general_purpose::STANDARD};
    use std::ptr::{null, null_mut};
    use windows_sys::Win32::{
        Foundation::LocalFree,
        Security::Cryptography::{
            CRYPT_INTEGER_BLOB, CRYPTPROTECT_UI_FORBIDDEN, CryptProtectData, CryptUnprotectData,
        },
    };

    impl Store for SystemStore {
        fn put(&self, _dir: &Path, data: &[u8]) -> Result<String, String> {
            Ok(format!("dpapi:{}", STANDARD.encode(transform(data, true)?)))
        }
        fn get(&self, account: &str) -> Result<Vec<u8>, String> {
            let encoded = account
                .strip_prefix("dpapi:")
                .ok_or("De sparade lösenorden tillhör en annan lösenordsförvaring.")?;
            let bytes = STANDARD
                .decode(encoded)
                .map_err(|_| "Ogiltiga skyddade lösenordsdata.")?;
            transform(&bytes, false)
        }
        // The encrypted blob lives in the atomically replaced settings file.
        fn remove(&self, _account: &str) {}
    }

    fn transform(data: &[u8], encrypt: bool) -> Result<Vec<u8>, String> {
        let input = CRYPT_INTEGER_BLOB {
            cbData: data
                .len()
                .try_into()
                .map_err(|_| "Lösenordsdata är för stora.")?,
            pbData: data.as_ptr().cast_mut(),
        };
        let mut output = CRYPT_INTEGER_BLOB::default();
        // DPAPI reads input during this call and allocates output with LocalAlloc.
        // Omitting CRYPTPROTECT_LOCAL_MACHINE binds protection to this user.
        let ok = unsafe {
            if encrypt {
                CryptProtectData(
                    &input,
                    null(),
                    null(),
                    null(),
                    null(),
                    CRYPTPROTECT_UI_FORBIDDEN,
                    &mut output,
                )
            } else {
                CryptUnprotectData(
                    &input,
                    null_mut(),
                    null(),
                    null(),
                    null(),
                    CRYPTPROTECT_UI_FORBIDDEN,
                    &mut output,
                )
            }
        };
        if ok == 0 {
            return Err(
                "Windows kunde inte skydda eller läsa lösenorden med det aktuella användarkontot."
                    .into(),
            );
        }
        // A successful call provides cbData valid bytes. Copy before freeing them.
        let bytes = if output.cbData == 0 {
            Vec::new()
        } else {
            unsafe { std::slice::from_raw_parts(output.pbData, output.cbData as usize) }.to_vec()
        };
        unsafe {
            LocalFree(output.pbData.cast());
        }
        Ok(bytes)
    }
}

#[derive(Serialize, Deserialize)]
struct Credential {
    id: u64,
    server: String,
    mount: String,
    user: String,
    password: String,
}

#[derive(Serialize, Deserialize)]
struct BoundPasswords {
    credentials: Vec<Credential>,
}

// Returns true for older formats that need upgrading before the UI can use them.
pub(crate) fn restore_passwords(
    bytes: &[u8],
    destinations: &mut [Destination],
    sources: &mut [RawSource],
) -> Result<bool, String> {
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Stored {
        Bound(BoundPasswords),
        Global(Vec<String>),
        Legacy(Vec<Vec<String>>),
    }
    const MISMATCH: &str = "De sparade lösenorden hör inte ihop med destinationerna i inställningsfilen. Kontrollera om filen har bytts ut.";
    match serde_json::from_slice(bytes).map_err(|_| "De sparade lösenorden kunde inte läsas.")? {
        Stored::Bound(bound) => {
            let mut by_id = HashMap::new();
            for record in bound.credentials {
                if by_id.insert(record.id, record).is_some() {
                    return Err(MISMATCH.into());
                }
            }
            let mut seen = HashSet::new();
            if destinations.len() != by_id.len()
                || destinations.iter().any(|d| {
                    !seen.insert(d.id)
                        || !by_id.get(&d.id).is_some_and(|r| {
                            r.server == d.server && r.mount == d.mount && r.user == d.user
                        })
                })
            {
                return Err(MISMATCH.into());
            }
            for d in destinations {
                d.password = by_id.remove(&d.id).unwrap().password;
            }
            Ok(false)
        }
        Stored::Global(passwords) => {
            if passwords.len() != destinations.len() {
                return Err(MISMATCH.into());
            }
            for (dest, password) in destinations.iter_mut().zip(passwords) {
                dest.password = password;
            }
            Ok(true)
        }
        Stored::Legacy(passwords) => {
            if passwords.len() != sources.len()
                || sources
                    .iter()
                    .zip(&passwords)
                    .any(|(s, row)| s.destinations.len() != row.len())
            {
                return Err(MISMATCH.into());
            }
            for (source, row) in sources.iter_mut().zip(passwords) {
                for (dest, password) in source.destinations.iter_mut().zip(row) {
                    dest.password = password;
                }
            }
            Ok(true)
        }
    }
}

fn lock(path: &Path) -> Result<fs::File, String> {
    let file_error = |err| settings_file_error(path, err);
    fs::create_dir_all(settings_dir(path)?).map_err(file_error)?;
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

fn hydrate(
    stored: StoredSettings,
    store: &impl Store,
) -> Result<(Settings, Option<String>, bool), String> {
    let mut raw = match stored {
        StoredSettings::Object(raw) => raw,
        StoredSettings::LegacyArray(sources) => RawSettingsObject {
            credential_account: None,
            compact_overview: None,
            theme: ThemePreference::System,
            destinations: Vec::new(),
            sources,
        },
    };
    let mut upgrade = raw
        .destinations
        .iter()
        .chain(raw.sources.iter().flat_map(|s| &s.destinations))
        .any(|d| !d.password.is_empty());
    if let Some(account) = &raw.credential_account {
        upgrade |= restore_passwords(
            &store.get(account)?,
            &mut raw.destinations,
            &mut raw.sources,
        )?;
    }
    let mut settings = migrate_raw(
        raw.destinations,
        raw.sources,
        raw.compact_overview,
        raw.theme,
    );
    for dest in &mut settings.destinations {
        if let Some(server) = dest.server_without_secrets() {
            dest.server = server;
            upgrade = true;
        }
    }
    Ok((settings, raw.credential_account, upgrade))
}

pub(super) fn load(path: &Path, store: &impl Store) -> Result<Settings, String> {
    let _lock = lock(path)?;
    let Some(stored) = read_settings(path)? else {
        return Ok(Settings::default());
    };
    let (settings, previous, upgrade) = hydrate(stored, store)?;
    if upgrade {
        commit(path, &settings, previous.as_deref(), store)?;
    }
    Ok(settings)
}

pub(super) fn save(path: &Path, settings: &Settings, store: &impl Store) -> Result<(), String> {
    let _lock = lock(path)?;
    let previous = read_settings(path)?
        .map(|stored| hydrate(stored, store))
        .transpose()?
        .and_then(|(_, previous, _)| previous);
    commit(path, settings, previous.as_deref(), store)
}

fn commit(
    path: &Path,
    settings: &Settings,
    previous: Option<&str>,
    store: &impl Store,
) -> Result<(), String> {
    #[derive(Serialize)]
    struct ProtectedSettings {
        credential_account: Option<String>,
        #[serde(flatten)]
        settings: Settings,
    }
    let mut safe = settings.clone();
    repair_ids(&mut safe.destinations, |destination| &mut destination.id);
    for dest in &mut safe.destinations {
        if let Some(server) = dest.server_without_secrets() {
            dest.server = server;
        }
    }
    let account = if safe.destinations.iter().any(|d| !d.password.is_empty()) {
        let credentials = safe
            .destinations
            .iter_mut()
            .map(|d| Credential {
                id: d.id,
                server: d.server.clone(),
                mount: d.mount.clone(),
                user: d.user.clone(),
                password: std::mem::take(&mut d.password),
            })
            .collect();
        let bytes = serde_json::to_vec(&BoundPasswords { credentials })
            .map_err(|_| "Lösenorden kunde inte sparas.")?;
        Some(store.put(settings_dir(path)?, &bytes)?)
    } else {
        None
    };
    let protected = ProtectedSettings {
        credential_account: account,
        settings: safe,
    };
    if let Err(error) = write_settings(path, &protected) {
        if let Some(new) = &protected.credential_account {
            store.remove(new);
        }
        return Err(error);
    }
    if let Some(old) = previous {
        store.remove(old);
    }
    Ok(())
}

#[cfg(test)]
pub(super) struct TestStore;

#[cfg(test)]
thread_local! {
    static SECRETS: std::cell::RefCell<HashMap<String, Vec<u8>>> = std::cell::RefCell::default();
}

#[cfg(test)]
impl Store for TestStore {
    fn put(&self, dir: &Path, data: &[u8]) -> Result<String, String> {
        let account = new_account(dir)?;
        SECRETS.with_borrow_mut(|s| s.insert(account.clone(), data.to_vec()));
        Ok(account)
    }
    fn get(&self, account: &str) -> Result<Vec<u8>, String> {
        SECRETS
            .with_borrow(|s| s.get(account).cloned())
            .ok_or_else(|| "Missing credential".into())
    }
    fn remove(&self, account: &str) {
        SECRETS.with_borrow_mut(|s| s.remove(account));
    }
}
