//! Orbit / pan / zoom camera state.
//!
//! A minimal camera state struct the viewer's input handler
//! manipulates, and a deterministic projection matrix computed
//! from it. The viewer layer (Three.js or WebGL) consumes the
//! matrix directly — the Rust side keeps the state as the single
//! source of truth so "reset view" / "share URL" behave
//! consistently.
//!
//! Conventions:
//!
//! - Right-handed coordinates, `+Z` up (matches Revit / IFC).
//! - Distances in caller units (feet by convention).
//! - Angles in radians throughout. `yaw` is rotation about `+Z`,
//!   `pitch` is elevation above the horizon.
//!
//! A `CameraState` is fully serializable so the "share URL" flow
//! just has to base64-encode the JSON.
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

impl CameraState {
    /// World-space eye position derived from `target + orbit`.
    pub fn eye(&self) -> [f64; 3] {
        let (sy, cy) = self.yaw.sin_cos();
        let (sp, cp) = self.pitch.sin_cos();
        [
            self.target[0] + self.distance * cp * cy,
            self.target[1] + self.distance * cp * sy,
            self.target[2] + self.distance * sp,
        ]
    }

    /// Orbit by `delta_yaw` and `delta_pitch` radians.
    /// Pitch is clamped to (`-π/2 + ε`, `π/2 - ε`) so the
    /// camera never looks through its own axis.
    // Tracker: VW1-07.
    pub fn orbit(&mut self, delta_yaw: f64, delta_pitch: f64) {
        self.yaw += delta_yaw;
        let eps = 1e-3;
        self.pitch = (self.pitch + delta_pitch).clamp(
            -std::f64::consts::FRAC_PI_2 + eps,
            std::f64::consts::FRAC_PI_2 - eps,
        );
    }

    /// Pan the target by `(dx, dy, dz)` feet. Moves the
    /// target and the eye in lockstep; distance + orientation
    /// unchanged.
    // Tracker: VW1-07.
    pub fn pan(&mut self, delta: [f64; 3]) {
        self.target[0] += delta[0];
        self.target[1] += delta[1];
        self.target[2] += delta[2];
    }

    /// Zoom by multiplying distance by `factor`.
    /// `factor > 1` zooms out, `factor < 1` zooms in. Clamps to
    /// `near` as a lower bound and `far` as an upper bound so the
    /// camera never crosses its own clip planes.
    // Tracker: VW1-07.
    pub fn zoom(&mut self, factor: f64) {
        let f = factor.max(0.0);
        self.distance = (self.distance * f).clamp(self.near * 2.0, self.far * 0.5);
    }

    /// Set the target directly. Useful for
    /// "double-click to focus" flows.
    // Tracker: VW1-07.
    pub fn focus_on(&mut self, target: [f64; 3]) {
        self.target = target;
    }

    /// Frame a bounding box so it fills the viewport.
    /// Places the target at the box center and sets the distance
    /// to cover the box's diagonal with the camera's current FOV.
    // Tracker: VW1-07.
    pub fn frame_bbox(&mut self, min: [f64; 3], max: [f64; 3]) {
        self.target = [
            (min[0] + max[0]) * 0.5,
            (min[1] + max[1]) * 0.5,
            (min[2] + max[2]) * 0.5,
        ];
        let dx = max[0] - min[0];
        let dy = max[1] - min[1];
        let dz = max[2] - min[2];
        let diag = (dx * dx + dy * dy + dz * dz).sqrt();
        // Distance = (diag/2) / tan(fov/2) with a safety factor.
        let half_fov = (self.fov_radians * 0.5).max(1e-3);
        let d = (diag * 0.5) / half_fov.tan();
        self.distance = d.max(self.near * 2.0).min(self.far * 0.5).max(1.0);
    }
}
