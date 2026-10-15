use md5::{Digest, Md5};
use std::{fs::File, io::Read, path::Path};
use tag_core::{AudioDigest, Fields};

pub fn compute(path: &Path, requested: &[String]) -> Result<Fields, String> {
    let mut fields = Fields::new();
    if requested.is_empty() {
        return Ok(fields);
    }
    // `_MD5AUDIO` is stored by FLAC and WavPack, hashed over the audio
    // between the tags of ID3/APE-tagged streams, and empty otherwise.
    let mut audio = None;
    if requested.iter().any(|name| name == "_MD5AUDIO") {
        match tag_core::audio_digest(path)? {
            AudioDigest::Stored(value) => {
                fields.insert("_MD5AUDIO".into(), vec![value]);
            }
            AudioDigest::Range(range) => audio = Some((range, Md5::new())),
            AudioDigest::Unavailable => {
                fields.insert("_MD5AUDIO".into(), vec![String::new()]);
            }
        }
    }
    let mut crc = requested
        .iter()
        .any(|name| name == "_CRC")
        .then(crc32fast::Hasher::new);
    let mut md5 = requested.iter().any(|name| name == "_MD5").then(Md5::new);
    if crc.is_none() && md5.is_none() && audio.is_none() {
        return Ok(fields);
    }
    let mut input = File::open(path).map_err(|e| e.to_string())?;
    let before = input.metadata().map_err(|e| e.to_string())?;
    let mut buffer = [0u8; 65536];
    let mut position = 0u64;
    loop {
        let length = input.read(&mut buffer).map_err(|e| e.to_string())?;
        if length == 0 {
            break;
        }
        if let Some(hash) = &mut crc {
            hash.update(&buffer[..length]);
        }
        if let Some(hash) = &mut md5 {
            hash.update(&buffer[..length]);
        }
        if let Some((range, hash)) = &mut audio {
            let end = position + length as u64;
            let from = range.start.clamp(position, end);
            let to = range.end.clamp(position, end);
            if from < to {
                hash.update(&buffer[(from - position) as usize..(to - position) as usize]);
            }
        }
        position += length as u64;
    }
    let after = input.metadata().map_err(|e| e.to_string())?;
    if before.len() != after.len() || before.modified().ok() != after.modified().ok() {
        return Err(format!(
            "{} changed while its checksum was calculated",
            path.display()
        ));
    }
    if let Some(hash) = crc {
        fields.insert("_CRC".into(), vec![format!("{:08X}", hash.finalize())]);
    }
    if let Some(hash) = md5 {
        fields.insert("_MD5".into(), vec![format!("{:x}", hash.finalize())]);
    }
    if let Some((_, hash)) = audio {
        fields.insert("_MD5AUDIO".into(), vec![format!("{:x}", hash.finalize())]);
    }
    Ok(fields)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn file_checksums_match_published_standard_vector_and_no_audio_hash_is_invented() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("bytes");
        std::fs::write(&path, b"123456789").unwrap();
        let hashes = compute(&path, &["_CRC".into(), "_MD5".into()]).unwrap();
        assert_eq!(hashes["_CRC"], ["CBF43926"]);
        assert_eq!(hashes["_MD5"], ["25f9e794323b453885f5181f1b624d0b"]);
        // Not an audio stream with a known audio range: empty, not invented.
        assert_eq!(
            compute(&path, &["_MD5AUDIO".into()]).unwrap()["_MD5AUDIO"],
            [""]
        );
        assert!(compute(Path::new("missing-file"), &[]).unwrap().is_empty());
    }

    #[test]
    fn audio_md5_skips_the_tags_of_id3_and_ape_tagged_streams() {
        let dir = tempfile::tempdir().unwrap();
        let mut frame = vec![0xFF, 0xFB, 0x90, 0x00];
        frame.resize(417, 7);
        let mut tagged = b"ID3\x03\x00\x00\x00\x00\x00\x05".to_vec();
        tagged.extend([0; 5]);
        tagged.extend(&frame);
        let mut id3v1 = b"TAG".to_vec();
        id3v1.resize(128, 1);
        tagged.extend(id3v1);
        let path = dir.path().join("a.mp3");
        std::fs::write(&path, &tagged).unwrap();
        let bare = dir.path().join("b.mp3");
        std::fs::write(&bare, &frame).unwrap();
        let expected = format!("{:x}", Md5::digest(&frame));
        let hashes = compute(&path, &["_MD5AUDIO".into(), "_MD5".into()]).unwrap();
        assert_eq!(hashes["_MD5AUDIO"][0], expected);
        assert_ne!(hashes["_MD5"][0], expected);
        assert_eq!(
            compute(&bare, &["_MD5AUDIO".into()]).unwrap()["_MD5AUDIO"],
            [expected]
        );
    }
}
