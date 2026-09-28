# RE-108 — Revit 2023 elements take Revit's own GlobalIds

**Date:** 2026-09-28
**Issues:** #421; follows RE-41, RE-48 and RE-107
**Artefacts (local only, no licence):** `255ribeiro/intro_ifc` `Exemplo_data.rvt` and `modelo_bim.rvt` (Revit 2023), each with Revit's own IFC4 export.
**Result:**
- RE-48 rebuilt the GlobalId Revit's exporter gives an element from the file. It takes the GUID of the editing episode that created the element (`Global/History`), with the ElementId XORed into its last 32 bits. The episode number sits at `+0x18` in a 40-byte `Global/ElemTable` record (Revit 2024 and later).
- A 28-byte record (Revit 2023) holds it at `+0x0c`: `u32 · u32 id · u32 second id · u32 episode · …`. The first word is `ffffffff`, the element's own id, or the element it is nested in: a window's trims name the window.
- `revit_global_ids` now reads both layouts. On Revit 2023 every element, room and storey rvt-rs exports takes Revit's own GlobalId.

| against Revit's IFC4 (`tools/re/global_ids_vs_ifc.py`) | elements | storeys | rooms |
|---|---:|---:|---:|
| Exemplo_data | 37 of 37 (main: 0) | 2 of 2 | 9 of 9 |
| modelo_bim | 37 of 37 (main: 0) | 4 of 4 | none in the file |

-----

## 1. Finding the field

For each element of the two exports, the episode whose GUID gives Revit's GlobalId was searched for in `Global/History` (643 episodes on Exemplo_data). Then the episode number was looked for in the element's 28-byte record.
- **Where it matched:** it sits at `+0x0c` on every element whose GlobalId an episode gives: 26 of 26 at first count on Exemplo_data.
- **Where it did not:** those were `IfcOpeningElement`s. They carry their door's or window's Tag but a GlobalId of Revit's own making, and rvt-rs does not write openings as separate elements with Revit's ids.

Leaving openings out:
- Exemplo_data: 45 of 45 elements' GlobalIds follow from `+0x0c`;
- modelo_bim: all of them. Roof 338653 is written by Revit as an `IfcRoof` and four `IfcSlab` parts that share its Tag, each with a GlobalId of its own. rvt-rs writes the roof with the `IfcRoof`'s.

## 2. Other files

Every 2024 and 2025 file is byte-identical to RE-107's output apart from the timestamp. `Revit_IFC5_Einhoven.rvt`, the third 2023 file, has no Revit export to score against; its elements now carry GlobalIds by the same rule.

## 3. End-to-end measurement

| file | sha256 |
|---|---|
| Exemplo_data.rvt | `c647c521262860bfa765fefda0d4c7542c9164a389d6af1c08b3d31d142588e2` |
| Exemplo_data.ifc | `42fdb8b0b540ebb993064e347ddf4a3e50d329f5cd8f45d0b6c73ee7a7da8c4a` |
| modelo_bim.rvt | `e06107f54481f408995ddf8f66093f1e4c4bc576ac0a3ff186edc4d32e21ff62` |
| modelo_bim.ifc | `58a6aa2872a818a43ed383e833a3a88ec768ef20ef75658bcbd9f7e846fab47f` |

```bash
./target/ci/rvt-ifc Exemplo_data.rvt -o exemplo.ifc
python3 tools/re/global_ids_vs_ifc.py exemplo.ifc Exemplo_data.ifc
./target/ci/rvt-ifc modelo_bim.rvt -o modelo.ifc
python3 tools/re/global_ids_vs_ifc.py modelo.ifc modelo_bim.ifc
```
