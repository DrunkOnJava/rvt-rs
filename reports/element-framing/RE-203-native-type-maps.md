# RE-203: a family type's materials from its native map; a material's name from its own record

**Date:** 2026-10-21
**Issue:** #355; follows RE-82, RE-149, RE-58
**Artefacts:** Snowdon Towers Sample Architectural (Revit 2025 copy, sha256 `8ac90a85…d334`) against Revit's IFC4 export (`bldrs-ai/test-models`, sha256 `ecfcb04e…985a`), measured locally only: it cannot be fetched in CI. Checked for regressions on `2024_Core_Interior.rvt` against `2024_Core_Interior_slim.ifc`, the four RE1 models and the 22 R2025 models (`R2025_Models.zip`, #356).
**Probe:** `examples/probe_re203_type_maps.rs`.
**Status:** positive. Exported. Completed by RE-204.

## 1. The question

RE-82 reads a type's geometry-material map by scanning its value block. puzzbobb's comment on #355 (2026-10-02) found the map is `FamilySymbol.m_geomTag2MaterialId` and that, read by name, it is right on all 61 counter tops, where the scan gives 21 of them their nested sink's appliance materials. Does reading it by name hold across the models, and what of its −1 values?

## 2. Method

`native_type_materials::geometry_materials` reads each column, door, window and product type's map, its named materials in order and whether any value is −1. Where the native set differs from the scan's it replaces it; where the map has −1 values the instance takes its own material parameters' materials (RE-200) or `<Unnamed>`. Material names were compared between the scan (RE-58) and each `MaterialElem`'s `Material.m_name`. Scored with `tools/re/element_materials_vs_ifc.py` (IfcOpenShell 0.9.0.post1), element by element against the build before the change.

## 3. Result

- Counter tops: 21 wrong → "Counter Top". Lobby Chandelier and the Residential Lobby Stair Landing Support wrong → Revit's, once the names were fixed.
- Types the scan did not read now have materials: on Snowdon 31 Sink Vanity-Round, 21 Refrigerator, 21 Dishwasher, 21 Sink Kitchen-Island, 5 Shelf Unit, 5 Basic Wall proxies and single appliances none → Revit's; RE1 Architecture's microwave and RE1 Electrical's BAS panel none → Revit's.
- 23 elements took `<Unnamed>` Revit does not write (21 Washer_Dryer_Combo, 1 TableWithUmbrella, RE1 Electrical's FirePanel): their −1 geometry is hidden. RE-204 resolves them.
- Names: the scan and `Material.m_name` agree on 514 of 515 materials across Snowdon, Core Interior, RE1 ×4 and R2025 model 12. The one difference is Snowdon's 1557297, scanned as '1/4" Head', named "Bronze, Architectural".

| | before | after |
|---|---:|---:|
| Snowdon Towers, sets equal to Revit's | 4,961 | 5,092 |
| RE1 Architecture | 70 | 71 |
| RE1 Electrical | 47 | 48 |

## 4. What this changes

- `native_type_materials`; `partition_schema_mvp::refine_type_materials` and `attach_partly_unset_materials`.
- `partition_materials::scan_material_names` prefers the material's own `Material.m_name`.
