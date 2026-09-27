# RE-76 — Element records name their class through the schema's definition ordinal

**Date:** 2026-09-27
**Issues:** #154, #223 (builds on #410)
**Credit:** STE1200 (Discussion #112: the schema grammar and the reading of `+0x4a` as `m_classDef`), jakobhirn-bit (Discussion #112: `+0x46` as a flag word), rosejn (#255: `schema_registry.rs` numbers classes the same way).

**Result:** positive.

- `Formats/Latest`, read whole and page-stripped (#410), parses to its last byte under a grammar: records separated by a zero `u16`; a class is a `u16`-counted name, a `u16` base reference (bit 15: the base is defined inline next), a `u32` version, a `u32` field count, the fields (a `u32`-counted name and a type) and a `u32`-counted list of 16-byte GUIDs. A type is a kind byte and a modifier byte (high nibble `0x10`: a `u32` array count follows; kind `0x0d`: `01 00 00 00 20` and an inner type; kind `0x0e` by value: a class reference). A class reference with bit 15 set is followed by that class's definition.
- **A class's tag is not stored.** It is the definition ordinal: 12 for the first class and one more for each further definition, top-level or inline. Every inline reference repeats its tag in its low 15 bits, so a misread stops the parse instead of shifting later tags. The `u16` after a class name, which rvt-rs has read as the class tag since Q4, is the base-class reference.
- **An element record's `u16` at `+0x4a` is its class under those tags.**

| file | release | classes (tags) | stopped |
|---|---|---|---|
| rac_basic_sample_family-2016 | 2016 | 3,792 (12..3803) | no, whole stream |
| 2024_Core_Interior, Snowdon Towers Architectural | 2024 | 4,492 (12..4503) | no |
| RE1-Architecture | 2025 | 4,600 (12..4611) | no |
| racbasicsamplefamily-2026 | 2026 | 4,690 (12..4701) | no |

Every element record of the ten categories below resolves to a class on Core Interior, RE1 Architecture and Snowdon Towers; none is left unresolved (`examples/probe_re76_schema_class_tags.rs`):

| category | Core Interior | Snowdon Towers | RE1 |
|---|---|---|---|
| OST_Walls | SWall 1,487, DirectShapeType 1 | SWall 1,080, ArcWall 32, FaceWall 16, FamilyInstance 1 | SWall 8 |
| OST_Floors | Floor 146 | Floor 191 | Floor 2 |
| OST_Rooms | RoomElem 399 | RoomElem 54 | RoomElem 13 |
| OST_Roofs | | ProfileRoof 20 | |
| OST_Ceilings | | Ceiling 68 | Ceiling 6 |
| OST_Stairs | | StairsElement 26, FamilyInstance 1 | |
| OST_Doors, OST_Windows, OST_Columns, OST_Furniture | FamilyInstance (placed), FamilySymbol (type symbols) | FamilyInstance | FamilyInstance |

- **Independent check.** RE-75 found that 32 of Snowdon Towers' walls store an arc as their location curve, 24 of them in Revit's export and 8 in non-primary design options. The tag gives exactly 32 `ArcWall` records, 24 of them exported. The class and the geometry were decoded from different bytes.
- **Families.** Door, window and column records split into `FamilyInstance` for placed instances and `FamilySymbol` for their types, the distinction RE-21 drew from `+0x32` and `+0x42`.

## What changes

- `formats::schema_classes` reads the schema by the grammar and gives every class its tag; `RevitFile::schema_classes` reads it from a file.
- `PartitionElementRecord::class_tag` carries `+0x4a`.
- Every record-backed element gets `RevitClass` in its `RvtElementRecordGeometry` property set: Core Interior 970 of 970, RE1 Architecture 85 of 85, Snowdon Towers 6,081 of 6,081. The viewer's element panel shows it with the other properties.

## Not claimed

- The `u16` after a class name is still what `SchemaTable`'s `tag` field and `class_tag_map` (the 2016-2026 tag-drift table) carry, and the `Global/Latest` walker still matches on it. Moving them to the definition ordinal changes the walker's input and is left to #154.
- `+0x46` is recorded as a flag word; its bits are not interpreted.
