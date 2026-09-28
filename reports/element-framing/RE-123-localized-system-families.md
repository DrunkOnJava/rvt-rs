# RE-123 — System families are named in the file's saved locale

**Date:** 2026-09-28
**Issues:** follows RE-61 and RE-63 (system family names)
**Artefacts:** `255ribeiro/intro_ifc` `Exemplo_data.rvt`, `modelo_bim.rvt` (Revit 2023), `Projeto1.rvt` and `teste_export_2025.rvt` (Revit 2025), each with Revit's own IFC export; local only, no licence.
**Result:** positive. Revit's export names a system family in the language of the locale the file was saved in, and the name is not in the file. rvt-rs now gives the three names measured in Brazilian Portuguese.

## 1. The observation

`system_family` (RE-63) derives a system family's name from its element's class, in English, as Revit's export of every `ENU` file names it. All four files of the ribeiro set are saved in `PTB` (BasicFileInfo's "Locale when saved"). Their exports name every wall `Parede básica`, every floor `Piso` and the one roof `Telhado básico`:

| file | walls | floors | roofs |
|---|---:|---:|---:|
| Exemplo_data | 17 | 1 | 0 |
| modelo_bim | 4 | 1 | 1 |
| Projeto1 | 1 | 0 | 0 |
| teste_export_2025 | 6 | 1 | 0 |

teste_export_2025's walls are three `IfcWall` and three `IfcCovering`.

Neither `Parede básica` nor `Basic Wall` occurs in modelo_bim's streams, as UTF-16 or UTF-8. Revit supplies the name when it exports, and the saved locale is the file's only record of the language.

## 2. The change

`partition_schema_mvp::localized_system_family` maps `Basic Wall`, `Floor` and `Basic Roof` to those names when the saved locale is `PTB`. Every other family (Curtain Wall, Pad, Railing, …) and every other locale keeps the English name: none occurs in these files, so no other translation is measured. Material layer sets, which Revit names `Family:Type` (RE-61), now take the element's own family name, so they change with it.

## 3. End-to-end measurement

`tools/re/names_vs_ifc.py`, elements by Tag:

| | family name as Revit's |
|---|---:|
| before | 0 of 32 |
| RE-123 | 32 of 32 |

The layer sets rvt-rs writes are named as Revit's on all four files. One teste_export_2025 wall still differs in its type (`Genérico - 200 mm` against the export's `300 mm`), RE-44's wall in a later state.

Core Interior, RE1 Architecture, Snowdon Towers Architectural and Einhoven (all `ENU`) are byte-identical but for the export timestamp.

| file | sha256 |
|---|---|
| Exemplo_data.rvt | `c647c521262860bfa765fefda0d4c7542c9164a389d6af1c08b3d31d142588e2` |
| modelo_bim.rvt | `e06107f54481f408995ddf8f66093f1e4c4bc576ac0a3ff186edc4d32e21ff62` |
| Projeto1.rvt | `51cf7850843d21c2c959a792a8bf2b1f019c402d5c8afe282b0bcde36c0248b2` |
| teste_export_2025.rvt | `73ee5cba253535f97afc9d6e2bd13a8c19a60d82a61b5b5c422a4a1173427115` |

```bash
./target/ci/rvt-ifc modelo_bim.rvt -o modelo.ifc
python3 tools/re/names_vs_ifc.py modelo.ifc modelo_bim.ifc
```
