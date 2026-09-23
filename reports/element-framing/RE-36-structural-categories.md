# RE-36 — Structural framing, columns, foundations and generic models from element records

**Date:** 2026-09-23
**Result:**
- **Positive.** Four more `BuiltInCategory` values decode under the unchanged RE-21 instance rule, with ElementIds from RE-35's enclosing records:
  - `OST_StructuralFraming` −2001320 exports as `IfcBeam` `.BEAM.`;
  - `OST_StructuralColumns` −2001330 as `IfcColumn` `.COLUMN.`;
  - `OST_StructuralFoundation` −2001300 as `IfcFooting`;
  - `OST_GenericModel` −2000151 as `IfcBuildingElementProxy`.
- **Structural sample:** 1,253 of the 1,281 elements exported are in the VIM export under the same category, with **none under another category**. Every framing member, column, foundation and generic model the two editions share is recovered. The other 28 are beam-system members that the VIM's later edition regenerated with new ids (§3).
- **Architectural sample:** 260 of 261 generic models are in Revit's own IFC4 export. The other is in a non-primary design option.
- **No change** on Core Interior, the four RE1 models or the two `255ribeiro` projects: none of them has records of these categories.

**Oracles:**

| file | Revit | reference | licence | sha256 (`.rvt`) |
|---|---|---|---|---|
| Snowdon Towers Sample Structural | 2024 | `vimaec/vim-hackathon` `vims/Snowdon.r2027.vim` (commit `50f4373`, sha256 `3e7de636…`), document "Snowdon Towers Sample Structural" | none (local only) | `6dc833f9…` |
| Snowdon Towers Sample Architectural (`bldrs-ai/test-models`) | 2024 | Revit 24.0.20.20 IFC4 export | none (local only) | `33271010…` |

A VIM file (`vimaec/vim-format`, MIT) is a BFAST container. Its `Vim.Element` table lists each element's Revit ElementId, category and source document, so it is an oracle for a model that has no Revit IFC export. Nothing from it is committed.

**Probe:** `examples/probe_re36_vim_oracle.rs` parses the VIM and scores every placed-instance element record of a model against one of its documents.

-----

## 1. The census

`probe_re36_vim_oracle` over the Structural sample (first prologue: `+0x00` id; inferred: RE-35 enclosing record):

| category | frames | first | enclosing record | same category in the VIM | absent from the VIM | other category |
|---|---:|---:|---:|---:|---:|---:|
| `OST_StructuralFraming` | 942 | 0 | 942 | **914** | 28 | **0** |
| `OST_StructuralFoundation` | 90 | 0 | 90 | **90** | 0 | **0** |
| `OST_StructuralColumns` | 74 | 0 | 74 | **74** | 0 | **0** |
| `OST_GenericModel` | 91 | 0 | 91 | **91** | 0 | **0** |
| `OST_Walls` | 58 | 8 | 50 | 58 | 0 | 0 |
| `OST_Floors` | 27 | 3 | 23 | 26 | 0 | 0 |
| `OST_Rebar` | 14 | 0 | 14 | 14 | 0 | 0 |
| `OST_StructConnections` | 70 | 0 | 70 | 56 | 14 | 0 |
| `OST_StructuralFramingSystem` | 76 | 0 | 76 | 75 | 1 | 0 |

In total 1,403 frames are in the VIM under the same category and none under another. The absent rows of the non-model categories (sketch lines, area-scheme lines, lines) are elements a VIM does not carry.

Nearly every frame of these categories uses the second prologue, so they only became reachable with RE-35. Under RE-34's inference the same census had 67 columns and 85 foundations, not 74 and 90.

## 2. What exports

| file | entity | exported | in the reference | not in it |
|---|---|---:|---:|---:|
| Structural (VIM) | `IfcBeam` | 942 | 914 | 28 |
| | `IfcColumn` | 74 | 74 | 0 |
| | `IfcFooting` | 90 | 90 | 0 |
| | `IfcBuildingElementProxy` | 91 | 91 | 0 |
| | `IfcWall`, `IfcSlab` | 84 | 84 | 0 |
| Architectural (Revit IFC4) | `IfcBuildingElementProxy` (generic models) | 261 | 260 | 1 |

`OST_StructuralFoundation` covers isolated footings (77), wall foundations (9) and foundation slabs (4); all 90 export as `IfcFooting`. The four slabs own sketches, the case RE-34 could never assign; they take their records' ids like everything else.

On the architectural sample the VIM identifies 270 of Revit's exported proxies as generic models. rvt-rs exports 253 of them. The other 17 are counted as `element_record_without_element_id`, which is now 37 there: 16 walls, 4 columns, 17 generic models.

## 3. The VIM is a later edition

`Snowdon.r2027.vim` was made from Autodesk's 2027 edition of the sample, not from this 2024 file. 347 of its 1,860 structural elements are not declared in the 2024 ElemTable, among them 45 framing members and 252 connection handlers.

The 28 exported framing ids the VIM lacks are 2024 elements the later edition no longer has:
- 24 of them name a beam system in their reference list, and a beam system regenerates its members with new ids when it is edited;
- they come in runs of three consecutive ids (702678–702680, 797199–797201, …), one run per system.

The 14 connection ids absent from the VIM are two runs of seven (793206–793212, 798950–798956), and the VIM has 14 connection ids the 2024 file does not declare. On this oracle "absent" is therefore edition drift, not a wrong id. The evidence for the ids is the zero other-category column: a wrong id would land on some other element's id, and none does.

## 4. Entity choices

- **Framing.** The VIM's `Structural Usage` parameter gives 606 joists, 315 girders, 25 purlins and 13 other, and no braces, so `IfcBeam` is the entity on this file. `.BEAM.` is `category_map`'s choice. Revit's own `PredefinedType` for joists and girders is not measured (there is no Revit IFC export of this sample). A brace would still export as `IfcBeam`: the record's structural type is not decoded.
- **Foundations** export as `IfcFooting` with no `PredefinedType`, whether footing, strip or slab.
- **Rebar, structural connections and beam systems** have right ids (§1) but are not exported. A bounding box is the wrong body for a rebar set or a connection plate, and a beam system is an assembly of the framing it owns.

## 5. What this does not claim

- Bodies are the record bounding box. For a sloped or curved beam that is an envelope, not the member.
- Structural analytical models, rebar shapes, connection geometry and the framing's structural usage are not decoded.
- The Structural sample, the VIM and the architectural IFC4 are measured locally and never committed. CI covers these categories through the unit tests and through the unchanged RE1 and Core exports.
- Revit 2024 and 2025 only.
