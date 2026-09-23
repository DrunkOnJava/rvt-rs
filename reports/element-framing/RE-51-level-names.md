# RE-51 — Level names and elevations on every 2024 and 2025 project

**Date:** 2026-09-23
**Result:**
- **Positive.** RE-24 read Core Interior's Level names and elevations from a name block. The same block holds them on Snowdon Towers (2024) and on the 2025 projects. RE-24 missed it there for two reasons, both corrected here:
  - two bytes RE-24 counted as marker are data;
  - the elevation's confirming copy is not at a fixed distance.
- Levels in second-prologue frames take their ElementId from their partition record (RE-35). A Level that sits inside another element names that element in its block, and is not a storey.
- Every storey rvt-rs writes now has the name, elevation and GlobalId (RE-48) of Revit's own storey:

  | file | storeys |
  |---|---|
  | Snowdon Towers | 18 of 18 (from 17 storeys named by elevation, 9 at Revit's elevations) |
  | RE1 Architecture | 2 of 2 |
  | Projeto1 | 2 of 2 |
  | teste_export_2025 | 2 of 2 |
  | Core Interior | 15 of 15, unchanged |

- With named storeys, elements bind to their Level. On Snowdon, 3,379 of the 3,380 elements both exports place in a storey are in Revit's (none were in a storey named as Revit's), and on RE1 Architecture 14 of 14.

**Oracle:** Revit's own IFC exports of each file:
- Core Interior and the RE1 models (licensed, CI tier 2);
- Snowdon Towers, Projeto1 and teste_export_2025 (local only).

-----

## 1. The block, restated

RE-24 measured this layout on Core Interior, where the name `V` is preceded by:

```text
V-0x47  u64   the Level's ElementId
V-0x3f  56 B  0xff
V-0x04  u32   name length, UTF-16 units
V       ...   name
```

On Snowdon Towers and the 2025 projects the same bytes are the start of the Level's element data (RE-44): `ff ff ff ff · u16 · 01 00 00 00` and the ElementId, then the 56-byte run.

- Two Snowdon Levels, M1 and "L1 - Block 37", are in second-prologue frames. Their data opens with `00 00 00 00 00 00 01 00 00 00` instead, and the block is the same.
- The block scan finds the name by its framing, not by what precedes the id, so all three forms read alike.

## 2. The elevation marker

RE-24's marker was `05 00 00 00 48 02 00 00`, with the elevation 55 bytes on:

- The marker is the first six bytes: `05 00 00 00`, then a `u16` per release, `0x0248` on 2024 and `0x025d` on 2025.
- Four plan extents follow it as `f64` feet, then the elevation. On Snowdon's L3 the extents are about −90.0, −114.0, 90.0 and 114.0.
- RE-24's last two bytes were the first extent's low bytes: zero on Core Interior, whose extents are round, and not zero on Snowdon.
- The marker sits 347 to 367 bytes past the name: 347 or 363 on Core Interior (RE-24), 361 on Snowdon, and 363 and 367 on RE1.

## 3. The confirming copy

RE-24 confirmed the elevation with a second copy 153 bytes on. That distance holds on Core Interior only: it is 161 bytes on Snowdon, and 246 and 181 on RE1's two Levels.

What holds on every file is that the second copy is preceded by the same 24 bytes as the first. So the elevation is accepted when those 32 bytes (the 24 before it and its own 8) recur within 2,048 bytes after it. This confirms all 15 of Core Interior's Levels, where the copy is found at +153 as before, all 18 of Snowdon's and both of RE1's.

## 4. Which Levels are storeys

A Level record is a storey candidate under RE-24's rule: standalone, placed, and declared in `Global/ElemTable`. Snowdon has 20:

- 16 in first-prologue frames, and 2 more in second-prologue frames (M1 and "L1 - Block 37"). Their ids come from the partition record each frame sits in, as RE-35 found for other elements.
- 2 whose block names an owner. After the Level's own id come eight `0xff` bytes and then another element's ElementId (1982117 names 1982107, and 1981744 names 1981691), where a project Level's run is unbroken. Neither is a storey in Revit's export. They are left out of the storey set and do not count against it.

The storey set is still all or nothing, as RE-24 made it: every candidate needs a confirmed block, or no Level is used. The loaded families' own Levels ("Ref. Level", "Ground floor" and so on) are not declared in the project's ElemTable and never become candidates.

## 5. Measured

Storeys, against Revit's export of the same file (name, elevation to 0.001 ft, GlobalId):

| file | release | storeys | name and elevation | GlobalId |
|---|---:|---:|---:|---:|
| Core Interior | 2024 | 15 | 15 | 15 |
| Snowdon Towers | 2024 | 18 | 18 | 18 |
| RE1 Architecture | 2025 | 2 | 2 | 2 |
| Projeto1 | 2025 | 2 | 2 | 2 |
| teste_export_2025 | 2025 | 2 | 2 | 2 |

Before this change:
- Snowdon's 17 storeys were derived from element boxes and named by elevation. 9 were at Revit's elevations, 8 were not Revit's, and 9 of Revit's were missing.
- RE1 Architecture had four storeys taken from partition strings: "Ground floor", "Level 1", "Level 2" and "Roof". The first and last are loaded families' Levels, not storeys of Revit's export.
- Projeto1 and teste_export_2025 had one default storey.

Containment, per element, against the storey Revit's export contains it in:

- Snowdon Towers: 3,380 elements are in a storey in both exports (1,489 were before), and 3,379 are in Revit's. The one that differs is in L3 in Revit's export and L2 in rvt-rs's.
- RE1 Architecture: 14 of 14, matched by GlobalId since that export is IFC2X3.

## 6. What this does not do

- Revit 2023 and earlier are not read. Exemplo_data and modelo_bim, both 2023, keep one default storey.
- Why a Level is a storey in Revit's export (its "Building Story" setting) is not read. The owner rule separates the two non-storey Levels on Snowdon, and no file measured has a declared, unowned Level that Revit does not export.
- The four plan extents after the marker are recorded here, not used.
- **Snowdon's structural model keeps its box-derived storeys.** All 19 of its Levels are in the VIM, and 11 read exactly as above. The other 8 (L1_43_High, L2, L3, L4, L5, R2, Parking and Parapet 2) carry `01 00 00` in the three bytes between the sentinel run and the name length, where every Level on the files with a Revit export carries `00 00 00`. The byte may be the "Building Story" setting, but there is no Revit export of that model to read it against. The storey set is all or nothing, so it is refused, as before.
