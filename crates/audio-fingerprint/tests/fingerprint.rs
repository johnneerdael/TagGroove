use opentag_audio_fingerprint::{Control, Error, Job};
use std::io::Write;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use std::time::Duration;

static SERIAL: Mutex<()> = Mutex::new(());

fn wav(seconds: u32) -> tempfile::NamedTempFile {
    let mut file = tempfile::NamedTempFile::new().unwrap();
    let frames = 11_025 * seconds;
    let bytes = frames * 2;
    file.write_all(b"RIFF").unwrap();
    file.write_all(&(36u32 + bytes).to_le_bytes()).unwrap();
    file.write_all(b"WAVEfmt ").unwrap();
    for data in [
        16u32.to_le_bytes(),
        [1, 0, 1, 0],
        11_025u32.to_le_bytes(),
        22_050u32.to_le_bytes(),
        [2, 0, 16, 0],
    ] {
        file.write_all(&data).unwrap();
    }
    file.write_all(b"data").unwrap();
    file.write_all(&bytes.to_le_bytes()).unwrap();
    for _ in 0..frames {
        file.write_all(&0i16.to_le_bytes()).unwrap();
    }
    file.flush().unwrap();
    file
}

#[test]
fn embedded_producer_preserves_full_duration_and_prefix_without_writing() {
    let _serial = SERIAL.lock().unwrap_or_else(|p| p.into_inner());
    let file = wav(135);
    let before = std::fs::read(file.path()).unwrap();
    let result = Job::new(file.path().to_owned(), Control::default())
        .unwrap()
        .run()
        .unwrap();
    assert_eq!(result.duration, 135.0);
    assert_eq!(result.processed_frames, 120 * 11_025);
    assert!(!result.encoded.is_empty());
    assert_eq!(before, std::fs::read(file.path()).unwrap());
}

#[test]
fn exact_prefix_does_not_decode_the_corrupt_following_frame() {
    let _serial = SERIAL.lock().unwrap_or_else(|p| p.into_inner());
    // Each FLAC frame is exactly one second at the normalized sample rate.
    // Only frame 121 is corrupt; the complete 120-second prefix is valid.
    let bytes = include_bytes!("fixtures/exact-prefix-corrupt-tail.flac");
    let mut file = tempfile::Builder::new().suffix(".flac").tempfile().unwrap();
    file.write_all(bytes).unwrap();
    file.flush().unwrap();
    let result = Job::new(file.path().to_owned(), Control::default())
        .unwrap()
        .run()
        .expect("A corrupt frame after the exact prefix must not be decoded");
    assert_eq!(result.duration, 121.0);
    assert_eq!(result.processed_frames, 120 * 11_025);
    assert!(!result.encoded.is_empty());
    assert_eq!(std::fs::read(file.path()).unwrap(), bytes);
}

#[test]
fn short_and_malformed_input_do_not_produce_usable_fingerprints() {
    let _serial = SERIAL.lock().unwrap_or_else(|p| p.into_inner());
    let short = wav(1);
    assert!(matches!(
        Job::new(short.path().to_owned(), Control::default())
            .unwrap()
            .run(),
        Err(Error::TooShort)
    ));
    let mut invalid = tempfile::NamedTempFile::new().unwrap();
    invalid.write_all(b"not media").unwrap();
    assert!(matches!(
        Job::new(invalid.path().to_owned(), Control::default())
            .unwrap()
            .run(),
        Err(Error::Decode(_))
    ));
}

#[test]
fn queued_native_work_is_bounded_and_released_when_job_is_dropped() {
    let _serial = SERIAL.lock().unwrap_or_else(|p| p.into_inner());
    let file = wav(10);
    let first = Job::new(file.path().to_owned(), Control::default()).unwrap();
    assert!(matches!(
        Job::new(file.path().to_owned(), Control::default()),
        Err(Error::Busy)
    ));
    drop(first);
    assert!(Job::new(file.path().to_owned(), Control::default()).is_ok());
}

#[test]
fn cancellation_and_zero_budget_stop_before_opening_the_file() {
    let _serial = SERIAL.lock().unwrap_or_else(|p| p.into_inner());
    let cancelled = Arc::new(AtomicBool::new(true));
    let control = Control::new(Duration::from_secs(120), move || {
        cancelled.load(Ordering::Acquire)
    });
    assert!(matches!(
        Job::new("/missing/should/not/open".into(), control)
            .unwrap()
            .run(),
        Err(Error::Cancelled)
    ));
    let control = Control::new(Duration::ZERO, || false);
    assert!(matches!(
        Job::new("/missing/should/not/open".into(), control)
            .unwrap()
            .run(),
        Err(Error::TimedOut)
    ));
}

#[test]
fn tag_readable_dsdiff_keeps_its_existing_fingerprint_capability() {
    use sha2::{Digest, Sha256};
    use std::io::Read;
    let _serial = SERIAL.lock().unwrap_or_else(|p| p.into_inner());
    let carrier = include_bytes!("fixtures/dsdiff12.dff.gz");
    assert_eq!(
        format!("{:x}", Sha256::digest(carrier)),
        "7d9c7dea40f7d658fdc5b9864c3ab73eb87f0cc7d29da556384b73e444394630"
    );
    let mut bytes = Vec::new();
    flate2::read::GzDecoder::new(carrier.as_slice())
        .read_to_end(&mut bytes)
        .unwrap();
    assert_eq!(bytes.len(), 8_467_330);
    assert_eq!(
        format!("{:x}", Sha256::digest(&bytes)),
        "7f1992e6d162c53d6493595d8924ed6935dfb460397d782f284b3d4341089ee5"
    );
    let mut file = tempfile::Builder::new().suffix(".dff").tempfile().unwrap();
    file.write_all(&bytes).unwrap();
    file.flush().unwrap();
    assert!(
        tag_core::read_entry(file.path()).is_ok(),
        "DSDIFF is already accepted by OpenTag's tag reader"
    );
    let result = Job::new(file.path().to_owned(), Control::default())
        .unwrap()
        .run()
        .expect("Tag-readable DSDIFF must remain fingerprintable");
    assert_eq!(result.duration, 12.0);
    assert_eq!(
        result.encoded,
        "AQAAS0mUaEkSRZEGAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA"
    );
    assert_eq!(std::fs::read(file.path()).unwrap(), bytes);
}

#[test]
fn tag_readable_vapc_aifc_keeps_its_existing_fingerprint_capability() {
    let _serial = SERIAL.lock().unwrap_or_else(|p| p.into_inner());
    fn chunk(kind: &[u8; 4], data: &[u8]) -> Vec<u8> {
        let mut out = Vec::from(kind.as_slice());
        out.extend_from_slice(&(data.len() as u32).to_be_bytes());
        out.extend_from_slice(data);
        if !data.len().is_multiple_of(2) {
            out.push(0);
        }
        out
    }
    let blocks = 27_563u32;
    let mut comm = Vec::new();
    comm.extend_from_slice(&1u16.to_be_bytes());
    comm.extend_from_slice(&(blocks * 16).to_be_bytes());
    comm.extend_from_slice(&16u16.to_be_bytes());
    comm.extend_from_slice(&[0x40, 0x0e, 0xac, 0x44, 0, 0, 0, 0, 0, 0]);
    comm.extend_from_slice(b"VAPC\0\0");
    let mut book = Vec::from(&b"stoc\x0bVADPCMCODES"[..]);
    book.extend_from_slice(&[0, 1, 0, 2, 0, 1]);
    book.extend_from_slice(&[0; 32]);
    let mut data = chunk(b"FVER", &[0xa2, 0x80, 0x51, 0x40]);
    data.extend(chunk(b"COMM", &comm));
    data.extend(chunk(b"APPL", &book));
    data.extend(chunk(b"SSND", &vec![0; 8 + blocks as usize * 9]));
    let mut file = tempfile::Builder::new().suffix(".aifc").tempfile().unwrap();
    file.write_all(b"FORM").unwrap();
    file.write_all(&(4 + data.len() as u32).to_be_bytes())
        .unwrap();
    file.write_all(b"AIFC").unwrap();
    file.write_all(&data).unwrap();
    file.flush().unwrap();
    assert!(
        tag_core::read_entry(file.path()).is_ok(),
        "VAPC-AIFC is already accepted by OpenTag's tag reader"
    );
    let result = Job::new(file.path().to_owned(), Control::default())
        .unwrap()
        .run()
        .unwrap();
    assert_eq!(
        result.encoded,
        "AQAAO0mUaEkSRZEGAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA"
    );
    // Reference stream metadata reports 160s for this constructed VAPC file,
    // independently of its decoded sample count. Preserve producer semantics.
    assert_eq!(result.duration, 160.0);
}

/// Frozen synthetic audio and fingerprints captured from the separately hashed
/// Chromaprint 1.6.2 CLI oracle. Test execution invokes no external media tools.
#[test]
fn embedded_api_matches_reference_corpus_without_external_programs() {
    use sha2::{Digest, Sha256};
    use std::io::Read;
    let _serial = SERIAL.lock().unwrap_or_else(|p| p.into_inner());
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let manifest: serde_json::Value =
        serde_json::from_slice(&std::fs::read(root.join("carrier-results.json")).unwrap()).unwrap();
    let cases = manifest["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 14);
    for case in cases {
        let name = case["name"].as_str().unwrap();
        let carrier = std::fs::read(root.join(case["carrier"].as_str().unwrap())).unwrap();
        assert_eq!(
            format!("{:x}", Sha256::digest(&carrier)),
            case["carrier_sha256"].as_str().unwrap(),
            "{name} carrier"
        );
        let mut bytes = Vec::new();
        match case["carrier_encoding"].as_str().unwrap() {
            "gzip" => {
                flate2::read::GzDecoder::new(carrier.as_slice())
                    .read_to_end(&mut bytes)
                    .unwrap();
            }
            "identity" => bytes = carrier,
            other => panic!("Unknown fixture carrier: {other}"),
        }
        assert_eq!(
            bytes.len() as u64,
            case["bytes"].as_u64().unwrap(),
            "{name} length"
        );
        let hash = format!("{:x}", Sha256::digest(&bytes));
        assert_eq!(hash, case["sha256"].as_str().unwrap(), "{name} input");
        let directory = tempfile::tempdir().unwrap();
        let filename = std::path::Path::new(case["file"].as_str().unwrap())
            .file_name()
            .unwrap();
        let path = directory.path().join(filename);
        std::fs::write(&path, &bytes).unwrap();
        let result = Job::new(path.clone(), Control::default())
            .unwrap()
            .run()
            .unwrap_or_else(|e| panic!("{name}: {e}"));
        assert_eq!(
            result.encoded,
            case["expected"]["fingerprint"].as_str().unwrap(),
            "{name} fingerprint"
        );
        assert_eq!(
            result.duration,
            case["expected"]["duration"].as_f64().unwrap(),
            "{name} full duration"
        );
        assert!(
            result.processed_frames > 0 && result.processed_frames <= 120 * 11_025,
            "{name} PCM prefix"
        );
        assert_eq!(
            format!("{:x}", Sha256::digest(std::fs::read(path).unwrap())),
            hash,
            "{name} unchanged"
        );
    }
}

#[test]
fn cooperative_cancel_after_native_callbacks_releases_all_resources() {
    use std::sync::atomic::AtomicUsize;
    let _serial = SERIAL.lock().unwrap_or_else(|p| p.into_inner());
    let file = wav(135);
    let polls = Arc::new(AtomicUsize::new(0));
    let counter = polls.clone();
    let control = Control::new(Duration::from_secs(120), move || {
        counter.fetch_add(1, Ordering::AcqRel) > 15
    });
    assert!(matches!(
        Job::new(file.path().to_owned(), control).unwrap().run(),
        Err(Error::Cancelled)
    ));
    assert!(
        polls.load(Ordering::Acquire) > 15,
        "cancel exercised callbacks after opening"
    );
    let result = Job::new(file.path().to_owned(), Control::default())
        .unwrap()
        .run()
        .unwrap();
    assert_eq!(result.duration, 135.0);
}

#[test]
fn a_waiting_caller_does_not_release_a_stalled_worker_admission() {
    use std::sync::{mpsc, Condvar};
    let _serial = SERIAL.lock().unwrap_or_else(|p| p.into_inner());
    let gate = Arc::new((Mutex::new(false), Condvar::new()));
    let worker_gate = gate.clone();
    let (entered_tx, entered_rx) = mpsc::channel();
    let control = Control::new(Duration::from_millis(20), move || {
        let (lock, wake) = &*worker_gate;
        let mut released = lock.lock().unwrap();
        let _ = entered_tx.send(());
        while !*released {
            released = wake.wait(released).unwrap();
        }
        true
    });
    let job = Job::new("/missing/not/opened".into(), control).unwrap();
    let worker = std::thread::spawn(move || job.run());
    entered_rx.recv_timeout(Duration::from_secs(2)).unwrap();
    std::thread::sleep(Duration::from_millis(30));
    assert!(matches!(
        Job::new("/missing/not/queued".into(), Control::default()),
        Err(Error::Busy)
    ));
    let (lock, wake) = &*gate;
    *lock.lock().unwrap() = true;
    wake.notify_all();
    assert!(matches!(worker.join().unwrap(), Err(Error::Cancelled)));
    assert!(Job::new("/missing/admitted".into(), Control::default()).is_ok());
}

#[test]
fn deadline_during_callbacks_stops_and_releases_the_worker() {
    use std::sync::atomic::AtomicUsize;
    let _serial = SERIAL.lock().unwrap_or_else(|p| p.into_inner());
    let file = wav(135);
    let polls = Arc::new(AtomicUsize::new(0));
    let counter = polls.clone();
    let control = Control::new(Duration::from_secs(2), move || {
        if counter.fetch_add(1, Ordering::AcqRel) > 15 {
            std::thread::sleep(Duration::from_millis(2100));
        }
        false
    });
    assert!(matches!(
        Job::new(file.path().to_owned(), control).unwrap().run(),
        Err(Error::TimedOut)
    ));
    assert!(polls.load(Ordering::Acquire) > 15);
    assert!(Job::new(file.path().to_owned(), Control::default()).is_ok());
}

#[test]
fn missing_paths_and_directories_are_structured_input_errors() {
    let _serial = SERIAL.lock().unwrap_or_else(|p| p.into_inner());
    let directory = tempfile::tempdir().unwrap();
    assert!(matches!(
        Job::new(directory.path().join("missing.wav"), Control::default())
            .unwrap()
            .run(),
        Err(Error::Io(_))
    ));
    assert!(matches!(
        Job::new(directory.path().to_owned(), Control::default())
            .unwrap()
            .run(),
        Err(Error::NotRegularFile)
    ));
}

#[cfg(unix)]
#[test]
fn fifo_input_is_rejected_without_a_blocking_open() {
    use std::os::unix::ffi::OsStrExt;
    let _serial = SERIAL.lock().unwrap_or_else(|p| p.into_inner());
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("not-an-audio-file");
    let name = std::ffi::CString::new(path.as_os_str().as_bytes()).unwrap();
    // SAFETY: NUL-terminated path points into the owned temporary directory.
    assert_eq!(unsafe { libc::mkfifo(name.as_ptr(), 0o600) }, 0);
    assert!(matches!(
        Job::new(path, Control::default()).unwrap().run(),
        Err(Error::NotRegularFile)
    ));
}
