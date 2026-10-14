//! The Discogs personal access token, kept in `tag-sources.json` beside the
//! other configuration files rather than in the shared Options store. It never
//! travels back to the webview, which only learns whether one is set.
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Credentials {
    pub discogs_token: String,
}

#[derive(Debug, PartialEq, Serialize)]
pub struct CredentialsView {
    pub discogs_token_set: bool,
}

#[derive(Debug, Deserialize)]
pub struct CredentialsUpdate {
    /// An empty string removes the saved token.
    pub discogs_token: String,
}

impl Credentials {
    pub fn view(&self) -> CredentialsView {
        CredentialsView {
            discogs_token_set: !self.discogs_token.is_empty(),
        }
    }
    pub fn apply(mut self, update: CredentialsUpdate) -> Result<Self, String> {
        let token = update.discogs_token.trim().to_owned();
        if token.len() > 200 || token.chars().any(|c| !c.is_ascii_graphic()) {
            return Err("The Discogs token contains unexpected characters. Paste the personal access token from your Discogs developer settings.".into());
        }
        self.discogs_token = token;
        Ok(self)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn token_is_replaced_or_cleared_and_never_viewed() {
        let saved = Credentials {
            discogs_token: "abc".into(),
        };
        let replaced = saved
            .clone()
            .apply(CredentialsUpdate {
                discogs_token: " xyz ".into(),
            })
            .unwrap();
        assert_eq!(replaced.discogs_token, "xyz");
        let cleared = saved
            .clone()
            .apply(CredentialsUpdate {
                discogs_token: String::new(),
            })
            .unwrap();
        assert_eq!(
            cleared.view(),
            CredentialsView {
                discogs_token_set: false
            }
        );
        assert!(saved
            .clone()
            .apply(CredentialsUpdate {
                discogs_token: "a b".into()
            })
            .is_err());
        let json = serde_json::to_string(&saved.view()).unwrap();
        assert!(!json.contains("abc") && json.contains("true"));
    }

    #[test]
    fn credentials_round_trip_privately_and_tolerate_old_files() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nested").join("tag-sources.json");
        assert_eq!(load(&path).unwrap(), Credentials::default());
        let credentials = Credentials {
            discogs_token: "token".into(),
        };
        save(&path, &credentials).unwrap();
        assert_eq!(load(&path).unwrap(), credentials);
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
