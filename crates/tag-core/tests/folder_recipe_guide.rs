//! Written folder recipes: actual core expressions and actual pure host path helper.
//! No native app, filesystem moves, collision/alias or overwrite qualification.
mod guide_examples;
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};
use tag_core::{evaluate_filename_with_context, EvalContext, Fields};
const ARTIST: &str =
    include_str!("../../../docs/user-guide/converters/recipes/rename-into-artist-album-folders.md");
const DISC: &str =
    include_str!("../../../docs/user-guide/converters/recipes/rename-multi-disc-albums.md");
const COMPILATION: &str =
    include_str!("../../../docs/user-guide/converters/recipes/rename-compilations.md");
const HOST: &str = include_str!("../../../src-tauri/src/lib.rs");
fn forms(page: &str) -> [String; 2] {
    let lf = page.replace("\r\n", "\n");
    [lf.clone(), lf.replace('\n', "\r\n")]
}
fn rows<'a>(page: &'a str, marker: &str) -> Vec<Vec<&'a str>> {
    page.split(&format!("<!-- {marker}:start -->"))
        .nth(1)
        .unwrap()
        .split(&format!("<!-- {marker}:end -->"))
        .next()
        .unwrap()
        .lines()
        .map(str::trim_start)
        .filter(|line| line.starts_with('|'))
        .skip(2)
        .map(|line| line.trim_matches('|').split('|').map(str::trim).collect())
        .collect()
}
fn code(value: &str) -> &str {
    value.strip_prefix('`').unwrap().strip_suffix('`').unwrap()
}
fn marked(page: &str, marker: &str) -> String {
    page.split(&format!("<!-- {marker} -->"))
        .nth(1)
        .unwrap()
        .split('`')
        .nth(1)
        .unwrap()
        .into()
}
#[derive(Clone)]
struct Case {
    fields: Fields,
    source: String,
    expected: String,
    template: String,
}
fn entry(row: &[&str], keys: &[&str], base: &[(&str, &str)], template: &str) -> Case {
    let mut fields: Fields = base
        .iter()
        .map(|(key, value)| ((*key).into(), vec![(*value).into()]))
        .collect();
    for (key, value) in keys.iter().zip(row) {
        if *value != "Absent" {
            fields.insert((*key).into(), vec![code(value).into()]);
        }
    }
    Case {
        fields,
        source: code(row[keys.len()]).into(),
        expected: code(row[keys.len() + 1]).into(),
        template: template.into(),
    }
}
fn artist_cases(page: &str) -> Vec<Case> {
    let patterns = guide_examples::text_blocks(page);
    assert_eq!(patterns.len(), 2);
    let cases = rows(page, "artist-folder-cases");
    assert_eq!(cases.len(), 5);
    let mut cases: Vec<_> = cases
        .into_iter()
        .map(|row| {
            entry(
                &row,
                &["ARTIST", "ALBUM", "YEAR", "TRACK", "TITLE"],
                &[],
                &patterns[0],
            )
        })
        .collect();
    let mut variant = cases[0].clone();
    variant
        .fields
        .insert("ALBUMARTIST".into(), vec!["Various Artists".into()]);
    variant.template = patterns[1].clone();
    variant.expected = marked(page, "artist-folder-albumartist-result");
    cases.push(variant);
    cases
}
fn disc_cases(page: &str) -> Vec<Case> {
    let patterns = guide_examples::text_blocks(page);
    assert_eq!(patterns.len(), 3);
    let cases = rows(page, "disc-folder-cases");
    assert_eq!(cases.len(), 4);
    let mut cases: Vec<_> = cases
        .into_iter()
        .map(|row| {
            entry(
                &row,
                &["DISCNUMBER", "TRACK", "TITLE"],
                &[("ALBUM", "Harbour Lights")],
                &patterns[0],
            )
        })
        .collect();
    for (index, marker, pattern) in [
        (3, "disc-folder-no-disc-result", 1),
        (1, "disc-folder-with-disc-result", 1),
        (1, "disc-folder-flat-result", 2),
    ] {
        let mut case = cases[index].clone();
        case.template = patterns[pattern].clone();
        case.expected = marked(page, marker);
        cases.push(case);
    }
    cases
}
fn compilation_cases(page: &str) -> Vec<Case> {
    let patterns = guide_examples::text_blocks(page);
    assert_eq!(patterns.len(), 3);
    let cases = rows(page, "compilation-folder-cases");
    assert_eq!(cases.len(), 5);
    let mut cases: Vec<_> = cases
        .into_iter()
        .map(|row| {
            entry(
                &row,
                &["ARTIST", "ALBUMARTIST", "COMPILATION", "TRACK", "TITLE"],
                &[("ALBUM", "Harbour Sessions")],
                &patterns[0],
            )
        })
        .collect();
    for (marker, pattern) in [
        ("compilation-folder-fixed-result", 1),
        ("compilation-folder-disc-result", 2),
    ] {
        let mut case = cases[0].clone();
        case.template = patterns[pattern].clone();
        case.expected = marked(page, marker);
        if pattern == 2 {
            case.fields.insert("DISCNUMBER".into(), vec!["2/2".into()]);
        }
        cases.push(case);
    }
    cases
}
fn render(case: &Case) -> String {
    let mut context = EvalContext::new(&case.fields);
    context.folder_separator = Some("_");
    evaluate_filename_with_context(&case.template, &context).unwrap()
}
fn check_core(cases: &[Case]) {
    for case in cases {
        let ext = Path::new(&case.source)
            .extension()
            .unwrap()
            .to_str()
            .unwrap();
        let expected = case
            .expected
            .strip_suffix(&format!(".{ext}"))
            .unwrap()
            .replace('/', "\\");
        assert_eq!(
            render(case),
            expected,
            "{} {:?}",
            case.template,
            case.fields
        );
    }
}
#[test]
fn artist_album_optional_year_and_value_sanitization_follow_written_cases() {
    for page in forms(ARTIST) {
        check_core(&artist_cases(&page));
    }
}
#[test]
fn disc_folders_and_conditional_literal_separators_follow_written_cases() {
    for page in forms(DISC) {
        let cases = disc_cases(&page);
        check_core(&cases);
        let mut zero = cases[4].clone();
        zero.fields.insert("DISCNUMBER".into(), vec!["0".into()]);
        assert_eq!(render(&zero), "Harbour Lights\\CD00\\01 - Intro");
    }
}
#[test]
fn compilation_albumartist_fallback_and_explicit_credits_follow_written_cases() {
    for page in forms(COMPILATION) {
        check_core(&compilation_cases(&page));
    }
}
struct HostTarget {
    _temp: tempfile::TempDir,
    binary: PathBuf,
}
impl HostTarget {
    fn compile() -> Self {
        // Extract the actual converter helper and its pure dependency. Adjacent
        // inline/native helpers are a separate policy and do not belong here.
        // Compile only std and a minimal path DTO; no Tauri app or file moves.
        let source = HOST.replace("\r\n", "\n");
        let helper = ["filename_target", "resolve_filename_target"]
            .into_iter()
            .map(|name| {
                let start = source.find(&format!("fn {name}(")).unwrap();
                let rest = &source[start..];
                let end = rest.find("\n}\n").unwrap() + 2;
                &rest[..end]
            })
            .collect::<Vec<_>>()
            .join("\n");
        let temp = tempfile::tempdir().unwrap();
        let file = temp.path().join("filename_host.rs");
        let binary = temp.path().join(if cfg!(windows) {
            "filename_host.exe"
        } else {
            "filename_host"
        });
        let wrapper=format!("use std::path::{{Path,PathBuf}};\nstruct AudioEntry{{path:String}}\n{helper}\nfn main(){{let mut args=std::env::args().skip(1);let entry=AudioEntry{{path:args.next().unwrap()}};match filename_target(&entry,args.next().unwrap()){{Ok(path)=>println!(\"{{}}\",path.display()),Err(error)=>{{eprintln!(\"{{}}\",error);std::process::exit(2)}}}}}}\n");
        fs::write(&file, wrapper).unwrap();
        let compiler = std::env::var_os("RUSTC").unwrap_or_else(|| "rustc".into());
        let result = Command::new(compiler)
            .args(["--edition=2021", "-Dwarnings"])
            .arg(&file)
            .arg("-o")
            .arg(&binary)
            .output()
            .unwrap();
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
        Self {
            _temp: temp,
            binary,
        }
    }
    fn run(&self, source: &Path, stem: &str) -> Result<PathBuf, String> {
        let result = Command::new(&self.binary)
            .arg(source)
            .arg(stem)
            .output()
            .unwrap();
        if result.status.success() {
            Ok(PathBuf::from(
                String::from_utf8(result.stdout)
                    .unwrap()
                    .trim_end_matches(['\n', '\r']),
            ))
        } else {
            Err(String::from_utf8(result.stderr).unwrap())
        }
    }
}
#[test]
fn actual_pure_host_helper_supplies_written_relative_roots_and_source_extensions() {
    let host = HostTarget::compile();
    let root = std::env::temp_dir().join("opentag-guide-incoming");
    for (page, make) in [
        (ARTIST, artist_cases as fn(&str) -> Vec<Case>),
        (DISC, disc_cases),
        (COMPILATION, compilation_cases),
    ] {
        for page in forms(page) {
            for case in make(&page) {
                let source = root.join(&case.source);
                let expected = root.join(case.expected.replace('/', std::path::MAIN_SEPARATOR_STR));
                assert_eq!(host.run(&source, &render(&case)).unwrap(), expected);
            }
        }
    }
    let mut missing_artist = artist_cases(ARTIST)[0].clone();
    missing_artist.fields.remove("ARTIST");
    let unexpected = host
        .run(&root.join(&missing_artist.source), &render(&missing_artist))
        .unwrap();
    assert!(
        !unexpected.starts_with(&root),
        "A missing first component changes the root; this recipe forbids applying it"
    );
    let source = root.join("old.mp3");
    assert_eq!(
        host.run(&source, "Same.MP3").unwrap(),
        root.join("Same.MP3")
    );
    assert!(host
        .run(&source, "   ")
        .unwrap_err()
        .contains("empty string"));
}
