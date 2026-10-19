//! Opt-in external codec carriers; no recordings or private paths are embedded.
//! TAK qualifies independent decoding; OFR qualifies encoded bytes/header only.
use serde::Deserialize;
use serde_json::{json, Value};
use std::{collections::BTreeMap, fs, ops::Range, path::Path, process::Command};
use tag_core::{
    history::{Journal, Limits},
    read_entry, write_fields, AudioEntry, Fields,
};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Manifest {
    version: u32,
    ffmpeg_path: String,
    ffmpeg_sha256: String,
    ffprobe_path: String,
    ffprobe_sha256: String,
    cases: BTreeMap<String, Carrier>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Carrier {
    path: String,
    sha256: String,
    source_url: String,
    retrieved_utc: String,
    format: String,
    sample_rate: u32,
    channels: u8,
    bits: u8,
    samples_per_channel: u64,
    pcm_s16le_sha256: Option<String>,
    packet_hashes: Option<Vec<String>>,
}

fn sha256(path: &Path) -> String {
    // Existing Python is an independent hash oracle, without a new Rust dependency.
    let out = Command::new("python3")
        .args(["-c", "import hashlib,sys; h=hashlib.sha256(); f=open(sys.argv[1],'rb'); [h.update(b) for b in iter(lambda:f.read(65536),b'')]; print(h.hexdigest())"])
        .arg(path)
        .output()
        .expect("existing python3 is required for the opt-in hash oracle");
    assert!(out.status.success(), "external SHA256 oracle failed");
    let hash = String::from_utf8(out.stdout).unwrap().trim().to_owned();
    assert!(hash.len() == 64 && hash.bytes().all(|b| b.is_ascii_hexdigit()));
    hash
}

struct ApeObservation {
    values: BTreeMap<String, Vec<String>>,
    value_ranges: BTreeMap<String, Range<usize>>,
}
fn le32(bytes: &[u8], at: usize) -> Result<u32, String> {
    let raw = bytes.get(at..at + 4).ok_or("truncated APE integer")?;
    Ok(u32::from_le_bytes(raw.try_into().unwrap()))
}
/// Independent tail-bound observer: never searches encoded audio for marker strings.
fn observe_ape(bytes: &[u8], original: &[u8]) -> Result<ApeObservation, String> {
    if !bytes.starts_with(original) {
        return Err("original encoded stream changed".into());
    }
    let footer = bytes.len().checked_sub(32).ok_or("missing APE footer")?;
    if bytes.get(footer..footer + 8) != Some(b"APETAGEX")
        || le32(bytes, footer + 8)? != 2000
        || le32(bytes, footer + 20)? & 0x2000_0000 != 0
    {
        return Err("invalid APEv2 footer".into());
    }
    let size = usize::try_from(le32(bytes, footer + 12)?).unwrap();
    let count = usize::try_from(le32(bytes, footer + 16)?).unwrap();
    if size < 32 || count > 64 {
        return Err("APE size/count bound".into());
    }
    let mut at = bytes.len().checked_sub(size).ok_or("APE exceeds file")?;
    let start = if le32(bytes, footer + 20)? & 0x8000_0000 != 0 {
        let start = at.checked_sub(32).ok_or("missing APE header")?;
        if bytes.get(start..start + 20) != bytes.get(footer..footer + 20)
            || le32(bytes, start + 20)? != le32(bytes, footer + 20)? | 0x2000_0000
        {
            return Err("APE header/footer disagree".into());
        }
        start
    } else {
        at
    };
    if start != original.len() {
        return Err("APE is not directly after the complete original stream".into());
    }
    let mut result = ApeObservation {
        values: BTreeMap::new(),
        value_ranges: BTreeMap::new(),
    };
    for _ in 0..count {
        let size = usize::try_from(le32(bytes, at)?).unwrap();
        if le32(bytes, at + 4)? & 6 != 0 {
            return Err("expected APE text item".into());
        }
        at = at.checked_add(8).ok_or("APE item overflow")?;
        let key_end = bytes
            .get(at..footer)
            .ok_or("APE key exceeds footer")?
            .iter()
            .take(256)
            .position(|b| *b == 0)
            .map(|n| at + n)
            .ok_or("unterminated APE key")?;
        let key = std::str::from_utf8(&bytes[at..key_end])
            .map_err(|_| "invalid APE key")?
            .to_ascii_uppercase();
        at = key_end + 1;
        let end = at
            .checked_add(size)
            .filter(|end| *end <= footer)
            .ok_or("APE value exceeds footer")?;
        let values = std::str::from_utf8(&bytes[at..end])
            .map_err(|_| "invalid APE UTF8")?
            .split('\0')
            .map(str::to_owned)
            .collect();
        if result.values.insert(key.clone(), values).is_some() {
            return Err("duplicate APE key".into());
        }
        result.value_ranges.insert(key, at..end);
        at = end;
    }
    if at != footer {
        return Err("APE item extents do not exhaust the tag".into());
    }
    Ok(result)
}

fn expected() -> Fields {
    [
        ("TITLE", vec!["Independent real carrier café 日本"]),
        ("ARTIST", vec!["First", "Second"]),
        ("CUSTOM", vec!["External qualification"]),
    ]
    .into_iter()
    .map(|(key, values)| (key.into(), values.into_iter().map(str::to_owned).collect()))
    .collect()
}
fn validate_observation(bytes: &[u8], original: &[u8]) -> Result<ApeObservation, String> {
    let observed = observe_ape(bytes, original)?;
    if observed.values != expected() {
        return Err("independent physical APE values differ".into());
    }
    Ok(observed)
}
fn assert_entry(entry: &AudioEntry, carrier: &Carrier) {
    assert_eq!(entry.format, carrier.format);
    assert_eq!(entry.sample_rate, carrier.sample_rate);
    assert_eq!(entry.channels, carrier.channels);
    assert_eq!(
        entry.technical["_BITSPERSAMPLE"],
        [carrier.bits.to_string()]
    );
    let seconds = carrier.samples_per_channel as f64 / f64::from(carrier.sample_rate);
    assert!((entry.duration - seconds).abs() < 1e-6);
}
fn packets(manifest: &Manifest, path: &Path, carrier: &Carrier) -> Value {
    let out = Command::new(&manifest.ffprobe_path)
        .args([
            "-v",
            "error",
            "-show_streams",
            "-show_packets",
            "-show_data_hash",
            "sha256",
            "-of",
            "json",
        ])
        .arg(path)
        .output()
        .expect("existing ffprobe is required for TAK qualification");
    assert!(
        out.status.success() && out.stderr.is_empty(),
        "TAK ffprobe failed"
    );
    let observed: Value = serde_json::from_slice(&out.stdout).unwrap();
    let streams = observed["streams"].as_array().unwrap();
    assert_eq!(streams.len(), 1);
    assert_eq!(streams[0]["codec_name"], "tak");
    assert_eq!(streams[0]["sample_rate"], carrier.sample_rate.to_string());
    assert_eq!(streams[0]["channels"], carrier.channels);
    assert_eq!(streams[0]["bits_per_raw_sample"], carrier.bits.to_string());
    assert_eq!(streams[0]["duration_ts"], carrier.samples_per_channel);
    let packets = observed["packets"].as_array().unwrap();
    let hashes: Vec<_> = packets
        .iter()
        .map(|packet| packet["data_hash"].as_str().unwrap().to_owned())
        .collect();
    assert_eq!(&hashes, carrier.packet_hashes.as_ref().unwrap());
    // File positions may differ after tags; payload identity/timing may not.
    json!(packets
        .iter()
        .map(|packet| json!([
            packet["pts"],
            packet["duration"],
            packet["size"],
            packet["data_hash"]
        ]))
        .collect::<Vec<_>>())
}
fn pcm(manifest: &Manifest, path: &Path, carrier: &Carrier, destination: &Path) -> String {
    let out = Command::new(&manifest.ffmpeg_path)
        .args([
            "-nostdin",
            "-hide_banner",
            "-v",
            "error",
            "-xerror",
            "-y",
            "-i",
        ])
        .arg(path)
        .args(["-map", "0:a:0", "-c:a", "pcm_s16le", "-f", "s16le"])
        .arg(destination)
        .output()
        .expect("existing ffmpeg is required for TAK qualification");
    assert!(
        out.status.success() && out.stderr.is_empty(),
        "complete TAK decoding failed"
    );
    assert_eq!(
        fs::metadata(destination).unwrap().len(),
        carrier.samples_per_channel * u64::from(carrier.channels) * 2
    );
    let hash = sha256(destination);
    assert_eq!(Some(&hash), carrier.pcm_s16le_sha256.as_ref());
    hash
}

fn qualify(kind: &str, decode: bool) {
    let manifest_path = std::env::var("TAGGROOVE_LOSSLESS_MANIFEST")
        .expect("ignored external test requires TAGGROOVE_LOSSLESS_MANIFEST");
    let manifest: Manifest = serde_json::from_slice(&fs::read(&manifest_path).unwrap()).unwrap();
    assert_eq!(manifest.version, 1);
    let carrier = &manifest.cases[kind];
    let source = Path::new(&carrier.path);
    assert!(source.is_absolute());
    assert!(carrier.source_url.starts_with("https://") && !carrier.retrieved_utc.is_empty());
    assert_eq!(
        sha256(source),
        carrier.sha256,
        "external input hash differs"
    );
    let original = fs::read(source).unwrap();
    assert!((1024..=8 * 1024 * 1024).contains(&original.len()));
    assert_eq!(&original[..4], if decode { b"tBaK" } else { b"OFR " });
    assert_ne!(
        &original[original.len() - 128..original.len() - 125],
        b"TAG"
    );
    assert_ne!(
        &original[original.len() - 32..original.len() - 24],
        b"APETAGEX"
    );
    let directory = tempfile::tempdir().unwrap();
    let path = directory
        .path()
        .join(if decode { "working.tak" } else { "working.ofr" });
    fs::write(&path, &original).unwrap();
    assert_entry(&read_entry(&path).unwrap(), carrier);
    let pcm_path = directory.path().join("decoded.pcm");
    let (before_packets, pcm_hash) = if decode {
        assert_eq!(
            carrier.bits, 16,
            "canonical s16le oracle requires a 16-bit carrier"
        );
        assert_eq!(
            sha256(Path::new(&manifest.ffmpeg_path)),
            manifest.ffmpeg_sha256
        );
        assert_eq!(
            sha256(Path::new(&manifest.ffprobe_path)),
            manifest.ffprobe_sha256
        );
        (
            Some(packets(&manifest, &path, carrier)),
            Some(pcm(&manifest, &path, carrier, &pcm_path)),
        )
    } else {
        assert!(carrier.pcm_s16le_sha256.is_none() && carrier.packet_hashes.is_none());
        (None, None)
    };
    let mut journal = Journal::open(&directory.path().join("journal"), Limits::default()).unwrap();
    let label = "External lossless tag edit";
    let capture = journal.begin(label, None, &[path.to_string_lossy().into_owned()]);
    let saved_entry = write_fields(&path, &expected()).unwrap();
    journal.commit(capture, &[], &[]);
    assert!(
        journal.state().notice.is_none(),
        "current operation did not receive undo admission"
    );
    let step = journal.state().undo.unwrap();
    assert_eq!(step.label, label);
    assert_eq!(step.files, 1);
    assert_entry(&saved_entry, carrier);
    for (key, values) in expected() {
        assert_eq!(
            saved_entry
                .fields
                .iter()
                .find(|(name, _)| name.eq_ignore_ascii_case(&key))
                .unwrap()
                .1,
            &values
        );
    }
    let saved = fs::read(&path).unwrap();
    let observed = validate_observation(&saved, &original).unwrap();
    if decode {
        assert_eq!(Some(packets(&manifest, &path, carrier)), before_packets);
        assert_eq!(Some(pcm(&manifest, &path, carrier, &pcm_path)), pcm_hash);
    }
    let mut encoded_mutation = saved.clone();
    encoded_mutation[original.len() / 2] ^= 1;
    assert_eq!(
        validate_observation(&encoded_mutation, &original)
            .err()
            .as_deref(),
        Some("original encoded stream changed")
    );
    let mut tag_mutation = saved.clone();
    tag_mutation[observed.value_ranges["TITLE"].start] ^= 1;
    assert_eq!(
        validate_observation(&tag_mutation, &original)
            .err()
            .as_deref(),
        Some("independent physical APE values differ")
    );
    assert_eq!(
        fs::read(&path).unwrap(),
        saved,
        "in-memory controls touched disk"
    );
    assert!(journal.undo().unwrap().errors.is_empty());
    assert_eq!(fs::read(&path).unwrap(), original);
    assert!(journal.redo().unwrap().errors.is_empty());
    assert_eq!(fs::read(&path).unwrap(), saved);
    validate_observation(&fs::read(&path).unwrap(), &original).unwrap();
    let state = journal.state();
    let capture = journal.begin(
        "Rejected external tag edit",
        None,
        &[path.to_string_lossy().into_owned()],
    );
    let invalid = [("TITLE".to_owned(), vec!["invalid\0value".to_owned()])]
        .into_iter()
        .collect();
    assert!(write_fields(&path, &invalid).is_err());
    journal.commit(capture, &[], &[]);
    assert_eq!(fs::read(&path).unwrap(), saved);
    assert_eq!(journal.state(), state, "refusal changed history");
    assert_eq!(sha256(source), carrier.sha256, "external original changed");
    println!(
        "REAL_LOSSLESS_EXTERNAL {}",
        json!({
            "case":kind,"input_sha256":carrier.sha256,"manifest_sha256":sha256(Path::new(&manifest_path)),
            "encoded_bytes_preserved":true,"physical_ape_values":observed.values,
            "encoded_mutation_detected":true,"tag_mutation_detected":true,"undo_exact":true,"redo_exact":true,"refusal_unchanged":true,
            "decoded_pcm_qualified":decode,"codec_completeness_qualified":decode,"pcm_sha256":pcm_hash,
            "packets":carrier.packet_hashes.as_ref().map(Vec::len),"original_runtime":false,"native_gui":false,"redistribution_permission_established":false
        })
    );
}

#[test]
#[ignore = "requires hash-bound external carrier manifest and existing ffmpeg/ffprobe; no media in Git"]
fn complete_tak_real_carrier_preserves_packets_pcm_ape_and_exact_history() {
    qualify("tak", true);
}
#[test]
#[ignore = "external OFR encoded bytes/header only; codec completeness and decoded PCM remain unqualified"]
fn optimfrog_external_encoded_bytes_and_header_survive_ape_and_exact_history() {
    qualify("ofr", false);
}
