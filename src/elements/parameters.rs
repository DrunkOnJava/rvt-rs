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
simple_decoder!(APropertyStringDecoder, "APropertyString");

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

/// How a parameter's value is produced (L5B-56).
///
/// Revit parameters come in three flavours by how their value is
/// produced:
///
/// - **User-set**: a value the end user types or picks from a menu.
///   Always writable, always carried on the element.
/// - **Calculated / formula-driven**: the value is computed from a
///   formula that references other parameters ("= 2 * Height +
///   Width" for a label derived from geometry). Read-only to the
///   user; Revit evaluates the formula at update time and writes
///   the result back to the parameter slot.
/// - **Reporting**: the value is pulled from geometry or the
///   element's type at read time. Revit's "Dimension reports its
///   length" is the canonical example. Read-only; never edited
///   directly.
///
/// The `m_is_calculated` / `m_is_reporting` flags on
/// `ParameterElement` are what distinguish these modes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ParameterValueSource {
    /// User-set (default). Writable, no implicit dependency on
    /// other parameters or geometry.
    #[default]
    UserSet,
    /// Formula-driven. Revit recomputes the value from a formula
    /// string each time an input parameter changes.
    Calculated,
    /// Reporting. Value sourced from geometry / element properties
    /// at read time. Not user-editable.
    Reporting,
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
    /// How this parameter's value is produced (L5B-56). `None` when
    /// the relevant flags weren't present in the decoded payload;
    /// treat as [`ParameterValueSource::UserSet`] for
    /// user-facing writability purposes.
    pub value_source: Option<ParameterValueSource>,
}

impl ParameterElement {
    pub fn from_decoded(decoded: &DecodedElement) -> Self {
        let mut out = Self::default();
        let mut is_calculated = None;
        let mut is_reporting = None;
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
                (
                    "iscalculated" | "calculated" | "isformula" | "formula",
                    InstanceField::Bool(b),
                ) => {
                    is_calculated = Some(*b);
                }
                ("isreporting" | "reporting", InstanceField::Bool(b)) => {
                    is_reporting = Some(*b);
                }
                _ => {}
            }
        }
        // Precedence: reporting > calculated > user-set. A single
        // parameter can't be both calculated and reporting in Revit's
        // model (reporting parameters don't accept formulas), but if
        // both flags are somehow set we surface the stronger
        // constraint (reporting — fully read-only).
        out.value_source = match (is_calculated, is_reporting) {
            (_, Some(true)) => Some(ParameterValueSource::Reporting),
            (Some(true), _) => Some(ParameterValueSource::Calculated),
            (Some(false), Some(false)) | (Some(false), None) | (None, Some(false)) => {
                Some(ParameterValueSource::UserSet)
            }
            (None, None) => None,
        };
        out
    }

    /// True when this parameter is user-writable — i.e., neither
    /// calculated nor reporting. When the flags weren't present in
    /// the decoded payload, assumes user-writable (the safe default).
    pub fn is_user_writable(&self) -> bool {
        !matches!(
            self.value_source,
            Some(ParameterValueSource::Calculated | ParameterValueSource::Reporting)
        )
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
    /// `APropertyString.m_value` — UTF-16 string (Mark, comments,
    /// free-form text parameters).
    String(String),
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
                if let Some(InstanceField::Integer { value, .. }) = find_value(&["value"]) {
                    return ParameterValue::Integer(*value);
                }
            }
            "APropertyEnum" => {
                if let Some(InstanceField::Integer { value, .. }) = find_value(&["value"]) {
                    return ParameterValue::Enum(*value as u32);
                }
            }
            "APropertyDouble1" => {
                if let Some(InstanceField::Float { value, .. }) = find_value(&["value"]) {
                    return ParameterValue::Double(*value);
                }
            }
            "APropertyFloat" => {
                if let Some(InstanceField::Float { value, .. }) = find_value(&["value"]) {
                    return ParameterValue::Float(*value as f32);
                }
            }
            "APropertyDouble3" => {
                if let Some(InstanceField::Vector(components)) = find_value(&["value"]) {
                    if let Some(tuple) = vector_to_f64_3(components) {
                        return ParameterValue::Double3(tuple);
                    }
                }
            }
            "APropertyFloat3" => {
                if let Some(InstanceField::Vector(components)) = find_value(&["value"]) {
                    if let Some(tuple) = vector_to_f32_3(components) {
                        return ParameterValue::Float3(tuple);
                    }
                }
            }
            "APropertyString" => {
                if let Some(InstanceField::String(s)) = find_value(&["value"]) {
                    return ParameterValue::String(s.clone());
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
            ParameterValue::Boolean(_) | ParameterValue::Integer(_) | ParameterValue::Enum(_) => {
                StorageType::Integer
            }
            ParameterValue::Double(_)
            | ParameterValue::Double3(_)
            | ParameterValue::Float(_)
            | ParameterValue::Float3(_) => StorageType::Double,
            ParameterValue::String(_) => StorageType::String,
            ParameterValue::Other { .. } => StorageType::Other,
        }
    }

    /// Stable JSON object for CLI / Python / viewer surfaces.
    pub fn to_json_value(&self) -> serde_json::Value {
        match self {
            ParameterValue::Boolean(v) => serde_json::json!({
                "kind": "boolean",
                "value": v,
            }),
            ParameterValue::Integer(v) => serde_json::json!({
                "kind": "integer",
                "value": v,
            }),
            ParameterValue::Enum(v) => serde_json::json!({
                "kind": "enum",
                "value": v,
            }),
            ParameterValue::Double(v) => serde_json::json!({
                "kind": "double",
                "value": v,
            }),
            ParameterValue::Double3(v) => serde_json::json!({
                "kind": "double3",
                "value": v,
            }),
            ParameterValue::Float(v) => serde_json::json!({
                "kind": "float",
                "value": v,
            }),
            ParameterValue::Float3(v) => serde_json::json!({
                "kind": "float3",
                "value": v,
            }),
            ParameterValue::String(v) => serde_json::json!({
                "kind": "string",
                "value": v,
            }),
            ParameterValue::Other {
                class_name,
                raw_bytes,
            } => serde_json::json!({
                "kind": "other",
                "class_name": class_name,
                "raw_len": raw_bytes.len(),
            }),
        }
    }
}

/// One parameter value recovered from an AProperty* decoded element.
///
/// Host-element ↔ AProperty joins are not yet recovered on the
/// production partition path, so [`parameter_entries_from_decoded`]
/// returns a non-empty list only when `decoded` itself is an
/// AProperty* carrier. Other classes return `[]` (honest empty).
#[derive(Debug, Clone, PartialEq)]
pub struct ParameterEntry {
    /// Parameter display name when known; AProperty carriers rarely
    /// embed the definition name on the value instance alone.
    pub name: Option<String>,
    pub value: ParameterValue,
    pub source_class: String,
}

impl ParameterEntry {
    pub fn to_json_value(&self) -> serde_json::Value {
        let mut obj = serde_json::Map::new();
        match &self.name {
            Some(n) => {
                obj.insert("name".into(), serde_json::Value::String(n.clone()));
            }
            None => {
                obj.insert("name".into(), serde_json::Value::Null);
            }
        }
        obj.insert(
            "source_class".into(),
            serde_json::Value::String(self.source_class.clone()),
        );
        if let serde_json::Value::Object(map) = self.value.to_json_value() {
            for (k, v) in map {
                obj.insert(k, v);
            }
        }
        serde_json::Value::Object(obj)
    }
}

/// True when `class_name` is a known AProperty* value carrier.
pub fn is_aproperty_class(class_name: &str) -> bool {
    matches!(
        class_name,
        "AProperty"
            | "APropertyBoolean"
            | "APropertyInteger"
            | "APropertyEnum"
            | "APropertyDouble1"
            | "APropertyDouble3"
            | "APropertyFloat"
            | "APropertyFloat3"
            | "APropertyString"
    )
}

/// Parameter entries carried by this decoded element.
///
/// - AProperty* → one entry with the typed value (name usually unknown)
/// - anything else → empty (host↔value join not recovered yet)
pub fn parameter_entries_from_decoded(decoded: &DecodedElement) -> Vec<ParameterEntry> {
    if !is_aproperty_class(&decoded.class) {
        return Vec::new();
    }
    let value = ParameterValue::from_decoded(decoded);
    if matches!(
        &value,
        ParameterValue::Other { raw_bytes, .. } if raw_bytes.is_empty()
    ) && decoded.class == "AProperty"
    {
        return Vec::new();
    }
    let name = decoded.fields.iter().find_map(|(n, f)| {
        let norm = normalise_field_name(n);
        if matches!(norm.as_str(), "name" | "sname" | "parametername") {
            if let InstanceField::String(s) = f {
                return Some(s.clone());
            }
        }
        None
    });
    vec![ParameterEntry {
        name,
        value,
        source_class: decoded.class.clone(),
    }]
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
    Some([
        tuple_f64[0] as f32,
        tuple_f64[1] as f32,
        tuple_f64[2] as f32,
    ])
}

/// A per-element (or per-type) collection of decoded parameter
/// values keyed by parameter name (L5B-55).
///
/// Revit's parameter-inheritance model has two tiers:
///
/// 1. **Type-level** parameters live on the `Symbol` (family type).
///    Any instance of that type that hasn't overridden the value
///    sees the type-level value.
/// 2. **Instance-level** parameters live on the FamilyInstance
///    (or any host element — Wall, Floor, etc.) directly. When an
///    instance-level value is present for a given parameter name,
///    it overrides the type-level value for that specific
///    instance.
///
/// This struct holds one tier's view of the name → value map.
/// Pair two of them — one for the type, one for the instance —
/// and resolve with [`effective_value`].
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ParameterBundle {
    /// `BTreeMap` rather than `HashMap` so iteration order is
    /// deterministic — useful for snapshot tests and STEP output
    /// stability.
    values: std::collections::BTreeMap<String, ParameterValue>,
}

impl ParameterBundle {
    pub fn new() -> Self {
        Self::default()
    }

    /// Insert a (name, value) pair. Later inserts overwrite
    /// earlier ones for the same name.
    pub fn insert(&mut self, name: impl Into<String>, value: ParameterValue) {
        self.values.insert(name.into(), value);
    }

    /// Look up a parameter by name. Returns `None` when the name
    /// isn't in this tier's map.
    pub fn get(&self, name: &str) -> Option<&ParameterValue> {
        self.values.get(name)
    }

    pub fn len(&self) -> usize {
        self.values.len()
    }

    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }

    /// Iterate over (name, value) pairs in sorted-by-name order.
    pub fn iter(&self) -> impl Iterator<Item = (&str, &ParameterValue)> {
        self.values.iter().map(|(k, v)| (k.as_str(), v))
    }

    /// Every parameter name present in this tier. Sorted.
    pub fn names(&self) -> impl Iterator<Item = &str> {
        self.values.keys().map(String::as_str)
    }
}

/// Resolve the effective value of a named parameter given both
/// tiers (L5B-55).
///
/// Precedence follows Revit's own model: **instance wins over
/// type**. The instance bundle is consulted first; if the
/// parameter name isn't present there, the type bundle is the
/// fallback; if the name is in neither, returns `None`.
///
/// Usage pattern:
///
/// ```rust
/// use rvt::elements::parameters::{ParameterBundle, ParameterValue, effective_value};
///
/// let mut type_params = ParameterBundle::new();
/// type_params.insert("Width", ParameterValue::Double(2.0));
/// type_params.insert("Height", ParameterValue::Double(6.8));
///
/// let mut instance_params = ParameterBundle::new();
/// instance_params.insert("Height", ParameterValue::Double(7.0));
///
/// // Instance overrides the type's 6.8-ft default.
/// assert_eq!(
///     effective_value(&instance_params, &type_params, "Height"),
///     Some(&ParameterValue::Double(7.0))
/// );
/// // Instance didn't override Width — type-level value falls through.
/// assert_eq!(
///     effective_value(&instance_params, &type_params, "Width"),
///     Some(&ParameterValue::Double(2.0))
/// );
/// // Unknown name → None.
/// assert!(effective_value(&instance_params, &type_params, "Unknown").is_none());
/// ```
pub fn effective_value<'a>(
    instance: &'a ParameterBundle,
    type_: &'a ParameterBundle,
    name: &str,
) -> Option<&'a ParameterValue> {
    instance.get(name).or_else(|| type_.get(name))
}

/// Merge a type bundle and an instance bundle into a single
/// effective-view bundle (L5B-55).
///
/// Every parameter name present in EITHER bundle appears in the
/// result. Instance values override type values for overlapping
/// names — the same precedence rule as [`effective_value`] applied
/// across the whole name set in one pass.
///
/// Useful when a downstream consumer (IFC property-set builder,
/// schedule extractor) needs the full effective parameter map for
/// an element without doing per-name lookups.
pub fn merge_effective(instance: &ParameterBundle, type_: &ParameterBundle) -> ParameterBundle {
    let mut out = type_.clone();
    for (name, value) in instance.iter() {
        out.insert(name.to_string(), value.clone());
    }
    out
}
