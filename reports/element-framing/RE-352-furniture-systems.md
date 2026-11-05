# RE-352: Furniture Systems are exported from their element records as `IfcSystemFurnitureElement`

**Date:** 2026-11-05
**Issues:** #1444 (reinforcing bars, element assemblies and Furniture Systems that RvtIO counts and rvt-rs does not read); Discussion #1241; follows RE-33 (the category census), RE-319 (a category added by the same path), RE-326 section 5 (the public export read here) and RE-350 (which measured it)
**Credit:** mahach666, author of RvtIO (a .NET reader of Revit files, MIT), whose counts showed that rvt-rs read no Furniture Systems (3 on `rac_basic`). Only the count is taken from RvtIO, never a name or a table.
**Artefacts:** Autodesk's `rac_basic` sample project of Revit 2015 to 2027 (`research/autodesk-sample-projects.tsv`; fetched for a Measure run and never stored); the public IFC4 export of its 2018 save that RE-326 section 5 describes (a forum attachment, sha256 `1114ce918f64885bd87ec3bc670c4c543a90898d1bf7e701fb8fbf95e5b5ea35`, 9,129,271 bytes, no licence: read locally for counts only, nothing quoted or committed from it).
**Status:** the category is added and written as `IfcSystemFurnitureElement` with no `PredefinedType`. It rests on **one project and 3 elements**: the one file with a Revit export that holds any placed Furniture System. On this head the 13 `rac_basic` saves of 2015 to 2027 move and no other sample does (section 4); the models, the held-out files and the same-project check are not yet measured.

## 1. What was missing

The element-record sweep reads 42 product categories (`PRODUCT_RECORD_CATEGORIES`); Furniture Systems (`OST_FurnitureSystems`, -2001100, "Furniture Systems" in `BUILTIN_CATEGORY_NAMES`) were not among them. `category_map.rs` and `docs/compatibility.md` mapped a `FurnitureSystem` class to `IfcFurniture`, but nothing produced it.

## 2. The evidence

**The files hold them.** Measure run 42097940515 (RE-350's census, head `32586847`, `probe_re350_category_stages` under its earlier name `probe_re348_category_stages`; 97 models and samples, 59 with a Revit export) finds Furniture Systems frames in nine files: Autodesk's `rac_basic` of 2020 to 2027 (6 frames on each: 3 placed instances and 3 type symbols) and one held-out file (2 type symbols and no placed instance). Runs 42108271073 and 42109994761 (the same probe, the 2018 to 2023 samples) find the same 6 frames, with the same 3 placed ids, in the 2018 and 2019 saves of `rac_basic`. 3 placed frames equal RvtIO's 3 Furniture Systems on `rac_basic`.

**Revit's own export writes them as `IfcSystemFurnitureElement`.** The public IFC4 export of the 2018 `rac_basic` (RE-326 section 5) holds 3 `IfcSystemFurnitureElement`, each with `PredefinedType` unset, and 3 `IfcSystemFurnitureElementType` objects. Read beside run 42108271073's output on Autodesk's 2018 sample, all 3 placed Furniture Systems frames the sweep selects are in that export (3 of 3, by ElementId) and the 3 type symbols are its 3 type objects (3 of 3). No Furniture Systems frame the rule selects is missing from the export, none is written that it lacks (0 false positives), and the sweep's written ids cover all 438 elements of the export (RE-326's 438). These are counts of ids the sweep reads from the `.rvt`: the export has no licence and no tag list or name from it is quoted.

**One project, 3 elements.** No other model in Measure places a Furniture System, so there is no held-out check of the rule: the one held-out file that holds any has 2 type symbols and no placed instance, so the rule selects none there. The evidence is 3 elements of one project, in one export made by a forum user, and says so wherever the category is claimed.

## 3. The change

- `partition_element_records.rs`: `OST_FURNITURE_SYSTEMS` (-2001100, Autodesk's published `BuiltInCategory` value) in `PRODUCT_RECORD_CATEGORIES` (43) and `RECOVERED_CATEGORIES` (50), under the unchanged RE-21 instance rule, on every release the exporter reads (the category is in the list that both the 2008 to 2023 and the 2024 to 2027 readers take).
- `category_map.rs`: `FurnitureSystem` as `IFCSYSTEMFURNITUREELEMENT` with `PredefinedType` unset, replacing `IFCFURNITURE`. The writer's default attribute tail (`Tag`, `PredefinedType`) is the entity's IFC4 shape, nine attributes, as in the export.
- `export_content.rs`: no `Constraints` Level parameter for the class (its name is not measured, as for RE-319's classes).
- `rvt_ifc.rs`, `viewer/src/main.ts`: the entity's display name.

Not done, and not claimed: the occurrences are not typed (Revit's export writes an `IfcSystemFurnitureElementType` for each type; rvt-rs types only the entities `type_entity_for` lists); they are not placed in their rooms' spaces (no measurement of where Revit's export contains them); the body is the record's box, as for every category added by this rule.

## 4. Measured on this head

Measure run 42116433579, head `8bf4e3f2` (`-f base=main -f samples=true -f sample_years="2015 2027" -f scorers=none -f models="-"`; main's side `fa93a53d`): Autodesk's 58 sample projects of Revit 2015 to 2027, each exported on both sides, nothing else.

- **13 of 58 exports move, 45 are byte for byte main's** (`ifc.sha256` per sample). The 13 are every `rac_basic` save, 2015 to 2027, and each moves by exactly 3 `IfcSystemFurnitureElement`: `exported.by_ifc_type` has 0 on main and 3 on the head, `building_elements` rises by 3 (470 to 473 on 2015, 464 to 467 on 2016 to 2027), and no other class's count changes. No other sample project (`rac_advanced`, `rst_basic`, `rme_*`, `rst_advanced`, the five Snowdon Towers 2024 models) writes one.
- **Models that exercise the rule:** 13 samples of one project, 3 elements each. In Measure's 59 models with a Revit export none places a Furniture System (RE-350: one held-out file holds 2 type symbols and no placed instance), so none of them is run here and none can move.
- **False positives and negatives against Revit's export:** the only export of any of these files is the public IFC4 export of the 2018 save, counts only (section 2). At the level of the sweep's frames (run 42108271073): 3 placed frames selected, 3 in the export, 0 false positives; the export holds 3, all 3 selected, 0 false negatives. The head writes 3 `IfcSystemFurnitureElement` for that save (above), as many as the export holds; the written ElementIds were not read from the IFC (it was not kept), so that the 3 written are the 3 frames is not shown. The other 12 saves have no Revit export. Their frames are the same 3 placed ids and 3 symbols in 2018 to 2023 (runs 42108271073, 42109994761) and the same counts on 2024 to 2027 (RE-350); 2015 to 2017 were read only by the count above.

Not measured: every model, with the held-out file; the written IFC's ElementIds against the export; and the same-project check. This run's `same-project gates` check run is `neutral`, "NOT MEASURED: the scorers asked for exclude bodies_across_saves" (I set `scorers=none`), so it is not a pass: the check needs a run of the samples with `bodies_across_saves` among its scorers, on the final head. An unfiltered run follows when the maintainer asks.

## 5. What this does not claim

- That Furniture Systems are right beyond 3 elements of one project, or that Revit writes `IfcSystemFurnitureElement` for system furniture of every family: the class is that of the 3 in one export.
- Anything about rebar and Structural Beam Systems, which have no Revit export that shows them (RE-350).
- That the elements are typed, contained in spaces or have Revit's property sets: none is written.
