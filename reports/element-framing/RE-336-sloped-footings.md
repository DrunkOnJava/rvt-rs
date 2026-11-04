# RE-336: a floor with shape-edited corners is drawn along its sloped top, and the walls on it start from that top

**Date:** 2026-11-04
**Issues:** #1384 (footings Revit draws sloped, the BIM4LCA office's Basement walls); follows RE-335 (#1383), RE-258 section 4 (the base), RE-267 and RE-320 (the top)
**Artefacts:** the BIM4LCA office as built and at building permit (Revit 2024, Revit 24.0.5's IFC4 exports, CC BY-SA 4.0, `research/bim4lca-measure.tsv`), Core Interior, RE1 Architecture, puzzbobb's 23 Revit 2025 models, Autodesk's sample projects and the held-out files, as Measure fetches them
**Probe, tools:** `probes/examples/probe_1067_native_dump.rs` (its `step SlabShapeEditGStep` and `relation JoinToRoofGStep base` lines), `tools/re/wall_heights_vs_ifc.py`, `tools/re/wall_top_cover_vs_ifc.py`, `tools/re/slab_bodies_vs_ifc.py` (new)
**Runs:** Measure on head d853a6268bc0c3c8b040067333a03ec9157884a9 against `main` at ac8befe2 (section 5)
**Status:** shipped. 12 of the office's 14 Basement walls whose base Revit raises (as built; 12 of 16 at permit) now have Revit's lowest point, and the other 2 (4 at permit) are 0.123 ft (0.044 ft) off instead of 0.656 ft, because their plans end short of Revit's. No other wall's lowest or highest point moved on either office file, and every other model's export is unchanged but for nine refusal markers on Autodesk's Snowdon Towers.

## 1. What the bytes say

The Basement's footings of the office (a foundation slab is a `Floor` element, `IfcSlab` in Revit's export and in rvt-rs's, drawn from its record box until now) carry a `SlabShapeEditGStep` in their native record. Its fields, as the probe prints them:

- `m_originalFaceHeight`: the elevation of the floor's top before the edits, model feet (174.8688 for the footing 2057673).
- `m_cornerVertices[]`: one entry per vertex of the floor's outline, each `m_vertex` holding `m_xyLocation` (the vertex's place in the plan, model feet) and `m_offset`, its height above `m_originalFaceHeight`. The footing's four corners are at offsets 1.9685, 1.9685, 0 and 0 ft.
- `m_interiorVertices[]`, `m_edgeVertices[]`, `m_explicitCreases[]`: points and creases added inside or on the edges. Empty on the footings.
- `m_dormant`: false when the edit applies.

The top of such a floor is the plane through its corners (`z = original + offset` at each `xy`). The four corners of 2057673 lie in one plane to 3e-7 ft, and the plane reproduces the floor's record box: the box's top is the highest corner (176.8373 ft), and its base, 173.9501 ft, is that top less the box's height; the lowest corner of the top is 174.8688 ft, so the underside is the top less 0.9187 ft **vertically** (the type's thickness, which RE-295 measured on every floor nothing cuts, flat or sloped). The body is the plan outline between the plane and the plane lowered 0.9187 ft, with upright sides.

Revit raises the base of a wall whose base join (`JoinToRoofGStep` with `m_baseOrTop` 0) names such a floor to the floor's top over the wall's plan: the wall's lowest point is the lowest point of the plane over the wall's plan (a footing's top slopes, so a wall across the slope is raised more at one end, and its body is the solid between the sloped top and the wall's own top). A floor that is level is the same rule with a level top. Computed from the probe's corners and the walls' plans before any code was written, the plane gave Revit's lowest point on 12 of the 14 Basement walls Revit raises as built.

## 2. The rule shipped

`partition_schema_mvp::attach_slab_uncut` (the native pass over floors, ceilings and roofs, which already read `CutPairGStep` for RE-295) now reads the `SlabShapeEditGStep` as well, from the same record:

- no vertex moved and nothing added: the floor is unedited (no marker);
- corners only, finite, at least three, the first three not collinear and all within 1e-5 ft of the plane through them, `m_dormant` false: `m_slab_top_plane` = `[a, b, c]` of `z = a*x + b*y + c`;
- anything else, or two copies of the record that disagree: `m_slab_shape_unread` = the reason (`interior_edge_vertices_or_creases`, `corners_not_in_one_plane`, `first_three_corners_collinear`, `fewer_than_three_corner_vertices`, `shape_edit_dormant_or_unmarked`, `corner_vertex_not_finite`, `record_copies_disagree`).

A `Floor` with a plane, whose plan outline is one loop and whose record box's height is reproduced by the plane within 1e-4 ft (the roofs' closure, `ROOF_SLOPE_CLOSURE_FEET`), is drawn as the slab between the plane and one thickness lower (`BodySource` `native_slab_shape_edit`, a `SolidShape::FacetedBrep` from `sloped_slab_mesh`). Its `extrusion` stays the record box prism. Any other floor with an edit keeps its box and gets `SlabShapeRefusal` (`plan_outline_not_read`, `plan_outline_in_pieces`, `plane_does_not_reproduce_record_box`, or the unread reason above).

A wall whose native record has a base join (`m_baseOrTop` 0) gets `m_wall_base_cutters` (the join's `m_ids`). `wall_tops::cut_wall_bases` takes a wall that is one plain extrusion, runs after the top cut, and for each named element built as a slab body (`build_cutter`: a plain extrusion, or a floor drawn along its plane) that covers the whole of the wall's plan (`covers`, the RE-267 test), takes the element's top: its level, or the plane's minimum and maximum over the wall's plan. The wall is cut when exactly one named element qualifies and its top lies between the wall's base and top over the whole of the plan (1e-3 ft inside each). A level top gives the wall's profile extruded from that level; a sloped one gives the solid between the plane and the wall's top (`between_surfaces_mesh`). The wall carries `BodyBaseSource` (`wall_base_sloped_at_covering_floor` or `wall_base_cut_at_covering_floor`; not `BodyHeightSource`, which a wall whose top was cut also carries and which a property set holds once) and `BaseCoverElementId`; its `extrusion` stays the full prism, which the passes that read a wall's plan and its record's box use, and the walls that host rectangular wall openings (RE-321) still do. A wall that names a floor with shape-edited corners and is not cut carries `BaseCoverRefusal`: `floor_shape_not_read`, `wall_not_one_plain_extrusion`, `no_element_covers_the_wall_plan`, `named_element_body_not_read`, `several_elements_cover_the_wall`, `top_crosses_the_wall_base_or_top`.

A wall with an edited elevation profile is cut like any other. The first head measured (d62cc942, run 41918446178) refused such walls, as the top cut does (RE-267: Revit does not cut their tops); six of the office's Basement walls have one, Revit raises their bases to the footing as it does the others, and the profile's openings void the solid as they void the prism. Allowing them moved those six from "not Revit's" to Revit's on four and to 0.123 ft off on two (section 5).

The top cut (RE-267, RE-320) no longer takes a floor drawn along its sloped top for a level one: its `build_cutter` requires the floor to be a plain extrusion, which a sloped floor no longer is. No wall's top moved (section 5).

## 3. Not done

- A wall that a named element covers in part (a stepped base, as RE-320 draws a stepped top) keeps its base.
- Floors whose edits add interior or edge vertices or creases, or whose corners do not lie in one plane, keep their box and say why. Autodesk's Snowdon Towers Architectural and Structural (2024) have nine such floors (section 5); there is no redistributable Revit export of them to score a reading against.
- Roofs and ceilings with shape edits are not read.
- The four walls whose plan ends short of Revit's (section 5) keep an error of the plan's, not of the footing's.
- Provenance (`--element-provenance`) lists no reader for the new floor body (`tests/value_sources_offsets.rs` lists it as `NotYet`); it is a follow-up of #1368.

## 4. Reproduce

```bash
gh workflow run measure.yml --ref fix/1384-sloped-footings -f ref=d853a6268bc0c3c8b040067333a03ec9157884a9 -f base=main -f nordic=true \
  -f models="nordic-office-as-built nordic-office-permit core-interior re1-architecture" \
  -f scorers="wall_heights_vs_ifc wall_top_cover_vs_ifc slab_bodies_vs_ifc wall_quantities_vs_ifc"
```

`-f scorers=schema` on the two offices, `-f r2025=true -f samples=true`, `-f holdout=true` and `-f samples=true -f sample_years=2024 -f keep_ifc=true -f scorers=none` are the other four runs.

## 5. Measured

Head d853a6268bc0 against `main` at ac8befe2; the Measure runs are [41921405983](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41921405983) (both offices, Core Interior, RE1 Architecture), [41921412690](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41921412690) (the 23 R2025 models and Autodesk's samples), [41921418484](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41921418484) (the held-out files, counts only), [41921422943](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41921422943) (the 2024 samples with the IFC kept) and [41921496500](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41921496500) (schema rules on both offices).

**The floors.** `slab_bodies_vs_ifc.py`, per `native_slab_shape_edit` body against Revit's body of the same Tag:

| | as built | at permit |
|---|---:|---:|
| floors drawn along their plane | 8 | 8 |
| in Revit's export | 8 | 8 |
| volume within 0.01% of Revit's | 8 | 8 |
| every vertex of each body within 0.001 ft of the other's surface | 8 | 8 |
| furthest vertex | 3.5e-6 ft | 1.6e-6 ft |

**The walls.** `wall_heights_vs_ifc.py`, lowest and highest point of each wall's body:

| | as built, before | as built, after | at permit, before | at permit, after |
|---|---:|---:|---:|---:|
| base and top Revit's | 586 | 598 | 565 | 577 |
| top Revit's, base not | 20 | 8 | 24 | 12 |
| base Revit's, top not | 1 | 1 | 0 | 0 |

The 12 walls moved to Revit's as built are 4 on level floors (1751938, 1752114, 1760735, 1760736: raised 3.28 ft) and 8 on sloped footings (2056705, 2056741, 2056770, 2057669, 2058500, 2070689, 2070690, 2070691), each to Revit's lowest point within 0.0001 ft. The same 12 at permit. Walls cut: 14 as built (4 `wall_base_cut_at_covering_floor`, 10 `wall_base_sloped_at_covering_floor`) and 16 at permit (4, 12). Every wall whose lowest point changed moved towards Revit's; none moved away (all 607 walls as built and 589 at permit compared, base and top).

What stays off: 2057668 and 2057670 (and at permit 2058499 and 2058501) are cut along the footing but their plans stop short of Revit's at the low end, 0.29 ft as built (rvt-rs's plan reaches y 75.73 ft where Revit's reaches 75.43; areas 3.816 and 4.070 ft2) and 0.09 to 0.10 ft at permit, so their lowest point is 0.123 ft (as built) and 0.044 ft (at permit) above Revit's instead of 0.656 ft below it. Walls 2058499 and 2058501 as built extend past the footing's edge: no element covers their plan (`BaseCoverRefusal` `no_element_covers_the_wall_plan`, 2) and they keep their base, which is Revit's. The other 6 as built are the walls of RE-335 that embed a curtain wall; the other 8 at permit are the same kind (five raised 8.2021 ft and three 9.5144 ft). The stub of RE-320 section 4 (3004978, top not Revit's) is unchanged.

**Nothing else moved.** `wall_top_cover_vs_ifc.py`: the top, the top's area and the taller and lower counts of all 607 walls as built and 589 at permit are as on `main`. The scorer's own "top: cut, not Revit's" (121) is unchanged. Core Interior's and RE1 Architecture's exports are byte-identical to `main`'s (SHA-256), as are the 23 R2025 models' and all but two of Autodesk's sample projects', and all 50 held-out exports (counts only).

Autodesk's Snowdon Towers Architectural and Structural (2024) are the two samples that moved. In the kept IFC (run 41921422943) the change is nine `SlabShapeRefusal` properties on floors and nothing else: 6 `interior_edge_vertices_or_creases` and 2 `corners_not_in_one_plane` on the Architectural, 1 `corners_not_in_one_plane` on the Structural; neither file has a floor drawn along a plane or a wall cut at its base, and their diagnostics are identical to `main`'s. There is no redistributable Revit export of either to score them against.

**The schema.** Run 41921496500, both offices: the IfcOpenShell validate step finds 0, the EXPRESS rules the same 4 as on `main` (`IfcAirTerminalType`, `IfcElectricApplianceType`, `IfcRelAssociatesMaterial`, `IfcSpaceType`), the property sets 0 with the same counts (27,330 as built, 27,451 at permit): a wall cut at the top and the base carries one `BodyHeightSource` and one `BodyBaseSource`.

**Quantities.** `wall_quantities_vs_ifc.py`: `Height` equal to rvt-rs's body span on 572 of 579 walls as built (563 before) and 557 of 565 at permit (547); `NetVolume` equal to rvt-rs's body volume on 297 of 607 (289) and 275 of 589 (267). Its `GrossFootprintArea` candidate "rvt-rs's plan" falls from 252 to 247 of 279 as built and from 207 to 201 of 276 at permit: it measures the horizontal faces of the body, and a wall with a sloped base has no horizontal bottom face. the property sets' counts (section on the schema above) are unchanged, so no quantity was added to or taken from a wall.

## 6. Not measured

- A shape-edited floor in a Revit 2025 or later project: the R2025 models hold none.
- A wall covered in part by a footing, a floor with interior vertices, creases or corners out of one plane, and a shape-edited roof or ceiling: refused, not read, and not scored.
- A base join on a profile-edited wall in a model other than the office: none of the other models measured has one cut by this rule.
- Snowdon Towers' floors with edits (nine, section 5), for want of a redistributable export.
