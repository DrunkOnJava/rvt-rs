# RE-336: a floor with shape-edited corners is drawn along its sloped top, and the walls on it start from that top

**Date:** 2026-11-04
**Issues:** #1384 (footings Revit draws sloped, the BIM4LCA office's Basement walls); follows RE-335 (#1383), RE-258 section 4 (the base), RE-267 and RE-320 (the top)
**Artefacts:** the BIM4LCA office as built and at building permit (Revit 2024, Revit 24.0.5's IFC4 exports, CC BY-SA 4.0, `research/bim4lca-measure.tsv`), Core Interior and RE1 Architecture, as Measure fetches them
**Probe, tools:** `probes/examples/probe_1067_native_dump.rs` (its `step SlabShapeEditGStep` and `relation JoinToRoofGStep base` lines), `tools/re/wall_heights_vs_ifc.py`, `tools/re/wall_top_cover_vs_ifc.py`, `tools/re/slab_bodies_vs_ifc.py` (new)
**Runs:** to be filled in section 5.
**Status:** implemented; measurement in progress (section 5).

## 1. What the bytes say

The Basement's footings of the office (a foundation slab is a `Floor` element, `IfcSlab` in Revit's export and in rvt-rs's, drawn from its record box until now) carry a `SlabShapeEditGStep` in their native record. Its fields, as the probe prints them:

- `m_originalFaceHeight`: the elevation of the floor's top before the edits, model feet (174.8688 for the footing 2057673).
- `m_cornerVertices[]`: one entry per vertex of the floor's outline, each `m_vertex` holding `m_xyLocation` (the vertex's place in the plan, model feet) and `m_offset`, its height above `m_originalFaceHeight`. The footing's four corners are at offsets 1.9685, 1.9685, 0 and 0 ft.
- `m_interiorVertices[]`, `m_edgeVertices[]`, `m_explicitCreases[]`: points and creases added inside or on the edges. Empty on the footings.
- `m_dormant`: false when the edit applies.

The top of such a floor is the plane through its corners (`z = original + offset` at each `xy`). The four corners of 2057673 lie in one plane to 3e-7 ft, and the plane reproduces the floor's record box: the box's top is the highest corner (176.8373 ft), and its base, 173.9501 ft, is that top less the box's height; the lowest corner of the top is 174.8688 ft, so the underside is the top less 0.9187 ft **vertically** (the type's thickness, which RE-295 measured on every floor nothing cuts, flat or sloped). The body is the plan outline between the plane and the plane lowered 0.9187 ft, with upright sides.

Revit raises the base of a wall whose base join (`JoinToRoofGStep` with `m_baseOrTop` 0) names such a floor to the floor's top over the wall's plan: the wall's lowest point is the lowest point of the plane over the wall's plan (a footing's top slopes, so a wall across the slope is raised more at one end and its body is the solid between the sloped top and the wall's own top). Computed from the probe's corners and the walls' plans before any code was written, that gives Revit's lowest point on 12 of the 14 Basement walls Revit raises as built; the other two (2057668 and 2057670) differ by the 0.31 ft by which rvt-rs's wall plan ends short of Revit's (RE-320 section 4's kind of end), and two walls whose plans extend past the footing's edge (2058499 and 2058501) are not raised by Revit and are not covered by the footing's plan.

## 2. The rule shipped

`partition_schema_mvp::attach_slab_uncut` (the native pass over floors, ceilings and roofs, which already read `CutPairGStep` for RE-295) now reads the `SlabShapeEditGStep` as well, from the same record:

- no vertex moved and nothing added: the floor is unedited (no marker);
- corners only, finite, at least three, the first three not collinear and all within 1e-5 ft of the plane through them, `m_dormant` false: `m_slab_top_plane` = `[a, b, c]` of `z = a*x + b*y + c`;
- anything else, or two copies of the record that disagree: `m_slab_shape_unread` = the reason (`interior_edge_vertices_or_creases`, `corners_not_in_one_plane`, `first_three_corners_collinear`, `fewer_than_three_corner_vertices`, `shape_edit_dormant_or_unmarked`, `corner_vertex_not_finite`, `record_copies_disagree`).

A `Floor` with a plane, whose plan outline is one loop and whose record box's height is reproduced by the plane within 1e-4 ft (the roofs' closure, `ROOF_SLOPE_CLOSURE_FEET`), is drawn as the slab between the plane and one thickness lower (`BodySource` `native_slab_shape_edit`, a `SolidShape::FacetedBrep` from `sloped_slab_mesh`). Its `extrusion` stays the record box prism. Any other floor with an edit keeps its box and gets `SlabShapeRefusal` (`plan_outline_not_read`, `plan_outline_in_pieces`, `plane_does_not_reproduce_record_box`, or the unread reason above).

A wall whose native record has a base join (`m_baseOrTop` 0) gets `m_wall_base_cutters` (the join's `m_ids`). `wall_tops::cut_wall_bases` takes a wall that is one plain extrusion with no edited elevation profile, runs after the top cut, and for each named element built as a slab body (`build_cutter`: a plain extrusion, or a floor drawn along its plane) that covers the whole of the wall's plan (`covers`, the RE-267 test), takes the element's top: its level, or the plane's minimum and maximum over the wall's plan. The wall is cut when exactly one named element qualifies and its top lies between the wall's base and top over the whole of the plan (1e-3 ft inside each). A level top gives the wall's profile extruded from that level; a sloped one gives the solid between the plane and the wall's top (`between_surfaces_mesh`). The wall carries `BodyBaseSource` (`wall_base_sloped_at_covering_floor` or `wall_base_cut_at_covering_floor`; not `BodyHeightSource`, which a wall whose top was cut also carries and which a property set holds once) and `BaseCoverElementId`; its `extrusion` stays the full prism, which the passes that read a wall's plan and its record's box use. A wall that names a floor with shape-edited corners and is not cut carries `BaseCoverRefusal`: `floor_shape_not_read`, `wall_not_one_plain_extrusion`, `wall_profile_edited`, `no_element_covers_the_wall_plan`, `named_element_body_not_read`, `several_elements_cover_the_wall`, `top_crosses_the_wall_base_or_top`.

The top cut (RE-267, RE-320) no longer takes a floor drawn along its sloped top for a level one: its `build_cutter` requires the floor to be a plain extrusion, which a sloped floor no longer is.

## 3. Not done

- A wall that a named element covers in part (a stepped base, as RE-320 draws a stepped top) keeps its base.
- Floors whose edits add interior or edge vertices or creases keep their box.
- Roofs and ceilings with shape edits are not read.
- Provenance (`--element-provenance`) lists no reader for the new floor body; it is a follow-up of #1368.

## 4. Reproduce

```bash
gh workflow run measure.yml --ref fix/1384-sloped-footings -f ref=<head SHA> -f base=main -f nordic=true \
  -f models="nordic-office-as-built nordic-office-permit" \
  -f scorers="wall_heights_vs_ifc wall_top_cover_vs_ifc slab_bodies_vs_ifc"
```

## 5. Measured

To be filled from the Measure runs.
