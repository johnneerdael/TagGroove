//! Cue sheet tracks (https://docs.mp3tag.de/playlists/): the TRACK blocks of
//! a cue sheet with their TITLE, PERFORMER, SONGWRITER, ISRC, INDEX and REM
//! lines, read as Mp3tag fields and edited line by line. Every line that is
//! not edited keeps its exact text, order and line ending.
//!
//! Album lines (before the first TRACK) supply ALBUM (TITLE), ALBUMARTIST
//! (PERFORMER, also the ARTIST of tracks without their own), GENRE, YEAR
//! (`REM DATE`), COMMENT, DISCNUMBER, TOTALDISCS and the album ReplayGain
//! values; TRACK blocks supply TITLE, ARTIST (PERFORMER), COMPOSER
//! (SONGWRITER), ISRC, the track ReplayGain values and the track number.
use crate::Fields;
use std::sync::RwLock;

/// CD frames per second of `INDEX mm:ss:ff` positions.
const FRAMES: u64 = 75;

/// Field name, cue keyword and whether it is a `REM` comment.
type Key = (&'static str, &'static str, bool);
const ALBUM_KEYS: [Key; 9] = [
    ("ALBUM", "TITLE", false),
    ("ALBUMARTIST", "PERFORMER", false),
    ("GENRE", "GENRE", true),
    ("YEAR", "DATE", true),
    ("COMMENT", "COMMENT", true),
    ("DISCNUMBER", "DISCNUMBER", true),
    ("TOTALDISCS", "TOTALDISCS", true),
    ("REPLAYGAIN_ALBUM_GAIN", "REPLAYGAIN_ALBUM_GAIN", true),
    ("REPLAYGAIN_ALBUM_PEAK", "REPLAYGAIN_ALBUM_PEAK", true),
];
const TRACK_KEYS: [Key; 6] = [
    ("TITLE", "TITLE", false),
    ("ARTIST", "PERFORMER", false),
    ("COMPOSER", "SONGWRITER", false),
    ("ISRC", "ISRC", false),
    ("REPLAYGAIN_TRACK_GAIN", "REPLAYGAIN_TRACK_GAIN", true),
    ("REPLAYGAIN_TRACK_PEAK", "REPLAYGAIN_TRACK_PEAK", true),
];
/// Keywords whose new lines are written with quotes.
const QUOTED: [&str; 5] = ["TITLE", "PERFORMER", "SONGWRITER", "GENRE", "COMMENT"];

static ANSI_CODEPAGE: RwLock<Option<String>> = RwLock::new(None);

/// The system ANSI codepage used for cue sheets that are not UTF-8.
pub fn set_ansi_codepage(codepage: Option<String>) {
    if let Ok(mut current) = ANSI_CODEPAGE.write() {
        *current = codepage;
    }
}
fn ansi_codepage() -> Option<String> {
    ANSI_CODEPAGE.read().ok().and_then(|c| c.clone())
}

#[derive(Debug, Clone, PartialEq)]
struct Line {
    text: String,
    ending: String,
}

/// One TRACK block.
#[derive(Debug, Clone, PartialEq)]
pub struct CueTrack {
    /// The number on the TRACK line.
    pub number: u32,
    /// The location on the governing FILE line, as written.
    pub file: Option<String>,
    /// Position of that FILE line among the sheet's FILE lines.
    pub file_index: Option<usize>,
    /// 0-based line of that FILE keyword.
    file_line: Option<usize>,
    /// `INDEX 01` in milliseconds.
    pub start: Option<u64>,
    /// `INDEX 00` (the pregap) in milliseconds, in the FILE it appears under.
    pub pregap: Option<u64>,
    /// An `AUDIO` track; data tracks are not listed.
    pub audio: bool,
    /// Line of the TRACK keyword and the first line after the block.
    line: usize,
    end: usize,
}

/// A parsed cue sheet that can be edited and rendered again.
#[derive(Debug, Clone, PartialEq)]
pub struct CueSheet {
    lines: Vec<Line>,
    pub tracks: Vec<CueTrack>,
    /// Number of FILE lines.
    pub files: usize,
}

/// The keyword (upper case) and the rest of a line.
fn split(text: &str) -> (String, &str) {
    let trimmed = text.trim_start_matches('\u{feff}').trim();
    match trimmed.split_once(char::is_whitespace) {
        Some((keyword, rest)) => (keyword.to_ascii_uppercase(), rest.trim()),
        None => (trimmed.to_ascii_uppercase(), ""),
    }
}

/// A value, without quotes. Quoted values run to the last quote, so titles
/// with inner quotes read back as written.
fn unquote(value: &str) -> &str {
    let value = value.trim();
    match value.strip_prefix('"') {
        Some(rest) => match rest.rfind('"') {
            Some(end) => &rest[..end],
            None => rest,
        },
        None => value,
    }
}

/// The location of a FILE line: quoted, or everything before the file type.
pub(crate) fn file_location(rest: &str) -> String {
    if let Some(quoted) = rest.strip_prefix('"') {
        return quoted.split('"').next().unwrap_or_default().to_string();
    }
    match rest.rsplit_once(char::is_whitespace) {
        Some((name, _)) => name.trim().to_string(),
        None => rest.to_string(),
    }
}

/// `mm:ss:ff` in milliseconds.
fn index_time(value: &str) -> Option<u64> {
    let parts: Vec<u64> = value
        .split(':')
        .map(|p| p.trim().parse().ok())
        .collect::<Option<_>>()?;
    let [minutes, seconds, frames] = parts[..] else {
        return None;
    };
    (seconds < 60 && frames < FRAMES)
        .then(|| (minutes * 60 + seconds) * 1000 + frames * 1000 / FRAMES)
}

impl CueTrack {
    /// The 0-based line of the governing FILE keyword, else of TRACK.
    pub fn line(&self) -> usize {
        self.file_line.unwrap_or(self.line)
    }
}

impl CueSheet {
    pub fn parse(text: &str) -> Self {
        let mut lines = Vec::new();
        let mut rest = text;
        while !rest.is_empty() {
            let (line, next) = match rest.find('\n') {
                Some(at) => (&rest[..=at], &rest[at + 1..]),
                None => (rest, ""),
            };
            let body = line.trim_end_matches(['\r', '\n']);
            lines.push(Line {
                text: body.to_string(),
                ending: line[body.len()..].to_string(),
            });
            rest = next;
        }
        let mut sheet = CueSheet {
            lines,
            tracks: Vec::new(),
            files: 0,
        };
        sheet.index();
        sheet
    }

    /// Rebuild the TRACK and FILE positions after the lines changed.
    fn index(&mut self) {
        self.tracks.clear();
        self.files = 0;
        let mut file: Option<(String, usize, usize)> = None;
        for (at, line) in self.lines.iter().enumerate() {
            let (keyword, rest) = split(&line.text);
            match keyword.as_str() {
                "FILE" => {
                    if let Some(track) = self.tracks.last_mut() {
                        track.end = track.end.min(at);
                    }
                    file = Some((file_location(rest), self.files, at));
                    self.files += 1;
                }
                "TRACK" => {
                    if let Some(track) = self.tracks.last_mut() {
                        track.end = track.end.min(at);
                    }
                    let mut words = rest.split_whitespace();
                    let number = words.next().and_then(|n| n.parse().ok()).unwrap_or(0);
                    let audio = words
                        .next()
                        .is_none_or(|kind| kind.eq_ignore_ascii_case("AUDIO"));
                    self.tracks.push(CueTrack {
                        number,
                        file: file.as_ref().map(|(f, ..)| f.clone()),
                        file_index: file.as_ref().map(|(_, i, _)| *i),
                        file_line: file.as_ref().map(|(.., line)| *line),
                        start: None,
                        pregap: None,
                        audio,
                        line: at,
                        end: usize::MAX,
                    });
                }
                // INDEX lines after a FILE line still belong to the last
                // TRACK: its INDEX 01 can sit in the next file, after a
                // pregap at the end of the previous one. The track plays from
                // the file of its INDEX 01.
                "INDEX" => {
                    if let Some(track) = self.tracks.last_mut().filter(|t| t.start.is_none()) {
                        let mut parts = rest.split_whitespace();
                        let number = parts.next().and_then(|n| n.parse::<u32>().ok());
                        let time = parts.next().and_then(index_time);
                        match number {
                            Some(0) => track.pregap = time,
                            Some(1) if time.is_some() => {
                                track.start = time;
                                track.file = file.as_ref().map(|(f, ..)| f.clone());
                                track.file_index = file.as_ref().map(|(_, i, _)| *i);
                                track.file_line = file.as_ref().map(|(.., line)| *line);
                            }
                            _ => {}
                        }
                    }
                }
                _ => {}
            }
        }
        let count = self.lines.len();
        for track in &mut self.tracks {
            track.end = track.end.min(count);
        }
    }

    pub fn render(&self) -> String {
        self.lines
            .iter()
            .map(|l| format!("{}{}", l.text, l.ending))
            .collect()
    }

    /// Lines before the first TRACK: the album scope.
    fn album_lines(&self) -> std::ops::Range<usize> {
        0..self.tracks.first().map_or(self.lines.len(), |t| t.line)
    }
    fn track_lines(&self, index: usize) -> std::ops::Range<usize> {
        let track = &self.tracks[index];
        track.line + 1..track.end
    }

    /// The line of `keyword` (`REM` key when `rem`) in `range`.
    fn find(&self, range: std::ops::Range<usize>, keyword: &str, rem: bool) -> Option<usize> {
        range.into_iter().find(|at| {
            let (word, rest) = split(&self.lines[*at].text);
            if rem {
                word == "REM" && split(rest).0 == keyword
            } else {
                word == keyword
            }
        })
    }
    fn value(&self, range: std::ops::Range<usize>, keyword: &str, rem: bool) -> Option<String> {
        let at = self.find(range, keyword, rem)?;
        let (_, rest) = split(&self.lines[at].text);
        let rest = if rem { split(rest).1 } else { rest };
        Some(unquote(rest).to_string())
    }

    /// Fields of the album lines.
    fn album_fields(&self) -> Fields {
        let mut fields = Fields::new();
        let album = self.album_lines();
        for (field, keyword, rem) in ALBUM_KEYS {
            if let Some(value) = self.value(album.clone(), keyword, rem) {
                fields.insert(field.into(), vec![value]);
            }
        }
        fields
    }

    /// Fields stored in the TRACK block at `index` (0-based position in the
    /// sheet), with its TRACK number.
    pub fn own_track_fields(&self, index: usize) -> Fields {
        let mut fields = Fields::new();
        let Some(track) = self.tracks.get(index) else {
            return fields;
        };
        let lines = self.track_lines(index);
        for (field, keyword, rem) in TRACK_KEYS {
            if let Some(value) = self.value(lines.clone(), keyword, rem) {
                fields.insert(field.into(), vec![value]);
            }
        }
        fields.insert("TRACK".into(), vec![track.number.to_string()]);
        fields.retain(|_, values| values.iter().any(|v| !v.is_empty()));
        fields
    }

    /// Mp3tag fields of the track at `index`: its own lines, the album lines
    /// and the album PERFORMER and SONGWRITER for tracks without their own.
    pub fn track_fields(&self, index: usize) -> Fields {
        let mut fields = self.album_fields();
        fields.extend(self.own_track_fields(index));
        if !fields.contains_key("ARTIST") {
            if let Some(artist) = fields.get("ALBUMARTIST").cloned() {
                fields.insert("ARTIST".into(), artist);
            }
        }
        if !fields.contains_key("COMPOSER") {
            if let Some(composer) = self.value(self.album_lines(), "SONGWRITER", false) {
                fields.insert("COMPOSER".into(), vec![composer]);
            }
        }
        fields.retain(|_, values| values.iter().any(|v| !v.is_empty()));
        fields
    }

    /// The tracks listed as subsongs, as positions in `tracks`: AUDIO
    /// tracks with an `INDEX 01`.
    pub fn subsongs(&self) -> Vec<usize> {
        (0..self.tracks.len())
            .filter(|at| self.tracks[*at].audio && self.tracks[*at].start.is_some())
            .collect()
    }

    /// Start of the track in milliseconds and its end: the `INDEX 01` of the
    /// next listed track when that plays from the same FILE (so a pregap
    /// belongs to the track before it), else the end of the file.
    pub fn track_time(&self, index: usize) -> (u64, Option<u64>) {
        let Some(track) = self.tracks.get(index) else {
            return (0, None);
        };
        let end = self.tracks[index + 1..]
            .iter()
            .find(|next| next.audio && next.start.is_some())
            .filter(|next| next.file_index == track.file_index)
            .and_then(|next| next.start);
        (track.start.unwrap_or(0), end)
    }

    fn indent(&self, range: std::ops::Range<usize>, fallback: &str) -> String {
        range
            .into_iter()
            .map(|at| &self.lines[at].text)
            .find(|text| !text.trim().is_empty())
            .map(|text| text[..text.len() - text.trim_start().len()].to_string())
            .unwrap_or_else(|| fallback.to_string())
    }
    fn ending(&self) -> String {
        self.lines
            .iter()
            .map(|l| l.ending.clone())
            .find(|e| !e.is_empty())
            .unwrap_or_else(|| "\r\n".into())
    }

    /// Set (or with an empty value remove) one keyword in a scope.
    fn set(&mut self, track: Option<usize>, keyword: &str, rem: bool, value: &str) {
        let range = match track {
            Some(index) => self.track_lines(index),
            None => self.album_lines(),
        };
        let value = value.replace(['\r', '\n'], " ");
        let existing = self.find(range.clone(), keyword, rem);
        if value.is_empty() {
            if let Some(at) = existing {
                self.lines.remove(at);
                self.index();
            }
            return;
        }
        let quote = |old: Option<&str>| match old {
            Some(old) => unquote(old).len() != old.trim().len() || value.contains(' '),
            None => QUOTED.contains(&keyword) || value.contains(' '),
        };
        let render = |indent: &str, old: Option<&str>| {
            let value = if quote(old) {
                format!("\"{value}\"")
            } else {
                value.clone()
            };
            if rem {
                format!("{indent}REM {keyword} {value}")
            } else {
                format!("{indent}{keyword} {value}")
            }
        };
        match existing {
            Some(at) => {
                let text = &self.lines[at].text;
                let indent = text[..text.len() - text.trim_start().len()].to_string();
                let (_, rest) = split(text);
                let old = if rem { split(rest).1 } else { rest }.to_string();
                self.lines[at].text = render(&indent, Some(&old));
            }
            None => {
                let (at, indent) = match track {
                    Some(index) => {
                        let track_line = &self.lines[self.tracks[index].line].text;
                        let outer = &track_line[..track_line.len() - track_line.trim_start().len()];
                        (
                            self.tracks[index].line + 1,
                            self.indent(range, &format!("{outer}  ")),
                        )
                    }
                    // Album lines go before the first FILE or TRACK line.
                    None => {
                        let first = range
                            .clone()
                            .find(|at| split(&self.lines[*at].text).0 == "FILE")
                            .unwrap_or(range.end);
                        (first, String::new())
                    }
                };
                let ending = self.ending();
                if at == self.lines.len() {
                    if let Some(last) = self.lines.last_mut().filter(|l| l.ending.is_empty()) {
                        last.ending = ending.clone();
                    }
                }
                self.lines.insert(
                    at,
                    Line {
                        text: render(&indent, None),
                        ending,
                    },
                );
                self.index();
            }
        }
    }

    /// Whether a field can be stored for a single track.
    pub fn track_field(name: &str) -> bool {
        TRACK_KEYS
            .iter()
            .any(|(f, _, _)| f.eq_ignore_ascii_case(name))
    }

    /// Apply field changes for the tracks at the given positions. Album
    /// fields must agree between the rows; TRACK and fields a cue sheet has
    /// no line for are refused.
    pub fn apply(&mut self, rows: &[(usize, Fields)]) -> Result<(), String> {
        let mut album: Vec<(&'static str, &'static str, bool, String)> = Vec::new();
        let mut tracks = Vec::new();
        for (index, updates) in rows {
            if *index >= self.tracks.len() {
                return Err(format!("Track {} is not in the cue sheet", index + 1));
            }
            for (name, values) in updates {
                let value = values.join("; ");
                if let Some((_, keyword, rem)) = TRACK_KEYS
                    .iter()
                    .find(|(f, _, _)| f.eq_ignore_ascii_case(name))
                {
                    tracks.push((*index, *keyword, *rem, value));
                } else if let Some((field, keyword, rem)) = ALBUM_KEYS
                    .iter()
                    .find(|(f, _, _)| f.eq_ignore_ascii_case(name))
                {
                    match album.iter().find(|(f, ..)| f == field) {
                        Some((.., other)) if *other != value => {
                            return Err(format!(
                                "{field} is stored once for the whole cue sheet; give every track the same value"
                            ))
                        }
                        Some(_) => {}
                        None => album.push((field, keyword, *rem, value)),
                    }
                } else if name.eq_ignore_ascii_case("TRACK") {
                    if values.join("") != self.tracks[*index].number.to_string() {
                        return Err("Track numbers come from the cue sheet's TRACK lines".into());
                    }
                } else {
                    return Err(format!("Cue sheets cannot store the field {name}"));
                }
            }
        }
        for (_, keyword, rem, value) in album {
            self.set(None, keyword, rem, &value);
        }
        for (index, keyword, rem, value) in tracks {
            self.set(Some(index), keyword, rem, &value);
        }
        Ok(())
    }
}

/// How a cue sheet file was encoded, so edits are written the same way.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CueEncoding {
    Bom(&'static encoding_rs::Encoding),
    Utf8,
    Ansi(&'static encoding_rs::Encoding),
}

/// Decode cue sheet bytes: a BOM wins, then UTF-8, then the ANSI codepage.
pub fn decode(bytes: &[u8]) -> (String, CueEncoding) {
    if let Some((encoding, bom)) = encoding_rs::Encoding::for_bom(bytes) {
        let text = encoding.decode_without_bom_handling(&bytes[bom..]).0;
        return (text.into_owned(), CueEncoding::Bom(encoding));
    }
    if let Ok(text) = std::str::from_utf8(bytes) {
        return (text.to_string(), CueEncoding::Utf8);
    }
    let encoding = crate::playlist::ansi_encoding(ansi_codepage().as_deref())
        .unwrap_or(encoding_rs::WINDOWS_1252);
    let text = encoding.decode_without_bom_handling(bytes).0;
    (text.into_owned(), CueEncoding::Ansi(encoding))
}

/// Encode edited text like the original; ANSI text that no longer fits its
/// codepage is written as UTF-8 with a BOM.
pub fn encode(text: &str, encoding: CueEncoding) -> Vec<u8> {
    match encoding {
        CueEncoding::Utf8 => text.as_bytes().to_vec(),
        CueEncoding::Bom(encoding) if encoding == encoding_rs::UTF_16LE => {
            let mut bytes = vec![0xFF, 0xFE];
            bytes.extend(text.encode_utf16().flat_map(u16::to_le_bytes));
            bytes
        }
        CueEncoding::Bom(encoding) if encoding == encoding_rs::UTF_16BE => {
            let mut bytes = vec![0xFE, 0xFF];
            bytes.extend(text.encode_utf16().flat_map(u16::to_be_bytes));
            bytes
        }
        CueEncoding::Bom(_) => [&[0xEF, 0xBB, 0xBF][..], text.as_bytes()].concat(),
        CueEncoding::Ansi(encoding) => {
            let (bytes, _, unmappable) = encoding.encode(text);
            if unmappable {
                [&[0xEF, 0xBB, 0xBF][..], text.as_bytes()].concat()
            } else {
                bytes.into_owned()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SHEET: &str = "REM GENRE Rock\r\nREM DATE 1999\r\nPERFORMER \"The Band\"\r\nTITLE \"Live \"Here\"\"\r\nFILE \"image.flac\" WAVE\r\n  TRACK 01 AUDIO\r\n    TITLE \"Opening\"\r\n    INDEX 01 00:00:00\r\n  TRACK 02 AUDIO\r\n    TITLE \"Second\"\r\n    PERFORMER \"Guest\"\r\n    INDEX 00 00:01:00\r\n    INDEX 01 00:02:37\r\nFILE other.wav WAVE\r\n  TRACK 03 AUDIO\r\n    INDEX 01 00:00:00\r\n";

    #[test]
    fn tracks_fields_and_times_are_read() {
        let sheet = CueSheet::parse(SHEET);
        assert_eq!(sheet.files, 2);
        assert_eq!(sheet.tracks.len(), 3);
        assert_eq!(sheet.tracks[1].file.as_deref(), Some("image.flac"));
        assert_eq!(sheet.tracks[2].file.as_deref(), Some("other.wav"));
        let first = sheet.track_fields(0);
        assert_eq!(first["TITLE"], ["Opening"]);
        assert_eq!(first["ARTIST"], ["The Band"]);
        assert_eq!(first["ALBUMARTIST"], ["The Band"]);
        assert_eq!(first["ALBUM"], ["Live \"Here\""]);
        assert_eq!(first["GENRE"], ["Rock"]);
        assert_eq!(first["YEAR"], ["1999"]);
        assert_eq!(first["TRACK"], ["1"]);
        assert_eq!(sheet.track_fields(1)["ARTIST"], ["Guest"]);
        assert_eq!(sheet.track_time(0), (0, Some(2493)));
        assert_eq!(sheet.track_time(1), (2493, None));
        assert_eq!(sheet.track_time(2), (0, None));
        assert_eq!(sheet.render(), SHEET);
    }

    #[test]
    fn tracks_spanning_files_pregaps_and_data_tracks() {
        let text = "FILE \"01.wav\" WAVE\n  TRACK 01 AUDIO\n    TITLE \"One\"\n    INDEX 01 00:00:00\n  TRACK 02 AUDIO\n    TITLE \"Two\"\n    INDEX 00 03:20:15\nFILE \"02.wav\" WAVE\n    INDEX 01 00:00:00\n  TRACK 03 AUDIO\n    INDEX 00 02:00:00\n    INDEX 01 02:02:00\n  TRACK 04 MODE1/2352\n    INDEX 01 03:00:00\n";
        let sheet = CueSheet::parse(text);
        assert_eq!(sheet.tracks.len(), 4);
        // Track 2's pregap ends 01.wav; the track plays from 02.wav.
        assert_eq!(sheet.tracks[1].file.as_deref(), Some("02.wav"));
        assert_eq!(sheet.tracks[1].pregap, Some(200_200));
        assert_eq!(sheet.tracks[1].line(), 7);
        // Track 1 runs to the end of 01.wav, pregap included.
        assert_eq!(sheet.track_time(0), (0, None));
        // A pregap belongs to the track before it: track 2 ends at the
        // INDEX 01 of track 3, not at its INDEX 00.
        assert_eq!(sheet.track_time(1), (0, Some(122_000)));
        // The data track is not listed and does not end track 3.
        assert_eq!(sheet.track_time(2), (122_000, None));
        assert_eq!(sheet.subsongs(), [0, 1, 2]);
        assert!(!sheet.tracks[3].audio);
        assert_eq!(sheet.track_fields(1)["TITLE"], ["Two"]);
        let mut edited = sheet.clone();
        edited
            .apply(&[(1, [("TITLE".to_string(), vec!["Deux".to_string()])].into())])
            .unwrap();
        assert_eq!(edited.render(), text.replace("\"Two\"", "\"Deux\""));
        assert_eq!(sheet.render(), text);
    }

    #[test]
    fn edits_change_only_their_lines() {
        let mut sheet = CueSheet::parse(SHEET);
        let fields = |pairs: &[(&str, &str)]| -> Fields {
            pairs
                .iter()
                .map(|(k, v)| (k.to_string(), vec![v.to_string()]))
                .collect()
        };
        sheet
            .apply(&[
                (
                    0,
                    fields(&[("TITLE", "Intro"), ("ARTIST", "Solo"), ("ALBUM", "Renamed")]),
                ),
                (
                    1,
                    fields(&[
                        ("ARTIST", ""),
                        ("ISRC", "USXXX0000001"),
                        ("ALBUM", "Renamed"),
                    ]),
                ),
                (
                    2,
                    fields(&[("REPLAYGAIN_TRACK_GAIN", "-6.50 dB"), ("GENRE", "Jazz")]),
                ),
            ])
            .unwrap();
        let text = sheet.render();
        assert_eq!(
            text,
            "REM GENRE Jazz\r\nREM DATE 1999\r\nPERFORMER \"The Band\"\r\nTITLE \"Renamed\"\r\nFILE \"image.flac\" WAVE\r\n  TRACK 01 AUDIO\r\n    PERFORMER \"Solo\"\r\n    TITLE \"Intro\"\r\n    INDEX 01 00:00:00\r\n  TRACK 02 AUDIO\r\n    ISRC USXXX0000001\r\n    TITLE \"Second\"\r\n    INDEX 00 00:01:00\r\n    INDEX 01 00:02:37\r\nFILE other.wav WAVE\r\n  TRACK 03 AUDIO\r\n    REM REPLAYGAIN_TRACK_GAIN \"-6.50 dB\"\r\n    INDEX 01 00:00:00\r\n"
        );
        let again = CueSheet::parse(&text);
        assert_eq!(again.track_fields(0)["TITLE"], ["Intro"]);
        assert_eq!(again.track_fields(0)["ARTIST"], ["Solo"]);
        assert_eq!(again.track_fields(1)["ARTIST"], ["The Band"]);
        assert_eq!(again.track_fields(1)["ISRC"], ["USXXX0000001"]);
        assert_eq!(again.track_fields(2)["REPLAYGAIN_TRACK_GAIN"], ["-6.50 dB"]);
        assert_eq!(again.track_fields(2)["ALBUM"], ["Renamed"]);

        // Album fields are shared; unknown fields and track numbers are refused.
        let mut sheet = CueSheet::parse(SHEET);
        assert!(sheet
            .apply(&[
                (0, fields(&[("ALBUM", "A")])),
                (1, fields(&[("ALBUM", "B")]))
            ])
            .is_err());
        assert!(sheet.apply(&[(0, fields(&[("LYRICIST", "x")]))]).is_err());
        assert!(sheet.apply(&[(0, fields(&[("TRACK", "7")]))]).is_err());
        assert!(sheet.apply(&[(0, fields(&[("TRACK", "1")]))]).is_ok());
        assert!(sheet.apply(&[(9, fields(&[("TITLE", "x")]))]).is_err());
    }

    #[test]
    fn sheets_without_album_lines_or_final_newline_get_new_lines() {
        let mut sheet = CueSheet::parse("FILE a.wav WAVE\n TRACK 1 AUDIO\n  INDEX 01 00:00:00");
        sheet
            .apply(&[(
                0,
                [
                    ("ALBUM".to_string(), vec!["Album".to_string()]),
                    ("TITLE".to_string(), vec!["Song".to_string()]),
                ]
                .into(),
            )])
            .unwrap();
        assert_eq!(
            sheet.render(),
            "TITLE \"Album\"\nFILE a.wav WAVE\n TRACK 1 AUDIO\n  TITLE \"Song\"\n  INDEX 01 00:00:00"
        );
    }

    #[test]
    fn encodings_round_trip() {
        let (text, encoding) = decode("TITLE \"Ä\"".as_bytes());
        assert_eq!(encoding, CueEncoding::Utf8);
        assert_eq!(encode(&text, encoding), "TITLE \"Ä\"".as_bytes());
        let (text, encoding) = decode(b"TITLE \"\xC4\"");
        assert_eq!(text, "TITLE \"Ä\"");
        assert_eq!(encode(&text, encoding), b"TITLE \"\xC4\"");
        assert_eq!(
            encode("TITLE \"\u{65E5}\"", encoding),
            "\u{FEFF}TITLE \"\u{65E5}\"".as_bytes()
        );
        let utf16 = encode("TRACK", CueEncoding::Bom(encoding_rs::UTF_16LE));
        assert_eq!(
            decode(&utf16),
            ("TRACK".into(), CueEncoding::Bom(encoding_rs::UTF_16LE))
        );
    }
}
