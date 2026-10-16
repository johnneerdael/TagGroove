//! freedb sources (Mp3tag's Tag Sources → freedb and freedb (local)): disc
//! IDs from track lengths, the CDDB protocol over HTTP (gnudb by default) and
//! folders of xmcd files from a freedb snapshot.
use super::model::*;
use reqwest::Url;
use std::path::Path;
use tag_core::Fields;

/// The eleven freedb categories; `cddb read` needs one of them.
pub const CATEGORIES: [&str; 11] = [
    "blues",
    "classical",
    "country",
    "data",
    "folk",
    "jazz",
    "misc",
    "newage",
    "reggae",
    "rock",
    "soundtrack",
];
const FRAMES_PER_SECOND: f64 = 75.0;
/// Audio CDs start two seconds (150 frames) into the disc.
const LEAD_IN: u32 = 150;
const MAX_ENTRY_BYTES: u64 = 1024 * 1024;

/// A table of contents in CD frames: track start offsets and the lead-out.
#[derive(Clone, Debug, PartialEq)]
pub struct Toc {
    pub offsets: Vec<u32>,
    pub leadout: u32,
}

impl Toc {
    /// Mp3tag's file-based lookup: tracks laid out back to back after the lead-in.
    pub fn from_lengths(lengths: &[f64]) -> Result<Toc, String> {
        if lengths.is_empty() {
            return Err("Select the files of one album to compute its disc ID.".into());
        }
        if lengths.len() > 99 {
            return Err("A CD holds at most 99 tracks. Select the files of one disc.".into());
        }
        if lengths.iter().any(|l| !l.is_finite() || *l <= 0.0) {
            return Err("Every selected file needs a known length to compute the disc ID.".into());
        }
        let mut offsets = Vec::with_capacity(lengths.len());
        let mut position = LEAD_IN;
        for length in lengths {
            offsets.push(position);
            let frames = (length * FRAMES_PER_SECOND).round().min(u32::MAX as f64) as u32;
            position = position.saturating_add(frames.max(1));
        }
        Ok(Toc {
            offsets,
            leadout: position,
        })
    }

    pub fn disc_id(&self) -> String {
        let sum: u32 = self.offsets.iter().map(|o| digit_sum(o / 75)).sum();
        let first = self.offsets.first().copied().unwrap_or(LEAD_IN);
        let seconds = (self.leadout / 75).saturating_sub(first / 75) & 0xffff;
        format!(
            "{:08x}",
            ((sum % 0xff) << 24) | (seconds << 8) | (self.offsets.len() as u32 & 0xff)
        )
    }

    pub fn query_command(&self) -> String {
        let offsets: Vec<String> = self.offsets.iter().map(u32::to_string).collect();
        format!(
            "cddb query {} {} {} {}",
            self.disc_id(),
            self.offsets.len(),
            offsets.join(" "),
            self.leadout / 75
        )
    }
}

fn digit_sum(mut n: u32) -> u32 {
    let mut sum = 0;
    while n > 0 {
        sum += n % 10;
        n /= 10;
    }
    sum
}

/// Accept `7a0a6b09`, `0x7A0A6B09` or `rock/7a0a6b09` and return the lower-case disc ID.
pub fn disc_id(input: &str) -> Result<String, String> {
    let input = input.trim();
    let bare = input.rsplit(['/', ' ']).next().unwrap_or(input);
    let bare = bare
        .strip_prefix("0x")
        .or_else(|| bare.strip_prefix("0X"))
        .unwrap_or(bare);
    if bare.len() == 8 && bare.chars().all(|c| c.is_ascii_hexdigit()) {
        Ok(bare.to_ascii_lowercase())
    } else {
        Err("Enter a freedb disc ID: eight hexadecimal digits, for example 7a0a6b09.".into())
    }
}

/// Release IDs are `category/discid`, as the search results return them.
pub fn entry_id(input: &str) -> Result<(String, String), String> {
    let (category, id) = input
        .trim()
        .split_once('/')
        .ok_or("Choose a freedb entry from the search results.")?;
    let category = category.trim().to_ascii_lowercase();
    if category.is_empty()
        || category.len() > 32
        || !category
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
    {
        return Err("This freedb category is not valid.".into());
    }
    Ok((category, disc_id(id)?))
}

/// Options → Tag Sources → freedb and → Local freedb.
#[derive(Clone, Debug, PartialEq)]
pub struct FreedbSettings {
    pub protocol: String,
    pub address: String,
    pub path: String,
    pub port: Option<u16>,
    pub email: String,
    pub comment_disc_id: bool,
    pub disc_id_field: bool,
    pub local_directory: String,
}
impl Default for FreedbSettings {
    fn default() -> Self {
        FreedbSettings {
            protocol: "https".into(),
            address: "gnudb.gnudb.org".into(),
            path: "/~cddb/cddb.cgi".into(),
            port: None,
            email: String::new(),
            comment_disc_id: false,
            disc_id_field: false,
            local_directory: String::new(),
        }
    }
}

pub fn settings_from(settings: &crate::settings::Settings) -> FreedbSettings {
    let defaults = FreedbSettings::default();
    let server = settings.pages.get("tag-sources-freedb");
    let local = settings.pages.get("tag-sources-local-freedb");
    let text = |page: Option<&serde_json::Value>, key: &str| {
        page.and_then(|p| p.get(key))
            .and_then(serde_json::Value::as_str)
            .map(|s| s.trim().to_owned())
            .filter(|s| !s.is_empty())
    };
    let flag = |key: &str| {
        server
            .and_then(|p| p.get(key))
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false)
    };
    FreedbSettings {
        protocol: match text(server, "protocol").as_deref() {
            Some("http") => "http".into(),
            _ => defaults.protocol,
        },
        address: text(server, "address").unwrap_or(defaults.address),
        path: text(server, "path").unwrap_or(defaults.path),
        port: text(server, "port").and_then(|p| p.parse().ok()),
        email: text(server, "email").unwrap_or_default(),
        comment_disc_id: flag("comment_disc_id"),
        disc_id_field: flag("disc_id_field"),
        local_directory: text(local, "directory").unwrap_or_default(),
    }
}

/// Display name for messages: gnudb when the default server is used.
pub fn server_name(settings: &FreedbSettings) -> &'static str {
    if settings.address.to_ascii_lowercase().ends_with("gnudb.org") {
        "gnudb"
    } else {
        "The freedb server"
    }
}

/// A CDDB command as an HTTP GET with the `hello` handshake and protocol 6 (UTF-8).
pub fn command_url(settings: &FreedbSettings, command: &str) -> Result<Url, String> {
    let invalid =
        || "The freedb server address is not valid. Check Options → Tag Sources → freedb.";
    let address = settings.address.trim();
    if address.is_empty()
        || !address
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-')
    {
        return Err(invalid().into());
    }
    let path = settings.path.trim();
    let path = if path.starts_with('/') {
        path.to_owned()
    } else {
        format!("/{path}")
    };
    let port = settings.port.map(|p| format!(":{p}")).unwrap_or_default();
    let mut url = Url::parse(&format!("{}://{address}{port}{path}", settings.protocol))
        .map_err(|_| invalid())?;
    if url.query().is_some() || url.fragment().is_some() {
        return Err(invalid().into());
    }
    let (user, host) = hello_identity(&settings.email);
    url.query_pairs_mut()
        .append_pair("cmd", command)
        .append_pair(
            "hello",
            &format!("{user} {host} OpenTag {}", env!("CARGO_PKG_VERSION")),
        )
        .append_pair("proto", "6");
    Ok(url)
}

/// The handshake takes the user and host of the configured email address.
fn hello_identity(email: &str) -> (String, String) {
    let clean = |value: &str| -> String {
        value
            .chars()
            .filter(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_'))
            .collect()
    };
    match email.trim().split_once('@') {
        Some((user, host)) if !clean(user).is_empty() && !clean(host).is_empty() => {
            (clean(user), clean(host))
        }
        _ => ("opentag".into(), "localhost".into()),
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Match {
    pub category: String,
    pub disc_id: String,
    pub artist: String,
    pub title: String,
}

fn status(body: &str) -> Result<(u16, &str, Vec<&str>), String> {
    let mut lines = body.lines().map(|line| line.trim_end_matches('\r'));
    let first = lines.next().unwrap_or_default();
    let code = first
        .get(..3)
        .and_then(|c| c.parse::<u16>().ok())
        .ok_or("The freedb server returned an unreadable response.")?;
    let rest: Vec<&str> = lines.take_while(|line| *line != ".").collect();
    Ok((code, first[3..].trim(), rest))
}

fn server_error(code: u16, value: &str) -> String {
    if value.to_ascii_lowercase().contains("unknown application") {
        return format!(
            "The freedb server does not accept requests from OpenTag (it answered “{value}”). gnudb only answers applications its operator has registered. Choose another CDDB server in Options → Tag Sources → freedb, or use a local freedb folder."
        );
    }
    match code {
        409 => "The freedb server rejected the handshake. Enter your email address in Options → Tag Sources → freedb.".into(),
        403 => "The freedb entry is damaged on the server.".into(),
        _ => format!("The freedb server answered: {code} {value}"),
    }
}

/// `cddb query`: 200 is one exact match, 210/211 list exact or inexact matches.
pub fn parse_query(body: &str) -> Result<Vec<Match>, String> {
    let (code, message, lines) = status(body)?;
    match code {
        200 => Ok(parse_match(message).into_iter().collect()),
        210 | 211 => Ok(lines.into_iter().filter_map(parse_match).collect()),
        202 => Ok(Vec::new()),
        _ => Err(server_error(code, message)),
    }
}

fn parse_match(line: &str) -> Option<Match> {
    let mut parts = line.trim().splitn(3, ' ');
    let category = parts.next()?.to_ascii_lowercase();
    let id = disc_id(parts.next()?).ok()?;
    let (artist, title) = split_title(parts.next().unwrap_or_default());
    (!category.is_empty() && category.chars().all(|c| c.is_ascii_alphanumeric())).then_some(Match {
        category,
        disc_id: id,
        artist,
        title,
    })
}

/// `cddb read`: the xmcd entry, or `None` when this category has no such disc.
pub fn parse_read(body: &str) -> Result<Option<String>, String> {
    let (code, message, lines) = status(body)?;
    match code {
        210 => Ok(Some(lines.join("\n"))),
        401 => Ok(None),
        _ => Err(server_error(code, message)),
    }
}

/// `Artist / Title`; without a separator artist and title are the same.
pub fn split_title(value: &str) -> (String, String) {
    match value.split_once(" / ") {
        Some((artist, title)) => (artist.trim().to_owned(), title.trim().to_owned()),
        None => (value.trim().to_owned(), value.trim().to_owned()),
    }
}

/// Parsed xmcd entry: keyword values (continuation lines joined) and the TOC comments.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Xmcd {
    values: Vec<(String, String)>,
    pub offsets: Vec<u32>,
    pub disc_length: Option<u32>,
}
impl Xmcd {
    pub fn get(&self, key: &str) -> &str {
        self.values
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.as_str())
            .unwrap_or_default()
    }
    pub fn track_count(&self) -> usize {
        self.values
            .iter()
            .filter_map(|(k, _)| k.strip_prefix("TTITLE")?.parse::<usize>().ok())
            .map(|n| n + 1)
            .max()
            .unwrap_or(0)
    }
}

pub fn parse_xmcd(text: &str) -> Xmcd {
    let mut xmcd = Xmcd::default();
    let mut in_offsets = false;
    for line in text.lines().map(|line| line.trim_end_matches('\r')) {
        if let Some(comment) = line.strip_prefix('#') {
            let comment = comment.trim();
            if comment.eq_ignore_ascii_case("Track frame offsets:") {
                in_offsets = true;
            } else if in_offsets && !comment.is_empty() && comment.parse::<u32>().is_ok() {
                xmcd.offsets.push(comment.parse().unwrap_or_default());
            } else {
                in_offsets = false;
                if let Some(rest) = comment.strip_prefix("Disc length:") {
                    xmcd.disc_length = rest.split_whitespace().next().and_then(|n| n.parse().ok());
                }
            }
            continue;
        }
        in_offsets = false;
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let key = key.trim().to_ascii_uppercase();
        let value = unescape(value);
        match xmcd.values.iter_mut().find(|(k, _)| *k == key) {
            Some((_, existing)) => existing.push_str(&value),
            None => xmcd.values.push((key, value)),
        }
    }
    xmcd
}

fn unescape(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    let mut chars = value.chars();
    while let Some(c) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match chars.next() {
            Some('n') => out.push('\n'),
            Some('t') => out.push('\t'),
            Some('\\') => out.push('\\'),
            Some(other) => {
                out.push('\\');
                out.push(other);
            }
            None => out.push('\\'),
        }
    }
    out
}

fn is_various(artist: &str) -> bool {
    matches!(
        artist.trim().to_ascii_lowercase().as_str(),
        "various"
            | "various artists"
            | "va"
            | "v.a."
            | "v. a."
            | "sampler"
            | "verschiedene"
            | "verschiedene interpreten"
            | "diverse"
            | "divers"
    )
}

/// Old rippers put `YEAR: 1998 ID3G: 17` into EXTD; keep the rest as a comment.
fn extended_data(extd: &str) -> (String, String) {
    let mut found_year = String::new();
    let mut words = Vec::new();
    let mut tokens = extd.split_whitespace().peekable();
    while let Some(token) = tokens.next() {
        match token {
            "YEAR:" => found_year = year(tokens.next().unwrap_or_default()),
            "ID3G:" => {
                tokens.next();
            }
            _ => words.push(token),
        }
    }
    let comment = if words.is_empty() {
        String::new()
    } else {
        extd.lines()
            .filter(|line| {
                let line = line.trim();
                !line.starts_with("YEAR:") && !line.starts_with("ID3G:")
            })
            .collect::<Vec<_>>()
            .join("\n")
            .trim()
            .to_owned()
    };
    (found_year, comment)
}

#[derive(Clone, Copy, Debug, Default)]
pub struct ImportOptions {
    pub comment_disc_id: bool,
    pub disc_id_field: bool,
}

pub fn release_url(provider: Provider, category: &str, disc_id: &str) -> String {
    match provider {
        Provider::Freedb => format!(
            "https://gnudb.org/cd/{}{disc_id}",
            category.get(..2).unwrap_or(category)
        ),
        _ => String::new(),
    }
}

/// Map an xmcd entry to OpenTag fields: DTITLE, DYEAR, DGENRE, TTITLEn and EXTD/EXTTn.
pub fn release(
    xmcd: &Xmcd,
    provider: Provider,
    category: &str,
    disc_id: &str,
    options: ImportOptions,
    server_is_gnudb: bool,
) -> Result<SourceRelease, String> {
    let count = xmcd.track_count();
    let dtitle = xmcd.get("DTITLE");
    if dtitle.trim().is_empty() && count == 0 {
        return Err("This freedb entry has no album information.".into());
    }
    let (artist, album) = split_title(dtitle);
    let titles: Vec<&str> = (0..count)
        .map(|i| xmcd.values.iter().find(|(k, _)| *k == format!("TTITLE{i}")))
        .map(|entry| entry.map(|(_, v)| v.as_str()).unwrap_or_default())
        .collect();
    let various =
        is_various(&artist) || (count > 1 && titles.iter().all(|title| title.contains(" / ")));
    let (extd_year, extd_comment) = extended_data(xmcd.get("EXTD"));

    let mut fields = Fields::new();
    put_one(&mut fields, "ALBUM", album.clone());
    put_one(
        &mut fields,
        if various { "ALBUMARTIST" } else { "ARTIST" },
        artist.clone(),
    );
    let dyear = year(xmcd.get("DYEAR"));
    put_one(
        &mut fields,
        "YEAR",
        if dyear.is_empty() { extd_year } else { dyear },
    );
    let genre = xmcd.get("DGENRE").trim();
    let genre = if genre.is_empty() && !matches!(category, "misc" | "data") {
        title_case(category)
    } else {
        genre.to_owned()
    };
    put_one(&mut fields, "GENRE", genre);
    if options.comment_disc_id {
        put_one(&mut fields, "COMMENT", disc_id);
    } else {
        put_one(&mut fields, "COMMENT", extd_comment);
    }
    if options.disc_id_field {
        put_one(&mut fields, "DISCID", disc_id);
    }

    let lengths = track_lengths(xmcd, count);
    let tracks = titles
        .iter()
        .enumerate()
        .map(|(index, raw)| {
            let (track_artist, title) = match raw.split_once(" / ") {
                Some((a, t)) if various => (a.trim().to_owned(), t.trim().to_owned()),
                _ => (String::new(), raw.trim().to_owned()),
            };
            let mut track_fields = Fields::new();
            put_one(&mut track_fields, "TITLE", title.clone());
            put_one(&mut track_fields, "ARTIST", track_artist.clone());
            put_one(&mut track_fields, "TRACK", format!("{}/{count}", index + 1));
            if !options.comment_disc_id {
                put_one(
                    &mut track_fields,
                    "COMMENT",
                    xmcd.get(&format!("EXTT{index}")).trim(),
                );
            }
            SourceTrack {
                position: (index + 1).to_string(),
                disc: 1,
                number: index as u32 + 1,
                title,
                artist: if track_artist.is_empty() {
                    artist.clone()
                } else {
                    track_artist
                },
                length: lengths.get(index).copied().flatten(),
                fields: track_fields,
            }
        })
        .collect();
    Ok(SourceRelease {
        provider,
        id: format!("{category}/{disc_id}"),
        url: if server_is_gnudb {
            release_url(provider, category, disc_id)
        } else {
            String::new()
        },
        title: album,
        artist,
        fields,
        tracks,
        covers: Vec::new(),
        more_covers: false,
    })
}

fn track_lengths(xmcd: &Xmcd, count: usize) -> Vec<Option<f64>> {
    let offsets = &xmcd.offsets;
    if offsets.len() != count {
        return vec![None; count];
    }
    let end = xmcd.disc_length.map(|seconds| seconds * 75);
    (0..count)
        .map(|i| {
            let next = offsets.get(i + 1).copied().or(end)?;
            next.checked_sub(offsets[i])
                .map(|frames| f64::from(frames) / FRAMES_PER_SECOND)
        })
        .collect()
}

pub fn search_result(release: &SourceRelease, category: &str) -> SearchResult {
    SearchResult {
        id: release.id.clone(),
        title: release.title.clone(),
        artist: release.artist.clone(),
        year: release
            .fields
            .get("YEAR")
            .and_then(|v| v.first())
            .cloned()
            .unwrap_or_default(),
        format: release
            .fields
            .get("GENRE")
            .and_then(|v| v.first())
            .cloned()
            .unwrap_or_else(|| category.to_owned()),
        track_count: Some(release.tracks.len() as u32),
        url: release.url.clone(),
        ..Default::default()
    }
}

pub fn match_result(found: &Match, provider: Provider, server_is_gnudb: bool) -> SearchResult {
    SearchResult {
        id: format!("{}/{}", found.category, found.disc_id),
        title: found.title.clone(),
        artist: found.artist.clone(),
        format: found.category.clone(),
        url: if server_is_gnudb {
            release_url(provider, &found.category, &found.disc_id)
        } else {
            String::new()
        },
        ..Default::default()
    }
}

/// xmcd files are UTF-8 since freedb 1.4; older entries are ISO-8859-1.
pub fn decode(bytes: &[u8]) -> String {
    match std::str::from_utf8(bytes) {
        Ok(text) => text.trim_start_matches('\u{feff}').to_owned(),
        Err(_) => encoding_rs::WINDOWS_1252.decode(bytes).0.into_owned(),
    }
}

pub fn check_local_directory(directory: &Path) -> Result<(), String> {
    if directory.as_os_str().is_empty() {
        return Err(
            "Choose your local freedb folder in Options → Tag Sources → Local freedb.".into(),
        );
    }
    if !directory.is_dir() {
        return Err(format!(
            "The local freedb folder {} does not exist. Check Options → Tag Sources → Local freedb.",
            directory.display()
        ));
    }
    Ok(())
}

/// Every entry with this disc ID in a local freedb folder: `<category>/<discid>`
/// files (Unix layout) or `<category>/<xx>to<yy>` files holding several
/// entries after `#FILENAME=<discid>` lines (Windows layout).
pub fn local_lookup(directory: &Path, disc_id: &str) -> Result<Vec<(String, String)>, String> {
    check_local_directory(directory)?;
    let prefix = u8::from_str_radix(&disc_id[..2], 16).map_err(|e| e.to_string())?;
    let mut categories: Vec<_> = std::fs::read_dir(directory)
        .map_err(|e| format!("The local freedb folder could not be read: {e}"))?
        .filter_map(Result::ok)
        .filter(|entry| entry.path().is_dir())
        .collect();
    categories.sort_by_key(|entry| entry.file_name());
    let mut found = Vec::new();
    for category in categories {
        let name = category.file_name().to_string_lossy().to_ascii_lowercase();
        let folder = category.path();
        let single = [disc_id.to_owned(), disc_id.to_ascii_uppercase()]
            .into_iter()
            .map(|file| folder.join(file))
            .find(|path| path.is_file());
        if let Some(path) = single {
            found.push((name, read_entry(&path)?));
            continue;
        }
        let Ok(files) = std::fs::read_dir(&folder) else {
            continue;
        };
        for file in files.filter_map(Result::ok) {
            let file_name = file.file_name().to_string_lossy().to_ascii_lowercase();
            if !covers_prefix(&file_name, prefix) {
                continue;
            }
            if let Some(entry) = windows_entry(&read_entry(&file.path())?, disc_id) {
                found.push((name.clone(), entry));
                break;
            }
        }
    }
    Ok(found)
}

fn read_entry(path: &Path) -> Result<String, String> {
    use std::io::Read;
    let file = std::fs::File::open(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut bytes = Vec::new();
    // Windows-layout files bundle many entries; read them whole up to a sane size.
    file.take(MAX_ENTRY_BYTES * 64)
        .read_to_end(&mut bytes)
        .map_err(|e| format!("{}: {e}", path.display()))?;
    Ok(decode(&bytes))
}

/// `a0toaf`-style names of the Windows layout: the first byte range of the disc IDs inside.
fn covers_prefix(name: &str, prefix: u8) -> bool {
    let Some((low, high)) = name.split_once("to") else {
        return false;
    };
    if low.len() != 2 || high.len() != 2 {
        return false;
    }
    match (u8::from_str_radix(low, 16), u8::from_str_radix(high, 16)) {
        (Ok(low), Ok(high)) => (low..=high).contains(&prefix),
        _ => false,
    }
}

fn windows_entry(text: &str, disc_id: &str) -> Option<String> {
    let mut inside = false;
    let mut entry = Vec::new();
    for line in text.lines() {
        if let Some(name) = line.trim().strip_prefix("#FILENAME=") {
            if inside {
                break;
            }
            inside = name.trim().eq_ignore_ascii_case(disc_id);
            continue;
        }
        if inside {
            entry.push(line);
        }
    }
    inside.then(|| entry.join("\n"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn server_errors_keep_remote_reply_text_as_data() {
        assert_eq!(
            server_error(400, "Help"),
            "The freedb server answered: 400 Help"
        );
        let reply = "unknown application: Help  Title";
        assert!(server_error(400, reply).contains(&format!("“{reply}”")));
    }

    const ENTRY: &str = include_str!("fixtures/freedb-entry.xmcd");

    /// libdiscid's reference disc: ten tracks, lead-out at frame 206535.
    const OFFSETS: [u32; 10] = [
        150, 18901, 39738, 59557, 79152, 100126, 124833, 147278, 166336, 182560,
    ];

    #[test]
    fn disc_ids_follow_the_cddb_algorithm() {
        let toc = Toc {
            offsets: OFFSETS.to_vec(),
            leadout: 206535,
        };
        assert_eq!(toc.disc_id(), "830abf0a");
        assert_eq!(
            toc.query_command(),
            "cddb query 830abf0a 10 150 18901 39738 59557 79152 100126 124833 147278 166336 182560 2753"
        );
    }

    #[test]
    fn file_lengths_become_a_toc_after_the_two_second_lead_in() {
        let mut lengths: Vec<f64> = OFFSETS
            .windows(2)
            .map(|pair| f64::from(pair[1] - pair[0]) / 75.0)
            .collect();
        lengths.push(f64::from(206535 - 182560) / 75.0);
        let toc = Toc::from_lengths(&lengths).unwrap();
        assert_eq!(toc.offsets, OFFSETS);
        assert_eq!(toc.leadout, 206535);
        assert_eq!(toc.disc_id(), "830abf0a");
        assert!(Toc::from_lengths(&[]).is_err());
        assert!(Toc::from_lengths(&[200.0, 0.0]).is_err());
        assert!(Toc::from_lengths(&[f64::NAN]).is_err());
        assert!(Toc::from_lengths(&vec![60.0; 100]).is_err());
    }

    #[test]
    fn disc_and_entry_ids_are_validated() {
        assert_eq!(disc_id(" 0x7A0A6B09 ").unwrap(), "7a0a6b09");
        assert_eq!(disc_id("rock/7a0a6b09").unwrap(), "7a0a6b09");
        assert!(disc_id("7a0a6b0").is_err());
        assert!(disc_id("../../etc").is_err());
        assert_eq!(
            entry_id("Rock/7a0a6b09").unwrap(),
            ("rock".to_string(), "7a0a6b09".to_string())
        );
        assert!(entry_id("../x/7a0a6b09").is_err());
        assert!(entry_id("7a0a6b09").is_err());
    }

    #[test]
    fn command_urls_carry_the_handshake_and_protocol() {
        let mut settings = FreedbSettings::default();
        let url = command_url(&settings, "cddb read rock 7a0a6b09").unwrap();
        assert_eq!(url.scheme(), "https");
        assert_eq!(url.host_str(), Some("gnudb.gnudb.org"));
        assert_eq!(url.path(), "/~cddb/cddb.cgi");
        let query = url.query().unwrap();
        assert!(query.starts_with("cmd=cddb+read+rock+7a0a6b09&hello=opentag+localhost+OpenTag+"));
        assert!(query.ends_with("&proto=6"));
        settings.email = "me@example.org".into();
        settings.protocol = "http".into();
        settings.port = Some(8880);
        settings.path = "cddb.cgi".into();
        let url = command_url(&settings, "cddb lscat").unwrap();
        assert_eq!(
            url.as_str().split('?').next().unwrap(),
            "http://gnudb.gnudb.org:8880/cddb.cgi"
        );
        assert!(url
            .query()
            .unwrap()
            .contains("hello=me+example.org+OpenTag"));
        for bad in ["", "evil.com/x", "user@host", "host:1"] {
            settings.address = bad.into();
            assert!(command_url(&settings, "x").is_err(), "{bad}");
        }
        settings.address = "gnudb.gnudb.org".into();
        settings.path = "/cgi?x=1".into();
        assert!(command_url(&settings, "x").is_err());
    }

    #[test]
    fn settings_come_from_the_freedb_options_pages() {
        let mut settings = crate::settings::Settings::default();
        assert_eq!(settings_from(&settings), FreedbSettings::default());
        settings.pages.insert(
            "tag-sources-freedb".into(),
            serde_json::json!({"protocol": "http", "address": " cddb.example.org ", "path": "", "port": "8080",
                "email": "a@b.c", "comment_disc_id": true, "disc_id_field": true}),
        );
        settings.pages.insert(
            "tag-sources-local-freedb".into(),
            serde_json::json!({"directory": "/data/freedb"}),
        );
        let freedb = settings_from(&settings);
        assert_eq!(freedb.protocol, "http");
        assert_eq!(freedb.address, "cddb.example.org");
        assert_eq!(freedb.path, "/~cddb/cddb.cgi");
        assert_eq!(freedb.port, Some(8080));
        assert!(freedb.comment_disc_id && freedb.disc_id_field);
        assert_eq!(freedb.local_directory, "/data/freedb");
        assert_eq!(server_name(&freedb), "The freedb server");
    }

    #[test]
    fn query_responses_list_exact_and_inexact_matches() {
        assert_eq!(
            parse_query("200 rock 7a0a6b09 Some Band / First Record\r\n").unwrap(),
            vec![Match {
                category: "rock".into(),
                disc_id: "7a0a6b09".into(),
                artist: "Some Band".into(),
                title: "First Record".into()
            }]
        );
        let inexact = "211 Found inexact matches, list follows (until terminating `.')\r\nrock 7a0a6b09 A / B\r\nmisc 7a0a6b0a Untitled\r\nbroken line\r\n.\r\n";
        let matches = parse_query(inexact).unwrap();
        assert_eq!(matches.len(), 2);
        assert_eq!(
            (matches[1].artist.as_str(), matches[1].title.as_str()),
            ("Untitled", "Untitled")
        );
        assert!(parse_query("202 No match found\r\n").unwrap().is_empty());
        let refused =
            parse_query("500 Unknown application, developer email for OpenTag 0.1").unwrap_err();
        assert!(
            refused.contains("does not accept requests from OpenTag"),
            "{refused}"
        );
        assert!(parse_query("409 No handshake")
            .unwrap_err()
            .contains("email"));
        assert!(parse_query("<html>").is_err());
    }

    #[test]
    fn read_responses_return_the_entry_or_nothing() {
        let body = format!(
            "210 rock 7a0a6b09 CD database entry follows (until terminating `.')\r\n{ENTRY}.\r\n"
        );
        let entry = parse_read(&body).unwrap().unwrap();
        assert!(entry.starts_with("# xmcd"));
        assert!(!entry.ends_with('.'));
        assert_eq!(
            parse_read("401 rock 7a0a6b09 No such CD entry in database.").unwrap(),
            None
        );
        assert!(parse_read("402 Server error.").is_err());
    }

    #[test]
    fn xmcd_entries_map_to_album_and_track_fields() {
        let xmcd = parse_xmcd(ENTRY);
        assert_eq!(xmcd.offsets, [150, 15150, 33150]);
        assert_eq!(xmcd.disc_length, Some(620));
        let release = release(
            &xmcd,
            Provider::Freedb,
            "rock",
            "1f026a03",
            ImportOptions::default(),
            true,
        )
        .unwrap();
        assert_eq!(release.id, "rock/1f026a03");
        assert_eq!(release.url, "https://gnudb.org/cd/ro1f026a03");
        assert_eq!(release.title, "Long Album Title, Continued");
        assert_eq!(release.fields["ARTIST"], ["The Example Band"]);
        assert_eq!(release.fields["ALBUM"], ["Long Album Title, Continued"]);
        assert_eq!(release.fields["YEAR"], ["1998"]);
        assert_eq!(release.fields["GENRE"], ["Alternative Rock"]);
        assert_eq!(release.fields["COMMENT"], ["Recorded live.\nSecond line"]);
        assert!(!release.fields.contains_key("ALBUMARTIST"));
        let titles: Vec<_> = release.tracks.iter().map(|t| t.title.as_str()).collect();
        assert_eq!(titles, ["Opening", "Song / With Slash", "Back\\slash"]);
        assert_eq!(release.tracks[0].fields["TRACK"], ["1/3"]);
        assert_eq!(release.tracks[1].fields["COMMENT"], ["Bonus"]);
        assert!(!release.tracks[1].fields.contains_key("ARTIST"));
        let lengths: Vec<_> = release.tracks.iter().map(|t| t.length).collect();
        assert_eq!(lengths, [Some(200.0), Some(240.0), Some(178.0)]);

        let with_ids = super::release(
            &xmcd,
            Provider::LocalFreedb,
            "rock",
            "1f026a03",
            ImportOptions {
                comment_disc_id: true,
                disc_id_field: true,
            },
            false,
        )
        .unwrap();
        assert_eq!(with_ids.url, "");
        assert_eq!(with_ids.fields["COMMENT"], ["1f026a03"]);
        assert_eq!(with_ids.fields["DISCID"], ["1f026a03"]);
        assert!(!with_ids.tracks[1].fields.contains_key("COMMENT"));
    }

    #[test]
    fn various_artist_discs_split_track_artists() {
        let text = "DTITLE=Various Artists / Hits\nDYEAR=\nDGENRE=\nTTITLE0=Singer A / Song A\nTTITLE1=Song B\nEXTD=YEAR: 1987 ID3G: 13\n";
        let release = release(
            &parse_xmcd(text),
            Provider::Freedb,
            "misc",
            "00000002",
            ImportOptions::default(),
            true,
        )
        .unwrap();
        assert_eq!(release.fields["ALBUMARTIST"], ["Various Artists"]);
        assert_eq!(release.fields["YEAR"], ["1987"]);
        assert!(!release.fields.contains_key("GENRE"), "misc is not a genre");
        assert!(!release.fields.contains_key("COMMENT"));
        assert_eq!(release.tracks[0].fields["ARTIST"], ["Singer A"]);
        assert_eq!(release.tracks[0].title, "Song A");
        assert_eq!(release.tracks[1].title, "Song B");
        assert_eq!(release.tracks[0].length, None);
        let all_split = "DTITLE=Mix / Night\nTTITLE0=A / One\nTTITLE1=B / Two\n";
        let mix = super::release(
            &parse_xmcd(all_split),
            Provider::Freedb,
            "rock",
            "00000002",
            ImportOptions::default(),
            true,
        )
        .unwrap();
        assert_eq!(mix.tracks[1].fields["ARTIST"], ["B"]);
        assert_eq!(mix.fields["GENRE"], ["Rock"]);
        assert!(super::release(
            &parse_xmcd("# nothing"),
            Provider::Freedb,
            "rock",
            "00000000",
            ImportOptions::default(),
            true
        )
        .is_err());
    }

    #[test]
    fn legacy_entries_decode_as_latin_1() {
        assert_eq!(decode(b"DTITLE=Caf\xe9"), "DTITLE=Café");
        assert_eq!(decode("DTITLE=Café".as_bytes()), "DTITLE=Café");
    }

    #[test]
    fn local_folders_find_unix_and_windows_layouts() {
        let dir = tempfile::tempdir().unwrap();
        assert!(local_lookup(Path::new(""), "1f026a03")
            .unwrap_err()
            .contains("Options"));
        assert!(local_lookup(&dir.path().join("missing"), "1f026a03").is_err());
        std::fs::create_dir(dir.path().join("rock")).unwrap();
        std::fs::create_dir(dir.path().join("misc")).unwrap();
        std::fs::write(dir.path().join("rock").join("1f026a03"), ENTRY).unwrap();
        let bundle = format!("#FILENAME=1e000001\nDTITLE=Other / Disc\n#FILENAME=1f026a03\n{ENTRY}#FILENAME=1f026a04\nDTITLE=Next / Disc\n");
        std::fs::write(dir.path().join("misc").join("1eto1f"), bundle).unwrap();
        std::fs::write(
            dir.path().join("misc").join("20to2f"),
            "#FILENAME=1f026a03\nDTITLE=Wrong / File\n",
        )
        .unwrap();
        let found = local_lookup(dir.path(), "1f026a03").unwrap();
        let categories: Vec<_> = found.iter().map(|(c, _)| c.as_str()).collect();
        assert_eq!(categories, ["misc", "rock"]);
        for (_, entry) in &found {
            assert_eq!(
                parse_xmcd(entry).get("DTITLE"),
                "The Example Band / Long Album Title, Continued"
            );
        }
        assert!(local_lookup(dir.path(), "00000000").unwrap().is_empty());
        assert!(
            covers_prefix("00toff", 0x7a)
                && !covers_prefix("80toff", 0x7a)
                && !covers_prefix("rock", 1)
        );
    }
}
