//! fanart.tv album artwork (API v3, `music/albums/{release group MBID}`),
//! a covers-only source. Albums are found with a MusicBrainz release-group
//! search. The project key and the user's personal key travel as `api-key` /
//! `client-key` headers, never in addresses.
use super::model::*;
use super::musicbrainz;
use serde_json::Value;

pub const API: &str = "https://webservice.fanart.tv/v3";

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

/// MusicBrainz release-group search for the albums fanart.tv knows by MBID.
pub fn search_query(query: &SearchQuery) -> Result<String, String> {
    let lucene = match query.mode {
        SearchMode::ArtistAlbum => {
            let mut parts = Vec::new();
            if !query.album.trim().is_empty() {
                parts.push(format!(
                    "releasegroup:{}",
                    musicbrainz::lucene(&query.album)
                ));
            }
            if !query.artist.trim().is_empty() {
                parts.push(format!("artist:{}", musicbrainz::lucene(&query.artist)));
            }
            parts.join(" AND ")
        }
        SearchMode::Text => query.text.trim().to_owned(),
        _ => return Err("fanart.tv albums are found by artist and album or free text.".into()),
    };
    if lucene.is_empty() {
        return Err("Enter an artist, album or search text.".into());
    }
    reqwest::Url::parse_with_params(
        &format!("{}/release-group/", musicbrainz::API),
        &[
            ("query", lucene.as_str()),
            ("fmt", "json"),
            ("limit", &PAGE_SIZE.to_string()),
            ("offset", &(query.page * PAGE_SIZE).to_string()),
        ],
    )
    .map(String::from)
    .map_err(|e| e.to_string())
}

/// A release-group MBID or musicbrainz.org/release-group/ address.
pub fn release_group_id(input: &str) -> Result<String, String> {
    let lower = input.trim().to_ascii_lowercase();
    if lower.contains("/release/") {
        return Err("Enter a release group: fanart.tv keeps artwork per release group.".into());
    }
    lower
        .as_bytes()
        .windows(36)
        .filter_map(|window| std::str::from_utf8(window).ok())
        .find(|candidate| musicbrainz::is_mbid(candidate))
        .map(str::to_owned)
        .ok_or_else(|| "Enter a MusicBrainz release group ID or address.".into())
}

pub fn parse_search(body: &Value) -> SearchPage {
    let results = list(body, "release-groups")
        .iter()
        .filter_map(|group| {
            let id = text(group, "id");
            musicbrainz::is_mbid(&id).then(|| SearchResult {
                title: text(group, "title"),
                artist: musicbrainz::credit(group),
                year: year(&text(group, "first-release-date")),
                format: text(group, "primary-type"),
                score: group.get("score").and_then(Value::as_u64).map(|n| n as u32),
                thumb: Some(format!(
                    "https://coverartarchive.org/release-group/{id}/front-250"
                )),
                url: format!("https://musicbrainz.org/release-group/{id}"),
                id,
                ..Default::default()
            })
        })
        .collect();
    SearchPage {
        results,
        total: body.get("count").and_then(Value::as_u64).map(|n| n as u32),
    }
}

pub fn album_url(id: &str) -> String {
    format!("{API}/music/albums/{id}")
}

/// fanart.tv lists `http://assets.fanart.tv/…`; the same files are served over HTTPS.
fn https(url: &str) -> String {
    match url.strip_prefix("http://") {
        Some(rest) => format!("https://{rest}"),
        None => url.to_owned(),
    }
}

/// Album covers by popularity, then disc art.
pub fn parse_album(body: &Value, id: &str) -> Result<SourceRelease, String> {
    if text(body, "status") == "error" {
        return Err("fanart.tv has no artwork for this album.".into());
    }
    let album = body
        .get("albums")
        .and_then(|albums| albums.get(id))
        .ok_or("fanart.tv has no artwork for this album.")?;
    let likes = |image: &Value| text(image, "likes").parse::<u32>().unwrap_or(0);
    let mut fronts: Vec<&Value> = list(album, "albumcover").iter().collect();
    fronts.sort_by_key(|image| std::cmp::Reverse(likes(image)));
    let cover = |image: &Value, kind: String| {
        let url = https(&text(image, "url"));
        let size = text(image, "size").parse().ok();
        (url.starts_with("https://")).then(|| SourceCover {
            thumb: Some(url.replacen("/fanart/", "/preview/", 1)),
            url,
            kind,
            width: size,
            height: size,
        })
    };
    let mut covers: Vec<SourceCover> = fronts
        .into_iter()
        .filter_map(|image| cover(image, format!("Front ({} likes)", likes(image))))
        .collect();
    covers.extend(list(album, "cdart").iter().filter_map(|image| {
        let disc = text(image, "disc");
        cover(
            image,
            if disc.is_empty() {
                "Media".into()
            } else {
                format!("Media (disc {disc})")
            },
        )
    }));
    if covers.is_empty() {
        return Err("fanart.tv has no album covers for this album.".into());
    }
    Ok(SourceRelease {
        preview_url: None,
        provider: Provider::Fanart,
        id: id.to_owned(),
        url: format!("https://fanart.tv/artist/{}/", text(body, "mbid_id")),
        title: "Album artwork".into(),
        artist: text(body, "name"),
        fields: Default::default(),
        tracks: Vec::new(),
        covers,
        more_covers: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    const GROUP: &str = "9ba659df-5814-32f6-b95f-02b738698e7c";

    #[test]
    fn albums_are_found_as_musicbrainz_release_groups() {
        let url = search_query(&SearchQuery {
            mode: SearchMode::ArtistAlbum,
            artist: "Evanescence".into(),
            album: "Anywhere but Home".into(),
            ..Default::default()
        })
        .unwrap();
        assert!(url.starts_with("https://musicbrainz.org/ws/2/release-group/?query=releasegroup"));
        assert!(search_query(&SearchQuery {
            mode: SearchMode::Barcode,
            text: "1".into(),
            ..Default::default()
        })
        .is_err());
        assert_eq!(
            release_group_id(&format!("https://musicbrainz.org/release-group/{GROUP}")).unwrap(),
            GROUP
        );
        assert!(release_group_id(&format!("https://musicbrainz.org/release/{GROUP}")).is_err());
        let page = parse_search(&serde_json::json!({"count": 1, "release-groups": [
            {"id": GROUP, "title": "Anywhere but Home", "first-release-date": "2004-11-23", "primary-type": "Album",
             "artist-credit": [{"name": "Evanescence"}]}]}));
        assert_eq!(page.results[0].artist, "Evanescence");
        assert_eq!(page.results[0].year, "2004");
    }

    #[test]
    fn covers_are_ordered_by_likes_then_disc_art() {
        let body = serde_json::json!({"name": "Evanescence", "mbid_id": "f4a31f0a-51dd-4fa7-986d-3095c40c5ed9", "albums": {GROUP: {
            "albumcover": [
                {"id": "1", "url": "http://assets.fanart.tv/fanart/music/x/albumcover/a.jpg", "likes": "0"},
                {"id": "2", "url": "http://assets.fanart.tv/fanart/music/x/albumcover/b.jpg", "likes": "3"}],
            "cdart": [{"id": "3", "url": "http://assets.fanart.tv/fanart/music/x/cdart/c.png", "likes": "0", "disc": "1", "size": "1000"}]}}});
        let release = parse_album(&body, GROUP).unwrap();
        let urls: Vec<_> = release.covers.iter().map(|c| c.url.as_str()).collect();
        assert_eq!(
            urls,
            [
                "https://assets.fanart.tv/fanart/music/x/albumcover/b.jpg",
                "https://assets.fanart.tv/fanart/music/x/albumcover/a.jpg",
                "https://assets.fanart.tv/fanart/music/x/cdart/c.png",
            ]
        );
        assert_eq!(release.covers[0].kind, "Front (3 likes)");
        assert_eq!(release.covers[2].kind, "Media (disc 1)");
        assert_eq!(release.covers[2].width, Some(1000));
        assert_eq!(
            release.covers[0].thumb.as_deref(),
            Some("https://assets.fanart.tv/preview/music/x/albumcover/b.jpg")
        );
        assert!(parse_album(
            &serde_json::json!({"status": "error", "error message": "Not found"}),
            GROUP
        )
        .is_err());
        assert!(parse_album(&serde_json::json!({"albums": {}}), GROUP).is_err());
    }
}
