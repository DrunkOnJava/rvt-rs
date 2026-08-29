//! Per-class element decoders.
//!
//! Each submodule implements [`crate::walker::ElementDecoder`] for
//! one Revit class. Adding a new class is a three-file change:
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
pub mod structural;
pub mod styling;
pub mod typed_json;
pub mod wall;
pub mod zones;

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
        Box::new(styling::MaterialDecoder),
        Box::new(styling::FillPatternDecoder),
        Box::new(styling::LinePatternDecoder),
        Box::new(styling::LineStyleDecoder),
        Box::new(reference_points::BasePointDecoder),
        Box::new(reference_points::SurveyPointDecoder),
        Box::new(reference_points::ProjectPositionDecoder),
        Box::new(grid::GridDecoder),
        Box::new(grid::GridTypeDecoder),
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
        Box::new(structural::ColumnDecoder),
        Box::new(structural::StructuralColumnDecoder),
        Box::new(structural::BeamDecoder),
        Box::new(structural::StructuralFramingDecoder),
        Box::new(circulation::StairDecoder),
        Box::new(circulation::StairTypeDecoder),
        Box::new(circulation::RailingDecoder),
        Box::new(circulation::RailingTypeDecoder),
        Box::new(zones::RoomDecoder),
        Box::new(zones::AreaDecoder),
        Box::new(zones::SpaceDecoder),
        Box::new(foundation_and_furnishings::StructuralFoundationDecoder),
        Box::new(foundation_and_furnishings::FurnitureDecoder),
        Box::new(foundation_and_furnishings::FurnitureSystemDecoder),
        Box::new(foundation_and_furnishings::CaseworkDecoder),
        Box::new(foundation_and_furnishings::RebarDecoder),
        Box::new(project_organization::PhaseDecoder),
        Box::new(project_organization::DesignOptionDecoder),
        Box::new(project_organization::WorksetDecoder),
        Box::new(generic::GenericModelDecoder),
        Box::new(generic::MassDecoder),
        Box::new(curtain_wall::CurtainWallDecoder),
        Box::new(curtain_wall::CurtainGridDecoder),
        Box::new(curtain_wall::CurtainMullionDecoder),
        Box::new(curtain_wall::CurtainPanelDecoder),
        Box::new(family::SymbolDecoder),
        Box::new(family::FamilyInstanceDecoder),
        Box::new(drafting::ViewDecoder),
        Box::new(drafting::SheetDecoder),
        Box::new(drafting::ScheduleDecoder),
        Box::new(drafting::ScheduleViewDecoder),
        Box::new(annotations::DimensionDecoder),
        Box::new(annotations::TagDecoder),
        Box::new(annotations::TextNoteDecoder),
        Box::new(annotations::AnnotationDecoder),
        Box::new(project_organization::RevisionDecoder),
        Box::new(parameters::ParameterElementDecoder),
        Box::new(parameters::SharedParameterDecoder),
        Box::new(mep::ElectricalEquipmentDecoder),
        Box::new(mep::ElectricalFixtureDecoder),
        Box::new(mep::LightingFixtureDecoder),
        Box::new(mep::LightingDeviceDecoder),
        Box::new(mep::DuctDecoder),
        Box::new(mep::DuctFittingDecoder),
        Box::new(mep::MechanicalEquipmentDecoder),
        Box::new(mep::PipeDecoder),
        Box::new(mep::PipeFittingDecoder),
        Box::new(mep::PlumbingFixtureDecoder),
        Box::new(mep::SpecialtyEquipmentDecoder),
        // L5B-54: AProperty* value-carrier classes. The schema
        // represents every element's parameter values as instances
        // of one of these subclasses, keyed by name to a matching
        // ParameterElement definition.
        Box::new(parameters::APropertyDecoder),
        Box::new(parameters::APropertyBooleanDecoder),
        Box::new(parameters::APropertyIntegerDecoder),
        Box::new(parameters::APropertyEnumDecoder),
        Box::new(parameters::APropertyDouble1Decoder),
        Box::new(parameters::APropertyDouble3Decoder),
        Box::new(parameters::APropertyFloatDecoder),
        Box::new(parameters::APropertyFloat3Decoder),
        Box::new(parameters::APropertyStringDecoder),
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

/// Prefer a typed MVP decoder for `class_name`, else generic
/// [`crate::walker::decode_instance_with_limits`].
///
/// When `class_name` is in [`MVP_TYPED_CLASSES`] and the registered
/// decoder rejects (wrong schema / empty), returns `None` — callers
/// must not invent a typed success via the generic fallback.
/// Non-MVP classes always use the generic decoder.
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

/// Same as [`decode_instance_prefer_typed`] with explicit walker caps
/// on the generic (non-MVP) path.
pub fn decode_instance_prefer_typed_with_limits(
    bytes: &[u8],
    start: usize,
    schema: &formats::ClassEntry,
    limits: crate::walker::WalkerLimits,
) -> Option<DecodedElement> {
    use typed_json::is_mvp_typed_class;
    if is_mvp_typed_class(&schema.name) {
        let slice = bytes.get(start..)?;
        match decode_typed(slice, schema, &HandleIndex::new()) {
            Ok(mut decoded) => {
                // Typed decoders walk from offset 0 of `slice`; remap
                // byte_range into the parent buffer coordinate space.
                decoded.byte_range.start = decoded.byte_range.start.saturating_add(start);
                decoded.byte_range.end = decoded.byte_range.end.saturating_add(start);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_decoders_len_is_eighty_one() {
        // Public docs (README / status / compatibility) claim 81.
        // Keep the count and the docs in lockstep.
        assert_eq!(all_decoders().len(), 81);
    }

    #[test]
    fn all_decoders_includes_level() {
        let decoders = all_decoders();
        let names: Vec<&str> = decoders.iter().map(|d| d.class_name()).collect();
        assert!(names.contains(&"Level"));
    }

    #[test]
    fn all_decoders_includes_mvp_set() {
        let names: std::collections::BTreeSet<&str> =
            all_decoders().iter().map(|d| d.class_name()).collect();
        for class in MVP_TYPED_CLASSES {
            assert!(
                names.contains(class),
                "MVP class {class} missing from all_decoders()"
            );
        }
    }

    #[test]
    fn all_decoders_includes_category_and_subcategory() {
        let decoders = all_decoders();
        let names: Vec<&str> = decoders.iter().map(|d| d.class_name()).collect();
        assert!(names.contains(&"Category"));
        assert!(names.contains(&"Subcategory"));
    }

    #[test]
    fn decoder_class_names_are_unique() {
        let decoders = all_decoders();
        let mut seen = std::collections::BTreeSet::new();
        for d in &decoders {
            assert!(
                seen.insert(d.class_name()),
                "duplicate decoder for class {}",
                d.class_name()
            );
        }
    }

    #[test]
    fn decoder_for_class_roundtrip() {
        assert_eq!(
            decoder_for_class("Wall").map(|d| d.class_name()),
            Some("Wall")
        );
        assert!(decoder_for_class("ArcWall").is_none());
        assert!(decoder_for_class("NoSuchClass").is_none());
    }

    #[test]
    fn decode_typed_rejects_unregistered_class() {
        let schema = formats::ClassEntry {
            name: "NoSuchClass".into(),
            offset: 0,
            fields: vec![],
            tag: None,
            parent: None,
            declared_field_count: None,
            was_parent_only: false,
            ancestor_tag: None,
        };
        let err = decode_typed(&[], &schema, &HandleIndex::new()).unwrap_err();
        assert!(err.to_string().contains("no typed decoder"));
    }
}
