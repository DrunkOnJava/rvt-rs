//! `Dimension` + `Tag` + `TextNote` — Revit's 2D annotation classes.
//!
//! These live on drafting / sheet views (see [`super::drafting`]) and
//! are Revit's way of putting measurements, labels, and free-form text
//! on top of the model graphics. They're not physical building
//! elements — IFC4 carries them via `IfcAnnotation` (not yet emitted
//! by rvt-rs's IFC exporter). We decode them here so round-trip /
//! analysis tools can enumerate annotations without waiting on the
//! IFC side.
//!
//! # Typical Revit field shape (observed 2016–2026)
//!
//! Dimension:
//!
//! | Field | Type | Semantics |
//! |---|---|---|
//! | `m_owner_view_id` | ElementId | The view this dimension annotates |
//! | `m_dimension_type_id` | ElementId | DimensionType that styles it |
//! | `m_value` | Primitive f64 | Reported distance (internal units — feet) |
//! | `m_override_text` | Optional String | Empty unless user typed over the auto-label |
//! | `m_is_locked` | Primitive bool | Is the dimension constrained |
//!
//! Tag:
//!
//! | Field | Type | Semantics |
//! |---|---|---|
//! | `m_owner_view_id` | ElementId | The view this tag lives on |
//! | `m_tagged_element_id` | ElementId | The element the tag annotates |
//! | `m_tag_head_position` | Vector | Local position of the tag on the view |
//! | `m_tag_orientation` | Primitive u32 | 0=Horizontal 1=Vertical |
//! | `m_leader` | Optional bool | Whether a leader line is drawn |
//!
//! TextNote:
//!
//! | Field | Type | Semantics |
//! |---|---|---|
//! | `m_owner_view_id` | ElementId | The view this note lives on |
//! | `m_text_note_type_id` | ElementId | TextNoteType that styles the note |
//! | `m_text` | String | The note's literal text |
//! | `m_width` | Primitive f64 | Display width (feet) — 0 means auto |
//! | `m_horizontal_alignment` | Primitive u32 | 0=Left 1=Center 2=Right |

use super::level::normalise_field_name;
use crate::formats;
use crate::walker::{DecodedElement, ElementDecoder, HandleIndex, InstanceField};
use crate::{Error, Result};

macro_rules! simple_decoder {
    ($Struct:ident, $name:literal) => {
        pub struct $Struct;

        impl ElementDecoder for $Struct {
            fn class_name(&self) -> &'static str {
                $name
            }

            fn decode(
                &self,
                bytes: &[u8],
                schema: &formats::ClassEntry,
                _index: &HandleIndex,
            ) -> Result<DecodedElement> {
                if schema.name != $name {
                    return Err(Error::BasicFileInfo(format!(
                        "{} received wrong schema: {}",
                        stringify!($Struct),
                        schema.name
                    )));
                }
                Ok(crate::walker::decode_instance(bytes, 0, schema))
            }
        }
    };
}

simple_decoder!(DimensionDecoder, "Dimension");
simple_decoder!(TagDecoder, "Tag");
simple_decoder!(TextNoteDecoder, "TextNote");
simple_decoder!(AnnotationDecoder, "Annotation");

/// Typed view of a decoded Dimension.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Dimension {
    pub owner_view_id: Option<u32>,
    pub dimension_type_id: Option<u32>,
    /// Reported distance in feet — Revit's native internal unit.
    pub value_feet: Option<f64>,
    /// Empty when the user has not overridden the auto-label.
    pub override_text: Option<String>,
    pub is_locked: Option<bool>,
}

impl Dimension {
    pub fn from_decoded(decoded: &DecodedElement) -> Self {
        let mut out = Self::default();
        for (field_name, value) in &decoded.fields {
            match (normalise_field_name(field_name).as_str(), value) {
                ("ownerviewid" | "viewid", InstanceField::ElementId { id, .. }) => {
                    out.owner_view_id = Some(*id);
                }
                (
                    "dimensiontypeid" | "typeid",
                    InstanceField::ElementId { id, .. },
                ) => {
                    out.dimension_type_id = Some(*id);
                }
                ("value", InstanceField::Float { value, .. }) => {
                    out.value_feet = Some(*value);
                }
                ("overridetext" | "text", InstanceField::String(s)) => {
                    out.override_text = Some(s.clone());
                }
                ("islocked" | "locked", InstanceField::Bool(b)) => {
                    out.is_locked = Some(*b);
                }
                _ => {}
            }
        }
        out
    }
}

/// Tag-head position in local view coordinates, in feet.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct TagHeadPosition {
    pub x: f64,
    pub y: f64,
}

/// Which axis the tag text runs along.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TagOrientation {
    #[default]
    Horizontal,
    Vertical,
    Other,
}

impl TagOrientation {
    pub fn from_code(code: u32) -> Self {
        match code {
            0 => Self::Horizontal,
            1 => Self::Vertical,
            _ => Self::Other,
        }
    }
}

/// Typed view of a decoded Tag.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Tag {
    pub owner_view_id: Option<u32>,
    /// ElementId of the element this tag annotates.
    pub tagged_element_id: Option<u32>,
    pub head_position: Option<TagHeadPosition>,
    pub orientation: Option<TagOrientation>,
    pub has_leader: Option<bool>,
}

impl Tag {
    pub fn from_decoded(decoded: &DecodedElement) -> Self {
        let mut out = Self::default();
        for (field_name, value) in &decoded.fields {
            match (normalise_field_name(field_name).as_str(), value) {
                ("ownerviewid" | "viewid", InstanceField::ElementId { id, .. }) => {
                    out.owner_view_id = Some(*id);
                }
                (
                    "taggedelementid" | "elementid",
                    InstanceField::ElementId { id, .. },
                ) => {
                    out.tagged_element_id = Some(*id);
                }
                (
                    "tagheadposition" | "headposition",
                    InstanceField::Vector(components),
                ) => {
                    if components.len() >= 2
                        && let (
                            Some(InstanceField::Float { value: x, .. }),
                            Some(InstanceField::Float { value: y, .. }),
                        ) = (components.first(), components.get(1))
                    {
                        out.head_position = Some(TagHeadPosition {
                            x: *x,
                            y: *y,
                        });
                    }
                }
                (
                    "tagorientation" | "orientation",
                    InstanceField::Integer { value, .. },
                ) => {
                    out.orientation = Some(TagOrientation::from_code(*value as u32));
                }
                ("leader" | "hasleader", InstanceField::Bool(b)) => {
                    out.has_leader = Some(*b);
                }
                _ => {}
            }
        }
        out
    }
}

/// Which side the text is anchored against.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum HorizontalAlignment {
    #[default]
    Left,
    Center,
    Right,
    Other,
}

impl HorizontalAlignment {
    pub fn from_code(code: u32) -> Self {
        match code {
            0 => Self::Left,
            1 => Self::Center,
            2 => Self::Right,
            _ => Self::Other,
        }
    }
}

/// Typed view of a decoded TextNote.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct TextNote {
    pub owner_view_id: Option<u32>,
    pub text_note_type_id: Option<u32>,
    pub text: Option<String>,
    /// Display width in feet. `0.0` means auto-width. `None` when not
    /// present in the schema shape.
    pub width_feet: Option<f64>,
    pub horizontal_alignment: Option<HorizontalAlignment>,
}

impl TextNote {
    pub fn from_decoded(decoded: &DecodedElement) -> Self {
        let mut out = Self::default();
        for (field_name, value) in &decoded.fields {
            match (normalise_field_name(field_name).as_str(), value) {
                ("ownerviewid" | "viewid", InstanceField::ElementId { id, .. }) => {
                    out.owner_view_id = Some(*id);
                }
                (
                    "textnotetypeid" | "typeid",
                    InstanceField::ElementId { id, .. },
                ) => {
                    out.text_note_type_id = Some(*id);
                }
                ("text", InstanceField::String(s)) => out.text = Some(s.clone()),
                ("width", InstanceField::Float { value, .. }) => {
                    out.width_feet = Some(*value);
                }
                (
                    "horizontalalignment" | "alignment",
                    InstanceField::Integer { value, .. },
                ) => {
                    out.horizontal_alignment =
                        Some(HorizontalAlignment::from_code(*value as u32));
                }
                _ => {}
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::formats::ClassEntry;

    fn wrong_schema() -> ClassEntry {
        ClassEntry {
            name: "Wall".into(),
            offset: 0,
            fields: vec![],
            tag: None,
            parent: None,
            declared_field_count: None,
            was_parent_only: false,
            ancestor_tag: None,
        }
    }

    #[test]
    fn dimension_rejects_wrong_schema() {
        assert!(
            DimensionDecoder
                .decode(&[], &wrong_schema(), &HandleIndex::new())
                .is_err()
        );
    }

    #[test]
    fn tag_rejects_wrong_schema() {
        assert!(
            TagDecoder
                .decode(&[], &wrong_schema(), &HandleIndex::new())
                .is_err()
        );
    }

    #[test]
    fn textnote_rejects_wrong_schema() {
        assert!(
            TextNoteDecoder
                .decode(&[], &wrong_schema(), &HandleIndex::new())
                .is_err()
        );
    }

    #[test]
    fn dimension_from_decoded() {
        let fields = vec![
            (
                "m_value".into(),
                InstanceField::Float {
                    value: 12.5,
                    size: 8,
                },
            ),
            (
                "m_override_text".into(),
                InstanceField::String("12'-6\"".into()),
            ),
            (
                "m_is_locked".into(),
                InstanceField::Bool(true),
            ),
            (
                "m_owner_view_id".into(),
                InstanceField::ElementId { tag: 0, id: 7 },
            ),
        ];
        let decoded = DecodedElement {
            id: None,
            class: "Dimension".into(),
            fields,
            byte_range: 0..0,
        };
        let d = Dimension::from_decoded(&decoded);
        assert_eq!(d.value_feet, Some(12.5));
        assert_eq!(d.override_text.as_deref(), Some("12'-6\""));
        assert_eq!(d.is_locked, Some(true));
        assert_eq!(d.owner_view_id, Some(7));
    }

    #[test]
    fn tag_from_decoded() {
        let fields = vec![
            (
                "m_owner_view_id".into(),
                InstanceField::ElementId { tag: 0, id: 42 },
            ),
            (
                "m_tagged_element_id".into(),
                InstanceField::ElementId { tag: 0, id: 99 },
            ),
            (
                "m_tag_orientation".into(),
                InstanceField::Integer {
                    value: 1,
                    signed: false,
                    size: 4,
                },
            ),
            (
                "m_leader".into(),
                InstanceField::Bool(true),
            ),
            (
                "m_tag_head_position".into(),
                InstanceField::Vector(vec![
                    InstanceField::Float {
                        value: 3.5,
                        size: 8,
                    },
                    InstanceField::Float {
                        value: -1.25,
                        size: 8,
                    },
                ]),
            ),
        ];
        let decoded = DecodedElement {
            id: None,
            class: "Tag".into(),
            fields,
            byte_range: 0..0,
        };
        let t = Tag::from_decoded(&decoded);
        assert_eq!(t.owner_view_id, Some(42));
        assert_eq!(t.tagged_element_id, Some(99));
        assert_eq!(t.orientation, Some(TagOrientation::Vertical));
        assert_eq!(t.has_leader, Some(true));
        assert_eq!(
            t.head_position,
            Some(TagHeadPosition { x: 3.5, y: -1.25 })
        );
    }

    #[test]
    fn textnote_from_decoded() {
        let fields = vec![
            (
                "m_text".into(),
                InstanceField::String("SEE STRUCT DETAIL".into()),
            ),
            (
                "m_width".into(),
                InstanceField::Float {
                    value: 2.0,
                    size: 8,
                },
            ),
            (
                "m_horizontal_alignment".into(),
                InstanceField::Integer {
                    value: 1,
                    signed: false,
                    size: 4,
                },
            ),
            (
                "m_owner_view_id".into(),
                InstanceField::ElementId { tag: 0, id: 17 },
            ),
        ];
        let decoded = DecodedElement {
            id: None,
            class: "TextNote".into(),
            fields,
            byte_range: 0..0,
        };
        let n = TextNote::from_decoded(&decoded);
        assert_eq!(n.text.as_deref(), Some("SEE STRUCT DETAIL"));
        assert_eq!(n.width_feet, Some(2.0));
        assert_eq!(
            n.horizontal_alignment,
            Some(HorizontalAlignment::Center)
        );
        assert_eq!(n.owner_view_id, Some(17));
    }

    #[test]
    fn tag_orientation_mapping() {
        assert_eq!(TagOrientation::from_code(0), TagOrientation::Horizontal);
        assert_eq!(TagOrientation::from_code(1), TagOrientation::Vertical);
        assert_eq!(TagOrientation::from_code(99), TagOrientation::Other);
    }

    #[test]
    fn horizontal_alignment_mapping() {
        assert_eq!(HorizontalAlignment::from_code(0), HorizontalAlignment::Left);
        assert_eq!(
            HorizontalAlignment::from_code(1),
            HorizontalAlignment::Center
        );
        assert_eq!(HorizontalAlignment::from_code(2), HorizontalAlignment::Right);
        assert_eq!(HorizontalAlignment::from_code(99), HorizontalAlignment::Other);
    }

    #[test]
    fn empty_tolerance() {
        let empty = DecodedElement {
            id: None,
            class: "Dimension".into(),
            fields: vec![],
            byte_range: 0..0,
        };
        assert!(Dimension::from_decoded(&empty).value_feet.is_none());
        assert!(Tag::from_decoded(&empty).tagged_element_id.is_none());
        assert!(TextNote::from_decoded(&empty).text.is_none());
    }

    #[test]
    fn class_names() {
        assert_eq!(DimensionDecoder.class_name(), "Dimension");
        assert_eq!(TagDecoder.class_name(), "Tag");
        assert_eq!(TextNoteDecoder.class_name(), "TextNote");
        assert_eq!(AnnotationDecoder.class_name(), "Annotation");
    }

    /// Generic Annotation decoder accepts any element whose schema is
    /// named "Annotation" — it's the base class. Tests the
    /// wrong-schema rejection path only; typed-view pattern-matching
    /// is deferred until we see real-world Annotation shapes. This
    /// means callers who walk Annotation today get the raw
    /// DecodedElement back (via decode_instance's schema-directed
    /// walker) and can inspect `fields` themselves.
    #[test]
    fn annotation_rejects_wrong_schema() {
        assert!(
            AnnotationDecoder
                .decode(&[], &wrong_schema(), &HandleIndex::new())
                .is_err()
        );
    }
}
