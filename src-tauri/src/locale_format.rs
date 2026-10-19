//! Reformat presentation fields already in memory without reading their files.
use serde::{Deserialize, Serialize};
use tag_core::Fields;

#[derive(Debug, Deserialize, Serialize, PartialEq, Eq)]
pub(crate) struct TimestampItem {
    path: String,
    technical: Fields,
}

#[tauri::command]
pub(crate) fn locale_format_state() -> String {
    tag_core::locale_format_state()
}

#[tauri::command]
pub(crate) fn reformat_loaded_timestamps(mut items: Vec<TimestampItem>) -> Vec<TimestampItem> {
    tag_core::reformat_technical_time_fields(items.iter_mut().map(|item| &mut item.technical));
    items
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, UNIX_EPOCH};

    fn item(fields: &[(&str, &[&str])]) -> TimestampItem {
        TimestampItem {
            path: "content://nonexistent/never-opened.wav".into(),
            technical: fields
                .iter()
                .map(|(key, values)| {
                    (
                        (*key).into(),
                        values.iter().map(|value| (*value).into()).collect(),
                    )
                })
                .collect(),
        }
    }

    #[test]
    fn loaded_dates_reuse_reader_format_without_reading_or_changing_raw_fields() {
        let input = item(&[
            ("_FILE_MOD_DATETIME_RAW", &["1700000000"]),
            ("_FILE_CREATE_DATETIME_RAW", &["-1"]),
            ("_MP4_CREATION_TIME_RAW", &["1700000000"]),
            ("_MP4_MODIFICATION_TIME_RAW", &["-1"]),
            ("_FILE_MOD_DATE", &["old date"]),
            ("_FILE_MOD_DATETIME", &["old datetime"]),
            ("_FILE_CREATE_DATE", &["old creation"]),
            ("_MP4_CREATION_TIME", &["old MP4"]),
            ("_CODEC", &["AAC"]),
            ("CUSTOM", &["keep", "both"]),
        ]);
        let original_raw: Fields = input
            .technical
            .iter()
            .filter(|(key, _)| key.ends_with("_RAW"))
            .map(|(key, values)| (key.clone(), values.clone()))
            .collect();
        let result = reformat_loaded_timestamps(vec![input]);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].path, "content://nonexistent/never-opened.wav");
        let current = tag_core::file_time_fields(
            Some(UNIX_EPOCH - Duration::from_secs(1)),
            Some(UNIX_EPOCH + Duration::from_secs(1_700_000_000)),
        );
        for key in [
            "_FILE_MOD_DATE",
            "_FILE_MOD_DATETIME",
            "_FILE_CREATE_DATE",
            "_FILE_CREATE_DATETIME",
        ] {
            assert_eq!(result[0].technical.get(key), current.get(key), "{key}");
        }
        assert_eq!(
            result[0].technical["_MP4_CREATION_TIME"],
            current["_FILE_MOD_DATETIME"]
        );
        assert_eq!(
            result[0].technical["_MP4_MODIFICATION_TIME"],
            current["_FILE_CREATE_DATETIME"]
        );
        for (key, values) in original_raw {
            assert_eq!(result[0].technical[&key], values);
        }
        assert_eq!(result[0].technical["_CODEC"], ["AAC"]);
        assert_eq!(result[0].technical["CUSTOM"], ["keep", "both"]);
    }

    #[test]
    fn unusable_raw_dates_clear_only_their_stale_presentation() {
        for values in [
            vec![],
            vec!["not a timestamp"],
            vec!["0", "1"],
            vec!["9223372036854775807"],
        ] {
            let input = item(&[
                ("_FILE_MOD_DATETIME_RAW", &values),
                ("_FILE_MOD_DATE", &["stale"]),
                ("_FILE_MOD_DATETIME", &["stale"]),
                ("_FILE_CREATE_DATE", &["legacy without raw"]),
                ("_MP4_CREATION_TIME_RAW", &values),
                ("_MP4_CREATION_TIME", &["stale"]),
                ("_MP4_MODIFICATION_TIME", &["legacy without raw"]),
                ("OTHER_RAW", &["retain"]),
                ("_CODEC", &["AAC"]),
            ]);
            let result = reformat_loaded_timestamps(vec![input]);
            for key in [
                "_FILE_MOD_DATE",
                "_FILE_MOD_DATETIME",
                "_FILE_CREATE_DATE",
                "_MP4_CREATION_TIME",
                "_MP4_MODIFICATION_TIME",
            ] {
                assert!(!result[0].technical.contains_key(key), "{key}: {values:?}");
            }
            assert_eq!(result[0].technical["_FILE_MOD_DATETIME_RAW"], values);
            assert_eq!(result[0].technical["OTHER_RAW"], ["retain"]);
            assert_eq!(result[0].technical["_CODEC"], ["AAC"]);
        }
        assert!(reformat_loaded_timestamps(vec![]).is_empty());
    }

    #[test]
    fn locale_state_is_nonempty_and_stable_without_a_clock_tick() {
        let first = locale_format_state();
        assert!(!first.is_empty());
        assert_eq!(first, locale_format_state());
    }

    #[test]
    #[ignore = "Manual in-memory production command benchmark; no timing assertion"]
    fn benchmark_reformat_loaded_timestamps_for_large_loaded_lists() {
        for count in [1_000, 10_000] {
            let items: Vec<_> = (0..count)
                .map(|index| {
                    let mut entry = item(&[
                        ("_FILE_MOD_DATE", &["stale"]),
                        ("_FILE_MOD_DATETIME", &["stale"]),
                        ("_FILE_CREATE_DATE", &["stale"]),
                        ("_FILE_CREATE_DATETIME", &["stale"]),
                        ("_MP4_CREATION_TIME", &["stale"]),
                        ("_MP4_MODIFICATION_TIME", &["stale"]),
                        ("_CODEC", &["AAC"]),
                        ("CUSTOM", &["preserve", "both values"]),
                    ]);
                    entry.path = format!("content://never-opened/{index}.m4a");
                    for (offset, raw) in [
                        "_FILE_MOD_DATETIME_RAW",
                        "_FILE_CREATE_DATETIME_RAW",
                        "_MP4_CREATION_TIME_RAW",
                        "_MP4_MODIFICATION_TIME_RAW",
                    ]
                    .into_iter()
                    .enumerate()
                    {
                        entry.technical.insert(
                            raw.into(),
                            vec![(980_258_706 + index * 4 + offset).to_string()],
                        );
                    }
                    entry
                })
                .collect();
            // Exclude fixture construction and output assertions from the timing.
            let start = std::time::Instant::now();
            let output =
                std::hint::black_box(reformat_loaded_timestamps(std::hint::black_box(items)));
            let elapsed = start.elapsed();
            eprintln!(
                "locale_refresh_benchmark rows={count} elapsed_ns={}",
                elapsed.as_nanos()
            );
            assert_eq!(output.len(), count);
            for (index, entry) in output.iter().enumerate() {
                assert_eq!(entry.path, format!("content://never-opened/{index}.m4a"));
                assert_eq!(entry.technical["CUSTOM"], ["preserve", "both values"]);
                for (offset, raw) in [
                    "_FILE_MOD_DATETIME_RAW",
                    "_FILE_CREATE_DATETIME_RAW",
                    "_MP4_CREATION_TIME_RAW",
                    "_MP4_MODIFICATION_TIME_RAW",
                ]
                .into_iter()
                .enumerate()
                {
                    assert_eq!(
                        entry.technical[raw],
                        [(980_258_706 + index * 4 + offset).to_string()]
                    );
                }
                for shown in [
                    "_FILE_MOD_DATE",
                    "_FILE_MOD_DATETIME",
                    "_FILE_CREATE_DATE",
                    "_FILE_CREATE_DATETIME",
                    "_MP4_CREATION_TIME",
                    "_MP4_MODIFICATION_TIME",
                ] {
                    assert!(!entry.technical[shown][0].is_empty());
                    assert_ne!(entry.technical[shown], ["stale"]);
                }
            }
        }
    }
}
