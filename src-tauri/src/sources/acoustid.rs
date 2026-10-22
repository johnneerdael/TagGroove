//! AcoustID fingerprint lookups (<https://acoustid.org/webservice>). Each
//! selected file is fingerprinted locally with the embedded Chromaprint and
//! audio decoder libraries. The lookup returns MusicBrainz recordings with
//! their releases. Releases holding the most selected files become search
//! results, and the chosen one is read from MusicBrainz.
//! AcoustID allows three requests per second and free use for
//! non-commercial applications with a registered application key.
use super::model::*;
use serde_json::Value;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{Duration, Instant};

pub const LOOKUP: &str = "https://api.acoustid.org/v2/lookup";
const FPCALC_TIMEOUT: Duration = Duration::from_secs(120);

/// An explicit optional desktop override. Blank and the old auto-discovery
/// sentinel migrate to the embedded default, including persisted settings.
pub fn external_override(configured: &str) -> Result<Option<PathBuf>, String> {
    let configured = configured.trim();
    if configured.is_empty()
        || configured.eq_ignore_ascii_case("fpcalc")
        || configured.eq_ignore_ascii_case("fpcalc.exe")
    {
        return Ok(None);
    }
    let path = PathBuf::from(configured);
    if path.is_file() {
        Ok(Some(path))
    } else {
        Err(format!("The optional fpcalc override was not found at {path}. Clear it to use built-in fingerprinting in Options → Tag Sources → Online services.", path=configured))
    }
}

/// `fpcalc -json` output: the file's length and its compressed fingerprint.
pub fn parse_fpcalc(output: &str) -> Result<(f64, String), String> {
    let value: Value = serde_json::from_str(output.trim())
        .map_err(|_| "fpcalc returned an unexpected answer.".to_string())?;
    let duration = value
        .get("duration")
        .and_then(Value::as_f64)
        .filter(|d| *d > 0.0);
    let fingerprint = value
        .get("fingerprint")
        .and_then(Value::as_str)
        .filter(|f| !f.is_empty() && f.chars().all(|c| c.is_ascii_graphic()));
    match (duration, fingerprint) {
        (Some(duration), Some(fingerprint)) => Ok((duration, fingerprint.to_owned())),
        _ => Err("fpcalc could not fingerprint this file.".into()),
    }
}

/// Run `fpcalc -json <file>` (no shell), stopping it after two minutes.
pub fn fingerprint(fpcalc: &Path, file: &Path) -> Result<(f64, String), String> {
    let mut child = std::process::Command::new(fpcalc)
        .arg("-json")
        .arg(file)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()
        .map_err(|e| format!("fpcalc could not be started: {value}", value = e))?;
    let mut stdout = child.stdout.take().ok_or("fpcalc has no output")?;
    let reader = std::thread::spawn(move || {
        let mut text = String::new();
        std::io::Read::read_to_string(&mut stdout, &mut text).map(|_| text)
    });
    let started = Instant::now();
    let status = loop {
        if let Some(status) = child.try_wait().map_err(|e| e.to_string())? {
            break status;
        }
        if started.elapsed() > FPCALC_TIMEOUT {
            let _ = child.kill();
            let _ = child.wait();
            return Err("fpcalc took too long and was stopped.".into());
        }
        std::thread::sleep(Duration::from_millis(50));
    };
    let output = reader
        .join()
        .map_err(|_| "fpcalc output could not be read")?
        .map_err(|e| e.to_string())?;
    if !status.success() {
        return Err("fpcalc could not read this file.".into());
    }
    parse_fpcalc(&output)
}

/// The lookup's form fields (sent as a POST, as AcoustID prefers), with
/// TagGroove's version as the client version.
pub fn lookup_form(key: &str, duration: f64, fingerprint: &str) -> Vec<(&'static str, String)> {
    vec![
        ("client", key.to_owned()),
        ("clientversion", env!("CARGO_PKG_VERSION").to_owned()),
        ("duration", (duration.round() as u64).to_string()),
        ("fingerprint", fingerprint.to_owned()),
        ("meta", "recordings releases compress".to_owned()),
        ("format", "json".to_owned()),
    ]
}

/// AcoustID reports errors as `{"status": "error", "error": {"code", "message"}}`.
pub fn check(body: &Value) -> Result<(), String> {
    if body.get("status").and_then(Value::as_str) == Some("ok") {
        return Ok(());
    }
    let code = body.pointer("/error/code").and_then(Value::as_u64);
    let message = body
        .pointer("/error/message")
        .and_then(Value::as_str)
        .unwrap_or("unknown error");
    Err(match code {
        Some(4) => "AcoustID rejected TagGroove's application key.".into(),
        Some(14) => "AcoustID rate limit reached. Wait a moment and try again.".into(),
        _ => format!("AcoustID returned an error: {message}"),
    })
}

/// What one file's lookup found.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct FileMatch {
    /// Recordings of every fingerprint match: (MusicBrainz recording ID,
    /// the AcoustID of the match that names it, its score), best match first.
    pub recordings: Vec<(String, String, f64)>,
    /// Releases holding one of the recordings: (release ID, summary).
    pub releases: Vec<(String, Value, f64)>,
}

pub fn parse_lookup(body: &Value) -> Result<FileMatch, String> {
    check(body)?;
    let mut found = FileMatch::default();
    let results = body
        .get("results")
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or_default();
    for result in results {
        let score = result.get("score").and_then(Value::as_f64).unwrap_or(0.0);
        let acoustid = result
            .get("id")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_owned();
        for recording in result
            .get("recordings")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            let Some(id) = recording.get("id").and_then(Value::as_str) else {
                continue;
            };
            found
                .recordings
                .push((id.to_owned(), acoustid.clone(), score));
            let artist = recording
                .get("artists")
                .and_then(Value::as_array)
                .map(|artists| {
                    artists
                        .iter()
                        .map(|a| {
                            format!(
                                "{}{}",
                                a.get("name").and_then(Value::as_str).unwrap_or_default(),
                                a.get("joinphrase")
                                    .and_then(Value::as_str)
                                    .unwrap_or_default()
                            )
                        })
                        .collect::<String>()
                })
                .unwrap_or_default();
            for release in recording
                .get("releases")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
            {
                let Some(release_id) = release.get("id").and_then(Value::as_str) else {
                    continue;
                };
                let mut summary = release.clone();
                if summary.get("artists").is_none() {
                    summary["recording_artist"] = Value::String(artist.clone());
                }
                found.releases.push((release_id.to_owned(), summary, score));
            }
        }
    }
    Ok(found)
}

/// Releases ranked by how many files they hold, then by fingerprint score.
pub fn rank(matches: &[FileMatch]) -> Vec<SearchResult> {
    struct Candidate {
        summary: Value,
        files: std::collections::BTreeSet<usize>,
        score: f64,
    }
    let mut candidates: Vec<(String, Candidate)> = Vec::new();
    for (file, found) in matches.iter().enumerate() {
        for (id, summary, score) in &found.releases {
            match candidates.iter_mut().find(|(known, _)| known == id) {
                Some((_, candidate)) => {
                    if candidate.files.insert(file) {
                        candidate.score += score;
                    }
                }
                None => candidates.push((
                    id.clone(),
                    Candidate {
                        summary: summary.clone(),
                        files: [file].into(),
                        score: *score,
                    },
                )),
            }
        }
    }
    candidates.sort_by(|a, b| {
        b.1.files
            .len()
            .cmp(&a.1.files.len())
            .then(b.1.score.total_cmp(&a.1.score))
    });
    let total = matches.len();
    candidates
        .into_iter()
        .filter(|(id, _)| super::musicbrainz::is_mbid(id))
        .map(|(id, candidate)| {
            let release = &candidate.summary;
            let text = |key: &str| {
                release
                    .get(key)
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .trim()
                    .to_owned()
            };
            let artist = match release.get("artists").and_then(Value::as_array) {
                Some(artists) => artists
                    .iter()
                    .map(|a| {
                        format!(
                            "{}{}",
                            a.get("name").and_then(Value::as_str).unwrap_or_default(),
                            a.get("joinphrase")
                                .and_then(Value::as_str)
                                .unwrap_or_default()
                        )
                    })
                    .collect(),
                None => text("recording_artist"),
            };
            let date_year = release
                .pointer("/date/year")
                .and_then(Value::as_u64)
                .map(|y| y.to_string())
                .unwrap_or_default();
            let files = candidate.files.len();
            SearchResult {
                title: text("title"),
                artist,
                year: date_year,
                country: text("country"),
                format: format!("{files} of {total} files"),
                track_count: release
                    .get("track_count")
                    .and_then(Value::as_u64)
                    .map(|n| n as u32),
                score: Some(((candidate.score / files as f64) * 100.0).round() as u32),
                thumb: Some(super::musicbrainz::front_thumb_url(&id)),
                url: format!("https://musicbrainz.org/release/{id}"),
                id,
                ..Default::default()
            }
        })
        .collect()
}

/// AcoustIDs of the recordings the last search identified, so the chosen
/// release can carry ACOUSTID_ID for the tracks that were identified. Each
/// completed search replaces it: an earlier search's files must not lend
/// their AcoustIDs to this one's tracks.
static ACOUSTIDS: Mutex<Option<HashMap<String, String>>> = Mutex::new(None);

pub fn remember(matches: &[FileMatch]) {
    let mut known = HashMap::new();
    for found in matches {
        // A recording named by several matches keeps the best one's AcoustID.
        let mut seen = std::collections::HashSet::new();
        for (recording, acoustid, _) in &found.recordings {
            if !acoustid.is_empty() && seen.insert(recording) {
                known.insert(recording.clone(), acoustid.clone());
            }
        }
    }
    *ACOUSTIDS.lock().unwrap_or_else(|e| e.into_inner()) = Some(known);
}

/// The recordings each file of the last search was identified as (best
/// first), so the review can pair every file with its own track. Replaced
/// by each completed search, like [`ACOUSTIDS`].
static FILES: Mutex<Option<HashMap<String, Vec<FileRecording>>>> = Mutex::new(None);

/// A recording one file was identified as, and the AcoustID of the match
/// that named it for that file: the file's own ACOUSTID_ID when it is saved
/// on that recording's track.
#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub struct FileRecording {
    pub recording: String,
    pub acoustid: String,
}

/// `paths` and `matches` are in the same (file list) order.
pub fn remember_files(paths: &[String], matches: &[FileMatch]) {
    let files = paths
        .iter()
        .zip(matches)
        .map(|(path, found)| {
            let mut recordings: Vec<FileRecording> = Vec::new();
            for (recording, acoustid, _) in &found.recordings {
                if !recordings.iter().any(|known| known.recording == *recording) {
                    recordings.push(FileRecording {
                        recording: recording.clone(),
                        acoustid: acoustid.clone(),
                    });
                }
            }
            (path.clone(), recordings)
        })
        .filter(|(_, recordings)| !recordings.is_empty())
        .collect();
    *FILES.lock().unwrap_or_else(|e| e.into_inner()) = Some(files);
}

pub fn file_matches() -> HashMap<String, Vec<FileRecording>> {
    FILES
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .clone()
        .unwrap_or_default()
}

/// Add ACOUSTID_ID to the tracks whose recording was identified, so the
/// review offers the field. When saving, each file gets its own AcoustID
/// (or none) from [`file_matches`], whatever track it ends up on.
pub fn annotate(release: &mut SourceRelease) {
    let known = ACOUSTIDS.lock().unwrap_or_else(|e| e.into_inner());
    let Some(known) = known.as_ref() else {
        return;
    };
    for track in &mut release.tracks {
        let acoustid = track
            .fields
            .get("MUSICBRAINZ_TRACKID")
            .and_then(|ids| ids.first())
            .and_then(|recording| known.get(recording))
            .cloned();
        if let Some(acoustid) = acoustid {
            put_one(&mut track.fields, "ACOUSTID_ID", acoustid);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    const RELEASE: &str = "d073287b-d1bd-4f11-a933-a4386f8cf701";
    /// The last search's matches are process-wide; tests that set them take turns.
    static SEARCHES: Mutex<()> = Mutex::new(());
    fn one_search_at_a_time() -> std::sync::MutexGuard<'static, ()> {
        SEARCHES.lock().unwrap_or_else(|e| e.into_inner())
    }

    fn lookup(recording: &str, releases: &[&str], score: f64) -> Value {
        serde_json::json!({"status": "ok", "results": [{"id": format!("acoustid-{recording}"), "score": score, "recordings": [{
            "id": recording, "artists": [{"name": "Daft Punk"}],
            "releases": releases.iter().map(|id| serde_json::json!({"id": id, "title": "Discovery", "country": "XE", "date": {"year": 2001}, "track_count": 14})).collect::<Vec<_>>()
        }]}]})
    }

    #[test]
    fn fpcalc_output_is_read_and_checked() {
        assert_eq!(
            parse_fpcalc(r#"{"duration": 320.36, "fingerprint": "AQADtE"}"#).unwrap(),
            (320.36, "AQADtE".to_owned())
        );
        assert!(parse_fpcalc(r#"{"duration": 0, "fingerprint": "AQ"}"#).is_err());
        assert!(parse_fpcalc("ERROR: could not open").is_err());
        let form = lookup_form("KEY", 320.36, "AQADtE");
        assert!(form.contains(&("duration", "320".to_owned())));
        assert!(form.contains(&("clientversion", env!("CARGO_PKG_VERSION").to_owned())));
        assert!(form.contains(&("meta", "recordings releases compress".to_owned())));
        assert_eq!(
            external_override("/no/such/fpcalc").unwrap_err(),
            "The optional fpcalc override was not found at /no/such/fpcalc. Clear it to use built-in fingerprinting in Options → Tag Sources → Online services."
        );
    }

    #[test]
    fn releases_holding_most_files_rank_first() {
        let other = "11111111-2222-3333-4444-555555555555";
        let matches = vec![
            parse_lookup(&lookup("rec-1", &[RELEASE, other], 0.9)).unwrap(),
            parse_lookup(&lookup("rec-2", &[RELEASE], 0.8)).unwrap(),
            FileMatch::default(),
        ];
        let results = rank(&matches);
        assert_eq!(results[0].id, RELEASE);
        assert_eq!(results[0].format, "2 of 3 files");
        assert_eq!(results[0].score, Some(85));
        assert_eq!(
            (results[0].artist.as_str(), results[0].year.as_str()),
            ("Daft Punk", "2001")
        );
        assert_eq!(results[1].id, other);
        assert!(check(&serde_json::json!({"status": "error", "error": {"code": 4, "message": "invalid API key"}}))
            .unwrap_err()
            .contains("TagGroove's application key"));
    }

    #[test]
    fn identified_tracks_get_their_acoustid() {
        let _turn = one_search_at_a_time();
        let matches = vec![parse_lookup(&lookup("rec-annotate", &[RELEASE], 1.0)).unwrap()];
        remember(&matches);
        let mut fields = tag_core::Fields::new();
        put_one(&mut fields, "MUSICBRAINZ_TRACKID", "rec-annotate");
        let mut release = SourceRelease {
            preview_url: None,
            provider: Provider::Acoustid,
            id: RELEASE.into(),
            url: String::new(),
            title: String::new(),
            artist: String::new(),
            fields: Default::default(),
            tracks: vec![SourceTrack {
                position: "1".into(),
                disc: 1,
                number: 1,
                title: String::new(),
                artist: String::new(),
                length: None,
                fields,
            }],
            covers: Vec::new(),
            more_covers: false,
        };
        annotate(&mut release);
        assert_eq!(
            release.tracks[0].fields["ACOUSTID_ID"],
            ["acoustid-rec-annotate"]
        );
    }

    /// Each match keeps its own AcoustID: a release found through the second
    /// match must not get the first match's ID.
    #[test]
    fn every_match_carries_its_own_acoustid() {
        let _turn = one_search_at_a_time();
        let body = serde_json::json!({"status": "ok", "results": [
            {"id": "acoustid-first", "score": 0.9, "recordings": [{"id": "rec-first-match", "releases": [{"id": RELEASE}]}]},
            {"id": "acoustid-second", "score": 0.7, "recordings": [
                {"id": "rec-second-match", "releases": [{"id": "11111111-2222-3333-4444-555555555555"}]},
                {"id": "rec-first-match"}]},
        ]});
        let found = parse_lookup(&body).unwrap();
        assert_eq!(
            found.recordings,
            [
                (
                    "rec-first-match".to_owned(),
                    "acoustid-first".to_owned(),
                    0.9
                ),
                (
                    "rec-second-match".to_owned(),
                    "acoustid-second".to_owned(),
                    0.7
                ),
                (
                    "rec-first-match".to_owned(),
                    "acoustid-second".to_owned(),
                    0.7
                ),
            ]
        );
        remember(&[found]);
        let known = ACOUSTIDS.lock().unwrap();
        let known = known.as_ref().unwrap();
        assert_eq!(known["rec-second-match"], "acoustid-second");
        assert_eq!(
            known["rec-first-match"], "acoustid-first",
            "the better match wins"
        );
    }

    #[test]
    fn files_remember_their_recordings_best_first() {
        let _turn = one_search_at_a_time();
        let matches = vec![
            parse_lookup(&lookup("rec-file-b", &[RELEASE], 0.9)).unwrap(),
            FileMatch::default(),
        ];
        remember_files(&["/m/b.mp3".to_owned(), "/m/x.mp3".to_owned()], &matches);
        let files = file_matches();
        assert_eq!(
            files["/m/b.mp3"],
            [FileRecording {
                recording: "rec-file-b".into(),
                acoustid: "acoustid-rec-file-b".into()
            }]
        );
        assert!(
            !files.contains_key("/m/x.mp3"),
            "unidentified files are left out"
        );
    }

    /// A second search replaces the first one's matches: a track whose
    /// recording only the earlier search identified gets no ACOUSTID_ID.
    #[test]
    fn each_search_replaces_the_previous_matches() {
        let _turn = one_search_at_a_time();
        let first = vec![
            parse_lookup(&lookup("rec-old", &[RELEASE], 1.0)).unwrap(),
            parse_lookup(&lookup("rec-kept", &[RELEASE], 1.0)).unwrap(),
        ];
        remember(&first);
        remember_files(&["/m/old.mp3".to_owned(), "/m/kept.mp3".to_owned()], &first);
        let second = vec![parse_lookup(&lookup("rec-kept", &[RELEASE], 1.0)).unwrap()];
        remember(&second);
        remember_files(&["/m/new.mp3".to_owned()], &second);

        let track = |recording: &str| {
            let mut fields = tag_core::Fields::new();
            put_one(&mut fields, "MUSICBRAINZ_TRACKID", recording);
            SourceTrack {
                position: "1".into(),
                disc: 1,
                number: 1,
                title: String::new(),
                artist: String::new(),
                length: None,
                fields,
            }
        };
        let mut release = SourceRelease {
            preview_url: None,
            provider: Provider::Acoustid,
            id: RELEASE.into(),
            url: String::new(),
            title: String::new(),
            artist: String::new(),
            fields: Default::default(),
            tracks: vec![track("rec-old"), track("rec-kept")],
            covers: Vec::new(),
            more_covers: false,
        };
        annotate(&mut release);
        assert!(
            !release.tracks[0].fields.contains_key("ACOUSTID_ID"),
            "stale AcoustID"
        );
        assert_eq!(
            release.tracks[1].fields["ACOUSTID_ID"],
            ["acoustid-rec-kept"]
        );
        let files = file_matches();
        assert_eq!(files.keys().collect::<Vec<_>>(), ["/m/new.mp3"]);
    }

    /// A stand-in fpcalc (a shell script) proves the process handling.
    #[cfg(unix)]
    #[test]
    fn fpcalc_runs_without_a_shell_and_reports_failures() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let script = dir.path().join("fpcalc");
        std::fs::write(
            &script,
            "#!/bin/sh\n[ \"$1\" = -json ] || exit 2\n[ -f \"$2\" ] || exit 3\necho '{\"duration\": 3.0, \"fingerprint\": \"AQAB\"}'\n",
        )
        .unwrap();
        std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();
        let audio = dir.path().join("a b.wav");
        std::fs::write(&audio, b"x").unwrap();
        assert_eq!(
            fingerprint(&script, &audio).unwrap(),
            (3.0, "AQAB".to_owned())
        );
        assert!(fingerprint(&script, &dir.path().join("missing.wav")).is_err());
        assert_eq!(
            external_override(script.to_str().unwrap()).unwrap(),
            Some(script)
        );
    }
}

#[cfg(test)]
mod embedded_default_tests {
    use super::*;
    #[test]
    fn empty_and_legacy_defaults_use_embedded_fingerprints() {
        for value in ["", "  ", "fpcalc", "fpcalc.exe", " FPCALC "] {
            assert_eq!(external_override(value).unwrap(), None);
        }
        let file = tempfile::NamedTempFile::new().unwrap();
        assert_eq!(
            external_override(file.path().to_str().unwrap()).unwrap(),
            Some(file.path().to_owned())
        );
        assert!(external_override("/missing/optional/fpcalc").is_err());
    }
}
