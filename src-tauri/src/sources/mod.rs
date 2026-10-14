//! Online tag sources (Mp3tag's Tag Sources menu): search a provider, review a
//! release and write the chosen fields and cover to the matched files.
//!
//! Providers only translate their API into the provider-agnostic
//! [`SourceRelease`]; matching, field selection and writing are shared, so
//! further sources (gnudb/freedb, local CUE or freedb files) plug in by adding
//! a module with `search_query`/`parse_search`/`release_id`/`parse_release`.
mod credentials;
mod discogs;
pub(crate) mod http;
mod model;
mod musicbrainz;

use base64::Engine;
use http::Channel;
pub use model::{Provider, SearchQuery, SearchResult, SourceRelease};
use serde::Deserialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::Duration;
use tag_core::{AudioEntry, Fields, ScanResult};

fn settings_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    Ok(crate::action_host::library_path(app)?.with_file_name("tag-sources.json"))
}
fn current_credentials(app: &tauri::AppHandle) -> Result<credentials::Credentials, String> {
    credentials::load(&settings_path(app)?)
}

/// Cover Art Archive size from Options → Tag Sources (`pages.tag-sources`).
fn cover_size() -> musicbrainz::CoverSize {
    cover_size_from(&crate::settings::current())
}
fn cover_size_from(settings: &crate::settings::Settings) -> musicbrainz::CoverSize {
    settings
        .pages
        .get("tag-sources")
        .and_then(|page| page.get("musicbrainz_cover_size"))
        .and_then(|value| serde_json::from_value(value.clone()).ok())
        .unwrap_or_default()
}

fn service(provider: Provider, credentials: &credentials::Credentials) -> http::Service {
    match provider {
        // MusicBrainz allows one request per second per client.
        Provider::Musicbrainz => http::Service {
            name: "MusicBrainz",
            gate: &http::MUSICBRAINZ,
            spacing: Duration::from_millis(1100),
            token: None,
            retry_busy: true,
        },
        // Discogs allows 60 authenticated or 25 anonymous requests per minute.
        Provider::Discogs => {
            let token = Some(credentials.discogs_token.clone()).filter(|t| !t.is_empty());
            let spacing = Duration::from_millis(if token.is_some() { 1100 } else { 2500 });
            http::Service {
                name: "Discogs",
                gate: &http::DISCOGS,
                spacing,
                token,
                retry_busy: false,
            }
        }
    }
}

#[tauri::command]
pub async fn tag_source_search(
    app: tauri::AppHandle,
    provider: Provider,
    query: SearchQuery,
    ticket: u64,
) -> Result<Vec<SearchResult>, String> {
    http::claim(Channel::Search, ticket);
    let credentials = current_credentials(&app)?;
    let url = match provider {
        Provider::Musicbrainz => musicbrainz::search_query(&query)?,
        Provider::Discogs => discogs::search_query(&query)?,
    };
    let body = http::json(
        &service(provider, &credentials),
        &url,
        Channel::Search,
        ticket,
    )
    .await?;
    Ok(match provider {
        Provider::Musicbrainz => musicbrainz::parse_search(&body),
        Provider::Discogs => discogs::parse_search(&body),
    })
}

/// Look up a release by ID or address (search results pass their own ID).
#[tauri::command]
pub async fn tag_source_release(
    app: tauri::AppHandle,
    provider: Provider,
    input: String,
    ticket: u64,
) -> Result<SourceRelease, String> {
    http::claim(Channel::Release, ticket);
    let credentials = current_credentials(&app)?;
    let service = service(provider, &credentials);
    match provider {
        Provider::Musicbrainz => {
            let id = musicbrainz::release_id(&input)?;
            let body = http::json(
                &service,
                &musicbrainz::release_url(&id),
                Channel::Release,
                ticket,
            )
            .await?;
            musicbrainz::parse_release(&body, cover_size())
        }
        Provider::Discogs => {
            let id = discogs::release_id(&input)?;
            let body = http::json(
                &service,
                &discogs::release_url(&id),
                Channel::Release,
                ticket,
            )
            .await?;
            discogs::parse_release(&body)
        }
    }
}

/// Every Cover Art Archive image of a MusicBrainz release (Discogs lists its images with the release).
#[tauri::command]
pub async fn tag_source_covers(id: String, ticket: u64) -> Result<Vec<model::SourceCover>, String> {
    let id = musicbrainz::release_id(&id)?;
    let archive = http::Service {
        name: "Cover Art Archive",
        gate: &http::ARCHIVE,
        spacing: Duration::ZERO,
        token: None,
        retry_busy: false,
    };
    let listing = http::json(
        &archive,
        &musicbrainz::cover_listing_url(&id),
        Channel::Release,
        ticket,
    )
    .await?;
    Ok(musicbrainz::parse_covers(&listing, cover_size()))
}

/// Preview image as a data URL (the webview's CSP does not load remote images).
#[tauri::command]
pub async fn tag_source_image(url: String, ticket: u64) -> Result<String, String> {
    http::claim(Channel::Image, ticket);
    let image = http::image(&url, Channel::Image, ticket).await?;
    Ok(format!(
        "data:{};base64,{}",
        image.mime,
        base64::engine::general_purpose::STANDARD.encode(&image.bytes)
    ))
}

#[tauri::command]
pub fn tag_source_cancel(ticket: u64) {
    http::cancel(ticket);
}

#[tauri::command]
pub async fn load_tag_source_settings(
    app: tauri::AppHandle,
) -> Result<credentials::CredentialsView, String> {
    Ok(current_credentials(&app)?.view())
}

#[tauri::command]
pub async fn save_tag_source_settings(
    app: tauri::AppHandle,
    update: credentials::CredentialsUpdate,
) -> Result<credentials::CredentialsView, String> {
    let path = settings_path(&app)?;
    let next = credentials::load(&path)?.apply(update)?;
    credentials::save(&path, &next)?;
    Ok(next.view())
}

#[derive(Debug, Deserialize)]
pub struct ApplyItem {
    pub path: String,
    pub updates: Fields,
}
#[derive(Debug, Deserialize)]
pub struct ApplyCover {
    pub url: String,
    pub replace_existing: bool,
    /// Files that already have artwork keep it (Options → Messages → at
    /// importing covers).
    #[serde(default)]
    pub preserve_existing: bool,
}

fn validate_items(items: &[ApplyItem], cover: bool) -> Result<(), String> {
    if items.is_empty() {
        return Err("No files are matched to tracks of this release.".into());
    }
    let mut seen = std::collections::HashSet::new();
    for item in items {
        if !seen.insert(item.path.as_str()) {
            return Err("A file is matched to more than one track.".into());
        }
        if item.updates.is_empty() && !cover {
            return Err("Choose at least one field or the cover to write.".into());
        }
        if item.updates.keys().any(|key| key.trim().is_empty()) {
            return Err("Field names cannot be empty.".into());
        }
    }
    Ok(())
}

/// The cover to write to `path`: none when existing artwork is preserved.
fn cover_target<'a>(
    path: &Path,
    cover: Option<&'a Path>,
    preserve: bool,
) -> Result<Option<&'a Path>, String> {
    if preserve && cover.is_some() && !crate::settings::read_summary(path)?.covers.is_empty() {
        return Ok(None);
    }
    Ok(cover)
}

/// Writes honour the user's tag write/remove preferences (MPEG/APE, covers).
fn apply_one(
    path: &Path,
    updates: &Fields,
    cover: Option<&Path>,
    replace: bool,
) -> Result<AudioEntry, String> {
    let options = crate::settings::tag_options();
    let mut entry = if updates.is_empty() {
        crate::settings::read_summary(path)?
    } else {
        crate::settings::write_summary(path, updates)?
    };
    if let Some(image) = cover {
        if replace {
            tag_core::remove_all_covers_with_options(path, &options)?;
        }
        entry = tag_core::add_cover_with_options(path, image, false, &options)?;
    }
    Ok(entry)
}

/// Write reviewed values immediately, like Mp3tag and OpenTag's other importers.
/// The cover is downloaded and validated once before any file is touched.
#[tauri::command]
pub async fn tag_source_apply(
    items: Vec<ApplyItem>,
    cover: Option<ApplyCover>,
) -> Result<ScanResult, String> {
    validate_items(&items, cover.is_some())?;
    let image = match &cover {
        Some(cover) => Some(http::image(&cover.url, Channel::Image, u64::MAX).await?),
        None => None,
    };
    let replace = cover.as_ref().is_some_and(|c| c.replace_existing);
    let preserve = cover.as_ref().is_some_and(|c| c.preserve_existing);
    crate::io(move || {
        let staging = tempfile::tempdir().map_err(|e| e.to_string())?;
        let cover_path = match image {
            Some(image) => {
                let path = staging.path().join(format!("cover.{}", image.extension));
                std::fs::write(&path, &image.bytes).map_err(|e| e.to_string())?;
                Some(path)
            }
            None => None,
        };
        let updates: HashMap<&str, &Fields> = items
            .iter()
            .map(|item| (item.path.as_str(), &item.updates))
            .collect();
        let paths: Vec<String> = items.iter().map(|item| item.path.clone()).collect();
        Ok(crate::batch(&paths, |path| {
            let fields = path
                .to_str()
                .and_then(|p| updates.get(p))
                .copied()
                .ok_or("File is not part of this import")?;
            crate::mutate_document(path, |working| {
                let cover = cover_target(working, cover_path.as_deref(), preserve)?;
                apply_one(working, fields, cover, replace)
            })
        }))
    })
    .await
}

#[cfg(test)]
mod tests {
    use super::*;

    fn wav(path: &Path) {
        let samples = vec![0u8; 8820];
        let mut data = b"RIFF".to_vec();
        data.extend((36 + samples.len() as u32).to_le_bytes());
        data.extend(b"WAVEfmt ");
        data.extend(16u32.to_le_bytes());
        data.extend(1u16.to_le_bytes());
        data.extend(1u16.to_le_bytes());
        data.extend(44100u32.to_le_bytes());
        data.extend(88200u32.to_le_bytes());
        data.extend(2u16.to_le_bytes());
        data.extend(16u16.to_le_bytes());
        data.extend(b"data");
        data.extend((samples.len() as u32).to_le_bytes());
        data.extend(samples);
        std::fs::write(path, data).unwrap();
    }
    fn png(dir: &Path, name: &str, width: u32) -> PathBuf {
        let path = dir.join(name);
        image::DynamicImage::new_rgb8(width, 1)
            .save_with_format(&path, image::ImageFormat::Png)
            .unwrap();
        path
    }

    #[test]
    fn apply_writes_fields_and_replaces_or_appends_covers() {
        let dir = tempfile::tempdir().unwrap();
        let audio = dir.path().join("a.wav");
        wav(&audio);
        tag_core::add_cover_summary(&audio, &png(dir.path(), "old.png", 1)).unwrap();
        let updates: Fields = [
            ("ALBUM".to_string(), vec!["Discovery".to_string()]),
            ("MUSICBRAINZ_ALBUMID".to_string(), vec!["id".to_string()]),
        ]
        .into();
        let entry = apply_one(
            &audio,
            &updates,
            Some(&png(dir.path(), "cover.png", 2)),
            true,
        )
        .unwrap();
        assert_eq!(entry.covers.len(), 1, "existing artwork was replaced");
        let saved = tag_core::read_entry(&audio).unwrap();
        assert_eq!(saved.fields["ALBUM"], ["Discovery"]);
        assert_eq!(saved.fields["MUSICBRAINZ_ALBUMID"], ["id"]);
        let appended = apply_one(
            &audio,
            &Fields::new(),
            Some(&png(dir.path(), "back.png", 3)),
            false,
        );
        // Formats that cannot hold two untitled front covers report it per file.
        if let Ok(entry) = appended {
            assert_eq!(entry.covers.len(), 2);
        }
        let unchanged = apply_one(&audio, &Fields::new(), None, false).unwrap();
        assert_eq!(unchanged.fields["ALBUM"], ["Discovery"]);
    }

    #[test]
    fn preserved_artwork_is_not_replaced() {
        let dir = tempfile::tempdir().unwrap();
        let (with, without) = (dir.path().join("a.wav"), dir.path().join("b.wav"));
        wav(&with);
        wav(&without);
        tag_core::add_cover_summary(&with, &png(dir.path(), "old.png", 1)).unwrap();
        let cover = png(dir.path(), "new.png", 2);
        assert_eq!(cover_target(&with, Some(&cover), true).unwrap(), None);
        assert_eq!(
            cover_target(&without, Some(&cover), true).unwrap(),
            Some(cover.as_path())
        );
        assert_eq!(
            cover_target(&with, Some(&cover), false).unwrap(),
            Some(cover.as_path())
        );
    }

    #[test]
    fn cover_size_comes_from_the_options_page() {
        let mut settings = crate::settings::Settings::default();
        assert_eq!(cover_size_from(&settings), musicbrainz::CoverSize::Medium);
        settings.pages.insert(
            "tag-sources".into(),
            serde_json::json!({"musicbrainz_cover_size": "1200"}),
        );
        assert_eq!(cover_size_from(&settings), musicbrainz::CoverSize::Large);
        settings.pages.insert(
            "tag-sources".into(),
            serde_json::json!({"musicbrainz_cover_size": "huge"}),
        );
        assert_eq!(cover_size_from(&settings), musicbrainz::CoverSize::Medium);
    }

    #[test]
    fn apply_items_must_be_unique_and_write_something() {
        let item = |path: &str, updates: Fields| ApplyItem {
            path: path.into(),
            updates,
        };
        let some: Fields = [("TITLE".to_string(), vec!["x".to_string()])].into();
        assert!(validate_items(&[], false).is_err());
        assert!(
            validate_items(&[item("a", some.clone()), item("a", some.clone())], false).is_err()
        );
        assert!(validate_items(&[item("a", Fields::new())], false).is_err());
        assert!(validate_items(&[item("a", Fields::new())], true).is_ok());
        assert!(validate_items(&[item("a", [(" ".to_string(), vec![])].into())], false).is_err());
        assert!(validate_items(&[item("a", some)], false).is_ok());
    }

    #[test]
    fn apply_rejects_untrusted_cover_addresses_before_writing() {
        let dir = tempfile::tempdir().unwrap();
        let audio = dir.path().join("a.wav");
        wav(&audio);
        let items = vec![ApplyItem {
            path: audio.to_string_lossy().into(),
            updates: [("TITLE".to_string(), vec!["New".to_string()])].into(),
        }];
        let error = tauri::async_runtime::block_on(tag_source_apply(
            items,
            Some(ApplyCover {
                url: "https://example.com/a.jpg".into(),
                replace_existing: true,
                preserve_existing: false,
            }),
        ))
        .unwrap_err();
        assert!(error.contains("not from a supported tag source"));
        assert!(!tag_core::read_entry(&audio)
            .unwrap()
            .fields
            .contains_key("TITLE"));
    }

    #[test]
    fn apply_without_cover_writes_every_matched_file() {
        let dir = tempfile::tempdir().unwrap();
        let (a, b) = (dir.path().join("a.wav"), dir.path().join("b.wav"));
        wav(&a);
        wav(&b);
        let item = |path: &Path, title: &str| ApplyItem {
            path: path.to_string_lossy().into(),
            updates: [("TITLE".to_string(), vec![title.to_string()])].into(),
        };
        let result = tauri::async_runtime::block_on(tag_source_apply(
            vec![item(&a, "One"), item(&b, "Two")],
            None,
        ))
        .unwrap();
        assert!(result.errors.is_empty(), "{:?}", result.errors);
        assert_eq!(tag_core::read_entry(&a).unwrap().fields["TITLE"], ["One"]);
        assert_eq!(tag_core::read_entry(&b).unwrap().fields["TITLE"], ["Two"]);
    }

    /// Manual check against the live services: `cargo test -p opentag live_ -- --ignored`.
    #[test]
    #[ignore]
    fn live_musicbrainz_and_discogs_lookups() {
        let run = |provider, input: &str, ticket| {
            let service = service(provider, &credentials::Credentials::default());
            tauri::async_runtime::block_on(async {
                match provider {
                    Provider::Musicbrainz => musicbrainz::parse_release(
                        &http::json(
                            &service,
                            &musicbrainz::release_url(&musicbrainz::release_id(input)?),
                            Channel::Release,
                            ticket,
                        )
                        .await?,
                        musicbrainz::CoverSize::Medium,
                    ),
                    Provider::Discogs => discogs::parse_release(
                        &http::json(
                            &service,
                            &discogs::release_url(&discogs::release_id(input)?),
                            Channel::Release,
                            ticket,
                        )
                        .await?,
                    ),
                }
            })
        };
        let mb = run(
            Provider::Musicbrainz,
            "d073287b-d1bd-4f11-a933-a4386f8cf701",
            u64::MAX,
        )
        .unwrap();
        assert_eq!(mb.fields["ALBUM"], ["Discovery"]);
        let discogs = run(Provider::Discogs, "1225", u64::MAX).unwrap();
        assert_eq!(discogs.fields["DISCOGS_RELEASE_ID"], ["1225"]);
        let archive = http::Service {
            name: "Cover Art Archive",
            gate: &http::ARCHIVE,
            spacing: Duration::ZERO,
            token: None,
            retry_busy: false,
        };
        let listing = tauri::async_runtime::block_on(http::json(
            &archive,
            &musicbrainz::cover_listing_url(&mb.id),
            Channel::Release,
            u64::MAX,
        ))
        .unwrap();
        let covers = musicbrainz::parse_covers(&listing, musicbrainz::CoverSize::Small);
        let image = tauri::async_runtime::block_on(http::image(
            covers[0].thumb.as_deref().unwrap(),
            Channel::Image,
            u64::MAX,
        ))
        .unwrap();
        assert_eq!(image.mime, "image/jpeg");
    }
}
