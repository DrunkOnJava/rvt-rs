//! `Phase` + `DesignOption` + `Workset` — the project-level
//! organization classes that slice the element set along non-spatial
//! axes. None of them map to a specific IFC4 entity; they surface as
//! property-set tags on the elements they group.
//!
//! - **Phase**: temporal classification. Elements are tagged with
//!   `phase_created` and `phase_demolished` so contractors can answer
//!   "what walls exist at phase 2?" without carrying two models.
//! - **DesignOption**: alternative-design classification. An element
//!   may exist in "Option A" but not "Option B" — when the user picks
//!   Option A as primary, B's elements are hidden.
//! - **Workset**: collaboration unit in workshared models — which
//!   team member "owns" a given element.
//!
//! # Typical field shape
//!
//! Phase:
//!
//! | Field | Type | Semantics |
//! |---|---|---|
//! | `m_name` | String | "Existing", "Phase 1", "New Construction" |
//! | `m_description` | String | Optional descriptive text |
//! | `m_sequence_number` | Primitive u32 | Ordering index (0 = earliest) |
//!
//! DesignOption:
//!
//! | Field | Type | Semantics |
//! |---|---|---|
//! | `m_name` | String | "Option 1", "Facade Study B" |
//! | `m_is_primary` | Primitive bool | True for the option shown by default |
//! | `m_option_set_id` | ElementId | Grouping set containing related options |
//!
//! Workset:
//!
//! | Field | Type | Semantics |
//! |---|---|---|
//! | `m_name` | String | "Architecture", "Mech/Plumb", "Shared Levels and Grids" |
//! | `m_unique_id` | String | Stable GUID for round-trip |
//! | `m_is_open` | Primitive bool | Whether the workset is loaded in the current view |
//! | `m_is_editable` | Primitive bool | Read-only vs writable |

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

simple_decoder!(PhaseDecoder, "Phase");
simple_decoder!(DesignOptionDecoder, "DesignOption");
simple_decoder!(WorksetDecoder, "Workset");

/// Typed view for Phase.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Phase {
    pub name: Option<String>,
    pub description: Option<String>,
    pub sequence_number: Option<u32>,
}

impl Phase {
    pub fn from_decoded(decoded: &DecodedElement) -> Self {
        let mut out = Self::default();
        for (field_name, value) in &decoded.fields {
            match (normalise_field_name(field_name).as_str(), value) {
                ("name", InstanceField::String(s)) => out.name = Some(s.clone()),
                ("description", InstanceField::String(s)) => {
                    out.description = Some(s.clone());
                }
                ("sequencenumber" | "sequence", InstanceField::Integer { value, .. }) => {
                    out.sequence_number = Some(*value as u32);
                }
                _ => {}
            }
        }
        out
    }
}

/// Typed view for DesignOption.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct DesignOption {
    pub name: Option<String>,
    pub is_primary: Option<bool>,
    pub option_set_id: Option<u32>,
}

impl DesignOption {
    pub fn from_decoded(decoded: &DecodedElement) -> Self {
        let mut out = Self::default();
        for (field_name, value) in &decoded.fields {
            match (normalise_field_name(field_name).as_str(), value) {
                ("name", InstanceField::String(s)) => out.name = Some(s.clone()),
                ("isprimary" | "primary", InstanceField::Bool(b)) => {
                    out.is_primary = Some(*b);
                }
                ("optionsetid" | "setid", InstanceField::ElementId { id, .. }) => {
                    out.option_set_id = Some(*id);
                }
                _ => {}
            }
        }
        out
    }
}

/// Typed view for Workset.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Workset {
    pub name: Option<String>,
    pub unique_id: Option<String>,
    pub is_open: Option<bool>,
    pub is_editable: Option<bool>,
}

impl Workset {
    pub fn from_decoded(decoded: &DecodedElement) -> Self {
        let mut out = Self::default();
        for (field_name, value) in &decoded.fields {
            match (normalise_field_name(field_name).as_str(), value) {
                ("name", InstanceField::String(s)) => out.name = Some(s.clone()),
                ("uniqueid" | "guid", InstanceField::String(s)) => {
                    out.unique_id = Some(s.clone());
                }
                ("isopen", InstanceField::Bool(b)) => out.is_open = Some(*b),
                ("iseditable" | "editable", InstanceField::Bool(b)) => {
                    out.is_editable = Some(*b);
                }
                _ => {}
            }
        }
        out
    }

    /// True when the workset is both open and editable — the typical
    /// "I can modify this" combination in workshared Revit models.
    pub fn is_modifiable(&self) -> Option<bool> {
        Some(self.is_open? && self.is_editable?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn phase_rejects_wrong_schema() {
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
            PhaseDecoder
                .decode(&[], &wrong, &HandleIndex::new())
                .is_err()
        );
    }

    #[test]
    fn phase_from_fields() {
        let fields = vec![
            (
                "m_name".into(),
                InstanceField::String("New Construction".into()),
            ),
            (
                "m_sequence_number".into(),
                InstanceField::Integer {
                    value: 1,
                    signed: false,
                    size: 4,
                },
            ),
        ];
        let decoded = DecodedElement {
            id: None,
            class: "Phase".into(),
            fields,
            byte_range: 0..0,
        };
        let p = Phase::from_decoded(&decoded);
        assert_eq!(p.name.as_deref(), Some("New Construction"));
        assert_eq!(p.sequence_number, Some(1));
    }

    #[test]
    fn design_option_primary_flag() {
        let primary = DesignOption {
            is_primary: Some(true),
            ..Default::default()
        };
        let alternative = DesignOption {
            is_primary: Some(false),
            ..Default::default()
        };
        assert_eq!(primary.is_primary, Some(true));
        assert_eq!(alternative.is_primary, Some(false));
    }

    #[test]
    fn workset_modifiability() {
        let modifiable = Workset {
            is_open: Some(true),
            is_editable: Some(true),
            ..Default::default()
        };
        let read_only = Workset {
            is_open: Some(true),
            is_editable: Some(false),
            ..Default::default()
        };
        let closed = Workset {
            is_open: Some(false),
            is_editable: Some(true),
            ..Default::default()
        };
        let unknown = Workset::default();
        assert_eq!(modifiable.is_modifiable(), Some(true));
        assert_eq!(read_only.is_modifiable(), Some(false));
        assert_eq!(closed.is_modifiable(), Some(false));
        assert_eq!(unknown.is_modifiable(), None);
    }

    #[test]
    fn workset_guid_preserves() {
        let fields = vec![
            (
                "m_unique_id".into(),
                InstanceField::String("abc-123-def-456".into()),
            ),
            ("m_is_open".into(), InstanceField::Bool(true)),
        ];
        let decoded = DecodedElement {
            id: None,
            class: "Workset".into(),
            fields,
            byte_range: 0..0,
        };
        let w = Workset::from_decoded(&decoded);
        assert_eq!(w.unique_id.as_deref(), Some("abc-123-def-456"));
        assert_eq!(w.is_open, Some(true));
    }

    #[test]
    fn empty_tolerance() {
        let empty = DecodedElement {
            id: None,
            class: "Phase".into(),
            fields: vec![],
            byte_range: 0..0,
        };
        assert!(Phase::from_decoded(&empty).name.is_none());
        assert!(DesignOption::from_decoded(&empty).name.is_none());
        assert!(Workset::from_decoded(&empty).name.is_none());
    }

    #[test]
    fn class_names() {
        assert_eq!(PhaseDecoder.class_name(), "Phase");
        assert_eq!(DesignOptionDecoder.class_name(), "DesignOption");
        assert_eq!(WorksetDecoder.class_name(), "Workset");
    }
}
