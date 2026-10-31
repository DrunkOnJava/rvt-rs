# RE-291 — A column Revit cuts at an end, and the base quantities of the ones it does not

**Date:** 2026-10-30
**Issues:** #1046 (quantities Revit computes from its joined geometry), #1016 (quantities), W7 of [`docs/release-1.0-criteria.md`](../../docs/release-1.0-criteria.md); follows RE-247 (a column's `Length` and the 18 meshed columns nothing told apart), RE-202 (a column's joins) and RE-221 (hollow-section columns)
**Artefacts:** the BIM4LCA office as built and at building permit (Revit 2024, Revit 24.0.5's IFC4 exports, CC BY-SA 4.0, `research/bim4lca-measure.tsv`) and puzzbobb's R2025 models (Revit 2025, Apache-2.0, `research/r2025-models.tsv`), as Measure fetches them
**Probe:** `probes/examples/probe_re291_column_pairs.rs`
**Runs:** Measure [41139043243](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41139043243) (the probe on both offices, joined offline to Revit's bodies and quantities by `Tag`), [41136551451](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41136551451) (`qto_candidates_misses`, `qto_presence_list` on `main`) and, for the shipped rule, [41147469913](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41147469913) (this change at c8d52649 against `main` at 9637d5ff, every model and scorer, rvt-rs's IFC kept), [41183092458](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41183092458) (this change at 7884e977, `main` merged in, against `main` at 9066ded5, every model and scorer, rvt-rs's IFC kept) and, for the final head, [41203821396](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41203821396) (this change at 92cca542, with the cut-slab check on unjoined columns, the read partners and the exact type rule, against `main` at 54651687, every model and scorer, rvt-rs's IFC kept, with the held-out models) and [41140980550](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41140980550) (`probe_re291_column_type_parameters`)
**Result:** positive, on one building in Revit 2024 (the BIM4LCA office; the R2025 models hold no column, and there is no out-of-sample test). On the office, Revit's export draws a column as a cut mesh exactly where a slab its `CutPairGStep` names lies across one end of it: all 18 of the office's meshed columns at building permit have one, none of the 327 it draws as an extrusion has, and none of the 279 as built has. On a column drawn as its record box whose type is a plain rectangle, joined by no clean pair or by clean pairs where nothing it is paired with cuts it or crosses an end of it, the box's `CrossSectionArea`, `OuterSurfaceArea`, `GrossVolume` and `NetVolume` are Revit's on every one: 199 columns as built (186 joined by a clean pair), 265 at permit (250 joined), none different and none Revit's export leaves out. The one round column, whose type holds a `Diameter`, gets none. A joined column's `Length` is left out, as the maintainer decided (2026-10-26); §5 says what this finding changes for it.

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

The record's box is Revit's extrusion for a column drawn as that box (`partition_element_record_bbox`) when its section is a rectangle and nothing cuts it or crosses an end of it. A column is taken when all of these hold:

- its body is the record box, a rectangle with a positive size;
- its type is a plain rectangle. `partition_type_parameters::type_named_lengths` reads the length parameters of the column's type by the captions of their `ParamElemFamily` elements (the reading RE-221 uses for hollow sections, now shared); the type's named length parameters must be exactly `b`, or `h` and `b` (a hollow section's `Ht`, `t` and `kr`, a round column's `Diameter`, an I, T or channel section's thicknesses or any other named length leave it out; a parameter whose element is not read is not seen), and its two sides must be the box's two sides in either order within 1e-4 ft. On the office as built (`probe_re291_column_type_parameters.rs`, run 41140980550) the square concrete types (PI-1, PI-7, PI-8 and PI-9) hold one length parameter, `b`, the box's side; the rectangular concrete types (PI-3, PI-10, PI-11) hold `h` and `b`, the box's two sides; the hollow sections hold `b`, `Ht`, `t` and `kr` (RE-221); and the one round column's type (`Round Column:PI-6`) holds one parameter, `Diameter` (0.492126 ft), and no `b`. A type read by other captions gets none until a file with one is measured;
- the record's pair steps all name an element rvt-rs exports (`native_column_joins::ColumnJoins::cut_partners`, `clean_partners`; a step whose `m_otherId` is not read leaves the column out, and so does a column whose steps were not read at all), whether or not a `CleanPairGStep` joins it (RE-247: Revit draws every column no clean pair joins as an extrusion on the office, 93 as built and 71 at permit, and its `Length` is RE-247's; but a Cut Geometry command can leave a `CutPairGStep` with no `CleanPairGStep`, and the mechanism of this report would draw such a column as a mesh, so its cut pairs are checked too: the 13 and 15 unjoined columns the rule takes have cut pairs that are all slabs lying clear, or none), and
  - every element its cut pairs name is a slab lying wholly above or wholly below the column, so that it cannot cut it;
  - every element its clean pairs name is a wall, a covering or a column, or a slab lying wholly above or wholly below it. A beam, roof or footing is not accepted: the only columns the office pairs with a beam are 8 hollow-section ones (RE-221's), and no box-drawn column is paired with one, so a beam's extent is not measured here.

`export_content/column_quantities.rs` writes `CrossSectionArea` (width times depth), `OuterSurfaceArea` (the perimeter times the height), and `GrossVolume` and `NetVolume` (the area times the height). Any other pair step, a wall or covering that cuts it, a slab across an end, or an element rvt-rs does not export leaves a column out. The 3 extruded columns with a slab across their top by a clean pair are left out although Revit draws them as their extrusion: nothing measured says a clean pair alone never cuts, and 3 of 3 is not that.

Measured offline on the probes' output joined to Revit's quantities (columns that are not hollow-section ones), and confirmed by the census of the unfiltered run:

| | office as built | office at permit |
|---|---:|---:|
| columns drawn as their box whose type is a plain rectangle | 199 | 289 |
| ... of them joined by a clean pair | 186 | 274 |
| ... taken by the rule | 199 | 265 |
| ... the four quantities equal Revit's, of those taken | 199 | 265 |
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

The office as built has no meshed column. On the 327 extruded columns at permit rvt-rs gives `Length` to 71 (RE-247's: none joined by a clean pair), `CrossSectionArea`, `OuterSurfaceArea` and `GrossVolume` to 320 and `NetVolume` to 265, and on the 279 as built `Length` to 93, the three to 278 and `NetVolume` to 199 (checked in rvt-rs's own IFC from run 41183092458 against Revit's: 0 of 18 for each of the five quantities). The unfiltered census agrees: of Revit's column quantities, none is different and none is one Revit's export leaves out (0 to 0 on both offices, the R2025 models and RE1).

## 4. What stays unwritten

- The round column (3053209, `Round Column:PI-6`, 150 mm; one on each file): its type holds a `Diameter` and no `b`, so it gets none. The box's `CrossSectionArea` is 0.0225 m² where Revit's is 0.01766, and rvt-rs draws it as a box (W6). Circle quantities from the diameter would be the next reading ([#1203](https://github.com/DrunkOnJava/rvt-rs/issues/1203)).
- A column whose type does not hold `b` (another family's captions): none, until a file with one is measured.
- A hollow-section column joined by a clean pair: none on either file.
- The 18 meshed columns' `CrossSectionArea` and `NetVolume`, which Revit writes and the box does not give.
- `Length`: see §5.

## 5. What this changes for a joined column's Length

The maintainer left a joined column's `Length` out (2026-10-26) because Revit's export draws 18 of them as a mesh without one and no field tells them apart (RE-247 §7). On the columns this rule takes that a clean pair joins, Revit writes a `Length` on every one (186 of 186 as built, 250 of 250 at permit) equal to the box's height. A `Length` on exactly those columns would therefore be written equal on 436 more column quantities, none Revit's export omits: it is the rule's own set, and the rule separates the meshes. rvt-rs does not write it here and leaves joined columns' `Length` out as decided on 2026-10-26: whether to write it is the maintainer's call, tracked in [#1203](https://github.com/DrunkOnJava/rvt-rs/issues/1203).

Census of the final run (run 41203821396, this change at 92cca542 against `main` at 54651687): the office as built goes from 28,329 to 29,125 of the 39,388 quantities equal (missing 11,058 to 10,262) and at building permit from 28,254 to 29,314 of 39,679 (missing 11,424 to 10,364); different stays 1 (the mullion) and the quantities rvt-rs writes that Revit's export does not stay 0 on both; the R2025 models (787 of 1,515), RE1 Mechanical (31) and RE1 Plumbing (63) are unchanged, and Core Interior's and RE1 Architecture's counted-apart figures (2,268 and 92) are unchanged. Per quantity, equal of the columns Revit writes it on: `CrossSectionArea` 79 to 278 of 279 as built and 55 to 320 of 345 at permit, `OuterSurfaceArea` and `GrossVolume` 79 to 278 of 279 and 55 to 320 of 327, `NetVolume` 0 to 199 of 279 and 0 to 265 of 345, `Length` 93 of 279 and 71 of 327 as before.

Held-out models (run 41203821396, scrubbed counts, and run 41180541556 before the review changes): the AA house, modelo_bim and teste_export_2025 have the same counts as `main`'s on all three, so the rule writes no column quantity on them; the 2 quantities rvt-rs writes on the AA house and the 3 on teste_export_2025 that Revit's export does not write are on `main` already (a floor's and a covering's `Width`, RE-250, RE-295 and RE-297, named by #1222), not this change's. The 13 and 15 unjoined columns the rule writes have only clear slabs among their cut partners, so the cut-slab check on unjoined columns changes no count (199 and 265, and 0 of the 18 meshed columns, again in the kept IFC).

## 6. Reproduce

```bash
gh workflow run measure.yml --ref main -f ref=5458791ee0fe6dae21361b3e38066d4f65b826d1 -f base=none -f nordic=true \
  -f models="nordic-office-*" -f scorers=none -f probe=probe_re291_column_pairs -f probe_scorer=-
gh workflow run measure.yml --ref main -f ref=92cca5423c0bbb40d2aae8c9e260cb6db70b9923 -f base=main -f nordic=true -f r2025=true -f keep_ifc=true
```

The probe's lines `col <tag> z <base> box <w> <d> <h> pairs <step>:<other>:<entity>:<base>:<height> ...` join to Revit's bodies by `Tag` (a column is meshed where its `Qto_ColumnBaseQuantities` has no `Length`).
