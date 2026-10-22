//! Last.fm (`ws.audioscrobbler.com/2.0`, read-only methods with TagGroove's own application key;
//! the user signs in through Last.fm's desktop flow instead of entering a key):
//! album search and album information, whose top tags become GENRE. Last.fm's
//! terms ask for credit and links back; results and releases link to the
//! album's Last.fm page.
use super::model::*;
use serde_json::Value;
use tag_core::Fields;

#[cfg(test)]
mod comparison_track_tests {
    #[test]
    fn a_refused_status_still_explains_itself_from_the_json_body() {
        assert_eq!(
            failure_reason(br#"{"error":14,"message":"Unauthorized Token - This token has not been authorized"}"#).as_deref(),
            Some("TagGroove is not approved yet. Approve it on the Last.fm page in your browser, then choose Finish.")
        );
        assert_eq!(
            failure_reason(br#"{"error":15,"message":"expired"}"#).as_deref(),
            Some("The Last.fm sign-in link expired. Start again.")
        );
        // No reason in the body: the caller keeps its status-based message.
        assert!(failure_reason(b"<html>Forbidden</html>").is_none());
        assert!(failure_reason(br#"{"token":"abc"}"#).is_none());
    }

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
        Some(10 | 26) => Err("Last.fm rejected TagGroove's application key.".into()),
        Some(4) => Err("Last.fm did not accept the sign-in. Start again.".into()),
        Some(9) => Err("Your Last.fm sign-in is no longer valid. Sign in again in Tag Sources → API keys….".into()),
        Some(14) => Err("TagGroove is not approved yet. Approve it on the Last.fm page in your browser, then choose Finish.".into()),
        Some(15) => Err("The Last.fm sign-in link expired. Start again.".into()),
        Some(29) => Err("Last.fm rate limit reached. Wait a minute and try again.".into()),
        Some(code) => Err(match text(body, "message") {
            message if message.is_empty() => format!("Last.fm returned error {code}."),
            message => format!("Last.fm returned an error: {message}"),
        }),
    }
}

/// Last.fm answers some failures (an unapproved sign-in, a dead session) with an HTTP 4xx status
/// and the reason in a JSON body. The wording for that reason, when the body carries one.
pub fn failure_reason(body: &[u8]) -> Option<String> {
    check(&serde_json::from_slice::<Value>(body).ok()?).err()
}

/// The signature Last.fm asks for: every parameter except `format` and `callback`, sorted by
/// name, written as name then value, followed by the shared secret, then hashed with MD5.
pub fn api_sig(params: &[(&str, &str)], secret: &str) -> String {
    use md5::{Digest, Md5};
    let mut sorted: Vec<&(&str, &str)> = params
        .iter()
        .filter(|(name, _)| !matches!(*name, "format" | "callback"))
        .collect();
    sorted.sort_by(|a, b| a.0.cmp(b.0));
    let mut text = String::new();
    for (name, value) in sorted {
        text.push_str(name);
        text.push_str(value);
    }
    text.push_str(secret);
    Md5::digest(text.as_bytes())
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

/// A signed call: the parameters, TagGroove's key, then the signature.
fn signed_url(key: &str, secret: &str, params: &[(&str, &str)]) -> Result<String, String> {
    let mut all = params.to_vec();
    all.push(("api_key", key));
    let signature = api_sig(&all, secret);
    all.push(("api_sig", &signature));
    all.push(("format", "json"));
    reqwest::Url::parse_with_params(API, &all)
        .map(String::from)
        .map_err(|e| e.to_string())
}

/// Step 1 of the desktop sign-in: ask Last.fm for a request token.
pub fn token_url(key: &str, secret: &str) -> Result<String, String> {
    signed_url(key, secret, &[("method", "auth.getToken")])
}

/// Step 2: the page where the person approves TagGroove (opened in the browser).
pub fn authorize_url(key: &str, token: &str) -> Result<String, String> {
    reqwest::Url::parse_with_params(
        "https://www.last.fm/api/auth/",
        &[("api_key", key), ("token", token)],
    )
    .map(String::from)
    .map_err(|e| e.to_string())
}

/// Step 3: trade the approved token for a session.
pub fn session_url(key: &str, secret: &str, token: &str) -> Result<String, String> {
    signed_url(
        key,
        secret,
        &[("method", "auth.getSession"), ("token", token)],
    )
}

pub fn parse_token(body: &Value) -> Result<String, String> {
    check(body)?;
    Some(text(body, "token"))
        .filter(|token| !token.is_empty() && token.len() <= 64)
        .ok_or_else(|| "Last.fm did not return a sign-in token.".into())
}

/// The account name and session key of a finished sign-in.
pub fn parse_session(body: &Value) -> Result<(String, String), String> {
    check(body)?;
    let session = body.get("session").unwrap_or(&Value::Null);
    let (name, key) = (text(session, "name"), text(session, "key"));
    if name.is_empty() || key.is_empty() || key.len() > 64 {
        return Err("Last.fm did not return a session.".into());
    }
    Ok((name, key))
}

/// Sign an already built read address with the signed-in session, so the call is made on behalf
/// of that account. The address keeps `format=json`, which is never part of the signature.
pub fn with_session(url: &str, secret: &str, session: &str) -> Result<String, String> {
    let mut address = reqwest::Url::parse(url).map_err(|e| e.to_string())?;
    let pairs: Vec<(String, String)> = address
        .query_pairs()
        .filter(|(name, _)| !matches!(name.as_ref(), "api_sig" | "sk"))
        .map(|(name, value)| (name.into_owned(), value.into_owned()))
        .collect();
    let mut params: Vec<(&str, &str)> = pairs
        .iter()
        .map(|(name, value)| (name.as_str(), value.as_str()))
        .collect();
    params.push(("sk", session));
    let signature = api_sig(&params, secret);
    // Rebuild the query without any earlier signature, then add the new one.
    address
        .query_pairs_mut()
        .clear()
        .extend_pairs(
            pairs
                .iter()
                .map(|(name, value)| (name.as_str(), value.as_str())),
        )
        .append_pair("sk", session)
        .append_pair("api_sig", &signature);
    Ok(address.into())
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
            .contains("application key"));
        assert!(check(&serde_json::json!({"error": 29}))
            .unwrap_err()
            .contains("rate limit"));
    }

    #[test]
    fn a_token_last_fm_will_not_accept_again_says_to_start_again() {
        // The sign-in dialog offers Start for a new token when a message ends this way.
        for code in [4, 15] {
            let message = check(&serde_json::json!({ "error": code })).unwrap_err();
            assert!(message.ends_with("Start again."), "{code}: {message}");
        }
        // Still waiting for the approval: Finish stays available, so this one does not.
        let pending = check(&serde_json::json!({"error": 14})).unwrap_err();
        assert!(!pending.ends_with("Start again."), "{pending}");
    }
}

#[cfg(test)]
mod sign_in_tests {
    use super::*;

    #[test]
    fn the_signature_sorts_parameters_and_ignores_format_and_callback() {
        // The worked example in Last.fm's desktop authentication documentation.
        let sig = api_sig(
            &[
                ("method", "auth.getSession"),
                ("token", "tok"),
                ("api_key", "key"),
                ("format", "json"),
            ],
            "mysecret",
        );
        use md5::{Digest, Md5};
        let expected: String = Md5::digest(b"api_keykeymethodauth.getSessiontokentokmysecret")
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        assert_eq!(sig, expected);
        assert_eq!(sig.len(), 32);
    }

    #[test]
    fn sign_in_addresses_carry_a_signature_but_never_the_secret() {
        for url in [
            token_url("key", "secret").unwrap(),
            session_url("key", "secret", "tok").unwrap(),
        ] {
            let parsed = reqwest::Url::parse(&url).unwrap();
            let query: std::collections::HashMap<_, _> = parsed.query_pairs().collect();
            assert_eq!(query["api_key"], "key");
            assert_eq!(query["api_sig"].len(), 32);
            assert_eq!(query["format"], "json");
            assert!(!url.contains("secret"));
        }
        let approve = authorize_url("key", "tok").unwrap();
        assert_eq!(
            approve,
            "https://www.last.fm/api/auth/?api_key=key&token=tok"
        );
    }

    #[test]
    fn token_and_session_responses_are_read_strictly() {
        assert_eq!(
            parse_token(&serde_json::json!({"token":"abc123"})).unwrap(),
            "abc123"
        );
        assert!(parse_token(&serde_json::json!({})).is_err());
        assert!(parse_token(&serde_json::json!({"error":15,"message":"x"}))
            .unwrap_err()
            .contains("expired"));
        let session =
            serde_json::json!({"session":{"name":"someone","key":"sk123","subscriber":0}});
        assert_eq!(
            parse_session(&session).unwrap(),
            ("someone".into(), "sk123".into())
        );
        assert!(parse_session(&serde_json::json!({"session":{"name":"someone"}})).is_err());
        assert!(
            parse_session(&serde_json::json!({"error":14,"message":"x"}))
                .unwrap_err()
                .contains("Approve")
        );
    }

    #[test]
    fn a_read_address_is_signed_with_the_session_and_resigned_when_signed_again() {
        let base = info_url("album:Artist\tAlbum", "key").unwrap();
        let signed = with_session(&base, "secret", "sk1").unwrap();
        let query: std::collections::HashMap<_, _> = reqwest::Url::parse(&signed)
            .unwrap()
            .query_pairs()
            .map(|(a, b)| (a.into_owned(), b.into_owned()))
            .collect();
        assert_eq!(query["sk"], "sk1");
        assert_eq!(query["api_sig"].len(), 32);
        assert!(!signed.contains("secret"));
        // Signing an already signed address replaces the old signature.
        let again = with_session(&signed, "secret", "sk2").unwrap();
        assert_eq!(again.matches("api_sig=").count(), 1);
        assert_eq!(again.matches("sk=").count(), 1);
    }
}
