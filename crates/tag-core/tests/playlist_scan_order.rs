//! Authored local PCM/CUE carriers; no installed application or external tools.
use std::{fs, path::Path};
use tag_core::{playlist::load_playlist, scan_paths, subsong_path};

fn wav(path: &Path) {
    let samples = vec![0u8; 16000];
    let mut bytes = Vec::new();
    bytes.extend(b"RIFF");
    bytes.extend((36 + samples.len() as u32).to_le_bytes());
    bytes.extend(b"WAVEfmt ");
    bytes.extend(16u32.to_le_bytes());
    bytes.extend(1u16.to_le_bytes());
    bytes.extend(1u16.to_le_bytes());
    bytes.extend(8000u32.to_le_bytes());
    bytes.extend(16000u32.to_le_bytes());
    bytes.extend(2u16.to_le_bytes());
    bytes.extend(16u16.to_le_bytes());
    bytes.extend(b"data");
    bytes.extend((samples.len() as u32).to_le_bytes());
    bytes.extend(samples);
    fs::write(path, bytes).unwrap();
}

#[test]
fn mixed_cue_rows_and_ordinary_members_keep_the_actual_input_order() {
    let temp = tempfile::tempdir().unwrap();
    let root = dunce::canonicalize(temp.path()).unwrap();
    let audio = root.join("image.wav");
    let single = root.join("single.wav");
    let cue = root.join("album.cue");
    wav(&audio);
    wav(&single);
    let source = b"FILE \"image.wav\" WAVE\n  TRACK 01 AUDIO\n    INDEX 01 00:00:00\n  TRACK 02 AUDIO\n    INDEX 01 00:00:30\n";
    fs::write(&cue, source).unwrap();
    let members = load_playlist(&cue, None).unwrap();
    assert_eq!(members.files.len(), 2);
    let expected = vec![
        subsong_path(&cue.to_string_lossy(), 2),
        single.to_string_lossy().into_owned(),
        subsong_path(&cue.to_string_lossy(), 1),
    ];
    let mut inputs = expected.clone();
    inputs.push(expected[0].clone());
    let result = scan_paths(&inputs, false);
    assert!(result.errors.is_empty(), "{:?}", result.errors);
    assert_eq!(
        result
            .entries
            .iter()
            .map(|entry| entry.path.clone())
            .collect::<Vec<_>>(),
        expected
    );
    assert_eq!(fs::read(&cue).unwrap(), source);

    let directory = root.join("tracks");
    fs::create_dir(&directory).unwrap();
    let first = directory.join("a.wav");
    let last = directory.join("z.wav");
    wav(&last);
    wav(&first);
    let mixed = vec![
        expected[0].clone(),
        directory.to_string_lossy().into_owned(),
        expected[1].clone(),
        expected[2].clone(),
        first.to_string_lossy().into_owned(),
    ];
    let result = scan_paths(&mixed, false);
    assert!(result.errors.is_empty(), "{:?}", result.errors);
    assert_eq!(
        result
            .entries
            .iter()
            .map(|entry| entry.path.clone())
            .collect::<Vec<_>>(),
        vec![
            expected[0].clone(),
            first.to_string_lossy().into_owned(),
            last.to_string_lossy().into_owned(),
            expected[1].clone(),
            expected[2].clone(),
        ]
    );
}
