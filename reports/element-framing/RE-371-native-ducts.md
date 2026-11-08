# RE-371: a duct's type, section, line and ports from its own native record, a port's SystemType, and the ports of 2015 to 2022

**Date:** 2026-11-08
**Issues:** #1190 (geometry and properties on 2015 to 2023, 2026 and 2027), #96 (ducts, sloped runs, fittings), #834 (a port's `SystemType`), #869 (MEP ports outside Revit 2024 and 2025); follows RE-134 (a duct's size, type and length from its box), B59 and B43 (a duct's system family and shape), RE-138 and RE-171 (joins), RE-172 (a Level parameter), RE-212 (a port's flow direction), RE-370 (a pipe from its own record).
**Artefacts:** Autodesk's own sample projects, pinned in `research/autodesk-sample-projects.tsv` and fetched by `tools/fetch-autodesk-samples.sh`, never stored; MIT RE1 Mechanical and Plumbing (`magnetar-io/revit-test-datasets`); Snowdon Towers HVAC (Revit 2025) with Revit 2025's own IFC4X3 export of it, and the BIM4LCA office with Revit's IFC4 export (both local only).
**Scorers:** `probes/examples/probe_re371_ducts.rs` (each duct as the export draws it, by ElementId, against the same project's 2024 save; with `--native`, each duct's record and every connector's calculation alone), `tools/re/ducts_vs_ifc.py` (the record against Revit's export) and `tools/re/ports_vs_ifc.py` (the export's ports against Revit's).
**Status:** positive for ducts, a port's `SystemType` and an Exhaust Air port's flow; partial for the rest of the flow directions and for the ports of 2015 to 2022. Ducts are held to Revit's own export on 2025 (Snowdon Towers HVAC, 1,053 ducts; RE1 Mechanical, 25), and to the same Autodesk samples saved by 2024 on 2015, 2019 and 2025 to 2027. **No duct of a file of 2015 to 2023, 2026 or 2027 is scored against a Revit export**, and flex ducts are not read.

## 1. Before

- **Ducts, 2015 to 2023, 2026 and 2027:** an `IfcDuctSegment` named `Duct-<ElementId>`, untyped, its record box as its body, no `Length`, and two unplaced ports: 724 ducts on the `rme_basic` sample, 728 on `rme_advanced`. RE-134's size, type and length and RE-131's end keys are read on 2024 and 2025 only.
- **Ducts, 2024 and 2025:** RE-134 types a duct by the name found near its type id in the partition's bytes and finds no type for some (none of Snowdon Towers HVAC's 1,053 on 2025, none of Snowdon Towers Plumbing's 181 on 2024); a duct is its box, and its length and ends are read only where it runs along one of the model's axes (695 of Snowdon Towers HVAC's 1,053 with a `Length`, 356 sloped or off the axes with none).
- **A duct's taps:** a connector past a duct's two ends, where another duct is tapped into it, had no port and its join was not written (406 on Snowdon Towers HVAC, 468 ports missing in all with the flex ducts').
- **`Constraints` on a duct:** `Reference Level` on 2024 only (RE-172), no elevation, slope or justification.
- **A port's `SystemType`:** never written (#834); Revit's export writes one on every IFC4 port.
- **Flow:** 1,253 of the 4,749 ports of Snowdon Towers HVAC had another `FlowDirection` than Revit's, all 1,004 Exhaust Air ports among them.
- **2015 to 2022:** a family instance's ports were not read: the connector pass and RE-171's joins read 2023 and later, and read a record's ids in their 2024 form only. 2,430 ports and no connection on 2019's `rme_basic`, against 7,370 ports and 2,948 connections on its 2024 save.

## 2. What a duct's record holds

A duct's `RbsDuctCurve` record, read through its native graph on every release from 2015 to 2027 (RE-280), has the shape of a pipe's (RE-370):

| object | field | read as |
|---|---|---|
| `RbsDuctCurve` | `m_idType` | its type, an `AbsDuctType` whose `SymbolInfo.m_name` is the type's name, and whose sweep profile names its shape (B59) |
| `RbsDuctCurve` | `m_dWidthOrDiameter`, `m_dHeight` | its section, feet (a round section's diameter twice) |
| `RbsDuctCurve` | `m_vNormal` | the direction the section's height lies along; the width lies across it and the line |
| `RbsDuctCurve` | `m_assocLevelId` (before 2024 `m_idLevelStart`, `m_assocLevelId` being -1), `m_dOffsetStart`, `m_dOffsetEnd`, `m_eHorOffset`, `m_eVertOffset` | its reference level, the elevations of its centreline's two ends above it, and its justification; on the twelve MEP saves every duct's level and offsets are the 2024 save's |
| `GLine` | `m_origin`, `m_dirVec`, `m_endParams` | its line, as a pipe's, at any slope |
| `SegmentConnectorPosition` | `m_pConnector`, `m_paramOnCurve` | which `Connector` sits at the line's start (0) and end (1), and each tap's place along it (between) |
| `Connector` | `m_arrRefs` | each connector's joins, type 1 (RE-171) |
| `SegmentConnectorCalculation` | `m_eDirection`, `m_eSystemType`, `m_idSystem` | each connector's calculated direction (RE-212), its system's classification, and the system |

`m_eSystemType` numbers a classification as a system type's own `m_basicSystemType` does (Snowdon's `Supply Air` system type, 712041, holds 1). A family instance's `MEPFamilyConnectorCalculationHvac` and `…Piping` hold the same three fields; `…Electrical` holds none of them.

`attach_native_ducts` reads them in one projected pass for every duct not yet drawn along its line, on the releases the native record path reads (`NATIVE_DUCT_REVIT_VERSIONS`, 2015 to 2027): on 2015 to 2023 from the 2015 to 2023 branch of `recover_partition_schema_mvp`, on 2024 to 2027 before RE-134's scan, so the record's type, size and length win. It writes the fields RE-134 writes (`CURVE_SIZE_FIELDS`, `DUCT_ENDS_FIELDS`, `DUCT_END_KEYS_FIELDS`, `DUCT_LENGTH_FIELD`, `TYPE_ID_FIELD`, `TYPE_NAME_FIELD`), so the system family and `Pset_DuctSegmentTypeCommon` (B59, B43), the quantities and the ports come from the same export code, and its own: the normal (`DUCT_NORMAL_FIELDS`), the taps (`DUCT_TAPS_FIELD`), each connector's join, and, where the duct is justified at its centre (both offsets 0, `Center` and `Middle`, every duct read here), its reference level's name and the two offsets (`DUCT_OFFSET_FIELDS`). The line is held to a box test like RE-131's: the section swept along it, a round type's as its circle and a rectangular one's as its rectangle, must be the record box within 0.002 ft; an oval duct keeps its box. The normal is squared to the line where it is within 1e-3 of square already (two ducts of `rme_basic` fall 0.00013 ft over 13.8 ft and keep a normal straight up). A value is written only where every copy of the record agrees. The body (`BodySource` `native_duct_line`) is the section extruded along the line.

The connector pass (`native_connectors::connector_ports`) now also reads each connector's `m_eSystemType`, for the duct's, pipe's and family instance's ports alike, and it and `family_instance_joins` read 2015 to 2027 and ids in either form (4 bytes before 2024, 8 from it). A pipe's own record gives its joins too.

## 3. Against Revit's own export

`probe_re371_ducts --native` prints each duct's record alone and every connector's calculation; `tools/re/ducts_vs_ifc.py` holds them against Revit's export by ElementId:

| | Snowdon Towers HVAC (2025, IFC4X3, local only) | RE1 Mechanical (2025, IFC2X3) |
|---|---:|---:|
| ducts read | 1,053 (of Revit's 1,084: the other 31 are flex ducts) | 25 |
| type name | 1,053 | 25 |
| `Length` | 1,053 | 25 |
| section (`Diameter`, or `Width` and `Height`) | 1,053 | not written |
| body: the record's swept section's world box against Revit's, within 0.01 ft | 1,053 (1,045 round, 8 rectangular) | 25 (rectangular) |
| `Constraints`: Start, End and Middle Elevation; Lower and Upper End Centerline, Bottom and Top Elevation; justification | 1,053 each | not written |
| `Constraints` `Slope` | 1,012 of 1,012; none on the 41 vertical ducts, as Revit | not written |

The rules, each held on every Snowdon duct:

- **Section:** the height lies along `m_vNormal`; on the 8 rectangular ducts, horizontal and vertical, Revit's profile has its height there.
- **`Middle Elevation`** is the start's offset, not the mean of the two ends', on all 1,053, the 215 sloped ones included.
- **Bottom and Top:** a lower or upper end's are its centreline less and plus half the section's height times the cosine of the slope, so a vertical duct's are its centreline (41 of 41).
- **`Slope`** is the rise over the run along the line. Revit writes it as an `IfcPositiveRatioMeasure`, 0 on a level duct, which that type does not allow; rvt-rs writes it as an `IfcRatioMeasure`.

**A port's `SystemType`** is its connector's `m_eSystemType` named as revit-ifc's `ConnectorExporter` names a classification (CLEANROOM.md source 5; read, not copied): on Snowdon Towers HVAC, 1 (Supply Air) and 2 (Return Air) are `VENTILATION` and 3 (Exhaust Air) `EXHAUST`, on every one of Revit's 5,268 port entities whose connector they are; on the BIM4LCA office, 7 (Supply Hydronic) is `HEATING` on 24 of 24. Only these four are named. RE1 Plumbing's systems are 16 (Sanitary), 19 (Domestic Hot Water), 20 (Domestic Cold Water) and 22 (a system type named `Other`), by its systems' types, but its Revit export is IFC2X3, which writes no `SystemType`, so they are not. An electrical connector's calculation holds no classification: Snowdon's 37 and the office's 43 electrical ports, `ELECTRICAL` in Revit's export, are written without one.

**Flow (C.5).** revit-ifc writes a join from whichever of its two elements it exports first: that element's connector's own direction (`Connector.Direction`) makes it `SOURCE` and the other `SINK`, or the reverse, or both `SOURCEANDSINK` where it is bidirectional. Which element that is cannot be told from the file: on Snowdon, 67 of 598 joins of a duct whose connector flows out and a fitting whose family's connector is bidirectional are written `SOURCEANDSINK` and the other 531 `SOURCE` and `SINK`, with the same directions, classifications and flows in both records. One rule does hold everywhere it applies: **every Exhaust Air connector is `SOURCEANDSINK`**, 1,004 of 1,004 ports on Snowdon, the ducts', fittings', terminals' and equipment's, whatever its calculated direction, and rvt-rs now writes it so (`port_flow_direction`). The rest of the directions are as RE-212 gives them.

## 4. What the export writes, against Revit's

The export of Snowdon Towers HVAC against Revit's, by `tools/re/ports_vs_ifc.py` and port name:

| | before (RE-370) | after |
|---|---:|---:|
| ports | 4,749 | 5,155 (Revit 5,217 connectors; the other 62 are the flex ducts') |
| connections | 2,122 | 2,528 of Revit's 2,590, none Revit's leaves out |
| `FlowDirection` as Revit's | 3,496 | 4,889 |
| `SystemType` as Revit's | 0 | 5,118 (the other 37 electrical) |
| ports at Revit's point, within 0.01 ft | 4,732 (17 unplaced) | 5,155 |
| … with Revit's axis | 4,100 | 5,008 |

The 406 taps' ports are on the duct's line at their parameter with its direction as their axis, as Revit's are, all 406. A tap is on the duct's axis where the element tapped into it does not put its own connector, so a tap places no other port, and a family instance's port joined to one keeps its own place. A port joined to a placed one is at its point (#869); its axis is the other's where exactly one of the two is a sink, as before, and the other's reversed where neither or both are (each connector's own axis points out of its element, and Revit's export turns a sink's round). That rule gives the 427 joined fitting ports of Snowdon's bidirectional pairs Revit's axis, and on RE1 Plumbing and Mechanical 10 and 6 fitting ports that had the reverse of Revit's, so every port of both now has Revit's point and axis. The 147 axes still reversed on Snowdon are of ports whose flow is not Revit's.

RE1 Mechanical and Plumbing keep 59 of 59 and 118 of 118 of Revit's connections and every flow direction. On the BIM4LCA office as built and at permit, the 24 pipe ports are `HEATING` as Revit's.

## 5. Against the 2024 save

`probe_re371_ducts` prints each duct's type, length, section and ends as the export draws them. Each MEP save against the same project's 2024 save:

| | ducts typed and drawn | lines different from the 2024 save's |
|---|---:|---:|
| `rme_basic` 2015 | 724 of 724 | 1 |
| `rme_basic` 2019, 2025, 2026, 2027 | 724 of 724 | 0 |
| `rme_advanced` 2015 | 728 of 728 | 8 |
| `rme_advanced` 2019, 2025, 2026, 2027 | 728 of 728 | 0 |

The 2015 differences are the project's edits: one duct of `rme_basic` and five of `rme_advanced` have other ElementIds in the later saves (deleted and drawn again, with the same type and section), and three `Taps` ducts of `rme_advanced` are 0.16 to 0.21 ft shorter in the 2015 record itself.

The ports, against this branch's export of the same project's 2024 save, by name (`rme_basic`; `rme_advanced` alike):

| save | ports | connections (shared) | at the 2024 save's place and axis | placed in one only | place or axis different | flow not read here | flow different | system not read here | system different |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| 2024 | 7,370 | 2,948 | | | | | | | |
| 2025 | 7,370 | 2,948 (2,948) | 7,364 | 0 | 0 | 0 | 0 | 0 | 0 |
| 2026, 2027 | 7,370 | 2,948 (2,948) | 5,074 | 2,290 | 0 | 0 | 0 | 0 | 0 |
| 2019 | 7,184 | 2,774 (2,769) | 5,029 | 2,104 | 45 | 1,191 | 38 | 595 | 0 |
| 2015 | 7,182 | 2,869 (2,858) | 5,014 | 2,107 | 46 | 917 | 39 | 1,697 | 0 |

The 6 ports unplaced in every save are of ducts whose record box their line does not fill. On 2026 and 2027, and on 2015 and 2019, the family instances' ports not joined to a placed port are unplaced: a family port is placed from its symbol on 2024 and 2025 only (#869). On 2019, 1,171 of the family instances the 2024 save's records calculate have records the native path does not decode (`unsupported_graph`): their ports come from their symbols with no calculated direction or system, and their joins only where a duct's or pipe's record lists them, 186 ports and 179 joins short of the 2024 save's. Such a port joined to one with a direction takes the opposite, as a bidirectional one does (RE-212), and takes the system of the port joined to it: every one of the 2,634 joins of Revit's export of Snowdon Towers HVAC joins two ports of the same `SystemType`. **No port's system is another than the 2024 save's.** The 38 or 39 duct ports whose flow is different are joined to such undecoded instances. On 2015 the project's own calculations name no system (0) for 715 of `rme_basic`'s duct connectors that the 2024 save classifies, and the ElementIds the project replaced (§5) have their ports only in their own save.

Before, 2015 and 2019 had 2,430 ports (`rme_basic`), none of a family instance, and no connection.

## 6. What else changes

Against the base of this change (RE-370's branch), with `SOURCE_DATE_EPOCH=1700000000`:

- **Samples.** Of the 58 sample saves of 2015 to 2027 (Autodesk's 53 and Snowdon Towers' five of 2024), 25 export IFC and diagnostics byte for byte as before. The 33 that change are the twelve MEP saves, Snowdon Towers HVAC, Plumbing and Architectural of 2024, and 18 architectural (`rac`) saves of 2015 to 2023, whose MEP family instances (lighting, sanitary and electrical fixtures, 30 to 36 a file) now have their ports, unplaced.
- **Corpus.** Of the 206 local project files of 2008 to 2027, 173 export byte for byte as before. On each of the 33 that change, an element-by-element comparison (class, name, every property set, representation, by GlobalId; each port's name, flow, type, system and place; the connections) finds nothing changed but ducts, ports and connections:

| file (release) | ducts typed and drawn | ports added | connections added | ports moved | ports' flow changed | ports' system written |
|---|---:|---:|---:|---:|---:|---:|
| Snowdon Towers HVAC (2025, two copies) | 1,053 | 406 | 406 | 531 | 1,004 | 4,712 |
| Snowdon Towers Plumbing (2025, two copies) | 181 | | | 436 | 693 | 693 |
| Snowdon Towers Architectural (2025, two copies) | | | | | 21 | 21 |
| サンプル設備 (2025, two copies) | 795 | 4 | 4 | 2,911 | 600 | 5,045 |
| サンプル意匠 (2025, two copies) | | | | 86 | 86 | 86 |
| Golden Nugget building services (2025, two copies) | 158 | 17 | 17 | 443 | 6 | 3,256 |
| Golden Nugget architecture (2025, two copies) | | | 4 | 8 | | 7 |
| `rme_basic`, `rme_advanced` (2019) | 724, 728 | 4,754, 4,665 | 2,774, 2,752 | 1,438, 1,446 | 1,448, 1,456 | 1,816, 1,826 |
| `rme_basic`, `rme_advanced` (2027) | 724, 728 | 296 | 296 | 2,847, 2,874 | 228 | 4,687, 4,615 |
| RE1 Mechanical (2025) | 25 | | | 6 | | 128 |
| RE1 Plumbing (2025) | | | | 10 | | |
| BIM4LCA office, as built and at permit (2024) | | | | | | 24 |
| 11 `rac` and other architectural files of 2019 to 2023 | | 30 to 36 | | | | |

  "Moved" counts a port whose place or axis changed, placed where it was not among them; RE1's 16 are the fitting ports whose axis is now Revit's (§4). On RE1 Mechanical the 25 ducts' type, `Length` and `Reference Level` are as before; they gain their swept body, their `Constraints` elevations and justification, and their ports `SystemType`.
- **Time.** No slower where the work is the same: 5.1 and 5.2 s before, 5.1 and 5.3 s now on Snowdon Towers HVAC; 15.2 s and 15.6 s on `rme_basic` 2027; 30.2 s and 28.1 s on Golden Nugget's building services; 3.9 s and 3.9 s on `rac_advanced` 2023, which holds no duct or pipe. On 2015 to 2022 an MEP file reads its family instances' ports and joins as 2024 does, which it did not before: `rme_basic` 2019 takes 6.7 s against 4.4 s (its 2024 save, 17.4 s), and 2015 6.6 s against 4.4 s.

## 7. Not measured, not read

- Against a Revit export on 2015 to 2023, 2026 and 2027: see the status above.
- **Flex ducts and flex pipes:** an `RbsFlexDuctCurve` record holds a `GHermiteSpline` through its points, which needs geometry rvt-rs does not write; the Flex Ducts category is not read, so Snowdon Towers HVAC's 31 flex ducts and their 62 ports are not in the export.
- **Oval ducts:** typed and sized, but kept as their boxes: no oval sweep is written.
- **A pipe's `Constraints` elevations:** no Revit export at hand writes Revit's own property sets on a pipe; Snowdon Towers Plumbing has sloped pipes but no Revit export. A level pipe's invert outside 2025 (RE-157) is not read either.
- **A port's `SystemType` other than Supply, Return and Exhaust Air and Supply Hydronic**, and any electrical port's: see §3.
- **Flow directions** other than an Exhaust Air connector's that differ from Revit's: 266 on Snowdon Towers HVAC; see §3.
- **2015 to 2022 family instances whose record the native path does not decode** (`unsupported_graph`: on `rme_basic` 2019, 1,171 of the family instances whose connectors the 2024 save's records calculate): their ports are read from their symbols, but have no calculated direction or system of their own, and a port not joined to a placed one is unplaced, as on 2026 and 2027 (#869).
- Revit 2008 to 2014: the native record path does not read them.

## 8. Reproduce

```bash
tools/fetch-autodesk-samples.sh samples 2015 2027
cargo run --profile ci -p rvt-probes --example probe_re371_ducts -- samples/2019-rme_basic_sample_project.rvt > 2019.txt
cargo run --profile ci -p rvt-probes --example probe_re371_ducts -- samples/2024-rmebasicsampleproject.rvt > 2024.txt
diff 2019.txt 2024.txt
cargo run --profile ci -p rvt-probes --example probe_re371_ducts -- --native _project_corpus/Revit/RE1-Mechanical.rvt > native.txt
python3 tools/re/ducts_vs_ifc.py native.txt _project_corpus/Revit/RE1-Mechanical.ifc
cargo run --profile ci --bin rvt-ifc -- _project_corpus/Revit/RE1-Mechanical.rvt -o re1.ifc
python3 tools/re/ports_vs_ifc.py re1.ifc _project_corpus/Revit/RE1-Mechanical.ifc
```
