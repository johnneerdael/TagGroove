//! Tag read/write preferences (https://docs.mp3tag.de/customization/options/tags/).
//! Callers pass these explicitly to every read and write; tag-core keeps no
//! process-wide configuration.
use crate::tags::{validate_user_mappings, UserFieldMapping};
use serde::{Deserialize, Serialize};

/// ID3v2 revision and text encoding written to MPEG files.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Id3v2Format {
    #[default]
    V23Utf16,
    V23Latin1,
    V24Utf8,
}

/// Options → Tags → Mpeg. Applies to MPEG audio (MP1/MP2/MP3).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct MpegTagOptions {
    pub read_id3v1: bool,
    pub read_id3v2: bool,
    pub read_ape: bool,
    pub write_id3v1: bool,
    pub write_id3v2: bool,
    pub write_ape: bool,
    /// Write ID3v2 only when the values do not fit an ID3v1 tag (requires
    /// `write_id3v1`). Existing ID3v2 tags are always updated.
    pub id3v2_only_if_needed: bool,
    pub id3v2_format: Id3v2Format,
    /// The Remove kinds: File → Remove tag strips these. Saving also strips
    /// a selected kind that is not being written.
    pub remove_id3v1: bool,
    pub remove_id3v2: bool,
    pub remove_ape: bool,
}
impl Default for MpegTagOptions {
    fn default() -> Self {
        Self {
            read_id3v1: true,
            read_id3v2: true,
            read_ape: true,
            write_id3v1: true,
            write_id3v2: true,
            write_ape: false,
            id3v2_only_if_needed: false,
            id3v2_format: Id3v2Format::V23Utf16,
            remove_id3v1: true,
            remove_id3v2: true,
            remove_ape: true,
        }
    }
}

/// Options → Tags → Ape, Mpc. Applies to Monkey's Audio, Musepack and WavPack.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct ApeTagOptions {
    pub read_id3v1: bool,
    pub read_ape: bool,
    pub write_id3v1: bool,
    pub write_ape: bool,
    /// As for MPEG: File → Remove tag strips these, saving strips the ones
    /// not written.
    pub remove_id3v1: bool,
    pub remove_id3v2: bool,
    pub remove_ape: bool,
}
impl Default for ApeTagOptions {
    fn default() -> Self {
        Self {
            read_id3v1: true,
            read_ape: true,
            write_id3v1: false,
            write_ape: true,
            remove_id3v1: true,
            // An ID3v2 tag on these formats is never displayed; saving keeps
            // it unless this is enabled.
            remove_id3v2: false,
            remove_ape: true,
        }
    }
}

/// Options → Tags → Advanced, limited to the settings tag-core honours.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct AdvancedTagOptions {
    /// ISO-639-2 code written to new ID3v2 COMM frames.
    pub id3v2_comment_language: String,
    /// Show POPM frames as POPULARIMETER instead of RATING MM/RATING WMP.
    pub popularimeter_ratings: bool,
    /// Use the recommended (video) Matroska mapping for files with a video
    /// track: TITLE at target type 50, ALBUM at 60 and TV episode/season.
    pub matroska_video_mapping: bool,
    /// List chapters (MP4, Matroska, ID3v2 `CHAP`, embedded cue sheets) as
    /// separate rows when files are read.
    pub list_chapters: bool,
    /// Store the values of a multi-value field as data atoms of one MP4 atom
    /// (otherwise one atom per value).
    pub mp4_single_atom: bool,
    /// Leave a user-defined MP4 atom for a field that has a documented atom
    /// where it is until the field changes. Off: every save moves it to the
    /// documented atom.
    pub reuse_mp4_atoms: bool,
    /// The same for user-defined ID3v2 (TXXX) frames.
    pub reuse_id3v2_frames: bool,
}
impl Default for AdvancedTagOptions {
    fn default() -> Self {
        Self {
            id3v2_comment_language: "eng".into(),
            popularimeter_ratings: false,
            matroska_video_mapping: true,
            list_chapters: false,
            mp4_single_atom: true,
            // Mp3tag 3.33.1 and later write the documented mappings by default.
            reuse_mp4_atoms: false,
            reuse_id3v2_frames: false,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct TagOptions {
    pub mpeg: MpegTagOptions,
    pub ape: ApeTagOptions,
    pub advanced: AdvancedTagOptions,
    /// Options → Files → Preserve timestamps of modified files.
    pub preserve_modified_time: bool,
    /// Options → Tags → Mapping, in list order. Functions with a `rules`
    /// parameter use that list; the others (covers, Remove tag, utils) use
    /// these rules for the entry they return.
    pub mappings: Vec<UserFieldMapping>,
}

impl TagOptions {
    pub fn validate(&self) -> Result<(), String> {
        let language = &self.advanced.id3v2_comment_language;
        if language.len() != 3 || !language.bytes().all(|b| b.is_ascii_alphabetic()) {
            return Err("The ID3v2 comment language must be a three-letter ISO-639-2 code".into());
        }
        let mpeg = &self.mpeg;
        if !(mpeg.write_id3v1 || mpeg.write_id3v2 || mpeg.write_ape) {
            return Err("Enable at least one tag type to write for MPEG files".into());
        }
        if !(self.ape.write_id3v1 || self.ape.write_ape) {
            return Err(
                "Enable at least one tag type to write for APE, MPC and WavPack files".into(),
            );
        }
        validate_user_mappings(&self.mappings)
    }
    pub(crate) fn comment_language(&self) -> [u8; 3] {
        let mut code = *b"eng";
        let language = self.advanced.id3v2_comment_language.to_ascii_lowercase();
        if let Ok(bytes) = <[u8; 3]>::try_from(language.as_bytes()) {
            if bytes.iter().all(u8::is_ascii_alphabetic) {
                code = bytes;
            }
        }
        code
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_follow_mp3tag_and_validate() {
        let options = TagOptions::default();
        assert!(options.validate().is_ok());
        assert_eq!(options.mpeg.id3v2_format, Id3v2Format::V23Utf16);
        assert!(options.mpeg.write_id3v1 && options.mpeg.write_id3v2 && !options.mpeg.write_ape);
        assert_eq!(options.comment_language(), *b"eng");
    }

    #[test]
    fn partial_json_is_completed_with_defaults_and_invalid_values_rejected() {
        let options: TagOptions =
            serde_json::from_str(r#"{"mpeg":{"id3v2_format":"v24_utf8"}}"#).unwrap();
        assert_eq!(options.mpeg.id3v2_format, Id3v2Format::V24Utf8);
        assert!(options.mpeg.read_ape);
        let mut invalid = options.clone();
        invalid.advanced.id3v2_comment_language = "en".into();
        assert!(invalid.validate().is_err());
        let mut invalid = options;
        invalid.mpeg.write_id3v1 = false;
        invalid.mpeg.write_id3v2 = false;
        assert!(invalid.validate().is_err());
    }
}
