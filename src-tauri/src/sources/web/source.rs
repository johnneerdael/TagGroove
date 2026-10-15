//! Web source description files: `[Key]=value` lines, multi-line parser
//! scripts introduced by `[Key]=...`, and `[Include]` files whose keys
//! replace those of the including file.
use std::collections::BTreeMap;

/// Includes nested deeper than this are reported as a loop.
const MAX_INCLUDE_DEPTH: usize = 8;

/// A parser script with the file and line numbers it came from.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Block {
    pub file: String,
    pub lines: Vec<(usize, String)>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum Entry {
    Value(String),
    Script(Block),
}

/// How search words are written into addresses (`[Encoding]`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Encoding {
    /// Percent-encoded UTF-8 (`url-utf-8`), also used when the key is absent.
    #[default]
    UrlUtf8,
    /// Percent-encoded ISO-8859-1 (`url`).
    Url,
    /// UTF-8 as typed (`utf-8`).
    Utf8,
    /// ISO-8859-1 as typed (`iso-8859-1`).
    Latin1,
    /// The Windows ANSI code page (`ansi`); Windows-1252 everywhere.
    Ansi,
}

/// One search field of `[SearchBy]`: its label, the format string that fills
/// it from the first selected file, and the address part `%s` is placed in.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub struct SearchField {
    pub label: String,
    pub format: String,
    #[serde(skip)]
    pub template: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SourceFile {
    pub name: String,
    pub based_on: String,
    pub index_url: String,
    pub album_url: String,
    pub word_separator: String,
    pub index_format: String,
    pub search_by: Vec<SearchField>,
    pub encoding: Encoding,
    pub user_agent: bool,
    pub index_script: Option<Block>,
    pub album_script: Option<Block>,
    pub settings: Option<String>,
    pub min_app_version_win: String,
    pub min_app_version_mac: String,
    /// Keys this interpreter does not use, for the debug output.
    pub unknown_keys: Vec<String>,
}

/// File bytes as text: a byte order mark decides, else UTF-8 when valid,
/// else Windows-1252 (files saved by older Windows editors).
pub fn decode(bytes: &[u8]) -> String {
    if let Some((encoding, skip)) = encoding_rs::Encoding::for_bom(bytes) {
        return encoding
            .decode_without_bom_handling(&bytes[skip..])
            .0
            .into_owned();
    }
    match std::str::from_utf8(bytes) {
        Ok(text) => text.to_owned(),
        Err(_) => encoding_rs::WINDOWS_1252.decode(bytes).0.into_owned(),
    }
}

fn header(line: &str) -> Option<(String, &str)> {
    let rest = line.strip_prefix('[')?;
    let close = rest.find("]=")?;
    let key = &rest[..close];
    (!key.is_empty() && key.chars().all(|c| c.is_ascii_alphanumeric() || c == '_'))
        .then(|| (key.to_ascii_lowercase(), &rest[close + 2..]))
}

fn entries(text: &str, file: &str) -> Vec<(String, Entry)> {
    let mut entries: Vec<(String, Entry)> = Vec::new();
    let mut block: Option<(String, Block)> = None;
    for (index, line) in text.lines().enumerate() {
        let line = line.strip_suffix('\r').unwrap_or(line);
        if let Some((key, value)) = header(line.trim_start()) {
            if let Some((key, block)) = block.take() {
                entries.push((key, Entry::Script(block)));
            }
            if value.trim() == "..." {
                block = Some((
                    key,
                    Block {
                        file: file.to_owned(),
                        lines: Vec::new(),
                    },
                ));
            } else {
                entries.push((key, Entry::Value(value.trim().to_owned())));
            }
        } else if let Some((_, block)) = block.as_mut() {
            block.lines.push((index + 1, line.to_owned()));
        }
        // Other lines outside a script (blank lines, notes) carry no meaning.
    }
    if let Some((key, block)) = block {
        entries.push((key, Entry::Script(block)));
    }
    entries
}

/// Resolve `file` and its includes. `read` returns the text of a file named
/// relative to the sources folder.
fn resolve(
    file: &str,
    read: &dyn Fn(&str) -> Result<String, String>,
    depth: usize,
) -> Result<BTreeMap<String, Entry>, String> {
    if depth > MAX_INCLUDE_DEPTH {
        return Err(format!(
            "{file}: [Include] files include each other in a loop"
        ));
    }
    let text = read(file)?;
    let mut keys = BTreeMap::new();
    let mut includes = Vec::new();
    for (key, entry) in entries(&text, file) {
        match (key.as_str(), entry) {
            ("include", Entry::Value(name)) if !name.is_empty() => includes.push(name),
            (_, entry) => {
                keys.insert(key, entry);
            }
        }
    }
    // The documented rule: keys of an included file replace the includer's.
    for name in includes {
        keys.extend(resolve(&name, read, depth + 1).map_err(|e| {
            if e.starts_with(&name) {
                e
            } else {
                format!("{file}: [Include]={name}: {e}")
            }
        })?);
    }
    Ok(keys)
}

/// `[SearchBy]`: up to three `Label||%format%||address part` triples joined
/// by `$$`. A value without `||` is a format string alone.
fn search_fields(value: &str) -> Vec<SearchField> {
    let value = value.trim();
    if value.is_empty() {
        return vec![SearchField {
            label: "Artist + Album".into(),
            format: "$if2(%albumartist%,%artist%) %album%".into(),
            template: "%s".into(),
        }];
    }
    value
        .split("$$")
        .filter(|part| !part.trim().is_empty())
        .take(3)
        .map(|part| {
            let pieces: Vec<&str> = part.split("||").collect();
            match pieces.as_slice() {
                [format] => SearchField {
                    label: "Search".into(),
                    format: format.trim().into(),
                    template: "%s".into(),
                },
                [label, format] => SearchField {
                    label: label.replace('&', "").trim().into(),
                    format: format.trim().into(),
                    template: "%s".into(),
                },
                [label, format, template, ..] => SearchField {
                    label: label.replace('&', "").trim().into(),
                    format: format.trim().into(),
                    template: (*template).into(),
                },
                [] => unreachable!("split yields at least one piece"),
            }
        })
        .collect()
}

fn encoding(value: &str) -> Result<Encoding, String> {
    Ok(match value.trim().to_ascii_lowercase().as_str() {
        "" | "url-utf-8" | "url-utf8" => Encoding::UrlUtf8,
        "url" => Encoding::Url,
        "utf-8" | "utf8" => Encoding::Utf8,
        "iso-8859-1" | "latin1" | "latin-1" => Encoding::Latin1,
        "ansi" => Encoding::Ansi,
        other => {
            return Err(format!(
                "[Encoding]={other} is not one of utf-8, iso-8859-1, url, url-utf-8 or ansi"
            ))
        }
    })
}

/// Load a description file through `read` (see [`resolve`]).
pub fn load(
    file: &str,
    read: &dyn Fn(&str) -> Result<String, String>,
) -> Result<SourceFile, String> {
    let mut keys = resolve(file, read, 0)?;
    let mut value = |key: &str| match keys.remove(key) {
        Some(Entry::Value(value)) => value,
        Some(Entry::Script(block)) => block
            .lines
            .iter()
            .map(|(_, line)| line.trim())
            .find(|line| !line.is_empty())
            .unwrap_or_default()
            .to_owned(),
        None => String::new(),
    };
    let stem = file.rsplit_once('.').map_or(file, |(stem, _)| stem);
    let name = Some(value("name"))
        .filter(|n| !n.is_empty())
        .unwrap_or_else(|| stem.rsplit('#').next().unwrap_or(stem).replace('&', ""));
    let mut source = SourceFile {
        name,
        based_on: value("basedon"),
        index_url: value("indexurl"),
        album_url: value("albumurl"),
        word_separator: {
            let separator = value("wordseparator");
            let misspelt = value("wordseperator");
            if separator.is_empty() {
                misspelt
            } else {
                separator
            }
        },
        index_format: value("indexformat"),
        search_by: search_fields(&value("searchby")),
        encoding: encoding(&value("encoding")).map_err(|e| format!("{file}: {e}"))?,
        user_agent: value("useragent").trim() == "1",
        settings: Some(value("settings")).filter(|s| !s.is_empty()),
        min_app_version_win: value("minappversionwin"),
        min_app_version_mac: value("minappversionmac"),
        ..SourceFile::default()
    };
    let mut script = |key: &str| match keys.remove(key) {
        Some(Entry::Script(block)) => Some(block),
        // A one-line script written directly after the key.
        Some(Entry::Value(line)) if !line.is_empty() => Some(Block {
            file: file.to_owned(),
            lines: vec![(0, line)],
        }),
        _ => None,
    };
    source.index_script = script("parserscriptindex");
    source.album_script = script("parserscriptalbum");
    source.unknown_keys = keys.into_keys().collect();
    if source.album_script.is_none() {
        return Err(format!("{file} has no [ParserScriptAlbum]"));
    }
    if source.album_url.is_empty() && source.index_url.is_empty() {
        return Err(format!("{file} has neither [IndexUrl] nor [AlbumUrl]"));
    }
    Ok(source)
}

/// Percent-encode `bytes`, keeping the unreserved characters.
fn percent(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|&b| {
            if b.is_ascii_alphanumeric() || b"-_.~".contains(&b) {
                (b as char).to_string()
            } else {
                format!("%{b:02X}")
            }
        })
        .collect()
}

/// A search word as `encoding` writes it into an address.
fn encode_word(word: &str, encoding: Encoding) -> String {
    let single_byte = |codec: &'static encoding_rs::Encoding| -> Vec<u8> {
        word.chars()
            .flat_map(|c| {
                let mut buffer = [0u8; 4];
                let (bytes, _, unmappable) = codec.encode(c.encode_utf8(&mut buffer));
                if unmappable {
                    vec![b'?']
                } else {
                    bytes.into_owned()
                }
            })
            .collect()
    };
    let latin1 = || -> Vec<u8> {
        word.chars()
            .map(|c| u8::try_from(u32::from(c)).unwrap_or(b'?'))
            .collect()
    };
    // Raw modes still escape the characters that would end the address part.
    let raw = |bytes: Vec<u8>| -> String {
        bytes
            .iter()
            .map(|&b| {
                if b.is_ascii() && !b"#%&+?".contains(&b) && !b.is_ascii_control() {
                    (b as char).to_string()
                } else {
                    format!("%{b:02X}")
                }
            })
            .collect()
    };
    match encoding {
        Encoding::UrlUtf8 => percent(word.as_bytes()),
        Encoding::Url => percent(&latin1()),
        Encoding::Utf8 => raw(word.as_bytes().to_vec()),
        Encoding::Latin1 => raw(latin1()),
        Encoding::Ansi => raw(single_byte(encoding_rs::WINDOWS_1252)),
    }
}

impl SourceFile {
    /// Searching goes straight to the album page when there is no index step.
    pub fn direct(&self) -> bool {
        self.index_url.trim().is_empty() || self.index_script.is_none()
    }

    /// The search words: each field's text encoded word by word, joined with
    /// `[WordSeparator]` (a space when absent) and placed in its address part.
    pub fn query(&self, values: &[String]) -> String {
        let separator = if self.word_separator.is_empty() {
            "%20"
        } else {
            self.word_separator.as_str()
        };
        self.search_by
            .iter()
            .enumerate()
            .map(|(index, field)| {
                let text = values.get(index).map(String::as_str).unwrap_or_default();
                let encoded = text
                    .split_whitespace()
                    .map(|word| encode_word(word, self.encoding))
                    .collect::<Vec<_>>()
                    .join(separator);
                field.template.replace("%s", &encoded)
            })
            .collect()
    }

    fn fill(template: &str, query: &str) -> String {
        if template.contains("%s") {
            template.replace("%s", query)
        } else {
            format!("{template}{query}")
        }
    }

    /// `[IndexUrl]` with the search words in place of `%s`.
    pub fn search_url(&self, values: &[String]) -> String {
        Self::fill(self.index_url.trim(), &self.query(values))
    }

    /// The album page of a source without an index step.
    pub fn direct_url(&self, values: &[String]) -> String {
        Self::fill(self.album_url.trim(), &self.query(values))
    }

    /// `[IndexFormat]` field names without their percent signs.
    pub fn index_fields(&self) -> Vec<String> {
        self.index_format
            .split('|')
            .map(|name| name.trim().trim_matches('%').to_owned())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn files(
        list: &'static [(&'static str, &'static str)],
    ) -> impl Fn(&str) -> Result<String, String> {
        move |name: &str| {
            list.iter()
                .find(|(file, _)| *file == name)
                .map(|(_, text)| text.to_string())
                .ok_or_else(|| format!("{name} was not found"))
        }
    }

    const BASIC: &str = "[Name]=Example\r\n[BasedOn]=https://example.org\n[IndexUrl]=https://example.org/search?q=%s\n[AlbumUrl]=https://example.org\n[WordSeperator]=+\n[IndexFormat]=%_url%|%Album%|%_coverurl%\n[SearchBy]=&Artist||%artist%||artist=%s$$Album||%album%||&album=%s\n[Encoding]=url-utf-8\n[UserAgent]=1\n[Custom]=x\n\n[ParserScriptIndex]=...\n# comment\nFindLine \"x\"\n\n[ParserScriptAlbum]=...\nOutputTo \"TITLE\"\nSayRest\n";

    #[test]
    fn keys_scripts_and_search_fields() {
        let source = load("Example.src", &files(&[("Example.src", BASIC)])).unwrap();
        assert_eq!(source.name, "Example");
        assert_eq!(source.based_on, "https://example.org");
        assert_eq!(source.word_separator, "+");
        assert!(source.user_agent);
        assert_eq!(source.encoding, Encoding::UrlUtf8);
        assert_eq!(source.unknown_keys, ["custom"]);
        assert_eq!(source.index_fields(), ["_url", "Album", "_coverurl"]);
        let index = source.index_script.as_ref().unwrap();
        assert_eq!(index.lines[1], (14, "FindLine \"x\"".to_string()));
        assert_eq!(source.album_script.as_ref().unwrap().lines.len(), 2);
        assert_eq!(
            source.search_by,
            [
                SearchField {
                    label: "Artist".into(),
                    format: "%artist%".into(),
                    template: "artist=%s".into()
                },
                SearchField {
                    label: "Album".into(),
                    format: "%album%".into(),
                    template: "&album=%s".into()
                },
            ]
        );
        assert!(!source.direct());
        assert_eq!(
            source.search_url(&["Daft Punk".into(), "Random Access & Memories".into()]),
            "https://example.org/search?q=artist=Daft+Punk&album=Random+Access+%26+Memories"
        );
    }

    #[test]
    fn includes_replace_keys_and_loops_are_reported() {
        let read = files(&[
            ("Main.src", "[Name]=Main\n[IndexUrl]=https://a/%s\n[Include]=Shared.inc\n[ParserScriptAlbum]=...\nSay \"main\"\n"),
            ("Shared.inc", "[IndexUrl]=https://b/%s\n[ParserScriptAlbum]=...\nSay \"shared\"\n"),
            ("Loop.src", "[Include]=Loop.src\n"),
            ("Missing.src", "[Include]=Gone.inc\n"),
        ]);
        let source = load("Main.src", &read).unwrap();
        assert_eq!(source.name, "Main");
        assert_eq!(source.index_url, "https://b/%s");
        let album = source.album_script.unwrap();
        assert_eq!(
            (album.file.as_str(), album.lines[0].1.as_str()),
            ("Shared.inc", "Say \"shared\"")
        );
        assert!(load("Loop.src", &read).unwrap_err().contains("loop"));
        assert!(load("Missing.src", &read)
            .unwrap_err()
            .contains("Gone.inc was not found"));
    }

    #[test]
    fn defaults_validation_and_direct_sources() {
        let read = files(&[
            ("Group#&Direct.src", "[AlbumUrl]=https://x.org/release/%s\n[ParserScriptAlbum]=...\nSayRest\n"),
            ("NoAlbum.src", "[IndexUrl]=https://x\n"),
            ("NoUrl.src", "[ParserScriptAlbum]=...\nSayRest\n"),
            ("BadEncoding.src", "[Encoding]=ebcdic\n[AlbumUrl]=x\n[ParserScriptAlbum]=...\nSayRest\n"),
            ("OneLine.src", "[IndexUrl]=https://x/?q=%s\n[SearchBy]=%title%\n[ParserScriptIndex]=SayRest\n[ParserScriptAlbum]=SayRest\n"),
        ]);
        let direct = load("Group#&Direct.src", &read).unwrap();
        assert_eq!(direct.name, "Direct");
        assert!(direct.direct());
        assert_eq!(
            direct.search_by[0].format,
            "$if2(%albumartist%,%artist%) %album%"
        );
        assert_eq!(
            direct.direct_url(&["12 34".into()]),
            "https://x.org/release/12%2034"
        );
        assert!(load("NoAlbum.src", &read)
            .unwrap_err()
            .contains("no [ParserScriptAlbum]"));
        assert!(load("NoUrl.src", &read).unwrap_err().contains("neither"));
        assert!(load("BadEncoding.src", &read)
            .unwrap_err()
            .contains("ebcdic"));
        let one = load("OneLine.src", &read).unwrap();
        assert_eq!(
            one.index_script.unwrap().lines,
            [(0, "SayRest".to_string())]
        );
        assert_eq!(
            (
                one.search_by[0].label.as_str(),
                one.search_by[0].format.as_str()
            ),
            ("Search", "%title%")
        );
    }

    #[test]
    fn words_are_encoded_per_encoding() {
        let word = "Beyoncé&Co";
        assert_eq!(encode_word(word, Encoding::UrlUtf8), "Beyonc%C3%A9%26Co");
        assert_eq!(encode_word(word, Encoding::Url), "Beyonc%E9%26Co");
        assert_eq!(encode_word(word, Encoding::Utf8), "Beyonc%C3%A9%26Co");
        assert_eq!(encode_word(word, Encoding::Latin1), "Beyonc%E9%26Co");
        assert_eq!(encode_word("€猫", Encoding::Ansi), "%80%3F");
        assert_eq!(encode_word("a/b", Encoding::Utf8), "a/b");
        assert_eq!(encode_word("a/b", Encoding::UrlUtf8), "a%2Fb");
        assert_eq!(encode_word("猫", Encoding::Url), "%3F");
    }

    #[test]
    fn files_decode_by_bom_or_fallback() {
        assert_eq!(decode(b"\xEF\xBB\xBF[Name]=x"), "[Name]=x");
        assert_eq!(decode(b"caf\xE9"), "café");
        assert_eq!(decode(&[0xFF, 0xFE, b'a', 0]), "a");
        assert_eq!(decode("ok ✓".as_bytes()), "ok ✓");
    }
}
