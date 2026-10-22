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

/// Words in a query parameter's name that mark a secret or who the person is. They are matched as
/// whole words (`api_key`, `access-token`, `clientSecret`), not as parts of other words, so
/// `keywords`, `author` and `design` are ordinary search parameters.
const CREDENTIAL_WORDS: [&str; 20] = [
    "key",
    "token",
    "secret",
    "password",
    "passwd",
    "pass",
    "auth",
    "authorization",
    "sig",
    "signature",
    "session",
    "sk",
    "bearer",
    "jwt",
    "hello",
    "email",
    "user",
    "username",
    "userid",
    "login",
];
/// Credential names written as one word.
const CREDENTIAL_COMPOUNDS: [&str; 9] = [
    "apikey",
    "accesskey",
    "secretkey",
    "accesstoken",
    "authtoken",
    "refreshtoken",
    "clientsecret",
    "sessionid",
    "sessionkey",
];

fn is_credential_name(name: &str) -> bool {
    // camelCase becomes separate words before the usual separators split the name.
    let mut spaced = String::with_capacity(name.len() + 4);
    let mut previous_lower = false;
    for c in name.chars() {
        if c.is_ascii_uppercase() && previous_lower {
            spaced.push('_');
        }
        previous_lower = c.is_ascii_lowercase() || c.is_ascii_digit();
        spaced.push(c.to_ascii_lowercase());
    }
    let words: Vec<&str> = spaced
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|word| !word.is_empty())
        .collect();
    words.iter().any(|word| CREDENTIAL_WORDS.contains(word))
        || CREDENTIAL_COMPOUNDS.contains(&words.concat().as_str())
}

/// Query parameters that carry a secret or say who the person is: such an address is never sent
/// to a solver. (freedb's `hello` handshake holds the user and domain of the configured email.)
fn carries_credentials(url: &Url) -> bool {
    if !url.username().is_empty() || url.password().is_some() {
        return true;
    }
    url.query_pairs().any(|(name, _)| is_credential_name(&name))
}

/// Why a request cannot go to a solver, when it cannot.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Eligibility {
    Allowed,
    Credentials,
    Image,
    Form,
}
impl Eligibility {
    pub fn allowed(self) -> bool {
        self == Eligibility::Allowed
    }
}

/// May this request be forwarded to a solver? Only a plain public GET for a page or an API
/// answer: no credentials of any kind, and no binary download.
pub fn eligible(auth: &Auth, form: Option<&str>, accept: &str, url: &Url) -> Eligibility {
    if form.is_some() {
        Eligibility::Form
    } else if !matches!(auth, Auth::None) || carries_credentials(url) {
        Eligibility::Credentials
    } else if accept.starts_with("image/") {
        Eligibility::Image
    } else {
        Eligibility::Allowed
    }
}

/// Before a direct request: a host that blocked us earlier is not contacted directly again, when
/// a solver can take the request. One that cannot (it carries a token, a form or an image
/// download) is simply tried. `Ok(None)` means "send it directly".
pub async fn before(
    url: &Url,
    name: &str,
    eligible: Eligibility,
    accept: &str,
    channel: Channel,
    ticket: u64,
) -> Result<Option<reqwest::Response>, String> {
    if !eligible.allowed() || !solver::is_blocked(url) {
        return Ok(None);
    }
    after_block(url, name, eligible, accept, channel, ticket)
        .await
        .map(Some)
}

/// Where the solver must go: the address that actually answered. A web page can redirect to another
/// allowed site before it is challenged, and the solver follows the same redirect, so the origin it
/// ends on is the challenged one. That address is checked for credentials like the first one was.
fn challenged_target(
    response: &reqwest::Response,
    requested: &Url,
    eligible: Eligibility,
) -> (Url, Eligibility) {
    if is_placeholder(response.url()) {
        return (requested.clone(), eligible);
    }
    let target = response.url().clone();
    let eligible = if eligible.allowed() && carries_credentials(&target) {
        Eligibility::Credentials
    } else {
        eligible
    };
    (target, eligible)
}

/// After a direct request: a Cloudflare block is repeated through the solver; any other answer
/// is handed back unchanged.
pub async fn after(
    mut response: reqwest::Response,
    url: &Url,
    name: &str,
    eligible: Eligibility,
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
    let (target, eligible) = challenged_target(&response, url, eligible);
    // Cloudflare's markers are near the top of its pages, so a refusal larger than the cap is read
    // only that far for the block check; a refusal that is not a block is handed back cut there.
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
        // Only a request a solver can take locks the host to the solver: a rate limit on a token
        // request must not stop the public lookups to that host (it is tried again next time).
        if eligible.allowed() {
            solver::mark_blocked(&target);
            // The challenge came from where a redirect led: remember the way, so the next request
            // to the address it started from goes straight to the solver.
            solver::remember_redirect(url, &target);
        }
        return after_block(&target, name, eligible, accept, channel, ticket).await;
    }
    rebuild(status, &headers, body)
}

async fn after_block(
    url: &Url,
    name: &str,
    eligible: Eligibility,
    accept: &str,
    channel: Channel,
    ticket: u64,
) -> Result<reqwest::Response, String> {
    match eligible {
        Eligibility::Allowed => {}
        Eligibility::Credentials => {
            return Err(format!(
                "{name} is blocking TagGroove (Cloudflare). A request that carries credentials cannot use a solver; try again later, or use Retry direct access in Options → Tag Sources → Cloudflare solver."
            ))
        }
        Eligibility::Image => {
            return Err(format!(
                "{name} is blocking TagGroove (Cloudflare). Images are not sent to a solver; try again later, or use Retry direct access in Options → Tag Sources → Cloudflare solver."
            ))
        }
        Eligibility::Form => {
            return Err(format!(
                "{name} is blocking TagGroove (Cloudflare). A form submission cannot use a solver; try again later, or use Retry direct access in Options → Tag Sources → Cloudflare solver."
            ))
        }
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
    // A site known to redirect elsewhere before it was challenged may end on that other origin.
    let landed = solver::redirected_to(url).map(|to| http::routed_for_solver(&to));
    for attempt in 0..2 {
        if http::stale(channel, ticket) {
            return Err(CANCELLED.into());
        }
        let solving = solver::solve(
            &client,
            &api,
            config.timeout_seconds,
            &target,
            landed.as_ref(),
        );
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
    Err("The solver timed out or is temporarily unavailable; retry the lookup".into())
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
            // The refused answer's cookies are not replayed as the page's own.
            || name == header::SET_COOKIE
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
        assert_eq!(
            eligible(&Auth::None, None, "application/json", &page),
            Eligibility::Allowed
        );
        // Each kind of refusal says what it was: a form, a credential, an image.
        assert_eq!(
            eligible(&Auth::None, Some("a=b"), "application/json", &page),
            Eligibility::Form
        );
        assert_eq!(
            eligible(&Auth::Key, None, "application/json", &page),
            Eligibility::Credentials
        );
        assert_eq!(
            eligible(
                &Auth::DiscogsToken("t".into()),
                None,
                "application/json",
                &page
            ),
            Eligibility::Credentials
        );
        assert_eq!(
            eligible(&Auth::None, None, "image/*", &page),
            Eligibility::Image
        );
        for refused in [
            "https://site.example/x?api_key=abc",
            "https://site.example/x?Signature=abc",
            "https://site.example/x?apiKey=abc",
            "https://site.example/x?access_token=abc",
            "https://site.example/x?clientSecret=abc",
            "https://site.example/x?sk=abc",
            "https://site.example/x?session=abc",
            "https://site.example/x?email=me@example.org",
            "https://site.example/x?hello=me+example.org",
            "https://site.example/x?user=me",
            "https://user:pw@site.example/",
        ] {
            let url = Url::parse(refused).unwrap();
            assert_eq!(
                eligible(&Auth::None, None, "text/html", &url),
                Eligibility::Credentials,
                "{refused}"
            );
        }
        // Ordinary search parameters that merely contain those letters are not credentials.
        for allowed in [
            "https://site.example/x?keywords=blue",
            "https://site.example/x?author=a&design=b",
            "https://site.example/x?sign=up&hint=none&monkey=1&keyword=blue",
            "https://site.example/x?term=key+lime+pie",
        ] {
            let url = Url::parse(allowed).unwrap();
            assert!(
                eligible(&Auth::None, None, "text/html", &url).allowed(),
                "{allowed}"
            );
        }
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
        assert!(!eligible(&Auth::None, None, "text/plain", &url).allowed());
        // Without a valid email the developer contact is sent: still a freedb handshake, still kept home.
        let anonymous = super::super::freedb::command_url(
            &super::super::freedb::FreedbSettings::default(),
            "cddb hello",
        )
        .unwrap();
        assert!(!eligible(&Auth::None, None, "text/plain", &anonymous).allowed());
    }

    #[test]
    fn the_solver_is_sent_to_the_address_that_answered_after_a_redirect() {
        use reqwest::ResponseBuilderExt;
        let requested = Url::parse("https://a.example/page").unwrap();
        // A real answer knows where it came from: the redirect's destination is the target.
        let landed = Url::parse("https://b.example/landing?term=blue").unwrap();
        let redirected: reqwest::Response = http_crate_response(Some(landed.clone())).into();
        assert_eq!(
            challenged_target(&redirected, &requested, Eligibility::Allowed),
            (landed, Eligibility::Allowed)
        );
        // A destination that carries a credential is never forwarded, whatever the first address was.
        let keyed = Url::parse("https://b.example/landing?api_key=abc").unwrap();
        let redirected: reqwest::Response = http_crate_response(Some(keyed.clone())).into();
        assert_eq!(
            challenged_target(&redirected, &requested, Eligibility::Allowed),
            (keyed, Eligibility::Credentials)
        );
        // An answer rebuilt without an address of its own keeps the requested one.
        let rebuilt: reqwest::Response = http_crate_response(None).into();
        assert_eq!(
            challenged_target(&rebuilt, &requested, Eligibility::Allowed),
            (requested.clone(), Eligibility::Allowed)
        );

        fn http_crate_response(url: Option<Url>) -> tauri::http::Response<Vec<u8>> {
            let mut builder = tauri::http::Response::builder().status(403);
            if let Some(url) = url {
                builder = builder.url(url);
            }
            builder.body(Vec::new()).unwrap()
        }
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
            Eligibility::Allowed,
            "application/json",
            http::Channel::Search,
            0,
        )
        .await
        .unwrap();
        assert_eq!(again.status(), StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(again.bytes().await.unwrap().as_ref(), b"busy");
    }

    /// A Cloudflare block page as a refused answer; `url` is where a redirect ended, if it did.
    fn cloudflare_block(url: Option<Url>) -> reqwest::Response {
        use reqwest::ResponseBuilderExt;
        let mut builder = tauri::http::Response::builder()
            .status(403)
            .header("server", "cloudflare")
            .header("cf-ray", "8a1b2c3d4e5f-AMS")
            .header("set-cookie", "cf_clearance=x");
        if let Some(url) = url {
            builder = builder.url(url);
        }
        builder
            .body(
                b"<title>Attention Required! | Cloudflare</title>Sorry, you have been blocked."
                    .to_vec(),
            )
            .unwrap()
            .into()
    }

    #[tokio::test]
    async fn a_block_on_a_request_a_solver_cannot_take_does_not_lock_the_host() {
        let url = Url::parse("https://app-nolock.example/api?x=1").unwrap();
        let search = http::Channel::Search;
        // Each refusal says why, and none remembers the host as blocked.
        let credentials = after(
            cloudflare_block(None),
            &url,
            "Site",
            Eligibility::Credentials,
            "application/json",
            search,
            0,
        )
        .await
        .unwrap_err();
        assert!(credentials.contains("carries credentials"), "{credentials}");
        let image = after(
            cloudflare_block(None),
            &url,
            "Site",
            Eligibility::Image,
            "image/*",
            search,
            0,
        )
        .await
        .unwrap_err();
        assert!(image.contains("Images are not sent to a solver"), "{image}");
        let form = after(
            cloudflare_block(None),
            &url,
            "Site",
            Eligibility::Form,
            "application/json",
            search,
            0,
        )
        .await
        .unwrap_err();
        assert!(form.contains("form submission"), "{form}");
        assert!(!solver::is_blocked(&url));
        // So a public lookup to the same host is still sent directly.
        assert!(
            before(&url, "Site", Eligibility::Allowed, "text/html", search, 0)
                .await
                .unwrap()
                .is_none()
        );
    }

    #[tokio::test]
    async fn a_block_on_a_public_lookup_sends_later_lookups_to_the_solver_but_not_other_requests() {
        let url = Url::parse("https://app-lock.example/search?term=blue").unwrap();
        let search = http::Channel::Search;
        // No solver is set up in tests: it says so, and the host is remembered.
        let first = after(
            cloudflare_block(None),
            &url,
            "Site",
            Eligibility::Allowed,
            "text/html",
            search,
            0,
        )
        .await
        .unwrap_err();
        assert!(
            first.contains("Add a FlareSolverr-compatible solver"),
            "{first}"
        );
        assert!(solver::is_blocked(&url));
        // The next lookup does not walk into the block again...
        let again = before(&url, "Site", Eligibility::Allowed, "text/html", search, 0)
            .await
            .unwrap_err();
        assert!(
            again.contains("Add a FlareSolverr-compatible solver"),
            "{again}"
        );
        // ...while one that cannot use a solver is simply tried.
        assert!(before(
            &url,
            "Site",
            Eligibility::Credentials,
            "application/json",
            search,
            0
        )
        .await
        .unwrap()
        .is_none());
    }

    #[tokio::test]
    async fn a_challenge_after_a_redirect_is_remembered_for_the_address_it_started_from() {
        let from = Url::parse("https://app-from.example/page").unwrap();
        let to = Url::parse("https://app-to.example/landing").unwrap();
        let search = http::Channel::Search;
        let _ = after(
            cloudflare_block(Some(to.clone())),
            &from,
            "Site",
            Eligibility::Allowed,
            "text/html",
            search,
            0,
        )
        .await;
        assert!(solver::is_blocked(&to));
        assert_eq!(solver::redirected_to(&from), Some(to));
        // The page that redirects there is not requested directly again: it goes to the solver.
        assert!(solver::is_blocked(&from));
        let again = before(&from, "Site", Eligibility::Allowed, "text/html", search, 0)
            .await
            .unwrap_err();
        assert!(
            again.contains("Add a FlareSolverr-compatible solver"),
            "{again}"
        );
    }

    #[test]
    fn a_rebuilt_answer_does_not_replay_cookies() {
        let mut headers = header::HeaderMap::new();
        headers.insert(header::SET_COOKIE, "cf_clearance=x".parse().unwrap());
        headers.insert(header::CONTENT_TYPE, "text/html".parse().unwrap());
        let rebuilt = rebuild(StatusCode::OK, &headers, b"x".to_vec()).unwrap();
        assert!(rebuilt.headers().get(header::SET_COOKIE).is_none());
        assert!(rebuilt.headers().get(header::CONTENT_TYPE).is_some());
    }

    #[tokio::test]
    async fn a_request_a_solver_cannot_take_is_sent_directly_even_to_a_redirecting_blocked_site() {
        let from = Url::parse("https://app-cred-from.example/page").unwrap();
        let to = Url::parse("https://app-cred-to.example/landing").unwrap();
        let search = http::Channel::Search;
        let _ = after(
            cloudflare_block(Some(to.clone())),
            &from,
            "Site",
            Eligibility::Allowed,
            "text/html",
            search,
            0,
        )
        .await;
        assert!(solver::is_blocked(&from) && solver::is_blocked(&to));
        // A freedb handshake, a token request, an image or a form still go out directly.
        for refused in [
            Eligibility::Credentials,
            Eligibility::Image,
            Eligibility::Form,
        ] {
            assert!(
                before(&from, "Site", refused, "text/html", search, 0)
                    .await
                    .unwrap()
                    .is_none(),
                "{refused:?}"
            );
        }
    }
}
