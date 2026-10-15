//! MP4 chapters: Nero chapters (`moov/udta/chpl`, 100 ns units) and
//! QuickTime chapter tracks (a text track named by a `chap` track
//! reference). They are listed as the CHAPTERS field, one "HH:MM:SS.mmm
//! Title" value per chapter, with `_SUBSONG_COUNT`. Writing CHAPTERS replaces
//! both kinds (see `mp4_layout::write_chapters`); an empty CHAPTERS, like
//! Utils → Strip chapters MP4, turns every chapter box into a `free` box of
//! the same size, so no sample offset moves and the media is never rewritten.
//! The same box reader supplies the `mvhd` timestamps and video properties.
use super::{nero_tags::NeroTags, *};
use lofty::picture::MimeType;
use std::{
    io::{Read, Write},
    ops::Range,
};

/// Bound for the boxes and samples read while looking for chapters.
const BODY_LIMIT: u64 = 16 << 20;
const SAMPLE_LIMIT: usize = 10_000;

#[derive(Debug, Clone)]
struct Atom {
    kind: [u8; 4],
    start: u64,
    body: u64,
    end: u64,
}

fn read_bytes(file: &mut fs::File, at: u64, length: u64) -> Result<Vec<u8>, String> {
    if length > BODY_LIMIT {
        return Err("MP4 chapter data is too large".into());
    }
    let mut bytes = vec![0u8; length as usize];
    file.seek(SeekFrom::Start(at)).map_err(|e| e.to_string())?;
    file.read_exact(&mut bytes).map_err(|e| e.to_string())?;
    Ok(bytes)
}

/// The boxes in `range`, read header by header. A malformed box ends the list.
fn atoms(file: &mut fs::File, range: Range<u64>) -> Result<Vec<Atom>, String> {
    let mut output = Vec::new();
    let mut at = range.start;
    while at + 8 <= range.end {
        let header = read_bytes(file, at, (range.end - at).min(16))?;
        let size = u64::from(u32::from_be_bytes(header[..4].try_into().expect("4")));
        let kind: [u8; 4] = header[4..8].try_into().expect("4");
        let (body, end) = match size {
            0 => (at + 8, range.end),
            1 if header.len() == 16 => (
                at + 16,
                at.saturating_add(u64::from_be_bytes(header[8..16].try_into().expect("8"))),
            ),
            1 => break,
            size => (at + 8, at + size),
        };
        if end < body || end > range.end {
            break;
        }
        output.push(Atom {
            kind,
            start: at,
            body,
            end,
        });
        at = end;
    }
    Ok(output)
}

fn child(file: &mut fs::File, parent: &Atom, kind: &[u8; 4]) -> Result<Option<Atom>, String> {
    Ok(atoms(file, parent.body..parent.end)?
        .into_iter()
        .find(|a| &a.kind == kind))
}

fn body(file: &mut fs::File, atom: &Atom) -> Result<Vec<u8>, String> {
    read_bytes(file, atom.body, atom.end - atom.body)
}

fn be32(bytes: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_be_bytes(bytes.get(at..at + 4)?.try_into().ok()?))
}
fn be64(bytes: &[u8], at: usize) -> Option<u64> {
    Some(u64::from_be_bytes(bytes.get(at..at + 8)?.try_into().ok()?))
}

/// Chapters found in one file, and the boxes that hold them.
#[derive(Debug, Default)]
pub(super) struct Chapters {
    /// Start in milliseconds and title: Nero chapters, else the track's.
    pub list: Vec<(u64, String)>,
    /// Nero starts in 100 ns units.
    nero: Vec<u64>,
    /// Chapters of the QuickTime chapter track.
    pub(super) track: Vec<(u64, String)>,
    boxes: Vec<Atom>,
    /// Nero tags (`udta/tags`), with the fields of each chapter.
    pub(super) nero_tags: Option<NeroTags>,
    nero_box: Option<Atom>,
}

/// Nero `chpl`: version, flags, (version 1: 4 reserved bytes), a count and
/// per chapter a 64-bit start in 100 ns units and a length-prefixed title.
fn nero_units(bytes: &[u8]) -> Vec<(u64, String)> {
    let mut list = Vec::new();
    let Some(version) = bytes.first() else {
        return list;
    };
    let mut at = if *version == 0 { 4 } else { 8 };
    let Some(count) = bytes.get(at) else {
        return list;
    };
    at += 1;
    for _ in 0..*count {
        let (Some(start), Some(length)) = (be64(bytes, at), bytes.get(at + 8)) else {
            break;
        };
        let Some(title) = bytes.get(at + 9..at + 9 + usize::from(*length)) else {
            break;
        };
        list.push((start, String::from_utf8_lossy(title).into_owned()));
        at += 9 + usize::from(*length);
    }
    list
}

/// A QuickTime text sample: a 16-bit length and UTF-8 (or BOM-marked UTF-16) text.
fn text_sample(bytes: &[u8]) -> String {
    let length = bytes
        .get(..2)
        .map_or(0, |b| usize::from(u16::from_be_bytes([b[0], b[1]])));
    let text = bytes.get(2..2 + length).unwrap_or(&[]);
    if let Some(utf16) = text.strip_prefix(&[0xFE, 0xFF]) {
        let units: Vec<u16> = utf16
            .as_chunks::<2>()
            .0
            .iter()
            .map(|c| u16::from_be_bytes(*c))
            .collect();
        return String::from_utf16_lossy(&units);
    }
    String::from_utf8_lossy(text).into_owned()
}

/// Chapters of a QuickTime text track: sample times from `stts` in the
/// `mdhd` time scale, sample positions from `stsz`, `stsc` and `stco`/`co64`.
fn text_track(file: &mut fs::File, trak: &Atom) -> Result<Vec<(u64, String)>, String> {
    let invalid = || String::from("Invalid MP4 chapter track");
    let mdia = child(file, trak, b"mdia")?.ok_or_else(invalid)?;
    let mdhd = child(file, &mdia, b"mdhd")?.ok_or_else(invalid)?;
    let mdhd = body(file, &mdhd)?;
    let scale = u64::from(
        if mdhd.first() == Some(&1) {
            be32(&mdhd, 20)
        } else {
            be32(&mdhd, 12)
        }
        .filter(|s| *s > 0)
        .ok_or_else(invalid)?,
    );
    let minf = child(file, &mdia, b"minf")?.ok_or_else(invalid)?;
    let stbl = child(file, &minf, b"stbl")?.ok_or_else(invalid)?;
    let table = |file: &mut fs::File, kind: &[u8; 4]| -> Result<Option<Vec<u8>>, String> {
        child(file, &stbl, kind)?
            .map(|atom| body(file, &atom))
            .transpose()
    };
    let stts = table(file, b"stts")?.ok_or_else(invalid)?;
    let stsz = table(file, b"stsz")?.ok_or_else(invalid)?;
    let stsc = table(file, b"stsc")?.ok_or_else(invalid)?;
    let (offsets, wide) = match table(file, b"stco")? {
        Some(stco) => (stco, false),
        None => (table(file, b"co64")?.ok_or_else(invalid)?, true),
    };

    let mut times = Vec::new();
    let mut time = 0u64;
    for entry in 0..be32(&stts, 4).unwrap_or(0) as usize {
        let at = 8 + entry * 8;
        let (Some(count), Some(delta)) = (be32(&stts, at), be32(&stts, at + 4)) else {
            break;
        };
        for _ in 0..count {
            if times.len() >= SAMPLE_LIMIT {
                break;
            }
            times.push(time);
            time += u64::from(delta);
        }
    }
    let uniform = be32(&stsz, 4).unwrap_or(0);
    let sample_count = (be32(&stsz, 8).unwrap_or(0) as usize).min(SAMPLE_LIMIT);
    let size = |index: usize| {
        if uniform > 0 {
            Some(uniform)
        } else {
            be32(&stsz, 12 + index * 4)
        }
    };
    let chunk_count = be32(&offsets, 4).unwrap_or(0) as usize;
    let chunk_offset = |index: usize| {
        if wide {
            be64(&offsets, 8 + index * 8)
        } else {
            be32(&offsets, 8 + index * 4).map(u64::from)
        }
    };
    let runs: Vec<(usize, u32)> = (0..be32(&stsc, 4).unwrap_or(0) as usize)
        .map_while(|entry| {
            let at = 8 + entry * 12;
            Some((be32(&stsc, at)? as usize, be32(&stsc, at + 4)?))
        })
        .collect();

    let mut list = Vec::new();
    let mut sample = 0usize;
    for chunk in 0..chunk_count {
        let per_chunk = runs
            .iter()
            .rev()
            .find(|(first, _)| *first <= chunk + 1)
            .map_or(0, |(_, count)| *count);
        let Some(mut offset) = chunk_offset(chunk) else {
            break;
        };
        for _ in 0..per_chunk {
            if sample >= sample_count || sample >= times.len() {
                break;
            }
            let length = u64::from(size(sample).ok_or_else(invalid)?);
            let bytes = read_bytes(file, offset, length.min(2 + u64::from(u16::MAX)))?;
            list.push((times[sample] * 1000 / scale, text_sample(&bytes)));
            offset += length;
            sample += 1;
        }
    }
    Ok(list)
}

/// Find the chapters of an MP4 file. Nero chapters take precedence; a
/// QuickTime chapter track is read when there are none.
pub(super) fn scan(path: &Path) -> Result<Chapters, String> {
    scan_file(&mut fs::File::open(path).map_err(|e| e.to_string())?)
}

/// [`scan`] on an open file.
fn scan_file(file: &mut fs::File) -> Result<Chapters, String> {
    let length = file.metadata().map_err(|e| e.to_string())?.len();
    let mut chapters = Chapters::default();
    let Some(moov) = atoms(file, 0..length)?
        .into_iter()
        .find(|a| &a.kind == b"moov")
    else {
        return Ok(chapters);
    };
    let top = atoms(file, moov.body..moov.end)?;
    for udta in top.iter().filter(|a| &a.kind == b"udta") {
        if let Some(chpl) = child(file, udta, b"chpl")? {
            if chapters.list.is_empty() {
                let units = nero_units(&body(file, &chpl)?);
                chapters.nero = units.iter().map(|(start, _)| *start).collect();
                chapters.list = units
                    .into_iter()
                    .map(|(start, title)| (start / 10_000, title))
                    .collect();
            }
            chapters.boxes.push(chpl);
        }
        if let Some(tags) = child(file, udta, b"tags")? {
            if chapters.nero_tags.is_none() {
                chapters.nero_tags = NeroTags::parse(&body(file, &tags)?);
                chapters.nero_box = Some(tags);
            }
        }
    }
    let mut referenced = Vec::new();
    let mut tracks = Vec::new();
    for trak in top.iter().filter(|a| &a.kind == b"trak") {
        let id = match child(file, trak, b"tkhd")? {
            Some(tkhd) => {
                let bytes = body(file, &tkhd)?;
                if bytes.first() == Some(&1) {
                    be32(&bytes, 20)
                } else {
                    be32(&bytes, 12)
                }
            }
            None => None,
        };
        if let Some(tref) = child(file, trak, b"tref")? {
            let references = atoms(file, tref.body..tref.end)?;
            for chap in references.iter().filter(|a| &a.kind == b"chap") {
                let ids = body(file, chap)?;
                referenced.extend(
                    ids.as_chunks::<4>()
                        .0
                        .iter()
                        .map(|c| u32::from_be_bytes(*c)),
                );
                // Blank the whole reference box when it only names chapters.
                if references.iter().all(|a| &a.kind == b"chap") {
                    chapters.boxes.push(tref.clone());
                    break;
                }
                chapters.boxes.push(chap.clone());
            }
        }
        tracks.push((id, trak.clone()));
    }
    for (id, trak) in tracks {
        if id.is_some_and(|id| referenced.contains(&id)) {
            if chapters.track.is_empty() {
                chapters.track = text_track(file, &trak)?;
            }
            chapters.boxes.push(trak);
        }
    }
    if chapters.list.is_empty() {
        chapters.list = chapters.track.clone();
    }
    Ok(chapters)
}

/// The CHAPTERS field, `_SUBSONG_COUNT` and the chapter list of an MP4
/// file, with the fields of each chapter from Nero tags. File-wide Nero
/// fields fill in fields the iTunes tag does not have, and `_TAG` names the
/// Nero tags after the iTunes tag, as Mp3tag lists them. Files whose chapter
/// boxes cannot be read simply show none.
pub(super) fn describe(path: &Path, entry: &mut AudioEntry) {
    let Ok(chapters) = scan(path) else {
        return;
    };
    if let Some(nero) = &chapters.nero_tags {
        for (name, values) in nero.global_fields() {
            if !entry.fields.keys().any(|k| k.eq_ignore_ascii_case(&name)) {
                entry.fields.insert(name, values);
            }
        }
        let tags = entry.technical.entry("_TAG".into()).or_default();
        match tags.first_mut() {
            Some(first) if !first.is_empty() => first.push_str(" Nero"),
            _ => *tags = vec!["Nero".into()],
        }
    }
    if chapters.list.is_empty() {
        return;
    }
    entry.fields.insert(
        subsongs::CHAPTERS_FIELD.into(),
        chapters
            .list
            .iter()
            .map(|(ms, title)| subsongs::chapter_value(ms * 1_000_000, title))
            .collect(),
    );
    let nero = chapters.nero_tags;
    subsongs::attach(
        entry,
        chapters
            .list
            .into_iter()
            .enumerate()
            .map(|(index, (start, title))| {
                let mut fields = nero
                    .as_ref()
                    .map(|n| n.chapter_fields(index))
                    .unwrap_or_default();
                fields.remove("TITLE");
                ChapterInfo {
                    start,
                    end: None,
                    title,
                    fields,
                    source: subsongs::ChapterSource::Mp4,
                }
            })
            .collect(),
    );
}

/// The covers of the Nero tags (`cvrx`) as pictures. The picture type is
/// the cover's 16-bit type; a cover without a MIME type is identified by
/// its bytes.
fn nero_pictures(chapters: &Chapters) -> Vec<Picture> {
    let Some(nero) = &chapters.nero_tags else {
        return Vec::new();
    };
    nero.covers()
        .into_iter()
        .map(|cover| {
            let mime = if cover.mime.is_empty() {
                let data = &cover.data;
                if data.starts_with(&[0xFF, 0xD8, 0xFF]) {
                    Some(MimeType::Jpeg)
                } else if data.starts_with(b"\x89PNG") {
                    Some(MimeType::Png)
                } else if data.starts_with(b"GIF8") {
                    Some(MimeType::Gif)
                } else if data.starts_with(b"BM") {
                    Some(MimeType::Bmp)
                } else {
                    None
                }
            } else {
                Some(MimeType::from_str(&cover.mime))
            };
            let kind = u8::try_from(cover.kind).map_or(PictureType::Other, PictureType::from_u8);
            let picture = Picture::unchecked(cover.data).pic_type(kind);
            match mime {
                Some(mime) => picture.mime_type(mime).build(),
                None => picture.build(),
            }
        })
        .collect()
}

/// Nero covers stand in for the iTunes covers of an MP4 file whose iTunes
/// tag has none, so they are shown, and edited covers are written to the
/// iTunes tag (after which [`drop_nero_covers`] removes the Nero ones).
pub(super) fn add_nero_covers(file: &mut TaggedFile, path: &Path) {
    if file.file_type() != FileType::Mp4
        || file
            .tag(TagType::Mp4Ilst)
            .is_some_and(|tag| !tag.pictures().is_empty())
    {
        return;
    }
    let pictures = scan(path).map(|c| nero_pictures(&c)).unwrap_or_default();
    if pictures.is_empty() {
        return;
    }
    if file.tag(TagType::Mp4Ilst).is_none() {
        file.insert_tag(Tag::new(TagType::Mp4Ilst));
    }
    if let Some(tag) = file.tag_mut(TagType::Mp4Ilst) {
        for picture in pictures {
            tag.push_picture(picture);
        }
    }
}

/// Write the Nero tags `nero` over the `tags` box `atom` when they fit,
/// with a `free` box after them for the bytes left over. Nothing else moves.
fn replace_in_place(file: &mut fs::File, atom: &Atom, nero: &NeroTags) -> Result<bool, String> {
    let body = nero.render();
    let old = atom.end - atom.start;
    let new = 8 + body.len() as u64;
    if atom.body - atom.start != 8 || !(new == old || new + 8 <= old) {
        return Ok(false);
    }
    let mut bytes = ((new) as u32).to_be_bytes().to_vec();
    bytes.extend(b"tags");
    bytes.extend(&body);
    if new < old {
        bytes.extend(((old - new) as u32).to_be_bytes());
        bytes.extend(b"free");
        bytes.resize(old as usize, 0);
    }
    file.seek(SeekFrom::Start(atom.start))
        .and_then(|_| file.write_all(&bytes))
        .map_err(|e| e.to_string())?;
    crate::write_cost::rewrote_region(old);
    Ok(true)
}

/// Check that the staged file's Nero tags read back as `nero`.
fn verify_nero(file: &mut fs::File, nero: &NeroTags) -> Result<(), String> {
    let written = scan_file(file)?.nero_tags;
    if written.as_ref().map(NeroTags::render) != Some(nero.render()) {
        return Err(
            "The Nero tags did not read back as written; the original file was kept".into(),
        );
    }
    Ok(())
}

/// After covers were written to the iTunes tag, remove the Nero covers they
/// replace (see [`add_nero_covers`]). This only shrinks the `tags` box.
pub(super) fn drop_nero_covers(file: &mut fs::File) -> Result<(), String> {
    let chapters = scan_file(file)?;
    let (Some(mut nero), Some(atom)) = (chapters.nero_tags, chapters.nero_box) else {
        return Ok(());
    };
    if !nero.remove_covers() {
        return Ok(());
    }
    if !replace_in_place(file, &atom, &nero)? {
        return Err("The Nero covers could not be removed; the original file was kept".into());
    }
    verify_nero(file, &nero)
}

/// After file fields `updates` were written to the iTunes tag, apply them to
/// the file-wide Nero fields that hold them (see `NeroTags::sync_global`),
/// so a removed field does not come back from Nero. The `tags` box is
/// rewritten in place when it fits, otherwise with the chapters; a longer
/// value in a file without chapters stays only in the iTunes tag, which takes
/// precedence anyway.
pub(super) fn sync_nero_fields(
    staging: &mut fs::File,
    staged_path: &Path,
    updates: &Fields,
) -> Result<(), String> {
    let chapters = scan_file(staging)?;
    let (Some(mut nero), Some(atom)) = (chapters.nero_tags.clone(), chapters.nero_box.clone())
    else {
        return Ok(());
    };
    if !nero.sync_global(updates)? {
        return Ok(());
    }
    if replace_in_place(staging, &atom, &nero)? {
        return verify_nero(staging, &nero);
    }
    if chapters.list.is_empty() {
        return Ok(());
    }
    let values: Vec<String> = chapters
        .list
        .iter()
        .map(|(ms, title)| subsongs::chapter_value(ms * 1_000_000, title))
        .collect();
    write_with_tags(staging, staged_path, &values, Some(&nero))
}

/// Write CHAPTERS `values` into the staged MP4 file: Nero chapters and a
/// QuickTime chapter track, or no chapters at all when `values` is empty.
pub(super) fn write(
    staging: &mut fs::File,
    staged_path: &Path,
    values: &[String],
) -> Result<(), String> {
    write_with_tags(staging, staged_path, values, None)
}

/// [`write`], also replacing the Nero tags (`udta/tags`) by `nero`.
pub(super) fn write_with_tags(
    staging: &mut fs::File,
    staged_path: &Path,
    values: &[String],
    nero: Option<&NeroTags>,
) -> Result<(), String> {
    let mut requested = values
        .iter()
        .map(|value| subsongs::parse_chapter_value(value))
        .collect::<Result<Vec<_>, _>>()?;
    requested.sort_by_key(|(start, _)| *start);
    let current = scan(staged_path)?;
    if requested.is_empty() {
        if nero.is_some() {
            return Err("Nero chapter tags need chapters".into());
        }
        if !current.boxes.is_empty() {
            blank(staging, current.boxes)?;
        }
    } else {
        // Keep the stored start while it still displays the same.
        let chapters: Vec<(u64, String)> = requested
            .iter()
            .enumerate()
            .map(|(index, (ns, title))| {
                let units = ns / 100;
                let start = current
                    .nero
                    .get(index)
                    .copied()
                    .filter(|old| old / 10_000 == units / 10_000)
                    .unwrap_or(units);
                (start, title.clone())
            })
            .collect();
        let tags = nero.map(NeroTags::render);
        mp4_layout::write_chapters(staging, staged_path, &chapters, tags.as_deref())?;
    }
    staging.sync_all().map_err(|e| e.to_string())?;
    let written = scan(staged_path)?;
    let expected: Vec<(u64, String)> = requested
        .iter()
        .map(|(ns, title)| (ns / 1_000_000, title.clone()))
        .collect();
    // The QuickTime track starts at zero and counts in its own time scale.
    let track_matches = written.track.len() == expected.len()
        && written.track.iter().zip(&expected).enumerate().all(
            |(index, ((ms, title), (start, wanted)))| {
                title == wanted && (index == 0 || ms.abs_diff(*start) <= 1)
            },
        );
    let tags_match = nero
        .is_none_or(|nero| written.nero_tags.as_ref().map(NeroTags::render) == Some(nero.render()));
    if written.list != expected || !track_matches || !tags_match {
        return Err(
            "The MP4 chapters did not read back as written; the original file was kept".into(),
        );
    }
    Ok(())
}

/// Seconds between the MP4 epoch (1904-01-01) and the Unix epoch.
const MP4_EPOCH_OFFSET: i64 = 2_082_844_800;

/// `_MP4_CREATION_TIME` and `_MP4_MODIFICATION_TIME` (with `_RAW` Unix
/// seconds) from `mvhd`, and `_VIDEO_*` from the first video track. Values
/// that cannot be read are left out.
pub(super) fn header_technical(path: &Path) -> Fields {
    let mut fields = Fields::new();
    // A malformed box only hides the values after it.
    let _ = header_fields(path, &mut fields);
    fields
}

fn video_codec(kind: &[u8]) -> String {
    match kind {
        b"avc1" | b"avc3" => "H.264".into(),
        b"hvc1" | b"hev1" => "H.265".into(),
        b"mp4v" => "MPEG-4 Visual".into(),
        b"av01" => "AV1".into(),
        b"vp08" => "VP8".into(),
        b"vp09" => "VP9".into(),
        b"s263" | b"h263" => "H.263".into(),
        b"jpeg" | b"mjpa" | b"mjpb" => "Motion JPEG".into(),
        b"apcn" | b"apch" | b"apcs" | b"apco" | b"ap4h" | b"ap4x" => "Apple ProRes".into(),
        other => String::from_utf8_lossy(other).trim().to_owned(),
    }
}

fn header_fields(path: &Path, fields: &mut Fields) -> Result<(), String> {
    let mut file = fs::File::open(path).map_err(|e| e.to_string())?;
    let length = file.metadata().map_err(|e| e.to_string())?.len();
    let Some(moov) = atoms(&mut file, 0..length)?
        .into_iter()
        .find(|a| &a.kind == b"moov")
    else {
        return Ok(());
    };
    let top = atoms(&mut file, moov.body..moov.end)?;
    if let Some(mvhd) = top.iter().find(|a| &a.kind == b"mvhd") {
        let bytes = body(&mut file, mvhd)?;
        let (created, modified) = if bytes.first() == Some(&1) {
            (be64(&bytes, 4), be64(&bytes, 12))
        } else {
            (
                be32(&bytes, 4).map(u64::from),
                be32(&bytes, 8).map(u64::from),
            )
        };
        for (name, value) in [
            ("_MP4_CREATION_TIME", created),
            ("_MP4_MODIFICATION_TIME", modified),
        ] {
            // Zero means "not set"; it is not a 1904 date.
            if let Some(seconds) = value.filter(|v| *v > 0).and_then(|v| i64::try_from(v).ok()) {
                technical::datetime(fields, name, seconds - MP4_EPOCH_OFFSET);
            }
        }
    }
    for trak in top.iter().filter(|a| &a.kind == b"trak") {
        let Some(mdia) = child(&mut file, trak, b"mdia")? else {
            continue;
        };
        let Some(hdlr) = child(&mut file, &mdia, b"hdlr")? else {
            continue;
        };
        if body(&mut file, &hdlr)?.get(8..12) != Some(b"vide") {
            continue;
        }
        let tkhd = match child(&mut file, trak, b"tkhd")? {
            Some(tkhd) => body(&mut file, &tkhd)?,
            None => Vec::new(),
        };
        let stbl = match child(&mut file, &mdia, b"minf")? {
            Some(minf) => child(&mut file, &minf, b"stbl")?,
            None => None,
        };
        let table = |file: &mut fs::File, kind: &[u8; 4]| -> Result<Option<Vec<u8>>, String> {
            match &stbl {
                Some(stbl) => child(file, stbl, kind)?
                    .map(|atom| body(file, &atom))
                    .transpose(),
                None => Ok(None),
            }
        };
        let stsd = table(&mut file, b"stsd")?.unwrap_or_default();
        if let Some(kind) = stsd.get(12..16) {
            fields.insert("_VIDEO_CODEC".into(), vec![video_codec(kind)]);
        }
        // The visual sample entry's size, else the track header's 16.16 size.
        let sample = |at: usize| {
            stsd.get(at..at + 2)
                .map(|b| u32::from(u16::from_be_bytes([b[0], b[1]])))
        };
        let header = |back: usize| {
            tkhd.len()
                .checked_sub(back)
                .and_then(|at| be32(&tkhd, at))
                .map(|fixed| fixed >> 16)
        };
        for (name, value) in [
            ("_VIDEO_WIDTH", sample(40).filter(|v| *v > 0).or(header(8))),
            ("_VIDEO_HEIGHT", sample(42).filter(|v| *v > 0).or(header(4))),
        ] {
            if let Some(value) = value.filter(|v| *v > 0) {
                fields.insert(name.into(), vec![value.to_string()]);
            }
        }
        let mdhd = match child(&mut file, &mdia, b"mdhd")? {
            Some(mdhd) => body(&mut file, &mdhd)?,
            None => Vec::new(),
        };
        let (scale, duration) = if mdhd.first() == Some(&1) {
            (be32(&mdhd, 20), be64(&mdhd, 24))
        } else {
            (be32(&mdhd, 12), be32(&mdhd, 16).map(u64::from))
        };
        if let (Some(scale), Some(duration), Some(stsz)) = (
            scale.filter(|s| *s > 0),
            duration.filter(|d| *d > 0),
            table(&mut file, b"stsz")?,
        ) {
            let count = u64::from(be32(&stsz, 8).unwrap_or(0));
            let bytes = match be32(&stsz, 4).unwrap_or(0) {
                0 => stsz
                    .get(12..)
                    .unwrap_or_default()
                    .as_chunks::<4>()
                    .0
                    .iter()
                    .map(|size| u64::from(u32::from_be_bytes(*size)))
                    .sum(),
                uniform => u64::from(uniform) * count,
            };
            let seconds = duration as f64 / f64::from(scale);
            let kbps = (bytes as f64 * 8.0 / seconds / 1000.0).round() as u64;
            if kbps > 0 {
                fields.insert("_VIDEO_BITRATE".into(), vec![kbps.to_string()]);
            }
        }
        break;
    }
    Ok(())
}

/// Remove tag: turn the Nero tags of the staged file into a `free` box.
pub(super) fn strip_nero_tags(staging: &mut fs::File, staged_path: &Path) -> Result<(), String> {
    if let Some(tags) = scan(staged_path)?.nero_box {
        blank(staging, vec![tags])?;
    }
    if scan(staged_path)?.nero_tags.is_some() {
        return Err("The Nero tags could not be removed".into());
    }
    Ok(())
}

/// Turn the chapter boxes of `original` into `free` boxes in `staging` and
/// check that everything else is unchanged.
pub(super) fn strip(
    staging: &mut fs::File,
    original: &Path,
    staged_path: &Path,
) -> Result<(), String> {
    let blanked = blank(staging, scan(original)?.boxes)?;
    if !scan(staged_path)?.list.is_empty() {
        return Err("The MP4 chapters could not be removed".into());
    }
    // Only the type and body of each blanked box may differ.
    let length = fs::metadata(original).map_err(|e| e.to_string())?.len();
    let mut kept = Vec::new();
    let mut at = 0;
    for atom in &blanked {
        kept.push(at..atom.start + 4);
        at = atom.end;
    }
    kept.push(at..length);
    for range in kept {
        let start = range.start;
        if !container_formats::same_bytes(original, range, staged_path, start)? {
            return Err("The MP4 media changed while removing chapters".into());
        }
    }
    Ok(())
}

/// Turn chapter boxes into `free` boxes of the same size; returns the boxes
/// blanked, outermost only.
fn blank(staging: &mut fs::File, mut boxes: Vec<Atom>) -> Result<Vec<Atom>, String> {
    boxes.sort_by_key(|a| a.start);
    // A reference inside a removed track is covered by the track.
    let mut blanked: Vec<Atom> = Vec::new();
    for atom in boxes {
        if blanked.last().is_some_and(|last| atom.start < last.end) {
            continue;
        }
        blanked.push(atom);
    }
    let mut region = 0;
    for atom in &blanked {
        staging
            .seek(SeekFrom::Start(atom.start + 4))
            .map_err(|e| e.to_string())?;
        staging.write_all(b"free").map_err(|e| e.to_string())?;
        staging
            .seek(SeekFrom::Start(atom.body))
            .map_err(|e| e.to_string())?;
        let zeros = vec![0u8; (atom.end - atom.body) as usize];
        staging.write_all(&zeros).map_err(|e| e.to_string())?;
        region += atom.end - atom.start;
    }
    crate::write_cost::rewrote_region(region);
    staging.sync_all().map_err(|e| e.to_string())?;
    Ok(blanked)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nero_titles_and_times_are_parsed() {
        let mut bytes = vec![1, 0, 0, 0, 0, 0, 0, 0, 2];
        for (start, title) in [(0u64, "One"), (15_000_000, "Two")] {
            bytes.extend(start.to_be_bytes());
            bytes.push(title.len() as u8);
            bytes.extend(title.as_bytes());
        }
        assert_eq!(
            nero_units(&bytes),
            [(0, "One".to_string()), (15_000_000, "Two".to_string())]
        );
        // Truncated data keeps what was complete.
        assert_eq!(nero_units(&bytes[..bytes.len() - 1]).len(), 1);
        assert_eq!(text_sample(b"\x00\x03abc\x00\x00"), "abc");
        assert_eq!(text_sample(b"\x00\x04\xFE\xFF\x00A"), "A");
    }
}
