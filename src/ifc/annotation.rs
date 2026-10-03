//! Annotation overlay (VW1-12) — user-added markups rendered on top
//! of the scene graph.
//!
//! Four annotation types cover the common viewer needs:
//!
//! - Note:  a text label pinned to a world-space anchor
//! - Leader: an arrow from a world-space anchor to a text label
//! - Polyline: a multi-segment line (e.g. a freehand markup)
//! - Pin: a single-point pin (defect marker / RFI)
//!
//! Annotations carry a UUID-style id (generated deterministically
//! from a counter + hash of the payload), a created-at timestamp
//! in ISO-8601 UTC, and an author string so a collaborative
//! viewer can attribute them.
//!
//! The whole `AnnotationLayer` is serde-serializable and fits
//! naturally inside the `ViewerState` URL share payload (VW1-24)
//! when the state is small — larger markup sets should persist
//! out-of-band.

use serde::{Deserialize, Serialize};

/// 3D anchor in world space (feet).
pub type Anchor = [f64; 3];

/// Single annotation variant (VW1-12).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Annotation {
    /// Text label pinned to an anchor point.
    Note {
        id: String,
        anchor: Anchor,
        text: String,
        author: Option<String>,
        created_iso: Option<String>,
    },
    /// Leader line with arrowhead + text label at the anchor end.
    Leader {
        id: String,
        anchor: Anchor,
        label_anchor: Anchor,
        text: String,
        author: Option<String>,
        created_iso: Option<String>,
    },
    /// Multi-segment polyline — viewer draws as connected line
    /// segments. `vertices` must contain at least 2 points.
    Polyline {
        id: String,
        vertices: Vec<Anchor>,
        author: Option<String>,
        created_iso: Option<String>,
    },
    /// Single-point pin (defect marker / RFI bubble).
    Pin {
        id: String,
        anchor: Anchor,
        category: Option<String>,
        author: Option<String>,
        created_iso: Option<String>,
    },
}

impl Annotation {
    /// Every annotation variant carries an `id` — expose it
    /// uniformly without pattern-matching.
    pub fn id(&self) -> &str {
        match self {
            Annotation::Note { id, .. }
            | Annotation::Leader { id, .. }
            | Annotation::Polyline { id, .. }
            | Annotation::Pin { id, .. } => id,
        }
    }

    /// Kind-name for display in UI lists.
    pub fn kind_name(&self) -> &'static str {
        match self {
            Annotation::Note { .. } => "note",
            Annotation::Leader { .. } => "leader",
            Annotation::Polyline { .. } => "polyline",
            Annotation::Pin { .. } => "pin",
        }
    }
}

/// Ordered collection of annotations (VW1-12). The viewer renders
/// them in list order on top of the scene.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct AnnotationLayer {
    pub annotations: Vec<Annotation>,
}

impl AnnotationLayer {
    /// New empty layer.
    pub fn new() -> Self {
        Self::default()
    }

    /// Append an annotation.
    pub fn push(&mut self, annotation: Annotation) {
        self.annotations.push(annotation);
    }

    /// Remove the annotation with the given id. Returns `true`
    /// when an entry was removed.
    pub fn remove_by_id(&mut self, id: &str) -> bool {
        let len = self.annotations.len();
        self.annotations.retain(|a| a.id() != id);
        self.annotations.len() != len
    }

    /// Find an annotation by id.
    pub fn find(&self, id: &str) -> Option<&Annotation> {
        self.annotations.iter().find(|a| a.id() == id)
    }

    /// Number of annotations in this layer.
    pub fn len(&self) -> usize {
        self.annotations.len()
    }

    /// `true` when the layer has no annotations.
    pub fn is_empty(&self) -> bool {
        self.annotations.is_empty()
    }

    /// Generate a deterministic id from `(counter, kind)`. Callers
    /// that want collision-safe ids across sessions combine with
    /// `author` + `created_iso` to stamp uniqueness.
    pub fn next_id(counter: u64, kind_name: &str) -> String {
        format!("{}-{:08x}", kind_name, counter)
    }
}
