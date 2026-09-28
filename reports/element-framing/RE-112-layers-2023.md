# RE-112 — Revit 2023 host types keep their layers, and walls and roofs are typed

**Date:** 2026-09-28
**Issues:** #421 (Revit 2023); follows RE-53, RE-57, RE-63, RE-110 and RE-111
**Artefacts (local only, no licence):** `255ribeiro/intro_ifc` `Exemplo_data.rvt` and `modelo_bim.rvt` (Revit 2023), each with Revit's own IFC4 export.
**Result:** a Revit 2023 wall, floor or roof type keeps its layers in its element data as 2024's does, in 29-byte records with `u32` ids. All five host types of the two projects read Revit's own layers.

With the layers read, RE-63 names the walls Basic Walls and the roof a Basic Roof, so every element Revit's export types is now typed by an IFC type object with Revit's type `Tag`:

| | Exemplo_data | modelo_bim |
|---|---:|---:|
| typed with Revit's type `Tag` | 37 of 37 (RE-111: 20) | 37 of 37 (RE-111: 32) |
| with Revit's type GlobalId | 18: all walls and the floor (RE-111: 1) | 10: all walls, the floor, the roof and the footings (RE-111: 5) |

Every 2024 and 2025 file is byte-identical: Core Interior, Snowdon Towers Architectural, RE1 Architecture, Projeto1 and teste_export_2025.

-----

## 1. The layers

The count follows the type's name (RE-111), framed as on 2024:
- on a wall type by `ff ff ff ff 6f 10`;
- on a floor or roof type by the name and `u32 0`.

Then one 29-byte record per layer, exterior or top first (`partition_compound_structure::layer_layout`):

```text
+0   f64  width, feet
+8   u32  function (1 structure, 200 structural deck, as on 2024)
+12  u32  not read (ff × 4 on a structure layer, 0 on the deck)
+16  u32  material ElementId, ff × 4 = the category's
+20  u32  deck profile ElementId, ff × 4 = none
+24  5 bytes not read
```

2024's record is 37 bytes with `u64` ids and the function at `+24`. The 29-byte length is fixed by the one two-layer type: its second layer's width and function sit 29 bytes after the first's.

A material names itself in its own element data, read as a type's name is (RE-111). Against the layer sets Revit's export writes for the same types:

| type | read | Revit's export |
|---|---|---|
| 398 `Genérico - 200 mm` (wall) | 0.2000 m, the category's material | an `IfcMaterial`, the category's default |
| 339 `Genérico 150 mm` (floor) | 0.1500 m, the category's material | an `IfcMaterial`, the category's default |
| 400 `Genérico - 140 mm Alvenaria` (wall) | 0.1400 m, 415 `Unidades de alvenaria de concreto` | the same layer set |
| 113426 `Concreto de 160 mm com deck de metal de 50 mm` (floor) | 0.2100 m, 523 `Concreto, Moldado in loco`; 0.0000 m structural deck, 529 `Deck de metal` | the same layer set |
| 335 `Genérico - 400 mm` (roof) | 0.4000 m, the category's material | 0.4 m, `Telhado padrão`, the category's |

- **Zero-width decks.** A structural deck layer can have no width: this one's deck profile is carried by the concrete layer above it, and Revit writes it at 0.0 m. The reader now accepts a zero width on a deck layer as well as on a membrane. That changes no 2024 or 2025 export.
- **Materials.** 2023 has no material records to check a layer's material against (a material has no bounding box, so no RE-81 record), so on 2023 the material only has to be declared in `Global/ElemTable`.

## 2. What is written

**Types.**
- Walls take the `Basic Wall` system family and the roof `Basic Roof` (RE-63), so each is named `Family:Type:ElementId` and typed by an `IfcWallType` or `IfcRoofType` (RE-110) with Revit's type `Tag` and GlobalId.
- Revit's Portuguese export names the families `Parede básica` and `Telhado básico`, as RE-63 records for the Portuguese 2025 files.
- modelo_bim's roof is an `IfcRoof` in rvt-rs and four `IfcSlab` pieces in Revit's export, so its type entity differs.

**Layers.** They are not yet written as material layer sets on 2023: no `IfcMaterialLayerSet` is written for a 2023 element. The export warning says so.

## 3. End-to-end measurement

| file | sha256 |
|---|---|
| Exemplo_data.rvt | `c647c521262860bfa765fefda0d4c7542c9164a389d6af1c08b3d31d142588e2` |
| Exemplo_data.ifc | `42fdb8b0b540ebb993064e347ddf4a3e50d329f5cd8f45d0b6c73ee7a7da8c4a` |
| modelo_bim.rvt | `e06107f54481f408995ddf8f66093f1e4c4bc576ac0a3ff186edc4d32e21ff62` |
| modelo_bim.ifc | `58a6aa2872a818a43ed383e833a3a88ec768ef20ef75658bcbd9f7e846fab47f` |

```bash
cargo run --profile ci --example probe_re112_layers_2023 -- modelo_bim.rvt
./target/ci/rvt-ifc modelo_bim.rvt -o modelo.ifc
python3 tools/re/type_objects_vs_ifc.py modelo.ifc modelo_bim.ifc
```
