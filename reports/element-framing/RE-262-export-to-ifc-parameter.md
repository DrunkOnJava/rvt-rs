# RE-262 — "Export to IFC" does not explain the single elements rvt-rs writes that Revit's export leaves out

**Date:** 2026-10-29
**Issues:** #792 (the BIM4LCA office permit model's extra column), #879 (BIMcollab's three 2015 air terminals); B38 (RE1 Plumbing's tank)
**Hypothesis checked:** Revit's exporter reads an element's `IFC_EXPORT_ELEMENT` (BuiltInParameter −1019012, "Export to IFC": By Type, Yes or No) and its type's `IFC_EXPORT_ELEMENT_TYPE` (−1019013) before any category mapping, and leaves out what says No (revit-ifc `ElementFilteringUtil`, CLEANROOM.md source 5). rvt-rs reads neither, so an element set to No would be written by rvt-rs and left out by Revit.
**Artefacts:** `RE1-Plumbing.rvt` (sha256 `03689df2…`), `RE1-Architecture.rvt`, `2024_Core_Interior.rvt`, and the BIM4LCA office as built (`747120005b…`) and at building permit (`1c12c205…`), as Measure fetches them
**Run:** Measure [40944230277](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40944230277) at 6a47bdb2 (`examples/probe_re262_export_to_ifc_parameter.rs`)
**Result:** negative. Neither parameter is saved on RE1 Plumbing's tank 442378 or the permit model's column 1927742, nor on any other element of RE1 Plumbing, RE1 Architecture, Core Interior or the permit model. So it cannot be what leaves those two out of Revit's exports. BIMcollab's 2015 air terminals are not testable: the native parameter reader reads Revit 2023 to 2027 only.

## 1. What the probe reads

Every declared element's saved parameters (`native_element_parameters::read`, RE-207: its own values, then its type's), and from them every entry of BuiltInParameter −1019012 or −1019013, with its value, level and source element.

## 2. Counts

| Model | declared elements | with saved parameters | carrying −1019012 | carrying −1019013 |
|---|---:|---:|---:|---:|
| RE1 Plumbing | 4,827 | 4,827 | 0 | 0 |
| RE1 Architecture | 4,668 | 4,668 | 0 | 0 |
| Core Interior | 26,425 | 26,424 | 0 | 0 |
| BIM4LCA office, building permit | 63,679 | 63,657 | 0 | 0 |
| BIM4LCA office, as built | 72,798 | 72,777 | 24 | 0 |

On the office as built, 24 family instances (3483397, 3483532 and onwards) carry −1019012 on themselves, all with value 1; the permit model has none. Which value Revit's dialog shows for 1 is not read here, and nothing in the as-built export depends on it: by ElementId that export holds exactly the elements rvt-rs writes (12,974 of 12,974, Measure run 40927543524).

## 3. What stays open

- 442378 (RE1 Plumbing) and 1927742 (office permit) are left out of Revit's exports for another reason. The remaining candidates are a nested-instance rule (RE-79's pattern; 442378 has two frames but no leading-chain record of its own) and, for 1927742 only, a model state the export saw and the file does not (as RE-253 found for #1048).
- A parameter whose value is the default may not be saved at all; an element left at "By Type" on a type left at its default would carry neither entry, which is what these files show.
