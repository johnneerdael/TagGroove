//! Cloudflare solver support shared by every tag source.
//!
//! A solver is any FlareSolverr-compatible service (FlareSolverr, Byparr, trawl): it receives a
//! public page address, loads it in a real browser and returns the page. TagGroove sends a
//! request to a solver only after a direct request was blocked by Cloudflare, and only for
//! plain public GET requests. A request that carries credentials (a bearer token, an API key,
//! a Discogs token) is never forwarded.
//!
//! Once a host has blocked a direct request, further direct requests to that host are refused
//! for the rest of the session: they would only repeat the block. They go through the solver, or
//! fail with a message that says how to configure one.

use reqwest::{header::HeaderMap, Client, Url};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashSet,
    sync::{Arc, Mutex, OnceLock},
};

/// The saved solver settings (Options → Tag Sources → Cloudflare solver).
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default)]
pub struct SolverConfig {
    pub endpoint: Option<String>,
    pub timeout_seconds: u64,
}
impl Default for SolverConfig {
    fn default() -> Self {
        Self {
            endpoint: None,
            timeout_seconds: 120,
        }
    }
}
impl SolverConfig {
    /// The solver's API address (`…/v1`), or `None` when no solver is configured.
    pub fn api_url(&self) -> Result<Option<Url>, String> {
        if !(1..=300).contains(&self.timeout_seconds) {
            return Err("Solver timeout must be between 1 and 300 seconds".into());
        }
        let Some(endpoint) = self
            .endpoint
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
        else {
            return Ok(None);
        };
        let mut url = Url::parse(endpoint).map_err(|_| "Invalid solver URL")?;
        if !matches!(url.scheme(), "http" | "https")
            || url.host_str().is_none()
            || !url.username().is_empty()
            || url.password().is_some()
            || url.query().is_some()
            || url.fragment().is_some()
        {
            return Err("Solver URL must use HTTP or HTTPS without credentials, query parameters or a fragment".into());
        }
        let path = url.path().trim_end_matches('/').to_owned();
        if path.ends_with("/v1") {
            url.set_path(&path)
        } else {
            url.set_path(&format!("{path}/v1"))
        }
        Ok(Some(url))
    }
}

/// Hosts (with port) that blocked a direct request this session.
#[derive(Default)]
pub struct BlockedHosts(Mutex<HashSet<String>>);
fn host_key(url: &Url) -> Option<String> {
    Some(format!(
        "{}:{}",
        url.host_str()?.to_ascii_lowercase(),
        url.port_or_known_default()?
    ))
}
impl BlockedHosts {
    /// Remember that a direct request to this host was blocked.
    pub fn mark(&self, url: &Url) {
        if let (Some(host), Ok(mut set)) = (host_key(url), self.0.lock()) {
            set.insert(host);
        }
    }
    /// Did a direct request to this host get blocked earlier in this session?
    pub fn contains(&self, url: &Url) -> bool {
        match (host_key(url), self.0.lock()) {
            (Some(host), Ok(set)) => set.contains(&host),
            _ => false,
        }
    }
    /// Forget every remembered block ("Retry direct access"). Saved settings are untouched.
    pub fn clear(&self) {
        if let Ok(mut set) = self.0.lock() {
            set.clear();
        }
    }
}
/// The process-wide registry every tag source shares.
pub fn global_blocked() -> Arc<BlockedHosts> {
    static GLOBAL: OnceLock<Arc<BlockedHosts>> = OnceLock::new();
    GLOBAL.get_or_init(Default::default).clone()
}
pub fn mark_blocked(url: &Url) {
    global_blocked().mark(url)
}
pub fn is_blocked(url: &Url) -> bool {
    global_blocked().contains(url)
}
pub fn reset_blocked() {
    global_blocked().clear()
}

/// A Cloudflare challenge page: the explicit `cf-mitigated` header or the interstitial's markup.
pub fn is_challenge(explicit: bool, body: &str) -> bool {
    let b = body.to_ascii_lowercase();
    explicit
        || ((b.contains("/cdn-cgi/challenge-platform/")
            || b.contains("_cf_chl_opt")
            || b.contains("cf-chl-"))
            && (b.contains("<title>just a moment")
                || b.contains("<title>attention required!")
                || b.contains("checking your browser")
                || b.contains("enable javascript and cookies to continue")))
}

fn fronted_by_cloudflare(headers: &HeaderMap) -> bool {
    headers.contains_key("cf-ray")
        || headers
            .get("server")
            .and_then(|value| value.to_str().ok())
            .is_some_and(|value| value.eq_ignore_ascii_case("cloudflare"))
}

/// Did Cloudflare block or challenge this response? That is a challenge page, or an error page
/// from Cloudflare itself (a bot rule, a firewall block, rate limiting) on a refused request.
/// An ordinary 403 from the site behind Cloudflare is not a block.
pub fn is_cloudflare_block(status: u16, headers: &HeaderMap, body: &str) -> bool {
    let explicit = headers
        .get("cf-mitigated")
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| value.eq_ignore_ascii_case("challenge"));
    if is_challenge(explicit, body) {
        return true;
    }
    if !matches!(status, 403 | 429 | 503) || !fronted_by_cloudflare(headers) {
        return false;
    }
    let b = body.to_ascii_lowercase();
    b.contains("cloudflare")
        && [
            "attention required",
            "you have been blocked",
            "you are being rate limited",
            "error code: 1020",
            "error code: 1015",
            "error code: 1010",
            "cf-error-details",
            "cf-wrapper",
            "ray id",
            "access denied",
            "just a moment",
        ]
        .iter()
        .any(|marker| b.contains(marker))
}

/// Said whenever a host blocks TagGroove and no solver is configured.
pub fn no_solver_message(host: &str) -> String {
    format!("{host} is blocking TagGroove (Cloudflare). Add a FlareSolverr-compatible solver (FlareSolverr, Byparr or trawl) in Options → Tag Sources → Cloudflare solver, then try again.")
}

/// What a solver returned for one address.
pub struct Solved {
    pub status: u16,
    pub body: String,
    pub content_type: Option<String>,
}
pub enum SolveError {
    /// A timeout or a busy solver: worth one more try.
    Transient,
    Permanent(String),
}

const MAX_PAGE_BYTES: usize = 8 * 1024 * 1024;
const MAX_ENVELOPE_BYTES: usize = 16 * 1024 * 1024;

/// Ask the solver for one public address. `client` must not follow redirects.
pub async fn solve(
    client: &Client,
    api: &Url,
    timeout_seconds: u64,
    target: &Url,
) -> Result<Solved, SolveError> {
    let request = client
        .post(api.clone())
        .timeout(std::time::Duration::from_secs(timeout_seconds + 10))
        .json(&serde_json::json!({
            "cmd": "request.get",
            "url": target.as_str(),
            "maxTimeout": timeout_seconds * 1000,
        }));
    let mut response = request.send().await.map_err(|_| SolveError::Transient)?;
    if response.url() != api {
        return Err(SolveError::Permanent(
            "Solver redirect is not allowed".into(),
        ));
    }
    let status = response.status();
    if [429, 502, 503, 504].contains(&status.as_u16()) {
        return Err(SolveError::Transient);
    }
    if !status.is_success() {
        return Err(SolveError::Permanent(format!(
            "Solver HTTP {}",
            status.as_u16()
        )));
    }
    let mut bytes = Vec::new();
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|_| SolveError::Permanent("Could not read the solver response".into()))?
    {
        if bytes.len() + chunk.len() > MAX_ENVELOPE_BYTES {
            return Err(SolveError::Permanent(
                "Solver response exceeds size limit".into(),
            ));
        }
        bytes.extend_from_slice(&chunk);
    }
    let envelope: serde_json::Value = serde_json::from_slice(&bytes)
        .map_err(|_| SolveError::Permanent("Invalid FlareSolverr-compatible response".into()))?;
    if envelope["status"].as_str() != Some("ok") {
        let message = envelope["message"]
            .as_str()
            .unwrap_or_default()
            .to_ascii_lowercase();
        if [
            "timeout",
            "timed out",
            "deadline",
            "pool saturated",
            "pool exhausted",
        ]
        .iter()
        .any(|s| message.contains(s))
        {
            return Err(SolveError::Transient);
        }
        return Err(SolveError::Permanent(
            "Solver could not retrieve the page; check the solver service".into(),
        ));
    }
    let solution = &envelope["solution"];
    let final_url = solution["url"]
        .as_str()
        .and_then(|s| Url::parse(s).ok())
        .ok_or_else(|| SolveError::Permanent("Solver returned an invalid final URL".into()))?;
    if target.origin() != final_url.origin() {
        return Err(SolveError::Permanent(
            "Solver redirected outside the original site".into(),
        ));
    }
    // The site's own status is handed back with its body: a 404 is an answer ("not found"), and each
    // transport applies its usual meaning to the status. Only an unresolved challenge is an error.
    let solved_status = solution["status"].as_u64().unwrap_or(0);
    if !(100..600).contains(&solved_status) {
        return Err(SolveError::Permanent(
            "The solver did not report the site's HTTP status".into(),
        ));
    }
    let success = (200..300).contains(&solved_status);
    let body = solution["response"].as_str().unwrap_or_default();
    if body.len() > MAX_PAGE_BYTES || (success && body.is_empty()) {
        return Err(SolveError::Permanent(
            "Solver returned empty or oversized content".into(),
        ));
    }
    let headers = solution["headers"].as_object();
    let header = |name: &str| {
        headers.and_then(|headers| {
            headers
                .iter()
                .find(|(key, _)| key.eq_ignore_ascii_case(name))
                .and_then(|(_, value)| value.as_str())
        })
    };
    if is_challenge(header("cf-mitigated") == Some("challenge"), body) {
        return Err(SolveError::Permanent(
            "Solver returned an unresolved challenge".into(),
        ));
    }
    Ok(Solved {
        status: solved_status as u16,
        body: body.to_owned(),
        content_type: header("content-type").map(str::to_owned),
    })
}

/// Browser-based solvers show a JSON document as `<html>…<pre>{…}</pre>…</html>`. When the
/// caller asked for JSON, return the document itself; any other body is returned unchanged.
pub fn unwrap_json_page(body: &str) -> String {
    let trimmed = body.trim();
    if trimmed.starts_with('{') || trimmed.starts_with('[') {
        return body.to_owned();
    }
    let lower = trimmed.to_ascii_lowercase();
    let (Some(open), Some(close)) = (lower.find("<pre"), lower.rfind("</pre>")) else {
        return body.to_owned();
    };
    let Some(start) = lower[open..].find('>').map(|i| open + i + 1) else {
        return body.to_owned();
    };
    if start > close {
        return body.to_owned();
    }
    let inner = &trimmed[start..close];
    if !(inner.trim_start().starts_with('{') || inner.trim_start().starts_with('[')) {
        return body.to_owned();
    }
    inner
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&amp;", "&")
}

#[cfg(test)]
mod tests {
    use super::*;
    use reqwest::header::{HeaderMap, HeaderValue};

    fn headers(pairs: &[(&'static str, &'static str)]) -> HeaderMap {
        let mut map = HeaderMap::new();
        for (name, value) in pairs {
            map.insert(*name, HeaderValue::from_static(value));
        }
        map
    }

    #[test]
    fn the_explicit_header_and_the_interstitial_are_challenges() {
        assert!(is_cloudflare_block(
            403,
            &headers(&[("cf-mitigated", "challenge")]),
            ""
        ));
        assert!(is_cloudflare_block(
            503,
            &HeaderMap::new(),
            "<title>Just a moment...</title><script src=\"/cdn-cgi/challenge-platform/h/b\"></script>"
        ));
    }

    #[test]
    fn a_cloudflare_error_page_is_a_block_even_without_challenge_markup() {
        let page = "<!DOCTYPE html><title>Attention Required! | Cloudflare</title>Sorry, you have been blocked";
        let fronted = headers(&[("server", "cloudflare"), ("cf-ray", "8a1b2c3d4e5f-AMS")]);
        assert!(is_cloudflare_block(403, &fronted, page));
        assert!(is_cloudflare_block(
            429,
            &fronted,
            "Cloudflare Error 1015: You are being rate limited"
        ));
    }

    #[test]
    fn an_ordinary_error_from_the_site_behind_cloudflare_is_not_a_block() {
        let fronted = headers(&[("server", "cloudflare")]);
        assert!(!is_cloudflare_block(
            403,
            &fronted,
            "{\"error\":\"forbidden\"}"
        ));
        assert!(!is_cloudflare_block(404, &fronted, "Not found"));
        assert!(!is_cloudflare_block(
            503,
            &HeaderMap::new(),
            "Service busy, try again later"
        ));
        assert!(!is_cloudflare_block(200, &fronted, "<html>ok</html>"));
    }

    #[test]
    fn a_block_is_remembered_per_host_and_port_until_cleared() {
        let registry = BlockedHosts::default();
        let a = Url::parse("https://blocked.example/path").unwrap();
        assert!(!registry.contains(&a));
        registry.mark(&a);
        assert!(registry.contains(&Url::parse("https://BLOCKED.example/other?x=1").unwrap()));
        assert!(!registry.contains(&Url::parse("https://other.example/").unwrap()));
        assert!(!registry.contains(&Url::parse("https://blocked.example:8443/").unwrap()));
        registry.clear();
        assert!(!registry.contains(&a));
    }

    #[test]
    fn endpoint_addresses_get_the_solver_api_path() {
        let config = |endpoint: &str| SolverConfig {
            endpoint: Some(endpoint.into()),
            timeout_seconds: 120,
        };
        for (given, expected) in [
            ("http://localhost:8191", "http://localhost:8191/v1"),
            ("http://localhost:8191/v1", "http://localhost:8191/v1"),
            ("https://trawl.example/", "https://trawl.example/v1"),
        ] {
            assert_eq!(config(given).api_url().unwrap().unwrap().as_str(), expected);
        }
        assert!(SolverConfig::default().api_url().unwrap().is_none());
        assert!(config("ftp://x").api_url().is_err());
        assert!(config("http://user:pw@x").api_url().is_err());
    }

    #[test]
    fn a_json_document_in_a_browser_page_is_unwrapped() {
        let page = "<html><head></head><body><pre style=\"word-wrap: break-word;\">{&quot;a&quot;: &quot;x &amp; y&quot;}</pre></body></html>";
        assert_eq!(unwrap_json_page(page), "{\"a\": \"x & y\"}");
        assert_eq!(unwrap_json_page("{\"a\":1}"), "{\"a\":1}");
        let html = "<html><body><pre>plain text</pre></body></html>";
        assert_eq!(unwrap_json_page(html), html);
    }
}
