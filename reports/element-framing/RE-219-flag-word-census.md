# RE-219: The element record's +0x46 flag word is not LoadBearing or IsExternal

**Date:** 2026-10-25
**Issues:** #223 (the flag word at `+0x46`). Follows RE-21 (the element record), RE-76 (`+0x4a` is the schema class), RE-135 (bit `0x10` follows the kind of record, not a host-wall stub) and puzzbobb's consecutive-save test on #223 (bit `0x1000` is not Room Bounding and does not follow a wall's height).
**Status:** negative.
**Artefacts:**

| file | sha256 | licence |
|---|---|---|
| `2024_Core_Interior.rvt` (Revit 2024) | `c805df445d613b408e37337765572021265e3f5dfdc7d1fa53b22ba1600b8014` | MIT, magnetar-io/revit-test-datasets |
| `2024_Core_Interior_slim.ifc` (Revit's IFC4 export) | `bfdf36ffb0bb768f3409d818403990e64d4c262c6780603be87f8077387ad86d` | MIT |
| `ARK_NordicLCA_Office_Concrete_As-Built.rvt` (Revit 2024) | `747120005b4b35c7cb66ec61158aab50ded95e052e172cb387b86cd15b31d906` | CC BY-SA 4.0, VTT (`research/bim4lca-measure.tsv`) |
| `ARK_NordicLCA_Office_Concrete_As-Built_Revit.ifc` (Revit's IFC4 export) | `b2f530eba8b4424c4542b98e9a2c7bbcc726367f21b2740f3074e4a6ce917382` | CC BY-SA 4.0 |
| `ARK_NordicLCA_Office_Concrete_BuildingPermit.rvt` (Revit 2024) | `1c12c2058128315edbd65a8bf97f60da971aee8625f21233b8477d9c6165c1d1` | CC BY-SA 4.0 |
| `ARK_NordicLCA_Office_Concrete_BuildingPermit_Revit.ifc` (Revit's IFC4 export) | `bc3bd3366a3c413ce74840300a590f59102b740118ef37898e906253b6b7c08d` | CC BY-SA 4.0 |
| puzzbobb's R2025 models with Revit's IFC4 exports | `research/r2025-models.tsv` | Apache-2.0 |

Measure fetches all of them and stores none.
**Probe:** `examples/probe_223_flag_census.rs`. **Scorer:** `tools/re/flag_bits_vs_ifc.py`.
**Run:** Measure [40169108869](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40169108869), branch `re/223-flag-census`, `-f nordic=true -f r2025=true`.

## 1. The question

#223 asks whether the `u32` at `+0x46` of a first-prologue element record carries element data. Two exported properties are booleans that Revit's export writes on most building elements and rvt-rs can compare by Tag: `LoadBearing` (a wall's or slab's Structural parameter) and `IsExternal`. If a bit equalled one of them on every element of a category, it would be that property.

## 2. What was measured

The probe reads `+0x46` of every first-prologue record of ten categories (walls, floors, roofs, ceilings, columns, structural columns, framing and foundations, doors, windows). The scorer pairs each with Revit's element of the same Tag, reads its `Pset_*Common`, and for each of the 32 bits counts the elements on which the bit equals the property. Only properties that take both values on an entity are tested.

| model | entity | paired | `LoadBearing` | `IsExternal` |
|---|---|---:|---|---|
| office as built | `IfcWall` | 607 | 53 true; no bit equals it | 96 true; no bit |
| | `IfcSlab` | 41 | 14 true; no bit | 31 true; no bit |
| | `IfcCurtainWall` | 220 | | 156 true; no bit |
| | `IfcDoor` | 268 | | 22 true; no bit |
| | `IfcWindow` | 269 | | 265 true; no bit |
| | `IfcColumn` | 279 | 278 true; bit `0x1000` equals it | 80 true; no bit |
| | `IfcCovering` | 99 | | 21 true; bit `0x1` equals it |
| office at building permit | `IfcWall` | 589 | 53 true; no bit | 90 true; no bit |
| | `IfcSlab` | 41 | 14 true; no bit | 31 true; no bit |
| | `IfcColumn` | 345 | 344 true; bit `0x1000` equals it | 56 true; no bit |

On Core Interior and the R2025 models every paired element has one value of each property, so nothing is tested there.

## 3. The two bits that match follow the record's class

- **`0x1000` on columns.** Every one of the office's 278 placed structural columns (`OST_StructuralColumns`) has it, and its one architectural column (`OST_Columns`) does not. Revit's export writes a structural column as load-bearing and an architectural one as not, and rvt-rs already takes `LoadBearing` from the category. The bit is also on 980 of the as-built file's 1,084 wall records and 31 of its 36 floors, where it is not `LoadBearing`, and puzzbobb found it is not Room Bounding.
- **`0x1` on coverings.** It marks the 21 coverings whose records are walls' (`0x929` and `0x1929`, the words of wall records), which Revit's export calls external; the 78 others carry `0x928`. On the building-permit file all 75 coverings carry `0x928` and none is external. The bit is on every wall record and on no other category's, so it follows the record's class.

Two more readings, from the words' distribution, untested against an export:

- **`0x80`** is set on records that pair with no element of Revit's export (`0x9a8` and `0x2009a8`: 35 to 38 structural-column and 2 to 3 column records per office file, 17 column records and one wall record on Core Interior), which are likely type records, and clear on every record that pairs.
- **`0x200000`** is set on some of those records and on none that pair.

## 4. What this binds

No bit of `+0x46` is a wall's, slab's, door's, window's or curtain wall's `LoadBearing` or `IsExternal` on these files. Do not read either property from the flag word. A bit that tracks the record's class or category tells rvt-rs nothing the schema class at `+0x4a` (RE-76) and the `BuiltInCategory` do not already say. #223 stays open for what the word does carry: re-probe it only with a property that varies within a category, or with consecutive saves like puzzbobb's.

## 5. Reproduce

```bash
gh workflow run measure.yml --ref main -f ref=d7ec5a15c7e1942b68934365ecf699ff84cd3fbb -f base=none \
  -f nordic=true -f r2025=true -f probe=probe_223_flag_census -f probe_scorer=flag_bits_vs_ifc
gh run download <run id>   # measure-head/<model>/probe.txt and probe_score.txt
```
