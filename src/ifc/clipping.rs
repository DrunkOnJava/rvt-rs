//! Section box and view mode, as a shared viewer state carries them.
//!
//! A `SectionBox` is an axis-aligned bounding box; anything strictly
//! outside it is clipped. A `ViewMode` is the viewer's plan / 3D /
//! section choice. Both are plain serializable data that
//! [`super::share::ViewerState`] stores.
// Tracker: VW1-14, VW1-10.

use serde::{Deserialize, Serialize};

/// Axis-aligned section box. Viewers expose this to the
/// user as a draggable box in the 3D canvas; elements outside the
/// box are culled from the rendered scene.
///
/// The box is inclusive on all boundaries — a point exactly on a
/// face is inside.
// Tracker: VW1-14.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SectionBox {
    pub min: [f64; 3],
    pub max: [f64; 3],
}

/// Viewer camera / projection mode. Viewers bind this to
/// a segmented-control UI; each mode dictates which clipping +
/// projection defaults apply.
// Tracker: VW1-10.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum ViewMode {
    /// Orthographic top-down plan view — typical default for
    /// floor plans. Usually paired with a horizontal `z_min <= z
    /// <= z_max` section box at the chosen storey.
    Plan,
    /// Perspective 3D view. Free-flying camera.
    ThreeD,
    /// Vertical elevation / section — orthographic camera facing
    /// along the Y axis (or rotated via separate state).
    Section,
}

impl Default for ViewMode {
    /// 3D is the safest "always looks like something" default.
    fn default() -> Self {
        Self::ThreeD
    }
}
