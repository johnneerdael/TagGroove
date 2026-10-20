//! Metadata acquisition adapted from OneTagger and CueRithm. GPL-3.0-only.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Track {
    pub id: String,
    pub title: String,
    pub version: String,
    pub artists: Vec<String>,
    pub number: u32,
    pub length: Option<f64>,
    pub release_id: String,
    pub release_title: String,
    pub url: String,
    pub fields: std::collections::BTreeMap<String, Vec<String>>,
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Release {
    pub id: String,
    pub title: String,
    pub artists: Vec<String>,
    pub url: String,
    pub cover: Option<String>,
    pub fields: std::collections::BTreeMap<String, Vec<String>>,
    pub tracks: Vec<Track>,
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Hit {
    pub id: String,
    pub title: String,
    pub artists: Vec<String>,
    pub release_id: Option<String>,
    pub release_title: Option<String>,
    pub url: String,
    pub cover: Option<String>,
    pub fields: std::collections::BTreeMap<String, Vec<String>>,
    pub track_count: Option<u32>,
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Page {
    pub results: Vec<Hit>,
    pub total: Option<u32>,
}
mod beatport;
pub mod http;
mod traxsource;
pub use beatport::parse_track as parse_beatport_track;
pub use traxsource::{
    parse_release as parse_traxsource_release, parse_search as parse_traxsource_search,
};
#[cfg(test)]
mod tests;

pub fn track_title(t: &Track) -> String {
    if t.version.is_empty() {
        t.title.clone()
    } else {
        format!("{} ({})", t.title, t.version)
    }
}

mod catalog;
pub use catalog::{normalize_id, Catalog, Endpoints, Provider};

mod auth;
