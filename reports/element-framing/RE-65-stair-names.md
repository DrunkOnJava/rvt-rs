# RE-65 — Stairs and their parts named as Revit names them

**Date:** 2026-09-24
**Result:**
- Stairs, runs, landings, stringers and carriages are named as Revit's own IFC export names them:
  - a stair `Family:Stair:ElementId`, for example "Assembled Stair:Stair:620883", with `ObjectType` "Assembled Stair:_Stair-Metal-Plate Stringer";
  - each part after its stair and its number, "Assembled Stair:Stair:620883 Run 1", with `ObjectType` such as "Non-Monolithic Run:1/4" Tread 1" Nosing 1/4" Riser" or "Carriage:Carriage - 24" Width".
- Measured against Revit's export of Snowdon Towers, matched by `Tag`:

| | before | after |
|---|---:|---:|
| stairs with Revit's `Name` and `ObjectType` | 0 of 26 | 26 of 26 |
| runs | 0 of 43 | 43 of 43 |
| landings | 0 of 17 | 17 of 17 |
| stringers and carriages | 0 of 170 | 170 of 170 |
| stair parts under Revit's parent stair | 229 | 230 |

- Revit writes a multistory stair once per storey under one `Tag`, and names the later copies with a `:2` suffix. rvt-rs writes each stair once. The table counts a name equal when it equals any of the copies (`names_vs_ifc.py --any-copy`). Compared with the last copy only, 23 of 26 stairs, 37 of 43 runs, 14 of 17 landings and 140 of 170 supports are equal, and every `ObjectType` is.
- No other element changes. Core Interior, RE1 Architecture and Einhoven, which have no stairs, export byte-identical IFC apart from the timestamp. On Snowdon, a per-element comparison of class, `Name`, `ObjectType`, storey and every property set finds only these 256 elements. The one new aggregation is carriage 1557841 (§3).
- Measured on Snowdon Towers only (Revit 2024). None of the other oracles has a stair. The MIT house's one stair names a type that does not read as a stair type (§4), and stays unnamed.

-----

## 1. Types and families

A stair type, a landing type and a support type are serialised as a run type is (RE-52): `01 00 00 00 · u64 id`, with tag `db 0f` 0x47 bytes past the id. RE-44's name read, which looks for the element-data header, finds none of them, so these elements had no `TypeName`. Each kind keeps its name and a flag giving its system family at offsets of its own (`partition_stairs::component_type_at`):

| kind | family flag | values | name |
|---|---|---|---|
| stair type | `u32` +0x129 | 0 on the types of all 23 stairs Revit names Assembled Stair; 1 on the 3 it names Cast-In-Place Stair; 2 on the unused type named "Precast Stair" | after the parameter entries at +0x13f (`u32` count; each an `i64` BuiltInParameter and a string, such as the Uniformat description "Interiors") |
| landing type | byte +0x95 | 0 on the one type Snowdon's 17 landings use, which Revit names Non-Monolithic Landing; 1 on two unused types | +0x98 |
| support type | byte +0x89 | 0 on the two types Revit names Stringer (164 supports), 1 on the two it names Carriage (6) | +0x92 |

A run's family is Monolithic Run or Non-Monolithic Run, by RE-52's monolithic flag: the 3 monolithic runs are Revit's 3 Monolithic Runs.

- **Unmeasured values stay unnamed.** A value not measured against Revit gives no family: construction 2, and a landing flag of 1. With no family the element keeps its `Class-ElementId` name.
- **Stairs.** A stair's family comes from its type, and Revit names the stair `Family:Stair:ElementId`. The middle word is "Stair" on 29 of Revit's 30 stair entities, whatever the type. The 30th is family instance 1603717 (§4).
- **Provenance.** The property set records `FamilyNameSource` `system_family_by_type_kind`.

## 2. Numbers

Revit numbers a stair's runs, landings and supports separately: Run 1, Run 2, Stringer 1 and so on. The number is the part's rank by ElementId among every record of its category whose reference list names the stair, whether or not it is exported.

- Stair 1240635's supports are numbered 1 to 4 and 6 to 8. Its record 1240922, Stringer 5, is in neither export, but counts.
- Ranked over the parts RE-39 aggregated, 4 supports were numbered wrong.

## 3. Choosing between two types or two stairs

A stair type lists its run type, landing type and support types in seven `u64` slots from +0xc9. `examples/probe_re65_stair_names.rs` prints them: 613998 lists run type 613941, landing type 168811 and support type 613997.

- **Two candidate types.** Eight supports name two support types, 168812 and 613997. For all seven that Revit exports, its `ObjectType` gives the one their stair's type lists (168812).
- **Two candidate stairs.** Carriage 1557841 names two stairs: 746421 and 1603717, a stairs-category family instance with no stair type. Only 746421's type lists the carriage's type (1557293), and 746421 is Revit's parent for it. RE-39 left the carriage standalone, and now it joins 746421.

So when a part names several types of its category, or several stairs, the one (stair, type) pair its stair's type lists decides.

## 4. What does not read

- **Type 54873, on Snowdon and on the MIT house.** It holds f64 values where a stair type holds its component slots, and a parameter count of 2.7 billion. The MIT house's one stair names it, and has no runs, landings or supports. `component_type_at` refuses a slot that is neither an ElementId nor unset, so the stair keeps `Stair-268838`.
- **Stair 1603717,** "Residential Lobby Stair Landing Support", is a family instance named by RE-38 already.

## 5. End-to-end measurement

| file | sha256 |
|---|---|
| Snowdon Towers Sample Architectural.rvt (local only) | `33271010919acb2a0d0d040851393a511d86ea7fd9a1f4c054797ae03e5e44a1` |
| Snowdon Towers Sample Architectural_IFC4.ifc (local only) | `ecfcb04e3e818090cf818873fe76fba8b447742bbf4d19dad6d590f09cbb985a` |
| 2024_Core_Interior.rvt | `c805df445d613b408e37337765572021265e3f5dfdc7d1fa53b22ba1600b8014` |
| RE1-Architecture.rvt | `4a78bdd68e7f4dd7806060db07c222da9a810e20b3f303c17a443294b91c8ce4` |

```bash
cargo run --profile ci --example probe_re65_stair_names -- MODEL.rvt
./target/ci/rvt-ifc MODEL.rvt -o model.ifc
python3 tools/re/names_vs_ifc.py --any-copy model.ifc REVIT_EXPORT.ifc
```

The witness observations do not change.
