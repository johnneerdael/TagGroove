//! Traxsource selectors adapted from OneTagger/CueRithm; release acquisition added here.
use crate::{Hit, Page, Release, Track};
use scraper::{ElementRef, Html, Selector};
fn selector(s: &str) -> Selector {
    Selector::parse(s).expect("static CSS selector")
}
fn first<'a>(e: ElementRef<'a>, s: &str) -> Option<ElementRef<'a>> {
    e.select(&selector(s)).next()
}
fn text(e: ElementRef<'_>) -> String {
    e.text()
        .collect::<Vec<_>>()
        .join(" ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}
fn get(e: ElementRef<'_>, s: &str) -> String {
    first(e, s).map(text).unwrap_or_default()
}
fn require(s: String, what: &str) -> Result<String, String> {
    if s.trim().is_empty() {
        Err(format!("Malformed Traxsource page: missing {what}"))
    } else {
        Ok(s)
    }
}
fn id(s: &str) -> Result<String, String> {
    if !s.is_empty()
        && s.chars().all(|c| c.is_ascii_digit())
        && s.parse::<u64>().is_ok_and(|n| n > 0)
    {
        Ok(s.into())
    } else {
        Err("Malformed Traxsource page: invalid catalog ID".into())
    }
}
fn link_id(s: &str, kind: &str) -> Option<String> {
    let url = reqwest::Url::parse("https://www.traxsource.com")
        .ok()?
        .join(s)
        .ok()?;
    if !matches!(
        url.host_str(),
        Some("www.traxsource.com" | "traxsource.com")
    ) {
        return None;
    }
    let mut parts = url.path_segments()?;
    if parts.next()? != kind {
        return None;
    }
    id(parts.next()?).ok()
}
fn absolute(s: &str) -> Option<String> {
    let url = reqwest::Url::parse("https://www.traxsource.com")
        .ok()?
        .join(s)
        .ok()?;
    (url.scheme() == "https"
        && matches!(
            url.host_str(),
            Some(
                "www.traxsource.com"
                    | "traxsource.com"
                    | "geo-static.traxsource.com"
                    | "geo-static2.traxsource.com"
                    | "geo-w-static.traxsource.com"
                    | "w-static.traxsource.com"
            )
        ))
    .then(|| url.to_string())
}
fn content(body: &str) -> Result<String, String> {
    if crate::http::is_challenge(false, body) {
        return Err("Traxsource returned an unresolved challenge".into());
    }
    if body.trim_start().starts_with("<?xml") || body.contains("<root>") {
        let start = body
            .find("<data><![CDATA[")
            .ok_or("Malformed Traxsource XML: missing data")?
            + "<data><![CDATA[".len();
        let end = body[start..]
            .find("]]></data>")
            .ok_or("Malformed Traxsource XML: incomplete data")?
            + start;
        return Ok(body[start..end].into());
    }
    Ok(body.into())
}
fn field(fields: &mut std::collections::BTreeMap<String, Vec<String>>, key: &str, value: String) {
    if !value.is_empty() {
        fields.insert(key.into(), vec![value]);
    }
}
fn names(e: ElementRef<'_>, s: &str) -> Vec<String> {
    e.select(&selector(s))
        .map(text)
        .filter(|s| !s.is_empty())
        .collect()
}
fn track(row: ElementRef<'_>, release_id: &str, release_title: &str) -> Result<Track, String> {
    let a = first(row, ".title a[href]").ok_or("Malformed Traxsource track: missing title link")?;
    let href = a
        .attr("href")
        .ok_or("Malformed Traxsource track: missing URL")?;
    let track_id = row
        .attr("data-trid")
        .map(id)
        .transpose()?
        .or_else(|| link_id(href, "track"))
        .ok_or("Malformed Traxsource track: missing ID")?;
    if let Some(link) = link_id(href, "track") {
        if link != track_id {
            return Err("Traxsource track identity mismatch".into());
        }
    }
    let title = require(text(a), "track title")?;
    let version = first(row, ".version")
        .map(|v| {
            v.children()
                .filter_map(|n| n.value().as_text())
                .map(|t| t.to_string())
                .collect::<String>()
                .trim()
                .to_owned()
        })
        .unwrap_or_default();
    let duration = get(row, ".duration")
        .trim_matches(['(', ')', ' '])
        .to_owned();
    let length = if duration.is_empty() {
        None
    } else {
        let parts: Vec<_> = duration.split(':').collect();
        if parts.len() != 2 {
            return Err("Malformed Traxsource duration".into());
        }
        let min = parts[0]
            .parse::<u32>()
            .map_err(|_| "Malformed Traxsource duration")?;
        let sec = parts[1]
            .parse::<u32>()
            .map_err(|_| "Malformed Traxsource duration")?;
        if sec >= 60 {
            return Err("Malformed Traxsource duration".into());
        }
        Some(f64::from(min) * 60. + f64::from(sec))
    };
    let number = get(row, ".tnum").parse().unwrap_or(0);
    let mut fields = std::collections::BTreeMap::new();
    field(&mut fields, "GENRE", get(row, ".genre"));
    field(&mut fields, "PUBLISHER", get(row, ".label"));
    field(
        &mut fields,
        "DATE",
        get(row, ".r-date").replace("Pre-order for ", ""),
    );
    if let Some(kb) = first(row, ".key-bpm") {
        let values: Vec<_> = kb.text().map(str::trim).filter(|s| !s.is_empty()).collect();
        if let Some(key) = values.first() {
            field(
                &mut fields,
                "INITIALKEY",
                key.replace("maj", "").replace("min", "m"),
            );
        }
        if let Some(bpm) = values.get(1) {
            if bpm.parse::<u16>().is_ok() {
                field(&mut fields, "BPM", (*bpm).into());
            }
        }
    }
    let release_id = if release_id.is_empty() {
        row.select(&selector("[data-cart]"))
            .find_map(|a| {
                a.attr("data-cart")
                    .and_then(|s| s.split("title_id:").nth(1))
                    .map(|s| {
                        s.trim_start()
                            .chars()
                            .take_while(char::is_ascii_digit)
                            .collect::<String>()
                    })
            })
            .filter(|s| !s.is_empty())
            .map(|s| id(&s))
            .transpose()?
            .or_else(|| link_id(href, "title"))
            .unwrap_or_default()
    } else {
        release_id.into()
    };
    if link_id(href, "title").is_some_and(|linked| linked != release_id) {
        return Err("Traxsource track release identity mismatch".into());
    }
    Ok(Track {
        id: track_id,
        title,
        version,
        artists: names(row, ".artists a"),
        number,
        length,
        release_id,
        release_title: release_title.into(),
        url: absolute(href).ok_or("Invalid Traxsource track URL")?,
        fields,
    })
}
pub fn parse_release(body: &str, expected_id: &str) -> Result<Release, String> {
    let body = content(body)?;
    let doc = Html::parse_document(&body);
    let root = doc
        .select(&selector("#ttlPageCont"))
        .next()
        .ok_or("Malformed Traxsource release: missing release container")?;
    let release_id = id(root
        .attr("data-tid")
        .ok_or("Malformed Traxsource release: missing ID")?)?;
    if release_id != expected_id {
        return Err("Traxsource release identity mismatch".into());
    }
    let block =
        first(root, ".ttl-block").ok_or("Malformed Traxsource release: missing metadata")?;
    let title = require(get(block, ".title"), "release title")?;
    let artists = first(block, ".artists")
        .map(|e| {
            let n = names(e, "a");
            if n.is_empty() {
                vec![text(e)]
            } else {
                n
            }
        })
        .unwrap_or_default();
    let cover = first(block, ".t-image img")
        .and_then(|e| e.attr("src"))
        .and_then(absolute);
    let mut fields = std::collections::BTreeMap::new();
    field(&mut fields, "PUBLISHER", get(block, ".com-label"));
    let cat = get(block, ".cat-rdate");
    let parts: Vec<_> = cat.split('|').map(str::trim).collect();
    if let Some(c) = parts.first() {
        field(&mut fields, "CATALOGNUMBER", (*c).into());
    }
    if let Some(d) = parts.get(1) {
        field(&mut fields, "DATE", (*d).into());
    }
    let list =
        first(root, "#trkListCont").ok_or("Malformed Traxsource release: missing tracklist")?;
    let mut tracks = Vec::new();
    let mut ids = std::collections::BTreeSet::new();
    let mut numbers = std::collections::BTreeSet::new();
    for row in list.select(&selector(".trk-row")) {
        let t = track(row, &release_id, &title)?;
        if !ids.insert(t.id.clone()) || t.number == 0 || !numbers.insert(t.number) {
            return Err("Malformed Traxsource release: duplicate or missing track position".into());
        }
        tracks.push(t)
    }
    if tracks.is_empty() {
        return Err("Malformed Traxsource release: empty tracklist".into());
    }
    if let Some(all) =
        first(block, ".com-play-all[data-tracks]").and_then(|a| a.attr("data-tracks"))
    {
        let expected = all.split(',').map(str::trim).collect::<Vec<_>>();
        if expected.len() != tracks.len() || expected.iter().zip(&tracks).any(|(id, t)| *id != t.id)
        {
            return Err("Incomplete Traxsource release tracklist".into());
        }
    }
    if numbers.iter().copied().ne(1..=tracks.len() as u32) {
        return Err("Incomplete Traxsource release track positions".into());
    }
    let total = tracks.len().to_string();
    for track in &mut tracks {
        field(&mut track.fields, "TRACKTOTAL", total.clone());
    }
    Ok(Release {
        id: release_id.clone(),
        title,
        artists,
        url: format!("https://www.traxsource.com/title/{release_id}"),
        cover,
        fields,
        tracks,
    })
}
pub fn release_id_from_track(body: &str) -> Result<String, String> {
    let body = content(body)?;
    let doc = Html::parse_document(&body);
    doc.select(&selector(".track-page .ttl-info a[href]"))
        .filter_map(|a| a.attr("href"))
        .find_map(|href| link_id(href, "title"))
        .ok_or_else(|| "Malformed Traxsource track: missing exact release link".into())
}
pub fn parse_search(body: &str, releases: bool) -> Result<Page, String> {
    let body = content(body)?;
    let doc = Html::parse_document(&body);
    let root = doc.root_element();
    if root
        .select(&selector(".search-res-cont h2"))
        .any(|e| text(e).starts_with("No results found for "))
    {
        return Ok(Page {
            results: vec![],
            total: Some(0),
        });
    }
    let list = first(
        root,
        if releases {
            ".release-grid"
        } else {
            "#searchTrackList"
        },
    )
    .ok_or("Malformed Traxsource page: missing search results")?;
    let mut results = Vec::new();
    if releases {
        for item in list.select(&selector(".grid-item")) {
            let a = first(item, ".links .com-title")
                .ok_or("Malformed Traxsource release result: missing title")?;
            let href = a
                .attr("href")
                .ok_or("Malformed Traxsource release result: missing link")?;
            let release_id =
                link_id(href, "title").ok_or("Invalid Traxsource release result identity")?;
            if item.attr("data-tid").is_some_and(|id| id != release_id) {
                return Err("Traxsource release result identity mismatch".into());
            }
            let title = require(text(a), "release title")?;
            let artists = first(item, ".links")
                .map(|e| {
                    e.children()
                        .filter_map(|n| n.value().as_text())
                        .map(|t| t.trim())
                        .filter(|t| !t.is_empty())
                        .map(str::to_owned)
                        .collect()
                })
                .unwrap_or_default();
            let mut fields = std::collections::BTreeMap::new();
            field(&mut fields, "PUBLISHER", get(item, ".com-label"));
            results.push(Hit {
                id: format!("release:{release_id}"),
                title,
                artists,
                url: absolute(href).ok_or("Invalid Traxsource release URL")?,
                cover: first(item, "img")
                    .and_then(|e| e.attr("src"))
                    .and_then(absolute),
                fields,
                ..Hit::default()
            });
        }
    } else {
        for row in list.select(&selector(".trk-row")) {
            let t = track(row, "", "")?;
            results.push(Hit {
                id: format!("track:{}", t.id),
                title: crate::track_title(&t),
                artists: t.artists,
                release_id: (!t.release_id.is_empty()).then(|| format!("release:{}", t.release_id)),
                url: t.url,
                fields: t.fields,
                cover: first(row, ".thumb img")
                    .and_then(|e| e.attr("src"))
                    .and_then(absolute),
                ..Hit::default()
            });
        }
    }
    // An explicit results container without rows is not evidence of no matches.
    if results.is_empty() {
        return Err("Malformed Traxsource page: empty results container".into());
    }
    let total = get(
        root,
        if releases {
            "#tab-titles"
        } else {
            "#tab-tracks"
        },
    )
    .rsplit_once('(')
    .and_then(|(_, n)| n.trim_end_matches(')').replace(',', "").parse().ok());
    Ok(Page { results, total })
}

pub(crate) fn track_detail(
    body: &str,
    expected_id: &str,
) -> Result<(String, std::collections::BTreeMap<String, Vec<String>>), String> {
    let release_id = release_id_from_track(body)?;
    let body = content(body)?;
    let doc = Html::parse_document(&body);
    let root = doc
        .select(&selector(".track-page"))
        .next()
        .ok_or("Malformed Traxsource track page")?;
    let buttons = first(root, ".btns").ok_or("Malformed Traxsource track: missing identity")?;
    let track_id = buttons
        .value()
        .classes()
        .find_map(|class| class.strip_prefix("ptk-"))
        .ok_or("Malformed Traxsource track: missing identity")?;
    if track_id != expected_id {
        return Err("Traxsource track identity mismatch".into());
    }
    let mut fields = std::collections::BTreeMap::new();
    for row in root.select(&selector(".tr-det-tbl tr")) {
        let label = get(row, "td:first-child")
            .trim_end_matches(':')
            .to_ascii_lowercase();
        let value = get(row, "td.det");
        match label.as_str() {
            "key" => field(
                &mut fields,
                "INITIALKEY",
                value.replace("maj", "").replace("min", "m"),
            ),
            "bpm" => {
                if value.parse::<u16>().is_ok_and(|n| n > 0) {
                    field(&mut fields, "BPM", value)
                }
            }
            "isrc" => field(&mut fields, "ISRC", value),
            "genre" => field(&mut fields, "GENRE", value),
            "label" => field(&mut fields, "PUBLISHER", value),
            "released" => field(&mut fields, "DATE", value),
            _ => {}
        }
    }
    Ok((release_id, fields))
}
