//! Independent saved-page → production review → real writer/history control.
//! HTTP acquisition is deliberately outside this filesystem replay's evidence.
use super::*;
use base64::Engine;
use serde_json::json;
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, ops::Range, path::PathBuf, process::Command};

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
            variables.clone(),
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
        variables,
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
fn review_and_save(root: &Path, replay: ReplayReview) {
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
    std::fs::write(&input, serde_json::to_vec(&json!({"version":1,"scenario":replay.scenario,"source":{"id":replay.source_id,"name":"Offline parser replay"},"checked_fields":checked_fields,"files":files,"search":replay.search,"release":replay.release,"png":base64::engine::general_purpose::STANDARD.encode(&image_bytes)})).unwrap()).unwrap();
    let child = review_helper(&input, &output);
    assert!(
        child.status.success(),
        "production review helper failed: {}",
        String::from_utf8_lossy(&child.stderr)
    );
    let mut reviewed: Value = serde_json::from_slice(&std::fs::read(output).unwrap()).unwrap();
    assert_eq!(reviewed["version"], 1);
    assert_eq!(reviewed["scenario"], replay.scenario);
    assert_eq!(reviewed["network_requests"], 0);
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
    crate::history::TEST_ROOTS
        .lock()
        .unwrap()
        .push(root.to_path_buf());
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
        json!({"scenario":replay.scenario,"case":replay.label,"independent_fixture":true,"private_parser":replay.private_input,"matched":matched,"untouched":1,"network_requests":reviewed["network_requests"],"history":{"label":history["undo"]["label"],"files":history["undo"]["files"]},"audio_sha256":{"before":hashes(&originals),"saved":hashes(&written),"undo":hashes(&undone),"redo":hashes(&redone)},"pcm_sha256":{"before":pcm_hashes(&originals),"saved":pcm_hashes(&written),"undo":pcm_hashes(&undone),"redo":pcm_hashes(&redone)},"pcm_mutation_control_detected":pcm_mutation_detected,"saved_cover_sha256":format!("{:x}",Sha256::digest(&image_bytes))})
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
        vars(&installed, &file).unwrap(),
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
