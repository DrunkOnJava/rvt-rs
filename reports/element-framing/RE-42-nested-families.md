# RE-42 — Telling a type's family from the families nested in it

**Date:** 2026-09-23
**Result:**
- **Positive.** RE-38 names an element `Family:Type:ElementId` when its type's partition record holds exactly one other named id of the type's category. A type whose family nests other families of the same category names those too, so RE-38 left it unnamed (#324).
- Among those candidates, the family is the one whose own partition record names every other candidate: the families nested in it. `partition_names::resolve_family` now picks it when exactly one candidate does.
- Every name the rule adds equals Revit's own:
  - RE1 Electrical's 12 lighting fixtures (none named before);
  - 6 duct fittings on RE1 Mechanical;
  - 117 elements on Snowdon Towers Architectural;
  - 50 on Snowdon Towers Structural, against the VIM export.
- Closes #324.

**Oracles:** Revit's own IFC exports of the RE1 models (`Drshelden/IFC-ECS`, MIT, CI tier 2), of Snowdon Towers Architectural (local only), of `teste_export_2025` and Projeto1 (local only), and the VIM export of Snowdon Towers Structural (local only).

**Probe:** `examples/probe_re42_nested_families.rs` prints, per file, how each named type's family candidates relate to its `Global/ElemTable` owner, and the rule's pick for every type with several candidates. `tests/element_names.rs` now covers RE1 Electrical.

-----

## 1. The candidates

#324's cases on RE1 Electrical:

| type | candidates | the rule picks | Revit's family |
|---|---|---|---|
| 439260 `6"` | 51892 Recessed Fixture, 756708 265100-LIGHT-CAN | 756708 | 265100-LIGHT-CAN |
| 444100 `24"x24"` | 51892 Recessed Fixture, 755868 265100-LIGHT-LED | 755868 | 265100-LIGHT-LED |
| 476009 `120V Duplex` | 475198 Receptacle Duplex, 51955 Recessed Back Box, 681952 262726-RECEPT | 681952 | 262726-RECEPT |
| 507239 `120V Single Pole` | 506469 SWITCH, 851975 260923-SWITCH | 851975 | 260923-SWITCH |

756708's own record names 51892, and 681952's names 475192, 475195, 475198, 51955 and a symbol family. The nested families' records do not name their hosts.

## 2. The ElemTable owner agrees where it is set

The type's `Global/ElemTable` owner (RE-31) is set on some types. On the types with several candidates, whenever the owner is one of them it is the family the rule picks: 4 on RE1 Electrical, 9 on Snowdon Architectural, 7 on Structural, 2 each on `teste_export_2025` and Projeto1.

It is not a rule of its own. It is unset on most types. On every file it also points outside the candidates on some single-candidate types (59 on Snowdon Architectural), where RE-38's family is Revit's.

## 3. Against Revit's names

| file | names written | equal to Revit's (or the VIM's) | new with this rule |
|---|---:|---:|---:|
| RE1 Electrical | 12 | **12** | 12 |
| RE1 Mechanical | 42 | **42** | 6 |
| RE1 Architecture, Plumbing | 57, 61 | 56, 60 (plus one each Revit omits) | 0 |
| Core Interior | 394 | **394** | 0 |
| Snowdon Architectural | 3,902 | **3,831** (the other 71 Revit omits) | 117 |
| Snowdon Structural (VIM) | 1,184 | 1,132 of the 1,156 in the VIM | 50, all equal |

The Snowdon additions are families that nest others: counter tops with sink holes, kitchenettes, trees, dining tables with chairs, bollard lights, a canopy and a chandelier.

The 24 structural names that differ from the VIM are one type, renamed in the VIM's later edition (RE-38 §4). A comparison must decode STEP's doubled apostrophe (`''`) before comparing.

Counted over the elements Revit's export also holds, rvt-rs now writes 4,397 names identical to Revit's: Core Interior 394, Snowdon Architectural 3,831, RE1 Architecture 56, Mechanical 42, Plumbing 60, Electrical 12, `teste_export_2025` 1, Projeto1 1. That is up from RE-38's 4,241, partly through RE-41's newly attributed elements.

## 4. What this does not claim

- A family type names its family the same way a family names a nested one. A type record that also named a sibling type of its own family would therefore make the rule pick that type. No such record occurs on the files measured, and no name the rule writes differs from Revit's.
- 83 types across the files resolve through the rule. Some are annotation families (elevation marks) that no exported element uses.
- System-family types are unchanged (#322).
