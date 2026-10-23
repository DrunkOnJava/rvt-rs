# RE-149: a family type whose geometry-material map holds only unset values draws with `<Unnamed>`

**Issue:** #355. **Backlog:** B09. **Status:** positive. **Shipped in:** #580 (2026-10-03).

**Written:** 2026-10-23, from the record of #580, because the code cited RE-149 without a report. Nothing was re-measured for this report.

**Evidence:** probe `examples/probe_re149_type_value_blocks.rs`; Measure run 37120595647 (head of #580 against `main`); red run of the real-file target `tests/element_materials.rs` on `main` in draft #579 (0 of 256 Core Interior columns with Revit's material set).

**Files:** Core Interior (`magnetar-2024-core-interior.rvt`, sha256 `c805df44…`) against Revit's `2024_Core_Interior_slim.ifc` (`bfdf36ff…`); RE1 Electrical, Mechanical and Plumbing (`ifc-ecs-re1-*.rvt`) against Revit's IFC2X3 exports of them. Full hashes are in `research/witness-registry.json`.

## 1. The question

A family type's geometry-material map (RE-82, puzzbobb's `FamilySymbol.m_geomTag2MaterialId`) names the material each geometry tag draws with. When every value in the map was unset (`0xff` × 8), rvt-rs read no map, and the type's instances got no material. Revit's export gives those instances `<Unnamed>`: 107 of Core Interior's 256 columns, and 150 of the 152 RE1 fittings, terminals, proxies and control elements it relates a material to.

## 2. The rule

The probe prints, for every family type that placed columns or products use and that has no type material, the maps in its value block. A type that draws with `<Unnamed>` is recognised by its value block's first map: distinct keys no larger than `0xffff` (the geometry tags) and only unset values. `partition_type_materials::unset_material_types` reads that rule.

A column whose element record's reference list joins it to overlapping walls takes those walls' material instead, as Revit gives Core Interior's other 149 columns "Default Wall". `element_record_column_cuts`, which cuts columns by the walls joined to them, already found the same 149 columns.

## 3. Measured

Elements with Revit's exact material set (Measure run 37120595647):

| model | before | after | still without materials |
|---|---|---|---|
| Core Interior | 580 | 836 | 12 slabs (Revit: `<Unnamed>`) |
| RE1 Electrical | 0 | 47 of 47 | 2 proxies |
| RE1 Mechanical | 0 | 43 of 43 | none |
| RE1 Plumbing | 0 | 60 of 60 | none |

No element gained a wrong material set. RE1 Architecture and Einhoven exported byte-identical to `main`.

## 4. Not established

- Doors, windows, walls and slabs keep their earlier paths; whether the rule holds for them is not measured.
- The Revit 2023 path is unchanged and not measured against this rule.
