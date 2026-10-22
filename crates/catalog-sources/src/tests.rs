use super::*;
const RELEASE: &str = r#"<?xml version="1.0"?><root><data><![CDATA[<div id="ttlPageCont" data-tid="42"><div class="ttl-block"><div class="t-image"><img src="https://geo-static.traxsource.com/cover.jpg"></div><div class="page-head"><h1 class="title">Night Signals</h1><h1 class="artists">Example Artist</h1><a class="com-label">Example Label</a></div><div class="cat-rdate">EX042 | 2026-01-02</div></div><div id="trkListCont"><div class="trklist"><div class="trk-row" data-trid="81"><div class="tnum">1</div><div class="title"><a href="/track/81/first">First</a><span class="version">Original Mix <span class="duration">(5:12)</span></span></div><div class="artists"><a>Example Artist</a></div><div class="genre"><a>House</a></div></div><div class="trk-row" data-trid="82"><div class="tnum">2</div><div class="title"><a href="/track/82/second">Second</a><span class="version">Extended Mix <span class="duration">(6:01)</span></span></div><div class="artists"><a>Example Artist</a></div></div></div></div><div class="more-from-list"><div class="trk-row" data-trid="999"><div class="title"><a href="/track/999/unrelated">Unrelated</a></div></div></div></div>]]></data></root>"#;
#[test]
fn traxsource_full_release_keeps_identity_order_and_ignores_recommendations() {
    let r = parse_traxsource_release(RELEASE, "42").unwrap();
    assert_eq!(r.id, "42");
    assert_eq!(r.title, "Night Signals");
    assert_eq!(r.tracks.len(), 2);
    assert_eq!(r.tracks[1].id, "82");
    assert_eq!(r.tracks[1].number, 2);
    assert_eq!(r.tracks[1].release_id, "42");
    assert_eq!(r.tracks[1].version, "Extended Mix");
    assert_eq!(r.tracks[1].length, Some(361.));
    assert_eq!(r.fields["CATALOGNUMBER"], vec!["EX042"]);
}
#[test]
fn traxsource_wrong_release_identity_is_rejected() {
    assert!(parse_traxsource_release(RELEASE, "43").is_err());
}

fn traxsource_covers(address: &str) -> [Option<String>; 3] {
    let release = RELEASE.replace("https://geo-static.traxsource.com/cover.jpg", address);
    let release_search = format!(
        r#"<div class="release-grid"><div class="grid-item" data-tid="42"><img src="{address}"><div class="links"><a class="com-title" href="/title/42/night-signals">Night Signals</a></div></div></div>"#
    );
    let track_search = format!(
        r#"<div id="searchTrackList"><div class="trk-row" data-trid="81"><div class="thumb"><img src="{address}"></div><div class="title"><a href="/track/81/first">First</a></div></div></div>"#
    );
    [
        parse_traxsource_release(&release, "42").unwrap().cover,
        parse_traxsource_search(&release_search, true)
            .unwrap()
            .results
            .remove(0)
            .cover,
        parse_traxsource_search(&track_search, false)
            .unwrap()
            .results
            .remove(0)
            .cover,
    ]
}

#[test]
fn traxsource_covers_keep_all_supported_artwork_hosts() {
    for host in [
        "www.traxsource.com",
        "traxsource.com",
        "geo-static.traxsource.com",
        "geo-static2.traxsource.com",
        "geo-w-static.traxsource.com",
        "w-static.traxsource.com",
    ] {
        let address = format!("https://{host}/cover.jpg");
        assert_eq!(
            traxsource_covers(&address),
            [Some(address.clone()), Some(address.clone()), Some(address)],
            "release detail, release search and track search must retain {host} covers"
        );
    }
}

#[test]
fn traxsource_covers_reject_unsupported_and_lookalike_hosts() {
    for address in [
        "https://geo-w-static.traxsource.com.example.invalid/cover.jpg",
        "https://evilw-static.traxsource.com/cover.jpg",
        "https://images.w-static.traxsource.com/cover.jpg",
        "https://w-static.traxsource.com@example.invalid/cover.jpg",
        "http://w-static.traxsource.com/cover.jpg",
    ] {
        assert_eq!(traxsource_covers(address), [None, None, None], "{address}");
    }
}

#[test]
fn beatport_artwork_comes_from_the_media_host_the_api_returns() {
    // The shape of a live release: the image is a template on geo-media.beatport.com.
    let release = serde_json::json!({
        "id": 7386956, "name": "Digital Renaissance Vol. 1",
        "image": {
            "id": 58414071,
            "uri": "https://geo-media.beatport.com/image_size/1400x1400/6389d0da-09b9-43dc-b4ea-91b30f4d79aa.jpg",
            "dynamic_uri": "https://geo-media.beatport.com/image_size/{w}x{h}/6389d0da-09b9-43dc-b4ea-91b30f4d79aa.jpg"
        }
    });
    let parsed = beatport::parse_release_value(&release).unwrap();
    assert_eq!(
        parsed.cover.as_deref(),
        Some("https://geo-media.beatport.com/image_size/1000x1000/6389d0da-09b9-43dc-b4ea-91b30f4d79aa.jpg")
    );
    // Without the template the plain address is used; the older media host still works.
    assert_eq!(
        beatport::cover(
            &serde_json::json!({"uri": "https://geo-media.beatport.com/image_size/1400x1400/x.jpg"})
        )
        .as_deref(),
        Some("https://geo-media.beatport.com/image_size/1400x1400/x.jpg")
    );
    assert!(beatport::cover(
        &serde_json::json!({"uri": "https://geo-media.beatportimage.com/x.jpg"})
    )
    .is_some());
    // Lookalikes, other schemes and other hosts are still refused.
    for address in [
        "https://geo-media.beatport.com.example.invalid/x.jpg",
        "https://evil-geo-media.beatport.com/x.jpg",
        "https://www.beatport.com/x.jpg",
        "http://geo-media.beatport.com/x.jpg",
        "https://user@geo-media.beatport.com@example.invalid/x.jpg",
    ] {
        assert!(
            beatport::cover(&serde_json::json!({"uri": address})).is_none(),
            "{address}"
        );
    }
}

#[test]
fn empty_results_are_different_from_malformed_or_challenged() {
    assert!(parse_traxsource_search(
        "<div class='search-res-cont'><h2>No results found for example</h2></div>",
        false
    )
    .unwrap()
    .results
    .is_empty());
    assert!(parse_traxsource_search("<html>unrelated</html>", false).is_err());
    assert!(parse_traxsource_release(
        "<div id='ttlPageCont' data-tid='42'><h1 class='title'>Empty</h1></div>",
        "42"
    )
    .is_err());
    assert!(parse_traxsource_search(
        "<title>Just a moment</title><script src='/cdn-cgi/challenge-platform/a'></script>",
        false
    )
    .is_err());
}
#[test]
fn traxsource_track_search_retains_track_and_exact_release_identity() {
    let p=parse_traxsource_search(r#"<div id="searchTrackList"><div class="trk-row" data-trid="82"><div class="title"><a href="/track/82/second">Second</a><span class="version">Extended Mix <span class="duration">(6:01)</span></span></div><div class="artists"><a>Example Artist</a></div><a data-cart="{title_id: 42, track_id: 82}"></a></div></div><a id="tab-tracks">Tracks (51)</a>"#,false).unwrap();
    assert_eq!(p.results[0].id, "track:82");
    assert_eq!(p.results[0].release_id.as_deref(), Some("release:42"));
    assert_eq!(p.results[0].title, "Second (Extended Mix)");
    assert_eq!(p.total, Some(51));
}
#[test]
fn beatport_single_track_keeps_actual_release_position() {
    let t=parse_beatport_track(r#"{"id":82,"name":"Second","mix_name":"Extended Mix","number":2,"artists":[{"name":"Example Artist"}],"release":{"id":42,"name":"Night Signals","label":{"name":"Example Label"}},"length_ms":361000,"bpm":125,"key":{"name":"A Minor"}}"#).unwrap();
    assert_eq!(t.number, 2);
    assert_eq!(t.id, "82");
    assert_eq!(t.release_id, "42");
    assert_eq!(t.version, "Extended Mix");
    assert_eq!(t.length, Some(361.));
    assert_eq!(t.fields["BPM"], vec!["125"]);
}

fn server(answers: Vec<String>) -> (String, std::thread::JoinHandle<Vec<String>>) {
    use std::io::{Read, Write};
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let address = format!("http://{}", listener.local_addr().unwrap());
    let task = std::thread::spawn(move || {
        answers
            .into_iter()
            .map(|answer| {
                let (mut socket, _) = listener.accept().unwrap();
                let mut request = vec![];
                let mut buf = [0; 4096];
                loop {
                    let n = socket.read(&mut buf).unwrap();
                    request.extend_from_slice(&buf[..n]);
                    let text = String::from_utf8_lossy(&request);
                    if let Some(end) = text.find("\r\n\r\n") {
                        let length = text
                            .lines()
                            .find_map(|line| {
                                line.to_ascii_lowercase()
                                    .strip_prefix("content-length:")
                                    .map(str::to_owned)
                            })
                            .and_then(|s| s.trim().parse::<usize>().ok())
                            .unwrap_or(0);
                        if request.len() >= end + 4 + length {
                            break;
                        }
                    }
                    if n == 0 {
                        break;
                    }
                }
                socket.write_all(answer.as_bytes()).unwrap();
                String::from_utf8_lossy(&request).into_owned()
            })
            .collect()
    });
    (address, task)
}
fn response(status: u16, body: &str) -> String {
    format!(
        "HTTP/1.1 {status} Fixture\r\nconnection: close\r\ncontent-length: {}\r\n\r\n{body}",
        body.len()
    )
}
fn response_with(status: u16, headers: &str, body: &str) -> String {
    format!(
        "HTTP/1.1 {status} Fixture\r\n{headers}connection: close\r\ncontent-length: {}\r\n\r\n{body}",
        body.len()
    )
}
fn transport(solver: Option<String>, cancelled: http::Cancel) -> http::Http {
    let _ = rustls::crypto::ring::default_provider().install_default();
    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .unwrap();
    http::Http::new(
        client.clone(),
        client,
        std::sync::Arc::default(),
        http::SolverConfig {
            endpoint: solver,
            timeout_seconds: 1,
        },
        cancelled,
        std::sync::Arc::new(|url| Ok(url.clone())),
    )
}
#[tokio::test]
async fn challenge_never_becomes_empty_and_solver_is_explicit() {
    let (base, task) = server(vec![response(
        403,
        "<title>Just a moment</title><script src='/cdn-cgi/challenge-platform/a'></script>",
    )]);
    let error = transport(None, std::sync::Arc::new(|| false))
        .get(&reqwest::Url::parse(&base).unwrap(), None, true)
        .await
        .unwrap_err();
    assert!(error.contains("FlareSolverr-compatible solver"), "{error}");
    assert!(error.contains("Cloudflare solver"), "{error}");
    task.join().unwrap();
}
#[tokio::test]
async fn solver_retries_once_without_forwarding_credentials() {
    let (base, direct) = server(vec![response(
        403,
        "<title>Just a moment</title><script src='/cdn-cgi/challenge-platform/a'></script>",
    )]);
    let(solver,solved)=server(vec![response(503,"busy"),response(200,&serde_json::json!({"status":"ok","solution":{"url":format!("{base}/"),"status":200,"response":"<div>catalog</div>"}}).to_string())]);
    let http = transport(Some(solver), std::sync::Arc::new(|| false));
    let page = http
        .get(&reqwest::Url::parse(&base).unwrap(), None, true)
        .await
        .unwrap();
    assert_eq!(page, "<div>catalog</div>");
    direct.join().unwrap();
    let requests = solved.join().unwrap();
    assert_eq!(requests.len(), 2);
    for r in requests {
        assert!(!r.to_ascii_lowercase().contains("authorization:"));
        assert!(!r.to_ascii_lowercase().contains("cookie:"));
        assert!(r.contains("request.get"));
    }
}
#[tokio::test]
async fn cancelled_request_performs_no_network() {
    let error = transport(None, std::sync::Arc::new(|| true))
        .get(
            &reqwest::Url::parse("http://127.0.0.1:9").unwrap(),
            None,
            true,
        )
        .await
        .unwrap_err();
    assert!(error.contains("Cancelled"), "{error}");
}
#[tokio::test]
async fn ordinary_http_failure_does_not_invoke_solver() {
    let (base, task) = server(vec![response(403, "Forbidden")]);
    let error = transport(
        Some("http://127.0.0.1:9".into()),
        std::sync::Arc::new(|| false),
    )
    .get(&reqwest::Url::parse(&base).unwrap(), None, true)
    .await
    .unwrap_err();
    assert!(error.contains("403"), "{error}");
    task.join().unwrap();
}
#[test]
fn release_search_retains_exact_release_link() {
    let p=parse_traxsource_search(r#"<div class="release-grid"><div class="grid-item" data-tid="42"><div class="links"><a class="com-title" href="/title/42/night-signals">Night Signals</a>Example Artist<a class="com-label">Example Label</a></div></div></div><a id="tab-titles">Releases (19)</a>"#,true).unwrap();
    assert_eq!(p.results[0].id, "release:42");
    assert_eq!(p.results[0].artists, vec!["Example Artist"]);
    assert_eq!(p.total, Some(19));
}

fn catalog(http: http::Http, base: &str) -> Catalog {
    Catalog::with_endpoints(
        http,
        Endpoints {
            beatport_api: base.into(),
            beatport_auth: base.into(),
            traxsource: base.into(),
        },
    )
}
#[tokio::test]
async fn traxsource_track_fetches_its_own_release_and_preserves_track_two() {
    let (base, task) = server(vec![
        response(
            200,
            r#"<div class="track-page"><div class="ttl-info"><a href="/title/42/night-signals">Night Signals</a></div><div class="btns ptk-82"></div></div>"#,
        ),
        response(200, RELEASE),
    ]);
    let c = catalog(transport(None, std::sync::Arc::new(|| false)), &base);
    let r = c.release(Provider::Traxsource, "track:82").await.unwrap();
    assert_eq!(r.tracks.len(), 1);
    assert_eq!(r.tracks[0].number, 2);
    assert_eq!(r.tracks[0].id, "82");
    assert_eq!(r.id, "42");
    let requests = task.join().unwrap();
    assert!(requests[0].contains("/track/82"));
    assert!(requests[1].contains("/title/42"));
}
fn release_grid(start: u32, count: u32, total: u32) -> String {
    let mut html = format!("<a id='tab-titles'>Releases ({total})</a><div class='release-grid'>");
    for id in start..start + count {
        html.push_str(&format!("<div class='grid-item' data-tid='{id}'><div class='links'><a class='com-title' href='/title/{id}/example'>Example {id}</a>Artist</div></div>"));
    }
    html.push_str("</div>");
    html
}
#[tokio::test]
async fn traxsource_native_18_item_pages_are_normalized_to_50() {
    let (base, task) = server(vec![
        response(200, &release_grid(37, 18, 86)),
        response(200, &release_grid(55, 18, 86)),
        response(200, &release_grid(73, 14, 86)),
    ]);
    let c = catalog(transport(None, std::sync::Arc::new(|| false)), &base);
    let p = c
        .search(Provider::Traxsource, true, "Artist", 1)
        .await
        .unwrap();
    assert_eq!(p.total, Some(86));
    assert_eq!(p.results.len(), 36);
    assert_eq!(p.results[0].id, "release:51");
    assert_eq!(p.results[35].id, "release:86");
    let requests = task.join().unwrap();
    for (i, r) in requests.iter().enumerate() {
        assert!(r.contains(&format!("page={}", i + 3)));
    }
}
#[tokio::test]
async fn beatport_release_follows_all_track_pages() {
    let track = |id: u32, n: u32| serde_json::json!({"id":id,"name":format!("Track {id}"),"number":n,"artists":[{"name":"Artist"}],"release":{"id":42,"name":"Release"}});
    let(base,task)=server(vec![response(200,r#"{"access_token":"fixture-token","expires_in":300}"#),response(200,r#"{"id":42,"name":"Release","track_count":2}"#),response(200,&serde_json::json!({"results":[track(81,1)],"next":"https://api.beatport.com/v4/catalog/releases/42/tracks?page=2"}).to_string()),response(200,&serde_json::json!({"results":[track(82,2)],"next":null}).to_string())]);
    let c = catalog(transport(None, std::sync::Arc::new(|| false)), &base);
    let r = c.release(Provider::Beatport, "release:42").await.unwrap();
    assert_eq!(r.tracks.len(), 2);
    assert_eq!(r.tracks[1].number, 2);
    assert_eq!(r.tracks[1].id, "82");
    let requests = task.join().unwrap();
    assert!(requests[0].starts_with("POST /o/token/"));
    assert!(requests[2].contains("page=1"));
    assert!(requests[3].contains("page=2"));
    assert!(requests[2]
        .to_ascii_lowercase()
        .contains("authorization: bearer fixture-token"));
}

#[test]
fn beatport_preserves_release_date_key_and_native_remixer_field() {
    let t=parse_beatport_track(r#"{"id":82,"name":"Second","mix_name":"Extended Mix","number":2,"artists":[{"name":"Artist"}],"release":{"id":42,"name":"Release"},"new_release_date":"2026-01-02","publish_date":"2026-02-03","key":{"name":"A Minor"},"remixers":[{"name":"Remixer"}]}"#).unwrap();
    assert_eq!(t.fields["DATE"], vec!["2026-01-02"]);
    assert_eq!(t.fields["RELEASETIME"], vec!["2026-02-03"]);
    assert_eq!(t.fields["INITIALKEY"], vec!["Am"]);
    assert_eq!(t.fields["MIXARTIST"], vec!["Remixer"]);
}

#[test]
fn search_track_direct_release_link_retains_both_identities() {
    let page=parse_traxsource_search(r#"<div id="searchTrackList"><div class="trk-row" data-trid="82"><div class="title"><a href="/title/42/night-signals">Second</a></div><div class="artists"><a>Artist</a></div></div></div>"#,false).unwrap();
    assert_eq!(page.results[0].id, "track:82");
    assert_eq!(page.results[0].release_id.as_deref(), Some("release:42"));
}
#[test]
fn declared_release_tracklist_must_be_complete() {
    let incomplete = RELEASE.replace(
        "<div class=\"cat-rdate\">",
        "<a class='com-play-all' data-tracks='81,82,83'></a><div class=\"cat-rdate\">",
    );
    assert!(parse_traxsource_release(&incomplete, "42").is_err());
}

#[tokio::test]
async fn a_status_the_site_returns_through_the_solver_keeps_its_meaning() {
    // A blocked host sends every request through the solver; the site's own answers still count.
    for (solved_status, body) in [(404, "<p>Not found</p>"), (404, ""), (500, "oops")] {
        let block = response_with(
            403,
            "server: cloudflare\r\ncf-ray: 8a1b2c3d4e5f-AMS\r\ncontent-type: text/html\r\n",
            "<title>Attention Required! | Cloudflare</title>Sorry, you have been blocked.",
        );
        let (base, direct) = server(vec![block]);
        let (solver, solved) = server(vec![response(
            200,
            &serde_json::json!({"status":"ok","solution":{"url":format!("{base}/"),"status":solved_status,"response":body}}).to_string(),
        )]);
        let error = transport(Some(solver), std::sync::Arc::new(|| false))
            .get(&reqwest::Url::parse(&base).unwrap(), None, true)
            .await
            .unwrap_err();
        assert_eq!(error, format!("Catalog HTTP {solved_status}"), "{body:?}");
        direct.join().unwrap();
        solved.join().unwrap();
    }
}
#[tokio::test]
async fn the_solver_may_end_on_the_site_a_redirect_is_known_to_lead_to() {
    let (solver_base, solved) = server(vec![
        response(
            200,
            &serde_json::json!({"status":"ok","solution":{"url":"https://b.example/landing","status":200,"response":"<p>page</p>"}}).to_string(),
        ),
        response(
            200,
            &serde_json::json!({"status":"ok","solution":{"url":"https://b.example/landing","status":200,"response":"<p>page</p>"}}).to_string(),
        ),
    ]);
    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .unwrap();
    let api = reqwest::Url::parse(&format!("{solver_base}/v1")).unwrap();
    let from = reqwest::Url::parse("https://a.example/start").unwrap();
    let landed = reqwest::Url::parse("https://b.example/").unwrap();
    // Without the known destination the solver ending elsewhere is refused; with it, accepted.
    let refused = solver::solve(&client, &api, 1, &from, None).await;
    assert!(
        matches!(refused, Err(solver::SolveError::Permanent(m)) if m.contains("outside the original site"))
    );
    let page = solver::solve(&client, &api, 1, &from, Some(&landed))
        .await
        .ok()
        .unwrap();
    assert_eq!(page.body, "<p>page</p>");
    solved.join().unwrap();
}

#[tokio::test]
async fn a_solver_that_cannot_be_reached_is_not_reported_as_busy() {
    let client = reqwest::Client::new();
    let api = reqwest::Url::parse("http://127.0.0.1:1/v1").unwrap();
    let target = reqwest::Url::parse("https://site.example/").unwrap();
    match solver::solve(&client, &api, 1, &target, None).await {
        Err(solver::SolveError::Permanent(message)) => {
            assert!(message.contains("Could not reach the solver"), "{message}")
        }
        other => panic!("expected a setup error, got {}", other.is_ok()),
    }
}

#[tokio::test]
async fn a_cloudflare_error_page_the_solver_ended_on_is_not_the_sites_answer() {
    let page = "<title>Attention Required! | Cloudflare</title>Sorry, you have been blocked. Cloudflare Ray ID: 1";
    let envelope = serde_json::json!({"status":"ok","solution":{"url":"https://site.example/x","status":403,"response":page,"headers":{"server":"cloudflare","cf-ray":"8a1b"}}}).to_string();
    let (solver_base, solved) = server(vec![response(200, &envelope)]);
    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .unwrap();
    let api = reqwest::Url::parse(&format!("{solver_base}/v1")).unwrap();
    let target = reqwest::Url::parse("https://site.example/x").unwrap();
    match solver::solve(&client, &api, 1, &target, None).await {
        Err(solver::SolveError::Permanent(message)) => {
            assert!(message.contains("unresolved challenge"), "{message}")
        }
        other => panic!("expected an unresolved challenge, got {}", other.is_ok()),
    }
    solved.join().unwrap();
}

#[tokio::test]
async fn a_blocked_request_a_solver_cannot_take_does_not_lock_the_host() {
    let block = response_with(
        403,
        "server: cloudflare\r\ncf-ray: 8a1b2c3d4e5f-AMS\r\ncontent-type: text/html\r\n",
        "<title>Attention Required! | Cloudflare</title>Sorry, you have been blocked.",
    );
    // A request that carries a token cannot use a solver: it fails, and the host is not remembered.
    let (base, direct) = server(vec![block.clone()]);
    let http = transport(None, std::sync::Arc::new(|| false));
    let url = reqwest::Url::parse(&base).unwrap();
    let error = http.get(&url, Some("token"), false).await.unwrap_err();
    assert!(error.contains("cannot use a solver"), "{error}");
    direct.join().unwrap();
    // So the next token request is tried again (it would otherwise be refused without a try)...
    let (again, retry) = server(vec![response(200, "{}")]);
    let url = reqwest::Url::parse(&again).unwrap();
    http.get(&url, Some("token"), false).await.unwrap();
    retry.join().unwrap();
    // ...and a public lookup to the same host is not locked out by it either.
    let (public, direct) = server(vec![response(200, "<p>fine</p>")]);
    let url = reqwest::Url::parse(&public).unwrap();
    assert_eq!(http.get(&url, None, true).await.unwrap(), "<p>fine</p>");
    direct.join().unwrap();
}

#[tokio::test]
async fn solver_attempts_are_bounded_at_two() {
    let (base, direct) = server(vec![response(
        403,
        "<title>Just a moment</title><script src='/cdn-cgi/challenge-platform/a'></script>",
    )]);
    let (solver, solved) = server(vec![response(503, "busy"), response(503, "busy")]);
    let error = transport(Some(solver), std::sync::Arc::new(|| false))
        .get(&reqwest::Url::parse(&base).unwrap(), None, true)
        .await
        .unwrap_err();
    assert!(error.contains("temporarily unavailable"), "{error}");
    direct.join().unwrap();
    assert_eq!(solved.join().unwrap().len(), 2);
}
#[tokio::test]
async fn solver_must_not_change_catalog_origin() {
    let (base, direct) = server(vec![response(
        403,
        "<title>Just a moment</title><script src='/cdn-cgi/challenge-platform/a'></script>",
    )]);
    let(solver,solved)=server(vec![response(200,&serde_json::json!({"status":"ok","solution":{"url":"https://other.example/secret","status":200,"response":"<div>catalog</div>"}}).to_string())]);
    let error = transport(Some(solver), std::sync::Arc::new(|| false))
        .get(&reqwest::Url::parse(&base).unwrap(), None, true)
        .await
        .unwrap_err();
    assert!(error.contains("outside the original site"), "{error}");
    assert!(!error.contains("secret"));
    direct.join().unwrap();
    solved.join().unwrap();
}
#[tokio::test]
async fn cancellation_interrupts_a_pending_solver_response() {
    use std::io::Read;
    use std::sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    };
    let (base, direct) = server(vec![response(
        403,
        "<title>Just a moment</title><script src='/cdn-cgi/challenge-platform/a'></script>",
    )]);
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let endpoint = format!("http://{}", listener.local_addr().unwrap());
    let stop = Arc::new(AtomicBool::new(false));
    let signal = stop.clone();
    let (release_solver, await_release) = std::sync::mpsc::channel();
    let solver = std::thread::spawn(move || {
        let (mut socket, _) = listener.accept().unwrap();
        let mut request = [0; 4096];
        assert!(socket.read(&mut request).unwrap() > 0);
        signal.store(true, Ordering::Relaxed);
        let _ = await_release.recv_timeout(std::time::Duration::from_secs(2));
    });
    let http = transport(
        Some(endpoint),
        Arc::new(move || stop.load(Ordering::Relaxed)),
    );
    let error = tokio::time::timeout(
        std::time::Duration::from_secs(1),
        http.get(&reqwest::Url::parse(&base).unwrap(), None, true),
    )
    .await
    .expect("cancellation must interrupt the pending solver")
    .unwrap_err();
    release_solver.send(()).unwrap();
    assert!(error.contains("Cancelled"), "{error}");
    direct.join().unwrap();
    solver.join().unwrap();
}
#[tokio::test]
async fn remembered_challenge_skips_direct_fetch_until_reset() {
    let challenge =
        "<title>Just a moment</title><script src='/cdn-cgi/challenge-platform/a'></script>";
    let (base, direct) = server(vec![
        response(403, challenge),
        response(200, "direct after reset"),
    ]);
    let answer=serde_json::json!({"status":"ok","solution":{"url":format!("{base}/"),"status":200,"response":"solved"}}).to_string();
    let (solver, solved) = server(vec![response(200, &answer), response(200, &answer)]);
    let http = transport(Some(solver), std::sync::Arc::new(|| false));
    let url = reqwest::Url::parse(&base).unwrap();
    assert_eq!(http.get(&url, None, true).await.unwrap(), "solved");
    assert_eq!(http.get(&url, None, true).await.unwrap(), "solved");
    http.state.reset();
    assert_eq!(
        http.get(&url, None, true).await.unwrap(),
        "direct after reset"
    );
    assert_eq!(direct.join().unwrap().len(), 2);
    assert_eq!(solved.join().unwrap().len(), 2);
}
#[tokio::test]
async fn oversized_catalog_is_rejected_without_reading_body() {
    let (base, task) = server(vec![
        "HTTP/1.1 200 OK\r\nconnection: close\r\ncontent-length: 8388609\r\n\r\n".into(),
    ]);
    let error = transport(None, std::sync::Arc::new(|| false))
        .get(&reqwest::Url::parse(&base).unwrap(), None, true)
        .await
        .unwrap_err();
    assert!(error.contains("size limit"), "{error}");
    task.join().unwrap();
}
#[tokio::test]
async fn failed_track_link_never_returns_a_different_release_track() {
    let (base, task) = server(vec![
        response(
            200,
            r#"<div class="track-page"><div class="ttl-info"><a href="/title/42/night-signals">Night Signals</a></div><div class="btns ptk-83"></div></div>"#,
        ),
        response(200, RELEASE),
    ]);
    let error = catalog(transport(None, std::sync::Arc::new(|| false)), &base)
        .release(Provider::Traxsource, "track:83")
        .await
        .unwrap_err();
    assert!(error.contains("absent"), "{error}");
    task.join().unwrap();
}

#[test]
fn catalog_addresses_normalize_to_typed_exact_ids() {
    for (provider, input, expected) in [
        (
            Provider::Beatport,
            "https://www.beatport.com/track/second-extended-mix/82",
            "track:82",
        ),
        (
            Provider::Beatport,
            "https://beatport.com/release/night-signals/42?utm_source=example",
            "release:42",
        ),
        (
            Provider::Traxsource,
            "https://www.traxsource.com/track/82/second",
            "track:82",
        ),
        (
            Provider::Traxsource,
            "https://traxsource.com/title/42/night-signals",
            "release:42",
        ),
        (Provider::Traxsource, "track:82", "track:82"),
    ] {
        assert_eq!(normalize_id(provider, input).unwrap(), expected);
    }
    for (provider, input) in [
        (
            Provider::Beatport,
            "https://www.beatport.com.evil.example/track/second/82",
        ),
        (
            Provider::Beatport,
            "https://user:secret@www.beatport.com/track/second/82",
        ),
        (
            Provider::Traxsource,
            "https://www.beatport.com/release/example/42",
        ),
        (
            Provider::Traxsource,
            "http://www.traxsource.com/title/42/example",
        ),
        (Provider::Traxsource, "42"),
    ] {
        assert!(normalize_id(provider, input).is_err(), "{input}");
    }
}

#[test]
fn traxsource_release_positions_must_be_complete() {
    let gap = RELEASE.replace("<div class=\"tnum\">2</div>", "<div class=\"tnum\">9</div>");
    assert!(parse_traxsource_release(&gap, "42").is_err());
}
#[tokio::test]
async fn beatport_release_without_row_positions_uses_the_listing_order() {
    // The release tracks endpoint returns rows in release order without a `number`.
    let row = |id: u32| serde_json::json!({"id":id,"name":format!("Track {id}"),"artists":[{"name":"Artist"}],"release":{"id":42,"name":"Release"}});
    let (base, task) = server(vec![
        response(200, r#"{"access_token":"fixture-token","expires_in":300}"#),
        response(200, r#"{"id":42,"name":"Release","track_count":3}"#),
        response(
            200,
            &serde_json::json!({"results":[row(30), row(10), row(20)],"next":null}).to_string(),
        ),
    ]);
    let release = catalog(transport(None, std::sync::Arc::new(|| false)), &base)
        .release(Provider::Beatport, "release:42")
        .await
        .unwrap();
    let positions: Vec<(String, u32)> = release
        .tracks
        .iter()
        .map(|t| (t.id.clone(), t.number))
        .collect();
    assert_eq!(
        positions,
        vec![("30".into(), 1), ("10".into(), 2), ("20".into(), 3)]
    );
    assert_eq!(release.tracks[2].fields["TRACKTOTAL"], vec!["3"]);
    task.join().unwrap();
}
#[tokio::test]
async fn beatport_release_with_some_row_positions_must_have_them_all() {
    let (base, task) = server(vec![
        response(200, r#"{"access_token":"fixture-token","expires_in":300}"#),
        response(200, r#"{"id":42,"name":"Release","track_count":2}"#),
        response(
            200,
            r#"{"results":[{"id":81,"name":"First","number":1,"release":{"id":42,"name":"Release"}},{"id":82,"name":"Second","release":{"id":42,"name":"Release"}}],"next":null}"#,
        ),
    ]);
    let error = catalog(transport(None, std::sync::Arc::new(|| false)), &base)
        .release(Provider::Beatport, "release:42")
        .await
        .unwrap_err();
    assert!(error.contains("positions"), "{error}");
    task.join().unwrap();
}
#[test]
fn traxsource_track_details_preserve_key_bpm_and_reject_other_track_identity() {
    let body = r#"<div class="track-page"><div class="ttl-info"><a href="/title/42/night-signals">Night Signals</a></div><div class="btns ptk-82"></div><table class="tr-det-tbl"><tr><td>Key:</td><td class="det">Amin</td></tr><tr><td>BPM:</td><td class="det">125</td></tr></table></div>"#;
    let (id, fields) = crate::traxsource::track_detail(body, "82").unwrap();
    assert_eq!(id, "42");
    assert_eq!(fields["INITIALKEY"], vec!["Am"]);
    assert_eq!(fields["BPM"], vec!["125"]);
    assert!(crate::traxsource::track_detail(body, "81").is_err());
}

#[tokio::test]
async fn a_cloudflare_error_page_without_challenge_markup_goes_through_the_solver() {
    let block = response_with(
        403,
        "server: cloudflare\r\ncf-ray: 8a1b2c3d4e5f-AMS\r\ncontent-type: text/html\r\n",
        "<title>Attention Required! | Cloudflare</title>Sorry, you have been blocked.",
    );
    let (base, direct) = server(vec![block]);
    let (solver, solved) = server(vec![response(
        200,
        &serde_json::json!({"status":"ok","solution":{"url":format!("{base}/"),"status":200,"response":"<div>catalog page</div>"}}).to_string(),
    )]);
    let http = transport(Some(solver), std::sync::Arc::new(|| false));
    let body = http
        .get(&reqwest::Url::parse(&base).unwrap(), None, true)
        .await
        .unwrap();
    assert_eq!(body, "<div>catalog page</div>");
    direct.join().unwrap();
    solved.join().unwrap();
}
#[tokio::test]
async fn a_blocked_request_that_carries_credentials_never_reaches_a_solver() {
    let block = response_with(403, "server: cloudflare\r\ncf-mitigated: challenge\r\n", "");
    let (base, direct) = server(vec![block, response(200, "{}")]);
    let http = transport(
        Some("http://127.0.0.1:9".into()),
        std::sync::Arc::new(|| false),
    );
    let url = reqwest::Url::parse(&base).unwrap();
    let error = http.get(&url, Some("token"), false).await.unwrap_err();
    assert!(error.contains("carry credentials"), "{error}");
    // Nothing was sent to the (unreachable) solver, and a request that cannot use one does not
    // lock the host: the next attempt is made directly (the fixture's second answer).
    assert_eq!(http.get(&url, Some("token"), false).await.unwrap(), "{}");
    direct.join().unwrap();
}
