# RE-294 — A floor's and a covering's Width: Revit's exporters disagree, so none is written

**Date:** 2026-10-31
**Issues:** #1046 (quantities Revit computes from its joined geometry), #1016 (quantities), W7 of [`docs/release-1.0-criteria.md`](../../docs/release-1.0-criteria.md); withdraws part of RE-250 (a ceiling's `Width`), RE-295 (a compound floor's `Width`) and RE-297 (a ceiling in pieces' and a covering wall's `Width`)
**Artefacts:** the BIM4LCA office as built and at building permit (Revit 2024, Revit 24.0.5's IFC4 exports, CC BY-SA 4.0, `research/bim4lca-measure.tsv`), puzzbobb's R2025 models (Revit 2025, Apache-2.0), and the held-out models of `research/holdout-models.tsv` (counts only, the licence not allowing more)
**Runs:** [41204218380](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41204218380) (this change at d6cf2fa6 against `main` at 904861a5, every model and scorer, with the held-out models) and, for the extras the held-out models showed before this change, [41182558730](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41182558730) (`main` at 1a54a8f3, the scrub of #1222: 2 slab and 3 covering `Width` values)
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

Measure [41204218380](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41204218380): this change at d6cf2fa6 against `main` at 904861a5, every model and every scorer, with the held-out models (`-f base=main -f nordic=true -f r2025=true -f holdout=true`, the scrub that names the extra quantities from #1222). Only the exports of the two offices, Core Interior, RE1 Architecture and the two held-out models that had extras move (`ifc.sha256`, `qtos_vs_ifc`, `counts.txt`, Core Interior's witness observations); the other 1,463 outputs are byte-identical to `main`'s.

| rule removed | Revit's export that omitted it | equal lost, office as built | equal lost, office at permit |
|---|---|---:|---:|
| a compound floor's `Qto_SlabBaseQuantities.Width` (RE-295) | the AA house's, Revit 26.4: 2 compound floors | 14 | 14 |
| a ceiling's `Qto_CoveringBaseQuantities.Width` (RE-250) | teste_export_2025's (a Revit 2025 file): its 3 coverings carry `GrossArea` and `NetArea` and no `Width` | 59 | 55 |
| a ceiling in pieces' `Width` (RE-297) | the same export, same coverings | 19 | 20 |
| a wall exported as a covering's `Width` (RE-297, the 21 walls of the type `AK-1.otsa`) | the same export; no held-out model exports a wall as a covering, so this rule is withdrawn with the others, on the same evidence | 21 | 0 |
| total | | 113 | 89 |

The scrubbed held-out counts name the quantity (`extra Qto_SlabBaseQuantities.Width: 2`, `extra Qto_CoveringBaseQuantities.Width: 3`), not the element, so which of the three covering rules wrote teste_export_2025's 3 values is not recorded; the three rest on the office alone, and a later exporter that writes none on a covering writes none on any of them.

| | office as built | at permit | R2025 (24 models) |
|---|---:|---:|---:|
| quantities equal, before | 28,329 | 28,254 | 787 |
| ... after | 28,216 | 28,165 | 787 |
| missing, before | 11,058 | 11,424 | 728 |
| ... after | 11,171 | 11,513 | 728 |
| different, before and after | 1 (the mullion) | 1 | 0 |
| written that Revit's export omits, before and after | 0 | 0 | 0 |

Held-out models (scrubbed counts, before and after): the AA house 846 quantities, 127 equal, 0 different, 719 missing, and **2 that Revit's export omits to 0**; modelo_bim 220, 7, 0, 213, and 0 to 0; teste_export_2025 44, 10, 0, 34, and **3 to 0**. No held-out equal count falls: neither export wrote a `Width` rvt-rs had right. `Qto_CoveringBaseQuantities.GrossArea` (57 of 90 and 45 of 75 on the office) and `Qto_SlabBaseQuantities.NetVolume` (31 of 63 on both) are as on `main`. Core Interior's and RE1 Architecture's exports were made with base quantities off (RE-242): the values counted apart fall by 1 (a slab `Width`; 2,384 to 2,383) and by 8 (6 covering and 2 slab `Width` values; 103 to 95), and Core Interior's witness observations lose one `IFCQUANTITYLENGTH` (1,340 to 1,339).

## 4. Reproduce

```bash
gh workflow run measure.yml --ref main -f ref=d6cf2fa64517aab996d98150371acf27dbc64f28 -f base=main -f nordic=true -f r2025=true -f holdout=true
```

`--ref main` carries the held-out scrub of #1222 (`extra Qto_X.Y: N` lines in each `holdout-*/counts.txt`); before it lands, use `--ref ci/holdout-extra-quantities`. The office's per-element values are in `qtos_vs_ifc.txt`'s `Qto_SlabBaseQuantities.Width` and `Qto_CoveringBaseQuantities.Width` lines: 34 and 111 Revit values as built, 35 and 75 at permit, all missing after this change.
