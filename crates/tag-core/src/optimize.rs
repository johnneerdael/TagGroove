//! Utils → Remove Unsupported Fields and Optimize (https://docs.mp3tag.de/utils/). Each util writes through a staged copy and
//! leaves files it has nothing to do for untouched.
use super::*;
use crate::flac_metadata;
use std::io::{Read, Write};

/// Padding that Optimize leaves in a FLAC file.
pub const FLAC_PADDING: usize = 4096;

/// ID3v2 frames OpenTag shows neither as a field, a cover nor a chapter. They
/// are listed by `%_id3v2_unknown_frames%`.
pub(super) fn unsupported_id3_frame(frame: &Frame<'_>) -> bool {
    match frame {
        Frame::Binary(binary) => !matches!(binary.id().as_str(), "IPLS" | "PCST"),
        Frame::Private(_)
        | Frame::RelativeVolumeAdjustment(_)
        | Frame::EventTimingCodes(_)
        | Frame::Ownership(_) => true,
        Frame::UniqueFileIdentifier(uid) => uid.owner != "http://musicbrainz.org",
        _ => false,
    }
}

fn unchanged(
    path: &Path,
    include_cover_data: bool,
    options: &TagOptions,
) -> Result<AudioEntry, String> {
    if include_cover_data {
        read_entry_with_options(path, &[], options)
    } else {
        read_entry_summary_with_options(path, &[], options)
    }
}

fn file_type(path: &Path) -> Result<Option<FileType>, String> {
    Ok(Probe::open(path)
        .map_err(|e| e.to_string())?
        .guess_file_type()
        .map_err(|e| e.to_string())?
        .file_type())
}

/// Remove the ID3v2 frames OpenTag cannot display. Other tags are not changed.
pub fn remove_unsupported_fields_with_options(
    path: &Path,
    include_cover_data: bool,
    options: &TagOptions,
) -> Result<AudioEntry, String> {
    let kind = file_type(path)?;
    let Some(kind) = kind.filter(|k| {
        matches!(
            k,
            FileType::Mpeg | FileType::Aac | FileType::Aiff | FileType::Wav
        )
    }) else {
        return Err(
            "Unsupported fields can only be removed from ID3v2 tags (MP3, AAC, AIFF and WAV files)"
                .into(),
        );
    };
    let native = native_tags(&mut fs::File::open(path).map_err(|e| e.to_string())?, kind)?;
    if !native
        .id3
        .as_ref()
        .is_some_and(|tag| tag.into_iter().any(unsupported_id3_frame))
    {
        return unchanged(path, include_cover_data, options);
    }
    let language = options.comment_language();
    staged_write(path, include_cover_data, &[], options, |staging, _, _| {
        let mut tag = native_tags(staging, kind)?
            .id3
            .ok_or("The ID3v2 tag disappeared")?;
        let target = mapping::Id3Target::preserving(&tag, language);
        tag.retain(|frame| !unsupported_id3_frame(frame));
        staging
            .seek(SeekFrom::Start(0))
            .map_err(|e| e.to_string())?;
        mapping::save_id3(staging, kind, &tag, &target)
    })
}

/// Optimize FLAC (exactly [`FLAC_PADDING`] bytes of padding) or MP4 (no free
/// space, `moov` before the media data). Other formats are rejected.
pub fn optimize_with_options(
    path: &Path,
    include_cover_data: bool,
    options: &TagOptions,
) -> Result<AudioEntry, String> {
    match file_type(path)? {
        Some(FileType::Flac) => optimize_flac(path, include_cover_data, options),
        Some(FileType::Mp4) => {
            let layout =
                mp4_layout::read_layout(&mut fs::File::open(path).map_err(|e| e.to_string())?)?;
            if layout.optimized() {
                return unchanged(path, include_cover_data, options);
            }
            staged_write(
                path,
                include_cover_data,
                &[],
                options,
                |staging, original, _| {
                    crate::write_cost::rewrote_all();
                    let mut source = fs::File::open(original).map_err(|e| e.to_string())?;
                    let mut layout = mp4_layout::read_layout(&mut source)?;
                    mp4_layout::optimize(&mut layout);
                    mp4_layout::write(&mut source, layout, true, staging)
                },
            )
        }
        _ => Err("Optimize applies to FLAC and MP4 files".into()),
    }
}

fn flac_region(file: &mut fs::File) -> Result<(u64, Vec<u8>), String> {
    let end = flac_metadata::metadata_end(file)
        .map_err(|e| e.to_string())?
        .ok_or("Optimize needs a FLAC file that starts with its stream marker. Remove its ID3v2 tag first.")?;
    let mut region = vec![0; usize::try_from(end - 4).map_err(|e| e.to_string())?];
    file.seek(SeekFrom::Start(4)).map_err(|e| e.to_string())?;
    file.read_exact(&mut region).map_err(|e| e.to_string())?;
    Ok((end, region))
}

fn optimize_flac(
    path: &Path,
    include_cover_data: bool,
    options: &TagOptions,
) -> Result<AudioEntry, String> {
    let (_, region) = flac_region(&mut fs::File::open(path).map_err(|e| e.to_string())?)?;
    let padded = flac_metadata::with_padding(&region, FLAC_PADDING)
        .ok_or("The FLAC metadata blocks are malformed")?;
    if padded == region {
        return unchanged(path, include_cover_data, options);
    }
    staged_write(
        path,
        include_cover_data,
        &[],
        options,
        |staging, original, _| {
            let mut source = fs::File::open(original).map_err(|e| e.to_string())?;
            let (end, region) = flac_region(&mut source)?;
            let padded = flac_metadata::with_padding(&region, FLAC_PADDING)
                .ok_or("The FLAC metadata blocks are malformed")?;
            staging
                .seek(SeekFrom::Start(4))
                .map_err(|e| e.to_string())?;
            staging.write_all(&padded).map_err(|e| e.to_string())?;
            if padded.len() == region.len() {
                crate::write_cost::rewrote_region(padded.len() as u64);
            } else {
                crate::write_cost::rewrote_all();
                source
                    .seek(SeekFrom::Start(end))
                    .map_err(|e| e.to_string())?;
                let mut out = std::io::BufWriter::new(&mut *staging);
                std::io::copy(&mut source, &mut out).map_err(|e| e.to_string())?;
                out.flush().map_err(|e| e.to_string())?;
                drop(out);
                let length = staging.stream_position().map_err(|e| e.to_string())?;
                staging.set_len(length).map_err(|e| e.to_string())?;
            }
            verify_flac(original, staging, end, &padded)
        },
    )
}

/// The new file holds the new metadata and exactly the original frames.
fn verify_flac(
    original: &Path,
    staging: &mut fs::File,
    end: u64,
    padded: &[u8],
) -> Result<(), String> {
    let failed =
        || "The optimized FLAC file did not verify; the original file was kept".to_string();
    let (new_end, region) = flac_region(staging)?;
    if region != padded {
        return Err(failed());
    }
    let mut source = fs::File::open(original).map_err(|e| e.to_string())?;
    let audio = source.metadata().map_err(|e| e.to_string())?.len() - end;
    if staging.metadata().map_err(|e| e.to_string())?.len() - new_end != audio {
        return Err(failed());
    }
    source
        .seek(SeekFrom::Start(end))
        .map_err(|e| e.to_string())?;
    staging
        .seek(SeekFrom::Start(new_end))
        .map_err(|e| e.to_string())?;
    let (mut left, mut right) = (vec![0; 1 << 16], vec![0; 1 << 16]);
    let mut remaining = audio;
    while remaining > 0 {
        let n = remaining.min(left.len() as u64) as usize;
        source
            .read_exact(&mut left[..n])
            .map_err(|e| e.to_string())?;
        staging
            .read_exact(&mut right[..n])
            .map_err(|e| e.to_string())?;
        if left[..n] != right[..n] {
            return Err(failed());
        }
        remaining -= n as u64;
    }
    Ok(())
}
