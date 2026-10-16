//! Network hygiene for tag sources: one shared rustls client, a trusted host
//! list (also enforced on redirects), serialised per-service rate limits, size
//! limits and superseded-request cancellation.
use crate::settings::{Network, ProxyMode};
use reqwest::{header, StatusCode, Url};
use serde_json::Value;
use std::collections::HashSet;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex as StdMutex};
use std::time::{Duration, Instant};
use tokio::sync::Mutex;

pub const USER_AGENT: &str = concat!(
    "OpenTag/",
    env!("CARGO_PKG_VERSION"),
    " ( https://github.com/johnneerdael/opentag )"
);
pub const CANCELLED: &str = "Cancelled: a newer tag source request replaced this one.";
const MAX_JSON_BYTES: usize = 8 * 1024 * 1024;
pub const MAX_IMAGE_BYTES: usize = 25 * 1024 * 1024;

/// Independent request streams; a newer ticket on a channel cancels older work on it.
#[derive(Clone, Copy, Debug)]
pub enum Channel {
    Search = 0,
    Release = 1,
    Image = 2,
}
static TICKETS: [AtomicU64; 3] = [const { AtomicU64::new(0) }; 3];
pub fn claim(channel: Channel, ticket: u64) {
    TICKETS[channel as usize].fetch_max(ticket, Ordering::SeqCst);
}
/// Cancel every request issued before `ticket` on all channels.
pub fn cancel(ticket: u64) {
    TICKETS.iter().for_each(|slot| {
        slot.fetch_max(ticket, Ordering::SeqCst);
    });
}
pub fn stale(channel: Channel, ticket: u64) -> bool {
    TICKETS[channel as usize].load(Ordering::SeqCst) > ticket
}

pub fn trusted(url: &Url) -> bool {
    let host = url.host_str().unwrap_or_default().to_ascii_lowercase();
    url.scheme() == "https"
        && url.username().is_empty()
        && url.password().is_none()
        && url.port().is_none()
        && (matches!(
            host.as_str(),
            "musicbrainz.org"
                | "coverartarchive.org"
                | "archive.org"
                | "api.discogs.com"
                | "i.discogs.com"
                | "img.discogs.com"
        ) || host.ends_with(".archive.org"))
}
pub fn parse_trusted(url: &str) -> Result<Url, String> {
    Url::parse(url)
        .ok()
        .filter(trusted)
        .ok_or_else(|| "This image address is not from a supported tag source.".to_string())
}

/// A client builder with OpenTag's TLS roots, user agent and the timeout and
/// proxy from Options → Networking.
pub(crate) fn builder(network: &Network) -> Result<reqwest::ClientBuilder, String> {
    let mut roots = rustls::RootCertStore::empty();
    roots.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
    let mut tls = rustls::ClientConfig::builder_with_provider(Arc::new(
        rustls::crypto::ring::default_provider(),
    ))
    .with_safe_default_protocol_versions()
    .map_err(|e| e.to_string())?
    .with_root_certificates(roots)
    .with_no_client_auth();
    tls.alpn_protocols = vec![b"http/1.1".to_vec()];
    let timeout = Duration::from_secs(network.timeout);
    let builder = reqwest::Client::builder()
        .tls_backend_preconfigured(tls)
        .user_agent(USER_AGENT)
        .connect_timeout(timeout.min(Duration::from_secs(10)))
        .timeout(timeout);
    Ok(match network.proxy {
        // reqwest reads the system configuration unless told otherwise.
        ProxyMode::System => builder,
        ProxyMode::None => builder.no_proxy(),
        ProxyMode::Manual => builder.proxy(
            reqwest::Proxy::all(network.proxy_address()?)
                .map_err(|e| format!("The proxy cannot be used: {e}"))?,
        ),
    })
}

type CachedClient = StdMutex<Option<(Network, reqwest::Client)>>;

/// A client from `slot`, rebuilt when the networking options change.
fn cached_client(
    slot: &CachedClient,
    redirect: fn() -> reqwest::redirect::Policy,
) -> Result<reqwest::Client, String> {
    let network = crate::settings::network();
    let mut cached = slot.lock().map_err(|_| "Network client lock failed")?;
    if let Some((_, client)) = cached.as_ref().filter(|(config, _)| *config == network) {
        return Ok(client.clone());
    }
    let client = builder(&network)?
        .redirect(redirect())
        .build()
        .map_err(|e| format!("Network client could not start: {e}"))?;
    *cached = Some((network, client.clone()));
    Ok(client)
}

/// The tag source client: redirects are followed only to trusted hosts.
fn client() -> Result<reqwest::Client, String> {
    static CLIENT: CachedClient = StdMutex::new(None);
    cached_client(&CLIENT, || {
        reqwest::redirect::Policy::custom(|attempt| {
            if attempt.previous().len() >= 5 {
                attempt.error("too many redirects")
            } else if trusted(attempt.url()) {
                attempt.follow()
            } else {
                attempt.error("untrusted redirect")
            }
        })
    })
}

/// The freedb server is user-configured (Options → Tag Sources → freedb), so
/// it is not on the trusted list; its client never follows redirects.
fn freedb_client() -> Result<reqwest::Client, String> {
    static CLIENT: CachedClient = StdMutex::new(None);
    cached_client(&CLIENT, reqwest::redirect::Policy::none)
}

/// Requests through one gate run one at a time, at least `spacing` apart.
pub struct Gate(Mutex<Option<Instant>>);
impl Gate {
    pub const fn new() -> Self {
        Gate(Mutex::const_new(None))
    }
}
pub static MUSICBRAINZ: Gate = Gate::new();
pub static DISCOGS: Gate = Gate::new();
pub static IMAGES: Gate = Gate::new();
/// Cover Art Archive listings can take many seconds; keep them off the image gate.
pub static ARCHIVE: Gate = Gate::new();
pub static FREEDB: Gate = Gate::new();

pub struct Service {
    pub name: &'static str,
    pub gate: &'static Gate,
    pub spacing: Duration,
    pub token: Option<String>,
    /// MusicBrainz answers 503 when its global limit is hit; one retry is polite.
    pub retry_busy: bool,
}

async fn send(
    service: &Service,
    url: &Url,
    accept: &str,
    channel: Channel,
    ticket: u64,
) -> Result<reqwest::Response, String> {
    send_with(&client()?, service, url, accept, channel, ticket).await
}

async fn send_with(
    client: &reqwest::Client,
    service: &Service,
    url: &Url,
    accept: &str,
    channel: Channel,
    ticket: u64,
) -> Result<reqwest::Response, String> {
    let mut last = service.gate.0.lock().await;
    if let Some(previous) = *last {
        let wait = service.spacing.saturating_sub(previous.elapsed());
        if !wait.is_zero() {
            tokio::time::sleep(wait).await;
        }
    }
    if stale(channel, ticket) {
        return Err(CANCELLED.into());
    }
    let mut request = client.get(url.clone()).header(header::ACCEPT, accept);
    if let Some(token) = service.token.as_deref().filter(|t| !t.is_empty()) {
        request = request.header(header::AUTHORIZATION, format!("Discogs token={token}"));
    }
    let result = request.send().await;
    *last = Some(Instant::now());
    result.map_err(|error| network_message(service.name, &error))
}

pub fn network_message(name: &str, error: &reqwest::Error) -> String {
    if error.is_timeout() {
        format!("{name} did not respond in time. Check your connection and try again.")
    } else if error.is_redirect() {
        format!("{name} redirected to an address OpenTag does not trust.")
    } else if error.is_connect() {
        format!("Could not connect to {name}. Check your network connection.")
    } else {
        format!("{name} request failed: {error}")
    }
}

pub fn status_message(name: &str, status: StatusCode, authenticated: bool) -> String {
    match status.as_u16() {
        401 | 403 if name == "Discogs" && authenticated => "Discogs rejected the personal access token. Update it in Tag Sources → Options.".into(),
        401 | 403 if name == "Discogs" => "Discogs requires a personal access token for this request. Add one in Tag Sources → Options.".into(),
        404 => format!("{name} could not find this release."),
        429 => format!("{name} rate limit reached. Wait a minute and try again."),
        503 => format!("{name} is busy. Try again in a moment."),
        code @ 500..=599 => format!("{name} is unavailable right now (HTTP {code})."),
        code => format!("{name} returned HTTP {code}."),
    }
}

pub(crate) async fn read_limited(
    mut response: reqwest::Response,
    limit: usize,
    service: &str,
) -> Result<Vec<u8>, String> {
    let name = service;
    let too_large = || format!("{name} returned more data than expected.");
    if response
        .content_length()
        .is_some_and(|length| length as usize > limit)
    {
        return Err(too_large());
    }
    let mut body = Vec::new();
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|e| network_message(service, &e))?
    {
        if body.len() + chunk.len() > limit {
            return Err(too_large());
        }
        body.extend_from_slice(&chunk);
    }
    Ok(body)
}

pub async fn json(
    service: &Service,
    url: &str,
    channel: Channel,
    ticket: u64,
) -> Result<Value, String> {
    let url = Url::parse(url)
        .ok()
        .filter(trusted)
        .ok_or("Unsupported tag source address")?;
    for attempt in 0..2 {
        let response = send(service, &url, "application/json", channel, ticket).await?;
        let status = response.status();
        if status == StatusCode::SERVICE_UNAVAILABLE && service.retry_busy && attempt == 0 {
            tokio::time::sleep(Duration::from_secs(1)).await;
            continue;
        }
        if !status.is_success() {
            return Err(status_message(
                service.name,
                status,
                service.token.is_some(),
            ));
        }
        let body = read_limited(response, MAX_JSON_BYTES, service.name).await?;
        if stale(channel, ticket) {
            return Err(CANCELLED.into());
        }
        return serde_json::from_slice(&body)
            .map_err(|_| format!("{} returned an unreadable response.", service.name));
    }
    Err(status_message(
        service.name,
        StatusCode::SERVICE_UNAVAILABLE,
        false,
    ))
}

const MAX_TEXT_BYTES: usize = 1024 * 1024;

/// A CDDB protocol response from the configured freedb server, as text.
pub async fn freedb_text(
    service: &Service,
    url: &Url,
    channel: Channel,
    ticket: u64,
) -> Result<String, String> {
    if !matches!(url.scheme(), "http" | "https") || url.host_str().is_none() {
        return Err(
            "The freedb server address is not valid. Check Options → Tag Sources → freedb.".into(),
        );
    }
    let response = send_with(
        &freedb_client()?,
        service,
        url,
        "text/plain",
        channel,
        ticket,
    )
    .await?;
    let status = response.status();
    if status.is_redirection() {
        return Err(format!(
            "{} redirected to another address. Check the server in Options → Tag Sources → freedb.",
            service.name
        ));
    }
    if !status.is_success() {
        return Err(match status.as_u16() {
            404 => format!(
                "{} has no CDDB program at this path. Check Options → Tag Sources → freedb.",
                service.name
            ),
            _ => status_message(service.name, status, false),
        });
    }
    let body = read_limited(response, MAX_TEXT_BYTES, service.name).await?;
    if stale(channel, ticket) {
        return Err(CANCELLED.into());
    }
    Ok(String::from_utf8_lossy(&body).into_owned())
}

/// Release pages that "Preview" may open in the browser: those of the
/// built-in sources and pages a web source found in this session.
pub fn release_page(url: &str) -> Result<Url, String> {
    Url::parse(url)
        .ok()
        .filter(|url| {
            if web_registered(url) {
                return true;
            }
            let host = url.host_str().unwrap_or_default().to_ascii_lowercase();
            url.scheme() == "https"
                && url.username().is_empty()
                && url.password().is_none()
                && url.port().is_none()
                && matches!(
                    host.as_str(),
                    "musicbrainz.org" | "www.discogs.com" | "discogs.com" | "gnudb.org"
                )
        })
        .ok_or_else(|| "This address is not a tag source release page.".to_string())
}

/// Addresses that user-installed web sources produced in this session (album
/// pages, cover images and their thumbnails). Only these may be fetched as
/// images or opened as release pages besides the built-in sources' hosts.
static WEB_URLS: StdMutex<Option<HashSet<String>>> = StdMutex::new(None);
const MAX_WEB_URLS: usize = 20_000;

pub fn register_web_url(url: &Url) {
    if !web_allowed(url) {
        return;
    }
    let mut urls = WEB_URLS
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let urls = urls.get_or_insert_with(HashSet::new);
    if urls.len() >= MAX_WEB_URLS {
        urls.clear();
    }
    urls.insert(url.as_str().to_owned());
}
pub fn web_registered(url: &Url) -> bool {
    WEB_URLS
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .as_ref()
        .is_some_and(|urls| urls.contains(url.as_str()))
}

/// Scripted native test runs must never reach the internet.
fn offline_test_run() -> bool {
    cfg!(any(debug_assertions, feature = "e2e"))
        && std::env::var_os("OPENTAG_E2E_SCRIPT").is_some()
        && std::env::var_os("OPENTAG_E2E_OFFLINE").is_some()
}

/// Web sources may contact any HTTP or HTTPS host the user's script names,
/// without credentials in the address.
pub fn web_allowed(url: &Url) -> bool {
    let local = || {
        let host = url.host_str().unwrap_or_default();
        let host = host.trim_start_matches('[').trim_end_matches(']');
        host.eq_ignore_ascii_case("localhost")
            || host
                .parse::<std::net::IpAddr>()
                .is_ok_and(|ip| ip.is_loopback())
    };
    matches!(url.scheme(), "http" | "https")
        && url.host_str().is_some_and(|host| !host.is_empty())
        && url.username().is_empty()
        && url.password().is_none()
        && (!offline_test_run() || local())
}

pub static WEB: Gate = Gate::new();

fn web_client() -> Result<reqwest::Client, String> {
    static CLIENT: CachedClient = StdMutex::new(None);
    cached_client(&CLIENT, || {
        reqwest::redirect::Policy::custom(|attempt| {
            if attempt.previous().len() >= 10 {
                attempt.error("too many redirects")
            } else if web_allowed(attempt.url()) {
                attempt.follow()
            } else {
                attempt.error("unsupported redirect")
            }
        })
    })
}

/// A downloaded web source page and the address it finally came from.
#[derive(Debug)]
pub struct Page {
    pub text: String,
    pub url: Url,
}

/// Text in the charset the server names, a byte order mark, a `<meta>`
/// charset, UTF-8 when valid, or else Windows-1252.
pub fn decode_text(bytes: &[u8], content_type: Option<&str>) -> String {
    let label = |text: &str| -> Option<&'static encoding_rs::Encoding> {
        let lower = text.to_ascii_lowercase();
        let start = lower.find("charset=")? + "charset=".len();
        let value: String = lower[start..]
            .trim_start_matches(['"', '\''])
            .chars()
            .take_while(|c| c.is_ascii_alphanumeric() || "-_:.".contains(*c))
            .collect();
        encoding_rs::Encoding::for_label(value.as_bytes())
    };
    if let Some((encoding, skip)) = encoding_rs::Encoding::for_bom(bytes) {
        return encoding
            .decode_without_bom_handling(&bytes[skip..])
            .0
            .into_owned();
    }
    let sniffed = || label(&String::from_utf8_lossy(&bytes[..bytes.len().min(4096)]));
    match content_type.and_then(label).or_else(sniffed) {
        Some(encoding) => encoding.decode_without_bom_handling(bytes).0.into_owned(),
        None => match std::str::from_utf8(bytes) {
            Ok(text) => text.to_owned(),
            Err(_) => encoding_rs::WINDOWS_1252.decode(bytes).0.into_owned(),
        },
    }
}

/// Download a page for a web source. `user_agent` replaces OpenTag's own.
pub async fn web_page(
    name: &str,
    url: &str,
    user_agent: Option<&str>,
    channel: Channel,
    ticket: u64,
) -> Result<Page, String> {
    let url = Url::parse(url)
        .ok()
        .filter(web_allowed)
        .ok_or_else(|| format!("{name} produced an address OpenTag cannot open: {url}"))?;
    let client = web_client()?;
    let response = {
        let mut last = WEB.0.lock().await;
        if stale(channel, ticket) {
            return Err(CANCELLED.into());
        }
        let mut request = client.get(url.clone());
        if let Some(agent) = user_agent {
            request = request.header(header::USER_AGENT, agent);
        }
        let result = request.send().await;
        *last = Some(Instant::now());
        result.map_err(|error| {
            if error.is_redirect() {
                format!("{name} redirected to an address OpenTag cannot open.")
            } else {
                network_message(name, &error)
            }
        })?
    };
    let status = response.status();
    if !status.is_success() {
        return Err(format!(
            "{name} returned HTTP {} for {url}",
            status.as_u16()
        ));
    }
    let final_url = response.url().clone();
    let content_type = response
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned);
    let body = read_limited(response, MAX_JSON_BYTES, name).await?;
    if stale(channel, ticket) {
        return Err(CANCELLED.into());
    }
    Ok(Page {
        text: decode_text(&body, content_type.as_deref()),
        url: final_url,
    })
}

pub async fn image(url: &str, channel: Channel, ticket: u64) -> Result<Image, String> {
    let (url, client) = match Url::parse(url) {
        // Cover addresses found by a web source; its client follows redirects
        // to other web hosts.
        Ok(url) if !trusted(&url) && web_registered(&url) => (url, web_client()?),
        _ => (parse_trusted(url)?, client()?),
    };
    let service = Service {
        name: "The image server",
        gate: &IMAGES,
        spacing: Duration::ZERO,
        token: None,
        retry_busy: false,
    };
    let response = send_with(&client, &service, &url, "image/*", channel, ticket).await?;
    if !response.status().is_success() {
        return Err(match response.status().as_u16() {
            404 => "This release has no image at that address.".into(),
            _ => status_message(service.name, response.status(), false),
        });
    }
    let bytes = read_limited(response, MAX_IMAGE_BYTES, service.name).await?;
    if stale(channel, ticket) {
        return Err(CANCELLED.into());
    }
    Image::validate(bytes)
}

/// Downloaded bytes that decode as a supported cover image.
pub struct Image {
    pub bytes: Vec<u8>,
    pub mime: &'static str,
    pub extension: &'static str,
}
impl Image {
    pub fn validate(bytes: Vec<u8>) -> Result<Image, String> {
        use image::ImageFormat;
        let invalid = || "The downloaded cover is not a supported image.".to_string();
        let format = image::guess_format(&bytes).map_err(|_| invalid())?;
        let (mime, extension) = match format {
            ImageFormat::Jpeg => ("image/jpeg", "jpg"),
            ImageFormat::Png => ("image/png", "png"),
            ImageFormat::Gif => ("image/gif", "gif"),
            ImageFormat::WebP => ("image/webp", "webp"),
            ImageFormat::Bmp => ("image/bmp", "bmp"),
            _ => return Err(invalid()),
        };
        let (width, height) = image::ImageReader::with_format(std::io::Cursor::new(&bytes), format)
            .into_dimensions()
            .map_err(|_| invalid())?;
        if width == 0 || height == 0 || width > 20_000 || height > 20_000 {
            return Err(invalid());
        }
        Ok(Image {
            bytes,
            mime,
            extension,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_https_source_hosts_are_trusted() {
        for good in [
            "https://musicbrainz.org/ws/2/release/x",
            "https://ia800.us.archive.org/a.jpg",
            "https://i.discogs.com/x.jpeg",
            "https://coverartarchive.org/release/x/front-250",
        ] {
            assert!(trusted(&Url::parse(good).unwrap()), "{good}");
        }
        for bad in [
            "http://coverartarchive.org/x",
            "https://example.com/x",
            "https://evilarchive.org/x",
            "https://musicbrainz.org:8443/x",
            "https://user@i.discogs.com/x",
            "file:///etc/passwd",
        ] {
            assert!(!trusted(&Url::parse(bad).unwrap()), "{bad}");
        }
        assert!(parse_trusted("not a url").is_err());
    }

    #[test]
    fn tickets_cancel_older_requests_per_channel() {
        // Tickets are global; use values far above any other test's.
        claim(Channel::Search, 1_000_010);
        assert!(stale(Channel::Search, 1_000_009));
        assert!(!stale(Channel::Search, 1_000_010));
        cancel(1_000_020);
        assert!(stale(Channel::Release, 1_000_019));
        assert!(!stale(Channel::Image, 1_000_020));
    }

    #[test]
    fn status_messages_explain_tokens_limits_and_outages() {
        assert!(status_message("Discogs", StatusCode::UNAUTHORIZED, true).contains("rejected"));
        assert!(status_message("Discogs", StatusCode::UNAUTHORIZED, false).contains("requires"));
        assert!(
            status_message("MusicBrainz", StatusCode::TOO_MANY_REQUESTS, false)
                .contains("rate limit")
        );
        assert!(status_message("MusicBrainz", StatusCode::BAD_GATEWAY, false).contains("502"));
        assert!(
            USER_AGENT.starts_with("OpenTag/")
                && USER_AGENT.ends_with("( https://github.com/johnneerdael/opentag )")
        );
    }

    #[test]
    fn images_are_validated_by_content() {
        let mut png = Vec::new();
        image::DynamicImage::new_rgb8(2, 3)
            .write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
            .unwrap();
        let image = Image::validate(png).unwrap();
        assert_eq!((image.mime, image.extension), ("image/png", "png"));
        assert!(Image::validate(b"<html>not an image</html>".to_vec()).is_err());
        assert!(
            Image::validate(vec![0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a]).is_err(),
            "truncated header"
        );
    }

    #[test]
    fn client_builds_with_the_ring_provider() {
        assert!(client().is_ok());
        assert!(freedb_client().is_ok());
    }

    #[test]
    fn preview_opens_only_release_pages_of_known_sources() {
        for good in [
            "https://musicbrainz.org/release/d073287b-d1bd-4f11-a933-a4386f8cf701",
            "https://www.discogs.com/release/1225",
            "https://gnudb.org/cd/ro7a0a6b09",
        ] {
            assert!(release_page(good).is_ok(), "{good}");
        }
        for bad in [
            "http://musicbrainz.org/release/x",
            "https://example.com/",
            "file:///etc/passwd",
            "javascript:alert(1)",
            "https://musicbrainz.org.evil.com/",
        ] {
            assert!(release_page(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn networking_options_configure_the_client() {
        for proxy in [ProxyMode::System, ProxyMode::None, ProxyMode::Manual] {
            let network = Network {
                timeout: 12,
                proxy,
                proxy_url: "127.0.0.1:3128".into(),
            };
            assert!(builder(&network).unwrap().build().is_ok(), "{proxy:?}");
        }
        let broken = Network {
            proxy: ProxyMode::Manual,
            proxy_url: "user:secret@proxy:1".into(),
            ..Network::default()
        };
        assert!(builder(&broken).is_err());
    }

    #[test]
    fn web_addresses_are_checked_and_registered() {
        let url = |text: &str| Url::parse(text).unwrap();
        assert!(web_allowed(&url("http://127.0.0.1:8080/search?q=x")));
        assert!(web_allowed(&url("https://example.org/a")));
        for bad in [
            "ftp://example.org/a",
            "file:///etc/passwd",
            "https://user:pw@example.org/",
            "data:text/plain,x",
        ] {
            assert!(!web_allowed(&url(bad)), "{bad}");
        }
        let cover = url("https://images.example.org/cover-web-test.jpg");
        assert!(!web_registered(&cover));
        assert!(parse_trusted(cover.as_str()).is_err());
        assert!(release_page(cover.as_str()).is_err());
        register_web_url(&cover);
        assert!(web_registered(&cover));
        assert!(release_page(cover.as_str()).is_ok());
        register_web_url(&url("file:///etc/passwd"));
        assert!(!web_registered(&url("file:///etc/passwd")));
    }

    #[test]
    fn web_pages_decode_by_declared_or_sniffed_charset() {
        assert_eq!(
            decode_text(b"caf\xE9", Some("text/html; charset=ISO-8859-1")),
            "café"
        );
        assert_eq!(decode_text("café".as_bytes(), Some("text/html")), "café");
        assert_eq!(decode_text(b"caf\xE9", None), "café");
        assert_eq!(
            decode_text(b"<meta charset=\"windows-1251\">\xC0", None),
            "<meta charset=\"windows-1251\">А"
        );
        assert_eq!(
            decode_text(b"\xEF\xBB\xBFok", Some("text/html; charset=latin1")),
            "ok"
        );
    }

    #[test]
    fn web_pages_are_fetched_with_redirects_and_user_agent() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let server = std::thread::spawn(move || {
            use std::io::{Read, Write};
            let mut requests = Vec::new();
            for answer in [
                "HTTP/1.1 302 Found\r\nlocation: /final\r\nconnection: close\r\ncontent-length: 0\r\n\r\n".to_string(),
                "HTTP/1.1 200 OK\r\ncontent-type: text/html; charset=iso-8859-1\r\nconnection: close\r\ncontent-length: 3\r\n\r\nn\u{e9}e".to_string(),
                "HTTP/1.1 404 Not Found\r\nconnection: close\r\ncontent-length: 0\r\n\r\n".to_string(),
            ] {
                let (mut socket, _) = listener.accept().unwrap();
                let mut request = [0; 2048];
                let n = socket.read(&mut request).unwrap();
                requests.push(String::from_utf8_lossy(&request[..n]).into_owned());
                // Each char is one byte: the body is Latin-1.
                let bytes: Vec<u8> = answer.chars().map(|c| c as u8).collect();
                socket.write_all(&bytes).unwrap();
            }
            requests
        });
        let base = format!("http://127.0.0.1:{port}");
        let page = tauri::async_runtime::block_on(web_page(
            "Fixture",
            &format!("{base}/start"),
            Some("Custom/1.0"),
            Channel::Search,
            u64::MAX,
        ))
        .unwrap();
        assert_eq!(page.text, "née");
        assert_eq!(page.url.path(), "/final");
        let missing = tauri::async_runtime::block_on(web_page(
            "Fixture",
            &format!("{base}/gone"),
            None,
            Channel::Search,
            u64::MAX,
        ))
        .unwrap_err();
        assert!(missing.contains("HTTP 404"), "{missing}");
        let requests = server.join().unwrap();
        assert!(requests[0]
            .to_ascii_lowercase()
            .contains("user-agent: custom/1.0"));
        assert!(requests[2]
            .to_ascii_lowercase()
            .contains("user-agent: opentag/"));
        assert!(tauri::async_runtime::block_on(web_page(
            "Fixture",
            "ftp://x/",
            None,
            Channel::Search,
            u64::MAX
        ))
        .unwrap_err()
        .contains("cannot open"));
    }

    #[test]
    fn manual_proxy_carries_requests() {
        // A one-shot proxy on loopback answers the tunnel request with 403.
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let seen = std::thread::spawn(move || {
            use std::io::{Read, Write};
            let (mut socket, _) = listener.accept().unwrap();
            let mut request = [0; 512];
            let n = socket.read(&mut request).unwrap();
            socket
                .write_all(b"HTTP/1.1 403 Forbidden\r\ncontent-length: 0\r\n\r\n")
                .unwrap();
            String::from_utf8_lossy(&request[..n]).into_owned()
        });
        let network = Network {
            timeout: 5,
            proxy: ProxyMode::Manual,
            proxy_url: format!("127.0.0.1:{port}"),
        };
        let result = tauri::async_runtime::block_on(tauri::async_runtime::spawn(async move {
            let client = builder(&network).unwrap().build().unwrap();
            client
                .get("https://musicbrainz.org/ws/2/release/x")
                .send()
                .await
                .map(|_| ())
        }))
        .unwrap();
        assert!(result.is_err(), "the proxy refused the tunnel");
        assert!(seen
            .join()
            .unwrap()
            .starts_with("CONNECT musicbrainz.org:443"));
    }
}
