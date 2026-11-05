# RE-352: Furniture Systems are exported from their element records as `IfcSystemFurnitureElement`

**Date:** 2026-11-05
**Issues:** #1444 (reinforcing bars, element assemblies and Furniture Systems that RvtIO counts and rvt-rs does not read); Discussion #1241; follows RE-33 (the category census), RE-319 (a category added by the same path) and RE-326 section 5 (the public export read here)
**Credit:** mahach666, author of RvtIO (a .NET reader of Revit files, MIT), whose counts showed that rvt-rs read no Furniture Systems (3 on `rac_basic`). Only the count is taken from RvtIO, never a name or a table.
**Artefacts:** Autodesk's `rac_basic` sample project of Revit 2015 to 2027 (`research/autodesk-sample-projects.tsv`; fetched for a Measure run and never stored); the public IFC4 export of its 2018 save that RE-326 section 5 describes (a forum attachment, sha256 `1114ce918f64885bd87ec3bc670c4c543a90898d1bf7e701fb8fbf95e5b5ea35`, 9,129,271 bytes, no licence: read locally for counts only, nothing quoted or committed from it).
**Status:** the category is added and written as `IfcSystemFurnitureElement` with no `PredefinedType`. It rests on **one project and 3 elements**: the one file with a Revit export that holds any placed Furniture System. The output of this head is not yet measured (section 4).

## 1. What was missing

The element-record sweep reads 42 product categories (`PRODUCT_RECORD_CATEGORIES`); Furniture Systems (`OST_FurnitureSystems`, -2001100, "Furniture Systems" in `BUILTIN_CATEGORY_NAMES`) were not among them. `category_map.rs` and `docs/compatibility.md` mapped a `FurnitureSystem` class to `IfcFurniture`, but nothing produced it.

## 2. The evidence

**The files hold them.** Measure run 42097940515 (head `32586847` of #1467, the category census `probe_re350_category_stages` under its earlier name `probe_re348_category_stages`; 97 models and samples, 59 with a Revit export) finds Furniture Systems frames in nine files: Autodesk's `rac_basic` of 2020 to 2027 (6 frames on each: 3 placed instances and 3 type symbols) and one held-out file (2 type symbols and no placed instance). Runs 42108271073 and 42109994761 (the same probe, the 2018 to 2023 samples) find the same 6 frames, with the same 3 placed ids, in the 2018 and 2019 saves of `rac_basic`. 3 placed frames equal RvtIO's 3 Furniture Systems on `rac_basic`.

**Revit's own export writes them as `IfcSystemFurnitureElement`.** The public IFC4 export of the 2018 `rac_basic` (RE-326 section 5) holds 3 `IfcSystemFurnitureElement`, each with `PredefinedType` unset, and 3 `IfcSystemFurnitureElementType` objects. Read beside run 42108271073's output on Autodesk's 2018 sample, all 3 placed Furniture Systems frames the sweep selects are in that export (3 of 3, by ElementId) and the 3 type symbols are its 3 type objects (3 of 3). No Furniture Systems frame the rule selects is missing from the export, none is written that it lacks (0 false positives), and the sweep's written ids cover all 438 elements of the export (RE-326's 438). These are counts of ids the sweep reads from the `.rvt`: the export has no licence and no tag list or name from it is quoted.

**One project, 3 elements.** No other model in Measure places a Furniture System, so there is no held-out check of the rule: the one held-out file that holds any has 2 type symbols and no placed instance, so the rule selects none there. The evidence is 3 elements of one project, in one export made by a forum user, and says so wherever the category is claimed.

## 3. The change

- `partition_element_records.rs`: `OST_FURNITURE_SYSTEMS` (-2001100, Autodesk's published `BuiltInCategory` value) in `PRODUCT_RECORD_CATEGORIES` (43) and `RECOVERED_CATEGORIES` (50), under the unchanged RE-21 instance rule, on every release the exporter reads (the category is in the list that both the 2008 to 2023 and the 2024 to 2027 readers take).
- `category_map.rs`: `FurnitureSystem` as `IFCSYSTEMFURNITUREELEMENT` with `PredefinedType` unset, replacing `IFCFURNITURE`. The writer's default attribute tail (`Tag`, `PredefinedType`) is the entity's IFC4 shape, nine attributes, as in the export.
- `export_content.rs`: no `Constraints` Level parameter for the class (its name is not measured, as for RE-319's classes).
- `rvt_ifc.rs`, `viewer/src/main.ts`: the entity's display name.

Not done, and not claimed: the occurrences are not typed (Revit's export writes an `IfcSystemFurnitureElementType` for each type; rvt-rs types only the entities `type_entity_for` lists); they are not placed in their rooms' spaces (no measurement of where Revit's export contains them); the body is the record's box, as for every category added by this rule.

## 4. Measured on this head

Not yet measured. The first filtered run is Measure with `-f base=main -f samples=true -f sample_years="2015 2027" -f scorers=none` on the head named in the pull request; its run id, which exports moved against `main`, the `same-project gates` verdict and the counts of `IfcSystemFurnitureElement` it writes are added here when it is read. A run of every model, including the held-out file above, follows when the maintainer asks.

## 5. What this does not claim

- That Furniture Systems are right beyond 3 elements of one project, or that Revit writes `IfcSystemFurnitureElement` for system furniture of every family: the class is that of the 3 in one export.
- Anything about rebar and Structural Beam Systems, which have no Revit export that shows them (#1467).
- That the elements are typed, contained in spaces or have Revit's property sets: none is written.
