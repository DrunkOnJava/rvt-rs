# RE-386: an opening a door cuts carries its door's FireExit

**Date:** 2026-11-09
**Issues:** #1193 (W7: `Pset_OpeningElementCommon.FireExit` is one of the values it lists); follows RE-321 and RE-338 (the openings an element cuts in a wall besides the one it fills, and their `Reference`), RE-194 (a parameter named for a standard property set's property) and RE-382 (the `Reference` of the openings of a host's own geometry), which left this one out
**Artefacts:** the BIM4LCA office as built and at building permit (Revit 2024, Revit 24.0.5's IFC4 exports, CC BY-SA 4.0, as Measure fetches them)
**Runs:** Measure [42658128847](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42658128847) (the property census of the two office files with Revit's set placement, `main` at `060e8f0a`: `Pset_OpeningElementCommon.FireExit` missing on 25 and 33 openings, `False`), and the unfilled-openings scorer of Measure [42657527737](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42657527737) (which openings Revit's export writes, and of what cutter)
**Status:** @@STATUS@@

## 1. What Revit writes

`Pset_OpeningElementCommon.FireExit` is on exactly the openings a door cuts, and always `false`:

| Opening in Revit's export | As built | At permit | `FireExit` |
|---|---:|---:|---|
| a door's, in a wall | 39 | 43 | `false` on all |
| a door's, in a door | 4 | 2 | `false` on all |
| a fire hose cabinet's, in a wall | 39 | 38 | none |
| a curtain wall's, in a wall | 70 | 72 | none |
| of no element (a rectangular wall opening), in a wall | 30 | 24 | none |
| of a floor's sketch or a roof's hatch | 37 keys | 36 keys | none |

The office defines a Yes/No project parameter `FireExit` on its doors, windows and ramps (RE-194), and every one of the 266 doors holds `false` (the 232 windows and 5 ramps too), which Revit's export writes as `Pset_DoorCommon.FireExit` (266 of 266 equal).

revit-ifc (CLEANROOM.md source 5, read and cited, no code copied) was read at the tags `IFC_v24.1.0` and `IFC_v24.3.40`, which agree on every line cited here; none was taken from the default branch, and the office's exporter 24.0.5 has no tag of its own (as RE-270 notes). The chain is:

1. `OpeningUtil.CreateOpening`, in both its overloads, sets `elementForProperties` to the cutting element where the opening's GUID is the one made for that host, insert, range and indices (`GUIDUtil.IsGUIDFor`), sets `registerAsOpening` with it, and calls `ProductWrapper.AddElement` with that element and the opening's handle.
2. `AddElement`, with `register` set, calls `RegisterHandleWithElement`, which puts the handle among the element's (`PropertySetsToCreate`) and records which element the handle belongs to (`HandleToElementCache.Register(handle, element.Id)`).
3. `ExporterUtil.ExportRelatedProperties` runs for the element with its wrapper, and `ExportElementProperties` takes every handle the wrapper holds (`GetAllObjects`), the opening's among them: the common sets that apply to that handle's entity (`GetCurrPSetsToCreate`, here `IfcOpeningElement`, so `Pset_OpeningElementCommon`) are evaluated for the element registered for the handle (`HandleToElementCache.Find`), the cutter, not for the host.
4. Each entry's value is read by `PropertySetEntryMap.CreatePropertyFromElementOrSymbol` from that element's parameter named `<set>.<property>` first and then its plain name (the lookup RE-194 measured). The openings of a cabinet and a curtain wall carry no `FireExit`, as the office defines the parameter on doors, windows and ramps (RE-194) and not on them.

So the door's `FireExit` is the cutter's, and the opening's value is that parameter's.

Every door holds `false`, so the 43 and 45 values cannot tell the door's parameter from a fixed `false`; the rule below is the one the source gives.

## 2. What rvt-rs writes

`attach_wall_cut_openings` (`src/ifc/wall_cut_openings.rs`) hands each cutter's `Pset_OpeningElementCommon` properties to the openings it writes in a second wall as an `IfcEntity::CutterOpeningSet`; the typed production pass sets them aside while it reads the parameters named for a standard property (RE-194), as hand-offs named `OpeningElementCommon.<property>`, where the property's set is `Pset_OpeningElementCommon` and not `Reference`, which stays the cutter's type name. The writer puts them in the opening's set of that name beside its `Reference`, and `pset_property_value_count` counts them.

## 3. Measured

@@MEASURE@@

## 4. Not shown

- **A door whose `FireExit` is `true`.** None of the office's 266 doors holds one, so a `true` value is untested; the value written is the parameter's, as the source gives.
- **The openings rvt-rs does not write.** Revit's export holds 18 openings with `FireExit` as built and 12 at permit that rvt-rs does not write: the 4 and 2 a door cuts in a door, and the doors' cuts in further walls whose box is not read (RE-321's reasons). They are counted among the values on openings rvt-rs does not write.
- **Other projects.** Only the office defines parameters named for a standard property (RE-194), so no other model exercises the rule.

## 5. Reproduce

```bash
# The census that names the values (the property census with Revit's set placement, two office files):
gh workflow run measure.yml --ref main -f ref=main -f base=none -f nordic=true \
    -f models="nordic-office-as-built nordic-office-permit" -f scorers="psets_vs_ifc" \
    -f ifc_flags=--revit-set-placement
# The unfiltered run of section 3 (the code commit against the main it branched from):
@@REPRODUCE@@
```
