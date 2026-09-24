# RE-59 — Elements naming two Levels are contained in their base constraint

**Date:** 2026-09-23
**Result:**
- A wall, column, stair or curtain wall whose element record names two Levels now takes the storey of its base constraint.
- RE-27 bound an element to a storey only when its record names exactly one Level. A record naming two, a base and a top constraint, bound nothing, and those elements fell back to matching their box base against storey elevations. That fallback misses:
  - walls with a base offset;
  - walls RE-54 draws from their centreline;
  - curtain walls.
- The base constraint is the higher of the two Levels at or below the record's base, else the lower one. It equals the storey Revit's own IFC export contains the element in on every record that names two Levels:

| file | records naming two Levels, in Revit's export | base constraint = Revit's storey |
|---|---:|---:|
| Snowdon Towers (2024, local only) | 650 | 650 |
| 2024_Core_Interior (2024) | 482 | 482 |
| RE1 Architecture (2025) | 7 | 7 |

- Storeys in rvt-rs's IFC against Revit's, per element (`tools/re/storeys_vs_ifc.py`):

| file | same storey as Revit's, before → after | on no storey, before → after | a different storey, before → after |
|---|---:|---:|---:|
| Snowdon Towers | 3,975 → 5,612 | 1,931 → 294 | 45 → 45 |
| 2024_Core_Interior | 853 → 854 | 1 → 0 | 0 → 0 |
| RE1 Architecture | 27 → 34 | 7 → 0 | 0 → 0 |
| the MIT tutorial house (2024 and 2025, local only) | no oracle | 230 → 1 | no oracle |

- On the MIT house, 230 elements were on no storey (#365): 44 walls, 5 curtain walls with their 177 panels and mullions, 2 columns, a stair and a railing. Only the railing, which names no Level, remains.

**Probe:** `examples/probe_re59_base_constraint_levels.rs` prints every element record with the Levels it names and its box base and top.

**End-to-end check:** `tools/re/storeys_vs_ifc.py` compares each element's storey in rvt-rs's IFC with the one Revit's export gives it, matched by `Tag`. A part aggregated under a whole takes the whole's storey.

-----

## 1. What a record's two Levels are

The counted reference list at +0x88 (RE-23, RE-27) names the Levels that constrain the element. A wall, column or stair names its base and top constraint in no fixed order: the lower Level comes first on 452 of Snowdon's 650 two-Level records and second on the other 198, and first on 194 of Core Interior's 493 and second on 299. RE-27 therefore declined every record naming two.

Revit's IFC export contains such an element in its base constraint. That is the lower Level, except where the element's base sits on the upper one:
- Snowdon's specialty equipment 2029960 names R3 (68.83 ft) and Parapet 2 (70.83 ft), and its box base is 70.83 ft. Revit contains it in Parapet 2.
- So the rule is: the higher of the two whose elevation is at or below the record's base (within 1e-3 ft), else the lower.

"The lower Level" alone gives 649 of 650 on Snowdon. "At or below the base" alone leaves 78 unresolved, the walls whose base offset puts them below their base Level. Together they give 650 of 650.

The rule declines two Levels at one elevation, since nothing tells the base from the top there. A record naming three or more stays unresolved: 3 Snowdon stairs name L3, L4 and L5, and Revit puts them in L4.

## 2. The change

- `element_record_level_refs`:
  - `named_levels` lists the distinct Levels a record names;
  - `base_constraint_level` applies the rule.
- `partition_schema_mvp::element_record_decoded` keeps a two-Level record's pair in `m_constraintLevelIds`.
- `resolve_base_constraint_levels` gives those elements their base constraint as their host Level, from the Levels' own elevations (RE-51). This runs for walls, columns, doors, windows, slabs, rooms and products.
- The element's `LevelBindSource` is `partition_element_record_base_constraint`, and `apply_record_level_reference_storeys` contains it in that Level's storey.

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

```bash
cargo run --profile ci --example probe_re59_base_constraint_levels -- MODEL.rvt > records.tsv
./target/ci/rvt-ifc MODEL.rvt -o model.ifc
python3 tools/re/storeys_vs_ifc.py [-v] model.ifc REVIT_EXPORT.ifc
```

On Snowdon the scorer prints:

```text
6051 elements, 294 not on a storey
    148  IfcLightFixture
     77  IfcBuildingElementProxy
     64  IfcRailing
      4  IfcSanitaryTerminal
      1  IfcStair
against Revit's export:
   5612  same storey as Revit's
    294  on no storey; Revit's is a storey
     45  a different storey from Revit's
  different, by type: IfcMember 30, IfcStairFlight 6, IfcSlab 3, IfcStair 3, IfcRailing 2, IfcPlate 1
```

- **Snowdon.**
  - 154 elements now take a storey from the two Levels their records name: 98 walls, 39 curtain walls, 10 columns, 4 stairs, 2 elevators and 1 proxy. Each is in Revit's storey.
  - 1,484 curtain-wall panels, mullions and stair flights now follow those wholes. In all, 1,637 more elements are in Revit's storey.
  - The disagreements stay 45. One wall that disagreed now agrees. One curtain panel now disagrees: 1593696, which Revit exports as a curtain wall nested under another curtain wall, and rvt-rs as a panel of a different curtain wall (RE-46's parent join), whose storey it now inherits.
  - The 294 left name no Level: light fixtures, generic proxies, railings, sanitary fixtures and one stair.
- **Core Interior.** All 854 elements matched by `Tag` are in Revit's storey, including RE-27's last unbound wall, 55840. All 970 building elements reach a storey through the Level their record names: 488 name one, and 482 name two. The containment relation that held the one unbound wall in the `IfcBuilding` is gone.
- **RE1.** The 7 walls now take Revit's storey. The other 39 elements Revit's export contains in rooms (`IfcSpace`); rvt-rs places them on the room's storey.
- **The MIT house.** One railing, which names no Level, remains.

The Core witness observations change: 970 elements now report their storey through their own Level, up from 488, and there are 965 more property values. Both verdicts PASS.

## 4. What this does not do

- Elements whose record names no Level: light fixtures hosted on ceilings, railings, and some proxies (294 on Snowdon).
- Records naming three or more Levels (3 Snowdon stairs).
- The 45 Snowdon elements in a different storey from Revit's, mostly curtain-wall mullions whose parent differs, and stair flights.
