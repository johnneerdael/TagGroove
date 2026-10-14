//! MusicBrainz web service (JSON, ws/2) requests and response parsing.
use super::model::*;
use serde_json::Value;
use tag_core::Fields;

pub const API: &str = "https://musicbrainz.org/ws/2";
const RELEASE_INCLUDES: &str = "artist-credits+labels+recordings+release-groups+genres+isrcs+recording-level-rels+work-rels+work-level-rels+artist-rels";

/// Cover Art Archive image size preference (Mp3tag offers the same choices).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum CoverSize {
    #[serde(rename = "250")]
    Small,
    #[default]
    #[serde(rename = "500")]
    Medium,
    #[serde(rename = "1200")]
    Large,
    #[serde(rename = "original")]
    Original,
}

fn text(value: &Value, key: &str) -> String {
    value
        .get(key)
        .and_then(Value::as_str)
        .unwrap_or_default()
        .trim()
        .to_owned()
}
fn list<'a>(value: &'a Value, key: &str) -> &'a [Value] {
    value
        .get(key)
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or_default()
}

fn lucene(value: &str) -> String {
    let escaped: String = value
        .trim()
        .chars()
        .flat_map(|c| match c {
            '"' | '\\' => vec!['\\', c],
            _ => vec![c],
        })
        .collect();
    format!("\"{escaped}\"")
}

pub fn search_query(query: &SearchQuery) -> Result<String, String> {
    let lucene_query = match query.mode {
        SearchMode::ArtistAlbum => {
            let mut parts = Vec::new();
            if !query.album.trim().is_empty() {
                parts.push(format!("release:{}", lucene(&query.album)));
            }
            if !query.artist.trim().is_empty() {
                parts.push(format!("artist:{}", lucene(&query.artist)));
            }
            parts.join(" AND ")
        }
        SearchMode::Text => query.text.trim().to_owned(),
        SearchMode::Barcode => {
            let digits: String = query.text.chars().filter(|c| !c.is_whitespace()).collect();
            if digits.is_empty() {
                String::new()
            } else {
                format!("barcode:{}", lucene(&digits))
            }
        }
        SearchMode::CatalogNumber if !query.text.trim().is_empty() => {
            format!("catno:{}", lucene(&query.text))
        }
        SearchMode::CatalogNumber => String::new(),
    };
    if lucene_query.is_empty() {
        return Err("Enter an artist, album or search text.".into());
    }
    let url = reqwest::Url::parse_with_params(
        &format!("{API}/release/"),
        &[
            ("query", lucene_query.as_str()),
            ("fmt", "json"),
            ("limit", "50"),
        ],
    )
    .map_err(|e| e.to_string())?;
    Ok(url.into())
}

/// Accept a bare MBID or any musicbrainz.org release URL that contains one.
pub fn release_id(input: &str) -> Result<String, String> {
    let lower = input.trim().to_ascii_lowercase();
    if lower.contains("/release-group/") {
        return Err("Enter a release, not a release group: open one release on MusicBrainz and copy its address.".into());
    }
    lower
        .as_bytes()
        .windows(36)
        .filter_map(|window| std::str::from_utf8(window).ok())
        .find(|candidate| is_mbid(candidate))
        .map(str::to_owned)
        .ok_or_else(|| "Enter a MusicBrainz release ID (MBID) or release address.".into())
}
fn is_mbid(value: &str) -> bool {
    value.len() == 36
        && value.char_indices().all(|(i, c)| match i {
            8 | 13 | 18 | 23 => c == '-',
            _ => c.is_ascii_hexdigit(),
        })
}

pub fn release_url(id: &str) -> String {
    format!("{API}/release/{id}?inc={RELEASE_INCLUDES}&fmt=json")
}
pub fn cover_listing_url(id: &str) -> String {
    format!("https://coverartarchive.org/release/{id}")
}
pub fn front_thumb_url(id: &str) -> String {
    format!("https://coverartarchive.org/release/{id}/front-250")
}

/// Join an artist credit with its join phrases: `A feat. B`.
fn credit(value: &Value) -> String {
    list(value, "artist-credit")
        .iter()
        .map(|part| {
            let name = part
                .get("name")
                .and_then(Value::as_str)
                .or_else(|| part.pointer("/artist/name").and_then(Value::as_str))
                .unwrap_or_default();
            // Join phrases carry their own spacing (" & ", " feat. ").
            format!(
                "{name}{}",
                part.get("joinphrase")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
            )
        })
        .collect::<String>()
        .trim()
        .to_owned()
}
fn credit_ids(value: &Value) -> Vec<String> {
    list(value, "artist-credit")
        .iter()
        .map(|part| text(part.get("artist").unwrap_or(&Value::Null), "id"))
        .collect()
}
fn labels(release: &Value) -> (Vec<String>, Vec<String>) {
    let info = list(release, "label-info");
    let names = info
        .iter()
        .filter_map(|i| i.pointer("/label/name").and_then(Value::as_str))
        .map(str::to_owned)
        .collect();
    let catalog = info
        .iter()
        .map(|i| text(i, "catalog-number"))
        .filter(|c| !c.eq_ignore_ascii_case("[none]"))
        .collect();
    (names, catalog)
}
fn format_summary(media: &[Value]) -> String {
    let mut parts: Vec<(String, usize)> = Vec::new();
    for medium in media {
        let format = match text(medium, "format") {
            f if f.is_empty() => "(unknown)".to_owned(),
            f => f,
        };
        match parts.iter_mut().find(|(name, _)| *name == format) {
            Some((_, count)) => *count += 1,
            None => parts.push((format, 1)),
        }
    }
    parts
        .into_iter()
        .map(|(name, count)| {
            if count > 1 {
                format!("{count}×{name}")
            } else {
                name
            }
        })
        .collect::<Vec<_>>()
        .join(" + ")
}

pub fn parse_search(body: &Value) -> Vec<SearchResult> {
    list(body, "releases")
        .iter()
        .map(|release| {
            let (label, catalog) = labels(release);
            let id = text(release, "id");
            SearchResult {
                title: text(release, "title"),
                artist: credit(release),
                year: year(&text(release, "date")),
                country: text(release, "country"),
                label: dedupe(label).join(", "),
                catalog_number: dedupe(catalog).join(", "),
                format: format_summary(list(release, "media")),
                track_count: release
                    .get("track-count")
                    .and_then(Value::as_u64)
                    .map(|n| n as u32),
                score: release
                    .get("score")
                    .and_then(Value::as_u64)
                    .map(|n| n as u32),
                barcode: text(release, "barcode"),
                thumb: Some(front_thumb_url(&id)),
                url: format!("https://musicbrainz.org/release/{id}"),
                id,
            }
        })
        .filter(|result| is_mbid(&result.id))
        .collect()
}
fn dedupe(values: Vec<String>) -> Vec<String> {
    let mut unique = Vec::new();
    for value in values {
        if !value.is_empty() && !unique.contains(&value) {
            unique.push(value);
        }
    }
    unique
}

/// Highest voted genre of the release, falling back to its release group.
fn genre(release: &Value) -> Option<String> {
    let pick = |genres: &[Value]| {
        genres
            .iter()
            .filter_map(|g| {
                Some((
                    g.get("count").and_then(Value::as_i64).unwrap_or(0),
                    g.get("name")?.as_str()?,
                ))
            })
            .max_by(|a, b| a.0.cmp(&b.0).then(b.1.cmp(a.1)))
            .map(|(_, name)| title_case(name))
    };
    pick(list(release, "genres")).or_else(|| {
        pick(
            release
                .get("release-group")
                .map(|g| list(g, "genres"))
                .unwrap_or_default(),
        )
    })
}

fn work_relations(recording: &Value) -> impl Iterator<Item = &Value> {
    list(recording, "relations")
        .iter()
        .filter(|r| text(r, "target-type") == "work" && text(r, "type") == "performance")
        .filter_map(|r| r.get("work"))
}

pub fn parse_release(body: &Value, size: CoverSize) -> Result<SourceRelease, String> {
    let id = text(body, "id");
    if !is_mbid(&id) {
        return Err("MusicBrainz returned an unexpected release.".into());
    }
    let mut fields = Fields::new();
    let title = text(body, "title");
    let artist = credit(body);
    let group = body.get("release-group").unwrap_or(&Value::Null);
    let (label, catalog) = labels(body);
    put_one(&mut fields, "ALBUM", title.clone());
    put_one(&mut fields, "ALBUMARTIST", artist.clone());
    put_one(&mut fields, "YEAR", year(&text(body, "date")));
    if let Some(genre) = genre(body) {
        put_one(&mut fields, "GENRE", genre);
    }
    put(&mut fields, "PUBLISHER", label);
    put(&mut fields, "CATALOGNUMBER", catalog);
    put_one(&mut fields, "BARCODE", text(body, "barcode"));
    put_one(&mut fields, "MUSICBRAINZ_ALBUMID", id.clone());
    put_one(&mut fields, "MUSICBRAINZ_RELEASEGROUPID", text(group, "id"));
    put(&mut fields, "MUSICBRAINZ_ALBUMARTISTID", credit_ids(body));
    put_one(
        &mut fields,
        "MUSICBRAINZ_ALBUMRELEASECOUNTRY",
        text(body, "country"),
    );
    put_one(
        &mut fields,
        "MUSICBRAINZ_ALBUMSTATUS",
        text(body, "status").to_lowercase(),
    );
    put_one(
        &mut fields,
        "MUSICBRAINZ_ALBUMTYPE",
        text(group, "primary-type").to_lowercase(),
    );

    let mut tracks = Vec::new();
    for (medium_index, medium) in list(body, "media").iter().enumerate() {
        let disc = medium
            .get("position")
            .and_then(Value::as_u64)
            .unwrap_or(medium_index as u64 + 1) as u32;
        for track in list(medium, "tracks") {
            let recording = track.get("recording").unwrap_or(&Value::Null);
            let mut track_fields = Fields::new();
            let title = match text(track, "title") {
                t if t.is_empty() => text(recording, "title"),
                t => t,
            };
            let mut track_artist = credit(track);
            if track_artist.is_empty() {
                track_artist = credit(recording);
            }
            if track_artist.is_empty() {
                track_artist = artist.clone();
            }
            put_one(&mut track_fields, "TITLE", title.clone());
            put_one(&mut track_fields, "ARTIST", track_artist.clone());
            let composers = work_relations(recording)
                .flat_map(|work| list(work, "relations"))
                .filter(|r| text(r, "target-type") == "artist" && text(r, "type") == "composer")
                .filter_map(|r| {
                    let credited = text(r, "target-credit");
                    if credited.is_empty() {
                        r.pointer("/artist/name")
                            .and_then(Value::as_str)
                            .map(str::to_owned)
                    } else {
                        Some(credited)
                    }
                })
                .collect();
            put(&mut track_fields, "COMPOSER", composers);
            put(
                &mut track_fields,
                "ISRC",
                list(recording, "isrcs")
                    .iter()
                    .filter_map(Value::as_str)
                    .map(str::to_owned)
                    .collect(),
            );
            put_one(
                &mut track_fields,
                "MUSICBRAINZ_TRACKID",
                text(recording, "id"),
            );
            put_one(
                &mut track_fields,
                "MUSICBRAINZ_RELEASETRACKID",
                text(track, "id"),
            );
            let ids = if track.get("artist-credit").is_some() {
                credit_ids(track)
            } else {
                credit_ids(recording)
            };
            put(&mut track_fields, "MUSICBRAINZ_ARTISTID", ids);
            put(
                &mut track_fields,
                "MUSICBRAINZ_WORKID",
                work_relations(recording).map(|w| text(w, "id")).collect(),
            );
            let length = track
                .get("length")
                .or_else(|| recording.get("length"))
                .and_then(Value::as_f64)
                .map(|ms| (ms / 1000.0).round());
            tracks.push(SourceTrack {
                position: match text(track, "number") {
                    n if n.is_empty() => track
                        .get("position")
                        .map(|p| p.to_string())
                        .unwrap_or_default(),
                    n => n,
                },
                disc,
                number: 0,
                title,
                artist: track_artist,
                length,
                fields: track_fields,
            });
        }
    }
    if tracks.is_empty() {
        return Err("This MusicBrainz release has no tracks.".into());
    }
    number_tracks(&mut tracks);
    // The archive listing is slow; offer the front cover now and list the rest on request.
    let archive = body.get("cover-art-archive").unwrap_or(&Value::Null);
    let covers: Vec<SourceCover> = archive
        .get("front")
        .and_then(Value::as_bool)
        .unwrap_or(false)
        .then(|| front_cover(&id, size))
        .into_iter()
        .collect();
    let more_covers =
        archive.get("count").and_then(Value::as_u64).unwrap_or(0) > covers.len() as u64;
    Ok(SourceRelease {
        provider: Provider::Musicbrainz,
        more_covers,
        url: format!("https://musicbrainz.org/release/{id}"),
        id,
        title,
        artist,
        fields,
        tracks,
        covers,
    })
}

pub fn front_cover(id: &str, size: CoverSize) -> SourceCover {
    let suffix = match size {
        CoverSize::Small => "-250",
        CoverSize::Medium => "-500",
        CoverSize::Large => "-1200",
        CoverSize::Original => "",
    };
    SourceCover {
        url: format!("https://coverartarchive.org/release/{id}/front{suffix}"),
        thumb: Some(front_thumb_url(id)),
        kind: "Front".into(),
        width: None,
        height: None,
    }
}

fn https(url: &str) -> String {
    match url.strip_prefix("http://") {
        Some(rest) => format!("https://{rest}"),
        None => url.to_owned(),
    }
}

/// Cover Art Archive listing: front images first, then the rest in archive order.
pub fn parse_covers(body: &Value, size: CoverSize) -> Vec<SourceCover> {
    let mut covers: Vec<(bool, SourceCover)> = list(body, "images")
        .iter()
        .filter_map(|image| {
            let original = image.get("image")?.as_str()?;
            let thumbnail = |key: &str| {
                image
                    .pointer(&format!("/thumbnails/{key}"))
                    .and_then(Value::as_str)
                    .map(https)
            };
            let url = match size {
                CoverSize::Small => thumbnail("250"),
                CoverSize::Medium => thumbnail("500"),
                CoverSize::Large => thumbnail("1200"),
                CoverSize::Original => None,
            }
            .unwrap_or_else(|| https(original));
            let types: Vec<&str> = list(image, "types")
                .iter()
                .filter_map(Value::as_str)
                .collect();
            let front = image.get("front").and_then(Value::as_bool).unwrap_or(false);
            Some((
                front,
                SourceCover {
                    url,
                    thumb: thumbnail("250").or_else(|| thumbnail("small")),
                    kind: if types.is_empty() {
                        if front {
                            "Front".into()
                        } else {
                            "Other".into()
                        }
                    } else {
                        types.join(", ")
                    },
                    width: None,
                    height: None,
                },
            ))
        })
        .collect();
    covers.sort_by_key(|(front, _)| !*front);
    covers.into_iter().map(|(_, cover)| cover).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    const MBID: &str = "d073287b-d1bd-4f11-a933-a4386f8cf701";

    #[test]
    fn search_queries_escape_lucene_and_encode_parameters() {
        let query = SearchQuery {
            mode: SearchMode::ArtistAlbum,
            artist: "Daft \"Punk\"".into(),
            album: "Discovery".into(),
            text: String::new(),
        };
        let url = search_query(&query).unwrap();
        let parsed = reqwest::Url::parse(&url).unwrap();
        let params: Vec<_> = parsed.query_pairs().collect();
        assert_eq!(
            params[0].1,
            "release:\"Discovery\" AND artist:\"Daft \\\"Punk\\\"\""
        );
        assert!(url.contains("fmt=json") && url.contains("limit=50"));
        let barcode = SearchQuery {
            mode: SearchMode::Barcode,
            artist: String::new(),
            album: String::new(),
            text: "0 724384 960629".into(),
        };
        assert!(search_query(&barcode)
            .unwrap()
            .contains("barcode%3A%22072438496062"));
        let empty = SearchQuery {
            mode: SearchMode::ArtistAlbum,
            artist: " ".into(),
            album: String::new(),
            text: "ignored".into(),
        };
        assert!(search_query(&empty).is_err());
    }

    #[test]
    fn release_ids_come_from_mbids_and_release_urls_only() {
        assert_eq!(release_id(MBID).unwrap(), MBID);
        assert_eq!(
            release_id(&format!(
                "https://musicbrainz.org/release/{}/discids",
                MBID.to_uppercase()
            ))
            .unwrap(),
            MBID
        );
        assert!(release_id(&format!("https://musicbrainz.org/release-group/{MBID}")).is_err());
        assert!(release_id("12345").is_err());
    }

    #[test]
    fn search_results_summarise_labels_formats_and_scores() {
        let body = json!({"releases": [{
            "id": MBID, "score": 98, "title": "Discovery", "date": "2001-02-26", "country": "FR", "barcode": "724384960629",
            "artist-credit": [{"name": "Daft Punk", "joinphrase": " & "}, {"name": "Friend", "artist": {"id": "x"}}],
            "label-info": [{"catalog-number": "8496062", "label": {"name": "Virgin"}}, {"catalog-number": "8496062", "label": {"name": "Virgin"}}],
            "track-count": 15, "media": [{"format": "CD"}, {"format": "CD"}, {"format": "DVD"}]
        }, {"id": "not-an-id", "title": "Broken"}]});
        let results = parse_search(&body);
        assert_eq!(results.len(), 1);
        let result = &results[0];
        assert_eq!(
            (result.artist.as_str(), result.year.as_str(), result.score),
            ("Daft Punk & Friend", "2001", Some(98))
        );
        assert_eq!(
            (result.label.as_str(), result.catalog_number.as_str()),
            ("Virgin", "8496062")
        );
        assert_eq!(result.format, "2×CD + DVD");
        assert_eq!(result.track_count, Some(15));
        assert_eq!(
            result.url,
            format!("https://musicbrainz.org/release/{MBID}")
        );
        assert_eq!(
            result.thumb.as_deref(),
            Some(format!("https://coverartarchive.org/release/{MBID}/front-250").as_str())
        );
    }

    fn release_fixture() -> Value {
        serde_json::from_str(include_str!("fixtures/musicbrainz-release.json")).unwrap()
    }

    #[test]
    fn release_maps_album_fields_track_credits_composers_and_ids() {
        let release = parse_release(&release_fixture(), CoverSize::Medium).unwrap();
        let f = |key: &str| release.fields.get(key).cloned().unwrap_or_default();
        assert_eq!(f("ALBUM"), ["Discovery"]);
        assert_eq!(f("ALBUMARTIST"), ["Daft Punk"]);
        assert_eq!(f("YEAR"), ["2001"]);
        assert_eq!(f("GENRE"), ["Electronic"], "ties break alphabetically");
        assert_eq!(f("PUBLISHER"), ["Virgin", "Labels"]);
        assert_eq!(f("CATALOGNUMBER"), ["8496062"]);
        assert_eq!(f("MUSICBRAINZ_ALBUMID"), [MBID]);
        assert_eq!(f("MUSICBRAINZ_ALBUMSTATUS"), ["official"]);
        assert_eq!(f("MUSICBRAINZ_ALBUMTYPE"), ["album"]);
        assert_eq!(f("MUSICBRAINZ_ALBUMRELEASECOUNTRY"), ["FR"]);
        assert_eq!(release.tracks.len(), 3);
        let first = &release.tracks[0];
        assert_eq!(first.fields["COMPOSER"], ["Thomas Bangalter", "Guy-Man"]);
        assert_eq!(first.fields["TRACK"], ["1/2"]);
        assert_eq!(first.fields["DISCNUMBER"], ["1/2"]);
        assert_eq!(first.fields["MUSICBRAINZ_TRACKID"], ["rec-1"]);
        assert_eq!(first.fields["MUSICBRAINZ_RELEASETRACKID"], ["track-1"]);
        assert_eq!(first.fields["MUSICBRAINZ_WORKID"], ["work-1"]);
        assert_eq!(
            first.fields["ARTIST"],
            ["Daft Punk"],
            "falls back to the release credit"
        );
        assert_eq!(first.length, Some(321.0));
        let second = &release.tracks[1];
        assert_eq!(second.fields["ARTIST"], ["Daft Punk feat. Guest"]);
        assert_eq!(second.fields["MUSICBRAINZ_ARTISTID"], ["a1", "a2"]);
        assert_eq!(
            second.length,
            Some(207.0),
            "recording length fills a missing track length"
        );
        let third = &release.tracks[2];
        assert_eq!(
            (third.title.as_str(), third.position.as_str(), third.disc),
            ("Video", "A", 2)
        );
        assert_eq!(third.fields["TRACK"], ["1/1"]);
        assert_eq!(third.fields["DISCNUMBER"], ["2/2"]);
        assert_eq!(release.covers, [front_cover(MBID, CoverSize::Medium)]);
        assert_eq!(
            release.covers[0].url,
            format!("https://coverartarchive.org/release/{MBID}/front-500")
        );
        assert!(release.more_covers, "the archive holds two images");
        let mut bare = release_fixture();
        bare["cover-art-archive"] = json!({"front": false, "count": 0});
        let bare = parse_release(&bare, CoverSize::Original).unwrap();
        assert!(bare.covers.is_empty() && !bare.more_covers);
    }

    #[test]
    fn release_without_tracks_or_id_is_rejected() {
        assert!(parse_release(
            &json!({"id": MBID, "title": "Empty", "media": []}),
            CoverSize::Medium
        )
        .is_err());
        assert!(parse_release(&json!({"error": "Not Found"}), CoverSize::Medium).is_err());
    }

    #[test]
    fn covers_prefer_fronts_requested_sizes_and_https() {
        let body = json!({"images": [
            {"image": "http://coverartarchive.org/release/x/2.jpg", "front": false, "types": ["Back"], "thumbnails": {"250": "http://coverartarchive.org/release/x/2-250.jpg", "500": "http://coverartarchive.org/release/x/2-500.jpg"}},
            {"image": "http://coverartarchive.org/release/x/1.png", "front": true, "types": ["Front", "Booklet"], "thumbnails": {"small": "http://coverartarchive.org/release/x/1-250.jpg", "1200": "http://coverartarchive.org/release/x/1-1200.jpg"}}
        ]});
        let covers = parse_covers(&body, CoverSize::Large);
        assert_eq!(covers[0].kind, "Front, Booklet");
        assert_eq!(
            covers[0].url,
            "https://coverartarchive.org/release/x/1-1200.jpg"
        );
        assert_eq!(
            covers[0].thumb.as_deref(),
            Some("https://coverartarchive.org/release/x/1-250.jpg")
        );
        assert_eq!(
            covers[1].url, "https://coverartarchive.org/release/x/2.jpg",
            "missing size falls back to the original"
        );
        assert_eq!(
            parse_covers(&body, CoverSize::Original)[0].url,
            "https://coverartarchive.org/release/x/1.png"
        );
    }
}
