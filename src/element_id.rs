//! Revit's `ElementId`: an element's id in its document.

use std::fmt;

/// An element's id in its Revit document, the value the Revit API's
/// `ElementId.Value` gives: a signed 64-bit integer since Revit 2024.
///
/// Every id rvt-rs reads from a file is positive. The type is signed because
/// Revit's own ids are: its invalid id is -1, and built-in categories and
/// parameters have negative ids. It serialises as the bare number, so JSON
/// and Python see the same integer the file holds.
///
/// ```
/// use rvt::ElementId;
///
/// let id = ElementId::from(392_844_u32);
/// assert_eq!(id.get(), 392_844);
/// assert_eq!(id.to_string(), "392844");
/// // A 64-bit slot holding 0, or u32::MAX and above, names no element.
/// assert_eq!(ElementId::from_raw(0), None);
/// assert_eq!(ElementId::from_raw(u64::from(u32::MAX)), None);
/// assert_eq!(ElementId::from_raw(7), Some(ElementId::new(7)));
/// ```
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize,
)]
#[serde(transparent)]
pub struct ElementId(i64);

impl ElementId {
    /// The id with this value.
    pub const fn new(value: i64) -> Self {
        Self(value)
    }

    /// The id's value.
    pub const fn get(self) -> i64 {
        self.0
    }

    /// The id a file's 64-bit id slot holds, or `None` when the slot names no
    /// element: `0`, or a value at or above `u32::MAX`, which is the 32-bit
    /// form of Revit's invalid id -1 or a slot that never held an id
    /// (`reports/element-framing/RE-30` and `RE-43`).
    pub const fn from_raw(raw: u64) -> Option<Self> {
        if raw == 0 || raw >= u32::MAX as u64 {
            None
        } else {
            Some(Self(raw as i64))
        }
    }

    /// The id as the `u32` a file stores, for an id rvt-rs read from one;
    /// `None` for a negative id or one past `u32::MAX`.
    pub fn as_u32(self) -> Option<u32> {
        u32::try_from(self.0).ok()
    }
}

impl From<u32> for ElementId {
    fn from(id: u32) -> Self {
        Self(i64::from(id))
    }
}

impl fmt::Display for ElementId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}
