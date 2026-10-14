use md5::{Digest, Md5};
use std::{fs::File, io::Read, path::Path};
use tag_core::Fields;

pub fn compute(path: &Path, requested: &[String]) -> Result<Fields, String> {
    let mut fields = Fields::new();
    if requested.is_empty() {
        return Ok(fields);
    }
    if requested.iter().any(|name| name == "_MD5AUDIO") {
        return Err(
            "Audio-only MD5 requires codec-specific audio ranges; this export was not written"
                .into(),
        );
    }
    let mut input = File::open(path).map_err(|e| e.to_string())?;
    let before = input.metadata().map_err(|e| e.to_string())?;
    let mut crc = requested
        .iter()
        .any(|name| name == "_CRC")
        .then(crc32fast::Hasher::new);
    let mut md5 = requested.iter().any(|name| name == "_MD5").then(Md5::new);
    let mut buffer = [0u8; 65536];
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
        assert!(compute(&path, &["_MD5AUDIO".into()]).is_err());
        assert!(compute(Path::new("missing-file"), &[]).unwrap().is_empty());
    }
}
