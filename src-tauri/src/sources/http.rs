//! Network hygiene for tag sources: one shared rustls client, a trusted host
//! list (also enforced on redirects), serialised per-service rate limits, size
//! limits and superseded-request cancellation.
use crate::settings::{Network, ProxyMode};
use reqwest::{header, StatusCode, Url};
use serde_json::Value;
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

/// The tag source client, rebuilt when the networking options change.
fn client() -> Result<reqwest::Client, String> {
    static CLIENT: StdMutex<Option<(Network, reqwest::Client)>> = StdMutex::new(None);
    let network = crate::settings::network();
    let mut cached = CLIENT.lock().map_err(|_| "Network client lock failed")?;
    if let Some((_, client)) = cached.as_ref().filter(|(config, _)| *config == network) {
        return Ok(client.clone());
    }
    let client = builder(&network)?
        .redirect(reqwest::redirect::Policy::custom(|attempt| {
            if attempt.previous().len() >= 5 {
                attempt.error("too many redirects")
            } else if trusted(attempt.url()) {
                attempt.follow()
            } else {
                attempt.error("untrusted redirect")
            }
        }))
        .build()
        .map_err(|e| format!("Network client could not start: {e}"))?;
    *cached = Some((network, client.clone()));
    Ok(client)
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
    let mut request = client()?.get(url.clone()).header(header::ACCEPT, accept);
    if let Some(token) = service.token.as_deref().filter(|t| !t.is_empty()) {
        request = request.header(header::AUTHORIZATION, format!("Discogs token={token}"));
    }
    let result = request.send().await;
    *last = Some(Instant::now());
    result.map_err(|error| network_message(service.name, &error))
}

pub fn network_message(service: &str, error: &reqwest::Error) -> String {
    if error.is_timeout() {
        format!("{service} did not respond in time. Check your connection and try again.")
    } else if error.is_redirect() {
        format!("{service} redirected to an address OpenTag does not trust.")
    } else if error.is_connect() {
        format!("Could not connect to {service}. Check your network connection.")
    } else {
        format!("{service} request failed: {error}")
    }
}

pub fn status_message(service: &str, status: StatusCode, authenticated: bool) -> String {
    match status.as_u16() {
        401 | 403 if service == "Discogs" && authenticated => "Discogs rejected the personal access token. Update it in Tag Sources → Options.".into(),
        401 | 403 if service == "Discogs" => "Discogs requires a personal access token for this request. Add one in Tag Sources → Options.".into(),
        404 => format!("{service} could not find this release."),
        429 => format!("{service} rate limit reached. Wait a minute and try again."),
        503 => format!("{service} is busy. Try again in a moment."),
        code @ 500..=599 => format!("{service} is unavailable right now (HTTP {code})."),
        code => format!("{service} returned HTTP {code}."),
    }
}

pub(crate) async fn read_limited(
    mut response: reqwest::Response,
    limit: usize,
    service: &str,
) -> Result<Vec<u8>, String> {
    let too_large = || format!("{service} returned more data than expected.");
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

pub async fn image(url: &str, channel: Channel, ticket: u64) -> Result<Image, String> {
    let url = parse_trusted(url)?;
    let service = Service {
        name: "The image server",
        gate: &IMAGES,
        spacing: Duration::ZERO,
        token: None,
        retry_busy: false,
    };
    let response = send(&service, &url, "image/*", channel, ticket).await?;
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
