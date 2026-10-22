//! In-memory baselines for a reviewed comparison, independent of HTTP lookups.
use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
    sync::Mutex,
};
use tag_core::{file_copy::FileIdentity, AudioEntry, Fields};

#[derive(serde::Serialize)]
pub struct ComparisonCoverView {
    pub digest: String,
    pub description: String,
}
#[derive(serde::Serialize)]
pub struct ComparisonFileView {
    pub path: String,
    pub fields: Fields,
    pub duration: f64,
    pub covers: Vec<ComparisonCoverView>,
}
#[derive(serde::Serialize)]
pub struct ComparisonSnapshotView {
    pub token: String,
    pub files: Vec<ComparisonFileView>,
}

#[derive(Clone, PartialEq, Eq)]
struct CoverState {
    digest: String,
    mime: String,
    description: String,
    picture_type: String,
    size: u64,
    width: Option<u32>,
    height: Option<u32>,
}
#[derive(Clone, PartialEq, Eq)]
struct MetadataState {
    fields: Fields,
    covers: Vec<CoverState>,
    media: (String, u64, u32, u32, u8, u64),
}
impl From<AudioEntry> for MetadataState {
    fn from(entry: AudioEntry) -> Self {
        Self {
            fields: entry.fields,
            covers: entry
                .covers
                .into_iter()
                .map(|cover| CoverState {
                    digest: cover.digest,
                    mime: cover.mime,
                    description: cover.description,
                    picture_type: cover.picture_type,
                    size: cover.size,
                    width: cover.width,
                    height: cover.height,
                })
                .collect(),
            media: (
                entry.format,
                entry.duration.to_bits(),
                entry.bitrate,
                entry.sample_rate,
                entry.channels,
                entry.size,
            ),
        }
    }
}

#[derive(Clone)]
pub(super) struct Baseline {
    identity: Option<FileIdentity>,
    state: MetadataState,
}
impl Baseline {
    fn read(path: &Path) -> Result<Self, String> {
        crate::require_audio_handle(path)?;
        let managed = crate::documents::is_managed(path);
        let identity = if managed {
            None
        } else {
            Some(FileIdentity::read(path)?)
        };
        let entry = if managed {
            crate::documents::read_working(path)?
        } else {
            crate::settings::read_summary(path)?
        };
        if let Some(before) = identity {
            if before != FileIdentity::read(path)? {
                return Err(
                    "The file changed while its metadata was being read. Reload the comparison."
                        .into(),
                );
            }
        }
        Ok(Self {
            identity,
            state: entry.into(),
        })
    }
    pub(super) fn verify(&self, working: &Path) -> Result<(), String> {
        if let Some(expected) = self.identity {
            if FileIdentity::read(working)? != expected {
                return Err(
                    "The file changed after comparison started. Reload its metadata before saving."
                        .into(),
                );
            }
        }
        let current: MetadataState = crate::settings::read_summary(working)?.into();
        if current != self.state {
            return Err(
                "Metadata changed after comparison started. Reload the comparison before saving."
                    .into(),
            );
        }
        Ok(())
    }
}

pub(super) struct Snapshot {
    token: String,
    ticket: u64,
    files: HashMap<PathBuf, Baseline>,
    in_flight: HashSet<PathBuf>,
    cancelled: bool,
}
impl Snapshot {
    pub(super) fn read(paths: &[String], ticket: u64) -> Result<Self, String> {
        if paths.is_empty() {
            return Err("Select a track or the files of one album to compare.".into());
        }
        let mut files = HashMap::new();
        for path in paths {
            let path = PathBuf::from(path);
            if files.contains_key(&path) {
                return Err("A file appears more than once in the comparison selection.".into());
            }
            let baseline = Baseline::read(&path)?;
            files.insert(path, baseline);
        }
        Ok(Self {
            token: uuid::Uuid::new_v4().to_string(),
            ticket,
            files,
            in_flight: HashSet::new(),
            cancelled: false,
        })
    }
}

#[derive(Default)]
pub(super) struct SnapshotStore {
    active: Option<Snapshot>,
    cancel_before: u64,
}
impl SnapshotStore {
    pub(super) fn install(&mut self, snapshot: Snapshot) -> Result<String, String> {
        if snapshot.ticket < self.cancel_before
            || self
                .active
                .as_ref()
                .is_some_and(|active| active.ticket > snapshot.ticket)
        {
            return Err(
                "This comparison was cancelled before its metadata finished loading.".into(),
            );
        }
        if self
            .active
            .as_ref()
            .is_some_and(|active| !active.in_flight.is_empty())
        {
            return Err(
                "Wait for the current metadata save before starting another comparison.".into(),
            );
        }
        let token = snapshot.token.clone();
        self.active = Some(snapshot);
        Ok(token)
    }
    pub(super) fn begin(&mut self, token: &str, path: &Path) -> Result<Baseline, String> {
        let active = self
            .active
            .as_mut()
            .filter(|snapshot| snapshot.token == token && !snapshot.cancelled)
            .ok_or("This comparison is no longer active. Open it again before saving.")?;
        let baseline = active
            .files
            .get(path)
            .ok_or("This file was not in the reviewed comparison, or was already saved.")?
            .clone();
        if !active.in_flight.insert(path.to_owned()) {
            return Err("This file is already being saved by the comparison.".into());
        }
        Ok(baseline)
    }
    pub(super) fn finish(&mut self, token: &str, path: &Path, success: bool) {
        if let Some(active) = self
            .active
            .as_mut()
            .filter(|snapshot| snapshot.token == token)
        {
            active.in_flight.remove(path);
            if success {
                active.files.remove(path);
            }
            if active.cancelled && active.in_flight.is_empty() {
                self.active = None;
            }
        }
    }
    pub(super) fn cancel(&mut self, ticket: u64) {
        self.cancel_before = self.cancel_before.max(ticket);
        if let Some(active) = self
            .active
            .as_mut()
            .filter(|snapshot| snapshot.ticket < ticket)
        {
            active.cancelled = true;
            if active.in_flight.is_empty() {
                self.active = None;
            }
        }
    }
    fn current(&self, token: &str, path: &Path) -> bool {
        self.active.as_ref().is_some_and(|active| {
            active.token == token && !active.cancelled && active.in_flight.contains(path)
        })
    }
}

static STORE: Mutex<SnapshotStore> = Mutex::new(SnapshotStore {
    active: None,
    cancel_before: 0,
});
const UNAVAILABLE: &str = "Comparison state is unavailable. Close and reopen the comparison.";
pub(super) fn capture_view(
    paths: &[String],
    ticket: u64,
) -> Result<ComparisonSnapshotView, String> {
    // Do file I/O outside the lock so closing a pending comparison stays responsive.
    let snapshot = Snapshot::read(paths, ticket)?;
    let files = paths
        .iter()
        .map(|path| {
            let state = &snapshot.files[Path::new(path)].state;
            ComparisonFileView {
                path: path.clone(),
                fields: state.fields.clone(),
                duration: f64::from_bits(state.media.1),
                covers: state
                    .covers
                    .iter()
                    .map(|cover| ComparisonCoverView {
                        digest: cover.digest.clone(),
                        description: cover.description.clone(),
                    })
                    .collect(),
            }
        })
        .collect();
    let token = STORE
        .lock()
        .map_err(|_| UNAVAILABLE.to_string())?
        .install(snapshot)?;
    Ok(ComparisonSnapshotView { token, files })
}
#[cfg(test)]
pub(super) fn capture(paths: &[String], ticket: u64) -> Result<String, String> {
    capture_view(paths, ticket).map(|view| view.token)
}
pub(super) fn begin(token: &str, path: &Path) -> Result<Baseline, String> {
    STORE
        .lock()
        .map_err(|_| UNAVAILABLE.to_string())?
        .begin(token, path)
}
pub(super) fn current(token: &str, path: &Path) -> Result<(), String> {
    if STORE
        .lock()
        .map_err(|_| UNAVAILABLE.to_string())?
        .current(token, path)
    {
        Ok(())
    } else {
        Err("This comparison was cancelled before the file was written.".into())
    }
}
pub(super) fn finish(token: &str, path: &Path, success: bool) {
    if let Ok(mut store) = STORE.lock() {
        store.finish(token, path, success);
    }
}
pub(super) fn cancel(ticket: u64) {
    if let Ok(mut store) = STORE.lock() {
        store.cancel(ticket);
    }
}
