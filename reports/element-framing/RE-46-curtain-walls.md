# RE-46 — Curtain walls are the walls their mullions name

**Date:** 2026-09-23
**Result:**
- **Positive.** A placed wall whose element record is named in the reference list of a placed curtain-wall mullion (`OST_CurtainWallMullions`) is a curtain wall.
- On Snowdon Towers Architectural those walls are exactly the 42 Revit's IFC4 export writes as `IfcCurtainWall`, and on RE1 Architecture (Revit 2025) the one. No other wall on any file in the corpus is named by a mullion.
- rvt-rs now exports them as bodiless `IfcCurtainWall` `.NOTDEFINED.` aggregates, as Revit does. The parts are the panels and mullions that name that curtain wall and no other.
- 1,798 of Revit's 1,906 aggregate relations on Snowdon, and 12 of 13 on RE1, are reproduced exactly. What remains is recorded below.
- Before this change the 43 curtain walls were `IfcWall` boxes drawn over their own glazing.

**Probe:** `examples/probe_re46_curtain_walls.rs` lists the curtain walls, the panels and mullions each aggregates, and the walls named only by panels.

**Oracles:**
- Snowdon Towers Sample Architectural (local only, `.rvt` sha256 `33271010…`) with Revit's IFC4 export (`ecfcb04e…`).
- `RE1-Architecture.rvt` (MIT) with its IFC2x3 export.
- Core Interior and `teste_export_2025` as controls: they have no curtain walls.

-----

## 1. The rule

| walls named by | Snowdon | RE1 | Revit's export |
|---|---:|---:|---|
| at least one mullion | 42 | 1 | `IfcCurtainWall`, every one |
| panels only | 6 | 0 | `IfcWall`, every one |
| neither | 1,080 | 7 | `IfcWall` or not exported |

The six walls named only by panels (660424, 1538585, 1546499, 1547379, 1594191, 1677357) are basic walls used as a panel's infill. Each is named by one or two panels and no mullion. Among the walls that are curtain walls, the fewest mullions naming one is 3.

A panel names its curtain wall as well, but because of those infill walls a panel alone does not make one.

Checked and rejected along the way: "the wall's own reference list names the panel or mullion that names it". Infill walls do that too.

## 2. The export

- The wall's class becomes `CurtainWall`, mapped to `IFCCURTAINWALL` with `PredefinedType` `.NOTDEFINED.`. That is Revit's value on all 60 `IFCCURTAINWALL` rows of the Snowdon export.
- Like a stair (RE-39), it has a placement and no representation. Revit's 42 top-level curtain walls have none either.
- Each `CurtainWallPanel` and `CurtainWallMullion` whose reference list names exactly one curtain wall is its part, through `IfcRelAggregates`, and leaves the storey containment it reaches through the wall.
- Wall joins, column cuts and door hosting are computed from the records before the class changes, so no other element's geometry moves. Core Interior's export is byte-identical.
- A whole's body is held back until its parts are known, and a whole no part names keeps it. That fixes a defect from RE-39. Snowdon stair 1603717 ("Residential Lobby Stair Landing Support") is a stair family placed on its own, and Revit exports it as an `IfcStair` with a representation and no aggregate. rvt-rs had been writing it with no body, so it drew nothing. It now keeps its record box.
- The export diagnostics gain `exported.building_elements_carried_by_parts`, and `rvt-ifc` prints it. On Snowdon: "6105 building element(s), 6037 with geometry and 68 carried by their parts". Before, the summary read as if 69 elements had no geometry.

## 3. Measured

Per part, against Revit's `IfcRelAggregates` whose whole is an `IfcCurtainWall`:

| | Snowdon | RE1 |
|---|---:|---:|
| part under the same whole as Revit's | 1,798 | 12 |
| part under a different whole | 1 | 0 |
| Revit aggregates it, rvt-rs does not | 112 | 1 |
| rvt-rs aggregates it, Revit's export does not hold it | 1 | 0 |

The rows that are not exact:

- **89 mullions** name no curtain wall. All have late ids (2340144 and on) and belong to curtain walls that other, earlier mullions do name. For 72 of them a record they name, in turn, names the wall. That record is likely the curtain grid, but its category is not measured and the two-hop join is not shipped. They stay standalone `IfcMember` elements with their own bodies.
- **18 nested curtain walls.** Revit writes 18 Snowdon panels as `IfcCurtainWall` parts of another curtain wall. No mullion names them, and nothing read so far tells them from ordinary panels. They stay `IfcPlate`: 13 standalone, 4 naming two curtain walls, and 1 (1593696) under a different whole than Revit's.
- **6 doors on Snowdon, 1 on RE1.** Nine placed Snowdon doors name a curtain wall. Revit aggregates 6 of them. The other 3 (1506352, 1514788, 1516507) all name curtain wall 1506500, and Revit leaves them standalone. Aggregating doors would give 6 right and 3 wrong, and nothing read so far tells them apart, so doors are not aggregated. RE1's one door naming its curtain wall (445975) is aggregated by Revit.
- **Panel 1442355** is exported by rvt-rs and not by Revit (the #309 class of elements Revit leaves out).

Per Tag, the 43 curtain walls no longer differ from Revit's entity. Of the curtain-wall elements, only the 18 nested ones still do. Unrelated differences are unchanged: plumbing fixtures are `IfcSanitaryTerminal` where Revit writes `IfcFlowTerminal`, and two ramps are `IfcRamp` where Revit writes `IfcRampFlight`.

IFC schema arity, `PredefinedType` and IfcOpenShell 0.8.5 validation pass on the RE1 and Snowdon exports. Core Interior's witness-agreement check is unchanged.

## 4. What this does not do

- It reads no curtain grid, grid line or panel type. The rule is only the mullion reference.
- It does not write `IfcCurtainWallType`.
- A curtain wall with no mullions at all (no grid, no border mullions) stays an `IfcWall` box, which is fail-closed. No such wall is in the corpus.
