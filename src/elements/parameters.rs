//! `ParameterElement` + `SharedParameter` — the metadata side of
//! Revit's parameter system.
//!
//! Revit parameters split into three layers:
//!
//! 1. **Definition** — what the parameter _is_ (name, storage type,
//!    unit, group, whether it's shared across projects). That's this
//!    module's responsibility via `ParameterElement` (project-local)
//!    and `SharedParameter` (shared-parameter-file-backed).
//! 2. **Attachment** — which categories or type/instance slots the
//!    parameter applies to. Handled elsewhere (category-bindings,
//!    future work).
//! 3. **Value** — the actual stored number / string / ElementId for
//!    a specific host element. Handled by the value-extraction pass
//!    (L5B-54, separate task).
//!
//! # Typical field shape (observed 2016–2026)
//!
//! ParameterElement:
//!
//! | Field | Type | Semantics |
//! |---|---|---|
//! | `m_name` | String | Human-visible parameter name ("Head Height", "Sill Height"). |
//! | `m_parameter_group` | Primitive u32 | Revit's enum of groupings (Identity Data, Dimensions, Constraints, …). |
//! | `m_storage_type` | Primitive u32 | 0 = None, 1 = Integer, 2 = Double, 3 = String, 4 = ElementId. |
//! | `m_unit_type` | Primitive u32 | Revit's unit-spec enum (length / area / volume / angle / currency / …). |
//! | `m_is_shared` | Primitive bool | True only for SharedParameter subclass instances. |
//! | `m_visible` | Primitive bool | False = hidden from Properties panel. |
//!
//! SharedParameter (subclass):
//!
//! | Field | Type | Semantics |
//! |---|---|---|
//! | `m_guid` | Guid | Stable cross-project identifier — the whole point of SharedParameter. |
//! | `m_description` | String | Free-form description shown in Shared Parameters dialog. |

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

simple_decoder!(ParameterElementDecoder, "ParameterElement");
simple_decoder!(SharedParameterDecoder, "SharedParameter");

/// Underlying wire-level storage kind of a parameter's value.
///
/// Maps to Revit's `StorageType` enum. Every ParameterElement has
/// exactly one `StorageType`, set at creation and never changed.
/// Value readers (L5B-54, separate task) dispatch on this to decide
/// how to interpret the raw bytes of a given element's value slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StorageType {
    /// No storage — the parameter is a category placeholder with no
    /// value (rare; mostly used for computed labels).
    #[default]
    None,
    /// 32-bit signed integer — used for counts, enum-valued options,
    /// and boolean flags (stored as 0 / 1).
    Integer,
    /// 64-bit IEEE double — used for lengths, angles, areas, volumes,
    /// and every other measurement type. Unit conversion happens at
    /// the display layer; the stored value is always in Revit's
    /// internal units (feet for length, radians for angle, …).
    Double,
    /// UTF-16 string — free-form text values like "Occupant", Mark
    /// labels, or custom user-supplied identifiers.
    String,
    /// ElementId reference to another element — used for Level refs,
    /// Type-to-Instance refs, linked-element pointers.
    ElementId,
    /// Unknown value — wire had a StorageType byte that doesn't match
    /// any of the above. Callers should treat the value slot as
    /// opaque bytes.
    Other,
}

impl StorageType {
    pub fn from_code(code: u32) -> Self {
        match code {
            0 => Self::None,
            1 => Self::Integer,
            2 => Self::Double,
            3 => Self::String,
            4 => Self::ElementId,
            _ => Self::Other,
        }
    }

    /// True when a value of this storage type is a numeric measurement
    /// (int or double). Useful for callers who want to extract only
    /// numeric parameters for analytics.
    pub fn is_numeric(self) -> bool {
        matches!(self, Self::Integer | Self::Double)
    }
}

/// Typed view of a decoded ParameterElement.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ParameterElement {
    pub name: Option<String>,
    pub parameter_group: Option<u32>,
    pub storage_type: Option<StorageType>,
    pub unit_type: Option<u32>,
    pub is_shared: Option<bool>,
    pub visible: Option<bool>,
}

impl ParameterElement {
    pub fn from_decoded(decoded: &DecodedElement) -> Self {
        let mut out = Self::default();
        for (field_name, value) in &decoded.fields {
            match (normalise_field_name(field_name).as_str(), value) {
                ("name", InstanceField::String(s)) => out.name = Some(s.clone()),
                (
                    "parametergroup" | "group",
                    InstanceField::Integer { value, .. },
                ) => {
                    out.parameter_group = Some(*value as u32);
                }
                (
                    "storagetype" | "storage",
                    InstanceField::Integer { value, .. },
                ) => {
                    out.storage_type = Some(StorageType::from_code(*value as u32));
                }
                ("unittype" | "unit", InstanceField::Integer { value, .. }) => {
                    out.unit_type = Some(*value as u32);
                }
                ("isshared" | "shared", InstanceField::Bool(b)) => {
                    out.is_shared = Some(*b);
                }
                ("visible", InstanceField::Bool(b)) => out.visible = Some(*b),
                _ => {}
            }
        }
        out
    }
}

/// Typed view of a decoded SharedParameter.
///
/// Inherits every field from `ParameterElement` and adds two:
/// `guid` (the stable cross-project identifier that makes a shared
/// parameter "shared"), and a free-form `description`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct SharedParameter {
    pub base: ParameterElement,
    /// Stable cross-project GUID. This is what lets two projects
    /// using the same shared-parameter file reconcile their
    /// parameter instances as "the same parameter."
    pub guid: Option<[u8; 16]>,
    pub description: Option<String>,
}

impl SharedParameter {
    pub fn from_decoded(decoded: &DecodedElement) -> Self {
        let mut out = Self {
            base: ParameterElement::from_decoded(decoded),
            ..Self::default()
        };
        for (field_name, value) in &decoded.fields {
            match (normalise_field_name(field_name).as_str(), value) {
                ("guid", InstanceField::Guid(bytes)) => out.guid = Some(*bytes),
                ("description", InstanceField::String(s)) => {
                    out.description = Some(s.clone());
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
    fn parameter_element_rejects_wrong_schema() {
        assert!(
            ParameterElementDecoder
                .decode(&[], &wrong_schema(), &HandleIndex::new())
                .is_err()
        );
    }

    #[test]
    fn shared_parameter_rejects_wrong_schema() {
        assert!(
            SharedParameterDecoder
                .decode(&[], &wrong_schema(), &HandleIndex::new())
                .is_err()
        );
    }

    #[test]
    fn storage_type_mapping() {
        assert_eq!(StorageType::from_code(0), StorageType::None);
        assert_eq!(StorageType::from_code(1), StorageType::Integer);
        assert_eq!(StorageType::from_code(2), StorageType::Double);
        assert_eq!(StorageType::from_code(3), StorageType::String);
        assert_eq!(StorageType::from_code(4), StorageType::ElementId);
        assert_eq!(StorageType::from_code(99), StorageType::Other);
        assert!(StorageType::Integer.is_numeric());
        assert!(StorageType::Double.is_numeric());
        assert!(!StorageType::String.is_numeric());
        assert!(!StorageType::ElementId.is_numeric());
    }

    #[test]
    fn parameter_element_from_decoded() {
        let fields = vec![
            (
                "m_name".into(),
                InstanceField::String("Head Height".into()),
            ),
            (
                "m_parameter_group".into(),
                InstanceField::Integer {
                    value: 7,
                    signed: false,
                    size: 4,
                },
            ),
            (
                "m_storage_type".into(),
                InstanceField::Integer {
                    value: 2,
                    signed: false,
                    size: 4,
                },
            ),
            (
                "m_is_shared".into(),
                InstanceField::Bool(false),
            ),
            (
                "m_visible".into(),
                InstanceField::Bool(true),
            ),
        ];
        let decoded = DecodedElement {
            id: None,
            class: "ParameterElement".into(),
            fields,
            byte_range: 0..0,
        };
        let p = ParameterElement::from_decoded(&decoded);
        assert_eq!(p.name.as_deref(), Some("Head Height"));
        assert_eq!(p.parameter_group, Some(7));
        assert_eq!(p.storage_type, Some(StorageType::Double));
        assert_eq!(p.is_shared, Some(false));
        assert_eq!(p.visible, Some(true));
    }

    #[test]
    fn shared_parameter_from_decoded_carries_base_fields() {
        let guid_bytes = [
            0x12, 0x34, 0x56, 0x78, 0x9a, 0xbc, 0xde, 0xf0, 0x0f, 0xed,
            0xcb, 0xa9, 0x87, 0x65, 0x43, 0x21,
        ];
        let fields = vec![
            (
                "m_name".into(),
                InstanceField::String("Fire Rating".into()),
            ),
            (
                "m_storage_type".into(),
                InstanceField::Integer {
                    value: 3,
                    signed: false,
                    size: 4,
                },
            ),
            (
                "m_is_shared".into(),
                InstanceField::Bool(true),
            ),
            ("m_guid".into(), InstanceField::Guid(guid_bytes)),
            (
                "m_description".into(),
                InstanceField::String("Hourly fire resistance rating".into()),
            ),
        ];
        let decoded = DecodedElement {
            id: None,
            class: "SharedParameter".into(),
            fields,
            byte_range: 0..0,
        };
        let sp = SharedParameter::from_decoded(&decoded);
        assert_eq!(sp.base.name.as_deref(), Some("Fire Rating"));
        assert_eq!(sp.base.storage_type, Some(StorageType::String));
        assert_eq!(sp.base.is_shared, Some(true));
        assert_eq!(sp.guid, Some(guid_bytes));
        assert_eq!(
            sp.description.as_deref(),
            Some("Hourly fire resistance rating")
        );
    }

    #[test]
    fn empty_tolerance() {
        let empty = DecodedElement {
            id: None,
            class: "ParameterElement".into(),
            fields: vec![],
            byte_range: 0..0,
        };
        assert!(ParameterElement::from_decoded(&empty).name.is_none());
        assert!(SharedParameter::from_decoded(&empty).guid.is_none());
    }

    #[test]
    fn class_names() {
        assert_eq!(ParameterElementDecoder.class_name(), "ParameterElement");
        assert_eq!(SharedParameterDecoder.class_name(), "SharedParameter");
    }
}
