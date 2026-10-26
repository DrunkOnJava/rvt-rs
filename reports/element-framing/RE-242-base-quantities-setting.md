# RE-242 — Whether an export has base quantities

**Date:** 2026-10-26
**Issues:** #1016 (quantities), W7 of [`docs/release-1.0-criteria.md`](../../docs/release-1.0-criteria.md); follows RE-239 §4
**Decision:** Griffin, 2026-10-26: base quantities stay on by default, `rvt-ifc --no-base-quantities` leaves them out, and the quantity census counts the quantities rvt-rs writes apart where Revit's reference export writes none.
**Artefacts:** the BIM4LCA office as built and at building permit (Revit 2024, Revit's IFC4 exports), RE1 Architecture, Mechanical, Plumbing and Electrical (Revit 2025, Revit's IFC2X3 exports), Core Interior (Revit 2024, Revit's IFC4 export), Einhoven (no export) and puzzbobb's R2025 models (Revit 2025, Revit's IFC4 exports), as Measure fetches them
**Runs:** Measure [40302694804](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40302694804) (`examples/probe_re242_export_setup.rs`, and `tools/re/qtos_vs_ifc.py` on `main` at 74c1fdd8), [40305428818](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40305428818) (this change against `main`) and [40305432150](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40305432150) (this change with `--no-base-quantities`)
**Result:** negative for reading the choice from the model: a model that keeps an export setup keeps two, one with base quantities on and one off, and nothing the probe reads says which made an export. Positive for the default: Revit's built-in IFC4 setups turn them on, and 23 of the 24 IFC4 reference exports with objects have them.

## 1. Revit's setting

revit-ifc (CLEANROOM.md source 5) writes the `Qto_` sets only when its export setup's `ExportBaseQuantities` is true (`PropertySetOptions`, read from the export options of that name). A new setup has it false (`IFCExportConfiguration`), and each built-in setup states its own (`IFCExportConfigurationsMap.AddBuiltInConfigurations`): on in the IFC4 Reference View and Design Transfer View setups, the IFC4x3 ones, COBie 2.4, the IFC2x3 Basic and Extended FM Handover setups and IFC-SG; off in IFC2x3 Coordination View 2.0, IFC2x3 Coordination View and IFC2x2.

## 2. The reference exports

`tools/re/qtos_vs_ifc.py` on `main` at 74c1fdd8 (run 40302694804), the quantities Revit's export writes on objects rvt-rs also writes:

| model | schema | Revit's base quantities |
|---|---|---:|
| office as built, at permit | IFC4 | 50,580, 50,823 |
| R2025 models with objects (21 scored) | IFC4 | 40 to 119 each |
| Core Interior | IFC4 | none: the export writes no property sets either (RE-239 §5) |
| RE1 Architecture, Electrical | IFC2X3 | none |
| RE1 Mechanical, Plumbing | IFC2X3 | none; the 94 pipe and duct lengths the export writes in property sets are scored as base quantities (#822) |

R2025's `00_Blank` has no objects and `07_1Slope_ArrowDefined_2Cut50mmFascia` no export. rvt-rs writes 715 quantities on Core Interior (`Width`, `Height` and `Area` on 132 doors, `Width` on 319 walls) and 29 on RE1 Architecture that their exports do not.

## 3. The model's saved setups

The probe prints every setting with a true, false or number value around each UTF-16 or UTF-8 `"ExportBaseQuantities"` in every stream, stored and inflated (string settings, which hold names and paths, are not printed). Three of the 31 models keep setups, each two, all IFC4 (`IFCVersion` 25):

| model | stream | setup | `ExportBaseQuantities` | `ExportIFCCommonPropertySets` | `ExchangeRequirement` | `ActivePhaseId` | matches the export |
|---|---|---:|---|---|---:|---:|---|
| Core Interior | `Partitions/65` | 1 | true | true | 0 | -1 | no |
| | | 2 | false | false | 0 | 3 | yes |
| office as built | `Partitions/425` | 1 | true | true | 3 | -1 | yes |
| | | 2 | false | true | 0 | -1 | no |
| office at permit | `Partitions/336` | 1 | false | true | 0 | -1 | no |
| | `Partitions/404` | 2 | true | true | 3 | -1 | yes |

The other 28 models (Einhoven, the four RE1 models and the 23 R2025 models) keep none: the probe finds no `"ExportBaseQuantities"` in any stream. In every model that keeps setups one has base quantities on and the other off, and no setting the probe prints marks one as the setup an export used. The setup that matches each export differs from the other in more than this setting, and not in the same way on every file: on Core Interior also in its common property sets and its phase, on the two office files in its `ExchangeRequirement`. Reading the choice from the model would be a guess on the three files that keep setups and impossible on the other 28.

## 4. What rvt-rs does

Base quantities stay on by default, as in each of Revit's built-in IFC4 setups (§1) and in 23 of the 24 IFC4 reference exports with objects (§2). `rvt-ifc --no-base-quantities` leaves every `Qto_` set out (`ifc::leave_out_base_quantities`), for a user matching an export made with them off. Where Revit's reference export writes no base quantities at all (no `Qto_` IfcElementQuantity, nor IFC2X3's `BaseQuantities`), `qtos_vs_ifc` counts the quantities rvt-rs writes apart, not as values Revit does not write.

## 5. Measured

This change against `main` at 67983335 (run 40305428818, `qtos_vs_ifc`): all 31 exports are byte for byte as before. The census now counts the quantities rvt-rs writes where the export has none apart: 715 on Core Interior and 29 on RE1 Architecture, against 0 values Revit does not write on every model.

With `--no-base-quantities` (run 40305432150, `qtos_vs_ifc`, `psets_vs_ifc` and the schema step):

| model | quantities left out | Revit's base quantities equal | the property census |
|---|---:|---:|---|
| office as built | 11,994 | 0 of 50,580 (11,993 by default) | 44,068 of 58,006 equal, 56 different, 504 Revit does not write, as by default |
| office at permit | 11,921 | 0 of 50,823 (11,920) | |
| Core Interior | 715 | none written | |
| RE1 Architecture | 29 | none written | |
| RE1 Mechanical, Plumbing | 31, 63 | 0 of 31 and 63 (all by default) | |
| R2025 `12_Rooms_Door_Windows` | 17 | 0 of 102 (17) | |

IfcOpenShell's validation finds nothing on any of them. RE1's pipe and duct lengths, which Revit's IFC2X3 export writes in a property set and rvt-rs writes as IFC4 base quantities, are left out with the rest: the option leaves every `Qto_` set out.

## 6. Reproduce

```bash
gh workflow run measure.yml --ref main -f ref=main -f base=none -f nordic=true -f r2025=true \
  -f probe=probe_re242_export_setup -f scorers=qtos_vs_ifc
gh workflow run measure.yml --ref main -f ref=main -f base=none -f nordic=true -f r2025=true \
  -f ifc_flags=--no-base-quantities -f scorers="qtos_vs_ifc psets_vs_ifc schema"
```
