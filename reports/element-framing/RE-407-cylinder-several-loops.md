# RE-407 — A cylinder face trimmed by several loops, measured again

**Date:** 2026-11-10
**Issues:** #1645; follows RE-304 and RE-305
**Runs:** Measure [42906883702](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42906883702) and [42906887386](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42906887386) (the reasons elements stay boxes, on the BIM4LCA files and on Autodesk's 2024 samples), [42910718296](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42910718296) and [42910722590](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42910722590) (`probe_1645_cylinder_trim_loops`: the loops of every such face), [42914809166](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42914809166) (the change, against `main` at `5dd1277d`, the scorers of #1658 at `6d498b8b`)
**Result:** negative for the 30 furniture the change draws on the BIM4LCA office, as RE-305 found: none reads as Revit's body, and the reading RE-397 and #1658 found for round sections does not explain them. Not shipped. The eight sauna doors of the housing it also draws read within every bar, which is no reason to ship a rule by class.

## 1. The gap

A cylinder face with more than one trim loop is refused (`multiple cylinder trim loops unsupported`), which keeps its element as a record box (RE-224). On the BIM4LCA office as built that is the whole reason for 30 `IfcFurniture` (`Stay_Wall_Mounted_:VAR-1`) and, on each housing file, for 4 `IfcDoor` (`O-5:O-5.9x21sauna`), and a part of the reason for 15 sanitary terminals of the office. On Autodesk's 2024 samples it is the largest reason after the instance masks: 404 air terminals, 135 pipe fittings, 108 sanitary terminals, 36 duct fittings, 34 doors (run 42906887386).

## 2. The loops

`probe_1645_cylinder_trim_loops` follows each exported element's instances into the graphics of the symbols they name (a furniture or a fitting keeps its faces there, not in its own record: run 42910718296 with the first version of the probe, which read only the elements' own graphs, found none) and lists the loops of every cylinder face that has several, by the depth of each loop in the others as the plane path takes it.

| File | Cylinder faces | With several loops | Arrangements |
|---|---:|---:|---|
| Housing, concrete as built | 1,653 | 18 | 2 outer rings, or an outer ring and a hole |
| Housing, timber as built | 1,865 | 26 | the same |
| Office as built; office timber as built | 4,580; 3,781 | 26; 26 | the same, and 12 more faces per file with a loop the ring reader refuses (`disconnected UV boundary`), which are not among the 26 |
| Autodesk's 2024 samples (16 models) | 43,079 | 175 | 108 two outer rings, 36 an outer ring and a hole, 26 an outer ring and two holes, 3 three outer rings, 2 an outer ring and nine holes (175 in all), and 30 more faces with a loop the ring reader refuses, which are not among the 175 |

None is nested deeper than a hole in an outer ring, so the plane path's reading (an outer ring of even depth, its holes of odd depth) covers every one that can be read.

## 3. The change

RE-305's mesher, archived at `0f377661` (`refs/archive/2026-11-03/research-1247-cylinder-holes`), rebased onto `main` and nothing else (`1d869996`, `refs/archive/2026-11-10/feat/cylinder-multi-loop`): each outer ring with the rings directly inside it as holes is earcut in arc-length coordinates, each triangle is cut exactly into strips across U at the grid's U values, and the pieces must cover the outer rings' area less the holes' to 1e-6, or the face is refused. The probe is `690e90cf` (`refs/archive/2026-11-10/probe/cylinder-trim-loops`).

## 4. Rows and result

Registered on #1645 before the change was run on anything; the result is on the same issue.

| Row | Result |
|---|---|
| The office's 30 furniture and the 4 sauna doors are drawn from their saved meshes (at least 32 of 34) | holds, with a correction: the doors are on the housing files, not the office. The 30 furniture are drawn (saved-mesh bodies 1,672 to 1,702 on the office as built, `IfcFurniture` 68 to 98, record boxes 168 to 138; the permit and timber files the same); the doors are drawn on both housing files (record boxes 76 to 72) |
| Of the 30 furniture, at least 27 read `equal` or `explained` shell by shell | **missed: 0 of 30** |
| Nothing else moves | holds on the scorer tables and the body sources (only the `IfcFurniture` and, on the housing, `IfcDoor` class rows change; the exports move on 5 of 5 by those bodies). Three `wall_cut_openings_left_out` counters moved on the office: they are #1655 in reverse, the head having been cut before it landed |

The furniture, per body: 2 or 3 of 5 shells differ, the body's volume is 1.1102 to 1.1105 of Revit's (a shell 0.9611 to 1.2900), Revit's vertices lie 0.0047 to 0.0051 ft from rvt-rs's surface and rvt-rs's up to 0.0118 ft from Revit's. RE-305 found 1.109 to 1.120, 0.0047 to 0.0051 and 0.0118 for the same 30. `fill` in `box_bodies_vs_ifc` was 0.062 for the box these replace, so the box was 16 times Revit's volume; the mesh is 1.11 times.

Not registered, observed: the eight sauna doors read 1.0004 in volume with every vertex within 0.0042 ft of Revit's surface and 0.0053 ft the other way.

## 5. What was ruled out for the furniture

- **Faces Revit's export writes twice** (the roofs and ramp flights that read 0.5000, #1658): no shell of body 1832497 repeats a triangle, in either file (0 of 8,438 triangles of rvt-rs's and 0 of 2,162 of Revit's).
- **A void rvt-rs does not cut:** the shell that reads 1.29 holds 4 planar faces with more than one boundary loop in rvt-rs's mesh and 4 in Revit's.
- **The arrangement of the loops:** the 30 are the arrangements of section 2, and the mesher's area check passed on every face.
- **A round section drawn as an inscribed polygon** (RE-397 section 5.2): the shells have no end face that is a circle, so the reading of #1658 has none to compare (`other`: `no round end face in rvt-rs's shell`).

## 6. Not shown

- **Why the shell at 1.29 differs.** It is a bar bent in places: Revit's export draws its curved faces as 604 groups of coplanar triangles of a few triangles each against rvt-rs's 2,406, and none of Revit's end faces is a single loop (faces with holes), so the end-face test has nothing to compare. That the excess is the tessellation of the bends is a reading and was not measured.
- **The 175 faces of the samples.** No export scores them. They are not evidence for or against the change.
- **That the eight doors are right for a reason.** A class is not a signal. Drawing doors but not furniture by their class would be a rule with no byte-level reason, so the change is not shipped for them; a door that reads within every bar is a candidate for a row of its own once something in the bytes tells it from the furniture.

## 7. Reproduce

```bash
gh workflow run measure.yml --ref main -f ref=1d86999611d9b9873aed927c38a3450b74e4058e -f base=main \
  -f nordic=true -f models="nordic-office-as-built nordic-office-permit nordic-office-timber-as-built nordic-housing-concrete-as-built nordic-housing-timber-as-built" \
  -f scorers="mesh_bodies_vs_ifc saved_meshes_vs_ifc box_bodies_vs_ifc" -f keep_ifc=true
# the loops: the probe at 690e90cf
gh workflow run measure.yml --ref main -f ref=690e90cf2a1f5c795d29dc08ca3d41d74769390d -f base=none -f nordic=true \
  -f models="nordic-*-as-built" -f scorers=none -f probe=probe_1645_cylinder_trim_loops -f probe_scorer=-
```

(`--ref main` has the scorers' reading of #1658 from the moment it lands; the second run needs a ref that holds the probe's `face_trim_loops` helper, which is the archived commit.)

## 8. Sources

RE-304, RE-305, RE-397 section 5.2, #1658. No Autodesk source was read.
