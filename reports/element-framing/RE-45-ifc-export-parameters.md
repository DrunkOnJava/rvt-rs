# RE-45 — IFC export overrides are parameter entries in the element's data

**Date:** 2026-09-23
**Result:**
- **Positive.** Revit's four IFC export parameters are entries `i64 BuiltInParameter · u32 n · n UTF-16 units` in an element's serialised data:
  - an instance's "Export to IFC As" and "IFC Predefined Type";
  - a type's "Export Type to IFC As" and "Type IFC Predefined Type".
- Each entry belongs to the element whose element-data header (RE-44) is the last one before it. This holds on Revit 2024 and 2025.
- rvt-rs now exports an element as the entity and predefined type its own parameters name, or else those its type's name.
- On Core Interior, every element rvt-rs writes now has Revit's IFC entity, and the two floors Revit writes as `IFCSLAB … .ROOF.` are written that way. On `teste_export_2025`, the three walls whose type is exported as `IfcCoveringType` / `CLADDING` are `IFCCOVERING … .CLADDING.`, as in Revit's export.
- It supersedes RE-22's owner-offset read of the same values. That read gave the same 21 `IfcShadingDevice` owners on Core but only worked on 2024, only for instance values, and only for values starting with `Ifc` in that case.

**Probe:** `examples/probe_re45_ifc_export_parameters.rs` lists every element carrying an export parameter, with its values.

**Oracles:** `2024_Core_Interior.rvt` with `2024_Core_Interior_slim.ifc`, and `teste_export_2025.rvt` (local only) with its export. Controls, where nothing may change: RE1 Architecture, Snowdon Towers Architectural and Structural, Einhoven and Projeto1.

-----

## 1. The entries

The keys are Autodesk's public `BuiltInParameter` values, read from McNeel's MIT-licensed rhino.inside-revit `ParameterId` listing:

| key | BuiltInParameter | user-facing name |
|---:|---|---|
| −1019014 | `IFC_EXPORT_ELEMENT_AS` | Export to IFC As |
| −1019015 | `IFC_EXPORT_ELEMENT_TYPE_AS` | Export Type to IFC As |
| −1019016 | `IFC_EXPORT_PREDEFINEDTYPE` | IFC Predefined Type |
| −1019017 | `IFC_EXPORT_PREDEFINEDTYPE_TYPE` | Type IFC Predefined Type |

Each entry is one of a counted list (`u32 count`, then that many entries) of string parameters in the element's data. Positive keys in the same list are project-parameter elements. Core Interior floor 20912:

```text
04 00 00 00                                   count 4
78 73 f0 ff ff ff ff ff  04 00 00 00  "ROOF"  IFC_EXPORT_PREDEFINEDTYPE
7a 73 f0 ff ff ff ff ff  07 00 00 00  "ifcSlab" IFC_EXPORT_ELEMENT_AS
d8 43 00 00 00 00 00 00  07 00 00 00  …       a project parameter (17368)
```

`teste_export_2025` wall type 402 carries a list of 2: `IFC_EXPORT_PREDEFINEDTYPE_TYPE = "CLADDING"` and `IFC_EXPORT_ELEMENT_TYPE_AS = "IfcCoveringType"`.

The same keys occur outside element data. Parameter-definition elements on Snowdon, RE1 and the ribeiro files carry `−1019015` / `−1019017` followed by a group string such as `autodesk.parameter.group:materials-1.0.0`. A value is therefore accepted only in the right shape:
- An "export as" value must be `Ifc` (any case) followed by ASCII letters and digits.
- A predefined type must be ASCII letters, digits and `_`.

## 2. The owner

An entry belongs to the element whose element-data header (`ff ff ff ff · u16 · 01 00 00 00 · u64 id`, RE-44) is the last one before it. The owner must be declared in `Global/ElemTable`. The furthest entry measured is `0x1ac` bytes past its header, and the scan looks back at most `0x800`. A key an element sets to two different values, in one stream or across streams, is dropped.

On Core the owners of `IfcShadingDevice` are the same 21 ids RE-22's owner-offset read gives (the owning id `u64` 220 and 286 bytes ahead of the value). That read is kept for API compatibility but no longer feeds the export. Twelve more entries on Core sit in loaded families' own documents, after the partition record chain (RE-35). Their ids are not declared, and they are not read.

## 3. Resolving an element

- The entity is the element's own "Export to IFC As", else its type's "Export Type to IFC As". The type is RE-38's family type or #322's system type. A type value names an IFC type entity, and the trailing `Type` is dropped: `IfcCoveringType` is `IfcCovering`.
- The predefined type is the element's own, else its type's.
- `ifc::category_map::lookup_export_override` honours only entities a reference export demonstrates: `IfcShadingDevice` (RE-22), and now `IfcSlab` and `IfcCovering`. Another value leaves the element on its class mapping.
- A predefined type is written only when it is an enumerator of the entity the element exports as, taken from the IFC4 schema (`IfcSlabTypeEnum`, `IfcCoveringTypeEnum`, `IfcShadingDeviceTypeEnum`, `IfcRoofTypeEnum`). Otherwise the mapping's default stays.

## 4. Measured

Per Tag, the entity (with `…StandardCase` folded into its supertype) and the `PredefinedType` rvt-rs writes, against Revit's export:

| file | before | after |
|---|---|---|
| Core Interior | 2 slabs `.FLOOR.` where Revit writes `.ROOF.` (20912, 70325) | every element rvt-rs writes has Revit's entity. The two slabs are `.ROOF.`, and the 20 shading devices are unchanged |
| `teste_export_2025` | 3 walls `IFCWALL` where Revit writes `IFCCOVERING … .CLADDING.` (325556, 328146, 328303) | all three `IFCCOVERING … .CLADDING.` |
| RE1 Architecture, Snowdon Towers, Einhoven, Projeto1 | — | none carries an export parameter, so nothing changes. The Snowdon Architectural and RE1 Architecture exports were compared byte for byte |

Core Interior's other overrides are on elements Revit does not export:
- 16884 carries `ifcSlab` / `ROOF`;
- 16925 carries `IfcShadingDevice`;
- 16874 carries `IfcSpace`;
- 16994 carries `ifcFooting` with predefined type `Sample`, which is not an enumerator.

`IfcSpace` and `IfcFooting` therefore stay unproven targets.

The committed Core witness observations are unchanged, because they count entity types and no Core element changes entity.

## 5. What this does not do

- It does not read Revit's own category-to-entity mapping table, which an export setup can change. The class mappings in `category_map` stay what they were.
- It does not write `IfcSlabType` / `IfcCoveringType` entities. Revit writes the type with the same predefined type, but rvt-rs writes no types for these classes.
- Values other than the three proven entities are carried on the element (`m_ifc_export_as`) but not acted on.
