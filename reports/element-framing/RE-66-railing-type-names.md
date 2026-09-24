# RE-66 — Railing type names from their type objects

**Date:** 2026-09-24
**Result:**
- The 30 Snowdon Towers railings that RE-63 left as `Railing-ElementId` are now named as Revit's own IFC export names them, for example "Railing:Gate:1991314", with `ObjectType` "Railing:Gate" (#322).
- Their 6 types have no element data, so RE-44's name read found nothing. Every railing type keeps its name in its type object: `01 00 00 00 · u64 id`, with tag `db 0f` 0x47 bytes past the id, the object run, stair, landing and support types use (RE-52, RE-65).
- Measured against Revit's export of Snowdon Towers, matched by `Tag`:

| | before | after |
|---|---:|---:|
| railings with Revit's `Name` and `ObjectType` | 101 of 131 | 131 of 131 |

- On the 14 railing types that have element data too, the type-object name equals the element-data name, for all 101 of their railings.
- No other element changes. Core Interior, RE1 Architecture and Einhoven export byte-identical IFC apart from the timestamp. On Snowdon, a per-element comparison of class, `Name`, `ObjectType`, storey and every property set finds only these 30 railings.

-----

## 1. The name

The name is the one `u32 n · n UTF-16 units` that ends where `ff ff ff ff 0b 02` (an unset field frame, tag 0x020b) first starts, searched from +0x100 to +0x300 past the id (`partition_names::find_railing_type_names`). What precedes it varies:

| types | name at | before it |
|---:|---|---|
| 18 of 20 | +0x1ae | an unset ElementId slot, `ff` × 8 |
| 2 of 20 ("Tree Guard", "Handrail - Glass Panel w Brackets") | +0x1cb | a Uniformat code parameter entry (`i64` −1002500, "C2010400") and more fields |

A fixed offset would miss the second pair. That is why the read is anchored at the name's end, as RE-58 anchors material names.

## 2. Scope

- **Revit 2024 only.** RE1 Architecture's one railing type (2025, 446543, named "-") has class tag 0x103c instead, and stays unnamed.
- **Fallback only.** The type-object name is read only for a railing whose type has no element-data name.
- **The MIT house (2024, no IFC oracle).** Its 3 railings now read "Railing:CABLE RAIL - CABLES" and "Railing:Handrail - Pipe", which are Revit's standard railing type names.

## 3. End-to-end measurement

| file | sha256 |
|---|---|
| Snowdon Towers Sample Architectural.rvt (local only) | `33271010919acb2a0d0d040851393a511d86ea7fd9a1f4c054797ae03e5e44a1` |
| Snowdon Towers Sample Architectural_IFC4.ifc (local only) | `ecfcb04e3e818090cf818873fe76fba8b447742bbf4d19dad6d590f09cbb985a` |
| 2024_Core_Interior.rvt | `c805df445d613b408e37337765572021265e3f5dfdc7d1fa53b22ba1600b8014` |
| RE1-Architecture.rvt | `4a78bdd68e7f4dd7806060db07c222da9a810e20b3f303c17a443294b91c8ce4` |

```bash
cargo run --profile ci --example probe_re66_railing_type_names -- MODEL.rvt > rows.tsv
./target/ci/rvt-ifc MODEL.rvt -o model.ifc
python3 tools/re/names_vs_ifc.py model.ifc REVIT_EXPORT.ifc
```

The probe gives 131 railing records and 20 railing types. 14 types have an element-data name and all 20 have a type-object name, and the two agree wherever both exist. The witness observations do not change.
