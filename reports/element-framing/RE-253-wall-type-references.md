# RE-253 — Which type a wall's records name

**Date:** 2026-10-27
**Issues:** #1048 (a wall's `Width` differs from Revit's on 1 of 3 walls of a held-out Revit 2025 model), W7 of [`docs/release-1.0-criteria.md`](../../docs/release-1.0-criteria.md); follows RE-28 (a wall's type is the one type definition its reference list names), RE-216 (data copies) and RE-252 (the held-out models)
**Artefacts:** held out, 255ribeiro's `teste_export_2025` and `modelo_bim` (Revit 2025, Revit 25.4's IFC4 exports) and the flowbim.ee house (Revit 2026), fetched for each run from `research/holdout-models.tsv`, only counts uploaded (`tools/ci/scrub-holdout-results.sh`); the BIM4LCA office as built and at building permit and puzzbobb's R2025 01 and 18 as controls
**Runs:** Measure (`examples/probe_re253_wall_type_refs.rs`, from the third with `tools/re/export_after_save.py`) [40446216295](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40446216295), [40446792523](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40446792523), [40448408486](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40448408486), [40449070841](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40449070841), [40449735663](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40449735663), [40450429155](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40450429155) and [40451116606](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40451116606)
**Result:** negative for a reader defect: every reading of the saved file gives the wall of #1048 the 200 mm type rvt-rs gives it, while Revit's export, made some hours after the file was last saved, gives it a 300 mm type the file also holds. The export most likely shows a change made after the last save; that is not proven.

## 1. The wall

On `teste_export_2025` rvt-rs writes one wall's `Qto_WallBaseQuantities.Width` as 0.666667 of Revit's, with the `Name` and `ObjectType` of a sibling type: its type's name ends in "200 mm" where Revit's export names a "300 mm" type, and its material layer set's names differ (#1048). The model carries no licence, so each run below uploads only counts.

## 2. What the saved file says

| reading | held-out walls | result |
|---|---|---|
| the type definitions each wall's reference list names (RE-28) | 59 (house), 6 (`teste_export_2025`) | exactly one on every wall, and it is the type rvt-rs picks |
| the type the wall's native `SWall` record names (`m_WallAttributesId`, the current copy, RE-216) | 59, 6 | the type rvt-rs picks on every wall |
| the copies of each picked type's data (RE-216) | 11 types, 2 types | one copy and one name each; nowhere does the latest partition's name, which the export takes, differ from the routed current copy's |
| the export's wall types among the file's own type names | 11 of 11, 2 of 2 | every one there: the file holds the 300 mm type too |

On the office at permit, where RE-145 found older copies, 6 of 30 picked types have data in more than one partition and 1 has two names, but none has a latest name other than its current copy's: the export's choice of copy is not what differs on `teste_export_2025` either.

So the file's current record of the wall names the 200 mm type, which has one copy and one name. The two-types hypothesis (a list naming a second type the scan does not take for a definition) is refuted, and so is a stale copy.

## 3. When the export was made

The file's save time (its `ProjectInformation` Atom `updated`, UTC) against the time in Revit's export's `FILE_NAME` header (the exporting machine's clock, no zone): `teste_export_2025` was exported 8.9 hours after its last save by those clocks, `modelo_bim` 600 hours after, R2025 01 2.1 and R2025 18 0.0. Revit exports the model as it is open, saved or not, so an export some hours after the last save can show changes that were never saved.

## 4. The wall's own thickness

Two readings that do not go through its type:

- The smaller side in plan of each wall's element-record box: equal to Revit's `Width` on 606 of the office's 607 walls, all 59 of the house's and all 4 of `modelo_bim`'s, but wider on all 3 compared walls of `teste_export_2025`, which run off the model's axes, so it says nothing there.
- The distance between the outermost two face planes of each wall's native record: on `teste_export_2025` it equals Revit's `Width` on 2 of the 3 walls and is 0.666667 of it on the third, the 200 mm of rvt-rs's type against Revit's 300 mm. But those planes are not always a wall's sides: they are thinner than Revit's `Width` on 191 of the office's 596 walls as built (0.74 of it on most) and on 51 of the house's 59, layered walls among them. On a single-layer wall they are its sides, so on the third wall they say 200 mm only if that wall's type has one layer.

## 5. What follows

Nothing in the saved file says the wall is of the 300 mm type, and each reading that can say which it is says 200 mm. rvt-rs's `Width` there is the saved file's, and the export most likely describes an unsaved state of the model. The maintainer chose (2026-10-27) to treat #1048 as an inconsistency of that reference pair, not a reader defect: `tools/re/qtos_vs_ifc.py` counts a wall's `Width` apart where the two exports give the wall different types (`ObjectType`) and the Widths differ, the type itself being scored by the names census, and `research/holdout-models.tsv` records it. No export changes.

The face planes correct RE-252 §2, which took the outermost two for the wall's sides on every wall. RE-252's rule takes a wall's end only where that reading of the record agrees with the partition join reach (§3), and every `Length` it writes is Revit's (§4), so the correction changes no output.

## 6. Reproduce

```bash
gh workflow run measure.yml --ref main -f ref=eac7a2a95075e30641df6ff83f50282785c5d048 -f base=none -f holdout=true -f nordic=true \
  -f models="holdout-* nordic-office-as-built" -f scorers=none \
  -f probe=probe_re253_wall_type_refs -f probe_scorer=export_after_save
```
