# RE-264 — The field encodings Revit 2027's schema adds

**Date:** 2026-10-29
**Issues:** #1092 (the 2027 sample family in the family corpus), #410 (the whole-schema classification gate)
**Artefacts:** the `phi-ag/rvt` family corpus at bec25470413fbe659c3ab30d4cfd029b73d63fc3, `racbasicsamplefamily-2016.rfa` to `racbasicsamplefamily-2027.rfa`, as Measure fetches it
**Run:** Measure [40949596730](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40949596730) (`examples/probe_re264_schema_field_encodings.rs` on every family of the corpus)
**Result:** positive. Revit 2027's schema gives 325 fields an encoding no earlier release uses: byte 2 of the type header set to 0x02, 0x12 or 0x22. Cleared, each decodes as an encoding 2016 to 2026 already use. `FieldType::Flagged` keeps the flag and that base type. What the flag means for a field's instance bytes is not measured.

## 1. What changed

With the 2027 family added to the corpus, `tests/field_type_coverage.rs` found 334 fields of its schema `FieldType::decode` leaves `Unknown`, where every release from 2016 to 2026 has 9 to 12. The probe prints each field's class, name, variant and encoding bytes for every family.

| Release | Classes | Fields | Unknown before this change |
|---|---:|---:|---:|
| 2016 | 3,490 | 9,932 | 10 |
| 2024 | 4,126 | 11,562 | 9 |
| 2026 | 4,285 | 12,144 | 10 |
| 2027 | 4,348 | 12,687 | 334 |

Of the 334, nine are the residual every release has (`GInfo::m_pFace`, `VarSketchObj::m_params` and the others in `RESIDUAL`). The other 325 are fields the 2026 schema does not list at all, and all 325 share one form: byte 2 is 0x02 (301 fields), 0x12 (15) or 0x22 (9), and byte 3 is 0. No field of 2016 to 2026 has byte 2 in that set, and no 2027 field classified before this change has it.

## 2. With byte 2 cleared

Every one of the 325 then decodes by the rules 2016 to 2026 use:

| Base type | Fields |
|---|---:|
| `bool` (`01 00`) | 77 |
| `u32` (`04 00`, `05 00`) | 44 |
| `f64` (`07 00`), `u64` (`0b 00`) | 8 |
| string (`08 60`) | 7 |
| `vector<f64>` (`07 10`) | 8 |
| `container<u32>` (`04 50`) | 3 |
| reference container (`0e 50`, `0e 51`) | 79 |
| pointer (`0e 01` to `0e 03`) | 40 |
| ElementId or ElementId reference (`0e 00`) | 59 |

Their names read as runtime state rather than saved data: `DBView::m_horzDirTemp`, `DBView::m_oTempFilterOverrides`, `ImageSymbol::m_flags_unsaved`, `DbgDrawnRoomCurvesElem::m_wasCreatedInThisSession`, `GeomTable::m_cacheIsCurrent`, and the members of edit-mode managers (`SurfaceConnectorEditModeMgr`, `AssemblyEditModeMgr`, `RbsLayoutPathMgr`). A likely reading is that Revit 2027 lists transient members in the schema with a flag, where earlier releases left them out, but nothing here measures it.

## 3. What rvt-rs does with them

`FieldType::decode` returns `FieldType::Flagged { flags, base }` for an encoding whose byte 2 is 0x02, 0x12 or 0x22, byte 3 is 0, and which decodes to a known type once byte 2 is cleared; `encode` writes the same bytes back. The walker reads a flagged field as it read an `Unknown` one, forwarding the record's remaining bytes, since whether instance data holds a value for it is not measured; walking 2027 records is therefore unchanged. The coverage gate counts a flagged field as classified, so it passes on all twelve families with the same nine residual names. The 2027 family's partition stream is `Partitions/70` (`streams::partition_for_year`).

Of the 11,750 `Class::field` names both the 2026 and the 2027 schema list, 11,739 keep their variant. The 11 others are names that repeat across classes (`first`, `second`), where the comparison keeps the last, and ElementId fields whose reading depends on the record after them (§4).

## 4. Also seen: an ElementId reference's second word

`FieldType::ElementIdRef` reads a reference as `0e 00 00 00`, the referenced class's tag, and a second u16 it calls `sub`. On the 2024, 2026 and 2027 schemas, where the next field is in the same class, `sub` equals the length of that next field's name on 1,819 of 1,944, 1,932 of 2,066 and 1,966 of 2,102 references, and is 0 on all but one of the rest (2 on one per release). So on most references `sub` is the low half of the next record's `u32` name length: the reference is six bytes, and only some carry two zero bytes after the tag. Nothing in this change depends on it; it is #1101.
