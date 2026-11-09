# RE-370: a pipe's type, cylinder and ports from its own native record, on Revit 2015 to 2027

**Date:** 2026-11-07
**Issues:** #1190 (geometry and properties on 2015 to 2023, 2026 and 2027), #869 (pipe cylinders and ports outside Revit 2024 and 2025), #96 (sloped pipes); follows RE-130 and RE-240 (a pipe's type), RE-131 (a pipe drawn as its cylinder), RE-157 (a pipe's invert), RE-212 (a port's flow direction), RE-247 (a pipe's cross-section and outer surface), RE-280 (native records read on 2015 to 2022).
**Artefacts:** Autodesk's own sample projects, pinned in `research/autodesk-sample-projects.tsv` and fetched by `tools/fetch-autodesk-samples.sh`, never stored; MIT RE1 Plumbing and Mechanical (`magnetar-io/revit-test-datasets`) and the BIM4LCA office (local only), with Revit's own exports.
**Scorers:** `probes/examples/probe_re370_pipes.rs` (each pipe as the export draws it, by ElementId, against the same project's 2024 save; with `--native`, each pipe's record alone) and `tools/re/pipes_vs_ifc.py` (the record against Revit's export).
**Status:** positive. The rule is held to Revit's own export where a file with pipes has one: on 2024 and 2025, 93 of 93 pipes; and on Revit 2018, the two Vitoria projects, counts only (Measure run 42518201407): `Qto_PipeSegmentBaseQuantities.Length` equal on 149 of 149 and 140 of 140, none different, `Pset_PipeSegmentTypeCommon.Reference` on 149 of 151 and 140 of 142, and record-box `IfcPipeSegment` from 151 to 2. That export, an IFC2X3 one, holds no `Pset_PipeSegmentOccurrence.InvertElevation`, so the 44 and 45 rvt-rs writes there count as values Revit's export does not write, which an IFC2X3 export does not remove. The rule is held as well to the same Autodesk samples saved by 2024 on 2015, 2019 and 2025 to 2027. **No pipe of a file of 2015 to 2017, 2019 to 2023, 2026 or 2027 is scored against a Revit export**: none of the local Revit exports of those releases holds a pipe, and a later save cannot catch an error it shares.

## 1. Before

- **2015 to 2023:** a pipe is an `IfcPipeSegment` with no type, its record box as its body, no `Qto_PipeSegmentBaseQuantities` or invert, and two unplaced ports: 491 pipes on the `rme_basic` sample and 488 on `rme_advanced`. RE-131's connector entries and RE-240's native type are read on 2024 and later only.
- **2026 and 2027:** every pipe is its box with unplaced ports (491 of 491 on the 2027 `rme_basic` sample), because RE-131's entries are measured on 2024 and 2025 only (#869).
- **2024 and 2025:** RE-131's entries do not place every pipe: 35 on `rme_basic` and 36 on `rme_advanced` (2024 and 2025 alike), and on local 2025 files 172 of Golden Nugget's building services 2,381, 2 of its architecture's 6, 49 of Snowdon Towers Plumbing's 3,051 and 12 of サンプル設備's 1,155. They stay boxes.
- **2024 and 2025, names:** a pipe's type was named by the name found near its type id in the partition's bytes (RE-130, RE-134), and RE-240's own record only where those found none. On Golden Nugget's building services (2025) that gives 272 pipes of one type the name of a rectangular flex duct, and 6 of another a stray character.
- **2015 to 2022:** a pipe's ports have no flow direction, so an `In` port points out of its pipe, where Revit points it in (RE-212); the connector pass that reads the direction reads 2023 and later.

## 2. What a pipe's record holds

A pipe's `RbsPipeCurve` record, read through its native graph on every release from 2015 to 2027 (RE-280), holds:

| object | field | read as |
|---|---|---|
| `RbsPipeCurve` | `m_idType` | its type, an `RbsPipeType` whose `SymbolInfo.m_name` is the type's name (RE-240's rule) |
| `GLine` | `m_origin`, `m_dirVec`, `m_endParams` | its centreline, from `origin + dirVec × endParams[0]` to `origin + dirVec × endParams[1]`, model feet |
| `PipeDomainDesignPropertyManager` | `m_dOuterDiameter` | twice the radius of the cylinder it is drawn as |
| `SegmentConnectorPosition` | `m_pConnector`, `m_paramOnCurve` | which `Connector` (by its `m_nIndex`) sits at the line's start (0) and end (1) |
| `SegmentConnectorCalculation` | `m_pConnector`, `m_eDirection` | each connector's calculated direction, 1 In, 2 Out, 0 bidirectional (RE-212's numbering) |

Every pipe record of the twelve MEP saves (2015, 2019, 2024 to 2027) holds exactly one `GLine`. Pipe 513756 of `rme_basic` 2019: the line runs 15.5475 ft, the 4.738869 m `Length` the 2024 save's export gives it; the outer diameter is 0.6667 ft, whose circle is its 0.032429 m² `GrossCrossSectionArea`.

`attach_native_pipes` reads them for every pipe still without a type or a body, on the releases the native record path reads (`NATIVE_PIPE_REVIT_VERSIONS`, 2015 to 2027): on 2015 to 2023 from the 2015 to 2023 branch of `recover_partition_schema_mvp`, every pipe; on 2024 to 2027 from `attach_pipe_axes`, after RE-131's entries and before their ends are keyed and before RE-130's and RE-134's names, so there it types every pipe and draws those the entries do not place (on 2026 and 2027 every pipe until RE-346, #1454, widened RE-131's entries to those releases; since then, as on 2024 and 2025). It writes the fields RE-131 and RE-240 write (`TYPE_ID_FIELD`, `TYPE_NAME_FIELD`, `PIPE_AXIS_START_FIELDS`, `PIPE_AXIS_END_FIELDS`, `PIPE_RADIUS_FIELD`, `PIPE_END_KEYS_FIELDS`) and the connectors' directions (`PIPE_FLOW_DIRECTIONS_FIELD`), so the type's family (`Pipe Types`, RE-130), the cylinder (RE-131), the quantities (RE-247), a vertical pipe's invert (RE-157) and the ports (#847, RE-212) are written by the same code as for a pipe RE-131 places. The cylinder is held to RE-131's own test: the record box must give the record's radius and be the box the cylinder fills (`partition_pipe_axes::pipe_body`). A value is written only where every copy of the record gives the same one; a pipe whose record holds no line, more than one, or no positive outer diameter keeps its box.

A pipe's inner diameter is not read: a level pipe's invert, which needs it, is written on 2025 only, the release RE-157 measured it on, and a 2024 pipe has none either.

## 3. Against Revit's own export

`probe_re370_pipes --native` prints each pipe's record alone, whatever else places it, and `tools/re/pipes_vs_ifc.py` holds it against Revit's export by ElementId:

| file | release | pipes | `Length` | vertical `InvertElevation` | `OuterSurfaceArea`, `GrossCrossSectionArea` |
|---|---|---:|---:|---:|---:|
| RE1 Plumbing | 2025 | 63 | 63 | 21 of 21 | not written (IFC2X3) |
| RE1 Mechanical | 2025 | 6 | 6 | 2 of 2 | not written (IFC2X3) |
| BIM4LCA office, as built | 2024 | 12 | 12 | 12 of 12 | 12 and 12 |
| BIM4LCA office, building permit | 2024 | 12 | 12 | 12 of 12 | 12 and 12 |

**93 of 93 pipes have Revit's length, and 47 of 47 vertical pipes its invert.** RE-131 places all 93 already, so on these files nothing the export writes changes; they measure the record's rule where Revit's answer is known. With RE-131's entries switched off in a local build (not part of this change), the record's connectors place every pipe port of RE1 Plumbing (124 of 124), RE1 Mechanical (62 of 62) and the as-built office (24 of 24) within 0.01 ft of Revit's, with Revit's axis, as RE-131's keys do.

On 2024's `rme_basic` 17 of the 491 pipes slope, and RE-131 places all 17; the record gives the same line and radius for each (#96).

## 4. Against the 2024 save

`probe_re370_pipes` prints each pipe's type, length and radius as the export draws it. Each MEP save against the same project's 2024 save:

| | pipes | lines different from the 2024 save's |
|---|---:|---:|
| `rme_basic` 2015 | 491 | 3 |
| `rme_basic` 2019, 2025, 2026, 2027 | 491 | 0 |
| `rme_advanced` 2015, 2019, 2025, 2026, 2027 | 488 | 0 |

On 2015's `rme_basic` pipe 727577 is another pipe: its own record gives it a 0.0984 ft outer diameter and 0.083 ft of line against 0.1042 ft and 0.053 ft in the later saves, so the project was edited between them. Pipes 726675 and 726676 hold an outer and inner diameter of 0 on that save alone, and keep their boxes.

On what the export writes, by pipe, against `main`'s export of the 2024 save (which this rule does not reach), over the class, name, type, every property set but `Constraints` and `Phasing` (RE-369, RE-368) and the world box of the body:

| | values equal | different | only this branch's | only `main`'s 2024 |
|---|---:|---:|---:|---:|
| `rme_basic` 2015 | 7,850 | 3 | 121 | 6 |
| `rme_basic` 2019, 2024, 2025, 2026, 2027 (each) | 7,859 | 0 | 121 | 0 |
| `rme_advanced` 2015, 2019, 2024, 2025, 2026, 2027 (each) | 7,807 | 0 | 125 | 0 |

**93,987 equal, 3 different, all three pipe 727577's.** Every pipe has the 2024 save's name, type, `Family`, `Type Id` and type `Reference`; every `Length`, `GrossCrossSectionArea` and `OuterSurfaceArea` both give is the same to 0.01 %, and so is every vertical pipe's `InvertElevation` (108 and 107); the world box of each body is the 2024 save's to 1 mm. The 6 only in `main`'s are the quantities of the two pipes whose 2015 diameters are 0. Only in this branch's are the quantities of the 35 or 36 pipes RE-131's entries do not place and the invert of the 16 or 17 of them that are vertical, on every save including 2024's own: the record gives each the same line and diameter on all six saves, and its cylinder passes RE-131's box test on each.

The pipes' ports, against this branch's export of the 2024 save, by name: on every save of 2015, 2019 and 2025 to 2027 every port is at the same point, with the same axis and flow direction, as the 2024 save's (982 on `rme_basic`, 976 on `rme_advanced`), but on 2015's `rme_basic` the four of the two zero-diameter pipes, which are unplaced, and one of pipe 727577's. Before, the ports of 2015 and 2019 had no direction, 475 of `rme_basic` 2019's pointed out of their pipe where the 2024 save's point in, and none of 2026's or 2027's was placed. On 2024 itself the 35 or 36 pipes' ports gain a place (8) or are now placed at the pipe's own end (7) where a join had placed them, with the axis every other pipe port of that flow direction has, the rule that gives Revit's axis on RE1 Plumbing's 124 pipe ports.

## 5. What else changes

- **Pipes and the ports joined to them.** Of the 58 sample saves of 2015 to 2027 (Autodesk's 53 and Snowdon Towers' five of 2024), 45 export IFC and diagnostics byte for byte as on `main`. The 13 that change are the twelve MEP saves and Snowdon Towers Plumbing of 2024 (60 of its 3,056 pipes, which RE-131's entries do not place, are now cylinders). On each, every product but a pipe keeps its class, Tag, GlobalId, name, storey and every property set, and every pipe its class, Tag, GlobalId and storey. The ports that change are the pipes', and on 2024 to 2027 the ports joined to them, which take the pipe port's point and axis (#869): on each 2024 MEP sample 7 fitting ports turn with their pipe port; on 2027, 974 and 968 fitting ports are placed, each at the 2024 save's point, axis and flow direction. The diagnostics differ in `parameter_value_count`, `total_entities` and the body sources alone.
- **Byte identity.** Of the 206 local project files of 2008 to 2027, 194 export IFC and diagnostics byte for byte as `main` does (`SOURCE_DATE_EPOCH=1700000000`), RE1 Plumbing, RE1 Mechanical and the BIM4LCA office among them. The 12 that change hold pipes: the 2019 and 2027 MEP samples, and Golden Nugget's building services and architecture, Snowdon Towers Plumbing and サンプル設備, two 2025 copies of each. On each only pipes and ports change.
- **Golden Nugget's type names.** On its building services 278 pipes are renamed, 272 of one type and 6 of another, from the wrong names above to their types' own. The file has no stated licence, so only counts are given. The Type Id is unchanged; the name is the type's own `RbsPipeType` record's, RE-240's rule. No Revit export of the file is at hand to score it. No other file's pipe type changes.
- **Time.** No slower: 4.29 s on `main` and 4.35 s here on `rme_basic` 2019 (within noise), 15.3 s and 14.7 s on `rme_basic` 2027, 30.9 s and 29.7 s on Golden Nugget's building services, 3.81 s on both for `rac_advanced` 2023, which holds no pipe. The faster two are not profiled; typed pipes no longer send RE-130's name scan through the partitions.

## 6. Not measured, not read

- Against a Revit export on 2015 to 2023, 2026 and 2027: see the status above.
- A level pipe's invert on 2015 to 2024: its inner diameter is in the same record (`m_dInnerDiameter`), but the invert rule is measured on 2025 only (RE-157). On 2026 and 2027 RE-346 writes it from the curve fields, held to the 2025 saves.
- Ducts: an `RbsDuctCurve` record holds a line too; not read here.
- Fitting and other family-instance ports on 2015 to 2022: their joins are read by the native connector pass on 2023 and later only, so a pipe's placed port is joined to nothing there, and the fitting's port stays unplaced (#869).
- A port's `SystemType` (#834): the calculation's `m_eSystemType` is in the same record, with no IFC4 export spanning several systems to map it against.
- Revit 2008 to 2014: the native record path does not read them.

## 7. Reproduce

```bash
tools/fetch-autodesk-samples.sh samples 2015 2027
cargo run --profile ci -p rvt-probes --example probe_re370_pipes -- samples/2019-rme_basic_sample_project.rvt > 2019.txt
cargo run --profile ci -p rvt-probes --example probe_re370_pipes -- samples/2024-rmebasicsampleproject.rvt > 2024.txt
diff 2019.txt 2024.txt
cargo run --profile ci -p rvt-probes --example probe_re370_pipes -- --native _project_corpus/Revit/RE1-Plumbing.rvt > native.txt
python3 tools/re/pipes_vs_ifc.py native.txt _project_corpus/Revit/RE1-Plumbing.ifc
```
