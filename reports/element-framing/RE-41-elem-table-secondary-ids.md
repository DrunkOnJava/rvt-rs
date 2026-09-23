# RE-41 — The ElemTable's second id is the element's ElementId where the two differ

**Date:** 2026-09-23
**Result:**
- **Positive.** A 40-byte `Global/ElemTable` record carries two `u32` ids, at `+16` (`id_primary`) and `+36` (`id_secondary`). rvt-rs declared only `id_primary`.
- Where the two differ, `id_secondary` is the id of the element's partition record (RE-35) and the `Tag` Revit's own IFC export writes. `id_primary` is neither.
- rvt-rs now declares both (`elem_table::declared_ids`). On Snowdon Towers Architectural this attributes all 37 element records that were left unattributed (16 walls, 4 columns, 17 generic models), and every one of them is in Revit's export. `element_record_without_element_id` is now absent on every file measured.
- No licensed file in CI has a record whose ids differ, and their exports are byte-identical.

**Oracles:**
- Snowdon Towers Sample Architectural and Structural (`bldrs-ai/test-models`, local only, `.rvt` sha256 `33271010…` for the architectural model) and Revit's IFC4 export of the architectural one (`ecfcb04e…`).
- The VIM export of the Snowdon models (`vimaec/vim-hackathon`, local only) for the structural sample.
- Core Interior, the four RE1 models, `teste_export_2025` and Projeto1 for the files where the ids agree.

**Probe:** `examples/probe_re41_elem_table_secondary_ids.rs` prints how many partition chain records each id set covers, and every record whose ids differ.

-----

## 1. How it was found

RE-40 §5: the 33 elements of the primary design option 1500724 on Snowdon are in Revit's export but were counted as `element_record_without_element_id`. They are second-prologue frames inside chain records 2220523 to 2220557, and those ids are not in the declared set, so RE-35 does not assign them.

The ElemTable has as many records (47,233) as the architectural model's partition chains. 85 chain ids are not any record's `id_primary`, and 85 `id_primary` values are not chain ids. Pairing them by record shows the table does hold the chain ids, one field later:

| record | `id_primary` (`+16`) | `id_secondary` (`+36`) |
|---|---:|---:|
| `+0x14763e` | 2120306 | **2120398** |
| `+0x147666` | 2120309 | **2120399** |
| `+0x14a816` | 2135933 | **2136635** |
| `+0x16a74e` | 2184087 | **2219686** |

## 2. Across the corpus

`probe_re41_elem_table_secondary_ids`:

| file | records | ids differ | chain ids not `id_primary` | not `id_secondary` | not declared now |
|---|---:|---:|---:|---:|---:|
| Snowdon Architectural (2024) | 47,233 | 84 | 85 | 1 | 1 |
| Snowdon Structural (2024) | 16,571 | 27 | 20 | 0 | 0 |
| Core Interior (2024) | 26,425 | 0 | 633 | 633 | 633 |
| RE1 Architecture (2025) | 4,668 | 0 | 1 | 1 | 1 |
| RE1 Electrical (2025) | 10,266 | 0 | 1,611 | 1,611 | 1,611 |
| RE1 Mechanical, Plumbing (2025) | 4,743, 4,827 | 0 | 0 | 0 | 0 |
| `teste_export_2025`, Projeto1 (2025) | 5,590, 5,512 | 0 | 1 | 1 | 1 |

On every record where the ids differ, `id_secondary` is a chain id. On the 28-byte layout (Revit 2016 to 2023: Einhoven, `Exemplo_data`, `modelo_bim`) no record's ids differ. Each family file has one record whose ids differ, and it is the same one: record 18, with `id_secondary` 5151 (2016) or 0 (2024).

The chain ids no id covers on Core Interior and RE1 Electrical (633 and 1,611), and the one on the other files, are the same with either id, and this finding does not change them.

## 3. Against Revit's export

Of the 84 architectural records whose ids differ, 39 have their `id_secondary` as an element or type `Tag` in Revit's IFC4 export: 17 proxies, 16 walls, 4 columns and 2 column types. No `id_primary` is a `Tag`.

`elem_table::declared_ids` keeps every `id_primary`, so nothing declared before is dropped. On the 40-byte layout it adds `id_secondary` when it is not `0`. Every one of the 13 production call sites that built the set from `id_primary` now uses it.

Snowdon Architectural export, against Revit's IFC4 export by `Tag`:

| | RE-40 | RE-41 |
|---|---:|---:|
| exported elements | 6,007 | **6,044** |
| in Revit's export | 5,907 | **5,944** |
| not in Revit's export | 100 | 100 |
| Revit's elements of an exported entity that rvt-rs misses | 38 | **1** |
| `element_record_without_element_id` | 37 | **absent** |
| `confidence.unexported_element_records` | 37 | **0** |

All 1,078 walls, 118 columns and 994 proxies of Revit's export are now exported. The one Revit element still missing is a stair run with no readable placed-instance frame (RE-39 §2). The 100 not in Revit's export are the #309 type omissions (9 doors, 38 windows, 47 curtain panels) and 6 slabs with no design option.

On Snowdon Structural the change adds one element, floor 670566. The VIM export holds it as an `OST_Floors` "NW Concrete on Metal Deck". The other structural secondary id the VIM holds is 709888, a `W8x31` column type, not a placed element.

Core Interior, RE1 Architecture and RE1 Electrical export byte-identically apart from the header and owner-history timestamps.

## 4. What this does not claim

- What `id_primary` holds where the two differ is not established. Some are ids of other chain records, most are not ids of any record.
- The chain ids that neither id covers on Core Interior and RE1 Electrical are unchanged.
- Measured on Revit 2024 files. No 2025 file has a record whose ids differ.
