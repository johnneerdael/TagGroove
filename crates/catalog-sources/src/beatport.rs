//! Beatport catalog structures/queries adapted from OneTagger/CueRithm.
use crate::{Hit, Page, Release, Track};
use serde_json::Value;
fn string(v: &Value, key: &str) -> String {
    v.get(key)
        .and_then(Value::as_str)
        .unwrap_or_default()
        .into()
}
fn id(v: &Value) -> Result<String, String> {
    v.as_u64()
        .filter(|n| *n > 0)
        .map(|n| n.to_string())
        .ok_or_else(|| "Malformed Beatport catalog ID".into())
}
fn names(v: &Value) -> Vec<String> {
    v.as_array()
        .map(|a| {
            a.iter()
                .map(|v| string(v, "name"))
                .filter(|s| !s.is_empty())
                .collect()
        })
        .unwrap_or_default()
}
fn field(f: &mut std::collections::BTreeMap<String, Vec<String>>, k: &str, v: String) {
    if !v.is_empty() {
        f.insert(k.into(), vec![v]);
    }
}
pub(crate) fn cover(v: &Value) -> Option<String> {
    let raw = string(v, "dynamic_uri");
    let raw = if raw.is_empty() {
        string(v, "uri")
    } else {
        raw.replace("{w}", "1000").replace("{h}", "1000")
    };
    let url = reqwest::Url::parse(&raw).ok()?;
    (url.scheme() == "https"
        && url
            .host_str()
            .is_some_and(|h| h == "beatportimage.com" || h.ends_with(".beatportimage.com")))
    .then(|| url.to_string())
}
fn web_url(kind: &str, value: &Value, id: &str) -> String {
    let supplied = string(value, "slug");
    let slug = if supplied.is_empty() {
        string(value, "name").to_lowercase().replace(' ', "-")
    } else {
        supplied
    };
    let mut url = reqwest::Url::parse("https://www.beatport.com").expect("static catalog URL");
    url.path_segments_mut()
        .expect("catalog URL supports paths")
        .push(kind)
        .push(&slug)
        .push(id);
    url.into()
}
pub(crate) fn parse_release_value(v: &Value) -> Result<Release, String> {
    let id = id(&v["id"])?;
    let title = string(v, "name");
    if title.is_empty() {
        return Err("Malformed Beatport release title".into());
    }
    let mut fields = std::collections::BTreeMap::new();
    for (k, value) in [
        ("PUBLISHER", string(&v["label"], "name")),
        ("CATALOGNUMBER", string(v, "catalog_number")),
        ("DATE", string(v, "publish_date")),
        ("BARCODE", string(v, "upc")),
    ] {
        field(&mut fields, k, value)
    }
    Ok(Release {
        url: web_url("release", v, &id),
        id,
        title,
        artists: names(&v["artists"]),
        cover: cover(&v["image"]),
        fields,
        tracks: vec![],
    })
}
pub fn parse_track(body: &str) -> Result<Track, String> {
    let v = serde_json::from_str(body).map_err(|_| "Malformed Beatport track JSON")?;
    parse_track_value(&v)
}
pub(crate) fn parse_track_value(v: &Value) -> Result<Track, String> {
    let release = parse_release_value(&v["release"])?;
    let id = id(&v["id"])?;
    let title = string(v, "name");
    if title.is_empty() {
        return Err("Malformed Beatport track title".into());
    }
    let mut fields = release.fields;
    for (key, value) in [
        ("ISRC", string(v, "isrc")),
        ("GENRE", string(&v["genre"], "name")),
        ("STYLE", string(&v["sub_genre"], "name")),
        (
            "INITIALKEY",
            string(&v["key"], "name")
                .replace(" Major", "")
                .replace(" Minor", "m"),
        ),
        ("CATALOGNUMBER", string(v, "catalog_number")),
        ("DATE", string(v, "publish_date")),
    ] {
        field(&mut fields, key, value)
    }
    let release_date = string(v, "new_release_date");
    if !release_date.is_empty() {
        field(&mut fields, "DATE", release_date);
    }
    field(&mut fields, "RELEASETIME", string(v, "publish_date"));
    if let Some(bpm) = v["bpm"].as_u64() {
        field(&mut fields, "BPM", bpm.to_string())
    }
    let remixers = names(&v["remixers"]);
    if !remixers.is_empty() {
        fields.insert("MIXARTIST".into(), remixers);
    }
    if let Some(total) = v["release"]["track_count"].as_u64() {
        field(&mut fields, "TRACKTOTAL", total.to_string())
    }
    Ok(Track {
        id: id.clone(),
        url: format!(
            "https://www.beatport.com/track/{}/{}",
            string(v, "slug"),
            id
        ),
        title,
        version: string(v, "mix_name"),
        artists: names(&v["artists"]),
        number: v["number"]
            .as_u64()
            .and_then(|n| u32::try_from(n).ok())
            .unwrap_or(0),
        length: v["length_ms"].as_u64().map(|ms| ms as f64 / 1000.),
        release_id: release.id,
        release_title: release.title,
        fields,
    })
}
pub(crate) fn parse_search(body: &str, releases: bool) -> Result<Page, String> {
    let v: Value = serde_json::from_str(body).map_err(|_| "Malformed Beatport search JSON")?;
    let key = if releases { "releases" } else { "tracks" };
    let rows = v[key]
        .as_array()
        .ok_or("Malformed Beatport search: missing results")?;
    let mut results = vec![];
    for v in rows {
        if releases {
            let r = parse_release_value(v)?;
            results.push(Hit {
                id: format!("release:{}", r.id),
                title: r.title,
                artists: r.artists,
                url: r.url,
                cover: r.cover,
                fields: r.fields,
                track_count: v["track_count"]
                    .as_u64()
                    .and_then(|n| u32::try_from(n).ok()),
                ..Hit::default()
            })
        } else {
            let t = parse_track_value(v)?;
            results.push(Hit {
                id: format!("track:{}", t.id),
                title: crate::track_title(&t),
                artists: t.artists,
                release_id: Some(format!("release:{}", t.release_id)),
                release_title: Some(t.release_title),
                url: t.url,
                cover: cover(&v["release"]["image"]),
                fields: t.fields,
                ..Hit::default()
            });
        }
    }
    let total = v["count"]
        .as_u64()
        .or_else(|| v["total"].as_u64())
        .and_then(|n| u32::try_from(n).ok());
    Ok(Page { results, total })
}
