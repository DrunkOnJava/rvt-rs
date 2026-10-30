# RE-291 — A column Revit cuts at an end, and the base quantities of the ones it does not

**Date:** 2026-10-30
**Issues:** #1046 (quantities Revit computes from its joined geometry), #1016 (quantities), W7 of [`docs/release-1.0-criteria.md`](../../docs/release-1.0-criteria.md); follows RE-247 (a column's `Length` and the 18 meshed columns nothing told apart), RE-202 (a column's joins) and RE-221 (hollow-section columns)
**Artefacts:** the BIM4LCA office as built and at building permit (Revit 2024, Revit 24.0.5's IFC4 exports, CC BY-SA 4.0, `research/bim4lca-measure.tsv`) and puzzbobb's R2025 models (Revit 2025, Apache-2.0, `research/r2025-models.tsv`), as Measure fetches them
**Probe:** `probes/examples/probe_re291_column_pairs.rs`
**Runs:** Measure [41139043243](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41139043243) (the probe on both offices, joined offline to Revit's bodies and quantities by `Tag`), [41136551451](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41136551451) (`qto_candidates_misses`, `qto_presence_list` on `main`) and, for the shipped rule, [41146562639](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41146562639) (this change at 9f04ff8c against `main` at 9637d5ff, every model and scorer, rvt-rs's IFC kept) and [41140980550](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41140980550) (`probe_re291_column_type_parameters`)
**Result:** positive. Revit's export draws a column as a cut mesh exactly where a slab its `CutPairGStep` names lies across one end of it: all 18 of the office's meshed columns at building permit have one, none of the 327 it draws as an extrusion has, and none of the 279 as built has. On a column drawn as its record box and joined by a clean pair, where nothing it is paired with cuts it or crosses an end of it, the box's `CrossSectionArea`, `OuterSurfaceArea`, `GrossVolume` and `NetVolume` are Revit's on every one: 186 columns as built, 250 at permit, none different and none Revit's export leaves out. A joined column's `Length` is left out, as the maintainer decided (2026-10-26); §5 says what this finding changes for it.

## 1. What RE-247 could not tell apart

Revit's export gives a column `Qto_ColumnBaseQuantities` with `Length`, `CrossSectionArea`, `OuterSurfaceArea`, `GrossVolume` and `NetVolume` where it draws the column as an extrusion (`IfcExtrudedAreaSolid`). At building permit it draws 18 of the office's 345 columns as a mesh (`IfcPolygonalFaceSet`) and gives those `CrossSectionArea` and `NetVolume` only, both different from the record box's: the box's `NetVolume` is 0.628 m³ where the mesh's is 0.6236, and the mesh's `CrossSectionArea` is 3.289 m². RE-247 §4 compared every scalar of those columns' records and the kinds of element their pair steps name, and found nothing that predicts the mesh: the meshed columns are paired with the same kinds of element, in the same numbers, as many extruded ones, and every column holds a `CutPairGStep`. The only fact that held was that a column with no `CleanPairGStep` is an extrusion, which says nothing of the 256 joined ones that are.

## 2. Where the paired elements lie

`probe_re291_column_pairs.rs` prints, for every column rvt-rs exports, the base and height of its body and, for each `CutPairGStep` and `CleanPairGStep`, the element `m_otherId` names, as rvt-rs exports it (entity type, base, height). Joined to Revit's export by `Tag` (a column with a `Length` is an extrusion), and with each paired slab, beam, wall, covering or column placed against the column's own span, ends within 0.005 ft counting as touching:

| a column's pair with | step | extruded, as built (279) | extruded, permit (327) | meshed, permit (18) |
|---|---|---:|---:|---:|
| a slab or beam wholly above or below it | cut | 278 | 326 | 18 |
| a slab whose span crosses its top | cut | 0 | 0 | 18 |
| a slab or beam wholly above or below it | clean | 185 | 255 | 18 |
| a slab whose span crosses its top | clean | 0 | 3 | 18 |
| a wall | cut | 0 | 3 | 1 |
| a wall | clean | 47 | 137 | 10 |
| a covering or a column | clean | 9 | 25 | 5 |

(Columns, not pairs: a column appears in several rows.) Every partner is an element rvt-rs exports with a body of one extrusion, so each has a span. The 18 meshed columns are 17 `Concrete-Square-Column:PI-1` (ElementIds 1929237 to 1929282) and the one `PI-2` 1343915. Each of the 17 is cut by slab 1474132, whose base lies 0.1804 ft below the column's top in rvt-rs's bodies; 1343915 by slab 1498471, 0.8202 ft below its top. Each of the 3 extruded columns that has a slab across its top (1929272, 1929274 and 1929275, with slab 2765485 across it by 0.89 to 1.07 ft) holds a clean pair with it and no cut pair: Revit joins the two without removing material from the column, and draws it as its extrusion.

The three extruded columns with a `CutPairGStep` to a wall (1343914, 1343926 and 1343928) are extrusions, but a wall can cut their section: 1343914 and 1343928 have a `CrossSectionArea` of 0.067 and 0.102 m² against the box's 0.16, and a `NetVolume` 42% and 64% of the box's; 1343926, cut by two walls, has the box's.

## 3. The rule

The record's box is Revit's extrusion for a column drawn as that box (`partition_element_record_bbox`) when nothing cuts it or crosses an end of it. A column is taken when all of these hold:

- its record holds a `CleanPairGStep` (the columns that hold none are RE-247's), and every pair step of its record names an element rvt-rs exports (`native_column_joins::ColumnJoins::cut_partners`, `clean_partners`; a step whose `m_otherId` is not read leaves the column out);
- every element its cut pairs name is a slab lying wholly above or wholly below the column, so that it cannot cut it;
- every element its clean pairs name is a wall, a covering or a column, or a slab lying wholly above or wholly below it. A beam, roof or footing is not accepted: the only columns the office pairs with a beam are 8 hollow-section ones (RE-221's), and no box-drawn column is paired with one, so a beam's extent is not measured here;
- its body is the record box, a rectangle, with a positive size.

`export_content/column_quantities.rs` writes `CrossSectionArea` (width times depth), `OuterSurfaceArea` (the perimeter times the height), and `GrossVolume` and `NetVolume` (the area times the height). Any other pair step, a wall or covering that cuts it, a slab across an end, or an element rvt-rs does not export leaves the column out. The 3 extruded columns with a slab across their top by a clean pair are left out although Revit draws them as their extrusion: nothing measured says a clean pair alone never cuts, and 3 of 3 is not that.

Measured offline on the probe's output joined to Revit's quantities (columns that are not hollow-section ones):

| | office as built | office at permit |
|---|---:|---:|
| columns drawn as their box, joined by a clean pair | 186 | 274 |
| ... taken by the rule | 186 | 250 |
| ... the four quantities equal Revit's, of those taken | 186 | 250 |
| ... different, or Revit gives none | 0 | 0 |
| left out: meshed | 0 | 18 |
| left out: a cut pair with a wall (two of the three differ) | 0 | 3 |
| left out: a slab across the top by a clean pair only (equal) | 0 | 3 |

### The 18 meshed columns

The maintainer's rule (2026-10-26) is that nothing is written where Revit's export gives a joined column none, or gives another value. rvt-rs's own export of the office at building permit (`rvt-ifc`, kept by Measure's `keep_ifc`) gives these 18 columns, by quantity:

| quantity | Revit writes on the 18 | rvt-rs gives a value to |
|---|---|---:|
| `Length` | none | 0 of 18 |
| `CrossSectionArea` | its mesh's, not the box's | 0 of 18 |
| `OuterSurfaceArea` | none | 0 of 18 |
| `GrossVolume` | none | 0 of 18 |
| `NetVolume` | its mesh's, not the box's | 0 of 18 |

The office as built has no meshed column. On the 327 extruded columns at permit rvt-rs gives `Length` to 71 (RE-247's, none joined by a clean pair), `CrossSectionArea`, `OuterSurfaceArea` and `GrossVolume` to 305 and `NetVolume` to 250; the unfiltered census agrees: none different and none Revit's export leaves out.

## 4. What stays unwritten

- A column no clean pair joins and that is drawn as its box (13 as built, 15 at permit): the box's four quantities are Revit's on all of them but the round column 3053209 (`Round Column:PI-6`, a 150 mm column whose `CrossSectionArea` is 0.01766 m² against the box's 0.0225), which has no pair at all and which rvt-rs draws as a box. Nothing the export reads yet says a box-drawn column is round, so they stay out (RE-247 §8). The family's parameters do: `probe_re291_column_type_parameters.rs` (run 41140980550) finds that the round column's type holds one length parameter, captioned `Diameter`, while every rectangular and square concrete column type holds a `b` (square) or an `h` and a `b` (rectangular) equal to its columns' record boxes' sides. A check that the type holds those would take the 13 and 15 columns and guard the joined rule against a round column it has not met; it is [#1203](https://github.com/DrunkOnJava/rvt-rs/issues/1203) item 2.
- A hollow-section column joined by a clean pair: none on either file.
- The 18 meshed columns' `CrossSectionArea` and `NetVolume`, which Revit writes and the box does not give.
- `Length`: see §5.

## 5. What this changes for a joined column's Length

The maintainer left a joined column's `Length` out (2026-10-26) because Revit's export draws 18 of them as a mesh without one and no field tells them apart (RE-247 §7). On the columns this rule takes, Revit writes a `Length` on every one (186 of 186, 250 of 250) equal to the box's height. A `Length` on exactly those columns would therefore be written equal on 436 more column quantities, none Revit's export omits: it is the rule's own set, and the rule separates the meshes. rvt-rs does not write it here: the decision is the maintainer's, and the finding is the issue's (#1046).

## 6. Reproduce

```bash
gh workflow run measure.yml --ref main -f ref=5458791ee0fe6dae21361b3e38066d4f65b826d1 -f base=none -f nordic=true \
  -f models="nordic-office-*" -f scorers=none -f probe=probe_re291_column_pairs -f probe_scorer=-
gh workflow run measure.yml --ref main -f ref=9f04ff8cb3723a6e7c0dfb64e97f78aa3da0c960 -f base=main -f nordic=true -f r2025=true -f keep_ifc=true
```

The probe's lines `col <tag> z <base> box <w> <d> <h> pairs <step>:<other>:<entity>:<base>:<height> ...` join to Revit's bodies by `Tag` (a column is meshed where its `Qto_ColumnBaseQuantities` has no `Length`).
