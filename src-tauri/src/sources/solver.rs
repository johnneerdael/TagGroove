//! One Cloudflare rule for every tag source (see `catalog_sources::solver`).
//!
//! A direct request that Cloudflare blocks or challenges is repeated through the configured
//! FlareSolverr-compatible solver (FlareSolverr, Byparr, trawl). A host that blocked a direct
//! request is not contacted directly again this session: its requests go through the solver, or
//! fail with a message that says how to set one up. Requests that carry credentials (Discogs
//! tokens, API keys) are never forwarded to a solver.

use super::http::{self, Auth, Channel, CANCELLED};
use catalog_sources::solver::{self, SolveError, SolverConfig};
use reqwest::{header, StatusCode, Url};
use std::time::Duration;

const PAGE: &str = "tag-sources-solver";
/// Where the Traxsource-only setting lived before the solver became a general option.
const LEGACY_PAGE: &str = "tag-sources-catalog";
const MAX_REFUSAL_BYTES: usize = 256 * 1024;
const MAX_SOLVED_BYTES: usize = 8 * 1024 * 1024;

/// The saved solver settings: Options → Tag Sources → Cloudflare solver.
pub fn config() -> Result<SolverConfig, String> {
    let settings = crate::settings::current();
    parse(
        settings
            .pages
            .get(PAGE)
            .or_else(|| settings.pages.get(LEGACY_PAGE)),
    )
}

pub fn parse(value: Option<&serde_json::Value>) -> Result<SolverConfig, String> {
    let mut config = SolverConfig::default();
    if let Some(value) = value {
        config.endpoint = match value.get("endpoint") {
            None | Some(serde_json::Value::Null) => None,
            Some(serde_json::Value::String(s)) => Some(s.trim().into()),
            _ => return Err("Solver endpoint must be a URL".into()),
        };
        if let Some(timeout) = value.get("timeout_seconds") {
            config.timeout_seconds = timeout
                .as_u64()
                .or_else(|| timeout.as_str().and_then(|s| s.trim().parse().ok()))
                .ok_or("Solver timeout must be a whole number of seconds")?;
        }
    }
    config.api_url()?;
    Ok(config)
}

/// Forget remembered blocks ("Retry direct access"); saved settings stay as they are.
pub fn reset() {
    solver::reset_blocked();
}

/// Query parameters that carry a secret or say who the person is: such an address is never sent
/// to a solver. (freedb's `hello` handshake holds the user and domain of the configured email.)
fn carries_credentials(url: &Url) -> bool {
    if !url.username().is_empty() || url.password().is_some() {
        return true;
    }
    url.query_pairs().any(|(name, _)| {
        let name = name.to_ascii_lowercase();
        [
            "key",
            "token",
            "secret",
            "password",
            "passwd",
            "auth",
            "sig",
            "signature",
            "hello",
            "email",
            "user",
        ]
        .iter()
        .any(|word| name.contains(word))
    })
}

/// May this request be forwarded to a solver? Only a plain public GET for a page or an API
/// answer: no credentials of any kind, and no binary download.
pub fn eligible(auth: &Auth, form: Option<&str>, accept: &str, url: &Url) -> bool {
    form.is_none()
        && matches!(auth, Auth::None)
        && !accept.starts_with("image/")
        && !carries_credentials(url)
}

/// Before a direct request: a host that blocked us earlier is not contacted directly again.
/// `Ok(None)` means "send it directly".
pub async fn before(
    url: &Url,
    name: &str,
    eligible: bool,
    accept: &str,
    channel: Channel,
    ticket: u64,
) -> Result<Option<reqwest::Response>, String> {
    if !solver::is_blocked(url) {
        return Ok(None);
    }
    after_block(url, name, eligible, accept, channel, ticket)
        .await
        .map(Some)
}

/// After a direct request: a Cloudflare block is repeated through the solver; any other answer
/// is handed back unchanged.
pub async fn after(
    mut response: reqwest::Response,
    url: &Url,
    name: &str,
    eligible: bool,
    accept: &str,
    channel: Channel,
    ticket: u64,
) -> Result<reqwest::Response, String> {
    let status = response.status();
    let explicit = response
        .headers()
        .get("cf-mitigated")
        .is_some_and(|value| value == "challenge");
    // Cloudflare refuses with 403, 429 or 503; a successful answer is never a block.
    if !explicit && !matches!(status.as_u16(), 403 | 429 | 503) {
        return Ok(response);
    }
    let headers = response.headers().clone();
    let mut body = Vec::new();
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|e| http::network_message(name, &e.without_url()))?
    {
        if body.len() + chunk.len() > MAX_REFUSAL_BYTES {
            break;
        }
        body.extend_from_slice(&chunk);
    }
    let text = String::from_utf8_lossy(&body);
    if solver::is_cloudflare_block(status.as_u16(), &headers, &text) {
        solver::mark_blocked(url);
        return after_block(url, name, eligible, accept, channel, ticket).await;
    }
    rebuild(status, &headers, body)
}

async fn after_block(
    url: &Url,
    name: &str,
    eligible: bool,
    accept: &str,
    channel: Channel,
    ticket: u64,
) -> Result<reqwest::Response, String> {
    if !eligible {
        return Err(format!(
            "{name} is blocking TagGroove (Cloudflare). A request that carries credentials cannot use a solver; try again later, or use Retry direct access in Options → Tag Sources → Cloudflare solver."
        ));
    }
    let config = config()?;
    let host = url.host_str().unwrap_or(name);
    let api = config
        .api_url()?
        .ok_or_else(|| solver::no_solver_message(host))?;
    let network = crate::settings::network();
    let client = http::builder(&network)?
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|_| "Could not create solver network client".to_owned())?;
    let target = http::routed_for_solver(url);
    for attempt in 0..2 {
        if http::stale(channel, ticket) {
            return Err(CANCELLED.into());
        }
        let solving = solver::solve(&client, &api, config.timeout_seconds, &target);
        tokio::pin!(solving);
        let solved = loop {
            match tokio::time::timeout(Duration::from_millis(25), solving.as_mut()).await {
                Ok(result) => break result,
                Err(_) if http::stale(channel, ticket) => return Err(CANCELLED.into()),
                Err(_) => {}
            }
        };
        match solved {
            Err(SolveError::Transient) if attempt == 0 => {
                tokio::time::sleep(Duration::from_millis(250)).await;
            }
            Err(SolveError::Transient) => {
                return Err(
                    "The solver timed out or is temporarily unavailable; retry the lookup".into(),
                )
            }
            Err(SolveError::Permanent(message)) => return Err(message),
            Ok(page) => {
                let text = if accept.contains("json") {
                    solver::unwrap_json_page(&page.body)
                } else {
                    page.body
                };
                if text.len() > MAX_SOLVED_BYTES {
                    return Err(format!("{name} returned more data than expected."));
                }
                let mut headers = header::HeaderMap::new();
                if let Some(Ok(value)) = page
                    .content_type
                    .as_deref()
                    .map(header::HeaderValue::from_str)
                {
                    headers.insert(header::CONTENT_TYPE, value);
                }
                return rebuild(
                    StatusCode::from_u16(page.status).unwrap_or(StatusCode::OK),
                    &headers,
                    text.into_bytes(),
                );
            }
        }
    }
    unreachable!()
}

/// A response built from an answer that was already read. Its address is a placeholder; callers
/// that need the final address use the one they asked for.
fn rebuild(
    status: StatusCode,
    headers: &header::HeaderMap,
    body: Vec<u8>,
) -> Result<reqwest::Response, String> {
    let mut builder = tauri::http::Response::builder().status(status);
    for (name, value) in headers {
        // The body was read in full and is no longer encoded or chunked.
        if name == header::CONTENT_LENGTH
            || name == header::TRANSFER_ENCODING
            || name == header::CONTENT_ENCODING
        {
            continue;
        }
        builder = builder.header(name, value);
    }
    builder
        .body(body)
        .map(reqwest::Response::from)
        .map_err(|_| "The answer could not be read.".to_owned())
}

/// `true` for the placeholder address of a rebuilt response.
pub fn is_placeholder(url: &Url) -> bool {
    url.host_str() == Some("no.url.provided.local")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn solver_settings_read_the_current_and_the_legacy_page() {
        let value =
            serde_json::json!({"endpoint": " https://trawl.example ", "timeout_seconds": "90"});
        let config = parse(Some(&value)).unwrap();
        assert_eq!(config.endpoint.as_deref(), Some("https://trawl.example"));
        assert_eq!(config.timeout_seconds, 90);
        assert!(parse(None).unwrap().endpoint.is_none());
        assert!(parse(Some(&serde_json::json!({"endpoint": 5}))).is_err());
        assert!(parse(Some(&serde_json::json!({"timeout_seconds": 0}))).is_err());
    }

    #[test]
    fn only_plain_public_gets_may_use_a_solver() {
        let page = Url::parse("https://site.example/search?term=blue").unwrap();
        assert!(eligible(&Auth::None, None, "application/json", &page));
        assert!(!eligible(
            &Auth::None,
            Some("a=b"),
            "application/json",
            &page
        ));
        assert!(!eligible(&Auth::Key, None, "application/json", &page));
        assert!(!eligible(
            &Auth::DiscogsToken("t".into()),
            None,
            "application/json",
            &page
        ));
        assert!(!eligible(&Auth::None, None, "image/*", &page));
        let keyed = Url::parse("https://site.example/x?api_key=abc").unwrap();
        assert!(!eligible(&Auth::None, None, "text/html", &keyed));
        let signed = Url::parse("https://site.example/x?Signature=abc").unwrap();
        assert!(!eligible(&Auth::None, None, "text/html", &signed));
        let userinfo = Url::parse("https://user:pw@site.example/").unwrap();
        assert!(!eligible(&Auth::None, None, "text/html", &userinfo));
        // An address that names the person is never forwarded either.
        let identity = Url::parse("https://site.example/x?email=me@example.org").unwrap();
        assert!(!eligible(&Auth::None, None, "text/html", &identity));
    }

    #[test]
    fn a_freedb_request_never_goes_to_a_solver_because_its_hello_names_the_person() {
        let settings = super::super::freedb::FreedbSettings {
            email: "someone@example.org".into(),
            ..Default::default()
        };
        let url = super::super::freedb::command_url(&settings, "cddb query abc 1 150 200").unwrap();
        assert!(
            url.query().unwrap().contains("hello=someone+example.org"),
            "{url}"
        );
        assert!(!eligible(&Auth::None, None, "text/plain", &url));
        // Without a valid email the developer contact is sent: still a freedb handshake, still kept home.
        let anonymous = super::super::freedb::command_url(
            &super::super::freedb::FreedbSettings::default(),
            "cddb hello",
        )
        .unwrap();
        assert!(!eligible(&Auth::None, None, "text/plain", &anonymous));
    }

    #[tokio::test]
    async fn a_refusal_that_is_not_a_block_is_handed_back_unchanged() {
        let response = rebuild(
            StatusCode::SERVICE_UNAVAILABLE,
            &header::HeaderMap::new(),
            b"busy".to_vec(),
        )
        .unwrap();
        let url = Url::parse("https://site.example/").unwrap();
        let again = after(
            response,
            &url,
            "Site",
            true,
            "application/json",
            http::Channel::Search,
            0,
        )
        .await
        .unwrap();
        assert_eq!(again.status(), StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(again.bytes().await.unwrap().as_ref(), b"busy");
    }
}
