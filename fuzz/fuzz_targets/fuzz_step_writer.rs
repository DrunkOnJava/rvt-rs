#![no_main]

//! Fuzz target for [`rvt::ifc::step_writer::write_step_with_options`].
//!
//! Unlike the other fuzz targets which feed raw bytes into a parser,
//! the STEP writer takes a structured `IfcModel` and produces a string
//! of IFC4 / ISO-10303-21 text. To fuzz it we build the `IfcModel` in
//! the harness from an `Arbitrary`-derived descriptor of primitives —
//! strings, counts, coordinates, option flags — and let libFuzzer
//! explore the combinations.
//!
//! What this target exercises:
//!
//! - STEP string escape (apostrophes, backslashes, BMP Unicode,
//!   supplementary-plane Unicode, ASCII control bytes) via all the
//!   `Option<String>` and `String` slots of `IfcModel`,
//!   `Storey.name`, `MaterialInfo.name`, `IfcEntity::BuildingElement.name`,
//!   `PropertySet.name`, `Property.name`, `PropertyValue::Text(..)`,
//!   `ClassificationItem.code`, `ClassificationItem.name`, and
//!   `Classification.edition`.
//! - `IfcEntity::BuildingElement` emission across the full matrix of
//!   optional slots: storey index, material index, per-element
//!   location + Z-rotation axis placement, rectangular-extrusion
//!   geometry, and the host-element opening + void/fill chain.
//! - The 8-field vs 10-field branch for `IFCDOOR` / `IFCWINDOW` (via
//!   a generated mix of `ifc_type` values).
//! - `IfcMaterial` color + surface-style emission (gated on the
//!   `color_packed` option).
//! - `IfcPropertySet` → `IfcRelDefinesByProperties` chain with
//!   `IfcText` / `IfcInteger` / `IfcReal` / `IfcBoolean` /
//!   `IfcLengthMeasure` / `IfcPlaneAngleMeasure` property values.
//! - `IfcClassification` + `IfcRelAssociatesClassification` emission
//!   for OmniClass / Uniformat / custom sources.
//! - Determinism: a fixed `StepOptions::timestamp` is supplied so the
//!   writer is a pure function of the input; libFuzzer's coverage
//!   signals stay stable across runs.
//!
//! Bugs it's trying to surface:
//!
//! - Panic from string-index arithmetic on non-ASCII / multi-byte
//!   content, or from the out-of-range storey_index / material_index /
//!   host_element_index clamps being wrong.
//! - Division-by-zero, NaN, or infinity mishandling in the feet→metre
//!   conversion or the `cos` / `sin` rotation math.
//! - Format-string slips where `{n}` or `{e}` should have been
//!   escaped as a literal brace.
//! - Unbounded allocation when element counts, storey counts,
//!   material counts, classification counts, or property counts
//!   combine multiplicatively.
//!
//! Safety net: all counts are clamped to small caps (`.min(16)` or
//! smaller) so libFuzzer workers don't OOM on adversarial inputs.
//! Without the caps a single 8-bit count could feed into a product
//! that allocates O(counts ^ N) entities and trips the default
//! libFuzzer rss limit.

use arbitrary::Arbitrary;
use libfuzzer_sys::fuzz_target;
use rvt::ifc::entities::{
    Classification, ClassificationItem, ClassificationSource, Extrusion, IfcEntity, Property,
    PropertySet, PropertyValue, UnitAssignment,
};
use rvt::ifc::step_writer::{StepOptions, write_step_with_options};
use rvt::ifc::{IfcModel, MaterialInfo, Storey};

/// Tag used to pick an IFC element constructor. Keeps the fuzzed
/// `ifc_type` field inside the set of values the writer actually
/// branches on (IfcDoor / IfcWindow → 10-field form; rest → 8-field).
/// Arbitrary random strings would still be valid, but this biases
/// coverage toward the interesting branches.
#[derive(Arbitrary, Debug)]
enum ElementTag {
    Wall,
    Slab,
    Roof,
    Covering,
    Column,
    Beam,
    Door,
    Window,
    Space,
    Railing,
    Stair,
    Furniture,
    /// Fallback — lets the fuzzer submit an arbitrary string that
    /// still routes through `to_ascii_uppercase()` and the 8-field
    /// branch. Kept short so it doesn't blow up the escape cost.
    Other(String),
}

impl ElementTag {
    fn as_str(&self) -> String {
        match self {
            Self::Wall => "IfcWall".into(),
            Self::Slab => "IfcSlab".into(),
            Self::Roof => "IfcRoof".into(),
            Self::Covering => "IfcCovering".into(),
            Self::Column => "IfcColumn".into(),
            Self::Beam => "IfcBeam".into(),
            Self::Door => "IfcDoor".into(),
            Self::Window => "IfcWindow".into(),
            Self::Space => "IfcSpace".into(),
            Self::Railing => "IfcRailing".into(),
            Self::Stair => "IfcStair".into(),
            Self::Furniture => "IfcFurniture".into(),
            Self::Other(s) => {
                // Clamp to 32 chars so the `Other` variant doesn't
                // dominate runtime via escape cost.
                let mut t = s.clone();
                if t.len() > 32 {
                    t.truncate(32);
                }
                t
            }
        }
    }
}

#[derive(Arbitrary, Debug)]
enum FuzzPropValue {
    Text(String),
    Integer(i64),
    Real(f64),
    Boolean(bool),
    LengthFeet(f64),
    AngleRadians(f64),
}

impl FuzzPropValue {
    fn into_prop(self) -> PropertyValue {
        match self {
            Self::Text(s) => PropertyValue::Text(truncate_string(s, 64)),
            Self::Integer(n) => PropertyValue::Integer(n),
            Self::Real(v) => PropertyValue::Real(v),
            Self::Boolean(b) => PropertyValue::Boolean(b),
            Self::LengthFeet(ft) => PropertyValue::LengthFeet(ft),
            Self::AngleRadians(r) => PropertyValue::AngleRadians(r),
        }
    }
}

#[derive(Arbitrary, Debug)]
struct FuzzProperty {
    name: String,
    value: FuzzPropValue,
}

#[derive(Arbitrary, Debug)]
struct FuzzPropertySet {
    name: String,
    properties: Vec<FuzzProperty>,
}

#[derive(Arbitrary, Debug)]
struct FuzzExtrusion {
    width_feet: f64,
    depth_feet: f64,
    height_feet: f64,
}

#[derive(Arbitrary, Debug)]
struct FuzzMaterial {
    name: String,
    color_packed: Option<u32>,
    transparency: Option<f64>,
}

#[derive(Arbitrary, Debug)]
struct FuzzStorey {
    name: String,
    elevation_feet: f64,
}

#[derive(Arbitrary, Debug)]
struct FuzzClassItem {
    code: String,
    name: Option<String>,
}

#[derive(Arbitrary, Debug)]
enum FuzzClassSource {
    OmniClass,
    Uniformat,
    Other(String),
}

#[derive(Arbitrary, Debug)]
struct FuzzClassification {
    source: FuzzClassSource,
    edition: Option<String>,
    items: Vec<FuzzClassItem>,
}

#[derive(Arbitrary, Debug)]
struct FuzzElement {
    tag: ElementTag,
    name: String,
    type_guid: Option<String>,
    storey_hint: Option<u8>,
    material_hint: Option<u8>,
    property_set: Option<FuzzPropertySet>,
    location_feet: Option<[f64; 3]>,
    rotation_radians: Option<f64>,
    extrusion: Option<FuzzExtrusion>,
    /// If set, tries to wire this element as a door/window in a
    /// preceding element's wall (opening + void + fill chain). The
    /// u8 is translated to an index modulo the number of prior
    /// elements at build time.
    host_hint: Option<u8>,
}

#[derive(Arbitrary, Debug)]
struct FuzzInput {
    project_name: Option<String>,
    description: Option<String>,
    timestamp: Option<i64>,
    storeys: Vec<FuzzStorey>,
    materials: Vec<FuzzMaterial>,
    classifications: Vec<FuzzClassification>,
    elements: Vec<FuzzElement>,
    /// Forge unit identifier strings — currently stored on the model
    /// but the writer doesn't consult them; exercising the field
    /// keeps future writer extensions covered without having to
    /// re-derive `Arbitrary`.
    unit_identifiers: Vec<String>,
}

fuzz_target!(|input: FuzzInput| {
    // Cap counts to keep the fuzzer worker bounded. A worst-case
    // product of (storeys × elements × properties × classification
    // items × materials) has to stay well below what libFuzzer's
    // default rss limit allows, so per-collection caps stay tight.
    const MAX_STOREYS: usize = 4;
    const MAX_MATERIALS: usize = 4;
    const MAX_CLASSIFICATIONS: usize = 4;
    const MAX_CLASS_ITEMS: usize = 4;
    const MAX_ELEMENTS: usize = 8;
    const MAX_PROPERTIES: usize = 4;
    const MAX_UNITS: usize = 4;
    const MAX_STRING: usize = 96;

    let project_name = input.project_name.map(|s| truncate_string(s, MAX_STRING));
    let description = input.description.map(|s| truncate_string(s, MAX_STRING));

    let building_storeys: Vec<Storey> = input
        .storeys
        .into_iter()
        .take(MAX_STOREYS)
        .map(|s| Storey {
            name: truncate_string(s.name, MAX_STRING),
            elevation_feet: sanitize_float(s.elevation_feet),
        })
        .collect();

    let materials: Vec<MaterialInfo> = input
        .materials
        .into_iter()
        .take(MAX_MATERIALS)
        .map(|m| MaterialInfo {
            name: truncate_string(m.name, MAX_STRING),
            color_packed: m.color_packed,
            transparency: m.transparency.map(sanitize_float),
        })
        .collect();

    let classifications: Vec<Classification> = input
        .classifications
        .into_iter()
        .take(MAX_CLASSIFICATIONS)
        .map(|c| Classification {
            source: match c.source {
                FuzzClassSource::OmniClass => ClassificationSource::OmniClass,
                FuzzClassSource::Uniformat => ClassificationSource::Uniformat,
                FuzzClassSource::Other(s) => {
                    ClassificationSource::Other(truncate_string(s, MAX_STRING))
                }
            },
            edition: c.edition.map(|e| truncate_string(e, MAX_STRING)),
            items: c
                .items
                .into_iter()
                .take(MAX_CLASS_ITEMS)
                .map(|i| ClassificationItem {
                    code: truncate_string(i.code, MAX_STRING),
                    name: i.name.map(|n| truncate_string(n, MAX_STRING)),
                })
                .collect(),
        })
        .collect();

    let units: Vec<UnitAssignment> = input
        .unit_identifiers
        .into_iter()
        .take(MAX_UNITS)
        .map(|id| UnitAssignment {
            forge_identifier: truncate_string(id, MAX_STRING),
            ifc_mapping: None,
        })
        .collect();

    // Build elements in order. `host_hint` is resolved at append
    // time: it references a strictly earlier index via modulo — that
    // matches the writer's precondition that a host be present in
    // `entities` before its dependent opening is emitted.
    let mut entities: Vec<IfcEntity> = Vec::with_capacity(MAX_ELEMENTS);
    for (i, fe) in input.elements.into_iter().take(MAX_ELEMENTS).enumerate() {
        let storey_index = fe
            .storey_hint
            .map(|h| usize::from(h) % (building_storeys.len().max(1)));
        let material_index = fe.material_hint.and_then(|h| {
            if materials.is_empty() {
                None
            } else {
                Some(usize::from(h) % materials.len())
            }
        });
        let host_element_index = fe.host_hint.and_then(|h| {
            if i == 0 {
                None
            } else {
                Some(usize::from(h) % i)
            }
        });
        let property_set = fe.property_set.map(|ps| PropertySet {
            name: truncate_string(ps.name, MAX_STRING),
            properties: ps
                .properties
                .into_iter()
                .take(MAX_PROPERTIES)
                .map(|p| Property {
                    name: truncate_string(p.name, MAX_STRING),
                    value: p.value.into_prop(),
                })
                .collect(),
        });
        let location_feet = fe.location_feet.map(|[x, y, z]| {
            [sanitize_float(x), sanitize_float(y), sanitize_float(z)]
        });
        let rotation_radians = fe.rotation_radians.map(sanitize_angle);
        let extrusion = fe.extrusion.map(|e| Extrusion {
            width_feet: sanitize_float(e.width_feet),
            depth_feet: sanitize_float(e.depth_feet),
            height_feet: sanitize_float(e.height_feet),
        });

        entities.push(IfcEntity::BuildingElement {
            ifc_type: fe.tag.as_str(),
            name: truncate_string(fe.name, MAX_STRING),
            type_guid: fe.type_guid.map(|g| truncate_string(g, MAX_STRING)),
            storey_index,
            material_index,
            property_set,
            location_feet,
            rotation_radians,
            extrusion,
            host_element_index,
        });
    }

    let model = IfcModel {
        project_name,
        description,
        entities,
        classifications,
        units,
        building_storeys,
        materials,
    };

    let opts = StepOptions {
        timestamp: input.timestamp,
    };
    let out = write_step_with_options(&model, &opts);

    // Lightweight shape check: the writer must always produce a
    // structurally-valid STEP envelope regardless of input. Any
    // input that breaks these invariants is a bug we want libFuzzer
    // to pinpoint — assert catches it, fuzzer records the crash.
    debug_assert!(out.starts_with("ISO-10303-21;\n"));
    debug_assert!(out.ends_with("END-ISO-10303-21;\n"));
    debug_assert!(out.contains("FILE_SCHEMA(('IFC4'));"));
    debug_assert!(out.contains("IFCPROJECT"));
});

/// Truncate on a char boundary so we never split a multi-byte UTF-8
/// sequence. Using raw byte slicing would panic on random non-ASCII
/// input — which is exactly the class of bug the STEP escape path is
/// supposed to handle, but the truncation itself should never be the
/// thing that trips it.
fn truncate_string(mut s: String, max_chars: usize) -> String {
    if s.chars().count() <= max_chars {
        return s;
    }
    // Find the byte boundary at `max_chars`.
    let cut = s
        .char_indices()
        .nth(max_chars)
        .map(|(i, _)| i)
        .unwrap_or(s.len());
    s.truncate(cut);
    s
}

/// Replace NaN / infinities with 0.0 so the writer's `format!` of
/// `{v:.6}` never emits non-finite literals (the STEP spec
/// technically allows them, but `IfcOpenShell` rejects NaN in most
/// numeric slots). Keeping them bounded also guarantees the escape
/// path reaches steady-state quickly. Finite values are passed
/// through unchanged, including subnormals.
fn sanitize_float(v: f64) -> f64 {
    if v.is_finite() { v } else { 0.0 }
}

/// Angles get clamped to a comfortable range so `sin` / `cos` stay
/// numerically well-behaved. Values outside this range are still
/// finite but arguably nonsense for a Revit yaw — still exercise the
/// code path, just wrapped.
fn sanitize_angle(v: f64) -> f64 {
    let sanitized = sanitize_float(v);
    // Wrap into [-2π, 2π] so the rotation axis emits a sane direction.
    let two_pi = std::f64::consts::TAU;
    sanitized.rem_euclid(2.0 * two_pi) - two_pi
}
