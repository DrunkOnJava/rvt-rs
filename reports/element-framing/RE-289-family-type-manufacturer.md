# RE-289: a family type's Manufacturer is not where RE-77 reads type texts

**Date:** 2026-10-30
**Issues:** [#1185](https://github.com/DrunkOnJava/rvt-rs/issues/1185), W7 of [`docs/release-1.0-criteria.md`](../../docs/release-1.0-criteria.md); follows RE-77
**Artefacts:** the BIM4LCA office as built (Revit 2024, CC BY-SA 4.0) with Revit's IFC4 export (exporter 24.0.5.432), as Measure fetches them
**Probe:** `probes/examples/probe_type_text_parameters.rs`, Measure run [41138121530](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41138121530) (`-f nordic=true -f models=nordic-office-as-built -f scorers=none`, commit 5d30bdb0)
**Status:** negative. Nothing is written.

## 1. The values

Revit's export of the office writes `Pset_ManufacturerTypeInformation.Manufacturer` (`IfcLabel`) on six family type objects: `IfcSanitaryTerminalType` 1414255 (`IDO`), 2575395 (`Ido`) and 1611986 (`Väinö Korpinen Oy`), `IfcLightFixtureType` 3365322 and 3376911 (`Thorlux Lighting`) and `IfcFurnitureType` 1533300 (`Korpinen Oy`). None of the instances of those types carries the property; it is on the type object alone. rvt-rs writes none (`Pset_ManufacturerTypeInformation.Manufacturer`: 6 of 6 missing in the census).

## 2. The probe

RE-77 reads a type's text parameters as entries `i64 BuiltInParameter · u32 n · UTF-16 × n` that follow the type's value block (`partition_type_parameters`). The probe looks for each of Revit's six texts, and for the Fire Rating texts `EI60` and `EI30`, as such an entry in every inflated partition of the office as built, and prints each hit's parameter id and the ElementId that opens the block it follows.

| text | entries found |
|---|---|
| `IDO` | 12 copies, parameter -1010108, in the block of ElementId 1412600 |
| `Ido`, `Väinö Korpinen Oy`, `Thorlux Lighting`, `Korpinen Oy` | none |
| `EI60` | 3, parameter -1001206 (RE-77's Fire Rating), the probe's own owner read does not give an ElementId |

## 3. Result

- Five of the six Manufacturer texts are in no partition as an entry of RE-77's form, so a reader of that form cannot give them. Where they are stored is not shown: the probe looks for UTF-16 text only, and these may be stored in another encoding or in a structure it does not search.
- The one hit, `IDO` under -1010108, belongs to the block of ElementId 1412600, which is none of Revit's type Tags (the type Revit writes it on is 1414255). The id -1010108 is numerically near Description (-1010103), but nothing here shows it is the Manufacturer parameter, and a parameter read on one of six types does not make a rule.
- The probe takes the 8 bytes before the value-block mark as the block's owner; RE-77's reader checks them against the declared ElementIds, so the probe's owner for the `EI60` hits is not an ElementId. Whether rvt-rs reads a type's Fire Rating is therefore judged by RE-286's census, not by this table.

## 4. What rvt-rs does

Nothing. The six values stay missing (RE-286 §6). Where a family type's Manufacturer is stored is not known, so no reader is written.

## 5. Reproduce

```bash
gh workflow run measure.yml --ref main -f ref=5d30bdb05272108fe14b0148b6cd5f29bc3e2c6d -f base=none \
  -f nordic=true -f models="nordic-office-as-built" -f scorers=none \
  -f probe=probe_type_text_parameters -f probe_scorer=-
```
