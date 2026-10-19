#![cfg(test)]
use super::*;
use std::io::Write;

fn wav(path: &Path) {
    let samples: Vec<u8> = (0..8000)
        .flat_map(|i| ((i % 251) as i16 * 31).to_le_bytes())
        .collect();
    let mut bytes = b"RIFF".to_vec();
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

fn external_change_during_staging_is_retained(replace: bool) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("audio.wav");
    wav(&path);
    let input = fs::read(&path).unwrap();
    let mut external = input.clone();
    if !replace {
        external[44] ^= 1;
    }
    fs::write(dir.path().join("input.wav"), &input).unwrap();
    fs::write(dir.path().join("external.wav"), &external).unwrap();
    let result = staged_write(
        &path,
        false,
        &[],
        &TagOptions::default(),
        |staging, original, _| {
            let before = fs::metadata(original).unwrap();
            // Independently mutate the staging image, making successful publication observable.
            staging.seek(SeekFrom::Start(44)).unwrap();
            staging.write_all(&[input[44] ^ 2]).unwrap();
            if replace {
                let mut replacement = tempfile::NamedTempFile::new_in(dir.path()).unwrap();
                replacement.write_all(&external).unwrap();
                replacement
                    .as_file()
                    .set_modified(before.modified().unwrap())
                    .unwrap();
                replacement.persist(original).unwrap();
            } else {
                let mut file = fs::OpenOptions::new().write(true).open(original).unwrap();
                file.seek(SeekFrom::Start(44)).unwrap();
                file.write_all(&[external[44]]).unwrap();
                file.sync_all().unwrap();
                file.set_modified(before.modified().unwrap()).unwrap();
            }
            let after = fs::metadata(original).unwrap();
            assert_eq!(after.len(), before.len());
            assert_eq!(after.modified().unwrap(), before.modified().unwrap());
            #[cfg(unix)]
            {
                use std::os::unix::fs::MetadataExt;
                assert_eq!(before.ino() == after.ino(), !replace);
            }
            Ok(())
        },
    );
    fs::copy(&path, dir.path().join("published.wav")).unwrap();
    // Keep independent inputs and actual output before an expected RED assertion.
    let retained = if result.is_ok() {
        Some(dir.keep())
    } else {
        None
    };
    if let Some(retained) = &retained {
        eprintln!("Retained unsafe staged publication: {}", retained.display());
    }
    assert!(
        result.is_err(),
        "staged publication accepted concealed external change; replace={replace}"
    );
    assert!(result.unwrap_err().contains("changed during editing"));
    assert!(
        fs::read(&path).unwrap() == external,
        "external bytes were overwritten"
    );
}

#[test]
fn concealed_same_object_edit_during_staging_is_not_overwritten() {
    external_change_during_staging_is_retained(false);
}

#[test]
fn same_content_replacement_during_staging_is_not_overwritten() {
    external_change_during_staging_is_retained(true);
}
