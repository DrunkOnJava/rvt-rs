//! IFC entity types targeted by the exporter.
//!
//! Intentionally minimal — only the entity categories the mapping table in
//! `mod.rs` references. Expand as Layer 4c progresses and more Revit
//! classes become decodable.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum IfcEntity {
    Project {
        name: Option<String>,
        description: Option<String>,
        long_name: Option<String>,
    },
    BuildingElementType {
        ifc_type: String,
        name: String,
        description: Option<String>,
    },
    BuildingElement {
        ifc_type: String,
        name: String,
        type_guid: Option<String>,
        /// Index into `IfcModel.building_storeys` — which storey
        /// contains this element. `None` means the writer should
        /// default it to the first storey (common when the element's
        /// `level_id` wasn't resolved yet).
        #[serde(default)]
        storey_index: Option<usize>,
        /// Index into `IfcModel.materials` — which material this
        /// element associates with. `None` means the element has no
        /// material, so no IfcRelAssociatesMaterial gets emitted for
        /// it (IFC4 treats material as optional for every concrete
        /// type).
        #[serde(default)]
        material_index: Option<usize>,
        /// Property-set name + list of (name, value) pairs to emit
        /// as IfcPropertySet → IfcPropertySingleValue → element via
        /// IfcRelDefinesByProperties. Empty = no property set. The
        /// property-set name should follow Revit / IFC convention:
        /// usually "Pset_RevitType_{ClassName}" to match what the
        /// Autodesk exporter produces.
        #[serde(default)]
        property_set: Option<PropertySet>,
        /// Element origin in feet, expressed in the project's
        /// coordinate system. When `Some`, the writer emits a
        /// unique IFCCARTESIANPOINT + IFCAXIS2PLACEMENT3D for this
        /// element (ft → m conversion at emit time). When `None`,
        /// the element uses the shared identity placement — fine
        /// until geometry lands and positions start mattering.
        #[serde(default)]
        location_feet: Option<[f64; 3]>,
        /// Element rotation about the Z (up) axis, in radians. Only
        /// consulted when `location_feet` is `Some` — the placement
        /// with a non-default X-axis direction needs a unique
        /// IFCDIRECTION.
        #[serde(default)]
        rotation_radians: Option<f64>,
        /// Optional rectangular extrusion geometry. When `Some`, the
        /// writer emits the IfcExtrudedAreaSolid chain and wires the
        /// element's Representation slot to it. When `None`, the
        /// element stays geometry-free (Representation = $).
        #[serde(default)]
        extrusion: Option<Extrusion>,
        /// Index into `IfcModel.entities` naming a host BuildingElement
        /// (typically the wall that contains this door/window). When
        /// set, the writer emits an IfcOpeningElement (same shape as
        /// this element's extrusion) + IfcRelVoidsElement (host →
        /// opening) + IfcRelFillsElement (opening → this element).
        /// The host must already be in `entities` before this element
        /// and must itself be a BuildingElement with an extrusion
        /// (otherwise the void subtracts from nothing).
        #[serde(default)]
        host_element_index: Option<usize>,
        /// Index into `IfcModel.material_layer_sets` — the layered
        /// assembly (gypsum / insulation / sheathing / …) that
        /// makes up this element (IFC-28). Used for walls, slabs,
        /// roofs, and ceilings that have a real multi-layer
        /// composition. When set, the writer emits
        /// `IfcMaterialLayerSet` + `IfcMaterialLayerSetUsage` + an
        /// `IfcRelAssociatesMaterial` pointing at the layer-set
        /// usage — *instead of* the single-material path driven by
        /// `material_index`. When both are set, the layer set
        /// wins (single-material falls back to the first layer).
        #[serde(default)]
        material_layer_set_index: Option<usize>,
        /// Index into `IfcModel.material_profile_sets` — the
        /// profile-based material assignment for structural framing
        /// (columns / beams) with named cross-sections (IFC-30).
        /// When set, the writer emits `IfcMaterialProfileSet` +
        /// `IfcMaterialProfileSetUsage` instead of the single-material
        /// or layer-set paths. Precedence order for material
        /// association: profile_set > layer_set > single material.
        #[serde(default)]
        material_profile_set_index: Option<usize>,
        /// Optional richer solid geometry (IFC-18 / IFC-19 / IFC-20).
        /// When `Some`, the writer emits the corresponding
        /// `IfcRevolvedAreaSolid`, `IfcBooleanResult`, or
        /// `IfcFacetedBrep` chain into the element's Representation
        /// slot **instead of** the `IfcExtrudedAreaSolid` chain
        /// driven by `extrusion`. When both `solid_shape` and
        /// `extrusion` are set, `solid_shape` wins; when both are
        /// `None`, the element stays geometry-free
        /// (Representation = $).
        #[serde(default)]
        solid_shape: Option<SolidShape>,
        /// Index into `IfcModel.representation_maps` — when set,
        /// the element shares a common shape representation with
        /// other instances pointing at the same map (IFC-21). The
        /// writer emits an `IfcMappedItem` + `IfcShapeRepresentation`
        /// with representation-type `'MappedRepresentation'` in
        /// place of the per-instance extrusion / solid chain.
        ///
        /// Precedence (highest wins):
        /// 1. `representation_map_index`   (IFC-21 mapped item)
        /// 2. `solid_shape`                (IFC-18/19/20 solid)
        /// 3. `extrusion`                  (IFC-16 extruded area)
        /// 4. none                         (Representation = $)
        ///
        /// Typical Revit → IFC use: every Door or Window sharing
        /// the same Symbol (type family) references a single
        /// representation map. The writer emits the shape chain
        /// once, instances emit a ~4-entity IfcMappedItem wrap.
        #[serde(default)]
        representation_map_index: Option<usize>,
    },
    TypeObject {
        name: String,
        shape_representations: Vec<String>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Classification {
    pub source: ClassificationSource,
    pub edition: Option<String>,
    pub items: Vec<ClassificationItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ClassificationSource {
    OmniClass,
    Uniformat,
    Other(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassificationItem {
    pub code: String,
    pub name: Option<String>,
}

/// Swept-area-solid geometry descriptor for a BuildingElement. The
/// writer turns this into an `IfcProfileDef` subclass +
/// `IfcExtrudedAreaSolid` + `IfcShapeRepresentation` +
/// `IfcProductDefinitionShape` chain and points the element's
/// Representation slot at the chain.
///
/// All values in feet; the writer converts to metres at emit
/// boundary (ft × 0.3048). The profile is centred on the element
/// origin and the extrusion runs +Z.
///
/// Backward-compat: the primary shape is still a rectangle defined
/// by [`width_feet`] × [`depth_feet`]. Callers that need a richer
/// cross-section (circle, I-beam, T, L, U, hollow rectangle, hollow
/// circle, arbitrary closed polyline) set [`profile_override`] to a
/// [`ProfileDef`] — when `Some`, the writer emits the matching
/// `IfcProfileDef` subclass (IFC-24) and ignores `width_feet` /
/// `depth_feet`. `height_feet` is always honoured as the extrusion
/// depth.
///
/// [`width_feet`]: Extrusion::width_feet
/// [`depth_feet`]: Extrusion::depth_feet
/// [`profile_override`]: Extrusion::profile_override
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Extrusion {
    /// Profile width (local X, in feet) when `profile_override` is
    /// `None`. For a wall = length along its location line. For a
    /// slab = plan dimension in X. Ignored when `profile_override`
    /// is `Some`.
    pub width_feet: f64,
    /// Profile depth (local Y, in feet) when `profile_override` is
    /// `None`. For a wall = thickness. For a slab = plan dimension
    /// in Y. Ignored when `profile_override` is `Some`.
    pub depth_feet: f64,
    /// Extrusion depth (local Z, in feet). For a wall = height;
    /// for a slab = slab thickness; for a column = height; for a
    /// beam = length along its structural axis.
    pub height_feet: f64,
    /// Optional non-rectangular profile (IFC-24). When `Some`, the
    /// writer emits the corresponding `IfcProfileDef` subclass
    /// (`IFCCIRCLEPROFILEDEF`, `IFCIShapeProfileDef`,
    /// `IFCTShapeProfileDef`, `IFCLShapeProfileDef`,
    /// `IFCUShapeProfileDef`, `IFCRectangleHollowProfileDef`,
    /// `IFCCircleHollowProfileDef`, or `IFCArbitraryClosedProfileDef`)
    /// instead of the default `IFCRectangleProfileDef`. Use
    /// `Extrusion::circle`, `Extrusion::i_shape`, etc. constructors
    /// for ergonomics.
    #[serde(default)]
    pub profile_override: Option<ProfileDef>,
}

impl Extrusion {
    /// Rectangular profile — the default / backward-compatible
    /// shape. Equivalent to leaving `profile_override` at `None`.
    pub fn rectangle(width_feet: f64, depth_feet: f64, height_feet: f64) -> Self {
        Self {
            width_feet,
            depth_feet,
            height_feet,
            profile_override: None,
        }
    }

    /// Solid circular profile (e.g. round column, round pier).
    /// Emits `IFCCIRCLEPROFILEDEF`.
    pub fn circle(radius_feet: f64, height_feet: f64) -> Self {
        let diameter = radius_feet * 2.0;
        Self {
            width_feet: diameter,
            depth_feet: diameter,
            height_feet,
            profile_override: Some(ProfileDef::Circle { radius_feet }),
        }
    }

    /// I-shape profile (wide-flange, W/S/HP shapes in AISC). Emits
    /// `IFCIShapeProfileDef`. `overall_width_feet` is the flange
    /// width; `overall_depth_feet` is the beam depth between outer
    /// flange faces.
    pub fn i_shape(
        overall_width_feet: f64,
        overall_depth_feet: f64,
        web_thickness_feet: f64,
        flange_thickness_feet: f64,
        height_feet: f64,
    ) -> Self {
        Self {
            width_feet: overall_width_feet,
            depth_feet: overall_depth_feet,
            height_feet,
            profile_override: Some(ProfileDef::IShape {
                overall_width_feet,
                overall_depth_feet,
                web_thickness_feet,
                flange_thickness_feet,
            }),
        }
    }

    /// T-shape profile (structural tee, WT/ST/MT cut from I-beams).
    /// Emits `IFCTShapeProfileDef`.
    pub fn t_shape(
        overall_depth_feet: f64,
        flange_width_feet: f64,
        web_thickness_feet: f64,
        flange_thickness_feet: f64,
        height_feet: f64,
    ) -> Self {
        Self {
            width_feet: flange_width_feet,
            depth_feet: overall_depth_feet,
            height_feet,
            profile_override: Some(ProfileDef::TShape {
                overall_depth_feet,
                flange_width_feet,
                web_thickness_feet,
                flange_thickness_feet,
            }),
        }
    }

    /// L-shape (angle) profile — equal-leg when `width == depth`.
    /// Emits `IFCLShapeProfileDef`.
    pub fn l_shape(
        overall_depth_feet: f64,
        overall_width_feet: f64,
        thickness_feet: f64,
        height_feet: f64,
    ) -> Self {
        Self {
            width_feet: overall_width_feet,
            depth_feet: overall_depth_feet,
            height_feet,
            profile_override: Some(ProfileDef::LShape {
                overall_depth_feet,
                overall_width_feet,
                thickness_feet,
            }),
        }
    }

    /// U-shape (channel) profile. Emits `IFCUShapeProfileDef`.
    pub fn u_shape(
        overall_depth_feet: f64,
        flange_width_feet: f64,
        web_thickness_feet: f64,
        flange_thickness_feet: f64,
        height_feet: f64,
    ) -> Self {
        Self {
            width_feet: flange_width_feet,
            depth_feet: overall_depth_feet,
            height_feet,
            profile_override: Some(ProfileDef::UShape {
                overall_depth_feet,
                flange_width_feet,
                web_thickness_feet,
                flange_thickness_feet,
            }),
        }
    }

    /// Rectangular hollow section (HSS tube). Emits
    /// `IFCRectangleHollowProfileDef`.
    pub fn rectangle_hollow(
        overall_width_feet: f64,
        overall_depth_feet: f64,
        wall_thickness_feet: f64,
        height_feet: f64,
    ) -> Self {
        Self {
            width_feet: overall_width_feet,
            depth_feet: overall_depth_feet,
            height_feet,
            profile_override: Some(ProfileDef::RectangleHollow {
                overall_width_feet,
                overall_depth_feet,
                wall_thickness_feet,
            }),
        }
    }

    /// Circular hollow section (round HSS pipe). Emits
    /// `IFCCIRCLEHOLLOWPROFILEDEF`.
    pub fn circle_hollow(radius_feet: f64, wall_thickness_feet: f64, height_feet: f64) -> Self {
        let diameter = radius_feet * 2.0;
        Self {
            width_feet: diameter,
            depth_feet: diameter,
            height_feet,
            profile_override: Some(ProfileDef::CircleHollow {
                radius_feet,
                wall_thickness_feet,
            }),
        }
    }

    /// Arbitrary closed polyline profile (e.g. curtain-mullion
    /// cross-section, custom sketched shape). Emits
    /// `IFCArbitraryClosedProfileDef` + an `IFCPOLYLINE` as the
    /// outer curve. The writer auto-closes the polyline if the
    /// last point doesn't equal the first.
    pub fn arbitrary_closed(points: Vec<(f64, f64)>, height_feet: f64) -> Self {
        let (min_x, max_x, min_y, max_y) = points.iter().fold(
            (
                f64::INFINITY,
                f64::NEG_INFINITY,
                f64::INFINITY,
                f64::NEG_INFINITY,
            ),
            |(mn_x, mx_x, mn_y, mx_y), (x, y)| {
                (mn_x.min(*x), mx_x.max(*x), mn_y.min(*y), mx_y.max(*y))
            },
        );
        let width = (max_x - min_x).max(0.0);
        let depth = (max_y - min_y).max(0.0);
        Self {
            width_feet: width,
            depth_feet: depth,
            height_feet,
            profile_override: Some(ProfileDef::ArbitraryClosed { points }),
        }
    }
}

/// Boolean operation between two solids (IFC-19).
/// Maps to the IFC4 `IfcBooleanOperator` enum values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum IfcBooleanOp {
    /// `.UNION.` — operand_a ∪ operand_b.
    Union,
    /// `.DIFFERENCE.` — operand_a − operand_b (the more common
    /// Revit case: wall minus opening void).
    Difference,
    /// `.INTERSECTION.` — operand_a ∩ operand_b.
    Intersection,
}

impl IfcBooleanOp {
    /// STEP-encoded keyword for the enum value (includes the
    /// surrounding dots, e.g. `".DIFFERENCE."`).
    pub fn as_step_keyword(self) -> &'static str {
        match self {
            IfcBooleanOp::Union => ".UNION.",
            IfcBooleanOp::Difference => ".DIFFERENCE.",
            IfcBooleanOp::Intersection => ".INTERSECTION.",
        }
    }
}

/// One triangular face of an `IfcFacetedBrep`. The three
/// `u32` values are indices into the `vertices` array on the
/// enclosing shell (range-checked at emit time; out-of-range
/// indices cause a panic in debug, are silently clamped to 0 in
/// release — invalid mesh topology is a caller bug).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct BrepTriangle(pub u32, pub u32, pub u32);

/// A rich solid geometry for a `BuildingElement` (IFC-18 /
/// IFC-19 / IFC-20). Covers three IFC4 solid-body paths that the
/// rectangular [`Extrusion`] can't express:
///
/// | Variant | IFC4 entity chain emitted |
/// |---|---|
/// | `RevolvedArea` | `IfcProfileDef subclass` + `IfcRevolvedAreaSolid` (IFC-18) |
/// | `BooleanResult` | `IfcBooleanResult(op, a, b)` with recursive operand emission (IFC-19) |
/// | `FacetedBrep` | `IfcCartesianPoint` × N + `IfcPolyLoop` × F + `IfcFaceBound` × F + `IfcFace` × F + `IfcClosedShell` + `IfcFacetedBrep` (IFC-20) |
///
/// The `RevolvedArea` variant is the right fit for axi-symmetric
/// elements — lathe-turned columns, bell curves, domes. The
/// `BooleanResult` variant is for elements whose body is a
/// *constructive-solid-geometry* result of simpler shapes
/// (typical Revit: wall minus opening void, when modelled as a
/// body solid rather than via `IfcRelVoidsElement`). The
/// `FacetedBrep` variant is the catch-all fallback for any
/// arbitrary mesh Revit produces (free-form roofs, imported
/// terrain, scanned point clouds meshed into polygons).
///
/// See the writer in `step_writer.rs::emit_solid_shape` for the
/// exact entity layout and the `emit_solid_shape_*` helper
/// methods.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SolidShape {
    /// Profile rotated about an axis through a specified angle.
    /// Emits `IfcRevolvedAreaSolid` (IFC-18). The profile is the
    /// same `ProfileDef` vocabulary used by
    /// [`Extrusion::profile_override`], so a pear-shaped turned
    /// pier column is `RevolvedArea { profile: ArbitraryClosed
    /// { points: <half-silhouette> }, axis_..., angle_radians:
    /// 2π }`.
    RevolvedArea {
        /// 2D cross-section to rotate.
        profile: ProfileDef,
        /// Axis origin in element-local coordinates (feet).
        axis_origin_feet: [f64; 3],
        /// Axis direction unit vector. Writer normalises at emit.
        axis_direction: [f64; 3],
        /// Sweep angle in radians. 2π = full rotation (dome /
        /// turned column / sphere-of-revolution).
        angle_radians: f64,
    },
    /// CSG combination of two nested solids (IFC-19). Emits
    /// `IfcBooleanResult(op, a, b)` with operand_a / operand_b
    /// recursively emitted by the same dispatcher. Use `Difference`
    /// for Revit "subtract void from body" patterns when the void
    /// can't be expressed as an `IfcOpeningElement` link (e.g. a
    /// coffered ceiling, a pier with a carved niche).
    BooleanResult {
        op: IfcBooleanOp,
        operand_a: Box<SolidShape>,
        operand_b: Box<SolidShape>,
    },
    /// Closed polyhedral surface as a faceted brep (IFC-20).
    /// Emits `IFCFACETEDBREP` with one `IfcClosedShell` wrapping
    /// `IfcFace` / `IfcFaceBound` / `IfcPolyLoop` entities and
    /// one `IfcCartesianPoint` per unique vertex. Used for any
    /// Revit element whose body is a mesh rather than a swept
    /// profile — imported terrain, free-form roofs, conceptual
    /// massing brep output, DirectShape IfcOpenShell triangulation.
    FacetedBrep {
        /// Vertex coordinates in element-local space (feet). The
        /// writer converts to metres at emit time.
        vertices_feet: Vec<[f64; 3]>,
        /// Triangular faces. Each triple indexes into
        /// `vertices_feet`. Non-triangular faces should be
        /// pre-tessellated by the caller — the writer emits a
        /// 3-vertex `IfcPolyLoop` per triangle.
        triangles: Vec<BrepTriangle>,
    },
    /// Simple extruded-area solid via the [`Extrusion`] struct.
    /// Emitted when a caller wants to keep `extrusion` set to
    /// `None` but route through `SolidShape` for uniform geometry
    /// handling. Most callers should leave this variant unused
    /// and just set `extrusion` directly — it exists for symmetry
    /// with the other CSG operands.
    ExtrudedArea(Extrusion),
    /// Profile swept along an arbitrary polyline directrix with a
    /// fixed up-reference direction (IFC-17). Emits IFC4
    /// `IfcFixedReferenceSweptAreaSolid`. Use for cable trays,
    /// pipes, duct runs, curtain-wall mullions along a curve,
    /// sloped handrails — anything where the profile stays
    /// orthogonal to the directrix at every sample.
    ///
    /// The directrix is a polyline through the supplied
    /// `directrix_points` (in feet, element-local coordinates).
    /// `fixed_reference` is a unit vector that together with the
    /// directrix tangent defines the profile orientation — it's
    /// the "up" direction the swept profile preserves as the
    /// directrix changes heading.
    ///
    /// IFC4 reserves start / end parameters for trimming the
    /// directrix; we always sweep the full polyline (0 → 1
    /// normalised) which is the correct default for Revit data.
    SweptPath {
        /// 2D cross-section (see [`ProfileDef`]).
        profile: ProfileDef,
        /// Polyline vertices of the directrix path, in feet,
        /// element-local coordinates.
        directrix_points_feet: Vec<[f64; 3]>,
        /// Fixed-reference direction (usually world +Z). Writer
        /// normalises to a unit vector at emit.
        fixed_reference: [f64; 3],
    },
}

/// Named cross-sections for an extrusion (IFC-24). Feeds one of
/// eight IFC4 `IfcProfileDef` subclasses. All values in feet
/// (writer converts to metres at emit time). Profiles are centred
/// on the element origin in the local XY frame; positive X is the
/// profile width direction, positive Y is the profile depth
/// direction.
///
/// Profile selection cheat-sheet:
///
/// | Revit class | Typical profile |
/// |---|---|
/// | Structural column (round) | `Circle` |
/// | Structural column (wide-flange) | `IShape` |
/// | Structural column (HSS square) | `RectangleHollow` |
/// | Structural column (round HSS) | `CircleHollow` |
/// | Beam (wide-flange W-shape) | `IShape` |
/// | Beam (channel) | `UShape` |
/// | Beam (angle) | `LShape` |
/// | Beam (tee) | `TShape` |
/// | Curtain mullion | `ArbitraryClosed` |
/// | Wall / slab / roof / ceiling | `Rectangle` (default, no override) |
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProfileDef {
    /// `IFCRectangleProfileDef` — the default if `profile_override`
    /// is `None`. Kept as an explicit variant so callers can round-
    /// trip through `profile_override` without losing shape info.
    Rectangle { width_feet: f64, depth_feet: f64 },
    /// `IFCCIRCLEPROFILEDEF` — solid circular cross-section.
    Circle { radius_feet: f64 },
    /// `IFCIShapeProfileDef` — wide-flange steel shape.
    ///
    /// - `overall_width_feet` is the flange width (local X span).
    /// - `overall_depth_feet` is the distance between outer flange
    ///   faces (local Y span).
    /// - `web_thickness_feet` is the web's thickness.
    /// - `flange_thickness_feet` is the (constant) flange thickness.
    ///
    /// All four values map directly onto the IFC4 attribute names
    /// `OverallWidth`, `OverallDepth`, `WebThickness`,
    /// `FlangeThickness`.
    IShape {
        overall_width_feet: f64,
        overall_depth_feet: f64,
        web_thickness_feet: f64,
        flange_thickness_feet: f64,
    },
    /// `IFCTShapeProfileDef` — structural tee.
    TShape {
        overall_depth_feet: f64,
        flange_width_feet: f64,
        web_thickness_feet: f64,
        flange_thickness_feet: f64,
    },
    /// `IFCLShapeProfileDef` — structural angle. `overall_depth` is
    /// the longer leg; `overall_width` the shorter leg (they can be
    /// equal for equal-leg angles).
    LShape {
        overall_depth_feet: f64,
        overall_width_feet: f64,
        thickness_feet: f64,
    },
    /// `IFCUShapeProfileDef` — structural channel (C-shape).
    UShape {
        overall_depth_feet: f64,
        flange_width_feet: f64,
        web_thickness_feet: f64,
        flange_thickness_feet: f64,
    },
    /// `IFCRectangleHollowProfileDef` — rectangular HSS tube.
    /// `wall_thickness` is the uniform wall thickness.
    RectangleHollow {
        overall_width_feet: f64,
        overall_depth_feet: f64,
        wall_thickness_feet: f64,
    },
    /// `IFCCircleHollowProfileDef` — round HSS pipe.
    CircleHollow {
        radius_feet: f64,
        wall_thickness_feet: f64,
    },
    /// `IFCArbitraryClosedProfileDef` with an `IFCPOLYLINE` outer
    /// curve. Points are in local 2D coordinates (feet); writer
    /// auto-closes the polyline if the last point isn't equal to
    /// the first. No self-intersection check is performed — callers
    /// supplying degenerate polygons will emit degenerate IFC.
    ArbitraryClosed { points: Vec<(f64, f64)> },
}

/// One layer of a compound building-element material assembly
/// (IFC-28). `thickness_feet` is the physical thickness (writer
/// converts to metres at emit time). `material_index` points into
/// `IfcModel.materials` — the material filling this layer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaterialLayer {
    pub material_index: usize,
    pub thickness_feet: f64,
    /// Optional per-layer name. When `Some`, emitted as the
    /// `IfcMaterialLayer.Name` attribute. Revit's convention is
    /// layer names like "Finish - Face Layer", "Structure", "Air
    /// Gap", "Insulation" — useful for downstream BIM tools that
    /// surface them in schedules.
    #[serde(default)]
    pub name: Option<String>,
}

/// An ordered set of [`MaterialLayer`]s representing the compound
/// composition of a wall / floor / roof / ceiling (IFC-28). Maps
/// to IFC4 `IfcMaterialLayerSet` + (via
/// `IfcEntity::BuildingElement::material_layer_set_index`)
/// `IfcMaterialLayerSetUsage`.
///
/// `name` is the set-level label ("Generic - 6\" Wall", "Ext - CMU").
/// Revit's exterior wall types often carry 3-5 layers; interior
/// partitions are usually 2-3. The ordering matters: IFC4
/// interprets index `0` as the outermost layer (exterior or top side)
/// with subsequent layers stacked inward.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaterialLayerSet {
    pub name: String,
    pub layers: Vec<MaterialLayer>,
    /// Optional description; emitted as `IfcMaterialLayerSet.Description`.
    #[serde(default)]
    pub description: Option<String>,
}

impl MaterialLayerSet {
    /// Total thickness in feet (sum of layer thicknesses). Useful
    /// for sanity-checking that the declared wall thickness matches
    /// the layer-set composition.
    pub fn total_thickness_feet(&self) -> f64 {
        self.layers.iter().map(|l| l.thickness_feet).sum()
    }
}

/// One material assigned to a structural profile in a
/// [`MaterialProfileSet`] (IFC-30). `material_index` references
/// `IfcModel.materials`; `profile_name` identifies the profile
/// (I-beam, HSS tube, circular column) defined elsewhere in the
/// model via `IfcProfileDef`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaterialProfile {
    pub material_index: usize,
    /// Profile identifier — emitted as `IfcMaterialProfile.Name`
    /// and matched against the corresponding `IfcProfileDef`.
    /// Revit convention: profile family name like
    /// "W12x26" / "HSS4x4x1/4" / "ROUND-6in".
    pub profile_name: String,
    /// Optional per-profile descriptive text.
    #[serde(default)]
    pub description: Option<String>,
}

/// An ordered set of [`MaterialProfile`]s for a structural
/// framing element (IFC-30). Analog of [`MaterialLayerSet`] but
/// for columns and beams rather than compound walls. Most
/// structural elements have exactly one profile; composite
/// sections (steel + concrete encasement) use multiple.
///
/// Maps to IFC4 `IfcMaterialProfileSet` + (via
/// `IfcEntity::BuildingElement::material_profile_set_index`)
/// `IfcMaterialProfileSetUsage`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaterialProfileSet {
    pub name: String,
    pub profiles: Vec<MaterialProfile>,
    #[serde(default)]
    pub description: Option<String>,
}

/// Shared geometry for a family / type / symbol (IFC-21).
/// Any number of `BuildingElement` instances can reference the
/// same `RepresentationMap` via `representation_map_index` —
/// the writer emits the underlying shape chain once and each
/// instance gets a ~4-entity `IfcMappedItem` wrap instead of a
/// full re-emission. Mirrors Revit's Symbol → FamilyInstance
/// relationship (a single door `Symbol` used by 20 `Door`
/// instances = 1 shape emission + 20 mapped items, not 20
/// shape chains).
///
/// Maps to IFC4 `IfcRepresentationMap` + (through the writer's
/// dispatch) `IfcMappedItem` per referencing instance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepresentationMap {
    /// Human-readable name for the family / type. Emitted as
    /// a comment-only field today (the IFC4
    /// `IfcRepresentationMap` entity itself doesn't carry a
    /// name slot — names live on the referencing type object
    /// when one exists). Kept here so downstream tooling (e.g.
    /// glTF export, schedule generation) can surface it.
    #[serde(default)]
    pub name: Option<String>,
    /// The shared geometry. Any `SolidShape` variant is valid:
    /// an extruded rectangle for simple doors, a revolved
    /// profile for lathe-turned piers, an arbitrary faceted-brep
    /// for imported free-form content. See [`SolidShape`] docs
    /// for the variant vocabulary.
    pub shape: SolidShape,
    /// Mapping origin in local coordinates (feet). Usually
    /// `[0.0, 0.0, 0.0]` — the identity origin. Non-zero origins
    /// are emitted as the mapping-origin's `IfcAxis2Placement3D`.
    #[serde(default)]
    pub origin_feet: [f64; 3],
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnitAssignment {
    /// e.g. "autodesk.unit.unit:millimeters-1.0.1"
    pub forge_identifier: String,
    /// IFC base unit name, e.g. "MILLI" + "METRE"
    pub ifc_mapping: Option<String>,
}

/// A parsed Forge (Autodesk) unit identifier (IFC-39).
///
/// Revit carries its project / family unit preferences as
/// "Forge-style" identifier strings in the `PartAtom` XML and in
/// serialized `UnitType` fields, e.g.
/// `autodesk.unit.unit:millimeters-1.0.1`,
/// `autodesk.unit.unit:squareFeet-1.0.1`,
/// `autodesk.unit.unit:degrees-1.0.1`.
///
/// This enum captures every common one that maps cleanly to an
/// IFC4 `IfcSIUnit` (metric SI) or `IfcConversionBasedUnit` (non-
/// SI Imperial / mixed). Identifiers that don't match a known
/// unit fall through to [`ForgeUnit::Other`] carrying the raw
/// identifier so downstream code can still introspect it.
///
/// Invariant: every non-`Other` variant has a defined
/// [`ForgeUnit::ifc_emission`] mapping — callers can always emit
/// a valid IFC unit for a matched identifier.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ForgeUnit {
    // ---- Length ----
    Millimeters,
    Centimeters,
    Decimeters,
    Meters,
    Kilometers,
    Inches,
    Feet,
    Yards,
    Miles,

    // ---- Area ----
    SquareMillimeters,
    SquareCentimeters,
    SquareMeters,
    SquareFeet,
    SquareInches,
    SquareYards,
    Acres,
    Hectares,

    // ---- Volume ----
    CubicMillimeters,
    CubicCentimeters,
    CubicMeters,
    CubicFeet,
    CubicInches,
    CubicYards,
    Liters,
    UsGallons,

    // ---- Angle ----
    Radians,
    Degrees,
    Grads,

    // ---- Mass ----
    Kilograms,
    Grams,
    Pounds,

    // ---- Time ----
    Seconds,
    Minutes,
    Hours,

    /// Unrecognised Forge identifier — carries the raw string
    /// so callers can still round-trip it through serialization
    /// without losing data.
    Other(String),
}

/// IFC4 dimensional category (`LENGTHUNIT`, `AREAUNIT`, …).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IfcUnitType {
    Length,
    Area,
    Volume,
    PlaneAngle,
    Mass,
    Time,
}

impl IfcUnitType {
    /// STEP keyword (no surrounding dots — caller wraps).
    pub fn as_step_token(self) -> &'static str {
        match self {
            IfcUnitType::Length => "LENGTHUNIT",
            IfcUnitType::Area => "AREAUNIT",
            IfcUnitType::Volume => "VOLUMEUNIT",
            IfcUnitType::PlaneAngle => "PLANEANGLEUNIT",
            IfcUnitType::Mass => "MASSUNIT",
            IfcUnitType::Time => "TIMEUNIT",
        }
    }
}

/// How an IFC unit should be emitted (IFC-40).
///
/// - `SI { prefix, name }`: emits a single `IFCSIUNIT(*, <type>,
///   <prefix>, <name>)`. Prefix is `None` for ambient (e.g. metres,
///   square metres) or `Some("MILLI")` / `"CENTI")` / `"DECI")` /
///   `"KILO")` for multiples.
/// - `ConversionBased { base_name, conversion_factor_to_si_base }`:
///   emits a pair `IFCMEASUREWITHUNIT(<measure>(<factor>), <si_base>)`
///   + `IFCCONVERSIONBASEDUNIT(<dim>, <type>, <name>, <measure_ref>)`.
///   Used for non-SI units (feet → metres at 0.3048, inches →
///   metres at 0.0254, degrees → radians at π/180, pounds →
///   kilograms at 0.453592, gallons → cubic metres at 0.00378541,
///   …).
#[derive(Debug, Clone, PartialEq)]
pub enum IfcUnitEmission {
    /// Pure SI unit.
    Si {
        unit_type: IfcUnitType,
        /// `None` for base unit, or `"MILLI"` / `"CENTI"` / `"DECI"`
        /// / `"KILO"` / `"HECTO"` for the standard IFC SI prefix
        /// vocabulary.
        prefix: Option<&'static str>,
        /// `"METRE"` / `"SQUARE_METRE"` / `"CUBIC_METRE"` /
        /// `"RADIAN"` / `"GRAM"` / `"SECOND"` — the unprefixed
        /// SI base name.
        name: &'static str,
    },
    /// Conversion-based unit derived from an SI base.
    ConversionBased {
        unit_type: IfcUnitType,
        /// Human / IFC name for the derived unit (`"FOOT"`,
        /// `"INCH"`, `"DEGREE"`, `"POUND"`, `"GALLON"`).
        derived_name: &'static str,
        /// Multiplier: `<derived_unit> = <this> × <si_base_unit>`.
        /// Example: foot = 0.3048 × metre; inch = 0.0254 × metre;
        /// degree = π/180 × radian.
        factor_to_si: f64,
        /// The SI base emission to attach the factor to (always
        /// `IfcUnitEmission::Si` conceptually, but inlined here
        /// as the bare IFC unit name to keep the data flat).
        si_base_name: &'static str,
    },
}

impl ForgeUnit {
    /// Parse an Autodesk Forge unit identifier. Returns
    /// `ForgeUnit::Other(raw)` for any string that doesn't match
    /// a known pattern.
    ///
    /// Accepted shapes:
    /// - `autodesk.unit.unit:millimeters-1.0.1` (canonical)
    /// - `autodesk.unit.unit:millimeters` (without version)
    /// - `millimeters` (bare — lenient)
    ///
    /// Case-insensitive on the unit token; versions are
    /// ignored (there's no meaningful difference between
    /// `-1.0.0` and `-1.0.1` for the unit vocabulary as of
    /// Revit 2026).
    pub fn from_forge_identifier(id: &str) -> Self {
        // Strip "autodesk.unit.unit:" prefix if present, then any
        // "-X.Y.Z" version suffix.
        let trimmed = id.trim().to_ascii_lowercase();
        let stripped = trimmed
            .strip_prefix("autodesk.unit.unit:")
            .unwrap_or(&trimmed);
        let bare = stripped.split('-').next().unwrap_or(stripped);
        match bare {
            "millimeters" | "mm" => ForgeUnit::Millimeters,
            "centimeters" | "cm" => ForgeUnit::Centimeters,
            "decimeters" | "dm" => ForgeUnit::Decimeters,
            "meters" | "m" => ForgeUnit::Meters,
            "kilometers" | "km" => ForgeUnit::Kilometers,
            "inches" | "in" => ForgeUnit::Inches,
            "feet" | "ft" | "fractionalinches" | "feetandfractionalinches" => ForgeUnit::Feet,
            "yards" | "yd" => ForgeUnit::Yards,
            "miles" | "mi" => ForgeUnit::Miles,
            "squaremillimeters" => ForgeUnit::SquareMillimeters,
            "squarecentimeters" => ForgeUnit::SquareCentimeters,
            "squaremeters" => ForgeUnit::SquareMeters,
            "squarefeet" => ForgeUnit::SquareFeet,
            "squareinches" => ForgeUnit::SquareInches,
            "squareyards" => ForgeUnit::SquareYards,
            "acres" => ForgeUnit::Acres,
            "hectares" => ForgeUnit::Hectares,
            "cubicmillimeters" => ForgeUnit::CubicMillimeters,
            "cubiccentimeters" => ForgeUnit::CubicCentimeters,
            "cubicmeters" => ForgeUnit::CubicMeters,
            "cubicfeet" => ForgeUnit::CubicFeet,
            "cubicinches" => ForgeUnit::CubicInches,
            "cubicyards" => ForgeUnit::CubicYards,
            "liters" | "litres" => ForgeUnit::Liters,
            "usgallons" | "gallons" => ForgeUnit::UsGallons,
            "radians" | "rad" => ForgeUnit::Radians,
            "degrees" | "deg" => ForgeUnit::Degrees,
            "grads" | "gradians" => ForgeUnit::Grads,
            "kilograms" | "kg" => ForgeUnit::Kilograms,
            "grams" | "g" => ForgeUnit::Grams,
            "pounds" | "lb" | "lbs" | "poundsmass" => ForgeUnit::Pounds,
            "seconds" | "s" => ForgeUnit::Seconds,
            "minutes" | "min" => ForgeUnit::Minutes,
            "hours" | "hr" | "h" => ForgeUnit::Hours,
            _ => ForgeUnit::Other(id.to_string()),
        }
    }

    /// Map a `ForgeUnit` to its IFC4 emission plan. Returns
    /// `None` for `Other(_)` — the caller should either fall back
    /// to a sensible default (usually metres) or surface a
    /// diagnostic.
    pub fn ifc_emission(&self) -> Option<IfcUnitEmission> {
        use IfcUnitType::*;
        match self {
            // Length — SI prefixes
            ForgeUnit::Millimeters => Some(IfcUnitEmission::Si {
                unit_type: Length,
                prefix: Some("MILLI"),
                name: "METRE",
            }),
            ForgeUnit::Centimeters => Some(IfcUnitEmission::Si {
                unit_type: Length,
                prefix: Some("CENTI"),
                name: "METRE",
            }),
            ForgeUnit::Decimeters => Some(IfcUnitEmission::Si {
                unit_type: Length,
                prefix: Some("DECI"),
                name: "METRE",
            }),
            ForgeUnit::Meters => Some(IfcUnitEmission::Si {
                unit_type: Length,
                prefix: None,
                name: "METRE",
            }),
            ForgeUnit::Kilometers => Some(IfcUnitEmission::Si {
                unit_type: Length,
                prefix: Some("KILO"),
                name: "METRE",
            }),
            // Length — conversion-based (Imperial)
            ForgeUnit::Inches => Some(IfcUnitEmission::ConversionBased {
                unit_type: Length,
                derived_name: "INCH",
                factor_to_si: 0.0254,
                si_base_name: "METRE",
            }),
            ForgeUnit::Feet => Some(IfcUnitEmission::ConversionBased {
                unit_type: Length,
                derived_name: "FOOT",
                factor_to_si: 0.3048,
                si_base_name: "METRE",
            }),
            ForgeUnit::Yards => Some(IfcUnitEmission::ConversionBased {
                unit_type: Length,
                derived_name: "YARD",
                factor_to_si: 0.9144,
                si_base_name: "METRE",
            }),
            ForgeUnit::Miles => Some(IfcUnitEmission::ConversionBased {
                unit_type: Length,
                derived_name: "MILE",
                factor_to_si: 1609.344,
                si_base_name: "METRE",
            }),
            // Area
            ForgeUnit::SquareMillimeters => Some(IfcUnitEmission::Si {
                unit_type: Area,
                prefix: Some("MILLI"),
                name: "SQUARE_METRE",
            }),
            ForgeUnit::SquareCentimeters => Some(IfcUnitEmission::Si {
                unit_type: Area,
                prefix: Some("CENTI"),
                name: "SQUARE_METRE",
            }),
            ForgeUnit::SquareMeters => Some(IfcUnitEmission::Si {
                unit_type: Area,
                prefix: None,
                name: "SQUARE_METRE",
            }),
            ForgeUnit::SquareFeet => Some(IfcUnitEmission::ConversionBased {
                unit_type: Area,
                derived_name: "SQUARE_FOOT",
                factor_to_si: 0.092_903_04,
                si_base_name: "SQUARE_METRE",
            }),
            ForgeUnit::SquareInches => Some(IfcUnitEmission::ConversionBased {
                unit_type: Area,
                derived_name: "SQUARE_INCH",
                factor_to_si: 0.000_645_16,
                si_base_name: "SQUARE_METRE",
            }),
            ForgeUnit::SquareYards => Some(IfcUnitEmission::ConversionBased {
                unit_type: Area,
                derived_name: "SQUARE_YARD",
                factor_to_si: 0.836_127_36,
                si_base_name: "SQUARE_METRE",
            }),
            ForgeUnit::Acres => Some(IfcUnitEmission::ConversionBased {
                unit_type: Area,
                derived_name: "ACRE",
                factor_to_si: 4046.856_422_4,
                si_base_name: "SQUARE_METRE",
            }),
            ForgeUnit::Hectares => Some(IfcUnitEmission::ConversionBased {
                unit_type: Area,
                derived_name: "HECTARE",
                factor_to_si: 10_000.0,
                si_base_name: "SQUARE_METRE",
            }),
            // Volume
            ForgeUnit::CubicMillimeters => Some(IfcUnitEmission::Si {
                unit_type: Volume,
                prefix: Some("MILLI"),
                name: "CUBIC_METRE",
            }),
            ForgeUnit::CubicCentimeters => Some(IfcUnitEmission::Si {
                unit_type: Volume,
                prefix: Some("CENTI"),
                name: "CUBIC_METRE",
            }),
            ForgeUnit::CubicMeters => Some(IfcUnitEmission::Si {
                unit_type: Volume,
                prefix: None,
                name: "CUBIC_METRE",
            }),
            ForgeUnit::CubicFeet => Some(IfcUnitEmission::ConversionBased {
                unit_type: Volume,
                derived_name: "CUBIC_FOOT",
                factor_to_si: 0.028_316_846_592,
                si_base_name: "CUBIC_METRE",
            }),
            ForgeUnit::CubicInches => Some(IfcUnitEmission::ConversionBased {
                unit_type: Volume,
                derived_name: "CUBIC_INCH",
                factor_to_si: 0.000_016_387_064,
                si_base_name: "CUBIC_METRE",
            }),
            ForgeUnit::CubicYards => Some(IfcUnitEmission::ConversionBased {
                unit_type: Volume,
                derived_name: "CUBIC_YARD",
                factor_to_si: 0.764_554_857_984,
                si_base_name: "CUBIC_METRE",
            }),
            ForgeUnit::Liters => Some(IfcUnitEmission::ConversionBased {
                unit_type: Volume,
                derived_name: "LITRE",
                factor_to_si: 0.001,
                si_base_name: "CUBIC_METRE",
            }),
            ForgeUnit::UsGallons => Some(IfcUnitEmission::ConversionBased {
                unit_type: Volume,
                derived_name: "US_GALLON",
                factor_to_si: 0.003_785_411_784,
                si_base_name: "CUBIC_METRE",
            }),
            // Plane angle
            ForgeUnit::Radians => Some(IfcUnitEmission::Si {
                unit_type: PlaneAngle,
                prefix: None,
                name: "RADIAN",
            }),
            ForgeUnit::Degrees => Some(IfcUnitEmission::ConversionBased {
                unit_type: PlaneAngle,
                derived_name: "DEGREE",
                factor_to_si: std::f64::consts::PI / 180.0,
                si_base_name: "RADIAN",
            }),
            ForgeUnit::Grads => Some(IfcUnitEmission::ConversionBased {
                unit_type: PlaneAngle,
                derived_name: "GRAD",
                factor_to_si: std::f64::consts::PI / 200.0,
                si_base_name: "RADIAN",
            }),
            // Mass
            ForgeUnit::Kilograms => Some(IfcUnitEmission::Si {
                unit_type: Mass,
                prefix: None,
                name: "GRAM",
            }),
            ForgeUnit::Grams => Some(IfcUnitEmission::Si {
                unit_type: Mass,
                prefix: Some("MILLI"),
                name: "GRAM",
            }),
            ForgeUnit::Pounds => Some(IfcUnitEmission::ConversionBased {
                unit_type: Mass,
                derived_name: "POUND",
                factor_to_si: 0.453_592_37,
                si_base_name: "GRAM",
            }),
            // Time
            ForgeUnit::Seconds => Some(IfcUnitEmission::Si {
                unit_type: Time,
                prefix: None,
                name: "SECOND",
            }),
            ForgeUnit::Minutes => Some(IfcUnitEmission::ConversionBased {
                unit_type: Time,
                derived_name: "MINUTE",
                factor_to_si: 60.0,
                si_base_name: "SECOND",
            }),
            ForgeUnit::Hours => Some(IfcUnitEmission::ConversionBased {
                unit_type: Time,
                derived_name: "HOUR",
                factor_to_si: 3600.0,
                si_base_name: "SECOND",
            }),
            ForgeUnit::Other(_) => None,
        }
    }
}

/// A named collection of typed property values attached to a
/// BuildingElement. Emits as IfcPropertySet → IfcPropertySingleValue
/// → IfcRelDefinesByProperties in the STEP writer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PropertySet {
    /// Property-set name. Convention: "Pset_WallCommon",
    /// "Pset_DoorCommon", or "Pset_RevitType_{ClassName}".
    pub name: String,
    pub properties: Vec<Property>,
}

/// A single property inside a [`PropertySet`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Property {
    pub name: String,
    pub value: PropertyValue,
}

/// IFC4 IfcValue subtypes we surface from Revit decoded fields.
/// Maps directly to the `NominalValue` slot of IfcPropertySingleValue.
///
/// Quantity variants (`AreaSquareFeet`, `VolumeCubicFeet`, `CountValue`,
/// `TimeSeconds`, `MassPounds`) are the measurement-flavoured siblings
/// of the primitive variants. They emit as the matching
/// `IfcAreaMeasure` / `IfcVolumeMeasure` / `IfcCountMeasure` / etc.
/// constructors — semantically they correspond to the IFC4 `IfcQuantity*`
/// family, but we route them through the existing
/// `IfcPropertySingleValue` carrier so the writer doesn't need a
/// parallel `IfcElementQuantity` path yet (tracked separately).
/// Feet / cubic-feet / pounds inputs are converted to metres /
/// cubic-metres / kilograms at emit time so the STEP output is
/// unit-consistent with the project-level `IfcUnitAssignment`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value")]
pub enum PropertyValue {
    /// Free-form text — `IfcText`.
    Text(String),
    /// Integer count — `IfcInteger`.
    Integer(i64),
    /// Floating-point real — `IfcReal`.
    Real(f64),
    /// Boolean — `IfcBoolean`.
    Boolean(bool),
    /// Length measurement in feet (writer converts to metres).
    /// Maps to `IfcLengthMeasure` with project length unit.
    LengthFeet(f64),
    /// Angle in radians. Maps to `IfcPlaneAngleMeasure`.
    AngleRadians(f64),
    /// Area in square feet (writer converts to square metres).
    /// Maps to `IfcAreaMeasure`. (IFC-32)
    AreaSquareFeet(f64),
    /// Volume in cubic feet (writer converts to cubic metres).
    /// Maps to `IfcVolumeMeasure`. (IFC-32)
    VolumeCubicFeet(f64),
    /// Unitless discrete count — occupancy, rebar count, fixture count.
    /// Maps to `IfcCountMeasure`. (IFC-32)
    CountValue(i64),
    /// Time measurement in seconds. Maps to `IfcTimeMeasure`. (IFC-32)
    TimeSeconds(f64),
    /// Mass in pounds (writer converts to kilograms).
    /// Maps to `IfcMassMeasure`. (IFC-32)
    MassPounds(f64),
}

impl PropertyValue {
    /// Emit the STEP-level IfcValue constructor for this value.
    /// Used by the writer; exposed for testing.
    pub fn to_step(&self) -> String {
        match self {
            PropertyValue::Text(s) => format!("IFCTEXT('{}')", escape_step_string(s)),
            PropertyValue::Integer(n) => format!("IFCINTEGER({n})"),
            PropertyValue::Real(v) => format!("IFCREAL({v:.6})"),
            PropertyValue::Boolean(b) => {
                format!("IFCBOOLEAN(.{})", if *b { "T" } else { "F" })
            }
            PropertyValue::LengthFeet(ft) => {
                // Convert to metres at emit time (project length unit).
                let metres = ft * 0.3048;
                format!("IFCLENGTHMEASURE({metres:.6})")
            }
            PropertyValue::AngleRadians(r) => format!("IFCPLANEANGLEMEASURE({r:.6})"),
            PropertyValue::AreaSquareFeet(sqft) => {
                // 1 ft² = 0.09290304 m² (exact, from international foot).
                let sqm = sqft * 0.09290304;
                format!("IFCAREAMEASURE({sqm:.6})")
            }
            PropertyValue::VolumeCubicFeet(cuft) => {
                // 1 ft³ = 0.028316846592 m³ (exact).
                let cum = cuft * 0.028316846592;
                format!("IFCVOLUMEMEASURE({cum:.6})")
            }
            PropertyValue::CountValue(n) => format!("IFCCOUNTMEASURE({n})"),
            PropertyValue::TimeSeconds(s) => format!("IFCTIMEMEASURE({s:.6})"),
            PropertyValue::MassPounds(lb) => {
                // 1 lb = 0.45359237 kg (exact, international avoirdupois pound).
                let kg = lb * 0.45359237;
                format!("IFCMASSMEASURE({kg:.6})")
            }
        }
    }
}

/// Minimal STEP string escape — apostrophes doubled, backslashes
/// doubled. Non-ASCII escape isn't needed here because the writer's
/// full `escape()` is used for BuildingElement names; property
/// values are typically numeric or short ASCII. Keeping the escape
/// local to entities.rs avoids a cross-module dependency.
fn escape_step_string(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '\'' => out.push_str("''"),
            '\\' => out.push_str("\\\\"),
            c => out.push(c),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn property_value_to_step_primitives() {
        assert_eq!(PropertyValue::Integer(42).to_step(), "IFCINTEGER(42)");
        assert_eq!(PropertyValue::Real(1.25).to_step(), "IFCREAL(1.250000)");
        assert_eq!(PropertyValue::Boolean(true).to_step(), "IFCBOOLEAN(.T)");
        assert_eq!(PropertyValue::Boolean(false).to_step(), "IFCBOOLEAN(.F)");
        assert_eq!(
            PropertyValue::Text("hello".into()).to_step(),
            "IFCTEXT('hello')"
        );
        assert_eq!(
            PropertyValue::Text("it's".into()).to_step(),
            "IFCTEXT('it''s')"
        );
    }

    #[test]
    fn property_value_to_step_length_and_angle() {
        // 10 ft = 3.048 m.
        assert_eq!(
            PropertyValue::LengthFeet(10.0).to_step(),
            "IFCLENGTHMEASURE(3.048000)"
        );
        // π/2 rad comes out to ~1.570796.
        let step = PropertyValue::AngleRadians(std::f64::consts::FRAC_PI_2).to_step();
        assert!(step.starts_with("IFCPLANEANGLEMEASURE(1.570796"));
    }

    /// IFC-32: quantity variants emit IfcAreaMeasure / IfcVolumeMeasure
    /// / IfcCountMeasure / IfcTimeMeasure / IfcMassMeasure, with the
    /// Imperial → SI conversion applied at emit time so downstream
    /// tools see unit-consistent output against the project's SI
    /// `IfcUnitAssignment`.
    /// IFC-28: MaterialLayerSet totals its layer thicknesses.
    #[test]
    fn material_layer_set_total_thickness() {
        let lset = MaterialLayerSet {
            name: "Ext - Generic 8\" Wall".into(),
            description: None,
            layers: vec![
                MaterialLayer {
                    material_index: 0,
                    thickness_feet: 5.0 / 12.0, // 5"
                    name: Some("Finish".into()),
                },
                MaterialLayer {
                    material_index: 1,
                    thickness_feet: 2.0 / 12.0, // 2"
                    name: Some("Structure".into()),
                },
                MaterialLayer {
                    material_index: 2,
                    thickness_feet: 1.0 / 12.0, // 1"
                    name: Some("Air Gap".into()),
                },
            ],
        };
        let total = lset.total_thickness_feet();
        assert!((total - (8.0 / 12.0)).abs() < 1e-9);
    }

    #[test]
    fn property_value_to_step_quantities() {
        // 1 ft² = 0.09290304 m² exact.
        assert_eq!(
            PropertyValue::AreaSquareFeet(1.0).to_step(),
            "IFCAREAMEASURE(0.092903)"
        );
        // 100 ft² = 9.290304 m².
        assert_eq!(
            PropertyValue::AreaSquareFeet(100.0).to_step(),
            "IFCAREAMEASURE(9.290304)"
        );

        // 1 ft³ = 0.028316846592 m³ exact.
        assert_eq!(
            PropertyValue::VolumeCubicFeet(1.0).to_step(),
            "IFCVOLUMEMEASURE(0.028317)"
        );

        // Counts are unitless integers.
        assert_eq!(
            PropertyValue::CountValue(12).to_step(),
            "IFCCOUNTMEASURE(12)"
        );

        // Time in seconds is already SI.
        assert_eq!(
            PropertyValue::TimeSeconds(3.0).to_step(),
            "IFCTIMEMEASURE(3.000000)"
        );

        // 1 lb = 0.45359237 kg exact.
        assert_eq!(
            PropertyValue::MassPounds(1.0).to_step(),
            "IFCMASSMEASURE(0.453592)"
        );
    }

    // ---------------- IFC-39 / IFC-40: ForgeUnit parsing + IFC map ----------------

    #[test]
    fn forge_unit_parses_canonical_ids() {
        // Canonical: `autodesk.unit.unit:<name>-<version>`.
        assert_eq!(
            ForgeUnit::from_forge_identifier("autodesk.unit.unit:millimeters-1.0.1"),
            ForgeUnit::Millimeters
        );
        assert_eq!(
            ForgeUnit::from_forge_identifier("autodesk.unit.unit:feet-1.0.1"),
            ForgeUnit::Feet
        );
        assert_eq!(
            ForgeUnit::from_forge_identifier("autodesk.unit.unit:degrees-1.0.1"),
            ForgeUnit::Degrees
        );
    }

    #[test]
    fn forge_unit_parses_without_version() {
        assert_eq!(
            ForgeUnit::from_forge_identifier("autodesk.unit.unit:meters"),
            ForgeUnit::Meters
        );
    }

    #[test]
    fn forge_unit_parses_bare_names() {
        // Bare-name lenient path — sometimes PartAtom carries just
        // the tail (`millimeters`) rather than the full identifier.
        assert_eq!(
            ForgeUnit::from_forge_identifier("millimeters"),
            ForgeUnit::Millimeters
        );
        assert_eq!(ForgeUnit::from_forge_identifier("Feet"), ForgeUnit::Feet);
    }

    #[test]
    fn forge_unit_unknown_falls_through_to_other() {
        let fu = ForgeUnit::from_forge_identifier("autodesk.unit.unit:furlongsPerFortnight-1.0.1");
        match fu {
            ForgeUnit::Other(id) => assert!(id.contains("furlongsPerFortnight")),
            _ => panic!("unknown units must map to ForgeUnit::Other(_)"),
        }
    }

    #[test]
    fn forge_unit_length_metric_emits_si() {
        match ForgeUnit::Millimeters.ifc_emission().unwrap() {
            IfcUnitEmission::Si {
                unit_type,
                prefix,
                name,
            } => {
                assert_eq!(unit_type, IfcUnitType::Length);
                assert_eq!(prefix, Some("MILLI"));
                assert_eq!(name, "METRE");
            }
            _ => panic!("millimeters should emit as IfcSIUnit"),
        }
    }

    #[test]
    fn forge_unit_feet_emits_conversion_based() {
        match ForgeUnit::Feet.ifc_emission().unwrap() {
            IfcUnitEmission::ConversionBased {
                unit_type,
                derived_name,
                factor_to_si,
                si_base_name,
            } => {
                assert_eq!(unit_type, IfcUnitType::Length);
                assert_eq!(derived_name, "FOOT");
                // 1 foot = 0.3048 metre exactly.
                assert!((factor_to_si - 0.3048).abs() < 1e-12);
                assert_eq!(si_base_name, "METRE");
            }
            _ => panic!("feet should emit as IfcConversionBasedUnit"),
        }
    }

    #[test]
    fn forge_unit_all_known_variants_have_emission() {
        // Every non-Other variant MUST have a defined ifc_emission.
        // (Pinned as an invariant — if a new unit lands without its
        // mapping, this test fails instead of the bug shipping.)
        for fu in [
            ForgeUnit::Millimeters,
            ForgeUnit::Centimeters,
            ForgeUnit::Decimeters,
            ForgeUnit::Meters,
            ForgeUnit::Kilometers,
            ForgeUnit::Inches,
            ForgeUnit::Feet,
            ForgeUnit::Yards,
            ForgeUnit::Miles,
            ForgeUnit::SquareMillimeters,
            ForgeUnit::SquareCentimeters,
            ForgeUnit::SquareMeters,
            ForgeUnit::SquareFeet,
            ForgeUnit::SquareInches,
            ForgeUnit::SquareYards,
            ForgeUnit::Acres,
            ForgeUnit::Hectares,
            ForgeUnit::CubicMillimeters,
            ForgeUnit::CubicCentimeters,
            ForgeUnit::CubicMeters,
            ForgeUnit::CubicFeet,
            ForgeUnit::CubicInches,
            ForgeUnit::CubicYards,
            ForgeUnit::Liters,
            ForgeUnit::UsGallons,
            ForgeUnit::Radians,
            ForgeUnit::Degrees,
            ForgeUnit::Grads,
            ForgeUnit::Kilograms,
            ForgeUnit::Grams,
            ForgeUnit::Pounds,
            ForgeUnit::Seconds,
            ForgeUnit::Minutes,
            ForgeUnit::Hours,
        ] {
            assert!(
                fu.ifc_emission().is_some(),
                "ForgeUnit {fu:?} has no ifc_emission mapping"
            );
        }
        // ...while Other must return None.
        assert!(ForgeUnit::Other("x".into()).ifc_emission().is_none());
    }

    #[test]
    fn ifc_unit_type_step_tokens() {
        assert_eq!(IfcUnitType::Length.as_step_token(), "LENGTHUNIT");
        assert_eq!(IfcUnitType::Area.as_step_token(), "AREAUNIT");
        assert_eq!(IfcUnitType::Volume.as_step_token(), "VOLUMEUNIT");
        assert_eq!(IfcUnitType::PlaneAngle.as_step_token(), "PLANEANGLEUNIT");
        assert_eq!(IfcUnitType::Mass.as_step_token(), "MASSUNIT");
        assert_eq!(IfcUnitType::Time.as_step_token(), "TIMEUNIT");
    }
}
