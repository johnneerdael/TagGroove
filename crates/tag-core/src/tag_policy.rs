//! Per-format tag policies from Options → Tags → Mpeg and Ape, Mpc: one
//! displayed tag kind, independent write and remove sets, and the configured
//! ID3v2 revision and encoding.
use super::*;
use crate::preferences::Id3v2Format;
use lofty::id3::{
    v1::{Id3v1Tag, GENRES},
    v2::Id3v2Version,
};

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

/// Write `change` to every enabled tag kind and strip the kinds selected for
/// removal. New kinds receive all displayed values and artwork. Returns the
/// kind used to verify the result (the highest-priority written kind).
pub(super) fn write(
    staging: &mut fs::File,
    file: &TaggedFile,
    rules: &[UserFieldMapping],
    options: &TagOptions,
    change: Change<'_>,
) -> Result<TagType, String> {
    let file_type = file.file_type();
    let kind = policy(file_type).ok_or("This format has no configurable tag policy")?;
    let native = native_tags(staging, file_type)?;
    let displayed = displayed_tags(file, options).first().copied();
    let mut full = displayed
        .map(|tag| tag_fields(tag, &native, rules, options))
        .transpose()?
        .unwrap_or_default();
    let mut pictures = displayed
        .map(|tag| tag.pictures().to_vec())
        .unwrap_or_default();
    let (mut updates, covers_changed) = match &change {
        Change::Fields(updates) => {
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
            let write_v1 = o.write_id3v1 || (o.write_ape && has(TagType::Id3v1));
            let remove = [
                (o.remove_id3v1 && !write_v1, TagType::Id3v1),
                (o.remove_id3v2 && !write_v2, TagType::Id3v2),
                (o.remove_ape && !o.write_ape, TagType::Ape),
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
    // An APE tag that is removed without being displayed would silently drop
    // gain data written by MP3Gain/ReplayGain tools; keep it in ID3v2 user
    // frames unless ID3v2 already carries those fields.
    if kind == Policy::Mpeg
        && remove[2].0
        && write_v2
        && displayed.map(Tag::tag_type) != Some(TagType::Ape)
    {
        if let Some(ape) = file.tag(TagType::Ape) {
            let current_v2 = file
                .tag(TagType::Id3v2)
                .map(|tag| tag_fields(tag, &native, rules, options))
                .transpose()?
                .unwrap_or_default();
            let present = |fields: &Fields, name: &str| {
                fields.keys().any(|key| key.eq_ignore_ascii_case(name))
            };
            for (name, values) in tag_fields(ape, &native, rules, options)? {
                let upper = name.to_ascii_uppercase();
                let gain = upper.starts_with("REPLAYGAIN_") || upper.starts_with("MP3GAIN_");
                if gain
                    && !values.is_empty()
                    && !present(&full, &name)
                    && !present(&current_v2, &name)
                {
                    full.insert(name.clone(), values.clone());
                    updates.insert(name, values);
                }
            }
        }
    }
    let id3 = if kind == Policy::Mpeg {
        Id3Write::Convert(id3_target(options))
    } else {
        Id3Write::preserving(options)
    };
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
                for name in tag_fields(existing, &native, rules, options)?.into_keys() {
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
        let native_updates = normalized_updates(&map_write_fields(source, kind, rules)?, kind)?;
        staging
            .seek(SeekFrom::Start(0))
            .map_err(|e| e.to_string())?;
        save_preserving_native(
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
            .map_err(|e| e.to_string())?;
        if !tag.is_empty() || has(TagType::Id3v1) {
            // An existing ID3v1 tag is overwritten in place (the last 128
            // bytes); adding or removing one changes the file length.
            if tag.is_empty() || !has(TagType::Id3v1) {
                crate::write_cost::rewrote_all();
            } else {
                crate::write_cost::rewrote_region(128);
            }
            tag.save_to(staging, WriteOptions::default())
                .map_err(|e| error_message(&e))?;
        }
    }
    for (enabled, kind) in remove {
        if enabled && has(kind) {
            staging
                .seek(SeekFrom::Start(0))
                .map_err(|e| e.to_string())?;
            crate::write_cost::rewrote_all();
            kind.remove_from(staging, WriteOptions::default())
                .map_err(|e| error_message(&e))?;
        }
    }
    staging
        .seek(SeekFrom::Start(0))
        .map_err(|e| e.to_string())?;
    let order = read_order(file_type, &TagOptions::default()).unwrap_or_default();
    order
        .into_iter()
        .find(|kind| match kind {
            TagType::Ape => write_ape,
            TagType::Id3v2 => write_v2,
            _ => false,
        })
        .or(write_v1.then_some(TagType::Id3v1))
        .ok_or_else(|| "Enable at least one tag type to write".into())
}

pub(super) fn verify_fields(
    staged_path: &Path,
    rules: &[UserFieldMapping],
    options: &TagOptions,
    written: TagType,
    updates: &Fields,
) -> Result<(), String> {
    // Native names: `native_updates` below are already mapped back to sources.
    let entry = read_description(staged_path, false, &[], &reading_only(options, written))?;
    if written == TagType::Id3v1 {
        // ID3v1 truncates and drops values by design; only its presence is checked.
        return if entry.technical.contains_key("_TAG_READ") {
            Ok(())
        } else {
            Err("The ID3v1 tag could not be written".into())
        };
    }
    let native_updates = normalized_updates(&map_write_fields(updates, written, rules)?, written)?;
    let latin1 = written == TagType::Id3v2
        && file_kind(staged_path)? == FileType::Mpeg
        && options.mpeg.id3v2_format == Id3v2Format::V23Latin1;
    verify_readback(&entry.fields, &native_updates, written, latin1)
}

fn file_kind(path: &Path) -> Result<FileType, String> {
    Probe::open(path)
        .map_err(|e| e.to_string())?
        .guess_file_type()
        .map_err(|e| e.to_string())?
        .file_type()
        .ok_or_else(|| "Unrecognized audio file type".into())
}

pub(super) fn verify_pictures(
    staged_path: &Path,
    options: &TagOptions,
    written: TagType,
    expected: &[Picture],
    message: &str,
) -> Result<(), String> {
    if written == TagType::Id3v1 && !expected.is_empty() {
        return Err("ID3v1 cannot store cover art; enable ID3v2 or APEv2 writing".into());
    }
    let entry = read_description(staged_path, false, &[], &reading_only(options, written))?;
    if entry.covers.len() != expected.len()
        || entry.covers.iter().zip(expected).any(|(cover, picture)| {
            cover.picture_type != format!("{:?}", picture.pic_type())
                || cover.description != picture.description().unwrap_or_default()
        })
    {
        return Err(message.into());
    }
    Ok(())
}
