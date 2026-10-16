//! Online tag sources (Mp3tag's Tag Sources menu): search a provider, review a
//! release and write the chosen fields and cover to the matched files.
//!
//! Providers only translate their API into the provider-agnostic
//! [`SourceRelease`]; matching, field selection and writing are shared, so
//! further sources plug in by adding a module that produces them, as
//! `freedb` does for gnudb and local freedb folders.
mod credentials;
mod discogs;
mod freedb;
mod freedb_index;
pub(crate) mod http;
mod model;
mod musicbrainz;
pub(crate) mod web;

use crate::evaluation::EvaluationSettings;
use base64::Engine;
use http::Channel;
pub use model::{Provider, SearchPage, SearchQuery, SourceRelease};
use serde::Deserialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::Duration;
use tag_core::batch_errors::{push_error, BatchError, Detailed, ErrorCause, IndexedDiagnostic};
use tag_core::{AudioEntry, Fields, ScanResult};

fn settings_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    Ok(crate::action_host::library_path(app)?.with_file_name("tag-sources.json"))
}
/// The keychain under this build's identifier; scripted end-to-end runs keep
/// the token in memory so they never reach a real keychain.
fn secret_store(app: &tauri::AppHandle) -> std::sync::Arc<dyn credentials::SecretStore> {
    use std::sync::{Arc, OnceLock};
    static E2E: OnceLock<Arc<credentials::MemoryStore>> = OnceLock::new();
    if crate::e2e::enabled() {
        return E2E.get_or_init(Default::default).clone();
    }
    #[cfg(not(any(target_os = "android", target_os = "ios")))]
    return Arc::new(credentials::KeychainStore {
        service: tauri::Manager::config(app).identifier.clone(),
    });
    #[cfg(any(target_os = "android", target_os = "ios"))]
    {
        let _ = app;
        Arc::new(credentials::NoKeychain)
    }
}

/// The Discogs token, or empty; the keychain is only asked for Discogs requests.
async fn discogs_token(app: &tauri::AppHandle, provider: Provider) -> Result<String, String> {
    if provider != Provider::Discogs {
        return Ok(String::new());
    }
    let path = settings_path(app)?;
    let store = secret_store(app);
    tauri::async_runtime::spawn_blocking(move || credentials::load(&path)?.discogs_token(&*store))
        .await
        .map_err(|e| e.to_string())?
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

fn freedb_service(settings: &freedb::FreedbSettings) -> http::Service {
    http::Service {
        name: freedb::server_name(settings),
        gate: &http::FREEDB,
        spacing: Duration::from_millis(500),
        token: None,
        retry_busy: false,
    }
}

fn service(provider: Provider, discogs_token: String) -> http::Service {
    match provider {
        Provider::Freedb | Provider::LocalFreedb | Provider::Web => {
            freedb_service(&freedb::settings_from(&crate::settings::current()))
        }
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
            let token = Some(discogs_token).filter(|t| !t.is_empty());
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

/// Web sources are run by `web_source_search` and `web_source_release`.
const WEB_COMMANDS: &str = "Web sources are searched by their script.";

#[tauri::command]
pub async fn tag_source_search(
    app: tauri::AppHandle,
    provider: Provider,
    query: SearchQuery,
    ticket: u64,
) -> Result<SearchPage, String> {
    http::claim(Channel::Search, ticket);
    if provider == Provider::Web {
        return Err(WEB_COMMANDS.into());
    }
    if matches!(provider, Provider::Freedb | Provider::LocalFreedb) {
        return freedb_search(&app, provider, query, ticket).await;
    }
    let token = discogs_token(&app, provider).await?;
    let url = match provider {
        Provider::Discogs => discogs::search_query(&query)?,
        _ => musicbrainz::search_query(&query)?,
    };
    let body = http::json(&service(provider, token), &url, Channel::Search, ticket).await?;
    Ok(match provider {
        Provider::Discogs => discogs::parse_search(&body),
        _ => musicbrainz::parse_search(&body),
    })
}

fn import_options(settings: &freedb::FreedbSettings) -> freedb::ImportOptions {
    freedb::ImportOptions {
        comment_disc_id: settings.comment_disc_id,
        disc_id_field: settings.disc_id_field,
    }
}

fn local_index_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    Ok(settings_path(app)?.with_file_name("local-freedb-index.tsv"))
}

/// Disc ID from the selected files (a `cddb query` with their TOC) or typed
/// in; local folders also search their text index.
async fn freedb_search(
    app: &tauri::AppHandle,
    provider: Provider,
    query: SearchQuery,
    ticket: u64,
) -> Result<SearchPage, String> {
    let settings = freedb::settings_from(&crate::settings::current());
    let gnudb = freedb::server_name(&settings) == "gnudb";
    let disc_id = match query.mode {
        model::SearchMode::Toc => freedb::Toc::from_lengths(&query.lengths)?.disc_id(),
        model::SearchMode::DiscId => freedb::disc_id(&query.text)?,
        model::SearchMode::Text if provider == Provider::LocalFreedb => {
            let index = local_index_path(app)?;
            let directory = PathBuf::from(&settings.local_directory);
            let found = tauri::async_runtime::spawn_blocking(move || {
                freedb::check_local_directory(&directory)?;
                freedb_index::search(&index, &directory, &query.text, query.page)
            })
            .await
            .map_err(|e| e.to_string())??;
            return Ok(SearchPage {
                results: found.results,
                total: Some(found.total),
            });
        }
        _ => return Err("freedb servers search by disc ID: computed from the selected files or typed in. Text search is available for a local freedb folder with a text index.".into()),
    };
    let results = if provider == Provider::LocalFreedb {
        let directory = PathBuf::from(&settings.local_directory);
        let options = import_options(&settings);
        tauri::async_runtime::spawn_blocking(move || {
            freedb::local_lookup(&directory, &disc_id)?
                .into_iter()
                .map(|(category, text)| {
                    freedb::release(
                        &freedb::parse_xmcd(&text),
                        provider,
                        &category,
                        &disc_id,
                        options,
                        false,
                    )
                    .map(|release| freedb::search_result(&release, &category))
                })
                .collect::<Result<Vec<_>, String>>()
        })
        .await
        .map_err(|e| e.to_string())??
    } else if query.mode == model::SearchMode::Toc {
        let toc = freedb::Toc::from_lengths(&query.lengths)?;
        let url = freedb::command_url(&settings, &toc.query_command())?;
        let body =
            http::freedb_text(&freedb_service(&settings), &url, Channel::Search, ticket).await?;
        freedb::parse_query(&body)?
            .iter()
            .map(|found| freedb::match_result(found, provider, gnudb))
            .collect()
    } else {
        // Without a TOC the server can only be asked category by category.
        let mut results = Vec::new();
        for category in freedb::CATEGORIES {
            let url = freedb::command_url(&settings, &format!("cddb read {category} {disc_id}"))?;
            let body = http::freedb_text(&freedb_service(&settings), &url, Channel::Search, ticket)
                .await?;
            if let Some(text) = freedb::parse_read(&body)? {
                let release = freedb::release(
                    &freedb::parse_xmcd(&text),
                    provider,
                    category,
                    &disc_id,
                    import_options(&settings),
                    gnudb,
                )?;
                results.push(freedb::search_result(&release, category));
            }
        }
        results
    };
    let total = Some(results.len() as u32);
    Ok(SearchPage { results, total })
}

async fn freedb_release(
    provider: Provider,
    input: &str,
    ticket: u64,
) -> Result<SourceRelease, String> {
    let settings = freedb::settings_from(&crate::settings::current());
    let (category, disc_id) = freedb::entry_id(input)?;
    let options = import_options(&settings);
    if provider == Provider::LocalFreedb {
        let directory = PathBuf::from(&settings.local_directory);
        return tauri::async_runtime::spawn_blocking(move || {
            let (category, text) = freedb::local_lookup(&directory, &disc_id)?
                .into_iter()
                .find(|(found, _)| *found == category)
                .ok_or("This disc is no longer in the local freedb folder.")?;
            freedb::release(
                &freedb::parse_xmcd(&text),
                provider,
                &category,
                &disc_id,
                options,
                false,
            )
        })
        .await
        .map_err(|e| e.to_string())?;
    }
    let url = freedb::command_url(&settings, &format!("cddb read {category} {disc_id}"))?;
    let body =
        http::freedb_text(&freedb_service(&settings), &url, Channel::Release, ticket).await?;
    let text = freedb::parse_read(&body)?
        .ok_or("The freedb server has no entry for this disc in this category.")?;
    freedb::release(
        &freedb::parse_xmcd(&text),
        provider,
        &category,
        &disc_id,
        options,
        freedb::server_name(&settings) == "gnudb",
    )
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
    let service = service(provider, discogs_token(&app, provider).await?);
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
        Provider::Freedb | Provider::LocalFreedb => freedb_release(provider, &input, ticket).await,
        Provider::Web => Err(WEB_COMMANDS.into()),
    }
}

/// Tag Sources "Preview": open the release page in the default browser.
#[tauri::command]
pub async fn tag_source_open_page(url: String) -> Result<(), String> {
    let url = http::release_page(&url)?;
    #[cfg(desktop)]
    return tauri_plugin_opener::open_url(url.as_str(), None::<&str>).map_err(|e| e.to_string());
    #[cfg(mobile)]
    return Err(format!("Open {url} in a browser"));
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

/// Options → Tag Sources → Local freedb: the text index and the folder it covers.
#[tauri::command]
pub async fn local_freedb_index_status(
    app: tauri::AppHandle,
) -> Result<freedb_index::IndexStatus, String> {
    let index = local_index_path(&app)?;
    tauri::async_runtime::spawn_blocking(move || freedb_index::status(&index))
        .await
        .map_err(|e| e.to_string())
}

/// Build the text index of a local freedb folder (as typed in Options). `job`
/// reports progress per file and can be cancelled with `cancel_job`.
#[tauri::command]
pub async fn build_local_freedb_index(
    app: tauri::AppHandle,
    directory: String,
    job: Option<String>,
) -> Result<freedb_index::IndexStatus, String> {
    let index = local_index_path(&app)?;
    let directory = PathBuf::from(directory.trim());
    tauri::async_runtime::spawn_blocking(move || {
        let job = crate::progress::Job::new(job);
        freedb_index::build(&directory, &index, &job)?;
        Ok(freedb_index::status(&index))
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub fn tag_source_cancel(ticket: u64) {
    http::cancel(ticket);
}

#[tauri::command]
pub async fn load_tag_source_settings(
    app: tauri::AppHandle,
) -> Result<credentials::CredentialsView, String> {
    Ok(credentials::load(&settings_path(&app)?)?.view(&*secret_store(&app)))
}

#[tauri::command]
pub async fn save_tag_source_settings(
    app: tauri::AppHandle,
    update: credentials::CredentialsUpdate,
) -> Result<credentials::CredentialsView, String> {
    let path = settings_path(&app)?;
    let store = secret_store(&app);
    tauri::async_runtime::spawn_blocking(move || credentials::update(&path, &*store, update))
        .await
        .map_err(|e| e.to_string())?
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
    /// Utils → Save image to disc: a file name format (Options → Tag Sources).
    #[serde(default)]
    pub save_as: Option<String>,
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
#[cfg(test)]
#[tauri::command]
pub async fn tag_source_apply(
    items: Vec<ApplyItem>,
    cover: Option<ApplyCover>,
    columns: Option<Vec<crate::evaluation::ColumnSpec>>,
) -> Result<ScanResult, String> {
    tag_source_apply_diagnostic(items, cover, columns)
        .await
        .map(|report| report.result)
}
pub async fn tag_source_apply_diagnostic(
    items: Vec<ApplyItem>,
    cover: Option<ApplyCover>,
    columns: Option<Vec<crate::evaluation::ColumnSpec>>,
) -> Result<Detailed<ScanResult>, String> {
    validate_items(&items, cover.is_some())?;
    let evaluation = crate::evaluation_settings(columns)?;
    let save_as = cover
        .as_ref()
        .and_then(|c| c.save_as.clone())
        .filter(|template| !template.trim().is_empty());
    if save_as.is_some() {
        let paths: Vec<String> = items.iter().map(|item| item.path.clone()).collect();
        crate::reject_provider_operation(&paths, "Saving the cover image to a file")?;
    }
    let image = match &cover {
        Some(cover) => Some(http::image(&cover.url, Channel::Image, u64::MAX).await?),
        None => None,
    };
    let options = ImportCover {
        replace: cover.as_ref().is_some_and(|c| c.replace_existing),
        preserve: cover.as_ref().is_some_and(|c| c.preserve_existing),
        save_as,
    };
    crate::io(move || write_import_diagnostic(&items, image.as_ref(), &options, &evaluation)).await
}

pub mod ipc {
    #[tauri::command(rename = "tag_source_apply")]
    pub async fn tag_source_apply_with_diagnostics(
        items: Vec<super::ApplyItem>,
        cover: Option<super::ApplyCover>,
        columns: Option<Vec<crate::evaluation::ColumnSpec>>,
    ) -> Result<tag_core::batch_errors::Detailed<tag_core::ScanResult>, String> {
        super::tag_source_apply_diagnostic(items, cover, columns).await
    }
}

struct ImportCover {
    replace: bool,
    preserve: bool,
    save_as: Option<String>,
}

/// Write the import as one undo step, including any image files saved beside the audio.
#[cfg(test)]
fn write_import(
    items: &[ApplyItem],
    image: Option<&http::Image>,
    options: &ImportCover,
    evaluation: &EvaluationSettings,
) -> Result<ScanResult, String> {
    write_import_diagnostic(items, image, options, evaluation).map(|report| report.result)
}
fn write_import_diagnostic(
    items: &[ApplyItem],
    image: Option<&http::Image>,
    options: &ImportCover,
    evaluation: &EvaluationSettings,
) -> Result<Detailed<ScanResult>, String> {
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
    crate::history::record_with_side_files_diagnostic(
        "Tag source import",
        None,
        &paths,
        |before_write| {
            let mut result = crate::batch(&paths, |path| {
                let fields = path
                    .to_str()
                    .and_then(|p| updates.get(p))
                    .copied()
                    .ok_or("File is not part of this import")?;
                crate::mutate_document(path, |working| {
                    let cover = cover_target(working, cover_path.as_deref(), options.preserve)?;
                    apply_one(working, fields, cover, options.replace)
                })
            });
            let mut diagnostics = Vec::new();
            if let (Some(template), Some(image)) = (&options.save_as, image) {
                let (errors, details) = save_cover_files_diagnostic(
                    &result.entries,
                    template,
                    image,
                    evaluation,
                    before_write,
                );
                let offset = result.errors.len();
                diagnostics.extend(details.into_iter().map(|mut item| {
                    item.index += offset;
                    item
                }));
                result.errors.extend(errors);
            }
            Ok(Detailed {
                result,
                error_diagnostics: diagnostics,
            })
        },
    )
}

/// The file name for a saved cover: the format evaluated on one written file,
/// reduced to a single safe file name (the extension comes from the image).
/// `counter` is the file's 1-based position among the written files.
#[cfg(test)]
fn cover_file_name(
    entry: &AudioEntry,
    template: &str,
    counter: usize,
    evaluation: &EvaluationSettings,
) -> Result<String, String> {
    cover_file_name_diagnostic(entry, template, counter, evaluation)
        .map_err(|cause| cause.to_string())
}
fn cover_file_name_diagnostic(
    entry: &AudioEntry,
    template: &str,
    counter: usize,
    evaluation: &EvaluationSettings,
) -> Result<String, ErrorCause> {
    // Cover names use the written tags, so pre-import column snapshots are stale.
    // Keep the operation's captured app folders, clock and separator settings.
    let technical = crate::evaluation::technical_fields(entry, counter);
    let name = evaluation.evaluate_filename_diagnostic(
        template,
        &entry.fields,
        &technical,
        &entry.path,
        false,
    )?;
    let name: String = name
        .chars()
        .map(|c| {
            if c.is_control() || r#"<>:"/\|?*"#.contains(c) {
                '_'
            } else {
                c
            }
        })
        .collect();
    let name = name.trim().trim_end_matches(['.', ' ']).to_owned();
    Ok(if name.is_empty() || name == ".." {
        "folder".into()
    } else {
        name
    })
}

/// Utils → Save image to disc: one image per folder of written files, named
/// from the first written file in it. An existing image of that name is replaced.
#[cfg(test)]
fn save_cover_files(
    entries: &[AudioEntry],
    template: &str,
    image: &http::Image,
    evaluation: &EvaluationSettings,
    before_write: &mut dyn FnMut(&Path) -> Result<(), String>,
) -> Vec<tag_core::FileError> {
    save_cover_files_diagnostic(entries, template, image, evaluation, &mut |path| {
        before_write(path).map_err(ErrorCause::external)
    })
    .0
}
fn save_cover_files_diagnostic(
    entries: &[AudioEntry],
    template: &str,
    image: &http::Image,
    evaluation: &EvaluationSettings,
    before_write: &mut dyn FnMut(&Path) -> Result<(), ErrorCause>,
) -> (Vec<tag_core::FileError>, Vec<IndexedDiagnostic>) {
    save_cover_files_with_writer(
        entries,
        template,
        image,
        evaluation,
        before_write,
        &mut |path, bytes| write_atomically_diagnostic(path, bytes),
    )
}
type CoverWriter<'a> = dyn FnMut(&Path, &[u8]) -> Result<(), ErrorCause> + 'a;
fn save_cover_files_with_writer(
    entries: &[AudioEntry],
    template: &str,
    image: &http::Image,
    evaluation: &EvaluationSettings,
    before_write: &mut dyn FnMut(&Path) -> Result<(), ErrorCause>,
    writer: &mut CoverWriter<'_>,
) -> (Vec<tag_core::FileError>, Vec<IndexedDiagnostic>) {
    let mut folders = std::collections::HashSet::new();
    let mut errors = Vec::new();
    let mut diagnostics = Vec::new();
    for (index, entry) in entries.iter().enumerate() {
        let Some(folder) = Path::new(&entry.path).parent() else {
            continue;
        };
        if !folders.insert(folder.to_path_buf()) {
            continue;
        }
        let saved =
            cover_file_name_diagnostic(entry, template, index + 1, evaluation).and_then(|name| {
                let target = folder.join(format!("{name}.{}", image.extension));
                before_write(&target)?;
                writer(&target, &image.bytes)
            });
        if let Err(error) = saved {
            push_error(
                &mut errors,
                &mut diagnostics,
                folder.to_string_lossy().into_owned(),
                BatchError::SourceCoverFailed { cause: error },
            );
        }
    }
    (errors, diagnostics)
}

fn write_atomically(target: &Path, bytes: &[u8]) -> Result<(), String> {
    write_atomically_diagnostic(target, bytes).map_err(|cause| cause.to_string())
}
fn write_atomically_diagnostic(target: &Path, bytes: &[u8]) -> Result<(), ErrorCause> {
    write_atomically_with_writer(target, bytes, |staging, bytes| {
        std::io::Write::write_all(staging, bytes)
    })
}
fn write_atomically_with_writer(
    target: &Path,
    bytes: &[u8],
    write: impl FnOnce(&mut tempfile::NamedTempFile, &[u8]) -> std::io::Result<()>,
) -> Result<(), ErrorCause> {
    let folder = target
        .parent()
        .filter(|folder| folder.is_dir())
        .ok_or_else(|| {
            ErrorCause::message(
                "The folder for {value} does not exist",
                [("value", target.to_string_lossy().into_owned())],
            )
        })?;
    let mut staging = tempfile::NamedTempFile::new_in(folder).map_err(ErrorCause::external)?;
    write(&mut staging, bytes).map_err(ErrorCause::external)?;
    staging
        .persist(target)
        .map_err(|e| ErrorCause::external(e.error))?;
    Ok(())
}

/// Utils → Extract cover: save the displayed artwork to a chosen file. The
/// extension is corrected to the image's real type.
#[tauri::command]
pub async fn tag_source_save_cover(url: String, path: String) -> Result<String, String> {
    if path.starts_with("content://") {
        return Err("Saving images to files is not available here.".into());
    }
    let image = http::image(&url, Channel::Image, u64::MAX).await?;
    tauri::async_runtime::spawn_blocking(move || {
        let target = image_target(PathBuf::from(path), image.extension);
        write_atomically(&target, &image.bytes)?;
        Ok(target.to_string_lossy().into_owned())
    })
    .await
    .map_err(|e| e.to_string())?
}

fn image_target(mut target: PathBuf, extension: &str) -> PathBuf {
    let current = target
        .extension()
        .map(|e| e.to_string_lossy().to_ascii_lowercase())
        .unwrap_or_default();
    let same = current == extension || (extension == "jpg" && current == "jpeg");
    if !same {
        target.set_extension(extension);
    }
    target
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
    fn diagnostic_cover_rows_distinguish_owned_guard_from_identical_io_without_writing() {
        let dir = tempfile::tempdir().unwrap();
        let folder = dunce::canonicalize(dir.path()).unwrap().join("Help; Title");
        std::fs::create_dir(&folder).unwrap();
        let audio = folder.join("Help.wav");
        wav(&audio);
        let entry = tag_core::read_entry_summary(&audio).unwrap();
        std::fs::remove_file(&audio).unwrap();
        std::fs::remove_dir(&folder).unwrap();
        let mut bytes = Vec::new();
        image::DynamicImage::new_rgb8(1, 1)
            .write_to(
                &mut std::io::Cursor::new(&mut bytes),
                image::ImageFormat::Png,
            )
            .unwrap();
        let image = http::Image::validate(bytes).unwrap();
        let evaluation = EvaluationSettings::default();
        let (owned, owned_details) = save_cover_files_diagnostic(
            std::slice::from_ref(&entry),
            "Title",
            &image,
            &evaluation,
            &mut |_| Ok(()),
        );
        assert_eq!(owned.len(), 1);
        let json = serde_json::to_value(&owned_details).unwrap();
        assert_eq!(json[0]["diagnostic"]["cause"]["kind"], "owned");
        assert_eq!(
            json[0]["diagnostic"]["cause"]["key"],
            "The folder for {value} does not exist"
        );
        let target = folder.join("Title.png");
        let same_english = format!("The folder for {} does not exist", target.display());
        assert_eq!(
            owned[0].error,
            format!("The cover image could not be saved: {same_english}")
        );
        std::fs::create_dir(&folder).unwrap();
        let (external, external_details) = save_cover_files_with_writer(
            &[entry],
            "Title",
            &image,
            &evaluation,
            &mut |_| Ok(()),
            &mut |path, data| {
                write_atomically_with_writer(path, data, |_, _| {
                    Err(std::io::Error::other(same_english.clone()))
                })
            },
        );
        let json = serde_json::to_value(&external_details).unwrap();
        assert_eq!(external.len(), 1);
        assert_eq!(external[0].error, owned[0].error);
        assert_eq!(json[0]["diagnostic"]["cause"]["kind"], "external");
        assert_eq!(json[0]["diagnostic"]["cause"]["value"], same_english);
        assert!(!target.exists());
        assert_eq!(std::fs::read_dir(folder).unwrap().count(), 0);
    }
    #[test]
    fn diagnostic_import_offsets_cover_frames_after_ordinary_errors_and_keeps_saved_tags() {
        let dir = tempfile::tempdir().unwrap();
        let root = dunce::canonicalize(dir.path()).unwrap();
        let audio = root.join("Help.wav");
        wav(&audio);
        let missing = root.join("missing.wav");
        let items = [
            ApplyItem {
                path: missing.to_string_lossy().into_owned(),
                updates: Fields::new(),
            },
            ApplyItem {
                path: audio.to_string_lossy().into_owned(),
                updates: Fields::from([("TITLE".into(), vec!["Help".into()])]),
            },
        ];
        let mut bytes = Vec::new();
        image::DynamicImage::new_rgb8(1, 1)
            .write_to(
                &mut std::io::Cursor::new(&mut bytes),
                image::ImageFormat::Png,
            )
            .unwrap();
        let image = http::Image::validate(bytes).unwrap();
        let options = ImportCover {
            replace: false,
            preserve: false,
            save_as: Some("$unknown()".into()),
        };
        let report = write_import_diagnostic(
            &items,
            Some(&image),
            &options,
            &EvaluationSettings::default(),
        )
        .unwrap();
        assert_eq!(report.result.entries.len(), 1);
        assert_eq!(report.result.errors.len(), 2);
        assert_eq!(report.error_diagnostics.len(), 1);
        assert_eq!(report.error_diagnostics[0].index, 1);
        assert_eq!(
            report.result.errors[1].error,
            report.error_diagnostics[0].diagnostic.to_string()
        );
        assert_eq!(
            serde_json::to_value(&report).unwrap()["error_diagnostics"][0]["diagnostic"]["kind"],
            "source_cover_failed"
        );
        assert_eq!(
            tag_core::read_entry_summary(&audio).unwrap().fields["TITLE"],
            ["Help"]
        );
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
                save_as: None,
            }),
            None,
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
            None,
        ))
        .unwrap();
        assert!(result.errors.is_empty(), "{:?}", result.errors);
        assert_eq!(tag_core::read_entry(&a).unwrap().fields["TITLE"], ["One"]);
        assert_eq!(tag_core::read_entry(&b).unwrap().fields["TITLE"], ["Two"]);
    }

    #[test]
    fn cover_images_are_saved_once_per_folder_with_a_formatted_name() {
        let dir = tempfile::tempdir().unwrap();
        let other = dir.path().join("cd2");
        std::fs::create_dir(&other).unwrap();
        let (a, b, c) = (
            dir.path().join("a.wav"),
            dir.path().join("b.wav"),
            other.join("c.wav"),
        );
        for (path, album) in [(&a, "One/Two"), (&b, "Ignored"), (&c, "Disc: 2")] {
            wav(path);
            let fields: Fields = [("ALBUM".to_string(), vec![album.to_string()])].into();
            apply_one(path, &fields, None, false).unwrap();
        }
        let entries: Vec<AudioEntry> = [&a, &b, &c]
            .iter()
            .map(|p| tag_core::read_entry(p).unwrap())
            .collect();
        let mut png = Vec::new();
        image::DynamicImage::new_rgb8(2, 2)
            .write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
            .unwrap();
        let image = http::Image::validate(png).unwrap();
        let evaluation = crate::evaluation_settings(None).unwrap();
        let errors = save_cover_files(&entries, "%album% cover", &image, &evaluation, &mut |_| {
            Ok(())
        });
        assert!(errors.is_empty(), "{errors:?}");
        assert!(dir.path().join("One_Two cover.png").is_file());
        assert!(!dir.path().join("Ignored cover.png").exists());
        assert!(other.join("Disc_ 2 cover.png").is_file());
        assert_eq!(
            cover_file_name(&entries[0], "", 1, &evaluation).unwrap(),
            "folder"
        );
        assert_eq!(
            cover_file_name(&entries[0], "..", 1, &evaluation).unwrap(),
            "folder"
        );
        assert_eq!(
            cover_file_name(&entries[1], "cover %_counter%", 2, &evaluation).unwrap(),
            "cover 2"
        );
        // Column scripting works as in every other format string.
        assert!(
            cover_file_name(&entries[2], "$columnValues(,' ')", 1, &evaluation)
                .unwrap()
                .contains("Disc_ 2")
        );
    }

    #[test]
    fn saved_cover_names_recompute_custom_columns_after_the_import() {
        let dir = tempfile::tempdir().unwrap();
        let audio = dir.path().join("a.wav");
        wav(&audio);
        let audio = dunce::canonicalize(audio).unwrap();
        let path = audio.to_string_lossy().into_owned();
        apply_one(
            &audio,
            &[("ALBUM".into(), vec!["Old Album".into()])].into(),
            None,
            false,
        )
        .unwrap();
        let mut png = Vec::new();
        image::DynamicImage::new_rgb8(2, 2)
            .write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
            .unwrap();
        let image = http::Image::validate(png.clone()).unwrap();
        let items = vec![ApplyItem {
            path: path.clone(),
            updates: [("ALBUM".into(), vec!["Fresh Album".into()])].into(),
        }];
        let evaluation = crate::evaluation_settings(Some(vec![crate::evaluation::ColumnSpec {
            name: "Imported column".into(),
            template: "$upper(%album%)".into(),
            values: [(path, "STALE SNAPSHOT".into())].into(),
        }]))
        .unwrap();
        let cover = |template: &str| ImportCover {
            replace: true,
            preserve: false,
            save_as: Some(template.into()),
        };
        let result = write_import(
            &items,
            Some(&image),
            &cover("$columnNames(,)-$columnValues(,)-%_counter%"),
            &evaluation,
        )
        .unwrap();
        assert!(result.errors.is_empty(), "{:?}", result.errors);
        assert_eq!(result.entries[0].fields["ALBUM"], ["Fresh Album"]);
        assert!(
            !dir.path()
                .join("Imported column-STALE SNAPSHOT-1.png")
                .exists(),
            "cover name must not use the pre-import column snapshot"
        );
        let fresh_cover = dir.path().join("Imported column-FRESH ALBUM-1.png");
        assert!(
            fresh_cover.is_file(),
            "cover name must use the imported album value"
        );
        assert_eq!(std::fs::read(fresh_cover).unwrap(), png);

        let default_evaluation = crate::evaluation_settings(None).unwrap();
        let result = write_import(
            &items,
            Some(&image),
            &cover("%album% cover"),
            &default_evaluation,
        )
        .unwrap();
        assert!(result.errors.is_empty(), "{:?}", result.errors);
        assert_eq!(
            std::fs::read(dir.path().join("Fresh Album cover.png")).unwrap(),
            png
        );
    }

    #[test]
    fn saved_cover_files_are_part_of_the_import_undo_step() {
        let _journal = crate::history::TEST_LOCK
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        let dir = tempfile::tempdir().unwrap();
        let root = dunce::canonicalize(dir.path()).unwrap();
        crate::history::TEST_ROOTS
            .lock()
            .unwrap()
            .push(root.clone());
        let audio = root.join("a.wav");
        wav(&audio);
        let replaced = root.join("Album.png");
        std::fs::write(&replaced, b"previous image").unwrap();
        let original = std::fs::read(&audio).unwrap();
        let mut png = Vec::new();
        image::DynamicImage::new_rgb8(2, 2)
            .write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
            .unwrap();
        let image = http::Image::validate(png.clone()).unwrap();
        let items = vec![ApplyItem {
            path: audio.to_string_lossy().into_owned(),
            updates: [("ALBUM".to_string(), vec!["Album".to_string()])].into(),
        }];
        let save = |template: &str| ImportCover {
            replace: true,
            preserve: false,
            save_as: Some(template.into()),
        };
        let run = |options: &ImportCover| {
            let result = write_import(
                &items,
                Some(&image),
                options,
                &crate::evaluation_settings(None).unwrap(),
            )
            .unwrap();
            assert!(result.errors.is_empty(), "{:?}", result.errors);
        };
        run(&save("%album%"));
        run(&save("folder"));
        let created = root.join("folder.png");
        assert_eq!(std::fs::read(&replaced).unwrap(), png);
        assert_eq!(std::fs::read(&created).unwrap(), png);
        use tauri::async_runtime::block_on;
        block_on(crate::history::undo_last()).unwrap();
        assert!(
            !created.exists(),
            "the second import's new image is removed"
        );
        assert_eq!(std::fs::read(&replaced).unwrap(), png);
        assert!(crate::history::history_state().unwrap().undo.is_some());
        block_on(crate::history::undo_last()).unwrap();
        assert_eq!(std::fs::read(&replaced).unwrap(), b"previous image");
        assert_eq!(std::fs::read(&audio).unwrap(), original);
        block_on(crate::history::redo_last()).unwrap();
        assert_eq!(std::fs::read(&replaced).unwrap(), png);
        assert_eq!(
            tag_core::read_entry(&audio).unwrap().fields["ALBUM"],
            ["Album"]
        );
        crate::history::TEST_ROOTS
            .lock()
            .unwrap()
            .retain(|r| r != &root);
    }

    #[test]
    fn extracted_covers_get_the_image_extension() {
        let path = |p: &str| PathBuf::from(p);
        assert_eq!(
            image_target(path("/x/front.jpeg"), "jpg"),
            path("/x/front.jpeg")
        );
        assert_eq!(
            image_target(path("/x/front.jpg"), "png"),
            path("/x/front.png")
        );
        assert_eq!(
            image_target(path("/x/front"), "webp"),
            path("/x/front.webp")
        );
        assert!(tauri::async_runtime::block_on(tag_source_save_cover(
            "https://example.com/a.jpg".into(),
            "/tmp/a.jpg".into()
        ))
        .unwrap_err()
        .contains("not from a supported tag source"));
    }

    /// gnudb answers only registered applications; this records what it says to OpenTag.
    #[test]
    #[ignore]
    fn live_gnudb_query() {
        let settings = freedb::FreedbSettings::default();
        let toc = freedb::Toc::from_lengths(&[200.0, 240.0, 178.0]).unwrap();
        let url = freedb::command_url(&settings, &toc.query_command()).unwrap();
        let body = tauri::async_runtime::block_on(http::freedb_text(
            &freedb_service(&settings),
            &url,
            Channel::Search,
            u64::MAX,
        ))
        .unwrap();
        match freedb::parse_query(&body) {
            Ok(matches) => println!("gnudb matches: {matches:?}"),
            Err(error) => assert!(error.contains("does not accept requests"), "{error}"),
        }
    }

    /// Manual check against the live services: `cargo test -p opentag live_ -- --ignored`.
    #[test]
    #[ignore]
    fn live_musicbrainz_and_discogs_lookups() {
        let run = |provider, input: &str, ticket| {
            let service = service(provider, String::new());
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
                    Provider::Freedb | Provider::LocalFreedb | Provider::Web => unreachable!(),
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
