//! Bridge from decoded elements into the IFC model.
//!
//! This is the "one call" integration layer: feed in `DecodedElement`
//! values (produced by the per-class decoders in `src/elements/`) and
//! get back an `IfcModel` that `write_step` will serialise as a valid
//! IFC4 STEP file with per-element `IfcWall` / `IfcSlab` / `IfcDoor`
//! / … entities wired to the storey.
//!
//! Callers bring their own decoded elements; we handle the class-name
//! → IFC-type mapping via [`super::category_map::lookup`] and the
//! spatial-containment wiring happens in the STEP writer.
//!
//! Each [`ElementInput`] carries what the caller knows about the element:
//! an extrusion, a material and its property sets. Its layer-set and
//! profile-set indices point into `IfcModel` collections the bridge leaves
//! empty, so a caller that uses them fills those collections itself. The production exporter, [`super::RvtDocExporter`], does
//! not go through this bridge: it builds its model from the file's element
//! records, with their bodies, type objects and GlobalIds. The bridge is for
//! callers that bring their own decoded elements, such as
//! `tests/ifc_synthetic_project.rs`.

use super::entities::{
    Classification, Extrusion, IfcEntity, Property, PropertySet, PropertyValue, UnitAssignment,
};
use super::{IfcModel, MaterialInfo, Storey};
use crate::elements::openings::Window;
use crate::elements::wall::Wall;
use crate::walker::DecodedElement;

/// Input record for the bridge: one decoded element plus a display
/// name resolved by the caller (usually the decoded element's `name`
/// field, an instance tag like "Wall-1"/"Wall-2", or a category
/// label). Keeping the display-name resolution out-of-band means this
/// bridge doesn't need to know about the caller's naming scheme.
///
/// The `guid` field is optional and, when present, is carried into
/// the IFC entity's `Tag` attribute — useful for round-tripping
/// Revit element IDs.
#[derive(Debug, Clone)]
pub struct ElementInput<'a> {
    pub decoded: &'a DecodedElement,
    pub display_name: String,
    pub guid: Option<String>,
    /// Which storey contains this element. Index into
    /// `BuilderOptions.storeys`. `None` → element lands in
    /// storey index 0 (fine when only one storey is defined or
    /// when the element's level hasn't been resolved yet).
    pub storey_index: Option<usize>,
    /// Which material the element associates with. Index into
    /// `BuilderOptions.materials`. `None` = no material emitted.
    pub material_index: Option<usize>,
    /// Property sets to emit for this element, typically from the
    /// decoded typed view (Wall, Window) through [`wall_property_set`]
    /// and [`window_property_set`]. The first is written as the
    /// element's own set, each further one as an
    /// [`IfcEntity::ElementPropertySet`] of it.
    pub property_sets: Vec<PropertySet>,
    /// Element origin in feet `[x, y, z]`. Typically sourced from
    /// the decoded typed view's `location` field. When `Some`, the
    /// writer emits a unique placement for this element; when
    /// `None`, elements share the identity placement at (0,0,0).
    pub location_feet: Option<[f64; 3]>,
    /// Element yaw rotation in radians (rotation about the +Z
    /// axis). Only consulted when `location_feet` is `Some`.
    pub rotation_radians: Option<f64>,
    /// Optional rectangular-extrusion geometry. When `Some`, the
    /// writer emits an IfcExtrudedAreaSolid chain so BlenderBIM /
    /// IfcOpenShell render the element as a 3D volume. See the
    /// `wall_extrusion` / `slab_extrusion` helpers for the
    /// typical recipe.
    pub extrusion: Option<Extrusion>,
    /// Host-element index for doors / windows / openings. When set
    /// alongside `extrusion`, the writer emits an
    /// IfcOpeningElement + IfcRelVoidsElement + IfcRelFillsElement
    /// so the host wall actually shows a hole. Index refers to
    /// position in the `ElementInput` vec passed to `build_ifc_model`
    /// (same as `model.entities` ordering).
    pub host_element_index: Option<usize>,
    /// Optional reference to a [`crate::ifc::entities::MaterialLayerSet`]
    /// in the outgoing `IfcModel.material_layer_sets`. When
    /// set, takes precedence over `material_index` — the element is
    /// emitted with an `IfcMaterialLayerSetUsage` association
    /// instead of a single `IfcMaterial`. `None` preserves the
    /// single-material behaviour.
    // Tracker: IFC-28.
    pub material_layer_set_index: Option<usize>,
    /// Optional reference to a [`crate::ifc::entities::MaterialProfileSet`]
    /// in the outgoing `IfcModel.material_profile_sets`.
    /// For structural framing (columns / beams) with named
    /// cross-sections. Takes precedence over `material_layer_set_index`
    /// and `material_index` when set.
    // Tracker: IFC-30.
    pub material_profile_set_index: Option<usize>,
    /// Optional richer solid geometry.
    /// When `Some`, the writer emits one of
    /// `IfcRevolvedAreaSolid` / `IfcBooleanResult` /
    /// `IfcFacetedBrep` into the element's Representation slot
    /// **instead of** the `IfcExtrudedAreaSolid` chain driven by
    /// `extrusion`. See
    /// [`crate::ifc::entities::SolidShape`] for the variant
    /// vocabulary. Precedence: `solid_shape` wins when both it
    /// and `extrusion` are set.
    // Tracker: IFC-18, IFC-19, IFC-20.
    pub solid_shape: Option<crate::ifc::entities::SolidShape>,
    /// Index into the outgoing `IfcModel.representation_maps` —
    /// when set, the writer emits an `IfcMappedItem` +
    /// `IfcShapeRepresentation` with representation-type
    /// `'MappedRepresentation'` instead of a per-instance
    /// extrusion / solid chain. Use this to share one
    /// compiled shape across many instances of a Revit Symbol
    /// (family type). Highest-precedence slot — beats
    /// `solid_shape` and `extrusion`.
    // Tracker: IFC-21.
    pub representation_map_index: Option<usize>,
}

/// Options controlling the bridge's output.
#[derive(Debug, Clone, Default)]
pub struct BuilderOptions {
    /// Classifications to carry through (usually populated from
    /// PartAtom taxonomies).
    pub classifications: Vec<Classification>,
    /// Unit assignments (usually populated by
    /// `RvtDocExporter::build_unit_list`).
    pub units: Vec<UnitAssignment>,
    /// Project name override. If `None`, falls back to the first
    /// element's class name.
    pub project_name: Option<String>,
    /// Project description.
    pub description: Option<String>,
    /// Building storeys, one per level the caller wants as an
    /// `IfcBuildingStorey`.
    pub storeys: Vec<Storey>,
    /// Materials the caller wants associated with elements.
    /// BuildingElement.material_index points into this list.
    pub materials: Vec<MaterialInfo>,
}

/// Build a rectangular `Extrusion` from a decoded [`Wall`] plus
/// an explicit length. Revit doesn't carry a wall's length on the
/// Wall element itself — it's derived from the location-curve
/// handle (not yet wired through). Callers that know the length
/// in feet can pass it directly; the bridge consumer is expected
/// to resolve the location curve once the walker surfaces it.
///
/// - `length_feet` → profile width (local X).
/// - `wall_type.width_feet` → profile depth (local Y = wall
///   thickness). Falls back to 8 inches (0.667 ft) if None.
/// - `wall.unconnected_height_feet` → extrusion height (local Z).
///   Falls back to 10 ft when not available.
pub fn wall_extrusion(
    wall: &Wall,
    wall_type: Option<&crate::elements::wall::WallType>,
    length_feet: f64,
) -> Extrusion {
    Extrusion {
        width_feet: length_feet,
        depth_feet: wall_type.and_then(|wt| wt.width_feet).unwrap_or(8.0 / 12.0),
        height_feet: wall.unconnected_height_feet.unwrap_or(10.0),
        profile_override: None,
    }
}

/// Length in feet of a straight 2D location-curve segment.
///
/// Revit walls store their geometry as a 2D location line — two
/// points in the project XY plane. The wall body is the rectangle
/// swept from this line, with thickness perpendicular to the line
/// direction and height along +Z.
///
/// This helper computes the length of a straight segment between
/// two endpoints. Callers with multi-segment polylines should call
/// it once per segment and build a [`wall_extrusion`] for each.
// Tracker: GEO-27.
pub fn wall_segment_length_feet(start: [f64; 2], end: [f64; 2]) -> f64 {
    let dx = end[0] - start[0];
    let dy = end[1] - start[1];
    (dx * dx + dy * dy).sqrt()
}

/// Planar rotation (about +Z, radians) of a straight location-line
/// segment. Zero when the line runs along +X; π/2 for +Y;
/// -π/2 for -Y; etc.
///
/// Used alongside [`wall_segment_length_feet`] to compute the
/// IfcWall's `rotation_radians` so the extrusion profile faces
/// the correct direction. Callers threading multi-segment walls
/// call this per segment.
// Tracker: GEO-27.
pub fn wall_segment_angle_radians(start: [f64; 2], end: [f64; 2]) -> f64 {
    let dx = end[0] - start[0];
    let dy = end[1] - start[1];
    dy.atan2(dx)
}

/// Build a rectangular `Extrusion` for a slab from its plan
/// dimensions and its thickness in feet.
///
/// Defaults: thickness 12 inches (1 ft) when `thickness_feet` is
/// `None`.
pub fn slab_extrusion(length_feet: f64, width_feet: f64, thickness_feet: Option<f64>) -> Extrusion {
    Extrusion {
        width_feet: length_feet,
        depth_feet: width_feet,
        height_feet: thickness_feet.unwrap_or(1.0),
        profile_override: None,
    }
}

/// Shoelace-formula area for a closed planar polygon
/// given as a sequence of `(x, y)` vertices in feet. Returns the
/// unsigned area in square feet.
///
/// The caller does not need to close the loop explicitly — we treat
/// the last vertex and the first vertex as connected. Degenerate
/// inputs (< 3 points) return 0.
///
/// Used to compute `Qto_SlabBaseQuantities.GrossArea` / `NetArea`
/// for IfcSlab / IfcFloor property sets, and as a sanity check that
/// a decoded boundary really encloses area before it is extruded.
// Tracker: GEO-28.
pub fn polygon_area_sqft(points: &[(f64, f64)]) -> f64 {
    crate::vector_math::polygon_area_xy(points)
}

/// The named sets that hold at least one property, in order.
fn non_empty_sets<const N: usize>(sets: [(&str, Vec<Property>); N]) -> Vec<PropertySet> {
    sets.into_iter()
        .filter(|(_, properties)| !properties.is_empty())
        .map(|(name, properties)| PropertySet {
            name: name.into(),
            properties,
        })
        .collect()
}

/// The property sets of a decoded [`Wall`]: rvt-rs's `RvtWall` with its
/// offsets, unconnected height, structural usage and location line,
/// none of which IFC4's `Pset_WallCommon` declares. Fields that are
/// `None` are skipped, and a set with nothing read is left out.
pub fn wall_property_set(wall: &Wall) -> Vec<PropertySet> {
    let mut props = Vec::new();
    if let Some(v) = wall.base_offset_feet {
        props.push(Property {
            name: "BaseOffset".into(),
            value: PropertyValue::LengthFeet(v),
        });
    }
    if let Some(v) = wall.top_offset_feet {
        props.push(Property {
            name: "TopOffset".into(),
            value: PropertyValue::LengthFeet(v),
        });
    }
    if let Some(v) = wall.unconnected_height_feet {
        props.push(Property {
            name: "UnconnectedHeight".into(),
            value: PropertyValue::LengthFeet(v),
        });
    }
    if let Some(usage) = wall.structural_usage {
        props.push(Property {
            name: "StructuralUsage".into(),
            value: PropertyValue::Text(format!("{usage:?}")),
        });
    }
    if let Some(line) = wall.location_line {
        props.push(Property {
            name: "LocationLine".into(),
            value: PropertyValue::Text(format!("{line:?}")),
        });
    }
    non_empty_sets([("RvtWall", props)])
}

/// The property sets of a decoded [`Window`]: rvt-rs's `RvtWindow` with
/// its sill height and rotation, which IFC4's `Pset_WindowCommon` does
/// not declare. A set with nothing read is left out.
pub fn window_property_set(window: &Window) -> Vec<PropertySet> {
    non_empty_sets([("RvtWindow", window_properties(window))])
}

fn window_properties(window: &Window) -> Vec<Property> {
    let mut props = Vec::new();
    if let Some(v) = window.sill_height_feet {
        props.push(Property {
            name: "SillHeight".into(),
            value: PropertyValue::LengthFeet(v),
        });
    }
    if let Some(v) = window.rotation_radians {
        props.push(Property {
            name: "Rotation".into(),
            value: PropertyValue::AngleRadians(v),
        });
    }
    props
}

/// Build an `IfcModel` from a slice of decoded elements.
///
/// Each input element is mapped to an `IfcEntity::BuildingElement`
/// via [`super::category_map::lookup`]. Unknown classes fall back to
/// `IFCBUILDINGELEMENTPROXY` (IFC4 catch-all) rather than being
/// silently dropped — round-tripping an unknown class is more useful
/// than losing it.
pub fn build_ifc_model(inputs: &[ElementInput<'_>], options: BuilderOptions) -> IfcModel {
    let mut entities: Vec<IfcEntity> = Vec::with_capacity(inputs.len());
    let mut further_sets = Vec::new();
    for (element, input) in inputs.iter().enumerate() {
        let mut sets = input.property_sets.iter().cloned();
        let property_set = sets.next();
        further_sets.extend(sets.map(|set| IfcEntity::ElementPropertySet { element, set }));
        let mapping = super::category_map::lookup(&input.decoded.class);
        let ifc_type = mapping
            .map(|m| m.ifc_type)
            .unwrap_or("IFCBUILDINGELEMENTPROXY");
        entities.push(IfcEntity::BuildingElement {
            ifc_type: ifc_type.to_string(),
            name: input.display_name.clone(),
            type_guid: input.guid.clone(),
            predefined_type: mapping.and_then(|m| m.predefined_type).map(str::to_string),
            storey_index: input.storey_index,
            material_index: input.material_index,
            property_set,
            location_feet: input.location_feet,
            rotation_radians: input.rotation_radians,
            extrusion: input.extrusion.clone(),
            host_element_index: input.host_element_index,
            material_layer_set_index: input.material_layer_set_index,
            material_profile_set_index: input.material_profile_set_index,
            solid_shape: input.solid_shape.clone(),
            representation_map_index: input.representation_map_index,
        });
    }
    entities.extend(further_sets);
    let project_name = options.project_name.or_else(|| {
        inputs
            .first()
            .map(|e| format!("RVT project ({})", e.decoded.class))
    });
    IfcModel {
        project_name,
        description: options.description,
        entities,
        classifications: options.classifications,
        units: options.units,
        building_storeys: options.storeys,
        materials: options.materials,
        material_layer_sets: Vec::new(),
        material_profile_sets: Vec::new(),
        representation_maps: Vec::new(),
        global_ids: Default::default(),
        element_layers: Default::default(),
        material_layer_usages: Default::default(),
        element_type_materials: Default::default(),
        element_type_ids: Default::default(),
        type_property_sets: Default::default(),
        material_constituent_sets: Vec::new(),
        opening_cuts: Default::default(),
        body_pieces: Default::default(),
    }
}
