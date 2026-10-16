//! Headless check of third-party web source scripts through the real loader,
//! interpreter and release conversion. It is ignored by default because it
//! downloads pages; nothing in the repository depends on such scripts.
//!
//! ```text
//! OPENTAG_SRC_DIR=<folder holding the scripts> OPENTAG_SRC=<file.src> \
//! OPENTAG_SRC_SEARCH="artist||album"          # one value per [SearchBy] field
//! OPENTAG_SRC_FIELDS='{"ARTIST":"x"}'          # or: fill the fields from these tags
//! OPENTAG_SRC_INDEX_PAGE=<saved page>          # optional: no download for the index
//! OPENTAG_SRC_ALBUM_PAGE=<saved page>          # optional: no download for the album
//!                                              # (alone: the index step is skipped)
//! OPENTAG_SRC_PAGE_URL=<address of saved pages>
//! OPENTAG_SRC_PICK=<result number, default 1>
//! OPENTAG_SRC_DEBUG=1                          # write traces to <folder>/debug
//! cargo test -p opentag --lib sources::web::compat -- --ignored --nocapture
//! ```
use super::*;

fn env(name: &str) -> Option<String> {
    std::env::var(name).ok().filter(|v| !v.is_empty())
}

fn page(file: &SourceFile, saved: Option<String>, address: &str) -> http::Page {
    if let Some(path) = saved {
        let bytes = std::fs::read(&path).unwrap_or_else(|e| panic!("{path}: {e}"));
        let url = env("OPENTAG_SRC_PAGE_URL").unwrap_or_else(|| address.to_owned());
        return http::Page {
            text: http::decode_text(&bytes, None),
            url: Url::parse(&url).expect("page address"),
        };
    }
    println!("GET {address}");
    tauri::async_runtime::block_on(http::web_page(
        &file.name,
        address,
        user_agent(file),
        Channel::Search,
        u64::MAX,
    ))
    .unwrap_or_else(|e| panic!("download failed: {e}"))
}

/// The search words: given, or the [SearchBy] formats evaluated for tags.
fn search_values(file: &SourceFile) -> Vec<String> {
    if let Some(given) = env("OPENTAG_SRC_SEARCH") {
        return given.split("||").map(str::to_owned).collect();
    }
    let tags: Map<String, Value> = env("OPENTAG_SRC_FIELDS")
        .map(|json| serde_json::from_str(&json).expect("OPENTAG_SRC_FIELDS is a JSON object"))
        .unwrap_or_default();
    let mut fields = Fields::new();
    for (name, value) in tags {
        let text = match value {
            Value::String(text) => text,
            other => other.to_string(),
        };
        fields.insert(name.to_uppercase(), vec![text]);
    }
    file.search_by
        .iter()
        .map(|field| {
            match tag_core::CompiledTemplate::compile(&field.format)
                .and_then(|template| template.evaluate(&fields))
            {
                Ok(value) => value,
                Err(error) => {
                    println!("SEARCHBY FORMAT ERROR ({}): {error}", field.label);
                    String::new()
                }
            }
        })
        .collect()
}

#[test]
#[ignore = "downloads pages; run by hand with OPENTAG_SRC_DIR and OPENTAG_SRC"]
fn community_script() {
    let (Some(dir), Some(id)) = (env("OPENTAG_SRC_DIR"), env("OPENTAG_SRC")) else {
        println!("set OPENTAG_SRC_DIR and OPENTAG_SRC");
        return;
    };
    let folder = PathBuf::from(dir);
    let debug = env("OPENTAG_SRC_DEBUG").is_some();
    let listing = list(&folder);
    for problem in &listing.problems {
        println!("PROBLEM {}: {}", problem.file, problem.error);
    }
    let file = load(&folder, &id).unwrap_or_else(|e| panic!("load: {e}"));
    let vars = vars(&folder, &file).unwrap_or_else(|e| panic!("settings: {e}"));
    println!(
        "SOURCE {} (direct: {}, settings: {:?}, unknown keys: {:?})",
        file.name,
        file.direct(),
        file.settings,
        file.unknown_keys
    );
    for field in &file.search_by {
        println!("  search field {:?} -> {:?}", field.label, field.template);
    }
    let values = search_values(&file);
    println!("SEARCH {values:?}");
    let album_only =
        env("OPENTAG_SRC_ALBUM_PAGE").is_some() && env("OPENTAG_SRC_INDEX_PAGE").is_none();
    let album_address = if file.direct() || album_only {
        file.direct_url(&values)
    } else {
        let address = file.search_url(&values);
        println!("INDEX URL {address}");
        let index = page(&file, env("OPENTAG_SRC_INDEX_PAGE"), &address);
        let block = file.index_script.as_ref().expect("index script");
        let outputs = execute(&folder, &id, block, "index", &index, vars.clone(), debug)
            .unwrap_or_else(|e| panic!("index script: {e}"));
        let found = index_results(
            &file,
            outputs
                .get(engine::DEFAULT_OUTPUT)
                .map(String::as_str)
                .unwrap_or_default(),
            &index.url,
        )
        .unwrap_or_else(|e| panic!("results: {e}"));
        println!(
            "RESULTS {} columns {:?}",
            found.results.len(),
            found.columns
        );
        for (n, result) in found.results.iter().take(5).enumerate() {
            println!(
                "  {} {} {:?} cover={:?}",
                n + 1,
                result.url,
                result.values,
                result.cover
            );
        }
        let pick: usize = env("OPENTAG_SRC_PICK")
            .and_then(|n| n.parse().ok())
            .unwrap_or(1);
        match found.results.get(pick.saturating_sub(1)) {
            Some(result) => result.url.clone(),
            None if env("OPENTAG_SRC_ALBUM_PAGE").is_some() => address,
            None => panic!("no search results"),
        }
    };
    println!("ALBUM URL {album_address}");
    let album = page(&file, env("OPENTAG_SRC_ALBUM_PAGE"), &album_address);
    let block = file.album_script.as_ref().expect("album script");
    let outputs = execute(&folder, &id, block, "album", &album, vars, debug)
        .unwrap_or_else(|e| panic!("album script: {e}"));
    let release = release_from(outputs, &album.url, &album_address);
    println!("RELEASE {:?} by {:?}", release.title, release.artist);
    for (name, values) in &release.fields {
        println!("  {name} = {values:?}");
    }
    for cover in &release.covers {
        println!("  cover {}", cover.url);
    }
    for track in &release.tracks {
        println!(
            "  track {} disc {} {:?} / {:?} length {:?}",
            track.number, track.disc, track.title, track.artist, track.length
        );
        for (name, values) in &track.fields {
            if !matches!(name.as_str(), "TITLE" | "ARTIST") {
                println!("      {name} = {values:?}");
            }
        }
    }
}
