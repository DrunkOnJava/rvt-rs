# RE-327: A family's extrusions from their own sketch curves

**Date:** 2026-11-04
**Issues:** #156 (the sketch-to-solid pipeline); Discussion #112 (STE1200's team: sketch curve records to closed loops to extrusion parameters to solids, 85 of 86 extrusions of one family); follows RE-135, RE-163 and RE-204 (a loaded family's own document), RE-280 (the native framing on Revit 2015 to 2022) and RE-323 (`VarParam::m_pSubst`)
**Artefacts:** the `phi-ag/rvt` family corpus at bec25470413fbe659c3ab30d4cfd029b73d63fc3 (`examples/Autodesk/`, sha256 as RE-323 lists them: `rac_basic_sample_family-2016.rfa` fde80383…9601 to `racbasicsamplefamily-2027.rfa` 2df5452e…84f4); Autodesk's 58 sample projects of Revit 2015 to 2027 as `research/autodesk-sample-projects.tsv` pins them; the maker family `stertil-closed-pit-sp-xp-pp-2014.rfa` (Revit 2014, `research/public-families.tsv`, sha256 b3b46ded…de0c)
**Probe:** `probes/examples/probe_re327_family_extrusions.rs`; reader: `src/research/native_form_geometry.rs`
**Runs:** Measure 41776509790, 41785847607 (what the records are), 41787574515, 41788969929, 41791166383, 41793701387 (the rebuild against the saved meshes; the last is the reader at `60151cd4`)
**Result:** positive. The two extrusions of the family corpus rebuild from their sketch curves on all 12 releases (24 of 24 within 0.001 ft of the mesh Revit saved for each), and 6,093 of the 6,172 extrusions of the sample projects whose saved mesh decodes whole do too. The other 79 share one signature, which the reader refuses. Not reproduced: the 85 of 86 on STE1200's family, which is a Revit 2014 file the native framing does not read.

## 1. The question

Discussion #112 reports that the team rebuilt 85 of 86 extrusions of a real family, in a CAD kernel, from the raw streams: sketch curve records, closed loops, extrusion parameters, solids. It gives the stages and the count, not the layouts, the loop rules, the placement or the oracle. This report finds the records on Revit 2015 to 2027, builds the solids and measures them against something independent of the build.

## 2. The records

A family file's elements are the plain groups of its own partitions, framed as a project's are (RE-280). `native_document` refuses every one of them (`invalid increment episode range`: its creation-episode index is a project's), so `native_form_geometry::walk` reads the groups without it. A loaded family's document in a project is the same records in keyed blocks (`native_family_forms`). The classes and fields below are the file's own `Formats/Latest`, decoded by the native graph reader; the same on every release 2015 to 2027 for the three classes the rebuild reads.

- `ExtrusionElem` (a `GenSweep`): `m_cutting` (a void), `m_materialId`; its `ExtrusionElemExtrusionHelper` names the sketch (`m_sketchId`) and stores the loops it was last built from (`m_pCurveLoops`: `CurveLoop`s of pointers to `GLine` and `GArc`). Its `ParamValueSetDouble` holds two parameters, -1001800 and -1001801: the extent along the sketch plane's normal. Every extrusion measured has -1001800 below -1001801, so which is the start is not shown, and the solid between them does not depend on it. Its `ParamValueSetInt` holds `Visible` (-1006205).
- `CurveElem`: one `GLine` (`m_origin`, `m_dirVec`, `m_endParams`: the parameter range) or `GArc` (`m_center`, `m_xVec`, `m_yVec`, `m_radius`, `m_endParams`: the angle range) through its `CurveElemDriver`; `SketchMembership.m_groupId` names its sketch and `m_sketchPlaneId` its plane. The curves are in the family's coordinates, lying in the plane, not in the plane's own: a curve is checked against its plane to 1e-5 ft. A circle is an arc whose two angles are equal (0, 0), and a drawn circle is stored by the extrusion record as two half arcs.
- `SketchPlane`: `Trf` (`m_3x3` and `m_or`) and `m_flipZ`.

The sketch itself (`VarSketch`, with `m_sketchPlaneId`, the constraints and the dimensions) is not read, and does not need to be. It decodes on every release from 2018, but on 2015, 2016 and 2017 the records that hold a `VarParam` are refused with `VarParam.m_pSubst: unsupported native modifier` (6 of its 7 records in the family corpus of 2016 and 2017; RE-323 §5: the field is `Unknown`, and neither layout tried is merged). That is a measured negative for `VarSketch` on those releases and costs the rebuild nothing: `ExtrusionElem`, `CurveElem` and `SketchPlane` decode without a refusal on all of them, and the extrusions rebuild.

## 3. The rebuild

1. The extrusion's helper names the sketch; the sketch's curves are the `CurveElem` records that name it.
2. `assemble_loops` chains them, end to end, into closed loops. Ends within 1e-4 ft join, and a gap is closed by a line (§5); a chain with two curves that could continue it is refused as branching.
3. `profile` sorts the loops into outlines and holes by nesting (even depth an outline, odd a hole) and refuses crossing loops.
4. The plane's frame: the columns of `m_3x3` are its x, y and normal axes, `m_or` its origin, and `m_flipZ` reverses the normal.
5. The solid is the profile swept along the normal between the two parameters.

The extrusion record's own stored loops are compared with the sketch's curves by where they lie (a drawn circle is one curve in the sketch and two in the record): they agree on 9,370 of the 9,384 built extrusions of the sample projects and 24 of 24 of the family corpus, and 14 store none that read.

## 4. Measured against the saved mesh

The oracle is the form's own saved display geometry, its channel-103 `GElement` record, meshed by `native_saved_mesh`: a family's saved mesh is Revit's solid, tessellated, not derived from the sketch. A form matches when its volume is within 1% (and the most a chord-tessellated arc loses, two thirds of its arc length times 0.001 ft times the height), its bounding box within 0.01 ft, every mesh vertex within 0.01 ft of the rebuilt surface and samples of the rebuilt surface within 0.01 ft of the mesh.

**The family corpus** (12 releases, 2 extrusions each: a 2.5 ft tabletop with rounded corners, and four round legs as four disjoint loops in one extrusion):

| | forms | match | volume ratio | bounding box | vertices | surface samples |
|---|---:|---:|---|---|---|---|
| tabletop | 12 | 12 | 1.00009 | 0.000000 | 0.000000 | 0.000713 |
| legs | 12 | 12 | 1.00739 | 0.000911 | 0.000000 | 0.000911 |

The legs' 0.74% is the mesh's own chords (0.001 ft against a radius of 0.125 ft). The rows are identical on all 12 releases, so the readings of §2 hold from 2016 to 2027. Every plane in the corpus is the identity, which cannot tell columns from rows.

**The sample projects' families** (58 files, Revit 2015 to 2027; the same families recur in every release's files, so the counts below are form-and-file pairs, not independent forms):

| | forms |
|---|---:|
| extrusions in loaded families' documents | 9,587 |
| built | 9,384 |
| of those, with a saved mesh | 6,201 |
| complete mesh | 6,172 |
| of those, matching | 6,093 |
| of those, down-facing signature (§5), refused | 79 |
| partial mesh (outside the verdict) | 29 |
| built, no saved mesh | 3,183 (889 voids of 971, 2,294 others) |
| not built | 203 (66 open loops, 137 `GEllipse`) |

By release the extrusions that match are 591 (2015), 283 or 284 (2016 to 2018 and 2020 to 2023), 573 (2019), 1,331 (2024, which has the Snowdon Towers projects), and 537 (2025 to 2027); the 2015, 2019 and 2024 files hold projects the others do not. Every release's files give the same two kinds of row, and the 82 voids with a saved mesh all match.

**Axis convention.** The matrix's columns are the plane's axes: 1,846 forms match only under columns, 3,173 under both (planes symmetric in the way that matters), none only under rows. A mirror (determinant -1) was tried with the normal as the cross product of x and y: no plane in the corpus is one, all 6,172 have determinant +1. A flipped plane's `m_flipZ` applies: all 164 flipped planes with a complete mesh match with the flip, and none matches only without it.

## 5. What does not match

**The down-facing signature (79 forms).** Every extrusion that does not match its complete saved mesh has the same three properties: its plane's third column is (0, 0, -1) to 0.001, the plane is not flipped (`m_flipZ` false), and the extrusion's `m_alwaysRefPlaneNorm` is false. All 79 do, none of the 6,093 matches does: down-facing planes with `m_alwaysRefPlaneNorm` set (131 forms) or `m_flipZ` set (99 forms) match as read. The volume is right and the solid sits on the wrong side of the plane, displaced by start plus end. The sketch's own `Plane` normal (x cross y of its `VarSketch`'s `Plane`) is opposite the matrix's on all 65 of these 79 that have one and also on 112 forms that match, so it does not separate them; `VarSketch.m_flipXY` was not tried against them. The reader names the signature (`Extrusion::down_facing_unreferenced`) and refuses these forms (`DOWN_FACING`), because a direction taken from the data of 79 forms that the same data does not explain would be a guess; the probe measures the reversed normal on them separately.

**Open loops (66 forms).** At a join tolerance of 1e-6 ft, a few sketches stay open; the four dumped show gaps of 1.36e-5 (two forms), 1.53e-5 and 6.49e-5 ft. The reader now joins ends within 1e-4 ft and closes the gap with a line.

**Ellipses (137 forms).** `GEllipse` is not drawn; the form is refused by name.

**Partial meshes (29).** A saved mesh with a face that did not decode (RE-224) is not an oracle: 12 match and 17 differ, and neither goes in the count above.

**Voids and meshless forms.** 889 of the 971 built voids and 2,294 other extrusions have no saved mesh (hidden by a parameter, or a void, which draws nothing): they are built and unmeasured.

## 6. The 2014 family

STE1200's family (Revit 2014, 86 extrusions) is in `public-families.tsv` and Measure fetches it, but 2014 is outside `NATIVE_RELEASES` (2015 to 2027): the framing is refused with `native record framing is unvalidated for Revit 2014`. The 85 of 86 is therefore not reproduced on its own subject. What this report shows is that the stages Discussion #112 names exist on every release 2015 to 2027 with the layouts above, and that the rebuild they describe holds for 98.7% (6,093 of 6,172) of the extrusions whose saved mesh decodes whole.

## 7. What changes

- `native_form_geometry`: a family file's records without the project index; the sketch chain and the solid; `mesh_agreement`.
- `native_family_forms::family_records_in`: `family_records` on any native release, and only for the given ids.
- `rvt-ifc --family-extrusions` (opt-in, implies `--saved-meshes`; the default export is unchanged): a family instance whose body is its saved mesh is drawn from its family's extrusions, as `IfcExtrudedAreaSolid`s over `IfcIndexedPolyCurve` profiles (a drawn circle two arcs), where they reproduce the mesh. Every vertex of the mesh must lie within 0.0005 ft of their surfaces, samples of their surfaces within 0.0015 ft of the mesh, and the volumes agree within 1% and the arcs' chord loss; an instance whose family has a void, a form that is not a rebuilt extrusion, or a mesh the extrusions do not reproduce keeps its mesh. It checks each instance on its own because a placed instance is drawn at its type's values, and the family's document holds the family as saved. The IFC it writes is measured against Revit's own exports only by the unfiltered run.

## 8. Not done

Sweeps, revolutions, blends, swept blends, free-form solids and `GeomCombination` (the other forms of `FORM_CLASSES`), voids cutting solids, ellipses and splines, the down-facing signature's side, the instance's own transform (the export assumes a rotation about the vertical axis), parametric instances of a type other than the one the family was saved at, extrusions in a Revit 2014 file, `VarSketch` on 2016 and 2017.

## 9. Reproduce

```bash
gh workflow run measure.yml --ref main -f ref=<sha> -f base=none \
  -f probe=probe_re327_family_extrusions -f families=true -f samples=true \
  -f sample_years="2015 2027" -f scorers=none -f models=-
```

Each `family-<file>/probe.txt` and `sample-<file>/probe.txt` ends in `tally` lines; `tools/` needs nothing else. The run ids above are the probe at the commits `21d6f874`, `e8f3afe9`, `dd5b49d0`, `44f0159b`, `1391a2a8` and `60151cd4`.

## 10. Credit

The chain is STE1200's team's (Discussion #112): sketch curve records, closed loops, extrusion and sweep parameters, solids, and the observation that Revit's feet are `mm / 25.4 / 12` (RE-310 measured it). jakobhirn-bit's comment there (a flag word at `+0x46` whose bits mark a driven form and a solid one, and the kind of a solid from the sketches under a form) was a lead; this report reads the schema's named fields instead (`m_cutting`, `Visible`) and did not measure the flag word's bits or the sketch-count rule.
