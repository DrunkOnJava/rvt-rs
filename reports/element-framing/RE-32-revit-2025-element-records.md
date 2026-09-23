# RE-32 — Revit 2025 element records: the bbox marker is per release, and the 2024 decode carries over

**Date:** 2026-09-22
**Result:** **Positive** on the record shape and on precision:
- Revit 2025 frames have the 2024 geometry, with a marker derived from the release.
- The RE-21 instance rule reproduces Revit's own export with **zero false positives** on six 2025 files.
- On the one 2025 oracle with element-bearing records it is exact: walls 7/7, slabs 2/2, rooms 11/11.

**Partial** on recall: as on Snowdon (RE-30), most door and window frames use the second prologue.

**Oracles:**

| file | Revit | IFC exporter | licence |
|---|---|---|---|
| `RE1-Architecture`, `-Electrical`, `-Mechanical`, `-Plumbing` (`Drshelden/IFC-ECS`, `data/RE1/`) | 2025, build `20250515_1515` | Revit 26.2.0.20 | **MIT** |
| `Projeto1`, `teste_export_2025` (`255ribeiro/intro_ifc`) | 2025 | Revit 25.4.20.11 | none (local only) |
| `modelo_bim`, `Exemplo_data` (same repo) | 2023 | 26.2 / 2023 | none (local only) |

`tools/fetch-corpus.sh` now clones the MIT repository. `tests/element_records_2025.rs` runs against it when `RVT_PROJECT_CORPUS_DIR` points at `IFC-ECS/data/RE1`.

-----

## 1. The marker is not fixed; it is derived from the release

`partition_element_records` recognised a record by an 8-byte marker at `+0x50`: `46 01 FF FF FF FF AB 05`. Revit 2025 files contain none of those bytes. They contain `59 01 FF FF FF FF D3 05` at the same offset, in front of the same six-`f64` bounding box, with the BuiltInCategory still at `+0x12`.

The marker's last `u16` is the `Global/ElemTable` header constant (RE-31 addendum, #301) plus 40:

| release | header constant | marker tail | marker | frames with a BuiltInCategory at `+0x12` |
|---|---:|---|---|---:|
| 2023 | 1370 | `0x0582` = 1370 + 40 | `3c 01 FF FF FF FF 82 05` | 0 |
| 2024 | 1411 | `0x05ab` = 1411 + 40 | `46 01 FF FF FF FF ab 05` | 23,641 (Core Interior) |
| 2025 | 1451 | `0x05d3` = 1451 + 40 | `59 01 FF FF FF FF d3 05` | 1,219 (RE1 Architecture) |

The 2023 row was a prediction made from the 2024 and 2025 rows and then checked. The predicted tail is present (181 times on Einhoven, 372 on `modelo_bim`), but no 2023 frame has a category at `+0x12`, so the 2023 prologue is laid out differently and stays unsupported.

The prologue constant at `+0x0c` follows the same pattern: `0x059f` = 1411 + 28 on 2024 and `0x05c7` = 1451 + 28 on 2025. The placement kind (`0xffffef7f` placed) and the container field (`+0x32`) are unchanged. The first `u16` of the marker (`0x013c`, `0x0146`, `0x0159`) has no rule yet. It is taken from measurement, which is why 2026 stays unsupported until a 2026 project is measured. The tail rule predicts `0x05f1` for 2026.

## 2. The 2024 decode on 2025, against Revit's own exports

The RE-21 rule was applied unchanged: the id at `+0x00` must be declared in ElemTable, there must be no container, and the placement kind must be "placed". It was scored against each export's `Tag` set, counting `IfcWall` and `IfcWallStandardCase` for walls.

| file | walls TP/FP/FN | doors | windows | floors TP/FP/FN | second-prologue frames |
|---|---|---|---|---|---:|
| RE1-Architecture | **7 / 0 / 1** | 0 / 0 / 5 | – | **2 / 0 / 0** | 18 |
| RE1-Electrical, -Mechanical, -Plumbing | – | – | – | – | 0 |
| Projeto1 | 0 / 0 / 1 | 0 / 0 / 1 | – | – | 22 |
| teste_export_2025 | 0 / 0 / 3 | – | – | 0 / 0 / 1 | 29 |
| **all six** | **7 / 0 / 5** | 0 / 0 / 6 | 0 / 0 / 0 | **2 / 0 / 1** | 69 |

There are **no false positives on any file.** Every miss corresponds to a frame in the second prologue (no ElementId at `+0x00`, RE-30), which the decode skips and the export diagnostics now count.

End to end, `rvt-ifc` on RE1 Architecture writes:
- the same 7 wall ElementIds and 2 slab ElementIds as Revit's export;
- 11 `IfcSpace`, against the export's 11;
- 20 elements with bodies;
- `element_record_without_element_id` = Door 17, Wall 1.

`tests/element_records_2025.rs` pins these numbers.

## 3. What does not carry over

- **Room names and numbers, storeys, IFC export-type overrides, wall types.** These come from record families RE-22…RE-29 measured on 2024 only. Their modules keep their own 2024 gate, so on 2025 rooms are `Room-<id>` and elements are not storey-bound.
- **Doors and windows.** On these files they are almost entirely second-prologue frames. The #295 selector question applies to 2025 exactly as it does to Snowdon.
- **2023 and 2026.** Unsupported, as above.

## 4. Change

- `partition_element_records::bbox_marker(release)` returns the measured marker for 2024 and 2025 and `None` otherwise.
- `supports_revit_version` now covers `[2024, 2025]`.
- The rf-level scans pick the marker by release, and `_with_marker` variants of `decode_at`, `find_category_records` and `count_unattributed_frames` take one explicitly.
- The existing functions keep the 2024 marker, so callers are unaffected.
