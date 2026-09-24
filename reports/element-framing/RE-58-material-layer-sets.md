# RE-58 — Material names, and layer sets in the IFC export

**Date:** 2026-09-23
**Result:**
- A material's name is read from its own object, by ElementId. Revit 2024 and 2025 materials are named as follows:

| file | release | materials named |
|---|---|---:|
| Snowdon Towers (local only) | 2024 | 219 of 220 |
| 2024_Core_Interior | 2024 | 78 of 86 |
| RE1 Architecture | 2025 | 69 of 69 |
| the MIT tutorial house (local only) | 2024 and 2025 | 171 of 171 in each |
| Projeto1 (local only) | 2025 | 114 of 114 |
| teste_export_2025 (local only) | 2025 | 114 of 114 |

- Every named material that Revit's own IFC export styles under that name has Revit's colour for it: 174 of 174 across the five files with an export, none different.
- The IFC export now writes the layers of walls, floors, roofs and ceilings as `IfcMaterialLayerSetUsage` over an `IfcMaterialLayerSet`. Each layer has its material and thickness, and the usage places the set on the element's body.
- Scored end to end against Revit's own exports (`tools/re/layer_sets_vs_ifc.py`):

| file | layer sets written | layer names Revit's, in Revit's order | whole sets within 0.001 ft of Revit's body |
|---|---:|---:|---:|
| Snowdon Towers | 1,216 | 3,321 of 3,322 | 1,196 of 1,207 |
| 2024_Core_Interior | 88 | 89 of 89 | 88 of 88 |
| RE1 Architecture | 15 | 37 of 37 | 15 of 15 |

- The 11 Snowdon sets beyond 0.001 ft are RE-53's soffit walls, whose stud layer Revit cuts back around a beam. Their gypsum layers match.
- On the MIT house, the 53 sets rvt-rs writes from the 2024 save and from the 2025 save are identical, and all 150 of their layers are named.
- The viewer's element panel lists an element's layers, with the order, each layer's material and thickness, and the total.

**Probe:** `examples/probe_re58_material_names.rs` prints how many of a file's materials are named, and every material a type's layers use, with its colour and name.

**End-to-end check:** `tools/re/layer_sets_vs_ifc.py` reads the IFC `rvt-ifc` writes and Revit's export of the same file. For each element with a layer set, it:
- compares layer names with Revit's layer or constituent names;
- places each layer from the usage, and compares it with the faces Revit styles with that material;
- compares the whole set with Revit's body;
- where Revit writes its own usage, compares that too.

-----

## 1. Where a material's name is

A material's object is `01 00 00 00 · u64 id` with its class tag at +0x47 (RE-53). Its name is in one of three places. `partition_materials::name_at` takes the first it finds:

1. **Its own name field.** After the class tag come two to four zero `u32`s, then `u32 n` and `n` UTF-16 units. When a BuiltInParameter id follows `n` instead, `n` counts parameter entries, and this is not the name.
2. **After parameter entries.** A material that carries parameter values keeps them in that field before its name. The entries come in more than one kind:
   - strings: `i64 id · u32 n · UTF-16 × n`;
   - integers: `i64 id · u32 value`.

   After them come up to `ff`×8 and some zero bytes, then the name. On every such material measured on Snowdon, RE1 and the MIT house, the name ends with `ff ff ff ff` and a release tag: `17 0c` on 2024 and `6b 0c` on 2025. So rvt-rs finds the first such terminator past the class tag. It takes the name only when exactly one `u32 n · UTF-16 × n` ends right before it. Many materials carry no such terminator; rules 1 and 3 name those.
3. **A framed string** (`ff ff ff ff · 49 01` on 2024, `5c 01` on 2025) **or a name parameter entry** (`i64 -1001203 · u32 0 · u32 n · UTF-16 × n`), further on.

`scan_material_names` reads every copy of every declared material. It drops an id whose copies disagree, and a name read for two or more materials, since Revit keeps material names unique.

Where the names come from:
- Rules 1 and 3 alone name Snowdon 187, Core 67, RE1 60, the MIT house 50, Projeto1 114 and teste_export_2025 114.
- Rule 2 adds 32, 11, 9, 121, 0 and 0.
- Rule 2 never disagrees with rules 1 and 3 on a material both name, and no name read by rules 1 and 3 changes.
- On the MIT house, rule 2 names Autodesk's template materials that the house's layers use ("Masonry - Brick", "Wood - Sheathing - Plywood", "Finishes - Exterior - Siding / Clapboard").
- On RE1, it names the three 2025 floor finishes: "Tile 300 x 300 Stacked", "Red Oak 75mm Running" and "Sleepers". In those, 8 integer and string entries come before the name, which sits at +0xf8 (#355).

Unnamed:
- Snowdon 1924356. Its field holds a texture path and parameter entries, and no terminator follows within the window. It is one layer of the wall type "Exterior - 12 5/8" Rainscreen w Insultation on Metal Stud (Alley)"; Revit's export names that layer "Default Wall".
- Eight of Core Interior's family materials.

Checked against Revit's IFC surface styles by name. A name read for the wrong id would carry another material's colour:

| file | named materials Revit styles | colour equal to Revit's |
|---|---:|---:|
| Snowdon Towers | 136 | 136 |
| 2024_Core_Interior | 9 | 9 |
| RE1 Architecture | 19 | 19 |
| Projeto1 | 3 | 3 |
| teste_export_2025 | 7 | 7 |

Every material Revit's export names is now named on Core Interior and RE1. On Snowdon, two are not: "Bronze, Architectural" and "Sheathing Roof Board".

Exemplo_data and modelo_bim (ribeiro, Revit 2023) name none: the layouts are measured on 2024 and 2025 only.

## 2. The layer sets

`ifc::material_layer_sets_from_layers` builds one `MaterialLayerSet` per type name and layer sequence. It shares the set among the type's elements, and gives each layer its material where the name is read. A material the model does not hold yet is added with its colour. Each element gets a `MaterialLayerSetUsage` in `IfcModel::material_layer_usages`. An element gets no set when it has one layer with no name, since the set would say only what its body says. Core Interior's walls are single layers by category, so they get none.

The usage is written only where the element's body is an extrusion whose extent across its layers equals their sum within 1e-4 ft (`body_geometry::LAYER_THICKNESS_TOLERANCE_FEET`):
- **A floor, roof or ceiling** (layers stacked, RE-57): `AXIS3`, `NEGATIVE`, `OffsetFromReferenceLine` the body's height. The first layer is at the top and they run down.
- **A wall** (RE-53): the local axis its exterior normal runs along, `AXIS1` or `AXIS2`. If the exterior faces that axis's positive side, the sense is `NEGATIVE` from the body's far face. Otherwise it is `POSITIVE` from its near face. Either way the first layer is the exterior one.

A wall whose exterior is not along a local axis gets no usage, and neither does a body that is not such an extrusion:
- RE-56's sloped roof, drawn as a faceted B-rep;
- walls drawn whole (#358).

`step_writer` writes `IFCMATERIALLAYER(material or $, thickness, …)`, `IFCMATERIALLAYERSET(layers, type name, $)` and `IFCMATERIALLAYERSETUSAGE(set, direction, sense, offset, $)`, in metres, associated through `IfcRelAssociatesMaterial`.

| file | usages | sets | layers | layers without a material |
|---|---:|---:|---:|---:|
| Snowdon Towers | 1,216 | 62 | 164 | 1 |
| 2024_Core_Interior | 88 | 4 | 5 | 0 |
| RE1 Architecture | 15 | 4 | 9 | 0 |
| the MIT house (2024; 2025 the same) | 53 | 11 | 32 | 0 |

Revit's own 2025 export of RE1 (Coordination View 2.0) writes 15 usages, 4 sets and 9 layers. They are the same sets, with the same thicknesses in the same order. Revit names each set `Family:Type` ("Basic Wall:-"); rvt-rs writes the type name, since system family names are not read.

## 3. End-to-end measurement

| file | sha256 |
|---|---|
| Snowdon Towers Sample Architectural.rvt | `33271010919acb2a0d0d040851393a511d86ea7fd9a1f4c054797ae03e5e44a1` |
| Snowdon Towers Sample Architectural_IFC4.ifc | `ecfcb04e3e818090cf818873fe76fba8b447742bbf4d19dad6d590f09cbb985a` |
| 2024_Core_Interior.rvt | `c805df445d613b408e37337765572021265e3f5dfdc7d1fa53b22ba1600b8014` |
| IFC Exports/2024_Core_Interior_slim.ifc | `bfdf36ffb0bb768f3409d818403990e64d4c262c6780603be87f8077387ad86d` |
| RE1-Architecture.rvt | `4a78bdd68e7f4dd7806060db07c222da9a810e20b3f303c17a443294b91c8ce4` |
| RE1-Architecture.ifc | `a9b5d36677aa6a8bb91b77d8bc354ed9028a7ca3e9e2491a47ba14cfd8e26200` |
| demo-01-Source_House_2024_tn2.rvt | `763f239e70db1ac90e5aa806ec4e6a2cebdc3e8a2e902bcfda6b8de43d2fdcfb` |
| demo-01-Source_House_2025_tn2.rvt | `28440eaf8b31aa9e91780c75661c3354fa1fad13976bcaf52675c4d11c1a1b6f` |
| Projeto1.rvt | `51cf7850843d21c2c959a792a8bf2b1f019c402d5c8afe282b0bcde36c0248b2` |
| teste_export_2025.rvt | `73ee5cba253535f97afc9d6e2bd13a8c19a60d82a61b5b5c422a4a1173427115` |

```bash
cargo run --profile ci --example probe_re58_material_names -- MODEL.rvt
./target/ci/rvt-ifc MODEL.rvt -o model.ifc
python3 tools/re/layer_sets_vs_ifc.py [-v] model.ifc REVIT_EXPORT.ifc
```

The scorer, with IfcOpenShell 0.8.5, prints for Snowdon:

```text
      3  IfcOpenShell cannot mesh Revit's element
   3321  layer names: Revit's, in Revit's order
      1  layer names: none written
      3  layer sets whose count differs from Revit's
   1216  layer sets written
      6  not in Revit's export
    924  positions compared
    283  positions not comparable
  layer spans within 0.001 ft of Revit's: 913 of 924
  whole layer sets within 0.001 ft of Revit's: 1196 of 1207
```

- **Snowdon.**
  - The 3 sets whose count differs are "Planting Bed" floors, which Revit exports with no material.
  - The 6 not in Revit's export are site floors, one per landscape material, that its export leaves out.
  - Of the 924 elements where a layer's material occurs once, 913 have every such layer within 0.001 ft of the faces Revit styles with it.
  - The other 11 are the "Soffit - Beam Wrap" walls RE-53 lists, off by 0.094 and 0.252 ft on the stud layer.
- **Core Interior.** 88 of 88 whole sets and 89 of 89 layer names match. The sets are its floors' and its shading devices'.
- **RE1.** Every rvt-rs set matches Revit's body, and all 37 layer names are Revit's. Revit also writes its own usages. On its 7 walls those place the layers within 0.001 ft of rvt-rs's. On the other 8, Revit's usage disagrees with Revit's own body:
  - 6 ceilings are placed from their level, 9.19 ft below the ceiling;
  - 2 floors run up from their top face instead of down, which would put the tile under the grout.

  rvt-rs places those layers where Revit's body has them.
- **The MIT house.** The 2024 and 2025 saves give the same 53 sets, usages and names.

IfcOpenShell's validator finds no issue in any of the five exports, and `tools/ci/ifc_schema_arity.py` passes on each. The Core witness observations change with the new materials and usages (IFCMATERIAL 102 to 104). Both verdicts still PASS against IfcOpenShell and IFClite, and `--witness-agreement` still holds.

In the viewer (`rvt_bg.wasm`, no network imports), loading the MIT house and selecting wall 166276 shows "Layers · Exterior - Brick on Mtl. Stud · exterior first":
- Masonry - Brick 0.302 ft;
- Misc. Air Layers - Air Space 0.250 ft;
- Wood - Sheathing - Plywood 0.062 ft;
- Metal - Stud Layer 0.500 ft;
- Finishes - Interior - Gypsum Wall Board 0.042 ft;
- total 1.156 ft, the wall's depth.

Floor 171319 and roof 169797 list theirs top first.

## 4. What this does not do

- Elements whose body is not an extrusion as thick as its layers get no layer set: sloped roofs (RE-56), walls drawn whole (#358), and slabs whose layers do not add up to their box.
- Set names are type names, not Revit's `Family:Type`. Layers carry no name, category or priority.
- Snowdon 1924356 and eight of Core Interior's family materials are unnamed.
- The IFC's material list is still the string heuristic's (#34). It holds far more names than the file has materials: 946 on Snowdon against 220 records, and 686 on the MIT house against 171.
- Only walls, floors, roofs and ceilings get materials. Family instances do not.
