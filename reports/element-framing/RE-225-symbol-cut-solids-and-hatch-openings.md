# RE-225 — The solid a symbol with no cut loop cuts, and the openings face-hosted hatches cut in a roof

**Date:** 2026-10-25
**Issues:** #824 (the office roof's hatch openings), #810 (door and window sizes); follows RE-215 (cut loops) and RE-218 (sizes from the record box)
**Artefacts:** the BIM4LCA office as built and at building permit (Revit 2024) with Revit's IFC4 exports, as Measure fetches them
**Runs:** Measure [40228031519](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40228031519) and [40229191653](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40229191653) (`examples/probe_re225_symbol_cut_geoms.rs` with `tools/re/symbol_cut_geoms_vs_ifc.py`), [40229102502](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40229102502) (`examples/probe_824_sketch_plane_json.rs`), [40230282324](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40230282324) (`tools/re/host_openings_list.py` on Revit's export), this change against `main`: [40232776228](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40232776228) and [40237486226](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40237486226)
**Result:** positive: roof 2991742's four openings are written where and as Revit's export writes them.

## 1. A symbol's cut solid

RE-215 sizes a door or window from the one `CurveLoop` its symbol's `m_pCutLoops` names. Where that list is empty, the `FamilySymbol` record names the solid it cuts in `m_complexCutGeoms`: one `Geometry` object, whose faces `native_saved_mesh::face` meshes. On both office files a symbol carries one or the other, never both.

| what the symbol draws | cut solid's extent against Revit's sizes |
|---|---|
| the office's 269 windows | along X, `OverallWidth` on all 269; along Z, `OverallHeight` plus 0.0594 ft on all |
| doors 1382481 and 1623922 | width and height exactly |
| roof hatches 2053330, 2053983, 2896990, 2896992 | a 1.9685 ft (600 mm) square prism reaching 6.6051 ft below the hatch |
| doors 1756089 and 1765267 | 11 of 12 faces mesh; height 4.3570 ft is Revit's, width 3.1168 ft is not (Revit 5.8629 ft) |

Every face `face` declines is an unbounded `Plane`, one with no first loop, and doors 1382481 and 1722215 carry such faces while giving Revit's size: they are part of how a symbol stores its cut solid. RE-218's box rule stays the source of door and window sizes; #810's two doors stay open (their width is in none of the box, the cut solid and the symbol's stored lengths, RE-229).

## 2. What hosts a hatch

A roof hatch's `FamilyInstance.m_hostId` names a `SketchPlane` (3079516, 3073092), not the roof. The plane's `GeomOnPlaneRef.m_geomRef.m_elemId` is roof 2991742 (`m_geomTag` 362), and its `FacePointRef` lists the vertices of that roof's top face, the holes the other hatches cut in it included. RE-221's dump kept only scalars, short arrays and bare ElementIds, so the reference held in `m_geomRef`'s struct was dropped there and the link looked absent; the full dump shows it.

## 3. Revit's openings

Revit's export cuts four `IfcOpeningElement`s in roof 2991742, which it writes as an `IfcSlab`: each tagged with the roof's ElementId, named `Basic Roof:YP-1:2991742:1` to `:4`, an extruded polygon running the roof's whole thickness (973 mm as built, 970 mm at permit) straight up. In the site frame each is a 1.9685 ft square, and `:1` to `:4` are cut by hatches 2896992, 2896990, 2053983 and 2053330: the cutting instance with the highest ElementId first.

## 4. What rvt-rs does

`native_symbol_cuts::face_hosted_cuts` reads, for each door and window, its sketch plane's host and its symbol's cut solid, in two passes. The export pass `attach_hosted_cut_openings` places the solid by the instance's transform (RE-87) and, where it is a prism standing upright through the host's whole thickness and the host is one element drawn as an extrusion, writes the solid's plan footprint as an opening of the host (`IfcEntity::VoidOpening`, the RE-151 entity) extruded through the host's thickness, tagged with the host's ElementId and named for the host and its place among the host's openings. Any other cut is left out. Each is an `IfcEntity::HostedCutOpening`, a new variant of the non-exhaustive enum, which the STEP writer places as it places a `VoidOpening` and gives Revit's GlobalId (§6).

## 5. Measured

| file | Revit's openings in roof 2991742 | rvt-rs's | box in the site frame | depth | name |
|---|---:|---:|---|---|---|
| office as built | 4 | 4 | every corner within 0.0001 ft of Revit's | 0.973 m, Revit's | Revit's, in Revit's order, with Revit's GlobalIds |
| office at permit | 4 | 4 | the same | 0.970 m, Revit's | the same |

Core Interior, Einhoven and the four RE1 models export byte for byte as before.

## 6. GlobalIds

revit-ifc's `OpeningUtil` gives an opening it cuts for a cutting element `GUIDUtil.CreateGUIDString("Opening:<i>Solid:<j>", host, cutter)`, and an opening its host's own extrusion data carries the hash of `IfcOpeningElement:<number>:<host GlobalId>`, naming it `<host name>:<number>` (CLEANROOM.md source 5; the key's wording is reproduced because the same identifier needs it). `tools/re/opening_guids_vs_ifc.py` tries both against Revit's export (Measure [40234655490](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40234655490) and [40236034012](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40236034012)): the first gives none of the 44 openings in hosts other than walls; the second gives all four of roof 2991742's on both office files, `number` being the index in each name, and the openings Revit cuts in doors 1382481, 1623922, 3444654 and 3448563 and in RE1's washing machine 377331, which rvt-rs does not write. A floor's sketch voids (RE-151) follow neither. rvt-rs writes each hatch opening as `IfcEntity::HostedCutOpening`, whose GlobalId the writer hashes the same way from the host's GlobalId.

Measure [40237486226](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40237486226), `tools/re/host_openings_list.py` on both exports: rvt-rs's four openings carry Revit's GlobalIds (`3YaXsZK7DWAzH8QHq$vLLn`, `2_vAcrRri2S49gzbvjYOW1`, `0DdCEMitnLfOdlYT2LDb3G`, `2bZgoAmVzSxbOlfXcVBbmd`), names and order, and the same box to 0.0001 ft, on both office files.

## 7. Reproduce

```bash
gh workflow run measure.yml --ref feat/824-hatch-openings -f ref=feat/824-hatch-openings -f base=main \
  -f nordic=true -f keep_ifc=true -f probe=basics_inspect -f probe_scorer=host_openings_list
```
