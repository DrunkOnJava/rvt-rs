# RE-91 — A category's material from the document's object styles

**Date:** 2026-09-28
**Issues:** #355
**Result:**
- The document's object styles hold an entry per category. The entry is the category's `BuiltInCategory` as an `i64`, an unset `u64`, `u32 1`, another unset `u64`, then the category's material as a `u64`:

  ```text
  i64 BuiltInCategory · ff × 8 · 01 00 00 00 · ff × 8 · u64 material
  ```

  On Core Interior (Revit 2024) the Walls entry holds 87, the material named "Default Wall", and Roofs holds 88, "Default Roof". The Floors, Ceilings, Doors, Windows and Columns entries hold all ones (unset).
- A wall layer that takes its category's material (its material stored as all ones, RE-53) now takes that material, where the Walls entry is set and the material is named. On Core Interior, Revit's IFC4 export writes "Default Wall" for all 356 walls whose layer takes its category's material. They now carry it.

| Core Interior's 360 walls against `2024_Core_Interior_slim.ifc` | main | RE-91 |
|---|---:|---:|
| Revit's material, same name | 4 | 360 |
| as a one-layer layer set named after it (RE-58) | 0 | 315 |
| as the `IfcMaterial` itself (RE-88) | 4 | 45 |
| without a material | 356 | 0 |

- Core Interior's layer sets go from 88 to 403. All 404 layer names are Revit's, and every whole set lies on Revit's body within 0.001 ft (`layer_sets_vs_ifc.py`).
- Snowdon Towers' Walls entry holds 0, and the MIT house's too, so nothing changes there. Revit's export still writes "Default Wall" for Snowdon's 15 such walls, so an unset entry falls back to something not read, and those walls stay without a material.

-----

## 1. The entry

On Core Interior the Walls entry is found as a `u64` 87 lying 28 bytes after OST_Walls (−2000011) in `Partitions/46`. The same shape occurs for every category, preceded by `ff ff ff ff 69 08`:

| category | material |
|---|---|
| Walls | 87, "Default Wall" |
| Roofs | 88, "Default Roof" |
| Floors, Ceilings, Doors, Windows, Columns | all ones (unset) |

Snowdon Towers' Walls entry holds 0, and the MIT house's (2024) holds 0. RE1's files (Revit 2025) hold no entry of this shape. The reader runs on Revit 2024 only, and takes the value only where every copy agrees and it is neither 0 nor all ones.

## 2. Evidence and its limit

One file sets the Walls material, and on it Revit's export agrees on every wall that takes its category's material (356 of 356). No file here sets a category material that Revit's export would contradict. What Revit writes where the entry is unset ("Default Wall" on Snowdon Towers, which also holds a material of that name, 18438) is not read, so those walls are left alone. Only walls use the entry: no measured export shows a floor, roof or ceiling layer that takes its category's material.

## 3. End-to-end measurement

| file | sha256 |
|---|---|
| 2024_Core_Interior.rvt (MIT, magnetar-io/revit-test-datasets) | `c805df44…` |
| 2024_Core_Interior_slim.ifc (Revit's export, same repository) | `bfdf36ff…` |

```bash
./target/ci/rvt-ifc 2024_Core_Interior.rvt -o model.ifc
python3 tools/re/wall_materials_vs_ifc.py 2024_Core_Interior_slim.ifc main.ifc model.ifc
python3 tools/re/layer_sets_vs_ifc.py model.ifc 2024_Core_Interior_slim.ifc
```

- **Core Interior:** only wall material associations change; both witness observations are refreshed, verdicts PASS with all replays matching.
- **Snowdon Towers (architectural, structural), the four RE1 models, Einhoven and the MIT house (2024 and 2025):** IFC byte-identical to main's apart from the timestamp.
