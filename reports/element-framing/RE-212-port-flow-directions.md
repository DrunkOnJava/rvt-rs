# RE-212: A port's flow direction, from its connector's own parameters and its calculation

**Date:** 2026-10-23
**Issues:** #834 (a port's direction, description and system classification); follows RE-197, which found no field for an HVAC or piping connector's direction, and RE-169 (a family instance's ports are its symbol's connectors).
**Artefacts:** RE1 Plumbing, Mechanical and Electrical (Revit 2025, MIT, `Drshelden/IFC-ECS` at `9e8b3717`: `RE1-Plumbing.rvt` sha256 `03689df2…`, `RE1-Mechanical.rvt` `d5597201…`, `RE1-Electrical.rvt` `9875e765…`) with Revit's own IFC2X3 exports of them (`294490b2…`, `9e56cf72…`, `c7002a10…`).
**Probe:** `examples/probe_re212_connector_data.rs`, in Measure run [40004442760](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40004442760) (`re/834-connector-data` at `6c006183`).
**Scorer:** `tools/re/port_directions_vs_ifc.py`, on that run's probe output and Revit's exports.
**Status:** positive. Every one of the three exports' 463 ports gets Revit's direction.

## 1. What Revit writes

Revit's exporter names a port by its direction: `InPort_<ElementId>_<index>` with `FlowDirection` `SINK`, `OutPort_…` with `SOURCE`, and `Port_…` with `SOURCEANDSINK`. On the RE1 exports, names and directions follow that without exception. revit-ifc's `ConnectorExporter` (CLEANROOM.md source 5) takes the direction from the API's `Connector.Direction`, for HVAC and piping connectors only; every other domain is bidirectional. Every port's `Description` is `Flow`.

## 2. Where it is in the file

**A family's connectors.** A `ConnectorDataCell` (RE-169) names its own parameters with `m_oRelevantParams`, an object of class `FamilyParams` that the `Family` record holds after all its cells. The n-th cell, ordered by that reference's offset, owns the n-th `FamilyParams` after the first cell. Its `m_params` hold the connector's built-in parameters by id. The Flow Direction is one of them, as an `m_int` (1 In, 2 Out, 0 bidirectional):

- `-1140248` on a piping connector;
- `-1140219` on an HVAC one.

On the air handler 427568 of RE1 Mechanical, piping connectors 6 and 7 hold 2 and connector 11 holds 1. Its HVAC connectors 9 and 10 hold 1 and 2. RE-197 compared the family's own `FamilyParams`, the one before the cells, which is why it found the same values on In and Out ports.

**An instance's connectors.** A `FamilyInstance` record's `Connector` objects each have a calculation modifier, `MEPFamilyConnectorCalculationPiping`, `…Hvac` or `…Electrical`, whose `m_pConnector` holds the connector object's token. Its `m_eDirection` uses the same numbering. It is the direction Revit calculated in the system: where a family leaves a connector bidirectional, the system gives it one. The same object holds `m_eSystemType` and `m_idSystem`.

**A duct's or pipe's connectors.** An `RbsPipeCurve` or `RbsDuctCurve` record has two `Connector` objects, indices 0 and 1. Each has a `SegmentConnectorCalculation` with an `m_eDirection`, joined to it by token in the same way.

## 3. The rule

1. A family instance's HVAC or piping connector takes its family's Flow Direction where the family sets one (1 or 2), and its calculation's where the family leaves it bidirectional (0).
2. A duct's or pipe's connector takes its calculation's.
3. A connector still bidirectional, joined to a connector with a direction, takes the opposite one. revit-ifc writes a join as one connection, an `InPort` on the element the flow enters and an `OutPort` on the one it leaves, decided by the connector with a direction.
4. Any other domain is `SOURCEANDSINK`.

| model | family instances | ducts and pipes | by step 3 | of Revit's ports |
|---|---:|---:|---:|---:|
| RE1 Plumbing | 136 of 136 | 124 of 124 | 0 | 260 of 260 |
| RE1 Mechanical | 88 of 88 | 59 of 59 | 3 of 3 | 150 of 150 |
| RE1 Electrical | 53 of 53 | none | 0 | 53 of 53 |

Two cases decide the order of steps 1 and 3:

- **Step 1 before the calculation.** The air handler's piping connector 11 is In in its family and calculated Out. Revit's export makes it In.
- **Step 3.** The three pipe ends joined to the air handler's connectors 6, 7 and 11 are bidirectional in their calculation. Revit's export makes them In, In and Out: the opposite of their partners.

## 4. What changes

- `native_connectors::connector_ports` reads, in one pass over the partitions: the family instances, their symbols, every family, and the ducts and pipes rvt-rs writes ports for. It gives each family instance's ports a direction by steps 1 and 4, and each duct's and pipe's connectors their calculated direction.
- The exporter then applies step 3 over the joins it writes.
- The writer names each port by its direction, `InPort_`, `OutPort_` or `Port_`, and writes `Flow` as its description, as Revit's export does. A port whose direction is not read keeps `NOTDEFINED` and `Port_`. The writer already turned a sink's axis into its pipe (#847), so the ports placed at a pipe's ends now point as Revit's do.
- `tests/family_instance_ports.rs` checks every port both exports hold for Revit's prefix and `FlowDirection`, on RE1 Electrical, Plumbing and Mechanical.

## 5. What it does not cover

- `SystemType` stays unset. The calculation's `m_eSystemType` is the connector's system classification, but RE1's exports are IFC2X3, which has no `SystemType`, so its mapping to IFC4's enumeration is not measured here.
- The electrical description (`Flow=Source`) RE-197 notes is still not read.
- One release and one project set: Revit 2025, three models, each a single system design. The family and segment layouts are the native records' and are read by member name, so the rule reaches every release the native record path reads (2023 to 2027). It is measured on 2025 only.

## 6. Reproduce

```bash
gh workflow run measure.yml -f ref=main -f base=none \
  -f probe=probe_re212_connector_data -f probe_scorer=port_directions_vs_ifc
gh run download <run id>   # measure-head/re1-*/probe_score.txt
```

`ports_vs_ifc` in the same run compares rvt-rs's own ports' `FlowDirection` with Revit's, and `port_places_vs_ifc` their axes.
