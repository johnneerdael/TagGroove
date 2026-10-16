//! Windows Media (ASF: .wma/.asf) files. Lofty has no ASF support, so
//! `header` decodes and rebuilds the Header Object and this module maps its
//! attributes onto the entry model shared by every other format. Writes only
//! replace the Header Object; the Data and Index objects are copied verbatim
//! and compared with the original before the staged file is accepted.
mod header;

use super::*;
use crate::batch_errors::ErrorCause;
use header::{Asf, Attribute, PictureValue, Value};
use lofty::picture::MimeType;
use std::io::{Read, Write};

const PICTURE: &str = "WM/Picture";
/// The track number older Windows Media writers stored zero-based. It is
/// shown as TRACK when the file has no WM/TrackNumber.
const LEGACY_TRACK: &str = "WM/Track";
const RATING: &str = "WM/SharedUserRating";
/// The values Windows Media Player stores for one to five stars.
const RATING_VALUES: [u32; 6] = [0, 1, 25, 50, 75, 99];
/// The tag name shown in %_tag% and accepted by user field mappings.
const TAG: &str = "WMA";

/// Whether the file starts with an ASF Header Object.
pub(super) fn sniff(path: &Path) -> bool {
    let mut start = [0u8; 16];
    fs::File::open(path)
        .and_then(|mut file| file.read_exact(&mut start))
        .is_ok()
        && header::is_asf(&start)
}

fn open(path: &Path) -> Result<(Asf, u64), String> {
    open_diagnostic(path).map_err(|error| error.to_string())
}

fn open_diagnostic(path: &Path) -> Result<(Asf, u64), ErrorCause> {
    let file = fs::File::open(path).map_err(ErrorCause::external)?;
    let size = file.metadata().map_err(ErrorCause::external)?.len();
    let asf = Asf::read_diagnostic(&mut std::io::BufReader::new(file), size)?;
    Ok((asf, size))
}

/// The Mp3tag field an attribute is shown as: the documented mapping, or the
/// native attribute name.
fn field_name(native: &str) -> String {
    mapping::asf_name(native).map_or_else(|| native.to_owned(), str::to_owned)
}

fn stars(rating: u32) -> u32 {
    match rating {
        0 => 0,
        1..=12 => 1,
        13..=37 => 2,
        38..=62 => 3,
        63..=86 => 4,
        _ => 5,
    }
}

fn format_guid(id: &header::Guid) -> String {
    let hex = |bytes: &[u8]| bytes.iter().map(|b| format!("{b:02X}")).collect::<String>();
    format!(
        "{{{:08X}-{:04X}-{:04X}-{}-{}}}",
        u32::from_le_bytes([id[0], id[1], id[2], id[3]]),
        u16::from_le_bytes([id[4], id[5]]),
        u16::from_le_bytes([id[6], id[7]]),
        hex(&id[8..10]),
        hex(&id[10..16])
    )
}
fn parse_guid(text: &str) -> Option<header::Guid> {
    let digits: String = text
        .strip_prefix('{')?
        .strip_suffix('}')?
        .chars()
        .filter(|c| *c != '-')
        .collect();
    let bytes = (0..16)
        .map(|i| u8::from_str_radix(digits.get(i * 2..i * 2 + 2)?, 16).ok())
        .collect::<Option<Vec<_>>>()?;
    let mut id: header::Guid = bytes.try_into().ok()?;
    id[0..4].reverse();
    id[4..6].reverse();
    id[6..8].reverse();
    (format_guid(&id) == text).then_some(id)
}

/// The text form of a value. Binary values and artwork are not text fields.
fn display(attribute: &Attribute) -> Option<String> {
    Some(match &attribute.value {
        Value::Unicode(text) => text.clone(),
        Value::Bool(flag) => u8::from(*flag).to_string(),
        Value::Dword(n) if attribute.name.eq_ignore_ascii_case(RATING) => stars(*n).to_string(),
        Value::Dword(n) => n.to_string(),
        Value::Qword(n) => n.to_string(),
        Value::Word(n) => n.to_string(),
        Value::Guid(id) => format_guid(id),
        Value::Bytes(_) | Value::Other { .. } => return None,
    })
}

/// Store text in the type the attribute already has. Text that would not
/// read back identically in that type is stored as a Unicode string.
fn parse_value(text: &str, template: Option<&Value>, rating: bool) -> Result<Value, String> {
    fn exact<T: std::str::FromStr + ToString>(text: &str) -> Option<T> {
        text.parse::<T>().ok().filter(|n| n.to_string() == text)
    }
    let value = match template {
        Some(Value::Dword(_)) | None if rating => {
            let half = mapping::parse_stars(text, false)?;
            Some(Value::Dword(RATING_VALUES[usize::from(half / 2)]))
        }
        Some(Value::Bool(_)) => match text {
            "0" => Some(Value::Bool(false)),
            "1" => Some(Value::Bool(true)),
            _ => None,
        },
        Some(Value::Dword(_)) => exact(text).map(Value::Dword),
        Some(Value::Qword(_)) => exact(text).map(Value::Qword),
        Some(Value::Word(_)) => exact(text).map(Value::Word),
        Some(Value::Guid(_)) => parse_guid(text).map(Value::Guid),
        _ => None,
    };
    Ok(value.unwrap_or_else(|| Value::Unicode(text.to_owned())))
}

fn is_picture(attribute: &Attribute) -> bool {
    picture_of(attribute).is_some()
}

/// Mp3tag field names and values, before user field mappings.
fn local_fields(asf: &Asf) -> Fields {
    let mut fields = Fields::new();
    let mut seen: std::collections::HashMap<(String, String), Option<header::Location>> =
        std::collections::HashMap::new();
    let legacy = |attribute: &Attribute| {
        attribute.name.eq_ignore_ascii_case(LEGACY_TRACK)
            && legacy_track(attribute).is_some()
            && !asf.attributes.iter().any(|other| {
                field_name(&other.name).eq_ignore_ascii_case("TRACK") && display(other).is_some()
            })
    };
    for attribute in &asf.attributes {
        if attribute.name.eq_ignore_ascii_case(PICTURE) {
            continue;
        }
        if legacy(attribute) {
            if let Some(track) = legacy_track(attribute) {
                fields.entry("TRACK".into()).or_default().push(track);
            }
            continue;
        }
        let Some(value) = display(attribute) else {
            continue;
        };
        let name = field_name(&attribute.name);
        // Writers such as FFmpeg repeat Content Description values in the
        // Extended Content Description; such copies are shown once.
        let origin = seen
            .entry((name.to_lowercase(), value.clone()))
            .or_insert(attribute.location);
        if *origin != attribute.location {
            continue;
        }
        let key = fields
            .keys()
            .find(|key| key.eq_ignore_ascii_case(&name))
            .cloned()
            .unwrap_or(name);
        fields.entry(key).or_default().push(value);
    }
    fields
}

/// The one-based TRACK of a zero-based WM/Track value.
fn legacy_track(attribute: &Attribute) -> Option<String> {
    let index = match &attribute.value {
        Value::Dword(n) => u64::from(*n),
        Value::Word(n) => u64::from(*n),
        Value::Qword(n) => *n,
        Value::Unicode(text) => text.trim().parse().ok()?,
        _ => return None,
    };
    index.checked_add(1).map(|track| track.to_string())
}

fn picture_of(attribute: &Attribute) -> Option<Picture> {
    if !attribute.name.eq_ignore_ascii_case(PICTURE) {
        return None;
    }
    let Value::Bytes(bytes) = &attribute.value else {
        return None;
    };
    let value = PictureValue::parse(bytes)?;
    let mut picture = Picture::unchecked(value.data).pic_type(PictureType::from_u8(value.kind));
    if !value.mime.is_empty() {
        picture = picture.mime_type(MimeType::from_str(&value.mime));
    }
    if !value.description.is_empty() {
        picture = picture.description(value.description);
    }
    Some(picture.build())
}
fn picture_value(picture: &Picture) -> PictureValue {
    PictureValue {
        kind: picture.pic_type().as_u8(),
        mime: picture
            .mime_type()
            .map(|mime| mime.as_str().to_owned())
            .unwrap_or_default(),
        description: picture.description().unwrap_or_default().to_owned(),
        data: picture.data().to_vec(),
    }
}
fn pictures(asf: &Asf) -> Vec<Picture> {
    asf.attributes.iter().filter_map(picture_of).collect()
}

pub(super) fn describe_diagnostic(
    path: &Path,
    include_cover_data: bool,
    rules: &[UserFieldMapping],
) -> Result<AudioEntry, ErrorCause> {
    let (asf, _) = open_diagnostic(path)?;
    let fields = mapping::map_read(&local_fields(&asf), mapping::RuleTag::Wma, rules)
        .map_err(ErrorCause::owned_text)?;
    let covers: Vec<_> = pictures(&asf)
        .iter()
        .enumerate()
        .map(|(index, picture)| cover_info(index, picture, include_cover_data))
        .collect();
    let metadata = fs::metadata(path).map_err(ErrorCause::external)?;
    let properties = &asf.properties;
    let mut technical = Fields::new();
    if let Some(codec) = properties.codec() {
        technical.insert("_CODEC".into(), vec![codec]);
    }
    technical::stream_fields(
        &mut technical,
        u8::try_from(properties.bits_per_sample).ok(),
        Some(properties.bitrate),
        Some(properties.sample_rate),
        u8::try_from(properties.channels).ok(),
        std::time::Duration::from_secs_f64(properties.duration),
    );
    if !asf.attributes.is_empty() {
        for name in ["_TAG", "_TAG_READ"] {
            technical.insert(name.into(), vec![TAG.into()]);
        }
    }
    technical::file_fields(&mut technical, &covers, &metadata, path);
    Ok(AudioEntry {
        path: path.to_string_lossy().into_owned(),
        filename: path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned(),
        format: TAG.into(),
        duration: properties.duration,
        bitrate: properties.bitrate,
        sample_rate: properties.sample_rate,
        channels: u8::try_from(properties.channels).unwrap_or(u8::MAX),
        size: metadata.len(),
        fields,
        covers,
        technical,
        chapters: Vec::new(),
    })
}

fn same_bytes(a: &mut impl Read, b: &mut impl Read, mut length: u64) -> Result<bool, String> {
    let (mut left, mut right) = (vec![0u8; 1 << 16], vec![0u8; 1 << 16]);
    while length > 0 {
        let n = length.min(left.len() as u64) as usize;
        a.read_exact(&mut left[..n]).map_err(|e| e.to_string())?;
        b.read_exact(&mut right[..n]).map_err(|e| e.to_string())?;
        if left[..n] != right[..n] {
            return Ok(false);
        }
        length -= n as u64;
    }
    Ok(true)
}

fn same_bytes_diagnostic(
    a: &mut impl Read,
    b: &mut impl Read,
    length: u64,
) -> Result<bool, ErrorCause> {
    // The comparison returns only errors from Read::read_exact; a byte mismatch
    // is a successful false result. No source-owned failure is flattened here.
    same_bytes(a, b, length).map_err(ErrorCause::external)
}

/// Write the rebuilt header to `staging`, followed by everything after the
/// original header, and confirm that the copied media bytes are identical.
fn commit(staging: &mut fs::File, original: &Path, asf: &Asf, size: u64) -> Result<(), String> {
    commit_diagnostic(staging, original, asf, size).map_err(|error| error.to_string())
}

fn commit_diagnostic(
    staging: &mut fs::File,
    original: &Path,
    asf: &Asf,
    size: u64,
) -> Result<(), ErrorCause> {
    let tail = size - asf.header_size;
    let header = asf.render_diagnostic(tail)?;
    let mut source = fs::File::open(original).map_err(ErrorCause::external)?;
    source
        .seek(SeekFrom::Start(asf.header_size))
        .map_err(ErrorCause::external)?;
    staging
        .seek(SeekFrom::Start(0))
        .map_err(ErrorCause::external)?;
    staging.write_all(&header).map_err(ErrorCause::external)?;
    let copied =
        std::io::copy(&mut (&mut source).take(tail), staging).map_err(ErrorCause::external)?;
    if copied != tail {
        return Err(ErrorCause::owned(
            "The file changed during editing; reload it before saving",
        ));
    }
    staging
        .set_len(header.len() as u64 + tail)
        .map_err(ErrorCause::external)?;
    staging.flush().map_err(ErrorCause::external)?;
    source
        .seek(SeekFrom::Start(asf.header_size))
        .map_err(ErrorCause::external)?;
    staging
        .seek(SeekFrom::Start(header.len() as u64))
        .map_err(ErrorCause::external)?;
    if !same_bytes_diagnostic(&mut source, staging, tail)? {
        return Err(ErrorCause::owned("The ASF media data changed while saving"));
    }
    Ok(())
}

/// Replace the attributes behind each field, keeping their position, type,
/// stream and language. Returns the values each field must read back as.
fn apply_fields(asf: &mut Asf, updates: &Fields) -> Result<Fields, ErrorCause> {
    let mut expected = Fields::new();
    for (field, values) in updates {
        if field.eq_ignore_ascii_case(PICTURE) && !values.is_empty() {
            return Err(ErrorCause::owned(
                "WM/Picture is edited through the cover commands",
            ));
        }
        let matches = |attribute: &Attribute| {
            !attribute.name.eq_ignore_ascii_case(PICTURE)
                && field_name(&attribute.name).eq_ignore_ascii_case(field)
        };
        let template = asf
            .attributes
            .iter()
            .find(|a| matches(a) && display(a).is_some())
            .cloned();
        let documented = field_mapping(field)
            .map(|m| m.asf)
            .filter(|native| !native.is_empty());
        let name = match (documented, &template) {
            (Some(native), _) => native.to_owned(),
            (None, Some(template)) => template.name.clone(),
            (None, None) => field.clone(),
        };
        if let Some(field2) = mapping::asf_name(&name)
            .filter(|f| !values.is_empty() && !f.eq_ignore_ascii_case(field))
        {
            return Err(ErrorCause::message(
                "{field} has no separate ASF attribute; {name} holds {field2}",
                [
                    ("field", field.clone()),
                    ("name", name.clone()),
                    ("field2", field2.into()),
                ],
            ));
        }
        let rating = name.eq_ignore_ascii_case(RATING);
        let attributes = values
            .iter()
            .map(|text| {
                Ok(Attribute {
                    stream: template.as_ref().map_or(0, |t| t.stream),
                    language: template.as_ref().map_or(0, |t| t.language),
                    ..Attribute::new(
                        &name,
                        parse_value(text, template.as_ref().map(|t| &t.value), rating)
                            .map_err(ErrorCause::owned_text)?,
                    )
                })
            })
            .collect::<Result<Vec<_>, ErrorCause>>()?;
        expected.insert(
            field.clone(),
            attributes.iter().filter_map(display).collect(),
        );
        let position = asf.attributes.iter().position(matches);
        asf.attributes.retain(|attribute| !matches(attribute));
        let at = position.unwrap_or(asf.attributes.len());
        asf.attributes.splice(at..at, attributes);
        if field.eq_ignore_ascii_case("TRACK") {
            update_legacy_track(asf, values);
        }
    }
    Ok(expected)
}

/// A WM/Track attribute follows TRACK zero-based in its own type, so older
/// players agree with WM/TrackNumber; it is dropped when TRACK is removed or
/// is not a plain number.
fn update_legacy_track(asf: &mut Asf, values: &[String]) {
    let index = match values {
        [value] => value
            .split('/')
            .next()
            .and_then(|n| n.trim().parse::<u64>().ok())
            .and_then(|n| n.checked_sub(1)),
        _ => None,
    };
    asf.attributes.retain_mut(|attribute| {
        if !attribute.name.eq_ignore_ascii_case(LEGACY_TRACK) {
            return true;
        }
        let Some(index) = index else {
            return false;
        };
        attribute.value = match &attribute.value {
            Value::Dword(_) => u32::try_from(index).map(Value::Dword),
            Value::Word(_) => u16::try_from(index).map(Value::Word),
            Value::Qword(_) => Ok(Value::Qword(index)),
            _ => Ok(Value::Unicode(index.to_string())),
        }
        .unwrap_or_else(|_| Value::Unicode(index.to_string()));
        true
    });
}

pub(super) fn write_fields_diagnostic(
    path: &Path,
    updates: &Fields,
    include_cover_data: bool,
    rules: &[UserFieldMapping],
    options: &TagOptions,
) -> Result<AudioEntry, ErrorCause> {
    // Validate names while keeping their spelling, like other free-form tags.
    let updates = normalized_updates_diagnostic(updates, TagType::Ape)?;
    let updates = normalized_updates_diagnostic(
        &mapping::map_write(&updates, mapping::RuleTag::Wma, rules)
            .map_err(ErrorCause::owned_text)?,
        TagType::Ape,
    )?;
    staged_write_diagnostic(
        path,
        include_cover_data,
        rules,
        options,
        |staging, original, staged| {
            let (mut asf, size) = open_diagnostic(original)?;
            let expected = apply_fields(&mut asf, &updates)?;
            commit_diagnostic(staging, original, &asf, size)?;
            let actual = local_fields(&open_diagnostic(staged)?.0);
            for (field, values) in &expected {
                let found = actual
                    .iter()
                    .find(|(name, _)| name.eq_ignore_ascii_case(field))
                    .map(|(_, values)| values);
                if values.is_empty() && found.is_some() {
                    return Err(ErrorCause::message(
                        "The format could not remove field {field}",
                        [("field", field.clone())],
                    ));
                }
                if !values.is_empty() && found != Some(values) {
                    return Err(ErrorCause::message(
                        "The format could not preserve all values for field {field}",
                        [("field", field.clone())],
                    ));
                }
            }
            Ok(())
        },
    )
}

/// Remove the tag: every attribute that is not tied to a specific stream.
pub(super) fn remove_tags(
    path: &Path,
    include_cover_data: bool,
    options: &TagOptions,
) -> Result<AudioEntry, String> {
    staged_write(
        path,
        include_cover_data,
        &options.mappings,
        options,
        |staging, original, staged| {
            let (mut asf, size) = open(original)?;
            asf.attributes.retain(|attribute| attribute.stream != 0);
            commit(staging, original, &asf, size)?;
            if open(staged)?.0.attributes.iter().any(|a| a.stream == 0) {
                return Err("The ASF tag could not be removed".into());
            }
            Ok(())
        },
    )
}

/// Replace the WM/Picture attributes, reusing the stored attribute (and its
/// object) for every picture that is unchanged.
fn set_pictures(asf: &mut Asf, pictures: &[Picture]) -> Result<(), ErrorCause> {
    let mut originals: Vec<_> = asf
        .attributes
        .iter()
        .filter(|a| is_picture(a))
        .cloned()
        .map(Some)
        .collect();
    let mut attributes = Vec::new();
    for picture in pictures {
        let value = Value::Bytes(picture_value(picture).encode_diagnostic()?);
        let reused = originals
            .iter_mut()
            .find(|original| original.as_ref().is_some_and(|o| o.value == value))
            .and_then(Option::take);
        attributes.push(reused.unwrap_or_else(|| Attribute::new(PICTURE, value)));
    }
    let position = asf.attributes.iter().position(is_picture);
    asf.attributes.retain(|attribute| !is_picture(attribute));
    let at = position.unwrap_or(asf.attributes.len());
    asf.attributes.splice(at..at, attributes);
    Ok(())
}

/// Edit the cover list and verify that every picture reads back unchanged.
pub(super) fn change_pictures(
    path: &Path,
    include_cover_data: bool,
    options: &TagOptions,
    change: impl FnOnce(&mut Vec<Picture>) -> Result<(), String>,
    failure: &str,
) -> Result<AudioEntry, String> {
    change_pictures_diagnostic(
        path,
        include_cover_data,
        options,
        |pictures| change(pictures).map_err(ErrorCause::external),
        failure,
    )
    .map_err(|error| error.to_string())
}

pub(super) fn change_pictures_diagnostic(
    path: &Path,
    include_cover_data: bool,
    options: &TagOptions,
    change: impl FnOnce(&mut Vec<Picture>) -> Result<(), ErrorCause>,
    failure: &str,
) -> Result<AudioEntry, ErrorCause> {
    staged_write_diagnostic(
        path,
        include_cover_data,
        &options.mappings,
        options,
        |staging, original, staged| {
            let (mut asf, size) = open_diagnostic(original)?;
            let mut edited = pictures(&asf);
            change(&mut edited)?;
            set_pictures(&mut asf, &edited)?;
            commit_diagnostic(staging, original, &asf, size)?;
            let written = pictures(&open_diagnostic(staged)?.0);
            if written
                .iter()
                .map(picture_value)
                .ne(edited.iter().map(picture_value))
            {
                return Err(ErrorCause::owned_text(failure));
            }
            Ok(())
        },
    )
}

#[cfg(test)]
mod tests;
