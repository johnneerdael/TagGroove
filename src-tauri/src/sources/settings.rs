//! Tag source options in `tag-sources.json` beside the other configuration files.
//! The Discogs token never travels back to the webview; it only reports whether one is set.
use super::musicbrainz::CoverSize;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub discogs_token: String,
    pub musicbrainz_cover_size: CoverSize,
}

#[derive(Debug, PartialEq, Serialize)]
pub struct SettingsView {
    pub discogs_token_set: bool,
    pub musicbrainz_cover_size: CoverSize,
}

#[derive(Debug, Deserialize)]
pub struct SettingsUpdate {
    /// `None` keeps the saved token, an empty string removes it.
    pub discogs_token: Option<String>,
    pub musicbrainz_cover_size: CoverSize,
}

impl Settings {
    pub fn view(&self) -> SettingsView {
        SettingsView {
            discogs_token_set: !self.discogs_token.is_empty(),
            musicbrainz_cover_size: self.musicbrainz_cover_size,
        }
    }
    pub fn apply(mut self, update: SettingsUpdate) -> Result<Self, String> {
        if let Some(token) = update.discogs_token {
            let token = token.trim().to_owned();
            if token.len() > 200 || token.chars().any(|c| !c.is_ascii_graphic()) {
                return Err("The Discogs token contains unexpected characters. Paste the personal access token from your Discogs developer settings.".into());
            }
            self.discogs_token = token;
        }
        self.musicbrainz_cover_size = update.musicbrainz_cover_size;
        Ok(self)
    }
}

pub fn load(path: &Path) -> Result<Settings, String> {
    match std::fs::read(path) {
        Ok(bytes) => serde_json::from_slice(&bytes)
            .map_err(|e| format!("Cannot read tag source options: {e}")),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Settings::default()),
        Err(e) => Err(e.to_string()),
    }
}

/// Written atomically through a private (0600 on Unix) temporary file.
pub fn save(path: &Path, settings: &Settings) -> Result<(), String> {
    let bytes = serde_json::to_vec_pretty(settings).map_err(|e| e.to_string())?;
    crate::action_host::atomic_output(path, &bytes, false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn token_is_kept_replaced_or_cleared_and_never_viewed() {
        let saved = Settings {
            discogs_token: "abc".into(),
            musicbrainz_cover_size: CoverSize::Medium,
        };
        let kept = saved
            .clone()
            .apply(SettingsUpdate {
                discogs_token: None,
                musicbrainz_cover_size: CoverSize::Large,
            })
            .unwrap();
        assert_eq!(
            (kept.discogs_token.as_str(), kept.musicbrainz_cover_size),
            ("abc", CoverSize::Large)
        );
        let replaced = saved
            .clone()
            .apply(SettingsUpdate {
                discogs_token: Some(" xyz ".into()),
                musicbrainz_cover_size: CoverSize::Medium,
            })
            .unwrap();
        assert_eq!(replaced.discogs_token, "xyz");
        let cleared = saved
            .clone()
            .apply(SettingsUpdate {
                discogs_token: Some(String::new()),
                musicbrainz_cover_size: CoverSize::Medium,
            })
            .unwrap();
        assert_eq!(
            cleared.view(),
            SettingsView {
                discogs_token_set: false,
                musicbrainz_cover_size: CoverSize::Medium
            }
        );
        assert!(saved
            .apply(SettingsUpdate {
                discogs_token: Some("a b".into()),
                musicbrainz_cover_size: CoverSize::Medium
            })
            .is_err());
        let json = serde_json::to_string(&kept.view()).unwrap();
        assert!(!json.contains("abc") && json.contains("\"1200\""));
    }

    #[test]
    fn settings_round_trip_and_tolerate_missing_files() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nested").join("tag-sources.json");
        assert_eq!(load(&path).unwrap(), Settings::default());
        let settings = Settings {
            discogs_token: "token".into(),
            musicbrainz_cover_size: CoverSize::Original,
        };
        save(&path, &settings).unwrap();
        assert_eq!(load(&path).unwrap(), settings);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                std::fs::metadata(&path).unwrap().permissions().mode() & 0o077,
                0,
                "token file is private"
            );
        }
        std::fs::write(&path, "{\"musicbrainz_cover_size\":\"250\"}").unwrap();
        assert_eq!(
            load(&path).unwrap().musicbrainz_cover_size,
            CoverSize::Small
        );
    }
}
