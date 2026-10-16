//! Apple's iTunes Search API (`search` and `lookup`): albums, their songs and
//! artwork. No key is needed; Apple asks for about 20 calls per minute.
use super::model::*;
use serde_json::Value;
use std::collections::HashSet;
use tag_core::Fields;

pub const API: &str = "https://itunes.apple.com";
const LOOKUP_LIMIT: usize = 200;

/// Artwork size from Options → Tag Sources → Online services.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Deserialize)]
pub enum CoverSize {
    #[serde(rename = "600")]
    Medium,
    #[default]
    #[serde(rename = "1200")]
    Large,
    #[serde(rename = "3000")]
    Huge,
}
impl CoverSize {
    fn pixels(self) -> u32 {
        match self {
            CoverSize::Medium => 600,
            CoverSize::Large => 1200,
            CoverSize::Huge => 3000,
        }
    }
}

fn text(value: &Value, key: &str) -> String {
    match value.get(key) {
        Some(Value::String(s)) => s.trim().to_owned(),
        Some(Value::Number(n)) => n.to_string(),
        _ => String::new(),
    }
}
fn number(value: &Value, key: &str) -> Option<u64> {
    value.get(key).and_then(Value::as_u64)
}
fn results(body: &Value) -> &[Value] {
    body.get("results")
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or_default()
}

/// A two-letter store code; anything else falls back to the US store.
pub fn country(value: &str) -> String {
    let value = value.trim();
    if value.len() == 2 && value.chars().all(|c| c.is_ascii_alphabetic()) {
        value.to_ascii_uppercase()
    } else {
        "US".into()
    }
}

pub fn search_query(query: &SearchQuery, store: &str) -> Result<String, String> {
    let store = country(store);
    let words = |values: &[&str]| {
        values
            .iter()
            .map(|v| v.trim())
            .filter(|v| !v.is_empty())
            .collect::<Vec<_>>()
            .join(" ")
    };
    let url = match query.mode {
        SearchMode::ArtistAlbum | SearchMode::Text => {
            let term = if query.mode == SearchMode::Text {
                words(&[&query.text])
            } else {
                words(&[&query.artist, &query.album])
            };
            if term.is_empty() {
                return Err("Enter an artist, album or search text.".into());
            }
            reqwest::Url::parse_with_params(
                &format!("{API}/search"),
                &[
                    ("term", term.as_str()),
                    ("media", "music"),
                    ("entity", "album"),
                    ("limit", &PAGE_SIZE.to_string()),
                    ("country", &store),
                ],
            )
        }
        SearchMode::Barcode => {
            let upc: String = query.text.chars().filter(char::is_ascii_digit).collect();
            if upc.is_empty() {
                return Err("Enter the barcode (UPC or EAN) of the album.".into());
            }
            reqwest::Url::parse_with_params(
                &format!("{API}/lookup"),
                &[("upc", upc.as_str()), ("country", &store)],
            )
        }
        _ => return Err("iTunes searches by artist and album, free text or barcode.".into()),
    };
    url.map(String::from).map_err(|e| e.to_string())
}

/// `1440841180`, `id1440841180` or an Apple Music / iTunes album address.
pub fn release_id(input: &str) -> Result<String, String> {
    let input = input.trim();
    let digits = |s: &str| !s.is_empty() && s.chars().all(|c| c.is_ascii_digit());
    let bare = input.strip_prefix("id").unwrap_or(input);
    if digits(bare) {
        return Ok(bare.to_owned());
    }
    let lower = input.to_ascii_lowercase();
    if lower.contains("apple.com/") && lower.contains("/album/") {
        // The album ID is the last path segment (`…/album/name/1440841180`,
        // older addresses `…/id1440841180`); `?i=` names a song on it.
        let path = lower.split(['?', '#']).next().unwrap_or_default();
        if let Some(last) = path.trim_end_matches('/').rsplit('/').next() {
            let last = last.strip_prefix("id").unwrap_or(last);
            if digits(last) {
                return Ok(last.to_owned());
            }
        }
    }
    Err("Enter an iTunes album ID or an Apple Music album address.".into())
}

pub fn release_url(id: &str, store: &str) -> String {
    format!(
        "{API}/lookup?id={id}&entity=song&limit={LOOKUP_LIMIT}&country={}",
        country(store)
    )
}

/// `…/100x100bb.jpg` → `…/1200x1200bb.jpg`: Apple's artwork addresses name the size.
fn artwork(url: &str, pixels: u32) -> Option<String> {
    let (base, _) = url.rsplit_once('/')?;
    (url.starts_with("https://") && url.contains(".mzstatic.com/"))
        .then(|| format!("{base}/{pixels}x{pixels}bb.jpg"))
}

pub fn parse_search(body: &Value) -> SearchPage {
    let results: Vec<SearchResult> = results(body)
        .iter()
        .filter(|r| text(r, "wrapperType") == "collection")
        .filter_map(|r| {
            let id = text(r, "collectionId");
            (!id.is_empty()).then(|| SearchResult {
                title: text(r, "collectionName"),
                artist: text(r, "artistName"),
                year: year(&text(r, "releaseDate")),
                country: text(r, "country"),
                label: text(r, "copyright"),
                format: text(r, "collectionType"),
                track_count: number(r, "trackCount").map(|n| n as u32),
                thumb: artwork(&text(r, "artworkUrl100"), 250),
                url: text(r, "collectionViewUrl"),
                id,
                ..Default::default()
            })
        })
        .collect();
    // The Search API has no paging: one answer holds every result.
    let total = Some(results.len() as u32);
    SearchPage { results, total }
}

pub fn parse_release(body: &Value, size: CoverSize) -> Result<SourceRelease, String> {
    let album = results(body)
        .iter()
        .find(|r| text(r, "wrapperType") == "collection")
        .ok_or("iTunes has no album with this ID in the chosen store.")?;
    let id = text(album, "collectionId");
    let title = text(album, "collectionName");
    let artist = text(album, "artistName");
    let mut fields = Fields::new();
    put_one(&mut fields, "ALBUM", title.clone());
    put_one(&mut fields, "ALBUMARTIST", artist.clone());
    put_one(&mut fields, "YEAR", year(&text(album, "releaseDate")));
    put_one(&mut fields, "GENRE", text(album, "primaryGenreName"));
    put_one(&mut fields, "COPYRIGHT", text(album, "copyright"));
    put_one(&mut fields, "ITUNESALBUMID", id.clone());
    put_one(&mut fields, "ITUNESARTISTID", text(album, "artistId"));
    let collection_id = number(album, "collectionId").filter(|id| *id > 0);
    let mut seen = HashSet::new();
    let mut songs: Vec<&Value> = results(body)
        .iter()
        .filter(|r| text(r, "wrapperType") == "track" && text(r, "kind") == "song")
        .filter(|r| collection_id.is_some_and(|id| number(r, "collectionId") == Some(id)))
        .filter(|r| number(r, "trackId").is_some_and(|id| id > 0 && seen.insert(id)))
        .collect();
    let count = songs.len();
    match number(album, "trackCount").filter(|total| *total > 0) {
        Some(total) if (count as u64) < total => {
            return Err(format!("iTunes returned {count} of {total} album tracks. The incomplete album cannot be imported."));
        }
        // Apple does not document continuation for this lookup. Count the actual
        // response rows conservatively: the cap may include the collection row.
        None if results(body).len() >= LOOKUP_LIMIT => {
            return Err("iTunes returned a capped track list without a valid album track count. The complete album cannot be verified.".into());
        }
        _ => {}
    }
    songs.sort_by_key(|s| (number(s, "discNumber"), number(s, "trackNumber")));
    let tracks = songs
        .into_iter()
        .map(|song| {
            let disc = number(song, "discNumber").unwrap_or(1).max(1) as u32;
            let track = number(song, "trackNumber").unwrap_or(0) as u32;
            let mut fields = Fields::new();
            let title = text(song, "trackName");
            let artist = text(song, "artistName");
            put_one(&mut fields, "TITLE", title.clone());
            put_one(&mut fields, "ARTIST", artist.clone());
            let total = |key| {
                number(song, key)
                    .map(|n| format!("/{n}"))
                    .unwrap_or_default()
            };
            put_one(
                &mut fields,
                "TRACK",
                format!("{track}{}", total("trackCount")),
            );
            put_one(
                &mut fields,
                "DISCNUMBER",
                format!("{disc}{}", total("discCount")),
            );
            put_one(&mut fields, "ITUNESCATALOGID", text(song, "trackId"));
            match text(song, "trackExplicitness").as_str() {
                "explicit" => put_one(&mut fields, "ITUNESADVISORY", "1"),
                "cleaned" => put_one(&mut fields, "ITUNESADVISORY", "2"),
                _ => {}
            }
            SourceTrack {
                position: if disc > 1 {
                    format!("{disc}-{track}")
                } else {
                    track.to_string()
                },
                disc,
                number: track,
                title,
                artist,
                length: number(song, "trackTimeMillis").map(|ms| ms as f64 / 1000.0),
                fields,
            }
        })
        .collect();
    let small = text(album, "artworkUrl100");
    let covers = artwork(&small, size.pixels())
        .map(|url| SourceCover {
            url,
            thumb: artwork(&small, 250),
            kind: "Front".into(),
            width: Some(size.pixels()),
            height: Some(size.pixels()),
        })
        .into_iter()
        .collect();
    Ok(SourceRelease {
        provider: Provider::Itunes,
        url: text(album, "collectionViewUrl"),
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

    fn fixture() -> Value {
        serde_json::from_str(include_str!("fixtures/itunes-lookup.json")).unwrap()
    }

    /// Independent synthetic album responses, with song totals stored per disc.
    fn lookup_body(disc_sizes: &[usize], advertised: Option<u64>) -> Value {
        let mut album = serde_json::json!({
            "wrapperType":"collection", "collectionType":"Album", "collectionId":42,
            "collectionName":"Raw {value} / action 1: Album", "artistName":"Help",
        });
        if let Some(total) = advertised {
            album["trackCount"] = total.into();
        }
        let mut rows = vec![album];
        let mut id = 1000;
        for (disc, size) in disc_sizes.iter().enumerate() {
            for track in 1..=*size {
                id += 1;
                rows.push(serde_json::json!({
                    "wrapperType":"track", "kind":"song", "collectionId":42, "trackId":id,
                    "discNumber":disc+1, "discCount":disc_sizes.len(), "trackNumber":track,
                    "trackCount":size, "trackName":format!("Song {}-{track}",disc+1),
                    "artistName":"Title", "trackTimeMillis":1234,
                }));
            }
        }
        serde_json::json!({"resultCount":rows.len(), "results":rows})
    }

    fn diagnostic_cases() -> Vec<Value> {
        serde_json::from_str(include_str!(
            "../../../tests/fixtures/itunes-release-diagnostics.json"
        ))
        .unwrap()
    }

    #[test]
    fn incomplete_album_lookup_is_rejected_before_review() {
        for case in diagnostic_cases()
            .into_iter()
            .filter(|case| case["total"].as_u64().is_some_and(|n| n > 0))
        {
            let body = lookup_body(
                &[case["songs"].as_u64().unwrap() as usize],
                case["total"].as_u64(),
            );
            let error = parse_release(&body, CoverSize::Large)
                .expect_err("a truncated album must not reach review with full totals");
            assert_eq!(error, case["error"].as_str().unwrap());
        }
    }

    #[test]
    fn capped_lookup_without_a_positive_album_count_cannot_claim_completeness() {
        for case in diagnostic_cases()
            .into_iter()
            .filter(|case| case["total"].as_u64().unwrap_or(0) == 0)
        {
            let body = lookup_body(
                &[case["songs"].as_u64().unwrap() as usize],
                case["total"].as_u64(),
            );
            assert!(body["results"].as_array().unwrap().len() >= 200);
            let error = parse_release(&body, CoverSize::Large).expect_err(
                "a capped response without a total must not silently become a full album",
            );
            assert_eq!(error, case["error"].as_str().unwrap());
        }
    }

    #[test]
    fn duplicate_foreign_and_invalid_song_ids_cannot_hide_missing_album_tracks() {
        let mut body = lookup_body(&[1], Some(3));
        let song = body["results"][1].clone();
        let mut foreign = song.clone();
        foreign["collectionId"] = 99.into();
        foreign["trackId"] = 2000.into();
        let mut invalid = song.clone();
        invalid["trackId"] = 0.into();
        let mut missing = song.clone();
        missing.as_object_mut().unwrap().remove("trackId");
        let mut wrong_type = song.clone();
        wrong_type["trackId"] = "1002".into();
        body["results"]
            .as_array_mut()
            .unwrap()
            .extend([song, foreign, invalid, missing, wrong_type]);
        let error = parse_release(&body, CoverSize::Large).expect_err(
            "only distinct positive song IDs from this collection establish completeness",
        );
        assert_eq!(
            error,
            "iTunes returned 1 of 3 album tracks. The incomplete album cannot be imported."
        );
    }

    #[test]
    fn complete_lookup_keeps_only_unique_collection_songs_and_preserves_disc_metadata() {
        let mut body = lookup_body(&[100, 100], Some(200));
        let song = body["results"][1].clone();
        let mut foreign = song.clone();
        foreign["collectionId"] = 99.into();
        foreign["trackId"] = 2000.into();
        let mut video = song.clone();
        video["kind"] = "music-video".into();
        video["trackId"] = 3000.into();
        body["results"].as_array_mut().unwrap()[1..].reverse();
        body["results"]
            .as_array_mut()
            .unwrap()
            .extend([song, foreign, video]);
        let release = parse_release(&body, CoverSize::Large).unwrap();
        assert_eq!(release.title, "Raw {value} / action 1: Album");
        assert_eq!(release.tracks.len(), 200);
        assert_eq!(release.tracks[0].title, "Song 1-1");
        assert_eq!(release.tracks[0].fields["TRACK"], ["1/100"]);
        assert_eq!(release.tracks[0].fields["DISCNUMBER"], ["1/2"]);
        assert_eq!(release.tracks[199].position, "2-100");
        assert_eq!(release.tracks[199].fields["TRACK"], ["100/100"]);
        assert_eq!(release.tracks[199].fields["DISCNUMBER"], ["2/2"]);
        assert_eq!(release.tracks[199].length, Some(1.234));
    }

    #[test]
    fn missing_or_zero_album_count_keeps_available_valid_tracks() {
        for advertised in [None, Some(0)] {
            let body = lookup_body(&[2], advertised);
            let release = parse_release(&body, CoverSize::Large).unwrap();
            assert_eq!(release.tracks.len(), 2);
            assert_eq!(release.tracks[0].fields["TRACK"], ["1/2"]);
            assert_eq!(release.tracks[1].title, "Song 1-2");
        }
    }

    #[test]
    fn offline_native_lookup_fixture_is_a_complete_two_song_album() {
        let script = include_str!("../../../tests/e2e/sources-4-fixtures.sh");
        let body = script
            .split_once("cat >\"$site/itunes.apple.com_lookup\" <<'JSON'\n")
            .unwrap()
            .1
            .split_once("\nJSON")
            .unwrap()
            .0;
        let release =
            parse_release(&serde_json::from_str(body).unwrap(), CoverSize::Large).unwrap();
        assert_eq!(release.title, "Fixture Apple Album");
        assert_eq!(
            release
                .tracks
                .iter()
                .map(|track| track.title.as_str())
                .collect::<Vec<_>>(),
            ["Apple First", "Apple Second"]
        );
        assert_eq!(release.tracks[0].fields["TRACK"], ["1/2"]);
        assert_eq!(release.tracks[1].fields["TRACK"], ["2/2"]);
    }

    #[test]
    fn queries_use_the_store_and_reject_unsupported_modes() {
        let query = |mode, text: &str| SearchQuery {
            mode,
            artist: "Daft Punk".into(),
            album: "Discovery".into(),
            text: text.into(),
            ..Default::default()
        };
        let url = search_query(&query(SearchMode::ArtistAlbum, ""), "nl").unwrap();
        assert!(url.starts_with("https://itunes.apple.com/search?term=Daft+Punk+Discovery&media=music&entity=album&limit=50&country=NL"), "{url}");
        assert!(search_query(&query(SearchMode::Text, "x"), "?")
            .unwrap()
            .ends_with("country=US"));
        assert_eq!(
            search_query(&query(SearchMode::Barcode, "0 724384 960650"), "us").unwrap(),
            "https://itunes.apple.com/lookup?upc=0724384960650&country=US"
        );
        assert!(search_query(&query(SearchMode::CatalogNumber, "x"), "us").is_err());
        assert!(search_query(&query(SearchMode::Text, " "), "us").is_err());
    }

    #[test]
    fn album_ids_come_from_numbers_and_addresses() {
        for input in [
            "697194953",
            "id697194953",
            "https://music.apple.com/us/album/discovery/697194953?uo=4",
            "https://music.apple.com/us/album/one-more-time/697194953?i=697195462",
            "https://itunes.apple.com/us/album/discovery/id697194953",
        ] {
            assert_eq!(release_id(input).unwrap(), "697194953", "{input}");
        }
        assert!(release_id("https://music.apple.com/us/artist/daft-punk/5468295").is_err());
        assert!(release_id("abc").is_err());
        assert_eq!(
            release_url("697194953", "gb"),
            "https://itunes.apple.com/lookup?id=697194953&entity=song&limit=200&country=GB"
        );
    }

    #[test]
    fn search_lists_albums_with_thumbnails() {
        let page = parse_search(&fixture());
        assert_eq!(page.total, Some(1));
        let album = &page.results[0];
        assert_eq!(
            (
                album.id.as_str(),
                album.title.as_str(),
                album.artist.as_str()
            ),
            ("697194953", "Discovery", "Daft Punk")
        );
        assert_eq!((album.year.as_str(), album.track_count), ("2001", Some(3)));
        assert!(album.thumb.as_deref().unwrap().ends_with("/250x250bb.jpg"));
    }

    #[test]
    fn release_maps_album_songs_and_high_resolution_artwork() {
        let release = parse_release(&fixture(), CoverSize::Huge).unwrap();
        assert_eq!(release.fields["ALBUM"], ["Discovery"]);
        assert_eq!(release.fields["GENRE"], ["Dance"]);
        assert_eq!(release.fields["COPYRIGHT"], ["℗ 2001 Daft Life Limited"]);
        assert_eq!(release.fields["ITUNESALBUMID"], ["697194953"]);
        assert_eq!(
            release.tracks.len(),
            3,
            "songs only, in disc and track order"
        );
        let first = &release.tracks[0];
        assert_eq!(first.title, "One More Time");
        assert_eq!(first.fields["TRACK"], ["1/3"]);
        assert_eq!(first.fields["DISCNUMBER"], ["1/1"]);
        assert_eq!(first.fields["ITUNESCATALOGID"], ["697195462"]);
        assert_eq!(first.length, Some(320.357));
        assert_eq!(release.tracks[2].fields["ITUNESADVISORY"], ["1"]);
        assert!(!release.tracks[0].fields.contains_key("ITUNESADVISORY"));
        let cover = &release.covers[0];
        assert!(cover.url.ends_with("/3000x3000bb.jpg"), "{}", cover.url);
        assert_eq!(cover.width, Some(3000));
        assert!(release.url.starts_with("https://music.apple.com/"));
        assert!(parse_release(&serde_json::json!({"results": []}), CoverSize::Large).is_err());
    }
}
