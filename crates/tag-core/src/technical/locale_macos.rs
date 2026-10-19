//! Native macOS regional date presentation. Formatter objects stay local to a call.
// Declarations follow the installed macOS SDK's CFLocale.h, CFDateFormatter.h,
// CFTimeZone.h and CFString.h. No global user preference is changed.
use std::{
    ffi::{c_char, c_void, CStr},
    ptr::NonNull,
};

type CFRef = *const c_void;
const UTF8: u32 = 0x0800_0100;
const UNIX_TO_CF: i64 = 978_307_200;
const SAMPLES: [i64; 2] = [980_258_706, 995_897_106];

#[link(name = "CoreFoundation", kind = "framework")]
extern "C" {
    fn CFRelease(value: CFRef);
    fn CFLocaleCopyCurrent() -> CFRef;
    fn CFLocaleGetIdentifier(locale: CFRef) -> CFRef;
    fn CFTimeZoneResetSystem();
    fn CFTimeZoneCopySystem() -> CFRef;
    fn CFTimeZoneGetName(zone: CFRef) -> CFRef;
    fn CFTimeZoneGetSecondsFromGMT(zone: CFRef, time: f64) -> f64;
    fn CFDateFormatterCreate(
        allocator: CFRef,
        locale: CFRef,
        date_style: isize,
        time_style: isize,
    ) -> CFRef;
    fn CFDateFormatterSetProperty(formatter: CFRef, key: CFRef, value: CFRef);
    fn CFDateFormatterGetFormat(formatter: CFRef) -> CFRef;
    fn CFDateFormatterCreateStringWithAbsoluteTime(
        allocator: CFRef,
        formatter: CFRef,
        time: f64,
    ) -> CFRef;
    fn CFStringGetLength(value: CFRef) -> isize;
    fn CFStringGetMaximumSizeForEncoding(length: isize, encoding: u32) -> isize;
    fn CFStringGetCString(value: CFRef, buffer: *mut c_char, size: isize, encoding: u32) -> u8;
    static kCFDateFormatterTimeZone: CFRef;
    #[cfg(test)]
    fn CFLocaleCreate(allocator: CFRef, identifier: CFRef) -> CFRef;
    #[cfg(test)]
    fn CFTimeZoneCreateWithTimeIntervalFromGMT(allocator: CFRef, interval: f64) -> CFRef;
    #[cfg(test)]
    fn CFStringCreateWithBytes(
        allocator: CFRef,
        bytes: *const u8,
        size: isize,
        encoding: u32,
        external: u8,
    ) -> CFRef;
    #[cfg(test)]
    fn CFDateFormatterSetFormat(formatter: CFRef, format: CFRef);
}

/// Own a Create/Copy result. NonNull also keeps CFDateFormatter off shared threads.
struct Owned(NonNull<c_void>);
impl Owned {
    fn new(value: CFRef) -> Option<Self> {
        NonNull::new(value.cast_mut()).map(Self)
    }
    fn as_ptr(&self) -> CFRef {
        self.0.as_ptr().cast_const()
    }
}
impl Drop for Owned {
    fn drop(&mut self) {
        // SAFETY: construction accepts only an owned non-null Create/Copy result.
        unsafe { CFRelease(self.as_ptr()) }
    }
}

fn string(value: CFRef) -> Option<String> {
    if value.is_null() {
        return None;
    }
    // SAFETY: every call passes a live CFString, either owned or borrowed from
    // an object retained for the duration of this conversion.
    unsafe {
        let capacity =
            CFStringGetMaximumSizeForEncoding(CFStringGetLength(value), UTF8).checked_add(1)?;
        // Formatter metadata is small; reject invalid/absurd native lengths.
        if !(1..=1_048_576).contains(&capacity) {
            return None;
        }
        let mut bytes = vec![0u8; usize::try_from(capacity).ok()?];
        if CFStringGetCString(value, bytes.as_mut_ptr().cast(), capacity, UTF8) == 0 {
            return None;
        }
        CStr::from_bytes_until_nul(&bytes)
            .ok()?
            .to_str()
            .ok()
            .map(str::to_owned)
    }
}

pub(super) struct Formatter {
    date: Owned,
    datetime: Owned,
    locale: Owned,
    zone: Owned,
}
impl Formatter {
    pub(super) fn current() -> Option<Self> {
        // SAFETY: Copy gives owned objects; Reset invalidates only CF's cached
        // system-zone object, so a running app observes actual OS zone changes.
        // CFLocale.h recommends a fresh temporary CopyCurrent per operation.
        unsafe {
            let locale = Owned::new(CFLocaleCopyCurrent())?;
            CFTimeZoneResetSystem();
            let zone = Owned::new(CFTimeZoneCopySystem())?;
            Self::new(locale, zone)
        }
    }
    fn new(locale: Owned, zone: Owned) -> Option<Self> {
        // SAFETY: objects remain retained throughout creation/property assignment.
        // Short date and medium time honor regional patterns and include seconds.
        unsafe {
            let date = Owned::new(CFDateFormatterCreate(
                std::ptr::null(),
                locale.as_ptr(),
                1,
                0,
            ))?;
            let datetime = Owned::new(CFDateFormatterCreate(
                std::ptr::null(),
                locale.as_ptr(),
                1,
                2,
            ))?;
            CFDateFormatterSetProperty(date.as_ptr(), kCFDateFormatterTimeZone, zone.as_ptr());
            CFDateFormatterSetProperty(datetime.as_ptr(), kCFDateFormatterTimeZone, zone.as_ptr());
            Some(Self {
                date,
                datetime,
                locale,
                zone,
            })
        }
    }
    pub(super) fn format(&self, seconds: i64) -> Option<(String, String)> {
        let time = (seconds.checked_sub(UNIX_TO_CF)?) as f64;
        // SAFETY: formatter objects are local, live and never used concurrently.
        unsafe {
            let date = Owned::new(CFDateFormatterCreateStringWithAbsoluteTime(
                std::ptr::null(),
                self.date.as_ptr(),
                time,
            ))?;
            let datetime = Owned::new(CFDateFormatterCreateStringWithAbsoluteTime(
                std::ptr::null(),
                self.datetime.as_ptr(),
                time,
            ))?;
            Some((string(date.as_ptr())?, string(datetime.as_ptr())?))
        }
    }
    pub(super) fn state(&self) -> String {
        // SAFETY: Get results are borrowed only while their owners remain alive.
        unsafe {
            format!(
                "{:?}",
                (
                    "macos-core-foundation",
                    string(CFLocaleGetIdentifier(self.locale.as_ptr())),
                    string(CFTimeZoneGetName(self.zone.as_ptr())),
                    string(CFDateFormatterGetFormat(self.date.as_ptr())),
                    string(CFDateFormatterGetFormat(self.datetime.as_ptr())),
                    SAMPLES.map(|sample| self.format(sample)),
                    SAMPLES.map(|sample| CFTimeZoneGetSecondsFromGMT(
                        self.zone.as_ptr(),
                        (sample - UNIX_TO_CF) as f64
                    )),
                )
            )
        }
    }
}

#[cfg(test)]
fn native_string(value: &str) -> Owned {
    // SAFETY: native code copies the valid UTF-8 slice during this call.
    Owned::new(unsafe {
        CFStringCreateWithBytes(
            std::ptr::null(),
            value.as_ptr(),
            value.len().try_into().unwrap(),
            UTF8,
            0,
        )
    })
    .unwrap()
}
#[cfg(test)]
fn explicit_formatter(locale: &str, offset: i32) -> Formatter {
    let identifier = native_string(locale);
    // SAFETY: locale input is retained and both Create calls return owned objects.
    unsafe {
        let locale = Owned::new(CFLocaleCreate(std::ptr::null(), identifier.as_ptr())).unwrap();
        let zone = Owned::new(CFTimeZoneCreateWithTimeIntervalFromGMT(
            std::ptr::null(),
            f64::from(offset),
        ))
        .unwrap();
        Formatter::new(locale, zone).unwrap()
    }
}
#[cfg(test)]
fn explicit_format(locale: &str, offset: i32, seconds: i64) -> Option<(String, String)> {
    explicit_formatter(locale, offset).format(seconds)
}

#[cfg(test)]
mod tests {
    use super::*;

    // 2001-01-23 14:05:06 UTC; day > 12 exposes month/day ordering.
    const SAMPLE: i64 = 980_258_706;

    #[test]
    fn native_date_format_uses_explicit_region_and_timezone() {
        let us = explicit_format("en_US", 0, SAMPLE).expect("native US date");
        let gb = explicit_format("en_GB", 0, SAMPLE).expect("native GB date");
        assert_eq!(us.0, "1/23/01");
        assert_eq!(gb.0, "23/01/2001");
        assert_ne!(us.1, gb.1);
        assert_eq!(
            explicit_format("en_GB", 14 * 3600, SAMPLE).unwrap().0,
            "24/01/2001"
        );
    }

    #[test]
    fn native_state_captures_region_zone_and_custom_pattern_without_clock_time() {
        let formatter = explicit_formatter("en_GB", 0);
        let state = formatter.state();
        assert_eq!(state, explicit_formatter("en_GB", 0).state());
        assert_ne!(state, explicit_formatter("en_US", 0).state());
        assert_ne!(state, explicit_formatter("en_GB", 3600).state());
        let pattern = native_string("yyyy|MM|dd");
        // SAFETY: both objects remain retained; only this test-local formatter changes.
        unsafe {
            CFDateFormatterSetFormat(formatter.date.as_ptr(), pattern.as_ptr());
        }
        assert_ne!(state, formatter.state());
        assert_eq!(formatter.format(SAMPLE).unwrap().0, "2001|01|23");
    }

    #[test]
    fn current_snapshot_matches_actual_reader_fields_and_native_patterns() {
        let formatter = Formatter::current().expect("current native OS formatter");
        let time = std::time::UNIX_EPOCH + std::time::Duration::from_secs(SAMPLE as u64);
        let fields = super::super::file_time_fields(None, Some(time));
        let formatted = formatter.format(SAMPLE).unwrap();
        assert_eq!(fields["_FILE_MOD_DATE"], [formatted.0]);
        assert_eq!(fields["_FILE_MOD_DATETIME"], [formatted.1]);
        let state = formatter.state();
        assert!(state.contains("macos-core-foundation"));
        // SAFETY: all borrowed objects remain retained by formatter.
        unsafe {
            for value in [
                CFLocaleGetIdentifier(formatter.locale.as_ptr()),
                CFTimeZoneGetName(formatter.zone.as_ptr()),
                CFDateFormatterGetFormat(formatter.date.as_ptr()),
            ] {
                let value = string(value).unwrap();
                assert!(!value.is_empty());
                assert!(state.contains(&format!("{value:?}")));
            }
        }
    }

    #[test]
    fn loaded_batch_uses_one_selected_regional_snapshot_for_all_timestamp_kinds() {
        use super::super::{Fields, TimestampFormatter};
        for (region, offset) in [("en_US", 0), ("en_GB", 14 * 3600)] {
            let snapshot = TimestampFormatter {
                locale: chrono::Locale::en_US,
                native: Some(explicit_formatter(region, offset)),
            };
            let mut rows: Vec<Fields> = (0..3)
                .map(|index| {
                    [
                        ("_FILE_MOD_DATETIME_RAW", vec![SAMPLE.to_string()]),
                        ("_FILE_CREATE_DATETIME_RAW", vec!["-1".into()]),
                        ("_MP4_CREATION_TIME_RAW", vec![SAMPLE.to_string()]),
                        ("_MP4_MODIFICATION_TIME_RAW", vec!["-1".into()]),
                        ("_FILE_MOD_DATE", vec!["stale".into()]),
                        ("_FILE_MOD_DATETIME", vec!["stale".into()]),
                        ("_FILE_CREATE_DATE", vec!["stale".into()]),
                        ("_FILE_CREATE_DATETIME", vec!["stale".into()]),
                        ("_MP4_CREATION_TIME", vec!["stale".into()]),
                        ("_MP4_MODIFICATION_TIME", vec!["stale".into()]),
                        ("CUSTOM", vec!["keep".into(), "both".into()]),
                        ("ROW", vec![index.to_string()]),
                    ]
                    .into_iter()
                    .map(|(key, values)| (key.into(), values))
                    .collect()
                })
                .collect();
            rows[1].remove("_FILE_MOD_DATETIME_RAW");
            rows[2].insert("_FILE_CREATE_DATETIME_RAW".into(), vec!["invalid".into()]);
            rows[2].insert(
                "_MP4_MODIFICATION_TIME_RAW".into(),
                vec!["0".into(), "1".into()],
            );
            let original = rows.clone();
            super::super::reformat_time_fields_with(rows.iter_mut(), &snapshot);
            let positive = explicit_format(region, offset, SAMPLE).unwrap();
            let negative = explicit_format(region, offset, -1).unwrap();
            for (index, row) in rows.iter().enumerate() {
                if index == 1 {
                    assert!(!row.contains_key("_FILE_MOD_DATE"));
                    assert!(!row.contains_key("_FILE_MOD_DATETIME"));
                } else {
                    assert_eq!(
                        row["_FILE_MOD_DATE"].as_slice(),
                        std::slice::from_ref(&positive.0),
                        "{region} row {index}"
                    );
                    assert_eq!(
                        row["_FILE_MOD_DATETIME"].as_slice(),
                        std::slice::from_ref(&positive.1),
                        "{region} row {index}"
                    );
                }
                if index == 2 {
                    assert!(!row.contains_key("_FILE_CREATE_DATE"));
                    assert!(!row.contains_key("_FILE_CREATE_DATETIME"));
                    assert!(!row.contains_key("_MP4_MODIFICATION_TIME"));
                } else {
                    assert_eq!(
                        row["_FILE_CREATE_DATE"].as_slice(),
                        std::slice::from_ref(&negative.0)
                    );
                    assert_eq!(
                        row["_FILE_CREATE_DATETIME"].as_slice(),
                        std::slice::from_ref(&negative.1)
                    );
                    assert_eq!(
                        row["_MP4_MODIFICATION_TIME"].as_slice(),
                        std::slice::from_ref(&negative.1)
                    );
                }
                assert_eq!(
                    row["_MP4_CREATION_TIME"].as_slice(),
                    std::slice::from_ref(&positive.1)
                );
                for (key, value) in &original[index] {
                    if key.ends_with("_RAW") || ["ROW", "CUSTOM"].contains(&key.as_str()) {
                        assert_eq!(&row[key], value, "{key} in row {index}");
                    }
                }
            }
        }
    }
}
