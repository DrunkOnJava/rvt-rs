# RE-107 — Revit 2023 Levels, and the storeys elements sit on

**Date:** 2026-09-28
**Issues:** #421; follows RE-24, RE-26, RE-51, RE-59, RE-81 and RE-85
**Artefacts (local only, no licence):** `255ribeiro/intro_ifc` `Exemplo_data.rvt` and `modelo_bim.rvt` (Revit 2023), each with Revit's own IFC4 export.
**Result:**
- **The elevation marker.** RE-51's marker is `05 00 00 00` and a per-release `u16`: `0x0248` on 2024, `0x025d` on 2025. That `u16` is the tag of the schema class `Plane` in each file's own schema, `0x0235` on 2023. It is now read from the schema where no release constant is recorded.
- **The Level record.** A Revit 2023 Level record is 2024's without the 24 sentinel bytes after the category, opened by the 32-bit chain header 2023's element records carry (RE-81). Relative to the category:

  | field | position |
  |---|---|
  | `u32` ElementId | 14 bytes before |
  | `ElementHeader` tag | 8 bytes after the id |
  | container reference | 8 bytes after |
  | placement kind | 24 bytes after |
  | record marker (`ff ff ff ff` and the `ElementParents` tag) | 38 bytes after |

- **The name block.** It is 2024's with a 32-bit owner: `u32 owner · 28 × ff · 00 00 00 · u32 n · UTF-16 name`, then the elevation marker and the elevation 55 bytes on.
- **The newest copy.** A Level edited after an earlier save is rewritten into a higher-numbered partition with the earlier copy left in place, as every element is (RE-26 §3). Only its newest partition's name block counts. modelo_bim's Fundação reads −4.0 m in `Partitions/1` and −1.7 m, Revit's, in `Partitions/2`.
- **The storey-set check** now counts distinct Levels, since a Level can be written in more than one partition.
- **Containment.** A 2023 element's record names its Level in its `u32` reference list, as on 2024. It now binds to that storey by RE-27's rule, with RE-59's base constraint and RE-68's rules on top. Of an element's frames, the newest is kept: on modelo_bim four columns are framed from −13.12 ft in `Partitions/1` and from −5.58 ft in `Partitions/2`, and Revit draws them from −1.7 m.

| against Revit's IFC4 | storeys: name and elevation | elements in Revit's storey |
|---|---:|---:|
| Exemplo_data | 2 of 2 (main: 1 derived storey) | 37 of 37 (main: 0) |
| modelo_bim | 4 of 4 (main: 2 derived storeys) | 29 of 37 (main: 0) |

-----

## 1. The Levels

| file | Level | name | elevation |
|---|---:|---|---:|
| Exemplo_data | 311 | Nível 1 | 0 m |
| Exemplo_data | 694 | Nível 2 | 3.15 m |
| modelo_bim | 338608 | Fundação | −1.7 m |
| modelo_bim | 311 | Nível 1 | 0 m |
| modelo_bim | 694 | Nível 2 | 4.0 m |
| modelo_bim | 336724 | Nível 3 | 8.0 m |

Each equals the `Name` and `Elevation` of an `IfcBuildingStorey` in Revit's export. Level 305 on both files is a type record (placement kind `0xffff8000`), not a storey. The storeys' GlobalIds are not Revit's: RE-48's GlobalId reader does not run on 2023.

## 2. What is left

On modelo_bim, 8 of 37 elements are not in Revit's storey (`tools/re/storeys_vs_ifc.py -v`):
- **4 beams (337513, 337530, 337544, 337558).** Each runs from 3.2 to 4.0 m, and its record names only Nível 1, where Revit puts it on Nível 2.
- **4 foundation beams (337632 to 337635).** They run from −0.8 to 0 m and name no Level, so they sit on no storey; Revit puts them on a storey.

`Revit_IFC5_Einhoven.rvt` (2023, no local Revit export) fails closed. Only one of its Level records is found, and its Roof Level's block has no elevation marker within reach, so its storeys stay as before. Its elements now carry the Level their records name (`LevelBindSource` `partition_element_record_reference_list`), as 2024's do.

## 3. Other files

Every 2024 and 2025 file is byte-identical to main's apart from the timestamp: Core Interior, the four RE1 models, Snowdon Towers and its structural model, Projeto1, `teste_export_2025` and the MIT house (2024, 2025). No witness observation changes.

## 4. End-to-end measurement

| file | sha256 |
|---|---|
| Exemplo_data.rvt | `c647c521262860bfa765fefda0d4c7542c9164a389d6af1c08b3d31d142588e2` |
| Exemplo_data.ifc | `42fdb8b0b540ebb993064e347ddf4a3e50d329f5cd8f45d0b6c73ee7a7da8c4a` |
| modelo_bim.rvt | `e06107f54481f408995ddf8f66093f1e4c4bc576ac0a3ff186edc4d32e21ff62` |
| modelo_bim.ifc | `58a6aa2872a818a43ed383e833a3a88ec768ef20ef75658bcbd9f7e846fab47f` |

```bash
./target/ci/rvt-ifc Exemplo_data.rvt -o exemplo.ifc
python3 tools/re/storeys_vs_ifc.py -v exemplo.ifc Exemplo_data.ifc
./target/ci/rvt-ifc modelo_bim.rvt -o modelo.ifc
python3 tools/re/storeys_vs_ifc.py -v modelo.ifc modelo_bim.ifc
```
