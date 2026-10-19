# RE-185: a wall's IsExternal is its type's native Function, and ExtendToStructure is its roof attachment

**Date:** 2026-10-05
**Issues:** #35; backlog items B28 and B42
**Artefacts:** RE1 Architecture (Revit 2025, `Drshelden/IFC-ECS`, MIT) and `R2025_Models.zip` from #356 (Apache-2.0, sha256 `8673d803…6a`): purpose-made Revit 2025 projects, 21 of them with walls and Revit's own IFC4 Reference View export. Measured locally, and reproduced on GitHub's runners by Measure with `-f r2025=true` (#805, run 39175059258): the same counts as §3.1 and §3.3.
**Probe:** `examples/probe_re185_wall_function.rs`.
**Status:** positive for the walls' `IsExternal` and `ExtendToStructure`; B42's `FireRating` id is already settled by RE-77; a family type's text parameters sit in a table RE-77's scan does not read. `LoadBearing` is not measured.

## 1. The question

RE-158 found a door's and a floor's `IsExternal` in its type's `FUNCTION_PARAM` entry, but no wall type holds one, and every wall boolean on RE1 is false, so nothing there tells the candidates apart. RE-154 found RE1's wall `FireRating` ("2") under four of the type's ids at once and left B42 waiting for a model with a longer rating. Where are these values?

## 2. Method

For every wall and curtain wall in Revit's export (`IfcWall`, `IfcWallStandardCase`, `IfcCurtainWall`, by `Tag`), the probe reads `Pset_WallCommon` and `Pset_CurtainWallCommon` and compares them with:

- **`IsExternal`**: the type's native `WallType.m_function`. The type comes from the wall's native record (`SWall.m_WallAttributesId`), and both records are read with `native_document::extract_graphs`.
- **`ExtendToStructure`**: whether the wall's own record graph holds a `JoinToRoofGStep` object.
- **`FireRating`**: the type's text entries -1001206, -1001405, -1002500 and -1010103 (RE-154's four ids), read with `scan_text_parameter`.

For each door or window `FireRating`, it looks for the text as a family-type table entry (§3.3) and names the parameter id that follows it from that id's definition (RE-156's `autodesk.parameter.group:` object).

## 3. Result

### 3.1 Walls

| models | walls | `IsExternal` true / false | equal to `m_function == 1` | `ExtendToStructure` true / false | equal to `JoinToRoofGStep` |
|---|---:|---|---:|---|---:|
| RE1 Architecture | 8 (1 curtain wall) | 0 / 8 | 8 | 0 / 7 | 7 |
| `R2025_Models.zip`, 21 models | 117 | 117 / 0 | 117 | 32 / 85 | 117 |

None differs. As a local check, 14 earlier models by the same author (65 walls, one a curtain wall; 8 walls `ExtendToStructure`) agree on every wall too.

- **`m_function` is the wall type's Function**: 0 on RE1's types 381192 and 474504 (the curtain wall's), 1 on the made models' types. That is the enum RE-158 read for doors (`FUNCTION_PARAM`: Interior 0, Exterior 1). It is a native field of `WallType` (`Formats/Latest`: `m_panel`, `m_intMullions`, …, `m_function`, `m_fixed`), not a parameter entry, which is why RE-158 found no wall type entry.
- **`JoinToRoofGStep`**: each of the 32 walls Revit marks `ExtendToStructure` has one, and none of the other 92 does. In `12_Rooms_Door_Windows`, wall 2892 is the one Revit marks. Its record holds `JoinToRoofGStep` and two `WallCutoutGStep`s that walls 2787 to 2790 lack. All of these walls' parameter entries and top-level `SWall` fields are equal.
- **`FireRating` on RE1**: the four ids hold "2" in each wall type, as RE-154 found. RE-77 has already named them by matching values against Snowdon Towers' VIM export: -1001405 Type Mark, -1010103 Description, -1001206 Fire Rating. It reads RE1's 7 wall fire ratings from -1001206, equal to Revit's. rvt-rs carries that value on each wall as the type parameter `Fire Rating`, in its own identity property set. It does not write it as `Pset_WallCommon.FireRating` or `Pset_ConcreteElementGeneral.FireRating`, the 14 values B42 counts.

### 3.2 What was ruled out

- The wall's own -1001101 entry: its first word is 1 on every wall, RE1's (false) and the made models' (true).
- `LoadBearing` is false on every wall measured, so this does not identify it.

### 3.3 A family type's text parameters

Models 12 to 17 give their windows a `FireRating` of `0.5h`. Revit writes it on 3 windows of each (`Pset_WindowCommon`, type `Window-Fixed:24" x 36"`), and neither RE-77's scan nor RE-154's finds it. The text is an entry in the window family types' parameter table, whose entries read:

```text
u32 n · UTF-16 × n · (u32 0 | ff ff ff ff · u16 tag) · f64 · i64 ElementId (-1 none) · i64 parameter · u32 · u16
```

The text belongs to the parameter id that follows it, not the one before it. The double before an id is that id's length (RE-93's `f64 · ff × 8 · id`). Window type 12501 shows this, decoded in full: Width -1001301 holds 3.0 ft and Height -1001300 5.417 ft (its name is `36" x 65"`), and family parameter 12487, "Frame Extension" (`autodesk.spec.aec:length`), holds 0.0208 ft (¼"). The `0.5h` that follows 12487's id comes before 12482's id.

| models | `0.5h` table entries, by the parameter after them | elsewhere |
|---|---|---:|
| models 12 to 17, each | 12482 `Fire Rating` × 18, 12220 `Fire Rating` × 9 | 0 |

Both ids are definitions named `Fire Rating` in group `identityData`. Both carry the same family-local GUID (`revit.local.family:7c0efa9d…`), so they are two definitions of one family's parameter. Which one the project's window type uses is not settled here; the text is in the tables of both. Revit's export takes the window's `FireRating` from a family parameter with that name. RE-77's scan reads `id · u32 n · UTF-16`, with the id first and no gap, so it never meets these entries. That may be some of the 166 rated Snowdon elements RE-77 does not read. Snowdon's export is not available here to check.

## 4. What this changes

- **B28, walls.** `Pset_WallCommon.IsExternal` and `Pset_CurtainWallCommon.IsExternal` can be written from the type's native `m_function` (1 true, 0 false, nothing for other values or an unread type). `Pset_WallCommon.ExtendToStructure` can be written from whether the wall's record holds a `JoinToRoofGStep`. On RE1 that is 8 `IsExternal` and 7 `ExtendToStructure` values (B28's walls 7 and curtain wall 1, and all 7 `ExtendToStructure`).
- **B42** is not blocked on a model. RE-77 already identifies -1001206, and the value is already on each wall. Writing it as `FireRating` in `Pset_WallCommon` and `Pset_ConcreteElementGeneral` covers B42's 14 values.
- **Family-type text parameters.** A reader of this table could fill the door and window `FireRating` that RE-77 misses, keyed by parameter definitions by id (B40 already notes that two definitions can share a name).

## 5. Not done

- **`LoadBearing`** needs a model with a wall marked Structural.
- **`IsExternal` false comes from RE1 alone.** A made model with an Interior wall type would confirm 0 → false outside RE1.
- **Attached tops.** Only walls attached to roofs are measured. A wall whose top is attached to a floor or ceiling is not.
- **Other releases.** The family-type table is read on Revit 2025 only, and `m_function` wherever the native record path reads (2023 to 2027).

## 6. Reproduce

```text
cargo run --profile ci --example probe_re185_wall_function -- RE1-Architecture.rvt R2025_Models/*.rvt
```

Each `MODEL.rvt` is read with Revit's export beside it as `MODEL.ifc`. RE1 is fetched as CI fetches it (`ci.yml`, "Fetch the Revit 2025 oracle"). `R2025_Models.zip` is on #356. On GitHub's runners, with each model's `probe.txt`:

```bash
gh workflow run measure.yml -f ref=main -f base=none -f r2025=true -f probe=probe_re185_wall_function
```
