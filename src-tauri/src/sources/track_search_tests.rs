#![cfg(test)]
use super::{deezer, itunes, lastfm, musicbrainz};
use serde_json::json;

const RECORDING: &str = "11111111-1111-4111-8111-111111111111";
const RELEASE: &str = "22222222-2222-4222-8222-222222222222";

#[test]
fn native_search_dispatch_parses_track_answers_instead_of_album_answers() {
    use super::model::{Provider, SearchMode};
    for (provider, body) in [
        (
            Provider::Musicbrainz,
            json!({"recordings":[{"id":RECORDING,"title":"Track","artist-credit":[{"name":"Artist"}]}]}),
        ),
        (
            Provider::Itunes,
            json!({"results":[{"wrapperType":"track","kind":"song","trackId":7,"collectionId":44,"trackName":"Track","artistName":"Artist"}]}),
        ),
        (
            Provider::Deezer,
            json!({"data":[{"id":7,"title":"Track","artist":{"name":"Artist"},"album":{"id":44}}]}),
        ),
        (
            Provider::Lastfm,
            json!({"results":{"trackmatches":{"track":[{"name":"Track","artist":"Artist"}]}}}),
        ),
    ] {
        let page = super::parse_search_response(provider, SearchMode::ArtistTitle, &body).unwrap();
        assert_eq!(page.results[0].title, "Track");
        assert!(page.results[0].track_id.is_some());
    }
    let album=super::parse_search_response(Provider::Itunes,SearchMode::ArtistAlbum,&json!({"results":[{"wrapperType":"collection","collectionId":44,"collectionName":"Compilation"}]})).unwrap();
    assert_eq!(album.results[0].title, "Compilation");
    assert_eq!(album.results[0].track_id, None);
}

#[test]
fn musicbrainz_track_search_keeps_recording_and_release_identity() {
    let body = json!({"count":1,"recordings":[{"id":RECORDING,"title":"One (Extended Mix)","artist-credit":[{"name":"Artist A","artist":{"id":"artist-a","name":"Artist A"}}],"releases":[{"id":RELEASE,"title":"Compilation","date":"2025-03-01"}]}]});
    let page = musicbrainz::parse_track_search(&body).unwrap();
    assert_eq!(page.results.len(), 1);
    let found = &page.results[0];
    assert_eq!(found.id, RELEASE);
    assert_eq!(found.track_id.as_deref(), Some(RECORDING));
    assert_eq!(found.release_id.as_deref(), Some(RELEASE));
    assert_eq!(found.release_title.as_deref(), Some("Compilation"));
    assert_eq!(found.artist, "Artist A");
    assert_eq!(found.title, "One (Extended Mix)");
}

#[test]
fn musicbrainz_recordings_without_an_album_do_not_invent_album_artist_or_position() {
    let body = json!({"id":RECORDING,"title":"One (Extended Mix)","length":300000,"artist-credit":[{"name":"Artist A","artist":{"name":"Artist A"}}],"isrcs":["NLZ992500001"]});
    let found =
        musicbrainz::parse_track_search(&json!({"count":1,"recordings":[body.clone()]})).unwrap();
    assert_eq!(found.results[0].id, format!("recording:{RECORDING}"));
    assert_eq!(found.results[0].release_id, None);
    let loaded = musicbrainz::parse_recording(&body, musicbrainz::CoverSize::Medium).unwrap();
    assert!(!loaded.fields.contains_key("ALBUM"));
    assert!(!loaded.fields.contains_key("ALBUMARTIST"));
    assert_eq!(loaded.tracks[0].fields["ARTIST"], ["Artist A"]);
    assert_eq!(loaded.tracks[0].fields["MUSICBRAINZ_TRACKID"], [RECORDING]);
    assert_eq!(loaded.tracks[0].fields["ISRC"], ["NLZ992500001"]);
    assert!(!loaded.tracks[0].fields.contains_key("TRACK"));
    assert_eq!(loaded.tracks[0].length, Some(300.0));
}

#[test]
fn itunes_song_search_and_album_loading_retain_each_performing_artist() {
    let first = json!({"wrapperType":"track","kind":"song","trackId":7,"collectionId":44,"trackName":"One (Extended Mix)","artistName":"Artist A","collectionName":"Compilation","trackNumber":1,"trackCount":2,"discNumber":1,"discCount":1,"trackTimeMillis":300000});
    let page =
        itunes::parse_track_search(&json!({"resultCount":1,"results":[first.clone()]})).unwrap();
    assert_eq!(page.results[0].id, "44");
    assert_eq!(page.results[0].track_id.as_deref(), Some("7"));
    assert_eq!(page.results[0].artist, "Artist A");
    assert_eq!(page.results[0].release_id.as_deref(), Some("44"));
    let second = json!({"wrapperType":"track","kind":"song","trackId":8,"collectionId":44,"trackName":"Two","artistName":"Artist B","trackNumber":2,"trackCount":2,"discNumber":1,"discCount":1});
    let loaded = itunes::parse_release(&json!({"results":[{"wrapperType":"collection","collectionId":44,"collectionName":"Compilation","artistName":"Various Artists","trackCount":2},first,second]}),itunes::CoverSize::Medium).unwrap();
    assert_eq!(loaded.fields["ALBUMARTIST"], ["Various Artists"]);
    assert_eq!(loaded.tracks[0].fields["ARTIST"], ["Artist A"]);
    assert_eq!(loaded.tracks[1].fields["ARTIST"], ["Artist B"]);
    assert_eq!(loaded.tracks[0].fields["ITUNESTRACKID"], ["7"]);
}

#[test]
fn deezer_track_detail_retains_its_original_album_position_and_extra_fields() {
    let detail = json!({"id":12,"title":"One (Extended Mix)","artist":{"id":1,"name":"Artist A"},"album":{"id":44,"title":"Compilation"},"track_position":7,"disk_number":1,"duration":300,"bpm":122.5,"isrc":"NLZ992500001"});
    let page = deezer::parse_track_search(&json!({"total":1,"data":[detail.clone()]})).unwrap();
    assert_eq!(page.results[0].id, "track:12");
    assert_eq!(page.results[0].track_id.as_deref(), Some("12"));
    assert_eq!(page.results[0].release_id.as_deref(), Some("44"));
    let tracks: Vec<_> = (1..=7).map(|n|json!({"id":n+5,"title":if n==7 {"One (Extended Mix)"} else {"Other track"},"track_position":n,"disk_number":1,"artist":{"name":if n==7 {"Artist A"} else {"Other artist"}}})).collect();
    let album =
        json!({"id":44,"title":"Compilation","artist":{"name":"Various Artists"},"nb_tracks":7});
    let loaded = deezer::parse_track(
        &detail,
        &album,
        &json!({"data":tracks}),
        deezer::CoverSize::Big,
    )
    .unwrap();
    assert_eq!(loaded.tracks.len(), 1);
    assert_eq!(loaded.fields["ALBUMARTIST"], ["Various Artists"]);
    assert_eq!(loaded.tracks[0].fields["ARTIST"], ["Artist A"]);
    assert_eq!(loaded.tracks[0].fields["TRACK"], ["7/7"]);
    assert_eq!(loaded.tracks[0].fields["DEEZER_TRACK_ID"], ["12"]);
    assert_eq!(loaded.tracks[0].fields["BPM"], ["122.5"]);
    assert_eq!(loaded.tracks[0].fields["ISRC"], ["NLZ992500001"]);
    let wrong_album = json!({"id":45,"title":"Wrong edition","artist":{"name":"Various Artists"}});
    assert!(deezer::parse_track(
        &detail,
        &wrong_album,
        &json!({"data":[]}),
        deezer::CoverSize::Big
    )
    .is_err());
}

#[test]
fn lastfm_track_identity_is_key_free_and_album_artist_stays_distinct() {
    let found = lastfm::parse_track_search(&json!({"results":{"trackmatches":{"track":[{"name":"One (Extended Mix)","artist":"Artist A","url":"https://www.last.fm/music/Artist+A/_/One"}]},"opensearch:totalResults":"1"}})).unwrap();
    let id = &found.results[0].id;
    let url = reqwest::Url::parse(&lastfm::track_info_url(id, "secret-test-key").unwrap()).unwrap();
    let query: std::collections::HashMap<_, _> = url.query_pairs().collect();
    assert_eq!(query["method"], "track.getInfo");
    assert_eq!(query["artist"], "Artist A");
    assert_eq!(query["track"], "One (Extended Mix)");
    assert!(!id.contains("secret-test-key"));
    let loaded = lastfm::parse_track(&json!({"track":{"name":"One (Extended Mix)","duration":"300000","artist":{"name":"Artist A"},"album":{"title":"Compilation","artist":"Various Artists","@attr":{"position":"7"}},"toptags":{"tag":[{"name":"deep house"},{"name":"123"}]}}}),id,3).unwrap();
    assert_eq!(loaded.fields["ALBUMARTIST"], ["Various Artists"]);
    assert_eq!(loaded.tracks[0].fields["ARTIST"], ["Artist A"]);
    assert_eq!(loaded.tracks[0].fields["TRACK"], ["7"]);
    assert_eq!(loaded.tracks[0].fields["GENRE"], ["Deep House"]);
    assert_eq!(loaded.tracks[0].length, Some(300.0));
}

#[test]
fn malformed_search_payloads_are_not_accepted_as_empty_track_catalogs() {
    assert!(musicbrainz::parse_track_search(&json!({"unexpected":true})).is_err());
    assert!(itunes::parse_track_search(&json!({"unexpected":true})).is_err());
    assert!(deezer::parse_track_search(&json!({"error":{"code":4,"message":"Busy"}})).is_err());
    assert!(lastfm::parse_track_search(&json!({"error":29,"message":"Rate limited"})).is_err());
    assert!(
        musicbrainz::parse_track_search(&json!({"count":0,"recordings":[]}))
            .unwrap()
            .results
            .is_empty()
    );
    assert!(
        itunes::parse_track_search(&json!({"resultCount":0,"results":[]}))
            .unwrap()
            .results
            .is_empty()
    );
    assert!(deezer::parse_track_search(&json!({"total":0,"data":[]}))
        .unwrap()
        .results
        .is_empty());
    assert!(
        lastfm::parse_track_search(&json!({"results":{"trackmatches":{"track":[]}}}))
            .unwrap()
            .results
            .is_empty()
    );
}
