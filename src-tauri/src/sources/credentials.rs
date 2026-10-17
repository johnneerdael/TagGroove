//! The Discogs personal access token, kept in `tag-sources.json` beside the
//! other configuration files rather than in the shared Options store, or, when
//! the user chooses it, in the system keychain. It never travels back to the
//! webview, which only learns whether one is set and where it is kept. The
//! user's own API keys for AcoustID, Last.fm and fanart.tv live in the same
//! private file and are never shown again either.
//!
//! The keychain is opt-in and only touched when the token is saved, moved or
//! needed for a Discogs request. A plain-text token stays in the private file
//! until the user moves it; moving it writes the keychain first and then
//! removes the copy from the file.
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Credentials {
    pub discogs_token: String,
    /// The token is in the system keychain; `discogs_token` is then empty.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub discogs_token_in_keychain: bool,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub acoustid_key: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub lastfm_key: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub fanart_key: String,
}

#[derive(Debug, PartialEq, Serialize)]
pub struct CredentialsView {
    pub discogs_token_set: bool,
    pub discogs_token_in_keychain: bool,
    /// Whether this platform build has a keychain to offer.
    pub keychain_available: bool,
    /// This build carries TagGroove's Discogs application key.
    pub discogs_app_key: bool,
    pub acoustid_key_set: bool,
    pub acoustid_built_in: bool,
    pub lastfm_key_set: bool,
    pub lastfm_built_in: bool,
    pub fanart_key_set: bool,
    pub fanart_built_in: bool,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct CredentialsUpdate {
    /// A new token, or an empty string to remove it; absent keeps the token.
    pub discogs_token: Option<String>,
    /// Where to keep the token; absent keeps the current place.
    pub keychain: Option<bool>,
    /// New API keys, or empty strings to remove them; absent keeps them.
    pub acoustid_key: Option<String>,
    pub lastfm_key: Option<String>,
    pub fanart_key: Option<String>,
}

/// One secret in a credential store. The app uses the system keychain; tests
/// and scripted end-to-end runs use [`MemoryStore`] and never reach it.
pub trait SecretStore: Send + Sync {
    fn available(&self) -> bool;
    fn get(&self) -> Result<Option<String>, String>;
    fn set(&self, secret: &str) -> Result<(), String>;
    /// Removing a secret that does not exist succeeds.
    fn delete(&self) -> Result<(), String>;
}

#[derive(Default)]
pub struct MemoryStore(std::sync::Mutex<Option<String>>);
impl SecretStore for MemoryStore {
    fn available(&self) -> bool {
        true
    }
    fn get(&self) -> Result<Option<String>, String> {
        Ok(self.0.lock().map_err(|e| e.to_string())?.clone())
    }
    fn set(&self, secret: &str) -> Result<(), String> {
        *self.0.lock().map_err(|e| e.to_string())? = Some(secret.to_owned());
        Ok(())
    }
    fn delete(&self) -> Result<(), String> {
        *self.0.lock().map_err(|e| e.to_string())? = None;
        Ok(())
    }
}

/// macOS Keychain, Windows Credential Manager or the Secret Service on Linux,
/// under the app identifier so test builds never see the app's entry.
#[cfg(not(any(target_os = "android", target_os = "ios")))]
pub struct KeychainStore {
    pub service: String,
}
#[cfg(not(any(target_os = "android", target_os = "ios")))]
impl KeychainStore {
    const ACCOUNT: &str = "discogs-token";
    /// Keychain calls may block (a Secret Service round trip or a system
    /// prompt), so they run on their own thread outside the async runtime.
    fn with_entry<T: Send + 'static>(
        &self,
        work: impl FnOnce(keyring::Entry) -> keyring::Result<T> + Send + 'static,
    ) -> Result<T, String> {
        let service = self.service.clone();
        std::thread::spawn(move || work(keyring::Entry::new(&service, Self::ACCOUNT)?))
            .join()
            .map_err(|_| "The system keychain stopped responding.".to_string())?
            .map_err(|e| format!("The system keychain refused the request: {e}"))
    }
}
#[cfg(not(any(target_os = "android", target_os = "ios")))]
impl SecretStore for KeychainStore {
    fn available(&self) -> bool {
        true
    }
    fn get(&self) -> Result<Option<String>, String> {
        self.with_entry(|entry| match entry.get_password() {
            Ok(secret) => Ok(Some(secret)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(error) => Err(error),
        })
    }
    fn set(&self, secret: &str) -> Result<(), String> {
        let secret = secret.to_owned();
        self.with_entry(move |entry| entry.set_password(&secret))
    }
    fn delete(&self) -> Result<(), String> {
        self.with_entry(|entry| match entry.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(error) => Err(error),
        })
    }
}

/// Mobile builds keep the token in the private file only.
#[cfg_attr(
    not(any(test, target_os = "android", target_os = "ios")),
    allow(dead_code)
)]
pub struct NoKeychain;
impl SecretStore for NoKeychain {
    fn available(&self) -> bool {
        false
    }
    fn get(&self) -> Result<Option<String>, String> {
        Ok(None)
    }
    fn set(&self, _: &str) -> Result<(), String> {
        Err("No system keychain is available here.".into())
    }
    fn delete(&self) -> Result<(), String> {
        Ok(())
    }
}

fn validated_key(service: &str, key: &str) -> Result<String, String> {
    let key = key.trim().to_owned();
    if key.len() > 200 || key.chars().any(|c| !c.is_ascii_graphic()) {
        return Err(format!(
            "The {service} API key contains unexpected characters. Paste the key exactly as {service} shows it."
        ));
    }
    Ok(key)
}

fn validated(token: &str) -> Result<String, String> {
    let token = token.trim().to_owned();
    if token.len() > 200 || token.chars().any(|c| !c.is_ascii_graphic()) {
        return Err("The Discogs token contains unexpected characters. Paste the personal access token from your Discogs developer settings.".into());
    }
    Ok(token)
}

impl Credentials {
    /// Never reads the keychain: a keychain entry counts as set.
    pub fn view(&self, store: &dyn SecretStore) -> CredentialsView {
        use super::keys;
        CredentialsView {
            discogs_token_set: self.discogs_token_in_keychain || !self.discogs_token.is_empty(),
            discogs_token_in_keychain: self.discogs_token_in_keychain,
            keychain_available: store.available(),
            discogs_app_key: keys::discogs_app().is_some(),
            acoustid_key_set: !self.acoustid_key.is_empty(),
            acoustid_built_in: keys::acoustid().is_some(),
            lastfm_key_set: !self.lastfm_key.is_empty(),
            lastfm_built_in: keys::lastfm().is_some(),
            fanart_key_set: !self.fanart_key.is_empty(),
            fanart_built_in: keys::fanart().is_some(),
        }
    }

    /// The token for a Discogs request; empty when none is set.
    pub fn discogs_token(&self, store: &dyn SecretStore) -> Result<String, String> {
        if !self.discogs_token_in_keychain {
            return Ok(self.discogs_token.clone());
        }
        store
            .get()
            .map(Option::unwrap_or_default)
            .map_err(|e| format!("The Discogs token could not be read. {e}"))
    }
}

pub fn load(path: &Path) -> Result<Credentials, String> {
    match std::fs::read(path) {
        Ok(bytes) => serde_json::from_slice(&bytes)
            .map_err(|e| format!("Cannot read tag source options: {e}")),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Credentials::default()),
        Err(e) => Err(e.to_string()),
    }
}

/// Written atomically through a private (0600 on Unix) temporary file.
pub fn save(path: &Path, credentials: &Credentials) -> Result<(), String> {
    let bytes = serde_json::to_vec_pretty(credentials).map_err(|e| e.to_string())?;
    crate::action_host::atomic_output(path, &bytes, false)
}

/// Replace, remove or move the token. The keychain is written and read back
/// before the file forgets the token, and the file holds the token again
/// before the keychain entry is removed, so a failure never loses it.
pub fn update(
    path: &Path,
    store: &dyn SecretStore,
    update: CredentialsUpdate,
) -> Result<CredentialsView, String> {
    let mut current = load(path)?;
    let mut keys_changed = false;
    for (service, slot, value) in [
        ("AcoustID", &mut current.acoustid_key, &update.acoustid_key),
        ("Last.fm", &mut current.lastfm_key, &update.lastfm_key),
        ("fanart.tv", &mut current.fanart_key, &update.fanart_key),
    ] {
        if let Some(value) = value {
            let key = validated_key(service, value)?;
            keys_changed |= *slot != key;
            *slot = key;
        }
    }
    let keychain = update.keychain.unwrap_or(current.discogs_token_in_keychain);
    let token = match &update.discogs_token {
        Some(token) => validated(token)?,
        None if keychain == current.discogs_token_in_keychain => {
            if keys_changed {
                save(path, &current)?;
            }
            return Ok(current.view(store));
        }
        None => current.discogs_token(store)?,
    };
    if keychain && !token.is_empty() {
        if !store.available() {
            return Err("No system keychain is available here.".into());
        }
        store.set(&token)?;
        if store.get()?.as_deref() != Some(token.as_str()) {
            return Err("The system keychain did not keep the token. It is unchanged.".into());
        }
        let next = Credentials {
            discogs_token: String::new(),
            discogs_token_in_keychain: true,
            ..current.clone()
        };
        save(path, &next)?;
        return Ok(next.view(store));
    }
    let next = Credentials {
        discogs_token: token,
        discogs_token_in_keychain: false,
        ..current.clone()
    };
    save(path, &next)?;
    if current.discogs_token_in_keychain {
        store.delete()?;
    }
    Ok(next.view(store))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn set(token: &str) -> CredentialsUpdate {
        CredentialsUpdate {
            discogs_token: Some(token.into()),
            ..Default::default()
        }
    }
    fn stored(keychain: bool) -> CredentialsUpdate {
        CredentialsUpdate {
            keychain: Some(keychain),
            ..Default::default()
        }
    }

    #[test]
    fn api_keys_are_saved_privately_kept_with_the_token_and_never_viewed() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("tag-sources.json");
        let store = MemoryStore::default();
        let view = update(
            &path,
            &store,
            CredentialsUpdate {
                lastfm_key: Some(" lastfm-key ".into()),
                acoustid_key: Some("acoustid-key".into()),
                ..Default::default()
            },
        )
        .unwrap();
        assert!(view.lastfm_key_set && view.acoustid_key_set && !view.fanart_key_set);
        let json = serde_json::to_string(&view).unwrap();
        assert!(!json.contains("lastfm-key") && !json.contains("acoustid-key"));
        assert_eq!(load(&path).unwrap().lastfm_key, "lastfm-key");
        // Moving or replacing the Discogs token keeps the API keys.
        update(&path, &store, set("token")).unwrap();
        update(&path, &store, stored(true)).unwrap();
        update(&path, &store, stored(false)).unwrap();
        let saved = load(&path).unwrap();
        assert_eq!(
            (saved.acoustid_key.as_str(), saved.discogs_token.as_str()),
            ("acoustid-key", "token")
        );
        // An empty key removes it; a malformed key changes nothing.
        let bad = CredentialsUpdate {
            fanart_key: Some("a b".into()),
            lastfm_key: Some(String::new()),
            ..Default::default()
        };
        assert!(update(&path, &store, bad)
            .unwrap_err()
            .contains("fanart.tv"));
        assert_eq!(load(&path).unwrap().lastfm_key, "lastfm-key");
        let cleared = update(
            &path,
            &store,
            CredentialsUpdate {
                lastfm_key: Some(String::new()),
                ..Default::default()
            },
        )
        .unwrap();
        assert!(!cleared.lastfm_key_set && cleared.acoustid_key_set);
    }

    #[test]
    fn token_is_replaced_or_cleared_and_never_viewed() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("tag-sources.json");
        let store = NoKeychain;
        save(
            &path,
            &Credentials {
                discogs_token: "abc".into(),
                ..Default::default()
            },
        )
        .unwrap();
        update(&path, &store, set(" xyz ")).unwrap();
        assert_eq!(load(&path).unwrap().discogs_token, "xyz");
        let cleared = update(&path, &store, set("")).unwrap();
        assert!(!cleared.discogs_token_set && !cleared.keychain_available);
        assert!(update(&path, &store, set("a b")).is_err());
        assert!(
            update(&path, &store, stored(true)).is_ok(),
            "nothing to move"
        );
        update(&path, &store, set("abc")).unwrap();
        assert!(update(&path, &store, stored(true)).is_err());
        assert_eq!(load(&path).unwrap().discogs_token, "abc", "kept on failure");
        let json = serde_json::to_string(&load(&path).unwrap().view(&store)).unwrap();
        assert!(!json.contains("abc") && json.contains("true"));
    }

    #[test]
    fn tokens_move_to_and_from_the_keychain_without_leaving_copies() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("tag-sources.json");
        let store = MemoryStore::default();
        // A plain-text token stays where it is until the user moves it.
        update(&path, &store, set("secret")).unwrap();
        assert_eq!(store.get().unwrap(), None);
        let moved = update(&path, &store, stored(true)).unwrap();
        assert!(moved.discogs_token_set && moved.discogs_token_in_keychain);
        let text = std::fs::read_to_string(&path).unwrap();
        assert!(!text.contains("secret"), "{text}");
        assert_eq!(store.get().unwrap().as_deref(), Some("secret"));
        assert_eq!(
            load(&path).unwrap().discogs_token(&store).unwrap(),
            "secret"
        );

        // A new token goes to the keychain; removing it deletes the entry.
        update(&path, &store, set("other")).unwrap();
        assert_eq!(store.get().unwrap().as_deref(), Some("other"));
        assert!(!std::fs::read_to_string(&path).unwrap().contains("other"));
        let back = update(&path, &store, stored(false)).unwrap();
        assert!(back.discogs_token_set && !back.discogs_token_in_keychain);
        assert_eq!(load(&path).unwrap().discogs_token, "other");
        assert_eq!(store.get().unwrap(), None);

        update(&path, &store, stored(true)).unwrap();
        let removed = update(&path, &store, set("")).unwrap();
        assert!(!removed.discogs_token_set);
        assert_eq!(store.get().unwrap(), None);
        assert_eq!(load(&path).unwrap(), Credentials::default());
    }

    /// A keychain that accepts writes but loses them must not cost the token.
    #[test]
    fn a_failing_keychain_keeps_the_file_token() {
        struct Forgetful;
        impl SecretStore for Forgetful {
            fn available(&self) -> bool {
                true
            }
            fn get(&self) -> Result<Option<String>, String> {
                Ok(None)
            }
            fn set(&self, _: &str) -> Result<(), String> {
                Ok(())
            }
            fn delete(&self) -> Result<(), String> {
                Err("locked".into())
            }
        }
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("tag-sources.json");
        update(&path, &Forgetful, set("secret")).unwrap();
        assert!(update(&path, &Forgetful, stored(true)).is_err());
        assert_eq!(load(&path).unwrap().discogs_token, "secret");
    }

    #[test]
    fn credentials_round_trip_privately_and_tolerate_old_files() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nested").join("tag-sources.json");
        assert_eq!(load(&path).unwrap(), Credentials::default());
        let credentials = Credentials {
            discogs_token: "token".into(),
            ..Default::default()
        };
        save(&path, &credentials).unwrap();
        assert_eq!(load(&path).unwrap(), credentials);
        assert!(!std::fs::read_to_string(&path).unwrap().contains("keychain"));
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                std::fs::metadata(&path).unwrap().permissions().mode() & 0o077,
                0,
                "token file is private"
            );
        }
        // Earlier builds also stored the cover size here; it now lives in Options.
        std::fs::write(
            &path,
            "{\"discogs_token\":\"t\",\"musicbrainz_cover_size\":\"250\"}",
        )
        .unwrap();
        assert_eq!(load(&path).unwrap().discogs_token, "t");
    }
}
