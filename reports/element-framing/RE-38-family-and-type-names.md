# RE-38 — Family and type names from partition name entries

**Date:** 2026-09-23
**Result:**
- **Positive.** Revit 2024 and 2025 partitions carry a name entry for every loaded family and family type: `u64 ElementId · u32 n · n UTF-16 code units · i64 BuiltInCategory`.
- An element's type is the one id in its reference list with an entry of the element's own category. The type's partition record (RE-35) names its family the same way.
- `Family:Type:ElementId` is how Revit's own IFC export names an element, with `Family:Type` as its `ObjectType`. rvt-rs now writes both, and on every element where it does they are **exactly** Revit's: 4,241 of 4,241 across Core Interior, Snowdon Towers, the RE1 models and `teste_export_2025`.
- **Scope:** family instances (doors, windows, columns, furniture, fixtures, mullions, panels, equipment…). System families (walls, floors, roofs, ceilings, stairs, railings, curtain walls) have types that are not in these entries and keep the `Class-ElementId` name.

**Oracles:**
- Revit's own IFC exports: Core Interior (`2024_Core_Interior_slim.ifc`, magnetar, MIT), the RE1 models (`Drshelden/IFC-ECS`, MIT), Snowdon Towers Architectural IFC4 (local only), and `255ribeiro` Projeto1 / `teste_export_2025` (local only).
- The VIM export of Snowdon Towers (`vimaec/vim-hackathon`, local only) for the name table itself.

**Probe:** `examples/probe_re38_names.rs` reports the name entries and how many placed instances resolve a type and a family. `tests/element_names.rs` checks every `Family:Type:ElementId` name against Revit's export on Core Interior and the RE1 models.

-----

## 1. How it was found

#309 left 47 curtain panels on Snowdon that Revit's export omits, all of type 21944, which the VIM names "Empty". The type's own partition record holds no string.

A UTF-16 search of the partition finds "Empty" at `0x7957b7d` of `Partitions/68`, framed as:

```text
b8 55 00 00 00 00 00 00   u64 21944
05 00 00 00               u32 5
45 00 6d 00 70 00 74 00 79 00   "Empty"
d6 7a e1 ff ff ff ff ff   i64 -2000170 (OST_CurtainWallPanels)
```

## 2. The name table

`partition_names::find_name_entries` anchors on the closing category: the top five bytes of any id in the `BuiltInCategory` band are `0xff`. It then looks back for a length that matches and a declared ElementId. Names with control characters or unpaired surrogates are rejected, and an id whose entries disagree is dropped.

| file | ids with an entry | also in the VIM | same name and category as the VIM |
|---|---:|---:|---:|
| Snowdon Architectural | 1,104 | 348 (198 family types, 132 families, 13 mullion types, 5 panel types) | **348** |
| Snowdon Structural | 732 | 57 | **57** |
| Core Interior | 73 | – | – |

The ids the VIM lacks are types and families that are loaded but not used, which a VIM does not carry. No id carries two different names.

## 3. Type and family

For an element record:
- **Type:** the reference-list ids (other than its own) that carry an entry of the record's category. Exactly one is the type.
- **Family:** the ids of the same category that appear anywhere in the type's partition record body, other than the type. Exactly one is the family.

On Snowdon the 21944 record's slot list is `[21943, 21944, 28431]`, and 21943 is "Empty System Panel".

| file | placed instances | resolve a type | resolve family and type |
|---|---:|---:|---:|
| Core Interior | 4,874 | 617 | 617 |
| Snowdon Architectural | 14,405 | 5,579 | 4,881 |
| RE1 Architecture | 142 | 57 | 57 |

## 4. Against Revit's export

rvt-rs writes `Family:Type:ElementId` as the IFC `Name` and `Family:Type` as the `ObjectType` of every exported element where both joins resolve. It also adds `FamilyName` and `TypeName` to the element-record property set. The `ObjectType` equals Revit's wherever the `Name` does.

| file | names written | equal to Revit's `Name` for the same `Tag` | different |
|---|---:|---:|---:|
| Core Interior | 394 (132 doors, 6 windows, 256 columns) | **394** | 0 |
| Snowdon Architectural | 3,765 | **3,693** | 0 |
| RE1 Architecture | 57 | **56** | 0 |
| RE1 Mechanical | 36 | **36** | 0 |
| RE1 Plumbing | 61 | **60** | 0 |
| teste_export_2025 | 1 | **1** | 0 |
| Projeto1 | 1 | **1** | 0 |

Snowdon's other 72, and one each on RE1 Architecture and Plumbing, are elements Revit's export leaves out (RE-35 §5).

On the Snowdon structural sample, which has no Revit IFC export, 1,099 named elements are in the VIM export. For 1,075 the name is exactly the VIM's `FamilyName:Name`. The other 24 are one type the VIM's later edition renamed: this file stores `1'0"x1'-7"x4"` and the VIM `1'0"x1'-9"x4"`. RE1 Electrical's 12 lighting fixtures resolve no unique family and keep the `Class-ElementId` name; RE-42 resolves them, since their types' records also name the family nested in the fixture family. Projeto1's door name, "M_Folha única:0915 x 2134mm:325405", is equal once the STEP escapes are decoded: rvt-rs writes `ú` as `\X2\00FA\X0\` and Revit as `\X\FA`.

## 5. What this does not claim

- System-family types (wall, floor, roof, ceiling, stair and railing types) are not in these entries, and their elements keep the `Class-ElementId` name.
- Names are as stored: a localised template gives localised names.
- Revit 2024 and 2025 only.
- The curtain panels of #309 are not filtered by their type's name. A name is language-dependent, and no byte seen so far marks an empty panel.
