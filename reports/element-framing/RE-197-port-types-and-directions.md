# RE-197 — A port's type and flow direction

**Date:** 2026-10-19
**Issues:** #828; follows RE-138 (connector joins), RE-169 (a family instance's ports and their connectors' domains), B54 (a duct's or pipe's two ports) and B83 (a proxy's ports).
**Artefacts:** the BIM4LCA office, as built, with Revit's own IFC4 export (Reference View 1.2; `research/bim4lca-measure.tsv`); RE1 Mechanical, Plumbing and Electrical (Revit 2025, MIT) with Revit's own IFC2X3 exports (Coordination View 2.0).
**Probe:** `examples/probe_re197_connector_directions.rs`, in runs [39253573356](https://github.com/DrunkOnJava/rvt-rs/actions/runs/39253573356), [39255994320](https://github.com/DrunkOnJava/rvt-rs/actions/runs/39255994320) and [39256707456](https://github.com/DrunkOnJava/rvt-rs/actions/runs/39256707456).
**Scorer:** `tools/re/ports_vs_ifc.py`, in run [39261416282](https://github.com/DrunkOnJava/rvt-rs/actions/runs/39261416282).
**Status:** positive for a port's `PredefinedType` and for the `FlowDirection` of an electrical connector; negative for an HVAC or piping connector's direction, for `SystemType`, and for a port property set.

## 1. Before

rvt-rs wrote every connector as an `IfcDistributionPort` with `FlowDirection` `.NOTDEFINED.` and no `PredefinedType` or `SystemType`. Revit's IFC4 export of the office gives all three on each of its 67 ports:

| Ports | `FlowDirection` | `PredefinedType` | `SystemType` |
|---:|---|---|---|
| 43 | `.SOURCEANDSINK.` | `.CABLE.` | `.ELECTRICAL.` |
| 24 | `.SOURCEANDSINK.` | `.PIPE.` | `.HEATING.` |

The 43 are its electrical fixtures' family connectors, the 24 the two ends of each of its 12 pipes. Revit's IFC2X3 exports of the RE1 models write `FlowDirection` only, the one of the three an IFC2X3 port has, and name each port `InPort_`, `OutPort_` or `Port_` by it.

## 2. Revit's rule

Read in revit-ifc `master` at e80e3d3 (CLEANROOM.md source 5; read and cited, no code copied), `ConnectorExporter.cs`:

- `PredefinedType` follows the connector's domain (`GetMappedIFCDistributionPortType`): HVAC `DUCT`, electrical `CABLE`, piping `PIPE`, cable tray and conduit `CABLECARRIER`, otherwise `NOTDEFINED`.
- Only HVAC and piping connectors have a direction (`ProcessConnections`): every other connector is taken as bidirectional and written `SOURCEANDSINK`. An electrical connector whose description gives `Flow=Source` or `Flow=Sink` is written so (`SetDistributionPortAttributes`); rvt-rs does not read a connector's description.
- `SystemType` is the connector's system classification (`DuctSystemType`, `ElectricalSystemType`, `PipeSystemType`) mapped to `IfcDistributionSystemEnum`.
- A pipe's or duct's connectors are exported only when the entity it is exported as is an `IfcDistributionElement` (`Exporter.cs`, `ElementFilteringUtil.IsMEPType`, `GenericElementExporter.cs`), as rvt-rs's ports already are.

## 3. The connectors

RE-169 reads each FamilySymbol's connectors from its family's `ConnectorDataCell` objects (`m_index`, `m_domain`). For every port of a family instance in Revit's export, the probe tallies each other field of the connector's cell against the port's direction there (`In` or `Out` from the IFC2X3 name, else its `FlowDirection`): `m_connectorType` (1 on every connector), `m_domain`, `m_profileType`, `m_propId2FamParamIdBindings` (BuiltInParameter ids bound to family parameter ids) and `m_oRelevantParams`, a pointer that the graph's pointer edges resolve to a `FamilyParams` object, whose fields (`m_params`, `m_geomRefHandles`) are tallied too.

| Model | Ports on family instances with a connector cell | In | Out | `SOURCEANDSINK` |
|---|---:|---:|---:|---:|
| RE1 Mechanical | 88 | 40 | 41 | 7 |
| RE1 Plumbing | 136 | 59 | 65 | 12 |
| RE1 Electrical | 53 | 0 | 0 | 53 |
| BIM4LCA office | 43 | | | 43 |

- No field decides an HVAC or piping connector's direction. Domain 1 (HVAC) gives 39 In and 39 Out ports on Mechanical; domain 3 (piping) gives 59 In, 65 Out and 11 `SOURCEANDSINK` on Plumbing. `m_profileType`, the parameter bindings and the `FamilyParams` objects take the same values on In and Out ports. The direction is the family's connector element's own setting, which these objects do not hold: negative.
- Every electrical (domain 2) connector's port is `SOURCEANDSINK`: 53 of 53 on RE1 Electrical, 1 of 1 on Mechanical, 1 of 1 on Plumbing and 43 of 43 on the office, 98 of 98, as revit-ifc's rule has it. No port in these exports is on a domain 0 (undefined) connector, which the same rule makes `SOURCEANDSINK` and `NOTDEFINED`: for those the rule rests on revit-ifc alone.

## 4. What changes

- A family instance's port takes its connector's domain as its `PredefinedType` (`native_connectors::port_type_of_domain`): `DUCT`, `CABLE` or `PIPE`, and `NOTDEFINED` for an undefined domain. A cable tray or conduit connector has no port (RE-169). A duct's or pipe's two ports are `DUCT` or `PIPE`.
- A connector of any domain but HVAC and piping is `SOURCEANDSINK` (`native_connectors::port_flow_direction_of_domain`). An HVAC or piping connector's port, and a duct's or pipe's, keeps `NOTDEFINED`. An electrical connector whose description sets `Flow=Source` or `Flow=Sink` is exported with that direction by Revit; the description is not read (#834), so such a port would be written `SOURCEANDSINK`. None of the 98 electrical ports measured has one.
- The writer writes a port's direction and type only as values of IFC4's `IfcFlowDirectionEnum` and `IfcDistributionPortTypeEnum` (`USERDEFINED` aside, which needs an `ObjectType`), `NOTDEFINED` and `$` otherwise.
- `SystemType` stays unset: the connector's system classification is not read.
- No port property set is written. Revit's IFC4 Reference View export of the office writes none on any of its 67 ports, and revit-ifc certifies `Pset_DistributionPortCommon` but none of the typed port sets (`Pset_DistributionPortTypePipe`, `…Duct`, `…Cable`) for the IFC4 Reference View or Design Transfer View (`IFCCertifiedEntitiesAndPSets.json`).
- `ports_vs_ifc` compares `FlowDirection`, `PredefinedType` and `SystemType` on the ports both exports hold, those the schema of Revit's export gives a port.

## 5. Results

Measure A/B against `main` (cdac7e6), run [39261416282](https://github.com/DrunkOnJava/rvt-rs/actions/runs/39261416282), ports both exports hold, both sides scored with this change's `ports_vs_ifc`:

| | `PredefinedType` equal | `FlowDirection` equal |
|---|---:|---:|
| BIM4LCA office, as built (67 ports) | 0 → 67 | 0 → 43 |
| BIM4LCA office, building permit (67 ports) | 0 → 67 | 0 → 43 |
| RE1 Electrical (53 ports, IFC2X3) | not in the schema | 0 → 53 |
| RE1 Mechanical (150 ports, IFC2X3) | not in the schema | 0 → 1 |
| RE1 Plumbing (260 ports, IFC2X3) | not in the schema | 0 → 1 |

- No port takes a direction Revit's export does not give it: every `SOURCEANDSINK` rvt-rs writes is Revit's, on all five models.
- What stays: the office's 24 `PIPE` ports, its 12 pipes' ends, are `SOURCEANDSINK` in Revit's export and `NOTDEFINED` in rvt-rs's, as a pipe's direction is not read; RE1's HVAC and piping ports (Mechanical 134 `SOURCE` or `SINK` and 15 `SOURCEANDSINK`, Plumbing 238 and 21) likewise. `SystemType` is equal on none of the office's 67 ports (Revit `ELECTRICAL` 43, `HEATING` 24).
- The ports themselves and their connections do not change: Mechanical 73 of 73 connections and 150 of 150 ports, Plumbing 126 of 126 with 2 that Revit's export does not hold, every port tied to its element.
- 29 of the 89 exports change, every one that has ports: the office's two, RE1's three MEP models and 24 of Autodesk's samples. IfcOpenShell finds nothing on any of the 89.

## 6. Reproduce

```bash
gh workflow run measure.yml -f ref=main -f base=none -f nordic=true
gh workflow run measure.yml -f ref=main -f base=none -f probe=probe_re197_connector_directions
```

`ports_vs_ifc.txt` in each MEP model's results gives the three attributes of every port both exports hold against Revit's; `probe.txt` the probe's tally.
