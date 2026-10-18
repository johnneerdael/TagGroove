//! SAF-only test adapter; all record, transaction and recovery logic is Store.
use super::*;
use sha2::{Digest, Sha256};
use std::{
    fs,
    sync::{Arc, Mutex},
};
use tauri_plugin_document_access::{DocumentInfo, DocumentSnapshot};

#[derive(Clone)]
pub(super) struct Provider(Arc<Mutex<Backend>>);
struct Backend {
    sources: BTreeMap<String, (DocumentInfo, PathBuf)>,
    failure: bool,
    copy_destination: Option<PathBuf>,
}
impl Provider {
    fn snapshot_of(backend: &Backend, uri: &str) -> Result<DocumentSnapshot, String> {
        let (document, path) = backend.sources.get(uri).ok_or("Unknown fixture URI")?;
        let bytes = fs::read(path).map_err(|e| e.to_string())?;
        Ok(DocumentSnapshot {
            document: document.clone(),
            sha256: format!("{:x}", Sha256::digest(&bytes)),
            bytes: bytes.len() as u64,
        })
    }
}
impl engine::Provider for Provider {
    fn snapshot(&self, uri: &str, output: &Path) -> Result<DocumentSnapshot, String> {
        let backend = self.0.lock().unwrap();
        let (_, path) = backend.sources.get(uri).ok_or("Unknown fixture URI")?;
        let mut out = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(output)
            .map_err(|e| e.to_string())?;
        std::io::copy(
            &mut fs::File::open(path).map_err(|e| e.to_string())?,
            &mut out,
        )
        .map_err(|e| e.to_string())?;
        Self::snapshot_of(&backend, uri)
    }
    fn publish(&self, uri: &str, input: &Path, expected: &str) -> Result<DocumentSnapshot, String> {
        let backend = self.0.lock().unwrap();
        let before = Self::snapshot_of(&backend, uri)?;
        if !before.document.writable {
            return Err("fixture source is read-only".into());
        }
        if before.sha256 != expected {
            return Err("fixture source changed".into());
        }
        if backend.failure {
            return Err("fixture provider publication failed".into());
        }
        fs::copy(input, &backend.sources[uri].1).map_err(|e| e.to_string())?;
        Self::snapshot_of(&backend, uri)
    }
    fn create(&self, name: &str, mime: Option<&str>) -> Result<Option<DocumentInfo>, String> {
        let mut backend = self.0.lock().unwrap();
        let Some(path) = backend.copy_destination.clone() else {
            return Ok(None);
        };
        let document = DocumentInfo {
            uri: format!("content://command-fixture/{}", uuid::Uuid::new_v4()),
            name: name.into(),
            mime_type: mime.map(str::to_owned),
            size: None,
            modified_ms: None,
            writable: true,
            persisted: true,
        };
        // Match CREATE_DOCUMENT's provider-defined initial bytes. Every test
        // explicitly chooses an isolated destination; never truncate an input.
        let mut output = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .map_err(|e| e.to_string())?;
        std::io::Write::write_all(&mut output, b"provider initial contents")
            .map_err(|e| e.to_string())?;
        backend
            .sources
            .insert(document.uri.clone(), (document.clone(), path));
        Ok(Some(document))
    }
}
pub(super) static STORES: Mutex<Vec<engine::Store<Provider>>> = Mutex::new(Vec::new());

pub struct Session {
    provider: Provider,
    pub path: String,
    pub uri: String,
}
impl Session {
    pub fn import(root: &Path, source: &Path, name: &str, writable: bool) -> Self {
        let uri = format!("content://command-fixture/{}", uuid::Uuid::new_v4());
        let info = DocumentInfo {
            uri: uri.clone(),
            name: name.into(),
            mime_type: Some("audio/wav".into()),
            size: None,
            modified_ms: Some(1_700_000_000_000),
            writable,
            persisted: true,
        };
        let provider = Provider(Arc::new(Mutex::new(Backend {
            sources: BTreeMap::from([(uri.clone(), (info.clone(), source.to_owned()))]),
            failure: false,
            copy_destination: None,
        })));
        let mut store =
            engine::Store::open(root, provider.clone(), crate::settings::tag_options).unwrap();
        let path = store.import(info).unwrap().path;
        STORES.lock().unwrap().push(store);
        Self {
            provider,
            path,
            uri,
        }
    }
    pub fn rename(&self, name: &str) {
        self.provider
            .0
            .lock()
            .unwrap()
            .sources
            .get_mut(&self.uri)
            .unwrap()
            .0
            .name = name.into();
    }
    pub fn fail_publish(&self) {
        self.provider.0.lock().unwrap().failure = true;
    }
    pub fn resume_publish(&self) {
        self.provider.0.lock().unwrap().failure = false;
    }
    pub fn copy_to(&self, path: &Path) {
        self.provider.0.lock().unwrap().copy_destination = Some(path.to_owned());
    }
    pub fn add_input(&self, uri: &str, path: &Path, name: &str) {
        let mut backend = self.provider.0.lock().unwrap();
        let mut info = backend.sources[&self.uri].0.clone();
        info.uri = uri.into();
        info.name = name.into();
        backend.sources.insert(uri.into(), (info, path.to_owned()));
    }
}
impl Drop for Session {
    fn drop(&mut self) {
        STORES
            .lock()
            .unwrap()
            .retain(|store| !store.is_managed(Path::new(&self.path)));
    }
}

pub(crate) fn snapshot_input(uri: &str) -> Option<Result<PathBuf, String>> {
    snapshot_input_with_name(uri).map(|result| result.map(|(path, _)| path))
}

pub(crate) fn snapshot_input_with_name(uri: &str) -> Option<Result<(PathBuf, String), String>> {
    STORES.lock().unwrap().iter().find_map(|store| {
        if store.states().values().any(|d| {
            uri.strip_prefix(&d.uri)
                .is_some_and(|suffix| suffix.starts_with('/'))
        }) {
            Some(store.snapshot_input_with_name(uri))
        } else {
            None
        }
    })
}

#[test]
fn provider_policy_adapter_reads_current_settings_for_reload_save_and_copy_results() {
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("source.mp3");
    let mut frames = Vec::new();
    for _ in 0..20 {
        frames.extend([0xff, 0xfb, 0x90, 0x64]);
        frames.extend([0; 413]);
    }
    fs::write(&source, frames).unwrap();
    let fields = |title: &str| Fields::from([("TITLE".into(), vec![title.into()])]);
    tag_core::write_fields(&source, &fields("Original title")).unwrap();
    let session = Session::import(&dir.path().join("private"), &source, "source.mp3", true);
    let path = Path::new(&session.path);
    let original_bytes = fs::read(&source).unwrap();
    let mut options = tag_core::TagOptions::default();
    options.mpeg.read_id3v1 = false;
    options.mpeg.read_ape = false;
    options.mappings.push(tag_core::UserFieldMapping {
        tag_type: "ID3v2".into(),
        source: "TITLE".into(),
        target: "DISPLAY_TITLE".into(),
    });
    crate::settings::with_test_tag_options(options, || {
        let assert_title = |entry: &AudioEntry, title: &str| {
            assert_eq!(entry.fields["DISPLAY_TITLE"], [title]);
            assert!(!entry.fields.contains_key("TITLE"));
        };
        assert_title(&super::read_working(path).unwrap(), "Original title");
        assert_title(&super::reload(path).unwrap(), "Original title");
        assert_eq!(fs::read(&source).unwrap(), original_bytes);
        // Discard the writer's default-policy description and describe the
        // published document using the current settings-backed supplier.
        let saved = super::mutate(path, |path| {
            tag_core::write_fields(path, &fields("Saved title"))
        })
        .unwrap();
        assert_title(&saved, "Saved title");
        assert_eq!(
            saved.technical["_PATH"].as_slice(),
            std::slice::from_ref(&session.uri)
        );
        for (version, expected) in [("original", "Saved title"), ("edited", "Copy title")] {
            session.copy_to(&dir.path().join(format!("{version}.mp3")));
            let result = super::save_copy(
                &session.path,
                &Fields::from([("DISPLAY_TITLE".into(), vec!["Copy title".into()])]),
                None,
                version,
            )
            .unwrap();
            assert!(result.errors.is_empty());
            assert_eq!(result.entries.len(), 1);
            assert_title(&result.entries[0], expected);
            assert_ne!(
                result.entries[0].technical["_PATH"].as_slice(),
                std::slice::from_ref(&session.uri)
            );
        }
        assert_eq!(
            tag_core::read_entry(&source).unwrap().fields["TITLE"],
            ["Saved title"]
        );
    });
}
