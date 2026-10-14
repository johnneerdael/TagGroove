//! What a staged write physically replaced, for undo journal accounting.
//!
//! A staged write clones the original, edits the clone and renames it over the
//! original. The journal keeps the old file, which then holds on only to the
//! blocks the edit rewrote. Writers that rewrite a known region in place
//! report it with [`rewrote_region`]; every other writer reports
//! [`rewrote_all`]. A write that reports nothing counts as a full rewrite.
use std::cell::Cell;

#[derive(Clone, Copy, Default)]
struct Rewrite {
    region: u64,
    full: bool,
}

thread_local! {
    static ACTIVE: Cell<Option<Rewrite>> = const { Cell::new(None) };
}

fn update(change: impl FnOnce(&mut Rewrite)) {
    ACTIVE.with(|active| {
        if let Some(mut rewrite) = active.get() {
            change(&mut rewrite);
            active.set(Some(rewrite));
        }
    });
}

/// `bytes` were written over existing content, nothing else moved.
pub(crate) fn rewrote_region(bytes: u64) {
    update(|r| r.region = r.region.saturating_add(bytes));
}

/// The file was rewritten or its content shifted.
pub(crate) fn rewrote_all() {
    update(|r| r.full = true);
}

/// Tracks one staged write on the current thread; dropping it unfinished
/// (an error return) restores the enclosing state.
pub(crate) struct Scope {
    outer: Option<Rewrite>,
    finished: bool,
}

impl Scope {
    pub(crate) fn begin() -> Self {
        Self {
            outer: ACTIVE.with(|active| active.replace(Some(Rewrite::default()))),
            finished: false,
        }
    }
    /// Bytes rewritten in place, or `None` for a full (or unreported) rewrite.
    pub(crate) fn finish(mut self) -> Option<u64> {
        self.finished = true;
        let rewrite = ACTIVE.with(|active| active.replace(self.outer));
        rewrite
            .filter(|r| !r.full && r.region > 0)
            .map(|r| r.region)
    }
}

impl Drop for Scope {
    fn drop(&mut self) {
        if !self.finished {
            ACTIVE.with(|active| active.set(self.outer));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_pure_region_writes_count_as_partial() {
        let scope = Scope::begin();
        rewrote_region(4096);
        rewrote_region(100);
        assert_eq!(scope.finish(), Some(4196));
        let scope = Scope::begin();
        rewrote_region(4096);
        rewrote_all();
        assert_eq!(scope.finish(), None);
        assert_eq!(Scope::begin().finish(), None, "unreported writes are full");
        // Reports outside a scope, and after an abandoned scope, change nothing.
        rewrote_region(1);
        drop(Scope::begin());
        rewrote_all();
        let scope = Scope::begin();
        rewrote_region(10);
        assert_eq!(scope.finish(), Some(10));
    }
}
