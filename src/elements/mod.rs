//! Per-class element decoders, and the typed views over what they decode.
//!
//! Every registered class decodes through a [`SimpleDecoder`]: it checks
//! that the schema entry is the class it was registered for, then the walk
//! reads the class's fields in the order `Formats/Latest` declares them.
//! Adding a class that needs no class-specific decoding is one line in
//! [`all_decoders`]: `Box::new(SimpleDecoder::new("MyClass"))`. A class
//! whose fields callers want typed adds a submodule with a view struct and
//! a `from_decoded` constructor, as `level.rs` does, and a class that must
//! fail closed on a malformed instance implements
//! [`crate::walker::ElementDecoder`] itself in that submodule.
//!
//! See `docs/extending-layer-5b.md` for the contributor walkthrough.
//!
//! # Relationship to the generic decoder
//!
//! [`crate::walker::decode_instance`] is the generic fallback — it
//! walks any class's declared fields using the schema's
//! `FieldType` classifications. It always works, but the output is
//! untyped (each field shows up as an `InstanceField` variant).
//!
//! A typed view does no decoding of its own: it pattern-matches the
//! `fields` vector of a decoded element into a class-specific struct (e.g.
//! `Level { name, elevation_feet, is_building_story, … }`). Callers who want
//! typed Wall / Floor / Door values use these; callers who want a
//! uniform untyped dump use `decode_instance` directly.
//!
//! # The typed set
//!
//! Typed views: Level, Wall (with WallType), Floor, Door, Window, Room
//! ([`zones::Zone`]) and Material, projected to JSON by [`typed_json`].
//! ArcWall uses a separate partition-byte path ([`arc_wall`]) because its
//! wire format is not schema-field based. Production
//! [`crate::walker::iter_elements`] prefers the registered decoder for
//! [`MVP_TYPED_CLASSES`] on `Global/Latest` and merges version-gated
//! ArcWall partition recovers — fail-closed when the decode rejects.

pub mod arc_wall;
pub mod floor;
pub mod level;
pub mod openings;
pub mod parameters;
pub mod simple_decoder;
pub mod styling;
pub mod typed_json;
pub mod wall;
pub mod zones;

pub use simple_decoder::SimpleDecoder;

use std::sync::LazyLock;

use crate::Result;
use crate::formats;
use crate::walker::{DecodedElement, ElementDecoder, HandleIndex};

/// Class names the element walk decodes through their typed decoder.
///
/// ArcWall is intentionally omitted — use [`arc_wall`] for partition
/// records.
// Tracker: M3-05.
pub const MVP_TYPED_CLASSES: &[&str] = &[
    "Level", "Wall", "Floor", "Door", "Window", "Room", "Material",
];

/// Every registered [`ElementDecoder`] in insertion order.
///
/// The walker dispatch table is built from this list at runtime.
/// Future registration helpers (inventory crate, etc.) could replace
/// this with compile-time discovery; for now an explicit Vec keeps
/// it obvious what ships with the crate.
pub fn all_decoders() -> Vec<Box<dyn ElementDecoder>> {
    vec![
        Box::new(SimpleDecoder::new("Level")),
        Box::new(SimpleDecoder::new("Category")),
        Box::new(SimpleDecoder::new("Subcategory")),
        Box::new(SimpleDecoder::new("Material")),
        Box::new(SimpleDecoder::new("FillPattern")),
        Box::new(SimpleDecoder::new("LinePattern")),
        Box::new(SimpleDecoder::new("LineStyle")),
        Box::new(SimpleDecoder::new("BasePoint")),
        Box::new(SimpleDecoder::new("SurveyPoint")),
        Box::new(SimpleDecoder::new("ProjectPosition")),
        Box::new(SimpleDecoder::new("Grid")),
        Box::new(SimpleDecoder::new("GridType")),
        Box::new(SimpleDecoder::new("ReferencePlane")),
        Box::new(SimpleDecoder::new("Wall")),
        Box::new(SimpleDecoder::new("WallType")),
        Box::new(SimpleDecoder::new("Floor")),
        Box::new(SimpleDecoder::new("FloorType")),
        Box::new(SimpleDecoder::new("Roof")),
        Box::new(SimpleDecoder::new("RoofType")),
        Box::new(SimpleDecoder::new("Ceiling")),
        Box::new(SimpleDecoder::new("CeilingType")),
        Box::new(SimpleDecoder::new("Door")),
        Box::new(SimpleDecoder::new("Window")),
        Box::new(SimpleDecoder::new("Column")),
        Box::new(SimpleDecoder::new("StructuralColumn")),
        Box::new(SimpleDecoder::new("Beam")),
        Box::new(SimpleDecoder::new("StructuralFraming")),
        Box::new(SimpleDecoder::new("Stair")),
        Box::new(SimpleDecoder::new("StairType")),
        Box::new(SimpleDecoder::new("Railing")),
        Box::new(SimpleDecoder::new("RailingType")),
        Box::new(SimpleDecoder::new("Room")),
        Box::new(SimpleDecoder::new("Area")),
        Box::new(SimpleDecoder::new("Space")),
        Box::new(SimpleDecoder::new("StructuralFoundation")),
        Box::new(SimpleDecoder::new("Furniture")),
        Box::new(SimpleDecoder::new("FurnitureSystem")),
        Box::new(SimpleDecoder::new("Casework")),
        Box::new(SimpleDecoder::new("Rebar")),
        Box::new(SimpleDecoder::new("Phase")),
        Box::new(SimpleDecoder::new("DesignOption")),
        Box::new(SimpleDecoder::new("Workset")),
        Box::new(SimpleDecoder::new("GenericModel")),
        Box::new(SimpleDecoder::new("Mass")),
        Box::new(SimpleDecoder::new("CurtainWall")),
        Box::new(SimpleDecoder::new("CurtainGrid")),
        Box::new(SimpleDecoder::new("CurtainMullion")),
        Box::new(SimpleDecoder::new("CurtainPanel")),
        Box::new(SimpleDecoder::new("Symbol")),
        Box::new(SimpleDecoder::new("FamilyInstance")),
        Box::new(SimpleDecoder::new("View")),
        Box::new(SimpleDecoder::new("Sheet")),
        Box::new(SimpleDecoder::new("Schedule")),
        Box::new(SimpleDecoder::new("ScheduleView")),
        Box::new(SimpleDecoder::new("Dimension")),
        Box::new(SimpleDecoder::new("Tag")),
        Box::new(SimpleDecoder::new("TextNote")),
        Box::new(SimpleDecoder::new("Annotation")),
        Box::new(SimpleDecoder::new("Revision")),
        Box::new(SimpleDecoder::new("ParameterElement")),
        Box::new(SimpleDecoder::new("SharedParameter")),
        Box::new(SimpleDecoder::new("ElectricalEquipment")),
        Box::new(SimpleDecoder::new("ElectricalFixture")),
        Box::new(SimpleDecoder::new("LightingFixture")),
        Box::new(SimpleDecoder::new("LightingDevice")),
        Box::new(SimpleDecoder::new("Duct")),
        Box::new(SimpleDecoder::new("DuctFitting")),
        Box::new(SimpleDecoder::new("MechanicalEquipment")),
        Box::new(SimpleDecoder::new("Pipe")),
        Box::new(SimpleDecoder::new("PipeFitting")),
        Box::new(SimpleDecoder::new("PlumbingFixture")),
        Box::new(SimpleDecoder::new("SpecialtyEquipment")),
        // L5B-54: AProperty* value-carrier classes. The schema
        // represents every element's parameter values as instances
        // of one of these subclasses, keyed by name to a matching
        // ParameterElement definition.
        Box::new(SimpleDecoder::new("AProperty")),
        Box::new(SimpleDecoder::new("APropertyBoolean")),
        Box::new(SimpleDecoder::new("APropertyInteger")),
        Box::new(SimpleDecoder::new("APropertyEnum")),
        Box::new(SimpleDecoder::new("APropertyDouble1")),
        Box::new(SimpleDecoder::new("APropertyDouble3")),
        Box::new(SimpleDecoder::new("APropertyFloat")),
        Box::new(SimpleDecoder::new("APropertyFloat3")),
        Box::new(SimpleDecoder::new("APropertyString")),
    ]
}

/// Look up a registered [`ElementDecoder`] by Revit class name.
///
/// Returns `None` when the class has no typed decoder in
/// [`all_decoders`]. ArcWall is never returned here — use
/// [`arc_wall::decode_at`] instead.
pub fn decoder_for_class(name: &str) -> Option<&'static dyn ElementDecoder> {
    REGISTRY
        .iter()
        .find(|d| d.class_name() == name)
        .map(Box::as_ref)
}

/// [`all_decoders`], built once: a lookup allocates nothing, and the walker
/// looks one up for every typed candidate.
static REGISTRY: LazyLock<Vec<Box<dyn ElementDecoder>>> = LazyLock::new(all_decoders);

/// Decode instance bytes through the registered typed decoder for
/// `schema.name`, enforcing wrong-schema rejection.
///
/// Prefer this over calling [`crate::walker::decode_instance`] when
/// the caller intends a typed MVP class: a mismatched schema name
/// fails closed instead of silently producing a mis-labeled
/// [`DecodedElement`].
pub fn decode_typed(
    bytes: &[u8],
    schema: &formats::ClassEntry,
    index: &HandleIndex,
) -> Result<DecodedElement> {
    let decoder = decoder_for_class(&schema.name)
        .ok_or_else(|| crate::Error::NoDecoder(schema.name.clone()))?;
    decoder.decode(bytes, schema, index)
}

/// [`decode_typed`] under the caller's walker caps (#852).
pub fn decode_typed_with_limits(
    bytes: &[u8],
    schema: &formats::ClassEntry,
    index: &HandleIndex,
    limits: crate::walker::WalkerLimits,
) -> Result<DecodedElement> {
    let decoder = decoder_for_class(&schema.name)
        .ok_or_else(|| crate::Error::NoDecoder(schema.name.clone()))?;
    decoder.decode_with_limits(bytes, schema, index, limits)
}

/// Prefer a typed MVP decoder for `class_name`, else generic
/// [`crate::walker::decode_instance_with_limits`].
///
/// When `class_name` is in [`MVP_TYPED_CLASSES`], returns `None` when
/// the registered decoder rejects (wrong schema), when any declared
/// field did not read as its schema type (the walk returns
/// [`crate::walker::InstanceField::Bytes`] for a short read or a type
/// it cannot read), or when the class's typed view
/// ([`typed_json::mvp_typed_view`]) reads nothing from the instance —
/// callers must not invent a typed success via the generic fallback
/// (#852). Non-MVP classes always use the generic decoder.
pub fn decode_instance_prefer_typed(
    bytes: &[u8],
    start: usize,
    schema: &formats::ClassEntry,
) -> Option<DecodedElement> {
    decode_instance_prefer_typed_with_limits(
        bytes,
        start,
        schema,
        crate::walker::WalkerLimits::default(),
    )
}

/// Same as [`decode_instance_prefer_typed`] with explicit walker caps,
/// applied to the typed and the generic path alike.
pub fn decode_instance_prefer_typed_with_limits(
    bytes: &[u8],
    start: usize,
    schema: &formats::ClassEntry,
    limits: crate::walker::WalkerLimits,
) -> Option<DecodedElement> {
    use typed_json::is_mvp_typed_class;
    if is_mvp_typed_class(&schema.name) {
        let slice = bytes.get(start..)?;
        match decode_typed_with_limits(slice, schema, &HandleIndex::new(), limits) {
            Ok(decoded) if !typed_instance_is_complete(&decoded) => None,
            Ok(mut decoded) => {
                // Typed decoders walk from offset 0 of `slice`; remap
                // byte_range into the parent buffer coordinate space.
                decoded.byte_range.start = decoded.byte_range.start.saturating_add(start);
                decoded.byte_range.end = decoded.byte_range.end.saturating_add(start);
                let decoder_name = format!("{}Decoder", schema.name);
                decoded.provenance = crate::walker::ElementProvenance::schema_typed(
                    "Global/Latest",
                    start,
                    &decoder_name,
                );
                Some(decoded)
            }
            Err(_) => None,
        }
    } else {
        Some(crate::walker::decode_instance_with_limits(
            bytes, start, schema, limits,
        ))
    }
}

/// Whether a typed decode read every declared field as its schema type
/// and its class's typed view read something from it (#852).
fn typed_instance_is_complete(decoded: &DecodedElement) -> bool {
    let fields_read = !decoded
        .fields
        .iter()
        .any(|(_, value)| matches!(value, crate::walker::InstanceField::Bytes(_)));
    let view_read = typed_json::mvp_typed_view(decoded).is_some_and(|view| {
        view.as_object()
            .is_some_and(|values| values.values().any(|value| !value.is_null()))
    });
    fields_read && view_read
}
