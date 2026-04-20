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

// L5B-54: AProperty* value-carrier classes. Each Revit element's
// parameters are stored as a sequence of AProperty* instances, one
// per parameter-definition × host-element tuple. The class name
// encodes the value type at schema time; the concrete instance
// carries the stored value.
//
// AProperty is the abstract base class (no standalone instances
// in the wild — any AProperty-tagged instance in Formats/Latest is
// one of the concrete subclasses below).
//
// See src/formats.rs note at line ~958 for the raw wire pattern
// (`06 10 00 00 03 00 00 00` = vector<f32>, used by APropertyFloat3.m_value).
simple_decoder!(APropertyDecoder, "AProperty");
simple_decoder!(APropertyBooleanDecoder, "APropertyBoolean");
simple_decoder!(APropertyIntegerDecoder, "APropertyInteger");
simple_decoder!(APropertyEnumDecoder, "APropertyEnum");
simple_decoder!(APropertyDouble1Decoder, "APropertyDouble1");
simple_decoder!(APropertyDouble3Decoder, "APropertyDouble3");
simple_decoder!(APropertyFloatDecoder, "APropertyFloat");
simple_decoder!(APropertyFloat3Decoder, "APropertyFloat3");

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
                ("parametergroup" | "group", InstanceField::Integer { value, .. }) => {
                    out.parameter_group = Some(*value as u32);
                }
                ("storagetype" | "storage", InstanceField::Integer { value, .. }) => {
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

/// A single decoded parameter value (L5B-54).
///
/// Revit's parameter system stores each element's parameter values
/// as a sequence of `AProperty*` class instances. The specific
/// subclass encodes the value's storage type; the instance body
/// carries the actual value in an `m_value` field.
///
/// This enum captures the full vocabulary (8 variants) so callers
/// can pattern-match once on the property's class + decoded body
/// without threading the storage type through every downstream
/// branch. Unknown AProperty subclasses fall through to
/// [`ParameterValue::Other`] carrying the raw class name + the
/// best-effort typed fields (useful when Revit ships a new
/// AProperty variant we haven't mapped yet — the field bytes
/// still round-trip, just without a typed view).
#[derive(Debug, Clone, PartialEq)]
pub enum ParameterValue {
    /// `APropertyBoolean.m_value` — single 8-bit bool (0 / 1).
    Boolean(bool),
    /// `APropertyInteger.m_value` — 32-bit signed integer. Used
    /// for counts, enum-valued options, flags.
    Integer(i64),
    /// `APropertyEnum.m_value` — 32-bit enum code. Revit's
    /// category-specific parameter enum (e.g. Wall.StructuralUsage
    /// = Bearing / Shear / NonBearing / …).
    Enum(u32),
    /// `APropertyDouble1.m_value` — single 64-bit IEEE double.
    /// Used for length (feet), angle (radians), area, volume,
    /// and every other measurement. Unit conversion is a display-
    /// layer concern; the stored value is always in Revit
    /// internal units.
    Double(f64),
    /// `APropertyDouble3.m_value` — triple of 64-bit doubles.
    /// Used for 3D coordinates, directions, colours as
    /// normalized RGB.
    Double3([f64; 3]),
    /// `APropertyFloat.m_value` — single 32-bit IEEE float.
    /// Legacy float storage still present in some element classes.
    Float(f32),
    /// `APropertyFloat3.m_value` — triple of 32-bit floats.
    /// Same role as Double3 but narrower precision — reserved for
    /// graphical-only data (material diffuse colour, UI accent).
    Float3([f32; 3]),
    /// `AProperty` or an unrecognised subclass. `class_name` is the
    /// raw schema class name; `raw_bytes` is the instance body
    /// before field-level decode. Round-trips through the walker
    /// unchanged.
    Other {
        class_name: String,
        raw_bytes: Vec<u8>,
    },
}

impl ParameterValue {
    /// Extract a typed [`ParameterValue`] from a [`DecodedElement`]
    /// produced by one of the AProperty* decoders.
    ///
    /// Field-name matching is lenient — we accept any of
    /// `m_value`, `value`, or `m_value_0` / `value_0` (Revit's
    /// convention for `_0` / `_1` / `_2` for the components of a
    /// vector-3 field). Returns [`ParameterValue::Other`] when
    /// the class doesn't match a known subclass OR the expected
    /// `m_value` field wasn't in the decoded payload.
    pub fn from_decoded(decoded: &DecodedElement) -> Self {
        let find_value = |names: &[&str]| -> Option<&InstanceField> {
            for (name, field) in &decoded.fields {
                let normalised = normalise_field_name(name);
                if names.iter().any(|wanted| normalised == *wanted) {
                    return Some(field);
                }
            }
            None
        };
        match decoded.class.as_str() {
            "APropertyBoolean" => {
                if let Some(InstanceField::Bool(b)) = find_value(&["value"]) {
                    return ParameterValue::Boolean(*b);
                }
            }
            "APropertyInteger" => {
                if let Some(InstanceField::Integer { value, .. }) =
                    find_value(&["value"])
                {
                    return ParameterValue::Integer(*value);
                }
            }
            "APropertyEnum" => {
                if let Some(InstanceField::Integer { value, .. }) =
                    find_value(&["value"])
                {
                    return ParameterValue::Enum(*value as u32);
                }
            }
            "APropertyDouble1" => {
                if let Some(InstanceField::Float { value, .. }) =
                    find_value(&["value"])
                {
                    return ParameterValue::Double(*value);
                }
            }
            "APropertyFloat" => {
                if let Some(InstanceField::Float { value, .. }) =
                    find_value(&["value"])
                {
                    return ParameterValue::Float(*value as f32);
                }
            }
            "APropertyDouble3" => {
                if let Some(InstanceField::Vector(components)) = find_value(&["value"])
                {
                    if let Some(tuple) = vector_to_f64_3(components) {
                        return ParameterValue::Double3(tuple);
                    }
                }
            }
            "APropertyFloat3" => {
                if let Some(InstanceField::Vector(components)) = find_value(&["value"])
                {
                    if let Some(tuple) = vector_to_f32_3(components) {
                        return ParameterValue::Float3(tuple);
                    }
                }
            }
            _ => {}
        }
        // Fallback — AProperty (base class) or unknown subclass.
        let raw_bytes = decoded
            .fields
            .iter()
            .find_map(|(_, f)| {
                if let InstanceField::Bytes(b) = f {
                    Some(b.clone())
                } else {
                    None
                }
            })
            .unwrap_or_default();
        ParameterValue::Other {
            class_name: decoded.class.clone(),
            raw_bytes,
        }
    }

    /// The [`StorageType`] this value corresponds to. Useful for
    /// joining a decoded value against its matching ParameterElement
    /// definition.
    pub fn storage_type(&self) -> StorageType {
        match self {
            ParameterValue::Boolean(_) | ParameterValue::Integer(_)
            | ParameterValue::Enum(_) => StorageType::Integer,
            ParameterValue::Double(_) | ParameterValue::Double3(_)
            | ParameterValue::Float(_) | ParameterValue::Float3(_) => {
                StorageType::Double
            }
            ParameterValue::Other { .. } => StorageType::Other,
        }
    }
}

fn vector_to_f64_3(components: &[InstanceField]) -> Option<[f64; 3]> {
    if components.len() < 3 {
        return None;
    }
    let to_f64 = |f: &InstanceField| match f {
        InstanceField::Float { value, .. } => Some(*value),
        InstanceField::Integer { value, .. } => Some(*value as f64),
        _ => None,
    };
    Some([
        to_f64(&components[0])?,
        to_f64(&components[1])?,
        to_f64(&components[2])?,
    ])
}

fn vector_to_f32_3(components: &[InstanceField]) -> Option<[f32; 3]> {
    let tuple_f64 = vector_to_f64_3(components)?;
    Some([tuple_f64[0] as f32, tuple_f64[1] as f32, tuple_f64[2] as f32])
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
            ("m_name".into(), InstanceField::String("Head Height".into())),
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
            ("m_is_shared".into(), InstanceField::Bool(false)),
            ("m_visible".into(), InstanceField::Bool(true)),
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
            0x12, 0x34, 0x56, 0x78, 0x9a, 0xbc, 0xde, 0xf0, 0x0f, 0xed, 0xcb, 0xa9, 0x87, 0x65,
            0x43, 0x21,
        ];
        let fields = vec![
            ("m_name".into(), InstanceField::String("Fire Rating".into())),
            (
                "m_storage_type".into(),
                InstanceField::Integer {
                    value: 3,
                    signed: false,
                    size: 4,
                },
            ),
            ("m_is_shared".into(), InstanceField::Bool(true)),
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

    // ----- L5B-54: AProperty* + ParameterValue -----

    fn mk_decoded(class: &str, fields: Vec<(String, InstanceField)>) -> DecodedElement {
        DecodedElement {
            id: None,
            class: class.into(),
            fields,
            byte_range: 0..0,
        }
    }

    #[test]
    fn aproperty_boolean_decodes_to_parameter_value() {
        let d = mk_decoded(
            "APropertyBoolean",
            vec![("m_value".into(), InstanceField::Bool(true))],
        );
        assert_eq!(ParameterValue::from_decoded(&d), ParameterValue::Boolean(true));
    }

    #[test]
    fn aproperty_integer_decodes_to_parameter_value() {
        let d = mk_decoded(
            "APropertyInteger",
            vec![(
                "m_value".into(),
                InstanceField::Integer {
                    value: 42,
                    signed: true,
                    size: 4,
                },
            )],
        );
        assert_eq!(ParameterValue::from_decoded(&d), ParameterValue::Integer(42));
    }

    #[test]
    fn aproperty_enum_decodes_to_parameter_value() {
        let d = mk_decoded(
            "APropertyEnum",
            vec![(
                "m_value".into(),
                InstanceField::Integer {
                    value: 7,
                    signed: false,
                    size: 4,
                },
            )],
        );
        assert_eq!(ParameterValue::from_decoded(&d), ParameterValue::Enum(7));
    }

    #[test]
    fn aproperty_double1_decodes_to_parameter_value() {
        let d = mk_decoded(
            "APropertyDouble1",
            vec![(
                "m_value".into(),
                InstanceField::Float {
                    value: 3.5,
                    size: 8,
                },
            )],
        );
        assert_eq!(ParameterValue::from_decoded(&d), ParameterValue::Double(3.5));
    }

    #[test]
    fn aproperty_double3_decodes_to_parameter_value() {
        let components = vec![
            InstanceField::Float { value: 1.0, size: 8 },
            InstanceField::Float { value: 2.0, size: 8 },
            InstanceField::Float { value: 3.0, size: 8 },
        ];
        let d = mk_decoded(
            "APropertyDouble3",
            vec![("m_value".into(), InstanceField::Vector(components))],
        );
        assert_eq!(
            ParameterValue::from_decoded(&d),
            ParameterValue::Double3([1.0, 2.0, 3.0])
        );
    }

    #[test]
    fn aproperty_float_decodes_to_parameter_value() {
        let d = mk_decoded(
            "APropertyFloat",
            vec![(
                "m_value".into(),
                InstanceField::Float {
                    value: 0.5,
                    size: 4,
                },
            )],
        );
        assert_eq!(ParameterValue::from_decoded(&d), ParameterValue::Float(0.5));
    }

    #[test]
    fn aproperty_float3_decodes_to_parameter_value() {
        let components = vec![
            InstanceField::Float { value: 0.1, size: 4 },
            InstanceField::Float { value: 0.2, size: 4 },
            InstanceField::Float { value: 0.3, size: 4 },
        ];
        let d = mk_decoded(
            "APropertyFloat3",
            vec![("m_value".into(), InstanceField::Vector(components))],
        );
        match ParameterValue::from_decoded(&d) {
            ParameterValue::Float3([x, y, z]) => {
                assert!((x - 0.1).abs() < 1e-6);
                assert!((y - 0.2).abs() < 1e-6);
                assert!((z - 0.3).abs() < 1e-6);
            }
            other => panic!("expected Float3, got {other:?}"),
        }
    }

    #[test]
    fn aproperty_unknown_falls_through_to_other() {
        let d = mk_decoded(
            "APropertyNewVariantFromFutureRevit",
            vec![("m_value".into(), InstanceField::Bytes(vec![0x01, 0x02]))],
        );
        match ParameterValue::from_decoded(&d) {
            ParameterValue::Other { class_name, raw_bytes } => {
                assert_eq!(class_name, "APropertyNewVariantFromFutureRevit");
                assert_eq!(raw_bytes, vec![0x01, 0x02]);
            }
            other => panic!("expected Other, got {other:?}"),
        }
    }

    #[test]
    fn aproperty_missing_value_field_falls_through_to_other() {
        // Class matches a known decoder, but the m_value field is
        // absent. Should still return Other (not panic, not silently
        // lose data).
        let d = mk_decoded("APropertyBoolean", vec![]);
        assert!(matches!(
            ParameterValue::from_decoded(&d),
            ParameterValue::Other { .. }
        ));
    }

    #[test]
    fn parameter_value_storage_type_mapping() {
        assert_eq!(
            ParameterValue::Boolean(false).storage_type(),
            StorageType::Integer
        );
        assert_eq!(
            ParameterValue::Integer(0).storage_type(),
            StorageType::Integer
        );
        assert_eq!(ParameterValue::Enum(0).storage_type(), StorageType::Integer);
        assert_eq!(
            ParameterValue::Double(0.0).storage_type(),
            StorageType::Double
        );
        assert_eq!(
            ParameterValue::Double3([0.0; 3]).storage_type(),
            StorageType::Double
        );
        assert_eq!(
            ParameterValue::Float(0.0).storage_type(),
            StorageType::Double
        );
        assert_eq!(
            ParameterValue::Float3([0.0; 3]).storage_type(),
            StorageType::Double
        );
        assert_eq!(
            ParameterValue::Other {
                class_name: String::new(),
                raw_bytes: vec![],
            }
            .storage_type(),
            StorageType::Other
        );
    }

    #[test]
    fn aproperty_decoders_reject_wrong_schema() {
        // Spot-check two of the eight new decoders.
        assert!(
            APropertyBooleanDecoder
                .decode(&[], &wrong_schema(), &HandleIndex::new())
                .is_err()
        );
        assert!(
            APropertyDouble3Decoder
                .decode(&[], &wrong_schema(), &HandleIndex::new())
                .is_err()
        );
    }
}
