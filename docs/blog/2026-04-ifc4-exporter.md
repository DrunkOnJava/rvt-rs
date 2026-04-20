# Shipping the rvt-rs IFC4 exporter

*Griffin Long — 2026-04-20*

`rvt-rs` reads Revit files. That was never the finish line — reading is only
useful if the graph that comes out can be handed to something downstream. The
obvious target is IFC4, the open BIM exchange schema published by
buildingSMART. This post is the walk from "decoded element structs live in
memory" to "a `.ifc` file that opens cleanly in BlenderBIM with real 3D
geometry, materials, openings, and property sets attached to the right
entities."

Everything below is real code from the repo — `src/ifc/from_decoded.rs`,
`src/ifc/step_writer.rs`, `src/ifc/entities.rs`. The integration test
`tests/ifc_synthetic_project.rs` builds a synthetic 20ft × 10ft one-room
building, writes it to `tests/fixtures/synthetic-project.ifc`, and asserts
the entity counts and byte patterns that this post describes.

## The shape of the problem

IFC4 is serialised as ISO-10303-21, the STEP text format. Every entity is a
numbered line:

```step
#15=IFCGEOMETRICREPRESENTATIONCONTEXT($,'Model',3,1.E-5,#14,$);
#16=IFCPROJECT('0rvtrs000000000000000G',#5,'Synthetic Test Project', ... );
#17=IFCLOCALPLACEMENT($,#14);
```

The `#N=` is the entity id. Inside the constructor, `#M` is a reference to
another entity, `$` means "attribute omitted," `*` means "derived," and all
type names are `UPPERCASE_WITH_UNDERSCORES`. The file starts with
`ISO-10303-21;` / `HEADER;` / `FILE_SCHEMA(('IFC4'));` and ends with
`END-ISO-10303-21;`.

A valid IFC4 project needs at minimum: owner-history plumbing (person,
organisation, application), a unit assignment, a geometric-representation
context, an `IfcProject`, an `IfcSite`, an `IfcBuilding`, one or more
`IfcBuildingStorey`s, the aggregation relationships that tie them together,
and then the actual elements with their placements, geometry, materials, and
property sets. One missing cross-reference and the file won't open.

There is no library dependency here. The emitter is string-based,
`#![deny(unsafe_code)]`-clean, and deterministic when you pass a fixed
timestamp via `StepOptions`. Tests pin byte-for-byte stability.

## The bridge: `ElementInput`

The walker in `src/elements/` produces typed views — `Wall`, `Door`,
`FloorType`, `Stair`, `Material` — but the STEP writer doesn't want to know
about any of them individually. Instead, `src/ifc/from_decoded.rs` defines a
flat bridge struct that every decoded element collapses into:

```rust
pub struct ElementInput<'a> {
    pub decoded: &'a DecodedElement,
    pub display_name: String,
    pub guid: Option<String>,
    pub storey_index: Option<usize>,
    pub material_index: Option<usize>,
    pub property_set: Option<PropertySet>,
    pub location_feet: Option<[f64; 3]>,
    pub rotation_radians: Option<f64>,
    pub extrusion: Option<Extrusion>,
    pub host_element_index: Option<usize>,
}
```

The writer consumes a `&[ElementInput<'_>]` plus a `BuilderOptions` (project
name, storeys, materials, classifications, units) and emits one
`IfcEntity::BuildingElement` per input. Class-name mapping
(`"Wall"` → `IFCWALL`, `"Floor"` → `IFCSLAB`, …) happens through
`category_map::lookup`; unknown classes fall back to `IFCBUILDINGELEMENTPROXY`
so an unrecognised Autodesk extension round-trips rather than being silently
dropped.

The `BuildingElement` variant in `src/ifc/entities.rs` is the stored form —
same fields as `ElementInput`, plus serde derives so a model can be
round-tripped through JSON for diffing. `location_feet`, `rotation_radians`,
and `extrusion` are all `Option<_>` because a geometry-free element is valid
IFC4, and a lot of decoded-but-unplaced elements in early pipelines won't
carry real coordinates yet.

## Geometry: the `IfcExtrudedAreaSolid` chain

The IFC4 idiom for a prismatic solid is five linked entities per element:

1. `IfcCartesianPoint` + `IfcDirection` + `IfcAxis2Placement2D` — the 2D
   frame the profile lives in.
2. `IfcRectangleProfileDef` — a width × depth rectangle in that frame.
3. `IfcExtrudedAreaSolid` — the profile swept along +Z for a given depth.
4. `IfcShapeRepresentation` — groups the solid inside the project's
   `IfcGeometricRepresentationContext` with the IFC4-standard
   `('Body','SweptSolid')` identifier tuple.
5. `IfcProductDefinitionShape` — the `Representation` slot on the element
   points here.

The writer emits that whole chain for every element that ships an
`Extrusion`. Unit conversion happens at the emit boundary — Revit's internal
feet go in, metres come out:

```rust
let x_dim = ex.width_feet * 0.3048;
let y_dim = ex.depth_feet * 0.3048;
let depth = ex.height_feet * 0.3048;
// ...
self.emit_entity(
    profile_id,
    format!(
        "IFCRECTANGLEPROFILEDEF(.AREA.,$,#{profile_placement},{x_dim:.6},{y_dim:.6})"
    ),
);
let solid_id = self.id();
self.emit_entity(
    solid_id,
    format!(
        "IFCEXTRUDEDAREASOLID(#{profile_id},#{element_axis},#{z_axis},{depth:.6})"
    ),
);
```

A 20ft × 8" wall ends up as `IFCRECTANGLEPROFILEDEF(.AREA.,$,#60,6.096000,0.203200)`
extruded 3.048 m. Those three numbers show up verbatim in the fixture; the
integration test pins them as an assertion so a bad ft → m cast would break
the build.

Helper functions in `from_decoded.rs` derive the `Extrusion` from the
relevant typed view — `wall_extrusion(&Wall, Option<&WallType>, length_ft)`,
`slab_extrusion(length_ft, width_ft, Option<&FloorType>)`,
`roof_extrusion`, `ceiling_extrusion`, `column_extrusion`. Each has
sensible fallbacks (8" wall thickness, 12" slab thickness, 1" ceiling
thickness) so a partial decode still produces plausible geometry.

## Openings: the host-element trick

A door in a wall needs a hole in the wall. IFC4 models this as three
entities bound by two relationships:

- An `IfcOpeningElement` with its own extrusion that matches the door's
  volume.
- `IfcRelVoidsElement(wall → opening)` — subtracts the opening from the host
  wall's body.
- `IfcRelFillsElement(opening → door)` — fills the hole with the door.

The `ElementInput` plumbs this through a single `host_element_index: Option<usize>`
field that points at the host's position in the inputs slice. When the
writer sees an element with both an `extrusion` and a `host_element_index`,
it:

1. Emits a second extrusion chain using the same rectangle profile and
   depth — that becomes the opening volume.
2. Emits the `IfcOpeningElement` with the opening's own `IfcLocalPlacement`.
3. Collects `(host_el_id, opening_el_id, element_el_id)` into a
   `void_fill_triples` vec.
4. After the element loop, walks the triples and emits the two relationships:

```rust
for (host_el_id, opening_id, el_id) in &void_fill_triples {
    let voids_rel = self.id();
    self.emit_entity(
        voids_rel,
        format!(
            "IFCRELVOIDSELEMENT('{}',#{owner_hist},$,$,#{host_el_id},#{opening_id})",
            make_guid(voids_rel),
        ),
    );
    let fills_rel = self.id();
    self.emit_entity(
        fills_rel,
        format!(
            "IFCRELFILLSELEMENT('{}',#{owner_hist},$,$,#{opening_id},#{el_id})",
            make_guid(fills_rel),
        ),
    );
}
```

The two-pass structure is deliberate. Host elements have to land in the
STEP output before any opening that references them, because STEP allows
forward references in principle but IFC consumers choke on back-references
to entity ids that haven't been assigned yet. `entity_index_to_el_id` maps
the logical input-vec index to the actual `#N` id once the host has been
emitted, and the triple-vec defers rel emission until after the pass.

In the synthetic fixture, the front door's `host_element_index = Some(1)`
points at the south wall. The resulting STEP shows:

```step
#122=IFCOPENINGELEMENT('0rvtrs000000000000001w',#5,'Opening for Front Entry Door', ...);
#131=IFCRELVOIDSELEMENT('...',#5,$,$,#65,#122);
#132=IFCRELFILLSELEMENT('...',#5,$,$,#122,#113);
```

`#65` is the south wall, `#113` the door, `#122` the opening — three
entities, two relationships, one visible hole in BlenderBIM's viewer.

## Materials

Materials follow the same indirect-through-styles pattern. A `MaterialInfo`
in `BuilderOptions.materials` carries a name, an optional packed RGB colour,
and an optional transparency. Per material, the writer emits:

```step
#30=IFCMATERIAL('Concrete',$,$);
#31=IFCCOLOURRGB($,0.666667,0.666667,0.666667);
#32=IFCSURFACESTYLERENDERING(#31,0.000000,$,$,$,$,$,$,.FLAT.);
#33=IFCSURFACESTYLE('Concrete',.BOTH.,(#32));
#34=IFCPRESENTATIONSTYLEASSIGNMENT((#33));
#35=IFCSTYLEDITEM($,(#34),'Concrete');
```

The colour chain is gated — colourless materials skip it, since a bare
`IFCMATERIAL` is valid IFC4 and we don't want to emit empty rendering
records.

Elements then cite the material by index. Rather than one
`IfcRelAssociatesMaterial` per element, the writer buckets
`(element_id, material_index)` pairs by material and emits one rel per
material bundling all its users:

```step
#144=IFCRELASSOCIATESMATERIAL('...',#5,$,$,(#53,#65,#77,#89,#101,#113,#128),#30);
#145=IFCRELASSOCIATESMATERIAL('...',#5,$,$,(#124,#126),#36);
```

`#30` is Concrete, tied to seven elements (four walls, slab, door, stair).
`#36` is Glass - Tinted, tied to two windows. That's exactly the minimum
`IfcRelAssociates*` emission — one rel per material — that IFC4 viewers
prefer.

## Property sets

Revit parameters land on IFC entities as `IfcPropertySet` →
`IfcPropertySingleValue` → `IfcRelDefinesByProperties`. The `PropertyValue`
enum in `entities.rs` enumerates the IFC4 `IfcValue` subtypes we actually
surface:

```rust
pub enum PropertyValue {
    Text(String),            // IfcText
    Integer(i64),            // IfcInteger
    Real(f64),               // IfcReal
    Boolean(bool),           // IfcBoolean
    LengthFeet(f64),         // IfcLengthMeasure (ft → m at emit)
    AngleRadians(f64),       // IfcPlaneAngleMeasure
}
```

`PropertyValue::to_step()` hard-codes the per-variant serialisation:
`IFCTEXT('...')` with STEP apostrophe escaping, `IFCBOOLEAN(.T.)` /
`IFCBOOLEAN(.F.)`, `IFCLENGTHMEASURE(0.762000)` for a 2.5ft sill after
conversion, `IFCPLANEANGLEMEASURE(0.000000)`. The typed enum is the one
place where unit conversion happens, which keeps the ft→m policy
centralised.

Helper builders in `from_decoded.rs` — `wall_property_set`,
`door_property_set`, `window_property_set`, `stair_property_set` — build
`Pset_WallCommon` / `Pset_DoorCommon` / `Pset_WindowCommon` /
`Pset_StairCommon` sets directly from the decoded typed views, skipping
fields that weren't decoded. A North Wall with `structural_usage = Bearing`
and `unconnected_height_feet = 10.0` emits:

```step
#133=IFCPROPERTYSINGLEVALUE('BaseOffset',$,IFCLENGTHMEASURE(0.000000),$);
#134=IFCPROPERTYSINGLEVALUE('TopOffset',$,IFCLENGTHMEASURE(0.000000),$);
#135=IFCPROPERTYSINGLEVALUE('UnconnectedHeight',$,IFCLENGTHMEASURE(3.048000),$);
#136=IFCPROPERTYSINGLEVALUE('StructuralUsage',$,IFCTEXT('Bearing'),$);
#137=IFCPROPERTYSINGLEVALUE('LocationLine',$,IFCTEXT('WallCenterline'),$);
#138=IFCPROPERTYSET('...',#5,'Pset_WallCommon',$,(#133,#134,#135,#136,#137));
#139=IFCRELDEFINESBYPROPERTIES('...',#5,$,$,(#53),#138);
```

## Validation

`tests/ifc_synthetic_project.rs` pins the entire chain without touching any
corpus. It synthesises ten `DecodedElement` values (four walls, slab, door,
two windows, stair, plus an unknown-class fallback), wires typed
`Wall` / `WallType` / `Window` views for the property-set helpers, builds
the model, writes STEP, and asserts:

- Header & footer sentinels, `FILE_SCHEMA(('IFC4'))`.
- 1 site, 1 building, 3 storeys; elevations 0 / 3.048 / 6.096 m.
- 4 `IFCWALL`, 1 `IFCSLAB`, 1 `IFCDOOR`, 2 `IFCWINDOW`, 1 `IFCSTAIR`,
  1 `IFCBUILDINGELEMENTPROXY`.
- 7 rectangle profiles, 7 extruded solids, 7 shape reps (6 elements + 1
  opening clone of the door).
- 1 `IFCOPENINGELEMENT`, 1 `IFCRELVOIDSELEMENT`, 1 `IFCRELFILLSELEMENT`.
- 2 `IFCMATERIAL`, 2 `IFCSURFACESTYLE`, 2 `IFCRELASSOCIATESMATERIAL`.
- 2 `IFCPROPERTYSET` + 2 `IFCRELDEFINESBYPROPERTIES`.
- 2 `IFCRELCONTAINEDINSPATIALSTRUCTURE` (ground floor + second floor — the
  east wall was deliberately assigned `storey_index: Some(1)` to exercise
  multi-storey containment).
- Specific byte patterns for the profile dimensions (`6.096000,0.203200`
  for a 20ft × 8" wall, `3.048000,0.203200` for a 10ft × 8" wall,
  `6.096000,3.048000` for the 20ft × 10ft slab) and placement coordinates
  (`(0.000000,3.048000,0.000000)` for the north wall at y = 10ft).

A second test asserts byte-for-byte stability under a fixed timestamp so
regression diffs stay tractable.

Setting `DUMP_IFC=1 cargo test synthetic_project_emits_valid_ifc4` dumps the
output to `tests/fixtures/synthetic-project.ifc` — a 157-line IFC4 file
that opens directly in BlenderBIM. A separate, independent sanity check:
running `ifcopenshell` over that fixture in Python produces the same object
hierarchy (project → site → building → three storeys → nine contained
elements + one opening) that the Rust assertions describe. Two different
parsers agreeing that the cross-references resolve is the strongest
non-self-referential evidence that the output is actually valid.

## What's still missing

- **Profile-curve geometry.** Windows come out as rectangles today; the
  decoded curve geometry (Hermite splines, ellipses, trimmed arcs from
  `GEO-07`..`GEO-10`) isn't routed into `IfcTrimmedCurve` /
  `IfcCompositeCurve` yet.
- **`IfcOpeningStandardCase` vs plain `IfcOpeningElement`.** The IFC4
  "StandardCase" variants carry tighter geometric invariants; the exporter
  emits the general form for all voids.
- **Site geo-location.** `ProjectPosition` and `SurveyPoint` are decoded
  but not threaded into `IfcSite.RefLatitude` / `RefLongitude` / `RefElevation`.
- **Pset_RevitType aliasing.** The Autodesk exporter uses
  `Pset_RevitType_{ClassName}` when there's no standard match;
  `from_decoded.rs` has the scaffolding but only emits the standard
  `Pset_*Common` sets so far.
- **Materials → layers for composite walls.** `IfcMaterialLayerSet` /
  `IfcMaterialLayerSetUsage` (tasks `IFC-28` / `IFC-29`) would render
  composite wall assemblies correctly; today each element associates to a
  single flat material.

None of these block a usable v1 — the synthetic project opens in
BlenderBIM with geometry, openings, materials, property sets, and the full
spatial hierarchy wired through — but they're the natural next few points
on the curve.

## Source

All code referenced above lives in the public repo:

- Bridge struct: `src/ifc/from_decoded.rs`
- STEP writer: `src/ifc/step_writer.rs`
- Entity types: `src/ifc/entities.rs`
- Integration test: `tests/ifc_synthetic_project.rs`
- Reference fixture: `tests/fixtures/synthetic-project.ifc`

`rvt-rs` is Apache-2, clean-room, and does not depend on any Autodesk or
`libredwg`-GPL code. Issues and PRs welcome at
[github.com/DrunkOnJava/rvt-rs](https://github.com/DrunkOnJava/rvt-rs).
