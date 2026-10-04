# RE-169: a family instance's ports are its symbol's connectors

**Issues:** #528, #328. **Backlog:** B54, B38. **Status:** positive for the ports (every family instance on three models); negative for the tank Revit's export leaves out.

**Evidence:** Measure runs 37230166352 (`examples/probe_re169_symbol_connectors.rs`), 37229407104 (`examples/probe_re169_tank_fields.rs`) and 37229769241 (`examples/probe_re169_export_flag.rs`), on the RE1 MEP models (Revit 2025, read by the native record path since #676).

## 1. The question

Revit's export of RE1 Electrical writes 53 ports and no connection: one per connector of each light, alarm, appliance, panel and device. It writes the open connectors of Plumbing's fixtures and fittings too. rvt-rs wrote a port only for a join it read (RE-138, RE-141), so it wrote none of Electrical's. Where does a family instance keep its connectors, joined or not, and which of them does Revit's export write?

## 2. The connectors

Read through the native record path:

- A `FamilySymbol` lists its connectors in `m_arrConnectorData`, each entry an `m_index` and a transform `m_trf` (a 3 × 3 matrix and an origin in the family's frame). The water closet tank's symbol 442362 has indices 1 and 2.
- Its `Family` holds a `ConnectorDataCell` per connector: the same `m_index`, `m_domain` (Revit's `Domain`: 1 HVAC, 2 electrical, 3 piping, 4 cable tray and conduit) and `m_connectorType`. The tank's family 491228 has two, both in domain 3.
- A `FamilyInstance` holds a `FamilyInstanceConnectorManager` and a `Connector` object per connector. Each `Connector` lists in `m_arrRefs` the elements joined to it, each with `m_connType` and `m_nIndex`: the tank's two connectors name pipe 444674 at index 0 and pipe 444124 at index 0, each with type 1, and two further references with type 4.

## 3. The ports

For each element Revit's export writes ports for, the probe compares the port indices with its symbol's connector indices:

| model | entity in Revit's export | elements | same indices | symbol has more |
|---|---|---:|---:|---:|
| RE1 Electrical | `IfcBuildingElementProxy` | 30 | 29 | 1 |
| RE1 Electrical | `IfcDistributionControlElement` | 5 | 5 | 0 |
| RE1 Electrical | `IfcFlowTerminal` | 14 | 14 | 0 |
| RE1 Plumbing | `IfcFlowFitting` | 54 | 54 | 0 |
| RE1 Plumbing | `IfcFlowTerminal` | 6 | 6 | 0 |
| RE1 Mechanical | `IfcFlowFitting` | 35 | 35 | 0 |
| RE1 Mechanical | `IfcFlowTerminal` | 7 | 7 | 0 |
| RE1 Mechanical | `IfcBuildingElementProxy` | 1 | 1 | 0 |

The one element with more is panel 428352: its symbol has connectors 1 to 6, and Revit writes a port for 1 only. Its family's cells put connector 1 in domain 2 (electrical) and 2 to 6 in domain 4 (cable tray and conduit). **Revit's export writes a port for each connector of a family instance's symbol outside the cable tray and conduit domain**, joined or not. Ducts and pipes are not family instances; their connectors 0 and 1 are B54's first part (#673).

#680 writes those ports on every family instance rvt-rs exports as a distribution element (`native_connectors::symbol_port_indices`). Electrical's 30 proxies keep none: IFC4 lets a port be tied only to an `IfcDistributionElement` (RE-138).

## 4. The tank Revit leaves out (B38)

rvt-rs writes RE1 Plumbing's water closet tank 442378; Revit's export does not. The tank's native record against an exported rough-in fixture (sink 442581):

- Both are `FamilyInstance`s created in phase 3 (New Construction), demolished in none (-1), in the main model (design option -1), visible (`m_invisible` false, `m_famElemVisibility` -1), with no super-instance.
- The tank is unhosted and level-based (`m_hostId` -1, `m_assocLevelId` 1819, `m_workPlaneBased` false); the sink is hosted by wall 442615 and placed on a work plane.
- No record in the file holds `DontExport`, `IfcExportAs` or `IFCExport`, so no export setting stored in the model names it.

Why the export leaves it out is not established. It is not its phase, its design option or a stored export flag.

## 5. Not established

- What `m_connType` 4 is. The tank's and the sink's connectors each list one such reference next to their physical joins (type 1), plausibly the piping system.
- Whether the rule holds on the MEP samples. They have no Revit export to compare with.
- Why Revit's export leaves out tank 442378 (B38).

## 6. Reproduce

```text
gh workflow run measure.yml -f ref=<branch> -f base=none -f probe=probe_re169_symbol_connectors
gh workflow run measure.yml -f ref=<branch> -f base=none -f probe=probe_re169_tank_fields
gh workflow run measure.yml -f ref=<branch> -f base=none -f probe=probe_re169_export_flag
```
