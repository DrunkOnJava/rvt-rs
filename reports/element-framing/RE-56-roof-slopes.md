# RE-56 — A roof edge's slope, and shed roofs drawn along it

**Date:** 2026-09-23
**Result:**
- **Positive.** Each sketch line of a footprint roof carries, in its element data, the slope Revit's roof tools set on that edge:
  - its angle, which is 9:12 by default;
  - a flag saying whether the edge defines the roof's slope;
  - on a defining edge, the slope as −rise/run.
- **Positive.** A roof type's compound structure (RE-53) is framed on 2024 roofs as on 2025 floors: the type's name, then `u32 0`, then the layer count. The "`2d 00` framing" RE-53 recorded for 2025 floors was this same pattern, with types named "-".
- rvt-rs now draws a Revit 2024 roof with exactly one slope-defining edge (a shed roof) along its slope, when doing so reproduces the roof's record box height.
  - On the Autodesk tutorial house, the one sloped roof rises at 1:12 from its defining edge across its 20.458 ft outline, with its type's 1.25 ft thickness square to the slope. That gives 2.9591938797 ft; Revit's record box is 2.959193880 ft tall.
  - Roofs with no defining edge stay level plates, as before. No Snowdon Towers roof has a defining edge, and one of them is sloped in Revit's export all the same (§5).
  - Hip and gable roofs (two or more defining edges) stay level plates. No oracle has one.

**Oracle:** the MIT 4.567 copy of the Autodesk tutorial house, saved in Revit 2024 (`demo-01-Source_House_2024_tn2.rvt`, sha256 `763f239e70db1ac90e5aa806ec4e6a2cebdc3e8a2e902bcfda6b8de43d2fdcfb`, local only, "copyright reserved"). It has no export of Revit's own. The check is the record box Revit computes and stores for each roof: the slope, the outline and the thickness must reproduce its height to 1e-4 ft, or the roof is not drawn sloped.

Snowdon Towers' roofs, with Revit's IFC4 export (local only):
- 18 have sketch lines. Every slope block found on them defines no slope. Six of them (1916295 to 1916596) each have one edge whose block is not found.
- The other two, 759489 and 882818, have no sketch lines.
- Revit's export of them is flat but for tapered insulation and roof 2112673. Its 4 edges all define no slope, yet Revit slopes its faces at 10.8° to 16.7°.

-----

## 1. The edge's slope block

A roof sketch line's data (RE-44's header and ElementId) holds this block, found by its run of 32 `ff`:

```text
-0x18  f64      slope angle, radians
-0x10  00 × 16
 0x00  ff × 32
+0x20  i64      −9 (tutorial house) or −6 (Snowdon)
+0x28  u32      0 or 1
+0x2c  u32      7
+0x30  u8 × 4   00 00 01 00, or 01 01 01 00 on an edge that defines the slope
+0x3b  f64      −rise/run, on an edge that defines the slope
```

On the tutorial house:
- three roofs have `00 00 01 00` on all 16 of their edges and are flat. Their record boxes are 1.25 ft tall, their type's thickness.
- roof 169835 has 8 edges. Seven hold the default 0.6435011 rad (9:12) and `00 00 01 00`.
- edge 169841, the roof's low edge from (20.927, −7.627) to (−48.969, −7.627), holds 0.0831412 rad, which is atan(1/12). It carries `01 01 01 00`, and −0.0833333 at +0x3b.

`partition_roof_slopes::edge_slope` accepts a block only in this shape. On a defining edge it also requires the rise/run to equal −tan of the angle to 1e-9.

## 2. The roof's thickness

The roof's type (RE-44) is 143439. Its data holds its name, then `u32 0`, then 4 layers of RE-53's 37-byte record: 1/2", 4", 10" and 1/2", 1.25 ft in all.

RE-53 read 2025 floors' and ceilings' layers behind "`u32 k`, k × `2d 00`, `u32 0`". `2d 00` is the UTF-16 hyphen: that framing is a type's name. RE1's types are named "-". `find_layers` now accepts any length-prefixed name of printable code units there.

With the change, Snowdon's GLB is byte-identical and Core Interior's IFC is unchanged but for its timestamps.

## 3. The shed

`partition_schema_mvp::attach_roof_slopes` gives a 2024 roof its defining edge, angle and type thickness. It requires:
- a sketch outline (RE-50) and a type;
- a slope block on every one of its sketch lines, exactly one of them defining;
- the defining edge's bounded line (RE-49).

`export_content::roof_slope_solid` then builds the roof body.
- Every outline point lies on one side of the edge, at distance d into the roof.
- The underside lies at d·tan θ above the record box's base, and the top T/cos θ above the underside, with upright sides. That is `body_geometry::sloped_slab_mesh`, written as an `IfcFacetedBrep`.
- It is drawn only if the farthest point's rise plus T/cos θ equals the record box's height to 1e-4 ft.

For roof 169835:
- d reaches 20.458333 ft across its outline;
- 20.458333 × tan θ + 1.25 / cos θ = 1.7048611 + 1.2543327 = 2.9591938797 ft;
- the record box runs from 18.916666667 to 21.875860546 ft, 2.959193880 ft.

## 4. End to end

```bash
./target/ci/rvt-ifc demo-01-Source_House_2024_tn2.rvt -o house.ifc
```

IfcOpenShell 0.8.5 meshes roof 169835 as an `IfcFacetedBrep` from z 18.916667 to 21.875860 ft, spanning x −48.969 to 20.927 and y −7.627 to 12.831. Every point of its underside lies at the base plus its distance from the defining edge over 12, and its top 1.25/cos θ above, to 2e-6 ft. The other three roofs are unchanged extrusions, 1.25 ft tall.

## 5. What this does not do

- Hip and gable roofs: two or more defining edges. Their surfaces meet along ridges and hips, and no oracle has one.
- Overhang and the rafter cut are not read. The height check refuses a roof whose shape they change.
- A slope set other than by a defining edge. Snowdon's 2112673 has no defining edge, yet Revit's export slopes it; a slope arrow is the likely carrier. It stays a level plate, as do roofs with an edge whose block is not found.
- Roofs with no sketch lines (Snowdon's 759489 and 882818).
- Revit 2025 roofs: sketch outlines are read on 2024 only (RE-50).
