//! `View` + `Sheet` + `Titleblock` + `Viewport` + `Schedule` +
//! `ScheduleView` — Revit's drafting/documentation classes.
//!
//! These don't represent physical building geometry; they're how
//! Revit lays out 2D drawings and tabular data for construction
//! documents. In IFC terms: out of scope for IFC4 base schema
//! (IFC 2D drawings are largely handled via IfcAnnotation +
//! IfcDocumentReference, neither of which we emit yet).
//!
//! We decode them for:
//! - **Round-trip write support** (Phase 7) — can't re-emit a
//!   file without knowing its drafting contents.
//! - **Schedule → data extraction** — Schedules aggregate element
//!   properties; useful for BIM-to-spreadsheet tooling even if
//!   they don't survive IFC export.
//!
//! # Typical Revit field shape (stable 2016–2026)
//!
//! View:
//!
//! | Field | Type | Semantics |
//! |---|---|---|
//! | `m_name` | String | User-visible view name ("Level 1", "West Elevation") |
//! | `m_view_type` | Primitive u32 | 0=FloorPlan 1=CeilingPlan 2=Elevation 3=Section 4=ThreeD 5=Drafting 6=Schedule … |
//! | `m_scale` | Primitive u32 | Drawing scale (e.g. 48 for 1/4" = 1'-0") |
//! | `m_associated_level_id` | ElementId | Level this view's plan is cut on (FloorPlan/CeilingPlan only) |
//! | `m_phase_filter_id` | ElementId | Which Phase filter applies |
//!
//! Sheet:
//!
//! | Field | Type | Semantics |
//! |---|---|---|
//! | `m_name` | String | Sheet name ("A-101 - First Floor Plan") |
//! | `m_number` | String | Sheet number ("A-101") |
//! | `m_titleblock_id` | ElementId | Referenced titleblock family instance |
//!
//! Schedule (ScheduleView subtype):
//!
//! | Field | Type | Semantics |
//! |---|---|---|
//! | `m_name` | String | "Door Schedule", "Window Schedule", "Room Schedule" |
//! | `m_category_id` | ElementId | Which Category's elements this schedules |

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

simple_decoder!(ViewDecoder, "View");
simple_decoder!(SheetDecoder, "Sheet");
simple_decoder!(ScheduleDecoder, "Schedule");
simple_decoder!(ScheduleViewDecoder, "ScheduleView");

/// Which kind of drawing the View represents. Revit's ViewType enum
/// expanded; we model the most common values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ViewKind {
    #[default]
    FloorPlan,
    CeilingPlan,
    Elevation,
    Section,
    ThreeD,
    Drafting,
    Schedule,
    Legend,
    Detail,
    Other,
}

impl ViewKind {
    pub fn from_code(code: u32) -> Self {
        match code {
            0 => Self::FloorPlan,
            1 => Self::CeilingPlan,
            2 => Self::Elevation,
            3 => Self::Section,
            4 => Self::ThreeD,
            5 => Self::Drafting,
            6 => Self::Schedule,
            7 => Self::Legend,
            8 => Self::Detail,
            _ => Self::Other,
        }
    }

    /// True when this is a 2D drawing view (floor / ceiling plan,
    /// elevation, section, detail). False for 3D views, schedules,
    /// legends, and drafting views.
    pub fn is_drawing(self) -> bool {
        matches!(
            self,
            Self::FloorPlan | Self::CeilingPlan | Self::Elevation | Self::Section | Self::Detail
        )
    }
}

/// Typed view of a decoded View.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct View {
    pub name: Option<String>,
    pub kind: Option<ViewKind>,
    pub scale: Option<u32>,
    pub associated_level_id: Option<u32>,
    pub phase_filter_id: Option<u32>,
}

impl View {
    pub fn from_decoded(decoded: &DecodedElement) -> Self {
        let mut out = Self::default();
        for (field_name, value) in &decoded.fields {
            match (normalise_field_name(field_name).as_str(), value) {
                ("name", InstanceField::String(s)) => out.name = Some(s.clone()),
                ("viewtype" | "kind", InstanceField::Integer { value, .. }) => {
                    out.kind = Some(ViewKind::from_code(*value as u32));
                }
                ("scale", InstanceField::Integer { value, .. }) => {
                    out.scale = Some(*value as u32);
                }
                ("associatedlevelid" | "levelid", InstanceField::ElementId { id, .. }) => {
                    out.associated_level_id = Some(*id);
                }
                ("phasefilterid", InstanceField::ElementId { id, .. }) => {
                    out.phase_filter_id = Some(*id);
                }
                _ => {}
            }
        }
        out
    }

    /// Scale as a formatted string ("1/4\" = 1'-0\"", "1:50") —
    /// convention is `scale` is the raw denominator in Revit's
    /// imperial/metric mixed units. We surface it verbatim.
    pub fn scale_label(&self) -> Option<String> {
        self.scale.map(|s| format!("1:{s}"))
    }
}

/// Typed view of a decoded Sheet.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Sheet {
    pub name: Option<String>,
    pub number: Option<String>,
    pub titleblock_id: Option<u32>,
}

impl Sheet {
    pub fn from_decoded(decoded: &DecodedElement) -> Self {
        let mut out = Self::default();
        for (field_name, value) in &decoded.fields {
            match (normalise_field_name(field_name).as_str(), value) {
                ("name" | "sheetname", InstanceField::String(s)) => out.name = Some(s.clone()),
                ("number" | "sheetnumber", InstanceField::String(s)) => {
                    out.number = Some(s.clone());
                }
                ("titleblockid", InstanceField::ElementId { id, .. }) => {
                    out.titleblock_id = Some(*id);
                }
                _ => {}
            }
        }
        out
    }

    /// Formatted sheet label — "A-101 — First Floor Plan" when both
    /// number and name present, just one or the other otherwise.
    pub fn label(&self) -> Option<String> {
        match (&self.number, &self.name) {
            (Some(n), Some(name)) => Some(format!("{n} — {name}")),
            (Some(n), None) => Some(n.clone()),
            (None, Some(name)) => Some(name.clone()),
            (None, None) => None,
        }
    }
}

/// Typed view of a decoded Schedule / ScheduleView.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Schedule {
    pub name: Option<String>,
    /// The Revit category that this schedule aggregates ("Walls",
    /// "Doors", "Rooms"). Reference to a decoded Category.
    pub category_id: Option<u32>,
}

impl Schedule {
    pub fn from_decoded(decoded: &DecodedElement) -> Self {
        let mut out = Self::default();
        for (field_name, value) in &decoded.fields {
            match (normalise_field_name(field_name).as_str(), value) {
                ("name", InstanceField::String(s)) => out.name = Some(s.clone()),
                ("categoryid", InstanceField::ElementId { id, .. }) => {
                    out.category_id = Some(*id);
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

    #[test]
    fn view_rejects_wrong_schema() {
        use crate::formats::ClassEntry;
        let wrong = ClassEntry {
            name: "Wall".into(),
            offset: 0,
            fields: vec![],
            tag: None,
            parent: None,
            declared_field_count: None,
            was_parent_only: false,
            ancestor_tag: None,
        };
        assert!(
            ViewDecoder
                .decode(&[], &wrong, &HandleIndex::new())
                .is_err()
        );
    }

    #[test]
    fn view_kind_mapping() {
        assert_eq!(ViewKind::from_code(0), ViewKind::FloorPlan);
        assert_eq!(ViewKind::from_code(2), ViewKind::Elevation);
        assert_eq!(ViewKind::from_code(4), ViewKind::ThreeD);
        assert_eq!(ViewKind::from_code(99), ViewKind::Other);
        assert!(ViewKind::FloorPlan.is_drawing());
        assert!(!ViewKind::ThreeD.is_drawing());
        assert!(!ViewKind::Schedule.is_drawing());
    }

    #[test]
    fn view_from_decoded() {
        let fields = vec![
            (
                "m_name".into(),
                InstanceField::String("Level 1 — Floor Plan".into()),
            ),
            (
                "m_view_type".into(),
                InstanceField::Integer {
                    value: 0,
                    signed: false,
                    size: 4,
                },
            ),
            (
                "m_scale".into(),
                InstanceField::Integer {
                    value: 48,
                    signed: false,
                    size: 4,
                },
            ),
        ];
        let decoded = DecodedElement {
            id: None,
            class: "View".into(),
            fields,
            byte_range: 0..0,
        };
        let v = View::from_decoded(&decoded);
        assert_eq!(v.name.as_deref(), Some("Level 1 — Floor Plan"));
        assert_eq!(v.kind, Some(ViewKind::FloorPlan));
        assert_eq!(v.scale, Some(48));
        assert_eq!(v.scale_label().as_deref(), Some("1:48"));
    }

    #[test]
    fn sheet_label_combinations() {
        let full = Sheet {
            name: Some("First Floor Plan".into()),
            number: Some("A-101".into()),
            ..Default::default()
        };
        let only_number = Sheet {
            number: Some("A-101".into()),
            ..Default::default()
        };
        let empty = Sheet::default();
        assert_eq!(full.label().as_deref(), Some("A-101 — First Floor Plan"));
        assert_eq!(only_number.label().as_deref(), Some("A-101"));
        assert_eq!(empty.label(), None);
    }

    #[test]
    fn schedule_from_decoded() {
        let fields = vec![
            (
                "m_name".into(),
                InstanceField::String("Door Schedule".into()),
            ),
            (
                "m_category_id".into(),
                InstanceField::ElementId { tag: 0, id: 42 },
            ),
        ];
        let decoded = DecodedElement {
            id: None,
            class: "Schedule".into(),
            fields,
            byte_range: 0..0,
        };
        let s = Schedule::from_decoded(&decoded);
        assert_eq!(s.name.as_deref(), Some("Door Schedule"));
        assert_eq!(s.category_id, Some(42));
    }

    #[test]
    fn empty_tolerance() {
        let empty = DecodedElement {
            id: None,
            class: "View".into(),
            fields: vec![],
            byte_range: 0..0,
        };
        assert!(View::from_decoded(&empty).name.is_none());
        assert!(Sheet::from_decoded(&empty).name.is_none());
        assert!(Schedule::from_decoded(&empty).name.is_none());
    }

    #[test]
    fn class_names() {
        assert_eq!(ViewDecoder.class_name(), "View");
        assert_eq!(SheetDecoder.class_name(), "Sheet");
        assert_eq!(ScheduleDecoder.class_name(), "Schedule");
        assert_eq!(ScheduleViewDecoder.class_name(), "ScheduleView");
    }
}
