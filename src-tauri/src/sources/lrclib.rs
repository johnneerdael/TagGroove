//! LRCLIB lyrics (<https://lrclib.net/docs>): no key, a descriptive user
//! agent. Each selected file is looked up by its own signature (title,
//! artist, album, length); `GET /api/get` matches lengths within a couple of
//! seconds, and `GET /api/search` is the fallback. Plain lyrics go to
//! UNSYNCEDLYRICS (Mp3tag's field: ID3 USLT, MP4 ©lyr, Vorbis/APE
//! UNSYNCEDLYRICS); with the option set, the time-stamped LRC text is written
//! there instead, as many players read it from that field. Mp3tag maps no
//! field to ID3's synchronised lyrics frame (SYLT), so neither does TagGroove.
use super::model::*;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Mutex;
use tag_core::Fields;

pub const API: &str = "https://lrclib.net/api";
/// Prefix of the search result that stands for every selected file.
pub const FILES: &str = "files:";

fn text(value: &Value, key: &str) -> String {
    match value.get(key) {
        Some(Value::String(s)) => s.trim().to_owned(),
        Some(Value::Number(n)) => n.to_string(),
        _ => String::new(),
    }
}
fn lyrics(value: &Value, key: &str) -> String {
    value
        .get(key)
        .and_then(Value::as_str)
        .map(|s| s.trim_end().to_owned())
        .unwrap_or_default()
}

/// The exact-signature lookup for one file; `None` without a title or artist.
pub fn get_url(file: &FileSignature) -> Option<String> {
    let (title, artist) = (file.title.trim(), file.artist.trim());
    if title.is_empty() || artist.is_empty() {
        return None;
    }
    let duration = (file.length.is_finite() && file.length > 0.0)
        .then(|| (file.length.round() as u64).to_string());
    let mut params = vec![
        ("track_name", title.to_owned()),
        ("artist_name", artist.to_owned()),
        ("album_name", file.album.trim().to_owned()),
    ];
    if let Some(duration) = duration {
        params.push(("duration", duration));
    }
    reqwest::Url::parse_with_params(&format!("{API}/get"), &params)
        .ok()
        .map(String::from)
}

/// The fallback search for one file: title and artist only.
pub fn file_search_url(file: &FileSignature) -> Option<String> {
    let (title, artist) = (file.title.trim(), file.artist.trim());
    if title.is_empty() {
        return None;
    }
    let mut params = vec![("track_name", title)];
    if !artist.is_empty() {
        params.push(("artist_name", artist));
    }
    reqwest::Url::parse_with_params(&format!("{API}/search"), &params)
        .ok()
        .map(String::from)
}

pub fn text_search_url(text: &str) -> Result<String, String> {
    let text = text.trim();
    if text.is_empty() {
        return Err("Enter words from the title, artist or album.".into());
    }
    reqwest::Url::parse_with_params(&format!("{API}/search"), &[("q", text)])
        .map(String::from)
        .map_err(|e| e.to_string())
}

pub fn record_url(id: &str) -> String {
    format!("{API}/get/{id}")
}

pub fn record_id(input: &str) -> Result<String, String> {
    let input = input.trim();
    if !input.is_empty() && input.len() <= 12 && input.chars().all(|c| c.is_ascii_digit()) {
        Ok(input.to_owned())
    } else {
        Err("Enter an LRCLIB lyrics record ID (a number).".into())
    }
}

/// The search hit closest in length to the file (within three seconds when
/// the file's length is known).
pub fn best_hit(hits: &Value, file: &FileSignature) -> Option<Value> {
    let hits = hits.as_array()?;
    let length = (file.length.is_finite() && file.length > 0.0).then_some(file.length);
    let mut usable = hits.iter().filter(|hit| {
        !lyrics(hit, "plainLyrics").is_empty() || !lyrics(hit, "syncedLyrics").is_empty()
    });
    match length {
        None => usable.next().cloned(),
        Some(length) => usable
            .filter_map(|hit| {
                let gap = (hit.get("duration")?.as_f64()? - length).abs();
                (gap <= 3.0).then_some((gap, hit))
            })
            .min_by(|a, b| a.0.total_cmp(&b.0))
            .map(|(_, hit)| hit.clone()),
    }
}

/// Records found by the last searches, so opening the result needs no second download.
static RECORDS: Mutex<Option<HashMap<String, Value>>> = Mutex::new(None);

pub fn remember(record: &Value) {
    let id = text(record, "id");
    if id.is_empty() {
        return;
    }
    let mut cache = RECORDS.lock().unwrap_or_else(|e| e.into_inner());
    let cache = cache.get_or_insert_with(HashMap::new);
    if cache.len() > 2000 {
        cache.clear();
    }
    cache.insert(id, record.clone());
}
pub fn remembered(id: &str) -> Option<Value> {
    RECORDS
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .as_ref()
        .and_then(|cache| cache.get(id).cloned())
}

fn kind(record: &Value) -> &'static str {
    if record.get("instrumental").and_then(Value::as_bool) == Some(true) {
        "Instrumental"
    } else if !lyrics(record, "syncedLyrics").is_empty() {
        "Synced"
    } else if !lyrics(record, "plainLyrics").is_empty() {
        "Plain"
    } else {
        "No lyrics"
    }
}

/// Free-text results: one row per lyrics record.
pub fn parse_search(body: &Value) -> SearchPage {
    let results: Vec<SearchResult> = body
        .as_array()
        .map(Vec::as_slice)
        .unwrap_or_default()
        .iter()
        .filter_map(|record| {
            let id = text(record, "id");
            (!id.is_empty()).then(|| {
                remember(record);
                SearchResult {
                    title: text(record, "trackName"),
                    artist: text(record, "artistName"),
                    label: text(record, "albumName"),
                    format: kind(record).into(),
                    track_count: record
                        .get("duration")
                        .and_then(Value::as_f64)
                        .map(|d| d.round() as u32),
                    id,
                    ..Default::default()
                }
            })
        })
        .collect();
    SearchPage {
        total: Some(results.len() as u32),
        results,
    }
}

/// What the per-file lookup found: one record ID (or nothing) per file.
#[derive(Debug, Default, PartialEq)]
pub struct FilesFound {
    pub ids: Vec<Option<String>>,
    pub synced: usize,
    pub failed: usize,
}

/// The single result that stands for the selected files: `files:12,,34`
/// lists each file's record in file list order.
pub fn files_result(found: &FilesFound) -> SearchResult {
    let count = found.ids.iter().flatten().count();
    let mut summary = format!(
        "Lyrics for {count} of {} selected file{}",
        found.ids.len(),
        if found.ids.len() == 1 { "" } else { "s" }
    );
    if found.synced > 0 {
        summary.push_str(&format!(", {} synchronized", found.synced));
    }
    if found.failed > 0 {
        summary.push_str(&format!(
            "; {} lookup{} failed",
            found.failed,
            if found.failed == 1 { "" } else { "s" }
        ));
    }
    SearchResult {
        id: format!(
            "{FILES}{}",
            found
                .ids
                .iter()
                .map(|id| id.clone().unwrap_or_default())
                .collect::<Vec<_>>()
                .join(",")
        ),
        title: summary,
        artist: "Selected files".into(),
        track_count: Some(found.ids.len() as u32),
        format: if count > 0 { "Found" } else { "None found" }.into(),
        ..Default::default()
    }
}

pub fn file_ids(input: &str) -> Option<Vec<Option<String>>> {
    let list = input.strip_prefix(FILES)?;
    Some(
        list.split(',')
            .map(|id| {
                let id = id.trim();
                (!id.is_empty() && id.chars().all(|c| c.is_ascii_digit())).then(|| id.to_owned())
            })
            .collect(),
    )
}

/// The text of LRC lines without their time stamps: `[mm:ss.xx]` line tags
/// (several per line allowed) and enhanced LRC's `<mm:ss.xx>` word tags go,
/// and ID tag lines such as `[ar:Artist]` are dropped.
pub fn strip_timestamps(lrc: &str) -> String {
    fn time_tag(tag: &str) -> bool {
        let mut parts = tag.splitn(2, ':');
        let minutes = parts.next().unwrap_or_default();
        let seconds = parts.next().unwrap_or_default();
        !minutes.is_empty()
            && minutes.chars().all(|c| c.is_ascii_digit())
            && !seconds.is_empty()
            && seconds
                .chars()
                .all(|c| c.is_ascii_digit() || c == '.' || c == ':')
    }
    let mut lines = Vec::new();
    for line in lrc.lines() {
        let mut rest = line.trim();
        let mut timed = false;
        while let Some(tag) = rest.strip_prefix('[').and_then(|r| r.split_once(']')) {
            if time_tag(tag.0) {
                timed = true;
                rest = tag.1.trim_start();
            } else {
                break;
            }
        }
        if !timed && rest.starts_with('[') && rest.ends_with(']') {
            continue; // [ar:…], [ti:…], [length:…] and other ID tags
        }
        let mut text = String::new();
        let mut words = rest;
        while let Some(start) = words.find('<') {
            match words[start + 1..].split_once('>') {
                Some((tag, after)) if time_tag(tag) => {
                    text.push_str(&words[..start]);
                    words = after;
                }
                _ => {
                    text.push_str(&words[..=start]);
                    words = &words[start + 1..];
                }
            }
        }
        text.push_str(words);
        lines.push(text.trim().to_owned());
    }
    lines.join("\n").trim().to_owned()
}

/// UNSYNCEDLYRICS: the LRC text with time stamps when the option asks for
/// it, else the plain lyrics, or the LRC lines without their time stamps
/// when LRCLIB has only synchronized lyrics.
fn lyrics_fields(record: &Value, prefer_synced: bool) -> Fields {
    let mut fields = Fields::new();
    let synced = lyrics(record, "syncedLyrics");
    let plain = lyrics(record, "plainLyrics");
    let chosen = if prefer_synced && !synced.is_empty() {
        synced
    } else if plain.trim().is_empty() {
        strip_timestamps(&synced)
    } else {
        plain
    };
    put_one(&mut fields, "UNSYNCEDLYRICS", chosen);
    fields
}

/// Lyrics for the selected files: one track per file, in file list order,
/// numbered like the file so matching pairs each file with its own lyrics.
pub fn files_release(
    files: &[FileSignature],
    records: &[Option<Value>],
    prefer_synced: bool,
) -> SourceRelease {
    let number = |value: &str| {
        value
            .split('/')
            .next()
            .and_then(|n| n.trim().parse::<u32>().ok())
    };
    let tracks = files
        .iter()
        .enumerate()
        .map(|(index, file)| {
            let record = records.get(index).and_then(Option::as_ref);
            SourceTrack {
                position: (index + 1).to_string(),
                disc: number(&file.disc).unwrap_or(1).max(1),
                number: number(&file.track).unwrap_or(index as u32 + 1),
                title: record
                    .map(|r| text(r, "trackName"))
                    .filter(|t| !t.is_empty())
                    .unwrap_or_else(|| file.title.clone()),
                artist: record
                    .map(|r| text(r, "artistName"))
                    .unwrap_or_else(|| file.artist.clone()),
                length: record.and_then(|r| r.get("duration")?.as_f64()),
                fields: record
                    .map(|r| lyrics_fields(r, prefer_synced))
                    .unwrap_or_default(),
            }
        })
        .collect();
    SourceRelease {
        preview_url: None,
        provider: Provider::Lrclib,
        id: String::new(),
        url: String::new(),
        title: "Lyrics".into(),
        artist: "Selected files".into(),
        fields: Fields::new(),
        tracks,
        covers: Vec::new(),
        more_covers: false,
    }
}

/// One lyrics record as a release with one track.
pub fn record_release(record: &Value, prefer_synced: bool) -> Result<SourceRelease, String> {
    let id = text(record, "id");
    if id.is_empty() {
        return Err("LRCLIB returned an unexpected record.".into());
    }
    let title = text(record, "trackName");
    let artist = text(record, "artistName");
    Ok(SourceRelease {
        preview_url: None,
        provider: Provider::Lrclib,
        url: String::new(),
        title: title.clone(),
        artist: artist.clone(),
        fields: Fields::new(),
        tracks: vec![SourceTrack {
            position: "1".into(),
            disc: 1,
            number: 1,
            title,
            artist,
            length: record.get("duration").and_then(Value::as_f64),
            fields: lyrics_fields(record, prefer_synced),
        }],
        covers: Vec::new(),
        more_covers: false,
        id,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record() -> Value {
        serde_json::from_str(include_str!("fixtures/lrclib-record.json")).unwrap()
    }
    fn file(title: &str, track: &str) -> FileSignature {
        FileSignature {
            path: format!("/music/{title}.mp3"),
            title: title.into(),
            artist: "Daft Punk".into(),
            album: "Discovery".into(),
            track: track.into(),
            length: 320.4,
            ..Default::default()
        }
    }

    #[test]
    fn lookups_use_the_file_signature() {
        let url = get_url(&file("One More Time", "1")).unwrap();
        assert_eq!(url, "https://lrclib.net/api/get?track_name=One+More+Time&artist_name=Daft+Punk&album_name=Discovery&duration=320");
        assert!(get_url(&FileSignature {
            title: "x".into(),
            ..Default::default()
        })
        .is_none());
        assert_eq!(
            file_search_url(&file("One More Time", "1")).unwrap(),
            "https://lrclib.net/api/search?track_name=One+More+Time&artist_name=Daft+Punk"
        );
        assert!(text_search_url(" ").is_err());
        assert_eq!(record_id(" 250327 ").unwrap(), "250327");
        assert!(record_id("12a").is_err());
    }

    #[test]
    fn search_fallback_takes_the_closest_length() {
        let hits = serde_json::json!([
            {"id": 1, "duration": 330.0, "plainLyrics": "far"},
            {"id": 2, "duration": 322.0, "plainLyrics": "close"},
            {"id": 3, "duration": 320.0, "plainLyrics": ""},
        ]);
        assert_eq!(best_hit(&hits, &file("x", "")).unwrap()["id"], 2);
        let mut unknown = file("x", "");
        unknown.length = 0.0;
        assert_eq!(best_hit(&hits, &unknown).unwrap()["id"], 1);
        assert!(best_hit(
            &serde_json::json!([{"id": 9, "duration": 10.0, "plainLyrics": "a"}]),
            &file("x", "")
        )
        .is_none());
    }

    #[test]
    fn files_result_round_trips_record_ids() {
        let found = FilesFound {
            ids: vec![Some("250327".into()), None, Some("7".into())],
            synced: 1,
            failed: 1,
        };
        let result = files_result(&found);
        assert_eq!(result.id, "files:250327,,7");
        assert!(result.title.contains("2 of 3") && result.title.contains("1 synchronized"));
        assert!(result.title.contains("1 lookup failed"));
        assert_eq!(file_ids(&result.id).unwrap(), found.ids);
        assert_eq!(file_ids("250327"), None);
    }

    #[test]
    fn synced_only_lyrics_are_written_without_time_stamps() {
        let lrc = "[ar:Daft Punk]\n[ti:One More Time]\n[00:30.23] One more time\n[00:33.69]\n[00:35.00][01:10.00] We're <00:35.50>gonna celebrate\n[00:40.00] <not a tag> stays";
        assert_eq!(
            strip_timestamps(lrc),
            "One more time\n\nWe're gonna celebrate\n<not a tag> stays"
        );
        let synced_only =
            serde_json::json!({"id": 6, "trackName": "T", "plainLyrics": "", "syncedLyrics": lrc});
        assert_eq!(
            record_release(&synced_only, false).unwrap().tracks[0].fields["UNSYNCEDLYRICS"],
            ["One more time\n\nWe're gonna celebrate\n<not a tag> stays"]
        );
        let kept = record_release(&synced_only, true).unwrap();
        assert!(kept.tracks[0].fields["UNSYNCEDLYRICS"][0].contains("[00:30.23] One more time"));
        let missing_plain = serde_json::json!({"id": 7, "syncedLyrics": "[00:01.00] Only line"});
        assert_eq!(
            record_release(&missing_plain, false).unwrap().tracks[0].fields["UNSYNCEDLYRICS"],
            ["Only line"]
        );
    }

    #[test]
    fn plain_or_synced_lyrics_become_unsyncedlyrics() {
        let record = record();
        let files = [file("One More Time", "3"), file("Unknown", "")];
        let release = files_release(&files, &[Some(record.clone()), None], false);
        assert_eq!(release.tracks.len(), 2);
        let first = &release.tracks[0];
        assert_eq!((first.number, first.disc), (3, 1));
        assert_eq!(
            first.fields["UNSYNCEDLYRICS"],
            ["One more time\nWe're gonna celebrate"]
        );
        assert!(release.tracks[1].fields.is_empty());
        assert_eq!(release.tracks[1].number, 2);
        let synced = record_release(&record, true).unwrap();
        assert!(
            synced.tracks[0].fields["UNSYNCEDLYRICS"][0].starts_with("[00:30.23] One more time")
        );
        assert_eq!(synced.id, "250327");
        // Without synced lyrics the plain text is used.
        let plain_only = serde_json::json!({"id": 5, "trackName": "T", "plainLyrics": "words"});
        assert_eq!(
            record_release(&plain_only, true).unwrap().tracks[0].fields["UNSYNCEDLYRICS"],
            ["words"]
        );
        let page = parse_search(&serde_json::json!([record]));
        assert_eq!(page.results[0].format, "Synced");
        assert_eq!(remembered("250327").unwrap()["trackName"], "One More Time");
    }
}
