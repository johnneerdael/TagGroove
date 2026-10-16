//! Run written web-source examples with the application's actual pure parser.
// These modules perform no networking or application startup. Keep the app's
// own source in place so documentation checks exercise its real implementation.
#[allow(dead_code)]
#[path = "../../../src-tauri/src/sources/web/source.rs"]
mod source_definition;
#[allow(dead_code)]
#[path = "../../../src-tauri/src/sources/web/engine.rs"]
mod web_engine;

mod guide_examples;
use guide_examples::{fenced_blocks, text_blocks};

const TUTORIAL: &str = include_str!("../../../docs/user-guide/tag-sources/writing-web-sources.md");
const REFERENCE: &str =
    include_str!("../../../docs/user-guide/tag-sources/web-source-reference.md");

fn section<'a>(page: &'a str, name: &str) -> &'a str {
    page.split(&format!("<!-- {name}:start -->"))
        .nth(1)
        .unwrap()
        .split(&format!("<!-- {name}:end -->"))
        .next()
        .unwrap()
}

#[test]
fn source_lesson_runs_its_actual_definition_response_settings_and_written_buffers() {
    let lf = TUTORIAL.replace("\r\n", "\n");
    for page in [lf.clone(), lf.replace('\n', "\r\n")] {
        let definition = fenced_blocks(section(&page, "tutorial-source"), "ini").remove(0);
        let response = fenced_blocks(section(&page, "tutorial-release-json"), "json").remove(0);
        let schema = fenced_blocks(section(&page, "tutorial-settings"), "json").remove(0);
        let schema: serde_json::Value = serde_json::from_str(&schema).unwrap();
        let file = source_definition::load("Practice#&Harbour.src", &|name| {
            assert_eq!(name, "Practice#&Harbour.src");
            Ok(definition.clone())
        })
        .unwrap();
        assert!(file.direct());
        assert_eq!(file.settings.as_deref(), Some("Practice#Settings.settings"));
        let url = file.direct_url(&[file.search_by[0].format.clone()]);
        assert_eq!(url, "http://127.0.0.1:8766/release/harbour.json");
        assert!(page.contains(&url));
        let script = web_engine::Script::compile(&file.album_script.unwrap().lines).unwrap();
        let setting = &schema["settings"][0];
        assert_eq!(setting["type"], "bool");
        let mut environment = web_engine::Environment {
            current_url: &url,
            ..Default::default()
        };
        environment.vars.insert(
            setting["key"].as_str().unwrap().into(),
            setting["default"].to_string(),
        );
        let result = web_engine::run(&script, &response, &environment)
            .result
            .unwrap();
        let mut count = 0;
        for line in section(&page, "tutorial-buffer-results")
            .lines()
            .filter(|line| line.starts_with("- **"))
        {
            let parts: Vec<_> = line.split("**").collect();
            let expected = parts[2]
                .trim()
                .strip_prefix(": `")
                .unwrap()
                .strip_suffix('`')
                .unwrap();
            assert_eq!(result[parts[1]], expected, "{line}");
            count += 1;
        }
        assert_eq!(count, 4);
        assert!(!result.contains_key("YEAR"));
        environment
            .vars
            .insert(setting["key"].as_str().unwrap().into(), "true".into());
        let result = web_engine::run(&script, &response, &environment)
            .result
            .unwrap();
        let year_line = page
            .lines()
            .find(|line| line.contains("The `YEAR` output should now be"))
            .unwrap();
        let year: Vec<_> = year_line.split('`').collect();
        assert_eq!(result[year[1]], year[3]);
    }
}

fn labelled<'a>(page: &'a str, label: &str) -> Option<&'a str> {
    page.lines().find_map(|line| {
        line.strip_prefix(&format!("**{label}:** `"))
            .and_then(|value| value.strip_suffix('`'))
    })
}

#[test]
fn index_extension_builds_the_written_search_row_and_keeps_the_album_parser() {
    let definition = fenced_blocks(section(TUTORIAL, "tutorial-source"), "ini").remove(0);
    let addition = fenced_blocks(section(TUTORIAL, "tutorial-index-source"), "ini").remove(0);
    let response = fenced_blocks(section(TUTORIAL, "tutorial-index-json"), "json").remove(0);
    let retained = definition
        .lines()
        .filter(|line| !line.starts_with("[AlbumUrl]=") && !line.starts_with("[SearchBy]="))
        .collect::<Vec<_>>()
        .join("\n");
    let source = format!("{retained}\n{addition}");
    let file = source_definition::load("Practice#&Harbour.src", &|_| Ok(source.clone())).unwrap();
    assert!(!file.direct());
    assert!(file.album_script.is_some());
    assert_eq!(file.index_fields(), ["_url", "Album", "Year"]);
    assert_eq!(
        file.search_url(&["Harbour Lights".into()]),
        "http://127.0.0.1:8766/search.json?q=Harbour%20Lights"
    );
    let script = web_engine::Script::compile(&file.index_script.unwrap().lines).unwrap();
    let result = web_engine::run(&script, &response, &web_engine::Environment::default())
        .result
        .unwrap();
    assert_eq!(
        result[web_engine::DEFAULT_OUTPUT],
        format!("{}\r\n", labelled(TUTORIAL, "Index row").unwrap())
    );
    assert_eq!(
        format!(
            "{}{}",
            file.album_url,
            result[web_engine::DEFAULT_OUTPUT]
                .split('|')
                .next()
                .unwrap()
        ),
        "http://127.0.0.1:8766/release/harbour.json"
    );
}

#[test]
fn command_reference_index_matches_the_actual_parser_registry() {
    let mut written: Vec<_> = REFERENCE
        .lines()
        .filter_map(|line| {
            line.strip_prefix("### `")
                .and_then(|line| line.strip_suffix('`'))
        })
        .map(str::to_ascii_lowercase)
        .collect();
    let mut actual: Vec<_> = web_engine::command_names().map(str::to_owned).collect();
    written.sort();
    actual.sort();
    assert_eq!(written, actual);
    assert_eq!(written.len(), 59);
}

#[test]
fn every_command_example_runs_its_written_input_script_and_output_under_lf_and_crlf() {
    let page_url = "https://catalogue.example.invalid/release/harbour";
    assert!(REFERENCE.contains(page_url));
    let examples: Vec<_> = REFERENCE.split("<!-- web-example:").skip(1).collect();
    assert_eq!(examples.len(), 59);
    for example in examples {
        let (name, body) = example.split_once("-->").unwrap();
        let blocks = text_blocks(body);
        assert_eq!(blocks.len(), 3, "{name}");
        let buffer = match labelled(body, "Output buffer").unwrap() {
            "unnamed" => "",
            buffer => buffer,
        };
        let decode_spaces = |text: &str| match labelled(body, "Visible spaces") {
            Some("␠") => text.replace('␠', " "),
            None => text.to_owned(),
            other => panic!("unknown visible-space notation: {other:?}"),
        };
        let expected = decode_spaces(&blocks[2]);
        let expected = if labelled(body, "Line endings") == Some("CRLF") {
            expected.replace('\n', "\r\n")
        } else {
            expected
        };
        for newline in ["\n", "\r\n"] {
            let input = decode_spaces(&blocks[0]).replace('\n', newline);
            let script = blocks[1].replace('\n', newline);
            let lines = script
                .lines()
                .enumerate()
                .map(|(index, line)| (index + 1, line.to_owned()))
                .collect::<Vec<_>>();
            let script = web_engine::Script::compile(&lines).unwrap();
            let mut environment = web_engine::Environment {
                current_url: page_url,
                ..Default::default()
            };
            if let Some(settings) = labelled(body, "Settings") {
                for setting in settings.split(',') {
                    let (key, value) = setting.split_once('=').unwrap();
                    environment.vars.insert(key.into(), value.into());
                }
            }
            let outcome = web_engine::run(&script, &input, &environment);
            let output = outcome
                .result
                .unwrap_or_else(|error| panic!("{name}: {error}"));
            assert_eq!(
                output.get(buffer).map(String::as_str).unwrap_or_default(),
                expected,
                "{name}"
            );
            if let Some(filename) = labelled(body, "Debug file") {
                let file = outcome
                    .files
                    .iter()
                    .find(|file| file.name == filename)
                    .unwrap();
                if filename == "snapshot.txt" {
                    assert_eq!(file.content, expected);
                } else {
                    assert!(file.content.contains("SayRest"));
                }
            }
        }
    }
}
