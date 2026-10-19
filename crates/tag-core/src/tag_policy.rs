//! Per-format tag policies from Options → Tags → Mpeg and Ape, Mpc: one
//! displayed tag kind, independent write and remove sets, and the configured
//! ID3v2 revision and encoding.
use super::*;
use crate::batch_errors::ErrorCause;
use crate::preferences::Id3v2Format;
use lofty::id3::{
    v1::{Id3v1Tag, GENRES},
    v2::Id3v2Version,
};
use std::io::Write;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Policy {
    Mpeg,
    Ape,
}
pub(super) fn policy(kind: FileType) -> Option<Policy> {
    match kind {
        FileType::Mpeg => Some(Policy::Mpeg),
        FileType::Ape | FileType::Mpc | FileType::WavPack => Some(Policy::Ape),
        _ => None,
    }
}

/// Enabled kinds from highest to lowest display priority. Mp3tag reads ID3v1,
/// then ID3v2, then APE, so a later enabled kind replaces an earlier one.
pub(super) fn read_order(kind: FileType, options: &TagOptions) -> Option<Vec<TagType>> {
    let candidates = match policy(kind)? {
        Policy::Mpeg => vec![
            (options.mpeg.read_ape, TagType::Ape),
            (options.mpeg.read_id3v2, TagType::Id3v2),
            (options.mpeg.read_id3v1, TagType::Id3v1),
        ],
        Policy::Ape => vec![
            (options.ape.read_ape, TagType::Ape),
            (options.ape.read_id3v1, TagType::Id3v1),
        ],
    };
    Some(
        candidates
            .into_iter()
            .filter(|(enabled, _)| *enabled)
            .map(|(_, kind)| kind)
            .collect(),
    )
}

/// Kinds File → Remove tag strips from `file`: the present kinds selected
/// under Remove for MPEG and APE-family files, every present kind otherwise.
pub(super) fn removed_kinds(
    file: &TaggedFile,
    options: &TagOptions,
) -> Result<Vec<TagType>, String> {
    let (selected, page) = match policy(file.file_type()) {
        None => return Ok(file.tags().iter().map(Tag::tag_type).collect()),
        Some(Policy::Mpeg) => {
            let o = &options.mpeg;
            ([o.remove_id3v1, o.remove_id3v2, o.remove_ape], "Mpeg")
        }
        Some(Policy::Ape) => {
            let o = &options.ape;
            ([o.remove_id3v1, o.remove_id3v2, o.remove_ape], "Ape, Mpc")
        }
    };
    if !selected.contains(&true) {
        return Err(format!(
            "No tag types are selected under Options → Tags → {page} → Remove"
        ));
    }
    Ok(selected
        .into_iter()
        .zip([TagType::Id3v1, TagType::Id3v2, TagType::Ape])
        .filter(|(enabled, kind)| *enabled && file.tag(*kind).is_some())
        .map(|(_, kind)| kind)
        .collect())
}

/// Options that read exactly one tag kind, used to verify what was written.
fn reading_only(options: &TagOptions, kind: TagType) -> TagOptions {
    let mut options = options.clone();
    options.mpeg.read_id3v1 = kind == TagType::Id3v1;
    options.mpeg.read_id3v2 = kind == TagType::Id3v2;
    options.mpeg.read_ape = kind == TagType::Ape;
    options.ape.read_id3v1 = kind == TagType::Id3v1;
    options.ape.read_ape = kind == TagType::Ape;
    options
}

pub(super) fn id3_target(options: &TagOptions) -> mapping::Id3Target {
    let (version, latin1) = match options.mpeg.id3v2_format {
        Id3v2Format::V23Utf16 => (Id3v2Version::V3, false),
        Id3v2Format::V23Latin1 => (Id3v2Version::V3, true),
        Id3v2Format::V24Utf8 => (Id3v2Version::V4, false),
    };
    mapping::Id3Target {
        version,
        latin1,
        language: options.comment_language(),
    }
}

pub(super) fn displayed_pictures(file: &TaggedFile, options: &TagOptions) -> Vec<Picture> {
    displayed_tags(file, options)
        .first()
        .map(|tag| tag.pictures().to_vec())
        .unwrap_or_default()
}

pub(super) enum Change<'a> {
    Fields(&'a Fields),
    RawLyricsFields(&'a Fields),
    Pictures(Vec<Picture>),
}

fn first<'a>(fields: &'a Fields, name: &str) -> Option<&'a str> {
    fields
        .iter()
        .find(|(key, _)| key.eq_ignore_ascii_case(name))
        .and_then(|(_, values)| values.first())
        .map(String::as_str)
}
fn genre_index(value: &str) -> Option<u8> {
    let value = value.trim();
    let number = value
        .strip_prefix('(')
        .and_then(|v| v.strip_suffix(')'))
        .unwrap_or(value);
    if let Ok(index) = number.parse::<u8>() {
        return (usize::from(index) < GENRES.len()).then_some(index);
    }
    GENRES
        .iter()
        .position(|genre| genre.eq_ignore_ascii_case(value))
        .and_then(|index| u8::try_from(index).ok())
}
fn track_number(value: &str) -> Option<u8> {
    value
        .split('/')
        .next()
        .and_then(|n| n.trim().parse::<u8>().ok())
        .filter(|n| *n > 0)
}

/// Whether ID3v1 holds every value exactly ("ID3v2 only if ID3v1 too small").
/// User-defined genres, extra fields, multiple values and artwork need ID3v2.
pub(super) fn id3v1_sufficient(fields: &Fields, pictures: &[Picture]) -> bool {
    if !pictures.is_empty() {
        return false;
    }
    let has_track = first(fields, "TRACK").is_some();
    fields.iter().all(|(name, values)| {
        let [value] = values.as_slice() else {
            return values.is_empty();
        };
        if value.chars().any(|c| u32::from(c) > 0xff) {
            return false;
        }
        match name.to_ascii_uppercase().as_str() {
            "TITLE" | "ARTIST" | "ALBUM" => value.chars().count() <= 30,
            "COMMENT" => value.chars().count() <= if has_track { 28 } else { 30 },
            "YEAR" => value.len() == 4 && value.bytes().all(|b| b.is_ascii_digit()),
            "TRACK" => value.trim().parse::<u8>().is_ok_and(|n| n > 0),
            "GENRE" => GENRES.iter().any(|genre| genre.eq_ignore_ascii_case(value)),
            _ => false,
        }
    })
}

fn id3v1_from(fields: &Fields) -> Id3v1Tag {
    let mut tag = Id3v1Tag::new();
    let text = |name| {
        first(fields, name)
            .filter(|v| !v.is_empty())
            .map(str::to_owned)
    };
    tag.title = text("TITLE");
    tag.artist = text("ARTIST");
    tag.album = text("ALBUM");
    tag.comment = text("COMMENT");
    tag.year = first(fields, "YEAR")
        .and_then(|y| y.get(..4))
        .and_then(|y| y.parse::<u16>().ok());
    tag.track_number = first(fields, "TRACK").and_then(track_number);
    tag.genre = first(fields, "GENRE").and_then(genre_index);
    tag
}

/// Exact written-kind verification intent, separate from normal read priority.
#[derive(Clone, Copy)]
pub(super) struct Written {
    kind: TagType,
    /// Explicit whole-kind removal of a known-empty supplied MPEG model.
    /// Order: ID3v1, ID3v2, APE. Ordinary writes require normal readback.
    absent: [bool; 3],
}

/// Fields and pictures do not describe all records in the displayed source.
/// Concrete APE binary/locator items and opaque or special ID3 records must
/// survive a field clear. The latter guard is deliberately conservative: their
/// mapping into the original application's binary collection is not qualified.
/// Hidden target records do not participate in the supplied model's emptiness.
fn source_has_preserved_records(displayed: Option<&Tag>, native: &NativeTags) -> bool {
    match displayed.map(Tag::tag_type) {
        None | Some(TagType::Id3v1) => false,
        Some(TagType::Ape) => native.ape.as_ref().is_none_or(|tag| {
            tag.into_iter().any(|item| match item.value() {
                ItemValue::Binary(data) => {
                    !item.key().starts_with("Cover Art (")
                        || Picture::from_ape_bytes(item.key(), data).is_err()
                }
                ItemValue::Locator(_) => true,
                ItemValue::Text(_) => false,
            })
        }),
        Some(TagType::Id3v2) => {
            !native.id3_placeholders.is_empty()
                || native.id3.as_ref().is_none_or(|tag| {
                    tag.into_iter().any(|frame| {
                        optimize::unsupported_id3_frame(frame)
                            || frame.flags() != Default::default()
                            || matches!(
                                frame,
                                Frame::Binary(_)
                                    | Frame::KeyValue(_)
                                    | Frame::UniqueFileIdentifier(_)
                                    | Frame::Chapter(_)
                                    | Frame::TableOfContents(_)
                            )
                    })
                })
        }
        Some(_) => true,
    }
}

/// Write enabled kinds. MPEG retains unwritten kinds; APE-family policy is unchanged.
pub(super) fn write(
    staging: &mut fs::File,
    file: &TaggedFile,
    rules: &[UserFieldMapping],
    options: &TagOptions,
    change: Change<'_>,
) -> Result<Written, String> {
    write_diagnostic(staging, file, rules, options, change).map_err(|error| error.to_string())
}

pub(super) fn write_diagnostic(
    staging: &mut fs::File,
    file: &TaggedFile,
    rules: &[UserFieldMapping],
    options: &TagOptions,
    change: Change<'_>,
) -> Result<Written, ErrorCause> {
    let file_type = file.file_type();
    let kind = policy(file_type)
        .ok_or_else(|| ErrorCause::owned("This format has no configurable tag policy"))?;
    let native = native_tags_diagnostic(staging, file_type)?;
    let displayed = displayed_tags(file, options).first().copied();
    let mut full = displayed
        .map(|tag| tag_fields_diagnostic(tag, &native, rules, options))
        .transpose()?
        .unwrap_or_default();
    let mut pictures = displayed
        .map(|tag| tag.pictures().to_vec())
        .unwrap_or_default();
    let (updates, covers_changed) = match &change {
        Change::Fields(updates) | Change::RawLyricsFields(updates) => {
            for (name, values) in updates.iter() {
                full.retain(|key, _| !key.eq_ignore_ascii_case(name));
                if !values.is_empty() {
                    full.insert(name.clone(), values.clone());
                }
            }
            ((*updates).clone(), false)
        }
        Change::Pictures(list) => {
            pictures = list.clone();
            (Fields::new(), true)
        }
    };
    let has = |kind| file.tag(kind).is_some();
    let (write_v1, write_v2, write_ape, remove) = match kind {
        Policy::Mpeg => {
            let o = &options.mpeg;
            let write_v2 = o.write_id3v2
                && (!(o.id3v2_only_if_needed && o.write_id3v1)
                    || has(TagType::Id3v2)
                    || !id3v1_sufficient(&full, &pictures));
            // MPEG Save follows Write only. Remove selections belong to the
            // explicit Remove command; unwritten tag kinds remain untouched.
            let write_v1 = o.write_id3v1;
            let remove = [
                (false, TagType::Id3v1),
                (false, TagType::Id3v2),
                (false, TagType::Ape),
            ];
            (write_v1, write_v2, o.write_ape, remove)
        }
        Policy::Ape => {
            let o = &options.ape;
            let write_v1 = o.write_id3v1 || (o.write_ape && has(TagType::Id3v1));
            let remove = [
                (o.remove_id3v1 && !write_v1, TagType::Id3v1),
                (o.remove_id3v2, TagType::Id3v2),
                (o.remove_ape && !o.write_ape, TagType::Ape),
            ];
            (write_v1, false, o.write_ape, remove)
        }
    };
    let id3 = if kind == Policy::Mpeg {
        Id3Write::converting(id3_target(options), options)
    } else {
        Id3Write::preserving(options)
    };
    let written_kind = [
        (write_ape, TagType::Ape),
        (write_v2, TagType::Id3v2),
        (write_v1, TagType::Id3v1),
    ]
    .into_iter()
    .find_map(|(enabled, kind)| enabled.then_some(kind))
    .ok_or_else(|| ErrorCause::owned("Enable at least one tag type to write"))?;
    if kind == Policy::Mpeg
        && full.is_empty()
        && pictures.is_empty()
        && !source_has_preserved_records(displayed, &native)
    {
        let absent = [write_v1, write_v2, write_ape];
        for (enabled, tag_type) in
            absent
                .into_iter()
                .zip([TagType::Id3v1, TagType::Id3v2, TagType::Ape])
        {
            if enabled && has(tag_type) {
                staging
                    .seek(SeekFrom::Start(0))
                    .map_err(ErrorCause::external)?;
                crate::write_cost::rewrote_all();
                tag_type
                    .remove_from(staging, WriteOptions::default())
                    .map_err(|e| ErrorCause::external(error_message(&e)))?;
            }
        }
        staging
            .seek(SeekFrom::Start(0))
            .map_err(ErrorCause::external)?;
        return Ok(Written {
            kind: written_kind,
            absent,
        });
    }
    for (enabled, kind) in [(write_ape, TagType::Ape), (write_v2, TagType::Id3v2)] {
        if !enabled {
            continue;
        }
        let existing = file.tag(kind);
        let mut tag = existing.cloned().unwrap_or_else(|| Tag::new(kind));
        // The displayed kind is patched in place. Every other written kind
        // receives the complete displayed field set, so all kinds agree.
        let mirrored;
        let source = match existing {
            Some(_) if displayed.map(Tag::tag_type) == Some(kind) => &updates,
            Some(existing) => {
                let mut values = full.clone();
                for name in tag_fields_diagnostic(existing, &native, rules, options)?.into_keys() {
                    if !values.keys().any(|key| key.eq_ignore_ascii_case(&name)) {
                        values.insert(name, Vec::new());
                    }
                }
                mirrored = values;
                &mirrored
            }
            None => &full,
        };
        let replace_covers = covers_changed || (existing.is_none() && !pictures.is_empty());
        if replace_covers {
            while !tag.pictures().is_empty() {
                tag.remove_picture(0);
            }
            for picture in &pictures {
                tag.push_picture(picture.clone());
            }
        }
        let source = match &change {
            Change::RawLyricsFields(changes) => {
                source_lyrics_updates_diagnostic(source, changes, kind, rules)?
            }
            _ => source.clone(),
        };
        let native_updates = normalized_updates_diagnostic(
            &map_write_fields_diagnostic(&source, kind, rules)?,
            kind,
        )?;
        staging
            .seek(SeekFrom::Start(0))
            .map_err(ErrorCause::external)?;
        save_preserving_native_diagnostic(
            staging,
            file_type,
            &tag,
            &native_updates,
            replace_covers,
            id3,
        )?;
    }
    if write_v1 {
        let tag = id3v1_from(&full);
        staging
            .seek(SeekFrom::Start(0))
            .map_err(ErrorCause::external)?;
        if kind == Policy::Mpeg && tag.is_empty() {
            // A nonempty full model can have no representable V1 fields.
            // Its enabled writer still emits a blank V1; save_to would instead
            // remove an existing tag. Use the existing native serializer.
            let mut bytes = Vec::new();
            tag.dump_to(&mut bytes, WriteOptions::default())
                .map_err(|e| ErrorCause::external(error_message(&e)))?;
            staging
                .seek(SeekFrom::End(if has(TagType::Id3v1) { -128 } else { 0 }))
                .map_err(ErrorCause::external)?;
            staging.write_all(&bytes).map_err(ErrorCause::external)?;
            if has(TagType::Id3v1) {
                crate::write_cost::rewrote_region(bytes.len() as u64);
            } else {
                crate::write_cost::rewrote_all();
            }
        } else if !tag.is_empty() || has(TagType::Id3v1) {
            // An existing ID3v1 tag is overwritten in place (the last 128
            // bytes); adding or removing one changes the file length.
            if tag.is_empty() || !has(TagType::Id3v1) {
                crate::write_cost::rewrote_all();
            } else {
                crate::write_cost::rewrote_region(128);
            }
            tag.save_to(staging, WriteOptions::default())
                .map_err(|e| ErrorCause::external(error_message(&e)))?;
        }
    }
    for (enabled, kind) in remove {
        if enabled && has(kind) {
            staging
                .seek(SeekFrom::Start(0))
                .map_err(ErrorCause::external)?;
            crate::write_cost::rewrote_all();
            kind.remove_from(staging, WriteOptions::default())
                .map_err(|e| ErrorCause::external(error_message(&e)))?;
        }
    }
    staging
        .seek(SeekFrom::Start(0))
        .map_err(ErrorCause::external)?;
    // Verification targets the highest actually written kind, independently
    // of read masks (which may deliberately prefer an unwritten retained tag).
    Ok(Written {
        kind: written_kind,
        absent: [false; 3],
    })
}

fn verify_absence(staged_path: &Path, written: Written) -> Result<bool, ErrorCause> {
    if !written.absent.contains(&true) {
        return Ok(false);
    }
    let file = load_diagnostic(staged_path)?;
    let native = native_tags_diagnostic(
        &mut fs::File::open(staged_path).map_err(ErrorCause::external)?,
        file.file_type(),
    )?;
    let present = [
        file.tag(TagType::Id3v1).is_some(),
        native.id3.is_some(),
        native.ape.is_some(),
    ];
    if written
        .absent
        .into_iter()
        .zip(present)
        .any(|(absent, present)| absent && present)
    {
        return Err(ErrorCause::owned("The tags could not be removed"));
    }
    Ok(true)
}

pub(super) fn verify_fields_diagnostic(
    staged_path: &Path,
    rules: &[UserFieldMapping],
    options: &TagOptions,
    written: Written,
    updates: &Fields,
    raw_lyrics: bool,
) -> Result<(), ErrorCause> {
    if verify_absence(staged_path, written)? {
        return Ok(());
    }
    let written = written.kind;
    // Native names: `native_updates` below are already mapped back to sources.
    let entry =
        read_description_diagnostic(staged_path, false, &[], &reading_only(options, written))?;
    if written == TagType::Id3v1 {
        // Ordinary V1 writes require presence, even for an empty projection
        // of a nonempty supplied model.
        return if entry.technical.contains_key("_TAG_READ") {
            Ok(())
        } else {
            Err(ErrorCause::owned("The ID3v1 tag could not be written"))
        };
    }
    let updates = if raw_lyrics {
        source_lyrics_updates_diagnostic(updates, updates, written, rules)?
    } else {
        updates.clone()
    };
    let native_updates = normalized_updates_diagnostic(
        &map_write_fields_diagnostic(&updates, written, rules)?,
        written,
    )?;
    let latin1 = written == TagType::Id3v2
        && file_kind_diagnostic(staged_path)? == FileType::Mpeg
        && options.mpeg.id3v2_format == Id3v2Format::V23Latin1;
    verify_readback_diagnostic(&entry.fields, &native_updates, written, latin1)
}

fn file_kind_diagnostic(path: &Path) -> Result<FileType, ErrorCause> {
    Probe::open(path)
        .map_err(ErrorCause::external)?
        .guess_file_type()
        .map_err(ErrorCause::external)?
        .file_type()
        .ok_or_else(|| ErrorCause::owned("Unrecognized audio file type"))
}

pub(super) fn verify_pictures(
    staged_path: &Path,
    options: &TagOptions,
    written: Written,
    expected: &[Picture],
    message: &str,
) -> Result<(), String> {
    verify_pictures_diagnostic(staged_path, options, written, expected, message)
        .map_err(|error| error.to_string())
}

pub(super) fn verify_pictures_diagnostic(
    staged_path: &Path,
    options: &TagOptions,
    written: Written,
    expected: &[Picture],
    message: &str,
) -> Result<(), ErrorCause> {
    if verify_absence(staged_path, written)? {
        return if expected.is_empty() {
            Ok(())
        } else {
            Err(ErrorCause::owned(message))
        };
    }
    let written = written.kind;
    if written == TagType::Id3v1 && !expected.is_empty() {
        return Err(ErrorCause::owned(
            "ID3v1 cannot store cover art; enable ID3v2 or APEv2 writing",
        ));
    }
    let entry =
        read_description_diagnostic(staged_path, false, &[], &reading_only(options, written))?;
    if entry.covers.len() != expected.len()
        || entry.covers.iter().zip(expected).any(|(cover, picture)| {
            cover.picture_type != format!("{:?}", picture.pic_type())
                || cover.description != picture.description().unwrap_or_default()
        })
    {
        return Err(ErrorCause::owned(message));
    }
    Ok(())
}
