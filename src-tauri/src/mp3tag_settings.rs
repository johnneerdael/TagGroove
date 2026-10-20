//! Read confirmed portable options from native Windows mp3tag.cfg records.
//! Unknown payloads are skipped, never decoded as credentials or copied to settings.
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::BTreeMap;
use std::path::Path;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum OptionKey {
    SaveOnMove,
    NoFolderImage,
    KeepModifiedTime,
    SelectNext,
    Autocomplete,
    ShowDirectory,
    Subfolders,
    UserGenres,
    HidePanel,
    HideFilter,
    TrayIcon,
    MinimizeTray,
    PlaylistAutomatic,
    PlaylistExtended,
    PlaylistRelative,
    PlaylistConfirm,
    PlaylistNoFolders,
    ExportDialog,
}
struct Spec {
    identifier: [u8; 16],
    key: OptionKey,
    label: &'static str,
    inverted: bool,
}
const fn identifier(text: &[u8; 32]) -> [u8; 16] {
    const fn nibble(value: u8) -> u8 {
        match value {
            b'0'..=b'9' => value - b'0',
            b'a'..=b'f' => value - b'a' + 10,
            _ => panic!("Invalid static binary option identifier"),
        }
    }
    let mut bytes = [0; 16];
    let mut index = 0;
    while index < 16 {
        bytes[index] = nibble(text[index * 2]) * 16 + nibble(text[index * 2 + 1]);
        index += 1;
    }
    bytes
}
// Raw byte identifiers, with no UUID byte-order conversion. Bound to the
// 3.36.1 static reader/object/serializer evidence in the migration receipt.
const SPECS: &[Spec] = &[
    Spec {
        identifier: identifier(b"8840da8e2956d74da1853b63f6df9037"),
        key: OptionKey::SaveOnMove,
        label: "Save tags when using arrow keys or a single mouse click",
        inverted: false,
    },
    Spec {
        identifier: identifier(b"ee0051ba892b9941bce4471869a8f2d0"),
        key: OptionKey::NoFolderImage,
        label: "Don’t display first image from file directory as cover art",
        inverted: false,
    },
    Spec {
        identifier: identifier(b"c43475eedd175e42b0d89661bd66f42d"),
        key: OptionKey::KeepModifiedTime,
        label: "Preserve timestamps of modified files",
        inverted: false,
    },
    Spec {
        identifier: identifier(b"53fe21e50af824458cf74de7edade298"),
        key: OptionKey::SelectNext,
        label: "Automatically select next file when editing in file view",
        inverted: false,
    },
    Spec {
        identifier: identifier(b"a17772f7656a2b4fb710657ed41cb01d"),
        key: OptionKey::Autocomplete,
        label: "Auto-completion at selection boxes",
        inverted: false,
    },
    Spec {
        identifier: identifier(b"6317969e3cb5424781aab010efedb4a2"),
        key: OptionKey::ShowDirectory,
        label: "Show current directory in window title",
        inverted: false,
    },
    Spec {
        identifier: identifier(b"fbb012dd766b664493dff2ac5f006238"),
        key: OptionKey::Subfolders,
        label: "Read subfolders",
        inverted: false,
    },
    Spec {
        identifier: identifier(b"a635489304121948aceaf4ad079f3fa1"),
        key: OptionKey::UserGenres,
        label: "Show only user-defined genres",
        inverted: false,
    },
    Spec {
        identifier: identifier(b"a0f6c4c7dcd9b44fb7d973852061f2cb"),
        key: OptionKey::HidePanel,
        label: "Show tag panel",
        inverted: true,
    },
    Spec {
        identifier: identifier(b"86fd6bff815ed0488b445b271955c35c"),
        key: OptionKey::HideFilter,
        label: "Show filter",
        inverted: true,
    },
    Spec {
        identifier: identifier(b"07a26324a1f7d84782538bb89e2a57e8"),
        key: OptionKey::TrayIcon,
        label: "Always show icon in notification area",
        inverted: false,
    },
    Spec {
        identifier: identifier(b"d5c6e9f135cd614aa8f3d9f513f774f1"),
        key: OptionKey::MinimizeTray,
        label: "Minimize to notification area",
        inverted: false,
    },
    Spec {
        identifier: identifier(b"b2efeb13fa6c2c44b8386c2182ac5965"),
        key: OptionKey::PlaylistAutomatic,
        label: "Create playlist automatically after saving tags or renaming files",
        inverted: false,
    },
    Spec {
        identifier: identifier(b"04d7ba2f4f3bfa4d8b0c1ccf2ab1c361"),
        key: OptionKey::PlaylistExtended,
        label: "Write extended info",
        inverted: false,
    },
    Spec {
        identifier: identifier(b"59fc84e13a7c8044b3192709f11d754d"),
        key: OptionKey::PlaylistRelative,
        label: "Entries relative to working directory (if possible)",
        inverted: false,
    },
    Spec {
        identifier: identifier(b"f25e55356aa23840bb4e104ab58ce1f3"),
        key: OptionKey::PlaylistConfirm,
        label: "Show confirm filename dialog",
        inverted: false,
    },
    Spec {
        identifier: identifier(b"b0b41d103f16944cb184a331ae6136a9"),
        key: OptionKey::PlaylistNoFolders,
        label: "Don't create directories for playlists",
        inverted: false,
    },
    Spec {
        identifier: identifier(b"97a81ca5caca824ab21be214338c735e"),
        key: OptionKey::ExportDialog,
        label: "Show export selection dialog",
        inverted: false,
    },
];
const EXTENDED_MODE: [u8; 16] = identifier(b"16ba6ee64495f544a1f46ad5ed57b876");
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum TextKey {
    PlaylistFilename,
    PlaylistInfo,
}
struct TextSpec {
    identifier: [u8; 16],
    key: TextKey,
    label: &'static str,
}
const TEXT_SPECS: &[TextSpec] = &[
    TextSpec {
        identifier: identifier(b"7eaf6b64ce85a54196043a3c8cfb1d2e"),
        key: TextKey::PlaylistFilename,
        label: "Filename of playlist",
    },
    TextSpec {
        identifier: identifier(b"aa0ec86bea2e7a40a7350a520bcf4a46"),
        key: TextKey::PlaylistInfo,
        label: "Extended info format string",
    },
];
const TEXT_LIMIT: usize = 4 * 1024 * 1024;
fn is_false(value: &bool) -> bool {
    !value
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Setting {
    pub label: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub verbatim: bool,
}
#[derive(Clone)]
pub struct Patch {
    booleans: BTreeMap<OptionKey, bool>,
    extended_mode: Option<&'static str>,
    text: BTreeMap<TextKey, String>,
}
pub struct Import {
    pub patch: Patch,
    pub settings: Vec<Setting>,
    pub warnings: Vec<String>,
}
pub fn parse(bytes: &[u8]) -> Result<Import, String> {
    let mut position = 0usize;
    let mut values = BTreeMap::new();
    let mut extended_mode = None;
    let mut text = BTreeMap::new();
    let mut unknown = 0usize;
    while position < bytes.len() {
        let header_end = position
            .checked_add(20)
            .ok_or("Binary settings length overflow")?;
        let header = bytes
            .get(position..header_end)
            .ok_or("Binary settings record header is incomplete")?;
        let length = u32::from_le_bytes(header[16..20].try_into().unwrap()) as usize;
        let end = header_end
            .checked_add(length)
            .ok_or("Binary settings length overflow")?;
        let payload = bytes
            .get(header_end..end)
            .ok_or("Binary settings record payload is incomplete")?;
        if let Some(spec) = SPECS.iter().find(|spec| header[..16] == spec.identifier) {
            // Native boolean readers consume one byte; the record envelope
            // skips any unconsumed extension bytes. Later records win.
            let value = *payload
                .first()
                .ok_or("Binary boolean setting has no value")?
                != 0;
            values.insert(spec.key, value ^ spec.inverted);
        } else if header[..16] == EXTENDED_MODE {
            // The native reader consumes a little-endian dword. Its save
            // handler saves for 1, ignores for 2 and prompts for any other value.
            let raw = u32::from_le_bytes(
                payload
                    .get(..4)
                    .ok_or("Binary extended-tag save mode is incomplete")?
                    .try_into()
                    .unwrap(),
            );
            extended_mode = Some(match raw {
                1 => "save",
                2 => "ignore",
                _ => "ask",
            });
        } else if let Some(spec) = TEXT_SPECS
            .iter()
            .find(|spec| header[..16] == spec.identifier)
        {
            // The native writer emits raw UTF-8 bytes without a terminator;
            // its reader's string setter stops at the first NUL.
            let value = payload.split(|byte| *byte == 0).next().unwrap();
            if value.len() > TEXT_LIMIT {
                return Err("A portable binary text setting exceeds 4 MB.".into());
            }
            let value = std::str::from_utf8(value)
                .map_err(|_| "A portable binary text setting is not valid UTF-8.")?;
            text.insert(spec.key, value.to_owned());
        } else {
            unknown += 1;
        }
        position = end;
    }
    let mut warnings = Vec::new();
    if let Some(filename) = text.get(&TextKey::PlaylistFilename) {
        let bytes = filename.as_bytes();
        let windows_drive =
            bytes.first().is_some_and(u8::is_ascii_alphabetic) && bytes.get(1) == Some(&b':');
        let reason = if filename.trim().is_empty() {
            Some("The empty playlist filename is not imported; the current filename is kept.")
        } else if !cfg!(windows) && (windows_drive || filename.contains('\\')) {
            Some("The Windows-specific playlist filename is not imported on this platform; review Options → Playlist. The current filename is kept.")
        } else {
            None
        };
        if let Some(reason) = reason {
            warnings.push(reason.into());
            text.remove(&TextKey::PlaylistFilename);
        }
    }
    if values.is_empty() && extended_mode.is_none() && text.is_empty() {
        if !warnings.is_empty() {
            return Err(warnings.join(" "));
        }
        return Err("The file contains no supported portable binary settings.".into());
    }
    let mut settings: Vec<Setting> = values
        .iter()
        .map(|(key, enabled)| Setting {
            label: SPECS
                .iter()
                .find(|spec| spec.key == *key)
                .unwrap()
                .label
                .into(),
            enabled: Some(*enabled),
            value: None,
            verbatim: false,
        })
        .collect();
    if let Some(mode) = extended_mode {
        settings.push(Setting {
            label: "Save mode at extended tag dialog".into(),
            enabled: None,
            value: Some(
                match mode {
                    "save" => "Save changes",
                    "ignore" => "Ignore changes",
                    _ => "Ask",
                }
                .into(),
            ),
            verbatim: false,
        });
    }
    for (key, value) in &text {
        settings.push(Setting {
            label: TEXT_SPECS
                .iter()
                .find(|spec| spec.key == *key)
                .unwrap()
                .label
                .into(),
            enabled: None,
            value: Some(value.clone()),
            verbatim: true,
        });
    }
    if unknown > 0 {
        warnings.push(format!("{unknown} unmapped binary setting records are not imported; their source file is unchanged."));
    }
    Ok(Import {
        patch: Patch {
            booleans: values,
            extended_mode,
            text,
        },
        settings,
        warnings,
    })
}

fn settings<'a>(
    slot: &'a mut Option<crate::settings::Settings>,
    directory: &Path,
) -> Result<&'a mut crate::settings::Settings, String> {
    if slot.is_none() {
        let value = match std::fs::read(directory.join("settings.json")) {
            Ok(bytes) => serde_json::from_slice::<crate::settings::Settings>(&bytes)
                .map_err(|error| error.to_string())?,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                crate::settings::Settings::default()
            }
            Err(error) => return Err(error.to_string()),
        };
        value.validate()?;
        *slot = Some(value);
    }
    Ok(slot.as_mut().unwrap())
}
fn panel<'a>(
    slot: &'a mut Option<crate::tag_panel::Settings>,
    directory: &Path,
) -> Result<&'a mut crate::tag_panel::Settings, String> {
    if slot.is_none() {
        *slot = Some(crate::tag_panel::load(&directory.join("tag-panel.json"))?);
    }
    Ok(slot.as_mut().unwrap())
}
fn page(
    settings: &mut crate::settings::Settings,
    id: &str,
    key: &str,
    value: impl Into<serde_json::Value>,
) -> Result<(), String> {
    settings
        .pages
        .entry(id.into())
        .or_insert_with(|| json!({}))
        .as_object_mut()
        .ok_or("The current options page is not an object")?
        .insert(key.into(), value.into());
    Ok(())
}
impl Patch {
    /// Work only in the caller's unpublished configuration stage. Browser
    /// preferences are returned for the same durable window/restore receipt.
    pub fn apply(&self, directory: &Path) -> Result<BTreeMap<String, String>, String> {
        let mut options = None;
        let mut tag_panel = None;
        let mut preferences = BTreeMap::new();
        for (key, value) in &self.booleans {
            match key {
                OptionKey::SaveOnMove => page(
                    settings(&mut options, directory)?,
                    "tags",
                    "save_on_move",
                    *value,
                )?,
                OptionKey::NoFolderImage => page(
                    settings(&mut options, directory)?,
                    "tags",
                    "no_folder_image",
                    *value,
                )?,
                OptionKey::KeepModifiedTime => {
                    settings(&mut options, directory)?
                        .tags
                        .preserve_modified_time = *value
                }
                OptionKey::SelectNext => page(
                    settings(&mut options, directory)?,
                    "general",
                    "auto_select_next",
                    *value,
                )?,
                OptionKey::Autocomplete => page(
                    settings(&mut options, directory)?,
                    "general",
                    "autocomplete",
                    *value,
                )?,
                OptionKey::ShowDirectory => {
                    settings(&mut options, directory)?
                        .general
                        .show_directory_in_title = *value
                }
                OptionKey::Subfolders => {
                    settings(&mut options, directory)?.directories.subfolders = *value
                }
                OptionKey::UserGenres => {
                    panel(&mut tag_panel, directory)?
                        .field_values
                        .only_user_genres = *value
                }
                OptionKey::HidePanel => panel(&mut tag_panel, directory)?.panel.show_panel = *value,
                OptionKey::HideFilter => {
                    preferences.insert("opentag.filter-shown".into(), value.to_string());
                }
                OptionKey::TrayIcon => page(
                    settings(&mut options, directory)?,
                    "general",
                    "tray_icon",
                    *value,
                )?,
                OptionKey::MinimizeTray => page(
                    settings(&mut options, directory)?,
                    "general",
                    "minimize_to_tray",
                    *value,
                )?,
                OptionKey::PlaylistAutomatic => page(
                    settings(&mut options, directory)?,
                    "playlist",
                    "auto_create",
                    *value,
                )?,
                OptionKey::PlaylistExtended => page(
                    settings(&mut options, directory)?,
                    "playlist",
                    "extended",
                    *value,
                )?,
                OptionKey::PlaylistRelative => page(
                    settings(&mut options, directory)?,
                    "playlist",
                    "relative",
                    *value,
                )?,
                OptionKey::PlaylistConfirm => page(
                    settings(&mut options, directory)?,
                    "playlist",
                    "confirm_filename",
                    *value,
                )?,
                OptionKey::PlaylistNoFolders => page(
                    settings(&mut options, directory)?,
                    "playlist",
                    "dont_create_directories",
                    *value,
                )?,
                OptionKey::ExportDialog => page(
                    settings(&mut options, directory)?,
                    "export",
                    "show_dialog",
                    *value,
                )?,
            }
        }
        if let Some(mode) = self.extended_mode {
            page(
                settings(&mut options, directory)?,
                "tags",
                "extended_save_mode",
                mode,
            )?;
        }
        for (key, value) in &self.text {
            let name = match key {
                TextKey::PlaylistFilename => "filename",
                TextKey::PlaylistInfo => "extinf_format",
            };
            page(
                settings(&mut options, directory)?,
                "playlist",
                name,
                value.as_str(),
            )?;
        }
        if let Some(options) = options {
            crate::settings::save(&directory.join("settings.json"), &options)?;
        }
        if let Some(tag_panel) = tag_panel {
            crate::tag_panel::save(&directory.join("tag-panel.json"), &tag_panel)?;
        }
        Ok(preferences)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn record(id: &[u8], value: &[u8]) -> Vec<u8> {
        [id, &(value.len() as u32).to_le_bytes(), value].concat()
    }
    fn native_id(text: &str) -> Vec<u8> {
        (0..text.len())
            .step_by(2)
            .map(|index| u8::from_str_radix(&text[index..index + 2], 16).unwrap())
            .collect()
    }
    #[test]
    fn binary_settings_migration_matches_the_static_manifest_for_zero_one_and_noncanonical_true() {
        let fixture: serde_json::Value = serde_json::from_str(include_str!(
            "../../tests/fixtures/mp3tag-settings-booleans.json"
        ))
        .unwrap();
        let options = fixture["options"].as_array().unwrap();
        assert_eq!(options.len(), 18);
        for raw in [0, 1, 255] {
            let bytes = options
                .iter()
                .flat_map(|option| {
                    record(&native_id(option["identifier"].as_str().unwrap()), &[raw])
                })
                .collect::<Vec<_>>();
            let parsed = parse(&bytes).unwrap();
            assert_eq!(parsed.settings.len(), options.len());
            let temp = tempfile::tempdir().unwrap();
            let preferences = parsed.patch.apply(temp.path()).unwrap();
            let settings = crate::settings::load(&temp.path().join("settings.json")).0;
            let panel = crate::tag_panel::load(&temp.path().join("tag-panel.json")).unwrap();
            let actual = json!({"settings":settings,"panel":panel,"preferences":preferences});
            for option in options {
                let mut value = &actual[option["component"].as_str().unwrap()];
                for key in option["path"].as_array().unwrap() {
                    value = &value[key.as_str().unwrap()];
                }
                let expected = (raw != 0) ^ option["inverted"].as_bool().unwrap();
                if option["component"] == "preferences" {
                    assert_eq!(value.as_str().unwrap(), expected.to_string());
                } else {
                    assert_eq!(
                        value,
                        &json!(expected),
                        "{} raw={raw}",
                        option["native_key"]
                    );
                }
            }
        }
    }
    #[test]
    fn binary_settings_migration_rejects_incomplete_headers_values_and_lengths() {
        let id = native_id("8840da8e2956d74da1853b63f6df9037");
        let valid = record(&id, &[1]);
        for length in 0..valid.len() {
            assert!(parse(&valid[..length]).is_err(), "prefix={length}");
        }
        assert!(parse(&record(&id, &[])).is_err());
        let malicious = [id, u32::MAX.to_le_bytes().to_vec()].concat();
        assert!(parse(&malicious).is_err());
    }
    #[test]
    fn binary_settings_migration_skips_unknown_records_and_uses_the_last_known_record() {
        let id = native_id("8840da8e2956d74da1853b63f6df9037");
        let bytes = [
            record(&id, &[0]),
            record(&[0x99; 16], b"unknown payload"),
            record(&id, &[2, 9, 8]),
        ]
        .concat();
        let parsed = parse(&bytes).unwrap();
        assert_eq!(parsed.settings.len(), 1);
        assert_eq!(parsed.settings[0].enabled, Some(true));
        assert_eq!(parsed.warnings.len(), 1);
        assert!(!parsed.warnings[0].contains("unknown payload"));
        assert!(parse(&record(&[0x99; 16], b"unknown payload")).is_err());
    }
    #[test]
    fn binary_settings_migration_refuses_invalid_current_options_without_replacing_them() {
        let id = native_id("8840da8e2956d74da1853b63f6df9037");
        let parsed = parse(&record(&id, &[1])).unwrap();
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("settings.json");
        std::fs::write(&path, b"invalid current options").unwrap();
        assert!(parsed.patch.apply(temp.path()).is_err());
        assert_eq!(std::fs::read(path).unwrap(), b"invalid current options");
    }
    #[test]
    fn binary_settings_extended_mode_matches_native_u32_branches() {
        let id = native_id("16ba6ee64495f544a1f46ad5ed57b876");
        for (raw, expected, label) in [
            (0u32, "ask", "Ask"),
            (1, "save", "Save changes"),
            (2, "ignore", "Ignore changes"),
            (3, "ask", "Ask"),
            (u32::MAX, "ask", "Ask"),
        ] {
            let parsed = parse(&record(&id, &raw.to_le_bytes())).unwrap();
            let preview = serde_json::to_value(&parsed.settings).unwrap();
            assert_eq!(preview[0]["label"], "Save mode at extended tag dialog");
            assert_eq!(preview[0]["value"], label);
            assert!(preview[0].get("enabled").is_none());
            let temp = tempfile::tempdir().unwrap();
            parsed.patch.apply(temp.path()).unwrap();
            let settings = crate::settings::load(&temp.path().join("settings.json")).0;
            assert_eq!(settings.pages["tags"]["extended_save_mode"], expected);
        }
        for length in 0..4 {
            assert!(parse(&record(&id, &vec![0; length])).is_err());
        }
        // Extra payload bytes are skipped; later records replace earlier ones.
        let parsed =
            parse(&[record(&id, &[1, 0, 0, 0, 9]), record(&id, &[2, 0, 0, 0])].concat()).unwrap();
        assert_eq!(parsed.settings.len(), 1);
        let temp = tempfile::tempdir().unwrap();
        parsed.patch.apply(temp.path()).unwrap();
        assert_eq!(
            crate::settings::load(&temp.path().join("settings.json"))
                .0
                .pages["tags"]["extended_save_mode"],
            "ignore"
        );
    }
    #[test]
    fn binary_settings_text_keeps_unicode_and_authored_templates_verbatim() {
        let fixture: serde_json::Value = serde_json::from_str(include_str!(
            "../../tests/fixtures/mp3tag-settings-text.json"
        ))
        .unwrap();
        let filename = "  %artist% - Björk 🎵.m3u8  ";
        let format = " %artist% | $if(%title%,%title%,Ask) ";
        let options = fixture["options"].as_array().unwrap();
        let bytes = options
            .iter()
            .zip([filename, format])
            .flat_map(|(option, value)| {
                record(
                    &native_id(option["identifier"].as_str().unwrap()),
                    value.as_bytes(),
                )
            })
            .collect::<Vec<_>>();
        let parsed = parse(&bytes).unwrap();
        let wire = serde_json::to_value(&parsed.settings).unwrap();
        assert_eq!(wire[0]["value"], filename);
        assert_eq!(wire[1]["value"], format);
        assert_eq!(wire[0]["verbatim"], true);
        assert_eq!(wire[1]["verbatim"], true);
        let temp = tempfile::tempdir().unwrap();
        let mut settings = crate::settings::Settings::default();
        settings.pages.insert(
            "playlist".into(),
            json!({"filename":"before.m3u8", "relative":false, "generator_partition":"%album%"}),
        );
        crate::settings::save(&temp.path().join("settings.json"), &settings).unwrap();
        assert!(parsed.patch.apply(temp.path()).unwrap().is_empty());
        let actual = crate::settings::load(&temp.path().join("settings.json")).0;
        assert_eq!(actual.pages["playlist"]["filename"], filename);
        assert_eq!(actual.pages["playlist"]["extinf_format"], format);
        assert_eq!(actual.pages["playlist"]["relative"], false);
        assert_eq!(actual.pages["playlist"]["generator_partition"], "%album%");
    }
    #[test]
    fn binary_settings_text_obeys_native_nul_and_last_record_rules_with_bounded_decoding() {
        let id = native_id("aa0ec86bea2e7a40a7350a520bcf4a46");
        let parsed = parse(
            &[
                record(&id, b"old"),
                record(&id, b"Ask\0\xffnot part of the value"),
            ]
            .concat(),
        )
        .unwrap();
        assert_eq!(parsed.settings.len(), 1);
        let temp = tempfile::tempdir().unwrap();
        parsed.patch.apply(temp.path()).unwrap();
        assert_eq!(
            crate::settings::load(&temp.path().join("settings.json"))
                .0
                .pages["playlist"]["extinf_format"],
            "Ask"
        );
        assert!(parse(&record(&id, b"\xff")).is_err());
        assert!(parse(&record(&id, &vec![b'a'; 4 * 1024 * 1024 + 1])).is_err());
        let parsed = parse(&record(&id, b"")).unwrap();
        assert_eq!(parsed.settings[0].value.as_deref(), Some(""));
        let mut cut = record(&id, b"value");
        cut.pop();
        assert!(parse(&cut).is_err());
    }
    #[test]
    fn binary_settings_text_skips_unusable_filenames_without_losing_other_settings() {
        let id = native_id("7eaf6b64ce85a54196043a3c8cfb1d2e");
        let flag = native_id("8840da8e2956d74da1853b63f6df9037");
        let mut values = vec!["", "   "];
        if !cfg!(windows) {
            values.extend([
                "C:\\Playlists\\all.m3u8",
                "C:/Playlists/all.m3u8",
                "\\\\server\\share\\all.m3u8",
            ]);
        }
        for value in values {
            // A later unusable value must not resurrect an earlier filename.
            let parsed = parse(
                &[
                    record(&id, b"earlier.m3u8"),
                    record(&flag, &[1]),
                    record(&id, value.as_bytes()),
                ]
                .concat(),
            )
            .unwrap();
            assert_eq!(parsed.settings.len(), 1);
            assert_eq!(parsed.warnings.len(), 1);
            let temp = tempfile::tempdir().unwrap();
            let mut settings = crate::settings::Settings::default();
            settings
                .pages
                .insert("playlist".into(), json!({"filename":"keep.m3u8"}));
            crate::settings::save(&temp.path().join("settings.json"), &settings).unwrap();
            parsed.patch.apply(temp.path()).unwrap();
            let actual = crate::settings::load(&temp.path().join("settings.json")).0;
            assert_eq!(actual.pages["playlist"]["filename"], "keep.m3u8");
            assert_eq!(actual.pages["tags"]["save_on_move"], true);
        }
        if cfg!(windows) {
            assert!(parse(&record(&id, b"C:\\Playlists\\all.m3u8")).is_ok());
        } else {
            assert!(
                matches!(parse(&record(&id,b"C:\\Playlists\\all.m3u8")),Err(reason) if reason.contains("Windows-specific"))
            );
        }
    }
}
