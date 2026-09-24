# RE-69 — Wall sweeps named as Revit names them

**Date:** 2026-09-24
**Result:**
- Wall sweeps are named as Revit's own IFC export names them, rather than `WallSweep-ElementId`:
  - 249 on Snowdon Towers are "Wall Sweep:<type>:<ElementId>", for example "Wall Sweep:Base_Simple:2074039", with `ObjectType` "Wall Sweep:Base_Simple";
  - 9 are named by their ElementId alone, with no `ObjectType`, as Revit names a sweep that is part of a wall type's own structure.
- Measured against Revit's export of Snowdon Towers, matched by `Tag`:

| | before | after |
|---|---:|---:|
| wall sweeps with Revit's `Name` and `ObjectType` | 0 of 258 | 258 of 258 |

- No other element changes. The 13 other local files export byte-identical IFC apart from the timestamp. On Snowdon, a per-element comparison finds only these 258.
- Measured on Snowdon only (Revit 2024).

-----

## 1. The type

A wall sweep's type has no type-definition record of the sweep category (`OST_Cornices`, −2000181), so RE-44's type join finds none. The sweep's record names the type all the same, among:
- its profile, a family type with a name entry (RE-38);
- its material;
- the walls it runs along.

`examples/probe_re69_wall_sweep_types.rs` lists, for every sweep record, the ids it names that have a type object (`01 00 00 00 · u64 id`, tag `db 0f`) and no name entry:

| sweep records | such ids named | Revit's export |
|---:|---:|---|
| 249 | 1 | `Wall Sweep:<that id's element-data name>:<ElementId>` for all 249 |
| 9 | 0 | the ElementId alone, for all 9 |
| 5 | 2 | not exported |

The element-data names (RE-44) of those ids are Revit's type names: "Base_Simple", "Crown_Flat", "Coping_Rainscreen Wall" and so on. The other ids a record names either have no type object (materials, walls) or have a name entry (the profile), so they cannot be taken for the type.

## 2. The rule

`partition_schema_mvp::attach_wall_sweep_types`, on Revit 2024:
- a sweep naming exactly one such id takes it as its type, and the system family Wall Sweep, with `FamilyNameSource` `system_family_by_category`;
- a sweep naming none is named by its ElementId;
- a sweep naming two or more gets neither.

## 3. End-to-end measurement

| file | sha256 |
|---|---|
| Snowdon Towers Sample Architectural.rvt (local only) | `33271010919acb2a0d0d040851393a511d86ea7fd9a1f4c054797ae03e5e44a1` |
| Snowdon Towers Sample Architectural_IFC4.ifc (local only) | `ecfcb04e3e818090cf818873fe76fba8b447742bbf4d19dad6d590f09cbb985a` |

```bash
cargo run --profile ci --example probe_re69_wall_sweep_types -- MODEL.rvt
./target/ci/rvt-ifc MODEL.rvt -o model.ifc
python3 tools/re/names_vs_ifc.py model.ifc REVIT_EXPORT.ifc
```

The witness observations do not change.
