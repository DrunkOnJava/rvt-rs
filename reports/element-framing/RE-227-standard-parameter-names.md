# RE-227 — The names Revit's export reads a standard property's parameter by

**Date:** 2026-10-25
**Issues:** #35 (parameters), W7 of [`docs/release-1.0-criteria.md`](../../docs/release-1.0-criteria.md); follows RE-194
**Artefacts:** the BIM4LCA office as built and at building permit (Revit 2024) with Revit's IFC4 exports, as Measure fetches them
**Runs:** Measure [40229722600](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40229722600) (`examples/probe_re227_parameter_names.rs`), [40228451279](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40228451279) (`main` with #992's `psets_vs_ifc`) and [40231201823](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40231201823) (this change against `main`)
**Result:** positive for the bare name; the display name is a separate, derived and verified slice.

## 1. Where Revit's export looks

revit-ifc's `PropertySetEntryMap.CreatePropertyFromElementOrSymbol` (CLEANROOM.md source 5) fills a standard property from a parameter on the element, trying in turn:

1. the property's display name in the export's language (`Fire Exit` in English);
2. `<set>.<property>` (`Pset_DoorCommon.FireExit`);
3. the property's own name (`FireExit`);

and then the same on the element's type. RE-194 reads the second on the element. The display names are a table in revit-ifc, which this tree does not copy (CLEANROOM.md); how rvt-rs gets them is a separate slice (§4).

## 2. What the office defines

`probe_re227_parameter_names` lists every project and shared parameter definition and marks those named for a standard property. Of the as-built file's 836 definitions, 58 are qualified names RE-194 already reads (`Pset_DoorCommon.HandicapAccessible`, `Pset_SlabCommon.LoadBearing`, `Pset_DoorWindowGlazingType.IsLaminated`, ...), five are bare property names (`Category`, `Description`, `Finish`, `FireExit` with ElementId 1104147, `Name`) and nine display names (`Acoustic Rating`, `Ceiling Covering`, `Fire Rating`, `Floor Covering`, `Frame Thickness`, `Nominal height`, `Nominal width`, `Skirting Board`, `Wall Covering`). The permit file has one qualified definition fewer.

Revit's export writes `Pset_DoorCommon.FireExit` false on all 266 of the office's doors that carry the parameter, and rvt-rs wrote none (#992's listing of the values Revit writes where rvt-rs writes none).

## 3. What rvt-rs does

`attach_named_parameters` now also takes a parameter named for a property alone, for every standard set holding a property of that name, where the element has no `<set>.<property>` value for it: the qualified name first, as Revit's export does. The value is written only in the sets that apply to the entity, as before.

## 4. Measured

`psets_vs_ifc`, this change against `main`:

| file | equal | different | written where Revit writes none |
|---|---|---:|---:|
| office as built | 32,167 → 32,433: `Pset_DoorCommon.FireExit` 266 of 266 | 55, unchanged | 495, unchanged |
| office at permit | 32,339 → 32,605: the same 266 | 55, unchanged | 496, unchanged |

Of the five bare names only `FireExit` gives a value on an element Revit's export writes; the other four change nothing. Core Interior, Einhoven and the four RE1 models export byte for byte as before: none of them defines such a parameter.

## 5. Next

The display names (`Floor Covering`, `Ceiling Covering`, `Acoustic Rating`, ...): each is derived from the buildingSMART property name by splitting it at its capitals, and used only once a Revit export confirms it (the maintainer's decision, 2026-10-25). `Nominal height` is not what that rule gives `NominalHeight`, and stays unread until an export shows Revit reads it.

## 6. Reproduce

```bash
gh workflow run measure.yml --ref feat/bare-named-parameters -f ref=feat/bare-named-parameters -f base=none \
  -f nordic=true -f probe=probe_re227_parameter_names
gh workflow run measure.yml --ref main -f ref=feat/bare-named-parameters -f base=main -f nordic=true
```
