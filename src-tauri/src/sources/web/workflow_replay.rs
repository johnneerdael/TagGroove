#![cfg(test)]

//! Independent saved-page → production review → real writer/history control.
//! HTTP acquisition is deliberately outside this filesystem replay's evidence.
use super::*;
use base64::Engine;
use serde_json::json;
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    ops::Range,
    path::PathBuf,
    process::{Command, Stdio},
};

#[path = "workflow_loopback.rs"]
mod loopback;

fn png(width: u32) -> Vec<u8> {
    let mut bytes = std::io::Cursor::new(Vec::new());
    image::DynamicImage::new_rgb8(width, 1)
        .write_to(&mut bytes, image::ImageFormat::Png)
        .unwrap();
    bytes.into_inner()
}

fn wav(path: &Path, number: usize, old_cover: &Path) {
    let mut bytes = b"RIFF".to_vec();
    bytes.extend(68u32.to_le_bytes());
    bytes.extend(b"WAVEfmt ");
    bytes.extend(16u32.to_le_bytes());
    bytes.extend(1u16.to_le_bytes());
    bytes.extend(1u16.to_le_bytes());
    bytes.extend(8000u32.to_le_bytes());
    bytes.extend(16000u32.to_le_bytes());
    bytes.extend(2u16.to_le_bytes());
    bytes.extend(16u16.to_le_bytes());
    bytes.extend(b"data");
    bytes.extend(32u32.to_le_bytes());
    for index in 0..16 {
        let sample = i16::try_from(number).unwrap() * 1000 + index * 37 - 8000;
        bytes.extend(sample.to_le_bytes());
    }
    std::fs::write(path, bytes).unwrap();
    tag_core::write_fields(
        path,
        &Fields::from([
            ("TITLE".into(), vec![format!("Before {number}")]),
            ("TRACK".into(), vec![number.to_string()]),
            ("ARTIST".into(), vec!["Keep original artist".into()]),
            ("COMMENT".into(), vec!["Before comment".into()]),
        ]),
    )
    .unwrap();
    tag_core::add_cover(path, old_cover).unwrap();
}

/// Independent fixture observer: walk bounded RIFF chunks instead of using
/// the production audio reader or searching for bytes that happen to say data.
fn wav_pcm_range(bytes: &[u8]) -> Range<usize> {
    assert!(
        bytes.len() <= 4 * 1024 * 1024,
        "fixture WAV exceeds observer bound"
    );
    assert!(bytes.len() >= 12, "truncated RIFF header");
    assert_eq!(&bytes[..4], b"RIFF");
    assert_eq!(&bytes[8..12], b"WAVE");
    let size = u32::from_le_bytes(bytes[4..8].try_into().unwrap()) as usize;
    let end = size.checked_add(8).unwrap();
    assert!((12..=bytes.len()).contains(&end), "invalid RIFF extent");
    let mut at = 12;
    let mut pcm = None;
    let mut format = false;
    while at < end {
        assert!(end - at >= 8, "truncated RIFF chunk header");
        let length = u32::from_le_bytes(bytes[at + 4..at + 8].try_into().unwrap()) as usize;
        let start = at + 8;
        let stop = start.checked_add(length).unwrap();
        assert!(stop <= end, "RIFF chunk exceeds its parent");
        match &bytes[at..at + 4] {
            b"fmt " => {
                assert!(!format && length >= 16, "invalid fixture PCM format");
                assert_eq!(
                    u16::from_le_bytes(bytes[start..start + 2].try_into().unwrap()),
                    1
                );
                assert_eq!(
                    u16::from_le_bytes(bytes[start + 2..start + 4].try_into().unwrap()),
                    1
                );
                assert_eq!(
                    u16::from_le_bytes(bytes[start + 12..start + 14].try_into().unwrap()),
                    2
                );
                assert_eq!(
                    u16::from_le_bytes(bytes[start + 14..start + 16].try_into().unwrap()),
                    16
                );
                format = true;
            }
            b"data" => {
                assert!(pcm.is_none(), "ambiguous fixture PCM chunks");
                assert_eq!(length, 32, "fixture PCM extent changed");
                pcm = Some(start..stop);
            }
            _ => {}
        }
        at = stop.checked_add(length % 2).unwrap();
        assert!(at <= end, "missing RIFF chunk padding");
    }
    assert!(format, "missing fixture PCM format");
    pcm.expect("missing fixture PCM chunk")
}

fn wav_pcm(bytes: &[u8]) -> &[u8] {
    &bytes[wav_pcm_range(bytes)]
}

fn review_helper(input: &Path, output: &Path) -> std::process::Output {
    Command::new("node")
        .arg(Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/helpers/source-workflow-review.mjs"))
        .arg(input).arg(output).output()
        .expect("source workflow replay requires Node and npm ci (reviewed-build installs both before cargo test)")
}

fn replay(scenario: &str) {
    let dir = tempfile::tempdir().unwrap();
    let root = dunce::canonicalize(dir.path()).unwrap();
    let incoming = root.join("incoming");
    let installed = root.join("sources");
    std::fs::create_dir(&incoming).unwrap();
    let host = format!("https://workflow-{}.invalid", uuid::Uuid::new_v4());
    let source = if scenario == "multi" {
        format!("[Name]=Independent Replay\n[IndexUrl]={host}/search?q=%s\n[AlbumUrl]={host}/release/%s\n[IndexFormat]=%_url%|%Album%\n[SearchBy]=Artist||%artist%||%s\n[Settings]=Replay.settings\n[Include]=Replay.inc\n[ParserScriptIndex]=...\nSayRest\n")
    } else {
        format!("[Name]=Independent Replay\n[AlbumUrl]={host}/release/%s\n[SearchBy]=Release ID||7||%s\n[Settings]=Replay.settings\n[Include]=Replay.inc\n")
    };
    std::fs::write(incoming.join("Replay.src"), source).unwrap();
    let mut album_script = String::from("[ParserScriptAlbum]=...\njson \"on\"\nOutputTo \"ALBUM\"\njson_select \"album\"\nSayRest\nOutputTo \"ARTIST\"\njson_select \"artist\"\nSayRest\nOutputTo \"COVERURL\"\njson_select \"cover\"\nSayRest\nIfVar \"addComment\" \"true\"\nOutputTo \"COMMENT\"\nSay \"Saved setting reached the writer\"\nEndif\n");
    if scenario == "multi" {
        album_script.push_str("json_foreach \"tracks\"\nOutputTo \"TITLE\"\njson_select \"title\"\nSayRest\nSay \"|\"\nOutputTo \"TRACK\"\njson_foreach_counter\nSayRest\nSay \"|\"\njson_foreach_end\n");
    } else {
        album_script.push_str("OutputTo \"TITLE\"\njson_select \"title\"\nSayRest\n");
    }
    std::fs::write(incoming.join("Replay.inc"), album_script).unwrap();
    std::fs::write(incoming.join("Replay.settings"), json!({"key":"independentReplay","settings":[{"type":"bool","key":"addComment","default":false}]}).to_string()).unwrap();
    install(&installed, &[incoming.to_string_lossy().into_owned()]).unwrap();
    save_settings(
        &installed,
        "Replay.settings",
        Map::from_iter([("addComment".into(), Value::Bool(true))]),
    )
    .unwrap();
    let file = load(&installed, "Replay.src").unwrap();
    let variables = vars(&installed, &file).unwrap();
    let album_url = format!("{host}/release/7");
    let cover_url = format!("{host}/cover.png");
    assert!(!http::web_registered(&Url::parse(&album_url).unwrap()));
    assert!(!http::web_registered(&Url::parse(&cover_url).unwrap()));
    let search = if file.direct() {
        let url = file.direct_url(&["7".into()]);
        assert_eq!(url, album_url);
        WebSearchPage {
            direct: true,
            columns: vec![],
            results: vec![WebResult {
                url,
                ..WebResult::default()
            }],
        }
    } else {
        let page_path = root.join("index.txt");
        std::fs::write(
            &page_path,
            format!("{album_url}|Replay Album\n{host}/release/8|Decoy\n"),
        )
        .unwrap();
        let page = http::Page {
            url: Url::parse(&file.search_url(&["Fixture Artist".into()])).unwrap(),
            text: std::fs::read_to_string(page_path).unwrap(),
        };
        let outputs = execute(
            &installed,
            "Replay.src",
            file.index_script.as_ref().unwrap(),
            "index",
            &page,
            engine::Environment {
                vars: variables.clone(),
                ..Default::default()
            },
            false,
        )
        .unwrap();
        index_results(&file, &outputs[engine::DEFAULT_OUTPUT], &page.url).unwrap()
    };
    assert_eq!(search.results[0].url, album_url);
    // The production session registry admits only addresses returned by this
    // fixture. No request is sent; HTTP ingress remains separately qualified.
    for row in &search.results {
        http::register_web_url(&Url::parse(&row.url).unwrap());
    }
    assert!(http::web_registered(&Url::parse(&album_url).unwrap()));
    let album_path = root.join("album.json");
    std::fs::write(&album_path,json!({"album":"Replay Album","artist":"Source Artist","cover":cover_url,"title":"Scalar Saved","tracks":[{"title":"First Saved"},{"title":"Second Saved"}]}).to_string()).unwrap();
    let page = http::Page {
        url: Url::parse(&album_url).unwrap(),
        text: std::fs::read_to_string(album_path).unwrap(),
    };
    let outputs = execute(
        &installed,
        "Replay.src",
        file.album_script.as_ref().unwrap(),
        "album",
        &page,
        engine::Environment {
            vars: variables,
            ..Default::default()
        },
        false,
    )
    .unwrap();
    let release = release_from(outputs, &page.url, &album_url);
    assert_eq!(
        release.tracks.len(),
        if scenario == "multi" { 2 } else { 0 }
    );
    assert_eq!(
        release.fields["COMMENT"],
        ["Saved setting reached the writer"]
    );
    assert_eq!(release.covers[0].url, cover_url);
    http::register_web_url(&Url::parse(&release.covers[0].url).unwrap());
    assert!(http::web_registered(&Url::parse(&cover_url).unwrap()));
    assert!(!http::web_registered(
        &Url::parse(&format!("{host}/unreturned.png")).unwrap()
    ));
    let expected_updates = (0..2)
        .map(|i| {
            let title = match (scenario, i) {
                ("multi", 0) => "First Saved",
                ("multi", _) => "Second Saved",
                _ => "Scalar Saved",
            };
            Fields::from([
                ("TITLE".into(), vec![title.into()]),
                ("ALBUM".into(), vec!["Replay Album".into()]),
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
            label: scenario.into(),
            scenario: scenario.into(),
            source_id: "Replay.src".into(),
            checked_fields: vec!["TITLE".into(), "ALBUM".into(), "COMMENT".into()],
            selected_indices: if scenario == "multi" {
                vec![1, 0, 2]
            } else {
                vec![1, 0]
            },
            search,
            release,
            expected_updates,
            cover_url,
            private_input: false,
        },
        None,
    );
}

struct ReplayReview {
    label: String,
    scenario: String,
    source_id: String,
    checked_fields: Vec<String>,
    selected_indices: Vec<usize>,
    search: WebSearchPage,
    release: SourceRelease,
    expected_updates: Vec<Fields>,
    cover_url: String,
    private_input: bool,
}

/// Reuse the production review and the existing writer/history seam for every
/// parser fixture. The image is controlled bytes, never a network acquisition.
fn review_and_save(root: &Path, replay: ReplayReview, public: Option<&PublicReplay>) {
    let matched = replay.expected_updates.len();
    assert!(matched > 0);
    assert!(replay
        .release
        .covers
        .iter()
        .any(|cover| cover.url == replay.cover_url));
    let old_png = png(3);
    let old_image = root.join("old.png");
    std::fs::write(&old_image, &old_png).unwrap();
    let paths: Vec<_> = (1..=matched + 1)
        .map(|n| root.join(format!("{n}.wav")))
        .collect();
    for (i, path) in paths.iter().enumerate() {
        wav(path, i + 1, &old_image);
    }
    let original_fields: Vec<_> = paths
        .iter()
        .map(|path| tag_core::read_entry_summary(path).unwrap().fields)
        .collect();
    let originals: Vec<_> = paths.iter().map(|p| std::fs::read(p).unwrap()).collect();
    let original_pcm: Vec<_> = originals
        .iter()
        .map(|bytes| wav_pcm(bytes).to_vec())
        .collect();
    assert!(
        original_pcm.windows(2).all(|pair| pair[0] != pair[1]),
        "fixture PCM must distinguish each file"
    );
    let files: Vec<_> = replay.selected_indices.iter().map(|&i| {
        let entry = tag_core::read_entry_summary(&paths[i]).unwrap();
        json!({"path":entry.path,"name":paths[i].file_name().unwrap().to_string_lossy(),"title":entry.fields["TITLE"][0],"artist":entry.fields["ARTIST"][0],"album":"","album_artist":"","disc":"1","track":entry.fields["TRACK"][0],"length":entry.duration})
    }).collect();
    let mutation = std::env::var("TAGGROOVE_WORKFLOW_ORACLE_MUTATION").ok();
    let checked_fields = if mutation.as_deref() == Some("checked-fields") {
        vec!["ARTIST".into()]
    } else {
        replay.checked_fields
    };
    let image_bytes = png(2);
    let saved_cover = root.join("folder.png");
    std::fs::write(&saved_cover, &old_png).unwrap();
    let input = root.join("review-input.json");
    let output = root.join("review-output.json");
    let search_by = public.map(|p| load(&p.installed, &replay.source_id).unwrap().search_by);
    let selected_result_url = public.map(|p| {
        if mutation.as_deref() == Some("selected-result") {
            p.manifest.expected_search["results"][0]["url"]
                .as_str()
                .unwrap()
        } else {
            &p.manifest.page_url
        }
    });
    std::fs::write(&input, serde_json::to_vec(&json!({"version":1,"scenario":replay.scenario,"source":{"id":replay.source_id,"name":"Offline parser replay","search_by":search_by},"checked_fields":checked_fields,"files":files,"search":replay.search,"release":replay.release,"png":base64::engine::general_purpose::STANDARD.encode(&image_bytes),"transport":public.map(|_|"public-acquisition-v1"),"values":public.map(|p|&p.values),"direct":replay.search.direct,"expected_search":public.map(|p|&p.manifest.expected_search),"selected_result_url":selected_result_url,"bridge_control":std::env::var("TAGGROOVE_WORKFLOW_BRIDGE_CONTROL").ok()})).unwrap()).unwrap();
    crate::history::TEST_ROOTS
        .lock()
        .unwrap()
        .push(root.to_path_buf());
    let expected_items = replay
        .expected_updates
        .iter()
        .enumerate()
        .map(|(i, updates)| json!({"path":paths[i].to_string_lossy(),"updates":updates}))
        .collect::<Vec<_>>();
    let expected_cover = json!({"url":replay.cover_url,"replace_existing":true,"save_as":"folder"});
    let child = if let Some(public) = public {
        public_review_helper(
            &input,
            &output,
            public,
            &expected_items,
            &expected_cover,
            &image_bytes,
            &replay.release,
        )
    } else {
        review_helper(&input, &output)
    };
    assert!(
        child.status.success(),
        "production review helper failed: {}",
        String::from_utf8_lossy(&child.stderr)
    );
    let mut reviewed: Value = serde_json::from_slice(&std::fs::read(output).unwrap()).unwrap();
    assert_eq!(reviewed["version"], 1);
    assert_eq!(reviewed["scenario"], replay.scenario);
    assert_eq!(reviewed["network_requests"], 0);
    if let Some(public) = public.filter(|_| !replay.search.direct) {
        let selected = public.manifest.expected_search["results"]
            .as_array()
            .unwrap()
            .iter()
            .enumerate()
            .filter_map(|(index, row)| (row["url"] == public.manifest.page_url).then_some(index))
            .collect::<Vec<_>>();
        assert_eq!(selected.len(), 1, "unique expected release result");
        assert_eq!(reviewed["selected_result_index"], selected[0]);
    }
    // Intentional oracle failures must be caught before the writer is called.
    match mutation.as_deref() {
        Some("matching") => {
            assert!(matched >= 2);
            let first = reviewed["items"][0]["path"].clone();
            reviewed["items"][0]["path"] = reviewed["items"][1]["path"].clone();
            reviewed["items"][1]["path"] = first;
        }
        Some("selected-field") => {
            reviewed["items"][0]["updates"]["ARTIST"] = json!(["Unexpected selected field"])
        }
        Some("artwork") => {
            reviewed["cover"]["url"] = json!("https://workflow.invalid/unreturned.png")
        }
        Some("checked-fields") | None => {}
        Some("selected-result") => panic!("selected-result mutation escaped the release oracle"),
        _ => panic!("unknown oracle mutation"),
    }
    for (i, updates) in replay.expected_updates.iter().enumerate() {
        assert_eq!(
            reviewed["items"][i],
            json!({"path":paths[i].to_string_lossy(),"updates":updates}),
            "review matching/selected-field oracle"
        );
    }
    assert_eq!(reviewed["items"].as_array().unwrap().len(), matched);
    assert_eq!(
        reviewed["cover"],
        json!({"url":replay.cover_url,"replace_existing":true,"save_as":"folder"}),
        "review artwork/session-provenance oracle"
    );
    let items: Vec<crate::sources::ApplyItem> =
        serde_json::from_value(reviewed["items"].clone()).unwrap();
    let cover: crate::sources::ApplyCover =
        serde_json::from_value(reviewed["cover"].clone()).unwrap();
    let image = http::Image::validate(image_bytes.clone()).unwrap();
    if public.is_none() {
        let report = crate::sources::write_import_diagnostic(
            &items,
            Some(&image),
            &crate::sources::ImportCover {
                replace: cover.replace_existing,
                preserve: cover.preserve_existing,
                save_as: cover.save_as,
            },
            &crate::evaluation_settings(None).unwrap(),
        )
        .unwrap();
        assert!(
            report.result.errors.is_empty(),
            "{:?}",
            report.result.errors
        );
        assert_eq!(report.result.entries.len(), matched);
    }
    let written: Vec<_> = paths.iter().map(|p| std::fs::read(p).unwrap()).collect();
    let saved_pcm: Vec<_> = written
        .iter()
        .map(|bytes| wav_pcm(bytes).to_vec())
        .collect();
    assert_eq!(
        saved_pcm, original_pcm,
        "each saved WAV retains its own PCM"
    );
    // Corrupt only an in-memory clone. The observer must detect this without
    // changing any disk file or the history exercised below.
    let mut mutated = written[0].clone();
    let data = wav_pcm_range(&mutated);
    mutated[data.start] ^= 1;
    let pcm_mutation_detected = wav_pcm(&mutated) != original_pcm[0].as_slice();
    assert!(
        pcm_mutation_detected,
        "PCM observer missed a changed sample byte"
    );
    for (i, updates) in replay.expected_updates.iter().enumerate() {
        let entry = tag_core::read_entry(&paths[i]).unwrap();
        let mut expected = original_fields[i].clone();
        expected.extend(updates.clone());
        assert_eq!(
            entry.fields, expected,
            "checked values and every unchecked field"
        );
        assert_eq!(entry.covers.len(), 1);
        assert_eq!(
            base64::engine::general_purpose::STANDARD
                .decode(&entry.covers[0].data_base64)
                .unwrap(),
            image_bytes
        );
    }
    assert_eq!(
        written[matched], originals[matched],
        "unmatched/unselected sentinel changed"
    );
    assert_eq!(std::fs::read(&saved_cover).unwrap(), image_bytes);
    let history = serde_json::to_value(crate::history::history_state().unwrap()).unwrap();
    assert_eq!(history["undo"]["label"], "Tag source import");
    assert_eq!(history["undo"]["files"], matched);
    let undo = tauri::async_runtime::block_on(crate::history::undo_last()).unwrap();
    assert_eq!(serde_json::to_value(undo).unwrap()["errors"], json!([]));
    let undone: Vec<_> = paths
        .iter()
        .map(|path| std::fs::read(path).unwrap())
        .collect();
    assert_eq!(undone, originals);
    assert_eq!(std::fs::read(&saved_cover).unwrap(), old_png);
    let redo = tauri::async_runtime::block_on(crate::history::redo_last()).unwrap();
    assert_eq!(serde_json::to_value(redo).unwrap()["errors"], json!([]));
    let redone: Vec<_> = paths
        .iter()
        .map(|path| std::fs::read(path).unwrap())
        .collect();
    assert_eq!(redone, written);
    assert_eq!(std::fs::read(&saved_cover).unwrap(), image_bytes);
    let hashes = |files: &[Vec<u8>]| {
        files
            .iter()
            .map(|bytes| format!("{:x}", Sha256::digest(bytes)))
            .collect::<Vec<_>>()
    };
    let pcm_hashes = |files: &[Vec<u8>]| {
        files
            .iter()
            .map(|bytes| format!("{:x}", Sha256::digest(wav_pcm(bytes))))
            .collect::<Vec<_>>()
    };
    println!(
        "WORKFLOW_RECEIPT {}",
        json!({"scenario":replay.scenario,"case":replay.label,"independent_fixture":true,"private_parser":replay.private_input,"public_acquisition":public.is_some(),"matched":matched,"untouched":1,"network_requests":reviewed["network_requests"],"selected_result_index":reviewed["selected_result_index"],"history":{"label":history["undo"]["label"],"files":history["undo"]["files"]},"audio_sha256":{"before":hashes(&originals),"saved":hashes(&written),"undo":hashes(&undone),"redo":hashes(&redone)},"pcm_sha256":{"before":pcm_hashes(&originals),"saved":pcm_hashes(&written),"undo":pcm_hashes(&undone),"redo":pcm_hashes(&redone)},"pcm_mutation_control_detected":pcm_mutation_detected,"saved_cover_sha256":format!("{:x}",Sha256::digest(&image_bytes))})
    );
    assert_eq!(
        serde_json::to_value(tauri::async_runtime::block_on(crate::history::undo_last()).unwrap())
            .unwrap()["errors"],
        json!([])
    );
    crate::history::TEST_ROOTS
        .lock()
        .unwrap()
        .retain(|p| p != root);
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct PrivateReplay {
    version: u32,
    case: String,
    bundle: PathBuf,
    source: String,
    settings_id: String,
    settings_values: Map<String, Value>,
    page: PathBuf,
    page_url: String,
    input_sha256: BTreeMap<PathBuf, String>,
    expected_fields: Fields,
    expected_title: String,
    expected_artist: String,
    expected_cover_url: String,
    expected_updates: Fields,
}

/// External-only inputs and independent expectations for the public acquisition
/// route. This schema contains no script bodies or baked-in provider fixtures.
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct PublicManifest {
    version: u32,
    private_manifest: PathBuf,
    values: Vec<String>,
    index: Option<PathBuf>,
    search_url: String,
    page_url: String,
    expected_search: Value,
    input_sha256: BTreeMap<PathBuf, String>,
}
struct PublicReplay {
    installed: PathBuf,
    fixture: PrivateReplay,
    manifest: PublicManifest,
    values: Vec<String>,
    wire: Option<std::sync::Arc<loopback::WireState>>,
}
impl PublicReplay {
    fn transport_counts(&self) -> (usize, usize, usize) {
        self.wire
            .as_ref()
            .map_or_else(http::fixture::active_counts, |wire| {
                (wire.request_count(), 0, 0)
            })
    }
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct BridgeRequest {
    version: u32,
    id: u64,
    command: String,
    args: Value,
}
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct SearchArgs {
    source: String,
    values: Vec<String>,
    ticket: u64,
}
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct ReleaseArgs {
    source: String,
    url: String,
    #[serde(default, rename = "previewUrl")]
    preview_url: Option<String>,
    ticket: u64,
}
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct ImageArgs {
    url: String,
    ticket: u64,
}
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct ApplyArgs {
    items: Value,
    cover: Value,
    columns: Value,
}

/// Kill and reap the child even when a pipe blocks, a protocol assertion
/// panics, or the child closes stdout but keeps running. The watchdog owns an
/// independent deadline; it does not depend on Node timers or pipe progress.
struct BridgeChild {
    child: std::sync::Arc<std::sync::Mutex<std::process::Child>>,
    stop: Option<std::sync::mpsc::Sender<()>>,
    watchdog: Option<std::thread::JoinHandle<()>>,
    timed_out: std::sync::Arc<std::sync::atomic::AtomicBool>,
    killed: std::sync::Arc<std::sync::atomic::AtomicBool>,
    reaped: std::sync::Arc<std::sync::atomic::AtomicBool>,
}
impl BridgeChild {
    fn new(child: std::process::Child, timeout: std::time::Duration) -> Self {
        use std::sync::{
            atomic::{AtomicBool, Ordering},
            mpsc, Arc, Mutex,
        };
        let child = Arc::new(Mutex::new(child));
        let timed_out = Arc::new(AtomicBool::new(false));
        let killed = Arc::new(AtomicBool::new(false));
        let reaped = Arc::new(AtomicBool::new(false));
        let (stop, receive) = mpsc::channel();
        let (process, expired, terminated, waited) = (
            child.clone(),
            timed_out.clone(),
            killed.clone(),
            reaped.clone(),
        );
        let watchdog = std::thread::spawn(move || {
            if matches!(
                receive.recv_timeout(timeout),
                Err(mpsc::RecvTimeoutError::Timeout)
            ) {
                expired.store(true, Ordering::SeqCst);
                let mut child = process.lock().unwrap_or_else(|p| p.into_inner());
                if child.try_wait().ok().flatten().is_none() {
                    child.kill().expect("bridge deadline kill");
                    terminated.store(true, Ordering::SeqCst);
                }
                child.wait().expect("bridge deadline reap");
                waited.store(true, Ordering::SeqCst);
            }
        });
        Self {
            child,
            stop: Some(stop),
            watchdog: Some(watchdog),
            timed_out,
            killed,
            reaped,
        }
    }
    fn expired(&self) -> bool {
        self.timed_out.load(std::sync::atomic::Ordering::SeqCst)
    }
    fn finish(&mut self, force: bool) -> std::process::ExitStatus {
        use std::sync::atomic::Ordering;
        let status = loop {
            {
                let mut child = self.child.lock().unwrap_or_else(|p| p.into_inner());
                if let Some(status) = child.try_wait().expect("bridge status") {
                    break status;
                }
                if force {
                    child.kill().expect("bridge refusal kill");
                    self.killed.store(true, Ordering::SeqCst);
                    break child.wait().expect("bridge refusal reap");
                }
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        };
        self.reaped.store(true, Ordering::SeqCst);
        if let Some(stop) = self.stop.take() {
            let _ = stop.send(());
        }
        if let Some(watchdog) = self.watchdog.take() {
            watchdog.join().expect("bridge watchdog");
        }
        status
    }
}
impl Drop for BridgeChild {
    fn drop(&mut self) {
        self.finish(true);
    }
}

fn public_review_helper(
    input: &Path,
    output: &Path,
    public: &PublicReplay,
    expected_items: &[Value],
    expected_cover: &Value,
    image: &[u8],
    expected_release: &SourceRelease,
) -> std::process::Output {
    use std::io::{BufRead, BufReader, Read, Write};
    let root = input.parent().unwrap();
    let mut observed = std::fs::read_dir(root)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|e| e == "wav"))
        .collect::<Vec<_>>();
    observed.push(root.join("folder.png"));
    observed.sort();
    let before = observed
        .iter()
        .map(|p| std::fs::read(p).unwrap())
        .collect::<Vec<_>>();
    let history_before = serde_json::to_value(crate::history::history_state().unwrap()).unwrap();
    let transport_before = public.transport_counts();
    let control = std::env::var("TAGGROOVE_WORKFLOW_BRIDGE_CONTROL").ok();
    assert!(
        matches!(
            control.as_deref(),
            None | Some("large-stderr" | "stall" | "closed-protocol")
        ),
        "unknown bridge control"
    );
    let mut child = Command::new("node")
        .arg(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../tests/helpers/source-workflow-review.mjs"),
        )
        .arg(input)
        .arg(output)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("public review requires Node");
    let mut stdout = BufReader::new(child.stdout.take().unwrap());
    let mut stdin = child.stdin.take().unwrap();
    let mut stderr = child.stderr.take().unwrap();
    let started = std::time::Instant::now();
    let mut child = BridgeChild::new(
        child,
        std::time::Duration::from_secs(if control.as_deref() == Some("stall") {
            1
        } else {
            30
        }),
    );
    // Drain continuously, retain only 64 KiB, and count the discarded bytes.
    // A data-URL stack trace must never fill the pipe or exhaust parent memory.
    let stderr_reader = std::thread::spawn(move || {
        let mut retained = Vec::new();
        let mut total = 0usize;
        let mut buffer = [0u8; 8192];
        loop {
            let count = stderr.read(&mut buffer).expect("bridge stderr read");
            if count == 0 {
                break;
            }
            total = total.saturating_add(count);
            let keep = count.min((64 * 1024usize).saturating_sub(retained.len()));
            retained.extend_from_slice(&buffer[..keep]);
        }
        (retained, total)
    });
    let mut sequence = 0;
    let mut searched = false;
    let mut released = false;
    let mut images = 0;
    let mut applied = false;
    let mut failure = None;
    let mut refusal_transport_before = None;
    let mut wire_error = None;
    loop {
        let mut line = Vec::new();
        let length = (&mut stdout)
            .take(1024 * 1024 + 1)
            .read_until(b'\n', &mut line)
            .unwrap();
        if length == 0 {
            if child.expired() {
                failure = Some("bridge parent deadline");
            } else if !applied {
                failure = Some("bridge closed before apply");
            }
            break;
        }
        if length > 1024 * 1024 || line.last() != Some(&b'\n') {
            failure = Some("bridge request bound");
            break;
        }
        let request: BridgeRequest = match serde_json::from_slice(&line) {
            Ok(request) => request,
            Err(_) => {
                failure = Some("bridge schema");
                break;
            }
        };
        sequence += 1;
        if request.version != 1 || request.id != sequence || sequence > 24 {
            failure = Some("bridge sequence");
            break;
        }
        // Errors sent back to Node intentionally omit private script diagnostics.
        let command_transport_before = public.transport_counts();
        let result: Result<Value, &'static str> = tauri::async_runtime::block_on(async {
            match request.command.as_str() {
                "web_source_search" => {
                    let args: SearchArgs =
                        serde_json::from_value(request.args).map_err(|_| "search schema")?;
                    if searched
                        || args.source != public.fixture.source
                        || args.values != public.values
                    {
                        return Err("search oracle");
                    }
                    let found = search_with_preparation(
                        args.source.clone(),
                        args.values,
                        args.ticket,
                        prepare_in(public.installed.clone(), &args.source),
                    )
                    .await
                    .map_err(|_| "search refused")?;
                    let found = serde_json::to_value(found).unwrap();
                    if found != public.manifest.expected_search {
                        if public.wire.is_some() {
                            println!(
                                "WIRE_INDEX_ORACLE_MISMATCH {}",
                                json!({"actual":found,"expected":public.manifest.expected_search})
                            );
                        }
                        return Err("independent index oracle");
                    }
                    searched = true;
                    Ok(found)
                }
                "web_source_release" => {
                    let args: ReleaseArgs =
                        serde_json::from_value(request.args).map_err(|_| "release schema")?;
                    if !searched
                        || released
                        || args.source != public.fixture.source
                        || args.url != public.manifest.page_url
                    {
                        return Err("release oracle");
                    }
                    if let Some(wire) = &public.wire {
                        wire.release_ticket
                            .store(args.ticket, std::sync::atomic::Ordering::SeqCst);
                    }
                    let release = release_with_preparation(
                        args.source.clone(),
                        args.url,
                        args.preview_url,
                        args.ticket,
                        prepare_in(public.installed.clone(), &args.source),
                    )
                    .await
                    .map_err(|error| {
                        wire_error = Some(error.to_string());
                        "release refused"
                    })?;
                    if release.provider != expected_release.provider
                        || release.id != expected_release.id
                        || release.url != expected_release.url
                        || release.fields != public.fixture.expected_fields
                        || release.title != public.fixture.expected_title
                        || release.artist != public.fixture.expected_artist
                        || release.tracks != expected_release.tracks
                        || release
                            .covers
                            .iter()
                            .map(|c| c.url.as_str())
                            .collect::<Vec<_>>()
                            != [public.fixture.expected_cover_url.as_str()]
                    {
                        return Err("independent album oracle");
                    }
                    released = true;
                    Ok(serde_json::to_value(release).unwrap())
                }
                "tag_source_image" => {
                    let args: ImageArgs =
                        serde_json::from_value(request.args).map_err(|_| "image schema")?;
                    if !released || args.url != public.fixture.expected_cover_url || images != 0 {
                        return Err("image provenance oracle");
                    }
                    let data = crate::sources::tag_source_image(args.url, args.ticket)
                        .await
                        .map_err(|_| "image refused")?;
                    if data
                        != format!(
                            "data:image/png;base64,{}",
                            base64::engine::general_purpose::STANDARD.encode(image)
                        )
                    {
                        return Err("preview bytes oracle");
                    }
                    images += 1;
                    Ok(json!(data))
                }
                "tag_source_apply" => {
                    let mut args: ApplyArgs =
                        serde_json::from_value(request.args).map_err(|_| "apply schema")?;
                    if !released || images != 1 || applied || !args.columns.is_null() {
                        return Err("apply ordering");
                    }
                    match std::env::var("TAGGROOVE_WORKFLOW_ORACLE_MUTATION")
                        .ok()
                        .as_deref()
                    {
                        Some("selected-field") => {
                            args.items[0]["updates"]["ARTIST"] = json!(["unexpected"])
                        }
                        Some("artwork") => {
                            args.cover["url"] = json!("https://unreturned.invalid/cover.png")
                        }
                        Some("checked-fields") | None => {}
                        _ => return Err("unknown public mutation"),
                    }
                    if args.items != json!(expected_items) || args.cover != *expected_cover {
                        return Err("pre-write field/artwork oracle");
                    }
                    let items = serde_json::from_value(args.items).map_err(|_| "apply items")?;
                    let cover = serde_json::from_value(args.cover).map_err(|_| "apply cover")?;
                    let report = crate::sources::ipc::tag_source_apply_with_diagnostics(
                        items,
                        Some(cover),
                        None,
                    )
                    .await
                    .map_err(|error| {
                        wire_error = Some(error.to_string());
                        "public apply refused"
                    })?;
                    if !report.result.errors.is_empty()
                        || report.result.entries.len() != expected_items.len()
                    {
                        return Err("public apply result");
                    }
                    applied = true;
                    Ok(Value::Null)
                }
                "tag_source_cancel" => {
                    #[derive(serde::Deserialize)]
                    #[serde(deny_unknown_fields)]
                    struct CancelArgs {
                        ticket: u64,
                    }
                    let args: CancelArgs =
                        serde_json::from_value(request.args).map_err(|_| "cancel schema")?;
                    crate::sources::tag_source_cancel(args.ticket);
                    Ok(Value::Null)
                }
                _ => Err("unknown bridge command"),
            }
        });
        let response = match result {
            Ok(value) => json!({"version":1,"id":request.id,"result":value}),
            Err(error) => {
                failure = Some(error);
                refusal_transport_before = Some(command_transport_before);
                json!({"version":1,"id":request.id,"error":error})
            }
        };
        let bytes = serde_json::to_vec(&response).unwrap();
        assert!(bytes.len() <= 1024 * 1024);
        if stdin
            .write_all(&bytes)
            .and_then(|_| stdin.write_all(b"\n"))
            .and_then(|_| stdin.flush())
            .is_err()
        {
            failure = Some(if child.expired() {
                "bridge parent deadline"
            } else {
                "bridge input closed"
            });
        }
        if failure.is_some() {
            break;
        }
    }
    drop(stdin);
    let status = child.finish(failure.is_some());
    if child.expired() {
        failure = Some("bridge parent deadline");
    }
    let (stderr, stderr_total) = stderr_reader.join().expect("bridge stderr drain");
    let result = std::process::Output {
        status,
        stdout: Vec::new(),
        stderr,
    };
    if let Some(wire) = &public.wire {
        wire.check_refusal(failure, wire_error.as_deref());
        if failure.is_some() {
            assert!(!applied);
            assert_eq!(
                observed
                    .iter()
                    .map(|p| std::fs::read(p).unwrap())
                    .collect::<Vec<_>>(),
                before
            );
            assert_eq!(
                serde_json::to_value(crate::history::history_state().unwrap()).unwrap(),
                history_before
            );
            assert!(child.reaped.load(std::sync::atomic::Ordering::SeqCst));
            println!(
                "WIRE_PREWRITE_REFUSAL {}",
                json!({"guard":failure,"cause":wire_error,"writer_called":false,"wav_sidecar_history_unchanged":true,"reaped":true,"requests":wire.request_count()})
            );
        }
    }
    if matches!(
        failure,
        Some("bridge parent deadline" | "bridge closed before apply")
    ) {
        assert!(!applied && sequence == 0);
        assert_eq!(
            public.transport_counts(),
            transport_before,
            "stalled/closed child consumed transport"
        );
        assert_eq!(
            observed
                .iter()
                .map(|p| std::fs::read(p).unwrap())
                .collect::<Vec<_>>(),
            before
        );
        assert_eq!(
            serde_json::to_value(crate::history::history_state().unwrap()).unwrap(),
            history_before
        );
        assert!(child.reaped.load(std::sync::atomic::Ordering::SeqCst));
        assert!(child.killed.load(std::sync::atomic::Ordering::SeqCst));
        println!(
            "ACQUISITION_BRIDGE_REFUSAL {}",
            json!({"guard":failure,"control":control,"writer_called":false,"commands":sequence,"transport_unchanged":true,"killed":true,"reaped":true,"wav_sidecar_history_unchanged":true,"elapsed_ms":started.elapsed().as_millis()})
        );
    }
    if control.as_deref() == Some("large-stderr") {
        assert!(stderr_total >= 256 * 1024);
        assert_eq!(result.stderr.len(), 64 * 1024);
        assert!(status.success() && failure.is_none());
        println!(
            "ACQUISITION_BRIDGE_STDERR {}",
            json!({"total_bytes":stderr_total,"retained_bytes":result.stderr.len(),"bounded_capture":true,"reaped":true,"public_flow_complete":applied})
        );
    }
    if failure == Some("pre-write field/artwork oracle") {
        assert!(!applied);
        assert_eq!(
            observed
                .iter()
                .map(|p| std::fs::read(p).unwrap())
                .collect::<Vec<_>>(),
            before
        );
        assert_eq!(
            serde_json::to_value(crate::history::history_state().unwrap()).unwrap(),
            history_before
        );
        println!(
            "ACQUISITION_PREWRITE_REFUSAL {}",
            json!({"guard":"pre-write field/artwork oracle","writer_called":false,"wav_sidecar_history_unchanged":true,"files":observed.len()})
        );
    }
    if matches!(
        failure,
        Some("independent index oracle" | "release oracle" | "independent album oracle")
    ) && public.wire.is_none()
    {
        assert!(!applied && !released);
        assert_eq!(
            observed
                .iter()
                .map(|p| std::fs::read(p).unwrap())
                .collect::<Vec<_>>(),
            before
        );
        assert_eq!(
            serde_json::to_value(crate::history::history_state().unwrap()).unwrap(),
            history_before
        );
        let counts = public.transport_counts();
        assert_eq!(counts.1, 0, "source refusal attempted unexpected transport");
        let index_bodies = usize::from(public.manifest.index.is_some());
        assert_eq!(
            counts.0,
            index_bodies + usize::from(failure == Some("independent album oracle"))
        );
        if failure == Some("release oracle") {
            assert_eq!(Some(counts), refusal_transport_before);
        }
        assert!(child.reaped.load(std::sync::atomic::Ordering::SeqCst));
        println!(
            "ACQUISITION_SOURCE_REFUSAL {}",
            json!({"guard":failure,"writer_called":false,"wav_sidecar_history_unchanged":true,"consumed_bodies":counts.0,"transport_misses":counts.1,"pending_bodies":counts.2,"release_body_consumed":failure==Some("independent album oracle"),"release_request_transport_unchanged":failure==Some("release oracle"),"reaped":true})
        );
    }
    assert!(
        failure.is_none(),
        "public bridge refused: {}",
        failure.unwrap_or_default()
    );
    assert!(
        searched && released && images == 1 && applied,
        "public bridge incomplete"
    );
    result
}

fn fixture_reply(url: &str, status: u16, body: Vec<u8>, image: bool) -> http::fixture::Reply {
    http::fixture::Reply {
        url: url.into(),
        status,
        body,
        content_type: if image {
            "image/png"
        } else {
            "text/html; charset=utf-8"
        },
        cancel: None,
    }
}

#[test]
#[ignore = "private public-acquisition fixture; separate process with TAGGROOVE_PRIVATE_ACQUISITION_MANIFEST and OS network denial"]
fn private_public_acquisition_review_apply_and_journaled_audio_workflow() {
    let _guard = crate::history::TEST_LOCK
        .lock()
        .unwrap_or_else(|p| p.into_inner());
    assert!(!debug_enabled(), "private parser debug must be disabled");
    let manifest: PublicManifest = serde_json::from_slice(
        &std::fs::read(
            std::env::var_os("TAGGROOVE_PRIVATE_ACQUISITION_MANIFEST")
                .expect("private acquisition manifest"),
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(manifest.version, 1);
    let fixture: PrivateReplay =
        serde_json::from_slice(&std::fs::read(&manifest.private_manifest).unwrap()).unwrap();
    assert_eq!(fixture.version, 1);
    let hashes = || {
        manifest
            .input_sha256
            .keys()
            .map(|path| {
                (
                    path.clone(),
                    format!("{:x}", Sha256::digest(std::fs::read(path).unwrap())),
                )
            })
            .collect::<BTreeMap<_, _>>()
    };
    assert_eq!(
        hashes(),
        manifest.input_sha256,
        "private inputs before acquisition"
    );
    for (path, hash) in &fixture.input_sha256 {
        assert_eq!(manifest.input_sha256.get(path), Some(hash));
    }
    let dir = tempfile::tempdir().unwrap();
    let root = dunce::canonicalize(dir.path()).unwrap();
    let installed = root.join("sources");
    install(&installed, &[fixture.bundle.to_string_lossy().into_owned()])
        .unwrap_or_else(|_| panic!("private install refused"));
    save_settings(
        &installed,
        &fixture.settings_id,
        fixture.settings_values.clone(),
    )
    .unwrap_or_else(|_| panic!("private settings refused"));
    let file = load(&installed, &fixture.source).unwrap_or_else(|_| panic!("private load refused"));
    assert_eq!(manifest.values.len(), file.search_by.len());
    assert_eq!(file.direct(), manifest.index.is_none());
    if file.direct() {
        assert_eq!(file.direct_url(&manifest.values), manifest.page_url);
    } else {
        assert_eq!(file.search_url(&manifest.values), manifest.search_url);
    }
    let page_url = Url::parse(&manifest.page_url).unwrap();
    let cover_url = Url::parse(&fixture.expected_cover_url).unwrap();
    assert!(!http::web_registered(&page_url));
    assert!(!http::web_registered(&cover_url));
    let mut replies = Vec::new();
    if let Some(index) = &manifest.index {
        replies.push(fixture_reply(
            &manifest.search_url,
            200,
            std::fs::read(index).unwrap(),
            false,
        ));
    }
    replies.push(fixture_reply(
        &manifest.page_url,
        200,
        std::fs::read(&fixture.page).unwrap(),
        false,
    ));
    replies.push(fixture_reply(
        &fixture.expected_cover_url,
        200,
        png(2),
        true,
    ));
    replies.push(fixture_reply(
        &fixture.expected_cover_url,
        200,
        png(2),
        true,
    ));
    let count = replies.len();
    let scope = http::fixture::Scope::start(replies);
    // Even a fixture entry must never act as a session grant. These refusals
    // occur before deferred preparation and before transport consumption.
    let denied = tauri::async_runtime::block_on(release_with_preparation(
        fixture.source.clone(),
        manifest.page_url.clone(),
        None,
        1,
        async { panic!("unauthorized release prepared source") },
    ));
    assert!(denied.is_err());
    assert!(
        tauri::async_runtime::block_on(crate::sources::tag_source_image(
            fixture.expected_cover_url.clone(),
            1
        ))
        .is_err()
    );
    assert_eq!(scope.counts(), (0, 0, count));
    let (matched, checked_fields) = match fixture.case.as_str() {
        "traxsource" => (1, vec!["ALBUM".into(), "YEAR".into()]),
        "soundcloud" => (2, vec!["TITLE".into()]),
        "traxsource-track-search" => (1, vec!["TITLE".into(), "LENGTH".into(), "YEAR".into()]),
        _ => panic!("unknown private case"),
    };
    let expected_release = SourceRelease {
        preview_url: None,
        provider: Provider::Web,
        id: manifest.page_url.clone(),
        url: manifest.page_url.clone(),
        title: fixture.expected_title.clone(),
        artist: fixture.expected_artist.clone(),
        fields: fixture.expected_fields.clone(),
        tracks: vec![],
        covers: vec![SourceCover {
            url: fixture.expected_cover_url.clone(),
            thumb: None,
            kind: "Front cover".into(),
            width: None,
            height: None,
        }],
        more_covers: false,
    };
    let review = ReplayReview {
        label: fixture.case.clone(),
        scenario: "scalar".into(),
        source_id: fixture.source.clone(),
        checked_fields,
        selected_indices: if matched == 1 { vec![0] } else { vec![1, 0] },
        search: WebSearchPage {
            direct: file.direct(),
            ..WebSearchPage::default()
        },
        release: expected_release,
        expected_updates: vec![fixture.expected_updates.clone(); matched],
        cover_url: fixture.expected_cover_url.clone(),
        private_input: true,
    };
    let public = PublicReplay {
        installed,
        values: manifest.values.clone(),
        fixture,
        manifest,
        wire: None,
    };
    review_and_save(&root, review, Some(&public));
    assert_eq!(
        scope.counts(),
        (count, 0, 0),
        "every acquisition used its exact fixture"
    );
    drop(scope);
    acquisition_refusal_controls(&root, &public);
    assert_eq!(
        hashes_for(&public.manifest.input_sha256),
        public.manifest.input_sha256,
        "private inputs after acquisition"
    );
    println!(
        "ACQUISITION_RECEIPT {}",
        json!({"case":public.fixture.case,"fixture_responses":count,"real_search_release_image_public_apply":true,"manual_grants":false,"wire_http_tls_provider_native":false,"input_hash_count":public.manifest.input_sha256.len()})
    );
}

fn hashes_for(expected: &BTreeMap<PathBuf, String>) -> BTreeMap<PathBuf, String> {
    expected
        .keys()
        .map(|path| {
            (
                path.clone(),
                format!("{:x}", Sha256::digest(std::fs::read(path).unwrap())),
            )
        })
        .collect()
}

fn acquisition_refusal_controls(root: &Path, public: &PublicReplay) {
    let mut paths = std::fs::read_dir(root)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|e| e == "wav"))
        .collect::<Vec<_>>();
    paths.sort();
    let mut observed = paths.clone();
    observed.push(root.join("folder.png"));
    let before = observed
        .iter()
        .map(|p| std::fs::read(p).unwrap())
        .collect::<Vec<_>>();
    let history_before = serde_json::to_value(crate::history::history_state().unwrap()).unwrap();
    let url = "https://unreturned.invalid/negative";
    let ticket = u64::MAX - 100;
    // Fail-closed miss: a valid page URL without a mapped GET never sends.
    {
        let scope = http::fixture::Scope::start(vec![]);
        let error = tauri::async_runtime::block_on(http::web_page(
            "Fixture",
            url,
            None,
            Channel::Search,
            ticket,
        ))
        .unwrap_err();
        assert!(error.contains("network fallback refused"));
        assert_eq!(scope.counts(), (0, 1, 0));
    }
    // Exact method as well as exact URL: mapped GET bytes never admit POST.
    {
        let scope = http::fixture::Scope::start(vec![fixture_reply(url, 200, vec![], false)]);
        let service = http::Service::new("Fixture", &http::IMAGES, 0);
        let error =
            tauri::async_runtime::block_on(http::fixture::post_probe(&service, url, ticket))
                .unwrap_err();
        assert!(error.contains("network fallback refused"));
        assert_eq!(scope.counts(), (0, 1, 1));
    }
    // A mapped but unreturned artwork address cannot authorize public apply.
    {
        let scope = http::fixture::Scope::start(vec![fixture_reply(url, 200, png(2), true)]);
        let result =
            tauri::async_runtime::block_on(crate::sources::ipc::tag_source_apply_with_diagnostics(
                vec![crate::sources::ApplyItem {
                    path: paths[0].to_string_lossy().into_owned(),
                    updates: public.fixture.expected_updates.clone(),
                    raw_lyrics: false,
                }],
                Some(crate::sources::ApplyCover {
                    url: url.into(),
                    replace_existing: true,
                    preserve_existing: false,
                    save_as: Some("folder".into()),
                }),
                None,
            ));
        let error = match result {
            Err(error) => error,
            Ok(_) => panic!("unreturned cover accepted"),
        };
        assert_eq!(
            error,
            "This image address is not from a supported tag source."
        );
        assert_eq!(scope.counts(), (0, 0, 1));
    }
    // Search/album status, actual bounded bytes, and supersession after response.
    for (label, status, body, cancel) in [
        ("status", 404, vec![], false),
        ("size", 200, vec![b'x'; 8 * 1024 * 1024 + 1], false),
        ("cancel", 200, b"unused parser input".to_vec(), true),
    ] {
        let mut reply = fixture_reply(url, status, body, false);
        if cancel {
            reply.cancel = Some((Channel::Release, ticket + 1));
        }
        let scope = http::fixture::Scope::start(vec![reply]);
        let result = tauri::async_runtime::block_on(http::web_page(
            "Fixture",
            url,
            None,
            Channel::Release,
            ticket,
        ));
        let error = result.unwrap_err();
        let expected = match label {
            "status" => format!("Fixture returned HTTP 404 for {url}"),
            "size" => "Fixture returned more data than expected.".into(),
            "cancel" => http::CANCELLED.into(),
            _ => unreachable!(),
        };
        assert_eq!(error, expected, "wrong {label} refusal");
        assert_eq!(scope.counts(), (1, 0, 0));
    }
    {
        let scope = http::fixture::Scope::start(vec![fixture_reply(url, 200, vec![], false)]);
        let result = tauri::async_runtime::block_on(http::web_page(
            "Fixture",
            url,
            None,
            Channel::Release,
            ticket,
        ));
        assert_eq!(result.unwrap_err(), http::CANCELLED);
        assert_eq!(scope.counts(), (0, 0, 1));
    }
    // Public apply still owns authorization, status, size and image validation
    // before mutation. Cover URL was granted only by the real album worker.
    for (label, status, bytes) in [
        ("status", 404, vec![]),
        ("size", 200, vec![0; http::MAX_IMAGE_BYTES + 1]),
        ("image", 200, b"not an image".to_vec()),
    ] {
        let scope = http::fixture::Scope::start(vec![fixture_reply(
            &public.fixture.expected_cover_url,
            status,
            bytes,
            true,
        )]);
        let items = vec![crate::sources::ApplyItem {
            path: paths[0].to_string_lossy().into_owned(),
            updates: public.fixture.expected_updates.clone(),
            raw_lyrics: false,
        }];
        let cover = crate::sources::ApplyCover {
            url: public.fixture.expected_cover_url.clone(),
            replace_existing: true,
            preserve_existing: false,
            save_as: Some("folder".into()),
        };
        let result = tauri::async_runtime::block_on(
            crate::sources::ipc::tag_source_apply_with_diagnostics(items, Some(cover), None),
        );
        let error = match result {
            Err(error) => error,
            Ok(_) => panic!("public apply {label} accepted"),
        };
        let expected = match label {
            "status" => "This release has no image at that address.",
            "size" => "The image server returned more data than expected.",
            "image" => "The downloaded cover is not a supported image.",
            _ => unreachable!(),
        };
        assert_eq!(error, expected, "wrong public apply {label} refusal");
        assert_eq!(scope.counts(), (1, 0, 0));
        assert_eq!(
            observed
                .iter()
                .map(|p| std::fs::read(p).unwrap())
                .collect::<Vec<_>>(),
            before,
            "refused apply changed audio or sidecar"
        );
    }
    {
        let image_ticket = u64::MAX - 50;
        let mut reply = fixture_reply(&public.fixture.expected_cover_url, 200, png(2), true);
        reply.cancel = Some((Channel::Image, image_ticket + 1));
        let scope = http::fixture::Scope::start(vec![reply]);
        assert_eq!(
            tauri::async_runtime::block_on(crate::sources::tag_source_image(
                public.fixture.expected_cover_url.clone(),
                image_ticket
            ))
            .unwrap_err(),
            http::CANCELLED
        );
        assert_eq!(scope.counts(), (1, 0, 0));
    }
    assert_eq!(
        observed
            .iter()
            .map(|p| std::fs::read(p).unwrap())
            .collect::<Vec<_>>(),
        before,
        "refused acquisition changed audio or sidecar"
    );
    assert_eq!(
        serde_json::to_value(crate::history::history_state().unwrap()).unwrap(),
        history_before,
        "refused acquisition changed history"
    );
    println!(
        "ACQUISITION_NEGATIVE_RECEIPT {}",
        json!({"unauthorized_release_and_image_before_transport":true,"unexpected_request_no_fallback":true,"unexpected_method_no_fallback":true,"unreturned_artwork_public_apply_refused_before_transport":true,"status_and_size_refused":true,"post_response_and_pre_dispatch_cancel_refused":true,"image_preview_post_response_cancel_refused":true,"invalid_image_public_apply_refused":true,"audio_sidecar_history_unchanged_on_refusal":true})
    );
}

#[test]
#[ignore = "private invented saved page; requires TAGGROOVE_PRIVATE_WORKFLOW_MANIFEST; run with OS network denial"]
fn private_saved_page_production_review_and_journaled_audio_workflow() {
    let _guard = crate::history::TEST_LOCK
        .lock()
        .unwrap_or_else(|p| p.into_inner());
    let path = std::env::var_os("TAGGROOVE_PRIVATE_WORKFLOW_MANIFEST")
        .expect("private replay manifest required");
    let fixture: PrivateReplay = serde_json::from_slice(&std::fs::read(path).unwrap())
        .expect("invalid private replay manifest");
    assert_eq!(fixture.version, 1);
    let (matched, checked_fields) = match fixture.case.as_str() {
        "traxsource" => (1, vec!["ALBUM".to_owned(), "YEAR".to_owned()]),
        "soundcloud" => (2, vec!["TITLE".to_owned()]),
        _ => panic!("unsupported private replay case"),
    };
    let input_hashes = || {
        fixture
            .input_sha256
            .keys()
            .map(|path| {
                (
                    path.clone(),
                    format!("{:x}", Sha256::digest(std::fs::read(path).unwrap())),
                )
            })
            .collect::<BTreeMap<_, _>>()
    };
    assert_eq!(
        input_hashes(),
        fixture.input_sha256,
        "private input hashes before replay"
    );
    let dir = tempfile::tempdir().unwrap();
    let root = dunce::canonicalize(dir.path()).unwrap();
    let installed = root.join("sources");
    install(&installed, &[fixture.bundle.to_string_lossy().into_owned()])
        .unwrap_or_else(|_| panic!("private bundle installation refused; source text omitted"));
    save_settings(
        &installed,
        &fixture.settings_id,
        fixture.settings_values.clone(),
    )
    .unwrap_or_else(|_| panic!("private settings refused; values omitted"));
    let file = load(&installed, &fixture.source)
        .unwrap_or_else(|_| panic!("private source load refused; source text omitted"));
    let page = http::Page {
        url: Url::parse(&fixture.page_url).unwrap(),
        text: http::decode_text(&std::fs::read(&fixture.page).unwrap(), None),
    };
    let outputs = execute(
        &installed,
        &fixture.source,
        file.album_script
            .as_ref()
            .expect("private album parser missing"),
        "album",
        &page,
        engine::Environment {
            vars: vars(&installed, &file).unwrap(),
            ..Default::default()
        },
        false,
    )
    .unwrap_or_else(|_| panic!("private album parser failed; script and response omitted"));
    let release = release_from(outputs, &page.url, &fixture.page_url);
    assert_eq!(
        release.fields, fixture.expected_fields,
        "independent private parser field oracle"
    );
    assert_eq!(release.title, fixture.expected_title);
    assert_eq!(release.artist, fixture.expected_artist);
    assert!(
        release.tracks.is_empty(),
        "private fixture uses scalar fallback, not explicit tracks"
    );
    assert_eq!(
        release
            .covers
            .iter()
            .map(|cover| cover.url.as_str())
            .collect::<Vec<_>>(),
        vec![fixture.expected_cover_url.as_str()]
    );
    // This is an adapter for a fixture-selected page, not a private index/HTTP
    // result or a session grant. Controlled image bytes bypass HTTP acquisition.
    let cover_address = Url::parse(&fixture.expected_cover_url).unwrap();
    assert!(!http::web_registered(&page.url));
    assert!(!http::web_registered(&cover_address));
    let search = WebSearchPage {
        direct: true,
        columns: vec![],
        results: vec![WebResult {
            url: fixture.page_url.clone(),
            ..WebResult::default()
        }],
    };
    review_and_save(
        &root,
        ReplayReview {
            label: fixture.case.clone(),
            scenario: "scalar".into(),
            source_id: fixture.source.clone(),
            checked_fields,
            selected_indices: if matched == 1 { vec![0] } else { vec![1, 0] },
            search,
            release,
            expected_updates: vec![fixture.expected_updates.clone(); matched],
            cover_url: fixture.expected_cover_url.clone(),
            private_input: true,
        },
        None,
    );
    assert!(!http::web_registered(&page.url));
    assert!(!http::web_registered(&cover_address));
    assert_eq!(
        input_hashes(),
        fixture.input_sha256,
        "private input hashes after replay"
    );
}

#[test]
fn saved_pages_production_review_and_journaled_audio_workflow() {
    let _guard = crate::history::TEST_LOCK
        .lock()
        .unwrap_or_else(|p| p.into_inner());
    replay("multi");
    replay("scalar");
}

#[test]
fn workflow_review_helper_rejects_invalid_protocol_before_output() {
    let root = tempfile::tempdir().unwrap();
    let input = root.path().join("input.json");
    let output = root.path().join("output.json");
    std::fs::write(&input, b"{\"version\":0}").unwrap();
    let child = review_helper(&input, &output);
    assert!(!child.status.success());
    assert!(
        String::from_utf8_lossy(&child.stderr).contains("unsupported workflow protocol version")
    );
    assert!(!output.exists());
}
