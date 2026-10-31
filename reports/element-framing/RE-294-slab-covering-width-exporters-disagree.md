# RE-294 — A floor's and a covering's Width: Revit's exporters disagree, so none is written

**Date:** 2026-10-31
**Issues:** #1046 (quantities Revit computes from its joined geometry), #1016 (quantities), W7 of [`docs/release-1.0-criteria.md`](../../docs/release-1.0-criteria.md); withdraws part of RE-250 (a ceiling's `Width`), RE-295 (a compound floor's `Width`) and RE-297 (a ceiling in pieces' and a covering wall's `Width`)
**Artefacts:** the BIM4LCA office as built and at building permit (Revit 2024, Revit 24.0.5's IFC4 exports, CC BY-SA 4.0, `research/bim4lca-measure.tsv`), puzzbobb's R2025 models (Revit 2025, Apache-2.0), and the held-out models of `research/holdout-models.tsv` (counts only, the licence not allowing more)
**Runs:** RUN_LIST
**Result:** positive as a withdrawal. A standing rule of the maintainer (2026-10-31): where Revit's exporters disagree on a value, rvt-rs leaves it out. Revit 2024's exporter writes `Qto_SlabBaseQuantities.Width` on a compound floor and `Qto_CoveringBaseQuantities.Width` on every ceiling and on the walls it exports as coverings; the exporters that wrote two held-out models (a Revit 2026 file and a Revit 2025 file) write neither, and rvt-rs's rules wrote 2 floor values and 3 covering values Revit's export omits there. Nothing in a file says which exporter will write it (an older file can be exported by a newer Revit), the rules' only evidence is the one building, so all three `Width` rules are removed: the held-out models show no quantity rvt-rs writes that Revit's export omits, and the office loses the values listed in §3 as missing, none as different.

## 1. The disagreement

On the office (exporter 24.0.5) Revit writes a covering's `Width` on all 111 coverings as built and all 75 at permit, and a floor's `Width` on 34 (as built) and 35 (at permit) floors, 14 of them the compound floors whose `Width` is their type's layers' total thickness (RE-295). On the held-out models, scrubbed to counts (`holdout-*/counts.txt`):

| held-out model | Revit's `Qto_CoveringBaseQuantities` | Revit's `Qto_SlabBaseQuantities.Width` | rvt-rs wrote that Revit omits |
|---|---|---|---|
| AA house (Revit 2026 exporter) | none | 3 floors, none one rvt-rs wrote | 2 floor `Width` values |
| teste_export_2025 (Revit 2025 file) | `GrossArea` and `NetArea` on 3 coverings, no `Width` | 1 floor | 3 covering `Width` values |
| modelo_bim | none | 1 floor | 0 |

The AA house's exporter writes a floor's `Width` on 3 floors that rvt-rs's rule did not take and none on the 2 it did; teste_export_2025's writes `GrossArea` and `NetArea` on its coverings and no `Width`, where the office's writes `Width` and `GrossArea` and no `NetArea`. So the set of quantities on a covering changed between releases, and the compound floors' `Width` is one more value one exporter writes and another does not.

## 2. Why nothing narrower

The non-held-out models that could separate the cases are the office (Revit 2024, one building) and the R2025 models (Revit 2025), which hold no floor with a `Width` and no covering. The file's release is what separates them, but the exporter that wrote the IFC is not the file's release: a Revit 2024 file opened and exported in Revit 2026 gets the later exporter's values. A rule gated to a file of release 2024 or earlier would keep the office's values at the risk of writing a value Revit omits whenever a later Revit exports it, so under the standing rule it is not taken; if the maintainer decides a release gate is wanted, it restores the office values below without changing anything else.

## 3. What was removed and what it costs

MEASURED_SECTION

## 4. Reproduce

REPRODUCE
