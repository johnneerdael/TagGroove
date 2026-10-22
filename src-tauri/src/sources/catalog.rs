//! Native metadata-only catalog adapter. GPL-3.0-only; see crates/catalog-sources/COPYING.
use super::{http, model::*};
use catalog_sources::{Catalog, Release};
use std::sync::{Arc, OnceLock};
fn state() -> Arc<catalog_sources::http::State> {
    static STATE: OnceLock<Arc<catalog_sources::http::State>> = OnceLock::new();
    // Blocks are remembered in the registry every tag source shares.
    STATE
        .get_or_init(|| {
            Arc::new(catalog_sources::http::State::shared(
                catalog_sources::solver::global_blocked(),
            ))
        })
        .clone()
}

fn provider(provider: Provider) -> Result<catalog_sources::Provider, String> {
    match provider {
        Provider::Beatport => Ok(catalog_sources::Provider::Beatport),
        Provider::Traxsource => Ok(catalog_sources::Provider::Traxsource),
        _ => Err("This source is not a native catalog provider".into()),
    }
}
fn catalog(channel: http::Channel, ticket: u64) -> Result<Catalog, String> {
    let network = crate::settings::network();
    // Follow redirects only within the original catalog origin. Tokens cannot
    // move to another catalog host, image server, or the optional solver.
    let redirect = reqwest::redirect::Policy::custom(|attempt| {
        if attempt.previous().len() >= 5 {
            return attempt.error("Catalog redirect limit exceeded");
        }
        if attempt
            .previous()
            .first()
            .is_some_and(|first| first.origin() == attempt.url().origin())
        {
            attempt.follow()
        } else {
            attempt.stop()
        }
    });
    let client = http::builder(&network)?
        .redirect(redirect)
        .build()
        .map_err(|_| "Could not create catalog network client")?;
    let solver_client = http::builder(&network)?
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|_| "Could not create solver network client")?;
    let solver = super::solver::config()?;
    Ok(Catalog::new(catalog_sources::http::Http::new(
        client,
        solver_client,
        state(),
        solver,
        Arc::new(move || http::stale(channel, ticket)),
        Arc::new(http::catalog_route),
    )))
}
fn normalized_fields(mut fields: tag_core::Fields) -> tag_core::Fields {
    if let Some(date) = fields.remove("DATE") {
        fields.entry("YEAR".into()).or_insert(date);
    }
    fields
}
fn map_release(provider: Provider, input: &str, release: Release) -> Result<SourceRelease, String> {
    let prefix = match provider {
        Provider::Beatport => "BEATPORT",
        Provider::Traxsource => "TRAXSOURCE",
        _ => return Err("This source is not a native catalog provider".into()),
    };
    let mut fields = normalized_fields(release.fields);
    put_one(&mut fields, "ALBUM", release.title.clone());
    put(&mut fields, "ALBUMARTIST", release.artists.clone());
    put_one(
        &mut fields,
        &format!("{prefix}_RELEASE_ID"),
        release.id.clone(),
    );
    let single = input.starts_with("track:");
    let url = if single {
        release
            .tracks
            .first()
            .map(|t| t.url.clone())
            .unwrap_or_else(|| release.url.clone())
    } else {
        release.url.clone()
    };
    let total = (!single).then_some(release.tracks.len());
    let mut tracks = vec![];
    for track in release.tracks {
        if track.release_id != release.id || track.number == 0 {
            return Err("Catalog track has an invalid release identity or track number".into());
        }
        let title = catalog_sources::track_title(&track);
        let artist = track.artists.join(", ");
        let mut fields = normalized_fields(track.fields);
        put_one(&mut fields, "TITLE", title.clone());
        put(&mut fields, "ARTIST", track.artists);
        put_one(&mut fields, "ALBUM", release.title.clone());
        put_one(&mut fields, &format!("{prefix}_TRACK_ID"), track.id);
        put_one(
            &mut fields,
            &format!("{prefix}_RELEASE_ID"),
            release.id.clone(),
        );
        let count = fields
            .remove("TRACKTOTAL")
            .and_then(|v| v.into_iter().next())
            .filter(|v| v.parse::<usize>().is_ok())
            .or_else(|| total.map(|n| n.to_string()));
        put_one(
            &mut fields,
            "TRACK",
            match count {
                Some(n) => format!("{}/{n}", track.number),
                None => track.number.to_string(),
            },
        );
        put_one(&mut fields, "DISCNUMBER", "1/1");
        tracks.push(SourceTrack {
            position: track.number.to_string(),
            disc: 1,
            number: track.number,
            title,
            artist,
            length: track.length,
            fields,
        });
    }
    if tracks.is_empty() {
        return Err("The catalog release has no tracks".into());
    }
    let covers = release
        .cover
        .into_iter()
        .map(|url| SourceCover {
            thumb: Some(url.clone()),
            url,
            kind: "Front".into(),
            width: None,
            height: None,
        })
        .collect();
    Ok(SourceRelease {
        provider,
        id: input.into(),
        url,
        preview_url: None,
        title: release.title,
        artist: release.artists.join(", "),
        fields,
        tracks,
        covers,
        more_covers: false,
    })
}
pub async fn search(
    provider_id: Provider,
    query: &SearchQuery,
    ticket: u64,
) -> Result<SearchPage, String> {
    let provider = provider(provider_id)?;
    let (releases, terms) = match query.mode {
        SearchMode::ArtistTitle => (false, vec![query.artist.trim(), query.title.trim()]),
        SearchMode::Release | SearchMode::ArtistAlbum => {
            (true, vec![query.artist.trim(), query.album.trim()])
        }
        SearchMode::Text => (false, vec![query.text.trim()]),
        _ => return Err("Catalog sources search by artist and title or release".into()),
    };
    let term = terms
        .into_iter()
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join(" ");
    let page = catalog(http::Channel::Search, ticket)?
        .search(provider, releases, &term, query.page)
        .await?;
    let field = |fields: &tag_core::Fields, key: &str| {
        fields.get(key).map(|v| v.join(", ")).unwrap_or_default()
    };
    Ok(SearchPage {
        total: page.total,
        results: page
            .results
            .into_iter()
            .map(|hit| SearchResult {
                id: hit.id,
                title: hit.title,
                artist: hit.artists.join(", "),
                year: field(&hit.fields, "DATE").chars().take(4).collect(),
                label: field(&hit.fields, "PUBLISHER"),
                catalog_number: field(&hit.fields, "CATALOGNUMBER"),
                barcode: field(&hit.fields, "BARCODE"),
                format: if releases { "Release" } else { "Track" }.into(),
                thumb: hit.cover,
                url: hit.url,
                release_id: hit.release_id,
                release_title: hit.release_title,
                track_count: hit.track_count,
                ..Default::default()
            })
            .collect(),
    })
}
pub async fn release(
    provider_id: Provider,
    input: &str,
    ticket: u64,
) -> Result<SourceRelease, String> {
    let provider = provider(provider_id)?;
    let input = catalog_sources::normalize_id(provider, input)?;
    let release = catalog(http::Channel::Release, ticket)?
        .release(provider, &input)
        .await?;
    map_release(provider_id, &input, release)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn single_track_mapping_keeps_release_identity_and_original_number() {
        let release = Release {
            id: "42".into(),
            title: "Night Signals".into(),
            artists: vec!["Example Artist".into()],
            tracks: vec![catalog_sources::Track {
                id: "82".into(),
                title: "Second".into(),
                version: "Extended Mix".into(),
                artists: vec!["Example Artist".into()],
                release_id: "42".into(),
                release_title: "Night Signals".into(),
                number: 2,
                fields: tag_core::Fields::from([("TRACKTOTAL".into(), vec!["14".into()])]),
                ..Default::default()
            }],
            ..Default::default()
        };
        let r = map_release(Provider::Traxsource, "track:82", release).unwrap();
        assert_eq!(r.id, "track:82");
        assert_eq!(r.fields["TRAXSOURCE_RELEASE_ID"], vec!["42"]);
        assert_eq!(r.fields["ALBUM"], vec!["Night Signals"]);
        assert_eq!(r.tracks[0].number, 2);
        assert_eq!(r.tracks[0].position, "2");
        assert_eq!(r.tracks[0].fields["TRACK"], vec!["2/14"]);
        assert_eq!(r.tracks[0].fields["TRAXSOURCE_TRACK_ID"], vec!["82"]);
        assert_eq!(r.tracks[0].title, "Second (Extended Mix)");
    }
    #[test]
    fn solver_timeout_accepts_saved_string_or_numeric_values() {
        for timeout in [serde_json::json!(120), serde_json::json!("120")] {
            let cfg = crate::sources::solver::parse(Some(
                &serde_json::json!({"endpoint":"http://127.0.0.1:8191","timeout_seconds":timeout}),
            ))
            .unwrap();
            assert_eq!(cfg.timeout_seconds, 120);
            assert_eq!(
                cfg.api_url().unwrap().unwrap().as_str(),
                "http://127.0.0.1:8191/v1"
            );
        }
        for timeout in [
            serde_json::json!(0),
            serde_json::json!(301),
            serde_json::json!("bad"),
        ] {
            assert!(crate::sources::solver::parse(Some(
                &serde_json::json!({"timeout_seconds":timeout})
            ))
            .is_err());
        }
    }
}
