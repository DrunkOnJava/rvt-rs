# RE-44 — System-family type names from the type's own element data

**Date:** 2026-09-23
**Issue:** #322
**Result:**
- **Positive.** A wall, floor, ceiling, roof or railing type has no RE-38 name entry, but its serialised element data names it.
- The data opens with `ff ff ff ff · u16 · 01 00 00 00` and the type's `u64` ElementId. The type's name is the first framed string after that.
- The `u16` is `0x02d3` on Revit 2024 and `0x02ef` on Revit 2025.
- Joined to its elements through the RE-28 type records, every name written equals the type half of Revit's `ObjectType` for the same `Tag`: 460 on Core Interior, 1,545 on Snowdon Towers Architectural, 16 on RE1 Architecture, and 6 of 7 on `teste_export_2025`.
- The one difference on `teste_export_2025` is the reference export, not the reader: that wall's frame, data and bounding box all describe the other type (§5).
- The RE-28 type records, which were measured on Revit 2024 only, have the same shape on Revit 2025 with the 2025 marker and prologue constant.

**Probe:** `examples/probe_re44_system_type_names.rs` lists every type-definition record of the system categories and the name its data gives.

**Oracles:**
- `2024_Core_Interior.rvt` (sha256 `c805df44…`) with `2024_Core_Interior_slim.ifc`.
- `RE1-Architecture.rvt` (Revit 2025, MIT) with its IFC2x3 export.
- Snowdon Towers Sample Architectural (local only, `.rvt` sha256 `33271010…`) with Revit's IFC4 export (`ecfcb04e…`).
- `teste_export_2025.rvt` and `Projeto1.rvt` (local only) with their exports.

-----

## 1. Where the name is

RE-38's name table covers loadable families and their types. System-family types (the types of walls, floors, roofs, ceilings, railings, ramps and curtain walls) are not in it. Their names were the last gap in Revit's `Name` / `ObjectType` pair.

An element's serialised data in a partition opens with:

```text
ff ff ff ff   u16 header word   01 00 00 00   u64 ElementId
```

After the id come unset slots (`0xff` runs) and fields, each framed as `ff ff ff ff · u16 tag · …`. For a system-family type, the first frame that holds a string is its name:

```text
ff ff ff ff   u16 tag (never 0xffff)   u32 n   n UTF-16LE code units
```

| release | header word | wall-type name tag | floor / pad / ceiling name tag |
|---|---|---|---|
| 2024 | `0x02d3` | `0x1002` | `0x0151` |
| 2025 | `0x02ef` | `0x106a` | `0x015c` |

The header word is not derived from the release constant the way the bbox marker is (RE-32). Each release is therefore listed in `partition_names::element_data_header`, and any other release reads nothing.

Before the name frame, 2025 data holds frames such as `ff ff ff ff 3c 10 ff ff ff ff`, whose length word is `0xffffffff`. A length outside `1..=NAME_MAX_UNITS`, a control character or a U+FFFD / U+FFFE / U+FFFF code unit rejects that frame, and the search moves on. An id whose copies in different streams give different names is dropped.

## 2. Which type an element has

The type comes from the RE-28 join: the one type-definition record (placement kind `0xffff8080`) of the element's own category that the element's reference list names. RE-28 measured that on 2024 walls, and it holds for every category below.

Two facts were needed beyond RE-28:

- **Second-prologue type records.** On Snowdon and on RE1 most type records hold no ElementId at `+0x00` (RE-30). They take the id of the partition record they sit in (RE-35), as element frames do, and the record's slot list must name that id. The second prologue does not carry the prologue constant at `+0x0c`, so it is not checked there.
- **Revit 2025.** The type record's marker is the last six bytes of the release's bbox marker (`ff ff ff ff d3 05`). Its prologue constant is `partition_element_records::record_prologue_constant` (`0x05c7`), the same constant RE-35's record chain uses. RE1 Architecture's wall, floor and ceiling types are all second-prologue records inside the chain.

## 3. The family is not in the file

Revit's `ObjectType` for a wall is `Basic Wall:<type>`. "Basic Wall" is not stored anywhere in the file: the Portuguese `teste_export_2025` export writes `Parede básica:<type>` for the same system family. Revit generates that name in the UI language, so rvt-rs does not invent it.

A system-family element therefore gains `TypeName` in its property set and no `FamilyName`. Its IFC `Name` stays `Class-ElementId` and its `ObjectType` stays unset, because Revit's would include the family.

## 4. Measured

For every element that carries a `TypeName` and no `FamilyName`, the name is compared with the part after the first `:` of Revit's `ObjectType` for the same `Tag`:

| file | release | equal | different | elements Revit does not export |
|---|---|---:|---:|---:|
| Core Interior | 2024 | 460 (360 walls, 79 IfcSlab, 20 IfcShadingDevice, the pad) | 0 | 0 |
| Snowdon Towers Architectural | 2024 | 1,545 (1,119 walls, 176 slabs, 101 railings, 68 ceilings, 59 proxies, 20 roofs, 2 ramps) | 0 | 6 slabs |
| RE1 Architecture | 2025 | 16 (7 walls, 1 curtain wall, 2 floors, 6 ceilings) | 0 | 0 |
| `teste_export_2025` | 2025 | 6 | 1 (§5) | 0 |
| `Projeto1` | 2025 | 1 | 0 | 0 |

RE1's two floor types are named `-` and `--`. The export gives floor 418660 `-` and floor 418772 `--`, and rvt-rs gives each the same, so the join does not merely land on a shared name.

The Core Interior export was diffed against the one the released 0.2.0 `rvt-ifc` writes. It has 460 more `IfcPropertySingleValue` entities (27,226 → 27,686 entities), and the property sets list them. Every other entity is unchanged, apart from the owner-history timestamp and 14 relationship GlobalIds, which the writer numbers by position.

### Types left unnamed

| file | types | why |
|---|---|---|
| Core Interior | walls 1711, 11356, 17339; roof 1719; railing 1743 | no element-data block; none is placed |
| RE1 Architecture | railing type 446543 | no element-data block anywhere in the file, under any header word |
| Snowdon Architectural | 7 of 22 railing types | no element-data block |

RE1's one railing, 462556, names 446543 in its reference list and so stays without a `TypeName`, which is fail-closed. Its data probably has a form not yet read.

## 5. `teste_export_2025` wall 327589

Revit's export types wall 327589 `Genérico - 300 mm` (`IfcWallType` Tag 6291). rvt-rs reads `Genérico - 200 mm` (Tag 398). Three independent reads of the `.rvt` all say 398:

- Its frame's reference list is `[311, 398, 694, 86961, 327589, 328146]`.
- Its serialised data names 398 at `+0x98`. The data is byte-for-byte that of wall 327728, which Revit exports as `Genérico - 200 mm`, apart from the id.
- Its record bounding box differs from Revit's IFC geometry by 42 mm and 27 mm in plan, while walls 325344 and 327728 agree to 0.1 mm.

The IFC was exported from a state of the model in which this wall had been given the thicker type. It does not describe this `.rvt`, so the difference is reported and not counted as a decoder miss.

## 6. What this does not do

- It writes no `IfcWallType` / `IfcSlabType` entity and no `IfcRelDefinesByType`. RE-28 records the type set as a superset of what Revit exports, and Revit's type `Name` includes the family (§3).
- It reads only the name, and no other field of the type's data: compound layers, thickness and function stay unread.
- Revit 2023 and earlier read nothing (no measured header).

## 7. Follow-ups found on the way

- On `teste_export_2025`, Revit exports walls 325556, 328146 and 328303 as `IfcCovering`, while rvt-rs writes `IfcWall`. All three are of type `Interior - 79 mm Divisória (1-hr)`. A per-type "export as" override is a likely carrier and is not read yet.
- Relationship GlobalIds are numbered by their position in the file, so adding entities before them changes them. Element GlobalIds are stable.
- rvt-rs writes non-ASCII STEP strings as raw UTF-8, while Revit writes `\X\` and `\X2\` escapes. Readers accept both. Whether ISO 10303-21 allows raw UTF-8 in a file that declares no encoding is checked separately.
