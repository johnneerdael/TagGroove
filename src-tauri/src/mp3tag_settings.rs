//! Read confirmed portable options from native Windows mp3tag.cfg records.
//! Unknown payloads are skipped, never decoded as credentials or copied to settings.
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::BTreeMap;
use std::path::Path;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum OptionKey {
    Page(&'static str, &'static str),
    LibraryEnabled,
    LibraryRestrict,
    CoverAnswer,
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
    Message(&'static str),
    NaturalSort,
    SelectAdded,
    KeepMoved,
    Advanced(&'static str),
    PreferGenres,
    ExportAppend,
    ExportMultiple,
    Numbering(&'static str),
    NumberingFieldEnabled(&'static str),
    NumberingPadding(&'static str),
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
        identifier: identifier(b"7bbbf22d6046a94d849c82eb792d0910"),
        key: OptionKey::LibraryEnabled,
        label: "Enable Library",
        inverted: false,
    },
    Spec {
        identifier: identifier(b"65378755075c9046b6d4ac4cd6346af9"),
        key: OptionKey::LibraryRestrict,
        label: "Restrict Library to included directories",
        inverted: false,
    },
    Spec {
        identifier: identifier(b"eada7c7f01da5f4a849392b72b37ef08"),
        key: OptionKey::Page("playlist", "write_bom"),
        label: "Write playlist byte order mark (BOM)",
        inverted: false,
    },
    Spec {
        identifier: identifier(b"e60558750d35d148b3f22c2c229005dd"),
        key: OptionKey::Page("export", "dont_create_directories"),
        label: "Export: do not create directories",
        inverted: false,
    },
    Spec {
        identifier: identifier(b"7c7d406cdfd18b48a914db716854fc2f"),
        key: OptionKey::Page("updates", "release_notes"),
        label: "Display release notes for new versions",
        inverted: false,
    },
    Spec {
        identifier: identifier(b"f0bb1392e33e1041801e42ce3b64f1b7"),
        key: OptionKey::CoverAnswer,
        label: "Keep existing covers when the import question is disabled",
        inverted: false,
    },
    Spec {
        identifier: identifier(b"81e9cd2aac92e14e9c71cf21ae4e0510"),
        key: OptionKey::NumberingPadding("track_digits"),
        label: "Numbering: Leading zeros for tracknumbers",
        inverted: false,
    },
    Spec {
        identifier: identifier(b"e4d56fdc8c9b154c99e8c0adf4f25b22"),
        key: OptionKey::NumberingPadding("disc_digits"),
        label: "Numbering: Leading zeros for discnumbers",
        inverted: false,
    },
    Spec {
        identifier: identifier(b"2da5f759503fa947932b7e5310a032fa"),
        key: OptionKey::NumberingFieldEnabled("next_disc_on_field"),
        label: "Numbering: Next disc on change of field",
        inverted: false,
    },
    Spec {
        identifier: identifier(b"b9f3835eafb14c4ba2ca0a1e319f33a3"),
        key: OptionKey::NumberingFieldEnabled("reset_disc_on_field"),
        label: "Numbering: Reset disc counter on change of field",
        inverted: false,
    },
    Spec {
        identifier: identifier(b"f4a314340ea7c044a6de766e7a2adc74"),
        key: OptionKey::Numbering("track_total"),
        label: "Numbering: Save total track count",
        inverted: false,
    },
    Spec {
        identifier: identifier(b"aaeda90a7158644ab6371c40786599a8"),
        key: OptionKey::Numbering("next_disc_on_directory"),
        label: "Numbering: Next disc on change of directory",
        inverted: false,
    },
    Spec {
        identifier: identifier(b"e242331383e5a2448bec645e3eabdd87"),
        key: OptionKey::Numbering("reset_track_on_next_disc"),
        label: "Numbering: Reset track counter on next disc",
        inverted: false,
    },
    Spec {
        identifier: identifier(b"c90c8b8c433f5f4e8c86ba63e902fb8b"),
        key: OptionKey::Numbering("disc_enabled"),
        label: "Numbering: Disc numbers",
        inverted: false,
    },
    Spec {
        identifier: identifier(b"5a98eb5f2200764a94daf6d4d9300c0b"),
        key: OptionKey::Numbering("disc_total"),
        label: "Numbering: Save total disc count",
        inverted: false,
    },
    Spec {
        identifier: identifier(b"a91684289cba6f498fd857faeaa3749c"),
        key: OptionKey::ExportAppend,
        label: "Append export data",
        inverted: false,
    },
    Spec {
        identifier: identifier(b"9e917d2becf8af4793061746fdf7bad5"),
        key: OptionKey::ExportMultiple,
        label: "Create one export file per partition",
        inverted: false,
    },
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
    Spec {
        identifier: identifier(b"a0d12c9bfeca104a9e28c4d677eeed5b"),
        key: OptionKey::Message("after_save"),
        label: "Notify after saving tags",
        inverted: false,
    },
    Spec {
        identifier: identifier(b"50ee520115df554ca05d5a2318e7dbb1"),
        key: OptionKey::Message("after_delete"),
        label: "Notify after removing tags",
        inverted: false,
    },
    Spec {
        identifier: identifier(b"c926caafa65b39429d863d45454e9457"),
        key: OptionKey::Message("after_change"),
        label: "Notify after actions and converters",
        inverted: false,
    },
    Spec {
        identifier: identifier(b"3e5445fa1d3cc94ea144ac8bcc08cbe6"),
        key: OptionKey::Message("confirm_cut_tags"),
        label: "Confirm cutting tags",
        inverted: false,
    },
    Spec {
        identifier: identifier(b"a5fea160d8e1d543b742a86a86a8f2d5"),
        key: OptionKey::Message("confirm_paste_tags"),
        label: "Confirm pasting tags",
        inverted: false,
    },
    Spec {
        identifier: identifier(b"f943c466dfced441ab7d90819f99baaa"),
        key: OptionKey::Message("confirm_export"),
        label: "Offer to open exported files and playlists",
        inverted: false,
    },
    Spec {
        identifier: identifier(b"c2b1b77b33dd5e42b8e2bc0a90d00586"),
        key: OptionKey::Message("confirm_delete_files"),
        label: "Confirm deleting files",
        inverted: false,
    },
    Spec {
        identifier: identifier(b"215db77b5ec4934c9d28593f17c07afd"),
        key: OptionKey::Message("action_warnings"),
        label: "Show warnings from Actions",
        inverted: false,
    },
    Spec {
        identifier: identifier(b"0175173ea37ff746bf47382a1c7c3f76"),
        key: OptionKey::Message("unsaved_changes"),
        label: "Ask about unsaved changes",
        inverted: false,
    },
    Spec {
        identifier: identifier(b"fc0a600b58c13744ab155cb882f9d901"),
        key: OptionKey::Message("confirm_import_covers"),
        label: "Ask before importing cover art",
        inverted: false,
    },
    Spec {
        identifier: identifier(b"6ffacd136f47034d95660ba5e0469de8"),
        key: OptionKey::Message("confirm_remove_tags"),
        label: "Confirm removing tags",
        inverted: false,
    },
    Spec {
        identifier: identifier(b"3ec5faaa30c28a4b8ea2c470e1f7a276"),
        key: OptionKey::NaturalSort,
        label: "Use natural sorting (1, 2, 11, …)",
        inverted: false,
    },
    Spec {
        identifier: identifier(b"de10ea102047ae448c2c53c4ba58d55e"),
        key: OptionKey::SelectAdded,
        label: "Select files automatically when they are added to the list",
        inverted: false,
    },
    Spec {
        identifier: identifier(b"8d7feb69eab69f4883076efb11f4b20c"),
        key: OptionKey::KeepMoved,
        label: "Keep moved files in the file list",
        inverted: false,
    },
    Spec {
        identifier: identifier(b"243cae0bb6d7804b932f5ea2f2d2a55d"),
        key: OptionKey::Advanced("popularimeter_ratings"),
        label: "Always display POPULARIMETER values for ID3v2 ratings",
        inverted: false,
    },
    Spec {
        identifier: identifier(b"ac740b3914557e479e4b32dbbbacaab0"),
        key: OptionKey::Advanced("list_chapters"),
        label: "List chapters as separate files",
        inverted: false,
    },
    Spec {
        identifier: identifier(b"e4f596ec68aba54da0bf8bff3a489238"),
        key: OptionKey::Advanced("matroska_video_mapping"),
        label: "Use recommended mapping for Matroska video",
        inverted: false,
    },
    Spec {
        identifier: identifier(b"40b8c799a9b6ac4080064606a8c10200"),
        key: OptionKey::Advanced("mp4_single_atom"),
        label: "Use single MP4 atom for multiple values",
        inverted: false,
    },
    Spec {
        identifier: identifier(b"a8c68011b761684596bd8f68db51d329"),
        key: OptionKey::Advanced("reuse_mp4_atoms"),
        label: "Reuse unmodified MP4 atoms",
        inverted: false,
    },
    Spec {
        identifier: identifier(b"8b86f06a405be14991918e4fc1cf13d8"),
        key: OptionKey::Advanced("reuse_id3v2_frames"),
        label: "Reuse unmodified ID3v2 frames",
        inverted: false,
    },
    Spec {
        identifier: identifier(b"a5a066496e5a1d409b6a874bb814b65b"),
        key: OptionKey::PreferGenres,
        label: "Prefer genre values from tags",
        inverted: false,
    },
];
const GRID: [u8; 16] = identifier(b"10ad9a5de1a201419efb4f510bd262f4");
const EXTENDED_MODE: [u8; 16] = identifier(b"16ba6ee64495f544a1f46ad5ed57b876");
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum TextKey {
    PlaylistFilename,
    PlaylistInfo,
    CommentLanguage,
    RestrictExtensions,
    FolderSeparator,
    ExportPartition,
    ExportConfiguration,
    ExportFilename,
    NumberingNextField,
    NumberingResetField,
    CoverDirectory,
    SourceCoverFilename,
    Language,
    GeneratorFilename,
    GeneratorPartition,
}
const NUMBERING_FIELDS: [(&str, TextKey); 2] = [
    ("next_disc_on_field", TextKey::NumberingNextField),
    ("reset_disc_on_field", TextKey::NumberingResetField),
];
struct TextSpec {
    identifier: [u8; 16],
    key: TextKey,
    label: &'static str,
}
const TEXT_SPECS: &[TextSpec] = &[
    TextSpec {
        identifier: identifier(b"f64318074ac91b4db1b909e2ce5a695c"),
        key: TextKey::SourceCoverFilename,
        label: "Default file name for cover art",
    },
    TextSpec {
        identifier: identifier(b"1a0bfd2a930e704dac9262461ddcd9c6"),
        key: TextKey::Language,
        label: "Language",
    },
    TextSpec {
        identifier: identifier(b"312fa18b404068489ebd20ea4183cc24"),
        key: TextKey::GeneratorFilename,
        label: "Generate Playlists: filename",
    },
    TextSpec {
        identifier: identifier(b"0e3b78b6452b224789abc77122e2d677"),
        key: TextKey::GeneratorPartition,
        label: "Generate Playlists: partition",
    },
    TextSpec {
        identifier: identifier(b"a7a810227a5a8d4db511e77851eb10dd"),
        key: TextKey::CoverDirectory,
        label: "Default folder for Add cover",
    },
    TextSpec {
        identifier: identifier(b"36d2f91d9589434ea5669328ab6b9486"),
        key: TextKey::NumberingNextField,
        label: "Numbering: Next-disc field",
    },
    TextSpec {
        identifier: identifier(b"89d6949ac805e64b9eab9c7fa533a0db"),
        key: TextKey::NumberingResetField,
        label: "Numbering: Reset-disc field",
    },
    TextSpec {
        identifier: identifier(b"e31f66267bd2a34baffa13f42d34fd95"),
        key: TextKey::ExportPartition,
        label: "Export partition format",
    },
    TextSpec {
        identifier: identifier(b"536d231fc389234bafa84663f0986a0c"),
        key: TextKey::ExportConfiguration,
        label: "Remembered export configuration",
    },
    TextSpec {
        identifier: identifier(b"d1c3c5a4c95780489c008de588e2eeea"),
        key: TextKey::ExportFilename,
        label: "Remembered export filename",
    },
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
    TextSpec {
        identifier: identifier(b"8eec759f9a170443abf0e47af87c1079"),
        key: TextKey::CommentLanguage,
        label: "ID3v2 comment language code (ISO-639-2)",
    },
    TextSpec {
        identifier: identifier(b"0c7f0a198af644488be02a0d19631585"),
        key: TextKey::RestrictExtensions,
        label: "Restrict incoming files to",
    },
    TextSpec {
        identifier: identifier(b"375e4b6e9c45af49a77caac696ced2b6"),
        key: TextKey::FolderSeparator,
        label: "Replace folder separator in tag values with",
    },
];
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum MaskKey {
    MpegRead,
    MpegWrite,
    MpegRemove,
    ApeRead,
    ApeWrite,
    ApeRemove,
}
struct MaskRule {
    field: &'static str,
    mask: u32,
    label: &'static str,
}
struct MaskSpec {
    identifier: [u8; 16],
    key: MaskKey,
    group: &'static str,
    known: u32,
    writable: u32,
    rules: &'static [MaskRule],
}
const MASK_SPECS: &[MaskSpec] = &[
    MaskSpec {
        identifier: identifier(b"10d6c497feac8340a1d9b78566f9aede"),
        key: MaskKey::MpegRead,
        group: "mpeg",
        known: 11,
        writable: 0,
        rules: &[
            MaskRule {
                field: "read_id3v1",
                mask: 1,
                label: "Mpeg: Read ID3v1",
            },
            MaskRule {
                field: "read_id3v2",
                mask: 2,
                label: "Mpeg: Read ID3v2",
            },
            MaskRule {
                field: "read_ape",
                mask: 8,
                label: "Mpeg: Read APE",
            },
        ],
    },
    MaskSpec {
        identifier: identifier(b"f4a149060db0964aa32d016776713477"),
        key: MaskKey::MpegWrite,
        group: "mpeg",
        known: 15,
        writable: 11,
        rules: &[
            MaskRule {
                field: "write_id3v1",
                mask: 1,
                label: "Mpeg: Write ID3v1",
            },
            MaskRule {
                field: "write_id3v2",
                mask: 2,
                label: "Mpeg: Write ID3v2",
            },
            MaskRule {
                field: "write_ape",
                mask: 8,
                label: "Mpeg: Write APEv2",
            },
            MaskRule {
                field: "id3v2_only_if_needed",
                mask: 7,
                label: "Mpeg: ID3v2 only if ID3v1 too small",
            },
        ],
    },
    MaskSpec {
        identifier: identifier(b"33172482169bb947865a0d6d2f644446"),
        key: MaskKey::MpegRemove,
        group: "mpeg",
        known: 11,
        writable: 0,
        rules: &[
            MaskRule {
                field: "remove_id3v1",
                mask: 1,
                label: "Mpeg: Remove ID3v1",
            },
            MaskRule {
                field: "remove_id3v2",
                mask: 2,
                label: "Mpeg: Remove ID3v2",
            },
            MaskRule {
                field: "remove_ape",
                mask: 8,
                label: "Mpeg: Remove APE",
            },
        ],
    },
    MaskSpec {
        identifier: identifier(b"ac7785e8c51b644fa377abc80a954b8b"),
        key: MaskKey::ApeRead,
        group: "ape",
        known: 48,
        writable: 0,
        rules: &[
            MaskRule {
                field: "read_id3v1",
                mask: 16,
                label: "Ape, Mpc: Read ID3v1",
            },
            MaskRule {
                field: "read_ape",
                mask: 32,
                label: "Ape, Mpc: Read APE",
            },
        ],
    },
    MaskSpec {
        identifier: identifier(b"1d46a2fc85e7654794a5490bdc3e9eec"),
        key: MaskKey::ApeWrite,
        group: "ape",
        known: 48,
        writable: 48,
        rules: &[
            MaskRule {
                field: "write_id3v1",
                mask: 16,
                label: "Ape, Mpc: Write ID3v1",
            },
            MaskRule {
                field: "write_ape",
                mask: 32,
                label: "Ape, Mpc: Write APEv2",
            },
        ],
    },
    MaskSpec {
        identifier: identifier(b"9e2950e85b1dbd45b4009b2ba739a744"),
        key: MaskKey::ApeRemove,
        group: "ape",
        known: 112,
        writable: 0,
        rules: &[
            MaskRule {
                field: "remove_id3v1",
                mask: 16,
                label: "Ape, Mpc: Remove ID3v1",
            },
            MaskRule {
                field: "remove_id3v2",
                mask: 64,
                label: "Ape, Mpc: Remove ID3v2",
            },
            MaskRule {
                field: "remove_ape",
                mask: 32,
                label: "Ape, Mpc: Remove APE",
            },
        ],
    },
];
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum ChoiceKey {
    Appearance,
    ToolbarScale,
    UiScale,
    Startup,
    CoverSize,
    FreedbComment,
    FreedbDiscId,
    Updates,
}
struct ChoiceSpec {
    identifier: [u8; 16],
    key: ChoiceKey,
    label: &'static str,
    values: &'static [(u32, &'static str)],
}
const CHOICE_SPECS: &[ChoiceSpec] = &[
    ChoiceSpec {
        identifier: identifier(b"345420b668107b40b253c362f84c07ae"),
        key: ChoiceKey::Appearance,
        label: "Color mode",
        values: &[(0, "system"), (1, "dark"), (2, "light")],
    },
    ChoiceSpec {
        identifier: identifier(b"2fc8d3d38c9a594394745f4bf5373c52"),
        key: ChoiceKey::ToolbarScale,
        label: "Toolbar scaling",
        values: &[(0, "100"), (1, "125"), (2, "150"), (3, "175"), (4, "200")],
    },
    ChoiceSpec {
        identifier: identifier(b"35de0f55b39ed3468af928dbf3a478f7"),
        key: ChoiceKey::UiScale,
        label: "User interface scaling",
        values: &[
            (100, "100"),
            (110, "110"),
            (120, "120"),
            (130, "130"),
            (140, "140"),
            (150, "150"),
            (160, "160"),
            (170, "170"),
            (180, "180"),
            (190, "190"),
            (200, "200"),
        ],
    },
    ChoiceSpec {
        identifier: identifier(b"73e3c9e1559ea5449bad26fadc7ea56d"),
        key: ChoiceKey::Startup,
        label: "Startup directory",
        values: &[(0, "empty"), (1, "previous")],
    },
    ChoiceSpec {
        identifier: identifier(b"a29cd34be23c764e89f5976ccaa2da25"),
        key: ChoiceKey::CoverSize,
        label: "MusicBrainz cover size",
        values: &[(1, "original"), (2, "250"), (3, "500"), (4, "1200")],
    },
    ChoiceSpec {
        identifier: identifier(b"a4b6e530061c5642a1adb455bf362784"),
        key: ChoiceKey::FreedbComment,
        label: "Copy disc ID to COMMENT field",
        values: &[(0, "false"), (1, "true")],
    },
    ChoiceSpec {
        identifier: identifier(b"192fa431f5594c4c9c17940c895f2eb9"),
        key: ChoiceKey::FreedbDiscId,
        label: "Copy disc ID to DISCID field",
        values: &[(0, "false"), (1, "true")],
    },
    ChoiceSpec {
        identifier: identifier(b"7b29e93f08c57b4abdec988fa4ab9583"),
        key: ChoiceKey::Updates,
        label: "Update checks on startup",
        values: &[(0, "off"), (1, "stable"), (2, "beta")],
    },
];
const FREEDB_SERVERS: [u8; 16] = identifier(b"1f60b2ade49d454cb0eeeaf43c19359e");
const FREEDB_ACTIVE: [u8; 16] = identifier(b"2345b1c1c149ad4fbaeed1d837e96668");
const MAPPINGS: [u8; 16] = identifier(b"41a00ac0f0ccf14f924a133d5bd6e0e7");
const EXPORT_BOM: [u8; 16] = identifier(b"759de112c0072d48b5650c2fbec586a8");
const TAG_ENCODING: [u8; 16] = identifier(b"025280d33d85d942b0a8699eb8a0b3d2");
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
    masks: BTreeMap<MaskKey, u32>,
    tag_encoding: Option<&'static str>,
    grid: Option<&'static str>,
    export_bom: Option<bool>,
    mappings: Option<Vec<tag_core::UserFieldMapping>>,
    choices: BTreeMap<ChoiceKey, &'static str>,
    freedb: Option<FreedbDataset>,
}
pub struct Import {
    pub patch: Patch,
    pub settings: Vec<Setting>,
    pub warnings: Vec<String>,
}
struct MappingList {
    rules: Vec<tag_core::UserFieldMapping>,
    reserved: bool,
    extension: bool,
}
fn list_u32(bytes: &[u8], position: &mut usize) -> Result<u32, String> {
    let end = position
        .checked_add(4)
        .ok_or("Binary list length overflow")?;
    let value = bytes
        .get(*position..end)
        .ok_or("Binary list is incomplete")?;
    *position = end;
    Ok(u32::from_le_bytes(value.try_into().unwrap()))
}
fn parse_mappings(bytes: &[u8]) -> Result<MappingList, String> {
    let mut position = 0;
    let count = list_u32(bytes, &mut position)? as usize;
    if count > 4096 {
        return Err("A binary field-mapping list exceeds 4096 rows.".into());
    }
    let mut rules = Vec::with_capacity(count);
    let mut reserved = false;
    for _ in 0..count {
        let mut fields: [String; 3] = Default::default();
        // The native list codec always frames five strings, although Mapping
        // load/save uses only the first three. Never decode reserved payloads.
        for output in fields.iter_mut().map(Some).chain([None, None]) {
            let length = list_u32(bytes, &mut position)? as usize;
            if length > TEXT_LIMIT {
                return Err("A binary field-mapping field exceeds 4 MB.".into());
            }
            let end = position
                .checked_add(length)
                .ok_or("Binary list length overflow")?;
            let field = bytes
                .get(position..end)
                .ok_or("Binary list is incomplete")?;
            position = end;
            if let Some(output) = output {
                let value = field.split(|byte| *byte == 0).next().unwrap();
                *output = std::str::from_utf8(value)
                    .map_err(|_| "A binary field-mapping field is not valid UTF-8.")?
                    .to_owned();
            } else {
                reserved |= !field.is_empty();
            }
        }
        let [tag_type, source, target] = fields;
        rules.push(tag_core::UserFieldMapping {
            tag_type,
            source,
            target,
        });
    }
    Ok(MappingList {
        rules,
        reserved,
        extension: position != bytes.len(),
    })
}
#[derive(Clone)]
struct FreedbDataset {
    servers: Vec<serde_json::Value>,
    index: usize,
}
fn parse_freedb_servers(bytes: &[u8]) -> Result<(Vec<[String; 4]>, bool), String> {
    let mut position = 0;
    let count = list_u32(bytes, &mut position)? as usize;
    if count > 4096 {
        return Err("A binary freedb server list exceeds 4096 rows.".into());
    }
    let mut rows = Vec::with_capacity(count);
    for _ in 0..count {
        let mut fields: [String; 4] = Default::default();
        for output in &mut fields {
            let length = list_u32(bytes, &mut position)? as usize;
            if length > TEXT_LIMIT {
                return Err("A binary freedb server field exceeds 4 MB.".into());
            }
            let end = position
                .checked_add(length)
                .ok_or("Binary list length overflow")?;
            let field = bytes
                .get(position..end)
                .ok_or("Binary freedb server list is incomplete")?;
            position = end;
            *output = std::str::from_utf8(field.split(|byte| *byte == 0).next().unwrap())
                .map_err(|_| "A binary freedb server field is not valid UTF-8.")?
                .to_owned();
        }
        rows.push(fields);
    }
    Ok((rows, position != bytes.len()))
}
fn freedb_dataset(rows: Vec<[String; 4]>, index: u32) -> Option<FreedbDataset> {
    if rows.is_empty() || index as usize >= rows.len() {
        return None;
    }
    let mut servers = Vec::with_capacity(rows.len());
    for [name, address, path, port] in rows {
        if [&name, &address, &path, &port]
            .iter()
            .any(|field| field.chars().any(char::is_control))
            || path.trim().is_empty()
            || path.contains('\\')
            || port.is_empty()
            || !port.bytes().all(|b| b.is_ascii_digit())
        {
            return None;
        }
        let number = port.parse::<u16>().ok().filter(|port| *port != 0)?;
        let settings = crate::sources::freedb::FreedbSettings {
            protocol: "http".into(),
            address: address.clone(),
            path: path.clone(),
            port: Some(number),
            ..Default::default()
        };
        // Validate the same URL shape as the provider without issuing a request.
        crate::sources::freedb::command_url(&settings, "cddb sites").ok()?;
        servers
            .push(json!({"name":name,"protocol":"http","address":address,"path":path,"port":port}));
    }
    Some(FreedbDataset {
        servers,
        index: index as usize,
    })
}
fn portable_cover_directory(value: &str) -> bool {
    // The chooser trims evaluated output; classify the same leading text but
    // preserve the accepted authored expression without rewriting it.
    let candidate = value.trim();
    let scheme = candidate.split_once(':').is_some_and(|(name, _)| {
        let mut bytes = name.bytes();
        bytes.next().is_some_and(|byte| byte.is_ascii_alphabetic())
            && bytes.all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'+' | b'-' | b'.'))
    });
    !scheme
        && !candidate.starts_with('/')
        && !candidate.contains('\\')
        && tag_core::CompiledTemplate::compile(value).is_ok()
}
pub fn parse(bytes: &[u8]) -> Result<Import, String> {
    let mut position = 0usize;
    let mut values = BTreeMap::new();
    let mut extended_mode = None;
    let mut grid_raw = None;
    let mut export_bom = None;
    let mut mappings_raw = None;
    let mut choice_raw = BTreeMap::new();
    let mut freedb_rows = None;
    let mut freedb_index = None;
    let mut text = BTreeMap::new();
    let mut masks = BTreeMap::new();
    let mut tag_encoding_raw = None;
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
        } else if let Some(spec) = CHOICE_SPECS
            .iter()
            .find(|spec| header[..16] == spec.identifier)
        {
            let raw = u32::from_le_bytes(
                payload
                    .get(..4)
                    .ok_or("Binary choice setting is incomplete")?
                    .try_into()
                    .unwrap(),
            );
            choice_raw.insert(spec.key, raw);
        } else if header[..16] == FREEDB_SERVERS {
            freedb_rows = Some(parse_freedb_servers(payload)?);
        } else if header[..16] == FREEDB_ACTIVE {
            let mut position = 0;
            freedb_index = Some(list_u32(payload, &mut position)?);
        } else if header[..16] == MAPPINGS {
            mappings_raw = Some(parse_mappings(payload)?);
        } else if header[..16] == EXPORT_BOM {
            export_bom = Some(
                u32::from_le_bytes(
                    payload
                        .get(..4)
                        .ok_or("Binary export BOM setting is incomplete")?
                        .try_into()
                        .unwrap(),
                ) != 0,
            );
        } else if header[..16] == GRID {
            grid_raw = Some(u32::from_le_bytes(
                payload
                    .get(..4)
                    .ok_or("Binary grid setting is incomplete")?
                    .try_into()
                    .unwrap(),
            ));
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
        } else if let Some(spec) = MASK_SPECS
            .iter()
            .find(|spec| header[..16] == spec.identifier)
        {
            let raw = u32::from_le_bytes(
                payload
                    .get(..4)
                    .ok_or("Binary tag settings are incomplete")?
                    .try_into()
                    .unwrap(),
            );
            masks.insert(spec.key, raw);
        } else if header[..16] == TAG_ENCODING {
            tag_encoding_raw = Some(u32::from_le_bytes(
                payload
                    .get(..4)
                    .ok_or("Binary tag settings are incomplete")?
                    .try_into()
                    .unwrap(),
            ));
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
    let has_freedb = freedb_rows.is_some() || freedb_index.is_some();
    let freedb = match (freedb_rows, freedb_index) {
        (Some((rows, extension)), Some(index)) => {
            if extension {
                warnings
                    .push("Trailing freedb server-list extension bytes are not imported.".into());
            }
            freedb_dataset(rows, index)
        }
        _ => None,
    };
    if has_freedb && freedb.is_none() {
        warnings.push("The freedb server list and active index do not form a supported complete dataset; the entire current server list and selection are kept.".into());
    }
    let choices: BTreeMap<_,_> = choice_raw.into_iter().filter_map(|(key,raw)| {
        let spec = CHOICE_SPECS.iter().find(|spec|spec.key==key).unwrap();
        match spec.values.iter().find(|(source,_)|*source==raw) {
            Some((_,value)) => Some((key,*value)),
            None if key==ChoiceKey::UiScale => {
                warnings.push("The original normalizes this unsupported UI scale to 100%; that normalized choice is imported.".into());
                Some((key,"100"))
            }
            None => {
                warnings.push("An unsupported native option choice is not imported; its current choice is kept.".into());
                None
            }
        }
    }).collect();
    if choices.get(&ChoiceKey::Startup) == Some(&"previous") {
        warnings.push("Previous startup directory uses TagGroove's current folder history; the original computer's history is not imported.".into());
    }
    let restriction = values.get(&OptionKey::LibraryRestrict).copied();
    if restriction == Some(true) {
        values.remove(&OptionKey::LibraryRestrict);
        warnings.push("The restricted source library's host folder list is not imported; the current restriction is kept.".into());
    }
    if values.get(&OptionKey::LibraryEnabled) == Some(&true) && restriction != Some(false) {
        values.remove(&OptionKey::LibraryEnabled);
        warnings.push("Library enabling without an explicit unrestricted source policy is not imported; current enablement is kept until source folders can be bound.".into());
    }
    let mappings = mappings_raw.and_then(|list| {
        if tag_core::validate_user_mappings(&list.rules).is_err() {
            warnings.push("The incoming field-mapping list is unsupported or conflicting; the entire current list is kept.".into());
            return None;
        }
        if list.reserved { warnings.push("Nonempty reserved mapping fields are not imported; only the supported type, source and target fields transfer.".into()); }
        if list.extension { warnings.push("Trailing field-mapping extension bytes are not imported.".into()); }
        Some(list.rules)
    });
    for field in ["track_digits", "disc_digits"] {
        let key = OptionKey::NumberingPadding(field);
        if values.get(&key) == Some(&true) {
            values.remove(&key);
            warnings.push(if field == "track_digits" {
                "Dynamic track leading-zero padding is not imported; the current minimum digits are kept."
            } else {
                "Dynamic disc leading-zero padding is not imported; the current minimum digits are kept."
            }.into());
        }
    }
    let grid = match grid_raw {
        Some(0) => Some("off"),
        Some(1) => Some("alternate"),
        Some(2) => Some("lines"),
        Some(_) => {
            warnings.push(
                "The unrecognized grid choice is not imported; the current choice is kept.".into(),
            );
            None
        }
        None => None,
    };
    text.retain(|key,value| {
        let reason = match key {
            TextKey::CoverDirectory if !portable_cover_directory(value) => Some("The host-specific or invalid cover-art directory is not imported; the current folder preference is kept."),
            TextKey::CommentLanguage if value.len()!=3 || !value.bytes().all(|byte|byte.is_ascii_alphabetic()) => Some("The invalid comment language is not imported; the current language is kept."),
            TextKey::RestrictExtensions if crate::settings::parse_extensions(value).is_err() => Some("The invalid incoming-file restriction is not imported; the current restriction is kept."),
            TextKey::FolderSeparator if tag_core::validate_folder_separator(value).is_err() => Some("The invalid folder-separator replacement is not imported; the current replacement is kept."),
            _ => None,
        };
        if let Some(reason)=reason {warnings.push(reason.into());false} else {true}
    });
    masks.retain(|key, raw| {
        let spec = MASK_SPECS.iter().find(|spec| spec.key == *key).unwrap();
        if spec.writable != 0 && *raw & spec.writable == 0 {
            warnings.push("A tag-write setting with no supported write type is not imported; the current write types are kept.".into());
            return false;
        }
        if *raw & !spec.known != 0 {
            warnings.push("Unmapped bits in a tag setting are not imported; only the listed switches change.".into());
        }
        true
    });
    let tag_encoding = match tag_encoding_raw {
        Some(0) => Some("v23_latin1"),
        Some(1) => Some("v23_utf16"),
        Some(3) => Some("v24_utf8"),
        Some(_) => {
            warnings.push("The unrecognized ID3v2 version/encoding choice is not imported; the current choice is kept.".into());
            None
        }
        None => None,
    };
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
    if let Some(expression) = text.get_mut(&TextKey::ExportPartition) {
        *expression = expression.trim().to_owned();
        if expression.is_empty() {
            *expression = "%_folderpath%".into();
        }
        if tag_core::CompiledTemplate::compile(expression).is_err() {
            text.remove(&TextKey::ExportPartition);
            values.remove(&OptionKey::ExportMultiple);
            warnings.push("The invalid export partition format and its enabled state are not imported; current partition settings are kept.".into());
        }
    }
    if let Some(filename) = text.get(&TextKey::ExportFilename) {
        let bytes = filename.as_bytes();
        let windows_drive =
            bytes.first().is_some_and(u8::is_ascii_alphabetic) && bytes.get(1) == Some(&b':');
        let reason = if !cfg!(windows) && (windows_drive || filename.contains('\\')) {
            Some("The Windows-specific export filename is not imported on this platform; the current filename is kept.")
        } else if tag_core::CompiledTemplate::compile(filename).is_err() {
            Some("The invalid export filename is not imported; the current filename is kept.")
        } else {
            None
        };
        if let Some(reason) = reason {
            text.remove(&TextKey::ExportFilename);
            warnings.push(reason.into());
        }
    }
    if text.contains_key(&TextKey::ExportConfiguration)
        || text.contains_key(&TextKey::ExportFilename)
    {
        warnings.push("The remembered export configuration and filename require its matching source template to be selected. Unresolved bindings keep the current configuration and filename.".into());
    }
    text.retain(|key,value| {
        let reason = match key {
            TextKey::Language if !["", "English", "Dutch", "German"].contains(&value.as_str()) => Some("The source language catalog is not available as a confirmed target catalog; the current language is kept."),
            TextKey::GeneratorPartition if tag_core::CompiledTemplate::compile(value).is_err() => Some("The invalid playlist-generator partition expression is not imported; the current expression is kept."),
            TextKey::GeneratorFilename | TextKey::SourceCoverFilename if !portable_cover_directory(value) => Some("A host-specific or invalid playlist-generator or cover filename expression is not imported; the current expression is kept."),
            _ => None,
        };
        if let Some(reason)=reason { warnings.push(reason.into()); false } else { true }
    });
    if text
        .get(&TextKey::GeneratorFilename)
        .is_some_and(|value| value.trim().is_empty())
    {
        if let Some(filename) = text
            .get(&TextKey::PlaylistFilename)
            .filter(|value| portable_cover_directory(value))
            .cloned()
        {
            text.insert(TextKey::GeneratorFilename, filename);
        } else {
            text.remove(&TextKey::GeneratorFilename);
            warnings.push("The empty generator filename has no admitted source playlist filename to fall back to; the current generator filename is kept.".into());
        }
    }
    for (field, text_key) in NUMBERING_FIELDS {
        let enabled_key = OptionKey::NumberingFieldEnabled(field);
        match values.get(&enabled_key) {
            Some(false) => {
                text.remove(&text_key);
            }
            Some(true) => {
                if !text.get(&text_key).is_some_and(|name| {
                    !name.trim().is_empty()
                        && !name.starts_with('_')
                        && !name.chars().any(char::is_control)
                }) {
                    values.remove(&enabled_key);
                    text.remove(&text_key);
                    warnings.push("An enabled numbering field choice lacks a usable explicit tag-field name; special path fields are not substituted. The current choice is kept.".into());
                }
            }
            None => {
                if text.remove(&text_key).is_some() {
                    warnings.push("A numbering field name without its enabled switch is not imported; the current choice is kept.".into());
                }
            }
        }
    }
    if values.is_empty()
        && extended_mode.is_none()
        && text.is_empty()
        && masks.is_empty()
        && tag_encoding.is_none()
        && grid.is_none()
        && export_bom.is_none()
        && mappings.is_none()
        && choices.is_empty()
        && freedb.is_none()
    {
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
    if let Some(dataset) = &freedb {
        settings.push(Setting {
            label: "freedb: replace current server list".into(),
            enabled: None,
            value: Some(dataset.servers.len().to_string()),
            verbatim: false,
        });
        for (index, row) in dataset.servers.iter().enumerate() {
            let raw_path = row["path"].as_str().unwrap().trim();
            let path = if raw_path.starts_with('/') {
                raw_path.to_owned()
            } else {
                format!("/{raw_path}")
            };
            settings.push(Setting {
                label: if index == dataset.index {
                    "freedb: selected server"
                } else {
                    "freedb: server"
                }
                .into(),
                enabled: None,
                value: Some(format!(
                    "{} — http://{}:{}{}",
                    row["name"].as_str().unwrap(),
                    row["address"].as_str().unwrap().trim(),
                    row["port"].as_str().unwrap(),
                    path
                )),
                verbatim: true,
            });
        }
    }
    for (key, value) in &choices {
        let spec = CHOICE_SPECS.iter().find(|spec| spec.key == *key).unwrap();
        let flag = matches!(key, ChoiceKey::FreedbComment | ChoiceKey::FreedbDiscId);
        let display = match (*key, *value) {
            (ChoiceKey::Appearance, "dark") => "Dark",
            (ChoiceKey::Appearance, "light") => "Light",
            (ChoiceKey::Appearance, "system") => "System",
            (ChoiceKey::Startup, "empty") => "Empty",
            (ChoiceKey::Startup, "previous") => "Previous directory",
            (ChoiceKey::CoverSize, "original") => "Original size",
            (ChoiceKey::Updates, "off") => "Off",
            (ChoiceKey::Updates, "stable") => "Stable releases",
            (ChoiceKey::Updates, "beta") => "Stable and beta releases",
            _ => value,
        };
        settings.push(Setting {
            label: spec.label.into(),
            enabled: flag.then_some(*value == "true"),
            value: (!flag).then(|| display.to_owned()),
            verbatim: display.bytes().all(|b| b.is_ascii_digit()),
        });
    }
    if let Some(rules) = &mappings {
        settings.push(Setting {
            label: "Tag field mappings: replace current list".into(),
            enabled: None,
            value: Some(rules.len().to_string()),
            verbatim: false,
        });
        settings.extend(rules.iter().map(|rule| Setting {
            label: "Tag field mapping".into(),
            enabled: None,
            value: Some(format!(
                "{}: {} → {}",
                rule.tag_type, rule.source, rule.target
            )),
            verbatim: true,
        }));
    }
    if let Some(enabled) = export_bom {
        settings.push(Setting {
            label: "Write export byte order mark (BOM)".into(),
            enabled: Some(enabled),
            value: None,
            verbatim: false,
        });
    }
    for (key, raw) in &masks {
        let spec = MASK_SPECS.iter().find(|spec| spec.key == *key).unwrap();
        settings.extend(spec.rules.iter().map(|rule| Setting {
            label: rule.label.into(),
            enabled: Some(*raw & rule.mask == rule.mask),
            value: None,
            verbatim: false,
        }));
    }
    if let Some(encoding) = tag_encoding {
        settings.push(Setting {
            label: "ID3v2 version".into(),
            enabled: None,
            verbatim: false,
            value: Some(
                match encoding {
                    "v23_latin1" => "ID3v2.3 ISO-8859-1",
                    "v23_utf16" => "ID3v2.3 UTF-16",
                    _ => "ID3v2.4 UTF-8",
                }
                .into(),
            ),
        });
    }
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
    if let Some(value) = grid {
        settings.push(Setting {
            label: "Display grid lines in file view".into(),
            enabled: None,
            value: Some(
                match value {
                    "off" => "Off",
                    "alternate" => "Alternate row colors",
                    _ => "Grid lines",
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
            masks,
            tag_encoding,
            grid,
            export_bom,
            mappings,
            choices,
            freedb,
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
    /// Overlay export choices on the library whose imported template identities
    /// have already been resolved. Return diagnostics for unresolved bindings.
    pub fn apply_exports(
        &self,
        library: &mut crate::exports::Library,
        admitted: &BTreeMap<String, String>,
    ) -> Vec<String> {
        let preferences = &mut library.preferences;
        if let Some(value) = self.booleans.get(&OptionKey::ExportAppend) {
            preferences.append = *value;
        }
        if self.text.contains_key(&TextKey::ExportPartition)
            || self.booleans.contains_key(&OptionKey::ExportMultiple)
        {
            let expression = self
                .text
                .get(&TextKey::ExportPartition)
                .cloned()
                .or_else(|| preferences.partition.clone())
                .or_else(|| preferences.partition_memory.clone())
                .unwrap_or_else(|| "%_folderpath%".into());
            let enabled = self
                .booleans
                .get(&OptionKey::ExportMultiple)
                .copied()
                .unwrap_or(preferences.partition.is_some());
            preferences.partition_memory = Some(expression.clone());
            preferences.partition = enabled.then_some(expression);
        }
        if let Some(value) = self
            .booleans
            .get(&OptionKey::Page("export", "dont_create_directories"))
        {
            preferences.create_directories = !value;
        }
        if let Some(value) = self.export_bom {
            preferences.write_bom = value;
        }
        if self.text.contains_key(&TextKey::ExportConfiguration)
            || self.text.contains_key(&TextKey::ExportFilename)
        {
            if let Some(id) = self
                .text
                .get(&TextKey::ExportConfiguration)
                .and_then(|name| admitted.get(name))
            {
                preferences.configuration = Some(id.clone());
                if let Some(value) = self.text.get(&TextKey::ExportFilename) {
                    preferences.filename = value.clone();
                }
            } else {
                return vec!["The remembered export configuration could not be bound to a selected export template; the current configuration and filename are kept.".into()];
            }
        }
        Vec::new()
    }
    /// Work only in the caller's unpublished configuration stage. Browser
    /// preferences are returned for the same durable window/restore receipt.
    pub fn apply(
        &self,
        directory: &Path,
        current: &BTreeMap<String, String>,
    ) -> Result<BTreeMap<String, String>, String> {
        let mut options = None;
        let mut tag_panel = None;
        let mut preferences = BTreeMap::new();
        let mut numbering = if self.booleans.keys().any(|key| {
            matches!(
                key,
                OptionKey::Numbering(_)
                    | OptionKey::NumberingFieldEnabled(_)
                    | OptionKey::NumberingPadding(_)
            )
        }) {
            let saved = current
                .get("opentag.numbering")
                .cloned()
                .unwrap_or_else(|| "{}".into());
            crate::configuration_transfer::validate_preferences(&BTreeMap::from([("opentag.numbering".into(), saved.clone())]))
                .map_err(|_| "The current numbering preferences are invalid; review Auto-numbering before importing its settings.")?;
            Some(
                serde_json::from_str::<serde_json::Value>(&saved)
                    .map_err(|error| error.to_string())?,
            )
        } else {
            None
        };
        for (key, value) in &self.booleans {
            match key {
                OptionKey::Page(id, field) => {
                    page(settings(&mut options, directory)?, id, field, *value)?
                }
                OptionKey::LibraryEnabled => page(
                    settings(&mut options, directory)?,
                    "library",
                    "enabled",
                    *value,
                )?,
                OptionKey::LibraryRestrict => page(
                    settings(&mut options, directory)?,
                    "library",
                    "restrict",
                    *value,
                )?,
                OptionKey::CoverAnswer => page(
                    settings(&mut options, directory)?,
                    "messages",
                    "import_covers_choice",
                    if *value { "preserve" } else { "replace" },
                )?,
                // Applied only to the final in-memory export library after
                // template collision decisions, never via a stale disk snapshot.
                OptionKey::ExportAppend | OptionKey::ExportMultiple => {}
                OptionKey::NumberingPadding(field) => {
                    numbering.as_mut().unwrap()[*field] = 1.into();
                }
                OptionKey::Numbering(field) => {
                    numbering.as_mut().unwrap()[*field] = (*value).into();
                }
                OptionKey::NumberingFieldEnabled(field) => {
                    let text_key = NUMBERING_FIELDS
                        .iter()
                        .find(|(name, _)| name == field)
                        .unwrap()
                        .1;
                    let name = if *value {
                        self.text[&text_key].clone()
                    } else {
                        String::new()
                    };
                    numbering.as_mut().unwrap()[*field] = name.into();
                }
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
                OptionKey::Message(field) => page(
                    settings(&mut options, directory)?,
                    "messages",
                    field,
                    *value,
                )?,
                OptionKey::Advanced(field) => {
                    let options = settings(&mut options, directory)?;
                    let mut advanced = serde_json::to_value(&options.tags.advanced)
                        .map_err(|error| error.to_string())?;
                    advanced[field] = (*value).into();
                    options.tags.advanced =
                        serde_json::from_value(advanced).map_err(|error| error.to_string())?;
                }
                OptionKey::PreferGenres => {
                    panel(&mut tag_panel, directory)?
                        .field_values
                        .prefer_tag_fields
                        .insert("GENRE".into(), *value);
                }
                OptionKey::NaturalSort => {
                    settings(&mut options, directory)?.general.natural_sort = *value;
                }
                OptionKey::SelectAdded => page(
                    settings(&mut options, directory)?,
                    "general",
                    "select_added",
                    *value,
                )?,
                OptionKey::KeepMoved => page(
                    settings(&mut options, directory)?,
                    "general",
                    "keep_moved",
                    *value,
                )?,
            }
        }
        if let Some(enabled) = self.export_bom {
            page(
                settings(&mut options, directory)?,
                "export",
                "write_bom",
                enabled,
            )?;
        }
        if let Some(mode) = self.extended_mode {
            page(
                settings(&mut options, directory)?,
                "tags",
                "extended_save_mode",
                mode,
            )?;
        }
        if let Some(grid) = self.grid {
            page(
                settings(&mut options, directory)?,
                "appearance",
                "grid",
                grid,
            )?;
        }
        for (key, value) in &self.choices {
            match key {
                ChoiceKey::Appearance => {
                    preferences.insert("opentag.appearance".into(), (*value).into());
                }
                ChoiceKey::ToolbarScale => page(
                    settings(&mut options, directory)?,
                    "appearance",
                    "toolbar_scale",
                    *value,
                )?,
                ChoiceKey::UiScale => page(
                    settings(&mut options, directory)?,
                    "appearance",
                    "ui_scale",
                    *value,
                )?,
                ChoiceKey::Startup => page(
                    settings(&mut options, directory)?,
                    "directories-startup",
                    "startup",
                    *value,
                )?,
                ChoiceKey::CoverSize => page(
                    settings(&mut options, directory)?,
                    "tag-sources",
                    "musicbrainz_cover_size",
                    *value,
                )?,
                ChoiceKey::FreedbComment => page(
                    settings(&mut options, directory)?,
                    "tag-sources-freedb",
                    "comment_disc_id",
                    *value == "true",
                )?,
                ChoiceKey::FreedbDiscId => page(
                    settings(&mut options, directory)?,
                    "tag-sources-freedb",
                    "disc_id_field",
                    *value == "true",
                )?,
                ChoiceKey::Updates => {
                    let options = settings(&mut options, directory)?;
                    page(options, "updates", "check_on_startup", *value != "off")?;
                    if *value != "off" {
                        page(options, "updates", "include_beta", *value == "beta")?;
                    }
                }
            }
        }
        for (key, value) in &self.text {
            match key {
                TextKey::ExportPartition
                | TextKey::ExportConfiguration
                | TextKey::ExportFilename
                | TextKey::NumberingNextField
                | TextKey::NumberingResetField => {}
                TextKey::Language => {
                    let locale = match value.as_str() {
                        "English" => "en",
                        "Dutch" => "nl",
                        "German" => "de",
                        _ => "system",
                    };
                    page(
                        settings(&mut options, directory)?,
                        "language",
                        "language",
                        locale,
                    )?;
                    preferences.insert("opentag.language".into(), locale.into());
                }
                TextKey::SourceCoverFilename => page(
                    settings(&mut options, directory)?,
                    "tag-sources",
                    "cover_filename",
                    value.as_str(),
                )?,
                TextKey::GeneratorFilename => page(
                    settings(&mut options, directory)?,
                    "playlist",
                    "generator_filename",
                    value.as_str(),
                )?,
                TextKey::GeneratorPartition => page(
                    settings(&mut options, directory)?,
                    "playlist",
                    "generator_partition",
                    value.as_str(),
                )?,
                TextKey::CoverDirectory => page(
                    settings(&mut options, directory)?,
                    "directories",
                    "cover_art",
                    value.as_str(),
                )?,
                TextKey::PlaylistFilename => page(
                    settings(&mut options, directory)?,
                    "playlist",
                    "filename",
                    value.as_str(),
                )?,
                TextKey::PlaylistInfo => page(
                    settings(&mut options, directory)?,
                    "playlist",
                    "extinf_format",
                    value.as_str(),
                )?,
                TextKey::CommentLanguage => {
                    settings(&mut options, directory)?
                        .tags
                        .advanced
                        .id3v2_comment_language = value.clone()
                }
                TextKey::RestrictExtensions => {
                    settings(&mut options, directory)?.files.restrict_extensions = value.clone()
                }
                TextKey::FolderSeparator => {
                    settings(&mut options, directory)?.files.folder_separator = value.clone()
                }
            }
        }
        if let Some(dataset) = &self.freedb {
            let options = settings(&mut options, directory)?;
            page(
                options,
                "tag-sources-freedb",
                "servers",
                json!(dataset.servers),
            )?;
            page(options, "tag-sources-freedb", "server", dataset.index)?;
            let selected = &dataset.servers[dataset.index];
            for key in ["protocol", "address", "path", "port"] {
                page(options, "tag-sources-freedb", key, selected[key].clone())?;
            }
        }
        if let Some(rules) = &self.mappings {
            settings(&mut options, directory)?.tags.mappings = rules.clone();
        }
        if !self.masks.is_empty() || self.tag_encoding.is_some() {
            let options = settings(&mut options, directory)?;
            let mut tags =
                serde_json::to_value(&options.tags).map_err(|error| error.to_string())?;
            for (key, raw) in &self.masks {
                let spec = MASK_SPECS.iter().find(|spec| spec.key == *key).unwrap();
                for rule in spec.rules {
                    tags[spec.group][rule.field] = (*raw & rule.mask == rule.mask).into();
                }
            }
            if let Some(encoding) = self.tag_encoding {
                tags["mpeg"]["id3v2_format"] = encoding.into();
            }
            options.tags = serde_json::from_value(tags).map_err(|error| error.to_string())?;
        }
        if let Some(numbering) = numbering {
            preferences.insert("opentag.numbering".into(), numbering.to_string());
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
    fn native_options_apply(
        records: &[(&str, &[u8])],
        before: &crate::settings::Settings,
    ) -> (crate::settings::Settings, BTreeMap<String, String>, Import) {
        let bytes: Vec<_> = records
            .iter()
            .flat_map(|(id, payload)| record(&native_id(id), payload))
            .collect();
        let parsed = parse(&bytes).unwrap();
        let dir = tempfile::tempdir().unwrap();
        crate::settings::save(&dir.path().join("settings.json"), before).unwrap();
        let preferences = parsed.patch.apply(dir.path(), &BTreeMap::new()).unwrap();
        (
            crate::settings::load(&dir.path().join("settings.json")).0,
            preferences,
            parsed,
        )
    }
    #[test]
    fn native_options_dword_choices_keep_distinct_codecs_and_last_record_semantics() {
        let cases = [
            (
                "345420b668107b40b253c362f84c07ae",
                1,
                "window",
                "opentag.appearance",
                "dark",
            ),
            (
                "345420b668107b40b253c362f84c07ae",
                2,
                "window",
                "opentag.appearance",
                "light",
            ),
            (
                "345420b668107b40b253c362f84c07ae",
                0,
                "window",
                "opentag.appearance",
                "system",
            ),
            (
                "2fc8d3d38c9a594394745f4bf5373c52",
                0,
                "appearance",
                "toolbar_scale",
                "100",
            ),
            (
                "2fc8d3d38c9a594394745f4bf5373c52",
                1,
                "appearance",
                "toolbar_scale",
                "125",
            ),
            (
                "2fc8d3d38c9a594394745f4bf5373c52",
                2,
                "appearance",
                "toolbar_scale",
                "150",
            ),
            (
                "2fc8d3d38c9a594394745f4bf5373c52",
                3,
                "appearance",
                "toolbar_scale",
                "175",
            ),
            (
                "2fc8d3d38c9a594394745f4bf5373c52",
                4,
                "appearance",
                "toolbar_scale",
                "200",
            ),
            (
                "35de0f55b39ed3468af928dbf3a478f7",
                120,
                "appearance",
                "ui_scale",
                "120",
            ),
            (
                "35de0f55b39ed3468af928dbf3a478f7",
                190,
                "appearance",
                "ui_scale",
                "190",
            ),
            (
                "35de0f55b39ed3468af928dbf3a478f7",
                125,
                "appearance",
                "ui_scale",
                "100",
            ),
            (
                "73e3c9e1559ea5449bad26fadc7ea56d",
                0,
                "directories-startup",
                "startup",
                "empty",
            ),
            (
                "73e3c9e1559ea5449bad26fadc7ea56d",
                1,
                "directories-startup",
                "startup",
                "previous",
            ),
            (
                "a29cd34be23c764e89f5976ccaa2da25",
                1,
                "tag-sources",
                "musicbrainz_cover_size",
                "original",
            ),
            (
                "a29cd34be23c764e89f5976ccaa2da25",
                2,
                "tag-sources",
                "musicbrainz_cover_size",
                "250",
            ),
            (
                "a29cd34be23c764e89f5976ccaa2da25",
                3,
                "tag-sources",
                "musicbrainz_cover_size",
                "500",
            ),
            (
                "a29cd34be23c764e89f5976ccaa2da25",
                4,
                "tag-sources",
                "musicbrainz_cover_size",
                "1200",
            ),
        ];
        for (id, raw, page_id, field, expected) in cases {
            let (after, prefs, _) = native_options_apply(
                &[(id, &u32::to_le_bytes(raw))],
                &crate::settings::Settings::default(),
            );
            if page_id == "window" {
                assert_eq!(prefs[field], expected);
            } else {
                assert_eq!(after.pages[page_id][field], expected, "{id}:{raw}");
            }
        }
        let mut before = crate::settings::Settings::default();
        page(&mut before, "tag-sources-freedb", "comment_disc_id", true).unwrap();
        for raw in [0u32, 1, 256] {
            let (after, _, imported) = native_options_apply(
                &[
                    ("a4b6e530061c5642a1adb455bf362784", &1u32.to_le_bytes()),
                    ("a4b6e530061c5642a1adb455bf362784", &raw.to_le_bytes()),
                    ("192fa431f5594c4c9c17940c895f2eb9", &0u32.to_le_bytes()),
                ],
                &before,
            );
            assert_eq!(
                after.pages["tag-sources-freedb"]["comment_disc_id"],
                raw != 0
            );
            assert_eq!(after.pages["tag-sources-freedb"]["disc_id_field"], false);
            assert_eq!(!imported.warnings.is_empty(), raw == 256);
        }
    }
    #[test]
    fn native_options_updates_preserve_product_history_and_beta_when_disabled() {
        let mut before = crate::settings::Settings::default();
        page(&mut before, "updates", "last_version", "9.8.7").unwrap();
        page(&mut before, "updates", "include_beta", true).unwrap();
        for raw in [0u32, 1, 2, 256] {
            let (after, _, imported) = native_options_apply(
                &[
                    ("7b29e93f08c57b4abdec988fa4ab9583", &raw.to_le_bytes()),
                    ("7c7d406cdfd18b48a914db716854fc2f", &[0]),
                ],
                &before,
            );
            assert_eq!(after.pages["updates"]["last_version"], "9.8.7");
            assert_eq!(after.pages["updates"]["release_notes"], false);
            assert_eq!(after.pages["updates"]["include_beta"], raw != 1);
            if raw <= 2 {
                assert_eq!(after.pages["updates"]["check_on_startup"], raw != 0);
            } else {
                assert!(after.pages["updates"].get("check_on_startup").is_none());
                assert!(!imported.warnings.is_empty());
            }
        }
    }
    #[test]
    fn native_options_library_never_enables_an_unbound_restricted_scope() {
        let mut before = crate::settings::Settings::default();
        page(&mut before, "library", "enabled", true).unwrap();
        page(&mut before, "library", "restrict", true).unwrap();
        page(
            &mut before,
            "library",
            "directories",
            json!(["/current-only"]),
        )
        .unwrap();
        for (enabled, restrict, expected_enabled, expected_restrict) in [
            (1, Some(1), true, true),
            (0, Some(1), false, true),
            (1, Some(0), true, false),
            (0, Some(0), false, false),
            (1, None, true, true),
        ] {
            let restriction = restrict.map(|raw| [raw]);
            let mut records = vec![
                ("7bbbf22d6046a94d849c82eb792d0910", vec![enabled]),
                ("eada7c7f01da5f4a849392b72b37ef08", vec![0]),
            ];
            if let Some(raw) = restriction {
                records.push(("65378755075c9046b6d4ac4cd6346af9", raw.to_vec()));
            }
            let refs: Vec<_> = records
                .iter()
                .map(|(id, raw)| (*id, raw.as_slice()))
                .collect();
            let (after, _, imported) = native_options_apply(&refs, &before);
            assert_eq!(after.pages["library"]["enabled"], expected_enabled);
            assert_eq!(after.pages["library"]["restrict"], expected_restrict);
            assert_eq!(
                after.pages["library"]["directories"],
                json!(["/current-only"])
            );
            if restrict != Some(0) {
                assert!(!imported.warnings.is_empty());
            }
        }
        let mut disabled = before;
        page(&mut disabled, "library", "enabled", false).unwrap();
        let (after, _, _) = native_options_apply(
            &[
                ("7bbbf22d6046a94d849c82eb792d0910", &[1]),
                ("65378755075c9046b6d4ac4cd6346af9", &[1]),
                ("eada7c7f01da5f4a849392b72b37ef08", &[1]),
            ],
            &disabled,
        );
        assert_eq!(after.pages["library"]["enabled"], false);
    }
    #[test]
    fn native_options_independent_bom_directory_and_cover_answer_flags() {
        for raw in [0, 1, 255] {
            let (after, _, _) = native_options_apply(
                &[
                    ("eada7c7f01da5f4a849392b72b37ef08", &[raw]),
                    ("e60558750d35d148b3f22c2c229005dd", &[raw]),
                    ("f0bb1392e33e1041801e42ce3b64f1b7", &[raw]),
                ],
                &crate::settings::Settings::default(),
            );
            assert_eq!(after.pages["playlist"]["write_bom"], raw != 0);
            assert!(after.pages["playlist"]
                .get("dont_create_directories")
                .is_none());
            assert_eq!(after.pages["export"]["dont_create_directories"], raw != 0);
            assert_eq!(
                after.pages["messages"]["import_covers_choice"],
                if raw == 0 { "replace" } else { "preserve" }
            );
            assert!(after.pages["messages"]
                .get("confirm_import_covers")
                .is_none());
        }
    }
    #[test]
    fn native_options_unknown_final_enums_do_not_revive_earlier_choices() {
        for (id, raw) in [
            ("345420b668107b40b253c362f84c07ae", 1u32),
            ("2fc8d3d38c9a594394745f4bf5373c52", 2),
            ("73e3c9e1559ea5449bad26fadc7ea56d", 1),
            ("a29cd34be23c764e89f5976ccaa2da25", 3),
        ] {
            let before = crate::settings::Settings::default();
            let (after, prefs, parsed) = native_options_apply(
                &[
                    (id, &raw.to_le_bytes()),
                    (id, &256u32.to_le_bytes()),
                    ("eada7c7f01da5f4a849392b72b37ef08", &[1]),
                ],
                &before,
            );
            assert_eq!(after.pages.len(), 1);
            assert!(prefs.is_empty());
            assert!(!parsed.warnings.is_empty());
            for length in 0..4 {
                assert!(parse(&record(&native_id(id), &raw.to_le_bytes()[..length])).is_err());
            }
        }
    }
    #[test]
    fn native_options_language_and_generator_bind_to_explicit_source_values() {
        for (stem, language) in [
            ("English", "en"),
            ("Dutch", "nl"),
            ("German", "de"),
            ("", "system"),
        ] {
            let (after, prefs, _) = native_options_apply(
                &[
                    ("1a0bfd2a930e704dac9262461ddcd9c6", stem.as_bytes()),
                    ("312fa18b404068489ebd20ea4183cc24", b""),
                    ("7eaf6b64ce85a54196043a3c8cfb1d2e", b"%album%.m3u8"),
                    ("0e3b78b6452b224789abc77122e2d677", b""),
                    ("f64318074ac91b4db1b909e2ce5a695c", b"%album% cover"),
                ],
                &crate::settings::Settings::default(),
            );
            assert_eq!(prefs["opentag.language"], language);
            assert_eq!(after.pages["language"]["language"], language);
            assert_eq!(
                after.pages["playlist"]["generator_filename"],
                "%album%.m3u8"
            );
            assert_eq!(after.pages["playlist"]["generator_partition"], "");
            assert_eq!(
                after.pages["tag-sources"]["cover_filename"],
                "%album% cover"
            );
        }
        let mut before = crate::settings::Settings::default();
        page(
            &mut before,
            "playlist",
            "generator_filename",
            "current.m3u8",
        )
        .unwrap();
        page(&mut before, "tag-sources", "cover_filename", "old").unwrap();
        for bad in [" /foreign/folder", "file:/outside", "C:/Music", "$if("] {
            let (after, prefs, imported) = native_options_apply(
                &[
                    ("1a0bfd2a930e704dac9262461ddcd9c6", b"Dutch.lng"),
                    ("312fa18b404068489ebd20ea4183cc24", b""),
                    ("f64318074ac91b4db1b909e2ce5a695c", bad.as_bytes()),
                    ("eada7c7f01da5f4a849392b72b37ef08", &[1]),
                ],
                &before,
            );
            assert!(prefs.is_empty());
            assert!(!imported.warnings.is_empty());
            assert_eq!(
                after.pages["playlist"]["generator_filename"],
                "current.m3u8"
            );
            assert_eq!(after.pages["tag-sources"]["cover_filename"], "old");
        }
        let (after, _, _) =
            native_options_apply(&[("f64318074ac91b4db1b909e2ce5a695c", b"")], &before);
        assert_eq!(after.pages["tag-sources"]["cover_filename"], "");
    }
    fn native_server_payload(rows: &[[&[u8]; 4]]) -> Vec<u8> {
        let mut bytes = (rows.len() as u32).to_le_bytes().to_vec();
        for row in rows {
            for field in row {
                bytes.extend((field.len() as u32).to_le_bytes());
                bytes.extend_from_slice(field);
            }
        }
        bytes
    }
    #[test]
    fn native_options_freedb_list_and_active_row_publish_as_one_dataset() {
        let list = native_server_payload(&[
            [b"First", b"first.example.org", b"/~cddb/cddb.cgi", b"80"],
            [
                "Twee 🎵".as_bytes(),
                b"second.example.org\0\xff",
                b"/custom/cddb.cgi",
                b"8880",
            ],
        ]);
        let mut before = crate::settings::Settings::default();
        page(
            &mut before,
            "tag-sources-freedb",
            "email",
            "target@example.org",
        )
        .unwrap();
        page(&mut before, "tag-sources-freedb", "comment_disc_id", true).unwrap();
        let (after, _, parsed) = native_options_apply(
            &[
                ("1f60b2ade49d454cb0eeeaf43c19359e", &list),
                ("2345b1c1c149ad4fbaeed1d837e96668", &1u32.to_le_bytes()),
            ],
            &before,
        );
        let actual = &after.pages["tag-sources-freedb"];
        assert_eq!(
            actual["servers"],
            json!([
                {"name":"First","protocol":"http","address":"first.example.org","path":"/~cddb/cddb.cgi","port":"80"},
                {"name":"Twee 🎵","protocol":"http","address":"second.example.org","path":"/custom/cddb.cgi","port":"8880"},
            ])
        );
        assert_eq!(actual["server"], 1);
        assert_eq!(actual["protocol"], "http");
        assert_eq!(actual["address"], "second.example.org");
        assert_eq!(actual["path"], "/custom/cddb.cgi");
        assert_eq!(actual["port"], "8880");
        assert_eq!(actual["email"], "target@example.org");
        assert_eq!(actual["comment_disc_id"], true);
        assert!(parsed.settings.iter().any(|row| row
            .value
            .as_deref()
            .is_some_and(|v| v.contains("Twee 🎵"))
            && row.verbatim));
    }
    #[test]
    fn native_options_freedb_preview_uses_provider_path_normalization_without_rewriting_fields() {
        for path in ["cddb.cgi", " /cddb.cgi "] {
            let list =
                native_server_payload(&[[b"Relative", b" example.org ", path.as_bytes(), b"80"]]);
            let (after, _, parsed) = native_options_apply(
                &[
                    ("1f60b2ade49d454cb0eeeaf43c19359e", &list),
                    ("2345b1c1c149ad4fbaeed1d837e96668", &[0, 0, 0, 0]),
                ],
                &crate::settings::Settings::default(),
            );
            assert!(parsed.settings.iter().any(
                |row| row.value.as_deref() == Some("Relative — http://example.org:80/cddb.cgi")
            ));
            assert_eq!(after.pages["tag-sources-freedb"]["path"], path);
            assert_eq!(
                after.pages["tag-sources-freedb"]["address"],
                " example.org "
            );
        }
    }
    #[test]
    fn native_options_freedb_incomplete_or_invalid_final_dataset_keeps_current_identity() {
        let valid = native_server_payload(&[[b"Good", b"old.example.org", b"/cddb", b"80"]]);
        let empty = native_server_payload(&[]);
        let mut before = crate::settings::Settings::default();
        page(&mut before,"tag-sources-freedb","servers",json!([{"name":"Current","protocol":"https","address":"target.example.org","path":"/current","port":"443"}])).unwrap();
        for (key, value) in [
            ("protocol", "https"),
            ("address", "target.example.org"),
            ("path", "/current"),
            ("port", "443"),
        ] {
            page(&mut before, "tag-sources-freedb", key, value).unwrap();
        }
        page(&mut before, "tag-sources-freedb", "server", 0).unwrap();
        let bad = native_server_payload(&[[b"Secret", b"user:secret@host", b"/cddb", b"80"]]);
        let pairs: Vec<Vec<(&str, &[u8])>> = vec![
            vec![("1f60b2ade49d454cb0eeeaf43c19359e", &valid)],
            vec![("2345b1c1c149ad4fbaeed1d837e96668", &[0, 0, 0, 0])],
            vec![
                ("1f60b2ade49d454cb0eeeaf43c19359e", &empty),
                ("2345b1c1c149ad4fbaeed1d837e96668", &[0, 0, 0, 0]),
            ],
            vec![
                ("1f60b2ade49d454cb0eeeaf43c19359e", &valid),
                ("2345b1c1c149ad4fbaeed1d837e96668", &[1, 0, 0, 0]),
            ],
            vec![
                ("1f60b2ade49d454cb0eeeaf43c19359e", &valid),
                ("1f60b2ade49d454cb0eeeaf43c19359e", &bad),
                ("2345b1c1c149ad4fbaeed1d837e96668", &[0, 0, 0, 0]),
            ],
        ];
        for mut records in pairs {
            records.push(("eada7c7f01da5f4a849392b72b37ef08", &[1]));
            let (after, _, parsed) = native_options_apply(&records, &before);
            assert_eq!(
                after.pages["tag-sources-freedb"],
                before.pages["tag-sources-freedb"]
            );
            assert!(!parsed.warnings.is_empty());
            assert!(parsed
                .warnings
                .iter()
                .all(|warning| !warning.contains("secret")));
            assert!(parsed.settings.iter().all(|setting| !setting
                .value
                .as_deref()
                .unwrap_or("")
                .contains("secret")));
        }
        for (host, path, port) in [
            ("http://host", "/cddb", "80"),
            ("host", "/cddb?token=secret", "80"),
            ("host", "/cddb#secret", "80"),
            ("host", "/cddb", "65536"),
            ("host", "/cddb", "0"),
            ("host", "/cddb", "80x"),
        ] {
            let list = native_server_payload(&[[
                b"Invalid",
                host.as_bytes(),
                path.as_bytes(),
                port.as_bytes(),
            ]]);
            let (after, _, _) = native_options_apply(
                &[
                    ("1f60b2ade49d454cb0eeeaf43c19359e", &list),
                    ("2345b1c1c149ad4fbaeed1d837e96668", &[0, 0, 0, 0]),
                    ("eada7c7f01da5f4a849392b72b37ef08", &[1]),
                ],
                &before,
            );
            assert_eq!(
                after.pages["tag-sources-freedb"],
                before.pages["tag-sources-freedb"]
            );
        }
        for length in 0..valid.len() {
            assert!(parse(&record(
                &native_id("1f60b2ade49d454cb0eeeaf43c19359e"),
                &valid[..length]
            ))
            .is_err());
        }
        let over = 4097u32.to_le_bytes();
        assert!(parse(&record(
            &native_id("1f60b2ade49d454cb0eeeaf43c19359e"),
            &over
        ))
        .is_err());
    }
    #[test]
    fn native_options_generator_fallback_never_adopts_an_invalid_source_expression() {
        let (after, _, parsed) = native_options_apply(
            &[
                ("7eaf6b64ce85a54196043a3c8cfb1d2e", b"$if("),
                ("312fa18b404068489ebd20ea4183cc24", b""),
                ("0e3b78b6452b224789abc77122e2d677", b"C:/Grouping"),
            ],
            &crate::settings::Settings::default(),
        );
        assert!(after.pages["playlist"].get("generator_filename").is_none());
        assert_eq!(
            after.pages["playlist"]["generator_partition"],
            "C:/Grouping"
        );
        assert!(!parsed.warnings.is_empty());
    }
    #[test]
    fn binary_settings_migration_isolates_every_static_mapping_and_boolean_codec() {
        let fixture: serde_json::Value = serde_json::from_str(include_str!(
            "../../tests/fixtures/mp3tag-settings-booleans.json"
        ))
        .unwrap();
        let options = fixture["options"].as_array().unwrap();
        assert_eq!(options.len(), 39);
        let mut cases = vec![
            vec![0; options.len()],
            vec![1; options.len()],
            vec![255; options.len()],
        ];
        // Distinct native flags must never get swapped just because all flags
        // happened to share the same value in an ordinary profile fixture.
        cases.extend((0..options.len()).map(|selected| {
            let mut raw = vec![0; options.len()];
            raw[selected] = 1;
            raw
        }));
        for raw in cases {
            let bytes = options
                .iter()
                .zip(&raw)
                .flat_map(|(option, raw)| {
                    record(&native_id(option["identifier"].as_str().unwrap()), &[*raw])
                })
                .collect::<Vec<_>>();
            let parsed = parse(&bytes).unwrap();
            assert_eq!(parsed.settings.len(), options.len());
            let temp = tempfile::tempdir().unwrap();
            let preferences = parsed.patch.apply(temp.path(), &BTreeMap::new()).unwrap();
            let settings = crate::settings::load(&temp.path().join("settings.json")).0;
            let panel = crate::tag_panel::load(&temp.path().join("tag-panel.json")).unwrap();
            let actual = json!({"settings":settings,"panel":panel,"preferences":preferences});
            for (option, raw) in options.iter().zip(raw) {
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
    fn mapping_error(bytes: &[u8]) -> String {
        match parse(bytes) {
            Err(error) => error,
            Ok(_) => panic!("invalid mapping record accepted"),
        }
    }
    fn mapping_payload(rows: &[[&[u8]; 5]]) -> Vec<u8> {
        let mut bytes = (rows.len() as u32).to_le_bytes().to_vec();
        for row in rows {
            for field in row {
                bytes.extend((field.len() as u32).to_le_bytes());
                bytes.extend_from_slice(field);
            }
        }
        bytes
    }
    #[test]
    fn native_mapping_list_decodes_all_five_fields_and_preserves_order() {
        let payload = mapping_payload(&[
            [b"VorbisComment", b"DATE\0\xff", b"YEAR", b"", b""],
            [
                b"MP4",
                "©nam".as_bytes(),
                "TITEL🎵".as_bytes(),
                &[255],
                b"unused",
            ],
        ]);
        let parsed = parse(&record(
            &native_id("41a00ac0f0ccf14f924a133d5bd6e0e7"),
            &payload,
        ))
        .unwrap();
        assert_eq!(parsed.settings.len(), 3);
        assert!(parsed
            .warnings
            .iter()
            .any(|w| w.contains("reserved mapping")));
        let temp = tempfile::tempdir().unwrap();
        parsed.patch.apply(temp.path(), &BTreeMap::new()).unwrap();
        let settings = crate::settings::load(&temp.path().join("settings.json")).0;
        assert_eq!(
            serde_json::to_value(&settings.tags.mappings).unwrap(),
            json!([
                {"tag_type":"VorbisComment","source":"DATE","target":"YEAR"},
                {"tag_type":"MP4","source":"©nam","target":"TITEL🎵"}
            ])
        );
        assert!(parsed.settings.iter().skip(1).all(|row| row.verbatim));
    }
    #[test]
    fn native_mapping_list_clear_and_last_record_override_are_explicit() {
        let id = native_id("41a00ac0f0ccf14f924a133d5bd6e0e7");
        let original = record(
            &id,
            &mapping_payload(&[[b"VorbisComment", b"DATE", b"YEAR", b"", b""]]),
        );
        let clear = record(&id, &0u32.to_le_bytes());
        let parsed = parse(&[original, clear].concat()).unwrap();
        assert_eq!(parsed.settings.len(), 1);
        assert_eq!(parsed.settings[0].value.as_deref(), Some("0"));
        let temp = tempfile::tempdir().unwrap();
        let mut settings = crate::settings::Settings::default();
        settings.tags.mappings = vec![tag_core::UserFieldMapping {
            tag_type: "APEv2".into(),
            source: "OLD".into(),
            target: "VALUE".into(),
        }];
        crate::settings::save(&temp.path().join("settings.json"), &settings).unwrap();
        parsed.patch.apply(temp.path(), &BTreeMap::new()).unwrap();
        assert!(crate::settings::load(&temp.path().join("settings.json"))
            .0
            .tags
            .mappings
            .is_empty());
    }
    #[test]
    fn native_mapping_list_refuses_truncation_utf8_and_declared_bounds() {
        let id = native_id("41a00ac0f0ccf14f924a133d5bd6e0e7");
        let valid = mapping_payload(&[[b"VorbisComment", b"DATE", b"YEAR", b"", b""]]);
        for end in 0..valid.len() {
            assert!(parse(&record(&id, &valid[..end])).is_err(), "prefix{end}");
        }
        assert!(mapping_error(&record(&id, &4097u32.to_le_bytes())).contains("4096"));
        let too_long = [1u32.to_le_bytes(), ((TEXT_LIMIT + 1) as u32).to_le_bytes()].concat();
        assert!(mapping_error(&record(&id, &too_long)).contains("4 MB"));
        let malformed = mapping_payload(&[[b"VorbisComment", &[255], b"YEAR", b"", b""]]);
        assert!(mapping_error(&record(&id, &malformed)).contains("UTF-8"));
    }
    #[test]
    fn native_mapping_unsupported_or_conflicting_final_list_keeps_all_current_rules() {
        let id = native_id("41a00ac0f0ccf14f924a133d5bd6e0e7");
        for payload in [
            mapping_payload(&[[b"FutureTag", b"DATE", b"YEAR", b"", b""]]),
            mapping_payload(&[
                [b"APEv2", b"X", b"Y", b"", b""],
                [b"APEv2", b"X", b"Z", b"", b""],
            ]),
        ] {
            let bytes = [
                record(&id, &0u32.to_le_bytes()),
                record(&id, &payload),
                record(&native_id("a17772f7656a2b4fb710657ed41cb01d"), &[0]),
            ]
            .concat();
            let parsed = parse(&bytes).unwrap();
            assert_eq!(parsed.settings.len(), 1);
            assert!(parsed.warnings.iter().any(|w| w.contains("mapping list")));
            assert!(!parsed.warnings.iter().any(|w| w.contains("FutureTag")));
            let temp = tempfile::tempdir().unwrap();
            let mut settings = crate::settings::Settings::default();
            settings.tags.mappings = vec![tag_core::UserFieldMapping {
                tag_type: "APEv2".into(),
                source: "KEEP".into(),
                target: "ME".into(),
            }];
            crate::settings::save(&temp.path().join("settings.json"), &settings).unwrap();
            parsed.patch.apply(temp.path(), &BTreeMap::new()).unwrap();
            assert_eq!(
                crate::settings::load(&temp.path().join("settings.json"))
                    .0
                    .tags
                    .mappings,
                settings.tags.mappings
            );
        }
    }
    #[test]
    fn native_cover_directory_and_supported_padding_values_are_portable() {
        for directory in [
            "",
            "Scans",
            "../Artwork",
            "%_folderpath%Scans",
            "$if2(%artist%,Unknown)/Art",
        ] {
            let bytes = [
                record(
                    &native_id("a7a810227a5a8d4db511e77851eb10dd"),
                    directory.as_bytes(),
                ),
                record(&native_id("81e9cd2aac92e14e9c71cf21ae4e0510"), &[0]),
                record(&native_id("e4d56fdc8c9b154c99e8c0adf4f25b22"), &[0]),
            ]
            .concat();
            let parsed = parse(&bytes).unwrap();
            assert_eq!(parsed.settings.len(), 3);
            let temp = tempfile::tempdir().unwrap();
            let current = BTreeMap::from([(
                "opentag.numbering".into(),
                json!({"track_digits":4,"disc_digits":3,"track_start":8}).to_string(),
            )]);
            let preferences = parsed.patch.apply(temp.path(), &current).unwrap();
            let numbering: serde_json::Value =
                serde_json::from_str(&preferences["opentag.numbering"]).unwrap();
            assert_eq!(
                numbering,
                json!({"track_digits":1,"disc_digits":1,"track_start":8})
            );
            assert_eq!(
                crate::settings::load(&temp.path().join("settings.json"))
                    .0
                    .pages["directories"]["cover_art"],
                directory
            );
        }
    }
    #[test]
    fn native_cover_host_paths_and_dynamic_padding_are_skipped_with_fixed_warnings() {
        for directory in [
            "C:\\Users\\Private\\Art",
            "\\\\server\\share\\Art",
            "/other/host/Art",
            " /other/host/Art ",
            " C:/Other/Art ",
            "file:/Other/Art",
            " file:/Other/Art ",
            "$if(",
        ] {
            let bytes = [
                record(
                    &native_id("a7a810227a5a8d4db511e77851eb10dd"),
                    directory.as_bytes(),
                ),
                record(&native_id("81e9cd2aac92e14e9c71cf21ae4e0510"), &[255]),
                record(&native_id("e4d56fdc8c9b154c99e8c0adf4f25b22"), &[1]),
                record(&native_id("a17772f7656a2b4fb710657ed41cb01d"), &[0]),
            ]
            .concat();
            let parsed = parse(&bytes).unwrap();
            assert_eq!(parsed.settings.len(), 1);
            assert!(parsed
                .warnings
                .iter()
                .any(|w| w.contains("cover-art directory")));
            assert!(
                parsed
                    .warnings
                    .iter()
                    .filter(|w| w.contains("leading-zero"))
                    .count()
                    == 2
            );
            assert!(!parsed.warnings.iter().any(|w| w.contains(directory)));
            let temp = tempfile::tempdir().unwrap();
            assert!(parsed
                .patch
                .apply(temp.path(), &BTreeMap::new())
                .unwrap()
                .is_empty());
            assert!(!crate::settings::load(&temp.path().join("settings.json"))
                .0
                .pages
                .contains_key("directories"));
        }
    }
    #[test]
    fn binary_export_records_reject_short_payloads_and_invalid_utf8() {
        for (id, length) in [
            ("a91684289cba6f498fd857faeaa3749c", 1),
            ("9e917d2becf8af4793061746fdf7bad5", 1),
            ("759de112c0072d48b5650c2fbec586a8", 4),
        ] {
            for short in 0..length {
                assert!(parse(&record(&native_id(id), &vec![0; short])).is_err());
            }
        }
        for id in [
            "e31f66267bd2a34baffa13f42d34fd95",
            "536d231fc389234bafa84663f0986a0c",
            "d1c3c5a4c95780489c008de588e2eeea",
        ] {
            assert!(parse(&record(&native_id(id), &[255])).is_err());
            assert!(parse(&record(&native_id(id), &vec![b'a'; TEXT_LIMIT + 1])).is_err());
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
    fn binary_settings_remainders_import_grid_and_advanced_text_without_coercion() {
        for (grid, expected) in [(0, "off"), (1, "alternate"), (2, "lines")] {
            for separator in ["", " - "] {
                let bytes = [
                    record(
                        &native_id("10ad9a5de1a201419efb4f510bd262f4"),
                        &[grid, 0, 0, 0, 99],
                    ),
                    record(
                        &native_id("8eec759f9a170443abf0e47af87c1079"),
                        b"NLD\0ignored",
                    ),
                    record(
                        &native_id("0c7f0a198af644488be02a0d19631585"),
                        b"*.MP3; *.flac, opus",
                    ),
                    record(
                        &native_id("375e4b6e9c45af49a77caac696ced2b6"),
                        separator.as_bytes(),
                    ),
                ]
                .concat();
                let parsed = parse(&bytes).unwrap();
                assert_eq!(parsed.settings.len(), 4);
                assert!(parsed.warnings.is_empty());
                let temp = tempfile::tempdir().unwrap();
                parsed.patch.apply(temp.path(), &BTreeMap::new()).unwrap();
                let options = crate::settings::load(&temp.path().join("settings.json")).0;
                assert_eq!(options.pages["appearance"]["grid"], expected);
                assert_eq!(options.tags.advanced.id3v2_comment_language, "NLD");
                assert_eq!(options.files.restrict_extensions, "*.MP3; *.flac, opus");
                assert_eq!(options.files.folder_separator, separator);
                assert!(parsed
                    .settings
                    .iter()
                    .any(|item| item.value.as_deref() == Some(separator) && item.verbatim));
            }
        }
    }

    #[test]
    fn binary_settings_remainders_skip_last_invalid_value_and_preserve_current_choices() {
        let temp = tempfile::tempdir().unwrap();
        let mut before = crate::settings::Settings::default();
        before.files.restrict_extensions = "*.wav".into();
        before.files.folder_separator = "__".into();
        before.tags.advanced.id3v2_comment_language = "deu".into();
        page(&mut before, "appearance", "grid", "lines").unwrap();
        crate::settings::save(&temp.path().join("settings.json"), &before).unwrap();
        let bytes = [
            record(
                &native_id("10ad9a5de1a201419efb4f510bd262f4"),
                &0u32.to_le_bytes(),
            ),
            record(
                &native_id("10ad9a5de1a201419efb4f510bd262f4"),
                &99u32.to_le_bytes(),
            ),
            record(&native_id("8eec759f9a170443abf0e47af87c1079"), b"eng"),
            record(&native_id("8eec759f9a170443abf0e47af87c1079"), b"english"),
            record(
                &native_id("0c7f0a198af644488be02a0d19631585"),
                b"*.mp3;../private",
            ),
            record(&native_id("375e4b6e9c45af49a77caac696ced2b6"), b"/"),
            record(&native_id("8840da8e2956d74da1853b63f6df9037"), &[1]),
        ]
        .concat();
        let parsed = parse(&bytes).unwrap();
        assert_eq!(parsed.settings.len(), 1);
        assert_eq!(parsed.warnings.len(), 4);
        assert!(parsed
            .warnings
            .iter()
            .all(|warning| !warning.contains("private") && !warning.contains("english")));
        parsed.patch.apply(temp.path(), &BTreeMap::new()).unwrap();
        let after = crate::settings::load(&temp.path().join("settings.json")).0;
        assert_eq!(
            after.files.restrict_extensions,
            before.files.restrict_extensions
        );
        assert_eq!(after.files.folder_separator, before.files.folder_separator);
        assert_eq!(after.tags.advanced.id3v2_comment_language, "deu");
        assert_eq!(after.pages["appearance"]["grid"], "lines");
        assert_eq!(after.pages["tags"]["save_on_move"], true);
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
        assert!(parsed.patch.apply(temp.path(), &BTreeMap::new()).is_err());
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
            parsed.patch.apply(temp.path(), &BTreeMap::new()).unwrap();
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
        parsed.patch.apply(temp.path(), &BTreeMap::new()).unwrap();
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
        assert!(parsed
            .patch
            .apply(temp.path(), &BTreeMap::new())
            .unwrap()
            .is_empty());
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
        parsed.patch.apply(temp.path(), &BTreeMap::new()).unwrap();
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
            parsed.patch.apply(temp.path(), &BTreeMap::new()).unwrap();
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
    #[test]
    fn binary_settings_tag_masks_match_native_controls_and_keep_unmapped_values() {
        let fixture: serde_json::Value = serde_json::from_str(include_str!(
            "../../tests/fixtures/mp3tag-settings-tags.json"
        ))
        .unwrap();
        let flag = native_id("8840da8e2956d74da1853b63f6df9037");
        for spec in fixture["masks"].as_array().unwrap() {
            let id = native_id(spec["identifier"].as_str().unwrap());
            let known = spec["known_mask"].as_u64().unwrap() as u32;
            let writable = spec["writable_mask"].as_u64().unwrap() as u32;
            for raw in [
                0u32, 1, 2, 3, 4, 7, 8, 11, 15, 16, 32, 48, 64, 112, 127, 256,
            ] {
                let bytes = [record(&flag, &[1]), record(&id, &raw.to_le_bytes())].concat();
                let parsed = parse(&bytes).unwrap();
                let temp = tempfile::tempdir().unwrap();
                let mut before = crate::settings::Settings::default();
                before.tags.advanced.popularimeter_ratings = true;
                before
                    .pages
                    .insert("playlist".into(), json!({"filename":"preserved.m3u8"}));
                crate::settings::save(&temp.path().join("settings.json"), &before).unwrap();
                parsed.patch.apply(temp.path(), &BTreeMap::new()).unwrap();
                let actual = crate::settings::load(&temp.path().join("settings.json")).0;
                assert!(actual.tags.advanced.popularimeter_ratings);
                assert_eq!(actual.pages["playlist"]["filename"], "preserved.m3u8");
                let actual = serde_json::to_value(actual.tags).unwrap();
                let before = serde_json::to_value(before.tags).unwrap();
                let group = spec["group"].as_str().unwrap();
                for rule in spec["rules"].as_array().unwrap() {
                    let field = rule["field"].as_str().unwrap();
                    let mask = rule["mask"].as_u64().unwrap() as u32;
                    let expected = if writable != 0 && raw & writable == 0 {
                        before[group][field].clone()
                    } else {
                        json!(raw & mask == mask)
                    };
                    assert_eq!(
                        actual[group][field], expected,
                        "{} {field} raw={raw}",
                        spec["key"]
                    );
                }
                if raw & !known != 0 || writable != 0 && raw & writable == 0 {
                    assert!(!parsed.warnings.is_empty());
                }
            }
            for length in 0..4 {
                assert!(parse(&record(&id, &vec![0; length])).is_err());
            }
        }
    }
    #[test]
    fn binary_settings_tag_encoding_uses_native_radio_values_and_last_record() {
        let id = native_id("025280d33d85d942b0a8699eb8a0b3d2");
        let flag = native_id("8840da8e2956d74da1853b63f6df9037");
        for (raw, expected) in [
            (0u32, "v23_latin1"),
            (1, "v23_utf16"),
            (3, "v24_utf8"),
            (2, "v24_utf8"),
            (u32::MAX, "v24_utf8"),
        ] {
            let bytes = [
                record(&flag, &[1]),
                record(&id, &0u32.to_le_bytes()),
                record(&id, &raw.to_le_bytes()),
            ]
            .concat();
            let parsed = parse(&bytes).unwrap();
            let temp = tempfile::tempdir().unwrap();
            let mut before = serde_json::to_value(crate::settings::Settings::default()).unwrap();
            before["tags"]["mpeg"]["id3v2_format"] = json!("v24_utf8");
            std::fs::write(
                temp.path().join("settings.json"),
                serde_json::to_vec(&before).unwrap(),
            )
            .unwrap();
            parsed.patch.apply(temp.path(), &BTreeMap::new()).unwrap();
            let actual =
                serde_json::to_value(crate::settings::load(&temp.path().join("settings.json")).0)
                    .unwrap();
            assert_eq!(actual["tags"]["mpeg"]["id3v2_format"], expected);
            if raw == 2 || raw == u32::MAX {
                assert_eq!(parsed.settings.len(), 1);
                assert!(!parsed.warnings.is_empty());
            }
        }
        for length in 0..4 {
            assert!(parse(&record(&id, &vec![0; length])).is_err());
        }
    }
}
