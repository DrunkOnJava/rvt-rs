# RE-37 — Lighting fixtures, air terminals, food service, site and circulation categories

**Date:** 2026-09-23
**Result:**
- **Positive.** Nine more `BuiltInCategory` values decode under the RE-21 instance rule, with ElementIds from RE-35's enclosing records. Every element they export is a `Tag` in Revit's own export of the same file, with **zero false positives** on Snowdon Towers Architectural, RE1 Mechanical and RE1 Electrical.
- **Complete recall** against Revit's export for each entity they map to: lighting fixtures 447/447 on Snowdon and 12/12 on RE1 Electrical, air terminals 7/7, food service 26/26, ramps 2/2, elevators 2/2. Planting, parking, entourage and hardscape (157 proxies) are all in Revit's export.
- **Measured, not added:** electrical fixtures, lighting devices, slab edges, and the categories whose IFC4 entity is unmeasured (§3).

**Oracles:** as in RE-35: Snowdon Towers Sample Architectural and Revit's IFC4 export (local only), and RE1 Mechanical and Electrical (`Drshelden/IFC-ECS`, MIT). RE1 Electrical joins CI tier 2 with this change.

-----

## 1. The categories

| category | class | IFC4 entity (Revit's own on Snowdon) | Snowdon | RE1 Mech | RE1 Elec | FP |
|---|---|---|---:|---:|---:|---:|
| `OST_LightingFixtures` −2001120 | LightingFixture | `IfcLightFixture` | 447 | – | 12 | 0 |
| `OST_DuctTerminal` −2008013 | DuctTerminal | `IfcAirTerminal` | – | 7 | – | 0 |
| `OST_FoodServiceEquipment` −2001043 | FoodServiceEquipment | `IfcElectricAppliance` | 26 | – | – | 0 |
| `OST_Planting` −2001360 | Planting | `IfcBuildingElementProxy` | 117 | – | – | 0 |
| `OST_Parking` −2001180 | Parking | `IfcBuildingElementProxy` | 20 | – | – | 0 |
| `OST_Entourage` −2001370 | Entourage | `IfcBuildingElementProxy` | 10 | – | – | 0 |
| `OST_Hardscape` −2001036 | Hardscape | `IfcBuildingElementProxy` | 10 | – | – | 0 |
| `OST_VerticalCirculation` −2001052 | VerticalCirculation | `IfcTransportElement` | 2 | – | – | 0 |
| `OST_Ramps` −2000180 | Ramp | `IfcRamp` | 2 | – | – | 0 |

Every entity choice is the one Revit's IFC4 export makes for that category on Snowdon, each with `.NOTDEFINED.`; rvt-rs writes `$`. Air terminals have no IFC4 reference: RE1's export is IFC2x3, where Revit writes `IfcFlowTerminal`, and `IfcAirTerminal` is its IFC4 subtype for air terminals.

On RE1 Electrical these fixtures are the first elements rvt-rs exports at all. Revit's export has 14 `IfcFlowTerminal`s there: the 12 lighting fixtures and 2 data devices.

## 2. What changes

- Snowdon Towers Architectural: 5,056 → 5,690 exported elements, the 634 new ones all in Revit's export. `IfcBuildingElementProxy` recall there is now 918 of 994.
- RE1 Mechanical: 66 → 73; RE1 Electrical: 0 → 12.
- Core Interior, RE1 Architecture and Plumbing, the Snowdon Structural sample and both `255ribeiro` projects export unchanged.

`tests/element_records_2025.rs` checks the air terminals against RE1 Mechanical and the lighting fixtures against RE1 Electrical. CI tier 2 now fetches RE1 Electrical by pinned commit and hash.

## 3. Measured and not added

| category | in Revit's export | not in it | why not added |
|---|---:|---:|---|
| `OST_ElectricalFixtures` (RE1 Electrical) | 15 | 15 | half the placed instances are elements Revit's export leaves out; cause not established |
| `OST_LightingDevices` (RE1 Electrical) | 12 | 6 | same |
| `OST_EdgeSlab` (Snowdon) | 59 | 9 | all 9 omissions carry non-primary design option 2059967 at `+0x2a` (RE-35 §5); the 9 in the primary option and the 50 in none are all exported. Waits for a primary-option test (#319). |
| `OST_FireAlarmDevices`, `OST_DataDevices`, `OST_ElectricalEquipment`, `OST_MechanicalEquipment` (RE1) | 5, 2, 3, 1 | 0 | only IFC2x3 references (`IfcDistributionControlElement`, `IfcFlowTerminal`, proxy); the IFC4 entity Revit writes is unmeasured |
| `OST_Stairs`, `OST_StairsRuns`, `OST_StairsLandings`, stringers, `OST_Roofs` (Snowdon) | 27, 43, 17, 170, 20 | 0, 0, 0, 1, 0 | Revit exports these as aggregates (a stair of flights, landings and stringers; a roof of slabs); a bounding box for each part would double the volume |

## 4. What this does not claim

- Bodies are the record bounding box.
- The `PredefinedType` is left unset; Revit writes `.NOTDEFINED.` on every one of these on Snowdon.
- Revit 2024 and 2025 only.
