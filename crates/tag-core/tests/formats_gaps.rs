//! Remaining format gaps: file-wide Nero fields and Nero covers in MP4, a
//! `plID` lofty drops, ADTS streams after junk, header-less APEv2 tags and
//! APE tags with "TAG" where ID3v1 starts, and compressed or encrypted ID3v2
//! frames in DSF and DSDIFF. Every fixture is built by hand, and every write
//! is checked byte for byte against the data it must not touch.
use lofty::{
    ape::{ApeItem, ApeTag},
    config::WriteOptions,
    id3::v2::{Frame, FrameId, Id3v2Tag, TextInformationFrame},
    prelude::TagExt,
    tag::ItemValue,
    TextEncoding,
};
use std::{fs, path::Path};
use tag_core::{
    add_cover_bytes_with_options, read_entry, read_entry_with_options,
    remove_all_covers_with_options, write_fields, write_fields_with_options, AudioEntry, Fields,
    TagOptions,
};

fn fields(values: &[(&str, &[&str])]) -> Fields {
    values
        .iter()
        .map(|(k, v)| (k.to_string(), v.iter().map(|s| s.to_string()).collect()))
        .collect()
}
fn value(entry: &AudioEntry, key: &str) -> Option<String> {
    entry
        .fields
        .iter()
        .find(|(name, _)| name.eq_ignore_ascii_case(key))
        .map(|(_, v)| v.join("|"))
}
fn technical(entry: &AudioEntry, key: &str) -> String {
    entry
        .technical
        .get(key)
        .map(|v| v.join("|"))
        .unwrap_or_default()
}
fn contains(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack.windows(needle.len()).position(|w| w == needle)
}
/// Payload bytes below 0xFF: no frame sync and no tag markers.
fn payload(length: usize) -> Vec<u8> {
    (0..length).map(|i| (i * 37 % 89 + 1) as u8).collect()
}
fn png() -> Vec<u8> {
    let mut bytes = std::io::Cursor::new(Vec::new());
    image::RgbImage::from_pixel(3, 2, image::Rgb([200, 10, 10]))
        .write_to(&mut bytes, image::ImageFormat::Png)
        .unwrap();
    bytes.into_inner()
}

// ---------------------------------------------------------------- MP4

fn boxed(kind: &[u8; 4], body: &[u8]) -> Vec<u8> {
    let mut out = ((body.len() + 8) as u32).to_be_bytes().to_vec();
    out.extend(kind);
    out.extend(body);
    out
}
/// A box with version and flags.
fn full(kind: &[u8; 4], flags: u32, body: &[u8]) -> Vec<u8> {
    boxed(kind, &[&flags.to_be_bytes()[..], body].concat())
}
fn words(values: &[u32]) -> Vec<u8> {
    values.iter().flat_map(|v| v.to_be_bytes()).collect()
}
const SAMPLES: usize = 20;
const SAMPLE: usize = 64;
const MATRIX: [u32; 9] = [0x10000, 0, 0, 0, 0x10000, 0, 0, 0, 0x4000_0000];

/// The `moov` of a one-track AAC file whose samples start at `offset`.
fn moov(udta: &[u8], offset: u32) -> Vec<u8> {
    let duration = (SAMPLES * 1024) as u32;
    let mut mvhd = words(&[0, 0, 44_100, duration, 0x10000]);
    mvhd.extend([1, 0]);
    mvhd.extend([0; 10]);
    mvhd.extend(words(&MATRIX));
    mvhd.extend([0; 24]);
    mvhd.extend(words(&[2]));
    let mut tkhd = words(&[0, 0, 1, 0, duration, 0, 0]);
    tkhd.extend([0, 0, 0, 0, 1, 0, 0, 0]);
    tkhd.extend(words(&MATRIX));
    tkhd.extend(words(&[0, 0]));
    let mut mdhd = words(&[0, 0, 44_100, duration]);
    mdhd.extend([0x55, 0xC4, 0, 0]);
    let hdlr = full(b"hdlr", 0, &[&words(&[0])[..], b"soun", &[0; 13]].concat());
    // ES descriptor: AAC LC, 44.1 kHz, stereo.
    let esds = full(
        b"esds",
        0,
        &[
            3, 25, 0, 0, 0, 4, 17, 0x40, 0x15, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 5, 2, 0x12, 0x10,
            6, 1, 2,
        ],
    );
    let mut mp4a = vec![0; 6];
    mp4a.extend([0, 1]);
    mp4a.extend([0; 8]);
    mp4a.extend([0, 2, 0, 16, 0, 0, 0, 0]);
    mp4a.extend((44_100u32 << 16).to_be_bytes());
    mp4a.extend(esds);
    let stbl = boxed(
        b"stbl",
        &[
            full(
                b"stsd",
                0,
                &[&words(&[1])[..], &boxed(b"mp4a", &mp4a)].concat(),
            ),
            full(b"stts", 0, &words(&[1, SAMPLES as u32, 1024])),
            full(b"stsc", 0, &words(&[1, 1, SAMPLES as u32, 1])),
            full(
                b"stsz",
                0,
                &[
                    &words(&[0, SAMPLES as u32])[..],
                    &words(&[SAMPLE as u32; SAMPLES]),
                ]
                .concat(),
            ),
            full(b"stco", 0, &words(&[1, offset])),
        ]
        .concat(),
    );
    let dinf = boxed(
        b"dinf",
        &full(
            b"dref",
            0,
            &[&words(&[1])[..], &full(b"url ", 1, &[])].concat(),
        ),
    );
    let minf = boxed(b"minf", &[full(b"smhd", 0, &[0; 4]), dinf, stbl].concat());
    let mdia = boxed(b"mdia", &[full(b"mdhd", 0, &mdhd), hdlr, minf].concat());
    let trak = boxed(b"trak", &[full(b"tkhd", 7, &tkhd), mdia].concat());
    boxed(
        b"moov",
        &[full(b"mvhd", 0, &mvhd), trak, boxed(b"udta", udta)].concat(),
    )
}
fn audio() -> Vec<u8> {
    payload(SAMPLES * SAMPLE)
}
/// `ftyp`, `moov` with the user data `udta`, and `mdat`.
fn mp4(udta: &[u8]) -> Vec<u8> {
    let ftyp = boxed(b"ftyp", b"M4A \0\0\0\0M4A mp42isom");
    let offset = (ftyp.len() + moov(udta, 0).len() + 8) as u32;
    [ftyp, moov(udta, offset), boxed(b"mdat", &audio())].concat()
}
fn item(kind: &[u8; 4], code: u8, data: &[u8]) -> Vec<u8> {
    boxed(
        kind,
        &boxed(b"data", &[&[0, 0, 0, code, 0, 0, 0, 0][..], data].concat()),
    )
}
/// The iTunes `meta` box with `items`.
fn itunes(items: &[Vec<u8>]) -> Vec<u8> {
    let hdlr = full(
        b"hdlr",
        0,
        &[&words(&[0])[..], b"mdirappl", &[0; 9]].concat(),
    );
    full(
        b"meta",
        0,
        &[hdlr, boxed(b"ilst", &items.concat())].concat(),
    )
}
/// Nero chapters: (start in 100 ns units, title).
fn chpl(chapters: &[(u64, &str)]) -> Vec<u8> {
    let mut body = vec![1, 0, 0, 0, 0, 0, 0, 0, chapters.len() as u8];
    for (start, title) in chapters {
        body.extend(start.to_be_bytes());
        body.push(title.len() as u8);
        body.extend(title.as_bytes());
    }
    boxed(b"chpl", &body)
}
/// A Nero `meta` box of text fields.
fn nero_meta(fields: &[(&str, &str)]) -> Vec<u8> {
    let mut body = (fields.len() as u32).to_be_bytes().to_vec();
    for (name, value) in fields {
        body.extend([0x80, 0, name.len() as u8]);
        body.extend(name.as_bytes());
        body.extend(1u16.to_be_bytes());
        body.extend((value.len() as u32).to_be_bytes());
        body.extend(value.as_bytes());
    }
    boxed(b"meta", &body)
}
fn cvrx(covers: &[(&str, u16, &[u8])]) -> Vec<u8> {
    let mut body = words(&[0, covers.len() as u32]);
    for (mime, kind, data) in covers {
        body.extend((mime.len() as u16).to_be_bytes());
        body.extend(mime.as_bytes());
        body.extend(kind.to_be_bytes());
        body.extend((data.len() as u32).to_be_bytes());
        body.extend(*data);
    }
    boxed(b"cvrx", &body)
}
/// A `udta/indx` box as Mp3tag reads it: a count, then per entry a 64-bit
/// value and a length-prefixed string.
fn indx() -> Vec<u8> {
    let mut body = words(&[1]);
    body.extend(4_000_000u64.to_be_bytes());
    body.extend(words(&[5]));
    body.extend(b"Index");
    boxed(b"indx", &body)
}
/// The body of the `tags` box of `bytes`.
fn tags_box(bytes: &[u8]) -> Vec<u8> {
    let at = contains(bytes, b"tags").unwrap() - 4;
    let size = u32::from_be_bytes(bytes[at..at + 4].try_into().unwrap()) as usize;
    bytes[at + 8..at + size].to_vec()
}
fn same_audio(bytes: &[u8]) {
    let audio = audio();
    let at = contains(bytes, &audio[..256]).expect("audio");
    assert_eq!(&bytes[at..at + audio.len()], audio.as_slice());
}

#[test]
fn hand_built_mp4_reads_like_a_real_one() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("plain.m4a");
    fs::write(&path, mp4(&itunes(&[item(b"\xa9nam", 1, b"Song")]))).unwrap();
    let entry = read_entry(&path).unwrap();
    assert_eq!(entry.format, "MP4");
    assert_eq!(value(&entry, "TITLE").as_deref(), Some("Song"));
    assert_eq!(technical(&entry, "_TAG"), "MP4");
    assert_eq!(entry.sample_rate, 44_100);
}

#[test]
fn file_wide_nero_fields_fill_gaps_and_follow_edits() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("nero.m4b");
    let udta = [
        itunes(&[
            item(b"\xa9nam", 1, b"Song"),
            item(b"\xa9alb", 1, b"iTunes album"),
        ]),
        chpl(&[(0, "One"), (2_000_000, "Two")]),
        boxed(
            b"tags",
            &nero_meta(&[
                ("album", "Nero album"),
                ("artist", "Nero artist"),
                ("genre", "Nero genre"),
            ]),
        ),
        indx(),
    ]
    .concat();
    fs::write(&path, mp4(&udta)).unwrap();
    let entry = read_entry(&path).unwrap();
    // The iTunes tag takes precedence; Nero fills the gaps.
    assert_eq!(value(&entry, "ALBUM").as_deref(), Some("iTunes album"));
    assert_eq!(value(&entry, "ARTIST").as_deref(), Some("Nero artist"));
    assert_eq!(value(&entry, "GENRE").as_deref(), Some("Nero genre"));
    assert_eq!(technical(&entry, "_TAG"), "MP4 Nero");
    assert_eq!(technical(&entry, "_TAG_READ"), "MP4");
    assert_eq!(entry.chapters.len(), 2);

    // An unrelated edit leaves the Nero tags and the index alone.
    write_fields(&path, &fields(&[("COMMENT", &["Note"])])).unwrap();
    let bytes = fs::read(&path).unwrap();
    assert_eq!(tags_box(&bytes), tags_box(&mp4(&udta)));
    assert!(contains(&bytes, &indx()).is_some());

    // Removing a field Nero holds removes it there too, in place.
    let before = bytes.len();
    let written = write_fields(&path, &fields(&[("ARTIST", &[])])).unwrap();
    assert_eq!(value(&written, "ARTIST"), None);
    let bytes = fs::read(&path).unwrap();
    assert_eq!(bytes.len(), before, "the tags box shrank in place");
    assert_eq!(
        tags_box(&bytes),
        nero_meta(&[("album", "Nero album"), ("genre", "Nero genre")])
    );
    assert!(contains(&bytes, b"Nero artist").is_none());
    same_audio(&bytes);
    assert_eq!(value(&read_entry(&path).unwrap(), "ARTIST"), None);

    // An edit too long for the box rewrites it with the chapters.
    let written = write_fields(&path, &fields(&[("GENRE", &["A much longer genre"])])).unwrap();
    assert_eq!(
        value(&written, "GENRE").as_deref(),
        Some("A much longer genre")
    );
    let bytes = fs::read(&path).unwrap();
    assert_eq!(
        tags_box(&bytes),
        nero_meta(&[("album", "Nero album"), ("genre", "A much longer genre")])
    );
    // Fields the iTunes tag holds stay in sync as well.
    write_fields(&path, &fields(&[("ALBUM", &["Shared"])])).unwrap();
    let bytes = fs::read(&path).unwrap();
    assert!(contains(&tags_box(&bytes), b"Shared").is_some());
    assert!(contains(&bytes, &indx()).is_some(), "indx is kept");
    same_audio(&bytes);
    let entry = read_entry(&path).unwrap();
    assert_eq!(value(&entry, "ALBUM").as_deref(), Some("Shared"));
    assert_eq!(entry.chapters.len(), 2);
    assert_eq!(value(&entry, "COMMENT").as_deref(), Some("Note"));
}

#[test]
fn nero_fields_without_chapters_are_removed_in_place() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("flat.m4a");
    let udta = [
        itunes(&[item(b"\xa9nam", 1, b"Song")]),
        boxed(b"tags", &nero_meta(&[("artist", "Nero artist")])),
    ]
    .concat();
    let original = mp4(&udta);
    fs::write(&path, &original).unwrap();
    assert_eq!(
        value(&read_entry(&path).unwrap(), "ARTIST").as_deref(),
        Some("Nero artist")
    );
    write_fields(&path, &fields(&[("ARTIST", &[])])).unwrap();
    assert_eq!(value(&read_entry(&path).unwrap(), "ARTIST"), None);
    let bytes = fs::read(&path).unwrap();
    assert_eq!(bytes.len(), original.len());
    // An empty file-wide set is left out, as when Nero tags are written.
    assert!(tags_box(&bytes).is_empty());
    same_audio(&bytes);
}

#[test]
fn nero_covers_are_shown_and_become_itunes_covers_when_edited() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("covers.m4b");
    let image = png();
    let covers = cvrx(&[("image/png", 3, &image)]);
    let udta = [
        itunes(&[item(b"\xa9nam", 1, b"Song")]),
        chpl(&[(0, "One"), (2_000_000, "Two")]),
        boxed(
            b"tags",
            &[nero_meta(&[("album", "A")]), covers.clone()].concat(),
        ),
    ]
    .concat();
    fs::write(&path, mp4(&udta)).unwrap();
    let entry = read_entry(&path).unwrap();
    assert_eq!(entry.covers.len(), 1);
    assert_eq!(entry.covers[0].mime, "image/png");
    assert_eq!(entry.covers[0].picture_type, "CoverFront");
    assert_eq!(technical(&entry, "_COVERS"), "1");
    assert_eq!(entry.covers[0].width, Some(3));

    // Field edits keep the Nero covers as they are.
    write_fields(&path, &fields(&[("TITLE", &["Edited"])])).unwrap();
    let bytes = fs::read(&path).unwrap();
    assert!(contains(&tags_box(&bytes), &covers).is_some());
    assert!(contains(&bytes, b"covr").is_none());

    // Adding a cover stores both in the iTunes tag and drops the Nero box.
    let options = TagOptions::default();
    let mut second = image::RgbImage::from_pixel(5, 4, image::Rgb([1, 2, 3]));
    second.put_pixel(0, 0, image::Rgb([9, 9, 9]));
    let mut extra = std::io::Cursor::new(Vec::new());
    second
        .write_to(&mut extra, image::ImageFormat::Png)
        .unwrap();
    let written = add_cover_bytes_with_options(
        &path,
        extra.into_inner(),
        "back.png",
        Some("CoverBack"),
        &options,
    )
    .unwrap();
    assert_eq!(written.covers.len(), 2, "{:?}", written.covers);
    let bytes = fs::read(&path).unwrap();
    assert!(contains(&bytes, b"covr").is_some());
    assert!(contains(&bytes, b"cvrx").is_none());
    assert!(
        contains(&bytes, &image).is_some(),
        "the Nero cover moved to covr"
    );
    assert!(contains(&tags_box(&bytes), b"album").is_some());
    same_audio(&bytes);

    // Removing every cover leaves none behind.
    let removed = remove_all_covers_with_options(&path, &options).unwrap();
    assert!(removed.covers.is_empty());
    assert!(read_entry(&path).unwrap().covers.is_empty());
}

#[test]
fn nero_covers_are_also_removed_when_shown_alone() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("alone.m4b");
    let image = png();
    let udta = [
        itunes(&[item(b"\xa9nam", 1, b"Song")]),
        chpl(&[(0, "One"), (2_000_000, "Two")]),
        boxed(b"tags", &cvrx(&[("", 4, &image)])),
    ]
    .concat();
    fs::write(&path, mp4(&udta)).unwrap();
    let entry = read_entry(&path).unwrap();
    // Without a MIME type the bytes tell the format; type 4 is the back cover.
    assert_eq!(entry.covers.len(), 1);
    assert_eq!(entry.covers[0].mime, "image/png");
    assert_eq!(entry.covers[0].picture_type, "CoverBack");
    let removed = remove_all_covers_with_options(&path, &TagOptions::default()).unwrap();
    assert!(removed.covers.is_empty());
    let bytes = fs::read(&path).unwrap();
    assert!(contains(&bytes, &image).is_none());
    same_audio(&bytes);
}

#[test]
fn a_plid_lofty_drops_is_read_and_kept() {
    use lofty::mp4::{AtomIdent, Mp4File};
    use lofty::prelude::AudioFile;
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("plid.m4a");
    let plid = item(b"plID", 21, &[1, 2, 3, 4]);
    fs::write(
        &path,
        mp4(&itunes(&[item(b"\xa9nam", 1, b"Song"), plid.clone()])),
    )
    .unwrap();
    // lofty reads the file strictly without complaint but loses the item.
    let file = Mp4File::read_from(
        &mut fs::File::open(&path).unwrap(),
        lofty::config::ParseOptions::new().parsing_mode(lofty::config::ParsingMode::Strict),
    )
    .unwrap();
    assert!(file
        .ilst()
        .unwrap()
        .get(&AtomIdent::Fourcc(*b"plID"))
        .is_none());

    let entry = read_entry(&path).unwrap();
    assert_eq!(value(&entry, "ITUNESALBUMID").as_deref(), Some("16909060"));
    write_fields(&path, &fields(&[("TITLE", &["Edited"])])).unwrap();
    let bytes = fs::read(&path).unwrap();
    assert!(contains(&bytes, &plid).is_some(), "kept byte for byte");
    let entry = read_entry(&path).unwrap();
    assert_eq!(value(&entry, "TITLE").as_deref(), Some("Edited"));
    assert_eq!(value(&entry, "ITUNESALBUMID").as_deref(), Some("16909060"));
    // Editing it writes the documented 8-byte form.
    write_fields(&path, &fields(&[("ITUNESALBUMID", &["77"])])).unwrap();
    let bytes = fs::read(&path).unwrap();
    assert!(contains(&bytes, &plid).is_none());
    assert_eq!(
        value(&read_entry(&path).unwrap(), "ITUNESALBUMID").as_deref(),
        Some("77")
    );
}

// ---------------------------------------------------------------- ADTS, APE

/// `count` ADTS frames (MPEG-4 AAC LC, 44.1 kHz, stereo, no CRC).
fn adts(count: usize, body: usize) -> Vec<u8> {
    let length = 7 + body;
    let mut stream = Vec::new();
    for _ in 0..count {
        stream.extend([
            0xFF,
            0xF1,
            (1 << 6) | (4 << 2),
            (2 << 6) | ((length >> 11) & 3) as u8,
            ((length >> 3) & 0xFF) as u8,
            (((length & 7) << 5) | 0x1F) as u8,
            0xFC,
        ]);
        stream.extend(payload(body));
    }
    stream
}
/// Twenty MPEG-1 Layer III frames (128 kbit/s, 44.1 kHz).
fn mpeg_frames() -> Vec<u8> {
    let mut frame = vec![0u8; 417];
    frame[..4].copy_from_slice(&[0xFF, 0xFB, 0x90, 0x00]);
    frame.repeat(20)
}
/// One WavPack block: 16-bit stereo at 44.1 kHz holding `samples` samples.
fn wavpack(samples: u32, body: usize) -> Vec<u8> {
    let mut block = b"wvpk".to_vec();
    block.extend(((24 + body) as u32).to_le_bytes());
    block.extend(0x410u16.to_le_bytes());
    block.extend([0, 0]);
    block.extend(samples.to_le_bytes());
    block.extend(0u32.to_le_bytes());
    block.extend(samples.to_le_bytes());
    let flags: u32 = 1 | 0x800 | 0x1000 | (9 << 23);
    block.extend(flags.to_le_bytes());
    block.extend(0u32.to_le_bytes());
    block.extend(payload(body));
    block
}
fn id3_bytes(title: &str) -> Vec<u8> {
    let mut tag = Id3v2Tag::new();
    tag.insert(Frame::Text(TextInformationFrame::new(
        FrameId::new("TIT2").unwrap(),
        TextEncoding::UTF8,
        title.to_owned(),
    )));
    let mut bytes = Vec::new();
    tag.dump_to(&mut bytes, WriteOptions::default()).unwrap();
    bytes
}
fn ape_bytes(items: &[(&str, &str)]) -> Vec<u8> {
    let mut tag = ApeTag::new();
    for (key, value) in items {
        tag.insert(ApeItem::new(key.to_string(), ItemValue::Text(value.to_string())).unwrap());
    }
    let mut bytes = Vec::new();
    tag.dump_to(&mut bytes, WriteOptions::default()).unwrap();
    bytes
}
/// An APEv2 tag with a footer and no header.
fn headerless_ape(items: &[(&str, &str)]) -> Vec<u8> {
    let mut body = Vec::new();
    for (key, value) in items {
        body.extend((value.len() as u32).to_le_bytes());
        body.extend(0u32.to_le_bytes());
        body.extend(key.as_bytes());
        body.push(0);
        body.extend(value.as_bytes());
    }
    let mut footer = b"APETAGEX".to_vec();
    for word in [2000, body.len() as u32 + 32, items.len() as u32, 0] {
        footer.extend(word.to_le_bytes());
    }
    footer.extend([0; 8]);
    [body, footer].concat()
}
fn ape_only() -> TagOptions {
    let mut options = TagOptions::default();
    options.mpeg.write_ape = true;
    options.mpeg.remove_ape = false;
    options.mpeg.write_id3v1 = false;
    options.mpeg.write_id3v2 = false;
    options.mpeg.remove_id3v2 = false;
    options
}
fn id3v2_only() -> TagOptions {
    let mut options = TagOptions::default();
    options.mpeg.write_ape = false;
    options.mpeg.remove_ape = false;
    options.mpeg.write_id3v1 = false;
    options
}

#[test]
fn adts_after_junk_is_a_raw_aac_stream() {
    let dir = tempfile::tempdir().unwrap();
    let stream = adts(40, 120);
    let mut junk = vec![0u8; 300];
    junk.extend(b"junk before the first frame");
    let mut bytes = id3_bytes("From ID3v2");
    bytes.extend(&junk);
    bytes.extend(&stream);
    bytes.extend(ape_bytes(&[("Album", "From APE")]));
    let path = dir.path().join("junk.aac");
    fs::write(&path, &bytes).unwrap();

    let entry = read_entry(&path).unwrap();
    assert_eq!(entry.format, "AAC");
    assert_eq!(technical(&entry, "_CODEC"), "AAC");
    assert_eq!((entry.sample_rate, entry.channels), (44_100, 2));
    assert_eq!(value(&entry, "ALBUM").as_deref(), Some("From APE"));
    let tags = technical(&entry, "_TAG");
    assert!(tags.contains("APE") && tags.contains("ID3v2"), "{tags}");

    // The APE/ID3v1 policy applies; junk and frames are kept as they are.
    let mut options = TagOptions::default();
    options.mpeg.write_ape = true;
    options.mpeg.remove_ape = false;
    write_fields_with_options(
        &path,
        &fields(&[("ARTIST", &["Band"])]),
        false,
        &[],
        &options,
    )
    .unwrap();
    let after = fs::read(&path).unwrap();
    let audio = [junk.as_slice(), stream.as_slice()].concat();
    let at = contains(&after, &audio).expect("junk and stream kept together");
    assert!(after[..at].starts_with(b"ID3"));
    let entry = read_entry_with_options(&path, &[], &options).unwrap();
    assert_eq!(value(&entry, "ARTIST").as_deref(), Some("Band"));
    assert_eq!(value(&entry, "ALBUM").as_deref(), Some("From APE"));
    let tags = technical(&entry, "_TAG");
    assert!(tags.contains("ID3v1") && tags.contains("APE"), "{tags}");
}

#[test]
fn headerless_apev2_tags_are_read_and_written_without_losing_audio() {
    let dir = tempfile::tempdir().unwrap();
    let ape = headerless_ape(&[("Title", "APE title"), ("Artist", "APE artist")]);
    for (name, stream) in [
        ("track.mp3", mpeg_frames()),
        ("track.wv", wavpack(44_100, 2000)),
    ] {
        let path = dir.path().join(name);
        fs::write(&path, [stream.as_slice(), &ape].concat()).unwrap();
        let entry = read_entry(&path).unwrap();
        assert_eq!(
            value(&entry, "TITLE").as_deref(),
            Some("APE title"),
            "{name}"
        );
        assert_eq!(technical(&entry, "_TAG_READ"), "APEv2", "{name}");
        assert!(entry.duration > 0.0, "{name}");

        // A write that leaves APE alone keeps it byte for byte.
        if name.ends_with(".mp3") {
            write_fields_with_options(
                &path,
                &fields(&[("ALBUM", &["In ID3v2"])]),
                false,
                &[],
                &id3v2_only(),
            )
            .unwrap();
            let after = fs::read(&path).unwrap();
            assert!(
                after.ends_with(&[stream.as_slice(), &ape].concat()),
                "{name}"
            );
            assert!(after.starts_with(b"ID3"));
            fs::write(&path, [stream.as_slice(), &ape].concat()).unwrap();
        }

        // Rewriting the APE tag keeps every audio byte before it.
        write_fields_with_options(
            &path,
            &fields(&[("TITLE", &["New"])]),
            false,
            &[],
            &ape_only(),
        )
        .unwrap_or_else(|e| panic!("{name}: {e}"));
        let after = fs::read(&path).unwrap();
        assert!(after.starts_with(&stream), "{name}: audio kept");
        assert_eq!(
            &after[stream.len()..stream.len() + 8],
            b"APETAGEX",
            "{name}"
        );
        let entry = read_entry_with_options(&path, &[], &ape_only()).unwrap();
        assert_eq!(value(&entry, "TITLE").as_deref(), Some("New"), "{name}");
        assert_eq!(
            value(&entry, "ARTIST").as_deref(),
            Some("APE artist"),
            "{name}"
        );
    }
}

#[test]
fn tag_inside_ape_items_is_not_taken_for_id3v1() {
    let dir = tempfile::tempdir().unwrap();
    // A COMMENT item whose value puts "TAG" 128 bytes before the end.
    let mut comment = "x".repeat(150);
    comment.replace_range(54..57, "TAG");
    let ape = ape_bytes(&[("Comment", &comment)]);
    let stream = mpeg_frames();
    let original = [stream.as_slice(), &ape].concat();
    assert_eq!(
        &original[original.len() - 128..original.len() - 125],
        b"TAG"
    );
    let path = dir.path().join("collide.mp3");
    fs::write(&path, &original).unwrap();

    let entry = read_entry(&path).unwrap();
    assert_eq!(value(&entry, "COMMENT").as_deref(), Some(comment.as_str()));
    assert!(!technical(&entry, "_TAG").contains("ID3v1"));
    assert_eq!(technical(&entry, "_TAG_READ"), "APEv2");

    // ID3v2 writes leave the APE tag byte for byte.
    write_fields_with_options(
        &path,
        &fields(&[("ALBUM", &["A"])]),
        false,
        &[],
        &id3v2_only(),
    )
    .unwrap();
    let after = fs::read(&path).unwrap();
    assert!(after.ends_with(&original));

    // APE writes keep the comment and the audio.
    fs::write(&path, &original).unwrap();
    write_fields_with_options(
        &path,
        &fields(&[("TITLE", &["T"])]),
        false,
        &[],
        &ape_only(),
    )
    .unwrap();
    let after = fs::read(&path).unwrap();
    assert!(after.starts_with(&stream));
    let entry = read_entry_with_options(&path, &[], &ape_only()).unwrap();
    assert_eq!(value(&entry, "COMMENT").as_deref(), Some(comment.as_str()));
    assert_eq!(value(&entry, "TITLE").as_deref(), Some("T"));

    // An ID3v1 tag is added after the APE tag, which stays intact.
    fs::write(&path, &original).unwrap();
    let mut options = id3v2_only();
    options.mpeg.write_id3v1 = true;
    write_fields_with_options(&path, &fields(&[("TITLE", &["V1"])]), false, &[], &options).unwrap();
    let after = fs::read(&path).unwrap();
    assert_eq!(
        &after[after.len() - 128 - ape.len()..after.len() - 128],
        ape.as_slice()
    );
    assert_eq!(&after[after.len() - 128..after.len() - 125], b"TAG");
    let entry = read_entry_with_options(&path, &[], &options).unwrap();
    let tags = technical(&entry, "_TAG");
    assert!(tags.contains("ID3v1") && tags.contains("APE"), "{tags}");
    assert_eq!(value(&entry, "COMMENT").as_deref(), Some(comment.as_str()));
}

#[test]
fn a_131_byte_ape_tag_keeps_its_header() {
    let ape = ape_bytes(&[("encoder", &"L".repeat(51))]);
    assert_eq!(ape.len(), 131);
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("track.mp3");
    let stream = mpeg_frames();
    let original = [stream.as_slice(), &ape].concat();
    fs::write(&path, &original).unwrap();
    // Writes that do not touch APE no longer strip its header.
    write_fields_with_options(
        &path,
        &fields(&[("ALBUM", &["A"])]),
        false,
        &[],
        &id3v2_only(),
    )
    .unwrap();
    assert!(fs::read(&path).unwrap().ends_with(&original));
    assert_eq!(
        value(&read_entry(&path).unwrap(), "ENCODER"),
        Some("L".repeat(51))
    );
}

// ---------------------------------------------------------------- DSF, DFF

fn syncsafe(value: usize) -> [u8; 4] {
    [
        ((value >> 21) & 127) as u8,
        ((value >> 14) & 127) as u8,
        ((value >> 7) & 127) as u8,
        (value & 127) as u8,
    ]
}
/// zlib data holding `data` in one stored (uncompressed) deflate block.
fn zlib(data: &[u8]) -> Vec<u8> {
    let mut bytes = vec![0x78, 0x01, 0x01];
    let length = data.len() as u16;
    bytes.extend(length.to_le_bytes());
    bytes.extend((!length).to_le_bytes());
    bytes.extend(data);
    let (mut a, mut b) = (1u32, 0u32);
    for byte in data {
        a = (a + u32::from(*byte)) % 65521;
        b = (b + a) % 65521;
    }
    bytes.extend(((b << 16) | a).to_be_bytes());
    bytes
}
fn raw_frame(major: u8, id: &[u8; 4], flags: [u8; 2], body: &[u8]) -> Vec<u8> {
    let mut bytes = id.to_vec();
    if major == 3 {
        bytes.extend((body.len() as u32).to_be_bytes());
    } else {
        bytes.extend(syncsafe(body.len()));
    }
    bytes.extend(flags);
    bytes.extend(body);
    bytes
}
fn raw_tag(major: u8, frames: &[Vec<u8>], padding: usize) -> Vec<u8> {
    let body = frames.concat();
    let mut bytes = b"ID3".to_vec();
    bytes.extend([major, 0, 0]);
    bytes.extend(syncsafe(body.len() + padding));
    bytes.extend(body);
    bytes.resize(bytes.len() + padding, 0);
    bytes
}
/// A v2.3 tag with a plain title, a zlib-compressed album and an encrypted
/// TXXX frame, and those two frames.
fn flagged_tag() -> (Vec<u8>, Vec<u8>, Vec<u8>) {
    let album = b"\x00Compressed album";
    let mut compressed_body = (album.len() as u32).to_be_bytes().to_vec();
    compressed_body.extend(zlib(album));
    let compressed = raw_frame(3, b"TALB", [0, 0x80], &compressed_body);
    let encrypted = raw_frame(3, b"TXXX", [0, 0x40], b"\x81\x13\x37opaque cipher bytes");
    let title = raw_frame(3, b"TIT2", [0, 0], b"\x00Plain title");
    let tag = raw_tag(3, &[title, compressed.clone(), encrypted.clone()], 64);
    (tag, compressed, encrypted)
}
fn dsf(data: &[u8], id3: &[u8]) -> Vec<u8> {
    let mut bytes = b"DSD ".to_vec();
    bytes.extend(28u64.to_le_bytes());
    bytes.extend(0u64.to_le_bytes());
    bytes.extend(0u64.to_le_bytes());
    bytes.extend(b"fmt ");
    bytes.extend(52u64.to_le_bytes());
    for value in [1u32, 0, 2, 2, 2_822_400, 1] {
        bytes.extend(value.to_le_bytes());
    }
    bytes.extend((data.len() as u64 * 4).to_le_bytes());
    bytes.extend(4096u32.to_le_bytes());
    bytes.extend(0u32.to_le_bytes());
    bytes.extend(b"data");
    bytes.extend((12 + data.len() as u64).to_le_bytes());
    bytes.extend(data);
    let pointer = bytes.len() as u64;
    bytes[20..28].copy_from_slice(&pointer.to_le_bytes());
    bytes.extend(id3);
    let total = bytes.len() as u64;
    bytes[12..20].copy_from_slice(&total.to_le_bytes());
    bytes
}
fn chunk(id: &[u8; 4], data: &[u8]) -> Vec<u8> {
    let mut bytes = id.to_vec();
    bytes.extend((data.len() as u64).to_be_bytes());
    bytes.extend(data);
    if data.len() % 2 == 1 {
        bytes.push(0);
    }
    bytes
}
fn dff(sound: &[u8], id3: &[u8]) -> Vec<u8> {
    let mut prop = b"SND ".to_vec();
    prop.extend(chunk(b"FS  ", &2_822_400u32.to_be_bytes()));
    prop.extend(chunk(
        b"CHNL",
        &[0, 2, b'S', b'L', b'F', b'T', b'S', b'R', b'G', b'T'],
    ));
    prop.extend(chunk(b"CMPR", b"DSD \x0enot compressed"));
    let mut body = b"DSD ".to_vec();
    body.extend(chunk(b"FVER", &[1, 5, 0, 0]));
    body.extend(chunk(b"PROP", &prop));
    body.extend(chunk(b"DSD ", sound));
    body.extend(chunk(b"ID3 ", id3));
    let mut bytes = b"FRM8".to_vec();
    bytes.extend((body.len() as u64).to_be_bytes());
    bytes.extend(body);
    bytes
}

#[test]
fn dsd_files_keep_compressed_and_encrypted_id3v2_frames() {
    let dir = tempfile::tempdir().unwrap();
    let sound: Vec<u8> = (0..2 * 4096 * 2).map(|i| (i * 37 % 251) as u8).collect();
    let (tag, compressed, encrypted) = flagged_tag();
    for (name, bytes) in [("a.dsf", dsf(&sound, &tag)), ("b.dff", dff(&sound, &tag))] {
        let path = dir.path().join(name);
        fs::write(&path, &bytes).unwrap();
        let entry = read_entry(&path).unwrap();
        assert_eq!(
            value(&entry, "TITLE").as_deref(),
            Some("Plain title"),
            "{name}"
        );
        assert_eq!(
            value(&entry, "ALBUM").as_deref(),
            Some("Compressed album"),
            "{name}"
        );

        // An unrelated edit copies both frames as stored.
        write_fields(&path, &fields(&[("ARTIST", &["Someone"])]))
            .unwrap_or_else(|e| panic!("{name}: {e}"));
        let after = fs::read(&path).unwrap();
        assert!(contains(&after, &compressed).is_some(), "{name}");
        assert!(contains(&after, &encrypted).is_some(), "{name}");
        assert!(contains(&after, &sound).is_some(), "{name}: sound kept");
        let entry = read_entry(&path).unwrap();
        assert_eq!(
            value(&entry, "ARTIST").as_deref(),
            Some("Someone"),
            "{name}"
        );
        assert_eq!(
            value(&entry, "ALBUM").as_deref(),
            Some("Compressed album"),
            "{name}"
        );

        // Editing the compressed field writes it plain; the encrypted frame stays.
        write_fields(&path, &fields(&[("ALBUM", &["New album"])])).unwrap();
        let after = fs::read(&path).unwrap();
        assert!(contains(&after, &compressed).is_none(), "{name}");
        assert!(contains(&after, &encrypted).is_some(), "{name}");
        assert_eq!(
            value(&read_entry(&path).unwrap(), "ALBUM").as_deref(),
            Some("New album"),
            "{name}"
        );
        assert!(Path::new(&path).exists());
    }
}
