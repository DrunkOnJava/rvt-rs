//! Per-class element decoders.
//!
//! Each submodule implements [`crate::walker::ElementDecoder`] for
//! one Revit class, or, where the walk's own field order is enough, the
//! class is registered as a [`SimpleDecoder`] with its name. Adding a
//! class that needs no class-specific decoding is one line in
//! [`all_decoders`]: `Box::new(SimpleDecoder::new("MyClass"))`. A class
//! that does is a three-file change:
//!
//! 1. Add `mod my_class;` here.
//! 2. Register it in [`all_decoders`] (consulted by production
//!    [`crate::walker::iter_elements`] for [`MVP_TYPED_CLASSES`]).
//! 3. Implement `ElementDecoder` in `src/elements/my_class.rs` —
//!    see `level.rs` as the reference example.
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
//! Concrete decoders in this module add a typed layer on top: they
//! call `decode_instance` first, then pattern-match the `fields`
//! vector into a class-specific struct (e.g. `Level { name,
//! elevation, is_building_story, … }`). Callers who want typed
//! Wall / Floor / Door values use these; callers who want a
//! uniform untyped dump use `decode_instance` directly.
//!
//! # Lane Five MVP set
//!
//! Schema-driven typed decoders with wrong-schema rejection:
//! Level, Wall, Floor, Door, Window, Room, Material.
//! ArcWall uses a separate partition-byte path
//! ([`arc_wall`]) because its wire format is not schema-field based.
//! Production [`crate::walker::iter_elements`] prefers the typed
//! registry for MVP classes on `Global/Latest` and merges
//! version-gated ArcWall partition recovers — fail-closed when
//! typed decode rejects.

pub mod annotations;
pub mod arc_wall;
pub mod category;
pub mod ceiling;
pub mod circulation;
pub mod curtain_wall;
pub mod drafting;
pub mod family;
pub mod floor;
pub mod foundation_and_furnishings;
pub mod generic;
pub mod grid;
pub mod level;
pub mod mep;
pub mod openings;
pub mod parameters;
pub mod project_organization;
pub mod reference_planes;
pub mod reference_points;
pub mod roof;
pub mod simple_decoder;
pub mod structural;
pub mod styling;
pub mod typed_json;
pub mod wall;
pub mod zones;

pub use simple_decoder::SimpleDecoder;

use crate::Result;
use crate::formats;
use crate::walker::{DecodedElement, ElementDecoder, HandleIndex};

/// Schema-driven MVP class names for Lane Five (M3-05).
///
/// ArcWall is intentionally omitted — use [`arc_wall`] for partition
/// records. `WallType` / `FloorType` remain available via
/// [`all_decoders`] but are not required for the MVP acceptance set.
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
        Box::new(level::LevelDecoder),
        Box::new(category::CategoryDecoder),
        Box::new(category::SubcategoryDecoder),
        Box::new(SimpleDecoder::new("Material")),
        Box::new(SimpleDecoder::new("FillPattern")),
        Box::new(SimpleDecoder::new("LinePattern")),
        Box::new(SimpleDecoder::new("LineStyle")),
        Box::new(SimpleDecoder::new("BasePoint")),
        Box::new(SimpleDecoder::new("SurveyPoint")),
        Box::new(SimpleDecoder::new("ProjectPosition")),
        Box::new(SimpleDecoder::new("Grid")),
        Box::new(SimpleDecoder::new("GridType")),
        Box::new(reference_planes::ReferencePlaneDecoder),
        Box::new(wall::WallDecoder),
        Box::new(wall::WallTypeDecoder),
        Box::new(floor::FloorDecoder),
        Box::new(floor::FloorTypeDecoder),
        Box::new(roof::RoofDecoder),
        Box::new(roof::RoofTypeDecoder),
        Box::new(ceiling::CeilingDecoder),
        Box::new(ceiling::CeilingTypeDecoder),
        Box::new(openings::DoorDecoder),
        Box::new(openings::WindowDecoder),
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
/// [`arc_wall::decode_candidate`] instead.
pub fn decoder_for_class(name: &str) -> Option<Box<dyn ElementDecoder>> {
    all_decoders().into_iter().find(|d| d.class_name() == name)
}

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
    let decoder = decoder_for_class(&schema.name).ok_or_else(|| {
        crate::Error::BasicFileInfo(format!(
            "no typed decoder registered for class {}",
            schema.name
        ))
    })?;
    decoder.decode(bytes, schema, index)
}

/// [`decode_typed`] under the caller's walker caps (#852).
pub fn decode_typed_with_limits(
    bytes: &[u8],
    schema: &formats::ClassEntry,
    index: &HandleIndex,
    limits: crate::walker::WalkerLimits,
) -> Result<DecodedElement> {
    let decoder = decoder_for_class(&schema.name).ok_or_else(|| {
        crate::Error::BasicFileInfo(format!(
            "no typed decoder registered for class {}",
            schema.name
        ))
    })?;
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
