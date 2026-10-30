# RE-269 — A wall's side areas and volumes

**Date:** 2026-10-30
**Issues:** #1046 (quantities Revit computes from its joined geometry, step 1: walls), #1016 (quantities), W7 of [`docs/release-1.0-criteria.md`](../../docs/release-1.0-criteria.md); follows RE-251 (a wall's record keeps none of Revit's wall quantities), RE-252 (`Length`), RE-257, RE-266 and RE-267 (`Height`, and the top cut at the floor that covers a wall)
**Artefacts:** the BIM4LCA office as built and at building permit (Revit 2024, Revit's IFC4 Reference View export, exporter 24.0.5) and puzzbobb's R2025 models (Revit 2025, Revit 25.0.2's IFC4 exports), as Measure fetches them; Core Interior and RE1 Architecture, whose exports have base quantities off (RE-242)
**Runs:** Measure (`tools/re/wall_quantities_vs_ifc.py`, with `probes/examples/probe_1067_native_dump.rs` for the walls' records) [41101457836](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41101457836) and [41103633258](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41103633258); the change against `main`, every model and scorer, [41108951821](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41108951821) (the scorer runs are of the same scorer on this change's first commits)
**Result:** positive, narrowed. Revit writes a wall's `GrossSideArea`, `NetSideArea` and `GrossVolume` on exactly the walls it writes a `Height` on, and its `NetVolume` on every wall. All four are fixed by the faces and the volume of the wall's joined solid. rvt-rs writes them on the walls whose native record holds no step that cuts, embeds or sweeps the solid and whose body is the record's box cut back at its joins, a plain rectangle: 49 of the office's walls as built, 48 at permit and 91 R2025 walls, Revit's on every one, none different and none Revit's export leaves out.

## 1. What Revit writes, and where

revit-ifc's `PropertyUtil.CreateWallBaseQuantities` (CLEANROOM.md source 5; read at the tags `IFC_v24.1.0` and `IFC_v24.2.0`, identical here, since the office's exporter 24.0.5 has no tag) takes, per solid of the wall:

- `GrossSideArea`: the largest, over the solid's vertical faces, of the area inside the face's first curve loop;
- `NetSideArea`: the area of the largest vertical face, its holes taken out;
- `NetVolume`: the solid's volume;
- `GrossVolume`: the wall type's width times the `GrossSideArea`, written only where the wall has a `Height`.

`IFC_v25.2.0` groups the vertical faces by normal instead and adds up each group; the two readings give the same value on a rectangular prism. The same function writes `Height` (the span of the solids, RE-257) and `GrossFootprintArea` (the extrusion's profile, RE-252) and is reached only for a wall exported with an axis. The 28 walls with only a `NetVolume` fit a wall that is not: the generic quantity calculators then run, among them `NetVolumeCalculator`, which sums the volumes of the solids and meshes and needs no extrusion.

That is the pattern in Revit's export. Every wall carries `Qto_WallBaseQuantities` with one of two sets of quantities (Measure 41103633258):

| | `Height`, `GrossSideArea`, `NetSideArea`, `GrossVolume`, `NetVolume` | `NetVolume` only | `GrossFootprintArea` too |
|---|---:|---:|---:|
| office as built | 579 | 28 | 279 |
| office at permit | 565 | 24 | 276 |
| R2025 models | 138 | 0 | 138 |

So the four quantities that depend on a body are written together with `Height`, and a rule for `Height` (RE-266, RE-267) is a rule for them.

## 2. The definitions on Revit's own bodies

`wall_quantities_vs_ifc.py` meshes the Body of every wall in both files with IfcOpenShell (openings applied), joins the coplanar vertical triangles into faces and works the quantities out of them. Against Revit's own quantity, on the walls it writes it on:

| | office as built | at permit | R2025 |
|---|---:|---:|---:|
| `GrossSideArea` = largest face's outer boundary (variant A) | 435 of 579 | 437 of 565 | 138 of 138 |
| ... = faces grouped by normal (variant B) | 270 | 266 | 138 |
| `NetSideArea` = largest face (A) | 430 | 425 | 138 |
| ... (B) | 276 | 275 | 138 |
| `GrossVolume` = width × A's gross | 435 | 437 | 138 |
| `NetVolume` = volume of the mesh | 416 of 607 | 410 of 589 | 138 of 138 |
| `GrossFootprintArea` = `Length` × `Width` | 279 of 279 | 276 of 276 | 136 of 138 |
| `Height` = span of the mesh | 579 of 579 | 565 of 565 | 138 of 138 |

The office's exporter is variant A (B fits 270 of 579). The R2025 models cannot tell the two apart: their walls are plain prisms. The shortfall on the office is the body in Revit's file: its IFC body is not always the solid the quantities came from (a layered wall's body is one extrusion; its quantities are of the layered solid), so the mesh is for learning the definitions, never for scoring.

## 3. rvt-rs's bodies against Revit's quantities

The same candidates worked out on rvt-rs's own export (variant A, Revit's `Width` for the gross volume), by what the wall's native record holds, on the office as built (Measure 41103633258; the permit office is as in §4):

| the wall's record holds | walls | `NetVolume` | `GrossSideArea` | `NetSideArea` | `GrossVolume` | Revit writes only `NetVolume` |
|---|---:|---:|---:|---:|---:|---:|
| 1. no step but `BaseWallGStep`, `JoinEndGStep`, `VCWSplitFaceGStep`, `WallRefPlanesGStep`, `WallJoinTweakGStep`, `VerticalExtensionOfLayersGStep`; body the record's box cut back at its joins, a rectangle | 49 | 49 | 49 | 49 | 49 | 0 |
| 2. the same, body from the wall's line or its layers | 7 | 5 | 6 | 6 | 6 | 0 |
| 3. also a roof or floor join, a clean join | 217 | 118 | 163 | 163 | 163 | 3 |
| 4. also a cut pair, a cutout, a rectangular opening, an embedded wall or a sweep | 334 | 71 | 123 | 119 | 123 | 25 |

Group 1 is exact on all four quantities, on both offices and the R2025 models. The walls of group 2 are bodies that are not Revit's: a layered wall's staircase of ends is a reading of its join lists, and a wall of R2025 `18_WallJoins`, 4511, has a longest face of 38.750 ft² in Revit's quantities where rvt-rs's rectangle gives 30.139. Group 3 is mostly right where the top is not cut (§6); group 4 needs the openings, the cutting elements and the sweeps worked into the solid.

## 4. The rule

A wall gets `GrossSideArea`, `NetSideArea`, `GrossVolume` and `NetVolume` when all of these hold:

- its native record holds none of `JoinToRoofGStep`, `CutPairGStep` or `WallCutoutGStep` (RE-266) and no other step but the six above: every other class of the record ending in `Step` leaves it out (`native_walls::HARMLESS_STEPS`, `partition_schema_mvp::WALL_WHOLE_FIELD`). `CleanPairGStep` is one that leaves it out: the walls that hold it are in group 3 of §3;
- its body is `partition_element_record_join_trimmed` (the record's box cut back at its joins, RE-26, RE-29), drawn as one rectangle, with a `Width` written (its type's thickness, #1025).

The values come from the rectangle of the body, `width` by `depth` and `height`: `GrossSideArea` and `NetSideArea` are the longer side times the height, `NetVolume` the rectangle's area times the height, and `GrossVolume` the written `Width` times the side area. A thin wall whose end is wider than its length gets the end's area, as Revit's largest face does (3184170, 0.164 ft long and 0.328 thick).

Measured on the walls this selects (Measure 41103633258 on the exported bodies, 41108951821 on the shipped export):

| | walls | `NetVolume` | `GrossSideArea` | `NetSideArea` | `GrossVolume` | different | Revit omits |
|---|---:|---:|---:|---:|---:|---:|---:|
| office as built | 49 | 49 | 49 | 49 | 49 | 0 | 0 |
| office at permit | 48 | 48 | 48 | 48 | 48 | 0 | 0 |
| R2025 models | 91 | 91 | 91 | 91 | 91 | 0 | 0 |

The other bodies from the record's box, `partition_wall_centreline`, are the ones that go wrong (R2025 `18_WallJoins` 4511 and 4567, an angled join), so they stay out, and so does the one record-box wall at permit whose body is the bare box (`partition_element_record_bbox`, right on its one wall).

## 5. Measured

This change against `main`, every model, every scorer (run 41108951821, `-f base=main -f nordic=true -f r2025=true`). Only `ifc.sha256` and `qtos_vs_ifc` move, apart from Core Interior's witness observations:

| model | quantities scored | equal before | equal after | different | not written by Revit |
|---|---:|---:|---:|---:|---:|
| office as built | 39,721 | 27,752 | 27,948 | 1 (the mullion) | 0 |
| office at permit | 40,013 | 27,686 | 27,878 | 1 | 0 |
| R2025 models (24) | 1,515 | 411 | 775 | 0 | 0 |

By quantity (equal, of the walls Revit writes it on): office as built `GrossSideArea`, `NetSideArea` and `GrossVolume` 0 to 49 of 579 and `NetVolume` 0 to 49 of 607; at permit 0 to 48 of 565 and of 589; R2025 0 to 91 of 138 each.

Core Interior's and RE1 Architecture's exports were made with base quantities off (RE-242): rvt-rs writes the four on 139 and 1 more walls there, counted apart, and Core Interior's witness observations gain 278 `IFCQUANTITYAREA` and 278 `IFCQUANTITYVOLUME`.

## 6. What stays unwritten

- Walls with a top join (217 of the office's as built that have only roof or floor joins and clean joins): 163 are right on the side areas and the gross volume and 118 on the net volume. What decides is whether the top is Revit's (RE-267) and whether the ends are, and a body from the wall's layers is right on the longest face only. A rule for them has to say the floors that join a wall do not intrude on it or cut it where rvt-rs draws it, and that its length is the one the record gives (`Length` written).
- Walls with openings (their doors and windows are voids of the body), cut pairs, embedded walls and sweeps (334): 71 of 334 are right on the net volume.
- `GrossFootprintArea` = `Length` × `Width` on all 279 walls that carry it, but one wall of the 49 as built and one of the 48 at permit (3184170, whose length is half its width) has none, extruded all the same. A footprint on the 49 would be one Revit omits.
- The 28 walls as built and 24 at permit with only a `NetVolume`: nothing in the record tells them from the rest, and their `NetVolume` waits on a rule for the body.

## 7. Reproduce

```bash
gh workflow run measure.yml --ref feat/1046-wall-areas -f ref=88905509d17fa768c83978c0d082b76ec03c787d -f base=main -f nordic=true -f r2025=true
gh workflow run measure.yml --ref feat/1046-wall-areas -f ref=88905509d17fa768c83978c0d082b76ec03c787d -f base=none -f nordic=true -f r2025=true \
  -f models="nordic-office-* r2025-*" -f scorers=wall_quantities_vs_ifc -f probe=probe_1067_native_dump -f probe_scorer=-
```

The scorer is this change's; once it is on `main`, use `--ref main`. Group 1's tallies join the probe's `wall <id> classes` lines (the steps of a wall's record) with the scorer's `wall` lines (the candidate values, `BodySource` and profile, the quantities Revit wrote).
