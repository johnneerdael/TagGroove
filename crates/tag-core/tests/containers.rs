//! DSF, DSDIFF and Matroska/WebM, built byte by byte from the public format
//! descriptions. ffmpeg-generated Matroska and WebM files are exercised when
//! ffmpeg and ffprobe are installed.
use lofty::{
    config::WriteOptions,
    id3::v2::{BinaryFrame, Frame, FrameId, Id3v2Tag},
    prelude::TagExt,
    tag::Accessor,
};
use std::{fs, path::Path, process::Command};
use tag_core::{
    add_cover, read_entry, remove_cover, remove_tags, scan_paths, set_cover_properties_summary,
    write_fields, write_fields_with_options, Fields, Id3v2Format, TagOptions,
};

fn fields(values: &[(&str, &[&str])]) -> Fields {
    values
        .iter()
        .map(|(key, values)| {
            (
                key.to_string(),
                values.iter().map(|v| v.to_string()).collect(),
            )
        })
        .collect()
}
fn value(entry: &tag_core::AudioEntry, key: &str) -> Option<String> {
    entry.fields.get(key).map(|v| v.join("|"))
}
fn technical(entry: &tag_core::AudioEntry, key: &str) -> String {
    entry
        .technical
        .get(key)
        .map(|v| v.join("|"))
        .unwrap_or_default()
}
fn samples(length: usize) -> Vec<u8> {
    (0..length).map(|i| (i * 37 % 251) as u8).collect()
}
fn png(path: &Path) {
    image::RgbImage::from_pixel(3, 2, image::Rgb([200, 10, 10]))
        .save(path)
        .unwrap();
}
fn id3_bytes(tag: &Id3v2Tag) -> Vec<u8> {
    let mut bytes = Vec::new();
    tag.dump_to(&mut bytes, WriteOptions::default()).unwrap();
    bytes
}

// ---------------------------------------------------------------- DSF

/// DSD ␣ header, fmt chunk and data chunk; `id3` is appended as metadata.
fn dsf(data: &[u8], id3: Option<&[u8]>) -> Vec<u8> {
    let channels = 2u32;
    let mut bytes = b"DSD ".to_vec();
    bytes.extend(28u64.to_le_bytes());
    let total_at = bytes.len();
    bytes.extend(0u64.to_le_bytes());
    bytes.extend(0u64.to_le_bytes());
    bytes.extend(b"fmt ");
    bytes.extend(52u64.to_le_bytes());
    for value in [1u32, 0, 2, channels, 2_822_400, 1] {
        bytes.extend(value.to_le_bytes());
    }
    bytes.extend((data.len() as u64 * 8 / u64::from(channels)).to_le_bytes());
    bytes.extend(4096u32.to_le_bytes());
    bytes.extend(0u32.to_le_bytes());
    bytes.extend(b"data");
    bytes.extend((12 + data.len() as u64).to_le_bytes());
    bytes.extend(data);
    if let Some(id3) = id3 {
        let pointer = bytes.len() as u64;
        bytes[20..28].copy_from_slice(&pointer.to_le_bytes());
        bytes.extend(id3);
    }
    let total = bytes.len() as u64;
    bytes[total_at..total_at + 8].copy_from_slice(&total.to_le_bytes());
    bytes
}
fn le64(bytes: &[u8], at: usize) -> u64 {
    u64::from_le_bytes(bytes[at..at + 8].try_into().unwrap())
}

#[test]
fn dsf_tags_round_trip_and_keep_sample_data() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("track.dsf");
    let data = samples(2 * 4096 * 3);
    fs::write(&path, dsf(&data, None)).unwrap();
    let entry = read_entry(&path).unwrap();
    assert_eq!(entry.format, "DSF");
    assert!(entry.fields.is_empty());
    assert_eq!(entry.sample_rate, 2_822_400);
    assert_eq!(entry.channels, 2);
    assert_eq!(technical(&entry, "_CODEC"), "DSD64");
    assert_eq!(technical(&entry, "_BITSPERSAMPLE"), "1");
    assert!((entry.duration - 3.0 * 4096.0 * 8.0 / 2_822_400.0).abs() < 1e-9);
    assert_eq!(entry.bitrate, 5644);

    let written = write_fields(
        &path,
        &fields(&[
            ("TITLE", &["Ünïcode title"]),
            ("ARTIST", &["One", "Two"]),
            ("TRACK", &["3/9"]),
            ("YEAR", &["2024"]),
            ("CUSTOM", &["x"]),
        ]),
    )
    .unwrap();
    assert_eq!(value(&written, "TITLE").as_deref(), Some("Ünïcode title"));
    assert_eq!(value(&written, "ARTIST").as_deref(), Some("One|Two"));
    assert_eq!(value(&written, "CUSTOM").as_deref(), Some("x"));
    // New tags follow the MPEG preference: ID3v2.3 UTF-16 by default.
    assert_eq!(technical(&written, "_TAG"), "ID3v2.3");
    let bytes = fs::read(&path).unwrap();
    assert_eq!(le64(&bytes, 12), bytes.len() as u64);
    let pointer = le64(&bytes, 20) as usize;
    assert_eq!(pointer, 28 + 52 + 12 + data.len());
    assert_eq!(&bytes[pointer..pointer + 4], b"ID3\x03");
    assert_eq!(&bytes[92..92 + data.len()], data.as_slice());

    // The existing revision is preserved even when the preference differs.
    let mut options = TagOptions::default();
    options.mpeg.id3v2_format = Id3v2Format::V24Utf8;
    let rewritten =
        write_fields_with_options(&path, &fields(&[("ALBUM", &["LP"])]), false, &[], &options)
            .unwrap();
    assert_eq!(technical(&rewritten, "_TAG"), "ID3v2.3");
    assert_eq!(value(&rewritten, "TITLE").as_deref(), Some("Ünïcode title"));

    let image = dir.path().join("front.png");
    png(&image);
    let covered = add_cover(&path, &image).unwrap();
    assert_eq!(covered.covers.len(), 1);
    assert_eq!(covered.covers[0].mime, "image/png");
    assert_eq!(covered.covers[0].width, Some(3));
    let back = set_cover_properties_summary(&path, Some("CoverBack"), Some("back"), None).unwrap();
    assert_eq!(back.covers[0].picture_type, "CoverBack");
    let uncovered = remove_cover(&path, 0).unwrap();
    assert!(uncovered.covers.is_empty());
    assert_eq!(value(&uncovered, "ALBUM").as_deref(), Some("LP"));

    let removed = remove_tags(&path).unwrap();
    assert!(removed.fields.is_empty());
    let bytes = fs::read(&path).unwrap();
    assert_eq!(bytes, dsf(&data, None));
}

#[test]
fn dsf_existing_id3v24_keeps_revision_and_unknown_frames() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("tagged.dsf");
    let mut tag = Id3v2Tag::new();
    tag.set_title("Old".into());
    tag.insert(Frame::Binary(BinaryFrame::new(
        FrameId::new("XOPQ").unwrap(),
        vec![1, 2, 3],
    )));
    let data = samples(2 * 4096);
    fs::write(&path, dsf(&data, Some(&id3_bytes(&tag)))).unwrap();
    let entry = read_entry(&path).unwrap();
    assert_eq!(value(&entry, "TITLE").as_deref(), Some("Old"));
    assert_eq!(technical(&entry, "_TAG"), "ID3v2.4");
    let written = write_fields(&path, &fields(&[("TITLE", &["New"])])).unwrap();
    assert_eq!(value(&written, "TITLE").as_deref(), Some("New"));
    assert_eq!(technical(&written, "_TAG"), "ID3v2.4");
    assert_eq!(technical(&written, "_ID3V2_UNKNOWN_FRAMES"), "XOPQ");
    // Shrinking the tag truncates the old metadata tail.
    let bytes = fs::read(&path).unwrap();
    assert_eq!(le64(&bytes, 12), bytes.len() as u64);
}

#[test]
fn malformed_dsf_is_rejected_without_changes() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("broken.dsf");
    let mut bytes = dsf(&samples(4096), None);
    bytes[20..28].copy_from_slice(&40u64.to_le_bytes());
    fs::write(&path, &bytes).unwrap();
    assert!(read_entry(&path).unwrap_err().contains("metadata pointer"));
    assert!(write_fields(&path, &fields(&[("TITLE", &["x"])])).is_err());
    assert_eq!(fs::read(&path).unwrap(), bytes);
    let truncated = dir.path().join("short.dsf");
    fs::write(&truncated, &bytes[..100]).unwrap();
    assert!(read_entry(&truncated).is_err());
}

// ---------------------------------------------------------------- DSDIFF

fn chunk(id: &[u8; 4], data: &[u8]) -> Vec<u8> {
    let mut bytes = id.to_vec();
    bytes.extend((data.len() as u64).to_be_bytes());
    bytes.extend(data);
    if data.len() % 2 == 1 {
        bytes.push(0);
    }
    bytes
}
fn text_chunk(id: &[u8; 4], text: &str) -> Vec<u8> {
    let mut data = (text.len() as u32).to_be_bytes().to_vec();
    data.extend(text.as_bytes());
    chunk(id, &data)
}
fn prop(compression: &[u8; 4], extra: &[u8]) -> Vec<u8> {
    let mut data = b"SND ".to_vec();
    data.extend(chunk(b"FS  ", &2_822_400u32.to_be_bytes()));
    data.extend(chunk(
        b"CHNL",
        &[0, 2, b'S', b'L', b'F', b'T', b'S', b'R', b'G', b'T'],
    ));
    let mut cmpr = compression.to_vec();
    cmpr.extend([14]);
    cmpr.extend(b"not compressed");
    data.extend(chunk(b"CMPR", &cmpr));
    data.extend(extra);
    chunk(b"PROP", &data)
}
fn form(chunks: &[Vec<u8>]) -> Vec<u8> {
    let mut body = b"DSD ".to_vec();
    body.extend(chunk(b"FVER", &[1, 5, 0, 0]));
    for c in chunks {
        body.extend(c);
    }
    let mut bytes = b"FRM8".to_vec();
    bytes.extend((body.len() as u64).to_be_bytes());
    bytes.extend(body);
    bytes
}
/// Root chunk identifiers in order, and the offset of the DSD sound data.
fn root_chunks(bytes: &[u8]) -> Vec<(String, usize, usize)> {
    let mut position = 16;
    let mut chunks = Vec::new();
    let end = 12 + u64::from_be_bytes(bytes[4..12].try_into().unwrap()) as usize;
    assert_eq!(end, bytes.len());
    while position + 12 <= end {
        let size =
            u64::from_be_bytes(bytes[position + 4..position + 12].try_into().unwrap()) as usize;
        chunks.push((
            String::from_utf8_lossy(&bytes[position..position + 4]).into_owned(),
            position + 12,
            size,
        ));
        position += 12 + size + size % 2;
    }
    chunks
}

#[test]
fn dff_reads_diin_writes_root_id3_and_keeps_sound_data() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("master.dff");
    let sound = samples(2 * 4096 + 1);
    let mut diin = text_chunk(b"EMID", "id-1");
    diin.extend(text_chunk(b"DIAR", "Edited Artist"));
    diin.extend(text_chunk(b"DITI", "Edited Title"));
    let original = form(&[
        prop(b"DSD ", &[]),
        chunk(b"DSD ", &sound),
        chunk(b"DIIN", &diin),
    ]);
    fs::write(&path, &original).unwrap();
    let entry = read_entry(&path).unwrap();
    assert_eq!(entry.format, "DFF");
    assert_eq!(value(&entry, "ARTIST").as_deref(), Some("Edited Artist"));
    assert_eq!(value(&entry, "TITLE").as_deref(), Some("Edited Title"));
    assert_eq!(technical(&entry, "_TAG"), "DIIN");
    assert_eq!(technical(&entry, "_CODEC"), "DSD64");
    assert!((entry.duration - sound.len() as f64 * 8.0 / (2.0 * 2_822_400.0)).abs() < 1e-9);

    let written = write_fields(&path, &fields(&[("ALBUM", &["Album"])])).unwrap();
    assert_eq!(value(&written, "ALBUM").as_deref(), Some("Album"));
    // DIIN still supplies the values the ID3v2 tag lacks.
    assert_eq!(value(&written, "ARTIST").as_deref(), Some("Edited Artist"));
    assert_eq!(technical(&written, "_TAG"), "ID3v2.3 DIIN");
    let bytes = fs::read(&path).unwrap();
    let chunks = root_chunks(&bytes);
    let ids: Vec<_> = chunks.iter().map(|c| c.0.as_str()).collect();
    assert_eq!(ids, ["FVER", "PROP", "DSD ", "DIIN", "ID3 "]);
    // Unchanged chunks are byte-identical at their original offsets.
    assert!(bytes[12..original.len()] == original[12..]);

    let written = write_fields(&path, &fields(&[("ARTIST", &["New Artist"])])).unwrap();
    assert_eq!(value(&written, "ARTIST").as_deref(), Some("New Artist"));
    let bytes = fs::read(&path).unwrap();
    assert!(bytes.windows(10).any(|w| w == b"New Artist"));
    assert!(!bytes.windows(13).any(|w| w == b"Edited Artist"));
    let (_, at, size) = root_chunks(&bytes)
        .into_iter()
        .find(|c| c.0 == "DSD ")
        .unwrap();
    assert_eq!(&bytes[at..at + size], sound.as_slice());

    let removed = remove_tags(&path).unwrap();
    assert!(removed.fields.is_empty(), "{:?}", removed.fields);
    let bytes = fs::read(&path).unwrap();
    let ids: Vec<_> = root_chunks(&bytes).into_iter().map(|c| c.0).collect();
    assert_eq!(ids, ["FVER", "PROP", "DSD ", "DIIN"]);
    assert!(bytes.windows(4).any(|w| w == b"EMID"));
}

#[test]
fn dff_property_id3_moves_to_root_and_duplicates_are_refused() {
    let dir = tempfile::tempdir().unwrap();
    let mut tag = Id3v2Tag::new();
    tag.set_artist("Prop Artist".into());
    let id3 = id3_bytes(&tag);
    let sound = samples(4096);

    let path = dir.path().join("prop.dff");
    fs::write(
        &path,
        form(&[prop(b"DSD ", &chunk(b"ID3 ", &id3)), chunk(b"DSD ", &sound)]),
    )
    .unwrap();
    assert_eq!(
        value(&read_entry(&path).unwrap(), "ARTIST").as_deref(),
        Some("Prop Artist")
    );
    let written = write_fields(&path, &fields(&[("TITLE", &["T"])])).unwrap();
    assert_eq!(value(&written, "ARTIST").as_deref(), Some("Prop Artist"));
    assert_eq!(technical(&written, "_TAG"), "ID3v2.4");
    let bytes = fs::read(&path).unwrap();
    assert_eq!(bytes.windows(4).filter(|w| w == b"ID3 ").count(), 1);
    let ids: Vec<_> = root_chunks(&bytes).into_iter().map(|c| c.0).collect();
    assert_eq!(ids, ["FVER", "PROP", "DSD ", "ID3 "]);

    let duplicate = dir.path().join("duplicate.dff");
    let bytes = form(&[
        prop(b"DSD ", &chunk(b"ID3 ", &id3)),
        chunk(b"DSD ", &sound),
        chunk(b"ID3 ", &id3),
    ]);
    fs::write(&duplicate, &bytes).unwrap();
    let error = write_fields(&duplicate, &fields(&[("TITLE", &["T"])])).unwrap_err();
    assert!(error.contains("more than one ID3"), "{error}");
    assert_eq!(fs::read(&duplicate).unwrap(), bytes);
    remove_tags(&duplicate).unwrap();
    assert!(!fs::read(&duplicate)
        .unwrap()
        .windows(4)
        .any(|w| w == b"ID3 "));
}

#[test]
fn dst_with_index_refuses_layouts_that_move_sound_data() {
    let dir = tempfile::tempdir().unwrap();
    let mut tag = Id3v2Tag::new();
    tag.set_title("x".into());
    let id3 = id3_bytes(&tag);
    let mut dst = chunk(b"FRTE", &[0, 0, 0, 75, 0, 75]);
    dst.extend(chunk(b"DSTF", &samples(300)));
    let index = chunk(b"DSTI", &[0; 12]);

    let moving = dir.path().join("moving.dff");
    let bytes = form(&[
        prop(b"DST ", &chunk(b"ID3 ", &id3)),
        chunk(b"DST ", &dst),
        index.clone(),
    ]);
    fs::write(&moving, &bytes).unwrap();
    let entry = read_entry(&moving).unwrap();
    assert_eq!(technical(&entry, "_CODEC"), "DST64");
    assert!((entry.duration - 1.0).abs() < 1e-9);
    let error = write_fields(&moving, &fields(&[("TITLE", &["y"])])).unwrap_err();
    assert!(error.contains("DST"), "{error}");
    assert_eq!(fs::read(&moving).unwrap(), bytes);

    let stable = dir.path().join("stable.dff");
    fs::write(
        &stable,
        form(&[prop(b"DST ", &[]), chunk(b"DST ", &dst), index]),
    )
    .unwrap();
    let written = write_fields(&stable, &fields(&[("TITLE", &["y"])])).unwrap();
    assert_eq!(value(&written, "TITLE").as_deref(), Some("y"));
}

// ---------------------------------------------------------------- Matroska

fn ebml_size(value: usize) -> Vec<u8> {
    if value < 127 {
        vec![0x80 | value as u8]
    } else {
        let mut bytes = vec![1];
        bytes.extend(&(value as u64).to_be_bytes()[1..]);
        bytes
    }
}
fn el(id: &[u8], payload: &[u8]) -> Vec<u8> {
    let mut bytes = id.to_vec();
    bytes.extend(ebml_size(payload.len()));
    bytes.extend(payload);
    bytes
}
fn cat(parts: &[Vec<u8>]) -> Vec<u8> {
    parts.concat()
}
fn simple(name: &str, value: &str) -> Vec<u8> {
    el(
        &[0x67, 0xC8],
        &cat(&[
            el(&[0x45, 0xA3], name.as_bytes()),
            el(&[0x44, 0x87], value.as_bytes()),
        ]),
    )
}
fn mkv_tag(targets: Vec<u8>, simple_tags: &[Vec<u8>]) -> Vec<u8> {
    let mut payload = el(&[0x63, 0xC0], &targets);
    payload.extend(simple_tags.concat());
    el(&[0x73, 0x73], &payload)
}
const CLUSTER: [u8; 4] = [0x1F, 0x43, 0xB6, 0x75];

/// A Segment with an 8-byte size: SeekHead + Void, Info, Tracks, Cluster, Tags.
fn matroska(video: bool) -> Vec<u8> {
    matroska_with(video, &[])
}
/// Like [`matroska`], with `chapters` (a Chapters element) after Tracks.
fn matroska_with(video: bool, chapters: &[u8]) -> Vec<u8> {
    let info = el(
        &[0x15, 0x49, 0xA9, 0x66],
        &cat(&[
            el(&[0x2A, 0xD7, 0xB1], &[0x0F, 0x42, 0x40]),
            el(&[0x44, 0x89], &2500f64.to_be_bytes()),
        ]),
    );
    let audio = el(
        &[0xAE],
        &cat(&[
            el(&[0xD7], &[1]),
            el(&[0x73, 0xC5], &[1]),
            el(&[0x83], &[2]),
            el(&[0x86], b"A_PCM/INT/LIT"),
            el(
                &[0xE1],
                &cat(&[
                    el(&[0xB5], &44100f64.to_be_bytes()),
                    el(&[0x9F], &[2]),
                    el(&[0x62, 0x64], &[16]),
                ]),
            ),
        ]),
    );
    let mut tracks_payload = audio;
    if video {
        tracks_payload.extend(el(
            &[0xAE],
            &cat(&[
                el(&[0xD7], &[2]),
                el(&[0x73, 0xC5], &[2]),
                el(&[0x83], &[1]),
                el(&[0x86], b"V_UNCOMPRESSED"),
            ]),
        ));
    }
    let tracks = el(&[0x16, 0x54, 0xAE, 0x6B], &tracks_payload);
    let cluster = cluster();
    let tags = el(
        &[0x12, 0x54, 0xC3, 0x67],
        &cat(&[
            mkv_tag(
                el(&[0x68, 0xCA], &[50]),
                &[
                    simple("TITLE", "The Album"),
                    simple("ARTIST", "Album Artist"),
                ],
            ),
            mkv_tag(
                el(&[0x68, 0xCA], &[30]),
                &[simple("TITLE", "The Song"), simple("COMMENT", "hi")],
            ),
            mkv_tag(el(&[0x63, 0xC5], &[1]), &[simple("DURATION", "00:00:02.5")]),
        ]),
    );
    let seek = |id: &[u8], position: usize| {
        el(
            &[0x4D, 0xBB],
            &cat(&[
                el(&[0x53, 0xAB], id),
                el(&[0x53, 0xAC], &(position as u32).to_be_bytes()),
            ]),
        )
    };
    // Positions relative to the Segment data; the SeekHead itself has a fixed size.
    let head_len = el(
        &[0x11, 0x4D, 0x9B, 0x74],
        &cat(&[seek(&[0; 4], 0), seek(&[0; 4], 0), seek(&[0; 4], 0)]),
    )
    .len();
    let void = el(&[0xEC], &[0; 60]);
    let info_at = head_len + void.len();
    let tracks_at = info_at + info.len();
    let tags_at = tracks_at + tracks.len() + chapters.len() + cluster.len();
    let head = el(
        &[0x11, 0x4D, 0x9B, 0x74],
        &cat(&[
            seek(&[0x15, 0x49, 0xA9, 0x66], info_at),
            seek(&[0x16, 0x54, 0xAE, 0x6B], tracks_at),
            seek(&[0x12, 0x54, 0xC3, 0x67], tags_at),
        ]),
    );
    assert_eq!(head.len(), head_len);
    let body = cat(&[head, void, info, tracks, chapters.to_vec(), cluster, tags]);
    let mut bytes = el(
        &[0x1A, 0x45, 0xDF, 0xA3],
        &cat(&[
            el(&[0x42, 0x82], b"matroska"),
            el(&[0x42, 0x87], &[4]),
            el(&[0x42, 0x85], &[2]),
        ]),
    );
    bytes.extend([0x18, 0x53, 0x80, 0x67, 0x01]);
    bytes.extend(&(body.len() as u64).to_be_bytes()[1..]);
    bytes.extend(body);
    bytes
}
fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack.windows(needle.len()).position(|w| w == needle)
}
fn cluster() -> Vec<u8> {
    el(
        &CLUSTER,
        &cat(&[el(&[0xE7], &[0]), el(&[0xA3], &samples(400))]),
    )
}
/// The cluster is byte-identical and at the same offset.
fn same_cluster(bytes: &[u8], original: &[u8]) {
    let at = find(original, &cluster()).unwrap();
    assert_eq!(bytes[at..at + cluster().len()], cluster()[..]);
}

/// A Chapters element with one edition: (start in ms, title, language).
fn chapters_element(atoms: &[(u64, &str, &str)]) -> Vec<u8> {
    let mut edition = el(&[0x45, 0xBC], &[0x11]);
    for (index, (start, title, language)) in atoms.iter().enumerate() {
        edition.extend(el(
            &[0xB6],
            &cat(&[
                el(&[0x73, 0xC4], &[0x40 + index as u8]),
                el(&[0x91], &(start * 1_000_000).to_be_bytes()),
                el(&[0x98], &[0]),
                el(
                    &[0x80],
                    &cat(&[
                        el(&[0x85], title.as_bytes()),
                        el(&[0x43, 0x7C], language.as_bytes()),
                    ]),
                ),
            ]),
        ));
    }
    el(&[0x10, 0x43, 0xA7, 0x70], &el(&[0x45, 0xB9], &edition))
}

#[test]
fn matroska_chapters_read_write_and_remove() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("chapters.mka");
    let original = matroska_with(
        false,
        &chapters_element(&[(0, "Intro", "eng"), (1500, "Verse", "ger")]),
    );
    fs::write(&path, &original).unwrap();
    let entry = read_entry(&path).unwrap();
    assert_eq!(
        value(&entry, "CHAPTERS").as_deref(),
        Some("00:00:00.000 Intro|00:00:01.500 Verse")
    );
    assert_eq!(technical(&entry, "_SUBSONG_COUNT"), "2");
    assert_eq!(value(&entry, "TITLE").as_deref(), Some("The Song"));

    // Editing a title keeps the chapter UIDs and languages; the shorthand
    // time is stored canonically and a new chapter is added.
    let written = write_fields(
        &path,
        &fields(&[(
            "CHAPTERS",
            &["0:00 Opening", "00:00:01.5 Verse", "0:02.25   Outro"],
        )]),
    )
    .unwrap();
    assert_eq!(
        value(&written, "CHAPTERS").as_deref(),
        Some("00:00:00.000 Opening|00:00:01.500 Verse|00:00:02.250 Outro")
    );
    assert_eq!(technical(&written, "_SUBSONG_COUNT"), "3");
    assert_eq!(value(&written, "TITLE").as_deref(), Some("The Song"));
    let bytes = fs::read(&path).unwrap();
    same_cluster(&bytes, &original);
    assert!(
        find(&bytes, &el(&[0x73, 0xC4], &[0x40])).is_some(),
        "UID kept"
    );
    assert!(
        find(&bytes, &el(&[0x43, 0x7C], b"ger")).is_some(),
        "language kept"
    );
    // Each chapter ends where the next starts; the last at the duration.
    assert!(find(&bytes, &el(&[0x92], &1_500_000_000u64.to_be_bytes()[4..])).is_some());
    assert!(find(&bytes, &el(&[0x92], &2_500_000_000u64.to_be_bytes()[4..])).is_some());

    // Ordinary tag edits leave the chapters alone.
    let tagged = write_fields(&path, &fields(&[("GENRE", &["Jazz"])])).unwrap();
    assert_eq!(value(&tagged, "CHAPTERS"), value(&written, "CHAPTERS"));

    assert!(write_fields(&path, &fields(&[("CHAPTERS", &["1:61:00 Bad"])])).is_err());
    assert!(write_fields(&path, &fields(&[("CHAPTERS", &["Title only"])])).is_err());

    // Clearing CHAPTERS drops the edition; Remove Tag also drops chapters.
    let cleared = write_fields(&path, &fields(&[("CHAPTERS", &[])])).unwrap();
    assert!(!cleared.fields.contains_key("CHAPTERS"));
    assert_eq!(technical(&cleared, "_SUBSONG_COUNT"), "");
    write_fields(&path, &fields(&[("CHAPTERS", &["00:00:00.000 Again"])])).unwrap();
    let removed = remove_tags(&path).unwrap();
    assert!(removed.fields.is_empty(), "{:?}", removed.fields);
    same_cluster(&fs::read(&path).unwrap(), &original);
}

#[test]
fn chapters_can_be_added_to_matroska_without_any() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("plain.mka");
    let original = matroska(false);
    fs::write(&path, &original).unwrap();
    let written = write_fields(
        &path,
        &fields(&[("CHAPTERS", &["00:00:00.000 Start", "00:00:01.000"])]),
    )
    .unwrap();
    assert_eq!(
        value(&written, "CHAPTERS").as_deref(),
        Some("00:00:00.000 Start|00:00:01.000")
    );
    same_cluster(&fs::read(&path).unwrap(), &original);
}

#[test]
fn matroska_audio_mapping_reads_and_writes_target_levels() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("song.mka");
    let original = matroska(false);
    fs::write(&path, &original).unwrap();
    let entry = read_entry(&path).unwrap();
    assert_eq!(entry.format, "Matroska");
    assert_eq!(value(&entry, "TITLE").as_deref(), Some("The Song"));
    assert_eq!(value(&entry, "ALBUM").as_deref(), Some("The Album"));
    assert_eq!(
        value(&entry, "ALBUMARTIST").as_deref(),
        Some("Album Artist")
    );
    assert_eq!(value(&entry, "COMMENT").as_deref(), Some("hi"));
    assert!(
        !entry.fields.contains_key("DURATION"),
        "track-scoped tags stay hidden"
    );
    assert_eq!(entry.sample_rate, 44100);
    assert_eq!(entry.channels, 2);
    assert!((entry.duration - 2.5).abs() < 1e-9);
    assert_eq!(technical(&entry, "_CODEC"), "PCM");
    assert_eq!(technical(&entry, "_TAG"), "Matroska");

    let written = write_fields(
        &path,
        &fields(&[
            ("ARTIST", &["A1", "A2"]),
            ("TRACK", &["4"]),
            ("YEAR", &["2023"]),
            ("TITLESORT", &["Song, The"]),
            ("MY FIELD", &["custom"]),
            ("COMMENT", &[]),
        ]),
    )
    .unwrap();
    assert_eq!(value(&written, "ARTIST").as_deref(), Some("A1|A2"));
    assert_eq!(value(&written, "TRACK").as_deref(), Some("4"));
    assert_eq!(value(&written, "YEAR").as_deref(), Some("2023"));
    assert_eq!(value(&written, "TITLESORT").as_deref(), Some("Song, The"));
    assert_eq!(value(&written, "MY FIELD").as_deref(), Some("custom"));
    assert_eq!(value(&written, "COMMENT"), None);
    assert_eq!(value(&written, "TITLE").as_deref(), Some("The Song"));
    let bytes = fs::read(&path).unwrap();
    same_cluster(&bytes, &original);
    assert!(find(&bytes, b"DATE_RECORDED").is_some());
    assert!(find(&bytes, b"PART_NUMBER").is_some());
    assert!(find(&bytes, b"DURATION").is_some(), "scoped tag kept");

    let error = write_fields(&path, &fields(&[("PART_NUMBER", &["1"])])).unwrap_err();
    assert!(error.contains("reserved"), "{error}");

    let image = dir.path().join("art.png");
    png(&image);
    let covered = add_cover(&path, &image).unwrap();
    assert_eq!(covered.covers.len(), 1);
    assert_eq!(covered.covers[0].picture_type, "CoverFront");
    assert_eq!(covered.covers[0].mime, "image/png");
    assert!(find(&fs::read(&path).unwrap(), b"cover.png").is_some());
    let two = add_cover(&path, &image).unwrap();
    assert_eq!(two.covers.len(), 2);
    assert!(find(&fs::read(&path).unwrap(), b"cover (2).png").is_some());
    let back = set_cover_properties_summary(&path, Some("CoverBack"), Some("rear"), None).unwrap();
    assert!(back
        .covers
        .iter()
        .all(|c| c.picture_type == "CoverBack" && c.description == "rear"));
    assert!(find(&fs::read(&path).unwrap(), b"back (2).png").is_some());
    let one = remove_cover(&path, 0).unwrap();
    assert_eq!(one.covers.len(), 1);
    assert_eq!(value(&one, "ARTIST").as_deref(), Some("A1|A2"));

    let removed = remove_tags(&path).unwrap();
    assert!(removed.fields.is_empty() && removed.covers.is_empty());
    let bytes = fs::read(&path).unwrap();
    assert!(find(&bytes, b"DURATION").is_some(), "scoped tag kept");
    same_cluster(&bytes, &original);
    // Repeated edits reuse freed space instead of growing without bound.
    for round in 0..5 {
        write_fields(&path, &fields(&[("TITLE", &[&format!("Round {round}")])])).unwrap();
    }
    assert!(fs::read(&path).unwrap().len() < original.len() + 512);
}

#[test]
fn matroska_video_mapping_follows_the_advanced_option() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("movie.mkv");
    fs::write(&path, matroska(true)).unwrap();
    let entry = read_entry(&path).unwrap();
    // Recommended video mapping: TITLE is T=50, T=30 TITLE is unmapped.
    assert_eq!(value(&entry, "TITLE").as_deref(), Some("The Album"));
    assert_eq!(value(&entry, "ARTIST").as_deref(), Some("Album Artist"));
    assert_eq!(value(&entry, "ALBUM"), None);
    let mut options = TagOptions::default();
    options.advanced.matroska_video_mapping = false;
    let audio = tag_core::read_entry_with_options(&path, &[], &options).unwrap();
    assert_eq!(value(&audio, "TITLE").as_deref(), Some("The Song"));
    let written = write_fields(
        &path,
        &fields(&[("TVEPISODE", &["7"]), ("TVSEASON", &["2"])]),
    )
    .unwrap();
    assert_eq!(value(&written, "TVEPISODE").as_deref(), Some("7"));
    assert_eq!(value(&written, "TVSEASON").as_deref(), Some("2"));
}

#[test]
fn scanning_includes_the_new_extensions() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("a.dsf"), dsf(&samples(4096), None)).unwrap();
    fs::write(
        dir.path().join("b.dff"),
        form(&[prop(b"DSD ", &[]), chunk(b"DSD ", &samples(64))]),
    )
    .unwrap();
    fs::write(dir.path().join("c.mka"), matroska(false)).unwrap();
    let result = scan_paths(&[dir.path().to_string_lossy().into_owned()], false);
    assert!(result.errors.is_empty(), "{:?}", result.errors);
    let mut formats: Vec<_> = result.entries.iter().map(|e| e.format.clone()).collect();
    formats.sort();
    assert_eq!(formats, ["DFF", "DSF", "Matroska"]);
}

fn tools() -> bool {
    Command::new("ffmpeg").arg("-version").output().is_ok()
        && Command::new("ffprobe").arg("-version").output().is_ok()
}
fn ffprobe_tags(path: &Path) -> String {
    let output = Command::new("ffprobe")
        .args([
            "-v",
            "error",
            "-show_entries",
            "format_tags:stream_tags",
            "-of",
            "default",
        ])
        .arg(path)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).into_owned()
}
/// Audio and video packet hashes.
fn packets(path: &Path) -> String {
    ["a", "v"]
        .iter()
        .map(|kind| {
            let output = Command::new("ffprobe")
                .args([
                    "-v",
                    "error",
                    "-select_streams",
                    kind,
                    "-show_data_hash",
                    "MD5",
                ])
                .args([
                    "-show_entries",
                    "packet=stream_index,pts,size,data_hash",
                    "-of",
                    "csv",
                ])
                .arg(path)
                .output()
                .unwrap();
            assert!(output.status.success());
            // Attachments appear as timestamp-less packets.
            String::from_utf8_lossy(&output.stdout)
                .lines()
                .filter(|line| !line.contains(",N/A,"))
                .map(|line| format!("{line}\n"))
                .collect::<String>()
        })
        .collect()
}
fn decodes(path: &Path) -> bool {
    let output = Command::new("ffmpeg")
        .args(["-v", "error", "-xerror", "-i"])
        .arg(path)
        .args(["-f", "null", "-"])
        .output()
        .unwrap();
    output.status.success() && output.stderr.is_empty()
}

/// ffprobe's chapter list as "start_time title" lines.
fn ffprobe_chapters(path: &Path) -> String {
    let output = Command::new("ffprobe")
        .args([
            "-v",
            "error",
            "-show_entries",
            "chapter=start_time:chapter_tags=title",
            "-of",
            "csv=p=0",
        ])
        .arg(path)
        .output()
        .unwrap();
    assert!(output.status.success());
    // Windows builds of ffprobe end lines with CRLF.
    String::from_utf8_lossy(&output.stdout).replace("\r\n", "\n")
}

#[test]
fn ffmpeg_matroska_chapters_are_edited_in_place() {
    if !tools() {
        eprintln!("Skipping optional ffmpeg chapter test: ffmpeg and ffprobe are required");
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let metadata = dir.path().join("chapters.txt");
    fs::write(
        &metadata,
        ";FFMETADATA1\n[CHAPTER]\nTIMEBASE=1/1000\nSTART=0\nEND=400\ntitle=One\n\
         [CHAPTER]\nTIMEBASE=1/1000\nSTART=400\nEND=1000\ntitle=Two\n",
    )
    .unwrap();
    let path = dir.path().join("chapters.mka");
    let output = Command::new("ffmpeg")
        .args(["-v", "error", "-f", "lavfi", "-i", "sine=duration=1", "-i"])
        .arg(&metadata)
        .args(["-map", "0", "-map_chapters", "1", "-c:a", "flac"])
        .arg(&path)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let before = packets(&path);
    let entry = read_entry(&path).unwrap();
    assert_eq!(
        value(&entry, "CHAPTERS").as_deref(),
        Some("00:00:00.000 One|00:00:00.400 Two")
    );
    write_fields(
        &path,
        &fields(&[("CHAPTERS", &["00:00:00.000 First", "00:00:00.600 Second"])]),
    )
    .unwrap();
    assert_eq!(ffprobe_chapters(&path), "0.000000,First\n0.600000,Second\n");
    assert!(decodes(&path));
    assert_eq!(packets(&path), before);
    remove_tags(&path).unwrap();
    assert_eq!(ffprobe_chapters(&path), "");
    assert_eq!(packets(&path), before);
}

#[test]
fn ffmpeg_matroska_and_webm_remain_decodable_after_edits() {
    if !tools() {
        eprintln!("Skipping optional ffmpeg Matroska test: ffmpeg and ffprobe are required");
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let image = dir.path().join("cover.png");
    png(&image);
    for (name, codec, extra) in [
        ("audio.mka", "flac", vec![]),
        ("audio.webm", "libopus", vec![]),
        (
            "video.mkv",
            "flac",
            vec!["-f", "lavfi", "-i", "color=blue:s=16x16:d=1"],
        ),
    ] {
        let path = dir.path().join(name);
        let mut command = Command::new("ffmpeg");
        command
            .args([
                "-v",
                "error",
                "-y",
                "-f",
                "lavfi",
                "-i",
                "sine=frequency=440:duration=1",
            ])
            .args(&extra)
            .args([
                "-c:a",
                codec,
                "-metadata",
                "artist=Initial",
                "-metadata",
                "album=Disc",
            ]);
        if name.ends_with(".mka") {
            command
                .arg("-attach")
                .arg(&image)
                .args(["-metadata:s:t", "mimetype=image/png"])
                // ffmpeg on Windows stores the full attach path as the file
                // name; use such a name everywhere so every runner agrees.
                .args(["-metadata:s:t", r"filename=C:\fixtures\cover.png"]);
        }
        if name.ends_with(".mkv") {
            command.args(["-c:v", "ffv1"]);
        }
        let output = command.arg(&path).output().unwrap();
        if !output.status.success() {
            eprintln!(
                "Skipping {name}: {}",
                String::from_utf8_lossy(&output.stderr)
            );
            continue;
        }
        let before = packets(&path);
        assert!(!before.is_empty());
        let entry = read_entry(&path).unwrap();
        assert!(!entry.fields.is_empty(), "{name}: {:?}", entry.fields);
        // ffmpeg writes ALBUM verbatim at the default target (50).
        assert_eq!(value(&entry, "ALBUM").as_deref(), Some("Disc"), "{name}");
        if name.ends_with(".mka") {
            assert_eq!(entry.covers.len(), 1, "{name}");
            assert_eq!(entry.covers[0].picture_type, "CoverFront");
        }
        let written = write_fields(
            &path,
            &fields(&[
                ("TITLE", &["Edited Title"]),
                ("ARTIST", &["Edited Artist"]),
                ("GENRE", &["Jazz"]),
            ]),
        )
        .unwrap();
        assert_eq!(
            value(&written, "TITLE").as_deref(),
            Some("Edited Title"),
            "{name}"
        );
        add_cover(&path, &image).unwrap();
        assert!(decodes(&path), "{name} no longer decodes");
        let tags = ffprobe_tags(&path);
        for expected in ["Edited Title", "Edited Artist", "Jazz"] {
            assert!(
                tags.contains(expected),
                "{name}: ffprobe tags lack {expected}:\n{tags}"
            );
        }
        assert_eq!(packets(&path), before, "{name}: packets changed");
        remove_tags(&path).unwrap();
        assert!(decodes(&path), "{name} no longer decodes after removal");
        assert_eq!(packets(&path), before, "{name}: packets changed");
        let entry = read_entry(&path).unwrap();
        assert!(entry.fields.is_empty() && entry.covers.is_empty(), "{name}");
    }
}

#[test]
fn cover_plans_reorder_and_retype_dsf_and_matroska_covers() {
    use tag_core::{apply_cover_changes_with_options, CoverChange, CoverInfo, CoverRef};
    // Identical covers are told apart by their occurrence.
    let keep = |cover: &CoverInfo, occurrence: usize, picture_type: &str| CoverChange {
        original: Some(CoverRef::of(cover)),
        picture_type: picture_type.into(),
        description: cover.description.clone(),
        occurrence: Some(occurrence),
        ..Default::default()
    };
    let dir = tempfile::tempdir().unwrap();
    let image = dir.path().join("art.png");
    png(&image);
    let dsf_path = dir.path().join("track.dsf");
    fs::write(&dsf_path, dsf(&samples(2 * 4096 * 3), None)).unwrap();
    let mkv_path = dir.path().join("track.mka");
    fs::write(&mkv_path, matroska(false)).unwrap();
    for path in [&dsf_path, &mkv_path] {
        add_cover(path, &image).unwrap();
        let two = add_cover(path, &image).unwrap();
        assert_eq!(two.covers.len(), 2);
        let second = usize::from(CoverRef::of(&two.covers[0]) == CoverRef::of(&two.covers[1]));
        let entry = apply_cover_changes_with_options(
            path,
            &[
                keep(&two.covers[1], second, "CoverBack"),
                keep(&two.covers[0], 0, "CoverFront"),
            ],
            &TagOptions::default(),
        )
        .unwrap();
        let types: Vec<_> = entry
            .covers
            .iter()
            .map(|c| c.picture_type.as_str())
            .collect();
        assert_eq!(types, ["CoverBack", "CoverFront"], "{}", path.display());
        assert_eq!(entry.covers[1].description, two.covers[0].description);
        let none = apply_cover_changes_with_options(path, &[], &TagOptions::default()).unwrap();
        assert!(none.covers.is_empty());
    }
}
