# RE-79 — MEP devices and equipment; nested components Revit leaves out

**Date:** 2026-09-27
**Issue:** #96
**Artefacts:** RE1 Electrical, Mechanical, Plumbing and Architecture (Revit 2025, `Drshelden/IFC-ECS` `data/RE1/`, MIT) with Revit's IFC2x3 exports of each; Core Interior (2024) and its slim export; Snowdon Towers Architectural and Structural (2024, local only).

**Result:** positive. Six more categories export, and one rule leaves out the nested components Revit's export leaves out.

## Categories

RE-37 measured electrical fixtures and lighting devices on RE1 Electrical and did not add them, because Revit's export omits some of them. It left fire alarm devices, data devices and electrical and mechanical equipment waiting for an IFC4 reference. The census (`probe_re33_category_census`) finds these instance-like records outside the exported categories on the RE1 models:

| BuiltInCategory | id | RE1 model | instance-like records |
|---|---:|---|---:|
| OST_LightingDevices | -2008087 | Electrical | 36 |
| OST_FireAlarmDevices | -2008085 | Electrical | 14 |
| OST_DataDevices | -2008083 | Electrical | 4 |
| OST_ElectricalFixtures | -2001060 | Electrical | 60 |
| OST_ElectricalEquipment | -2001040 | Electrical | 7 |
| OST_MechanicalEquipment | -2001140 | Mechanical | 2 |

The names are the public `BuiltInCategory` members for those values. Records are framed more than once, so these counts exceed the elements.

With all six exported under the RE-21 instance rule and RE-35's enclosing-record ids, RE1 Electrical gains 58 elements. 37 are Tags of Revit's export: every element it holds that rvt-rs did not export. The other 21 are not in it.

## Nested components

The 21 are `SWITCH:SWITCH` (6), `SYMBOL-Electrical:Receptacle Duplex` (6), `Receptacle Special Purpose` (4), `Receptacle Duplex GFCI` (3) and `Junction Box` (2). They pair one to one with the switches, receptacles and junction boxes Revit does export. Each is the shared nested family of its device, stored as an element record of its own:

- the nested record and its parent are in the same category;
- each lists the other in its first reference list (`+0x88`);
- the nested one has the larger ElementId (on RE1 Electrical, the parent's plus 2).

`examples/probe_re79_nested_mep_components.rs` prints every such pair: 21 on RE1 Electrical, all with the parent exported and the nested one not.

Mutual references are not specific to nesting. Applied to every category, the rule removed elements Revit exports: 1,366 on Snowdon Towers (joined walls, columns), 298 on Core Interior, and connected fittings on the MEP models. It therefore applies only to lighting devices, electrical fixtures and electrical equipment (`NESTED_COMPONENT_CATEGORIES`), with the parent in the child's category.

## Measured

Exported Tags against the Tags of Revit's export (IfcOpeningElement, type objects and spatial structure excluded):

| file | added | in Revit's export | Revit elements not exported | removed |
|---|---:|---:|---:|---:|
| RE1 Electrical | 37 | 37 | 0 | 0 |
| RE1 Mechanical | 1 (the AHU) | 1 | 0 | 0 |
| RE1 Plumbing | 0 | | 0 | 0 |
| RE1 Architecture | 0 | | 0 | 0 |
| Core Interior | 0 | | 0 | 0 |
| Snowdon Towers Architectural | 0 | | 0 | 0 |
| Snowdon Towers Structural | 0 | | | 0 |

RE1 Electrical exports 49 elements, exactly the 49 of Revit's export. `tests/element_records_2025.rs` (`revit_2025_electrical_devices_are_revits_own`) checks each entity's set and that nothing else is exported. All 49 carry Revit's own GlobalId (was 12), and the AHU on RE1 Mechanical does too (74 of 74). 46 of RE1 Electrical's names are Revit's `Family:Type:ElementId` (was 12); `tests/element_names.rs` and `tests/revit_global_ids.rs` pin both.

Revit names electrical equipment `Family:<Panel Name>:ElementId` (`262416-PANEL:RE-1:428352`, `260900-BAS:BAS:872430`, `284621-FirePanel:FirePanel:925565`), from an instance parameter. "RE-1" sits BuiltInParameter-keyed after -1140078, but "BAS" and "FirePanel" do not, so the parameter is not read. Those three are named `ElectricalEquipment-<ElementId>` rather than given a `Family:Type` name Revit does not use; their ObjectType is Revit's.

## IFC entities

Following RE-37, each is the IFC4 subtype of what Revit's IFC2x3 export writes for the same elements:

| category | Revit's export | rvt-rs |
|---|---|---|
| Fire alarm devices | `IfcDistributionControlElement`, typed `IfcAlarmType` | `IfcAlarm` |
| Data devices (the thermostats) | `IfcFlowTerminal`, typed `IfcElectricApplianceType` | `IfcElectricAppliance` |
| Lighting devices, electrical fixtures, electrical and mechanical equipment | `IfcBuildingElementProxy` | `IfcBuildingElementProxy` |

The existing mappings for electrical fixtures (`IfcLightFixture`), electrical equipment (`IfcElectricAppliance`) and mechanical equipment (`IfcFlowController`) were never reached by a record. They now follow Revit's export, because each category mixes kinds it cannot tell apart: switches and sensors, receptacles and junction boxes, panels and controllers.

## Not claimed

- Other MEP device categories (communication, security, nurse call, sprinklers, conduits, cable trays, accessories): no reference model holds them.
- Bodies are the record's bounding box. With `--saved-meshes` (RE-78) they can be drawn from their saved graphics on 2024 files.
- The nested rule is measured on one model. A nested component with a 3D body that Revit's export does write would be lost; none occurs in these files.
