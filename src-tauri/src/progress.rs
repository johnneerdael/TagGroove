//! Progress and cancellation for long file operations (reading folders, saving
//! many files). The UI passes a job id, listens for [`EVENT`] and may call
//! [`cancel_job`]; work checks `proceed` before each file, so files that
//! were already written stay written and the rest are left untouched.
use serde::Serialize;
use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Arc, Mutex,
    },
    time::{Duration, Instant},
};
use tauri::Emitter;

pub const EVENT: &str = "opentag://progress";
const INTERVAL: Duration = Duration::from_millis(50);

static JOBS: Mutex<Option<HashMap<String, Arc<AtomicBool>>>> = Mutex::new(None);

#[derive(Clone, Serialize)]
struct Update<'a> {
    job: &'a str,
    done: usize,
    total: usize,
}

/// A registered job; dropping it unregisters the id.
pub struct Job {
    id: Option<String>,
    cancelled: Arc<AtomicBool>,
    done: AtomicUsize,
    total: AtomicUsize,
    last: Mutex<Option<Instant>>,
}

impl Job {
    /// Register `id` (when the caller asked for progress) for cancellation.
    pub fn new(id: Option<String>) -> Self {
        let cancelled = Arc::new(AtomicBool::new(false));
        if let Some(id) = &id {
            if let Ok(mut jobs) = JOBS.lock() {
                jobs.get_or_insert_with(HashMap::new)
                    .insert(id.clone(), cancelled.clone());
            }
        }
        Self {
            id,
            cancelled,
            done: AtomicUsize::new(0),
            total: AtomicUsize::new(0),
            last: Mutex::new(None),
        }
    }

    fn emit(&self, done: usize, force: bool) {
        let Some(id) = &self.id else { return };
        {
            let Ok(mut last) = self.last.lock() else {
                return;
            };
            if !force && last.is_some_and(|at| at.elapsed() < INTERVAL) {
                return;
            }
            *last = Some(Instant::now());
        }
        if let Some(app) = crate::APP_HANDLE.get() {
            let total = self.total.load(Ordering::Relaxed);
            let _ = app.emit(
                EVENT,
                Update {
                    job: id,
                    done,
                    total,
                },
            );
        }
    }
}

impl tag_core::parallel::Progress for Job {
    fn start(&self, total: usize) {
        self.total.store(total, Ordering::Relaxed);
        self.emit(0, true);
    }
    fn proceed(&self) -> bool {
        !self.cancelled.load(Ordering::Relaxed)
    }
    /// Events are throttled; the last item always reports.
    fn advance(&self) {
        let done = self.done.fetch_add(1, Ordering::Relaxed) + 1;
        self.emit(done, done == self.total.load(Ordering::Relaxed));
    }
}

impl Drop for Job {
    fn drop(&mut self) {
        if let (Some(id), Ok(mut jobs)) = (&self.id, JOBS.lock()) {
            if let Some(jobs) = jobs.as_mut() {
                jobs.remove(id);
            }
        }
    }
}

/// Ask a running job to stop after the items already in progress.
#[tauri::command]
pub fn cancel_job(job: String) {
    if let Ok(jobs) = JOBS.lock() {
        if let Some(flag) = jobs.as_ref().and_then(|jobs| jobs.get(&job)) {
            flag.store(true, Ordering::Relaxed);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tag_core::parallel::map_ordered;

    #[test]
    fn cancelled_job_starts_no_items() {
        let job = Job::new(Some("test-cancel".into()));
        cancel_job("test-cancel".into());
        let items: Vec<usize> = (0..50).collect();
        assert!(map_ordered(&items, &job, |n| *n)
            .iter()
            .all(Option::is_none));
        drop(job);
        assert!(!JOBS
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .contains_key("test-cancel"));
    }
}
