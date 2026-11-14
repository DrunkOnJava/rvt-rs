# RE-469: a column that names three Levels and a footing that names none, on Revit 2027

**Date:** 2026-11-14
**Issues:** #1842 (elements on no storey in a Revit 2027 structural model); follows RE-454 (a beam's or brace's storey is its Reference Level's), RE-27 and RE-59 (the Level a record names, and the base constraint of two), RE-68 (elevation-placed classes)
**Probes:** `probes/examples/probe_1842_storey_rules.rs` and `tools/re/storey_rules_vs_ifc.py` (the rules scored against Revit's export, counts only without `--list`)
**Artefacts:** a field reporter's Revit 2027 structural model, saved on Revit 27.4, and Revit's IFC4 export of that save (confidential; the reporter's counts only, #1842)
**Runs:** the reporter's run of the probe build [43544251294](https://github.com/DrunkOnJava/rvt-rs/actions/runs/43544251294) (comment on #1842 of 2026-11-14); Measure RUN_FINAL
**Status:** Positive on that one model, shipped for Revit 2027 only and labelled. No model of ours with Revit's export holds either case, so neither rule is measured on a second project.

## 1. What the reporter's run showed

On the save, 0.22.0's default export read by `storeys_vs_ifc.py` against Revit's put 51 of 2,417 elements on no storey (48 `IfcColumn`, 3 `IfcFooting`) and 27 on another (21 `IfcBuildingElementProxy`, 6 `IfcFooting`). The storey-rule scorer sets each rule beside Revit's storey, grouped by the IFC class rvt-rs writes and by how many Levels the element's record names:

| Class | Levels named | Elements | `lo` | `<base` | `base*` |
|---|---:|---:|---:|---:|---:|
| `IfcColumn` | 3 or more | 48 | 48 | 48 | 27 |
| `IfcFooting` | none | 88 | - | - | 88 |
| `IfcSlab` | none | 14 | - | - | 0 |
| `IfcStairFlight` | none | 18 | - | - | 1 |

(`lo` is the lowest Level the record names, `<base` the highest of them at or below the record box's base, else the lowest, `base*` the highest Level of the model at or below the base; a dash is a rule that picks no Level where the record names none.) The 14 slabs that name no Level are stair landings, and the flights and landings are parts of their `IfcStair` in both exports: Revit contains the stair, which rvt-rs also puts in Revit's storey (8 of 8), so they are in no storey of their own in either file.

## 2. What rvt-rs writes

On Revit 2027 only:

- A column whose record names three or more Levels is in the storey of the highest of them at or below its record box's base, else of the lowest (`<base`, `below_base_level`). `LevelBindSource` is `partition_element_record_column_base_level`.
- A footing whose record names no Level is in the storey of the highest Level of the model at or below its record box's base (`base*`, `level_at_or_below`), placed ahead of the elevation band of RE-68. `LevelBindSource` is `partition_element_record_footing_base_level`.

A column that names two Levels, a beam, and every element that names one are bound as before. The rule for a footing also reaches a Structural Foundations floor that names no Level, which the reporter's model does not hold.

## 3. Measured

RESULTS_SLOT

## 4. Not shown

- **A second project.** "No model of ours with Revit's export holds a column that names three Levels." "No model of ours with Revit's export holds a footing that names no Level." Both rules rest on the reporter's model, which is why they are gated to its release and tagged, so that an element they place is counted apart from one bound by a Level its record names.
- **Other releases.** A Revit 2023 to 2026 structural model with the same cases is not measured; those elements stay unbound there, as before.
- **The 21 proxies.** The 21 `IfcBuildingElementProxy` on another storey than Revit's are outside the probe's classes.
- **The diagnostics miscount.** The 14 landings and 14 flights that `unsupported_geometry_missing_level` counted as storeyless are a count of the parts of a stair, fixed apart from the rules (#2012).

## 5. Reproduce

```bash
cargo run --profile ci -p rvt-probes --example probe_1842_storey_rules -- MODEL.rvt > storey.txt
python3 tools/re/storey_rules_vs_ifc.py storey.txt <Revit's export of the save>.ifc
```

Neither file leaves the reporter's machine; only the counts the scorer prints without `--list` are posted.
