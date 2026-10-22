//! Last.fm (`ws.audioscrobbler.com/2.0`, read-only methods with an API key):
//! album search and album information, whose top tags become GENRE. Last.fm's
//! terms ask for credit and links back; results and releases link to the
//! album's Last.fm page.
use super::model::*;
use serde_json::Value;
use tag_core::Fields;

#[cfg(test)]
mod comparison_track_tests {
    use super::*;
    #[test]
    fn track_query_keeps_performing_artist_and_title_separate() {
        let query = SearchQuery {
            mode: SearchMode::ArtistTitle,
            artist: "Artist A".into(),
            title: "One (Extended Mix)".into(),
            album: "Wrong compilation".into(),
            ..Default::default()
        };
        let url = reqwest::Url::parse(&search_url(&query, "test-key").unwrap()).unwrap();
        let values: std::collections::HashMap<_, _> = url.query_pairs().collect();
        assert_eq!(values["method"], "track.search");
        assert_eq!(values["artist"], "Artist A");
        assert_eq!(values["track"], "One (Extended Mix)");
    }
}

pub const API: &str = "https://ws.audioscrobbler.com/2.0/";
/// Result IDs of albums without a MusicBrainz ID: `album:<artist>\t<album>`.
const BY_NAME: &str = "album:";

fn text(value: &Value, key: &str) -> String {
    match value.get(key) {
        Some(Value::String(s)) => s.trim().to_owned(),
        Some(Value::Number(n)) => n.to_string(),
        _ => String::new(),
    }
}
/// Last.fm sends a single entry as an object instead of a one-element array.
fn entries(value: Option<&Value>) -> Vec<&Value> {
    match value {
        Some(Value::Array(items)) => items.iter().collect(),
        Some(item @ Value::Object(_)) => vec![item],
        _ => Vec::new(),
    }
}
fn image(value: &Value, sizes: &[&str]) -> Option<String> {
    let images = entries(value.get("image"));
    sizes.iter().find_map(|size| {
        images
            .iter()
            .find(|i| text(i, "size") == *size)
            .map(|i| text(i, "#text"))
            .filter(|url| url.starts_with("https://"))
    })
}

/// Last.fm reports failures as `{"error": 6, "message": "…"}`.
pub fn check(body: &Value) -> Result<(), String> {
    match body.get("error").and_then(Value::as_u64) {
        None => Ok(()),
        Some(6) => Err("Last.fm could not find this album.".into()),
        Some(10 | 26) => {
            Err("Last.fm rejected the API key. Check it in Tag Sources → API keys….".into())
        }
        Some(29) => Err("Last.fm rate limit reached. Wait a minute and try again.".into()),
        Some(code) => Err(match text(body, "message") {
            message if message.is_empty() => format!("Last.fm returned error {code}."),
            message => format!("Last.fm returned an error: {message}"),
        }),
    }
}

fn url(key: &str, params: &[(&str, &str)]) -> Result<String, String> {
    let mut all = params.to_vec();
    all.push(("api_key", key));
    all.push(("format", "json"));
    reqwest::Url::parse_with_params(API, &all)
        .map(String::from)
        .map_err(|e| e.to_string())
}

pub fn search_url(query: &SearchQuery, key: &str) -> Result<String, String> {
    if query.mode == SearchMode::ArtistTitle {
        if query.title.trim().is_empty() {
            return Err("Enter a track title.".into());
        }
        return url(
            key,
            &[
                ("method", "track.search"),
                ("artist", query.artist.trim()),
                ("track", query.title.trim()),
                ("limit", &PAGE_SIZE.to_string()),
                ("page", &(query.page + 1).to_string()),
            ],
        );
    }
    let text = query.text.trim();
    if text.is_empty() {
        return Err("Enter words from the album title.".into());
    }
    url(
        key,
        &[
            ("method", "album.search"),
            ("album", text),
            ("limit", &PAGE_SIZE.to_string()),
            ("page", &(query.page + 1).to_string()),
        ],
    )
}

/// Artist and album name a lookup by name uses (Artist and album search).
pub fn name_id(artist: &str, album: &str) -> Result<String, String> {
    let (artist, album) = (artist.trim(), album.trim());
    if artist.is_empty() || album.is_empty() {
        return Err("Last.fm looks albums up by artist and album name: enter both.".into());
    }
    Ok(format!("{BY_NAME}{artist}\t{album}"))
}

pub fn info_url(id: &str, key: &str) -> Result<String, String> {
    if let Some((artist, album)) = id.strip_prefix(BY_NAME).and_then(|s| s.split_once('\t')) {
        return url(
            key,
            &[
                ("method", "album.getinfo"),
                ("artist", artist),
                ("album", album),
                ("autocorrect", "1"),
            ],
        );
    }
    let id = id.trim().to_ascii_lowercase();
    if id.len() == 36 && id.chars().all(|c| c.is_ascii_hexdigit() || c == '-') {
        return url(key, &[("method", "album.getinfo"), ("mbid", &id)]);
    }
    Err("Search Last.fm by artist and album, or enter a MusicBrainz release ID.".into())
}

fn album_result(album: &Value, id: String) -> SearchResult {
    SearchResult {
        title: text(album, "name"),
        artist: match album.get("artist") {
            Some(Value::Object(_)) => text(&album["artist"], "name"),
            _ => text(album, "artist"),
        },
        track_count: album
            .pointer("/tracks/track")
            .map(|t| entries(Some(t)).len() as u32)
            .filter(|n| *n > 0),
        thumb: image(album, &["large", "medium"]),
        url: text(album, "url"),
        id,
        ..Default::default()
    }
}

pub fn parse_search(body: &Value) -> Result<SearchPage, String> {
    check(body)?;
    let total = body
        .pointer("/results/opensearch:totalResults")
        .and_then(|v| v.as_str()?.parse().ok());
    let results = entries(body.pointer("/results/albummatches/album"))
        .into_iter()
        .filter_map(|album| {
            let mbid = text(album, "mbid");
            let id = if mbid.len() == 36 {
                mbid
            } else {
                name_id(&text(album, "artist"), &text(album, "name")).ok()?
            };
            Some(album_result(album, id))
        })
        .collect();
    Ok(SearchPage { results, total })
}

pub fn parse_track_search(body: &Value) -> Result<SearchPage, String> {
    check(body)?;
    let rows = body
        .pointer("/results/trackmatches/track")
        .ok_or("Last.fm returned an unexpected track search response.")?;
    if !rows.is_array() && !rows.is_object() {
        return Err("Last.fm returned an unexpected track list.".into());
    }
    let mut results = Vec::new();
    for track in entries(Some(rows)) {
        let title = text(track, "name");
        let artist = track
            .get("artist")
            .and_then(Value::as_str)
            .map(str::to_owned)
            .unwrap_or_else(|| {
                track
                    .get("artist")
                    .map(|value| text(value, "name"))
                    .unwrap_or_default()
            });
        if title.is_empty() || artist.is_empty() {
            continue;
        }
        let id = format!(
            "track:{}",
            serde_json::to_string(&(&artist, &title)).map_err(|error| error.to_string())?
        );
        results.push(SearchResult {
            id: id.clone(),
            track_id: Some(id),
            title,
            artist,
            format: "Track".into(),
            track_count: Some(1),
            thumb: image(track, &["large", "medium"]),
            url: text(track, "url"),
            ..Default::default()
        });
    }
    if !entries(Some(rows)).is_empty() && results.is_empty() {
        return Err("Last.fm returned no usable track identities.".into());
    }
    let total = body
        .pointer("/results/opensearch:totalResults")
        .and_then(|value| value.as_u64().or_else(|| value.as_str()?.parse().ok()))
        .map(|value| value as u32);
    Ok(SearchPage { results, total })
}

pub fn track_info_url(id: &str, key: &str) -> Result<String, String> {
    let (artist, title): (String, String) = serde_json::from_str(
        id.strip_prefix("track:")
            .ok_or("Search for a Last.fm track first.")?,
    )
    .map_err(|_| "Invalid Last.fm track identity.")?;
    if artist.trim().is_empty() || title.trim().is_empty() {
        return Err("Invalid Last.fm track identity.".into());
    }
    url(
        key,
        &[
            ("method", "track.getInfo"),
            ("artist", &artist),
            ("track", &title),
            ("autocorrect", "0"),
        ],
    )
}

pub fn parse_track(body: &Value, id: &str, genres: usize) -> Result<SourceRelease, String> {
    check(body)?;
    let track = body
        .get("track")
        .filter(|value| value.is_object())
        .ok_or("Last.fm could not find this track.")?;
    let title = text(track, "name");
    let artist = track
        .get("artist")
        .and_then(Value::as_str)
        .map(str::to_owned)
        .unwrap_or_else(|| {
            track
                .get("artist")
                .map(|value| text(value, "name"))
                .unwrap_or_default()
        });
    if title.is_empty() || artist.is_empty() {
        return Err("Last.fm returned an unexpected track.".into());
    }
    let album = track.get("album").unwrap_or(&Value::Null);
    let mut fields = Fields::new();
    put_one(&mut fields, "ALBUM", text(album, "title"));
    put_one(
        &mut fields,
        "ALBUMARTIST",
        album
            .get("artist")
            .and_then(Value::as_str)
            .map(str::to_owned)
            .unwrap_or_else(|| {
                album
                    .get("artist")
                    .map(|value| text(value, "name"))
                    .unwrap_or_default()
            }),
    );
    let mut track_fields = Fields::new();
    put_one(&mut track_fields, "TITLE", title.clone());
    put_one(&mut track_fields, "ARTIST", artist.clone());
    put_one(&mut track_fields, "LASTFM_TRACK_ID", id.to_owned());
    put_one(
        &mut track_fields,
        "MUSICBRAINZ_TRACKID",
        text(track, "mbid"),
    );
    let tags = entries(track.pointer("/toptags/tag"))
        .into_iter()
        .chain(entries(track.pointer("/tags/tag")))
        .map(|value| title_case(&text(value, "name")))
        .filter(|value| genre_tag(value))
        .fold(Vec::<String>::new(), |mut found, value| {
            if !found.iter().any(|old| old.eq_ignore_ascii_case(&value)) {
                found.push(value);
            }
            found
        });
    put(
        &mut track_fields,
        "GENRE",
        tags.into_iter().take(genres.clamp(1, 10)).collect(),
    );
    let position = album
        .pointer("/@attr/position")
        .and_then(|value| value.as_u64().or_else(|| value.as_str()?.parse().ok()))
        .unwrap_or(0) as u32;
    if position > 0 {
        put_one(&mut track_fields, "TRACK", position.to_string());
    }
    let covers = image(album, &["mega", "extralarge", "large"])
        .map(|url| SourceCover {
            url,
            thumb: image(album, &["large", "medium"]),
            kind: "Front".into(),
            width: None,
            height: None,
        })
        .into_iter()
        .collect();
    let length = track
        .get("duration")
        .and_then(|value| value.as_f64().or_else(|| value.as_str()?.parse().ok()))
        .filter(|value| *value > 0.0)
        .map(|value| value / 1000.0);
    Ok(SourceRelease {
        provider: Provider::Lastfm,
        id: id.to_owned(),
        url: text(track, "url"),
        preview_url: None,
        title: text(album, "title"),
        artist: artist.clone(),
        fields,
        tracks: vec![SourceTrack {
            position: if position > 0 {
                position.to_string()
            } else {
                String::new()
            },
            disc: 1,
            number: position,
            title,
            artist,
            length,
            fields: track_fields,
        }],
        covers,
        more_covers: false,
    })
}

/// An Artist and album search is a direct lookup; its answer is the one result.
pub fn parse_info_result(body: &Value, id: &str) -> Result<SearchPage, String> {
    check(body)?;
    let album = body
        .get("album")
        .ok_or("Last.fm could not find this album.")?;
    Ok(SearchPage {
        results: vec![album_result(album, id.to_owned())],
        total: Some(1),
    })
}

/// Tags that describe listeners rather than music.
fn genre_tag(tag: &str) -> bool {
    let lower = tag.to_lowercase();
    !lower.is_empty()
        && !lower.chars().all(|c| c.is_ascii_digit())
        && ![
            "seen live",
            "albums i own",
            "favorite",
            "favourite",
            "favorites",
            "favourites",
            "favorite albums",
            "favourite albums",
            "owned",
            "to listen",
        ]
        .contains(&lower.as_str())
}

pub fn parse_release(body: &Value, id: &str, genres: usize) -> Result<SourceRelease, String> {
    check(body)?;
    let album = body
        .get("album")
        .ok_or("Last.fm could not find this album.")?;
    let title = text(album, "name");
    let artist = text(album, "artist");
    let mut fields = Fields::new();
    put_one(&mut fields, "ALBUM", title.clone());
    put_one(&mut fields, "ALBUMARTIST", artist.clone());
    let tags = entries(album.pointer("/tags/tag"))
        .into_iter()
        .chain(entries(album.pointer("/toptags/tag")))
        .map(|tag| title_case(&text(tag, "name")))
        .filter(|tag| genre_tag(tag))
        .fold(Vec::<String>::new(), |mut unique, tag| {
            if !unique.iter().any(|t| t.eq_ignore_ascii_case(&tag)) {
                unique.push(tag);
            }
            unique
        });
    put(
        &mut fields,
        "GENRE",
        tags.into_iter().take(genres.clamp(1, 10)).collect(),
    );
    let mut tracks: Vec<SourceTrack> = entries(album.pointer("/tracks/track"))
        .into_iter()
        .enumerate()
        .map(|(index, track)| {
            let title = text(track, "name");
            let artist = match track.get("artist") {
                Some(Value::Object(_)) => text(&track["artist"], "name"),
                _ => text(track, "artist"),
            };
            let mut fields = Fields::new();
            put_one(&mut fields, "TITLE", title.clone());
            put_one(&mut fields, "ARTIST", artist.clone());
            let number = track
                .pointer("/@attr/rank")
                .and_then(|r| {
                    r.as_u64()
                        .or_else(|| r.as_str().and_then(|s| s.parse().ok()))
                })
                .unwrap_or(index as u64 + 1) as u32;
            SourceTrack {
                position: number.to_string(),
                disc: 1,
                number,
                title,
                artist,
                length: track
                    .get("duration")
                    .and_then(|d| d.as_f64().or_else(|| d.as_str()?.parse().ok()))
                    .filter(|d| *d > 0.0),
                fields,
            }
        })
        .collect();
    tracks.sort_by_key(|t| t.number);
    let total = tracks.len();
    for track in &mut tracks {
        put_one(
            &mut track.fields,
            "TRACK",
            format!("{}/{total}", track.number),
        );
    }
    let covers = image(album, &["mega", "extralarge", "large"])
        .map(|url| SourceCover {
            url,
            thumb: image(album, &["large", "medium"]),
            kind: "Front".into(),
            width: None,
            height: None,
        })
        .into_iter()
        .collect();
    Ok(SourceRelease {
        preview_url: None,
        provider: Provider::Lastfm,
        id: id.to_owned(),
        url: text(album, "url"),
        title,
        artist,
        fields,
        tracks,
        covers,
        more_covers: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn info() -> Value {
        serde_json::from_str(include_str!("fixtures/lastfm-album.json")).unwrap()
    }

    #[test]
    fn requests_carry_the_key_and_lookups_by_name_or_mbid() {
        let id = name_id(" Daft Punk ", "Discovery").unwrap();
        assert_eq!(id, "album:Daft Punk\tDiscovery");
        assert_eq!(info_url(&id, "KEY").unwrap(), "https://ws.audioscrobbler.com/2.0/?method=album.getinfo&artist=Daft+Punk&album=Discovery&autocorrect=1&api_key=KEY&format=json");
        assert!(info_url("48140466-CFF6-3222-bd55-63c27e43190d", "KEY")
            .unwrap()
            .contains("mbid=48140466-cff6-3222-bd55-63c27e43190d"));
        assert!(info_url("nonsense", "KEY").is_err());
        assert!(name_id("", "Discovery").is_err());
        let search = search_url(
            &SearchQuery {
                mode: SearchMode::Text,
                text: "discovery".into(),
                page: 1,
                ..Default::default()
            },
            "KEY",
        )
        .unwrap();
        assert!(search.contains("method=album.search&album=discovery&limit=50&page=2"));
    }

    #[test]
    fn search_lists_albums_and_single_matches() {
        let body = serde_json::json!({"results": {"opensearch:totalResults": "1",
            "albummatches": {"album": {"name": "Discovery", "artist": "Daft Punk", "url": "https://www.last.fm/music/Daft+Punk/Discovery", "mbid": "",
                "image": [{"#text": "https://lastfm.freetls.fastly.net/i/u/174s/a.png", "size": "large"}]}}}});
        let page = parse_search(&body).unwrap();
        assert_eq!(page.total, Some(1));
        assert_eq!(page.results[0].id, "album:Daft Punk\tDiscovery");
        assert!(page.results[0].thumb.as_deref().unwrap().ends_with("a.png"));
        let single = parse_info_result(&info(), "album:Daft Punk\tDiscovery").unwrap();
        assert_eq!(single.results[0].track_count, Some(2));
    }

    #[test]
    fn top_tags_become_genres() {
        let release = parse_release(&info(), "x", 2).unwrap();
        assert_eq!(release.fields["GENRE"], ["Electronic", "French House"]);
        assert_eq!(release.fields["ALBUMARTIST"], ["Daft Punk"]);
        let all = parse_release(&info(), "x", 10).unwrap();
        assert!(
            !all.fields["GENRE"]
                .iter()
                .any(|g| g == "Seen Live" || g == "2001"),
            "{:?}",
            all.fields["GENRE"]
        );
        assert_eq!(release.tracks[0].fields["TRACK"], ["1/2"]);
        assert_eq!(release.tracks[1].title, "Aerodynamic");
        assert_eq!(release.tracks[0].length, Some(320.0));
        assert!(release.covers[0].url.contains("/300x300/"));
        assert!(release.url.starts_with("https://www.last.fm/"));
    }

    #[test]
    fn errors_are_explained() {
        assert!(
            check(&serde_json::json!({"error": 6, "message": "Album not found"}))
                .unwrap_err()
                .contains("could not find")
        );
        assert!(check(&serde_json::json!({"error": 10}))
            .unwrap_err()
            .contains("API key"));
        assert!(check(&serde_json::json!({"error": 29}))
            .unwrap_err()
            .contains("rate limit"));
    }
}
