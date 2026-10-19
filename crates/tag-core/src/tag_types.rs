//! Utils → Remove Tag Types (https://docs.mp3tag.de/utils/): strip chosen tag
//! kinds from MPEG, AAC, TTA and APE-family files regardless of Options → Tags.
use super::*;

/// A tag kind that can be removed on its own.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RemovableTag {
    Id3v1,
    Id3v2,
    Ape,
}
impl RemovableTag {
    fn tag_type(self) -> TagType {
        match self {
            Self::Id3v1 => TagType::Id3v1,
            Self::Id3v2 => TagType::Id3v2,
            Self::Ape => TagType::Ape,
        }
    }
}

/// Remove the given tag kinds. Files without any of them are left untouched and
/// files whose format cannot hold them are rejected. Unlike removal on save,
/// nothing is carried over into the remaining tags.
pub fn remove_tag_kinds_with_options(
    path: &Path,
    kinds: &[RemovableTag],
    include_cover_data: bool,
    options: &TagOptions,
) -> Result<AudioEntry, String> {
    if kinds.is_empty() {
        return Err("Choose at least one tag type to remove".into());
    }
    let file = load(path)?;
    if tag_policy::policy(file.file_type()).is_none() {
        return Err("Tag types can only be removed from MPEG, AAC, TTA, APE, Musepack, WavPack, TAK and OptimFROG files".into());
    }
    let dependent =
        tag_policy::dependent_v1_removal(path, &file, kinds.contains(&RemovableTag::Id3v1))?;
    if !kinds.iter().any(|kind| file.tag(kind.tag_type()).is_some()) && !dependent {
        return if include_cover_data {
            read_entry_with_options(path, &options.mappings, options)
        } else {
            read_entry_summary_with_options(path, &options.mappings, options)
        };
    }
    staged_mutation(
        path,
        include_cover_data,
        options,
        |staging, file, _, effects| {
            effects.removed_v1 |= dependent;
            for kind in [RemovableTag::Ape, RemovableTag::Id3v2, RemovableTag::Id3v1] {
                if kinds.contains(&kind) && file.tag(kind.tag_type()).is_some() {
                    staging
                        .seek(SeekFrom::Start(0))
                        .map_err(|e| e.to_string())?;
                    kind.tag_type()
                        .remove_from(staging, WriteOptions::default())
                        .map_err(|e| error_message(&e))?;
                    effects.removed(kind.tag_type());
                }
            }
            Ok(())
        },
    )
}
