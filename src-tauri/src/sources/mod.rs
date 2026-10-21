//! Online tag sources (Mp3tag's Tag Sources menu): search a provider, review a
//! release and write the chosen fields and cover to the matched files.
//!
//! Providers only translate their API into the provider-agnostic
//! [`SourceRelease`]; matching, field selection and writing are shared, so
//! further sources plug in by adding a module that produces them, as
//! `freedb` does for gnudb and local freedb folders.
mod acoustid;
mod catalog;
mod credentials;
mod deezer;
mod discogs;
mod fanart;
pub(crate) mod freedb;
mod freedb_index;
pub(crate) mod http;
mod itunes;
mod keys;
mod lastfm;
mod lrclib;
mod model;
mod musicbrainz;
#[cfg(test)]
mod track_search_tests;
pub(crate) mod web;

use crate::evaluation::EvaluationSettings;
use base64::Engine;
use http::Channel;
pub use model::{Provider, SearchPage, SearchQuery, SourceRelease};
use serde::Deserialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
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

/// The private credentials file (user API keys); never the keychain.
async fn saved_credentials(app: &tauri::AppHandle) -> Result<credentials::Credentials, String> {
    let path = settings_path(app)?;
    tauri::async_runtime::spawn_blocking(move || credentials::load(&path))
        .await
        .map_err(|e| e.to_string())?
}

/// The user's key, else the build's; without either the source cannot run.
fn required_key(name: &str, user: &str, built_in: Option<&'static str>) -> Result<String, String> {
    keys::choose(user, built_in)
        .ok_or_else(|| format!("{name} needs an API key. Add one in Tag Sources → API keys…."))
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

/// Options → Tag Sources → Online services (`pages.tag-sources-online`).
#[derive(Debug, Default, PartialEq)]
struct Online {
    itunes_country: String,
    itunes_cover: itunes::CoverSize,
    deezer_cover: deezer::CoverSize,
    lrclib_synced: bool,
    lastfm_genres: usize,
    fpcalc: String,
}
fn online() -> Online {
    online_from(&crate::settings::current())
}
fn online_from(settings: &crate::settings::Settings) -> Online {
    let page = settings.pages.get("tag-sources-online");
    let value = |key: &str| page.and_then(|p| p.get(key)).cloned();
    let text = |key: &str| {
        value(key)
            .and_then(|v| v.as_str().map(str::to_owned))
            .unwrap_or_default()
    };
    Online {
        itunes_country: itunes::country(&text("itunes_country")),
        itunes_cover: value("itunes_cover_size")
            .and_then(|v| serde_json::from_value(v).ok())
            .unwrap_or_default(),
        deezer_cover: value("deezer_cover_size")
            .and_then(|v| serde_json::from_value(v).ok())
            .unwrap_or_default(),
        lrclib_synced: value("lrclib_synced")
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
        lastfm_genres: value("lastfm_genres")
            .and_then(|v| v.as_u64().or_else(|| v.as_str()?.parse().ok()))
            .map_or(1, |n| n.clamp(1, 10) as usize),
        fpcalc: value("fpcalc")
            .or_else(|| value("fpcalc_path"))
            .and_then(|v| v.as_str().map(str::trim).map(str::to_owned))
            .unwrap_or_default(),
    }
}

fn freedb_service(settings: &freedb::FreedbSettings) -> http::Service {
    http::Service::new(freedb::server_name(settings), &http::FREEDB, 500)
}

/// Each service's own gate and spacing, from its published limits.
fn service(provider: Provider, discogs_token: String) -> http::Service {
    match provider {
        Provider::Freedb
        | Provider::LocalFreedb
        | Provider::Web
        | Provider::Beatport
        | Provider::Traxsource => {
            freedb_service(&freedb::settings_from(&crate::settings::current()))
        }
        // MusicBrainz allows one request per second per client. AcoustID
        // releases and fanart.tv's album search come from MusicBrainz.
        Provider::Musicbrainz | Provider::Acoustid => http::Service {
            retry_busy: true,
            ..http::Service::new("MusicBrainz", &http::MUSICBRAINZ, 1100)
        },
        // Use the application's 25/minute budget unless a user token is present.
        Provider::Discogs => {
            let auth = keys::discogs_auth(&discogs_token, keys::discogs_app());
            let spacing = keys::discogs_spacing(&auth);
            http::Service {
                auth,
                ..http::Service::new("Discogs", &http::DISCOGS, spacing)
            }
        }
        // Apple: "approximately 20 calls per minute".
        Provider::Itunes => http::Service::new("iTunes", &http::ITUNES, 3100),
        // Deezer: 50 requests per 5 seconds.
        Provider::Deezer => http::Service::new("Deezer", &http::DEEZER, 120),
        // LRCLIB publishes no limit; stay gentle and retry once when busy.
        Provider::Lrclib => http::Service {
            retry_busy: true,
            ..http::Service::new("LRCLIB", &http::LRCLIB, 200)
        },
        Provider::Lastfm => http::Service {
            auth: http::Auth::Key,
            ..http::Service::new("Last.fm", &http::LASTFM, 250)
        },
        Provider::Fanart => http::Service::new("MusicBrainz", &http::MUSICBRAINZ, 1100),
    }
}

/// AcoustID: "Do not make more than 3 requests per second."
fn acoustid_service() -> http::Service {
    http::Service {
        auth: http::Auth::Key,
        ..http::Service::new("AcoustID", &http::ACOUSTID, 350)
    }
}

/// fanart.tv: the project key, and the user's key as the personal client key
/// (or as the project key when the build has none).
fn fanart_service(user: &str) -> Result<http::Service, String> {
    let user = user.trim().to_owned();
    let headers = match keys::fanart() {
        Some(project) => vec![("api-key", project.to_owned()), ("client-key", user)],
        None if !user.is_empty() => vec![("api-key", user)],
        None => {
            return Err("fanart.tv needs an API key. Add one in Tag Sources → API keys….".into())
        }
    };
    Ok(http::Service {
        auth: http::Auth::Headers(headers),
        ..http::Service::new("fanart.tv", &http::FANART, 250)
    })
}

/// Web sources are run by `web_source_search` and `web_source_release`.
const WEB_COMMANDS: &str = "Web sources are searched by their script.";

fn parse_search_response(
    provider: Provider,
    mode: model::SearchMode,
    body: &serde_json::Value,
) -> Result<SearchPage, String> {
    use model::SearchMode;
    match (provider, mode) {
        (Provider::Musicbrainz, SearchMode::ArtistTitle) => musicbrainz::parse_track_search(body),
        (Provider::Musicbrainz, _) => Ok(musicbrainz::parse_search(body)),
        (Provider::Discogs, _) => Ok(discogs::parse_search(body)),
        (Provider::Itunes, SearchMode::ArtistTitle) => itunes::parse_track_search(body),
        (Provider::Itunes, _) => Ok(itunes::parse_search(body)),
        (Provider::Deezer, SearchMode::ArtistTitle) => deezer::parse_track_search(body),
        (Provider::Deezer, _) => deezer::parse_search(body),
        (Provider::Lastfm, SearchMode::ArtistTitle) => lastfm::parse_track_search(body),
        (Provider::Lastfm, _) => lastfm::parse_search(body),
        _ => Err("This source uses its own search response parser.".into()),
    }
}

#[tauri::command]
pub fn tag_source_catalog_reset() -> Result<(), String> {
    catalog::reset()
}

#[tauri::command]
pub async fn tag_source_search(
    app: tauri::AppHandle,
    provider: Provider,
    query: SearchQuery,
    ticket: u64,
) -> Result<SearchPage, String> {
    http::claim(Channel::Search, ticket);
    match provider {
        Provider::Web => Err(WEB_COMMANDS.into()),
        Provider::Beatport | Provider::Traxsource => {
            catalog::search(provider, &query, ticket).await
        }
        Provider::Freedb | Provider::LocalFreedb => {
            freedb_search(&app, provider, query, ticket).await
        }
        Provider::Musicbrainz | Provider::Discogs => {
            let token = discogs_token(&app, provider).await?;
            let url = match provider {
                Provider::Discogs => discogs::search_query(&query)?,
                _ => musicbrainz::search_query(&query)?,
            };
            let body = http::json(&service(provider, token), &url, Channel::Search, ticket).await?;
            parse_search_response(provider, query.mode, &body)
        }
        Provider::Itunes => {
            let url = itunes::search_query(&query, &online().itunes_country)?;
            let body = http::json(
                &service(provider, String::new()),
                &url,
                Channel::Search,
                ticket,
            )
            .await?;
            parse_search_response(provider, query.mode, &body)
        }
        Provider::Deezer => {
            let service = service(provider, String::new());
            if query.mode == model::SearchMode::Barcode {
                let url = deezer::barcode_url(&query.text)?;
                let body = http::json(&service, &url, Channel::Search, ticket).await?;
                return deezer::parse_album_result(&body);
            }
            let url = deezer::search_query(&query)?;
            parse_search_response(
                provider,
                query.mode,
                &http::json(&service, &url, Channel::Search, ticket).await?,
            )
        }
        Provider::Lrclib => lrclib_search(query, ticket).await,
        Provider::Acoustid => acoustid_search(&app, query, ticket).await,
        Provider::Lastfm => {
            let key = required_key(
                "Last.fm",
                &saved_credentials(&app).await?.lastfm_key,
                keys::lastfm(),
            )?;
            let service = service(provider, String::new());
            if query.mode == model::SearchMode::ArtistAlbum {
                let id = lastfm::name_id(&query.artist, &query.album)?;
                let url = lastfm::info_url(&id, &key)?;
                return match http::json_optional(&service, &url, Channel::Search, ticket).await? {
                    Some(body) => lastfm::parse_info_result(&body, &id),
                    None => Ok(SearchPage {
                        results: Vec::new(),
                        total: Some(0),
                    }),
                };
            }
            let url = lastfm::search_url(&query, &key)?;
            parse_search_response(
                provider,
                query.mode,
                &http::json(&service, &url, Channel::Search, ticket).await?,
            )
        }
        Provider::Fanart => {
            // Albums are found on MusicBrainz; the key is only needed for artwork.
            required_key_for_fanart(&app).await?;
            let url = fanart::search_query(&query)?;
            let body = http::json(
                &service(provider, String::new()),
                &url,
                Channel::Search,
                ticket,
            )
            .await?;
            Ok(fanart::parse_search(&body))
        }
    }
}

async fn required_key_for_fanart(app: &tauri::AppHandle) -> Result<http::Service, String> {
    fanart_service(&saved_credentials(app).await?.fanart_key)
}

/// LRCLIB: each selected file's lyrics (one result for all of them), or a
/// free-text search listing lyrics records.
async fn lrclib_search(query: SearchQuery, ticket: u64) -> Result<SearchPage, String> {
    let service = service(Provider::Lrclib, String::new());
    match query.mode {
        model::SearchMode::Text => {
            let url = lrclib::text_search_url(&query.text)?;
            Ok(lrclib::parse_search(
                &http::json(&service, &url, Channel::Search, ticket).await?,
            ))
        }
        model::SearchMode::Files => {
            if query.files.is_empty() {
                return Err("Select the files to look up.".into());
            }
            let mut found = lrclib::FilesFound::default();
            let mut first_error = None;
            for file in &query.files {
                match lrclib_lookup(&service, file, ticket).await {
                    Ok(Some(record)) => {
                        lrclib::remember(&record);
                        if record
                            .get("syncedLyrics")
                            .and_then(serde_json::Value::as_str)
                            .is_some_and(|s| !s.trim().is_empty())
                        {
                            found.synced += 1;
                        }
                        found.ids.push(
                            record
                                .get("id")
                                .and_then(serde_json::Value::as_u64)
                                .map(|id| id.to_string()),
                        );
                    }
                    Ok(None) => found.ids.push(None),
                    Err(error) if error == http::CANCELLED => return Err(error),
                    Err(error) => {
                        found.failed += 1;
                        first_error.get_or_insert(error);
                        found.ids.push(None);
                    }
                }
            }
            if found.failed == query.files.len() {
                return Err(first_error.unwrap_or_default());
            }
            Ok(SearchPage {
                results: vec![lrclib::files_result(&found)],
                total: Some(1),
            })
        }
        _ => Err("LRCLIB looks up the selected files, search words or a lyrics record ID.".into()),
    }
}

/// The exact signature first, then the closest search hit.
async fn lrclib_lookup(
    service: &http::Service,
    file: &model::FileSignature,
    ticket: u64,
) -> Result<Option<serde_json::Value>, String> {
    let has_lyrics = |record: &serde_json::Value| {
        ["plainLyrics", "syncedLyrics"].iter().any(|key| {
            record
                .get(key)
                .and_then(serde_json::Value::as_str)
                .is_some_and(|s| !s.trim().is_empty())
        })
    };
    if let Some(url) = lrclib::get_url(file) {
        if let Some(record) = http::json_optional(service, &url, Channel::Search, ticket).await? {
            if has_lyrics(&record) {
                return Ok(Some(record));
            }
        }
    }
    let Some(url) = lrclib::file_search_url(file) else {
        return Ok(None);
    };
    let hits = http::json(service, &url, Channel::Search, ticket).await?;
    Ok(lrclib::best_hit(&hits, file))
}

/// Keep supersession terminal across worker completion/failure and UI timeout.
/// Dropping a blocking JoinHandle does not terminate its native worker.
async fn await_fingerprint(
    worker: tauri::async_runtime::JoinHandle<Result<(f64, String), String>>,
    budget: std::time::Duration,
    stale: impl Fn() -> bool,
) -> Result<Result<(f64, String), String>, String> {
    let outcome = tokio::time::timeout(budget, worker).await;
    if stale() {
        return Err(http::CANCELLED.into());
    }
    outcome
        .map_err(|_| "Fingerprinting reached its time limit. The audio worker may still be stopping; try again shortly.".to_string())?
        .map_err(|e| e.to_string())
}

/// Actual linked backend identity for About notices and source verification.
#[tauri::command]
pub fn audio_fingerprint_backend() -> serde_json::Value {
    let info = opentag_audio_fingerprint::backend_info();
    serde_json::json!({"chromaprint":info.chromaprint,"ffmpeg":info.ffmpeg,
        "license":info.license,"audio_decoders":info.audio_decoders,
        "non_audio_decoders":info.non_audio_decoders,"protocols":info.protocols})
}

/// AcoustID: fingerprint each file locally, look the fingerprints up and
/// rank the releases that hold the most files.
async fn acoustid_search(
    app: &tauri::AppHandle,
    query: SearchQuery,
    ticket: u64,
) -> Result<SearchPage, String> {
    let key = required_key(
        "AcoustID",
        &saved_credentials(app).await?.acoustid_key,
        keys::acoustid(),
    )?;
    if query.mode != model::SearchMode::Fingerprint || query.files.is_empty() {
        return Err("AcoustID identifies the selected files by their audio fingerprints.".into());
    }
    // Mobile always uses embedded libraries; a desktop-only override setting
    // imported with preferences must not make mobile depend on an executable.
    let fpcalc = if cfg!(mobile) {
        None
    } else {
        acoustid::external_override(&online().fpcalc)?
    };
    let service = acoustid_service();
    let mut matches = Vec::new();
    let mut first_error = None;
    for file in &query.files {
        if http::stale(Channel::Search, ticket) {
            return Err(http::CANCELLED.into());
        }
        let path = PathBuf::from(&file.path);
        let worker = if let Some(program) = fpcalc.clone() {
            tauri::async_runtime::spawn_blocking(move || acoustid::fingerprint(&program, &path))
        } else {
            let control = opentag_audio_fingerprint::Control::new(
                std::time::Duration::from_secs(120),
                move || http::stale(Channel::Search, ticket),
            );
            // Admission is reserved before spawning, and remains held until
            // native work actually returns even if the UI deadline expires.
            #[cfg(target_os = "android")]
            let approved_path = path.clone();
            let job =
                opentag_audio_fingerprint::Job::new(path, control).map_err(|e| e.to_string())?;
            tauri::async_runtime::spawn_blocking(move || {
                #[cfg(target_os = "android")]
                if !crate::documents::is_managed(&approved_path) {
                    return Err("Choose the audio file through TagGroove's folder picker before fingerprinting it.".into());
                }
                job.run().map(|fp| (fp.duration, fp.encoded)).map_err(|e| {
                    if matches!(e, opentag_audio_fingerprint::Error::Cancelled) {
                        http::CANCELLED.into()
                    } else {
                        e.to_string()
                    }
                })
            })
        };
        let fingerprint = await_fingerprint(worker, std::time::Duration::from_secs(120), || {
            http::stale(Channel::Search, ticket)
        })
        .await?;
        let (duration, fingerprint) = match fingerprint {
            Ok(found) => found,
            Err(error) => {
                if error == http::CANCELLED {
                    return Err(error);
                }
                first_error.get_or_insert(error);
                matches.push(acoustid::FileMatch::default());
                continue;
            }
        };
        let form = acoustid::lookup_form(&key, duration, &fingerprint);
        let form: Vec<(&str, &str)> = form.iter().map(|(k, v)| (*k, v.as_str())).collect();
        let body =
            http::json_post(&service, acoustid::LOOKUP, &form, Channel::Search, ticket).await?;
        matches.push(acoustid::parse_lookup(&body)?);
    }
    if let Some(error) = first_error.filter(|_| matches.iter().all(|m| m.releases.is_empty())) {
        return Err(error);
    }
    if http::stale(Channel::Search, ticket) {
        return Err(http::CANCELLED.into());
    }
    acoustid::remember(&matches);
    let paths: Vec<String> = query.files.iter().map(|f| f.path.clone()).collect();
    acoustid::remember_files(&paths, &matches);
    let results = acoustid::rank(&matches);
    Ok(SearchPage {
        total: Some(results.len() as u32),
        results,
    })
}

fn import_options(settings: &freedb::FreedbSettings) -> freedb::ImportOptions {
    freedb::ImportOptions {
        comment_disc_id: settings.comment_disc_id,
        disc_id_field: settings.disc_id_field,
        cover_size: cover_size(),
    }
}

fn local_index_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    Ok(settings_path(app)?.with_file_name("local-freedb-index.tsv"))
}

/// Local freedb folders look discs up by the disc ID of the selected files,
/// a typed disc ID or their text index. Servers are asked with a `cddb query`
/// of the files' track layout or gnudb's artist and album `search`; a typed
/// disc ID is never sent to them (gnudb only reads entries a query listed).
async fn freedb_search(
    app: &tauri::AppHandle,
    provider: Provider,
    query: SearchQuery,
    ticket: u64,
) -> Result<SearchPage, String> {
    let settings = freedb::settings_from(&crate::settings::current());
    let results = if provider == Provider::LocalFreedb {
        let disc_id = match query.mode {
            model::SearchMode::Toc => freedb::Toc::from_lengths(&query.lengths)?.disc_id(),
            model::SearchMode::DiscId => freedb::disc_id(&query.text)?,
            model::SearchMode::Text => {
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
            _ => return Err("A local freedb folder is searched by disc ID or, with a text index, by artist and album words.".into()),
        };
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
    } else {
        let gnudb = freedb::offers_gnudb_commands(&settings);
        let toc = query.mode == model::SearchMode::Toc;
        let url = match query.mode {
            model::SearchMode::Toc => freedb::command_url(
                &settings,
                &freedb::Toc::from_lengths(&query.lengths)?.query_command(),
            )?,
            model::SearchMode::ArtistAlbum => {
                freedb::search_url(&settings, &query.artist, &query.album, query.lengths.len())?
            }
            _ => return Err(REMOTE_LOOKUPS.into()),
        };
        let body =
            http::freedb_text(&freedb_service(&settings), &url, Channel::Search, ticket).await?;
        let matches = if toc {
            freedb::parse_query(&body)?
        } else {
            freedb::parse_search(&body)?
        };
        freedb::remember_listed(&settings, &matches);
        matches
            .iter()
            .map(|found| freedb::match_result(found, provider, gnudb))
            .collect()
    };
    let total = Some(results.len() as u32);
    Ok(SearchPage { results, total })
}

const REMOTE_LOOKUPS: &str = "The freedb server finds discs by the track layout of the selected files or by artist and album. A typed disc ID can only be looked up in a local freedb folder.";

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
    // gnudb: `cddb read` only with the category and gnucdid a query returned.
    if !freedb::was_listed(&settings, &category, &disc_id) {
        return Err(freedb::LOOK_UP_FIRST.into());
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
        freedb::offers_gnudb_commands(&settings),
    )
}

/// gnudb's cover art search (`artsearch`) for Tag Sources → freedb: Cover Art
/// Archive images of the releases matching the artist and album.
#[tauri::command]
pub async fn freedb_cover_search(
    artist: String,
    album: String,
    ticket: u64,
) -> Result<Vec<model::SourceCover>, String> {
    http::claim(Channel::Release, ticket);
    let settings = freedb::settings_from(&crate::settings::current());
    let url = freedb::artsearch_url(&settings, &artist, &album)?;
    let body =
        http::freedb_text(&freedb_service(&settings), &url, Channel::Release, ticket).await?;
    freedb::parse_artsearch(&body, cover_size())
}

/// Look up a release by ID or address (search results pass their own ID).
/// `files` are the selected files, for sources that look files up one by one.
#[tauri::command]
pub async fn tag_source_release(
    app: tauri::AppHandle,
    provider: Provider,
    input: String,
    ticket: u64,
    files: Option<Vec<model::FileSignature>>,
) -> Result<SourceRelease, String> {
    http::claim(Channel::Release, ticket);
    let service = service(provider, discogs_token(&app, provider).await?);
    match provider {
        Provider::Beatport | Provider::Traxsource => {
            catalog::release(provider, &input, ticket).await
        }
        Provider::Musicbrainz | Provider::Acoustid => {
            if provider == Provider::Musicbrainz {
                if let Some(recording) = input.strip_prefix("recording:") {
                    if !musicbrainz::is_mbid(recording) {
                        return Err("Invalid MusicBrainz recording identifier.".into());
                    }
                    let body = http::json(
                        &service,
                        &musicbrainz::recording_url(recording),
                        Channel::Release,
                        ticket,
                    )
                    .await?;
                    return musicbrainz::parse_recording(&body, cover_size());
                }
            }
            let id = musicbrainz::release_id(&input)?;
            let body = http::json(
                &service,
                &musicbrainz::release_url(&id),
                Channel::Release,
                ticket,
            )
            .await?;
            let mut release = musicbrainz::parse_release(&body, cover_size())?;
            if provider == Provider::Acoustid {
                acoustid::annotate(&mut release);
            }
            Ok(release)
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
        Provider::Itunes => {
            let settings = online();
            let id = itunes::release_id(&input)?;
            let url = itunes::release_url(&id, &settings.itunes_country);
            let body = http::json(&service, &url, Channel::Release, ticket).await?;
            itunes::parse_release(&body, settings.itunes_cover)
        }
        Provider::Deezer => {
            if input.starts_with("track:") {
                let id = deezer::track_id(&input)?;
                let track =
                    http::json(&service, &deezer::track_url(&id), Channel::Release, ticket).await?;
                deezer::check(&track)?;
                let album_id = track
                    .pointer("/album/id")
                    .and_then(serde_json::Value::as_u64)
                    .filter(|value| *value > 0)
                    .ok_or("Deezer did not identify this track's album.")?
                    .to_string();
                let album = http::json(
                    &service,
                    &deezer::release_url(&album_id),
                    Channel::Release,
                    ticket,
                )
                .await?;
                let tracks = deezer::load_tracks(&album_id, |url| {
                    let service = &service;
                    async move { http::json(service, &url, Channel::Release, ticket).await }
                })
                .await?;
                return deezer::parse_track(&track, &album, &tracks, online().deezer_cover);
            }
            let id = deezer::release_id(&input)?;
            let album = http::json(
                &service,
                &deezer::release_url(&id),
                Channel::Release,
                ticket,
            )
            .await?;
            deezer::check(&album)?;
            let tracks = deezer::load_tracks(&id, |url| {
                let service = &service;
                async move { http::json(service, &url, Channel::Release, ticket).await }
            })
            .await?;
            deezer::parse_release(&album, &tracks, online().deezer_cover)
        }
        Provider::Lrclib => lrclib_release(&service, &input, files, ticket).await,
        Provider::Lastfm => {
            let key = required_key(
                "Last.fm",
                &saved_credentials(&app).await?.lastfm_key,
                keys::lastfm(),
            )?;
            if input.starts_with("track:") {
                let url = lastfm::track_info_url(&input, &key)?;
                let body = http::json_optional(&service, &url, Channel::Release, ticket)
                    .await?
                    .ok_or("Last.fm could not find this track.")?;
                return lastfm::parse_track(&body, &input, online().lastfm_genres);
            }
            let url = lastfm::info_url(&input, &key)?;
            let body = http::json_optional(&service, &url, Channel::Release, ticket)
                .await?
                .ok_or("Last.fm could not find this album.")?;
            lastfm::parse_release(&body, &input, online().lastfm_genres)
        }
        Provider::Fanart => {
            let id = fanart::release_group_id(&input)?;
            let fanart = required_key_for_fanart(&app).await?;
            let body =
                http::json_optional(&fanart, &fanart::album_url(&id), Channel::Release, ticket)
                    .await?
                    .ok_or("fanart.tv has no artwork for this album.")?;
            fanart::parse_album(&body, &id)
        }
    }
}

/// The MusicBrainz recordings the last AcoustID search identified each file
/// as, so the review pairs files with their own tracks.
#[tauri::command]
pub fn acoustid_file_matches() -> HashMap<String, Vec<acoustid::FileRecording>> {
    acoustid::file_matches()
}

/// The selected files' lyrics (`files:…`) or one lyrics record.
async fn lrclib_release(
    service: &http::Service,
    input: &str,
    files: Option<Vec<model::FileSignature>>,
    ticket: u64,
) -> Result<SourceRelease, String> {
    let record = |id: String| async move {
        match lrclib::remembered(&id) {
            Some(record) => Ok(record),
            None => http::json(service, &lrclib::record_url(&id), Channel::Release, ticket).await,
        }
    };
    let synced = online().lrclib_synced;
    if let Some(ids) = lrclib::file_ids(input) {
        let files = files.unwrap_or_default();
        if files.len() != ids.len() {
            return Err("The selection changed since the search. Search again.".into());
        }
        let mut records = Vec::new();
        for id in ids {
            records.push(match id {
                Some(id) => Some(record(id).await?),
                None => None,
            });
        }
        return Ok(lrclib::files_release(&files, &records, synced));
    }
    lrclib::record_release(&record(lrclib::record_id(input)?).await?, synced)
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
    let archive = http::Service::new("Cover Art Archive", &http::ARCHIVE, 0);
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
    /// LRCLIB supplies raw text, rather than ID3's language||lyrics syntax.
    #[serde(default)]
    pub raw_lyrics: bool,
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
    raw_lyrics: bool,
) -> Result<AudioEntry, String> {
    let options = crate::settings::tag_options();
    let mut entry = if updates.is_empty() {
        crate::settings::read_summary(path)?
    } else if raw_lyrics {
        tag_core::write_source_fields_with_options(
            path,
            updates,
            false,
            &options.mappings,
            &options,
        )?
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

/// Write reviewed values immediately, like Mp3tag and TagGroove's other importers.
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
    let updates: HashMap<&str, &ApplyItem> = items
        .iter()
        .map(|item| (item.path.as_str(), item))
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
                    apply_one(
                        working,
                        &fields.updates,
                        cover,
                        options.replace,
                        fields.raw_lyrics,
                    )
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
                raw_lyrics: false,
            },
            ApplyItem {
                path: audio.to_string_lossy().into_owned(),
                updates: Fields::from([("TITLE".into(), vec!["Help".into()])]),
                raw_lyrics: false,
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
            save_as: Some("$substr()".into()),
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
            false,
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
            false,
        );
        // Formats that cannot hold two untitled front covers report it per file.
        if let Ok(entry) = appended {
            assert_eq!(entry.covers.len(), 2);
        }
        let unchanged = apply_one(&audio, &Fields::new(), None, false, false).unwrap();
        assert_eq!(unchanged.fields["ALBUM"], ["Discovery"]);
    }

    /// LRCLIB lyrics have no language: ID3v2 files get "XXX", others the plain text.
    #[test]
    fn lyrics_without_a_language_are_written_to_id3_and_other_tags() {
        let dir = tempfile::tempdir().unwrap();
        let wav_file = dir.path().join("a.wav");
        wav(&wav_file);
        let updates: Fields = [(
            "UNSYNCEDLYRICS".to_string(),
            vec!["Line one\nLine two".to_string()],
        )]
        .into();
        apply_one(&wav_file, &updates, None, false, true).unwrap();
        let lyrics = &tag_core::read_entry(&wav_file).unwrap().fields["UNSYNCEDLYRICS"];
        assert_eq!(
            lyrics,
            &["XXX||Line one\nLine two"],
            "WAV keeps lyrics in ID3v2"
        );
    }

    #[test]
    fn raw_online_lyrics_keep_literal_language_delimiters() {
        let dir = tempfile::tempdir().unwrap();
        for (index, text) in ["A line || another line", "eng||literal lyric text"]
            .iter()
            .enumerate()
        {
            let audio = dir.path().join(format!("{index}.wav"));
            wav(&audio);
            let updates: Fields = [("UNSYNCEDLYRICS".to_string(), vec![text.to_string()])].into();
            apply_one(&audio, &updates, None, false, true).unwrap();
            assert_eq!(
                tag_core::read_entry(&audio).unwrap().fields["UNSYNCEDLYRICS"],
                [format!("XXX||{text}")]
            );
        }
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
    fn online_service_options_have_safe_defaults() {
        let mut settings = crate::settings::Settings::default();
        let defaults = online_from(&settings);
        assert_eq!(defaults.itunes_country, "US");
        assert_eq!(defaults.itunes_cover, itunes::CoverSize::Large);
        assert_eq!(defaults.deezer_cover, deezer::CoverSize::Xl);
        assert_eq!(defaults.lastfm_genres, 1);
        assert!(!defaults.lrclib_synced && defaults.fpcalc.is_empty());
        settings.pages.insert(
            "tag-sources-online".into(),
            serde_json::json!({"itunes_country": "nl", "itunes_cover_size": "3000", "deezer_cover_size": "500",
                "lrclib_synced": true, "lastfm_genres": "3", "fpcalc": " /usr/bin/fpcalc "}),
        );
        let chosen = online_from(&settings);
        assert_eq!(chosen.itunes_country, "NL");
        assert_eq!(chosen.itunes_cover, itunes::CoverSize::Huge);
        assert_eq!(chosen.deezer_cover, deezer::CoverSize::Big);
        assert_eq!(chosen.lastfm_genres, 3);
        assert!(chosen.lrclib_synced);
        assert_eq!(chosen.fpcalc, "/usr/bin/fpcalc");
    }

    #[test]
    fn keyed_services_need_a_key_and_keep_it_out_of_addresses() {
        assert!(required_key("Last.fm", "", None)
            .unwrap_err()
            .contains("API keys"));
        assert_eq!(required_key("Last.fm", " mine ", None).unwrap(), "mine");
        if keys::fanart().is_none() {
            assert!(fanart_service("").is_err());
            let service = fanart_service("personal").unwrap();
            assert!(
                matches!(&service.auth, http::Auth::Headers(h) if h == &[("api-key", "personal".to_owned())])
            );
        }
        assert!(matches!(
            service(Provider::Lastfm, String::new()).auth,
            http::Auth::Key
        ));
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
            raw_lyrics: false,
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
            raw_lyrics: false,
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
            raw_lyrics: false,
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
            apply_one(path, &fields, None, false, false).unwrap();
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
            raw_lyrics: false,
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
            raw_lyrics: false,
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
            assert_eq!(
                crate::history::history_state().unwrap().notice,
                None,
                "the import must retain its Undo snapshot before this test replays history"
            );
        };
        run(&save("%album%"));
        run(&save("folder"));
        let created = root.join("folder.png");
        assert_eq!(std::fs::read(&replaced).unwrap(), png);
        assert_eq!(std::fs::read(&created).unwrap(), png);
        use tauri::async_runtime::block_on;
        let undone = serde_json::to_value(block_on(crate::history::undo_last()).unwrap()).unwrap();
        assert_eq!(undone["errors"], serde_json::json!([]), "{undone}");
        assert!(
            !created.exists(),
            "the second import's new image is removed"
        );
        assert_eq!(std::fs::read(&replaced).unwrap(), png);
        assert!(crate::history::history_state().unwrap().undo.is_some());
        let undone = serde_json::to_value(block_on(crate::history::undo_last()).unwrap()).unwrap();
        assert_eq!(undone["errors"], serde_json::json!([]), "{undone}");
        assert_eq!(std::fs::read(&replaced).unwrap(), b"previous image");
        assert_eq!(std::fs::read(&audio).unwrap(), original);
        let redone = serde_json::to_value(block_on(crate::history::redo_last()).unwrap()).unwrap();
        assert_eq!(redone["errors"], serde_json::json!([]), "{redone}");
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

    #[test]
    fn remote_freedb_reads_need_a_query_first() {
        // Refused before any request: the entry was never listed by a query or search.
        let error = tauri::async_runtime::block_on(freedb_release(
            Provider::Freedb,
            "rock/7a0a6b09",
            u64::MAX,
        ))
        .unwrap_err();
        assert_eq!(error, freedb::LOOK_UP_FIRST);
    }

    /// gnudb answers only registered applications; this records what it says to TagGroove.
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

    /// Manual check of the keyless round-4 services:
    /// `cargo test -p opentag live_round_four -- --ignored`.
    #[test]
    #[ignore]
    fn live_round_four_itunes_deezer_and_lrclib() {
        use tauri::async_runtime::block_on;
        let get = |provider, url: String| {
            block_on(http::json(
                &service(provider, String::new()),
                &url,
                Channel::Release,
                u64::MAX,
            ))
            .unwrap()
        };
        let apple = get(Provider::Itunes, itunes::release_url("697194953", "US"));
        let release = itunes::parse_release(&apple, itunes::CoverSize::Large).unwrap();
        assert_eq!(release.fields["ALBUM"], ["Discovery"]);
        let album = get(Provider::Deezer, deezer::release_url("302127"));
        let tracks = get(Provider::Deezer, deezer::tracks_url("302127"));
        let release = deezer::parse_release(&album, &tracks, deezer::CoverSize::Xl).unwrap();
        assert_eq!(release.tracks[0].fields["ISRC"], ["GBDUW0000053"]);
        let lyrics = get(Provider::Lrclib, lrclib::record_url("250327"));
        assert!(lrclib::record_release(&lyrics, false).is_ok());
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
                    _ => unreachable!(),
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
        let archive = http::Service::new("Cover Art Archive", &http::ARCHIVE, 0);
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

#[cfg(test)]
mod fingerprint_worker_tests {
    use super::*;
    use std::sync::{
        atomic::{AtomicBool, Ordering},
        mpsc, Arc, Condvar, Mutex,
    };
    #[test]
    fn superseded_worker_cannot_return_partial_success_and_timeout_keeps_admission() {
        let stale = Arc::new(AtomicBool::new(false));
        let gate = Arc::new((Mutex::new(false), Condvar::new()));
        let worker_gate = gate.clone();
        let (entered, waiting) = mpsc::channel();
        let (finished, stopped) = mpsc::channel();
        let control = opentag_audio_fingerprint::Control::new(
            std::time::Duration::from_secs(10),
            move || {
                let (lock, wake) = &*worker_gate;
                let mut released = lock.lock().unwrap();
                let _ = entered.send(());
                while !*released {
                    released = wake.wait(released).unwrap();
                }
                true
            },
        );
        let job =
            opentag_audio_fingerprint::Job::new("/missing/never-opened".into(), control).unwrap();
        let worker = tauri::async_runtime::spawn_blocking(move || {
            let result = job
                .run()
                .map(|fp| (fp.duration, fp.encoded))
                .map_err(|e| e.to_string());
            let _ = finished.send(());
            result
        });
        waiting
            .recv_timeout(std::time::Duration::from_secs(2))
            .unwrap();
        let cancelled = stale.clone();
        let result = tauri::async_runtime::block_on(await_fingerprint(
            worker,
            std::time::Duration::from_millis(10),
            move || cancelled.load(Ordering::Acquire),
        ));
        assert!(result.unwrap_err().contains("time limit"));
        assert!(matches!(
            opentag_audio_fingerprint::Job::new("/not/queued".into(), Default::default()),
            Err(opentag_audio_fingerprint::Error::Busy)
        ));
        stale.store(true, Ordering::Release);
        let late = tauri::async_runtime::spawn_blocking(|| Ok((12.0, "old-partial-result".into())));
        let result = tauri::async_runtime::block_on(await_fingerprint(
            late,
            std::time::Duration::from_secs(2),
            || stale.load(Ordering::Acquire),
        ));
        assert_eq!(result.unwrap_err(), http::CANCELLED);
        let (lock, wake) = &*gate;
        *lock.lock().unwrap() = true;
        wake.notify_all();
        stopped
            .recv_timeout(std::time::Duration::from_secs(2))
            .unwrap();
        assert!(opentag_audio_fingerprint::Job::new(
            "/admitted/after/return".into(),
            Default::default()
        )
        .is_ok());
    }
}
