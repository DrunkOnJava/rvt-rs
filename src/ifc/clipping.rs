//! Clipping planes + view modes — spatial culling
//! primitives and the viewer's plan/3D/section state machine.
//!
//! A `ClippingPlane` is an oriented half-space: `dot(point - origin,
//! normal) >= 0` is "kept," negative is "clipped." A `SectionBox` is
//! an axis-aligned bounding box; anything strictly outside it is
//! clipped.
//!
//! Viewers combine these: a user draws a section box to isolate a
//! room; the scene graph is re-emitted with every element's
//! location tested against the box. Elements wholly inside are
//! passed through, elements wholly outside are pruned, elements
//! that straddle the box boundary are kept (the viewer's mesh
//! clipper handles the pixel-level cut).
//!
//! The math here is kept pure-Rust and no-deps so the same logic
//! runs identically on native and WASM targets.
// Tracker: VW1-14, VW1-10.

use serde::{Deserialize, Serialize};

/// An oriented half-space. A point `p` is **kept** when
/// `(p - origin) · normal >= 0`; **clipped** otherwise.
///
/// `normal` is interpreted as pointing into the visible half-space.
/// Does not need to be unit-length — the sign of the dot product is
/// what matters — but viewers that render the clip indicator line
/// may prefer unit normals.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ClippingPlane {
    pub origin: [f64; 3],
    pub normal: [f64; 3],
}

impl ClippingPlane {
    /// Evaluate the plane at `point`. Returns the signed distance
    /// along `normal` — positive = kept half-space, negative =
    /// clipped, zero = on the plane itself.
    pub fn signed_distance(&self, point: [f64; 3]) -> f64 {
        let dx = point[0] - self.origin[0];
        let dy = point[1] - self.origin[1];
        let dz = point[2] - self.origin[2];
        dx * self.normal[0] + dy * self.normal[1] + dz * self.normal[2]
    }

    /// `true` when `point` lies in the kept half-space (or on the
    /// plane itself — the boundary is inclusive).
    pub fn contains(&self, point: [f64; 3]) -> bool {
        self.signed_distance(point) >= 0.0
    }

    /// Convenience constructor for a horizontal "floor-up" plane
    /// at elevation `z`. Kept half-space is `z' >= z`. Matches the
    /// typical viewer behaviour when the user drags a horizontal
    /// cut line.
    pub fn horizontal_cut(z_feet: f64) -> Self {
        Self {
            origin: [0.0, 0.0, z_feet],
            normal: [0.0, 0.0, 1.0],
        }
    }

    /// Convenience constructor for a vertical plane through
    /// `(x_origin, y_origin)` facing `+X` (keep everything to the
    /// east / right). Callers who need an arbitrary azimuth build
    /// the plane manually.
    pub fn vertical_east(x_origin: f64) -> Self {
        Self {
            origin: [x_origin, 0.0, 0.0],
            normal: [1.0, 0.0, 0.0],
        }
    }
}

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

impl SectionBox {
    /// New section box from explicit min/max corners. `min` and
    /// `max` are normalised per-axis so callers can pass either
    /// corner pair in any order.
    pub fn new(a: [f64; 3], b: [f64; 3]) -> Self {
        Self {
            min: [a[0].min(b[0]), a[1].min(b[1]), a[2].min(b[2])],
            max: [a[0].max(b[0]), a[1].max(b[1]), a[2].max(b[2])],
        }
    }

    /// Infinite box — always contains everything. Useful as a
    /// no-op starting state before the user draws a real box.
    pub fn infinite() -> Self {
        Self {
            min: [f64::NEG_INFINITY; 3],
            max: [f64::INFINITY; 3],
        }
    }

    /// `true` when `point` is inside (or on the boundary of) this
    /// box.
    pub fn contains(&self, point: [f64; 3]) -> bool {
        point
            .iter()
            .zip(self.min.iter().zip(self.max.iter()))
            .all(|(p, (lo, hi))| p >= lo && p <= hi)
    }

    /// Expand this box to include `point`. No-op when the point is
    /// already inside.
    pub fn expand_to(&mut self, point: [f64; 3]) {
        for (i, p) in point.iter().enumerate() {
            if *p < self.min[i] {
                self.min[i] = *p;
            }
            if *p > self.max[i] {
                self.max[i] = *p;
            }
        }
    }

    /// Width/depth/height of the box (may be 0 along an axis when
    /// the box is degenerate).
    pub fn size(&self) -> [f64; 3] {
        [
            (self.max[0] - self.min[0]).max(0.0),
            (self.max[1] - self.min[1]).max(0.0),
            (self.max[2] - self.min[2]).max(0.0),
        ]
    }

    /// Center point.
    pub fn center(&self) -> [f64; 3] {
        [
            (self.min[0] + self.max[0]) * 0.5,
            (self.min[1] + self.max[1]) * 0.5,
            (self.min[2] + self.max[2]) * 0.5,
        ]
    }
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

impl ViewMode {
    /// Default section box for this mode. `Plan` slices
    /// to a horizontal band at the storey elevation ± 10 ft;
    /// `Section` returns a vertical strip along the X axis;
    /// `ThreeD` returns the infinite box (no spatial clipping).
    ///
    /// Callers override with their own SectionBox when the user
    /// draws an explicit cut.
    // Tracker: VW1-10.
    pub fn default_section_box(
        self,
        storey_elevation_feet: f64,
        model_bbox: SectionBox,
    ) -> SectionBox {
        match self {
            ViewMode::Plan => {
                let z = storey_elevation_feet;
                SectionBox {
                    min: [model_bbox.min[0], model_bbox.min[1], z - 0.5],
                    max: [model_bbox.max[0], model_bbox.max[1], z + 10.0],
                }
            }
            ViewMode::Section => SectionBox {
                min: [model_bbox.min[0], -0.5, model_bbox.min[2]],
                max: [model_bbox.max[0], 0.5, model_bbox.max[2]],
            },
            ViewMode::ThreeD => SectionBox::infinite(),
        }
    }

    /// Whether this view uses an orthographic camera. `Plan` and
    /// `Section` do; `ThreeD` uses perspective.
    pub fn is_orthographic(self) -> bool {
        matches!(self, ViewMode::Plan | ViewMode::Section)
    }

    /// Human-readable label for menus / toolbars.
    pub fn label(self) -> &'static str {
        match self {
            ViewMode::Plan => "Plan",
            ViewMode::ThreeD => "3D",
            ViewMode::Section => "Section",
        }
    }
}

impl Default for ViewMode {
    /// 3D is the safest "always looks like something" default.
    fn default() -> Self {
        Self::ThreeD
    }
}
