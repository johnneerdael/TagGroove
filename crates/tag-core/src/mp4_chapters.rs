//! MP4 chapters: Nero chapters (`moov/udta/chpl`, 100 ns units) and
//! QuickTime chapter tracks (a text track named by a `chap` track
//! reference). They are listed read-only as `_CHAPTERS` values
//! ("HH:MM:SS.mmm Title") with `_SUBSONG_COUNT`, like Mp3tag's subsong fields.
//! Utils → Strip chapters MP4 turns every chapter box into a `free` box of the
//! same size, so no sample offset moves and the media is never rewritten.
use super::*;
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
    /// Start in milliseconds and title.
    pub list: Vec<(u64, String)>,
    boxes: Vec<Atom>,
}

/// Nero `chpl`: version, flags, (version 1: 4 reserved bytes), a count and
/// per chapter a 64-bit start in 100 ns units and a length-prefixed title.
fn nero(bytes: &[u8]) -> Vec<(u64, String)> {
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
        list.push((start / 10_000, String::from_utf8_lossy(title).into_owned()));
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
            .chunks_exact(2)
            .map(|c| u16::from_be_bytes([c[0], c[1]]))
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
    let mut file = fs::File::open(path).map_err(|e| e.to_string())?;
    let length = file.metadata().map_err(|e| e.to_string())?.len();
    let mut chapters = Chapters::default();
    let Some(moov) = atoms(&mut file, 0..length)?
        .into_iter()
        .find(|a| &a.kind == b"moov")
    else {
        return Ok(chapters);
    };
    let top = atoms(&mut file, moov.body..moov.end)?;
    for udta in top.iter().filter(|a| &a.kind == b"udta") {
        if let Some(chpl) = child(&mut file, udta, b"chpl")? {
            if chapters.list.is_empty() {
                chapters.list = nero(&body(&mut file, &chpl)?);
            }
            chapters.boxes.push(chpl);
        }
    }
    let mut referenced = Vec::new();
    let mut tracks = Vec::new();
    for trak in top.iter().filter(|a| &a.kind == b"trak") {
        let id = match child(&mut file, trak, b"tkhd")? {
            Some(tkhd) => {
                let bytes = body(&mut file, &tkhd)?;
                if bytes.first() == Some(&1) {
                    be32(&bytes, 20)
                } else {
                    be32(&bytes, 12)
                }
            }
            None => None,
        };
        if let Some(tref) = child(&mut file, trak, b"tref")? {
            let references = atoms(&mut file, tref.body..tref.end)?;
            for chap in references.iter().filter(|a| &a.kind == b"chap") {
                let ids = body(&mut file, chap)?;
                referenced.extend(
                    ids.chunks_exact(4)
                        .map(|c| u32::from_be_bytes(c.try_into().expect("4"))),
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
            if chapters.list.is_empty() {
                chapters.list = text_track(&mut file, &trak)?;
            }
            chapters.boxes.push(trak);
        }
    }
    Ok(chapters)
}

/// `_CHAPTERS` and `_SUBSONG_COUNT` for an MP4 file with chapters. Files
/// whose chapter boxes cannot be read simply show none.
pub(super) fn technical(path: &Path) -> Fields {
    let mut fields = Fields::new();
    let Ok(chapters) = scan(path) else {
        return fields;
    };
    if chapters.list.is_empty() {
        return fields;
    }
    fields.insert(
        "_SUBSONG_COUNT".into(),
        vec![chapters.list.len().to_string()],
    );
    fields.insert(
        "_CHAPTERS".into(),
        chapters
            .list
            .iter()
            .map(|(ms, title)| {
                let time = format!(
                    "{:02}:{:02}:{:02}.{:03}",
                    ms / 3_600_000,
                    ms / 60_000 % 60,
                    ms / 1000 % 60,
                    ms % 1000
                );
                if title.is_empty() {
                    time
                } else {
                    format!("{time} {title}")
                }
            })
            .collect(),
    );
    fields
}

/// Turn the chapter boxes of `original` into `free` boxes in `staging` and
/// check that everything else is unchanged.
pub(super) fn strip(
    staging: &mut fs::File,
    original: &Path,
    staged_path: &Path,
) -> Result<(), String> {
    let mut boxes = scan(original)?.boxes;
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
            nero(&bytes),
            [(0, "One".to_string()), (1500, "Two".to_string())]
        );
        // Truncated data keeps what was complete.
        assert_eq!(nero(&bytes[..bytes.len() - 1]).len(), 1);
        assert_eq!(text_sample(b"\x00\x03abc\x00\x00"), "abc");
        assert_eq!(text_sample(b"\x00\x04\xFE\xFF\x00A"), "A");
    }
}
