//! Minimal STEP / ISO-10303-21 serializer for `IfcModel` → valid IFC4.
//!
//! This is the serialization half of Layer 5. Given an `IfcModel` in
//! memory, produce a .ifc text file that a spec-compliant reader
//! (IfcOpenShell, BlenderBIM, buildingSMART validator) accepts.
//!
//! The output is intentionally minimal but **structurally valid**: a
//! well-formed IFC4 schema header, the required framework entities
//! (`IfcPerson` / `IfcOrganization` / `IfcApplication` /
//! `IfcOwnerHistory` / `IfcSIUnit` / `IfcUnitAssignment` /
//! `IfcGeometricRepresentationContext`), and an `IfcProject` populated
//! from the model's metadata. As the walker grows, `BuildingElement`
//! and family entities will land too; those extensions plug in here
//! without touching the header-level plumbing.
//!
//! Design principle: string-based emission, no external IFC library
//! dependency, fully `#![deny(unsafe_code)]`-clean.

use std::collections::HashMap;

use super::IfcModel;
use super::entities::{Extrusion, OpeningCut, SolidShape};

/// Options controlling STEP serialization.
#[derive(Debug, Clone, Default)]
pub struct StepOptions {
    /// If `Some`, use this Unix timestamp (in seconds) for both the
    /// `FILE_NAME` header and the `IfcOwnerHistory` creation time
    /// instead of `SystemTime::now()`. Setting this makes the
    /// output deterministic — identical `(IfcModel, StepOptions)`
    /// pairs produce byte-identical STEP strings, which makes
    /// STEP-text diffs tractable and regression tests reliable.
    pub timestamp: Option<i64>,
}

/// Serialize an `IfcModel` into an IFC4 STEP text stream. The output
/// includes the ISO-10303-21 envelope and a minimal but spec-valid
/// data section centred on `IfcProject`. Uses current wall-clock
/// timestamp. For deterministic output (e.g. tests), use
/// [`write_step_with_options`] with `StepOptions::timestamp = Some(_)`.
pub fn write_step(model: &IfcModel) -> String {
    write_step_with_options(model, &StepOptions::default())
}

/// Deterministic-option variant of [`write_step`].
///
/// When `options.timestamp = Some(t)`, the emitted STEP is a pure
/// function of `(model, t)` — no wall-clock access. Use this in
/// tests and regression fixtures.
pub fn write_step_with_options(model: &IfcModel, options: &StepOptions) -> String {
    let now = options.timestamp.unwrap_or_else(unix_seconds);
    let mut w = StepWriter::new(now);
    w.emit_header(model);
    w.emit_data(model);
    w.finish()
}

/// Attribute layout of the tail an IFC4 building-element instance
/// carries after `IfcProduct`'s seven common attributes
/// (`GlobalId`, `OwnerHistory`, `Name`, `Description`, `ObjectType`,
/// `ObjectPlacement`, `Representation`).
///
/// IFC4 requires an instance to list **every** attribute its entity
/// type declares; an unknown optional occupies its slot as `$`.
/// Omitting the trailing ones entirely — which is what the writer did
/// before #214 — yields a record that most readers tolerate but that
/// no spec-conformant consumer can index, so `IfcColumn.PredefinedType`
/// raised "index 8 is out of range for variant of size 8".
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ElementTail {
    /// `Tag` only — the entity declares no `PredefinedType`
    /// (`IfcFurnishingElement`, `IfcFlowController`). Arity 8.
    Tag,
    /// `Tag, PredefinedType` — the shape almost every
    /// `IfcBuildingElement` subtype takes. Arity 9.
    TagPredefined,
    /// `Tag, OverallHeight, OverallWidth, PredefinedType,
    /// OperationType|PartitioningType, UserDefined…Type` —
    /// `IfcDoor` / `IfcWindow`. Arity 13.
    Opening,
    /// `LongName, CompositionType, PredefinedType,
    /// ElevationWithFlooring` — `IfcSpace`, which is an
    /// `IfcSpatialStructureElement` and so declares no `Tag` at all.
    /// Arity 11.
    Space,
    /// `Tag, SteelGrade, NominalDiameter, CrossSectionArea,
    /// BarLength, PredefinedType, BarSurface` —
    /// `IfcReinforcingBar`. Arity 14.
    ReinforcingBar,
    /// `Tag, NumberOfRisers, NumberOfTreads, RiserHeight,
    /// TreadLength, PredefinedType` — `IfcStairFlight`. Arity 13.
    StairFlight,
}

/// Which tail an emitted STEP entity name takes.
///
/// Every value here is the IFC4 EXPRESS declaration, cross-checked
/// against `ifcopenshell.ifcopenshell_wrapper.schema_by_name("IFC4")
/// .declaration_by_name(<name>).attribute_count()`; the same table is
/// re-derived from the schema by `tools/ci/ifc_schema_arity.py` on
/// every emitted file, so a wrong row cannot survive CI.
///
/// Unlisted names fall through to [`ElementTail::TagPredefined`],
/// which is the shape of every remaining `IfcBuildingElement`
/// subtype in IFC4.
fn element_tail_for(ifc_upper: &str) -> ElementTail {
    match ifc_upper {
        "IFCFURNISHINGELEMENT" | "IFCFLOWCONTROLLER" => ElementTail::Tag,
        "IFCDOOR" | "IFCWINDOW" => ElementTail::Opening,
        "IFCSPACE" => ElementTail::Space,
        "IFCREINFORCINGBAR" => ElementTail::ReinforcingBar,
        "IFCSTAIRFLIGHT" => ElementTail::StairFlight,
        _ => ElementTail::TagPredefined,
    }
}

/// Accept a `PredefinedType` token only when it is a well-formed STEP
/// enumeration name: uppercase ASCII letters, digits and underscores,
/// starting with a letter.
///
/// An enumeration reference is written bare between dots, so unlike a
/// string it has no escape form — a caller-supplied value with a
/// quote, dot or newline in it would corrupt the record. Rejecting
/// writes `$` instead, which is always legal for an optional
/// attribute. (`tests/fuzz_regressions.rs` drives hostile names
/// through this path.)
fn step_enum_token(raw: &str) -> Option<String> {
    let token = raw.trim().to_ascii_uppercase();
    let mut chars = token.chars();
    match chars.next() {
        Some(c) if c.is_ascii_uppercase() => {}
        _ => return None,
    }
    if chars.all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_') {
        Some(token)
    } else {
        None
    }
}

/// The IFC4 type entity an occurrence of `ifc_upper` is typed by, and the
/// attributes that entity declares after `ElementType`, with `predefined`
/// the occurrence's `PredefinedType` (RE-110). An occurrence and its type
/// share one enumeration. `None` for entities rvt-rs does not type.
fn type_entity_for(ifc_upper: &str, predefined: Option<&str>) -> Option<(String, String)> {
    let pt = format!(
        ".{}.",
        predefined
            .and_then(step_enum_token)
            .unwrap_or_else(|| "NOTDEFINED".into())
    );
    let tail = match ifc_upper {
        "IFCDOOR" | "IFCWINDOW" => format!("{pt},.NOTDEFINED.,$,$"),
        "IFCFURNITURE" => format!(".NOTDEFINED.,{pt}"),
        "IFCWALL"
        | "IFCSLAB"
        | "IFCBEAM"
        | "IFCCOLUMN"
        | "IFCMEMBER"
        | "IFCPLATE"
        | "IFCCOVERING"
        | "IFCCURTAINWALL"
        | "IFCFOOTING"
        | "IFCRAMP"
        | "IFCROOF"
        | "IFCSTAIR"
        | "IFCSTAIRFLIGHT"
        | "IFCSHADINGDEVICE"
        | "IFCBUILDINGELEMENTPROXY"
        | "IFCAIRTERMINAL"
        | "IFCALARM"
        | "IFCDUCTFITTING"
        | "IFCDUCTSEGMENT"
        | "IFCELECTRICAPPLIANCE"
        | "IFCLIGHTFIXTURE"
        | "IFCPIPEFITTING"
        | "IFCPIPESEGMENT"
        | "IFCSANITARYTERMINAL"
        | "IFCTRANSPORTELEMENT" => pt,
        _ => return None,
    };
    Some((format!("{ifc_upper}TYPE"), tail))
}

/// Render the attribute tail for one building element.
///
/// `tag_quoted` and `name_quoted` are already STEP-quoted (`'…'` or
/// `$`). `predefined` is the bare enum token (`"COLUMN"`), written as
/// `.COLUMN.`; `None` writes `$` — the writer never invents an enum
/// value it did not decode.
fn element_attribute_tail(
    ifc_upper: &str,
    tag_quoted: &str,
    predefined: Option<&str>,
    long_name_quoted: &str,
    overall_height_width_feet: Option<(f64, f64)>,
) -> String {
    let pt = match predefined.and_then(step_enum_token) {
        Some(token) => format!(".{token}."),
        None => "$".to_string(),
    };
    match element_tail_for(ifc_upper) {
        ElementTail::Tag => tag_quoted.to_string(),
        ElementTail::TagPredefined => format!("{tag_quoted},{pt}"),
        // OverallHeight / OverallWidth are the type's, where read (RE-93),
        // and otherwise `$`.
        ElementTail::Opening => match overall_height_width_feet {
            Some((height, width)) => format!(
                "{tag_quoted},{:.6},{:.6},{pt},$,$",
                height * 0.3048,
                width * 0.3048
            ),
            None => format!("{tag_quoted},$,$,{pt},$,$"),
        },
        // IfcSpace has no Tag attribute — slot 8 is LongName. Before
        // #214 the writer put the type GUID there, which typed as a
        // label but meant the wrong thing; the element's own Name
        // carries the identity instead. CompositionType `.ELEMENT.`
        // matches what Revit's exporter writes for a room.
        //
        // `long_name_quoted` is the room's name from its `RoomName`
        // property, the slot Revit's own exporter puts it in, or `$`
        // (RE-117). The element Name is its room number.
        ElementTail::Space => format!("{long_name_quoted},.ELEMENT.,{pt},$"),
        ElementTail::ReinforcingBar => format!("{tag_quoted},$,$,$,$,{pt},$"),
        ElementTail::StairFlight => format!("{tag_quoted},$,$,$,$,{pt}"),
    }
}

struct StepWriter {
    out: String,
    next_id: usize,
    /// Unix timestamp (seconds) used for FILE_NAME + IfcOwnerHistory.
    /// Injected at construction so the output is a pure function of
    /// `(model, timestamp)` when a fixed timestamp is supplied.
    timestamp: i64,
}

impl StepWriter {
    /// The `Position` and depth, metres, of an opening's extruded solid: the
    /// shared `solid_position` and the body's `depth_m`, or, for a cut with
    /// its own height (RE-93), a placement that far above the element's and
    /// that height.
    fn opening_solid_position(
        &mut self,
        solid_position: usize,
        cut: Option<&OpeningCut>,
        depth_m: f64,
    ) -> (usize, f64) {
        let Some([z, height]) = cut.and_then(|cut| cut.z_range_feet) else {
            return (solid_position, depth_m);
        };
        let point = self.id();
        self.emit_entity(
            point,
            format!("IFCCARTESIANPOINT((0.,0.,{:.6}))", z * 0.3048),
        );
        let position = self.id();
        self.emit_entity(position, format!("IFCAXIS2PLACEMENT3D(#{point},$,$)"));
        (position, height * 0.3048)
    }

    fn new(timestamp: i64) -> Self {
        Self {
            out: String::new(),
            next_id: 1,
            timestamp,
        }
    }

    fn id(&mut self) -> usize {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    fn emit_line<S: AsRef<str>>(&mut self, line: S) {
        self.out.push_str(line.as_ref());
        self.out.push('\n');
    }

    fn emit_entity<S: AsRef<str>>(&mut self, id: usize, body: S) {
        self.out.push_str(&format!("#{id}={};\n", body.as_ref()));
    }

    /// Emit one `IfcProfileDef` subclass from an [`Extrusion`],
    /// dispatching on [`Extrusion::profile_override`]. Returns the
    /// entity ID of the emitted profile so the caller can wire it
    /// into an `IFCEXTRUDEDAREASOLID`.
    ///
    /// - `ex.profile_override = None` emits the default
    ///   `IFCRECTANGLEPROFILEDEF` from `width_feet` × `depth_feet`.
    /// - `Circle { radius }` emits `IFCCIRCLEPROFILEDEF`.
    /// - `IShape { … }` emits `IFCIShapeProfileDef` with a full
    ///   OverallWidth / OverallDepth / WebThickness /
    ///   FlangeThickness attribute set (fillet radius = $).
    /// - `TShape { … }` emits `IFCTShapeProfileDef`.
    /// - `LShape { … }` emits `IFCLShapeProfileDef` (EdgeRadius,
    ///   LegSlope = $).
    /// - `UShape { … }` emits `IFCUShapeProfileDef`.
    /// - `RectangleHollow { … }` emits
    ///   `IFCRectangleHollowProfileDef`.
    /// - `CircleHollow { … }` emits `IFCCircleHollowProfileDef`.
    /// - `ArbitraryClosed { points }` emits `IFCPOLYLINE` +
    ///   `IFCArbitraryClosedProfileDef`; if the polyline isn't
    ///   already closed (last == first), the writer appends the
    ///   first point at the tail.
    /// - `ArbitraryWithVoids { points, voids }` emits one
    ///   `IFCPOLYLINE` for the outer ring and one per hole, then
    ///   `IFCArbitraryProfileDefWithVoids`.
    ///
    /// All length values are converted from feet to metres at emit
    /// time (factor 0.3048).
    fn emit_profile_def(&mut self, ex: &Extrusion, profile_placement: usize) -> usize {
        use super::entities::ProfileDef;

        let profile_id = self.id();
        match &ex.profile_override {
            None | Some(ProfileDef::Rectangle { .. }) => {
                let (w_ft, d_ft) = match ex.profile_override {
                    Some(ProfileDef::Rectangle {
                        width_feet,
                        depth_feet,
                    }) => (width_feet, depth_feet),
                    _ => (ex.width_feet, ex.depth_feet),
                };
                let x_dim = w_ft * 0.3048;
                let y_dim = d_ft * 0.3048;
                self.emit_entity(
                    profile_id,
                    format!(
                        "IFCRECTANGLEPROFILEDEF(.AREA.,$,#{profile_placement},{x_dim:.6},{y_dim:.6})"
                    ),
                );
            }
            Some(ProfileDef::Circle { radius_feet }) => {
                let r = radius_feet * 0.3048;
                self.emit_entity(
                    profile_id,
                    format!("IFCCIRCLEPROFILEDEF(.AREA.,$,#{profile_placement},{r:.6})"),
                );
            }
            Some(ProfileDef::IShape {
                overall_width_feet,
                overall_depth_feet,
                web_thickness_feet,
                flange_thickness_feet,
            }) => {
                let w = overall_width_feet * 0.3048;
                let d = overall_depth_feet * 0.3048;
                let tw = web_thickness_feet * 0.3048;
                let tf = flange_thickness_feet * 0.3048;
                // IFCIShapeProfileDef(ProfileType, ProfileName,
                // Position, OverallWidth, OverallDepth,
                // WebThickness, FlangeThickness, FilletRadius?,
                // FlangeEdgeRadius?, FlangeSlope?) — ten attributes
                // in IFC4; the last two are IFC4 additions over
                // IFC2X3 and must still occupy their slots (#214).
                self.emit_entity(
                    profile_id,
                    format!(
                        "IFCISHAPEPROFILEDEF(.AREA.,$,#{profile_placement},{w:.6},{d:.6},{tw:.6},{tf:.6},$,$,$)"
                    ),
                );
            }
            Some(ProfileDef::TShape {
                overall_depth_feet,
                flange_width_feet,
                web_thickness_feet,
                flange_thickness_feet,
            }) => {
                let d = overall_depth_feet * 0.3048;
                let fw = flange_width_feet * 0.3048;
                let tw = web_thickness_feet * 0.3048;
                let tf = flange_thickness_feet * 0.3048;
                // IFCTShapeProfileDef(ProfileType, ProfileName,
                // Position, Depth, FlangeWidth, WebThickness,
                // FlangeThickness, FilletRadius?, FlangeEdgeRadius?,
                // WebEdgeRadius?, WebSlope?, FlangeSlope?).
                self.emit_entity(
                    profile_id,
                    format!(
                        "IFCTSHAPEPROFILEDEF(.AREA.,$,#{profile_placement},{d:.6},{fw:.6},{tw:.6},{tf:.6},$,$,$,$,$)"
                    ),
                );
            }
            Some(ProfileDef::LShape {
                overall_depth_feet,
                overall_width_feet,
                thickness_feet,
            }) => {
                let d = overall_depth_feet * 0.3048;
                let w = overall_width_feet * 0.3048;
                let t = thickness_feet * 0.3048;
                // IFCLShapeProfileDef(ProfileType, ProfileName,
                // Position, Depth, Width, Thickness, FilletRadius?,
                // EdgeRadius?, LegSlope?).
                self.emit_entity(
                    profile_id,
                    format!(
                        "IFCLSHAPEPROFILEDEF(.AREA.,$,#{profile_placement},{d:.6},{w:.6},{t:.6},$,$,$)"
                    ),
                );
            }
            Some(ProfileDef::UShape {
                overall_depth_feet,
                flange_width_feet,
                web_thickness_feet,
                flange_thickness_feet,
            }) => {
                let d = overall_depth_feet * 0.3048;
                let fw = flange_width_feet * 0.3048;
                let tw = web_thickness_feet * 0.3048;
                let tf = flange_thickness_feet * 0.3048;
                // IFCUShapeProfileDef(ProfileType, ProfileName,
                // Position, Depth, FlangeWidth, WebThickness,
                // FlangeThickness, FilletRadius?,
                // EdgeRadius?, FlangeSlope?).
                self.emit_entity(
                    profile_id,
                    format!(
                        "IFCUSHAPEPROFILEDEF(.AREA.,$,#{profile_placement},{d:.6},{fw:.6},{tw:.6},{tf:.6},$,$,$)"
                    ),
                );
            }
            Some(ProfileDef::RectangleHollow {
                overall_width_feet,
                overall_depth_feet,
                wall_thickness_feet,
            }) => {
                let w = overall_width_feet * 0.3048;
                let d = overall_depth_feet * 0.3048;
                let t = wall_thickness_feet * 0.3048;
                // IFCRectangleHollowProfileDef(ProfileType, ProfileName,
                // Position, XDim, YDim, WallThickness,
                // InnerFilletRadius?, OuterFilletRadius?).
                self.emit_entity(
                    profile_id,
                    format!(
                        "IFCRECTANGLEHOLLOWPROFILEDEF(.AREA.,$,#{profile_placement},{w:.6},{d:.6},{t:.6},$,$)"
                    ),
                );
            }
            Some(ProfileDef::CircleHollow {
                radius_feet,
                wall_thickness_feet,
            }) => {
                let r = radius_feet * 0.3048;
                let t = wall_thickness_feet * 0.3048;
                // IFCCircleHollowProfileDef(ProfileType, ProfileName,
                // Position, Radius, WallThickness).
                self.emit_entity(
                    profile_id,
                    format!(
                        "IFCCIRCLEHOLLOWPROFILEDEF(.AREA.,$,#{profile_placement},{r:.6},{t:.6})"
                    ),
                );
            }
            Some(ProfileDef::ArbitraryClosed { points }) => {
                // Build the polyline by emitting one IFCCARTESIANPOINT
                // per vertex, then an IFCPOLYLINE that references them
                // as its Points list. Auto-close by appending the first
                // point if the last doesn't already equal it.
                let mut pts: Vec<(f64, f64)> = points
                    .iter()
                    .map(|(x, y)| (*x * 0.3048, *y * 0.3048))
                    .collect();
                if let (Some(first), Some(last)) = (pts.first().copied(), pts.last().copied()) {
                    if (first.0 - last.0).abs().max((first.1 - last.1).abs()) > 1e-9_f64 {
                        pts.push(first);
                    }
                }
                let mut point_ids: Vec<usize> = Vec::with_capacity(pts.len());
                for (x, y) in &pts {
                    let pt_id = self.id();
                    self.emit_entity(pt_id, format!("IFCCARTESIANPOINT(({x:.6},{y:.6}))"));
                    point_ids.push(pt_id);
                }
                let polyline_id = self.id();
                let refs = point_ids
                    .iter()
                    .map(|id| format!("#{id}"))
                    .collect::<Vec<_>>()
                    .join(",");
                self.emit_entity(polyline_id, format!("IFCPOLYLINE(({refs}))"));
                // IFCArbitraryClosedProfileDef(ProfileType, ProfileName,
                // OuterCurve).
                self.emit_entity(
                    profile_id,
                    format!("IFCARBITRARYCLOSEDPROFILEDEF(.AREA.,$,#{polyline_id})"),
                );
            }
            Some(ProfileDef::ArbitraryWithVoids { points, voids }) => {
                let outer_id = self.emit_closed_polyline(points);
                let inner_ids: Vec<usize> = voids
                    .iter()
                    .map(|hole| self.emit_closed_polyline(hole))
                    .collect();
                let inner_refs = inner_ids
                    .iter()
                    .map(|id| format!("#{id}"))
                    .collect::<Vec<_>>()
                    .join(",");
                // IFCArbitraryProfileDefWithVoids(ProfileType,
                // ProfileName, OuterCurve, InnerCurves) — four
                // attributes in IFC4, the fourth a set of curves.
                self.emit_entity(
                    profile_id,
                    format!("IFCARBITRARYPROFILEDEFWITHVOIDS(.AREA.,$,#{outer_id},({inner_refs}))"),
                );
            }
        }
        profile_id
    }

    /// Emit one `IFCCARTESIANPOINT` per vertex plus the
    /// `IFCPOLYLINE` that lists them, closing the ring by repeating
    /// the first point when the caller did not. Returns the polyline
    /// entity id. Coordinates are feet in, metres out.
    fn emit_closed_polyline(&mut self, points: &[(f64, f64)]) -> usize {
        let mut metres: Vec<(f64, f64)> = points
            .iter()
            .map(|(x, y)| (*x * 0.3048, *y * 0.3048))
            .collect();
        if let (Some(first), Some(last)) = (metres.first().copied(), metres.last().copied()) {
            if (first.0 - last.0).abs().max((first.1 - last.1).abs()) > 1e-9_f64 {
                metres.push(first);
            }
        }
        let mut point_ids: Vec<usize> = Vec::with_capacity(metres.len());
        for (x, y) in &metres {
            let pt_id = self.id();
            self.emit_entity(pt_id, format!("IFCCARTESIANPOINT(({x:.6},{y:.6}))"));
            point_ids.push(pt_id);
        }
        let polyline_id = self.id();
        let refs = point_ids
            .iter()
            .map(|id| format!("#{id}"))
            .collect::<Vec<_>>()
            .join(",");
        self.emit_entity(polyline_id, format!("IFCPOLYLINE(({refs}))"));
        polyline_id
    }

    /// Emit a richer solid body (IFC-18 / IFC-19 / IFC-20) and
    /// return `(solid_entity_id, representation_type_token)`. The
    /// second value is the `IfcShapeRepresentation.RepresentationType`
    /// token that the caller should use: `"SweptSolid"` for
    /// extruded / revolved, `"CSG"` for boolean results, `"Brep"`
    /// for faceted breps.
    ///
    /// The caller is responsible for wrapping the returned entity
    /// ID in an `IfcShapeRepresentation` + `IfcProductDefinitionShape`
    /// chain.
    ///
    /// `solid_position` is the `IFCAXIS2PLACEMENT3D` that goes into
    /// the swept solid's `Position` slot, and `z_axis` the
    /// `IFCDIRECTION` used as the extrusion direction.
    ///
    /// `Position` is the profile's frame **inside** the object's own
    /// coordinate system, not a second copy of the object placement:
    /// a conforming consumer composes `ObjectPlacement × Position`, so
    /// passing the element's own axis here applies the element
    /// translation twice (#232). Callers that author their profiles in
    /// the element-local frame — every rvt-rs path does — pass the
    /// project-level identity placement.
    fn emit_solid_shape(
        &mut self,
        shape: &SolidShape,
        solid_position: usize,
        z_axis: usize,
    ) -> (usize, &'static str) {
        match shape {
            SolidShape::ExtrudedArea(ex) => {
                // Same chain as the inline extrusion path — call
                // emit_profile_def then wrap in IFCEXTRUDEDAREASOLID.
                let profile_origin = self.id();
                self.emit_entity(profile_origin, "IFCCARTESIANPOINT((0.,0.))");
                let profile_x_axis = self.id();
                self.emit_entity(profile_x_axis, "IFCDIRECTION((1.,0.))");
                let profile_placement = self.id();
                self.emit_entity(
                    profile_placement,
                    format!("IFCAXIS2PLACEMENT2D(#{profile_origin},#{profile_x_axis})"),
                );
                let profile_id = self.emit_profile_def(ex, profile_placement);
                let depth = ex.height_feet * 0.3048;
                let solid_id = self.id();
                self.emit_entity(
                    solid_id,
                    format!(
                        "IFCEXTRUDEDAREASOLID(#{profile_id},#{solid_position},#{z_axis},{depth:.6})"
                    ),
                );
                (solid_id, "SweptSolid")
            }
            SolidShape::RevolvedArea {
                profile,
                axis_origin_feet,
                axis_direction,
                angle_radians,
            } => {
                // Axis-of-revolution: IfcAxis1Placement (Location,
                // Direction). Both location and direction are
                // element-local.
                let axis_origin_id = self.id();
                let [ox, oy, oz] = *axis_origin_feet;
                let (ox, oy, oz) = (ox * 0.3048, oy * 0.3048, oz * 0.3048);
                self.emit_entity(
                    axis_origin_id,
                    format!("IFCCARTESIANPOINT(({ox:.6},{oy:.6},{oz:.6}))"),
                );
                let axis_dir_id = self.id();
                let [dx, dy, dz] = *axis_direction;
                // Normalise to unit vector per IFC4.
                let mag = (dx * dx + dy * dy + dz * dz).sqrt().max(1e-12);
                let (dx, dy, dz) = (dx / mag, dy / mag, dz / mag);
                self.emit_entity(
                    axis_dir_id,
                    format!("IFCDIRECTION(({dx:.6},{dy:.6},{dz:.6}))"),
                );
                let axis1_id = self.id();
                self.emit_entity(
                    axis1_id,
                    format!("IFCAXIS1PLACEMENT(#{axis_origin_id},#{axis_dir_id})"),
                );
                // Profile (same 2D placement boilerplate as
                // extruded-area). Reuse emit_profile_def by wrapping
                // our ProfileDef in a temporary Extrusion — the
                // emit_profile_def signature takes an Extrusion but
                // only reads profile_override / width_feet / depth_feet.
                let profile_origin = self.id();
                self.emit_entity(profile_origin, "IFCCARTESIANPOINT((0.,0.))");
                let profile_x_axis = self.id();
                self.emit_entity(profile_x_axis, "IFCDIRECTION((1.,0.))");
                let profile_placement = self.id();
                self.emit_entity(
                    profile_placement,
                    format!("IFCAXIS2PLACEMENT2D(#{profile_origin},#{profile_x_axis})"),
                );
                let wrap_ex = Extrusion {
                    width_feet: 0.0,
                    depth_feet: 0.0,
                    height_feet: 0.0,
                    profile_override: Some(profile.clone()),
                };
                let profile_id = self.emit_profile_def(&wrap_ex, profile_placement);
                let solid_id = self.id();
                // IFCREVOLVEDAREASOLID(SweptArea, Position, Axis, Angle).
                // SI units = radians, writer converts caller's angle
                // (already in radians) at identity.
                self.emit_entity(
                    solid_id,
                    format!(
                        "IFCREVOLVEDAREASOLID(#{profile_id},#{solid_position},#{axis1_id},{angle_radians:.6})"
                    ),
                );
                (solid_id, "SweptSolid")
            }
            SolidShape::BooleanResult {
                op,
                operand_a,
                operand_b,
            } => {
                // Recursively emit operands, then wrap in
                // IFCBOOLEANRESULT(op, first, second). Nested
                // booleans compose naturally.
                let (a_id, _rep_a) = self.emit_solid_shape(operand_a, solid_position, z_axis);
                let (b_id, _rep_b) = self.emit_solid_shape(operand_b, solid_position, z_axis);
                let solid_id = self.id();
                self.emit_entity(
                    solid_id,
                    format!("IFCBOOLEANRESULT({},#{a_id},#{b_id})", op.as_step_keyword()),
                );
                (solid_id, "CSG")
            }
            SolidShape::FacetedBrep {
                vertices_feet,
                triangles,
            } => {
                // One IfcCartesianPoint per vertex.
                let mut vert_ids: Vec<usize> = Vec::with_capacity(vertices_feet.len());
                for [x, y, z] in vertices_feet {
                    let (xm, ym, zm) = (x * 0.3048, y * 0.3048, z * 0.3048);
                    let pid = self.id();
                    self.emit_entity(pid, format!("IFCCARTESIANPOINT(({xm:.6},{ym:.6},{zm:.6}))"));
                    vert_ids.push(pid);
                }
                // One IfcPolyLoop + IfcFaceBound + IfcFace per triangle.
                // Triangles with out-of-range indices are skipped so
                // downstream parsers don't choke on a dangling ref;
                // debug builds panic instead so the caller learns
                // about bad mesh input during testing.
                let mut face_ids: Vec<usize> = Vec::new();
                for tri in triangles {
                    let idx_a = tri.0 as usize;
                    let idx_b = tri.1 as usize;
                    let idx_c = tri.2 as usize;
                    if idx_a >= vert_ids.len() || idx_b >= vert_ids.len() || idx_c >= vert_ids.len()
                    {
                        debug_assert!(
                            false,
                            "FacetedBrep triangle ({},{},{}) refers to vertex out of range (len={})",
                            tri.0,
                            tri.1,
                            tri.2,
                            vert_ids.len(),
                        );
                        continue;
                    }
                    let (va, vb, vc) = (vert_ids[idx_a], vert_ids[idx_b], vert_ids[idx_c]);
                    let loop_id = self.id();
                    self.emit_entity(loop_id, format!("IFCPOLYLOOP((#{va},#{vb},#{vc}))"));
                    let bound_id = self.id();
                    self.emit_entity(bound_id, format!("IFCFACEBOUND(#{loop_id},.T.)"));
                    let face_id = self.id();
                    self.emit_entity(face_id, format!("IFCFACE((#{bound_id}))"));
                    face_ids.push(face_id);
                }
                // IfcClosedShell wrapping all faces, then IfcFacetedBrep.
                let shell_id = self.id();
                let face_refs = face_ids
                    .iter()
                    .map(|id| format!("#{id}"))
                    .collect::<Vec<_>>()
                    .join(",");
                self.emit_entity(shell_id, format!("IFCCLOSEDSHELL(({face_refs}))"));
                let brep_id = self.id();
                self.emit_entity(
                    brep_id,
                    format!(
                        "IFCFACETEDBREP(#{brep_id_placeholder})",
                        brep_id_placeholder = shell_id
                    ),
                );
                (brep_id, "Brep")
            }
            SolidShape::TriangulatedFaceSet {
                vertices_feet,
                triangles,
            } => {
                let points = vertices_feet
                    .iter()
                    .map(|[x, y, z]| {
                        format!("({:.6},{:.6},{:.6})", x * 0.3048, y * 0.3048, z * 0.3048)
                    })
                    .collect::<Vec<_>>()
                    .join(",");
                let list_id = self.id();
                self.emit_entity(list_id, format!("IFCCARTESIANPOINTLIST3D(({points}))"));
                // `CoordIndex` is 1-based; triangles pointing outside the
                // list are left out rather than written dangling.
                let count = vertices_feet.len();
                let indices = triangles
                    .iter()
                    .filter(|t| [t.0, t.1, t.2].iter().all(|&i| (i as usize) < count))
                    .map(|t| format!("({},{},{})", t.0 + 1, t.1 + 1, t.2 + 1))
                    .collect::<Vec<_>>()
                    .join(",");
                let set_id = self.id();
                self.emit_entity(
                    set_id,
                    format!("IFCTRIANGULATEDFACESET(#{list_id},$,$,({indices}),$)"),
                );
                (set_id, "Tessellation")
            }
            SolidShape::SweptPath {
                profile,
                directrix_points_feet,
                fixed_reference,
            } => {
                // IFC-17: IfcFixedReferenceSweptAreaSolid. Profile
                // sweeps along the directrix polyline, orthogonal
                // at every sample, with `fixed_reference` providing
                // the "up" direction.

                // Directrix — one IfcCartesianPoint per vertex, then
                // an IfcPolyline referencing them.
                let mut dir_pt_ids: Vec<usize> = Vec::with_capacity(directrix_points_feet.len());
                for [x, y, z] in directrix_points_feet {
                    let (xm, ym, zm) = (x * 0.3048, y * 0.3048, z * 0.3048);
                    let pt = self.id();
                    self.emit_entity(pt, format!("IFCCARTESIANPOINT(({xm:.6},{ym:.6},{zm:.6}))"));
                    dir_pt_ids.push(pt);
                }
                let directrix = self.id();
                let pt_refs = dir_pt_ids
                    .iter()
                    .map(|id| format!("#{id}"))
                    .collect::<Vec<_>>()
                    .join(",");
                self.emit_entity(directrix, format!("IFCPOLYLINE(({pt_refs}))"));

                // Profile — same 2D placement pattern as
                // emit_profile_def. Wrap ProfileDef in a temporary
                // Extrusion to reuse the existing helper (its
                // dimensions aren't used since we override via
                // profile_override).
                let profile_origin = self.id();
                self.emit_entity(profile_origin, "IFCCARTESIANPOINT((0.,0.))");
                let profile_x_axis = self.id();
                self.emit_entity(profile_x_axis, "IFCDIRECTION((1.,0.))");
                let profile_placement = self.id();
                self.emit_entity(
                    profile_placement,
                    format!("IFCAXIS2PLACEMENT2D(#{profile_origin},#{profile_x_axis})"),
                );
                let wrap_ex = Extrusion {
                    width_feet: 0.0,
                    depth_feet: 0.0,
                    height_feet: 0.0,
                    profile_override: Some(profile.clone()),
                };
                let profile_id = self.emit_profile_def(&wrap_ex, profile_placement);

                // Fixed reference — normalise to unit length.
                let [fx, fy, fz] = *fixed_reference;
                let mag = (fx * fx + fy * fy + fz * fz).sqrt().max(1e-12);
                let (fx, fy, fz) = (fx / mag, fy / mag, fz / mag);
                let fixed_ref_id = self.id();
                self.emit_entity(
                    fixed_ref_id,
                    format!("IFCDIRECTION(({fx:.6},{fy:.6},{fz:.6}))"),
                );

                // IFCFIXEDREFERENCESWEPTAREASOLID(SweptArea, Position,
                //   Directrix, StartParam, EndParam, FixedReference).
                // StartParam / EndParam = $ means use the full
                // directrix from start to end — the IFC4 default.
                let solid_id = self.id();
                self.emit_entity(
                    solid_id,
                    format!(
                        "IFCFIXEDREFERENCESWEPTAREASOLID(#{profile_id},#{solid_position},#{directrix},$,$,#{fixed_ref_id})"
                    ),
                );
                (solid_id, "SweptSolid")
            }
            SolidShape::PlacedExtrusion {
                profile,
                origin_feet,
                axis,
                ref_direction,
                depth_feet,
            } => {
                // RE-52: IfcExtrudedAreaSolid with its own Position, so the
                // profile's plane is stated rather than implied.
                let profile_origin = self.id();
                self.emit_entity(profile_origin, "IFCCARTESIANPOINT((0.,0.))");
                let profile_x_axis = self.id();
                self.emit_entity(profile_x_axis, "IFCDIRECTION((1.,0.))");
                let profile_placement = self.id();
                self.emit_entity(
                    profile_placement,
                    format!("IFCAXIS2PLACEMENT2D(#{profile_origin},#{profile_x_axis})"),
                );
                let wrap_ex = Extrusion {
                    width_feet: 0.0,
                    depth_feet: 0.0,
                    height_feet: 0.0,
                    profile_override: Some(profile.clone()),
                };
                let profile_id = self.emit_profile_def(&wrap_ex, profile_placement);
                let unit = |v: [f64; 3]| {
                    let mag = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt().max(1e-12);
                    (v[0] / mag, v[1] / mag, v[2] / mag)
                };
                let [ox, oy, oz] = *origin_feet;
                let origin_id = self.id();
                self.emit_entity(
                    origin_id,
                    format!(
                        "IFCCARTESIANPOINT(({:.6},{:.6},{:.6}))",
                        ox * 0.3048,
                        oy * 0.3048,
                        oz * 0.3048
                    ),
                );
                let (ax, ay, az) = unit(*axis);
                let axis_id = self.id();
                self.emit_entity(axis_id, format!("IFCDIRECTION(({ax:.9},{ay:.9},{az:.9}))"));
                let (rx, ry, rz) = unit(*ref_direction);
                let ref_id = self.id();
                self.emit_entity(ref_id, format!("IFCDIRECTION(({rx:.9},{ry:.9},{rz:.9}))"));
                let position = self.id();
                self.emit_entity(
                    position,
                    format!("IFCAXIS2PLACEMENT3D(#{origin_id},#{axis_id},#{ref_id})"),
                );
                let depth = depth_feet * 0.3048;
                let solid_id = self.id();
                self.emit_entity(
                    solid_id,
                    format!("IFCEXTRUDEDAREASOLID(#{profile_id},#{position},#{z_axis},{depth:.6})"),
                );
                (solid_id, "SweptSolid")
            }
        }
    }

    /// The IfcUnitAssignment: the SI units every value is written in
    /// (#403). The writer converts Revit's feet, square and cubic feet and
    /// pounds on the way out and writes angles in radians, so these are
    /// the units a reader must apply. `model.units`, Revit's display units
    /// (IFC-40), describe the source model and stay out of it; the export
    /// diagnostics report them.
    fn emit_unit_assignment(&mut self) -> Vec<usize> {
        [
            ("LENGTHUNIT", "$", "METRE"),
            ("AREAUNIT", "$", "SQUARE_METRE"),
            ("VOLUMEUNIT", "$", "CUBIC_METRE"),
            ("PLANEANGLEUNIT", "$", "RADIAN"),
            ("MASSUNIT", ".KILO.", "GRAM"),
            ("TIMEUNIT", "$", "SECOND"),
        ]
        .into_iter()
        .map(|(unit_type, prefix, name)| {
            let id = self.id();
            self.emit_entity(id, format!("IFCSIUNIT(*,.{unit_type}.,{prefix},.{name}.)"));
            id
        })
        .collect()
    }

    fn emit_header(&mut self, model: &IfcModel) {
        let project = escape(model.project_name.as_deref().unwrap_or("Untitled"));
        let desc = escape(model.description.as_deref().unwrap_or(
            "Produced by rvt-rs (https://github.com/DrunkOnJava/rvt-rs) — \
                 clean-room Apache-2 Revit reader.",
        ));
        self.emit_line("ISO-10303-21;");
        self.emit_line("HEADER;");
        self.emit_line("FILE_DESCRIPTION(('ViewDefinition [CoordinationView]'),'2;1');");
        self.emit_line(format!(
            "FILE_NAME('{project}.ifc','{}',('rvt-rs'),('DrunkOnJava/rvt-rs'),'rvt-rs {version}','rvt-rs STEP writer','');",
            iso_timestamp_from(self.timestamp),
            version = env!("CARGO_PKG_VERSION"),
        ));
        self.emit_line("FILE_SCHEMA(('IFC4'));");
        self.emit_line("ENDSEC;");
        self.emit_line(format!("/* {desc} */"));
    }

    fn emit_data(&mut self, model: &IfcModel) {
        self.emit_line("DATA;");
        let document = model.global_ids.document.as_deref().unwrap_or("");
        let gid = |key: &[&str]| stable_guid(document, key);

        // Required framework entities (buildingSMART minimum viable).
        let person = self.id();
        self.emit_entity(person, "IFCPERSON($,$,'rvt-rs',$,$,$,$,$)");
        let org = self.id();
        self.emit_entity(
            org,
            "IFCORGANIZATION($,'rvt-rs','Clean-room Apache-2 Revit reader',$,$)",
        );
        let person_and_org = self.id();
        self.emit_entity(
            person_and_org,
            format!("IFCPERSONANDORGANIZATION(#{person},#{org},$)"),
        );
        let application = self.id();
        self.emit_entity(
            application,
            format!(
                "IFCAPPLICATION(#{org},'{}','rvt-rs','rvt_rs')",
                env!("CARGO_PKG_VERSION")
            ),
        );
        let owner_hist = self.id();
        self.emit_entity(
            owner_hist,
            format!(
                "IFCOWNERHISTORY(#{person_and_org},#{application},$,.ADDED.,$,#{person_and_org},#{application},{})",
                self.timestamp
            ),
        );

        // Unit assignment (IFC-40, #403).
        let unit_entity_ids = self.emit_unit_assignment();
        let unit_assignment = self.id();
        let unit_refs = unit_entity_ids
            .iter()
            .map(|id| format!("#{id}"))
            .collect::<Vec<_>>()
            .join(",");
        self.emit_entity(unit_assignment, format!("IFCUNITASSIGNMENT(({unit_refs}))"));

        // Representation context — needs IfcAxis2Placement3D +
        // IfcDirection + IfcCartesianPoint (origin, X, Z axes).
        let origin = self.id();
        self.emit_entity(origin, "IFCCARTESIANPOINT((0.,0.,0.))");
        let z_axis = self.id();
        self.emit_entity(z_axis, "IFCDIRECTION((0.,0.,1.))");
        let x_axis = self.id();
        self.emit_entity(x_axis, "IFCDIRECTION((1.,0.,0.))");
        let axis_placement = self.id();
        self.emit_entity(
            axis_placement,
            format!("IFCAXIS2PLACEMENT3D(#{origin},#{z_axis},#{x_axis})"),
        );
        let geom_ctx = self.id();
        self.emit_entity(
            geom_ctx,
            format!("IFCGEOMETRICREPRESENTATIONCONTEXT($,'Model',3,1.E-5,#{axis_placement},$)"),
        );

        // Root project.
        let project_name = escape(model.project_name.as_deref().unwrap_or("Untitled"));
        let project_desc = escape(model.description.as_deref().unwrap_or("Exported by rvt-rs"));
        let project_id = self.id();
        self.emit_entity(
            project_id,
            format!(
                "IFCPROJECT('{}',#{owner_hist},'{}',{},$,$,$,(#{geom_ctx}),#{unit_assignment})",
                gid(&["project"]),
                project_name,
                quoted_or_dollar(&project_desc),
            ),
        );

        // Spatial containment hierarchy — required by IFC4 for any
        // project with building content. We emit a minimal but valid
        // IfcSite → IfcBuilding → IfcBuildingStorey chain with
        // identity placements so downstream viewers (BlenderBIM,
        // IfcOpenShell-based tools, buildingSMART validator) render
        // the file directly without needing to synthesise a host
        // structure. Names default to "Default {Site,Building,Level
        // 1}"; once the walker surfaces site/level instances they'll
        // flow in here.
        //
        // Every IfcSpatialStructureElement needs its own
        // IfcLocalPlacement — we share the `axis_placement` across
        // the three (they're all identity), then chain the
        // placements via `PlacementRelTo` so the coordinate frames
        // compose correctly.
        let site_placement = self.id();
        self.emit_entity(
            site_placement,
            format!("IFCLOCALPLACEMENT($,#{axis_placement})"),
        );
        let site_id = self.id();
        self.emit_entity(
            site_id,
            format!(
                "IFCSITE('{}',#{owner_hist},'Default Site',$,$,#{site_placement},$,'Default Site',.ELEMENT.,$,$,$,$,$)",
                gid(&["site"]),
            ),
        );

        let building_placement = self.id();
        self.emit_entity(
            building_placement,
            format!("IFCLOCALPLACEMENT(#{site_placement},#{axis_placement})"),
        );
        let building_id = self.id();
        self.emit_entity(
            building_id,
            format!(
                "IFCBUILDING('{}',#{owner_hist},'Default Building',$,$,#{building_placement},$,'Default Building',.ELEMENT.,$,$,$)",
                gid(&["building"]),
            ),
        );

        // Emit one IfcBuildingStorey per Revit Level. Every storey gets
        // its own IfcLocalPlacement; their IDs are later bundled into a
        // single IfcRelAggregates bound to the building. A model with no
        // decoded Levels has no storey: IFC4 does not require one, and an
        // invented `Level 1` at elevation 0 is not the model's (RE-124).
        // Elements without a storey are contained in the building.
        let mut storey_ids: Vec<usize> = Vec::new();
        let mut storey_placements: Vec<usize> = Vec::new();
        let storeys = model.building_storeys.clone();
        let mut storey_gids: Vec<String> = Vec::with_capacity(storeys.len());
        let mut storey_name_counts: HashMap<&str, usize> = HashMap::new();
        for (storey_index, storey) in storeys.iter().enumerate() {
            let occurrence = storey_name_counts.entry(storey.name.as_str()).or_default();
            // RE-48: the Level's own GlobalId where the file yields it.
            let storey_gid = model
                .global_ids
                .storeys
                .get(&storey_index)
                .cloned()
                .unwrap_or_else(|| gid(&["storey", &storey.name, &occurrence.to_string()]));
            *occurrence += 1;
            let placement_id = self.id();
            self.emit_entity(
                placement_id,
                format!("IFCLOCALPLACEMENT(#{building_placement},#{axis_placement})"),
            );
            let id = self.id();
            // Convert feet → metres at emit boundary; IFC4 elevation
            // attribute is in the project's length unit (metres here).
            let elevation_m = storey.elevation_feet * 0.3048;
            let name_escaped = escape(&storey.name);
            self.emit_entity(
                id,
                format!(
                    // `Elevation` is an IfcLengthMeasure (REAL), so it
                    // needs a decimal point — `0` is an INTEGER literal
                    // and ISO-10303-21 does not admit it here (#214).
                    "IFCBUILDINGSTOREY('{storey_gid}',#{owner_hist},'{name_escaped}',$,$,#{placement_id},$,'{name_escaped}',.ELEMENT.,{elevation_m:.6})",
                ),
            );
            storey_gids.push(storey_gid);
            storey_ids.push(id);
            storey_placements.push(placement_id);
        }

        // Aggregation relationships — IfcRelAggregates is how the
        // spatial hierarchy binds in IFC4. Each level of the chain
        // gets one relationship pointing from parent to child.
        let rel_proj_site = self.id();
        self.emit_entity(
            rel_proj_site,
            format!(
                "IFCRELAGGREGATES('{}',#{owner_hist},$,$,#{project_id},(#{site_id}))",
                gid(&["aggregates", "project"]),
            ),
        );
        let rel_site_building = self.id();
        self.emit_entity(
            rel_site_building,
            format!(
                "IFCRELAGGREGATES('{}',#{owner_hist},$,$,#{site_id},(#{building_id}))",
                gid(&["aggregates", "site"]),
            ),
        );
        // IfcRelAggregates needs at least one related object, so a
        // building with no storey has no aggregation below it.
        if !storey_ids.is_empty() {
            let rel_building_storey = self.id();
            let storey_refs = storey_ids
                .iter()
                .map(|id| format!("#{id}"))
                .collect::<Vec<_>>()
                .join(",");
            self.emit_entity(
                rel_building_storey,
                format!(
                    "IFCRELAGGREGATES('{}',#{owner_hist},$,$,#{building_id},({storey_refs}))",
                    gid(&["aggregates", "building"]),
                ),
            );
        }

        // Classifications — one IfcClassification per source
        // (OmniClass, Uniformat, …), with one IfcClassificationReference
        // per coded item. Each classification gets its own
        // IfcRelAssociatesClassification tying its references back to
        // the project, which is how IFC4 consumers (BlenderBIM,
        // IfcOpenShell's classification viewer) discover code refs.
        //
        // RvtDocExporter populates `model.classifications` from
        // PartAtom's `<category term="...">` blocks. Previously those
        // codes were collected but never emitted; this wires them
        // through the STEP writer so downstream consumers can see
        // them directly.
        let mut classification_counts: HashMap<(&str, &str), usize> = HashMap::new();
        for classification in &model.classifications {
            let source_name = match &classification.source {
                super::entities::ClassificationSource::OmniClass => "OmniClass",
                super::entities::ClassificationSource::Uniformat => "Uniformat",
                super::entities::ClassificationSource::Other(s) => s.as_str(),
            };
            let source_name_escaped = escape(source_name);
            let edition = classification
                .edition
                .as_deref()
                .map(escape)
                .map(|e| format!("'{e}'"))
                .unwrap_or_else(|| "$".into());

            let classification_id = self.id();
            self.emit_entity(
                classification_id,
                format!("IFCCLASSIFICATION($,{edition},$,'{source_name_escaped}',$,$,$)"),
            );

            // One IfcClassificationReference per item; collect their
            // ids so we can bundle them into the IfcRelAssociatesClassification.
            let mut ref_ids: Vec<usize> = Vec::with_capacity(classification.items.len());
            let mut ref_gids: Vec<String> = Vec::with_capacity(classification.items.len());
            for item in &classification.items {
                let code_escaped = escape(&item.code);
                let name_str = item
                    .name
                    .as_deref()
                    .map(escape)
                    .map(|n| format!("'{n}'"))
                    .unwrap_or_else(|| "$".into());
                let ref_id = self.id();
                self.emit_entity(
                    ref_id,
                    format!(
                        // (Location, Identification, Name,
                        //  ReferencedSource, Description, Sort) — six
                        // attributes in IFC4 (#214).
                        "IFCCLASSIFICATIONREFERENCE($,'{code_escaped}',{name_str},#{classification_id},$,$)"
                    ),
                );
                ref_ids.push(ref_id);
                let occurrence = classification_counts
                    .entry((source_name, item.code.as_str()))
                    .or_default();
                ref_gids.push(gid(&[
                    "classification",
                    source_name,
                    &item.code,
                    &occurrence.to_string(),
                ]));
                *occurrence += 1;
            }

            if !ref_ids.is_empty() {
                let refs_list = ref_ids
                    .iter()
                    .map(|id| format!("#{id}"))
                    .collect::<Vec<_>>()
                    .join(",");
                // IfcRelAssociatesClassification binds a set of objects
                // to one classification reference. IFC4's schema
                // requires the RelatingClassification to be a single
                // IfcClassificationReferenceSelect; we pick the last
                // reference as the relating one and treat the rest as
                // project associations. If the project only has one
                // reference this is exact; when there are multiple,
                // each gets its own association relationship.
                for (ref_id, ref_gid) in ref_ids.iter().zip(&ref_gids) {
                    let rel_id = self.id();
                    self.emit_entity(
                        rel_id,
                        format!(
                            "IFCRELASSOCIATESCLASSIFICATION('{ref_gid}',#{owner_hist},$,$,(#{project_id}),#{ref_id})",
                        ),
                    );
                }
                // Silence the warning about an unused local when the
                // outer `for` loop only iterates once.
                let _ = refs_list;
            }
        }

        // BuildingElement emission — one `IFC<TYPE>` instance per decoded
        // Revit element (Wall, Floor, Roof, Ceiling, Door, Window, Column,
        // Beam…). Each element gets its own `IFCLOCALPLACEMENT` relative
        // to whichever storey contains it (index from
        // `BuildingElement.storey_index`; falls back to storey[0] when
        // unset). At the end, elements are grouped by storey and one
        // `IFCRELCONTAINEDINSPATIALSTRUCTURE` is emitted per non-empty
        // storey — which is how IFC4 tools (BlenderBIM, IfcOpenShell)
        // show "Floor 2 contains Wall-7, Wall-8" in the project
        // browser.
        //
        // Geometry (`IfcShapeRepresentation`) is intentionally omitted
        // here — tasks IFC-15 through IFC-22 produce proper
        // representations once Phase-5 geometry lands. For now every
        // element carries its placement + name + GUID, which validates
        // against the IFC4 schema as a "geometry-free" element.
        // Emit IfcMaterials upfront so BuildingElements can reference
        // them. Each material gets:
        //   - IFCMATERIAL with just a name (IFC4 minimum)
        //   - If the material has a color: IFCCOLOURRGB +
        //     IFCSURFACESTYLERENDERING + IFCSURFACESTYLE +
        //     IFCSTYLEDITEM to surface the color to IFC4 viewers
        // The color emission is gated because a color-less material
        // is valid IFC4 — we don't want to emit empty rendering
        // records when there's nothing to render.
        let mut material_ids: Vec<usize> = Vec::with_capacity(model.materials.len());
        for mat in &model.materials {
            let mat_id = self.id();
            let name_escaped = escape(&mat.name);
            self.emit_entity(mat_id, format!("IFCMATERIAL('{name_escaped}',$,$)"));
            if let Some(packed) = mat.color_packed {
                let r = (packed & 0xFF) as f64 / 255.0;
                let g = ((packed >> 8) & 0xFF) as f64 / 255.0;
                let b = ((packed >> 16) & 0xFF) as f64 / 255.0;
                let colour_id = self.id();
                self.emit_entity(colour_id, format!("IFCCOLOURRGB($,{r:.6},{g:.6},{b:.6})"));
                let rendering_id = self.id();
                let transparency = mat.transparency.unwrap_or(0.0);
                self.emit_entity(
                    rendering_id,
                    format!(
                        "IFCSURFACESTYLERENDERING(#{colour_id},{transparency:.6},$,$,$,$,$,$,.FLAT.)"
                    ),
                );
                let style_id = self.id();
                self.emit_entity(
                    style_id,
                    format!("IFCSURFACESTYLE('{name_escaped}',.BOTH.,(#{rendering_id}))"),
                );
                let presentation_id = self.id();
                self.emit_entity(
                    presentation_id,
                    format!("IFCPRESENTATIONSTYLEASSIGNMENT((#{style_id}))"),
                );
                let styled_item_id = self.id();
                self.emit_entity(
                    styled_item_id,
                    format!("IFCSTYLEDITEM($,(#{presentation_id}),'{name_escaped}')"),
                );
            }
            material_ids.push(mat_id);
        }

        // IFC-28: Emit IfcMaterialLayer + IfcMaterialLayerSet for
        // every compound assembly declared on the model. Layers
        // reference the `material_ids` vector above by index, so
        // layer_set emission MUST follow single-material emission.
        // Track layer-set ids for later IfcRelAssociatesMaterial
        // pairing.
        let mut layer_set_ids: Vec<usize> = Vec::with_capacity(model.material_layer_sets.len());
        for lset in &model.material_layer_sets {
            let mut layer_ids: Vec<usize> = Vec::with_capacity(lset.layers.len());
            for layer in &lset.layers {
                // A layer with no material writes `$` (IFC4 makes
                // IfcMaterialLayer.Material optional). An out-of-range
                // index gets the first material as a defensive fallback;
                // losing the layer would be worse.
                let material_slot = match layer.material_index {
                    None => "$".to_string(),
                    Some(index) => {
                        let mat_id = material_ids
                            .get(index)
                            .copied()
                            .or_else(|| material_ids.first().copied())
                            .unwrap_or(0);
                        if mat_id == 0 {
                            // No materials at all — skip this layer. IFC4
                            // allows IfcMaterialLayerSet with zero layers,
                            // so the set will still emit below, just empty.
                            continue;
                        }
                        format!("#{mat_id}")
                    }
                };
                let layer_id = self.id();
                // IfcMaterialLayer(Material, LayerThickness, IsVentilated=$,
                //                  Category=$, Priority=$, Name=$, Description=$)
                // Thickness in metres (feet × 0.3048).
                let thickness_m = layer.thickness_feet * 0.3048;
                let name_slot = match &layer.name {
                    Some(n) => format!("'{}'", escape(n)),
                    None => "$".into(),
                };
                self.emit_entity(
                    layer_id,
                    format!(
                        "IFCMATERIALLAYER({material_slot},{thickness_m:.6},$,$,$,{name_slot},$)"
                    ),
                );
                layer_ids.push(layer_id);
            }
            let set_id = self.id();
            let set_name = escape(&lset.name);
            let desc_slot = match &lset.description {
                Some(d) => format!("'{}'", escape(d)),
                None => "$".into(),
            };
            let layer_refs = if layer_ids.is_empty() {
                "()".to_string()
            } else {
                format!(
                    "({})",
                    layer_ids
                        .iter()
                        .map(|id| format!("#{id}"))
                        .collect::<Vec<_>>()
                        .join(",")
                )
            };
            // IfcMaterialLayerSet(MaterialLayers, LayerSetName, Description)
            self.emit_entity(
                set_id,
                format!("IFCMATERIALLAYERSET({layer_refs},'{set_name}',{desc_slot})"),
            );
            layer_set_ids.push(set_id);
        }

        // IFC-30: Emit IfcMaterialProfileSet for structural framing
        // (columns / beams) with named cross-sections. Each profile
        // carries a material index + profile name; at IfcMaterialProfile
        // emission time the profile name is also echoed as the
        // associated IfcProfileDef name — downstream tools that care
        // about precise cross-section geometry still need the profile
        // def itself (tracked separately in IFC-24).
        let mut profile_set_ids: Vec<usize> = Vec::with_capacity(model.material_profile_sets.len());
        for pset in &model.material_profile_sets {
            let mut profile_ids: Vec<usize> = Vec::with_capacity(pset.profiles.len());
            for profile in &pset.profiles {
                let mat_id = material_ids
                    .get(profile.material_index)
                    .copied()
                    .or_else(|| material_ids.first().copied())
                    .unwrap_or(0);
                if mat_id == 0 {
                    continue;
                }
                // The profile itself where the model carries it (RE-105);
                // otherwise a 1 x 1 m IfcRectangleProfileDef stand-in, as
                // IfcMaterialProfile requires a profile def reference.
                let profile_def_id = match &profile.profile {
                    Some(def) => {
                        let origin = self.id();
                        self.emit_entity(origin, "IFCCARTESIANPOINT((0.,0.))");
                        let x_axis = self.id();
                        self.emit_entity(x_axis, "IFCDIRECTION((1.,0.))");
                        let placement = self.id();
                        self.emit_entity(
                            placement,
                            format!("IFCAXIS2PLACEMENT2D(#{origin},#{x_axis})"),
                        );
                        let wrap = Extrusion {
                            width_feet: 0.0,
                            depth_feet: 0.0,
                            height_feet: 0.0,
                            profile_override: Some(def.clone()),
                        };
                        self.emit_profile_def(&wrap, placement)
                    }
                    None => {
                        let profile_def_id = self.id();
                        let profile_def_name = escape(&profile.profile_name);
                        self.emit_entity(
                            profile_def_id,
                            format!("IFCRECTANGLEPROFILEDEF(.AREA.,'{profile_def_name}',$,1.,1.)"),
                        );
                        profile_def_id
                    }
                };
                let profile_id = self.id();
                let profile_name = escape(&profile.profile_name);
                let desc_slot = match &profile.description {
                    Some(d) => format!("'{}'", escape(d)),
                    None => "$".into(),
                };
                // IfcMaterialProfile(Name, Description, Material, Profile,
                //                    Priority=$, Category=$)
                self.emit_entity(
                    profile_id,
                    format!(
                        "IFCMATERIALPROFILE('{profile_name}',{desc_slot},#{mat_id},#{profile_def_id},$,$)"
                    ),
                );
                profile_ids.push(profile_id);
            }
            let set_id = self.id();
            let set_name = escape(&pset.name);
            let desc_slot = match &pset.description {
                Some(d) => format!("'{}'", escape(d)),
                None => "$".into(),
            };
            let profile_refs = if profile_ids.is_empty() {
                "()".to_string()
            } else {
                format!(
                    "({})",
                    profile_ids
                        .iter()
                        .map(|id| format!("#{id}"))
                        .collect::<Vec<_>>()
                        .join(",")
                )
            };
            // IfcMaterialProfileSet(Name, Description, MaterialProfiles, CompositeProfile)
            self.emit_entity(
                set_id,
                format!("IFCMATERIALPROFILESET('{set_name}',{desc_slot},{profile_refs},$)"),
            );
            profile_set_ids.push(set_id);
        }

        // RE-82: family instances' material constituent sets, as Revit's
        // export writes them: unnamed, one constituent per material named
        // after it, in the category "Materials".
        let mut constituent_set_ids: Vec<Option<usize>> =
            Vec::with_capacity(model.material_constituent_sets.len());
        for cset in &model.material_constituent_sets {
            let mut constituent_ids = Vec::with_capacity(cset.material_indices.len());
            for (position, &material_index) in cset.material_indices.iter().enumerate() {
                let (Some(&mat_id), Some(material)) = (
                    material_ids.get(material_index),
                    model.materials.get(material_index),
                ) else {
                    continue;
                };
                let constituent_id = self.id();
                // IfcMaterialConstituent(Name, Description, Material,
                //   Fraction, Category)
                self.emit_entity(
                    constituent_id,
                    format!(
                        "IFCMATERIALCONSTITUENT('{}',$,#{mat_id},$,'Materials')",
                        escape(cset.names.get(position).unwrap_or(&material.name))
                    ),
                );
                constituent_ids.push(constituent_id);
            }
            if constituent_ids.is_empty() {
                constituent_set_ids.push(None);
                continue;
            }
            let set_id = self.id();
            // IfcMaterialConstituentSet(Name, Description, MaterialConstituents)
            self.emit_entity(
                set_id,
                format!(
                    "IFCMATERIALCONSTITUENTSET($,$,({}))",
                    constituent_ids
                        .iter()
                        .map(|id| format!("#{id}"))
                        .collect::<Vec<_>>()
                        .join(",")
                ),
            );
            constituent_set_ids.push(Some(set_id));
        }
        let constituent_set_of: HashMap<usize, usize> = model
            .material_constituent_sets
            .iter()
            .enumerate()
            .flat_map(|(set, cset)| cset.elements.iter().map(move |&entity| (entity, set)))
            .collect();
        let mut element_constituent_pairs: Vec<(usize, usize)> = Vec::new();

        // IFC-21: emit IfcRepresentationMap entities up-front. Each
        // map carries its shape representation (emitted once) and an
        // IFCAXIS2PLACEMENT3D mapping origin; instances reference
        // the map via IFCMAPPEDITEM in their own
        // IfcShapeRepresentation body. The emitted-map ID is stored
        // by index so the per-element loop can look it up from
        // `representation_map_index`.
        let mut representation_map_ids: Vec<usize> =
            Vec::with_capacity(model.representation_maps.len());
        for rmap in &model.representation_maps {
            // Mapping origin: per IFC4, usually identity (0,0,0)
            // with +X east / +Z up. Non-identity origins are rare —
            // they shift the shared shape relative to the mapped-item
            // transform.
            let [mx, my, mz] = rmap.origin_feet;
            let (mx, my, mz) = (mx * 0.3048, my * 0.3048, mz * 0.3048);
            let map_origin_pt = self.id();
            self.emit_entity(
                map_origin_pt,
                format!("IFCCARTESIANPOINT(({mx:.6},{my:.6},{mz:.6}))"),
            );
            // Reuse the project-level #z_axis / project-level #x_axis
            // directions — identity orientation is the common case.
            let map_placement = self.id();
            self.emit_entity(
                map_placement,
                format!("IFCAXIS2PLACEMENT3D(#{map_origin_pt},#{z_axis},#{x_axis})"),
            );
            // Emit the shared shape body. The emission helper returns
            // (solid_entity_id, rep_type_token). For the map's own
            // IfcShapeRepresentation we use the returned rep_type
            // (SweptSolid / CSG / Brep).
            //
            // Passing `map_placement` as the solid `Position` here is
            // not the #232 double-translation: a mapped item resolves
            // as `MappingTarget × MappingOrigin⁻¹ × geometry`, so a
            // Position equal to the MappingOrigin cancels against the
            // inverse rather than compounding. `origin_feet` is
            // `[0,0,0]` on every map rvt-rs builds today, which makes
            // the two placements the same identity frame anyway.
            let (solid_id, rep_type) = self.emit_solid_shape(&rmap.shape, map_placement, z_axis);
            let body_rep_id = self.id();
            self.emit_entity(
                body_rep_id,
                format!("IFCSHAPEREPRESENTATION(#{geom_ctx},'Body','{rep_type}',(#{solid_id}))"),
            );
            // IFCREPRESENTATIONMAP(MappingOrigin, MappedRepresentation).
            let rep_map_id = self.id();
            self.emit_entity(
                rep_map_id,
                format!("IFCREPRESENTATIONMAP(#{map_placement},#{body_rep_id})"),
            );
            representation_map_ids.push(rep_map_id);
        }

        let mut per_storey_elements: Vec<Vec<usize>> = vec![Vec::new(); storeys.len()];
        // #219: elements no storey join reached. They are contained in
        // the `IfcBuilding` rather than in whichever storey happens to
        // be first — "somewhere in this building" is the honest claim,
        // and putting them on a named storey would state a containment
        // nothing measured.
        let mut unplaced_elements: Vec<usize> = Vec::new();
        // Track (element_id, material_index) pairs so we can emit
        // IfcRelAssociatesMaterial per material after the element
        // loop completes.
        let mut element_material_pairs: Vec<(usize, usize)> = Vec::new();
        // Track (element_id, &PropertySet) so property-set emission
        // runs after all element IDs are assigned.
        let mut element_property_sets: Vec<(usize, &super::entities::PropertySet)> = Vec::new();
        // Map vec index in model.entities → emitted IFC element id.
        // None when that entity wasn't a BuildingElement. Consulted
        // when resolving `host_element_index` for openings + for
        // IfcRelVoidsElement / IfcRelFillsElement emission.
        let mut entity_index_to_el_id: Vec<Option<usize>> = vec![None; model.entities.len()];
        // Each building element's IfcLocalPlacement, for the openings placed
        // in their host's frame (RE-151).
        let mut entity_index_to_placement: Vec<Option<usize>> = vec![None; model.entities.len()];
        // Void/fill tracking: for each element with a host, note
        // (host_el_id, opening_el_id, element_el_id) so the rels can
        // be emitted after the element loop. Openings themselves are
        // also BuildingElements (IfcOpeningElement) but they're never
        // contained in a storey — IFC4 treats them as "virtual"
        // elements that only live through IfcRelVoidsElement.
        let mut void_fill_triples: Vec<(usize, usize, Option<usize>, String)> = Vec::new();
        // #323: parts of an aggregate whose whole is a building element
        // reach the spatial structure through that whole, so they are left
        // out of the storey containment below.
        let is_building_element = |index: usize| {
            matches!(
                model.entities.get(index),
                Some(super::entities::IfcEntity::BuildingElement { .. })
            )
        };
        let aggregate_parts: std::collections::BTreeSet<usize> = model
            .entities
            .iter()
            .filter_map(|entity| match entity {
                super::entities::IfcEntity::Aggregate { whole, parts }
                    if is_building_element(*whole) =>
                {
                    Some(
                        parts
                            .iter()
                            .copied()
                            .filter(|part| *part != *whole && is_building_element(*part)),
                    )
                }
                _ => None,
            })
            .flatten()
            .collect();
        // #400: each building element's GlobalId, fixed from what the
        // element is rather than where it lands in the file. Revit's own
        // where the file yields it (RE-48); otherwise the element's
        // ElementId (its `Tag`) and which of the entities sharing that Tag
        // it is — a slab's further pieces (#331) or a curtain wall's parts
        // share their element's. An element with no Tag falls back to its
        // entity type and name. Everything the element owns (its opening,
        // property sets, material association) derives from this one.
        let mut element_gids: Vec<Option<String>> = vec![None; model.entities.len()];
        let mut element_counts: HashMap<(&str, &str, &str), usize> = HashMap::new();
        for (entity_idx, entity) in model.entities.iter().enumerate() {
            let super::entities::IfcEntity::BuildingElement {
                ifc_type,
                name,
                type_guid,
                ..
            } = entity
            else {
                continue;
            };
            let key = match type_guid.as_deref() {
                Some(tag) => ("element", tag, ""),
                None => ("untagged-element", ifc_type.as_str(), name.as_str()),
            };
            let occurrence = element_counts.entry(key).or_default();
            let element_gid = model
                .global_ids
                .elements
                .get(&entity_idx)
                .cloned()
                .unwrap_or_else(|| gid(&[key.0, key.1, key.2, &occurrence.to_string()]));
            *occurrence += 1;
            element_gids[entity_idx] = Some(element_gid);
        }
        let mut el_id_to_gid: HashMap<usize, &str> = HashMap::new();
        for (entity_idx, entity) in model.entities.iter().enumerate() {
            if let super::entities::IfcEntity::BuildingElement {
                ifc_type,
                name,
                type_guid,
                predefined_type,
                storey_index,
                material_index,
                property_set,
                location_feet,
                rotation_radians,
                extrusion,
                host_element_index,
                material_layer_set_index,
                material_profile_set_index,
                solid_shape,
                representation_map_index,
            } = entity
            {
                // An out-of-range index is a caller bug; treat it as
                // unbound rather than silently dropping the element or
                // pinning it to an unrelated storey.
                let idx = storey_index.filter(|index| *index < storeys.len());
                let placement_parent = match idx {
                    Some(index) => storey_placements[index],
                    None => building_placement,
                };
                // Decide whether to emit a per-element axis placement
                // (with real origin + rotation) or share the project-
                // level identity placement. Sharing keeps byte counts
                // down for elements where no location has been decoded;
                // unique placements matter once geometry attaches to
                // the element and tools read element.position.
                let element_axis = if let Some([x_ft, y_ft, z_ft]) = location_feet {
                    let x_m = x_ft * 0.3048;
                    let y_m = y_ft * 0.3048;
                    let z_m = z_ft * 0.3048;
                    let point_id = self.id();
                    self.emit_entity(
                        point_id,
                        format!("IFCCARTESIANPOINT(({x_m:.6},{y_m:.6},{z_m:.6}))"),
                    );
                    // Rotation is applied about Z only — all Revit
                    // element placements we've seen so far are upright
                    // with yaw-only rotation. Full 3D rotation needs
                    // the BasePoint / ProjectPosition transform chain
                    // (already decoded, not yet threaded here).
                    let x_axis_id = if let Some(angle) = rotation_radians {
                        let cx = angle.cos();
                        let cy = angle.sin();
                        let dir = self.id();
                        self.emit_entity(dir, format!("IFCDIRECTION(({cx:.6},{cy:.6},0.))"));
                        Some(dir)
                    } else {
                        None
                    };
                    let axis_id = self.id();
                    // If we have a yaw rotation, reference our own X-
                    // axis IfcDirection; otherwise reuse the shared
                    // +X axis_placement points at (via `x_axis`).
                    let axis_body = match x_axis_id {
                        Some(d) => format!("IFCAXIS2PLACEMENT3D(#{point_id},#{z_axis},#{d})"),
                        None => format!("IFCAXIS2PLACEMENT3D(#{point_id},#{z_axis},#{x_axis})"),
                    };
                    self.emit_entity(axis_id, axis_body);
                    axis_id
                } else {
                    axis_placement
                };
                let placement_id = self.id();
                self.emit_entity(
                    placement_id,
                    format!("IFCLOCALPLACEMENT(#{placement_parent},#{element_axis})"),
                );
                // The element translation is carried exactly once, by
                // the `IfcLocalPlacement` above. Every swept solid this
                // element emits is authored in the element-local frame
                // (profiles are centred on their own origin), so its
                // `Position` is the project-level identity placement —
                // reusing `element_axis` there made a conforming
                // consumer composing `ObjectPlacement × Position` apply
                // the translation twice (#232). Revit's own exporter
                // splits it the same way: on
                // `2024_Core_Interior_slim.ifc` the `IfcWall`
                // `SweptSolid` bodies sit at `Position = (0,0,0)` with
                // the wall's `IfcLocalPlacement` carrying the location,
                // and the `IfcSlab` / `IfcSpace` bodies do the mirror
                // image — an identity placement with the frame in
                // `Position`. Never both.
                let solid_position = axis_placement;

                // Emit the extrusion chain when geometry is present.
                // Chain: IfcProfileDef subclass → IfcExtrudedAreaSolid
                // → IfcShapeRepresentation → IfcProductDefinitionShape.
                // Profile placement uses a single fresh 2D axis per
                // element (profile-local XY frame centred on origin).
                // Precedence (highest wins):
                //   1. representation_map_index (IFC-21 mapped item)
                //   2. solid_shape              (IFC-18/19/20 solid)
                //   3. extrusion                (IFC-16 extruded area)
                //   4. none                     (Representation = $)
                let shape_ref = if let Some(rm_idx) = representation_map_index {
                    representation_map_ids.get(*rm_idx).map(|rep_map_id| {
                        // Mapped item: the instance's own
                        // transformation operator (identity for now —
                        // the element's own IfcLocalPlacement carries
                        // the instance position, and the mapped item
                        // rides on top of that).
                        let tx_op_origin = self.id();
                        self.emit_entity(
                            tx_op_origin,
                            "IFCCARTESIANPOINT((0.,0.,0.))",
                        );
                        // IFC4 uses IfcCartesianTransformationOperator3D
                        // for the mapped-target transform. Identity =
                        // ($,$,ORIGIN,$,$) per spec (Axis1/Axis2 left
                        // null, scale defaults to 1.0, Axis3 null).
                        let tx_op = self.id();
                        self.emit_entity(
                            tx_op,
                            format!(
                                "IFCCARTESIANTRANSFORMATIONOPERATOR3D($,$,#{tx_op_origin},$,$)"
                            ),
                        );
                        let mapped_item = self.id();
                        self.emit_entity(
                            mapped_item,
                            format!("IFCMAPPEDITEM(#{rep_map_id},#{tx_op})"),
                        );
                        // The instance's IfcShapeRepresentation wraps
                        // the mapped item with representation-type
                        // 'MappedRepresentation' (IFC4 convention for
                        // type-instance shared geometry).
                        let rep_id = self.id();
                        self.emit_entity(
                            rep_id,
                            format!(
                                "IFCSHAPEREPRESENTATION(#{geom_ctx},'Body','MappedRepresentation',(#{mapped_item}))"
                            ),
                        );
                        let prod_shape_id = self.id();
                        self.emit_entity(
                            prod_shape_id,
                            format!("IFCPRODUCTDEFINITIONSHAPE($,$,(#{rep_id}))"),
                        );
                        prod_shape_id
                    })
                } else if let Some(shape) = solid_shape {
                    let (solid_id, rep_type) = self.emit_solid_shape(shape, solid_position, z_axis);
                    let rep_id = self.id();
                    self.emit_entity(
                        rep_id,
                        format!(
                            "IFCSHAPEREPRESENTATION(#{geom_ctx},'Body','{rep_type}',(#{solid_id}))"
                        ),
                    );
                    let prod_shape_id = self.id();
                    self.emit_entity(
                        prod_shape_id,
                        format!("IFCPRODUCTDEFINITIONSHAPE($,$,(#{rep_id}))"),
                    );
                    Some(prod_shape_id)
                } else if let Some(ex) = extrusion {
                    let depth = ex.height_feet * 0.3048;
                    // IfcProfileDef has a 2D placement; we emit a
                    // fresh 2D origin + direction + 2D axis per
                    // extrusion. Sharing a single 2D placement across
                    // all extrusions would be possible but muddies
                    // byte-by-byte diff tooling, so pay the ~3-entity
                    // cost for clarity.
                    // RE-84: an opening standing for a door or window takes
                    // the same cut to its host's depth as a filled one (#227).
                    let own_cut = (ifc_type == "IFCOPENINGELEMENT")
                        .then(|| model.opening_cuts.get(&entity_idx))
                        .flatten();
                    let [cx, cy] = own_cut.map_or([0.0, 0.0], |c| c.centre_feet);
                    let profile_origin = self.id();
                    self.emit_entity(
                        profile_origin,
                        format!("IFCCARTESIANPOINT(({:.6},{:.6}))", cx * 0.3048, cy * 0.3048),
                    );
                    let profile_x_axis = self.id();
                    self.emit_entity(profile_x_axis, "IFCDIRECTION((1.,0.))");
                    let profile_placement = self.id();
                    self.emit_entity(
                        profile_placement,
                        format!("IFCAXIS2PLACEMENT2D(#{profile_origin},#{profile_x_axis})"),
                    );
                    let profile_id = match own_cut {
                        Some(cut) => {
                            let profile_id = self.id();
                            self.emit_entity(
                                profile_id,
                                format!(
                                    "IFCRECTANGLEPROFILEDEF(.AREA.,$,#{profile_placement},{:.6},{:.6})",
                                    cut.x_dim_feet * 0.3048,
                                    cut.y_dim_feet * 0.3048,
                                ),
                            );
                            profile_id
                        }
                        None => self.emit_profile_def(ex, profile_placement),
                    };
                    // Solid-local placement: the identity axis, so the
                    // extrusion sits at the element origin and the
                    // element's own IfcLocalPlacement moves it into
                    // the world exactly once (#232).
                    let (position, depth) =
                        self.opening_solid_position(solid_position, own_cut, depth);
                    let solid_id = self.id();
                    self.emit_entity(
                        solid_id,
                        format!(
                            "IFCEXTRUDEDAREASOLID(#{profile_id},#{position},#{z_axis},{depth:.6})"
                        ),
                    );
                    // The representation groups the solid inside the
                    // project's 3D geometric context (#geom_ctx) with
                    // the IFC4-standard identifier + type for a
                    // swept-solid body.
                    let rep_id = self.id();
                    self.emit_entity(
                        rep_id,
                        format!(
                            "IFCSHAPEREPRESENTATION(#{geom_ctx},'Body','SweptSolid',(#{solid_id}))"
                        ),
                    );
                    let prod_shape_id = self.id();
                    self.emit_entity(
                        prod_shape_id,
                        format!("IFCPRODUCTDEFINITIONSHAPE($,$,(#{rep_id}))"),
                    );
                    Some(prod_shape_id)
                } else {
                    None
                };

                let el_id = self.id();
                // A room's own text property, when recovered and not blank.
                let room_text = |key: &str| {
                    property_set.as_ref().and_then(|set| {
                        set.properties
                            .iter()
                            .find(|p| p.name == key)
                            .and_then(|p| match &p.value {
                                super::entities::PropertyValue::Text(text)
                                    if !text.trim().is_empty() =>
                                {
                                    Some(quoted_or_dollar(&escape(text)))
                                }
                                _ => None,
                            })
                    })
                };
                let is_space = ifc_type.eq_ignore_ascii_case("IFCSPACE");
                // An `IfcSpace` is named by its room number and described
                // by its room name, as Revit's own exporter writes it
                // (RE-117): `Name` the number, `LongName` the name. A room
                // with no number keeps the `Room-<ElementId>` Name, and one
                // with no name has no LongName.
                let name_quoted = is_space
                    .then(|| room_text(super::export_content::ROOM_NUMBER_PROPERTY))
                    .flatten()
                    .unwrap_or_else(|| quoted_or_dollar(&escape(name)));
                let long_name_quoted = room_text(super::export_content::ROOM_NAME_PROPERTY)
                    .unwrap_or_else(|| "$".into());
                // `ObjectType` is `Family:Type` when RE-38 named the
                // element's family and type, as in Revit's own export.
                let property_text = |key: &str| {
                    property_set.as_ref().and_then(|set| {
                        set.properties.iter().find_map(|p| match &p.value {
                            super::entities::PropertyValue::Text(text) if p.name == key => {
                                Some(text.clone())
                            }
                            _ => None,
                        })
                    })
                };
                let object_type_quoted = match (
                    property_text(super::export_content::FAMILY_NAME_PROPERTY),
                    property_text(super::export_content::TYPE_NAME_PROPERTY),
                ) {
                    (Some(family), Some(type_name)) => {
                        quoted_or_dollar(&escape(&format!("{family}:{type_name}")))
                    }
                    _ => "$".into(),
                };
                let tag_quoted = type_guid
                    .as_deref()
                    .map(escape)
                    .map(|t| format!("'{t}'"))
                    .unwrap_or_else(|| "$".into());
                let rep_slot = match shape_ref {
                    Some(id) => format!("#{id}"),
                    None => "$".into(),
                };
                // Every IFC4 instance carries every attribute its
                // entity type declares — an unknown optional is `$`,
                // never an omitted slot (#214). The shared head is
                // IfcProduct's:
                //   (GlobalId, OwnerHist, Name, Desc, ObjectType,
                //    ObjectPlacement, Representation)
                // and `element_attribute_tail` supplies the rest for
                // the type, so a reader can index PredefinedType
                // instead of hitting "index out of range".
                let ifc_upper = ifc_type.to_ascii_uppercase();
                // RE-93, RE-94: a window's or door's OverallHeight and
                // OverallWidth are its opening's, which Revit's export writes
                // there.
                let property_length = |key: &str| {
                    property_set.as_ref().and_then(|set| {
                        set.properties.iter().find_map(|p| match &p.value {
                            super::entities::PropertyValue::PositiveLengthFeet(value)
                                if p.name == key =>
                            {
                                Some(*value)
                            }
                            _ => None,
                        })
                    })
                };
                let [.., width_key, height_key] = super::export_content::FILLER_OPENING_PROPERTIES;
                let overall = property_length(height_key).zip(property_length(width_key));
                let tail = element_attribute_tail(
                    &ifc_upper,
                    &tag_quoted,
                    predefined_type.as_deref(),
                    &long_name_quoted,
                    overall,
                );
                let global_id: &str = element_gids[entity_idx]
                    .as_deref()
                    .expect("every building element has a GlobalId");
                el_id_to_gid.insert(el_id, global_id);
                let line = format!(
                    "{ifc_upper}('{global_id}',#{owner_hist},{name_quoted},$,{object_type_quoted},#{placement_id},{rep_slot},{tail})",
                );
                self.emit_entity(el_id, line);
                // RE-84: an opening standing for a door or window that draws
                // no geometry voids its host and fills nothing. Like every
                // opening it lives through that relation, not in a storey.
                let hosted_opening =
                    ifc_upper == "IFCOPENINGELEMENT" && host_element_index.is_some();
                if let (true, Some(h_idx)) = (hosted_opening, host_element_index) {
                    void_fill_triples.push((*h_idx, el_id, None, global_id.to_string()));
                } else if !aggregate_parts.contains(&entity_idx) {
                    match idx {
                        Some(index) => per_storey_elements[index].push(el_id),
                        None => unplaced_elements.push(el_id),
                    }
                }
                entity_index_to_el_id[entity_idx] = Some(el_id);
                entity_index_to_placement[entity_idx] = Some(placement_id);
                // IFC-30 / IFC-28: precedence order for material
                // association is profile_set > layer_set > single
                // material. Try each in order; `profile_or_layer_applied`
                // carries the "already emitted" flag through so the
                // single-material path below doesn't double-associate.
                let profile_set_applied = if let Some(ps_idx) = material_profile_set_index {
                    if let Some(&ps_id) = profile_set_ids.get(*ps_idx) {
                        let usage_id = self.id();
                        // IfcMaterialProfileSetUsage(ForProfileSet,
                        //   CardinalPoint=5, ReferenceExtent=$). In IFC4's
                        // IfcCardinalPointReference, 5 is the mid-depth
                        // centre: the profile is centred on the member's
                        // axis, as every profile the exporter writes is.
                        self.emit_entity(
                            usage_id,
                            format!("IFCMATERIALPROFILESETUSAGE(#{ps_id},5,$)"),
                        );
                        let rel_id = self.id();
                        self.emit_entity(
                            rel_id,
                            format!(
                                "IFCRELASSOCIATESMATERIAL('{}',#{owner_hist},$,$,(#{el_id}),#{usage_id})",
                                gid(&["material-association", global_id]),
                            ),
                        );
                        true
                    } else {
                        false
                    }
                } else {
                    false
                };
                // IFC-28: material_layer_set_index falls through when
                // profile-set didn't apply. Emits IfcMaterialLayerSetUsage
                // + IfcRelAssociatesMaterial.
                let layer_set_applied = !profile_set_applied
                    && if let Some(ls_idx) = material_layer_set_index {
                        if let Some(&ls_id) = layer_set_ids.get(*ls_idx) {
                            let usage_id = self.id();
                            // IfcMaterialLayerSetUsage(ForLayerSet, LayerSetDirection,
                            //   DirectionSense, OffsetFromReferenceLine, ReferenceExtent).
                            // The element's own usage when the model states
                            // one (RE-58); otherwise .AXIS2. (the extrusion's
                            // Y axis, a wall's thickness), .POSITIVE. from 0,
                            // the IFC4 defaults most exporters emit.
                            // ReferenceExtent stays `$` — undecoded, but its
                            // slot is still written (#214).
                            let usage = model.material_layer_usages.get(&entity_idx);
                            let direction = match usage.map(|u| u.direction) {
                                Some(super::entities::LayerSetDirection::Axis1) => ".AXIS1.",
                                Some(super::entities::LayerSetDirection::Axis3) => ".AXIS3.",
                                _ => ".AXIS2.",
                            };
                            let sense = if usage.is_none_or(|u| u.positive) {
                                ".POSITIVE."
                            } else {
                                ".NEGATIVE."
                            };
                            let offset = usage.map_or_else(
                                || "0.".to_string(),
                                |u| format!("{:.6}", u.offset_feet * 0.3048),
                            );
                            self.emit_entity(
                                usage_id,
                                format!(
                                    "IFCMATERIALLAYERSETUSAGE(#{ls_id},{direction},{sense},{offset},$)"
                                ),
                            );
                            let rel_id = self.id();
                            self.emit_entity(
                            rel_id,
                            format!(
                                "IFCRELASSOCIATESMATERIAL('{}',#{owner_hist},$,$,(#{el_id}),#{usage_id})",
                                gid(&["material-association", global_id]),
                            ),
                        );
                            true
                        } else {
                            false
                        }
                    } else {
                        false
                    };
                if !profile_set_applied && !layer_set_applied {
                    if let Some(m_idx) = material_index {
                        if *m_idx < material_ids.len() {
                            element_material_pairs.push((el_id, *m_idx));
                        }
                    } else if let Some(&set) = constituent_set_of.get(&entity_idx) {
                        element_constituent_pairs.push((el_id, set));
                    }
                }
                if let Some(pset) = property_set {
                    if !pset.properties.is_empty() {
                        element_property_sets.push((el_id, pset));
                    }
                }
                // Void / fill chain: when the element has both an
                // extrusion (describes its volume) and a host_element_
                // index (points at its parent wall/floor), emit an
                // IfcOpeningElement matching the shape + the two
                // binding relationships.
                if let (false, Some(h_idx), Some(ex)) =
                    (hosted_opening, host_element_index, extrusion)
                {
                    {
                        // Emit a second extrusion chain — same shape
                        // as the element, placed on the element's
                        // placement so the void sits where the door
                        // sits. We reuse the element's placement for
                        // simplicity; IFC4 validators accept this. The
                        // solid's own `Position` stays identity for the
                        // same reason it does on the filling element
                        // (#232) — the opening's IfcLocalPlacement
                        // below already carries `element_axis`.
                        // #227: cut to the host wall's thickness where it is
                        // known, else the element's own body.
                        let cut = model.opening_cuts.get(&entity_idx);
                        let x_dim = cut.map_or(ex.width_feet, |c| c.x_dim_feet) * 0.3048;
                        let y_dim = cut.map_or(ex.depth_feet, |c| c.y_dim_feet) * 0.3048;
                        let [cx, cy] = cut.map_or([0.0, 0.0], |c| c.centre_feet);
                        let depth_m = ex.height_feet * 0.3048;
                        let o_profile_origin = self.id();
                        self.emit_entity(
                            o_profile_origin,
                            format!("IFCCARTESIANPOINT(({:.6},{:.6}))", cx * 0.3048, cy * 0.3048),
                        );
                        let o_profile_x = self.id();
                        self.emit_entity(o_profile_x, "IFCDIRECTION((1.,0.))");
                        let o_profile_place = self.id();
                        self.emit_entity(
                            o_profile_place,
                            format!("IFCAXIS2PLACEMENT2D(#{o_profile_origin},#{o_profile_x})"),
                        );
                        let o_profile = self.id();
                        self.emit_entity(
                            o_profile,
                            format!(
                                "IFCRECTANGLEPROFILEDEF(.AREA.,$,#{o_profile_place},{x_dim:.6},{y_dim:.6})"
                            ),
                        );
                        let (position, depth_m) =
                            self.opening_solid_position(solid_position, cut, depth_m);
                        let o_solid = self.id();
                        self.emit_entity(
                            o_solid,
                            format!(
                                "IFCEXTRUDEDAREASOLID(#{o_profile},#{position},#{z_axis},{depth_m:.6})"
                            ),
                        );
                        let o_rep = self.id();
                        self.emit_entity(
                            o_rep,
                            format!(
                                "IFCSHAPEREPRESENTATION(#{geom_ctx},'Body','SweptSolid',(#{o_solid}))"
                            ),
                        );
                        let o_prod_shape = self.id();
                        self.emit_entity(
                            o_prod_shape,
                            format!("IFCPRODUCTDEFINITIONSHAPE($,$,(#{o_rep}))"),
                        );
                        // IfcOpeningElement uses its own placement
                        // (relative to the host wall's) — we reuse the
                        // door/window placement.
                        let o_placement = self.id();
                        self.emit_entity(
                            o_placement,
                            format!("IFCLOCALPLACEMENT(#{placement_parent},#{element_axis})"),
                        );
                        let opening_id = self.id();
                        // #400: an opening exists only because the element
                        // filling it does, so its identity is that element's.
                        let opening_gid = gid(&["opening", global_id]);
                        // IfcOpeningElement: (GlobalId, OwnerHist, Name,
                        //   Desc, ObjectType, Placement, Rep, Tag,
                        //   PredefinedType). IFC4 adds PredefinedType.
                        //
                        // `Tag` repeats the filling element's own tag.
                        // That is what Revit's exporter does — on
                        // `2024_Core_Interior_slim.ifc` all 138
                        // door/window openings carry the Tag of the
                        // element that fills them — and it invents
                        // nothing: the opening exists only because
                        // that element does.
                        self.emit_entity(
                            opening_id,
                            format!(
                                "IFCOPENINGELEMENT('{opening_gid}',#{owner_hist},'Opening for {name_esc}',$,$,#{o_placement},#{o_prod_shape},{tag_quoted},.OPENING.)",
                                name_esc = escape(name),
                            ),
                        );
                        void_fill_triples.push((*h_idx, opening_id, Some(el_id), opening_gid));
                    }
                }
            }
        }

        // RE-151 (#227): an opening nothing fills, a hole in a floor's sketch.
        // As in Revit's export it is placed in its host's frame, its outline
        // extruded up through the host, and it voids the host without being
        // contained in a storey.
        for entity in &model.entities {
            let super::entities::IfcEntity::VoidOpening {
                host,
                tag,
                name,
                outline_feet,
                depth_feet,
            } = entity
            else {
                continue;
            };
            let (Some(Some(host_el_id)), Some(Some(host_placement))) = (
                entity_index_to_el_id.get(*host),
                entity_index_to_placement.get(*host),
            ) else {
                continue;
            };
            let profile_origin = self.id();
            self.emit_entity(profile_origin, "IFCCARTESIANPOINT((0.,0.))");
            let profile_x_axis = self.id();
            self.emit_entity(profile_x_axis, "IFCDIRECTION((1.,0.))");
            let profile_placement = self.id();
            self.emit_entity(
                profile_placement,
                format!("IFCAXIS2PLACEMENT2D(#{profile_origin},#{profile_x_axis})"),
            );
            let body = Extrusion {
                width_feet: 0.0,
                depth_feet: 0.0,
                height_feet: *depth_feet,
                profile_override: Some(super::entities::ProfileDef::ArbitraryClosed {
                    points: outline_feet.clone(),
                }),
            };
            let profile_id = self.emit_profile_def(&body, profile_placement);
            let solid_id = self.id();
            self.emit_entity(
                solid_id,
                format!(
                    "IFCEXTRUDEDAREASOLID(#{profile_id},#{axis_placement},#{z_axis},{:.6})",
                    depth_feet * 0.3048
                ),
            );
            let rep_id = self.id();
            self.emit_entity(
                rep_id,
                format!("IFCSHAPEREPRESENTATION(#{geom_ctx},'Body','SweptSolid',(#{solid_id}))"),
            );
            let prod_shape_id = self.id();
            self.emit_entity(
                prod_shape_id,
                format!("IFCPRODUCTDEFINITIONSHAPE($,$,(#{rep_id}))"),
            );
            // Its own identity axis, so the solid's Position is never also the
            // placement's (#232).
            let opening_axis = self.id();
            self.emit_entity(opening_axis, format!("IFCAXIS2PLACEMENT3D(#{origin},$,$)"));
            let placement_id = self.id();
            self.emit_entity(
                placement_id,
                format!("IFCLOCALPLACEMENT(#{host_placement},#{opening_axis})"),
            );
            let host_gid = el_id_to_gid.get(host_el_id).copied().unwrap_or_default();
            let opening_gid = gid(&["void-opening", host_gid, tag]);
            let opening_id = self.id();
            self.emit_entity(
                opening_id,
                format!(
                    "IFCOPENINGELEMENT('{opening_gid}',#{owner_hist},{},$,$,#{placement_id},#{prod_shape_id},'{}',.OPENING.)",
                    quoted_or_dollar(&escape(name)),
                    escape(tag),
                ),
            );
            void_fill_triples.push((*host, opening_id, None, opening_gid));
        }

        // IfcRelVoidsElement + IfcRelFillsElement — for each
        // (host, opening, element) triple we collected during
        // element emission, emit:
        //   - IFCRELVOIDSELEMENT(host_wall, opening)
        //   - IFCRELFILLSELEMENT(opening, door/window)
        // Together these tell IFC4 viewers "subtract this opening's
        // volume from the host wall, and fill the hole with this
        // door/window."
        //
        // The host is resolved here, after the element loop, rather
        // than inside it: `entity_index_to_el_id` is only complete
        // once every BuildingElement has been emitted, so resolving
        // inline silently dropped the chain whenever a host happened
        // to sit after its opening in `model.entities`.
        for (host_entity_idx, opening_id, el_id, opening_gid) in &void_fill_triples {
            let Some(host_el_id) = entity_index_to_el_id
                .get(*host_entity_idx)
                .and_then(|slot| *slot)
            else {
                continue;
            };
            let voids_rel = self.id();
            self.emit_entity(
                voids_rel,
                format!(
                    "IFCRELVOIDSELEMENT('{}',#{owner_hist},$,$,#{host_el_id},#{opening_id})",
                    gid(&["voids", opening_gid]),
                ),
            );
            let Some(el_id) = el_id else {
                continue;
            };
            let fills_rel = self.id();
            self.emit_entity(
                fills_rel,
                format!(
                    "IFCRELFILLSELEMENT('{}',#{owner_hist},$,$,#{opening_id},#{el_id})",
                    gid(&["fills", opening_gid]),
                ),
            );
        }

        // #323: one IfcRelAggregates per aggregate, whole to parts. Resolved
        // after the element loop for the same reason as the void chain.
        for entity in &model.entities {
            let super::entities::IfcEntity::Aggregate { whole, parts } = entity else {
                continue;
            };
            if !is_building_element(*whole) {
                continue;
            }
            let Some(whole_el_id) = entity_index_to_el_id.get(*whole).and_then(|slot| *slot) else {
                continue;
            };
            let part_refs: Vec<String> = parts
                .iter()
                .filter(|part| aggregate_parts.contains(part))
                .filter_map(|part| entity_index_to_el_id.get(*part).and_then(|slot| *slot))
                .map(|id| format!("#{id}"))
                .collect();
            if part_refs.is_empty() {
                continue;
            }
            let rel_id = self.id();
            self.emit_entity(
                rel_id,
                format!(
                    "IFCRELAGGREGATES('{}',#{owner_hist},$,$,#{whole_el_id},({}))",
                    gid(&["aggregates", el_id_to_gid[&whole_el_id]]),
                    part_refs.join(","),
                ),
            );
        }

        // RE-138 (#528): the joins of ducts and pipes. A connector is an
        // `IfcDistributionPort` named after its element and index, with no
        // flow direction (the file's lists carry none), tied to its element
        // by an `IfcRelConnectsPortToElement`; a join is an
        // `IfcRelConnectsPorts` between two of them. A port is shared by every
        // join it is in.
        let mut ports: HashMap<(u32, u32), usize> = HashMap::new();
        for entity in &model.entities {
            let super::entities::IfcEntity::PortConnection {
                a,
                a_id,
                a_index,
                b,
                b_id,
                b_index,
            } = entity
            else {
                continue;
            };
            let (Some(a_el), Some(b_el)) = (
                entity_index_to_el_id.get(*a).and_then(|slot| *slot),
                entity_index_to_el_id.get(*b).and_then(|slot| *slot),
            ) else {
                continue;
            };
            let ends = [(a_el, *a_id, *a_index), (b_el, *b_id, *b_index)];
            let mut port_ids = [0usize; 2];
            for (slot, &(element, id, index)) in ends.iter().enumerate() {
                if let Some(&port) = ports.get(&(id, index)) {
                    port_ids[slot] = port;
                    continue;
                }
                let (id_text, index_text) = (id.to_string(), index.to_string());
                let port = self.id();
                self.emit_entity(
                    port,
                    format!(
                        "IFCDISTRIBUTIONPORT('{}',#{owner_hist},'Port_{id}_{index}',$,$,$,$,.NOTDEFINED.,$,$)",
                        gid(&["port", &id_text, &index_text]),
                    ),
                );
                let to_element = self.id();
                self.emit_entity(
                    to_element,
                    format!(
                        "IFCRELCONNECTSPORTTOELEMENT('{}',#{owner_hist},$,$,#{port},#{element})",
                        gid(&["port_to_element", &id_text, &index_text]),
                    ),
                );
                ports.insert((id, index), port);
                port_ids[slot] = port;
            }
            let rel = self.id();
            self.emit_entity(
                rel,
                format!(
                    "IFCRELCONNECTSPORTS('{}',#{owner_hist},$,$,#{},#{},$)",
                    gid(&[
                        "ports",
                        &a_id.to_string(),
                        &a_index.to_string(),
                        &b_id.to_string(),
                        &b_index.to_string(),
                    ]),
                    port_ids[0],
                    port_ids[1],
                ),
            );
        }

        // RE-47: further sets of an element, such as Pset_StairCommon.
        for entity in &model.entities {
            let super::entities::IfcEntity::ElementPropertySet { element, set } = entity else {
                continue;
            };
            if !is_building_element(*element) || set.properties.is_empty() {
                continue;
            }
            if let Some(el_id) = entity_index_to_el_id.get(*element).and_then(|slot| *slot) {
                element_property_sets.push((el_id, set));
            }
        }

        // IfcPropertySet emission — one set per element that ships
        // properties. Each property becomes an
        // IfcPropertySingleValue, the set wraps them, then an
        // IfcRelDefinesByProperties links the set to its element.
        let mut pset_counts: HashMap<(usize, &str), usize> = HashMap::new();
        for (el_id, pset) in &element_property_sets {
            let occurrence = pset_counts.entry((*el_id, pset.name.as_str())).or_default();
            let set_key = [
                el_id_to_gid[el_id],
                pset.name.as_str(),
                &occurrence.to_string(),
            ]
            .join("\u{1f}");
            *occurrence += 1;
            let mut prop_ids: Vec<usize> = Vec::with_capacity(pset.properties.len());
            for prop in &pset.properties {
                let p_id = self.id();
                let name_esc = escape(&prop.name);
                let value_step = prop.value.to_step();
                self.emit_entity(
                    p_id,
                    format!("IFCPROPERTYSINGLEVALUE('{name_esc}',$,{value_step},$)"),
                );
                prop_ids.push(p_id);
            }
            let refs = prop_ids
                .iter()
                .map(|id| format!("#{id}"))
                .collect::<Vec<_>>()
                .join(",");
            let set_id = self.id();
            let set_name = escape(&pset.name);
            self.emit_entity(
                set_id,
                format!(
                    "IFCPROPERTYSET('{}',#{owner_hist},'{set_name}',$,({refs}))",
                    gid(&["property-set", &set_key])
                ),
            );
            let rel_id = self.id();
            self.emit_entity(
                rel_id,
                format!(
                    "IFCRELDEFINESBYPROPERTIES('{}',#{owner_hist},$,$,(#{el_id}),#{set_id})",
                    gid(&["defines-by-properties", &set_key])
                ),
            );
        }

        // Bucket element_id lists by material_index so each material
        // gets one IfcRelAssociatesMaterial (rather than N, where N
        // is the number of elements using it).
        let mut by_material: Vec<Vec<usize>> = vec![Vec::new(); material_ids.len()];
        // RE-82: one association per constituent set, keyed by its
        // materials' names so its GlobalId does not depend on file order.
        let mut by_constituent_set: Vec<Vec<usize>> =
            vec![Vec::new(); model.material_constituent_sets.len()];
        for (el_id, set) in &element_constituent_pairs {
            by_constituent_set[*set].push(*el_id);
        }
        for (set, elements) in by_constituent_set.iter().enumerate() {
            let Some(Some(set_id)) = constituent_set_ids.get(set) else {
                continue;
            };
            if elements.is_empty() {
                continue;
            }
            let names: Vec<&str> = model.material_constituent_sets[set]
                .material_indices
                .iter()
                .filter_map(|&index| model.materials.get(index).map(|m| m.name.as_str()))
                .collect();
            let mut key = vec!["material-constituents"];
            key.extend(names);
            let rel_id = self.id();
            self.emit_entity(
                rel_id,
                format!(
                    "IFCRELASSOCIATESMATERIAL('{}',#{owner_hist},$,$,({}),#{set_id})",
                    gid(&key),
                    elements
                        .iter()
                        .map(|id| format!("#{id}"))
                        .collect::<Vec<_>>()
                        .join(","),
                ),
            );
        }

        for (el_id, m_idx) in &element_material_pairs {
            by_material[*m_idx].push(*el_id);
        }
        let mut material_name_counts: HashMap<&str, usize> = HashMap::new();
        for (m_idx, elements) in by_material.iter().enumerate() {
            let material_name = model.materials[m_idx].name.as_str();
            let occurrence = material_name_counts.entry(material_name).or_default();
            let material_key = occurrence.to_string();
            *occurrence += 1;
            if elements.is_empty() {
                continue;
            }
            let rel_id = self.id();
            let refs_list = elements
                .iter()
                .map(|id| format!("#{id}"))
                .collect::<Vec<_>>()
                .join(",");
            self.emit_entity(
                rel_id,
                format!(
                    "IFCRELASSOCIATESMATERIAL('{}',#{owner_hist},$,$,({refs_list}),#{})",
                    gid(&["material", material_name, &material_key]),
                    material_ids[m_idx],
                ),
            );
        }

        // RE-110: every element named `Family:Type` whose type is read is
        // typed by one IFC type object per type, as in Revit's own export:
        // named `Family:Type`, its `Tag` the type's ElementId and its
        // GlobalId Revit's.
        struct TypeGroup {
            type_id: u32,
            entity: String,
            name: String,
            tail: String,
            elements: Vec<usize>,
        }
        let mut type_groups: Vec<TypeGroup> = Vec::new();
        for (entity_idx, entity) in model.entities.iter().enumerate() {
            let super::entities::IfcEntity::BuildingElement {
                ifc_type,
                type_guid,
                predefined_type,
                property_set,
                ..
            } = entity
            else {
                continue;
            };
            let (Some(el_id), Some(type_id)) = (
                entity_index_to_el_id[entity_idx],
                type_guid
                    .as_deref()
                    .and_then(|tag| tag.parse::<u32>().ok())
                    .and_then(|id| model.element_type_ids.get(&id)),
            ) else {
                continue;
            };
            let property_text = |key: &str| {
                property_set.as_ref().and_then(|set| {
                    set.properties.iter().find_map(|p| match &p.value {
                        super::entities::PropertyValue::Text(text) if p.name == key => {
                            Some(text.clone())
                        }
                        _ => None,
                    })
                })
            };
            let (Some(family), Some(type_name)) = (
                property_text(super::export_content::FAMILY_NAME_PROPERTY),
                property_text(super::export_content::TYPE_NAME_PROPERTY),
            ) else {
                continue;
            };
            let Some((entity_name, tail)) =
                type_entity_for(&ifc_type.to_ascii_uppercase(), predefined_type.as_deref())
            else {
                continue;
            };
            match type_groups
                .iter_mut()
                .find(|group| group.type_id == *type_id && group.entity == entity_name)
            {
                Some(group) => group.elements.push(el_id),
                None => type_groups.push(TypeGroup {
                    type_id: *type_id,
                    entity: entity_name,
                    name: format!("{family}:{type_name}"),
                    tail,
                    elements: vec![el_id],
                }),
            }
        }
        let mut typed_ids: std::collections::BTreeSet<u32> = Default::default();
        for TypeGroup {
            type_id,
            entity: entity_name,
            name,
            tail,
            elements,
        } in &type_groups
        {
            let tag = type_id.to_string();
            // A type split across two entities keeps Revit's GlobalId once.
            let type_gid = match model.global_ids.types.get(type_id) {
                Some(global_id) if typed_ids.insert(*type_id) => global_id.clone(),
                _ => gid(&["type", entity_name, &tag]),
            };
            let type_el_id = self.id();
            self.emit_entity(
                type_el_id,
                format!(
                    "{entity_name}('{type_gid}',#{owner_hist},{},$,$,$,$,'{tag}',$,{tail})",
                    quoted_or_dollar(&escape(name)),
                ),
            );
            let rel_id = self.id();
            self.emit_entity(
                rel_id,
                format!(
                    "IFCRELDEFINESBYTYPE('{}',#{owner_hist},$,$,({}),#{type_el_id})",
                    gid(&["defines-by-type", &type_gid]),
                    elements
                        .iter()
                        .map(|id| format!("#{id}"))
                        .collect::<Vec<_>>()
                        .join(","),
                ),
            );
        }

        for (idx, element_ids) in per_storey_elements
            .iter()
            .enumerate()
            .map(|(idx, ids)| (Some(idx), ids))
            .chain(std::iter::once((None, &unplaced_elements)))
        {
            if element_ids.is_empty() {
                continue;
            }
            let (target_storey, container_gid) = match idx {
                Some(index) => (storey_ids[index], gid(&["contains", &storey_gids[index]])),
                None => (building_id, gid(&["contains", "building"])),
            };
            let rel_id = self.id();
            let refs_list = element_ids
                .iter()
                .map(|id| format!("#{id}"))
                .collect::<Vec<_>>()
                .join(",");
            self.emit_entity(
                rel_id,
                format!(
                    "IFCRELCONTAINEDINSPATIALSTRUCTURE('{container_gid}',#{owner_hist},$,$,({refs_list}),#{target_storey})",
                ),
            );
        }
        self.emit_line("ENDSEC;");
    }

    fn finish(self) -> String {
        let mut out = self.out;
        out.push_str("END-ISO-10303-21;\n");
        out
    }
}

/// STEP-style string escape per ISO-10303-21:
///
/// - Literal apostrophe → `''` (doubled).
/// - Literal backslash → `\\`.
/// - ASCII printable (0x20–0x7E) → pass through.
/// - ASCII control (0x00–0x1F, 0x7F) → `\X\<HH>` (2-hex-digit byte).
/// - Non-ASCII code points:
///   - BMP (≤ U+FFFF): `\X2\<HHHH>\X0\`
///   - Supplementary plane (> U+FFFF): `\X4\<HHHHHHHH>\X0\`
///
/// Previous implementation replaced non-ASCII with underscore, which
/// silently mangled accented project names, CJK text, and any Unicode
/// symbols in Revit metadata. Real RVT files routinely have
/// non-ASCII in title, path, and taxonomy strings; this escape
/// preserves them round-trip per the STEP spec.
///
/// Two consecutive non-ASCII chars produce separate `\X2\<HHHH>\X0\`
/// sequences rather than a concatenated run. The spec allows either
/// form; separate sequences keep the encoder stateless and the
/// output diff-friendly.
pub(crate) fn escape(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for c in input.chars() {
        match c {
            '\'' => out.push_str("''"),
            '\\' => out.push_str("\\\\"),
            c if c.is_ascii() && !c.is_control() => out.push(c),
            c if c.is_ascii() => {
                // ASCII control byte.
                out.push_str(&format!("\\X\\{:02X}", c as u32));
            }
            c if (c as u32) <= 0xFFFF => {
                // BMP non-ASCII.
                out.push_str(&format!("\\X2\\{:04X}\\X0\\", c as u32));
            }
            c => {
                // Supplementary plane (emoji, rare scripts).
                out.push_str(&format!("\\X4\\{:08X}\\X0\\", c as u32));
            }
        }
    }
    out
}

fn quoted_or_dollar(s: &str) -> String {
    if s.is_empty() {
        "$".into()
    } else {
        format!("'{s}'")
    }
}

fn iso_timestamp_from(secs: i64) -> String {
    // Format a Unix epoch as ISO 8601. Avoids chrono to stay
    // dep-lean. Pure function — deterministic given its input.
    let (y, m, d, hh, mm, ss) = epoch_to_ymdhms(secs);
    format!("{y:04}-{m:02}-{d:02}T{hh:02}:{mm:02}:{ss:02}")
}

/// Wall-clock Unix seconds for the header stamps. `std::time::SystemTime`
/// panics on `wasm32-unknown-unknown` ("time not implemented on this
/// platform"), which made the browser viewer's Export IFC fail on every
/// click; the wasm build reads the JavaScript host clock instead.
fn unix_seconds() -> i64 {
    #[cfg(all(target_arch = "wasm32", feature = "wasm"))]
    {
        (crate::wasm::host_now_millis() / 1000.0) as i64
    }
    #[cfg(not(all(target_arch = "wasm32", feature = "wasm")))]
    {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0)
    }
}

/// Gregorian breakdown without chrono. Good from 1970 through 2400.
fn epoch_to_ymdhms(secs: i64) -> (i32, u32, u32, u32, u32, u32) {
    let days = secs.div_euclid(86_400);
    let remainder = secs.rem_euclid(86_400) as u32;
    let hh = remainder / 3600;
    let mm = (remainder % 3600) / 60;
    let ss = remainder % 60;

    // Days since 1970-01-01 → Gregorian date. Algorithm from Howard
    // Hinnant's date.h (public domain).
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = (yoe as i64) + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = (if mp < 10 { mp + 3 } else { mp - 9 }) as u32;
    let y = (y + if m <= 2 { 1 } else { 0 }) as i32;
    (y, m, d, hh, mm, ss)
}

/// The GlobalId of an entity Revit gives none (#400): SHA-256 over a fixed
/// namespace, the document's identity and `key` (the entity's role and the
/// identities it derives from), as an RFC 9562 version-8 UUID in IFC's
/// 22-character form. It depends on nothing about the entity's position in
/// the file, so it survives changes elsewhere in the model, and the
/// namespace keeps it apart from GlobalIds other tools derive.
fn stable_guid(document: &str, key: &[&str]) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(b"rvt-rs ifc GlobalId v1\0");
    hasher.update(document.as_bytes());
    for part in key {
        hasher.update([0x1f]);
        hasher.update(part.as_bytes());
    }
    let digest = hasher.finalize();
    let mut value = [0u8; 16];
    value.copy_from_slice(&digest[..16]);
    value[6] = (value[6] & 0x0f) | 0x80;
    value[8] = (value[8] & 0x3f) | 0x80;
    crate::revit_global_ids::compress_ifc_guid(value)
}
