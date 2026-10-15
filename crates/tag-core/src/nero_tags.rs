//! Nero Digital tags (`moov/udta/tags`): file-wide fields in a `meta` box
//! and per-chapter fields in `tseg` boxes, each a `tshd` with the 0-based
//! chapter index and a `meta` box. Mp3tag reads them for the subsongs of MP4
//! files with chapters and writes the fields that differ between chapters
//! there (https://docs.mp3tag.de/options/tags/advanced/).
//!
//! A `meta` payload has no version or flags: a big-endian field count, then
//! per field a flag byte (0x80 for text, 0x00 for ReplayGain values), a zero
//! byte, a length-prefixed name, a 16-bit value count and per value a 32-bit
//! length and UTF-8 bytes. Names are lowercase. Fields with other flag bytes,
//! covers (`cvrx`) and unknown boxes are kept as they are.
//!
//! A `cvrx` payload, as Mp3tag 3.36.1 reads and writes it: a 32-bit zero, a
//! 32-bit count, then per cover a 16-bit length and an ASCII MIME type, a
//! 16-bit picture type and a 32-bit length and the image bytes (at most
//! 64 MiB). Values are big-endian.
use crate::Fields;
use std::collections::BTreeMap;

/// Longest value read, as in Mp3tag.
const VALUE_LIMIT: usize = 0xF_FFFF;
/// Largest cover read, as in Mp3tag.
const COVER_LIMIT: usize = 0x400_0000;

/// One cover of a `cvrx` box.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Cover {
    pub mime: String,
    pub kind: u16,
    pub data: Vec<u8>,
}

fn parse_covers(bytes: &[u8]) -> Option<Vec<Cover>> {
    if be32(bytes, 0)? != 0 {
        return Some(Vec::new());
    }
    let count = be32(bytes, 4)?;
    let mut at = 8;
    let mut covers = Vec::new();
    for _ in 0..count {
        let length = be16(bytes, at)?;
        let mime = bytes.get(at + 2..at + 2 + length)?;
        at += 2 + length;
        let kind = be16(bytes, at)? as u16;
        let size = be32(bytes, at + 2)?;
        if size > COVER_LIMIT {
            return None;
        }
        let data = bytes.get(at + 6..at + 6 + size)?;
        at += 6 + size;
        covers.push(Cover {
            mime: String::from_utf8_lossy(mime).into_owned(),
            kind,
            data: data.to_vec(),
        });
    }
    Some(covers)
}

#[derive(Debug, Clone, PartialEq)]
enum Field {
    Text {
        flag: u8,
        name: String,
        values: Vec<String>,
    },
    /// A field of a kind that is not read, kept byte for byte.
    Raw(Vec<u8>),
}

/// The fields of one `meta` box.
type Set = Vec<Field>;

/// The parsed `udta/tags` box.
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct NeroTags {
    global: Option<Set>,
    segments: BTreeMap<u32, Set>,
    /// Covers and unknown boxes, whole.
    kept: Vec<Vec<u8>>,
}

fn be16(bytes: &[u8], at: usize) -> Option<usize> {
    Some(usize::from(u16::from_be_bytes(
        bytes.get(at..at + 2)?.try_into().ok()?,
    )))
}
fn be32(bytes: &[u8], at: usize) -> Option<usize> {
    usize::try_from(u32::from_be_bytes(bytes.get(at..at + 4)?.try_into().ok()?)).ok()
}

/// A child box: type, body and the whole box.
type Child<'a> = ([u8; 4], &'a [u8], &'a [u8]);

/// Boxes in `bytes`.
fn boxes(bytes: &[u8]) -> Option<Vec<Child<'_>>> {
    let mut out = Vec::new();
    let mut at = 0;
    while at < bytes.len() {
        let size = match be32(bytes, at)? {
            0 => bytes.len() - at,
            size => size,
        };
        if size < 8 || size > bytes.len() - at {
            return None;
        }
        let kind: [u8; 4] = bytes[at + 4..at + 8].try_into().ok()?;
        out.push((kind, &bytes[at + 8..at + size], &bytes[at..at + size]));
        at += size;
    }
    Some(out)
}

fn boxed(kind: &[u8; 4], body: &[u8]) -> Vec<u8> {
    let mut out = ((body.len() + 8) as u32).to_be_bytes().to_vec();
    out.extend(kind);
    out.extend(body);
    out
}

fn parse_meta(bytes: &[u8]) -> Option<Set> {
    let count = be32(bytes, 0)?;
    let mut at = 4;
    let mut set = Vec::new();
    for _ in 0..count.min(0xFFFF) {
        let start = at;
        let (flag, reserved) = (*bytes.get(at)?, *bytes.get(at + 1)?);
        let name_length = usize::from(*bytes.get(at + 2)?);
        let name = bytes.get(at + 3..at + 3 + name_length)?;
        at += 3 + name_length;
        let value_count = be16(bytes, at)?;
        at += 2;
        let mut values = Vec::new();
        for _ in 0..value_count {
            let length = be32(bytes, at)?;
            if length > VALUE_LIMIT {
                return None;
            }
            let value = bytes.get(at + 4..at + 4 + length)?;
            let value = value.split(|b| *b == 0).next().unwrap_or_default();
            values.push(String::from_utf8_lossy(value).into_owned());
            at += 4 + length;
        }
        let text = (flag == 1 || flag & 0x7F == 0) && reserved == 0;
        set.push(if text {
            Field::Text {
                flag,
                name: String::from_utf8_lossy(name).into_owned(),
                values,
            }
        } else {
            Field::Raw(bytes[start..at].to_vec())
        });
    }
    Some(set)
}

fn render_meta(set: &Set) -> Vec<u8> {
    let mut out = (set.len() as u32).to_be_bytes().to_vec();
    for field in set {
        match field {
            Field::Raw(bytes) => out.extend(bytes),
            Field::Text { flag, name, values } => {
                out.extend([*flag, 0, name.len() as u8]);
                out.extend(name.as_bytes());
                out.extend((values.len() as u16).to_be_bytes());
                for value in values {
                    out.extend((value.len() as u32).to_be_bytes());
                    out.extend(value.as_bytes());
                }
            }
        }
    }
    boxed(b"meta", &out)
}

/// The Mp3tag field of a Nero name.
fn field_name(name: &str) -> String {
    match name.to_ascii_lowercase().as_str() {
        "date" => "YEAR".into(),
        "tracknumber" | "track" => "TRACK".into(),
        "tempo" => "BPM".into(),
        "disc" | "discnumber" => "DISCNUMBER".into(),
        "lyrics" => "UNSYNCEDLYRICS".into(),
        _ => name.to_ascii_uppercase(),
    }
}

/// The Nero name of an Mp3tag field.
fn nero_name(field: &str) -> String {
    match field.to_ascii_uppercase().as_str() {
        "UNSYNCEDLYRICS" => "lyrics".into(),
        "BPM" => "tempo".into(),
        "TRACK" => "track".into(),
        "DISCNUMBER" => "disc".into(),
        _ => field.to_ascii_lowercase(),
    }
}

fn fields(set: &Set) -> Fields {
    let mut out = Fields::new();
    for field in set {
        if let Field::Text { name, values, .. } = field {
            let values: Vec<String> = values.iter().filter(|v| !v.is_empty()).cloned().collect();
            if !values.is_empty() {
                out.entry(field_name(name)).or_default().extend(values);
            }
        }
    }
    out
}

impl NeroTags {
    /// Read the body of a `tags` box; `None` when it is malformed.
    pub(crate) fn parse(body: &[u8]) -> Option<Self> {
        let mut tags = Self::default();
        for (kind, data, whole) in boxes(body)? {
            match &kind {
                b"meta" if tags.global.is_none() => tags.global = Some(parse_meta(data)?),
                b"tseg" => {
                    let mut index = None;
                    let mut set = None;
                    for (kind, data, _) in boxes(data)? {
                        match &kind {
                            b"tshd" if data.len() == 4 && index.is_none() => {
                                index = be32(data, 0).map(|i| i as u32)
                            }
                            b"meta" if set.is_none() => set = Some(parse_meta(data)?),
                            _ => return None,
                        }
                    }
                    tags.segments.insert(index?, set.unwrap_or_default());
                }
                _ => tags.kept.push(whole.to_vec()),
            }
        }
        Some(tags)
    }

    /// The body of the `tags` box: `meta`, the kept boxes (covers first, as
    /// they were) and one `tseg` per chapter in index order.
    pub(crate) fn render(&self) -> Vec<u8> {
        let mut out = Vec::new();
        if let Some(global) = self.global.as_ref().filter(|g| !g.is_empty()) {
            out.extend(render_meta(global));
        }
        for kept in &self.kept {
            out.extend(kept);
        }
        for (index, set) in &self.segments {
            let mut segment = boxed(b"tshd", &index.to_be_bytes());
            segment.extend(render_meta(set));
            out.extend(boxed(b"tseg", &segment));
        }
        out
    }

    /// File-wide fields.
    pub(crate) fn global_fields(&self) -> Fields {
        self.global.as_ref().map(fields).unwrap_or_default()
    }

    /// The covers of the `cvrx` box; a malformed box has none.
    pub(crate) fn covers(&self) -> Vec<Cover> {
        self.kept
            .iter()
            .find(|whole| whole.get(4..8) == Some(b"cvrx"))
            .and_then(|whole| parse_covers(&whole[8..]))
            .unwrap_or_default()
    }

    /// Drop the `cvrx` box; whether there was one.
    pub(crate) fn remove_covers(&mut self) -> bool {
        let before = self.kept.len();
        self.kept.retain(|whole| whole.get(4..8) != Some(b"cvrx"));
        self.kept.len() != before
    }

    /// Apply file field `updates` to the file-wide fields that already
    /// exist: new values replace theirs and empty values remove them, so the
    /// file-wide set never contradicts the iTunes tag it fills gaps in.
    /// Fields it does not hold are not added. Returns whether it changed.
    pub(crate) fn sync_global(&mut self, updates: &Fields) -> Result<bool, String> {
        let Some(global) = self.global.as_mut() else {
            return Ok(false);
        };
        let mut changed = false;
        for (name, list) in updates {
            let matches = |field: &Field| {
                matches!(field, Field::Text { name: stored, .. }
                    if field_name(stored).eq_ignore_ascii_case(name))
            };
            // Fields Nero does not hold are only in the iTunes tag, whose
            // limits apply to them.
            if !global.iter().any(matches) {
                continue;
            }
            let list: Vec<String> = list.iter().filter(|v| !v.is_empty()).cloned().collect();
            if list.len() > 0xFFFF
                || list
                    .iter()
                    .any(|v| v.len() > VALUE_LIMIT || v.contains('\0'))
            {
                return Err(format!("{name} cannot be stored in the Nero tag"));
            }
            if list.is_empty() {
                let before = global.len();
                global.retain(|field| !matches(field));
                changed |= global.len() != before;
                continue;
            }
            let mut first = true;
            let mut kept = Vec::new();
            for mut field in std::mem::take(global) {
                if matches(&field) {
                    if !first {
                        changed = true;
                        continue;
                    }
                    first = false;
                    if let Field::Text { values, .. } = &mut field {
                        if *values != list {
                            *values = list.clone();
                            changed = true;
                        }
                    }
                }
                kept.push(field);
            }
            *global = kept;
        }
        Ok(changed)
    }

    /// Fields of the chapter at `index` (0-based).
    pub(crate) fn chapter_fields(&self, index: usize) -> Fields {
        u32::try_from(index)
            .ok()
            .and_then(|i| self.segments.get(&i))
            .map(fields)
            .unwrap_or_default()
    }

    /// Replace the fields of the chapter at `index` by `values` (empty
    /// values are left out). Fields of other kinds are kept.
    pub(crate) fn set_chapter(&mut self, index: usize, values: &Fields) -> Result<(), String> {
        let index = u32::try_from(index).map_err(|_| "Too many chapters")?;
        let mut set: Set = self
            .segments
            .get(&index)
            .map(|set| {
                set.iter()
                    .filter(|f| matches!(f, Field::Raw(_)))
                    .cloned()
                    .collect()
            })
            .unwrap_or_default();
        for (name, list) in values {
            let list: Vec<String> = list.iter().filter(|v| !v.is_empty()).cloned().collect();
            if list.is_empty() {
                continue;
            }
            let name = nero_name(name);
            if name.len() > 255 || list.len() > 0xFFFF {
                return Err(format!("{name} is too long for a Nero chapter tag"));
            }
            if let Some(value) = list
                .iter()
                .find(|v| v.len() > VALUE_LIMIT || v.contains('\0'))
            {
                return Err(format!(
                    "{name}: {} bytes cannot be stored in a Nero chapter tag",
                    value.len()
                ));
            }
            let flag = if name.starts_with("replaygain_") {
                0
            } else {
                0x80
            };
            set.push(Field::Text {
                flag,
                name,
                values: list,
            });
        }
        self.segments.insert(index, set);
        Ok(())
    }

    /// Drop the segments of chapters from `count` on.
    pub(crate) fn truncate(&mut self, count: usize) {
        self.segments.retain(|index, _| (*index as usize) < count);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn field(name: &str, values: &[&str]) -> (String, Vec<String>) {
        (
            name.to_string(),
            values.iter().map(|v| v.to_string()).collect(),
        )
    }

    #[test]
    fn segments_round_trip_and_keep_unknown_data() {
        let mut tags = NeroTags::default();
        tags.set_chapter(
            1,
            &Fields::from([
                field("TITLE", &["Zwei"]),
                field("ARTIST", &["Gast", "Chor"]),
                field("UNSYNCEDLYRICS", &["la"]),
                field("REPLAYGAIN_TRACK_GAIN", &["-3.00 dB"]),
                field("COMMENT", &[""]),
            ]),
        )
        .unwrap();
        tags.kept.push(boxed(b"cvrx", &[0, 0, 0, 0, 0, 0, 0, 0]));
        let body = tags.render();
        let parsed = NeroTags::parse(&body).unwrap();
        assert_eq!(parsed, tags);
        let read = parsed.chapter_fields(1);
        assert_eq!(read["TITLE"], ["Zwei"]);
        assert_eq!(read["ARTIST"], ["Gast", "Chor"]);
        assert_eq!(read["UNSYNCEDLYRICS"], ["la"]);
        assert!(!read.contains_key("COMMENT"));
        assert!(parsed.chapter_fields(0).is_empty());
        // Layout: cvrx first, then tseg = tshd(1) + meta.
        assert_eq!(&body[4..8], b"cvrx");
        let tseg = &body[16..];
        assert_eq!(&tseg[4..8], b"tseg");
        assert_eq!(&tseg[8..20], b"\0\0\0\x0ctshd\0\0\0\x01");
        assert_eq!(&tseg[24..28], b"meta");
        assert_eq!(&tseg[28..32], &4u32.to_be_bytes());
        // ARTIST is first (sorted), as text with two values.
        assert_eq!(&tseg[32..41], b"\x80\0\x06artist");
        let gain = body.windows(21).position(|w| w == b"replaygain_track_gain");
        assert_eq!(body[gain.unwrap() - 3], 0);
    }

    #[test]
    fn names_map_like_mp3tag_and_odd_fields_are_kept() {
        let mut meta = 4u32.to_be_bytes().to_vec();
        for (flag, name, value) in [
            (0x80, "date", "1999"),
            (0x01, "tracknumber", "3"),
            (0x80, "tempo", "120"),
            (0x81, "secret", "x"),
        ] {
            meta.extend([flag, 0, name.len() as u8]);
            meta.extend(name.as_bytes());
            meta.extend(1u16.to_be_bytes());
            meta.extend((value.len() as u32).to_be_bytes());
            meta.extend(value.as_bytes());
        }
        let body = boxed(b"meta", &meta);
        let tags = NeroTags::parse(&body).unwrap();
        let global = tags.global_fields();
        assert_eq!(global["YEAR"], ["1999"]);
        assert_eq!(global["TRACK"], ["3"]);
        assert_eq!(global["BPM"], ["120"]);
        assert!(!global.contains_key("SECRET"));
        assert_eq!(tags.render(), body);
        assert!(NeroTags::parse(&body[..body.len() - 1]).is_none());
        // Values end at a NUL byte.
        let padded = [&1u32.to_be_bytes()[..], b"\x80\0\x01a\0\x01\0\0\0\x03b\0c"].concat();
        assert_eq!(
            parse_meta(&padded).unwrap(),
            [Field::Text {
                flag: 0x80,
                name: "a".into(),
                values: vec!["b".into()]
            }]
        );
        assert!(NeroTags::parse(&boxed(b"tseg", &boxed(b"meta", &[0; 4]))).is_none());
    }

    fn meta_body(fields: &[(u8, &str, &[&str])]) -> Vec<u8> {
        let mut meta = (fields.len() as u32).to_be_bytes().to_vec();
        for (flag, name, values) in fields {
            meta.extend([*flag, 0, name.len() as u8]);
            meta.extend(name.as_bytes());
            meta.extend((values.len() as u16).to_be_bytes());
            for value in *values {
                meta.extend((value.len() as u32).to_be_bytes());
                meta.extend(value.as_bytes());
            }
        }
        boxed(b"meta", &meta)
    }

    #[test]
    fn covers_are_read_and_kept_byte_for_byte() {
        let mut cvrx = [0u32.to_be_bytes(), 2u32.to_be_bytes()].concat();
        for (mime, kind, data) in [
            ("image/jpeg", 3u16, &b"\xFF\xD8\xFFjpeg"[..]),
            ("", 4, b"png"),
        ] {
            cvrx.extend((mime.len() as u16).to_be_bytes());
            cvrx.extend(mime.as_bytes());
            cvrx.extend(kind.to_be_bytes());
            cvrx.extend((data.len() as u32).to_be_bytes());
            cvrx.extend(data);
        }
        let body = [meta_body(&[(0x80, "album", &["A"])]), boxed(b"cvrx", &cvrx)].concat();
        let mut tags = NeroTags::parse(&body).unwrap();
        assert_eq!(
            tags.covers(),
            [
                Cover {
                    mime: "image/jpeg".into(),
                    kind: 3,
                    data: b"\xFF\xD8\xFFjpeg".to_vec()
                },
                Cover {
                    mime: String::new(),
                    kind: 4,
                    data: b"png".to_vec()
                }
            ]
        );
        assert_eq!(tags.render(), body);
        // A truncated or non-zero-version box has no covers.
        let short = [meta_body(&[]), boxed(b"cvrx", &cvrx[..cvrx.len() - 1])].concat();
        assert!(NeroTags::parse(&short).unwrap().covers().is_empty());
        let other = [meta_body(&[]), boxed(b"cvrx", &[0, 0, 0, 1, 0, 0, 0, 0])].concat();
        assert!(NeroTags::parse(&other).unwrap().covers().is_empty());
        assert!(tags.remove_covers());
        assert!(tags.covers().is_empty());
        assert_eq!(tags.render(), meta_body(&[(0x80, "album", &["A"])]));
        assert!(!tags.remove_covers());
    }

    #[test]
    fn file_fields_follow_edits_and_removals_of_fields_they_hold() {
        let body = meta_body(&[
            (0x80, "album", &["Old"]),
            (0x80, "date", &["1999"]),
            (0x80, "lyrics", &["la"]),
            (0x81, "secret", &["x"]),
        ]);
        let mut tags = NeroTags::parse(&body).unwrap();
        let updates = Fields::from([
            field("ALBUM", &["New", "Two"]),
            field("YEAR", &[""]),
            field("ARTIST", &["Not added"]),
        ]);
        assert!(tags.sync_global(&updates).unwrap());
        let global = tags.global_fields();
        assert_eq!(global["ALBUM"], ["New", "Two"]);
        assert!(!global.contains_key("YEAR") && !global.contains_key("ARTIST"));
        assert_eq!(global["UNSYNCEDLYRICS"], ["la"]);
        assert_eq!(
            tags.render(),
            meta_body(&[
                (0x80, "album", &["New", "Two"]),
                (0x80, "lyrics", &["la"]),
                (0x81, "secret", &["x"]),
            ])
        );
        // Unchanged values and absent fields change nothing.
        assert!(!tags
            .sync_global(&Fields::from([field("ALBUM", &["New", "Two"])]))
            .unwrap());
        assert!(!NeroTags::default().sync_global(&updates).unwrap());
        assert!(tags
            .sync_global(&Fields::from([field("ALBUM", &["a\0b"])]))
            .is_err());
        // Limits only apply to fields Nero holds.
        let long = "x".repeat(VALUE_LIMIT + 1);
        assert!(!tags
            .sync_global(&Fields::from([field("COMMENT", &[long.as_str()])]))
            .unwrap());
        assert!(tags
            .sync_global(&Fields::from([field("ALBUM", &[long.as_str()])]))
            .is_err());
    }
}
