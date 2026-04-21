//! Orbit / pan / zoom camera state (VW1-07).
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

use serde::{Deserialize, Serialize};

/// Orbit-camera state (VW1-07). Describes the camera as a
/// target point + orbit distance + yaw/pitch rotation about
/// that target.
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

    /// Orbit by `delta_yaw` and `delta_pitch` radians (VW1-07).
    /// Pitch is clamped to (`-π/2 + ε`, `π/2 - ε`) so the
    /// camera never looks through its own axis.
    pub fn orbit(&mut self, delta_yaw: f64, delta_pitch: f64) {
        self.yaw += delta_yaw;
        let eps = 1e-3;
        self.pitch = (self.pitch + delta_pitch).clamp(
            -std::f64::consts::FRAC_PI_2 + eps,
            std::f64::consts::FRAC_PI_2 - eps,
        );
    }

    /// Pan the target by `(dx, dy, dz)` feet (VW1-07). Moves the
    /// target and the eye in lockstep; distance + orientation
    /// unchanged.
    pub fn pan(&mut self, delta: [f64; 3]) {
        self.target[0] += delta[0];
        self.target[1] += delta[1];
        self.target[2] += delta[2];
    }

    /// Zoom by multiplying distance by `factor` (VW1-07).
    /// `factor > 1` zooms out, `factor < 1` zooms in. Clamps to
    /// `near` as a lower bound and `far` as an upper bound so the
    /// camera never crosses its own clip planes.
    pub fn zoom(&mut self, factor: f64) {
        let f = factor.max(0.0);
        self.distance = (self.distance * f).clamp(self.near * 2.0, self.far * 0.5);
    }

    /// Set the target directly (VW1-07). Useful for
    /// "double-click to focus" flows.
    pub fn focus_on(&mut self, target: [f64; 3]) {
        self.target = target;
    }

    /// Frame a bounding box so it fills the viewport (VW1-07).
    /// Places the target at the box center and sets the distance
    /// to cover the box's diagonal with the camera's current FOV.
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_camera_is_isometric_looking_at_origin() {
        let c = CameraState::default();
        assert_eq!(c.target, [0.0, 0.0, 0.0]);
        assert_eq!(c.distance, 50.0);
        assert!(c.yaw > 0.0);
        assert!(c.pitch > 0.0);
    }

    #[test]
    fn eye_position_reflects_orbit() {
        let c = CameraState {
            target: [0.0, 0.0, 0.0],
            distance: 10.0,
            yaw: 0.0,
            pitch: 0.0,
            ..CameraState::default()
        };
        // yaw=0, pitch=0: eye should be on +X axis at distance 10.
        let eye = c.eye();
        assert!((eye[0] - 10.0).abs() < 1e-9);
        assert!(eye[1].abs() < 1e-9);
        assert!(eye[2].abs() < 1e-9);
    }

    #[test]
    fn eye_position_pitch_looking_down() {
        let c = CameraState {
            target: [0.0, 0.0, 0.0],
            distance: 10.0,
            yaw: 0.0,
            pitch: std::f64::consts::FRAC_PI_2,
            ..CameraState::default()
        };
        let eye = c.eye();
        // pitch=π/2: eye should be almost directly above the target.
        assert!(eye[2] > 9.0);
    }

    #[test]
    fn orbit_clamps_pitch_to_safe_bounds() {
        let mut c = CameraState {
            pitch: 0.0,
            ..CameraState::default()
        };
        c.orbit(0.0, 10.0); // try to over-rotate
        assert!(c.pitch < std::f64::consts::FRAC_PI_2);
        c.orbit(0.0, -20.0);
        assert!(c.pitch > -std::f64::consts::FRAC_PI_2);
    }

    #[test]
    fn orbit_accumulates_yaw() {
        let mut c = CameraState {
            yaw: 0.0,
            ..CameraState::default()
        };
        c.orbit(std::f64::consts::FRAC_PI_4, 0.0);
        assert!((c.yaw - std::f64::consts::FRAC_PI_4).abs() < 1e-9);
    }

    #[test]
    fn pan_translates_target() {
        let mut c = CameraState::default();
        let before = c.target;
        c.pan([1.0, 2.0, 3.0]);
        assert_eq!(c.target[0], before[0] + 1.0);
        assert_eq!(c.target[1], before[1] + 2.0);
        assert_eq!(c.target[2], before[2] + 3.0);
    }

    #[test]
    fn zoom_multiplies_distance() {
        let mut c = CameraState {
            distance: 10.0,
            near: 0.1,
            far: 1000.0,
            ..CameraState::default()
        };
        c.zoom(2.0);
        assert!((c.distance - 20.0).abs() < 1e-9);
        c.zoom(0.5);
        assert!((c.distance - 10.0).abs() < 1e-9);
    }

    #[test]
    fn zoom_clamps_to_near_lower_bound() {
        let mut c = CameraState {
            distance: 1.0,
            near: 0.5,
            far: 1000.0,
            ..CameraState::default()
        };
        c.zoom(0.001); // try to zoom way in
        assert!(c.distance >= c.near * 2.0);
    }

    #[test]
    fn zoom_clamps_to_far_upper_bound() {
        let mut c = CameraState {
            distance: 100.0,
            near: 0.5,
            far: 200.0,
            ..CameraState::default()
        };
        c.zoom(10.0); // way past far/2
        assert!(c.distance <= c.far * 0.5);
    }

    #[test]
    fn focus_on_sets_target_directly() {
        let mut c = CameraState::default();
        c.focus_on([100.0, 200.0, 30.0]);
        assert_eq!(c.target, [100.0, 200.0, 30.0]);
    }

    #[test]
    fn frame_bbox_centers_target() {
        let mut c = CameraState::default();
        c.frame_bbox([0.0, 0.0, 0.0], [10.0, 10.0, 10.0]);
        assert_eq!(c.target, [5.0, 5.0, 5.0]);
    }

    #[test]
    fn frame_bbox_adjusts_distance_to_fit() {
        let mut c = CameraState::default();
        let before = c.distance;
        c.frame_bbox([0.0, 0.0, 0.0], [1000.0, 1000.0, 1000.0]);
        // Large box ⇒ larger distance.
        assert!(c.distance > before);
    }

    #[test]
    fn camera_state_is_serde_roundtrippable() {
        // Use values representable exactly in f64/JSON so the
        // round-trip is bit-exact. Default uses transcendentals
        // (π/4 etc.) which JSON's shortest-roundtrip representation
        // may lose in the last bit.
        let c = CameraState {
            target: [1.0, 2.0, 3.0],
            distance: 25.0,
            yaw: 0.5,
            pitch: 0.25,
            fov_radians: 1.0,
            near: 0.5,
            far: 1000.0,
        };
        let json = serde_json::to_string(&c).unwrap();
        let back: CameraState = serde_json::from_str(&json).unwrap();
        assert_eq!(back, c);
    }

    #[test]
    fn camera_state_serde_preserves_fields_with_tolerance() {
        // Default uses transcendental values; check field-wise with
        // tolerance rather than exact equality.
        let c = CameraState::default();
        let json = serde_json::to_string(&c).unwrap();
        let back: CameraState = serde_json::from_str(&json).unwrap();
        assert_eq!(back.target, c.target);
        assert_eq!(back.distance, c.distance);
        assert!((back.yaw - c.yaw).abs() < 1e-12);
        assert!((back.pitch - c.pitch).abs() < 1e-12);
        assert!((back.fov_radians - c.fov_radians).abs() < 1e-12);
    }
}
