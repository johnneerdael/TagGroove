//! Independently authored ID3 envelopes and DSD wrappers; the MPEG payload is
//! the frozen independently encoded save-corpus carrier. No reference writer.
use std::{fs, path::Path};
use tag_core::history::{Journal, Limits};
use tag_core::{
    read_entry_with_options, write_fields_with_options, Fields, Id3v2Format, TagOptions,
};

fn syncsafe(value: usize) -> [u8; 4] {
    [21, 14, 7, 0].map(|shift| ((value >> shift) & 127) as u8)
}

fn unsync(bytes: &[u8]) -> Vec<u8> {
    let mut output = Vec::new();
    for (index, byte) in bytes.iter().enumerate() {
        output.push(*byte);
        if *byte == 255
            && bytes
                .get(index + 1)
                .is_none_or(|next| *next == 0 || *next >= 224)
        {
            output.push(0);
        }
    }
    output
}

fn deunsync(bytes: &[u8]) -> Vec<u8> {
    let mut output = Vec::new();
    let mut index = 0;
    while index < bytes.len() {
        output.push(bytes[index]);
        index += 1;
        if output.last() == Some(&255) && bytes.get(index) == Some(&0) {
            index += 1;
        }
    }
    output
}

fn crc32(bytes: &[u8]) -> u32 {
    let mut crc = !0u32;
    for byte in bytes {
        crc ^= u32::from(*byte);
        for _ in 0..8 {
            crc = (crc >> 1) ^ (0xedb8_8320 & (0u32.wrapping_sub(crc & 1)));
        }
    }
    !crc
}

fn zlib(bytes: &[u8]) -> Vec<u8> {
    let length = u16::try_from(bytes.len()).unwrap();
    let mut output = vec![0x78, 0x01, 0x01];
    output.extend(length.to_le_bytes());
    output.extend((!length).to_le_bytes());
    output.extend(bytes);
    let (mut a, mut b) = (1u32, 0u32);
    for byte in bytes {
        a = (a + u32::from(*byte)) % 65521;
        b = (b + a) % 65521;
    }
    output.extend(((b << 16) | a).to_be_bytes());
    output
}

fn frame(major: u8, id: &[u8; 4], flags: u8, body: &[u8]) -> Vec<u8> {
    let mut output = id.to_vec();
    output.extend(if major == 3 {
        (body.len() as u32).to_be_bytes()
    } else {
        syncsafe(body.len())
    });
    output.extend([0, flags]);
    output.extend(body);
    output
}

#[derive(Clone, Copy, Debug)]
enum Envelope {
    Plain3,
    Extended3,
    Crc3,
    Unsync3,
    Plain4,
    Crc4,
    Footer4,
    FrameUnsync4,
}

impl Envelope {
    fn supported(self) -> bool {
        matches!(
            self,
            Self::Plain3 | Self::Extended3 | Self::Crc3 | Self::Plain4 | Self::FrameUnsync4
        )
    }

    fn major(self) -> u8 {
        if matches!(
            self,
            Self::Plain3 | Self::Extended3 | Self::Crc3 | Self::Unsync3
        ) {
            3
        } else {
            4
        }
    }

    fn tag(self) -> Vec<u8> {
        let major = self.major();
        let album = b"\0Compressed album";
        let mut compressed = if major == 3 {
            (album.len() as u32).to_be_bytes().to_vec()
        } else {
            syncsafe(album.len()).to_vec()
        };
        compressed.extend(zlib(album));
        // Synthetic opaque bytes with actual false-sync/FF00 sequences. They
        // carry an encryption-method marker, not a claimed encoder ciphertext.
        let encrypted = b"\x81\x13\x37\xff\xe0opaque\xff\0cipher";
        let per_frame = matches!(self, Self::FrameUnsync4);
        let encrypted = if per_frame {
            unsync(encrypted)
        } else {
            encrypted.to_vec()
        };
        let frames = [
            frame(major, b"TIT2", 0, b"\0Plain title"),
            // Register synthetic method 0x81 under an independently authored owner.
            frame(
                major,
                b"ENCR",
                0,
                b"qualification.example.invalid\0\x81fixture method",
            ),
            frame(
                major,
                b"TALB",
                if major == 3 { 0x80 } else { 0x09 },
                &compressed,
            ),
            frame(
                major,
                b"TXXX",
                if major == 3 {
                    0x40
                } else {
                    0x04 | if per_frame { 0x02 } else { 0 }
                },
                &encrypted,
            ),
        ]
        .concat();
        let padding = if matches!(self, Self::Footer4) {
            0
        } else {
            128
        };
        let mut body = Vec::new();
        let flags = match self {
            Self::Extended3 => {
                body.extend(6u32.to_be_bytes());
                body.extend(0u16.to_be_bytes());
                body.extend((padding as u32).to_be_bytes());
                0x40
            }
            Self::Crc3 => {
                body.extend(10u32.to_be_bytes());
                body.extend(0x8000u16.to_be_bytes());
                body.extend((padding as u32).to_be_bytes());
                body.extend(crc32(&frames).to_be_bytes());
                0x40
            }
            Self::Crc4 => {
                body.extend(syncsafe(12));
                body.extend([1, 0x20, 5]);
                let mut crc_input = frames.clone();
                crc_input.resize(crc_input.len() + padding, 0);
                let crc = u64::from(crc32(&crc_input));
                body.extend([28, 21, 14, 7, 0].map(|shift| ((crc >> shift) & 127) as u8));
                0x40
            }
            Self::Unsync3 => 0x80,
            Self::Footer4 => 0x10,
            _ => 0,
        };
        body.extend(frames);
        body.resize(body.len() + padding, 0);
        if matches!(self, Self::Unsync3) {
            body = unsync(&body);
        }
        let mut output = vec![b'I', b'D', b'3', major, 0, flags];
        output.extend(syncsafe(body.len()));
        output.extend(body);
        if matches!(self, Self::Footer4) {
            let mut footer = output[..10].to_vec();
            footer[..3].copy_from_slice(b"3DI");
            output.extend(footer);
        }
        output
    }
}

fn dff_chunk(id: &[u8; 4], body: &[u8]) -> Vec<u8> {
    let mut output = id.to_vec();
    output.extend((body.len() as u64).to_be_bytes());
    output.extend(body);
    if body.len() % 2 == 1 {
        output.push(0);
    }
    output
}

fn carrier(extension: &str, tag: &[u8]) -> Vec<u8> {
    if extension == "mp3" {
        let base = include_bytes!("fixtures/save-corpus/tone.mp3");
        assert_eq!(&base[..20], b"ID3\x04\0\0\0\0\0\x0a\0\0\0\0\0\0\0\0\0\0");
        return [tag, &base[20..]].concat();
    }
    // The same independently authored DSD sample formula as formats_gaps.rs.
    let sound: Vec<u8> = (0..2 * 4096 * 2).map(|i| (i * 37 % 251) as u8).collect();
    if extension == "dsf" {
        let mut output = b"DSD ".to_vec();
        output.extend(28u64.to_le_bytes());
        output.extend(0u64.to_le_bytes());
        output.extend(0u64.to_le_bytes());
        output.extend(b"fmt ");
        output.extend(52u64.to_le_bytes());
        for value in [1u32, 0, 2, 2, 2_822_400, 1] {
            output.extend(value.to_le_bytes());
        }
        output.extend((sound.len() as u64 * 4).to_le_bytes());
        output.extend(4096u32.to_le_bytes());
        output.extend(0u32.to_le_bytes());
        output.extend(b"data");
        output.extend((12 + sound.len() as u64).to_le_bytes());
        output.extend(sound);
        let pointer = output.len() as u64;
        output[20..28].copy_from_slice(&pointer.to_le_bytes());
        output.extend(tag);
        let length = output.len() as u64;
        output[12..20].copy_from_slice(&length.to_le_bytes());
        output
    } else {
        let mut properties = b"SND ".to_vec();
        properties.extend(dff_chunk(b"FS  ", &2_822_400u32.to_be_bytes()));
        properties.extend(dff_chunk(b"CHNL", b"\0\x02SLFTSRGT"));
        properties.extend(dff_chunk(b"CMPR", b"DSD \x0enot compressed"));
        let body = [
            b"DSD ".as_slice(),
            &dff_chunk(b"FVER", &[1, 5, 0, 0]),
            &dff_chunk(b"PROP", &properties),
            &dff_chunk(b"DSD ", &sound),
            &dff_chunk(b"ID3 ", tag),
        ]
        .concat();
        [
            b"FRM8".as_slice(),
            &(body.len() as u64).to_be_bytes(),
            &body,
        ]
        .concat()
    }
}

fn native_parts<'a>(extension: &str, bytes: &'a [u8]) -> (&'a [u8], &'a [u8]) {
    if extension == "mp3" {
        let size = bytes[6..10]
            .iter()
            .fold(0usize, |n, b| (n << 7) | usize::from(*b));
        let end = 10 + size + if bytes[5] & 0x10 != 0 { 10 } else { 0 };
        (&bytes[..end], &bytes[end..])
    } else if extension == "dsf" {
        let pointer = u64::from_le_bytes(bytes[20..28].try_into().unwrap()) as usize;
        let size = u64::from_le_bytes(bytes[84..92].try_into().unwrap()) as usize;
        (&bytes[pointer..], &bytes[92..80 + size])
    } else {
        let mut at = 16;
        let (mut id3, mut sound) = (None, None);
        while at + 12 <= bytes.len() {
            let size = u64::from_be_bytes(bytes[at + 4..at + 12].try_into().unwrap()) as usize;
            let body = &bytes[at + 12..at + 12 + size];
            match &bytes[at..at + 4] {
                b"ID3 " => id3 = Some(body),
                b"DSD " => sound = Some(body),
                _ => {}
            }
            at += 12 + size + size % 2;
        }
        (id3.unwrap(), sound.unwrap())
    }
}

fn opaque_records(bytes: &[u8]) -> Vec<Vec<u8>> {
    let major = bytes[3];
    let size = bytes[6..10]
        .iter()
        .fold(0usize, |n, b| (n << 7) | usize::from(*b));
    let body = if major == 3 && bytes[5] & 0x80 != 0 {
        deunsync(&bytes[10..10 + size])
    } else {
        bytes[10..10 + size].to_vec()
    };
    let mut at = if bytes[5] & 0x40 != 0 {
        if major == 3 {
            4 + u32::from_be_bytes(body[..4].try_into().unwrap()) as usize
        } else {
            body[..4]
                .iter()
                .fold(0usize, |n, b| (n << 7) | usize::from(*b))
        }
    } else {
        0
    };
    let mut output = Vec::new();
    while at + 10 <= body.len() && body[at] != 0 {
        let size = if major == 3 {
            u32::from_be_bytes(body[at + 4..at + 8].try_into().unwrap()) as usize
        } else {
            body[at + 4..at + 8]
                .iter()
                .fold(0usize, |n, b| (n << 7) | usize::from(*b))
        };
        let end = at + 10 + size;
        assert!(end <= body.len(), "authored native frame exceeds tag");
        if &body[at..at + 4] == b"ENCR" || body[at + 9] & if major == 3 { 0xc0 } else { 0x0c } != 0
        {
            output.push(body[at..end].to_vec());
        }
        at = end;
    }
    output.sort();
    output
}

fn keep_evidence(name: &str, stage: &str, bytes: &[u8]) {
    if let Some(directory) = std::env::var_os("TAGGROOVE_L26_RECEIPTS") {
        let path = Path::new(&directory).join(name);
        fs::create_dir_all(&path).unwrap();
        fs::write(path.join(stage), bytes).unwrap();
    }
}

#[test]
fn unrelated_edits_keep_opaque_frames_and_audio_or_refuse_without_touching_the_original() {
    let mut failures = Vec::new();
    for envelope in [
        Envelope::Plain3,
        Envelope::Extended3,
        Envelope::Crc3,
        Envelope::Unsync3,
        Envelope::Plain4,
        Envelope::Crc4,
        Envelope::Footer4,
        Envelope::FrameUnsync4,
    ] {
        for extension in ["mp3", "dsf", "dff"] {
            let name = format!("{envelope:?}.{extension}");
            let dir = tempfile::tempdir().unwrap();
            let path = dir.path().join(&name);
            let before = carrier(extension, &envelope.tag());
            let (before_tag, before_audio) = native_parts(extension, &before);
            let expected_frames = opaque_records(before_tag);
            assert_eq!(expected_frames.len(), 3);
            fs::write(&path, &before).unwrap();
            keep_evidence(&name, "input", &before);
            let mut options = TagOptions::default();
            options.mpeg.write_id3v1 = false;
            if envelope.major() == 4 {
                options.mpeg.id3v2_format = Id3v2Format::V24Utf8;
            }
            let read = read_entry_with_options(&path, &[], &options);
            match &read {
                Err(error) => {
                    println!("{name}: read-refused: {error}");
                    keep_evidence(&name, "initial-error.txt", error.as_bytes());
                    assert!(!envelope.supported(), "{name}: supported read refused");
                    assert_eq!(fs::read(&path).unwrap(), before);
                }
                Ok(entry) => {
                    println!("{name}: initial displayed fields {:?}", entry.fields);
                    keep_evidence(
                        &name,
                        "initial-fields.json",
                        &serde_json::to_vec(&entry.fields).unwrap(),
                    );
                    if entry.fields.get("TITLE") != Some(&vec!["Plain title".into()])
                        || entry.fields.get("ALBUM") != Some(&vec!["Compressed album".into()])
                    {
                        failures.push(format!(
                            "{name}: accepted read omitted authored title/album: {:?}",
                            entry.fields
                        ));
                    }
                }
            }
            let mut history =
                Journal::open(&dir.path().join("journal"), Limits::default()).unwrap();
            history.set_tag_options(options.clone());
            // A failed edit must not replace an already usable Undo operation.
            let control = dir.path().join("control.mp3");
            fs::write(&control, include_bytes!("fixtures/save-corpus/tone.mp3")).unwrap();
            let prior = history.begin(
                "Earlier control edit",
                None,
                &[control.to_string_lossy().into()],
            );
            let control_fields: Fields = [("ARTIST".into(), vec!["Earlier".into()])]
                .into_iter()
                .collect();
            write_fields_with_options(&control, &control_fields, false, &[], &options).unwrap();
            history.commit(prior, &[], &[]);
            let prior_bytes = history.retained_bytes();
            let capture = history.begin(
                "Unrelated artist edit",
                None,
                &[path.to_string_lossy().into()],
            );
            let fields: Fields = [("ARTIST".into(), vec!["Someone".into()])]
                .into_iter()
                .collect();
            let result = write_fields_with_options(&path, &fields, false, &[], &options);
            history.commit(capture, &[], &[]);
            let after = fs::read(&path).unwrap();
            keep_evidence(&name, "after", &after);
            if let Err(error) = result {
                println!("{name}: write-refused: {error}");
                keep_evidence(&name, "write-error.txt", error.as_bytes());
                assert_eq!(after, before, "{name}: refusal changed bytes");
                assert!(
                    history.state().undo.unwrap().label == "Earlier control edit",
                    "{name}: refusal replaced history"
                );
                assert_eq!(
                    history.retained_bytes(),
                    prior_bytes,
                    "{name}: refusal changed retained history"
                );
                assert!(!envelope.supported(), "{name}: supported write refused");
                continue;
            }
            assert!(
                read.is_ok(),
                "{name}: unreadable input was accepted for writing"
            );
            let (after_tag, after_audio) = native_parts(extension, &after);
            println!(
                "{name}: saved; header flags {:02x}->{:02x}; opaque records {}->{}",
                before_tag[5],
                after_tag[5],
                expected_frames.len(),
                opaque_records(after_tag).len()
            );
            if opaque_records(after_tag) != expected_frames {
                failures.push(format!("{name}: saved opaque native frames changed"));
            }
            assert_eq!(after_audio, before_audio, "{name}: encoded sound changed");
            let reread = read_entry_with_options(&path, &[], &options).unwrap();
            keep_evidence(
                &name,
                "after-fields.json",
                &serde_json::to_vec(&reread.fields).unwrap(),
            );
            assert_eq!(reread.fields["ARTIST"], ["Someone"], "{name}");
            assert_eq!(reread.fields["TITLE"], ["Plain title"], "{name}");
            assert_eq!(reread.fields["ALBUM"], ["Compressed album"], "{name}");
            assert!(
                history.undo().unwrap().errors.is_empty(),
                "{name}: Undo failed"
            );
            assert_eq!(fs::read(&path).unwrap(), before, "{name}: Undo bytes");
            keep_evidence(&name, "undone", &before);
            assert!(
                history.redo().unwrap().errors.is_empty(),
                "{name}: Redo failed"
            );
            assert_eq!(fs::read(&path).unwrap(), after, "{name}: Redo bytes");
            keep_evidence(&name, "redone", &after);
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn malformed_and_combined_unsync_extended_headers_refuse_without_changing_disk_or_history() {
    for variant in [
        "short",
        "oversize",
        "flags",
        "padding",
        "nonzero-padding",
        "crc",
        "truncated",
        "unsync",
    ] {
        let mut tag = Envelope::Crc3.tag();
        match variant {
            "short" => tag[10..14].copy_from_slice(&4u32.to_be_bytes()),
            "oversize" => {
                let size = tag.len() as u32 + 1;
                tag[10..14].copy_from_slice(&size.to_be_bytes());
            }
            "flags" => tag[14] = 0x40,
            "padding" => tag[16..20].copy_from_slice(&u32::MAX.to_be_bytes()),
            "nonzero-padding" => *tag.last_mut().unwrap() = 1,
            "crc" => tag[20] ^= 1,
            "truncated" => tag.truncate(21),
            "unsync" => {
                let body = unsync(&tag[10..]);
                tag.truncate(10);
                tag[5] |= 0x80;
                tag[6..10].copy_from_slice(&syncsafe(body.len()));
                tag.extend(body);
            }
            _ => unreachable!(),
        }
        for extension in ["mp3", "dsf", "dff"] {
            let dir = tempfile::tempdir().unwrap();
            let path = dir.path().join(format!("{variant}.{extension}"));
            let original = carrier(extension, &tag);
            fs::write(&path, &original).unwrap();
            let mut options = TagOptions::default();
            options.mpeg.write_id3v1 = false;
            let read = read_entry_with_options(&path, &[], &options);
            assert!(
                read.is_err(),
                "{variant}.{extension}: malformed read accepted"
            );
            let mut history =
                Journal::open(&dir.path().join("journal"), Limits::default()).unwrap();
            let control = dir.path().join("control.mp3");
            fs::write(&control, include_bytes!("fixtures/save-corpus/tone.mp3")).unwrap();
            let fields: Fields = [("ARTIST".into(), vec!["Earlier".into()])]
                .into_iter()
                .collect();
            let prior = history.begin(
                "Earlier control edit",
                None,
                &[control.to_string_lossy().into()],
            );
            write_fields_with_options(&control, &fields, false, &[], &options).unwrap();
            history.commit(prior, &[], &[]);
            let retained = history.retained_bytes();
            let capture = history.begin("Refused edit", None, &[path.to_string_lossy().into()]);
            assert!(
                write_fields_with_options(&path, &fields, false, &[], &options).is_err(),
                "{variant}.{extension}"
            );
            history.commit(capture, &[], &[]);
            assert_eq!(fs::read(&path).unwrap(), original, "{variant}.{extension}");
            assert_eq!(history.state().undo.unwrap().label, "Earlier control edit");
            assert_eq!(history.retained_bytes(), retained);
        }
    }
}

#[test]
fn extended_header_view_composes_with_headerless_ape_read_overlay() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("extended-ape.mp3");
    let mut original = carrier("mp3", &Envelope::Crc3.tag());
    // Hand-authored APEv2 without its optional header exercises the existing
    // APE read overlay at the opposite end of the same real media file.
    let mut item = (9u32).to_le_bytes().to_vec();
    item.extend(0u32.to_le_bytes());
    item.extend(b"Title\0APE title");
    let mut footer = b"APETAGEX".to_vec();
    for value in [2000u32, item.len() as u32 + 32, 1, 0] {
        footer.extend(value.to_le_bytes());
    }
    footer.extend([0; 8]);
    original.extend(item);
    original.extend(footer);
    fs::write(&path, &original).unwrap();
    let mut options = TagOptions::default();
    options.mpeg.write_id3v1 = false;
    options.mpeg.read_ape = false;
    options.mpeg.remove_ape = false;
    let before = read_entry_with_options(&path, &[], &options).unwrap();
    assert_eq!(before.fields["TITLE"], ["Plain title"]);
    assert_eq!(before.fields["ALBUM"], ["Compressed album"]);
    let fields: Fields = [("ARTIST".into(), vec!["Someone".into()])]
        .into_iter()
        .collect();
    write_fields_with_options(&path, &fields, false, &[], &options).unwrap();
    let after = fs::read(&path).unwrap();
    let payload = &include_bytes!("fixtures/save-corpus/tone.mp3")[20..];
    assert!(native_parts("mp3", &after).1.starts_with(payload));
    assert_eq!(
        opaque_records(native_parts("mp3", &after).0),
        opaque_records(native_parts("mp3", &original).0)
    );
    options.mpeg.read_ape = true;
    let ape = read_entry_with_options(&path, &[], &options).unwrap();
    assert_eq!(ape.fields["Title"], ["APE title"]);
}

fn pcm_container(extension: &str, tag: &[u8]) -> Vec<u8> {
    let little = extension == "wav";
    let chunk = |id: &[u8; 4], data: &[u8]| {
        let mut output = id.to_vec();
        output.extend(if little {
            (data.len() as u32).to_le_bytes()
        } else {
            (data.len() as u32).to_be_bytes()
        });
        output.extend(data);
        if data.len() % 2 == 1 {
            output.push(0);
        }
        output
    };
    let samples: Vec<u8> = (0..1600)
        .flat_map(|index| {
            let value = (index % 251) as i16 * 31;
            if little {
                value.to_le_bytes()
            } else {
                value.to_be_bytes()
            }
        })
        .collect();
    let mut body = if little {
        b"WAVE".to_vec()
    } else {
        b"AIFF".to_vec()
    };
    if little {
        let mut format = Vec::new();
        format.extend(1u16.to_le_bytes());
        format.extend(1u16.to_le_bytes());
        format.extend(8000u32.to_le_bytes());
        format.extend(16000u32.to_le_bytes());
        format.extend(2u16.to_le_bytes());
        format.extend(16u16.to_le_bytes());
        body.extend(chunk(b"fmt ", &format));
    } else {
        let mut common = 1u16.to_be_bytes().to_vec();
        common.extend(1600u32.to_be_bytes());
        common.extend(16u16.to_be_bytes());
        common.extend([0x40, 0x0b, 0xfa, 0, 0, 0, 0, 0, 0, 0]); // 8000Hz, IEEE extended.
        body.extend(chunk(b"COMM", &common));
    }
    // Keep audio after the ID3 chunk: an excessive tag declaration can remain
    // within the physical file while escaping its owning metadata chunk.
    body.extend(chunk(b"ID3 ", tag));
    if little {
        body.extend(chunk(b"data", &samples));
    } else {
        let mut sound = vec![0; 8]; // SSND offset and block size.
        sound.extend(samples);
        body.extend(chunk(b"SSND", &sound));
    }
    let mut output = if little {
        b"RIFF".to_vec()
    } else {
        b"FORM".to_vec()
    };
    output.extend(if little {
        (body.len() as u32).to_le_bytes()
    } else {
        (body.len() as u32).to_be_bytes()
    });
    output.extend(body);
    output
}

fn pcm_parts<'a>(extension: &str, bytes: &'a [u8]) -> (&'a [u8], &'a [u8]) {
    let mut at = 12;
    let (mut tag, mut sound) = (None, None);
    while at + 8 <= bytes.len() {
        let size: [u8; 4] = bytes[at + 4..at + 8].try_into().unwrap();
        let size = if extension == "wav" {
            u32::from_le_bytes(size)
        } else {
            u32::from_be_bytes(size)
        } as usize;
        let body = &bytes[at + 8..at + 8 + size];
        match &bytes[at..at + 4] {
            b"ID3 " | b"id3 " => tag = Some(body),
            b"data" => sound = Some(body),
            b"SSND" => sound = Some(&body[8..]),
            _ => {}
        }
        at += 8 + size + size % 2;
    }
    (tag.unwrap(), sound.unwrap())
}

#[test]
fn wav_and_aiff_extended_header_views_respect_the_owning_chunk() {
    for extension in ["wav", "aiff"] {
        for exceeds_chunk in [false, true] {
            let dir = tempfile::tempdir().unwrap();
            let path = dir.path().join(format!("extended.{extension}"));
            let mut tag = Envelope::Crc3.tag();
            if exceeds_chunk {
                // This remains below the physical file length, so file bounds
                // alone cannot protect the following PCM sample chunk.
                let declared = tag.len() - 10 + 32;
                tag[6..10].copy_from_slice(&syncsafe(declared));
            }
            let original = pcm_container(extension, &tag);
            let (original_tag, original_sound) = pcm_parts(extension, &original);
            let tag_offset = original
                .windows(6)
                .position(|window| window == b"ID3\x03\0\x40")
                .unwrap();
            if exceeds_chunk {
                assert!(tag_offset + tag.len() + 32 < original.len());
            }
            fs::write(&path, &original).unwrap();
            let options = TagOptions::default();
            let read = read_entry_with_options(&path, &[], &options);
            if exceeds_chunk {
                assert!(
                    read.is_err(),
                    "{extension}: escaped ID3 chunk read accepted"
                );
            } else {
                let entry = read.unwrap();
                assert_eq!(entry.fields["TITLE"], ["Plain title"], "{extension}");
                assert_eq!(entry.fields["ALBUM"], ["Compressed album"], "{extension}");
            }
            let mut history =
                Journal::open(&dir.path().join("journal"), Limits::default()).unwrap();
            let control = dir.path().join("control.mp3");
            fs::write(&control, include_bytes!("fixtures/save-corpus/tone.mp3")).unwrap();
            let fields: Fields = [("ARTIST".into(), vec!["Someone".into()])]
                .into_iter()
                .collect();
            let prior = history.begin(
                "Earlier control edit",
                None,
                &[control.to_string_lossy().into()],
            );
            write_fields_with_options(&control, &fields, false, &[], &options).unwrap();
            history.commit(prior, &[], &[]);
            let retained = history.retained_bytes();
            let capture = history.begin(
                "Embedded artist edit",
                None,
                &[path.to_string_lossy().into()],
            );
            let write = write_fields_with_options(&path, &fields, false, &[], &options);
            history.commit(capture, &[], &[]);
            let after = fs::read(&path).unwrap();
            if exceeds_chunk {
                assert!(
                    write.is_err(),
                    "{extension}: escaped ID3 chunk write accepted"
                );
                assert_eq!(after, original, "{extension}: refusal changed bytes");
                assert_eq!(pcm_parts(extension, &after).1, original_sound);
                assert_eq!(history.state().undo.unwrap().label, "Earlier control edit");
                assert_eq!(history.retained_bytes(), retained);
                println!(
                    "{extension} exceeds_chunk={exceeds_chunk}: protected refusal {:?}",
                    write.unwrap_err()
                );
            } else {
                write.unwrap_or_else(|error| {
                    panic!("{extension}: supported extended-header write refused: {error}")
                });
                let (after_tag, after_sound) = pcm_parts(extension, &after);
                assert_eq!(after_sound, original_sound, "{extension}");
                assert_eq!(opaque_records(after_tag), opaque_records(original_tag));
                let entry = read_entry_with_options(&path, &[], &options).unwrap();
                assert_eq!(entry.fields["TITLE"], ["Plain title"]);
                assert_eq!(entry.fields["ALBUM"], ["Compressed album"]);
                assert_eq!(entry.fields["ARTIST"], ["Someone"]);
                assert!(history.undo().unwrap().errors.is_empty());
                assert_eq!(fs::read(&path).unwrap(), original);
                assert!(history.redo().unwrap().errors.is_empty());
                assert_eq!(fs::read(&path).unwrap(), after);
                println!("{extension}: positive read/save/Undo/Redo control passed");
            }
        }
    }
}
