# RE-111 — Revit 2023 walls, floors and roofs carry their type

**Date:** 2026-09-28
**Issues:** #421 (Revit 2023), #322 (system-family type names); follows RE-44, RE-63, RE-109 and RE-110
**Artefacts (local only, no licence):** `255ribeiro/intro_ifc` `Exemplo_data.rvt` and `modelo_bim.rvt` (Revit 2023), each with Revit's own IFC4 export.
**Result:** every wall, floor and roof of the two 2023 projects now carries its Revit type's ElementId and name. The floors are typed by an `IfcSlabType` with Revit's own Tag and GlobalId.

A 2023 system-family type has no record and no name entry (RE-109), but its element data names it as 2024's does (#322). Of the ids the element's record names that have such a name, the type is the most specific one.

| | Exemplo_data | modelo_bim |
|---|---:|---:|
| elements Revit's export types, with Revit's type name as `TypeName` | 37 of 37 (main: 19) | 37 of 37 (main: 31) |
| of them walls, floors and roofs | 17 walls, 1 floor (main: 0) | 4 walls, 1 floor, 1 roof (main: 0) |
| typed by an IFC type object with Revit's type `Tag` | 20 of 37 (main: 19) | 32 of 37 (main: 31) |

Every 2024 and 2025 file is byte-identical: Core Interior, Snowdon Towers Architectural, RE1 Architecture, Projeto1 and teste_export_2025.

-----

## 1. The type's name

**The header.** A 2023 element's data opens with `ff ff ff ff c0 02 01 00 00 00` and a `u32` ElementId. 2024 and 2025 open with a `u16` of `0x02d3` and `0x02ef` and a `u64` id. After the id come four bytes that are not zero: `ff ff ff ff` on every type measured, an ElementId on walls (`partition_names::ELEMENT_DATA_HEADER_2023`).

**The name.** A system-family type's name is the first framed string after that: `ff ff ff ff`, a `u16` tag, a `u32` length in UTF-16 units, the name. The tag is `0x0fd0` for the wall types and `0x013f` for the floor and roof types:

| type | name |
|---|---|
| 398 | `Genérico - 200 mm` |
| 339 | `Genérico 150 mm` |
| 400 | `Genérico - 140 mm Alvenaria` |
| 113426 | `Concreto de 160 mm com deck de metal de 50 mm` |
| 335 | `Genérico - 400 mm` |

**Skipping nested headers.** Another element's data header nested in the window reads as a frame of length 1 whose "name" is its ElementId's low bytes. The reader now skips a frame that is the data header itself, on every release. It cannot skip every length-1 frame: RE1's types are named `-`, and skipping them changed RE1's export.

## 2. The type

**Candidates.** The type is one of the ids the element's record names that have no record of their own, no name entry and a name as above. On both files Revit's type is always one of them. The others are named by records of more categories:

| element | its type, named by records of | the other candidates |
|---|---|---|
| a wall | 398 or 400: walls, doors, windows | 86961 (`Insira aqui o novo nome`, "enter the new name here"): eight categories, rooms included |
| modelo_bim's floor | 113426: floors | 99880 (`Recobrimento do vergalhão 1`, a rebar cover setting): floors, structural columns, framing and foundations; and 86961 |
| modelo_bim's roof | 335: roofs | 86961 |
| Exemplo_data's floor | 339: floors | 86961 |

**The rule.** The type is the candidate whose naming categories are a strict subset of every other candidate's (`partition_schema_mvp::system_type_names_2023`). Where none is, the element gets no type. The rule is measured on these 24 elements of two files. On both it picks Revit's type every time and never another.

## 3. What is written

**TypeName.** Walls, floors and roofs now carry `TypeName` equal to the type half of Revit's type name: all 24 elements, as the table shows.

**Family and type object.**
- The floors take RE-63's system family, `Floor`, so they are named `Floor:<type>:<ElementId>` and typed by an `IfcSlabType` (RE-110). Its Tag and GlobalId are Revit's on both files.
- Revit's Portuguese export names the family `Piso`, as RE-63 records for the 2025 Portuguese files.
- A wall's, roof's or ceiling's system family follows from whether its type has layers (RE-63), which 2023 does not read. So walls and roofs keep their category's name and no type object: Exemplo_data's 17 walls, and modelo_bim's 4 walls and its roof.

## 4. End-to-end measurement

| file | sha256 |
|---|---|
| Exemplo_data.rvt | `c647c521262860bfa765fefda0d4c7542c9164a389d6af1c08b3d31d142588e2` |
| Exemplo_data.ifc | `42fdb8b0b540ebb993064e347ddf4a3e50d329f5cd8f45d0b6c73ee7a7da8c4a` |
| modelo_bim.rvt | `e06107f54481f408995ddf8f66093f1e4c4bc576ac0a3ff186edc4d32e21ff62` |
| modelo_bim.ifc | `58a6aa2872a818a43ed383e833a3a88ec768ef20ef75658bcbd9f7e846fab47f` |

```bash
cargo run --profile ci --example probe_re111_system_types_2023 -- Exemplo_data.rvt
./target/ci/rvt-ifc Exemplo_data.rvt -o exemplo.ifc
python3 tools/re/type_objects_vs_ifc.py exemplo.ifc Exemplo_data.ifc
```

The scorer's `TypeName carried` line is the table's first row.
