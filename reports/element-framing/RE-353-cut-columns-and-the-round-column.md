# RE-353: a column cut across its top by one slab, and the round column

**Date:** 2026-11-05
**Issues:** [#1203](https://github.com/DrunkOnJava/rvt-rs/issues/1203), W7 of [`docs/release-1.0-criteria.md`](../../docs/release-1.0-criteria.md); follows RE-247 and RE-291 (the columns Revit draws as a cut mesh and the box-drawn columns' quantities)
**Artefacts:** the BIM4LCA office as built and at building permit (Revit 2024, Revit 24.0.5's IFC4 exports, CC BY-SA 4.0, `research/bim4lca-measure.tsv`), as Measure fetches them
**Probe:** `probes/examples/probe_re353_column_cut_footprints.rs`, Measure runs [42112543949](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42112543949) and [42115439931](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42115439931) (commit 38d9e426, a scratch branch; the same file is at e5247625)
**Status:** positive on one building, the office. The meshed columns: reproduced offline from the probe's output to Revit's own mesh for all 18 columns, and the rule written from it gives Revit's `CrossSectionArea` and `NetVolume` on 17 of the 18 at permit (the office as built, the R2025 models and the held-out models hold none). The round column: `OuterSurfaceArea` is a formula that is Revit's exactly on one column on each of the two office files, the same column; its other three quantities are measured negatives (section 4). Neither rule is scored on any model but the office: the held-out, R2025 and RE1 models' counts, and Core Interior's, Einhoven's and RE1 Architecture's IFC, are unchanged, and the 2 round columns of one sample project, whose Revit export holds no base quantities, gain the `OuterSurfaceArea` (section 5.1).

## 1. What Revit writes where there is no extrusion

revit-ifc's column quantities (CLEANROOM.md source 5, `Source/Revit.IFC.Export/Exporter/PropertySet/PropertyUtil.cs`, `CreateBeamColumnBaseQuantities`, tag `IFC_v24.3.40`; read and cited, no code copied) take `Length` from the extrusion its exporter finds, `CrossSectionArea` from that extrusion's profile area and, only where that is zero, from the element's computed area parameter, `OuterSurfaceArea` from the extrusion's length and outer perimeter, `GrossVolume` from the extrusion's length and area, and `NetVolume` from the element's computed volume parameter. A column Revit draws as a mesh has no extrusion, so it gets `CrossSectionArea` and `NetVolume` alone: RE-291 section 3's table (`Length`, `OuterSurfaceArea` and `GrossVolume` absent on the 18, `CrossSectionArea` and `NetVolume` the mesh's) is what that reading gives.

On Revit's own export of the office at permit, the mesh (`IfcPolygonalFaceSet`) of each of the 18 meshed columns is in the file, and its coordinates are whole millimetres. For all 18, `NetVolume` is the mesh's volume and `CrossSectionArea` is half the sum of its faces' areas, both to 1e-12 relative. Three shapes of cut hold 17 of them (every column is 400 x 400 mm and 3,925 mm high, cut 55 mm down from the top by the slab's base), and the eighteenth is a column 3,350 mm high:

| columns | what is cut away (plan) | `NetVolume`, m³ | `CrossSectionArea`, m² |
|---|---|---:|---:|
| 14 (1929241, 1929245, 1929247, 1929249, 1929253, 1929257, 1929261, 1929265, 1929269, 1929273, 1929277, 1929278, 1929281, 1929282) | 400 x 200 mm on one side | 0.6236 | 3.289 |
| 2 (1929237, 1929238) | 370 x 200 mm, 30 mm in from one side | 0.62393 | 3.300 |
| 1 (1929246) | an L: 400 x 200 and 200 x 200 mm | 0.6214 | 3.278 |
| 1 (1343915, 250 mm down) | 6-sided | 0.5185 | 2.840 |

The box alone has 0.628 m³ and 3.300 m², so the 370 x 200 mm cut, whose added walls are as long as the side walls it removes, is the one that leaves the box's area.

## 2. The slab, and the cut

Seventeen of the columns are cut by one floor, `Floor:VP-1` (1474132), 450 mm thick, which Revit's export draws as four layer solids and rvt-rs as one extrusion of its whole thickness with an outline of 28 points and three voids (of 6, 4 and 4 points); the eighteenth by slab 1498471, a six-sided polygon with a re-entrant corner. Each column holds a `CutPairGStep` and a `CleanPairGStep` for the slab, and the slab's span starts inside the column and reaches its top.

The part cut away is the slab's plan region (its outline less its voids) clipped to the column's section, from the slab's base up to the column's top. With V0 and S0 the box's volume and surface, A the plan area of the part, d its depth, Lb the length of the section's sides inside the slab and Li the length of the slab's edges inside the section:

- `NetVolume` = V0 - A d;
- the surface of the cut box = S0 - Lb d + Li d (the top is replaced by the floor of the cut, the side walls it removes by the walls it adds), and `CrossSectionArea` is half of it.

Checked offline: the probe prints each column's and each partner's plan placement and profile as rvt-rs exports them, and joined offline to Revit's mesh volumes and areas, the formulas above give all 18 within 2e-12 relative (the nearest to a miss, 1929281 and 1929282, are 1.5e-12), with the slab clipped by Sutherland-Hodgman against the column's section. Two details the formulas need, both found by the offline join and not by the first version: a slab's edge may lie along a side of the section (a void's edge on 1929246 and 1929247, the outline's on 1929246), and then the section's side is inside the slab where the section's interior just beside it is, tested 1e-3 ft inside the side; and a segment is along a side when it is within 1e-4 ft of it and parallel to 1e-9, since a slab edge 4e-13 ft off a side is not inside the section.

## 3. The rule

A column is taken as cut, and gets `NetVolume` and, where the next limit allows, `CrossSectionArea` as above and no other quantity, when its body is the record box and its type a plain rectangle (RE-291), a clean pair joins it, and

- every element its cut pairs name is a slab lying wholly above or wholly below it, except one slab whose span starts inside the column and reaches its top (a wall, a covering or any other cut partner leaves the column out), and a clean pair names that slab too;
- every element its clean pairs name is a wall, a covering, a column, a slab lying clear, that slab, or another slab across its top that has no plan area inside the column's section;
- the slab, and any other slab across the top, is one exported extrusion whose profile was set (a rectangle, a closed polyline or one with voids: a floor whose sketch was not read is its record box and has none), and the wall pass knows its top as flat (`SlabTop::Flat`: its record was read and no shape edit moves it). A sloped or shape-edited floor's underside is not its base, so the slab's base is only the cut's where the top is flat. A floor with a shaft: the shaft is cut from the floor's outline on Revit 2024, the one release read, and the later releases that leave the floor whole are not read;
- the part of the slab inside the column's section is more than a nick and less than the whole;
- the file is Revit 2024 and the column's body is not one RE-178 trimmed (it keeps the record box's source though its body is cut).

`CrossSectionArea` as half the surface of the cut box is measured on 18 meshes whose every face is parallel to a side of the column's section, and those cannot tell it from any other one-sided area: on a cut face that is oblique to the section's sides it may differ. So it is written only where every edge of the slab inside the section runs parallel to a side of it (to 1e-6 in the sine of the angle), and where one does not, `NetVolume` alone is written, which is exact geometry. The 17 columns measured here all have square cuts. The pass reads Revit 2024 alone because the office is the only file either rule is measured on; the files of 2023 and 2025 to 2027 have no cut column measured, and `samples=true` does not compare column quantities, so a release is not opened by a run that cannot see it.

Measured on the office at permit: 17 of the 18 (section 5). Left out, with the reason: 1343915, whose cut pairs name a wall that only touches it (a wall that cuts a column changes its section, RE-291 section 2, so a cut wall is refused without testing where it lies). Its two quantities the same formulas give Revit's to 1e-13 offline.

## 4. The round column

`Round Column:PI-6` (3053209; type 3053174, whose named lengths are `Diameter`, 0.492126 ft, 150 mm, and the round column family's `Offset Base` and `Offset Top`, both 0, in each of the two partitions that hold the type: `probe_re353_round_column`, run 42121438036, which RE-291 took to be `Diameter` alone; height 6,000 mm; no pair joins it) is one column on each of the two office files, the same column. Revit's export writes it as an extrusion of three arcs with:

| quantity | Revit | the circle's | ratio |
|---|---:|---:|---:|
| `Length` | 6.0 m | the box's height, written since RE-247 | 1 |
| `OuterSurfaceArea` | 2.8274333882307761 m² | π D L = 2.8274333882308 | 1 (to 1e-14) |
| `CrossSectionArea` | 0.017659785064800041 m² | π r² = 0.0176714587 | 0.99934 |
| `GrossVolume` | 0.10595871038880049 m³ | 0.1060287520 | 0.99934 |
| `NetVolume` | 0.10600540431829328 m³ | 0.1060287520 | 0.99978 |

- `OuterSurfaceArea` is the circumference times the height, exactly. The column is taken as round from the caption `Diameter` of its type's length parameter alone, with the two zero offsets, and not from its body (rvt-rs draws it as a box, W6); that is one column, and it is read on Revit 2024 only and not where RE-178 trimmed the column, as for the cut columns. rvt-rs writes it where the type's named lengths are `Diameter`, equal to both sides of the record box, and at most `Offset Base` and `Offset Top` each exactly 0 (any other named length, `b` or `h` among them, leaves it out), and no pair joins the column. That is one column on each of the two files, the same column; no other measured file has a round column, so this is a formula checked on one column, not a rule measured across several.
- `CrossSectionArea` and `GrossVolume` (which is the area times the length to 1e-15) are not the circle's: Revit's extrusion area is 0.99934 of it. No whole numbers of chords for the three arcs give a tessellation that reproduces it: a search over 1 to 400 chords of the half circle and 1 to 300 of each quarter finds the nearest at 8e-12 with 319, 13 and 48 chords, about what chance gives for 18 million combinations and not a tessellation anyone would choose. The area is the exporter's own, not a function of the diameter that rvt-rs has.
- `NetVolume` is the element's computed volume parameter (section 1), 0.99978 of the cylinder's. It differs from `GrossVolume`, and from the circle's, by amounts no formula of the diameter and the height gives; the same column would need its joins and cuts, and none is recorded.

So the three stay out. A formula has to explain both volumes or stay out, and none does.

## 5. Measured

Measure run [42130190386](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42130190386), unfiltered (head 35cfd89f, which holds every limit of section 3, against `main` at 8060ecc8; `-f base=main -f nordic=true -f r2025=true -f holdout=true -f samples=true`, every scorer, `qtos_vs_ifc` and `psets_vs_ifc` with `--revit-set-placement`; the office as built and at permit, the 18 held-out models, the 24 R2025 models, the four RE1 models, Core Interior, Einhoven and the sample projects). **17 columns stay**, the same count as run [42122584803](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42122584803) (head 223968aa, before the limits were added), which measured the rule without them, and the same counts after:

| | office as built | office at permit |
|---|---:|---:|
| quantities Revit writes on objects rvt-rs writes | 39,765 | 40,015 |
| equal, `main` / this change | 29,760 / 29,761 | 30,002 / 30,037 |
| missing, `main` / this change | 10,004 / 10,003 | 10,012 / 9,977 |
| `CrossSectionArea` equal | 278 of 279, unchanged | 320 to 337 of 345 |
| `NetVolume` equal | 199 of 279, unchanged | 265 to 282 of 345 |
| `OuterSurfaceArea` equal | 278 to 279 of 279 | 320 to 321 of 327 |
| different, quantities Revit does not write | 1, 0 (unchanged) | 1, 0 (unchanged) |

The gains are the 17 columns' two quantities at permit (34) and the round column's `OuterSurfaceArea` on each file (1 each); the scorer reports counts, not which columns, and the 17 are the number of the 17 square cuts the rule is meant to keep. The property census is identical on both files, every other scorer output of every model (the 18 held-out models', the 24 R2025 models' and the four RE1 models') is byte for byte `main`'s, and the IFC sha256 of Core Interior, Einhoven, RE1 Architecture, the 18 held-out models and 108 of the 111 exports in the run is `main`'s. The three that moved are the office as built, the office at permit, and `sample-2024-snowdon_towers_sample_architectural` (Revit 2024), whose IFC is 136 bytes longer with the same entity total (40,945) and diagnostics; section 5.1 names the bytes. The `same-project gates` check passes on 35cfd89f (9 samples held to their earlier saves; it does not compare column quantities).

The rules are exercised on the office, where 17 cut columns and one round column on each file are every value they write that a Revit export scores. Section 5.1 gives the third file.

### 5.1 Snowdon Towers Architectural

Measure run [42135731462](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42135731462) (filtered: head f60e0d38 against `main` at daf36102, `-f keep_ifc=true -f samples=true -f sample_years="2024 2024" -f scorers=none`) kept the sample's IFC on both sides. With entity ids normalised away and the file's timestamp and owner-history lines dropped, the two differ in 2 `IfcQuantityArea` named `OuterSurfaceArea` and the 2 `IfcElementQuantity` that list them: the `Qto_ColumnBaseQuantities` of two columns gain it beside their `Length`. Both are of the family type `Round Column: 4" Diameter (Spiral Stair Post)`, and they are the only 2 round columns of the file (it holds 118 columns; Revit's export of it also holds 2 columns of that type), so the round-column rule reached every round column Snowdon has. The diagnostics' entity total stayed 40,945 with the two lines added. The other 9 samples of 2024 in the run are `main`'s byte for byte.

Revit's IFC4 export of the sample (local only, Revit 24.0.20.20) holds no base quantities at all: 0 `IfcElementQuantity`, and no `Qto_` or `BaseQuantities` set (its export setup had them off, RE-242). So the two values are neither equal, nor different, nor values Revit does not write: they are counted apart, as is every other quantity rvt-rs writes on the sample, the `Length` of its 118 columns among them. What Revit's export does hold is the geometry revit-ifc takes the `OuterSurfaceArea` from (section 1): each of the two columns is an `IfcExtrudedAreaSolid` over an `IfcIndexedPolyCurve` of arcs, and the outer perimeter times the depth is rvt-rs's value to 2.4e-7 and 1.5e-7 m² (relative 2.3e-7 and 1.1e-7), inside the writer's six decimals. That checks the rule's inputs, the type's `Diameter` and the column's height, against Revit's own extrusion; it does not check a value Revit wrote.

How many files exercise the rules: the cut-column rule, the office at permit alone (17 columns, scored equal); the round-column rule, the office as built (1 column, scored equal), the office at permit (the same column, scored equal) and Snowdon Towers Architectural (2 columns, not scored against a value). No other model of the run has a column either rule takes.

## 6. What stays out

- The round column's `CrossSectionArea`, `GrossVolume` and `NetVolume` (section 4).
- Column 1343915's two quantities (section 3), and any meshed column whose slab is not one piece, has an outline with an arc, has no outline set, is sloped or shape-edited, or lies wholly across the section.
- `CrossSectionArea` where an edge of the slab inside the section is oblique to its sides: `NetVolume` alone is written.
- The cut columns and the round column on any release but Revit 2024, and a column RE-178 trimmed.
- The three extruded columns with a slab across their top by a clean pair alone (RE-291): unchanged.
- Every other form of cut: a column cut by a wall, a beam, a roof or a footing.

## 7. Reproduce

```bash
gh workflow run measure.yml --ref main -f ref=e5247625 -f base=none -f nordic=true \
  -f models="nordic-office-as-built nordic-office-permit" -f scorers=none \
  -f probe=probe_re353_column_cut_footprints -f probe_scorer=-
gh workflow run measure.yml --ref main -f ref=35cfd89f -f base=main -f nordic=true -f r2025=true \
  -f holdout=true -f samples=true
```

The probe's lines `col <tag> at <x> <y> <z> rot <r> prof <profile> h <height> pairs <step>:<other>:<entity>:<x>:<y>:<z>:<rot>:<profile>:<height> ...` join to Revit's mesh volumes and areas by `Tag` (the mesh is the column's `IfcPolygonalFaceSet`).
