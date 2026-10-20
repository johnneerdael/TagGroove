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
];

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Setting {
    pub label: String,
    pub enabled: bool,
}
#[derive(Clone)]
pub struct Patch(BTreeMap<OptionKey, bool>);
pub struct Import {
    pub patch: Patch,
    pub settings: Vec<Setting>,
    pub warnings: Vec<String>,
}
pub fn parse(bytes: &[u8]) -> Result<Import, String> {
    let mut position = 0usize;
    let mut values = BTreeMap::new();
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
        } else {
            unknown += 1;
        }
        position = end;
    }
    if values.is_empty() {
        return Err("The file contains no supported portable binary settings.".into());
    }
    let settings = values
        .iter()
        .map(|(key, enabled)| Setting {
            label: SPECS
                .iter()
                .find(|spec| spec.key == *key)
                .unwrap()
                .label
                .into(),
            enabled: *enabled,
        })
        .collect();
    Ok(Import {
        patch: Patch(values),
        settings,
        warnings: if unknown == 0 {
            Vec::new()
        } else {
            vec![format!("{unknown} unmapped binary setting records are not imported; their source file is unchanged.")]
        },
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
    value: bool,
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
        for (key, value) in &self.0 {
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
            }
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
        assert_eq!(options.len(), 12);
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
        assert!(parsed.settings[0].enabled);
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
}
