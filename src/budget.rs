//! The work budgets a reader stopped at, for the export's diagnostics.
//!
//! A reader that counts its work against a budget (#1833, #1845) leaves out
//! what is past it, as an unread stream leaves out what it holds. No real
//! model is known to reach one; a file that does is either built to make the
//! reader do unbounded work or larger than any measured, and either way what
//! was left out must show (`ifc::record_unread_streams`), not go quietly.
//!
//! The readers that stop are deep in the format and hold no handle to report
//! through, so each notes its stop here, per thread: the export reads on one.

use std::cell::RefCell;
use std::collections::BTreeMap;

thread_local! {
    static STOPPED: RefCell<BTreeMap<&'static str, usize>> = const { RefCell::new(BTreeMap::new()) };
}

/// Note that the reader `what` stopped at its work budget, once for each
/// stop, and left out what was past it.
pub(crate) fn stopped(what: &'static str) {
    STOPPED.with(|stopped| *stopped.borrow_mut().entry(what).or_insert(0) += 1);
}

/// Forget every stop: an export starts from none.
pub(crate) fn reset() {
    STOPPED.with(|stopped| stopped.borrow_mut().clear());
}

/// Every reader that stopped since the last [`reset`], with how often.
pub(crate) fn stopped_so_far() -> BTreeMap<&'static str, usize> {
    STOPPED.with(|stopped| stopped.borrow().clone())
}
