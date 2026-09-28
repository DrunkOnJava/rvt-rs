# RE-110 — Elements are typed by IFC type objects of their Revit type

**Date:** 2026-09-28
**Issues:** #322 (type names), #421 (Revit 2023); follows RE-28, RE-38, RE-44, RE-48, RE-63, RE-108 and RE-109
**Result:** rvt-rs now writes the IFC4 type layer that Revit's own export writes.

Every element that rvt-rs names `Family:Type` and whose Revit type it reads is typed through `IfcRelDefinesByType` by one type object per Revit type:
- the IFC4 type entity of its occurrence (`IfcWallType`, `IfcDoorType`, `IfcColumnType`, …);
- named `Family:Type`;
- its `Tag` the type's ElementId;
- its `PredefinedType` the occurrence's;
- its GlobalId the one Revit's UniqueId gives the type (RE-48, RE-108).

Before RE-110 no type object was written for any release, though RE-28, RE-38 and RE-44 already joined elements to their types.

The table covers every element Revit's export types that rvt-rs also exports, keyed by `Tag`:

| file | release | Revit types | typed by rvt-rs | same name | same Tag | same entity | same GlobalId |
|---|---|---:|---:|---:|---:|---:|---:|
| 2024_Core_Interior | 2024 | 854 | 854 | 854 | 854 | 854 | 438 |
| Snowdon Towers Architectural (local only) | 2024 | 5,812 | 5,803 | 5,803 | 5,341 | 5,662 | 2,816 |
| RE1-Architecture | 2025 | 73 | 72 | 72 | 70 | IFC2X3 export | 36 |
| teste_export_2025 (local only) | 2025 | 8 | 8 | 1 | 7 | 8 | 6 |
| Projeto1 (local only) | 2025 | 2 | 2 | 1 | 2 | 2 | 1 |
| Exemplo_data (local only) | 2023 | 37 | 19 | 19 | 19 | 19 | 0 |
| modelo_bim (local only) | 2023 | 37 | 31 | 31 | 31 | 27 | 4 |

Every file passes `tools/ci/ifc_schema_arity.py`: every type entity is written with every attribute IFC4 declares.

-----

## 1. What is written

Types are grouped by the pair (type ElementId, type entity). Each group becomes one type object and one `IfcRelDefinesByType` listing its instances.

- **Type entity.**
  - An occurrence's type entity is its IFC4 `…Type`.
  - `IfcDoorType` and `IfcWindowType` take `.NOTDEFINED.` operation and partitioning types.
  - `IfcFurnitureType` takes `.NOTDEFINED.` assembly place.
  - Occurrences outside the table in `type_entity_for` (openings, spaces, reinforcing bars) are not typed.
  - Railings are also left untyped. Revit's Snowdon export types none of its 131 railings, and RE1's types its one.
- **PredefinedType.** In IFC4 an occurrence and its type share one enumeration, so the occurrence's validated value is written, or `.NOTDEFINED.`.
- **Name.** `Family:Type`, from the same `FamilyName` and `TypeName` properties the occurrence's `ObjectType` is written from. A type object and its instances therefore always agree.
- **Tag and GlobalId.**
  - The Tag is the type's ElementId.
  - The GlobalId is the type's creating-episode GUID XOR its ElementId, from `Global/ElemTable` as for every element (RE-48; RE-108 on 2023).
  - A type written under two type entities keeps Revit's GlobalId on the first. The second gets one derived from the document (#400), so no GlobalId repeats.

The 2023 walls, slabs and roofs stayed untyped here because their system-family type names were not read (RE-109). Since RE-111 the floors are typed; walls and roofs still lack their system family. The modelo_bim footing types are `IfcFootingType` because rvt-rs writes those occurrences as `IfcFooting` where Revit writes `IfcSlab`.

## 2. Where Revit differs, measured

**Names.** Every disagreement is RE-63's system-family name: rvt-rs writes it in English, while the two Portuguese 2025 exports write `Parede básica`, `Piso` and `Telhado básico`. This is the same difference those elements' `ObjectType` already had.

**Tags.**
- Snowdon's 462 curtain panels have a type whose `Tag` is the first panel's own ElementId. rvt-rs writes the panel type's.
- teste_export_2025's one wall was exported from a later state of the model (see `element-record-system-type-names`).

**Entities.**
- Revit's Snowdon IFC4 export writes plumbing fixtures as the deprecated `IfcFlowTerminal`, typed by `IfcDistributionElementType`. rvt-rs writes `IfcSanitaryTerminal` and `IfcSanitaryTerminalType`.
- Roof slabs are `IfcRoof` and `IfcRoofType` in rvt-rs, `IfcSlab` and `IfcSlabType` in Revit's.
- RE1's reference is IFC2X3, so its door styles and distribution types have no IFC4 counterpart to agree with.

**GlobalIds.** Revit's exporter writes one type object per Revit type only for some types. Each Revit type GlobalId, expanded to 16 bytes, was XORed with the type's own GlobalId (what rvt-rs writes) and with its first instance's:

| Revit's type objects, by GlobalId | Core | Snowdon | modelo_bim |
|---|---:|---:|---:|
| the type's own | 9 | 209 | 1 |
| one instance's GlobalId with bit `0x08` of byte 10 flipped: a type object per instance | 164 | 108 | 16 |
| any other value | 26 | 699 | 2 |
| total | 199 | 1,016 | 19 of the 22 whose type rvt-rs writes |

- The per-instance split does not follow the category. Core's one column type becomes 164 per-instance type objects and 11 that group several columns, and nothing in the element records measured here tells the two apart. rvt-rs therefore writes one type object per Revit type and does not guess which instances Revit would group.
- The last row includes the loaded-family door and window types. On Core, the GlobalIds of two of them, the door type 17333 and the window type 17335, were searched for in every stream, with the type's ElementId XORed out, whole and by their first 8 bytes, in stored and canonical order. None was found, so the exporter derives them.

## 3. Everything else is unchanged

- **Core witnesses.** Core Interior's IFC gains 14 type objects and 14 relations; its entities are otherwise unchanged. Both committed Core Interior `rvt-rs.json` observations are refreshed. `tools/ci/witness-verdict.py` passes both artifacts with all three replays matching, and `ifc_schema_arity.py --witness-agreement` passes.
- **Other outputs.** The GLB, plan SVG and schedules do not read the type layer and are unchanged.

## 4. End-to-end measurement

| file | sha256 |
|---|---|
| 2024_Core_Interior.rvt | `c805df445d613b408e37337765572021265e3f5dfdc7d1fa53b22ba1600b8014` |
| 2024_Core_Interior_slim.ifc | `bfdf36ffb0bb768f3409d818403990e64d4c262c6780603be87f8077387ad86d` |
| RE1-Architecture.rvt | `4a78bdd68e7f4dd7806060db07c222da9a810e20b3f303c17a443294b91c8ce4` |
| RE1-Architecture.ifc | `a9b5d36677aa6a8bb91b77d8bc354ed9028a7ca3e9e2491a47ba14cfd8e26200` |
| Snowdon Towers Sample Architectural.rvt (local only) | `33271010919acb2a0d0d040851393a511d86ea7fd9a1f4c054797ae03e5e44a1` |
| Snowdon Towers Sample Architectural_IFC4.ifc (local only) | `ecfcb04e3e818090cf818873fe76fba8b447742bbf4d19dad6d590f09cbb985a` |
| Exemplo_data.rvt / .ifc (local only) | `c647c521…588e2` / `42fdb8b0…a8c4a` |
| modelo_bim.rvt / .ifc (local only) | `e06107f5…21ff62` / `58a6aa28…fab47f` |
| Projeto1.rvt / .ifc (local only) | `51cf7850…0248b2` / `95f8fc73…674092` |
| teste_export_2025.rvt / .ifc (local only) | `73ee5cba…427115` / `f98dd81b…b445` |

```bash
./target/ci/rvt-ifc 2024_Core_Interior.rvt -o core.ifc
python3 tools/re/type_objects_vs_ifc.py core.ifc "IFC Exports/2024_Core_Interior_slim.ifc"
python3 tools/ci/ifc_schema_arity.py --witness-agreement core.ifc
```

`-v` prints the first disagreements of each kind with both values.
