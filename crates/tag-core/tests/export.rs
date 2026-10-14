use tag_core::{
    encode_export_document, render_export, ExportOptions, ExportRow, Fields, ScriptColumn,
};

fn row(
    path: &str,
    artist: &str,
    album: &str,
    title: &str,
    track: &str,
    duration: f64,
    size: u64,
) -> ExportRow {
    let fields: Fields = [
        ("ARTIST".into(), vec![artist.into()]),
        ("ALBUM".into(), vec![album.into()]),
        ("TITLE".into(), vec![title.into()]),
        ("TRACK".into(), vec![track.into()]),
    ]
    .into();
    ExportRow::new(fields, path, duration, size)
}
fn options() -> ExportOptions {
    ExportOptions {
        filename: "/out/report.txt".into(),
        locale: Some("en-US".into()),
        ..Default::default()
    }
}
fn rows() -> Vec<ExportRow> {
    vec![
        row("/music/b.mp3", "A", "Z", "Second", "2", 20., 200),
        row("/music/a.mp3", "A", "Z", "First", "1", 10., 100),
        row("/music/c.mp3", "B", "Y", "Third", "1", 30., 300),
    ]
}

#[test]
fn csv_export_sorts_and_keeps_exact_template_line_endings() {
    let docs = render_export(
        "$filename(csv,utf-8)Title;Artist\r\n$loop(%_filename_ext%)%title%;%artist%\r\n$loopend()",
        &rows(),
        &options(),
    )
    .unwrap();
    assert_eq!(docs.len(), 1);
    assert_eq!(docs[0].filename, "/out/report.csv");
    assert_eq!(docs[0].encoding, "utf-8");
    assert_eq!(
        docs[0].text,
        "Title;Artist\r\nFirst;A\r\nSecond;A\r\nThird;B\r\n"
    );
}

#[test]
fn unsorted_loop_and_per_key_limits_do_not_become_global_limits() {
    assert_eq!(
        render_export("$loop(1)%title%;$loopend()", &rows(), &options()).unwrap()[0].text,
        "Second;First;Third;"
    );
    assert_eq!(
        render_export(
            "$loop(%artist%,1)%artist%;$loopend()%_max_counter%/%_total_files%",
            &rows(),
            &options()
        )
        .unwrap()[0]
            .text,
        "A;B;2/3"
    );
    assert_eq!(
        render_export("$loop(%artist%,2)%title%;$loopend()", &rows(), &options()).unwrap()[0].text,
        "Second;First;Third;"
    );
}

#[test]
fn nested_loops_form_groups_and_restore_outer_counters() {
    let rows = vec![
        row("/z.mp3", "A", "Z", "Z1", "1", 10., 100),
        row("/y.mp3", "A", "Z", "Z2", "2", 20., 200),
        row("/x.mp3", "A", "X", "X1", "1", 30., 300),
        row("/b.mp3", "B", "Z", "B1", "1", 40., 400),
    ];
    let text=render_export("$loop(%artist%)%_counter%:%artist%{$loop(%album%,1)%album%:%_total_files%;$loopend()}%_counter%'['%_max_counter%/%_total_files%']'|$loopend()%_max_counter%/%_total_files%",&rows,&options()).unwrap().remove(0).text;
    assert_eq!(text, "1:A{X:1;Z:2;}1[2/3]|2:B{Z:1;}2[1/1]|2/4");
}

#[test]
fn variables_work_inside_normal_functions_and_retain_typed_flags() {
    let text=render_export("$puts(n,0)$puts(flag,$eql(a,b))$if($get(flag),bad,good):$loop(1)$put(n,$add($get(n),1))=%title%;$loopend()/$get(n)",&rows(),&options()).unwrap().remove(0).text;
    assert_eq!(text, "good:1=Second;2=First;3=Third;/3");
    assert_eq!(
        render_export(
            "$puts(v,'literal')[bad$get(v)]$puts(v,%artist%)[good$get(v)]",
            &rows(),
            &options()
        )
        .unwrap()[0]
            .text,
        "goodA"
    );
}

#[test]
fn loops_can_occur_inside_conditionals_without_stale_counter_snapshots() {
    let text=render_export("$loop(%artist%)$loop(%artist%,1)$loopend()$ifgreater(%_total_files%,1,'D:'$loop(%_path%)%title%';'$loopend(),)$loopend()",&rows(),&options()).unwrap().remove(0).text;
    assert_eq!(text, "D:First;Second;");
}

#[test]
fn global_totals_cover_all_source_rows_even_when_unique_output_is_limited() {
    let text=render_export("$loop(%artist%,1)$loopend()%_max_counter%|%_total_files%|%_total_size_raw%|%_total_time_raw%|%_total_time%|%_total_time2%",&rows(),&options()).unwrap().remove(0).text;
    assert_eq!(text, "2|3|600|60|00:01:00|00:01:00");
    let rows = vec![row("/a", "A", "X", "T", "1", 90061., 1_048_576)];
    let text = render_export(
        "%_total_time%|%_total_time2%|%_total_size%",
        &rows,
        &options(),
    )
    .unwrap()
    .remove(0)
    .text;
    assert_eq!(text, "1d 01:01:01|25:01:01|1.00 MB");
}

#[test]
fn partitioned_exports_evaluate_filename_per_group_and_reset_variables() {
    let options = ExportOptions {
        partition: Some("%artist%".into()),
        ..options()
    };
    let docs=render_export("$filename(%artist%.txt,utf-8)$puts(n,0)$loop(1)$puts(n,$add($get(n),1))%title%;$loopend()$get(n)",&rows(),&options).unwrap();
    assert_eq!(docs.len(), 2);
    assert_eq!(docs[0].filename, "/out/A.txt");
    assert_eq!(docs[0].text, "Second;First;2");
    assert_eq!(docs[1].filename, "/out/B.txt");
    assert_eq!(docs[1].text, "Third;1");
}

#[test]
fn explicit_dialog_filename_override_and_encoding_bom_are_separate() {
    let options = ExportOptions {
        filename_override: Some("/chosen/final.txt".into()),
        bom: true,
        ..options()
    };
    let doc = render_export("$filename(ignored.csv,utf-16)猫", &rows(), &options)
        .unwrap()
        .remove(0);
    assert_eq!(doc.filename, "/chosen/final.txt");
    assert_eq!(doc.encoding, "utf-16");
    assert!(doc.bom);
    assert_eq!(
        encode_export_document(&doc, None).unwrap(),
        vec![0xff, 0xfe, 0x2b, 0x73]
    );
    let doc = render_export("$filename(,ansi)Björk 猫", &rows(), &options)
        .unwrap()
        .remove(0);
    assert!(!doc.bom);
    assert_eq!(
        encode_export_document(&doc, Some("windows-1252")).unwrap(),
        b"Bj\xf6rk ?"
    );
}

#[test]
fn relative_paths_follow_actual_output_location_and_keep_other_drives_absolute() {
    let text = render_export(
        "%_filename_rel%|%_folderpath_rel%",
        &rows()[1..2],
        &options(),
    )
    .unwrap()
    .remove(0)
    .text;
    assert_eq!(text, "../music/a.mp3|../music/");
    let rows = vec![
        row("C:\\music\\a.mp3", "A", "X", "T", "1", 1., 1),
        row("D:\\music\\b.mp3", "B", "Y", "T", "1", 1., 1),
    ];
    let options = ExportOptions {
        filename: "C:\\out\\list.txt".into(),
        ..options()
    };
    let text = render_export("$loop(1)%_filename_rel%;$loopend()", &rows, &options)
        .unwrap()
        .remove(0)
        .text;
    assert_eq!(text, "..\\music\\a.mp3;D:\\music\\b.mp3;");
}

#[test]
fn verbatim_drive_export_destinations_keep_the_windows_namespace_and_long_paths() {
    let long_folder = "音楽".repeat(100);
    let base = format!(r"\\?\D:\{long_folder}\{long_folder}\exports");
    assert!(base.encode_utf16().count() > 260);
    let options = ExportOptions {
        filename: r"reports\..\list.txt".into(),
        base_directory: base.clone(),
        ..options()
    };
    let document = render_export("contents", &rows(), &options)
        .unwrap()
        .remove(0);
    assert_eq!(document.filename, format!(r"{base}\list.txt"));
    assert!(!document.filename.contains('/'));

    let options = ExportOptions {
        filename: r"\\?\D:\out\list.txt".into(),
        ..ExportOptions::default()
    };
    assert_eq!(
        render_export("contents", &rows(), &options).unwrap()[0].filename,
        r"\\?\D:\out\list.txt"
    );
}

#[test]
fn ordinary_and_verbatim_drive_paths_share_the_same_relative_export_volume() {
    let rows = vec![
        row(r"\\?\c:\music\a.mp3", "A", "X", "T", "1", 1., 1),
        row(r"\\?\D:\music\b.mp3", "B", "Y", "T", "1", 1., 1),
    ];
    let options = ExportOptions {
        filename: r"C:\out\list.txt".into(),
        ..options()
    };
    assert_eq!(
        render_export(
            "$loop(1)%_filename_rel%|%_folderpath_rel%;$loopend()",
            &rows,
            &options
        )
        .unwrap()[0]
            .text,
        r"..\music\a.mp3|..\music\;\\?\D:\music\b.mp3|\\?\D:\music\;"
    );
    let options = ExportOptions {
        filename: r"\\?\C:\out\list.txt".into(),
        ..ExportOptions::default()
    };
    let rows = vec![row(r"c:\music\a.mp3", "A", "X", "T", "1", 1., 1)];
    assert_eq!(
        render_export("%_filename_rel%", &rows, &options).unwrap()[0].text,
        r"..\music\a.mp3"
    );
}

#[test]
fn verbatim_unc_export_names_and_relative_placeholders_preserve_share_identity() {
    let rows = vec![
        row(r"\\server\share\music\a.mp3", "A", "X", "T", "1", 1., 1),
        row(r"\\?\UNC\server\other\b.mp3", "B", "Y", "T", "1", 1., 1),
    ];
    let options = ExportOptions {
        filename: r"\\?\UNC\SERVER\SHARE\out\list.txt".into(),
        ..options()
    };
    let document = render_export(
        "$loop(1)%_filename_rel%|%_folderpath_rel%;$loopend()",
        &rows,
        &options,
    )
    .unwrap()
    .remove(0);
    assert_eq!(document.filename, r"\\?\UNC\SERVER\SHARE\out\list.txt");
    assert_eq!(
        document.text,
        r"..\music\a.mp3|..\music\;\\?\UNC\server\other\b.mp3|\\?\UNC\server\other\;"
    );
    let rows = vec![row(
        r"\\?\UNC\server\share\music\a.mp3",
        "A",
        "X",
        "T",
        "1",
        1.,
        1,
    )];
    let options = ExportOptions {
        filename: r"\\SERVER\SHARE\out\list.txt".into(),
        ..ExportOptions::default()
    };
    assert_eq!(
        render_export("%_filename_rel%", &rows, &options).unwrap()[0].text,
        r"..\music\a.mp3"
    );
}

#[test]
fn filename_directives_and_row_folder_placeholders_keep_verbatim_destinations() {
    let rows = vec![row(r"\\?\D:\music\a.mp3", "A", "X", "T", "1", 1., 1)];
    let options = ExportOptions {
        filename: "%_folderpath%export.txt".into(),
        ..options()
    };
    let document = render_export(
        "$filename('lists\\report.txt')%_filename_rel%",
        &rows,
        &options,
    )
    .unwrap()
    .remove(0);
    assert_eq!(document.filename, r"\\?\D:\music\lists\report.txt");
    assert_eq!(document.text, r"..\a.mp3");
}

#[test]
fn hash_placeholders_require_real_supplied_values_including_meta_access() {
    assert!(render_export("%_crc%", &rows(), &options()).is_err());
    assert!(render_export("$meta(_md5)", &rows(), &options()).is_err());
    let mut rows = rows();
    rows[0]
        .context_fields
        .insert("_CRC".into(), vec!["DEADBEEF".into()]);
    assert_eq!(
        render_export("%_crc%", &rows, &options()).unwrap()[0].text,
        "DEADBEEF"
    );
    assert_eq!(
        render_export("'%_crc%'", &rows, &options()).unwrap()[0].text,
        "%_crc%"
    );
}

#[test]
fn raw_tags_cannot_impersonate_file_hashes_even_through_dynamic_meta() {
    let mut rows = rows();
    rows[0].fields.insert(
        "_MD5".into(),
        vec!["0123456789abcdef0123456789abcdef".into()],
    );
    let template = "$puts(hash,'_MD5')$meta($get(hash))";
    assert!(render_export(template, &rows, &options()).is_err());
    assert_eq!(
        render_export("$list(,'=',';')", &rows, &options()).unwrap()[0].text,
        "ALBUM=Z;ARTIST=A;TITLE=Second;TRACK=2;_MD5=0123456789abcdef0123456789abcdef;"
    );
    rows[0].context_fields.insert(
        "_MD5".into(),
        vec!["FEDCBA9876543210FEDCBA9876543210".into()],
    );
    assert_eq!(
        render_export(template, &rows, &options()).unwrap()[0].text,
        "FEDCBA9876543210FEDCBA9876543210"
    );
}

#[test]
fn script_columns_and_csv_or_html_escaping_use_normal_scripting() {
    let mut rows = rows();
    rows[0]
        .fields
        .insert("TITLE".into(), vec!["a\"b<猫>".into()]);
    rows[0].columns = Some(vec![ScriptColumn {
        name: "Title".into(),
        value: "a\"b<猫>".into(),
    }]);
    let text=render_export(r#"$columnNames('<','>')'"'$replace(%title%,'"','""')'"'|$replace(%title%,'<','&lt;','>','&gt;')"#,&rows,&options()).unwrap().remove(0).text;
    assert_eq!(text, "<Title>\"a\"\"b<猫>\"|a\"b&lt;猫&gt;");
}

#[test]
fn escaped_export_literals_and_empty_selection_are_supported() {
    assert_eq!(
        render_export("100%% $$ '$loop(x)'", &[], &options()).unwrap()[0].text,
        "100% $ $loop(x)"
    );
    assert_eq!(
        render_export(
            "$loop(1)%title%$loopend()%_total_files%/%_max_counter%",
            &[],
            &options()
        )
        .unwrap()[0]
            .text,
        "0/0"
    );
}

#[test]
fn malformed_templates_invalid_rows_and_colliding_partitions_fail_before_writes() {
    for template in [
        "$loop(1)x",
        "$loopend()",
        "$filename(a)$filename(b)",
        "x$filename(a)",
        "$get()",
        "$puts(x)",
        "$filename(a,b,c)",
        "$filename(a,utf-32)",
        "$not_known(x)",
    ] {
        assert!(
            render_export(template, &rows(), &options()).is_err(),
            "{template}"
        );
    }
    let options = ExportOptions {
        partition: Some("%artist%".into()),
        filename_override: Some("/same.txt".into()),
        ..options()
    };
    assert!(render_export("x", &rows(), &options).is_err());
    let invalid = vec![row("/a", "A", "A", "A", "1", f64::NAN, 1)];
    assert!(render_export("x", &invalid, &options).is_err());
}

#[test]
fn sort_counter_expressions_reverse_the_current_input_order() {
    let text = render_export(
        "$loop($sub(999,%_counter%))%_counter%=%title%;$loopend()",
        &rows(),
        &options(),
    )
    .unwrap()
    .remove(0)
    .text;
    assert_eq!(text, "1=Third;2=First;3=Second;");
}

#[test]
fn large_equal_key_loop_retains_every_row_and_counter_in_input_order() {
    let count = 40_000;
    let rows: Vec<_> = (1..=count)
        .map(|index| {
            row(
                &format!("/music/{index}.mp3"),
                "Same artist",
                "Same album",
                &format!("Track {index}"),
                &index.to_string(),
                1.,
                1,
            )
        })
        .collect();
    let expected: String = (1..=count)
        .map(|index| format!("{index}:Track {index};"))
        .chain(std::iter::once(format!("{count}/{count}")))
        .collect();
    let started = std::time::Instant::now();
    let documents = render_export(
        "$loop(%artist%)%_counter%:%title%;$loopend()%_max_counter%/%_total_files%",
        &rows,
        &options(),
    )
    .unwrap();
    eprintln!("Equal-key export: {count} rows in {:?}", started.elapsed());
    assert_eq!(documents.len(), 1);
    assert_eq!(documents[0].text, expected);
}

#[test]
fn natural_sort_does_not_collapse_distinct_number_spellings_as_duplicates() {
    let rows = vec![
        row("/a", "A", "X", "10", "10", 1., 1),
        row("/b", "A", "X", "2", "2", 1., 1),
        row("/c", "A", "X", "1", "1", 1., 1),
        row("/d", "A", "X", "01", "01", 1., 1),
    ];
    let options = ExportOptions {
        natural_sort: true,
        ..options()
    };
    assert_eq!(
        render_export("$loop(%track%,1)%track%;$loopend()", &rows, &options).unwrap()[0].text,
        "01;1;2;10;"
    );
}

#[test]
fn fallback_filenames_are_generating_expressions_and_paths_are_normalized() {
    let options = ExportOptions {
        filename: "/out/%artist%/../%album%.txt".into(),
        ..options()
    };
    assert_eq!(
        render_export("x", &rows(), &options).unwrap()[0].filename,
        "/out/Z.txt"
    );
}

#[test]
fn mte_decoding_supports_boms_and_explicit_legacy_codepages() {
    assert_eq!(
        tag_core::decode_export_template(b"\xef\xbb\xbf$loop(1)\xe7\x8c\xab$loopend()", None)
            .unwrap(),
        "$loop(1)猫$loopend()"
    );
    assert_eq!(
        tag_core::decode_export_template(&[0xff, 0xfe, 0x2b, 0x73], None).unwrap(),
        "猫"
    );
    assert_eq!(
        tag_core::decode_export_template(&[0xfe, 0xff, 0x73, 0x2b], None).unwrap(),
        "猫"
    );
    assert_eq!(
        tag_core::decode_export_template(b"Bj\xf6rk", Some("1252")).unwrap(),
        "Björk"
    );
    assert!(tag_core::decode_export_template(b"Bj\xf6rk", None).is_err());
}

#[test]
fn isolated_apostrophe_literals_keep_exact_escaping_semantics() {
    assert_eq!(
        render_export("''", &rows(), &options()).unwrap()[0].text,
        "'"
    );
    let mut rows = rows();
    rows[0].fields.insert("TITLE".into(), vec!["it's".into()]);
    assert_eq!(
        render_export("$replace(%title%,'',)", &rows, &options()).unwrap()[0].text,
        "its"
    );
}

#[test]
fn static_hash_requests_include_filename_partition_and_meta_without_raw_literals() {
    let options = ExportOptions {
        filename: "/out/%_crc%.txt".into(),
        partition: Some("%_md5audio%".into()),
        ..options()
    };
    assert_eq!(
        tag_core::requested_hash_fields("'%_crc%'$meta(_md5)", &options).unwrap(),
        vec!["_CRC", "_MD5", "_MD5AUDIO"]
    );
    assert_eq!(
        tag_core::requested_hash_fields("'%_crc%'", &ExportOptions::default()).unwrap(),
        Vec::<String>::new()
    );
}
