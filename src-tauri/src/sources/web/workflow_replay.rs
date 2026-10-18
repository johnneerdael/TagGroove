//! Independent saved-page → production review → real writer/history control.
//! HTTP acquisition is deliberately outside this filesystem replay's evidence.
use super::*;
use base64::Engine;
use serde_json::json;
use sha2::{Digest, Sha256};
use std::process::Command;

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
    bytes.extend([0; 32]);
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

fn review_helper(input: &Path, output: &Path) -> std::process::Output {
    Command::new("node")
        .arg(Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/helpers/source-workflow-review.mjs"))
        .arg(input).arg(output).output()
        .expect("source workflow replay requires Node and npm ci (reviewed-build installs both before cargo test)")
}

fn replay(scenario: &str) {
    let dir = tempfile::tempdir().unwrap();
    let root = dunce::canonicalize(dir.path()).unwrap();
    crate::history::TEST_ROOTS
        .lock()
        .unwrap()
        .push(root.clone());
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
    let old_png = png(3);
    let old_image = root.join("old.png");
    std::fs::write(&old_image, &old_png).unwrap();
    let paths: Vec<_> = (1..=3).map(|n| root.join(format!("{n}.wav"))).collect();
    for (i, path) in paths.iter().enumerate() {
        wav(path, i + 1, &old_image);
    }
    let originals: Vec<_> = paths.iter().map(|p| std::fs::read(p).unwrap()).collect();
    let files:Vec<_>=[1,0,2].into_iter().take(if scenario=="multi" {3} else {2}).map(|i|{
        let entry=tag_core::read_entry_summary(&paths[i]).unwrap();
        json!({"path":entry.path,"name":paths[i].file_name().unwrap().to_string_lossy(),"title":entry.fields["TITLE"][0],"artist":entry.fields["ARTIST"][0],"album":"","album_artist":"","disc":"1","track":entry.fields["TRACK"][0],"length":entry.duration})
    }).collect();
    let image_bytes = png(2);
    let saved_cover = root.join("folder.png");
    std::fs::write(&saved_cover, &old_png).unwrap();
    let input = root.join("review-input.json");
    let output = root.join("review-output.json");
    std::fs::write(&input,serde_json::to_vec(&json!({"version":1,"scenario":scenario,"files":files,"search":search,"release":release,"png":base64::engine::general_purpose::STANDARD.encode(&image_bytes)})).unwrap()).unwrap();
    let child = review_helper(&input, &output);
    assert!(
        child.status.success(),
        "production review helper failed: {}",
        String::from_utf8_lossy(&child.stderr)
    );
    let mut reviewed: Value = serde_json::from_slice(&std::fs::read(output).unwrap()).unwrap();
    assert_eq!(reviewed["version"], 1);
    assert_eq!(reviewed["scenario"], scenario);
    assert_eq!(reviewed["network_requests"], 0);
    // Opt-in intentional oracle failures prove these checks detect integration
    // corruption; they are not presented as production defects.
    if let Ok(mutation) = std::env::var("TAGGROOVE_WORKFLOW_ORACLE_MUTATION") {
        match mutation.as_str() {
            "matching" => {
                let first = reviewed["items"][0]["path"].clone();
                reviewed["items"][0]["path"] = reviewed["items"][1]["path"].clone();
                reviewed["items"][1]["path"] = first;
            }
            "selected-field" => {
                reviewed["items"][0]["updates"]["ARTIST"] = json!(["Unexpected selected field"])
            }
            "artwork" => reviewed["cover"]["url"] = json!(format!("{host}/unreturned.png")),
            _ => panic!("unknown oracle mutation"),
        }
    }
    for (i, path) in paths[..2].iter().enumerate() {
        let title = if scenario == "multi" {
            if i == 0 {
                "First Saved"
            } else {
                "Second Saved"
            }
        } else {
            "Scalar Saved"
        };
        assert_eq!(
            reviewed["items"][i],
            json!({"path":path.to_string_lossy(),"updates":{"TITLE":[title],"ALBUM":["Replay Album"],"COMMENT":["Saved setting reached the writer"]}}),
            "review matching/selected-field oracle"
        );
    }
    assert_eq!(reviewed["items"].as_array().unwrap().len(), 2);
    assert_eq!(
        reviewed["cover"],
        json!({"url":cover_url,"replace_existing":true,"save_as":"folder"}),
        "review artwork/session-provenance oracle"
    );
    let items: Vec<crate::sources::ApplyItem> =
        serde_json::from_value(reviewed["items"].clone()).unwrap();
    let cover: crate::sources::ApplyCover =
        serde_json::from_value(reviewed["cover"].clone()).unwrap();
    let image = http::Image::validate(image_bytes.clone()).unwrap();
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
    assert_eq!(report.result.entries.len(), 2);
    let written: Vec<_> = paths.iter().map(|p| std::fs::read(p).unwrap()).collect();
    for (i, path) in paths[..2].iter().enumerate() {
        let entry = tag_core::read_entry(path).unwrap();
        assert_eq!(
            entry.fields["TITLE"],
            serde_json::from_value::<Vec<String>>(reviewed["items"][i]["updates"]["TITLE"].clone())
                .unwrap()
        );
        assert_eq!(entry.fields["ARTIST"], ["Keep original artist"]);
        assert_eq!(entry.fields["ALBUM"], ["Replay Album"]);
        assert_eq!(
            entry.fields["COMMENT"],
            ["Saved setting reached the writer"]
        );
        assert_eq!(entry.fields["TRACK"], [(i + 1).to_string()]);
        assert_eq!(entry.covers.len(), 1);
        assert_eq!(
            base64::engine::general_purpose::STANDARD
                .decode(&entry.covers[0].data_base64)
                .unwrap(),
            image_bytes
        );
    }
    assert_eq!(
        written[2], originals[2],
        "unmatched/unselected file changed"
    );
    assert_eq!(std::fs::read(&saved_cover).unwrap(), image_bytes);
    let history = serde_json::to_value(crate::history::history_state().unwrap()).unwrap();
    assert_eq!(history["undo"]["label"], "Tag source import");
    assert_eq!(history["undo"]["files"], 2);
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
    println!(
        "WORKFLOW_RECEIPT {}",
        json!({"scenario":scenario,"independent_fixture":true,"matched":2,"untouched":1,"network_requests":reviewed["network_requests"],"history":{"label":history["undo"]["label"],"files":history["undo"]["files"]},"audio_sha256":{"before":hashes(&originals),"saved":hashes(&written),"undo":hashes(&undone),"redo":hashes(&redone)},"saved_cover_sha256":format!("{:x}",Sha256::digest(&image_bytes))})
    );
    assert_eq!(
        serde_json::to_value(tauri::async_runtime::block_on(crate::history::undo_last()).unwrap())
            .unwrap()["errors"],
        json!([])
    );
    crate::history::TEST_ROOTS
        .lock()
        .unwrap()
        .retain(|p| p != &root);
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
