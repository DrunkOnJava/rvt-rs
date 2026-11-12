//! The work budgets a reader stopped at, for the export's diagnostics.
//!
//! A reader that counts its work against a budget (#1833, #1845) leaves out
//! what is past it, as an unread stream leaves out what it holds. No real
//! model is known to reach one; a file that does is either built to make the
//! reader do unbounded work or larger than any measured, and either way what
//! was left out must show (`ifc::record_work_budget_stops`), not go quietly.
//!
//! The readers that stop are deep in the format and hold no handle, so each
//! notes its stop on its thread ([`stopped`]). The code that holds the file
//! collects what was noted under it ([`Notes`]) and keeps it with the file,
//! as the file keeps its unread streams: around the walk, whose elements the
//! file memoises, and around an export. A stop kept with the file still shows
//! when a later export reads the memoised walk back.
//!
//! A note made under no [`Notes`] is dropped: that of a reader called on its
//! own, and that of a reader on another thread, such as the saved meshes'
//! workers. Only the thread an export runs on reports; a reader on a worker
//! has to carry its stop back in its result.

use std::cell::RefCell;
use std::collections::BTreeMap;

/// The readers that stopped, each with how often.
pub(crate) type Stops = BTreeMap<&'static str, usize>;

thread_local! {
    /// The notes of the innermost [`Notes`] open on this thread.
    static OPEN: RefCell<Option<Stops>> = const { RefCell::new(None) };
}

/// Note that the reader `what` stopped at its work budget, once for each
/// stop, and left out what was past it.
pub(crate) fn stopped(what: &'static str) {
    OPEN.with(|open| {
        if let Some(stops) = open.borrow_mut().as_mut() {
            *stops.entry(what).or_insert(0) += 1;
        }
    });
}

/// The stops noted on this thread from [`Notes::open`] to [`Notes::close`].
/// A [`Notes`] opened inside another takes what is noted while it is open,
/// and the outer one does not see it again.
#[must_use = "the stops are collected for the file that keeps them"]
pub(crate) struct Notes {
    /// What was open on the thread before, put back on close, or on drop
    /// when the work returned early (its notes are then dropped).
    outer: Option<Option<Stops>>,
}

impl Notes {
    /// Collect the stops noted on this thread from now.
    pub(crate) fn open() -> Self {
        let outer = OPEN.with(|open| open.replace(Some(Stops::new())));
        Self { outer: Some(outer) }
    }

    /// The stops noted since [`Notes::open`], and the notes open before it
    /// open again.
    pub(crate) fn close(mut self) -> Stops {
        let outer = self.outer.take().flatten();
        OPEN.with(|open| open.replace(outer)).unwrap_or_default()
    }
}

impl Drop for Notes {
    fn drop(&mut self) {
        if let Some(outer) = self.outer.take() {
            OPEN.with(|open| *open.borrow_mut() = outer);
        }
    }
}
