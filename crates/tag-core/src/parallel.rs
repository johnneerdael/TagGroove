//! Ordered parallel work over files with progress and cancellation.
use std::sync::atomic::{AtomicUsize, Ordering};

/// Progress sink for long operations. `proceed` is checked before each item.
pub trait Progress: Sync {
    fn start(&self, _total: usize) {}
    fn proceed(&self) -> bool {
        true
    }
    fn advance(&self) {}
}

/// No progress reporting and no cancellation.
pub struct Silent;
impl Progress for Silent {}

/// Run `work` over `items` on up to eight threads, keeping input order in the
/// output. Items not started before a cancellation yield `None`. A panic in
/// `work` is propagated to the caller.
pub fn map_ordered<T: Sync, R: Send>(
    items: &[T],
    progress: &dyn Progress,
    work: impl Fn(&T) -> R + Sync,
) -> Vec<Option<R>> {
    let threads = std::thread::available_parallelism().map_or(4, |n| n.get());
    map_ordered_on(threads.clamp(1, 8), items, progress, work)
}

/// [`map_ordered`] with an explicit thread count; 1 processes items in order.
pub fn map_ordered_on<T: Sync, R: Send>(
    threads: usize,
    items: &[T],
    progress: &dyn Progress,
    work: impl Fn(&T) -> R + Sync,
) -> Vec<Option<R>> {
    let next = AtomicUsize::new(0);
    let threads = threads.clamp(1, items.len().max(1));
    let finished: Vec<Vec<(usize, R)>> = std::thread::scope(|scope| {
        let workers: Vec<_> = (0..threads)
            .map(|_| {
                scope.spawn(|| {
                    let mut done = Vec::new();
                    while progress.proceed() {
                        let index = next.fetch_add(1, Ordering::Relaxed);
                        let Some(item) = items.get(index) else { break };
                        done.push((index, work(item)));
                        progress.advance();
                    }
                    done
                })
            })
            .collect();
        workers
            .into_iter()
            .map(|worker| {
                worker
                    .join()
                    .unwrap_or_else(|panic| std::panic::resume_unwind(panic))
            })
            .collect()
    });
    let mut results: Vec<Option<R>> = (0..items.len()).map(|_| None).collect();
    for (index, result) in finished.into_iter().flatten() {
        results[index] = Some(result);
    }
    results
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicBool;

    struct StopAfter(AtomicUsize, usize, AtomicBool);
    impl Progress for StopAfter {
        fn proceed(&self) -> bool {
            !self.2.load(Ordering::SeqCst)
        }
        fn advance(&self) {
            if self.0.fetch_add(1, Ordering::SeqCst) + 1 >= self.1 {
                self.2.store(true, Ordering::SeqCst);
            }
        }
    }

    #[test]
    fn keeps_input_order() {
        let items: Vec<usize> = (0..500).collect();
        let out = map_ordered(&items, &Silent, |n| n * 2);
        assert_eq!(
            out.into_iter().map(Option::unwrap).collect::<Vec<_>>(),
            (0..500).map(|n| n * 2).collect::<Vec<_>>()
        );
    }

    #[test]
    fn cancellation_leaves_remaining_items_unstarted() {
        let items: Vec<usize> = (0..500).collect();
        let stop = StopAfter(AtomicUsize::new(0), 10, AtomicBool::new(false));
        let out = map_ordered(&items, &stop, |n| *n);
        let done = out.iter().filter(|r| r.is_some()).count();
        // Workers already running may each finish one more item.
        assert!((10..=18).contains(&done), "{done} items ran");
    }
}
