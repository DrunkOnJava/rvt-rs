# RE-327: A family's extrusions from their own sketch curves

**Date:** 2026-11-04
**Issues:** #156 (the sketch-to-solid pipeline); Discussion #112 (STE1200's team: sketch curve records to closed loops to extrusion parameters to solids, 85 of 86 extrusions of one family); follows RE-135, RE-163 and RE-204 (a loaded family's own document), RE-280 (the native framing on Revit 2015 to 2022) and RE-323 (`VarParam::m_pSubst`)
**Artefacts:** the `phi-ag/rvt` family corpus at bec25470413fbe659c3ab30d4cfd029b73d63fc3 (`examples/Autodesk/`, sha256 as RE-323 lists them: `rac_basic_sample_family-2016.rfa` fde80383…9601 to `racbasicsamplefamily-2027.rfa` 2df5452e…84f4); Autodesk's 58 sample projects of Revit 2015 to 2027 as `research/autodesk-sample-projects.tsv` pins them; the maker family `stertil-closed-pit-sp-xp-pp-2014.rfa` (Revit 2014, `research/public-families.tsv`, sha256 b3b46ded…de0c)
**Probe:** `probes/examples/probe_re327_family_extrusions.rs`; reader: `src/research/native_form_geometry.rs`; export: `src/ifc/family_extrusions.rs`
**Runs:** Measure 41776509790 and 41785847607 (what the records are); 41787574515, 41788969929, 41791166383 and 41793701387 (the rebuild against the saved meshes, the reader as it grew); 41797915963 (the probe at `7f13d1a3`) and 41797938617 (the export path at `7f13d1a3` on the 2023 to 2025 samples); 41801584312, 41801588969 and 41801593095 (byte identity, the flag and the probe at `b13be61a`); at `aaa38de8`, 41805327625 (byte identity), 41805333248 (the flag), 41805339819 (the same models with `--saved-meshes` alone) and 41805344172 (the probe); at `b482eba2`, 41808965272 (the flag) and 41808993073 (byte identity). §9 reads them.
**Result:** positive. The two extrusions of the family corpus rebuild from their sketch curves on all 12 releases (24 of 24 within 0.001 ft of the mesh Revit saved for each), and so do all 6,172 extrusions of the sample projects whose saved mesh decodes whole (the probe at `aaa38de8`, run 41805344172): 6,093 as the sketch plane's matrix gives its normal, and 79 with the normal reversed by a signature found on those 79 (not held out). That reversal runs by default in the reader; what tests it is the export, which checks every instance against its own mesh. Not reproduced: the 85 of 86 on STE1200's family, which is a Revit 2014 file the native framing does not read. The export path (`rvt-ifc --family-extrusions`, opt-in) first replaced 335 bodies on the ten models with a Revit export beside them. The body scorer matched 251 of them and 250 pass the saved-mesh bar (Revit's vertices within 0.001 ft of rvt-rs's surface); the 251st, a toilet-paper dispenser in RE1 Architecture, does not, and the other 84 are family instances written as spaces, scored by their plan profiles, which improve. The tolerances were then narrowed to those the sample projects' 6,014 printed forms keep, and §9 gives what the narrowed rule replaces.

## 1. The question

Discussion #112 reports that the team rebuilt 85 of 86 extrusions of a real family, in a CAD kernel, from the raw streams: sketch curve records, closed loops, extrusion parameters, solids. It gives the stages and the count, not the layouts, the loop rules, the placement or the oracle. This report finds the records on Revit 2015 to 2027, builds the solids and measures them against something independent of the build.

## 2. The records

A family file's elements are the plain groups of its own partitions, framed as a project's are (RE-280). `native_document` refuses every one of them (`invalid increment episode range`: its creation-episode index is a project's), so `native_form_geometry::walk` reads the groups without it. A loaded family's document in a project is the same records in keyed blocks (`native_family_forms`). The classes and fields below are the file's own `Formats/Latest`, decoded by the native graph reader; the same on every release 2015 to 2027 for the three classes the rebuild reads.

- `ExtrusionElem` (a `GenSweep`): `m_cutting` (a void), `m_materialId`, `m_alwaysRefPlaneNorm`; its `ExtrusionElemExtrusionHelper` names the sketch (`m_sketchId`) and stores the loops it was last built from (`m_pCurveLoops`: `CurveLoop`s of pointers to `GLine` and `GArc`). Its `ParamValueSetDouble` holds two parameters, -1001800 and -1001801: the extent along the sketch plane's normal. Every extrusion measured has -1001800 below -1001801, so which is the start is not shown, and the solid between them does not depend on it. Its `ParamValueSetInt` holds `Visible` (-1006205).
- `CurveElem`: one `GLine` (`m_origin`, `m_dirVec`, `m_endParams`: the parameter range) or `GArc` (`m_center`, `m_xVec`, `m_yVec`, `m_radius`, `m_endParams`: the angle range) through its `CurveElemDriver`; `SketchMembership.m_groupId` names its sketch and `m_sketchPlaneId` its plane. The curves are in the family's coordinates, lying in the plane, not in the plane's own: a curve is checked against its plane to 1e-5 ft. A circle is an arc whose two angles are equal (0, 0), and a drawn circle is stored by the extrusion record as two half arcs.
- `SketchPlane`: `Trf` (`m_3x3` and `m_or`) and `m_flipZ`.

The sketch itself (`VarSketch`, with `m_sketchPlaneId`, the constraints and the dimensions) is not read, and does not need to be. It decodes on every release from 2018, but on 2015, 2016 and 2017 the records that hold a `VarParam` are refused with `VarParam.m_pSubst: unsupported native modifier` (6 of its 7 records in the family corpus of 2016 and 2017; RE-323 §5: the field is `Unknown`, and neither layout tried is merged). That is a measured negative for `VarSketch` on those releases and costs the rebuild nothing: `ExtrusionElem`, `CurveElem` and `SketchPlane` decode without a refusal on all of them, and the extrusions rebuild.

## 3. The rebuild

1. The extrusion's helper names the sketch; the sketch's curves are the `CurveElem` records that name it (a sketch of more than 4,096 curves is refused before chaining).
2. `assemble_loops` chains them, end to end, into closed loops. Ends within 1e-4 ft join, and a gap is closed by a line (§5); a chain with two curves that could continue it is refused as branching.
3. `profile` sorts the loops into outlines and holes by nesting (even depth an outline, odd a hole) and refuses crossing loops.
4. The plane's frame: the columns of `m_3x3` are its x, y and normal axes, `m_or` its origin, and `m_flipZ` reverses the normal; a `SketchPlane` whose `m_flipZ` does not read is refused.
5. The solid is the profile swept along the normal between the two parameters.

The extrusion record's own stored loops are compared with the sketch's curves by where they lie (a drawn circle is one curve in the sketch and two in the record): they agree on 9,410 of the 9,424 built extrusions of the sample projects (run 41797915963) and 24 of 24 of the family corpus; 14 store none that read.

## 4. Measured against the saved mesh

The oracle is the form's own saved display geometry, its channel-103 `GElement` record, meshed by `native_saved_mesh`: a family's saved mesh is Revit's solid, tessellated, not derived from the sketch. A form matches when its volume is within 1% (and the most a chord-tessellated arc loses, two thirds of its arc length times 0.001 ft times the height), its bounding box within 0.01 ft, every mesh vertex within 0.01 ft of the rebuilt surface and samples of the rebuilt surface within 0.01 ft of the mesh.

**The family corpus** (12 releases, 2 extrusions each: a 2.5 ft tabletop with rounded corners, and four round legs as four disjoint loops in one extrusion):

| | forms | match | volume ratio | bounding box | vertices | surface samples |
|---|---:|---:|---|---|---|---|
| tabletop | 12 | 12 | 1.00009 | 0.000000 | 0.000000 | 0.000713 |
| legs | 12 | 12 | 1.00739 | 0.000911 | 0.000000 | 0.000911 |

The legs' 0.74% is the mesh's own chords (0.001 ft against a radius of 0.125 ft). The rows are identical on all 12 releases, so the readings of §2 hold from 2016 to 2027. Every plane in the corpus is the identity, which cannot tell columns from rows.

**The sample projects' families** (58 files, Revit 2015 to 2027; the same families recur in every release's files, so the counts below are form-and-file pairs, not independent forms; the probe at `aaa38de8`, run 41805344172):

| | forms |
|---|---:|
| extrusions in loaded families' documents | 9,587 |
| built | 9,423 |
| of those, with a saved mesh | 6,201 |
| complete mesh | 6,172 |
| of those, matching | 6,172 (6,093 as read, 79 with the normal reversed, §5) |
| partial mesh (outside the verdict) | 29 (12 match, 17 differ) |
| built, no saved mesh | 3,222 (902 voids, 2,320 others) |
| not built | 164 (137 `GEllipse`, 26 open loops, 1 up-facing flipped plane) |

The 79 are the run 41797915963's "reversed MATCH, as read DIFFER 79": the probe at `7f13d1a3` refused them and measured the reversed normal on them, and the reader applies it by default (§5). Of 984 built voids, 82 have a saved mesh and all 82 match. By release the extrusions that match are 597 (2015), 287 (2016), 288 (2017, 2018 and 2020 to 2023), 579 (2019), 1,352 (2024, which has the Snowdon Towers projects), and 543 (2025 to 2027); the 2015, 2019 and 2024 files hold projects the others do not. The probe prints at most 400 form lines of a file and always prints the tallies these counts come from; the distributions in §7 are over the 6,014 printed lines of the 6,172 matches.

**Axis convention.** The matrix's columns are the plane's axes: at `aaa38de8`, 6,104 of the 6,184 forms that match under the reader's default match under columns and 3,781 under rows (run 41805344172); in run 41797915963, 1,846 forms matched only under columns, 3,173 under both (planes symmetric in the way that matters), and none only under rows. A mirror (determinant -1) was tried with the normal as the cross product of x and y: no plane in the corpus is one, all 6,172 have determinant +1. A flipped plane's `m_flipZ` applies: all 164 flipped planes with a complete mesh match with the flip, and none matches only without it.

## 5. What does not match as read, and what was closed

**The down-facing signature (79 forms).** Every extrusion that did not match its complete saved mesh as read has the same three properties: its plane's third column is (0, 0, -1) to 0.001, the plane is not flipped (`m_flipZ` false), and the extrusion's `m_alwaysRefPlaneNorm` is false. All 79 do, none of the 6,093 matches does: down-facing planes with `m_alwaysRefPlaneNorm` set (131 forms) or `m_flipZ` set (99 forms) match as read. The volume was right and the solid sat on the wrong side of the plane, displaced by start plus end. With the normal reversed all 79 match (run 41797915963), and the reader reverses it for that signature, `Extrusion::down_facing_unreferenced`, and for no other form; where the plane is down-facing and unflipped but `m_alwaysRefPlaneNorm` does not read at all, the form is refused, since the rule is measured on the field's values and not on its absence. Three things bound the claim. The rule was found on these 79, so they do not test it; the held-out check is the export's own (§7), against forms of the other models. The 10 forms that have the signature and no saved mesh are reversed by the rule and are unmeasured. And the signature is the exact -z column: a plane tilted off -z is read as the matrix gives it, and an export checks each solid against its mesh. The sketch's own `Plane` normal (x cross y of its `VarSketch`'s `Plane`) does not explain the rule: it is opposite the matrix's on all 65 of the 79 that have one and also on 112 forms that match as read; `VarSketch.m_flipXY` was not tried against them.

**One up-facing flipped plane (refused).** One 2024 form, with a complete mesh, has a plane whose third column is +z, flipped, and differs from its mesh under the default convention (it matched under another one only). The reader refuses an up-facing flipped plane ("up-facing sketch plane that is flipped"): the 6,172 that match are unchanged, so the refusal cost none of them, Unread `m_flipZ` and unread `m_alwaysRefPlaneNorm` (on the down-facing signature) are refused as well, and the head's probe shows the same 6,172 matches.

**Open loops (66 forms, now 26).** At a join tolerance of 1e-6 ft, sketches stay open; the four dumped showed gaps of 1.36e-5 (two forms), 1.53e-5 and 6.49e-5 ft. The reader now joins ends within 1e-4 ft and closes the gap with a line: that built 40 of the 66, none of which has a saved mesh, so the join is not measured against an oracle, and none of the 6,093 forms that matched changed. The 26 left (2 forms in each of the 13 releases) are a single open curve, with no other end to join.

**Ellipses (137 forms).** `GEllipse` is not drawn; the form is refused by name.

**Partial meshes (29).** A saved mesh with a face that did not decode (RE-224) is not an oracle: 12 match and 17 differ, and neither goes in the count above.

**Voids and meshless forms.** 902 of the 984 built voids and 2,320 other extrusions have no saved mesh (hidden by a parameter, or a void, which draws nothing): they are built and unmeasured.

## 6. The 2014 family

STE1200's family (Revit 2014, 86 extrusions) is in `public-families.tsv` and Measure fetches it, but 2014 is outside `NATIVE_RELEASES` (2015 to 2027): the framing is refused with `native record framing is unvalidated for Revit 2014`. The 85 of 86 is therefore not reproduced on its own subject. What this report shows is that the stages Discussion #112 names exist on every release 2015 to 2027 with the layouts above, and that the rebuild they describe holds for every extrusion whose saved mesh decodes whole (6,172 of 6,172, one rule found on the data).

## 7. What changes

- `native_form_geometry`: a family file's records without the project index; the sketch chain and the solid; `mesh_agreement`.
- `native_family_forms::family_records_in`: `family_records` on any native release, and only for the given ids.
- `rvt-ifc --family-extrusions` (off by default, implies `--saved-meshes`; the default export is unchanged, §9): a family instance whose body is its saved mesh is drawn from its family's extrusions, as `IfcExtrudedAreaSolid`s over `IfcIndexedPolyCurve` profiles (a drawn circle two arcs), where they reproduce the mesh. The extrusions are placed by the instance's transform (RE-87), then into the element's frame as `saved_meshes::attach` puts the mesh there (the model's vertices less the element's location, turned by minus its rotation about the vertical axis), so the mesh and the extrusions are in one frame for any transform, tilted or reflected as well; an instance with no transform that reads keeps its mesh. The check is made for each instance on its own (a placed instance is drawn at its type's values, and the family's document holds the family as saved), with tolerances taken from the sample projects' own forms: over the 6,014 printed forms that match their complete mesh, the largest distance from a mesh vertex to the extrusions' surface is under 5e-7 ft (the export allows 1e-4 ft, the single-precision rounding of a mesh far from the origin), the largest distance from a sampled point of the surface to the mesh is 0.000998 ft (the export allows 1.1e-3 ft, the 0.001 ft chords Revit tessellates arcs with, and a tenth more), and the volume difference never exceeds the arcs' chord loss, above or below (the export allows that and 0.01% of the mesh's volume). An instance whose family has a void, a form that is not a rebuilt extrusion, or an unread hidden form, or whose mesh the extrusions do not reproduce, keeps its mesh. The first filtered run of it (41797938617, `7f13d1a3`, which placed the extrusions in the family's own frame) replaced 2 of 2,337 bodies on Snowdon Towers' architectural model and none of the others' tens of thousands: the check refused what it should, and the placement, not the geometry, was why. The tolerances were first 5e-4 ft, 1.5e-3 ft and 1% of the volume; run 41805333248 showed that they were looser than the data (§9), and `b482eba2` narrows them.

## 8. Not done

Sweeps, revolutions, blends, swept blends, free-form solids and `GeomCombination` (the other forms of `FORM_CLASSES`), voids cutting solids, ellipses and splines, a down-facing plane tilted off -z whose normal the matrix gives wrongly (an export checks it against the mesh), parametric instances of a type other than the one the family was saved at (each instance is checked against its own mesh), extrusions in a Revit 2014 file, `VarSketch` on 2015 to 2017.

## 9. Measured against Revit's own exports

**Byte identity.** With no flag, the default export is the bytes it was. Run 41805327625 (`aaa38de8` against `main` at `4810c4d2`, `-f base=main -f scorers=none -f holdout=true -f nordic=true`, the models `core-interior re1-* einhoven nordic-office-* holdout-* spent-*`): 29 models have an `ifc.sha256` on both sides, 29 are the same and none moved. They are Core Interior, Einhoven, the four RE1 models, the four BIM4LCA office files, 18 held-out models and the spent Vitoria pair; the held-out ones are compared by hash only, and the six that the unfiltered run at `b13be61a` (41801584312: 105 models the same, none moved) showed on the head side only are among the 29. Run 41808993073 repeats it at `b482eba2`.

**The flag.** Run 41805333248 (`aaa38de8`, `-f base=none -f nordic=true`, the same models, `-f ifc_flags="--saved-meshes --family-extrusions"`, every IFC scorer and `family_extrusion_bodies_vs_ifc`, which is `mesh_bodies_vs_ifc` on the bodies whose `BodySource` is `family_extrusions`) against run 41805339819 (the same with `--saved-meshes` alone). Replaced bodies, and what `rvt-ifc` printed beside them:

| model | candidates | replaced | scored | ref on ours | no rebuilt family | no transform | volume | vertices | surface | export ms with / without |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| Core Interior | 138 | 37 | 37 | 37 | 0 | 0 | 6 | 95 | 0 | 7,621 / 7,831 |
| Einhoven | 11 | 0 | 0 | | 0 | 11 | 0 | 0 | 0 | 352 / 265 |
| BIM4LCA office, as built | 11,078 | 129 | 87 (+42 spaces) | 87 | 10,444 | 12 | 479 | 8 | 6 | 40,211 / 36,554 |
| BIM4LCA office, permit | 11,193 | 129 | 87 (+42 spaces) | 87 | 10,467 | 19 | 564 | 8 | 6 | 38,531 / 33,893 |
| office terrain, concrete | 19 | 11 | 11 | 11 | 8 | 0 | 0 | 0 | 0 | 2,920 / 2,478 |
| office terrain, timber | 18 | 11 | 11 | 11 | 7 | 0 | 0 | 0 | 0 | 2,464 / 2,271 |
| RE1 Architecture | 53 | 8 | 8 | **7** | 24 | 0 | 15 | 5 | 1 | 3,009 / 2,512 |
| RE1 Electrical | 47 | 8 | 8 | 8 | 12 | 0 | 27 | 0 | 0 | 3,491 / 3,282 |
| RE1 Mechanical | 68 | 1 | 1 | 1 | 60 | 0 | 7 | 0 | 0 | 1,371 / 1,506 |
| RE1 Plumbing | 60 | 1 | 1 | 1 | 53 | 0 | 6 | 0 | 0 | 1,583 / 1,410 |

(0 unchecked, 0 unwritable and 0 ambiguous in every model.) "Scored" is what the body scorer matched to a body in Revit's export; the 42 others of each office file are family instances written as `IfcSpace`, which `space_profiles_vs_ifc` scores. Export milliseconds are one run of each, with the other exports of the run going at once, so they compare machine load as much as code; the flag's change runs from -9% (RE1 Mechanical) to +20% (RE1 Architecture), the office files +10% and +14%, Core Interior -3%, and Einhoven's 87 ms is noise on a 0.3 s export. No slowdown that matters.

The scorer lines, flag against baseline: every scorer output is identical except three. `mesh_bodies_vs_ifc` and `saved_meshes_vs_ifc` score fewer saved-mesh bodies, the replaced ones moving to `family_extrusion_bodies_vs_ifc`; the combined rows are the baseline's except where they improve, and RE1 Architecture's "ref on ours" (the blocker): the office files' bodies within 0.01% of Revit's volume go from 1,547 and 1,633 to 1,625 and 1,711 of 1,650 and 1,738, and within 0.001 ft in both directions from 1,554 and 1,638 to 1,632 and 1,716; the terrain files' from 1 and 5 of 16 to 12 and 16 of 16; Electrical's from 31 to 37 of 47. And `space_profiles_vs_ifc`: the vertices within 0.01 ft of Revit's go from 117 to 159 (as built) and from 115 to 157 (permit), every other line of it unchanged.

**The blocker.** One replaced body fails the existing saved-mesh bar, "ref on ours" (every vertex of Revit's body within 0.001 ft of rvt-rs's surface): RE1 Architecture's element 360331, an `IfcBuildingElementProxy` 'Dispenser, Toilet Paper'. As its saved mesh it scored volume ratio 1.0047 and Revit's vertices 0.0009143 ft from the mesh (a pass by 9%); as extrusions it scores volume ratio 1.0006 and Revit's vertices 0.001439 ft from the surface. Revit's own export of it is not its saved mesh (Revit's vertices 0.0009 ft from the mesh, the mesh's from Revit's surface 0.0016 ft), and what the export allowed between the mesh and the extrusions, 0.0005 ft at a vertex and 1% of the volume (here the extrusions are 0.4% under the mesh), added to that passes the bar: an inference from the arithmetic, since the export does not report a body's own distances. The other 250 bodies the scorer matched have Revit's vertices within 3.4e-5 ft of the surface or better. RE1 Architecture's combined "ref on ours" is 49 of 53 against the baseline's 50 of 53, and no other model's combined row is below the baseline's in any column. This is a blocker under the rule that a replaced body must pass the bar the saved mesh passes, so `b482eba2` narrows the tolerances to the data's own (§7), not fitted to this body: nothing in the sample projects' forms needs more than 1e-4 ft at a vertex, 1.1e-3 ft at a surface sample, or more than the chord loss in volume. Whether the dispenser is now refused, and what else the narrowing leaves out, is read from run 41808965272 (below).

**The probe at `aaa38de8`** (41805344172, `-f base=none -f probe=probe_re327_family_extrusions -f families=true -f samples=true -f sample_years="2015 2027" -f scorers=none -f models=-`): the tallies of §4. The family corpus 24 of 24; the samples 9,587 forms, 9,423 built, 6,172 matching their complete mesh, none differing, 29 partial (12 match, 17 differ), 164 not built (137 `GEllipse`, 26 open loops, 1 up-facing flipped plane). The refusal of an unread `m_flipZ` cost none of the 6,172.

**Reproduce.**

```bash
gh workflow run measure.yml --ref main -f ref=aaa38de808f1731acf6f6a6dace59904f873335b -f base=none \
  -f probe=probe_re327_family_extrusions -f families=true -f samples=true \
  -f sample_years="2015 2027" -f scorers=none -f models=-
gh workflow run measure.yml --ref re/156-family-extrusions -f ref=<sha> -f base=none -f nordic=true \
  -f models="core-interior re1-* einhoven nordic-office-*" -f ifc_flags="--saved-meshes --family-extrusions" \
  -f scorers="family_extrusion_bodies_vs_ifc mesh_bodies_vs_ifc saved_meshes_vs_ifc space_profiles_vs_ifc"
gh workflow run measure.yml --ref main -f ref=<sha> -f base=main -f scorers=none -f holdout=true -f nordic=true \
  -f models="core-interior re1-* einhoven nordic-office-* holdout-* spent-*"
```

Each `family-<file>/probe.txt` and `sample-<file>/probe.txt` ends in `tally` lines. The probe runs are at the commits `21d6f874`, `e8f3afe9`, `dd5b49d0`, `44f0159b`, `1391a2a8`, `60151cd4`, `7f13d1a3`, `b13be61a` and `aaa38de8`. The scorer comes from `--ref` (a branch), the code from `-f ref` (a commit).

## 10. Credit

The chain is STE1200's team's (Discussion #112): sketch curve records, closed loops, extrusion and sweep parameters, solids, and the observation that Revit's feet are `mm / 25.4 / 12` (RE-310 measured it). jakobhirn-bit's comment there (a flag word at `+0x46` whose bits mark a driven form and a solid one, and the kind of a solid from the sketches under a form) was a lead; this report reads the schema's named fields instead (`m_cutting`, `Visible`) and did not measure the flag word's bits or the sketch-count rule.
