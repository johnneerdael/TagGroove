//! Retain the v2.3 extensions documented by Mp3tag. Lofty's downgrade writer
//! drops several of those identifiers; serialize their frames without dropping
//! them and replace only the metadata region in the staged audio file.
use crate::batch_errors::ErrorCause;
use lofty::{
    config::WriteOptions,
    file::FileType,
    id3::v2::{Frame, Id3v2Tag, Id3v2TagFlags},
    io::FileLike,
    tag::TagExt,
    TextEncoding,
};
use std::{
    fs::File,
    io::{Read, Seek, SeekFrom, Write},
};

fn syncsafe(bytes: &[u8]) -> Result<u32, String> {
    if bytes.len() != 4 || bytes.iter().any(|v| v & 0x80 != 0) {
        return Err("Invalid ID3 syncsafe size".into());
    }
    Ok(bytes.iter().fold(0u32, |v, b| (v << 7) | u32::from(*b)))
}
fn syncsafe_bytes(value: usize) -> Result<[u8; 4], String> {
    let v = u32::try_from(value).map_err(|_| "ID3 tag exceeds the size limit")?;
    if v > 0x0fff_ffff {
        return Err("ID3 tag exceeds the syncsafe size limit".into());
    }
    Ok([
        ((v >> 21) & 127) as u8,
        ((v >> 14) & 127) as u8,
        ((v >> 7) & 127) as u8,
        (v & 127) as u8,
    ])
}
fn encoding(encoding: &mut TextEncoding, latin1: bool) {
    if latin1 {
        *encoding = TextEncoding::Latin1;
    } else if matches!(encoding, TextEncoding::UTF8 | TextEncoding::UTF16BE) {
        *encoding = TextEncoding::UTF16;
    }
}
fn prepare(frame: &mut Frame<'_>, latin1: bool) -> Result<(), String> {
    let flags = frame.flags();
    if flags.compression || flags.encryption.is_some() || flags.data_length_indicator.is_some() {
        return Err("Rewriting compressed/encrypted ID3v2.3 frames is not yet supported; original file preserved".into());
    }
    match frame {
        Frame::Text(v) => encoding(&mut v.encoding, latin1),
        Frame::UserText(v) => encoding(&mut v.encoding, latin1),
        Frame::UserUrl(v) => encoding(&mut v.encoding, latin1),
        Frame::Comment(v) => encoding(&mut v.encoding, latin1),
        Frame::UnsynchronizedText(v) => encoding(&mut v.encoding, latin1),
        Frame::Picture(v) => encoding(&mut v.encoding, latin1),
        Frame::KeyValue(v) => encoding(&mut v.encoding, latin1),
        Frame::Timestamp(v) => encoding(&mut v.encoding, latin1),
        Frame::Ownership(v) => encoding(&mut v.encoding, latin1),
        Frame::Chapter(v) => {
            let mut error = None;
            v.children.retain_mut(|v| {
                if let Err(e) = prepare(v, latin1) {
                    error = Some(e);
                }
                true
            });
            if let Some(error) = error {
                return Err(error);
            }
        }
        Frame::TableOfContents(v) => {
            let mut error = None;
            v.children.retain_mut(|v| {
                if let Err(e) = prepare(v, latin1) {
                    error = Some(e);
                }
                true
            });
            if let Some(error) = error {
                return Err(error);
            }
        }
        _ => {}
    }
    Ok(())
}
fn latin1(value: &str) -> Result<Vec<u8>, String> {
    value
        .chars()
        .map(|c| {
            u8::try_from(u32::from(c))
                .map_err(|_| "ID3 URL content must be Latin-1 or percent-encoded".into())
        })
        .collect()
}
fn user_url(frame: &lofty::id3::v2::ExtendedUrlFrame<'_>) -> Result<Vec<u8>, String> {
    let mut output = if frame.encoding == TextEncoding::Latin1 {
        let mut v = vec![0];
        v.extend(latin1(&frame.description)?);
        v.push(0);
        v
    } else {
        let mut v = vec![1, 255, 254];
        for c in frame.description.encode_utf16() {
            v.extend(c.to_le_bytes());
        }
        v.extend([0, 0]);
        v
    };
    output.extend(latin1(&frame.content)?);
    Ok(output)
}
fn convert_frames(
    bytes: &[u8],
    urls: &[Vec<u8>],
    url_index: &mut usize,
) -> Result<(Vec<u8>, usize), String> {
    let mut result = Vec::new();
    let mut position = 0;
    while position + 10 <= bytes.len() && bytes[position..position + 4] != [0, 0, 0, 0] {
        let mut id: [u8; 4] = bytes[position..position + 4]
            .try_into()
            .map_err(|_| "Invalid frame ID")?;
        let size = syncsafe(&bytes[position + 4..position + 8])? as usize;
        let end = position
            .checked_add(10 + size)
            .ok_or("ID3 frame size overflow")?;
        if end > bytes.len() {
            return Err("ID3 frame exceeds metadata bounds".into());
        }
        let flags = u16::from_be_bytes([bytes[position + 8], bytes[position + 9]]);
        if flags & 0x000f != 0 {
            return Err("Unsupported v2.4-only flags in v2.3 serialization".into());
        }
        let mut payload = bytes[position + 10..end].to_vec();
        if id == *b"WXXX" {
            payload = urls
                .get(*url_index)
                .cloned()
                .ok_or("Missing URL frame payload")?;
            *url_index += 1;
        }
        if id == *b"CHAP" || id == *b"CTOC" {
            let nul = payload
                .iter()
                .position(|b| *b == 0)
                .ok_or("Chapter element ID is not terminated")?;
            let children = if id == *b"CHAP" {
                nul.checked_add(17).ok_or("Chapter offset overflow")?
            } else {
                if nul + 3 > payload.len() {
                    return Err("Truncated table of contents".into());
                }
                let count = payload[nul + 2];
                let mut p = nul + 3;
                for _ in 0..count {
                    let n = payload[p..]
                        .iter()
                        .position(|b| *b == 0)
                        .ok_or("Unterminated child chapter ID")?;
                    p += n + 1;
                }
                p
            };
            if children > payload.len() {
                return Err("Truncated chapter fields".into());
            }
            let (converted, padding) = convert_frames(&payload[children..], urls, url_index)?;
            payload.truncate(children);
            payload.extend(converted);
            payload.resize(payload.len() + padding, 0);
        }
        if id == *b"TIPL" {
            id = *b"IPLS";
        }
        let mut v3_flags = 0u16;
        if flags & 0x4000 != 0 {
            v3_flags |= 0x8000;
        }
        if flags & 0x2000 != 0 {
            v3_flags |= 0x4000;
        }
        if flags & 0x1000 != 0 {
            v3_flags |= 0x2000;
        }
        if flags & 0x0040 != 0 {
            v3_flags |= 0x0020;
        }
        result.extend(id);
        result.extend(
            u32::try_from(payload.len())
                .map_err(|_| "Frame too large")?
                .to_be_bytes(),
        );
        result.extend(v3_flags.to_be_bytes());
        result.extend(payload);
        position = end;
    }
    if bytes[position..].iter().any(|b| *b != 0) {
        return Err("Unexpected bytes after ID3 frame data".into());
    }
    Ok((result, bytes.len() - position))
}
fn collect_urls(frame: &Frame<'_>, output: &mut Vec<Vec<u8>>) -> Result<(), String> {
    match frame {
        Frame::UserUrl(v) => output.push(user_url(v)?),
        Frame::Chapter(v) => {
            for f in v.children.iter() {
                collect_urls(f, output)?;
            }
        }
        Frame::TableOfContents(v) => {
            for f in v.children.iter() {
                collect_urls(f, output)?;
            }
        }
        _ => {}
    }
    Ok(())
}
fn crc32(bytes: &[u8]) -> u32 {
    let mut crc = 0xffff_ffffu32;
    for b in bytes {
        crc ^= u32::from(*b);
        for _ in 0..8 {
            crc = (crc >> 1) ^ if crc & 1 != 0 { 0xedb8_8320 } else { 0 };
        }
    }
    !crc
}
fn unsync(bytes: &[u8]) -> Vec<u8> {
    let mut output = Vec::with_capacity(bytes.len());
    for (i, b) in bytes.iter().enumerate() {
        output.push(*b);
        if *b == 255 && bytes.get(i + 1).is_none_or(|n| *n == 0 || *n >= 224) {
            output.push(0);
        }
    }
    output
}

/// Serialize `tag` as ID3v2.3 with exactly `padding` bytes of padding, or
/// lofty's default amount when `None`.
#[cfg(test)]
pub(super) fn serialize_padded(
    tag: &Id3v2Tag,
    latin1: bool,
    padding: Option<u32>,
) -> Result<Vec<u8>, String> {
    serialize_padded_diagnostic(tag, latin1, padding).map_err(|error| error.to_string())
}

pub(super) fn serialize_padded_diagnostic(
    tag: &Id3v2Tag,
    latin1: bool,
    padding: Option<u32>,
) -> Result<Vec<u8>, ErrorCause> {
    let flags = *tag.flags();
    if flags.crc && flags.unsynchronisation {
        return Err(ErrorCause::owned("Combined CRC and unsynchronisation in ID3v2.3 needs further verification; original file preserved"));
    }
    let mut tag = tag.clone();
    tag.set_flags(Id3v2TagFlags::default());
    let mut failure = None;
    tag.retain_mut(|frame| {
        if let Err(error) = prepare(frame, latin1).map_err(ErrorCause::owned_text) {
            failure = Some(error);
        }
        true
    });
    if let Some(error) = failure {
        return Err(error);
    }
    let mut urls = Vec::new();
    for frame in &tag {
        collect_urls(frame, &mut urls).map_err(ErrorCause::owned_text)?;
    }
    if tag.is_empty() {
        // An ID3v2 tag without frames is removed rather than written.
        return Ok(Vec::new());
    }
    let mut bytes = Vec::new();
    tag.dump_to(&mut bytes, WriteOptions::default())
        .map_err(ErrorCause::external)?;
    if bytes.len() < 10 || &bytes[..3] != b"ID3" {
        return Err(ErrorCause::owned("Invalid ID3 serialization header"));
    }
    let end = 10usize + syncsafe(&bytes[6..10]).map_err(ErrorCause::owned_text)? as usize;
    if end > bytes.len() {
        return Err(ErrorCause::owned("Serialized ID3 size exceeds buffer"));
    }
    let (frames, default_padding) =
        convert_frames(&bytes[10..end], &urls, &mut 0).map_err(ErrorCause::owned_text)?;
    let padding = padding.map_or(default_padding, |p| p as usize);
    let mut body = Vec::new();
    let mut header_flags = if flags.experimental { 0x20 } else { 0 };
    if flags.crc {
        header_flags |= 0x40;
        body.extend(10u32.to_be_bytes());
        body.extend(0x8000u16.to_be_bytes());
        body.extend(
            u32::try_from(padding)
                .map_err(|_| ErrorCause::owned("Padding too large"))?
                .to_be_bytes(),
        );
        body.extend(crc32(&frames).to_be_bytes());
    }
    body.extend(frames);
    body.resize(body.len() + padding, 0);
    if flags.unsynchronisation {
        header_flags |= 0x80;
        body = unsync(&body);
    }
    let mut output = b"ID3\x03\x00".to_vec();
    output.push(header_flags);
    output.extend(syncsafe_bytes(body.len()).map_err(ErrorCause::owned_text)?);
    output.extend(body);
    Ok(output)
}

/// Replace the ID3v2 tag of `file` with the serialized tag `bytes` (empty
/// removes it).
#[cfg(test)]
pub(super) fn save(file: &mut File, kind: FileType, bytes: &[u8]) -> Result<(), String> {
    save_diagnostic(file, kind, bytes).map_err(|error| error.to_string())
}

pub(super) fn save_diagnostic(
    file: &mut File,
    kind: FileType,
    bytes: &[u8],
) -> Result<(), ErrorCause> {
    file.seek(SeekFrom::Start(0))
        .map_err(ErrorCause::external)?;
    if matches!(kind, FileType::Mpeg | FileType::Aac) {
        let mut header = [0u8; 10];
        let read = file.read(&mut header).map_err(ErrorCause::external)?;
        let size = if read == 10 && &header[..3] == b"ID3" {
            10u64
                + u64::from(syncsafe(&header[6..10]).map_err(ErrorCause::owned_text)?)
                + if header[3] == 4 && header[5] & 0x10 != 0 {
                    10
                } else {
                    0
                }
        } else {
            0
        };
        replace_diagnostic(file, 0..size, bytes)?;
        return Ok(());
    }
    let little = kind == FileType::Wav;
    if !matches!(kind, FileType::Wav | FileType::Aiff) {
        return Err(ErrorCause::owned(
            "This format does not support writing ID3v2.3",
        ));
    }
    let length = file.metadata().map_err(ErrorCause::external)?.len();
    let mut position = 12u64;
    let mut ranges = Vec::new();
    while position + 8 <= length {
        file.seek(SeekFrom::Start(position))
            .map_err(ErrorCause::external)?;
        let mut header = [0u8; 8];
        file.read_exact(&mut header).map_err(ErrorCause::external)?;
        let size = if little {
            u32::from_le_bytes(
                header[4..8]
                    .try_into()
                    .map_err(|_| ErrorCause::owned("Invalid chunk size"))?,
            )
        } else {
            u32::from_be_bytes(
                header[4..8]
                    .try_into()
                    .map_err(|_| ErrorCause::owned("Invalid chunk size"))?,
            )
        };
        let end = position + 8 + u64::from(size) + u64::from(size % 2);
        if end > length {
            return Err(ErrorCause::owned(
                "Metadata chunk exceeds audio file bounds",
            ));
        }
        if header[..4] == *b"ID3 " || header[..4] == *b"id3 " {
            ranges.push(position..end);
        }
        position = end;
    }
    for range in ranges.iter().rev() {
        replace_diagnostic(file, range.clone(), &[])?;
    }
    if bytes.is_empty() {
        return write_container_size_diagnostic(file, little);
    }
    let mut chunk = b"ID3 ".to_vec();
    let size = u32::try_from(bytes.len()).map_err(|_| ErrorCause::owned("ID3 chunk too large"))?;
    chunk.extend(if little {
        size.to_le_bytes()
    } else {
        size.to_be_bytes()
    });
    chunk.extend_from_slice(bytes);
    if size % 2 != 0 {
        chunk.push(0);
    }
    let end = file.metadata().map_err(ErrorCause::external)?.len();
    replace_diagnostic(file, end..end, &chunk)?;
    write_container_size_diagnostic(file, little)
}

fn write_container_size_diagnostic(file: &mut File, little: bool) -> Result<(), ErrorCause> {
    let size = u32::try_from(
        file.metadata()
            .map_err(ErrorCause::external)?
            .len()
            .checked_sub(8)
            .ok_or_else(|| ErrorCause::owned("Invalid container size"))?,
    )
    .map_err(|_| ErrorCause::owned("Audio container exceeds its 32-bit size limit"))?;
    file.seek(SeekFrom::Start(4))
        .map_err(ErrorCause::external)?;
    file.write_all(&if little {
        size.to_le_bytes()
    } else {
        size.to_be_bytes()
    })
    .map_err(ErrorCause::external)?;
    Ok(())
}

fn replace_diagnostic(
    file: &mut File,
    range: std::ops::Range<u64>,
    bytes: &[u8],
) -> Result<(), ErrorCause> {
    let length = file.metadata().map_err(ErrorCause::external)?.len();
    if range.end == length {
        // Lofty 0.25.4's EOF shrink path truncates to the removed byte count.
        // Calculate the final absolute length ourselves for that path.
        file.seek(SeekFrom::Start(range.start))
            .map_err(ErrorCause::external)?;
        file.write_all(bytes).map_err(ErrorCause::external)?;
        file.set_len(range.start + bytes.len() as u64)
            .map_err(ErrorCause::external)
    } else {
        file.splice(range, &bytes).map_err(ErrorCause::external)
    }
}

#[cfg(test)]
mod diagnostic_tests {
    use super::*;

    #[test]
    fn typed_v23_guards_preserve_legacy_english_and_do_not_write() {
        let mut tag = Id3v2Tag::new();
        let mut flags = *tag.flags();
        flags.crc = true;
        flags.unsynchronisation = true;
        tag.set_flags(flags);
        let owned = serialize_padded_diagnostic(&tag, false, None).unwrap_err();
        assert!(matches!(&owned, ErrorCause::Owned { .. }));
        assert_eq!(
            serialize_padded(&tag, false, None).unwrap_err(),
            owned.to_string()
        );
        let mut file = tempfile::tempfile().unwrap();
        file.write_all(b"original").unwrap();
        let unsupported = save_diagnostic(&mut file, FileType::Flac, b"replacement").unwrap_err();
        assert!(
            matches!(&unsupported, ErrorCause::Owned { key, .. } if key == "This format does not support writing ID3v2.3")
        );
        assert_eq!(
            save(&mut file, FileType::Flac, b"replacement").unwrap_err(),
            unsupported.to_string()
        );
        file.seek(SeekFrom::Start(0)).unwrap();
        let mut after = Vec::new();
        file.read_to_end(&mut after).unwrap();
        assert_eq!(after, b"original");
    }
}
