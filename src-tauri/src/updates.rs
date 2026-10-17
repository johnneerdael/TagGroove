//! Help → Check for updates and Options → Updates
//! (https://docs.mp3tag.de/customization/options/updates/): compare the running
//! version with the GitHub releases of TagGroove. Nothing is downloaded or
//! installed; the release page opens in the browser on request.
use crate::sources::http;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

const RELEASES_API: &str = "https://api.github.com/repos/johnneerdael/TagGroove/releases?per_page=30";
pub const RELEASES_PAGE: &str = "https://github.com/johnneerdael/TagGroove/releases/";
const MAX_BYTES: usize = 2 * 1024 * 1024;

/// A semantic version (`1.2.3`, `v1.2.3-beta.1`); build metadata is ignored.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Version {
    numbers: [u64; 3],
    pre: Vec<String>,
}
impl Version {
    pub fn parse(text: &str) -> Option<Self> {
        let text = text.trim().trim_start_matches(['v', 'V']);
        let text = text.split('+').next()?;
        let (core, pre) = match text.split_once('-') {
            Some((core, pre)) => (core, pre.split('.').map(str::to_string).collect()),
            None => (text, Vec::new()),
        };
        let mut numbers = [0; 3];
        let parts: Vec<&str> = core.split('.').collect();
        if parts.is_empty() || parts.len() > 3 {
            return None;
        }
        for (slot, part) in numbers.iter_mut().zip(&parts) {
            *slot = part.parse().ok()?;
        }
        if pre.iter().any(String::is_empty) {
            return None;
        }
        Some(Self { numbers, pre })
    }
    pub fn prerelease(&self) -> bool {
        !self.pre.is_empty()
    }
}
impl Ord for Version {
    fn cmp(&self, other: &Self) -> Ordering {
        self.numbers.cmp(&other.numbers).then_with(|| {
            match (self.pre.is_empty(), other.pre.is_empty()) {
                (true, true) => Ordering::Equal,
                (true, false) => Ordering::Greater,
                (false, true) => Ordering::Less,
                (false, false) => {
                    for (a, b) in self.pre.iter().zip(&other.pre) {
                        let order = match (a.parse::<u64>(), b.parse::<u64>()) {
                            (Ok(a), Ok(b)) => a.cmp(&b),
                            (Ok(_), Err(_)) => Ordering::Less,
                            (Err(_), Ok(_)) => Ordering::Greater,
                            (Err(_), Err(_)) => a.cmp(b),
                        };
                        if order != Ordering::Equal {
                            return order;
                        }
                    }
                    self.pre.len().cmp(&other.pre.len())
                }
            }
        })
    }
}
impl std::fmt::Display for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let [major, minor, patch] = self.numbers;
        write!(f, "{major}.{minor}.{patch}")?;
        if !self.pre.is_empty() {
            write!(f, "-{}", self.pre.join("."))?;
        }
        Ok(())
    }
}
impl PartialOrd for Version {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(Debug, Deserialize)]
struct GithubRelease {
    tag_name: String,
    #[serde(default)]
    name: Option<String>,
    html_url: String,
    #[serde(default)]
    draft: bool,
    #[serde(default)]
    prerelease: bool,
    #[serde(default)]
    published_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Release {
    pub version: String,
    pub name: String,
    pub url: String,
    pub prerelease: bool,
    pub published: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct UpdateCheck {
    pub current: String,
    /// The newest published release that matches the channel.
    pub latest: Option<Release>,
    pub newer: bool,
    /// The newest beta build, when beta builds were left out and it is newer
    /// than `latest`.
    pub skipped_beta: Option<String>,
}

/// Pick the newest release. Drafts, unparsable tags and pages outside the
/// TagGroove repository are ignored; beta builds only when asked for.
fn newest(releases: &[GithubRelease], current: &str, include_prereleases: bool) -> UpdateCheck {
    let published: Vec<(Version, &GithubRelease)> = releases
        .iter()
        .filter(|r| !r.draft && r.html_url.starts_with(RELEASES_PAGE))
        .filter_map(|r| Version::parse(&r.tag_name).map(|v| (v, r)))
        .collect();
    let beta = |(v, r): &&(Version, &GithubRelease)| r.prerelease || v.prerelease();
    let latest = published
        .iter()
        .filter(|release| include_prereleases || !beta(release))
        .max_by(|a, b| a.0.cmp(&b.0));
    let skipped_beta = published
        .iter()
        .filter(|release| !include_prereleases && beta(release))
        .max_by(|a, b| a.0.cmp(&b.0))
        .filter(|(v, _)| latest.is_none_or(|(l, _)| v > l))
        .map(|(v, _)| v.to_string());
    let newer = match (&latest, Version::parse(current)) {
        (Some((version, _)), Some(current)) => *version > current,
        _ => false,
    };
    UpdateCheck {
        current: current.to_string(),
        latest: latest.map(|(version, r)| Release {
            version: version.to_string(),
            name: r
                .name
                .clone()
                .filter(|n| !n.trim().is_empty())
                .unwrap_or_else(|| r.tag_name.clone()),
            url: r.html_url.clone(),
            prerelease: r.prerelease || version.prerelease(),
            published: r.published_at.clone(),
        }),
        newer,
        skipped_beta,
    }
}

#[tauri::command]
pub async fn check_for_updates(include_prereleases: bool) -> Result<UpdateCheck, String> {
    let network = crate::settings::network();
    let client = http::builder(&network)?
        // GitHub answers the API directly; a redirect would leave api.github.com.
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|e| format!("Network client could not start: {e}"))?;
    let response = client
        .get(RELEASES_API)
        .header(reqwest::header::ACCEPT, "application/vnd.github+json")
        .send()
        .await
        .map_err(|error| http::network_message("GitHub", &error))?;
    if !response.status().is_success() {
        return Err(match response.status().as_u16() {
            403 | 429 => {
                "GitHub's rate limit for update checks was reached. Try again later.".into()
            }
            code => format!("GitHub returned HTTP {code} for the release list."),
        });
    }
    let body = http::read_limited(response, MAX_BYTES, "GitHub").await?;
    let releases: Vec<GithubRelease> = serde_json::from_slice(&body)
        .map_err(|_| "GitHub returned an unreadable release list.".to_string())?;
    Ok(newest(
        &releases,
        env!("CARGO_PKG_VERSION"),
        include_prereleases,
    ))
}

/// Open a page of TagGroove's GitHub releases in the browser.
#[tauri::command]
pub async fn open_release_page(url: String) -> Result<(), String> {
    let valid = url.strip_prefix(RELEASES_PAGE).is_some_and(|rest| {
        rest.chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '/' | '.' | '-' | '_' | '+'))
            && !rest.contains("..")
    });
    if !valid {
        return Err("Only TagGroove release pages can be opened".into());
    }
    #[cfg(desktop)]
    return tauri_plugin_opener::open_url(&url, None::<&str>).map_err(|e| e.to_string());
    #[cfg(mobile)]
    return Err(format!("Open {url} in a browser"));
}

#[cfg(test)]
mod tests {
    use super::*;

    fn release(tag: &str, prerelease: bool, draft: bool) -> GithubRelease {
        GithubRelease {
            tag_name: tag.into(),
            name: Some(format!("TagGroove {tag}")),
            html_url: format!("{RELEASES_PAGE}tag/{tag}"),
            draft,
            prerelease,
            published_at: Some("2026-10-01T00:00:00Z".into()),
        }
    }

    #[test]
    fn versions_order_like_semver() {
        let v = |s| Version::parse(s).unwrap();
        assert!(v("v0.2.0") > v("0.1.9"));
        assert!(v("1.0.0") > v("1.0.0-rc.1"));
        assert!(v("1.0.0-rc.2") > v("1.0.0-rc.1"));
        assert!(v("1.0.0-beta.11") > v("1.0.0-beta.2"));
        assert!(v("1.0.0-beta") > v("1.0.0-alpha.1"));
        assert!(v("1.0.0-alpha.1") > v("1.0.0-alpha"));
        assert_eq!(v("1.2"), v("1.2.0+build.5"));
        for bad in ["", "x", "1.2.3.4", "1.-2", "1.0.0-"] {
            assert!(Version::parse(bad).is_none(), "{bad}");
        }
    }

    #[test]
    fn newest_release_respects_the_beta_channel() {
        let releases = vec![
            release("v0.1.0", false, false),
            release("v0.3.0-beta.1", true, false),
            release("v0.2.0", false, false),
            release("v9.0.0", false, true),
            release("nightly", false, false),
            GithubRelease {
                html_url: "https://example.com/taggroove/v5.0.0".into(),
                ..release("v5.0.0", false, false)
            },
        ];
        let stable = newest(&releases, "0.1.0", false);
        assert!(stable.newer);
        let latest = stable.latest.unwrap();
        assert_eq!(latest.version, "0.2.0");
        assert_eq!(latest.url, format!("{RELEASES_PAGE}tag/v0.2.0"));
        let beta = newest(&releases, "0.1.0", true).latest.unwrap();
        assert_eq!(
            (beta.version.as_str(), beta.prerelease),
            ("0.3.0-beta.1", true)
        );
        assert_eq!(stable.skipped_beta.as_deref(), Some("0.3.0-beta.1"));
        let current = newest(&releases, "0.2.0", false);
        assert!(!current.newer);
        assert!(!newest(&[], "0.1.0", true).newer);
        // Only a beta is published: a stable check reports it was left out.
        let only_beta = newest(&[release("v0.1.0", true, false)], "0.1.0", false);
        assert_eq!(
            (only_beta.latest, only_beta.skipped_beta.as_deref()),
            (None, Some("0.1.0"))
        );
        assert_eq!(
            newest(&[release("v0.1.0", true, false)], "0.1.0", true).skipped_beta,
            None
        );
    }

    #[test]
    fn only_release_pages_open() {
        for bad in [
            "https://github.com/johnneerdael/TagGroove/issues",
            "https://evil.example/johnneerdael/TagGroove/releases/",
            "https://github.com/johnneerdael/TagGroove/releases/../../x",
            "https://github.com/johnneerdael/TagGroove/releases/tag/v1?x=1",
        ] {
            let error = tauri::async_runtime::block_on(open_release_page(bad.into())).unwrap_err();
            assert!(error.contains("Only TagGroove"), "{bad}");
        }
    }
}
