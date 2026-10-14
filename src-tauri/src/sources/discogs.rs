//! Discogs API v2 requests and response parsing.
use super::model::*;
use serde_json::Value;
use tag_core::Fields;

pub const API: &str = "https://api.discogs.com";
const COMPOSER_ROLES: [&str; 5] = [
    "composed by",
    "written-by",
    "written by",
    "music by",
    "songwriter",
];

fn text(value: &Value, key: &str) -> String {
    match value.get(key) {
        Some(Value::String(s)) => s.trim().to_owned(),
        Some(Value::Number(n)) => n.to_string(),
        _ => String::new(),
    }
}
fn list<'a>(value: &'a Value, key: &str) -> &'a [Value] {
    value
        .get(key)
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or_default()
}
fn strings(value: &Value, key: &str) -> Vec<String> {
    list(value, key)
        .iter()
        .filter_map(Value::as_str)
        .map(|s| s.trim().to_owned())
        .filter(|s| !s.is_empty())
        .collect()
}

/// Discogs disambiguates duplicate names as `Name (2)`; tags use the plain name.
pub fn clean_name(name: &str) -> String {
    let name = name.trim().trim_end_matches('*').trim_end();
    if let Some(open) = name.rfind(" (") {
        let suffix = &name[open + 2..];
        if suffix.ends_with(')')
            && suffix.len() > 1
            && suffix[..suffix.len() - 1]
                .chars()
                .all(|c| c.is_ascii_digit())
        {
            return name[..open].to_owned();
        }
    }
    name.to_owned()
}

/// Join Discogs artists with their join words: `A & B`, `A, B`, `A Feat. B`.
fn artists(values: &[Value]) -> String {
    let mut out = String::new();
    for (index, artist) in values.iter().enumerate() {
        let name = match text(artist, "anv") {
            anv if anv.is_empty() => clean_name(&text(artist, "name")),
            anv => anv,
        };
        out.push_str(&name);
        if index + 1 < values.len() {
            match text(artist, "join").as_str() {
                "" => out.push_str(", "),
                "," => out.push_str(", "),
                join => {
                    out.push(' ');
                    out.push_str(join);
                    out.push(' ');
                }
            }
        }
    }
    out.trim().to_owned()
}

pub fn search_query(query: &SearchQuery) -> Result<String, String> {
    let mut params: Vec<(&str, String)> =
        vec![("type", "release".into()), ("per_page", "50".into())];
    let trimmed = |value: &str| value.trim().to_owned();
    match query.mode {
        SearchMode::ArtistAlbum => {
            if !query.artist.trim().is_empty() {
                params.push(("artist", trimmed(&query.artist)));
            }
            if !query.album.trim().is_empty() {
                params.push(("release_title", trimmed(&query.album)));
            }
        }
        SearchMode::Text if !query.text.trim().is_empty() => {
            params.push(("q", trimmed(&query.text)))
        }
        SearchMode::Barcode if !query.text.trim().is_empty() => params.push((
            "barcode",
            query.text.chars().filter(|c| !c.is_whitespace()).collect(),
        )),
        SearchMode::CatalogNumber if !query.text.trim().is_empty() => {
            params.push(("catno", trimmed(&query.text)))
        }
        _ => {}
    }
    if params.len() == 2 {
        return Err("Enter an artist, album or search text.".into());
    }
    reqwest::Url::parse_with_params(&format!("{API}/database/search"), &params)
        .map(String::from)
        .map_err(|e| e.to_string())
}

/// Accept `12345`, `r12345`, `[r12345]` or any discogs.com release address.
pub fn release_id(input: &str) -> Result<String, String> {
    let input = input.trim();
    let bare = input.trim_start_matches('[').trim_end_matches(']');
    let bare = bare
        .strip_prefix('r')
        .or_else(|| bare.strip_prefix('R'))
        .unwrap_or(bare);
    if !bare.is_empty() && bare.chars().all(|c| c.is_ascii_digit()) {
        let id = bare.trim_start_matches('0');
        return if id.is_empty() {
            Err("Enter a Discogs release ID.".into())
        } else {
            Ok(id.to_owned())
        };
    }
    let lower = input.to_ascii_lowercase();
    if lower.contains("discogs.com") && lower.contains("/master/") {
        return Err("Master release addresses are not supported. Open one release of the master on Discogs and copy its address.".into());
    }
    if lower.contains("discogs.com") {
        if let Some(rest) = lower.split("/release/").nth(1) {
            let id: String = rest.chars().take_while(char::is_ascii_digit).collect();
            if !id.is_empty() {
                return Ok(id);
            }
        }
    }
    Err("Enter a Discogs release ID or release address.".into())
}

pub fn release_url(id: &str) -> String {
    format!("{API}/releases/{id}")
}

pub fn parse_search(body: &Value) -> Vec<SearchResult> {
    list(body, "results")
        .iter()
        .filter(|result| matches!(text(result, "type").as_str(), "" | "release"))
        .filter_map(|result| {
            let id = text(result, "id");
            if id.is_empty() || !id.chars().all(|c| c.is_ascii_digit()) {
                return None;
            }
            let combined = text(result, "title");
            let (artist, title) = match combined.split_once(" - ") {
                Some((artist, title)) => (clean_name(artist), title.trim().to_owned()),
                None => (String::new(), combined.clone()),
            };
            let thumb = text(result, "thumb");
            let uri = text(result, "uri");
            let mut labels: Vec<String> = Vec::new();
            for label in strings(result, "label").iter().map(|l| clean_name(l)) {
                if !labels.contains(&label) {
                    labels.push(label);
                }
            }
            Some(SearchResult {
                title,
                artist,
                year: year(&text(result, "year")),
                country: text(result, "country"),
                label: labels.join(", "),
                catalog_number: match text(result, "catno") {
                    c if c.eq_ignore_ascii_case("none") => String::new(),
                    c => c,
                },
                format: strings(result, "format").join(", "),
                track_count: None,
                score: None,
                barcode: strings(result, "barcode")
                    .into_iter()
                    .next()
                    .unwrap_or_default(),
                thumb: (!thumb.is_empty()).then_some(thumb),
                url: if uri.starts_with('/') {
                    format!("https://www.discogs.com{uri}")
                } else if uri.is_empty() {
                    format!("https://www.discogs.com/release/{id}")
                } else {
                    uri
                },
                id,
            })
        })
        .collect()
}

fn seconds(duration: &str) -> Option<f64> {
    let parts: Vec<u64> = duration
        .trim()
        .split(':')
        .map(|p| p.trim().parse().ok())
        .collect::<Option<_>>()?;
    if parts.is_empty() || parts.len() > 3 {
        return None;
    }
    Some(parts.iter().fold(0u64, |total, part| total * 60 + part) as f64)
}

/// Disc number for positions such as `2-03`, `CD2-3`, `2.3`; vinyl sides and
/// plain numbers belong to the current disc.
fn disc(position: &str) -> Option<u32> {
    let position = position.trim();
    let start = position.find(|c: char| c.is_ascii_digit())?;
    let prefix = &position[..start];
    if !prefix.chars().all(|c| c.is_ascii_alphabetic()) || prefix.len() == 1 || prefix.len() > 4 {
        return None;
    }
    let rest = &position[start..];
    let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
    let after = &rest[digits.len()..];
    if after.starts_with(['-', '.'])
        && after[1..]
            .chars()
            .next()
            .is_some_and(|c| c.is_ascii_alphanumeric())
    {
        digits.parse().ok()
    } else {
        None
    }
}

fn composer_role(role: &str) -> bool {
    role.split(',')
        .map(|r| {
            r.split('[')
                .next()
                .unwrap_or_default()
                .trim()
                .to_ascii_lowercase()
        })
        .any(|r| COMPOSER_ROLES.contains(&r.as_str()))
}

/// Whether a release-level credit's `tracks` text (`1 to 3, A5`) names this track.
fn credit_applies(tracks: &str, position: &str, index: usize, positions: &[String]) -> bool {
    let tracks = tracks.trim();
    if tracks.is_empty() {
        return true;
    }
    let find = |p: &str| {
        positions
            .iter()
            .position(|candidate| candidate.eq_ignore_ascii_case(p.trim()))
    };
    tracks.split(',').any(|item| match item.split_once(" to ") {
        Some((from, to)) => {
            matches!((find(from), find(to)), (Some(a), Some(b)) if a <= index && index <= b)
        }
        None => item.trim().eq_ignore_ascii_case(position),
    })
}

pub fn parse_release(body: &Value) -> Result<SourceRelease, String> {
    let id = text(body, "id");
    if id.is_empty() || !id.chars().all(|c| c.is_ascii_digit()) {
        return Err("Discogs returned an unexpected release.".into());
    }
    let title = text(body, "title");
    let artist = artists(list(body, "artists"));
    let mut fields = Fields::new();
    put_one(&mut fields, "ALBUM", title.clone());
    put_one(&mut fields, "ALBUMARTIST", artist.clone());
    let release_year = match text(body, "year") {
        y if y == "0" || y.is_empty() => year(&text(body, "released")),
        y => year(&y),
    };
    put_one(&mut fields, "YEAR", release_year);
    put(&mut fields, "GENRE", strings(body, "genres"));
    put(&mut fields, "STYLE", strings(body, "styles"));
    let labels = list(body, "labels");
    put(
        &mut fields,
        "PUBLISHER",
        labels
            .iter()
            .map(|l| clean_name(&text(l, "name")))
            .collect(),
    );
    put(
        &mut fields,
        "CATALOGNUMBER",
        labels
            .iter()
            .map(|l| text(l, "catno"))
            .filter(|c| !c.eq_ignore_ascii_case("none"))
            .collect(),
    );
    if let Some(barcode) = list(body, "identifiers")
        .iter()
        .find(|i| text(i, "type") == "Barcode")
    {
        put_one(
            &mut fields,
            "BARCODE",
            text(barcode, "value")
                .chars()
                .filter(|c| !c.is_whitespace())
                .collect::<String>(),
        );
    }
    put_one(&mut fields, "DISCOGS_RELEASE_ID", id.clone());
    if let Some(master) = body
        .get("master_id")
        .and_then(Value::as_u64)
        .filter(|m| *m > 0)
    {
        put_one(&mut fields, "DISCOGS_MASTER_ID", master.to_string());
    }

    // Flatten index tracks to one entry and drop headings.
    let entries: Vec<&Value> = list(body, "tracklist")
        .iter()
        .filter(|t| !matches!(text(t, "type_").as_str(), "heading"))
        .collect();
    let positions: Vec<String> = entries
        .iter()
        .map(|t| match text(t, "position") {
            p if p.is_empty() => list(t, "sub_tracks")
                .first()
                .map(|s| text(s, "position"))
                .unwrap_or_default(),
            p => p,
        })
        .collect();
    let release_composers: Vec<(&Value, String)> = list(body, "extraartists")
        .iter()
        .filter(|c| composer_role(&text(c, "role")))
        .map(|c| (c, text(c, "tracks")))
        .collect();
    let mut tracks = Vec::new();
    let mut current_disc = 1;
    for (index, entry) in entries.iter().enumerate() {
        let position = positions[index].clone();
        if let Some(disc) = disc(&position) {
            current_disc = disc;
        }
        let track_title = text(entry, "title");
        let track_artist = match artists(list(entry, "artists")) {
            a if a.is_empty() => artist.clone(),
            a => a,
        };
        let mut credits: Vec<&Value> = list(entry, "extraartists")
            .iter()
            .filter(|c| composer_role(&text(c, "role")))
            .collect();
        credits.extend(
            release_composers
                .iter()
                .filter(|(_, tracks)| credit_applies(tracks, &position, index, &positions))
                .map(|(c, _)| *c),
        );
        let mut track_fields = Fields::new();
        put_one(&mut track_fields, "TITLE", track_title.clone());
        put_one(&mut track_fields, "ARTIST", track_artist.clone());
        put(
            &mut track_fields,
            "COMPOSER",
            credits
                .iter()
                .map(|c| match text(c, "anv") {
                    a if a.is_empty() => clean_name(&text(c, "name")),
                    a => a,
                })
                .collect(),
        );
        tracks.push(SourceTrack {
            position,
            disc: current_disc,
            number: 0,
            title: track_title,
            artist: track_artist,
            length: seconds(&text(entry, "duration")),
            fields: track_fields,
        });
    }
    if tracks.is_empty() {
        return Err("This Discogs release has no tracks.".into());
    }
    number_tracks(&mut tracks);
    let covers = list(body, "images")
        .iter()
        .filter_map(|image| {
            let url = text(image, "uri");
            url.starts_with("https://").then(|| SourceCover {
                thumb: Some(text(image, "uri150")).filter(|t| !t.is_empty()),
                kind: if text(image, "type") == "primary" {
                    "Front".into()
                } else {
                    "Other".into()
                },
                width: image.get("width").and_then(Value::as_u64).map(|w| w as u32),
                height: image
                    .get("height")
                    .and_then(Value::as_u64)
                    .map(|h| h as u32),
                url,
            })
        })
        .collect();
    let uri = text(body, "uri");
    Ok(SourceRelease {
        provider: Provider::Discogs,
        more_covers: false,
        url: if uri.starts_with("https://") {
            uri
        } else {
            format!("https://www.discogs.com/release/{id}")
        },
        id,
        title,
        artist,
        fields,
        tracks,
        covers,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn names_drop_numeric_disambiguation_only() {
        assert_eq!(clean_name("Jazzanova (2)"), "Jazzanova");
        assert_eq!(clean_name("Artist*"), "Artist");
        assert_eq!(clean_name("Band (UK)"), "Band (UK)");
        assert_eq!(clean_name("Song (1999)"), "Song");
    }

    #[test]
    fn release_ids_accept_numbers_tags_and_release_urls() {
        assert_eq!(release_id("1225").unwrap(), "1225");
        assert_eq!(release_id(" [r1225] ").unwrap(), "1225");
        assert_eq!(
            release_id("https://www.discogs.com/release/1225-Jazzanova-Jazzanova-EP").unwrap(),
            "1225"
        );
        assert_eq!(
            release_id("https://www.discogs.com/Jazzanova-Jazzanova-EP/release/1225").unwrap(),
            "1225"
        );
        assert!(
            release_id("https://www.discogs.com/master/56255-Jazzanova-EP")
                .unwrap_err()
                .contains("Master")
        );
        assert!(release_id("abc").is_err());
        assert!(release_id("0").is_err());
    }

    #[test]
    fn search_parameters_follow_the_selected_mode() {
        let query = |mode, text: &str| SearchQuery {
            mode,
            artist: "Jazzanova".into(),
            album: "EP".into(),
            text: text.into(),
        };
        let url = search_query(&query(SearchMode::ArtistAlbum, "")).unwrap();
        assert!(
            url.contains("type=release")
                && url.contains("artist=Jazzanova")
                && url.contains("release_title=EP")
        );
        assert!(search_query(&query(SearchMode::Barcode, "4 018289 000617"))
            .unwrap()
            .contains("barcode=4018289000617"));
        assert!(search_query(&query(SearchMode::CatalogNumber, "JCR 004"))
            .unwrap()
            .contains("catno=JCR+004"));
        assert!(search_query(&query(SearchMode::Text, " ")).is_err());
    }

    #[test]
    fn search_results_split_titles_and_skip_non_releases() {
        let body = json!({"results": [
            {"id": 8268, "type": "release", "title": "Jazzanova (2) - Jazzanova EP", "year": "1997", "country": "Germany",
             "label": ["Jazzanova Records", "Jazzanova Records"], "catno": "none", "format": ["Vinyl", "12\""], "barcode": ["EP-1 A"],
             "thumb": "", "uri": "/release/8268-Jazzanova-Jazzanova-EP"},
            {"id": 56255, "type": "master", "title": "Jazzanova - Jazzanova EP"}
        ]});
        let results = parse_search(&body);
        assert_eq!(results.len(), 1);
        let r = &results[0];
        assert_eq!(
            (r.artist.as_str(), r.title.as_str(), r.year.as_str()),
            ("Jazzanova", "Jazzanova EP", "1997")
        );
        assert_eq!(
            (
                r.label.as_str(),
                r.catalog_number.as_str(),
                r.format.as_str()
            ),
            ("Jazzanova Records", "", "Vinyl, 12\"")
        );
        assert_eq!(r.thumb, None);
        assert_eq!(
            r.url,
            "https://www.discogs.com/release/8268-Jazzanova-Jazzanova-EP"
        );
    }

    #[test]
    fn positions_and_durations_parse_common_shapes() {
        assert_eq!(disc("2-03"), Some(2));
        assert_eq!(disc("CD2-3"), Some(2));
        assert_eq!(disc("1.5"), Some(1));
        assert_eq!(disc("A1"), None);
        assert_eq!(disc("12"), None);
        assert_eq!(disc("1-A"), Some(1));
        assert_eq!(seconds("7:17"), Some(437.0));
        assert_eq!(seconds("1:02:03"), Some(3723.0));
        assert_eq!(seconds(""), None);
    }

    #[test]
    fn credit_ranges_cover_listed_positions() {
        let positions: Vec<String> = ["A1", "A2", "B1", "B2"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        assert!(credit_applies("", "B2", 3, &positions));
        assert!(credit_applies("A2 to B1", "B1", 2, &positions));
        assert!(!credit_applies("A2 to B1", "B2", 3, &positions));
        assert!(credit_applies("A1, B2", "B2", 3, &positions));
    }

    fn release_fixture() -> Value {
        json!({
            "id": 1225, "title": "Jazzanova EP", "year": 0, "released": "1998-05-00", "country": "UK & Europe", "master_id": 56255,
            "uri": "https://www.discogs.com/release/1225-Jazzanova-Jazzanova-EP",
            "artists": [{"name": "Jazzanova (2)", "anv": "", "join": "&"}, {"name": "Guest", "anv": "G.", "join": ""}],
            "genres": ["Electronic", "Jazz"], "styles": ["Future Jazz"],
            "labels": [{"name": "Compost (3)", "catno": "JCR 004"}, {"name": "Compost (3)", "catno": "none"}],
            "identifiers": [{"type": "Label Code", "value": "LC 1661"}, {"type": "Barcode", "value": "4 018289 000617"}],
            "extraartists": [{"name": "Writer", "anv": "", "role": "Written-By", "tracks": "1-2"}, {"name": "Player", "role": "Oboe", "tracks": ""}],
            "tracklist": [
                {"position": "", "type_": "heading", "title": "Disc one"},
                {"position": "1-1", "type_": "track", "title": "Caravelle", "duration": "7:17",
                 "extraartists": [{"name": "Composer (2)", "anv": "", "role": "Music By [Arranged]"}]},
                {"position": "1-2", "type_": "track", "title": "Konclave", "duration": "8:21",
                 "artists": [{"name": "Other", "anv": "", "join": "Feat."}, {"name": "Singer", "anv": "", "join": ""}]},
                {"position": "", "type_": "index", "title": "Suite", "sub_tracks": [{"position": "2-1", "title": "Part 1"}, {"position": "2-2", "title": "Part 2"}]},
                {"position": "2-3", "type_": "track", "title": "Fedime's Flight", "duration": ""}
            ],
            "images": [{"type": "primary", "uri": "https://i.discogs.com/a.jpeg", "uri150": "https://i.discogs.com/a150.jpeg", "width": 600, "height": 518},
                       {"type": "secondary", "uri": "https://i.discogs.com/b.jpeg", "uri150": ""},
                       {"type": "secondary", "uri": "http://insecure.example/c.jpeg"}]
        })
    }

    #[test]
    fn release_maps_fields_discs_index_tracks_credits_and_images() {
        let release = parse_release(&release_fixture()).unwrap();
        let f = |key: &str| release.fields.get(key).cloned().unwrap_or_default();
        assert_eq!(f("ALBUMARTIST"), ["Jazzanova & G."]);
        assert_eq!(f("YEAR"), ["1998"], "falls back to the released date");
        assert_eq!(f("GENRE"), ["Electronic", "Jazz"]);
        assert_eq!(f("STYLE"), ["Future Jazz"]);
        assert_eq!(f("PUBLISHER"), ["Compost"]);
        assert_eq!(f("CATALOGNUMBER"), ["JCR 004"]);
        assert_eq!(f("BARCODE"), ["4018289000617"]);
        assert_eq!(f("DISCOGS_RELEASE_ID"), ["1225"]);
        assert_eq!(f("DISCOGS_MASTER_ID"), ["56255"]);
        let summary: Vec<_> = release
            .tracks
            .iter()
            .map(|t| {
                (
                    t.position.as_str(),
                    t.title.as_str(),
                    t.fields["TRACK"][0].as_str(),
                    t.fields["DISCNUMBER"][0].as_str(),
                )
            })
            .collect();
        assert_eq!(
            summary,
            [
                ("1-1", "Caravelle", "1/2", "1/2"),
                ("1-2", "Konclave", "2/2", "1/2"),
                ("2-1", "Suite", "1/2", "2/2"),
                ("2-3", "Fedime's Flight", "2/2", "2/2")
            ]
        );
        assert_eq!(
            release.tracks[0].fields["COMPOSER"],
            ["Composer"],
            "the release credit's range text does not name 1-1"
        );
        assert_eq!(release.tracks[0].fields["ARTIST"], ["Jazzanova & G."]);
        assert_eq!(release.tracks[1].fields["ARTIST"], ["Other Feat. Singer"]);
        assert_eq!(release.tracks[0].length, Some(437.0));
        assert_eq!(release.tracks[3].length, None);
        assert_eq!(release.covers.len(), 2, "non-https images are ignored");
        assert_eq!(
            (release.covers[0].kind.as_str(), release.covers[0].width),
            ("Front", Some(600))
        );
        assert_eq!(release.covers[1].thumb, None);
    }

    #[test]
    fn release_level_composers_without_ranges_apply_to_every_track() {
        let mut body = release_fixture();
        body["extraartists"] =
            json!([{"name": "Everyone (4)", "role": "Composed By", "tracks": ""}]);
        let release = parse_release(&body).unwrap();
        assert!(release
            .tracks
            .iter()
            .all(|t| t.fields["COMPOSER"].contains(&"Everyone".to_string())));
    }
}
