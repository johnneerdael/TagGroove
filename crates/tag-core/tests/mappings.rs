use lofty::{
    config::{ParseOptions, WriteOptions},
    file::AudioFile,
    id3::v2::{Frame, FrameId, Id3v2Tag, TextInformationFrame},
    mp4::{AtomIdent, Mp4File},
    ogg::tag::VorbisComments,
    prelude::TagExt,
    TextEncoding,
};
use std::{fs, path::Path, process::Command};
use tag_core::{
    read_entry, read_entry_summary, read_entry_with_mappings, write_fields,
    write_fields_with_mappings, Fields, UserFieldMapping,
};

fn values(items: &[(&str, &[&str])]) -> Fields {
    items
        .iter()
        .map(|(k, v)| (k.to_string(), v.iter().map(|s| s.to_string()).collect()))
        .collect()
}
fn wav(path: &Path) {
    let mut b = Vec::new();
    b.extend(b"RIFF");
    b.extend(16036u32.to_le_bytes());
    b.extend(b"WAVEfmt ");
    b.extend(16u32.to_le_bytes());
    b.extend(1u16.to_le_bytes());
    b.extend(1u16.to_le_bytes());
    b.extend(8000u32.to_le_bytes());
    b.extend(16000u32.to_le_bytes());
    b.extend(2u16.to_le_bytes());
    b.extend(16u16.to_le_bytes());
    b.extend(b"data");
    b.extend(16000u32.to_le_bytes());
    b.extend(vec![0u8; 16000]);
    fs::write(path, b).unwrap();
}
fn encoded(path: &Path, codec: &str) -> bool {
    if Command::new("ffmpeg").arg("-version").output().is_err() {
        eprintln!("ffmpeg absent: skipping optional encoded mapping fixture");
        return false;
    }
    let source = path.with_extension("wav");
    wav(&source);
    let r = Command::new("ffmpeg")
        .args(["-v", "error", "-y", "-i"])
        .arg(source)
        .args(["-c:a", codec])
        .arg(path)
        .output()
        .unwrap();
    assert!(r.status.success(), "{}", String::from_utf8_lossy(&r.stderr));
    true
}

#[test]
fn vorbis_keeps_native_dates_numbering_and_field_names_independent() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("names.flac");
    if !encoded(&path, "flac") {
        return;
    }
    let native = values(&[
        ("DATE", &["2024-05-06"]),
        ("YEAR", &["1999"]),
        ("TRACKNUMBER", &["03/012"]),
        ("TRACK", &["A1"]),
        ("DISCTOTAL", &["4"]),
        ("TOTALDISCS", &["9"]),
        ("CONTENTGROUP", &["A"]),
        ("GROUPING", &["B"]),
        ("lower_case_custom", &["one", "two"]),
    ]);
    let mut tag = VorbisComments::new();
    for (k, vs) in &native {
        for v in vs {
            tag.push(k.clone(), v.clone());
        }
    }
    tag.save_to_path(&path, WriteOptions::default()).unwrap();
    assert_eq!(read_entry(&path).unwrap().fields, native);
    assert_eq!(read_entry_summary(&path).unwrap().fields, native);
    write_fields(
        &path,
        &values(&[("YEAR", &["2000"]), ("DATE", &["2025"]), ("GROUPING", &[])]),
    )
    .unwrap();
    let entry = read_entry(&path).unwrap();
    assert_eq!(entry.fields["YEAR"], ["2000"]);
    assert_eq!(entry.fields["DATE"], ["2025"]);
    assert_eq!(entry.fields["TRACKNUMBER"], ["03/012"]);
    assert_eq!(entry.fields["TRACK"], ["A1"]);
    assert_eq!(entry.fields["CONTENTGROUP"], ["A"]);
    assert!(!entry.fields.contains_key("GROUPING"));
}

#[test]
fn ape_keeps_native_year_track_disc_and_custom_name_spelling() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("names.wv");
    if !encoded(&path, "wavpack") {
        return;
    }
    let native = values(&[
        ("Year", &["2024"]),
        ("DATE", &["1999"]),
        ("Track", &["03/012"]),
        ("Disc", &["02/04"]),
        ("TITLE", &["Native"]),
        ("lower_case_custom", &["one", "two"]),
    ]);
    let mut tag = lofty::ape::ApeTag::new();
    for (k, v) in &native {
        tag.insert(
            lofty::ape::ApeItem::new(k.clone(), lofty::tag::ItemValue::Text(v.join("\0"))).unwrap(),
        );
    }
    tag.save_to_path(&path, WriteOptions::default()).unwrap();
    assert_eq!(read_entry(&path).unwrap().fields, native);
    write_fields(
        &path,
        &values(&[("Year", &["2000"]), ("TITLE", &["Edited"])]),
    )
    .unwrap();
    let entry = read_entry(&path).unwrap();
    assert_eq!(entry.fields["Year"], ["2000"]);
    assert_eq!(entry.fields["DATE"], ["1999"]);
    assert_eq!(entry.fields["Track"], ["03/012"]);
    assert_eq!(entry.fields["Disc"], ["02/04"]);
    assert!(!entry.fields.contains_key("TOTALDISCS"));
}

#[test]
fn id3_documented_fields_use_exact_frame_ids_and_txxx_descriptions() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("id3.wav");
    wav(&path);
    let patch = values(&[
        ("CONTENTGROUP", &["Content"]),
        ("GROUPING", &["iTunes group"]),
        ("ENCODERSETTINGS", &["Encoder settings"]),
        ("ENCODER", &["Custom encoder"]),
        ("BPM", &["123"]),
        ("DATE", &["2503"]),
        ("YEAR", &["2024-03-25"]),
        ("ACOUSTID_ID", &["id-123"]),
        ("MUSICBRAINZ_ALBUMID", &["album-id"]),
        ("TRACK", &["03/012"]),
        ("DISCNUMBER", &["02/04"]),
        ("TOTALTRACKS", &["custom total"]),
    ]);
    write_fields(&path, &patch).unwrap();
    let entry = read_entry(&path).unwrap();
    assert_eq!(entry.fields, patch);
    let native = lofty::iff::wav::WavFile::read_from(
        &mut fs::File::open(&path).unwrap(),
        ParseOptions::new().implicit_conversions(false),
    )
    .unwrap();
    let tag = native.id3v2().unwrap();
    for (field, id, value) in [
        ("CONTENTGROUP", "TIT1", "Content"),
        ("GROUPING", "GRP1", "iTunes group"),
        ("ENCODERSETTINGS", "TSSE", "Encoder settings"),
        ("BPM", "TBPM", "123"),
        ("TRACK", "TRCK", "03/012"),
        ("DISCNUMBER", "TPOS", "02/04"),
    ] {
        assert_eq!(
            tag.get_text(&FrameId::new(id).unwrap()),
            Some(value),
            "{field}"
        );
    }
    assert_eq!(tag.get_user_text("Acoustid Id"), Some("id-123"));
    assert_eq!(tag.get_user_text("MusicBrainz Album Id"), Some("album-id"));
    assert_eq!(tag.get_user_text("ENCODER"), Some("Custom encoder"));
    assert_eq!(tag.get_user_text("DATE"), Some("2503"));
}

#[test]
fn id3_v23_version_and_separate_date_frame_are_preserved() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("v23.wav");
    wav(&path);
    let mut tag = Id3v2Tag::new();
    for (id, value) in [
        ("TYER", "2024"),
        ("TDAT", "2503"),
        ("TSSE", "Native encoder"),
    ] {
        tag.insert(Frame::Text(TextInformationFrame::new(
            FrameId::new(id).unwrap(),
            TextEncoding::UTF16,
            value,
        )));
    }
    tag.save_to_path(&path, WriteOptions::default().use_id3v23(true))
        .unwrap();
    let fields = read_entry(&path).unwrap().fields;
    assert_eq!(fields["YEAR"], ["2024"]);
    assert_eq!(fields["DATE"], ["2503"]);
    assert_eq!(fields["ENCODERSETTINGS"], ["Native encoder"]);
    write_fields(&path, &values(&[("DATE", &["0101"]), ("YEAR", &["2025"])])).unwrap();
    let native = lofty::iff::wav::WavFile::read_from(
        &mut fs::File::open(&path).unwrap(),
        ParseOptions::new().implicit_conversions(false),
    )
    .unwrap();
    assert_eq!(
        native.id3v2().unwrap().original_version(),
        lofty::id3::v2::Id3v2Version::V3
    );
    let fields = read_entry(&path).unwrap().fields;
    assert_eq!(fields["DATE"], ["0101"]);
    assert_eq!(fields["YEAR"], ["2025"]);
}

#[test]
fn mp4_number_errors_name_the_raw_field_without_changing_the_file() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("invalid-number.m4a");
    if !encoded(&path, "aac") {
        return;
    }
    let original = fs::read(&path).unwrap();
    for field in ["TRACK", "TOTALTRACKS", "DISCNUMBER", "TOTALDISCS"] {
        let error = write_fields(&path, &values(&[(field, &["not a number"])])).unwrap_err();
        assert_eq!(error, format!("{field} requires a 16-bit unsigned number"));
        assert_eq!(fs::read(&path).unwrap(), original);
    }
}

#[test]
fn mp4_freeform_names_flags_and_track_atoms_match_documented_storage() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("mp4.m4a");
    if !encoded(&path, "aac") {
        return;
    }
    let patch = values(&[
        ("ACOUSTID_ID", &["id-123"]),
        ("MUSICBRAINZ_ALBUMID", &["album-id"]),
        ("CONTENTGROUP", &["content"]),
        ("GROUPING", &["custom group"]),
        ("COMPILATION", &["1"]),
        ("ITUNESGAPLESS", &["1"]),
        ("ITUNESHDVIDEO", &["0"]),
        ("ITUNESADVISORY", &["2"]),
        ("ITUNESALBUMID", &["1099511627776"]),
        ("TRACK", &["03/12"]),
        ("DISCNUMBER", &["02/4"]),
        ("----:com.vendor.Tag:FieldName", &["one", "two"]),
    ]);
    write_fields(&path, &patch).unwrap();
    let native =
        Mp4File::read_from(&mut fs::File::open(&path).unwrap(), ParseOptions::new()).unwrap();
    let tag = native.ilst().unwrap();
    for name in ["Acoustid Id", "MusicBrainz Album Id"] {
        assert!(
            tag.get(&AtomIdent::Freeform {
                mean: "com.apple.iTunes".into(),
                name: name.into()
            })
            .is_some(),
            "{name}"
        );
    }
    for id in [
        *b"cpil", *b"pgap", *b"hdvd", *b"rtng", *b"plID", *b"trkn", *b"disk",
    ] {
        assert!(tag.get(&AtomIdent::Fourcc(id)).is_some());
    }
    let fields = read_entry(&path).unwrap().fields;
    assert_eq!(fields["TRACK"], ["3/12"]);
    assert_eq!(fields["TOTALTRACKS"], ["12"]);
    assert_eq!(fields["DISCNUMBER"], ["2/4"]);
    assert_eq!(fields["TOTALDISCS"], ["4"]);
    assert_eq!(fields["ITUNESALBUMID"], ["1099511627776"]);
    assert_eq!(fields["----:com.vendor.Tag:FieldName"], ["one", "two"]);
    assert_eq!(fields["GROUPING"], ["custom group"]);
    write_fields(&path, &values(&[("TITLE", &["Other edit"])])).unwrap();
    assert_eq!(
        read_entry(&path).unwrap().fields["----:com.vendor.Tag:FieldName"],
        ["one", "two"]
    );
}

#[test]
fn id3_ratings_credits_timestamps_lyrics_and_musicbrainz_uid_use_native_frames() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("special.wav");
    wav(&path);
    let patch = values(&[
        ("POPULARIMETER", &["listener@example.org|201|17"]),
        ("RATING MM", &["**-"]),
        ("RATING WMP", &["4"]),
        (
            "INVOLVEDPEOPLE",
            &["Producer:Person One;Engineer:Person Two;"],
        ),
        ("MUSICIANCREDITS", &["Drums:Player;"]),
        ("ENCODINGTIME", &["2024-03-25T12:34:56"]),
        ("TAGGINGTIME", &["2024-03-25"]),
        ("RELEASETIME", &["2024-03"]),
        ("ORIGYEAR", &["2001"]),
        ("UNSYNCEDLYRICS", &["eng||Actual lyrics"]),
        (
            "MUSICBRAINZ_TRACKID",
            &["01234567-89ab-cdef-0123-456789abcdef"],
        ),
        ("MOVEMENT", &["3"]),
        ("MOVEMENTTOTAL", &["12"]),
        ("COMPILATION", &["1"]),
        ("PODCAST", &["1"]),
    ]);
    write_fields(&path, &patch).unwrap();
    let entry = read_entry(&path).unwrap();
    assert_eq!(entry.fields["RATING MM"], ["2.5"]);
    assert_eq!(entry.fields["RATING WMP"], ["4"]);
    assert_eq!(
        entry.fields["POPULARIMETER"],
        ["listener@example.org|201|17"]
    );
    assert_eq!(entry.fields["MOVEMENT"], ["3"]);
    assert_eq!(entry.fields["MOVEMENTTOTAL"], ["12"]);
    assert_eq!(entry.fields["UNSYNCEDLYRICS"], ["eng||Actual lyrics"]);
    let file = lofty::iff::wav::WavFile::read_from(
        &mut fs::File::open(&path).unwrap(),
        ParseOptions::new().implicit_conversions(false),
    )
    .unwrap();
    let native = file.id3v2().unwrap();
    let popms: Vec<_> = native
        .into_iter()
        .filter_map(|f| {
            if let Frame::Popularimeter(p) = f {
                Some((p.email.as_ref(), p.rating, p.counter))
            } else {
                None
            }
        })
        .collect();
    assert!(popms.contains(&("listener@example.org", 201, 17)));
    assert!(popms.contains(&("no@email", 118, 0)));
    assert!(popms.contains(&("Windows Media Player 9 Series", 196, 0)));
    assert_eq!(
        native.get_text(&FrameId::new("MVIN").unwrap()),
        Some("3/12")
    );
    assert!(native.get(&FrameId::new("TIPL").unwrap()).is_some());
    assert!(native.get(&FrameId::new("TMCL").unwrap()).is_some());
    assert!(native.into_iter().any(|f|matches!(f,Frame::UniqueFileIdentifier(v) if v.owner=="http://musicbrainz.org"&&v.identifier.as_ref()==b"01234567-89ab-cdef-0123-456789abcdef")));
    let before = fs::read(&path).unwrap();
    assert!(write_fields(&path, &values(&[("COMPILATION", &["true"])])).is_err());
    assert!(write_fields(&path, &values(&[("RATING WMP", &["2.5"])])).is_err());
    assert!(write_fields(&path, &values(&[("TAGGINGTIME", &["not-a-timestamp"])])).is_err());
    assert_eq!(fs::read(&path).unwrap(), before);
}

#[test]
fn opt_in_mappings_reverse_writes_to_every_source_and_reject_one_to_many_rules() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("mapped.flac");
    if !encoded(&path, "flac") {
        return;
    }
    let rule = |source: &str, target: &str| UserFieldMapping {
        tag_type: "VorbisComment".into(),
        source: source.into(),
        target: target.into(),
    };
    let rules = vec![rule("DATE", "YEAR"), rule("RELEASE", "YEAR")];
    write_fields(
        &path,
        &values(&[
            ("DATE", &["2024"]),
            ("RELEASE", &["2025"]),
            ("TRACKNUMBER", &["01"]),
        ]),
    )
    .unwrap();
    let raw = read_entry(&path).unwrap();
    assert!(!raw.fields.contains_key("YEAR"));
    let mapped = read_entry_with_mappings(&path, &rules).unwrap();
    assert_eq!(mapped.fields["YEAR"], ["2024", "2025"]);
    assert!(!mapped.fields.contains_key("DATE"));
    assert!(!mapped.fields.contains_key("RELEASE"));
    let saved = write_fields_with_mappings(&path, &values(&[("YEAR", &["2026"])]), &rules).unwrap();
    assert_eq!(saved.fields["YEAR"], ["2026"]);
    let raw = read_entry(&path).unwrap();
    assert_eq!(raw.fields["DATE"], ["2026"]);
    assert_eq!(raw.fields["RELEASE"], ["2026"]);
    assert!(!raw.fields.contains_key("YEAR"));
    assert_eq!(raw.fields["TRACKNUMBER"], ["01"]);
    let before = fs::read(&path).unwrap();
    let invalid = vec![rule("DATE", "YEAR"), rule("DATE", "OTHER")];
    assert!(read_entry_with_mappings(&path, &invalid).is_err());
    assert!(write_fields_with_mappings(&path, &values(&[("YEAR", &["2030"])]), &invalid).is_err());
    assert_eq!(fs::read(&path).unwrap(), before);
}

#[test]
fn id3_numeric_genre_is_displayed_as_standard_name_without_reencoding_untouched_frame() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("genre.wav");
    wav(&path);
    let mut tag = Id3v2Tag::new();
    let id = FrameId::new("TCON").unwrap();
    tag.insert(Frame::Text(TextInformationFrame::new(
        id.clone(),
        TextEncoding::UTF8,
        "(17)",
    )));
    tag.save_to_path(&path, WriteOptions::default()).unwrap();
    assert_eq!(read_entry(&path).unwrap().fields["GENRE"], ["Rock"]);
    write_fields(&path, &values(&[("TITLE", &["Other change"])])).unwrap();
    let file = lofty::iff::wav::WavFile::read_from(
        &mut fs::File::open(&path).unwrap(),
        ParseOptions::new(),
    )
    .unwrap();
    assert_eq!(file.id3v2().unwrap().get_text(&id), Some("(17)"));
    let entry = write_fields(&path, &values(&[("GENRE", &["17", "RX", "CR"])])).unwrap();
    assert_eq!(entry.fields["GENRE"], ["Rock", "Remix", "Cover"]);
}

#[test]
fn mp4_all_documented_media_type_names_encode_their_native_stik_values() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("types.m4a");
    if !encoded(&path, "aac") {
        return;
    }
    for (name, code) in [
        ("Normal", 1),
        ("Audiobook", 2),
        ("Bookmark", 5),
        ("Music Video", 6),
        ("Movie", 9),
        ("TV Show", 10),
        ("Booklet", 11),
        ("Ringtone", 14),
        ("Podcast", 21),
        ("iTunes U", 23),
    ] {
        let result = write_fields(&path, &values(&[("ITUNESMEDIATYPE", &[name])])).unwrap();
        assert_eq!(result.fields["ITUNESMEDIATYPE"], [name]);
        let file =
            Mp4File::read_from(&mut fs::File::open(&path).unwrap(), ParseOptions::new()).unwrap();
        let value = file
            .ilst()
            .unwrap()
            .get(&AtomIdent::Fourcc(*b"stik"))
            .unwrap()
            .data()
            .next()
            .unwrap();
        assert!(matches!(value,lofty::mp4::AtomData::SignedInteger(v) if *v==code));
    }
    let before = fs::read(&path).unwrap();
    assert!(write_fields(
        &path,
        &values(&[("ITUNESMEDIATYPE", &["Not a media type"])])
    )
    .is_err());
    assert_eq!(fs::read(&path).unwrap(), before);
}

fn representative(field: &str) -> &'static str {
    match field {
        "DATE" => "2503",
        "YEAR" | "ORIGYEAR" => "2024",
        "ENCODINGTIME" | "TAGGINGTIME" | "RELEASETIME" => "2024-03-25T12:34:56",
        "COMPILATION" | "PODCAST" | "ITUNESGAPLESS" | "SHOWMOVEMENT" => "1",
        "ITUNESHDVIDEO" => "0",
        "ITUNESADVISORY" => "2",
        "ITUNESMEDIATYPE" => "Normal",
        "BPM" => "123",
        "LENGTH" => "1000",
        "MOVEMENT" => "3",
        "MOVEMENTTOTAL" => "12",
        "TOTALTRACKS" => "12",
        "TOTALDISCS" => "4",
        "TRACK" => "03/12",
        "DISCNUMBER" => "02/4",
        "RATING MM" => "2.5",
        "RATING WMP" => "4",
        "POPULARIMETER" => "test@example.org|128|7",
        "INVOLVEDPEOPLE" | "MUSICIANCREDITS" => "Producer:Person;",
        "UNSYNCEDLYRICS" => "eng||Lyrics",
        "ITUNESALBUMID" => "1099511627776",
        "ITUNESARTISTID" | "ITUNESCATALOGID" | "ITUNESCOMPOSERID" | "ITUNESCOUNTRYID"
        | "ITUNESGENREID" | "TVEPISODE" | "TVSEASON" | "RATE" => "123",
        field if field.starts_with("WWW") || field == "PODCASTURL" => "https://example.org/",
        _ => "Example value",
    }
}

#[test]
fn every_documented_fixed_id3_mapping_has_a_real_native_codec_in_both_versions() {
    let temp = tempfile::tempdir().unwrap();
    let mut failures = Vec::new();
    let mut tested = 0;
    for version in [3, 4] {
        let base = temp.path().join(format!("base-{version}.wav"));
        wav(&base);
        let mut tag = Id3v2Tag::new();
        tag.insert(Frame::Text(TextInformationFrame::new(
            FrameId::new("TIT2").unwrap(),
            TextEncoding::UTF16,
            "Seed",
        )));
        tag.save_to_path(&base, WriteOptions::default().use_id3v23(version == 3))
            .unwrap();
        for mapping in tag_core::FIELD_MAPPINGS {
            let native = if version == 3 {
                mapping.id3v23
            } else {
                mapping.id3v24
            };
            if native.is_empty() || mapping.field == "UNIQUEFILEID" {
                continue;
            }
            let path = temp.path().join("one.wav");
            fs::copy(&base, &path).unwrap();
            let value = representative(mapping.field);
            if let Err(error) = write_fields(&path, &values(&[(mapping.field, &[value])])) {
                failures.push(format!(
                    "v2.{version} {} -> {native}: {error}",
                    mapping.field
                ));
                continue;
            }
            let parsed = lofty::iff::wav::WavFile::read_from(
                &mut fs::File::open(&path).unwrap(),
                ParseOptions::new().implicit_conversions(false),
            )
            .unwrap();
            let tag = parsed.id3v2().unwrap();
            let expected_id = if native.starts_with("TXXX:") {
                "TXXX"
            } else if native.starts_with("UFID:") {
                "UFID"
            } else {
                native
            };
            if !tag.into_iter().any(|f| f.id().as_str() == expected_id) {
                failures.push(format!(
                    "v2.{version} {}: missing native {expected_id}",
                    mapping.field
                ));
            }
            tested += 1;
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
    assert!(tested >= 140, "unexpectedly narrow sweep: {tested}");
}

#[test]
fn every_documented_fixed_mp4_mapping_has_a_real_native_codec() {
    let temp = tempfile::tempdir().unwrap();
    let base = temp.path().join("base.m4a");
    if !encoded(&base, "aac") {
        return;
    }
    let path = temp.path().join("one.m4a");
    let mut failures = Vec::new();
    let mut tested = 0;
    for mapping in tag_core::FIELD_MAPPINGS {
        if mapping.mp4.is_empty() {
            continue;
        }
        fs::copy(&base, &path).unwrap();
        let value = representative(mapping.field);
        if let Err(error) = write_fields(&path, &values(&[(mapping.field, &[value])])) {
            failures.push(format!("{} -> {}: {error}", mapping.field, mapping.mp4));
            continue;
        }
        let parsed =
            Mp4File::read_from(&mut fs::File::open(&path).unwrap(), ParseOptions::new()).unwrap();
        let tag = parsed.ilst().unwrap();
        let native = mapping.mp4.split(" | ").next().unwrap();
        let expected = if let Some(rest) = native.strip_prefix("----:") {
            let (mean, name) = rest.split_once(':').unwrap();
            AtomIdent::Freeform {
                mean: mean.to_owned().into(),
                name: name.to_owned().into(),
            }
        } else {
            let native = if mapping.field == "ARTIST" {
                "©ART"
            } else {
                native
            };
            AtomIdent::Fourcc(
                native
                    .chars()
                    .map(|c| c as u8)
                    .collect::<Vec<_>>()
                    .try_into()
                    .unwrap(),
            )
        };
        if tag.get(&expected).is_none() {
            failures.push(format!("{}: missing native {expected}", mapping.field));
        }
        tested += 1;
    }
    assert!(tested >= 60, "unexpectedly narrow sweep: {tested}");
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn every_documented_riff_mapping_preserves_unknown_info_chunks_and_audio() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("riff.wav");
    wav(&path);
    let original = fs::read(&path).unwrap();
    let mut native = lofty::iff::wav::RiffInfoList::new();
    native.insert("IART".into(), "Seed".into());
    native.insert("XABC".into(), "Opaque text".into());
    native.save_to_path(&path, WriteOptions::default()).unwrap();
    let mut count = 0;
    for mapping in tag_core::FIELD_MAPPINGS
        .iter()
        .filter(|m| !m.riff.is_empty())
    {
        let value = representative(mapping.field);
        write_fields(&path, &values(&[(mapping.field, &[value])])).unwrap();
        let parsed = lofty::iff::wav::WavFile::read_from(
            &mut fs::File::open(&path).unwrap(),
            ParseOptions::new(),
        )
        .unwrap();
        let tag = parsed.riff_info().unwrap();
        assert_eq!(tag.get(mapping.riff), Some(value), "{}", mapping.field);
        assert_eq!(tag.get("XABC"), Some("Opaque text"));
        count += 1;
    }
    assert_eq!(count, 9);
    assert_eq!(&fs::read(path).unwrap()[44..16044], &original[44..16044]);
}

#[test]
fn v23_unicode_urls_credits_and_opaque_frames_survive_repeated_staged_edits() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("v23.wav");
    wav(&path);
    let original = fs::read(&path).unwrap();
    let mut tag = Id3v2Tag::new();
    tag.insert(Frame::Binary(lofty::id3::v2::BinaryFrame::new(
        FrameId::new("XABC").unwrap(),
        vec![0, 255, 224, 17],
    )));
    tag.save_to_path(&path, WriteOptions::default().use_id3v23(true))
        .unwrap();
    write_fields(
        &path,
        &values(&[
            ("WWW", &["https://example.org/path"]),
            ("INVOLVEDPEOPLE", &["制作:名前;Producer:Person;"]),
            ("ALBUMSORT", &["順序"]),
            ("RELEASETIME", &["2024-03-25T12:34:56"]),
        ]),
    )
    .unwrap();
    write_fields(&path, &values(&[("TITLE", &["Edited twice"])])).unwrap();
    let fields = read_entry(&path).unwrap().fields;
    assert_eq!(fields["WWW"], ["https://example.org/path"]);
    assert_eq!(fields["INVOLVEDPEOPLE"], ["制作:名前;Producer:Person;"]);
    assert_eq!(fields["ALBUMSORT"], ["順序"]);
    let parsed = lofty::iff::wav::WavFile::read_from(
        &mut fs::File::open(&path).unwrap(),
        ParseOptions::new().implicit_conversions(false),
    )
    .unwrap();
    let tag = parsed.id3v2().unwrap();
    assert_eq!(tag.original_version(), lofty::id3::v2::Id3v2Version::V3);
    assert!(tag.into_iter().any(|f| matches!(f,Frame::Binary(b) if b.id().as_str()=="XABC" && b.data.as_ref()==[0,255,224,17])));
    assert_eq!(&fs::read(path).unwrap()[44..16044], &original[44..16044]);
}
