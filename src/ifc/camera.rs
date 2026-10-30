//! Orbit-camera state.
//!
//! The pose a shared viewer state ([`super::share::ViewerState`])
//! carries: a target point, an orbit distance and yaw/pitch about it,
//! and the projection's field of view and clip planes. The viewer's own
//! camera controls move it.
//!
//! Conventions:
//!
//! - Right-handed coordinates, `+Z` up (matches Revit / IFC).
//! - Distances in caller units (feet by convention).
//! - Angles in radians throughout. `yaw` is rotation about `+Z`,
//!   `pitch` is elevation above the horizon.
//!
//! A `CameraState` is fully serializable, as part of a `ViewerState`.
// Tracker: VW1-07.

use serde::{Deserialize, Serialize};

/// Orbit-camera state. Describes the camera as a
/// target point + orbit distance + yaw/pitch rotation about
/// that target.
// Tracker: VW1-07.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct CameraState {
    /// Point the camera is looking at, in world space (feet).
    pub target: [f64; 3],
    /// Distance from target to the camera eye, in feet. Zoom in
    /// by decreasing; zoom out by increasing.
    pub distance: f64,
    /// Rotation about +Z (yaw) in radians. 0 = looking along +X.
    pub yaw: f64,
    /// Rotation above the horizon (pitch) in radians. 0 =
    /// horizon, π/2 = looking straight down, -π/2 = looking up.
    pub pitch: f64,
    /// Vertical field-of-view in radians (for perspective).
    /// Orthographic projections ignore this.
    pub fov_radians: f64,
    /// Near clip plane distance (feet).
    pub near: f64,
    /// Far clip plane distance (feet).
    pub far: f64,
}

impl Default for CameraState {
    /// Default pose: looking at the origin from a comfortable
    /// isometric angle, 60° FOV, near/far tuned for typical
    /// building-scale scenes (0.5 ft to 10 000 ft).
    fn default() -> Self {
        Self {
            target: [0.0, 0.0, 0.0],
            distance: 50.0,
            yaw: std::f64::consts::FRAC_PI_4,
            pitch: std::f64::consts::FRAC_PI_6,
            fov_radians: std::f64::consts::FRAC_PI_3, // 60°
            near: 0.5,
            far: 10_000.0,
        }
    }
}
