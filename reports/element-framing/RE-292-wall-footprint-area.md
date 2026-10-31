# RE-292 — A wall's GrossFootprintArea

**Date:** 2026-10-30
**Issues:** #1046 (quantities Revit computes from its joined geometry, step 1: walls), #1016 (quantities), W7 of [`docs/release-1.0-criteria.md`](../../docs/release-1.0-criteria.md); follows RE-252 (`GrossFootprintArea` is `Length` × `Width` on every wall that has one, and Revit writes it only where its exporter draws the wall as an extrusion) and RE-270 (the walls whose side areas and volumes rvt-rs writes)
**Artefacts:** the BIM4LCA office as built and at building permit (Revit 2024, Revit 24.0.5's IFC4 exports, CC BY-SA 4.0, `research/bim4lca-measure.tsv`) and puzzbobb's R2025 models (Revit 2025, Apache-2.0, `research/r2025-models.tsv`), as Measure fetches them
**Scorer:** `tools/re/wall_quantities_vs_ifc.py` (RE-270)
**Runs:** Measure [41136551451](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41136551451) (`main` at f73908bf: the scorer and `probe_1067_native_dump`, both offices and the R2025 models) and [41146199810](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41146199810) (this change at e2e273bd against `main` at 9637d5ff, every model and scorer)
**Result:** positive, narrowed. Revit's export writes a wall's `GrossFootprintArea` on 186 of the 188 walls the first RE-270 rule takes (a record box cut back at its joins, nothing joined to or cutting the wall); the two without are the same wall in the two office files, 0.164 ft long across a 0.328 ft thickness, and the next shortest wall is 3.12 times as long as it is thick. rvt-rs writes it, `Length` times `Width`, on the walls it writes the four RE-270 quantities on by the first rule (a record box cut back at its joins, nothing joined to or cutting the wall) that are longer than three times their thickness: 48 of the office's walls as built, 47 at permit and 91 R2025 walls, Revit's on every one, none different and none Revit's export leaves out.

## 1. Where Revit's export gets it

revit-ifc's `PropertyUtil.CreateWallBaseQuantities` (CLEANROOM.md source 5; read at the tags `IFC_v24.1.0` and `IFC_v24.2.0`) writes `GrossFootprintArea` when `WallExporter` hands it a footprint area. `WallExporter.TryToCreateAsExtrusion` sets that area from the wall's boundary loops (`ExporterIFCUtils.ComputeAreaOfCurveLoops`) once Revit's own check (`CanExportWallGeometryAsExtrusion`, an internal call) says the wall can be reproduced as an extrusion with clippings and openings; the other route, `BodyExporter` exporting the whole solid as one vertical extrusion, hands it the extrusion's area instead. In Revit's IFC4 Reference View export a wall with clippings or openings is tessellated though the area is the same. The check is Revit's, not the model's, so it cannot be read; it can only be measured.

## 2. Measured against the record

On the office, `GrossFootprintArea` is written on 279 walls as built and 276 at permit and is `Length` × `Width` on all of them (RE-252). Splitting the walls by what their records hold:

| walls whose record holds no step but the harmless ones (RE-270 §4), drawn as the record's box cut back at its joins | as built | at permit | R2025 |
|---|---:|---:|---:|
| walls | 49 | 48 | 91 |
| ... Revit writes `GrossFootprintArea` | 48 | 47 | 91 |
| ... and it is the body's plan area, `Length` × `Width` | 48 | 47 | 91 |
| ... Revit writes none | 1 | 1 | 0 |

The wall Revit gives none (3184170 in both files, 0.164 ft long across a 0.328 ft thickness, so its long side is its thickness) is extruded all the same, and the shortest wall Revit writes one on is 3.12 times as long as it is thick. That is one wall (the same wall in two files), so what decides it cannot be told from this corpus: it may be its length against its thickness, or something else about it that the record does not show. The rule therefore claims only what is measured. Every wall longer than 3.12 times its thickness that the rule takes has a footprint in Revit's export (186 of 186: 48, 47 and 91), and no wall between 1.0 and 3.12 times was measured, so the rule writes the footprint on walls longer than three times their thickness and on none shorter.

The layered walls rvt-rs draws from their layers (a body that is not the box) are extruded in Revit's export too, 5 as built and 3 at permit among the walls with only harmless steps, and carry none.

## 3. What stays unwritten

- Walls joined to roofs or floors (RE-270's second rule's walls and the rest): Revit writes the footprint on 78 of the 85 record-box walls with only roof and clean joins as built and 78 of 93 at permit, all drawn as a tessellated body, and none on the others (drawn as an extrusion, apart from the 2 as built it writes only a `NetVolume` on). Nothing the record holds separates them: of the 258 record-box walls as built, the best single feature of any step's content (its flags, history counts, list lengths and the classes of the record) predicts the footprint on 216 and is wrong on 42.
- Walls with openings, cut pairs, embedded walls or sweeps, and layered walls: their bodies are not the box.

## 4. Measured

This change at e2e273bd against `main` at 9637d5ff, every model and every scorer (run [41146199810](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41146199810), `-f base=main -f nordic=true -f r2025=true`). The exports of both offices, Core Interior, RE1 Architecture and the 23 R2025 models that have walls change (their `ifc.sha256` and `qtos_vs_ifc` outputs, and Core Interior's witness observations); the other 1,398 scorer outputs are byte-identical to `main`'s.

| | office as built | at permit | R2025 (24 models) |
|---|---:|---:|---:|
| `GrossFootprintArea` equal, before | 0 of 279 | 0 of 276 | 0 of 138 |
| ... after | 48 of 279 | 47 of 276 | 91 of 138 |
| different, before and after | 0 | 0 | 0 |
| written that Revit's export omits, before and after | 0 | 0 | 0 |
| quantities equal, before | 28,065 | 28,011 | 775 |
| ... after | 28,113 | 28,058 | 866 |
| quantities scored | 39,388 | 39,679 | 1,515 |

`GrossSideArea` and the other three walls' quantities are as on `main` (67 of 579 as built, 70 of 565 at permit, 91 of 138 on R2025). Core Interior's and RE1 Architecture's exports were made with base quantities off (RE-242): the 139 and 1 footprint areas rvt-rs now writes on their walls are counted apart (Core Interior 2,268 to 2,407, RE1 Architecture 86 to 87), and Core Interior's witness observations gain 139 `IFCQUANTITYAREA` (491 to 630); both verdicts replay PASS with the verdict files unchanged.

## 5. Reproduce

```bash
gh workflow run measure.yml --ref main -f ref=e2e273bd9962946c78d7b74771e436c904cdd3ea -f base=main -f nordic=true -f r2025=true
gh workflow run measure.yml --ref main -f ref=f73908bf1b04c0b6d9c9acd8340fe3b5f73af67a -f base=none -f nordic=true -f r2025=true \
  -f models="nordic-office-* r2025-*" -f scorers=wall_quantities_vs_ifc -f probe=probe_1067_native_dump -f probe_scorer=-
```

The second command is the scorer on `main` with the walls' records; join its `wall` lines with the probe's `wall <id> classes` lines by ElementId.
