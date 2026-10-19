#![cfg(test)]

//! Authored generic source over real loopback HTTP. No private/provider hosts.
use super::*;
use std::{
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    sync::{
        atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering},
        Arc, Mutex,
    },
    thread::JoinHandle,
    time::{Duration, Instant},
};

#[derive(Clone, Copy, Debug, PartialEq)]
enum Case {
    Success,
    Redirect,
    CredentialRedirect,
    Oversize,
    InvalidImage,
    Cancel,
    IndexOracle,
    ReleaseOracle,
    SelectedResult,
}
impl Case {
    fn read() -> Self {
        match std::env::var("TAGGROOVE_LOOPBACK_CASE").as_deref() {
            Ok("success") => Self::Success,
            Ok("redirect") => Self::Redirect,
            Ok("credential-redirect") => Self::CredentialRedirect,
            Ok("oversize") => Self::Oversize,
            Ok("invalid-image") => Self::InvalidImage,
            Ok("cancel") => Self::Cancel,
            Ok("index-oracle") => Self::IndexOracle,
            Ok("release-oracle") => Self::ReleaseOracle,
            Ok("selected-result") => Self::SelectedResult,
            _ => panic!("explicit loopback case required"),
        }
    }
    fn expected_paths(self) -> Vec<&'static str> {
        let mut paths = vec!["/search-start?q=Fixture+Artist", "/catalog/index"];
        if matches!(self, Self::IndexOracle | Self::SelectedResult) {
            return paths;
        }
        paths.push("/album/start/7");
        if matches!(self, Self::Redirect | Self::CredentialRedirect) {
            return paths;
        }
        paths.push("/releases/7/page");
        if matches!(self, Self::Oversize | Self::Cancel | Self::ReleaseOracle) {
            return paths;
        }
        paths.extend([
            "/releases/front-start.png",
            "/images/final.png",
            "/releases/front-start.png",
            "/images/final.png",
        ]);
        paths
    }
}

#[derive(Clone, serde::Serialize)]
struct Observed {
    target: String,
    method: String,
    headers: BTreeMap<String, String>,
    status: u16,
    response_headers: BTreeMap<String, String>,
    payload_sha256: String,
    payload_bytes: usize,
    chunked: bool,
    written_chunk_lengths: Vec<usize>,
    complete: bool,
    write_error: Option<String>,
}

pub(super) struct WireState {
    case: Case,
    requested_release_url: String,
    requests: Mutex<Vec<Observed>>,
    errors: Mutex<Vec<String>>,
    stop: AtomicBool,
    cancelled: AtomicBool,
    connections: AtomicUsize,
    pub(super) release_ticket: AtomicU64,
}
impl WireState {
    pub(super) fn request_count(&self) -> usize {
        self.requests.lock().unwrap().len()
    }
    fn check_paths(&self) {
        assert_eq!(
            self.requests
                .lock()
                .unwrap()
                .iter()
                .map(|r| r.target.clone())
                .collect::<Vec<_>>(),
            self.case.expected_paths(),
            "independent wire request order/count"
        );
        assert!(self.errors.lock().unwrap().is_empty(), "wire server errors");
    }
    pub(super) fn check_refusal(&self, failure: Option<&str>, cause: Option<&str>) {
        let scheme_refusal = format!(
            "Loopback Replay returned HTTP 302 for {}",
            self.requested_release_url
        );
        let (guard, message) = match self.case {
            Case::Success => {
                assert!(
                    failure.is_none(),
                    "unexpected wire failure: {failure:?}: {cause:?}"
                );
                self.check_paths();
                return;
            }
            Case::Redirect => ("release refused", Some(scheme_refusal.as_str())),
            Case::CredentialRedirect => (
                "release refused",
                Some("Loopback Replay redirected to an address TagGroove cannot open."),
            ),
            Case::Oversize => (
                "release refused",
                Some("Loopback Replay returned more data than expected."),
            ),
            Case::InvalidImage => (
                "public apply refused",
                Some("The downloaded cover is not a supported image."),
            ),
            Case::Cancel => ("release refused", Some(http::CANCELLED)),
            Case::IndexOracle => ("independent index oracle", None),
            Case::ReleaseOracle => ("independent album oracle", None),
            Case::SelectedResult => ("release oracle", None),
        };
        assert_eq!(failure, Some(guard), "exact wire refusal stage");
        assert_eq!(cause, message, "exact production wire failure");
        self.check_paths();
        if self.case == Case::Cancel {
            assert!(
                self.cancelled.load(Ordering::SeqCst),
                "body cancellation barrier not reached"
            );
        }
    }
}

struct Server {
    base: String,
    state: Arc<WireState>,
    thread: Option<JoinHandle<()>>,
}
impl Server {
    fn start(case: Case) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let address = listener.local_addr().unwrap();
        let base = format!("http://{address}");
        let state = Arc::new(WireState {
            case,
            requested_release_url: format!("{base}/album/start/7"),
            requests: Mutex::new(Vec::new()),
            errors: Mutex::new(Vec::new()),
            stop: AtomicBool::new(false),
            cancelled: AtomicBool::new(false),
            connections: AtomicUsize::new(0),
            release_ticket: AtomicU64::new(0),
        });
        let shared = state.clone();
        let host = address.to_string();
        let thread = std::thread::spawn(move || {
            let deadline = Instant::now() + Duration::from_secs(20);
            while !shared.stop.load(Ordering::SeqCst) {
                if Instant::now() >= deadline {
                    shared.errors.lock().unwrap().push("server deadline".into());
                    break;
                }
                match listener.accept() {
                    Ok((mut socket, peer)) => {
                        if shared.connections.fetch_add(1, Ordering::SeqCst) >= 12 {
                            shared
                                .errors
                                .lock()
                                .unwrap()
                                .push("connection bound".into());
                            break;
                        }
                        if !peer.ip().is_loopback() {
                            shared
                                .errors
                                .lock()
                                .unwrap()
                                .push("non-loopback peer".into());
                            break;
                        }
                        if let Err(error) = serve(&mut socket, &host, &shared) {
                            shared.errors.lock().unwrap().push(error);
                        }
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        std::thread::sleep(Duration::from_millis(5));
                    }
                    Err(error) => {
                        shared.errors.lock().unwrap().push(error.to_string());
                        break;
                    }
                }
            }
        });
        Self {
            base,
            state,
            thread: Some(thread),
        }
    }
    fn finish(&mut self) {
        self.state.stop.store(true, Ordering::SeqCst);
        if let Some(thread) = self.thread.take() {
            let joined = thread.join().is_ok();
            println!(
                "WIRE_SERVER_RECEIPT {}",
                json!({"case":format!("{:?}",self.state.case),"requested_release_url":self.state.requested_release_url,"joined":joined,"connections":self.state.connections.load(Ordering::SeqCst),"requests":self.state.requests.lock().unwrap().clone(),"errors":self.state.errors.lock().unwrap().clone(),"cancelled_during_body":self.state.cancelled.load(Ordering::SeqCst)})
            );
            if !std::thread::panicking() {
                assert!(joined);
                self.state.check_paths();
                assert!(self
                    .state
                    .requests
                    .lock()
                    .unwrap()
                    .iter()
                    .all(|r| r.complete
                        || self.state.case == Case::Oversize
                        || (self.state.case == Case::Cancel
                            && self.state.cancelled.load(Ordering::SeqCst)
                            && !r.written_chunk_lengths.is_empty())));
            }
        }
    }
}
impl Drop for Server {
    fn drop(&mut self) {
        self.finish();
    }
}

fn index_body() -> Vec<u8> {
    "../album/wrong/0|Decoy\n../album/start/7|Wire Café\n"
        .as_bytes()
        .to_vec()
}
fn album_body() -> Vec<u8> {
    // Actual Latin-1 bytes, not a UTF-8 response merely labelled Latin-1.
    r#"{"album":"Wire Café","artist":"Wire Artist","cover":"../front-start.png","tracks":[{"title":"First Saved"},{"title":"Second Saved"}]}"#
        .chars().map(|c| u8::try_from(u32::from(c)).unwrap()).collect()
}

fn serve(socket: &mut TcpStream, host: &str, state: &WireState) -> Result<(), String> {
    let started = Instant::now();
    let deadline = Instant::now() + Duration::from_secs(5);
    // Accepted streams must not inherit a platform-dependent nonblocking mode
    // from the listener; the explicit I/O timeouts below provide their bound.
    socket.set_nonblocking(false).map_err(|e| e.to_string())?;
    socket
        .set_read_timeout(Some(Duration::from_secs(2)))
        .map_err(|e| e.to_string())?;
    socket
        .set_write_timeout(Some(Duration::from_secs(2)))
        .map_err(|e| e.to_string())?;
    let mut head = Vec::new();
    let mut byte = [0u8; 1];
    while !head.ends_with(b"\r\n\r\n") {
        if Instant::now() >= deadline {
            return Err("request deadline".into());
        }
        if head.len() >= 16 * 1024 {
            return Err("request head bound".into());
        }
        socket.read_exact(&mut byte).map_err(|error| {
            format!(
                "request read after {}ms: kind={:?}, errno={:?}: {error}",
                started.elapsed().as_millis(),
                error.kind(),
                error.raw_os_error()
            )
        })?;
        head.push(byte[0]);
    }
    let head = String::from_utf8(head).map_err(|e| e.to_string())?;
    let mut lines = head.split("\r\n");
    let first = lines.next().unwrap().split(' ').collect::<Vec<_>>();
    if first.len() != 3 || first[0] != "GET" || first[2] != "HTTP/1.1" {
        return Err("request line".into());
    }
    let target = first[1].to_owned();
    let mut headers = BTreeMap::new();
    for line in lines.filter(|line| !line.is_empty()) {
        let (key, value) = line.split_once(':').ok_or("request header")?;
        if headers
            .insert(key.to_ascii_lowercase(), value.trim().to_owned())
            .is_some()
        {
            return Err("duplicate header".into());
        }
    }
    if headers.get("host").map(String::as_str) != Some(host)
        || headers.contains_key("authorization")
        || headers.contains_key("proxy-authorization")
    {
        return Err("host/credential header oracle".into());
    }
    let image = target == "/releases/front-start.png" || target == "/images/final.png";
    let expected_agent = if image {
        http::USER_AGENT.to_owned()
    } else {
        format!(
            "Mozilla/5.0 (compatible; OpenTag/{})",
            env!("CARGO_PKG_VERSION")
        )
    };
    if headers.get("user-agent") != Some(&expected_agent) {
        return Err("user-agent oracle".into());
    }
    if image && headers.get("accept").map(String::as_str) != Some("image/*") {
        return Err("image Accept oracle".into());
    }
    let index = state.request_count();
    let expected = state.case.expected_paths();
    if index >= expected.len() || target != expected[index] {
        return Err(format!("unexpected request: {target}"));
    }
    let mut response_headers = BTreeMap::new();
    let mut chunked = false;
    let (status, body) = match target.as_str() {
        "/search-start?q=Fixture+Artist" => {
            response_headers.insert("location".into(), "/catalog/index".into());
            (302, vec![])
        }
        "/catalog/index" => {
            response_headers.insert("content-type".into(), "text/plain; charset=utf-8".into());
            (200, index_body())
        }
        "/album/start/7" => {
            response_headers.insert(
                "location".into(),
                if state.case == Case::Redirect {
                    "file:///must-not-open".to_owned()
                } else if state.case == Case::CredentialRedirect {
                    format!("http://blocked@{host}/must-not-open")
                } else {
                    "/releases/7/page".to_owned()
                },
            );
            (302, vec![])
        }
        "/releases/7/page" => {
            chunked = true;
            response_headers.insert(
                "content-type".into(),
                "application/json; charset=iso-8859-1".into(),
            );
            (
                200,
                if state.case == Case::Oversize {
                    vec![b'x'; 8 * 1024 * 1024 + 1]
                } else {
                    album_body()
                },
            )
        }
        "/releases/front-start.png" => {
            response_headers.insert("location".into(), "/images/final.png".into());
            (302, vec![])
        }
        "/images/final.png" => {
            response_headers.insert("content-type".into(), "image/png".into());
            (
                200,
                if state.case == Case::InvalidImage && index == 7 {
                    b"<html>not a PNG</html>".to_vec()
                } else {
                    png(2)
                },
            )
        }
        _ => return Err("unhandled request".into()),
    };
    response_headers.insert("connection".into(), "close".into());
    if chunked {
        response_headers.insert("transfer-encoding".into(), "chunked".into());
    } else {
        response_headers.insert("content-length".into(), body.len().to_string());
    }
    state.requests.lock().unwrap().push(Observed {
        target,
        method: first[0].into(),
        headers,
        status,
        response_headers: response_headers.clone(),
        payload_sha256: format!("{:x}", Sha256::digest(&body)),
        payload_bytes: body.len(),
        chunked,
        written_chunk_lengths: Vec::new(),
        complete: false,
        write_error: None,
    });
    let mut chunk_lengths = Vec::new();
    let write = (|| -> std::io::Result<()> {
        write!(
            socket,
            "HTTP/1.1 {status} {}\r\n",
            if status == 302 { "Found" } else { "OK" }
        )?;
        for (key, value) in &response_headers {
            write!(socket, "{key}: {value}\r\n")?;
        }
        socket.write_all(b"\r\n")?;
        if chunked {
            let size = if state.case == Case::Oversize {
                16 * 1024
            } else {
                7
            };
            for (part, chunk) in body.chunks(size).enumerate() {
                if Instant::now() >= deadline {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::TimedOut,
                        "response deadline",
                    ));
                }
                write!(socket, "{:x}\r\n", chunk.len())?;
                socket.write_all(chunk)?;
                socket.write_all(b"\r\n")?;
                chunk_lengths.push(chunk.len());
                if state.case == Case::Cancel && part == 0 {
                    // Body barrier: actual request is on the wire; the final
                    // chunk is withheld until the production cancel completes.
                    let ticket = state.release_ticket.load(Ordering::SeqCst);
                    assert!(ticket > 0 && ticket < u64::MAX);
                    crate::sources::tag_source_cancel(ticket + 1);
                    state.cancelled.store(true, Ordering::SeqCst);
                }
            }
            socket.write_all(b"0\r\n\r\n")?;
        } else {
            socket.write_all(&body)?;
        }
        socket.flush()
    })();
    let mut requests = state.requests.lock().unwrap();
    let observed = &mut requests[index];
    observed.written_chunk_lengths = chunk_lengths;
    match write {
        Ok(()) => observed.complete = true,
        Err(error) => {
            observed.write_error = Some(error.to_string());
            let cancellation_disconnect = state.case == Case::Cancel
                && state.cancelled.load(Ordering::SeqCst)
                && !observed.written_chunk_lengths.is_empty();
            if (state.case != Case::Oversize && !cancellation_disconnect)
                || !matches!(
                    error.kind(),
                    std::io::ErrorKind::BrokenPipe
                        | std::io::ErrorKind::ConnectionReset
                        | std::io::ErrorKind::ConnectionAborted
                )
            {
                return Err(format!("response write: {error}"));
            }
        }
    }
    Ok(())
}

/// Ordinary CI runs the cases in fresh processes: tickets and HTTP fixture
/// globals cannot leak from other tests, and no private input is required.
#[test]
fn loopback_http_workflow_runs_positive_and_exact_refusals_in_fresh_workers() {
    let temporary = tempfile::tempdir().unwrap();
    let root = std::env::var_os("TAGGROOVE_LOOPBACK_ARTIFACT_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(|| temporary.path().to_path_buf());
    std::fs::create_dir_all(&root).unwrap();
    let test = "sources::web::workflow_replay::loopback::joined_loopback_http_review_apply_and_journaled_audio_workflow";
    let cases = [
        ("success", 8, None),
        // The scheme refusal contains the ephemeral requested fixture URL;
        // construct its exact expected message below from fixture provenance.
        ("redirect", 3, Some(("release refused", None))),
        (
            "credential-redirect",
            3,
            Some((
                "release refused",
                Some("Loopback Replay redirected to an address TagGroove cannot open."),
            )),
        ),
        (
            "oversize",
            4,
            Some((
                "release refused",
                Some("Loopback Replay returned more data than expected."),
            )),
        ),
        (
            "invalid-image",
            8,
            Some((
                "public apply refused",
                Some("The downloaded cover is not a supported image."),
            )),
        ),
        (
            "cancel",
            4,
            Some(("release refused", Some(http::CANCELLED))),
        ),
        ("index-oracle", 2, Some(("independent index oracle", None))),
        (
            "release-oracle",
            4,
            Some(("independent album oracle", None)),
        ),
        ("selected-result", 2, Some(("release oracle", None))),
    ];
    for (case, requests, expected_failure) in cases {
        let log = root.join(format!("{case}.log"));
        let file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&log)
            .unwrap();
        let mut command = Command::new(std::env::current_exe().unwrap());
        command
            .args([test, "--exact", "--ignored", "--nocapture"])
            .env_clear()
            .stdout(Stdio::from(file.try_clone().unwrap()))
            .stderr(Stdio::from(file));
        for key in [
            "PATH",
            "HOME",
            "USERPROFILE",
            "TMPDIR",
            "TMP",
            "TEMP",
            "SystemRoot",
            "SYSTEMROOT",
            "WINDIR",
            "PATHEXT",
            "COMSPEC",
            "LOCALAPPDATA",
            "APPDATA",
            "LANG",
            "LC_ALL",
        ] {
            if let Some(value) = std::env::var_os(key) {
                command.env(key, value);
            }
        }
        command
            .env("OPENTAG_E2E_SCRIPT", "loopback-source-qualification")
            .env("OPENTAG_E2E_OFFLINE", "1")
            .env("NO_PROXY", "127.0.0.1")
            .env("no_proxy", "127.0.0.1")
            .env("TAGGROOVE_LOOPBACK_CASE", case);
        if case == "selected-result" {
            command.env("TAGGROOVE_WORKFLOW_ORACLE_MUTATION", "selected-result");
        }
        let started = Instant::now();
        let mut child = BridgeChild::new(command.spawn().unwrap(), Duration::from_secs(45));
        let status = child.finish(false);
        assert!(!child.expired(), "wire worker deadline: {case}");
        assert!(child.reaped.load(Ordering::SeqCst));
        assert!(
            std::fs::metadata(&log).unwrap().len() <= 512 * 1024,
            "worker output bound"
        );
        let output = std::fs::read_to_string(&log).unwrap();
        let record = |prefix: &str| -> Value {
            let rows = output
                .lines()
                .filter_map(|line| line.strip_prefix(prefix))
                .collect::<Vec<_>>();
            assert_eq!(
                rows.len(),
                1,
                "{case}: missing/duplicate {prefix}: {output}"
            );
            serde_json::from_str(rows[0]).unwrap()
        };
        let server = record("WIRE_SERVER_RECEIPT ");
        let preflight = record("WIRE_PARSER_PREFLIGHT ");
        assert_eq!(preflight["rows"], 2);
        assert_eq!(preflight["tracks"], 2);
        assert_eq!(preflight["http_requests"], 0);
        assert_eq!(server["joined"], true, "{output}");
        assert_eq!(server["errors"], json!([]), "{output}");
        assert_eq!(server["connections"], requests, "{output}");
        assert_eq!(server["requests"].as_array().unwrap().len(), requests);
        if let Some((guard, cause)) = expected_failure {
            assert_eq!(status.code(), Some(101), "{case}: {output}");
            let refusal = record("WIRE_PREWRITE_REFUSAL ");
            assert_eq!(refusal["guard"], guard);
            let expected_cause = if case == "redirect" {
                let requested = server["requested_release_url"].as_str().unwrap();
                let address = reqwest::Url::parse(requested).unwrap();
                assert_eq!(address.host_str(), Some("127.0.0.1"));
                assert_eq!(address.path(), "/album/start/7");
                Some(format!("Loopback Replay returned HTTP 302 for {requested}"))
            } else {
                cause.map(str::to_owned)
            };
            assert_eq!(refusal["cause"], json!(expected_cause));
            assert_eq!(refusal["writer_called"], false);
            assert_eq!(refusal["wav_sidecar_history_unchanged"], true);
            assert_eq!(refusal["reaped"], true);
            assert_eq!(refusal["requests"], requests);
            assert!(!output.contains("WORKFLOW_RECEIPT "));
        } else {
            assert!(status.success(), "{case}: {output}");
            let saved = record("WORKFLOW_RECEIPT ");
            assert_eq!(saved["private_parser"], false);
            assert_eq!(saved["public_acquisition"], true);
            assert_eq!(saved["matched"], 2);
            assert_eq!(saved["untouched"], 1);
            assert_eq!(saved["selected_result_index"], 1);
            assert_eq!(saved["network_requests"], 0);
            assert_eq!(saved["pcm_mutation_control_detected"], true);
            for phase in ["saved", "undo", "redo"] {
                assert_eq!(saved["pcm_sha256"]["before"], saved["pcm_sha256"][phase]);
            }
            let release = &server["requests"][3];
            assert_eq!(release["chunked"], true);
            assert!(release["written_chunk_lengths"].as_array().unwrap().len() > 1);
            assert_eq!(
                server["requests"][5]["payload_sha256"],
                server["requests"][7]["payload_sha256"]
            );
        }
        if case == "cancel" {
            assert_eq!(server["cancelled_during_body"], true);
        }
        println!(
            "WIRE_PARENT_RESULT {}",
            json!({"case":case,"exit":status.code(),"requests":requests,"expected_refusal":expected_failure.map(|(guard,_)|guard),"elapsed_ms":started.elapsed().as_millis(),"reaped":true,"log_sha256":format!("{:x}",Sha256::digest(output.as_bytes()))})
        );
    }
}

#[test]
#[ignore = "real loopback HTTP; separate process with explicit TAGGROOVE_LOOPBACK_CASE and reviewed loopback-only sandbox"]
fn joined_loopback_http_review_apply_and_journaled_audio_workflow() {
    let _guard = crate::history::TEST_LOCK
        .lock()
        .unwrap_or_else(|p| p.into_inner());
    assert!(
        http::offline_test_run(),
        "loopback-only debug guard must be active"
    );
    assert!(!http::web_allowed(
        &Url::parse("https://external.invalid/").unwrap()
    ));
    let case = Case::read();
    let mut server = Server::start(case);
    let dir = tempfile::tempdir().unwrap();
    let root = dunce::canonicalize(dir.path()).unwrap();
    let incoming = root.join("incoming");
    let installed = root.join("sources");
    std::fs::create_dir(&incoming).unwrap();
    std::fs::write(incoming.join("Loopback.src"),format!("[Name]=Loopback Replay\n[IndexUrl]={}/search-start?q=%s\n[AlbumUrl]=\n[IndexFormat]=%_url%|%Album%\n[SearchBy]=Artist||%artist%||%s\n[WordSeparator]=+\n[Settings]=Replay.settings\n[Include]=Replay.inc\n[ParserScriptIndex]=...\nSayUntilML \"\"\n",server.base)).unwrap();
    std::fs::write(incoming.join("Replay.inc"),"[ParserScriptAlbum]=...\njson \"on\"\nOutputTo \"ALBUM\"\njson_select \"album\"\nSayRest\nOutputTo \"ARTIST\"\njson_select \"artist\"\nSayRest\nOutputTo \"COVERURL\"\njson_select \"cover\"\nSayRest\nIfVar \"addComment\" \"true\"\nOutputTo \"COMMENT\"\nSay \"Saved setting reached the writer\"\nEndif\njson_foreach \"tracks\"\nOutputTo \"TITLE\"\njson_select \"title\"\nSayRest\nSay \"|\"\nOutputTo \"TRACK\"\njson_foreach_counter\nSayRest\nSay \"|\"\njson_foreach_end\n").unwrap();
    std::fs::write(incoming.join("Replay.settings"),json!({"key":"independentReplay","settings":[{"type":"bool","key":"addComment","default":false}]}).to_string()).unwrap();
    install(&installed, &[incoming.to_string_lossy().into_owned()]).unwrap();
    save_settings(
        &installed,
        "Replay.settings",
        Map::from_iter([("addComment".into(), Value::Bool(true))]),
    )
    .unwrap();
    let requested = format!("{}/album/start/7", server.base);
    let final_url = format!("{}/releases/7/page", server.base);
    let cover_url = format!("{}/releases/front-start.png", server.base);
    assert!(!http::web_registered(&Url::parse(&requested).unwrap()));
    assert!(!http::web_registered(&Url::parse(&cover_url).unwrap()));
    assert!(tauri::async_runtime::block_on(release_with_preparation(
        "Loopback.src".into(),
        requested.clone(),
        1,
        async { panic!("unregistered preparation") }
    ))
    .is_err());
    assert!(
        tauri::async_runtime::block_on(crate::sources::tag_source_image(cover_url.clone(), 1))
            .is_err()
    );
    assert_eq!(
        server.state.request_count(),
        0,
        "unregistered ingress reached a socket"
    );
    let fields = Fields::from([
        ("ALBUM".into(), vec!["Wire Café".into()]),
        ("ARTIST".into(), vec!["Wire Artist".into()]),
        (
            "COMMENT".into(),
            vec!["Saved setting reached the writer".into()],
        ),
    ]);
    let mut expected_fields = fields.clone();
    if case == Case::ReleaseOracle {
        expected_fields.insert("ALBUM".into(), vec!["Wrong independent album".into()]);
    }
    let tracks = ["First Saved", "Second Saved"]
        .into_iter()
        .enumerate()
        .map(|(i, title)| SourceTrack {
            position: (i + 1).to_string(),
            disc: 1,
            number: (i + 1) as u32,
            title: title.into(),
            artist: "Wire Artist".into(),
            length: None,
            fields: Fields::from([
                ("TITLE".into(), vec![title.into()]),
                ("TRACK".into(), vec![(i + 1).to_string()]),
            ]),
        })
        .collect();
    let release = SourceRelease {
        provider: Provider::Web,
        id: requested.clone(),
        url: final_url,
        title: "Wire Café".into(),
        artist: "Wire Artist".into(),
        fields,
        tracks,
        covers: vec![SourceCover {
            url: cover_url.clone(),
            thumb: None,
            kind: "Cover".into(),
            width: None,
            height: None,
        }],
        more_covers: false,
    };
    let mut expected_search = json!({"direct":false,"columns":["Album"],"results":[{"url":format!("{}/album/wrong/0",server.base),"cover":null,"preview":null,"values":["Decoy"]},{"url":requested,"cover":null,"preview":null,"values":["Wire Café"]}]});
    let canonical_search = expected_search.clone();
    if case == Case::IndexOracle {
        expected_search["results"][1]["values"][0] = json!("Wrong independent index");
    }
    let fixture = PrivateReplay {
        version: 1,
        case: "authored-loopback-multi".into(),
        bundle: incoming,
        source: "Loopback.src".into(),
        settings_id: "Replay.settings".into(),
        settings_values: Map::new(),
        page: root.join("unused-page"),
        page_url: requested.clone(),
        input_sha256: BTreeMap::new(),
        expected_fields,
        expected_title: "Wire Café".into(),
        expected_artist: "Wire Artist".into(),
        expected_cover_url: cover_url.clone(),
        expected_updates: Fields::new(),
    };
    let public = PublicReplay {
        installed,
        fixture,
        manifest: PublicManifest {
            version: 1,
            private_manifest: root.join("unused-manifest"),
            values: vec!["Fixture Artist".into()],
            index: Some(root.join("unused-index")),
            search_url: format!("{}/search-start?q=Fixture+Artist", server.base),
            page_url: requested,
            expected_search,
            input_sha256: BTreeMap::new(),
        },
        values: vec!["Fixture Artist".into()],
        wire: Some(server.state.clone()),
    };
    // Validate the exact installed authored fixture before any HTTP request.
    // Deliberately corrupted case oracles below remain separate from these
    // literal positive expectations, so the preflight cannot bless a mutation.
    let file = load(&public.installed, "Loopback.src").unwrap();
    let variables = vars(&public.installed, &file).unwrap();
    let index_page = http::Page {
        text: String::from_utf8(index_body()).unwrap(),
        url: Url::parse(&format!("{}/catalog/index", server.base)).unwrap(),
    };
    let outputs = execute(
        &public.installed,
        "Loopback.src",
        file.index_script.as_ref().unwrap(),
        "index",
        &index_page,
        variables.clone(),
        false,
    )
    .unwrap();
    let parsed_index =
        index_results(&file, &outputs[engine::DEFAULT_OUTPUT], &index_page.url).unwrap();
    assert_eq!(
        serde_json::to_value(&parsed_index).unwrap(),
        canonical_search,
        "authored index fixture preflight"
    );
    let album_page = http::Page {
        text: http::decode_text(&album_body(), Some("application/json; charset=iso-8859-1")),
        url: Url::parse(&release.url).unwrap(),
    };
    let outputs = execute(
        &public.installed,
        "Loopback.src",
        file.album_script.as_ref().unwrap(),
        "album",
        &album_page,
        variables,
        false,
    )
    .unwrap();
    let parsed_release = release_from(outputs, &album_page.url, &public.manifest.page_url);
    assert_eq!(
        serde_json::to_value(&parsed_release).unwrap(),
        serde_json::to_value(&release).unwrap(),
        "authored album fixture preflight"
    );
    assert_eq!(server.state.request_count(), 0);
    println!(
        "WIRE_PARSER_PREFLIGHT {}",
        json!({"rows":parsed_index.results.len(),"tracks":parsed_release.tracks.len(),"http_requests":0,"source_sha256":format!("{:x}",Sha256::digest(std::fs::read(public.installed.join("Loopback.src")).unwrap()))})
    );
    if case == Case::SelectedResult {
        assert_eq!(
            std::env::var("TAGGROOVE_WORKFLOW_ORACLE_MUTATION").as_deref(),
            Ok("selected-result")
        );
    }
    let updates = ["First Saved", "Second Saved"]
        .into_iter()
        .map(|title| {
            Fields::from([
                ("TITLE".into(), vec![title.into()]),
                ("ALBUM".into(), vec!["Wire Café".into()]),
                (
                    "COMMENT".into(),
                    vec!["Saved setting reached the writer".into()],
                ),
            ])
        })
        .collect();
    review_and_save(
        &root,
        ReplayReview {
            label: "authored-loopback-multi".into(),
            scenario: "multi".into(),
            source_id: "Loopback.src".into(),
            checked_fields: vec!["TITLE".into(), "ALBUM".into(), "COMMENT".into()],
            selected_indices: vec![1, 0],
            search: WebSearchPage::default(),
            release,
            expected_updates: updates,
            cover_url,
            private_input: false,
        },
        Some(&public),
    );
    server.finish();
}
