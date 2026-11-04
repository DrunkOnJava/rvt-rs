# RE-327: A family's extrusions from their own sketch curves

**Date:** 2026-11-04
**Issues:** #156 (the sketch-to-solid pipeline); Discussion #112 (STE1200's team: sketch curve records to closed loops to extrusion parameters to solids, 85 of 86 extrusions of one family); follows RE-135, RE-163 and RE-204 (a loaded family's own document), RE-280 (the native framing on Revit 2015 to 2022) and RE-323 (`VarParam::m_pSubst`)
**Artefacts:** the `phi-ag/rvt` family corpus at bec25470413fbe659c3ab30d4cfd029b73d63fc3 (`examples/Autodesk/`, sha256 as RE-323 lists them: `rac_basic_sample_family-2016.rfa` fde80383…9601 to `racbasicsamplefamily-2027.rfa` 2df5452e…84f4); Autodesk's 58 sample projects of Revit 2015 to 2027 as `research/autodesk-sample-projects.tsv` pins them; the maker family `stertil-closed-pit-sp-xp-pp-2014.rfa` (Revit 2014, `research/public-families.tsv`, sha256 b3b46ded…de0c)
**Probe:** `probes/examples/probe_re327_family_extrusions.rs`; arcs (§11): `tools/re/arc_tessellation_study.py` and the census of `tools/re/family_extrusion_bodies_vs_ifc.py`; reader: `src/research/native_form_geometry.rs`; export: `src/ifc/family_extrusions.rs`
**Runs:** Measure 41776509790 and 41785847607 (what the records are); 41787574515, 41788969929, 41791166383 and 41793701387 (the rebuild against the saved meshes, the reader as it grew); 41797915963 (the probe at `7f13d1a3`) and 41797938617 (the export path at `7f13d1a3` on the 2023 to 2025 samples); 41801584312, 41801588969 and 41801593095 (byte identity, the flag and the probe at `b13be61a`); at `aaa38de8`, 41805327625 (byte identity), 41805333248 (the flag), 41805339819 (the same models with `--saved-meshes` alone) and 41805344172 (the probe); at `b482eba2`, 41808965272 (the flag) and 41808993073 (byte identity); at `0b286d83`, 41811170320 (the flag) and 41811173603 (byte identity); at the merged head `9f3814f6`, 41813677577 (byte identity); 41810287027 (the probe at `b482eba2` on RE1 Architecture alone); and at `9412048f`, after the review fixes, 41816308266 (byte identity, every model) and 41816314683 (the flag). §9 reads them. At `d267612f` (§11, #1346, short arcs, dropped), 41832799725 (the flag) and 41832804165 (byte identity, every model); at `681e42dd`, 41835957331 (the flag) and 41835962201 (the baseline); at `6dbcae8d`, 41836478995 (the flag) and 41836483266 (byte identity); at `e5b4bda6`, the final rule, 41838483706 (the flag), 41838489013 (the baseline) and 41838493193 (byte identity).
**Result:** positive for the rebuild, and for the opt-in export on lines. The two extrusions of the family corpus rebuild from their sketch curves on all 12 releases (24 of 24 within 0.001 ft of the mesh Revit saved for each), and so do all 6,172 extrusions of the sample projects whose saved mesh decodes whole (the probe at `aaa38de8`, run 41805344172): 6,093 as the sketch plane's matrix gives its normal, and 79 with the normal reversed by a signature found on those 79 (not held out). That reversal runs by default in the reader; what tests it is the export, which checks every instance against its own mesh. Not reproduced: the 85 of 86 on STE1200's family, which is a Revit 2014 file the native framing does not read. The export path (`rvt-ifc --family-extrusions`, opt-in) first replaced 335 bodies on the ten models with a Revit export beside them; 250 of the 251 the body scorer matched passed the saved-mesh bar, and one, a toilet-paper dispenser in RE1 Architecture, did not. Narrowing its tolerances to the sample projects' own changed nothing (run 41808965272): the extrusions agree with the saved mesh. The volumes suggested the difference was where Revit's tessellation of an arc starts against ours; §11 measures it, and it is not the start: Revit's export draws that circle as an `IfcCircleProfileDef`, IfcOpenShell cuts it into 27 chords, and the two half arcs rvt-rs wrote come out as 28, so no vertex coincides. Instances with an arc in a profile first kept their mesh (`0b286d83`), which left 271 of the 335 out; the 64 that remained (37 of Core Interior's doors, 9 in each BIM4LCA office file, 6 in RE1 Architecture, 2 in Electrical, 1 in Mechanical) all had Revit's vertices within 0.001 ft of their surface, and every scorer row equalled the saved-mesh baseline's (§9). #1346 (§11) draws the arcs again, as arcs, as the first arc run did, except a loop that is one whole circle over 0.12 ft in radius, whose instance keeps its mesh in every release: the dispenser is a difference between Revit's two exporters (Revit 26.2's IFC2X3 export writes the circle as an `IfcCircleProfileDef`, Revit 24.0.5's IFC4 export as arcs), and IfcOpenShell's default tessellation of the two forms differs by one chord at those radii, so no one form meets both (the bar's tessellation is #1355).

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
- `rvt-ifc --family-extrusions` (off by default, implies `--saved-meshes`; the default export is unchanged, §9): a family instance whose body is its saved mesh is drawn from its family's extrusions, as `IfcExtrudedAreaSolid`s over `IfcIndexedPolyCurve` profiles, where they reproduce the mesh; an arc in a profile is written as the arc, and an instance with a loop that is one whole circle over 0.12 ft in radius keeps its mesh (§11, #1355). The extrusions are placed by the instance's transform (RE-87), then into the element's frame as `saved_meshes::attach` puts the mesh there (the model's vertices less the element's location, turned by minus its rotation about the vertical axis), so the mesh and the extrusions are in one frame for any rotation of the instance, tilted ones included. An instance with no transform that reads keeps its mesh: the transform is read on Revit 2024 to 2027 only (`element_data_header` is measured for those releases), so a file before 2024 has none (Einhoven: 11 candidates, 11 unplaced, 0 replaced), and only as a rotation, so a reflected instance's is never read (all 6,172 planes measured have determinant +1, so a reflected plane is unmeasured too). The check is made for each instance on its own (a placed instance is drawn at its type's values, and the family's document holds the family as saved), with tolerances taken from the sample projects' own forms: over the 6,014 printed forms that match their complete mesh, the largest distance from a mesh vertex to the extrusions' surface is under 5e-7 ft (the export allows 1e-4 ft, the single-precision rounding of a mesh far from the origin), the largest distance from a sampled point of the surface to the mesh is 0.000998 ft (the export allows 1.1e-3 ft, the 0.001 ft chords Revit tessellates arcs with, and a tenth more), and the volume difference never exceeds the arcs' chord loss, above or below (the export allows that and 0.01% of the mesh's volume). Only a family's `ExtrusionElem`, `CurveElem` and `SketchPlane` records are read. A family with a void (an extrusion that cuts), or an extrusion the reader does not rebuild (an ellipse, an unclosed sketch, a plane or a field it does not read), or none left once the extrusions whose `Visible` parameter is 0 are set aside, draws nothing. Any other form a family holds (a sweep, a blend), and an extrusion whose `Visible` parameter does not read (taken as visible), are not seen; it is the check of each instance against its own mesh that refuses an instance they make differ. The first filtered run of it (41797938617, `7f13d1a3`, which placed the extrusions in the family's own frame) replaced 2 of 2,337 bodies on Snowdon Towers' architectural model and none of the others' tens of thousands: the check refused what it should, and the placement, not the geometry, was why. The tolerances were first 5e-4 ft, 1.5e-3 ft and 1% of the volume; `b482eba2` narrowed them to these, which changed no replaced body (§9).

## 8. Not done

Revit 2023 and earlier (no instance transform reads there, so every candidate is `unplaced`), a reflected instance (its transform is never read), sweeps, revolutions, blends, swept blends, free-form solids and `GeomCombination` (the other forms of `FORM_CLASSES`), voids cutting solids, ellipses and splines, a down-facing plane tilted off -z whose normal the matrix gives wrongly (an export checks it against the mesh), parametric instances of a type other than the one the family was saved at (each instance is checked against its own mesh), extrusions in a Revit 2014 file, `VarSketch` on 2015 to 2017.

## 9. Measured against Revit's own exports

**Byte identity.** With no flag, the default export is the bytes it was. Run 41805327625 (`aaa38de8` against `main` at `4810c4d2`, `-f base=main -f scorers=none -f holdout=true -f nordic=true`, the models `core-interior re1-* einhoven nordic-office-* holdout-* spent-*`): 29 models have an `ifc.sha256` on both sides, 29 are the same and none moved. They are Core Interior, Einhoven, the four RE1 models, the four BIM4LCA office files, 18 held-out models and the spent Vitoria pair; the held-out ones are compared by hash only, and the six that the unfiltered run at `b13be61a` (41801584312: 105 models the same, none moved) showed on the head side only are among the 29. Runs 41808993073 (`b482eba2`), 41811173603 (`0b286d83`) and 41813677577 (`9f3814f6`, after merging `main` at `999fc7b3`, the v0.14.0 preparation) repeat it: 29 of 29 the same, none moved, each. Run 41816308266 (`9412048f`, the head, `-f base=main -f nordic=true -f r2025=true -f samples=true -f holdout=true -f families=true -f scorers=none`, no model filter) takes every model: 111 have an `ifc.sha256` on both sides, 111 are the same and none moved.

**The flag, first run.** Run 41805333248 (`aaa38de8`, `-f base=none -f nordic=true`, the same models, `-f ifc_flags="--saved-meshes --family-extrusions"`, every IFC scorer and `family_extrusion_bodies_vs_ifc`, which is `mesh_bodies_vs_ifc` on the bodies whose `BodySource` is `family_extrusions`) against run 41805339819 (the same with `--saved-meshes` alone). It drew arcs as arcs and allowed 5e-4 ft at a vertex, 1.5e-3 ft at a surface sample and 1% of the volume. Replaced bodies, and what `rvt-ifc` printed beside them:

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

"Scored" is what the body scorer matched to a body in Revit's export; the 42 others of each office file are family instances written as `IfcSpace`, which `space_profiles_vs_ifc` scores. The scorer lines, flag against baseline, were identical except where they improved: the office files' bodies within 0.01% of Revit's volume went from 1,547 and 1,633 to 1,625 and 1,711 of 1,650 and 1,738, and within 0.001 ft in both directions from 1,554 and 1,638 to 1,632 and 1,716; the terrain files' from 1 and 5 of 16 to 12 and 16 of 16; Electrical's from 31 to 37 of 47; the space profiles' vertices within 0.01 ft from 117 and 115 to 159 and 157. One line got worse, RE1 Architecture's "ref on ours" (below).

**The blocker.** One replaced body failed the existing saved-mesh bar, "ref on ours" (every vertex of Revit's body within 0.001 ft of rvt-rs's surface): RE1 Architecture's element 360331, an `IfcBuildingElementProxy` 'Dispenser, Toilet Paper'. As its saved mesh it scored volume ratio 1.0047 and Revit's vertices 0.0009143 ft from the mesh (a pass by 9%); as extrusions it scored volume ratio 1.0006 and Revit's vertices 0.001439 ft from the surface, and RE1 Architecture's combined "ref on ours" was 49 of 53 against the baseline's 50 of 53. The other 250 bodies the scorer matched had Revit's vertices within 3.4e-5 ft of the surface or better (most within 1e-5 ft): the bodies either coincide with Revit's tessellation or are about 1e-3 ft off it.

**The tolerances were not the cause.** The sample projects' 6,014 printed forms that match their own complete mesh keep every mesh vertex within 5e-7 ft of the extrusions, every surface sample within 0.000998 ft of the mesh and the volume within the arcs' chord loss, so `b482eba2` narrowed the export's tolerances to 1e-4 ft, 1.1e-3 ft and the chord loss with 0.01%. Run 41808965272 replaced the same 335 bodies (the vertex and volume refusals moved by one in each office file and in Architecture), and the dispenser stayed at 0.001439 ft. The extrusions therefore agree with its saved mesh to within 1e-4 ft at every vertex; what differs is Revit's export from that mesh. The volumes say how. RE1 Architecture's probe (run 41810287027) has one family of dispenser-like forms with arcs, family 360350 (family 373084 is the same at another size); if it is the dispenser's, its three forms have a true volume 1.0013 times their meshes', and the mesh is 1.0047 times Revit's export, so the export is 0.4% under the mesh while IfcOpenShell's tessellation of our true arcs is 0.06% over the export's: the export is a coarse tessellation of arcs of its own, as the scorer sees ours, and the two polygons have their vertices at different angles. Where both start an arc at the same point they coincide (the 250, at about 1e-6 ft); where they do not, a vertex of one is up to a chord's sagitta from the other. That is an inference from the volumes, not a measurement of the dispenser's profile, which the export does not report. §11 measures it and finds a different cause: the count of chords (27 and 28), not where they start.

**The flag, with arcs left out.** The existing bar says a replaced body must hold what its mesh held, and the file does not show where Revit starts an arc (the cause above is an inference), so `0b286d83` leaves an instance with an arc in any profile with its mesh. Run 41811170320 (the same command as the first run, scorers included) against the same baseline 41805339819:

| model | candidates | replaced | ref on ours | arc, kept mesh | no rebuilt family | no transform | volume | vertices | surface | export ms with / without |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| Core Interior | 138 | 37 | 37 of 37 | 0 | 0 | 0 | 6 | 95 | 0 | 8,042 / 7,831 |
| Einhoven | 11 | 0 | | 0 | 0 | 11 | 0 | 0 | 0 | 340 / 265 |
| BIM4LCA office, as built | 11,078 | 9 | 9 of 9 | 120 | 10,444 | 12 | 480 | 7 | 6 | 40,315 / 36,554 |
| BIM4LCA office, permit | 11,193 | 9 | 9 of 9 | 120 | 10,467 | 19 | 565 | 7 | 6 | 38,434 / 33,893 |
| office terrain, concrete | 19 | 0 | | 11 | 8 | 0 | 0 | 0 | 0 | 2,465 / 2,478 |
| office terrain, timber | 18 | 0 | | 11 | 7 | 0 | 0 | 0 | 0 | 2,508 / 2,271 |
| RE1 Architecture | 53 | 6 | 6 of 6 | 2 | 24 | 0 | 16 | 4 | 1 | 3,001 / 2,512 |
| RE1 Electrical | 47 | 2 | 2 of 2 | 6 | 12 | 0 | 27 | 0 | 0 | 4,500 / 3,282 |
| RE1 Mechanical | 68 | 1 | 1 of 1 | 0 | 60 | 0 | 7 | 0 | 0 | 1,318 / 1,506 |
| RE1 Plumbing | 60 | 0 | | 1 | 53 | 0 | 6 | 0 | 0 | 1,566 / 1,410 |

(0 unchecked, 0 unwritable and 0 ambiguous in every model.) 64 bodies are replaced and all 64 pass "ref on ours"; every combined row of `mesh_bodies_vs_ifc` and `saved_meshes_vs_ifc` equals the baseline's in every column, and the other scorers, `space_profiles_vs_ifc` included, are identical to it. Nothing got worse and, with the arcs left out, nothing got better either: the lines are as exact as the mesh they replace. What the rule leaves out is 271 of the 335 bodies of the first run (office files 120 each, terrain files 11 each, Architecture 2, Electrical 6, Plumbing 1), of which 270 held the bar and gained what the first run's paragraph lists. Export milliseconds are one export each, going at once with the others, so they compare machine load as much as code: Electrical's export took 3,491, 2,714 and 4,500 ms in the three flag runs against 3,282 without it, Core Interior's 7,621, 6,035 and 8,042 against 7,831, and the office files' flagged exports ran from 11% under to 14% over theirs. The spread between the runs is as large as any difference the flag makes: no slowdown that matters.

Run 41816314683 repeats it at `9412048f`, after the review fixes (the flag's cancellation, the bounds on hostile sketches and meshes, a replaced space's `SpaceOutlineSource` set to `family_body`): the same 64 bodies replaced, 0 unchecked, the same refusals model by model, every combined row equal to the baseline's, and `space_profiles_vs_ifc` identical to it. None of the 64 is an `IfcSpace` (the body scorer matched all 64), so the marker change is not exercised by these models; the 42 spaces per office file that the first run replaced had arcs.

**Not done, and why.** Writing the arcs as short sub-arcs (a chord within about 2.5e-4 ft each) would bound "ref on ours" at that sagitta whatever the phase of Revit's tessellation, since Revit's vertices lie on the true arc, but it gives the exact coincidence and the first run's gains back, and could move the both-directions 0.001 ft column; starting each loop where the extrusion record's stored loops start (`m_pCurveLoops`) is a guess about Revit's order. Neither was run as written; §11 (#1346) measures the cause, draws the arcs again and says how.

**The probe at `aaa38de8`** (41805344172, `-f base=none -f probe=probe_re327_family_extrusions -f families=true -f samples=true -f sample_years="2015 2027" -f scorers=none -f models=-`): the tallies of §4. The family corpus 24 of 24; the samples 9,587 forms, 9,423 built, 6,172 matching their complete mesh, none differing, 29 partial (12 match, 17 differ), 164 not built (137 `GEllipse`, 26 open loops, 1 up-facing flipped plane). The refusal of an unread `m_flipZ` cost none of the 6,172.

**Reproduce.**

```bash
gh workflow run measure.yml --ref main -f ref=aaa38de808f1731acf6f6a6dace59904f873335b -f base=none \
  -f probe=probe_re327_family_extrusions -f families=true -f samples=true \
  -f sample_years="2015 2027" -f scorers=none -f models=-
gh workflow run measure.yml --ref main -f ref=9412048f7414596ff5e6e558c75cc52c68353709 -f base=none -f nordic=true \
  -f models="core-interior re1-* einhoven nordic-office-*" -f ifc_flags="--saved-meshes --family-extrusions" \
  -f scorers="family_extrusion_bodies_vs_ifc mesh_bodies_vs_ifc saved_meshes_vs_ifc space_profiles_vs_ifc"
gh workflow run measure.yml --ref main -f ref=9412048f7414596ff5e6e558c75cc52c68353709 -f base=main -f nordic=true \
  -f r2025=true -f samples=true -f holdout=true -f families=true -f scorers=none
```

Each `family-<file>/probe.txt` and `sample-<file>/probe.txt` ends in `tally` lines. The probe runs are at the commits `21d6f874`, `e8f3afe9`, `dd5b49d0`, `44f0159b`, `1391a2a8`, `60151cd4`, `7f13d1a3`, `b13be61a` and `aaa38de8` (run 41810287027 is the probe at `b482eba2` on RE1 Architecture alone, `-f models=re1-architecture`). The scorer comes from `--ref` (`main`, which has `family_extrusion_bodies_vs_ifc` once this lands), the code from `-f ref` (a commit); the runs above named every IFC scorer of `tools/ci/measure-reference-models.sh` and `family_extrusion_bodies_vs_ifc`, and the command shows the four that matter.

## 10. Credit

The chain is STE1200's team's (Discussion #112): sketch curve records, closed loops, extrusion and sweep parameters, solids, and the observation that Revit's feet are `mm / 25.4 / 12` (RE-310 measured it). jakobhirn-bit's comment there (a flag word at `+0x46` whose bits mark a driven form and a solid one, and the kind of a solid from the sketches under a form) was a lead; this report reads the schema's named fields instead (`m_cutting`, `Visible`) and did not measure the flag word's bits or the sketch-count rule.

## 11. Arcs in a profile, again (#1346)

**Date:** 2026-11-04. **Artefacts:** the RE-32 oracle pair of RE1 Architecture, `RE1-Architecture.ifc` (Revit's export, sha256 a9b5d366…6200) and `RE1-Architecture.rvt` (sha256 4a78bdd6…8ce4); IfcOpenShell 0.9.0.post1, the version `tools/ci/requirements-ifc.txt` pins. **Reproduce:** `python3 tools/re/arc_tessellation_study.py RE1-Architecture.ifc` (the three tables below, no rvt-rs binary needed); the Measure runs are in §11.6.

**Why arcs were left out.** The first run drew arcs as `IfcArcIndex` arcs and replaced 335 bodies; 250 of the 251 the body scorer matched had Revit's vertices within 3.4e-5 ft of the surface, and RE1 Architecture's element 360331 had them 0.001439 ft off, over the 0.001 ft "ref on ours" bar (§9). §9 left the cause as an inference from the volumes: that Revit's tessellation of an arc starts elsewhere than IfcOpenShell's tessellation of ours.

### 11.1 What Revit writes for the dispenser

Element 360331, 'Dispenser, Toilet Paper - NON-COLLECTUS:-', is an `IfcBuildingElementProxy` whose body is an `IfcMappedItem` of a `SweptSolid` representation of seven `IfcExtrudedAreaSolid`: two over an `IfcRectangleProfileDef` (114.3 by 0.893 mm, 6.35 by 152.4 mm) and five over an `IfcCircleProfileDef`, of radius 69.85, 25.4, 25.4, 12.7 and 6.35 mm (0.2292, 0.0833, 0.0833, 0.0417 and 0.0208 ft). It is not a mesh and not an indexed curve: Revit writes each circle as a true circle. In the four RE1 exports every circle in an extruded body is an `IfcCircleProfileDef` or an `IfcCircleHollowProfileDef` (Architecture 5 elements, Electrical 16, Mechanical 6, Plumbing 70); their other extrusions are rectangles and polylines of straight segments, and Mechanical's flow fittings have 90 degree arcs as two chords. 16, 6 and 53 elements of Architecture, Mechanical and Plumbing are `IfcFacetedBrep`s. What Revit's export writes for the BIM4LCA office files is measured in §11.4.

### 11.2 The cause: 27 chords against 28

IfcOpenShell cuts a circle into chords by its own rule, and the rule gives a whole `IfcCircleProfileDef` and the two half arcs of the same circle different counts at some radii (a hand-written STEP file in the shape rvt-rs writes, metres, meshed with the scorers' own settings):

| radius mm | radius ft | `IfcCircleProfileDef` | 2 half arcs | 4 quarter arcs | sagitta ft, circle / half arcs |
|---:|---:|---:|---:|---:|---|
| 6.35 | 0.0208 | 26 | 26 | 28 | 0.00015 / 0.00015 |
| 25.40 | 0.0833 | 26 | 26 | 28 | 0.00061 / 0.00061 |
| 41.80 | 0.1371 | 26 | 26 | 28 | 0.00100 / 0.00100 |
| 50.00 | 0.1640 | 26 | 26 | 28 | 0.00120 / 0.00120 |
| **69.85** | **0.2292** | **27** | **28** | 28 | 0.00155 / 0.00144 |
| 100.00 | 0.3281 | 32 | 32 | 32 | 0.00158 / 0.00158 |
| 150.00 | 0.4921 | 39 | 40 | 40 | 0.00160 / 0.00152 |
| 300.00 | 0.9843 | 55 | 56 | 56 | 0.00161 / 0.00155 |

Up to 0.164 ft (50 mm), the largest of the small radii measured, the steps are 0.2417 rad (26 chords a circle) and the counts agree, so the sagitta grows with the radius and passes the 0.001 ft bar at 0.137 ft; between 0.164 and 0.229 ft the count first leaves 26, and from there the steps shrink to hold the sagitta near 0.0016 ft (0.49 mm) and the counts differ by one at some radii (0.2292, 0.4921, 0.9843, 1.9685 ft). The dispenser's big circle (0.2292 ft) is Revit's 27 chords against rvt-rs's 28: no vertex of one lies on the other's, Revit's vertices sit on the true circle, and the worst of them lies half a chord from ours, 0.2292 ft (1 - cos(pi/28)) = 0.001441 ft, the sagitta of a 28-gon of that radius. The five circles are redrawn from Revit's own placements in a file of rvt-rs's shape and scored with `mesh_bodies_vs_ifc`'s arithmetic:

| drawing of the circles | volume ratio | Revit's vertices to ours, ft | ours to Revit's, ft |
|---|---:|---:|---:|
| two half arcs, start 0 (the first run) | 1.00055 | **0.001439** | 0.001542 |
| two half arcs, start 90 degrees | 1.00055 | 0.001439 | 0.001542 |
| four quarter arcs | 1.00061 | 0.001439 | 0.001541 |
| `IfcCircleProfileDef`, reference direction as Revit's | 1.00000 | 0.000003 | 0.000000 |
| `IfcCircleProfileDef`, turned 90 degrees | 1.00000 | 0.001162 | 0.001162 |
| `IfcCircleProfileDef`, turned 180 degrees | 1.00000 | 0.001550 | 0.001550 |
| arcs up to 0.12 ft, short arcs over (§11.3, not taken) | 1.00778 | 0.000027 | 0.001550 |

The first row is run 41805333248's dispenser to the digit (0.001439 ft; the volume ratio 1.0006 of §9). The start of the half arcs changes nothing (rows 1 and 2), so the start is not the cause §9 inferred: the cause is the count, and 28 against 27 leaves no vertex in common. The same circle as an `IfcCircleProfileDef` coincides to 3e-6 ft when its reference direction is Revit's, and misses by up to a chord's sagitta when it is not: for an odd count a turn of 180 degrees moves every vertex half a step. Revit's reference direction for these circles is `(0,1,0)` for a profile in the YZ plane and `(1,0,0)` in XZ; whether it is the sketch plane's x axis in every family is not measured, so the circle form cannot be relied on to coincide.

### 11.3 What bounds the miss, and the option that was tried and not taken

Revit's vertices lie on the true arc whether it wrote a circle, arcs or a mesh. A vertex on the true arc lies within the sagitta of the surface that cuts that arc into chords, so what bounds "ref on ours" is the sagitta of rvt-rs's own surface, and rvt-rs controls two things: the arc, and how short the arcs it writes are. Over Revit's circle as an `IfcCircleProfileDef` and as half and quarter arcs, each at four starts (0, 90, 7.3 and 180 degrees), by radius, the worst "Revit's vertices to ours" for half arcs and for short arcs over 0.12 ft (the first head of this change wrote them; section 11.4 says why they were dropped), and the range of volume ratios:

| radius ft | half arcs: ref on ours, ours on ref, volume ratio | short arcs over 0.12 ft: ref on ours, ours on ref, volume ratio (arcs written) |
|---:|---|---|
| 0.020 | 0.00015, 0.00015, 0.9985 to 1.0000 | the same (2) |
| 0.080 | 0.00058, 0.00058, 0.9987 to 1.0000 | the same (2) |
| 0.120 | 0.00087, 0.00087, 0.9986 to 1.0000 | the same (2) |
| 0.125 | 0.00091, 0.00091, 0.9986 to 1.0000 | 0.00003, 0.00091, 1.0081 to 1.0095 (50) |
| 0.140 | **0.00102**, 0.00102, 0.9986 to 1.0000 | 0.00003, 0.00102, 1.0082 to 1.0095 (53) |
| 0.200 | **0.00146**, 0.00146, 0.9987 to 1.0000 | 0.00003, 0.00146, 1.0083 to 1.0096 (63) |
| 0.500 | **0.00154**, 0.00162, 1.0000 to 1.0002 | 0.00003, 0.00162, 1.0041 to 1.0043 (100) |
| 2.000 | **0.00162**, 0.00162, 0.9999 to 1.0000 | 0.00003, 0.00162, 1.0010 to 1.0011 (199) |

An arc written as an arc keeps "ref on ours" under 0.001 ft up to a radius of about 0.137 ft whatever Revit wrote, and misses it from about 0.14 ft up wherever Revit's tessellation does not coincide with IfcOpenShell's of the arc: the dispenser is the case where it does not. Arcs short enough that each chord lies within 0.00025 ft of the arc hold 3e-5 ft at every radius: the bound is 0.00025 ft, and IfcOpenShell cuts each short arc into three chords or more (measured on arcs of 5 to 20 degrees, radii of 2 to 640 mm). What they give up is the coincidence: "ours to Revit's" stays where it was (it is Revit's own sagitta, 0.0010 to 0.0016 ft above 0.14 ft), and the volume is the arc's true one, which is 0.1% to 1% over a Revit body cut into chords (1.0096 at the worst, inside the 1% column, and a Revit polygon coarser than IfcOpenShell's would be coarser against the saved mesh, too).

### 11.4 Why short arcs were dropped: Revit's two exporters write a circle two ways

The first head of this change wrote an arc over 0.12 ft as short arcs, and its flag run (41832799725) held "ref on ours" for every scored arc body. It also threw away exact matches. The office's 78 `IfcWasteTerminal`s (62 of Lattiakaivo:LK-1 and 16 of Lattiakaivo:VK-1.Kattokaivo) were exact in the first arc run (41805333248: volume ratio 1.0000, 1.9e-6 ft in both directions, 78 of 78 in the `vol .01%` and `0.001 ft` columns); written as short arcs they scored a volume ratio of 1.0072, 0.00144 ft from rvt-rs's to Revit's, and 0 of 78 in both columns, worse than the first run and, in median volume ratio, worse than the saved mesh (1.0035).

The census names Revit's form for the terminals: `extrusion of indexed curve with arcs`, the form rvt-rs writes. Revit's exports of the two corpora are not the same dialect:

| | RE1 (Architecture, Electrical, Mechanical, Plumbing) | BIM4LCA office as built | Core Interior |
|---|---|---|---|
| exporter, schema | Revit 26.2, IFC2X3 | Revit 24.0.5, IFC4 | Revit 24.0.20, IFC4 |
| the .rvt | Revit 2025 | Revit 2024 | Revit 2024 |
| a circle in a profile | `IfcCircleProfileDef` or `IfcCircleHollowProfileDef` (97 elements) | an `IfcIndexedPolyCurve` of arcs; 0 `IfcCircleProfileDef`, 4,717 indexed curves, 3,295 extrusions | indexed curves |

The office's terminals are one circle of radius 80 mm (0.2625 ft) as two `IfcArcIndex` half arcs of 180 degrees from (-80, 0) through (0, -80) to (80, 0), under a solid at the origin with the default axes; the exit signs and the ladder are the same, a round column is 180, 90 and 90 degrees, a pipe four quarters, the rounded corners of hollow sections quarters. At 80 mm IfcOpenShell cuts an `IfcCircleProfileDef` into 29 chords and two half arcs into 30 (`tools/re/arc_tessellation_study.py` counts: 60 mm 26 and 26, 70 mm 27 and 28, 75 mm 28 and 28, 85 mm 29 and 30, 100 mm 32 and 32). So the dispenser (Revit 26.2, a circle profile of 69.85 mm: 27 against our 28) and the terminals (Revit 24.0.5, two half arcs of 80 mm: 30 against a circle profile's 29) are the same problem from the two sides, and no one drawing of a circle meets both at those radii: half arcs meet the office and miss RE1, an `IfcCircleProfileDef` meets RE1 (3e-6 ft when its start is Revit's) and would miss the office, and short arcs hold "ref on ours" for both and meet neither exactly.

What separates the two is the exporter, not the record: both circles are a sketch circle in a family. Whether it is the release of the exporter (24 against 26) or the schema (IFC4 against IFC2X3, which has no `IfcIndexedPolyCurve`) is not settled by these corpora: puzzbobb's Revit 2025 models have IFC4 exports from Revit 2025 (research/r2025-models.tsv), and they hold no body of this kind (§11.7). Nor is the exporter that will read a file in the `.rvt`: a 2024 file can be exported by Revit 2026 and a 2025 file by Revit 2024.

### 11.5 The change

`src/ifc/family_extrusions.rs` writes arcs as the first run did, byte for byte, which is Revit 24.0.5's IFC4 form: one `IfcArcIndex`, or two for a span over a half turn. The short arcs and their bounds are gone. An instance with a loop that is one whole circle (a single arc turning the full way round) over 0.12 ft in radius keeps its saved mesh, in every release.

What the rule says, and why:

- Revit 24.0.5's IFC4 export writes these circles as arcs (two half arcs in the office), and Revit 26.2's IFC2X3 export writes them as circle profiles (`IfcCircleProfileDef`).
- At radii over about 0.14 ft IfcOpenShell's default tessellation of the two forms differs by one chord, so no single form meets both, and nothing in the `.rvt` says which exporter will read it. The release does not say either: a 2024 file exported by Revit 2026 is on the other side of any gate keyed on the release, which is why the 2025 gate of the two earlier heads (§11.7) was dropped.
- rvt-rs therefore keeps the saved mesh for them. That meets the bar against both exporters, and it is what `main` writes with `--saved-meshes`, so nothing regresses.
- The rule follows from the scorers' default tessellation of the two exporters' forms. True arcs for these circles come back if #1355 (a fine-tessellation mode for the body scorers, measured beside today's bar) changes the bar; the bar's change is Griffin's to make. Under a fine tessellation the dispenser's two forms are one exact circle, and the miss of §11.2 is a tessellation artefact, not a geometry error.
- A circle of 0.12 ft or less is drawn: IfcOpenShell cuts it into 26 chords in either form, 0.00088 ft off the circle at most. The r2025 IFC4 exports hold none of these bodies (§11.7).

`FamilyExtrusionReport` and the `rvt-ifc` note keep `arcs`, now the candidates the extrusions reproduce that are left with their mesh for an arc the rule does not draw: a whole circle over 0.12 ft in radius (it counted the instances kept for any arc before). They add `with_arcs`, the replaced bodies with an arc in a profile. The default export does not reach any of this and stays byte for byte.

### 11.6 What the scorer measures

`tools/re/family_extrusion_bodies_vs_ifc.py` prints, after its existing rows, a census of the scored bodies with an arc in a profile by the form Revit's export gives the body (`IfcCircleProfileDef`, polyline, indexed curve with arcs, faceted brep and so on) and by the largest radius of their arcs (up to 0.12 ft or over), with how many of each hold "ref on ours", both directions within 0.001 ft and within 1e-5 ft (the two tessellations coincide), and the volume within 1% and 0.01%, then the bodies that miss "ref on ours", one line each with Revit's form. It reads the two files while `mesh_bodies_vs_ifc` has them open (`DESCRIBE`), so it adds no meshing.

### 11.7 What the runs measured

Two heads were run before the rule above, to find where the exporters' dialects divide; their census figures stand.

- **Keep-the-mesh head `681e42dd`** (run 41835957331; arcs as arcs, and in a Revit 2025 or later file a whole circle over 0.12 ft kept as the mesh; the baseline is 41835962201, the same code with `--saved-meshes` alone). No scored arc body misses "ref on ours". Replaced bodies: Core Interior 37 (none with an arc), the office as built 129 and at permit 129 (120 each with an arc), the terrain files 11 each (11 with an arc), RE1 Architecture 7 (1 with an arc, 1 kept for a circle: the dispenser), Electrical 2 (6 kept), Mechanical 1, Plumbing 1 (with an arc). On the office as built 87 bodies are scored; the 78 waste terminals (62 of Lattiakaivo:LK-1, 16 of Lattiakaivo:VK-1.Kattokaivo, which are Revit 2024 files, so the gate was off) hold "ref on ours" 78 of 78, the `0.001 ft` column 78 of 78, the `1e-5 ft` column 78 of 78 and `vol .01%` 78 of 78, at a median volume ratio of 1.0000 and 2e-6 ft in both directions; the 11 `Hulevesikaivo:HVK-1` of the terrain files are exact the same way (1.0 to 2.5e-6 ft). Their form in Revit's export is `extrusion of indexed curve with arcs`.
- **Circle-form head `6dbcae8d`** (run 41836478995; in a Revit 2025 or later file a whole circle with nothing in it written as an `IfcCircleProfileDef` whose first vertex is where the record's arc starts). The nine RE1 bodies so drawn, whose form in Revit's export is an extrusion of a circle (Architecture 2, among them the dispenser, Electrical 6, Plumbing 1), all hold "ref on ours", all are within 1e-5 ft in both directions and all hold `vol .01%`: the dispenser at 4.6e-6 ft and 4.0e-6 ft, a volume ratio of 1.0000. Revit 26.2's circle starts where the record's arc does, then, on these nine; the head was not kept because that is a fact about one exporter (§11.5).
- **The r2025 models** (puzzbobb's Revit 2025 files, IFC4 exports of Revit 2025; 24 models in both runs): 18 have no candidate body, and 6 have 4 or 6, none of which the extrusions reproduce by volume. They hold no body of this kind, so nothing tests whether the circle form divides by release or by schema.
- Byte identity at `6dbcae8d` (run 41836483266): 111 of 111 the same. The first head (`d267612f`, short arcs, run 41832799725) held "ref on ours" for every scored arc body and lost the terminals' exactness (§11.4); its byte identity (41832804165) was 111 of 111 the same.

The figures of the final rule's runs are in §11.8.

### 11.8 Not measured

- IfcOpenShell other than 0.9.0.post1, which is what the bound of §11.3 is measured on and what CI pins; another consumer's tessellation of an arc.
- Under a finer tessellation of the scorers (#1355) the same bodies; that comparison is that issue's.

**Runs.** On the models and scorers of 41816314683 (Core Interior, Einhoven, RE1, the BIM4LCA office files), with puzzbobb's Revit 2025 models (`-f r2025=true`, `r2025-*`), from `--ref feat/1346-extrusion-arcs` because the census is on the branch's scorers. The final rule (`e5b4bda6`): 41838483706 (flag, `--saved-meshes --family-extrusions`), 41838489013 (baseline, `--saved-meshes` alone, the same code) and 41838493193 (byte identity against `main`, every model with `-f nordic=true -f r2025=true -f samples=true -f holdout=true -f families=true`). Their results are added here when they finish; the earlier runs are above.

```bash
gh workflow run measure.yml --ref feat/1346-extrusion-arcs -f ref=<head> -f base=none -f nordic=true -f r2025=true \
  -f models="core-interior re1-* einhoven nordic-office-* r2025-*" -f ifc_flags="--saved-meshes --family-extrusions" \
  -f scorers="family_extrusion_bodies_vs_ifc mesh_bodies_vs_ifc saved_meshes_vs_ifc space_profiles_vs_ifc"
gh workflow run measure.yml --ref feat/1346-extrusion-arcs -f ref=<head> -f base=none -f nordic=true -f r2025=true \
  -f models="core-interior re1-* einhoven nordic-office-* r2025-*" -f ifc_flags="--saved-meshes" \
  -f scorers="family_extrusion_bodies_vs_ifc mesh_bodies_vs_ifc saved_meshes_vs_ifc space_profiles_vs_ifc"
gh workflow run measure.yml --ref main -f ref=<head> -f base=main -f nordic=true -f r2025=true \
  -f samples=true -f holdout=true -f families=true -f scorers=none
```
