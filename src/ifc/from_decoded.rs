//! Bridge from Layer-5b decoded elements into the IFC model.
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
//! # What this does NOT do yet
//!
//! - **Geometry.** Every emitted element has no `IfcShapeRepresentation`
//!   attached. Phase 5 produces the solids / curves; then we add
//!   `IfcExtrudedAreaSolid` / `IfcFacetedBrep` here.
//! - **Materials.** Need the Material / FillPattern decoders' output
//!   threaded through — straightforward once we accept a
//!   `StylingCatalog` alongside the decoded elements.
//! - **Property sets.** Parameter decoding is L5B-53..56.
//! - **Type → instance linking.** `IfcTypeObject` is IFC-21 / IFC-22.
//!
//! The deliberate minimalism here keeps the integration surface tiny
//! so each future layer (geometry, materials, properties) attaches
//! orthogonally.

use super::entities::{
    Classification, Extrusion, IfcEntity, Property, PropertySet, PropertyValue, UnitAssignment,
};
use super::{IfcModel, MaterialInfo, Storey};
use crate::elements::circulation::Stair;
use crate::elements::level::Level;
use crate::elements::openings::{Door, Window};
use crate::elements::styling::Material;
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
    /// Optional property set to emit for this element. Populated
    /// typically from the decoded typed view (Wall/Floor/Door/…) —
    /// see the `*_property_set` helpers below.
    pub property_set: Option<PropertySet>,
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
    /// in the outgoing `IfcModel.material_layer_sets` (IFC-28). When
    /// set, takes precedence over `material_index` — the element is
    /// emitted with an `IfcMaterialLayerSetUsage` association
    /// instead of a single `IfcMaterial`. `None` preserves the
    /// single-material behaviour.
    pub material_layer_set_index: Option<usize>,
    /// Optional reference to a [`crate::ifc::entities::MaterialProfileSet`]
    /// in the outgoing `IfcModel.material_profile_sets` (IFC-30).
    /// For structural framing (columns / beams) with named
    /// cross-sections. Takes precedence over `material_layer_set_index`
    /// and `material_index` when set.
    pub material_profile_set_index: Option<usize>,
    /// Optional richer solid geometry (IFC-18 / IFC-19 / IFC-20).
    /// When `Some`, the writer emits one of
    /// `IfcRevolvedAreaSolid` / `IfcBooleanResult` /
    /// `IfcFacetedBrep` into the element's Representation slot
    /// **instead of** the `IfcExtrudedAreaSolid` chain driven by
    /// `extrusion`. See
    /// [`crate::ifc::entities::SolidShape`] for the variant
    /// vocabulary. Precedence: `solid_shape` wins when both it
    /// and `extrusion` are set.
    pub solid_shape: Option<crate::ifc::entities::SolidShape>,
    /// Index into the outgoing `IfcModel.representation_maps` —
    /// when set, the writer emits an `IfcMappedItem` +
    /// `IfcShapeRepresentation` with representation-type
    /// `'MappedRepresentation'` instead of a per-instance
    /// extrusion / solid chain (IFC-21). Use this to share one
    /// compiled shape across many instances of a Revit Symbol
    /// (family type). Highest-precedence slot — beats
    /// `solid_shape` and `extrusion`.
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
    /// Building storeys derived from Revit `Level` decoders. See
    /// [`storeys_from_levels`] to derive these from a slice of
    /// decoded [`Level`] values.
    pub storeys: Vec<Storey>,
    /// Materials derived from Revit `Material` decoders. See
    /// [`materials_from_revit`]. BuildingElement.material_index
    /// points into this list.
    pub materials: Vec<MaterialInfo>,
}

/// Derive [`Storey`] entries from a slice of decoded Revit
/// [`Level`] values, dropping entries that lack a name (name is
/// required for IFC `LongName`).
///
/// `is_building_story = false` entries are skipped — those are
/// reference planes used only by drafting views, not real floors
/// (Revit's own IFC exporter makes the same filter).
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

/// Length in feet of a straight 2D location-curve segment (GEO-27).
///
/// Revit walls store their geometry as a 2D location line — two
/// points in the project XY plane. The wall body is the rectangle
/// swept from this line, with thickness perpendicular to the line
/// direction and height along +Z.
///
/// This helper computes the length of a straight segment between
/// two endpoints. Callers with multi-segment polylines should call
/// it once per segment and build a [`wall_extrusion`] for each, or
/// use [`wall_extrusion_from_location_line`] for the single-segment
/// common case.
pub fn wall_segment_length_feet(start: [f64; 2], end: [f64; 2]) -> f64 {
    let dx = end[0] - start[0];
    let dy = end[1] - start[1];
    (dx * dx + dy * dy).sqrt()
}

/// Planar rotation (about +Z, radians) of a straight location-line
/// segment (GEO-27). Zero when the line runs along +X; π/2 for +Y;
/// -π/2 for -Y; etc.
///
/// Used alongside [`wall_segment_length_feet`] to compute the
/// IfcWall's `rotation_radians` so the extrusion profile faces
/// the correct direction. Callers threading multi-segment walls
/// call this per segment.
pub fn wall_segment_angle_radians(start: [f64; 2], end: [f64; 2]) -> f64 {
    let dx = end[0] - start[0];
    let dy = end[1] - start[1];
    dy.atan2(dx)
}

/// Build a rectangular `Extrusion` from a decoded [`Wall`] + its
/// 2D location-line endpoints (GEO-27).
///
/// Equivalent to calling [`wall_extrusion`] with the computed
/// segment length, but expresses the caller's intent directly:
/// "this wall goes from `start` to `end` in the project XY plane."
///
/// - Profile width = segment length (Euclidean distance between
///   endpoints in feet).
/// - Profile depth = `wall_type.width_feet` (thickness), falling
///   back to 8 inches.
/// - Extrusion height = `wall.unconnected_height_feet` or 10 ft.
///
/// Companion [`wall_segment_angle_radians`] gives the rotation to
/// thread into `ElementInput.rotation_radians` so the profile
/// faces the right way.
pub fn wall_extrusion_from_location_line(
    wall: &Wall,
    wall_type: Option<&crate::elements::wall::WallType>,
    start: [f64; 2],
    end: [f64; 2],
) -> Extrusion {
    wall_extrusion(wall, wall_type, wall_segment_length_feet(start, end))
}

/// Decompose a compound wall into its per-layer extrusions (GEO-27).
///
/// A real Revit wall carries a layer set (gypsum / insulation /
/// sheathing / brick). Each layer has its own thickness but the
/// whole stack shares the wall's length and height. This helper
/// takes the location line + the layer thicknesses (in outermost-
/// to-innermost order) and returns one [`Extrusion`] per layer,
/// each offset inward along the thickness axis by the cumulative
/// sum of previous layers.
///
/// The returned vec preserves layer order. Callers can emit each
/// layer as its own `BuildingElement` (IFC4's
/// `IfcRelAggregates(MultiLayerWall, [Layer1, Layer2, …])` pattern)
/// or combine them into a single extrusion with the total thickness
/// and attach the full layer set via
/// [`BuildingElement::material_layer_set_index`] (IFC-28).
///
/// Empty or zero-thickness layer lists return an empty vec.
pub fn wall_layered_extrusions_from_location_line(
    wall: &Wall,
    start: [f64; 2],
    end: [f64; 2],
    layer_thicknesses_feet: &[f64],
) -> Vec<Extrusion> {
    let length = wall_segment_length_feet(start, end);
    let height = wall.unconnected_height_feet.unwrap_or(10.0);
    layer_thicknesses_feet
        .iter()
        .filter(|t| **t > 0.0)
        .map(|thickness| Extrusion {
            width_feet: length,
            depth_feet: *thickness,
            height_feet: height,
            profile_override: None,
        })
        .collect()
}

/// Build a rectangular `Extrusion` for a slab from its plan
/// dimensions and a thickness from the
/// [`crate::elements::floor::FloorType`].
///
/// Defaults: thickness 12 inches (1 ft) when `floor_type` is
/// `None` or lacks a thickness.
pub fn slab_extrusion(
    length_feet: f64,
    width_feet: f64,
    floor_type: Option<&crate::elements::floor::FloorType>,
) -> Extrusion {
    Extrusion {
        width_feet: length_feet,
        depth_feet: width_feet,
        height_feet: floor_type.and_then(|ft| ft.thickness_feet).unwrap_or(1.0),
        profile_override: None,
    }
}

/// Shoelace-formula area (GEO-28) for a closed planar polygon
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
pub fn polygon_area_sqft(points: &[(f64, f64)]) -> f64 {
    if points.len() < 3 {
        return 0.0;
    }
    let n = points.len();
    let mut sum = 0.0;
    for i in 0..n {
        let (x0, y0) = points[i];
        let (x1, y1) = points[(i + 1) % n];
        sum += x0 * y1 - x1 * y0;
    }
    (sum * 0.5).abs()
}

/// Summed edge length (GEO-28) of a closed polygon given as a
/// sequence of `(x, y)` vertices in feet. The last-to-first closing
/// edge is included.
///
/// Matches `Qto_SlabBaseQuantities.Perimeter`. Returns 0 for inputs
/// with fewer than 2 points.
pub fn polygon_perimeter_feet(points: &[(f64, f64)]) -> f64 {
    if points.len() < 2 {
        return 0.0;
    }
    let n = points.len();
    let mut sum = 0.0;
    for i in 0..n {
        let (x0, y0) = points[i];
        let (x1, y1) = points[(i + 1) % n];
        let dx = x1 - x0;
        let dy = y1 - y0;
        sum += (dx * dx + dy * dy).sqrt();
    }
    sum
}

/// Build an `Extrusion` for a floor from its **decoded boundary
/// sketch** (GEO-28).
///
/// Revit stores slab geometry as a 2D closed loop of model lines
/// (the "sketch") plus a per-FloorType thickness. This helper
/// threads the sketch directly into an
/// `IFCArbitraryClosedProfileDef` + vertical extrusion, replacing
/// the bounding-box approximation in [`slab_extrusion`] with the
/// actual polygon. Thickness falls back to 12 inches when
/// `floor_type` is `None` or lacks it.
///
/// Inputs with fewer than 3 points fall back to a 1×1 ft rectangle
/// at the requested thickness so downstream STEP emission still
/// produces a valid (if degenerate) solid.
///
/// Companion to [`wall_extrusion_from_location_line`] for the slab
/// case. Paired with [`polygon_area_sqft`] and
/// [`polygon_perimeter_feet`] to populate
/// `Qto_SlabBaseQuantities`.
pub fn floor_extrusion_from_boundary(
    boundary: &[(f64, f64)],
    floor_type: Option<&crate::elements::floor::FloorType>,
) -> Extrusion {
    let thickness = floor_type.and_then(|ft| ft.thickness_feet).unwrap_or(1.0);
    if boundary.len() < 3 {
        return Extrusion {
            width_feet: 1.0,
            depth_feet: 1.0,
            height_feet: thickness,
            profile_override: None,
        };
    }
    Extrusion::arbitrary_closed(boundary.to_vec(), thickness)
}

/// Build a `Qto_SlabBaseQuantities`-style [`PropertySet`] (GEO-28)
/// from a decoded floor boundary + thickness. Populates GrossArea,
/// Perimeter, Depth, and GrossVolume. Empty boundaries yield an
/// empty property set (no bogus zero-valued quantities).
pub fn floor_base_quantities(
    boundary: &[(f64, f64)],
    floor_type: Option<&crate::elements::floor::FloorType>,
) -> PropertySet {
    let mut props = Vec::new();
    let thickness = floor_type.and_then(|ft| ft.thickness_feet).unwrap_or(1.0);
    if boundary.len() >= 3 {
        let area = polygon_area_sqft(boundary);
        let perim = polygon_perimeter_feet(boundary);
        props.push(Property {
            name: "GrossArea".into(),
            value: PropertyValue::AreaSquareFeet(area),
        });
        props.push(Property {
            name: "Perimeter".into(),
            value: PropertyValue::LengthFeet(perim),
        });
        props.push(Property {
            name: "Depth".into(),
            value: PropertyValue::LengthFeet(thickness),
        });
        props.push(Property {
            name: "GrossVolume".into(),
            value: PropertyValue::VolumeCubicFeet(area * thickness),
        });
    }
    PropertySet {
        name: "Qto_SlabBaseQuantities".into(),
        properties: props,
    }
}

/// Build a rectangular `Extrusion` for a roof. Identical shape to
/// a slab — the IfcRoof emission already handles the semantic
/// distinction. Thickness from [`crate::elements::roof::RoofType`]
/// falls back to 12 inches.
pub fn roof_extrusion(
    length_feet: f64,
    width_feet: f64,
    roof_type: Option<&crate::elements::roof::RoofType>,
) -> Extrusion {
    Extrusion {
        width_feet: length_feet,
        depth_feet: width_feet,
        height_feet: roof_type.and_then(|rt| rt.thickness_feet).unwrap_or(1.0),
        profile_override: None,
    }
}

// ---- GEO-29: Roof geometry with slopes ----

/// Convert a US-construction "rise:run" pitch (GEO-29) to radians.
/// Rise is the vertical component, run is the horizontal — e.g.
/// a "6 in 12" pitch passes `rise = 6.0, run = 12.0` and gets
/// back roughly `0.4636` rad (26.57°).
///
/// Returns 0 when `run <= 0.0` so callers can pipe raw Revit
/// parameter values through without pre-validating.
pub fn roof_pitch_radians_from_rise_run(rise: f64, run: f64) -> f64 {
    if run <= 0.0 {
        return 0.0;
    }
    (rise / run).atan()
}

/// Convert a slope in degrees to radians (GEO-29). Helper for
/// callers reading `RVT_PARAM_ROOF_SLOPE` as a plain degree value.
pub fn roof_pitch_radians_from_degrees(degrees: f64) -> f64 {
    degrees.to_radians()
}

/// Ridge-above-eave height (GEO-29) of a symmetric gabled roof
/// spanning `span_feet` between the two eave walls, with rafters
/// rising at `pitch_rad` from horizontal. The ridge sits at the
/// midline, so `height = (span/2) * tan(pitch)`.
///
/// A flat roof (pitch 0) returns 0. Negative span or pitch are
/// clamped to 0.
pub fn gabled_roof_ridge_height(span_feet: f64, pitch_rad: f64) -> f64 {
    let s = span_feet.max(0.0);
    let p = pitch_rad.max(0.0);
    (s * 0.5) * p.tan()
}

/// Ridge length (GEO-29) of a hipped roof with a rectangular
/// footprint. For a hip with equal pitch on all four sides, the
/// ridge runs along the longer dimension for
/// `length - width` feet. When `length == width` the hip degen-
/// erates to a pyramid and the returned ridge length is 0.
///
/// Inputs are normalised so callers can pass length/width in any
/// order: the function uses `max - min` internally.
pub fn hip_roof_ridge_length(length_feet: f64, width_feet: f64) -> f64 {
    (length_feet.max(width_feet) - length_feet.min(width_feet)).max(0.0)
}

/// Build an `Extrusion` for a **gabled** roof (GEO-29): the gable
/// profile is a triangle of base `span_feet` and height
/// `(span_feet/2) * tan(pitch_rad)`, extruded along `length_feet`.
///
/// The caller gets an `IFCArbitraryClosedProfileDef` with three
/// vertices (left eave, ridge, right eave) plus an
/// `IfcExtrudedAreaSolid` of length `length_feet`. The resulting
/// `rotation_radians` on the parent `ElementInput` should place
/// the ridge parallel to `length_feet`.
///
/// A flat (pitch 0) roof falls back to `roof_extrusion(length,
/// span, roof_type)` so the output is still a valid slab solid.
pub fn gabled_roof_extrusion(
    length_feet: f64,
    span_feet: f64,
    pitch_rad: f64,
    roof_type: Option<&crate::elements::roof::RoofType>,
) -> Extrusion {
    if pitch_rad <= 0.0 || span_feet <= 0.0 {
        return roof_extrusion(length_feet, span_feet, roof_type);
    }
    let half_span = span_feet * 0.5;
    let ridge_h = gabled_roof_ridge_height(span_feet, pitch_rad);
    let profile = vec![(-half_span, 0.0), (half_span, 0.0), (0.0, ridge_h)];
    Extrusion::arbitrary_closed(profile, length_feet.max(0.0))
}

/// Build vertex + triangle data (GEO-29) for a **hipped** roof
/// over a rectangular footprint. Returns `(vertices_feet,
/// triangles)` ready for `SolidShape::FacetedBrep`.
///
/// Geometry: eight triangles — two trapezoids on the long sides
/// (each split into 2 triangles) plus two triangles on the short
/// sides. When `length == width` the ridge length is zero and the
/// hip degenerates to a pyramid (4 triangles meeting at the apex).
///
/// All vertices sit in the element-local XY plane at `Z=0`
/// (eaves) or `Z=ridge_height` (ridge line). Callers are
/// responsible for translating to world coordinates.
pub fn hip_roof_brep(
    length_feet: f64,
    width_feet: f64,
    pitch_rad: f64,
) -> (Vec<[f64; 3]>, Vec<super::entities::BrepTriangle>) {
    let l = length_feet.max(0.0);
    let w = width_feet.max(0.0);
    let (long, short) = if l >= w { (l, w) } else { (w, l) };
    let ridge = hip_roof_ridge_length(long, short);
    let ridge_h = gabled_roof_ridge_height(short, pitch_rad.max(0.0));
    let overhang = (long - ridge) * 0.5;

    let swap = l < w;
    // Eave rectangle (CCW from origin)
    let eave = if !swap {
        [
            [0.0, 0.0, 0.0],    // 0
            [long, 0.0, 0.0],   // 1
            [long, short, 0.0], // 2
            [0.0, short, 0.0],  // 3
        ]
    } else {
        // When width > length we're laying the ridge along +Y,
        // so (long, short) must be mapped back to (w, l) axes.
        [
            [0.0, 0.0, 0.0],
            [short, 0.0, 0.0],
            [short, long, 0.0],
            [0.0, long, 0.0],
        ]
    };

    // Ridge endpoints (axis-dependent)
    let (ra, rb) = if !swap {
        (
            [overhang, short * 0.5, ridge_h],
            [long - overhang, short * 0.5, ridge_h],
        )
    } else {
        (
            [short * 0.5, overhang, ridge_h],
            [short * 0.5, long - overhang, ridge_h],
        )
    };

    let mut vertices: Vec<[f64; 3]> = Vec::with_capacity(6);
    vertices.extend_from_slice(&eave);
    vertices.push(ra); // 4
    vertices.push(rb); // 5

    // Face winding: outward-facing normals. 4 = ra, 5 = rb.
    // Long slope A: eave 0-1 up to ridge 5-4.
    // Long slope B: eave 2-3 up to ridge 4-5.
    // Short slope at A-end: eave 3-0 + ridge 4.
    // Short slope at B-end: eave 1-2 + ridge 5.
    let triangles = vec![
        super::entities::BrepTriangle(0, 1, 5),
        super::entities::BrepTriangle(0, 5, 4),
        super::entities::BrepTriangle(2, 3, 4),
        super::entities::BrepTriangle(2, 4, 5),
        super::entities::BrepTriangle(3, 0, 4),
        super::entities::BrepTriangle(1, 2, 5),
    ];
    (vertices, triangles)
}

/// Build a rectangular `Extrusion` for a ceiling. Thickness from
/// [`crate::elements::ceiling::CeilingType`] falls back to 1 inch
/// (ACT ceilings are typically 0.08 ft thick).
pub fn ceiling_extrusion(
    length_feet: f64,
    width_feet: f64,
    ceiling_type: Option<&crate::elements::ceiling::CeilingType>,
) -> Extrusion {
    Extrusion {
        width_feet: length_feet,
        depth_feet: width_feet,
        height_feet: ceiling_type
            .and_then(|ct| ct.thickness_feet)
            .unwrap_or(1.0 / 12.0),
        profile_override: None,
    }
}

/// Build a rectangular `Extrusion` for a column, using the column's
/// own height from level offsets. Profile dimensions are caller-
/// supplied (column profile shape lives on the Symbol, not yet
/// wired through).
pub fn column_extrusion(
    column: &crate::elements::structural::Column,
    profile_width_feet: f64,
    profile_depth_feet: f64,
    level_elevation_diff_feet: f64,
) -> Extrusion {
    // Column height = (top_level.elevation + top_offset) -
    //                 (base_level.elevation + base_offset).
    // Callers provide the level-elevation delta; we add the
    // decoded offsets (None → 0).
    let offset_delta =
        column.top_offset_feet.unwrap_or(0.0) - column.base_offset_feet.unwrap_or(0.0);
    Extrusion {
        width_feet: profile_width_feet,
        depth_feet: profile_depth_feet,
        height_feet: (level_elevation_diff_feet + offset_delta).max(0.1),
        profile_override: None,
    }
}

/// Build a `Pset_WallCommon`-style property set from a decoded
/// [`Wall`]. Fields that are `None` are skipped — property sets
/// only carry what we actually decoded.
pub fn wall_property_set(wall: &Wall) -> PropertySet {
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
    PropertySet {
        name: "Pset_WallCommon".into(),
        properties: props,
    }
}

/// Build a `Pset_DoorCommon`-style property set from a decoded
/// [`Door`].
pub fn door_property_set(door: &Door) -> PropertySet {
    let mut props = Vec::new();
    if let Some(v) = door.rotation_radians {
        props.push(Property {
            name: "Rotation".into(),
            value: PropertyValue::AngleRadians(v),
        });
    }
    if let Some(v) = door.flip_hand {
        props.push(Property {
            name: "FlipHand".into(),
            value: PropertyValue::Boolean(v),
        });
    }
    if let Some(v) = door.flip_facing {
        props.push(Property {
            name: "FlipFacing".into(),
            value: PropertyValue::Boolean(v),
        });
    }
    PropertySet {
        name: "Pset_DoorCommon".into(),
        properties: props,
    }
}

/// Build a `Pset_WindowCommon`-style property set from a decoded
/// [`Window`].
pub fn window_property_set(window: &Window) -> PropertySet {
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
    PropertySet {
        name: "Pset_WindowCommon".into(),
        properties: props,
    }
}

/// Build a `Pset_StairCommon`-style property set from a decoded
/// [`Stair`]. Includes riser/tread counts + calibrated dimensions.
pub fn stair_property_set(stair: &Stair) -> PropertySet {
    let mut props = Vec::new();
    if let Some(v) = stair.actual_riser_count {
        props.push(Property {
            name: "NumberOfRisers".into(),
            value: PropertyValue::Integer(v as i64),
        });
    }
    if let Some(v) = stair.actual_tread_depth_feet {
        props.push(Property {
            name: "TreadDepth".into(),
            value: PropertyValue::LengthFeet(v),
        });
    }
    if let Some(v) = stair.actual_riser_height_feet {
        props.push(Property {
            name: "RiserHeight".into(),
            value: PropertyValue::LengthFeet(v),
        });
    }
    if let Some(total) = stair.total_rise_feet() {
        props.push(Property {
            name: "TotalRise".into(),
            value: PropertyValue::LengthFeet(total),
        });
    }
    PropertySet {
        name: "Pset_StairCommon".into(),
        properties: props,
    }
}

/// Derive [`MaterialInfo`] entries from a slice of decoded Revit
/// [`Material`] values. Drops entries with no name (IFC4 IfcMaterial
/// requires a non-empty Name attribute).
pub fn materials_from_revit(materials: &[Material]) -> Vec<MaterialInfo> {
    materials
        .iter()
        .filter_map(|m| {
            Some(MaterialInfo {
                name: m.name.clone()?,
                color_packed: m.color,
                transparency: m.transparency,
            })
        })
        .collect()
}

pub fn storeys_from_levels(levels: &[Level]) -> Vec<Storey> {
    levels
        .iter()
        .filter(|l| l.is_building_story.unwrap_or(true))
        .filter_map(|l| {
            Some(Storey {
                name: l.name.clone()?,
                elevation_feet: l.elevation_feet.unwrap_or(0.0),
            })
        })
        .collect()
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
    for input in inputs {
        let mapping = super::category_map::lookup(&input.decoded.class);
        let ifc_type = mapping
            .map(|m| m.ifc_type)
            .unwrap_or("IFCBUILDINGELEMENTPROXY");
        entities.push(IfcEntity::BuildingElement {
            ifc_type: ifc_type.to_string(),
            name: input.display_name.clone(),
            type_guid: input.guid.clone(),
            storey_index: input.storey_index,
            material_index: input.material_index,
            property_set: input.property_set.clone(),
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
    }
}

/// Compute counts of each IFC entity type produced by the bridge.
/// Useful for end-to-end smoke tests: "did we actually get 47
/// walls out of a file that contains 47 walls?"
pub fn entity_type_histogram(model: &IfcModel) -> std::collections::BTreeMap<String, usize> {
    let mut out: std::collections::BTreeMap<String, usize> = std::collections::BTreeMap::new();
    for e in &model.entities {
        if let IfcEntity::BuildingElement { ifc_type, .. } = e {
            *out.entry(ifc_type.clone()).or_insert(0) += 1;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::walker::{DecodedElement, InstanceField};

    fn mk_decoded(class: &str) -> DecodedElement {
        DecodedElement {
            id: None,
            class: class.to_string(),
            fields: vec![("name".to_string(), InstanceField::String(class.to_string()))],
            byte_range: 0..0,
        }
    }

    #[test]
    fn build_model_maps_known_classes() {
        let wall = mk_decoded("Wall");
        let floor = mk_decoded("Floor");
        let roof = mk_decoded("Roof");
        let inputs = vec![
            ElementInput {
                decoded: &wall,
                display_name: "Wall-1".into(),
                guid: None,
                storey_index: None,
                material_index: None,
                property_set: None,
                location_feet: None,
                rotation_radians: None,
                extrusion: None,
                host_element_index: None,
                material_layer_set_index: None,
                material_profile_set_index: None,
                solid_shape: None,
                representation_map_index: None,
            },
            ElementInput {
                decoded: &floor,
                display_name: "Slab-1".into(),
                guid: None,
                storey_index: None,
                material_index: None,
                property_set: None,
                location_feet: None,
                rotation_radians: None,
                extrusion: None,
                host_element_index: None,
                material_layer_set_index: None,
                material_profile_set_index: None,
                solid_shape: None,
                representation_map_index: None,
            },
            ElementInput {
                decoded: &roof,
                display_name: "Roof-1".into(),
                guid: None,
                storey_index: None,
                material_index: None,
                property_set: None,
                location_feet: None,
                rotation_radians: None,
                extrusion: None,
                host_element_index: None,
                material_layer_set_index: None,
                material_profile_set_index: None,
                solid_shape: None,
                representation_map_index: None,
            },
        ];
        let model = build_ifc_model(&inputs, BuilderOptions::default());
        assert_eq!(model.entities.len(), 3);
        let hist = entity_type_histogram(&model);
        assert_eq!(hist.get("IFCWALL"), Some(&1));
        assert_eq!(hist.get("IFCSLAB"), Some(&1));
        assert_eq!(hist.get("IFCROOF"), Some(&1));
    }

    #[test]
    fn unknown_class_falls_back_to_proxy() {
        let custom = mk_decoded("SomeCustomAutodeskExtension");
        let inputs = vec![ElementInput {
            decoded: &custom,
            display_name: "Mystery-1".into(),
            guid: None,
            storey_index: None,
            material_index: None,
            property_set: None,
            location_feet: None,
            rotation_radians: None,
            extrusion: None,
            host_element_index: None,
            material_layer_set_index: None,
            material_profile_set_index: None,
            solid_shape: None,
            representation_map_index: None,
        }];
        let model = build_ifc_model(&inputs, BuilderOptions::default());
        let hist = entity_type_histogram(&model);
        assert_eq!(hist.get("IFCBUILDINGELEMENTPROXY"), Some(&1));
    }

    #[test]
    fn project_name_default_uses_first_class() {
        let wall = mk_decoded("Wall");
        let inputs = vec![ElementInput {
            decoded: &wall,
            display_name: "Wall-1".into(),
            guid: None,
            storey_index: None,
            material_index: None,
            property_set: None,
            location_feet: None,
            rotation_radians: None,
            extrusion: None,
            host_element_index: None,
            material_layer_set_index: None,
            material_profile_set_index: None,
            solid_shape: None,
            representation_map_index: None,
        }];
        let model = build_ifc_model(&inputs, BuilderOptions::default());
        assert!(
            model
                .project_name
                .as_deref()
                .unwrap()
                .starts_with("RVT project")
        );
    }

    #[test]
    fn project_name_override_wins() {
        let wall = mk_decoded("Wall");
        let inputs = vec![ElementInput {
            decoded: &wall,
            display_name: "Wall-1".into(),
            guid: None,
            storey_index: None,
            material_index: None,
            property_set: None,
            location_feet: None,
            rotation_radians: None,
            extrusion: None,
            host_element_index: None,
            material_layer_set_index: None,
            material_profile_set_index: None,
            solid_shape: None,
            representation_map_index: None,
        }];
        let opts = BuilderOptions {
            project_name: Some("Acme HQ".into()),
            ..Default::default()
        };
        let model = build_ifc_model(&inputs, opts);
        assert_eq!(model.project_name.as_deref(), Some("Acme HQ"));
    }

    #[test]
    fn empty_input_produces_empty_model() {
        let model = build_ifc_model(&[], BuilderOptions::default());
        assert!(model.entities.is_empty());
        assert!(model.project_name.is_none());
    }

    #[test]
    fn histogram_counts_multiple_of_same_type() {
        let w1 = mk_decoded("Wall");
        let w2 = mk_decoded("Wall");
        let w3 = mk_decoded("Wall");
        let inputs = vec![
            ElementInput {
                decoded: &w1,
                display_name: "Wall-N".into(),
                guid: None,
                storey_index: None,
                material_index: None,
                property_set: None,
                location_feet: None,
                rotation_radians: None,
                extrusion: None,
                host_element_index: None,
                material_layer_set_index: None,
                material_profile_set_index: None,
                solid_shape: None,
                representation_map_index: None,
            },
            ElementInput {
                decoded: &w2,
                display_name: "Wall-E".into(),
                guid: None,
                storey_index: None,
                material_index: None,
                property_set: None,
                location_feet: None,
                rotation_radians: None,
                extrusion: None,
                host_element_index: None,
                material_layer_set_index: None,
                material_profile_set_index: None,
                solid_shape: None,
                representation_map_index: None,
            },
            ElementInput {
                decoded: &w3,
                display_name: "Wall-S".into(),
                guid: None,
                storey_index: None,
                material_index: None,
                property_set: None,
                location_feet: None,
                rotation_radians: None,
                extrusion: None,
                host_element_index: None,
                material_layer_set_index: None,
                material_profile_set_index: None,
                solid_shape: None,
                representation_map_index: None,
            },
        ];
        let model = build_ifc_model(&inputs, BuilderOptions::default());
        let hist = entity_type_histogram(&model);
        assert_eq!(hist.get("IFCWALL"), Some(&3));
    }

    #[test]
    fn storeys_from_levels_filters_non_building_stories() {
        let l1 = Level {
            name: Some("Level 1".into()),
            elevation_feet: Some(0.0),
            is_building_story: Some(true),
            ..Default::default()
        };
        let roof = Level {
            name: Some("Roof".into()),
            elevation_feet: Some(30.0),
            is_building_story: Some(true),
            ..Default::default()
        };
        let drafting_ref = Level {
            name: Some("Drafting Ref".into()),
            elevation_feet: Some(8.0),
            is_building_story: Some(false),
            ..Default::default()
        };
        let unnamed = Level {
            name: None,
            elevation_feet: Some(20.0),
            is_building_story: Some(true),
            ..Default::default()
        };
        let storeys = storeys_from_levels(&[l1, roof, drafting_ref, unnamed]);
        assert_eq!(storeys.len(), 2);
        assert_eq!(storeys[0].name, "Level 1");
        assert_eq!(storeys[1].name, "Roof");
        assert_eq!(storeys[1].elevation_feet, 30.0);
    }

    #[test]
    fn storeys_threaded_through_to_step_output() {
        // Real Level names + elevations should appear in the emitted
        // IfcBuildingStorey, not the placeholder "Level 1".
        let wall = mk_decoded("Wall");
        let opts = BuilderOptions {
            storeys: vec![
                super::super::Storey {
                    name: "Ground Floor".into(),
                    elevation_feet: 0.0,
                },
                super::super::Storey {
                    name: "Second Floor".into(),
                    elevation_feet: 10.0,
                },
                super::super::Storey {
                    name: "Roof Deck".into(),
                    elevation_feet: 20.0,
                },
            ],
            ..Default::default()
        };
        let inputs = vec![ElementInput {
            decoded: &wall,
            display_name: "W-1".into(),
            guid: None,
            storey_index: None,
            material_index: None,
            property_set: None,
            location_feet: None,
            rotation_radians: None,
            extrusion: None,
            host_element_index: None,
            material_layer_set_index: None,
            material_profile_set_index: None,
            solid_shape: None,
            representation_map_index: None,
        }];
        let model = build_ifc_model(&inputs, opts);
        let s = super::super::write_step(&model);
        // Three IfcBuildingStorey entities — one per level.
        assert_eq!(s.matches("IFCBUILDINGSTOREY(").count(), 3);
        // Names survive STEP escape (ASCII → pass-through).
        assert!(s.contains("Ground Floor"));
        assert!(s.contains("Second Floor"));
        assert!(s.contains("Roof Deck"));
        // Second floor's elevation (10 ft = 3.048 m) lands somewhere.
        assert!(s.contains("3.048"), "second-floor elevation missing");
        // One IfcRelAggregates for the building→storeys rel — bundle
        // of all 3 storeys, not 3 separate rels.
        // (Site + building + storeys = 3 total IFCRELAGGREGATES)
        assert_eq!(s.matches("IFCRELAGGREGATES(").count(), 3);
    }

    #[test]
    fn empty_storeys_still_emits_one_placeholder() {
        // When storeys is empty, the writer falls back to one
        // "Level 1" placeholder — the IFC spatial hierarchy still
        // has to be valid.
        let model = build_ifc_model(&[], BuilderOptions::default());
        let s = super::super::write_step(&model);
        assert_eq!(s.matches("IFCBUILDINGSTOREY(").count(), 1);
        assert!(s.contains("Level 1"));
    }

    #[test]
    fn built_model_round_trips_through_step_writer() {
        // End-to-end: decoded elements → IfcModel → STEP text. We
        // don't parse the output, but we do check that the entity
        // names we expect land in the string. This is the tightest
        // unit test of the "one call" pipeline.
        let wall = mk_decoded("Wall");
        let door = mk_decoded("Door");
        let inputs = vec![
            ElementInput {
                decoded: &wall,
                display_name: "North Wall".into(),
                guid: None,
                storey_index: None,
                material_index: None,
                property_set: None,
                location_feet: None,
                rotation_radians: None,
                extrusion: None,
                host_element_index: None,
                material_layer_set_index: None,
                material_profile_set_index: None,
                solid_shape: None,
                representation_map_index: None,
            },
            ElementInput {
                decoded: &door,
                display_name: "Front Door".into(),
                guid: Some("DOOR-GUID-42".into()),
                storey_index: None,
                material_index: None,
                property_set: None,
                location_feet: None,
                rotation_radians: None,
                extrusion: None,
                host_element_index: None,
                material_layer_set_index: None,
                material_profile_set_index: None,
                solid_shape: None,
                representation_map_index: None,
            },
        ];
        let model = build_ifc_model(&inputs, BuilderOptions::default());
        let step = super::super::write_step(&model);
        assert!(step.contains("IFCWALL("));
        assert!(step.contains("IFCDOOR("));
        assert!(step.contains("North Wall"));
        assert!(step.contains("Front Door"));
        assert!(step.contains("DOOR-GUID-42"));
        // Exactly one containment rel regardless of element count.
        assert_eq!(
            step.matches("IFCRELCONTAINEDINSPATIALSTRUCTURE(").count(),
            1
        );
    }

    // ----- GEO-27: wall geometry from location curve + layers -----

    #[test]
    fn wall_segment_length_cardinal_directions() {
        // +X run: length equals |dx|.
        assert!((wall_segment_length_feet([0.0, 0.0], [10.0, 0.0]) - 10.0).abs() < 1e-9);
        // +Y run: length equals |dy|.
        assert!((wall_segment_length_feet([0.0, 0.0], [0.0, 7.5]) - 7.5).abs() < 1e-9);
        // Diagonal 3-4-5 triangle.
        assert!((wall_segment_length_feet([0.0, 0.0], [3.0, 4.0]) - 5.0).abs() < 1e-9);
    }

    #[test]
    fn wall_segment_length_reverse_direction_matches() {
        // Length is direction-independent.
        let fwd = wall_segment_length_feet([0.0, 0.0], [3.0, 4.0]);
        let rev = wall_segment_length_feet([3.0, 4.0], [0.0, 0.0]);
        assert!((fwd - rev).abs() < 1e-12);
    }

    #[test]
    fn wall_segment_angle_cardinal_directions() {
        use std::f64::consts::{FRAC_PI_2, PI};
        // +X → 0 rad.
        assert!(wall_segment_angle_radians([0.0, 0.0], [1.0, 0.0]).abs() < 1e-9);
        // +Y → π/2.
        assert!((wall_segment_angle_radians([0.0, 0.0], [0.0, 1.0]) - FRAC_PI_2).abs() < 1e-9);
        // -X → π.
        assert!((wall_segment_angle_radians([0.0, 0.0], [-1.0, 0.0]).abs() - PI).abs() < 1e-9);
        // -Y → -π/2.
        assert!((wall_segment_angle_radians([0.0, 0.0], [0.0, -1.0]) + FRAC_PI_2).abs() < 1e-9);
    }

    #[test]
    fn wall_extrusion_from_location_line_computes_length() {
        use crate::elements::wall::{Wall, WallType};
        let wall = Wall {
            unconnected_height_feet: Some(10.0),
            ..Default::default()
        };
        let wall_type = WallType {
            width_feet: Some(8.0 / 12.0),
            ..Default::default()
        };
        let ex =
            wall_extrusion_from_location_line(&wall, Some(&wall_type), [0.0, 0.0], [20.0, 0.0]);
        assert!((ex.width_feet - 20.0).abs() < 1e-9);
        assert!((ex.depth_feet - 8.0 / 12.0).abs() < 1e-9);
        assert!((ex.height_feet - 10.0).abs() < 1e-9);
    }

    #[test]
    fn wall_layered_extrusions_preserves_order_and_thickness() {
        use crate::elements::wall::Wall;
        let wall = Wall {
            unconnected_height_feet: Some(9.0),
            ..Default::default()
        };
        let layers = [0.25, 0.5, 0.125]; // 3 inches / 6 inches / 1.5 inches
        let out =
            wall_layered_extrusions_from_location_line(&wall, [0.0, 0.0], [10.0, 0.0], &layers);
        assert_eq!(out.len(), 3);
        for (idx, ex) in out.iter().enumerate() {
            assert!((ex.width_feet - 10.0).abs() < 1e-9);
            assert!((ex.depth_feet - layers[idx]).abs() < 1e-9);
            assert!((ex.height_feet - 9.0).abs() < 1e-9);
        }
    }

    #[test]
    fn wall_layered_extrusions_skips_zero_thickness_layers() {
        use crate::elements::wall::Wall;
        let wall = Wall::default();
        let layers = [0.25, 0.0, 0.125, -1.0]; // includes invalid values
        let out =
            wall_layered_extrusions_from_location_line(&wall, [0.0, 0.0], [1.0, 0.0], &layers);
        // Zero and negative thicknesses are dropped.
        assert_eq!(out.len(), 2);
        assert!((out[0].depth_feet - 0.25).abs() < 1e-9);
        assert!((out[1].depth_feet - 0.125).abs() < 1e-9);
    }

    #[test]
    fn wall_layered_extrusions_empty_input_returns_empty() {
        use crate::elements::wall::Wall;
        let wall = Wall::default();
        let out = wall_layered_extrusions_from_location_line(&wall, [0.0, 0.0], [5.0, 0.0], &[]);
        assert!(out.is_empty());
    }

    // ---- GEO-28: Floor geometry from boundary sketch ----

    #[test]
    fn polygon_area_unit_square_is_one_sqft() {
        let sq = [(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0)];
        assert!((polygon_area_sqft(&sq) - 1.0).abs() < 1e-9);
    }

    #[test]
    fn polygon_area_is_winding_order_invariant() {
        let ccw = [(0.0, 0.0), (10.0, 0.0), (10.0, 5.0), (0.0, 5.0)];
        let cw = [(0.0, 0.0), (0.0, 5.0), (10.0, 5.0), (10.0, 0.0)];
        let a_ccw = polygon_area_sqft(&ccw);
        let a_cw = polygon_area_sqft(&cw);
        assert!((a_ccw - 50.0).abs() < 1e-9);
        assert!((a_cw - 50.0).abs() < 1e-9);
    }

    #[test]
    fn polygon_area_l_shape() {
        // L-shape: unit square with top-right corner cut out
        //   (0,2)-(1,2)-(1,1)-(2,1)-(2,0)-(0,0)
        // Area = 2*2 - 1*1 = 3 sqft
        let ell = [
            (0.0, 0.0),
            (2.0, 0.0),
            (2.0, 1.0),
            (1.0, 1.0),
            (1.0, 2.0),
            (0.0, 2.0),
        ];
        assert!((polygon_area_sqft(&ell) - 3.0).abs() < 1e-9);
    }

    #[test]
    fn polygon_area_degenerate_returns_zero() {
        assert_eq!(polygon_area_sqft(&[]), 0.0);
        assert_eq!(polygon_area_sqft(&[(0.0, 0.0)]), 0.0);
        assert_eq!(polygon_area_sqft(&[(0.0, 0.0), (1.0, 1.0)]), 0.0);
    }

    #[test]
    fn polygon_perimeter_unit_square_is_four_feet() {
        let sq = [(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0)];
        assert!((polygon_perimeter_feet(&sq) - 4.0).abs() < 1e-9);
    }

    #[test]
    fn polygon_perimeter_rectangle_includes_closing_edge() {
        let rect = [(0.0, 0.0), (10.0, 0.0), (10.0, 3.0), (0.0, 3.0)];
        assert!((polygon_perimeter_feet(&rect) - 26.0).abs() < 1e-9);
    }

    #[test]
    fn polygon_perimeter_degenerate_returns_zero() {
        assert_eq!(polygon_perimeter_feet(&[]), 0.0);
        assert_eq!(polygon_perimeter_feet(&[(0.0, 0.0)]), 0.0);
    }

    #[test]
    fn floor_extrusion_from_boundary_uses_polygon_profile() {
        use crate::elements::floor::FloorType;
        let ft = FloorType {
            thickness_feet: Some(0.5),
            ..Default::default()
        };
        let boundary = [(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0)];
        let ex = floor_extrusion_from_boundary(&boundary, Some(&ft));
        assert!((ex.height_feet - 0.5).abs() < 1e-9);
        // arbitrary_closed sets width/depth to the bounding box of the points.
        assert!((ex.width_feet - 10.0).abs() < 1e-9);
        assert!((ex.depth_feet - 10.0).abs() < 1e-9);
        // The profile_override carries the actual polygon.
        match ex.profile_override {
            Some(crate::ifc::entities::ProfileDef::ArbitraryClosed { points }) => {
                assert_eq!(points.len(), 4);
            }
            other => panic!("expected ArbitraryClosed profile, got {:?}", other),
        }
    }

    #[test]
    fn floor_extrusion_falls_back_for_degenerate_boundary() {
        let ex = floor_extrusion_from_boundary(&[], None);
        assert_eq!(ex.width_feet, 1.0);
        assert_eq!(ex.depth_feet, 1.0);
        assert!((ex.height_feet - 1.0).abs() < 1e-9); // default thickness
        assert!(ex.profile_override.is_none());
    }

    #[test]
    fn floor_base_quantities_populates_four_properties() {
        use crate::elements::floor::FloorType;
        let ft = FloorType {
            thickness_feet: Some(0.5),
            ..Default::default()
        };
        let boundary = [(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0)];
        let qto = floor_base_quantities(&boundary, Some(&ft));
        assert_eq!(qto.name, "Qto_SlabBaseQuantities");
        assert_eq!(qto.properties.len(), 4);
        let names: Vec<&str> = qto.properties.iter().map(|p| p.name.as_str()).collect();
        assert!(names.contains(&"GrossArea"));
        assert!(names.contains(&"Perimeter"));
        assert!(names.contains(&"Depth"));
        assert!(names.contains(&"GrossVolume"));
    }

    #[test]
    fn floor_base_quantities_empty_for_degenerate_boundary() {
        let qto = floor_base_quantities(&[], None);
        assert_eq!(qto.name, "Qto_SlabBaseQuantities");
        assert!(qto.properties.is_empty());
    }

    // ---- GEO-29: Roof geometry with slopes ----

    #[test]
    fn roof_pitch_6_in_12_is_about_26_degrees() {
        let rad = roof_pitch_radians_from_rise_run(6.0, 12.0);
        let deg = rad.to_degrees();
        assert!(
            (deg - 26.565).abs() < 0.01,
            "6:12 pitch should be ~26.57°, got {deg}"
        );
    }

    #[test]
    fn roof_pitch_12_in_12_is_45_degrees() {
        let rad = roof_pitch_radians_from_rise_run(12.0, 12.0);
        assert!((rad.to_degrees() - 45.0).abs() < 1e-9);
    }

    #[test]
    fn roof_pitch_zero_run_returns_zero() {
        assert_eq!(roof_pitch_radians_from_rise_run(5.0, 0.0), 0.0);
        assert_eq!(roof_pitch_radians_from_rise_run(5.0, -1.0), 0.0);
    }

    #[test]
    fn roof_pitch_from_degrees_matches() {
        let deg = 30.0;
        let rad = roof_pitch_radians_from_degrees(deg);
        assert!((rad - std::f64::consts::FRAC_PI_6).abs() < 1e-9);
    }

    #[test]
    fn gabled_ridge_height_at_45_degrees() {
        // span 20 ft at 45° → ridge 10 ft above eave.
        let h = gabled_roof_ridge_height(20.0, std::f64::consts::FRAC_PI_4);
        assert!((h - 10.0).abs() < 1e-9);
    }

    #[test]
    fn gabled_ridge_height_flat_is_zero() {
        assert_eq!(gabled_roof_ridge_height(20.0, 0.0), 0.0);
    }

    #[test]
    fn gabled_ridge_negative_inputs_clamp_to_zero() {
        assert_eq!(gabled_roof_ridge_height(-5.0, 1.0), 0.0);
        assert_eq!(gabled_roof_ridge_height(5.0, -1.0), 0.0);
    }

    #[test]
    fn hip_roof_ridge_length_rectangle_is_diff() {
        assert!((hip_roof_ridge_length(40.0, 20.0) - 20.0).abs() < 1e-9);
        assert!((hip_roof_ridge_length(20.0, 40.0) - 20.0).abs() < 1e-9); // order-invariant
    }

    #[test]
    fn hip_roof_pyramid_has_zero_ridge() {
        assert_eq!(hip_roof_ridge_length(20.0, 20.0), 0.0);
    }

    #[test]
    fn gabled_extrusion_emits_triangular_profile() {
        let ex = gabled_roof_extrusion(30.0, 20.0, std::f64::consts::FRAC_PI_4, None);
        match ex.profile_override {
            Some(crate::ifc::entities::ProfileDef::ArbitraryClosed { points }) => {
                assert_eq!(points.len(), 3);
                // The two eave points must share Y=0.
                assert!((points[0].1 - 0.0).abs() < 1e-9);
                assert!((points[1].1 - 0.0).abs() < 1e-9);
                // The ridge (3rd point) sits above the midline.
                assert!(points[2].1 > 0.0);
            }
            other => panic!("expected triangular profile, got {:?}", other),
        }
        assert!((ex.height_feet - 30.0).abs() < 1e-9);
    }

    #[test]
    fn gabled_extrusion_flat_falls_back_to_slab() {
        let ex = gabled_roof_extrusion(30.0, 20.0, 0.0, None);
        assert!(ex.profile_override.is_none()); // rectangular fallback
        assert!((ex.width_feet - 30.0).abs() < 1e-9);
        assert!((ex.depth_feet - 20.0).abs() < 1e-9);
    }

    #[test]
    fn hip_brep_has_six_vertices_six_triangles() {
        let (v, t) = hip_roof_brep(40.0, 20.0, std::f64::consts::FRAC_PI_4);
        assert_eq!(v.len(), 6);
        assert_eq!(t.len(), 6); // 2 long-slope tris * 2 + 2 hip-end tris
        // All triangle indices reference valid vertices.
        for crate::ifc::entities::BrepTriangle(a, b, c) in &t {
            assert!((*a as usize) < v.len());
            assert!((*b as usize) < v.len());
            assert!((*c as usize) < v.len());
        }
    }

    #[test]
    fn hip_brep_pyramid_still_produces_faces() {
        // length == width → ridge collapses to a point but we still
        // emit 6 triangles (two degenerate along the ridge).
        let (v, t) = hip_roof_brep(20.0, 20.0, std::f64::consts::FRAC_PI_6);
        assert_eq!(v.len(), 6);
        assert_eq!(t.len(), 6);
        // The two ridge-endpoints coincide at the apex.
        assert!((v[4][0] - v[5][0]).abs() < 1e-9);
        assert!((v[4][1] - v[5][1]).abs() < 1e-9);
        assert!((v[4][2] - v[5][2]).abs() < 1e-9);
    }

    #[test]
    fn hip_brep_swapped_axes_when_width_larger() {
        // length < width should route the ridge along the Y axis.
        let (v, _) = hip_roof_brep(20.0, 40.0, std::f64::consts::FRAC_PI_4);
        // Ridge endpoint X ≈ short/2 = 10; Y inside [0, 40].
        assert!((v[4][0] - 10.0).abs() < 1e-9);
        assert!(v[4][1] > 0.0 && v[4][1] < 40.0);
    }
}
