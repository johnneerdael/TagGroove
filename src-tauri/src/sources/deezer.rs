//! Deezer's public API (no key): album search, albums with their tracks
//! (ISRC, disc and track position) and cover art. Deezer allows 50 requests
//! per 5 seconds and reports errors, including its quota, inside an HTTP 200
//! answer.
use super::model::*;
use serde_json::Value;
use tag_core::Fields;

pub const API: &str = "https://api.deezer.com";

/// Cover size from Options → Tag Sources → Online services.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Deserialize)]
pub enum CoverSize {
    #[serde(rename = "500")]
    Big,
    #[default]
    #[serde(rename = "1000")]
    Xl,
}

fn text(value: &Value, key: &str) -> String {
    match value.get(key) {
        Some(Value::String(s)) => s.trim().to_owned(),
        Some(Value::Number(n)) => n.to_string(),
        _ => String::new(),
    }
}
fn data(value: &Value) -> &[Value] {
    value
        .get("data")
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or_default()
}

/// Deezer answers errors with HTTP 200 and an `error` object.
pub fn check(body: &Value) -> Result<(), String> {
    let Some(error) = body.get("error") else {
        return Ok(());
    };
    Err(match error.get("code").and_then(Value::as_u64) {
        Some(4) => "Deezer rate limit reached. Wait a few seconds and try again.".into(),
        Some(800) => "Deezer could not find this release.".into(),
        _ => match text(error, "message") {
            message if message.is_empty() => "Deezer returned an error.".into(),
            message => format!("Deezer returned an error: {message}"),
        },
    })
}

pub fn search_query(query: &SearchQuery) -> Result<String, String> {
    let terms: Vec<&str> = match query.mode {
        SearchMode::ArtistAlbum => vec![query.artist.trim(), query.album.trim()],
        SearchMode::Text => vec![query.text.trim()],
        _ => return Err("Deezer searches by artist and album, free text or barcode.".into()),
    };
    let q = terms
        .into_iter()
        .filter(|t| !t.is_empty())
        .collect::<Vec<_>>()
        .join(" ");
    if q.is_empty() {
        return Err("Enter an artist, album or search text.".into());
    }
    reqwest::Url::parse_with_params(
        &format!("{API}/search/album"),
        &[
            ("q", q.as_str()),
            ("limit", &PAGE_SIZE.to_string()),
            ("index", &(query.page * PAGE_SIZE).to_string()),
        ],
    )
    .map(String::from)
    .map_err(|e| e.to_string())
}

/// The album with this barcode (`/album/upc:…`).
pub fn barcode_url(text: &str) -> Result<String, String> {
    let upc: String = text.chars().filter(char::is_ascii_digit).collect();
    if upc.is_empty() {
        return Err("Enter the barcode (UPC or EAN) of the album.".into());
    }
    Ok(format!("{API}/album/upc:{upc}"))
}

/// `302127` or a deezer.com album address.
pub fn release_id(input: &str) -> Result<String, String> {
    let input = input.trim();
    if !input.is_empty() && input.chars().all(|c| c.is_ascii_digit()) {
        return Ok(input.to_owned());
    }
    let lower = input.to_ascii_lowercase();
    if lower.contains("deezer.com/") {
        if let Some(rest) = lower.split("/album/").nth(1) {
            let id: String = rest.chars().take_while(char::is_ascii_digit).collect();
            if !id.is_empty() {
                return Ok(id);
            }
        }
    }
    Err("Enter a Deezer album ID or album address.".into())
}

pub fn release_url(id: &str) -> String {
    format!("{API}/album/{id}")
}
pub fn tracks_url(id: &str) -> String {
    format!("{API}/album/{id}/tracks?limit=500")
}

/// Collect a complete album before calculating per-disc track totals.
pub async fn load_tracks<F, Fut>(id: &str, mut fetch: F) -> Result<Value, String>
where
    F: FnMut(String) -> Fut,
    Fut: std::future::Future<Output = Result<Value, String>>,
{
    let mut url = tracks_url(id);
    let mut tracks = Vec::new();
    let mut total = None;
    loop {
        let page = fetch(url).await?;
        check(&page)?;
        let items = page
            .get("data")
            .and_then(Value::as_array)
            .ok_or("Deezer returned an unexpected track list.")?;
        total = total.or_else(|| page.get("total").and_then(Value::as_u64));
        // Bound aggregate memory and requests as well as each HTTP response.
        if tracks.len() + items.len() > 10_000 {
            return Err("Deezer returned more tracks than expected.".into());
        }
        tracks.extend(items.iter().cloned());
        let next = page
            .get("next")
            .filter(|v| !v.is_null())
            .map(|v| {
                v.as_str()
                    .ok_or("Deezer returned an invalid next track page.")
            })
            .transpose()?
            .filter(|s| !s.is_empty());
        if next.is_none() && total.is_none_or(|n| tracks.len() as u64 >= n) {
            return Ok(serde_json::json!({"data": tracks}));
        }
        if items.is_empty() {
            return Err("Deezer returned an incomplete track list.".into());
        }
        if let Some(next) = next {
            let next = reqwest::Url::parse(next)
                .map_err(|_| "Deezer returned an invalid next track page.")?;
            let indices: Vec<_> = next
                .query_pairs()
                .filter(|(key, _)| key == "index")
                .map(|(_, value)| value.parse::<usize>().ok())
                .collect();
            if !matches!(next.scheme(), "http" | "https")
                || next.host_str() != Some("api.deezer.com")
                || !next.username().is_empty()
                || next.password().is_some()
                || next.port().is_some()
                || next.fragment().is_some()
                || next.path() != format!("/album/{id}/tracks")
                || indices != [Some(tracks.len())]
            {
                return Err("Deezer returned an invalid next track page.".into());
            }
        }
        // Rebuild on the fixed HTTPS endpoint, including for legacy HTTP next
        // links. Never send requests to an arbitrary response-supplied URL.
        url = format!("{}&index={}", tracks_url(id), tracks.len());
    }
}

fn result(album: &Value) -> Option<SearchResult> {
    let id = text(album, "id");
    if id.is_empty() || !id.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    let thumb = text(album, "cover_medium");
    Some(SearchResult {
        title: text(album, "title"),
        artist: album
            .get("artist")
            .map(|a| text(a, "name"))
            .unwrap_or_default(),
        year: year(&text(album, "release_date")),
        label: text(album, "label"),
        format: super::model::title_case(&text(album, "record_type")),
        track_count: album
            .get("nb_tracks")
            .and_then(Value::as_u64)
            .map(|n| n as u32),
        barcode: text(album, "upc"),
        thumb: (!thumb.is_empty()).then_some(thumb),
        url: match text(album, "link") {
            link if link.is_empty() => format!("https://www.deezer.com/album/{id}"),
            link => link,
        },
        id,
        ..Default::default()
    })
}

pub fn parse_search(body: &Value) -> Result<SearchPage, String> {
    check(body)?;
    Ok(SearchPage {
        results: data(body).iter().filter_map(result).collect(),
        total: body.get("total").and_then(Value::as_u64).map(|n| n as u32),
    })
}

/// A barcode lookup answers with the album itself.
pub fn parse_album_result(body: &Value) -> Result<SearchPage, String> {
    check(body)?;
    let results: Vec<SearchResult> = result(body).into_iter().collect();
    Ok(SearchPage {
        total: Some(results.len() as u32),
        results,
    })
}

pub fn parse_release(
    album: &Value,
    track_list: &Value,
    size: CoverSize,
) -> Result<SourceRelease, String> {
    check(album)?;
    check(track_list)?;
    let id = text(album, "id");
    if id.is_empty() {
        return Err("Deezer returned an unexpected album.".into());
    }
    let title = text(album, "title");
    let artist = album
        .get("artist")
        .map(|a| text(a, "name"))
        .unwrap_or_default();
    let mut fields = Fields::new();
    put_one(&mut fields, "ALBUM", title.clone());
    put_one(&mut fields, "ALBUMARTIST", artist.clone());
    put_one(&mut fields, "YEAR", year(&text(album, "release_date")));
    put(
        &mut fields,
        "GENRE",
        album
            .get("genres")
            .map(data)
            .unwrap_or_default()
            .iter()
            .map(|g| text(g, "name"))
            .collect(),
    );
    put_one(&mut fields, "PUBLISHER", text(album, "label"));
    put_one(&mut fields, "BARCODE", text(album, "upc"));
    let mut tracks: Vec<SourceTrack> = data(track_list)
        .iter()
        .enumerate()
        .map(|(index, track)| {
            let disc = track
                .get("disk_number")
                .and_then(Value::as_u64)
                .unwrap_or(1)
                .max(1) as u32;
            let number = track
                .get("track_position")
                .and_then(Value::as_u64)
                .unwrap_or(index as u64 + 1) as u32;
            let title = text(track, "title");
            let artist = track
                .get("artist")
                .map(|a| text(a, "name"))
                .unwrap_or_default();
            let mut fields = Fields::new();
            put_one(&mut fields, "TITLE", title.clone());
            put_one(&mut fields, "ARTIST", artist.clone());
            put_one(&mut fields, "ISRC", text(track, "isrc"));
            SourceTrack {
                position: number.to_string(),
                disc,
                number,
                title,
                artist,
                length: track.get("duration").and_then(Value::as_f64),
                fields,
            }
        })
        .collect();
    tracks.sort_by_key(|t| (t.disc, t.number));
    let discs: std::collections::BTreeSet<u32> = tracks.iter().map(|t| t.disc).collect();
    let counts: Vec<(u32, usize)> = discs
        .iter()
        .map(|disc| (*disc, tracks.iter().filter(|t| t.disc == *disc).count()))
        .collect();
    for track in &mut tracks {
        let on_disc = counts
            .iter()
            .find(|(disc, _)| *disc == track.disc)
            .map_or(0, |(_, count)| *count);
        put_one(
            &mut track.fields,
            "TRACK",
            format!("{}/{on_disc}", track.number),
        );
        put_one(
            &mut track.fields,
            "DISCNUMBER",
            format!("{}/{}", track.disc, discs.len().max(1)),
        );
        if discs.len() > 1 {
            track.position = format!("{}-{}", track.disc, track.number);
        }
    }
    let cover = match size {
        CoverSize::Xl => text(album, "cover_xl"),
        CoverSize::Big => text(album, "cover_big"),
    };
    let covers = (!cover.is_empty())
        .then(|| SourceCover {
            url: cover,
            thumb: Some(text(album, "cover_medium")).filter(|t| !t.is_empty()),
            kind: "Front".into(),
            width: Some(if size == CoverSize::Xl { 1000 } else { 500 }),
            height: Some(if size == CoverSize::Xl { 1000 } else { 500 }),
        })
        .into_iter()
        .collect();
    Ok(SourceRelease {
        provider: Provider::Deezer,
        url: match text(album, "link") {
            link if link.is_empty() => format!("https://www.deezer.com/album/{id}"),
            link => link,
        },
        id,
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

    #[test]
    fn paginated_album_keeps_all_tracks_and_disc_totals() {
        let tracks = tauri::async_runtime::block_on(load_tracks("302127", |url| {
            let url = reqwest::Url::parse(&url).unwrap();
            let index = url
                .query_pairs()
                .find(|(key, _)| key == "index")
                .map(|(_, value)| value.parse::<usize>().unwrap())
                .unwrap_or(0);
            let end = (index + 100).min(205);
            let data: Vec<_> = (index..end)
                .map(|i| {
                    serde_json::json!({
                        "id": i + 1, "title": format!("Track {}", i + 1),
                        "disk_number": i / 100 + 1, "track_position": i % 100 + 1,
                        "isrc": format!("TEST{i:08}"), "duration": 120,
                        "artist": {"name": "Fixture Artist"}
                    })
                })
                .collect();
            let next = (end < 205).then(|| {
                format!("https://api.deezer.com/album/302127/tracks?limit=100&index={end}")
            });
            std::future::ready(Ok(
                serde_json::json!({"data": data, "total": 205, "next": next}),
            ))
        }))
        .unwrap();
        let release = parse_release(&fixture("album"), &tracks, CoverSize::Xl).unwrap();
        assert_eq!(release.tracks.len(), 205);
        assert_eq!(release.tracks[100].title, "Track 101");
        assert_eq!(release.tracks[204].fields["ISRC"], ["TEST00000204"]);
        assert_eq!(release.tracks[204].fields["TRACK"], ["5/5"]);
        assert_eq!(release.tracks[204].fields["DISCNUMBER"], ["3/3"]);
    }

    fn fixture(name: &str) -> Value {
        let text = match name {
            "album" => include_str!("fixtures/deezer-album.json"),
            "tracks" => include_str!("fixtures/deezer-tracks.json"),
            _ => include_str!("fixtures/deezer-search.json"),
        };
        serde_json::from_str(text).unwrap()
    }

    #[test]
    fn pagination_reports_later_page_errors_without_partial_results() {
        for (failure, message) in [
            (Err("Cancelled: newer request".to_owned()), "Cancelled"),
            (Ok(serde_json::json!({"error": {"code": 4}})), "rate limit"),
            (
                Ok(serde_json::json!({"data": [], "total": 2})),
                "incomplete",
            ),
        ] {
            let result = tauri::async_runtime::block_on(load_tracks("302127", |url| {
                std::future::ready(if url.contains("index=") {
                    failure.clone()
                } else {
                    Ok(serde_json::json!({"data": [{"title": "First"}], "total": 2,
                        "next": "https://api.deezer.com/album/302127/tracks?index=1&limit=500"}))
                })
            }));
            assert!(result.unwrap_err().contains(message));
        }
    }

    #[test]
    fn pagination_rejects_unsafe_or_nonadvancing_next_addresses() {
        for next in [
            "https://evil.example/album/302127/tracks?index=1",
            "https://api.deezer.com/album/42/tracks?index=1",
            "https://api.deezer.com/search?index=1",
            "https://user@api.deezer.com/album/302127/tracks?index=1",
            "https://api.deezer.com:8080/album/302127/tracks?index=1",
            "ftp://api.deezer.com/album/302127/tracks?index=1",
            "https://api.deezer.com/album/302127/tracks?index=0",
            "https://api.deezer.com/album/302127/tracks?index=2",
            "not a URL",
        ] {
            let result = tauri::async_runtime::block_on(load_tracks("302127", |url| {
                assert!(
                    !url.contains("index="),
                    "invalid next must never be fetched"
                );
                std::future::ready(Ok(
                    serde_json::json!({"data": [{"id": 1}], "total": 2, "next": next}),
                ))
            }));
            assert!(result.is_err(), "accepted {next}");
        }
    }

    #[test]
    fn pagination_upgrades_provider_http_links_and_uses_total_without_next() {
        for next in [
            Value::Null,
            Value::String("http://api.deezer.com/album/302127/tracks?index=1&limit=500".into()),
        ] {
            let tracks = tauri::async_runtime::block_on(load_tracks("302127", |url| {
                assert!(url.starts_with("https://api.deezer.com/album/302127/tracks?"));
                std::future::ready(Ok(if url.contains("index=1") {
                    serde_json::json!({"data": [{"id": 2}], "total": 2})
                } else {
                    serde_json::json!({"data": [{"id": 1}], "total": 2, "next": next})
                }))
            }))
            .unwrap();
            assert_eq!(data(&tracks).len(), 2);
        }
    }

    #[test]
    fn search_and_paging_parameters() {
        let query = SearchQuery {
            mode: SearchMode::ArtistAlbum,
            artist: "Daft Punk".into(),
            album: "Discovery".into(),
            page: 2,
            ..Default::default()
        };
        assert_eq!(
            search_query(&query).unwrap(),
            "https://api.deezer.com/search/album?q=Daft+Punk+Discovery&limit=50&index=100"
        );
        let page = parse_search(&fixture("search")).unwrap();
        assert_eq!(page.total, Some(2));
        let first = &page.results[0];
        assert_eq!(
            (
                first.id.as_str(),
                first.title.as_str(),
                first.artist.as_str()
            ),
            ("302127", "Discovery", "Daft Punk")
        );
        assert_eq!(
            (first.format.as_str(), first.track_count),
            ("Album", Some(14))
        );
        assert_eq!(
            barcode_url("724384 960650").unwrap(),
            "https://api.deezer.com/album/upc:724384960650"
        );
        assert!(search_query(&SearchQuery {
            mode: SearchMode::CatalogNumber,
            ..Default::default()
        })
        .is_err());
    }

    #[test]
    fn errors_inside_successful_answers_are_reported() {
        let quota = serde_json::json!({"error": {"type": "Exception", "message": "Quota limit exceeded", "code": 4}});
        assert!(check(&quota).unwrap_err().contains("rate limit"));
        let missing = serde_json::json!({"error": {"type": "DataException", "message": "no data", "code": 800}});
        assert!(parse_search(&missing)
            .unwrap_err()
            .contains("could not find"));
        assert!(check(&serde_json::json!({"error": {"message": "Odd"}}))
            .unwrap_err()
            .contains("Odd"));
    }

    #[test]
    fn album_ids_come_from_numbers_and_addresses() {
        assert_eq!(release_id("302127").unwrap(), "302127");
        assert_eq!(
            release_id("https://www.deezer.com/en/album/302127?utm_source=x").unwrap(),
            "302127"
        );
        assert!(release_id("https://www.deezer.com/en/artist/27").is_err());
    }

    #[test]
    fn release_maps_album_tracks_isrc_and_discs() {
        let release = parse_release(&fixture("album"), &fixture("tracks"), CoverSize::Xl).unwrap();
        assert_eq!(release.fields["ALBUM"], ["Discovery"]);
        assert_eq!(release.fields["GENRE"], ["Electro"]);
        assert_eq!(release.fields["PUBLISHER"], ["Daft Life Ltd./ADA France"]);
        assert_eq!(release.fields["BARCODE"], ["724384960650"]);
        assert_eq!(release.fields["YEAR"], ["2001"]);
        let titles: Vec<_> = release.tracks.iter().map(|t| t.title.as_str()).collect();
        assert_eq!(titles, ["One More Time", "Aerodynamic", "Bonus"]);
        let first = &release.tracks[0];
        assert_eq!(first.fields["ISRC"], ["GBDUW0000053"]);
        assert_eq!(first.fields["TRACK"], ["1/2"]);
        assert_eq!(first.fields["DISCNUMBER"], ["1/2"]);
        assert_eq!(first.length, Some(320.0));
        assert_eq!(release.tracks[2].fields["DISCNUMBER"], ["2/2"]);
        assert_eq!(release.tracks[2].position, "2-1");
        assert!(release.covers[0].url.contains("1000x1000"));
        assert_eq!(release.url, "https://www.deezer.com/album/302127");
    }
}
