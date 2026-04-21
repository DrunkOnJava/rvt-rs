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

#[cfg(test)]
mod tests {
    use super::*;

    fn mk_note(id: &str, text: &str) -> Annotation {
        Annotation::Note {
            id: id.into(),
            anchor: [0.0, 0.0, 0.0],
            text: text.into(),
            author: None,
            created_iso: None,
        }
    }

    #[test]
    fn empty_layer_is_empty() {
        let layer = AnnotationLayer::new();
        assert!(layer.is_empty());
        assert_eq!(layer.len(), 0);
    }

    #[test]
    fn push_adds_in_order() {
        let mut layer = AnnotationLayer::new();
        layer.push(mk_note("a", "first"));
        layer.push(mk_note("b", "second"));
        assert_eq!(layer.len(), 2);
        assert_eq!(layer.annotations[0].id(), "a");
        assert_eq!(layer.annotations[1].id(), "b");
    }

    #[test]
    fn remove_by_id_removes_matching() {
        let mut layer = AnnotationLayer::new();
        layer.push(mk_note("a", "x"));
        layer.push(mk_note("b", "y"));
        assert!(layer.remove_by_id("a"));
        assert_eq!(layer.len(), 1);
        assert_eq!(layer.annotations[0].id(), "b");
    }

    #[test]
    fn remove_by_id_false_on_miss() {
        let mut layer = AnnotationLayer::new();
        layer.push(mk_note("a", "x"));
        assert!(!layer.remove_by_id("nonexistent"));
        assert_eq!(layer.len(), 1);
    }

    #[test]
    fn find_returns_matching() {
        let mut layer = AnnotationLayer::new();
        layer.push(mk_note("a", "x"));
        layer.push(mk_note("b", "y"));
        let found = layer.find("b").unwrap();
        assert_eq!(found.id(), "b");
    }

    #[test]
    fn next_id_is_deterministic() {
        assert_eq!(AnnotationLayer::next_id(0, "note"), "note-00000000");
        assert_eq!(AnnotationLayer::next_id(42, "pin"), "pin-0000002a");
    }

    #[test]
    fn kind_name_covers_all_variants() {
        let note = mk_note("a", "x");
        assert_eq!(note.kind_name(), "note");
        let leader = Annotation::Leader {
            id: "l".into(),
            anchor: [0.0, 0.0, 0.0],
            label_anchor: [1.0, 0.0, 0.0],
            text: "t".into(),
            author: None,
            created_iso: None,
        };
        assert_eq!(leader.kind_name(), "leader");
        let polyline = Annotation::Polyline {
            id: "p".into(),
            vertices: vec![[0.0; 3], [1.0; 3]],
            author: None,
            created_iso: None,
        };
        assert_eq!(polyline.kind_name(), "polyline");
        let pin = Annotation::Pin {
            id: "pin".into(),
            anchor: [0.0, 0.0, 0.0],
            category: Some("Defect".into()),
            author: None,
            created_iso: None,
        };
        assert_eq!(pin.kind_name(), "pin");
    }

    #[test]
    fn annotation_layer_serde_roundtrips() {
        let mut layer = AnnotationLayer::new();
        layer.push(Annotation::Pin {
            id: "p1".into(),
            anchor: [1.0, 2.0, 3.0],
            category: Some("Defect".into()),
            author: Some("Griffin".into()),
            created_iso: Some("2026-04-20T12:00:00Z".into()),
        });
        layer.push(mk_note("n1", "hello"));
        let json = serde_json::to_string(&layer).unwrap();
        let back: AnnotationLayer = serde_json::from_str(&json).unwrap();
        assert_eq!(back, layer);
    }

    #[test]
    fn annotation_tagged_serialization_uses_kind_field() {
        let note = mk_note("x", "hi");
        let json = serde_json::to_string(&note).unwrap();
        // Serde's `tag = "kind"` attribute produces `"kind":"note"`
        // as the discriminant.
        assert!(json.contains("\"kind\":\"note\""));
    }

    #[test]
    fn all_variants_expose_id_uniformly() {
        let anns = [
            mk_note("n", "x"),
            Annotation::Leader {
                id: "l".into(),
                anchor: [0.0; 3],
                label_anchor: [0.0; 3],
                text: String::new(),
                author: None,
                created_iso: None,
            },
            Annotation::Polyline {
                id: "p".into(),
                vertices: vec![[0.0; 3]; 2],
                author: None,
                created_iso: None,
            },
            Annotation::Pin {
                id: "pin".into(),
                anchor: [0.0; 3],
                category: None,
                author: None,
                created_iso: None,
            },
        ];
        let ids: Vec<&str> = anns.iter().map(|a| a.id()).collect();
        assert_eq!(ids, vec!["n", "l", "p", "pin"]);
    }
}
