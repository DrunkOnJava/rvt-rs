# RE-354: door types' Fire Rating and Heat Transfer Coefficient, and family types' Manufacturer

**Date:** 2026-11-05
**Issues:** [#1185](https://github.com/DrunkOnJava/rvt-rs/issues/1185), W7 of [`docs/release-1.0-criteria.md`](../../docs/release-1.0-criteria.md); follows RE-77 (type text parameters), RE-185 section 3.3 (a family type table's entries), RE-286 (type objects' common sets) and RE-289 (a Manufacturer negative that this report overturns)
**Artefacts:** the BIM4LCA office as built and at building permit (Revit 2024, Revit 24.0.5's IFC4 exports, CC BY-SA 4.0, `research/bim4lca-measure.tsv`), as Measure fetches them
**Probes:** `probes/examples/probe_re354_door_type_parameters.rs` (Measure run [42112543949](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42112543949)) and `probe_re354_door_type_blocks.rs` (runs [42115439931](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42115439931) and [42118085660](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42118085660), commit 42707c3a)
**Status:** positive, on one building (the office; the held-out models are measured below). A door type's `FireRating` and `ThermalTransmittance` and six family types' `Manufacturer` are written, equal to Revit's export on all 13 type values on both files. Every value of #1185 that Revit's export writes and rvt-rs can read is now written; what stays out is `IsExternal` on type objects, by the maintainer's decision (section 6).

## 1. What Revit's export writes

On type objects of the office, beside what RE-286 wrote: `Pset_DoorCommon.FireRating` on six door types, `Pset_DoorCommon.ThermalTransmittance` on one, and `Pset_ManufacturerTypeInformation.Manufacturer` on six family types. The two doors of one of the rated types write the same `FireRating` as their type, which is `-`; the doors of the other five carry a value of their own, the shared parameter `Pset_DoorCommon.FireRating` that RE-194 reads (`EI60` on every door of type 2803674, whose own is `EI30`), so a type's value and its doors' differ.

| door type (Tag) | name | `FireRating` |
|---|---|---|
| 2803674 | O-1.12x21 | EI30 |
| 2324339 | O-17.12x21 | EI30 |
| 1110711 | O-16.1200x2000_180° | EI60 |
| 1106810 | LU-3.VSS.700x1200_90° | EI60 |
| 1106812 | LU-3.VSS.700x1200_180° | EI60 |
| 2391770 | O-14 | - |

`ThermalTransmittance` 3.7021 is on door type 3261757 (UO-6.30x24) and on none of its doors. The six `Manufacturer` types are 1414255 (`IDO`), 2575395 (`Ido`), 1611986 (`Väinö Korpinen Oy`), 3365322 and 3376911 (`Thorlux Lighting`) and 1533300 (`Korpinen Oy`); none of their instances carries it.

## 2. Where they are not

The saved parameters of the doors and their types (RE-207, `probe_re354_door_type_parameters`) list the doors' own `Pset_DoorCommon.FireRating` (1877482, shared) and `Paloluokka` (1105117, shared) and, on the types, family and shared parameters of a door's dimensions and finishes, but no fire rating, no manufacturer text (`Valmistaja` is empty on type 2803674) and no heat transfer coefficient. RE-77's reader takes `i64 BuiltInParameter · u32 n · UTF-16 × n`; the three `EI60` texts it finds (RE-289 section 2) are in the number of the three wall types Revit's export gives that rating.

## 3. Where they are

In the type's own value block, the entries are in the form of a family type table's (RE-185 section 3.3), the text first and the parameter after it:

```text
u32 n · UTF-16 × n · (u32 0 | ff ff ff ff · u16) · f64 · i64 -1 · i64 BuiltInParameter
```

`probe_re354_door_type_blocks` lists every `EI30`, `EI60`, `EI90` and `Manufacturer` text of the partitions with the bytes after it and the value block it lies in. On the office as built:

- 49 `EI30` and `EI60` entries have the parameter -1001206 (the built-in Fire Rating RE-77 named), in the blocks of 25 owners. The blocks of 2803674, 2324339, 1110711, 1106810 and 1106812 hold the text Revit writes for those types. The other 20 owners are none of the 41 door types of Revit's export; what they are was not read. The `-` of 2391770 was not searched for, and its type and both its doors are equal in the census, so the scan reads it.
- The six `Manufacturer` texts are entries of this form under -1010108, in the blocks whose owners are the six type Tags above. Other blocks hold the same texts too. Ido's type and Thorlux's two use the second flag form (`ff ff ff ff · u16`) and the other three the first.
- `ThermalTransmittance`: 3.7021 is the `f64 · ff × 8 · i64 -1005430` entry (RE-93's form) of block 3261757, and it is in four other blocks (3261276, 3261284, 3262425 and 35764), none a door type of Revit's export.

## 4. What rvt-rs does

`partition_type_parameters::scan_partition` reads, beside RE-77's entries, the table form for the Fire Rating (-1001206) and the Manufacturer (-1010108), both flag forms, in the block of a declared owner; two copies that disagree give the type neither. The text then goes where RE-77's goes: onto each element of the type as the type parameter `Fire Rating`, which an element's own `Pset_DoorCommon.FireRating` replaces (the two doors of type 2391770 take the type's `-`), and onto the type object (`TypeCommon`). A `Manufacturer` goes in `Pset_ManufacturerTypeInformation` on the type object alone, for any entity. A door type's `-1005430` entry above zero (`type_thermal_transmittances`) is its `ThermalTransmittance` in `Pset_DoorCommon`, on the type object only: Revit's export gives the doors of that type none.

## 5. Measured

`psets_vs_ifc`, head against `main` (c5a3e9dc), the office as built and at permit, `--revit-set-placement`:

| run, head | office as built, equal | office at permit, equal | missing, as built / at permit | different | values rvt-rs writes that Revit does not |
|---|---:|---:|---:|---:|---:|
| `main` | 45,485 of 45,808 | 45,634 of 45,967 | 322 / 332 | 1 / 1 | 2 / 7 |
| 42117846026, cd18a1e2 (Fire Rating and Heat Transfer Coefficient) | 45,494 | 45,643 | 313 / 323 | 1 / 1 | 2 / 7 |
| 42120473989, c714d2cc (with Manufacturer) | 45,500 | 45,649 | 307 / 317 | 1 / 1 | 2 / 7 |

Per property, the head's: `Pset_DoorCommon.FireRating` 55 of 55 equal (was 47), `ThermalTransmittance` 19 of 19 (was 18) and `Pset_ManufacturerTypeInformation.Manufacturer` 6 of 6 (was 0), on both files; the quantities are unchanged (`qtos_vs_ifc`: 29,760 and 30,002 equal). Of the 182 values on type objects as built, 47 are written equal (34 by RE-286, 13 by this report) and the other 135 are `IsExternal`; at permit the same 13 are added to RE-286's.

The held-out and reference models are not yet measured on this head.

## 6. What stays out

- **`IsExternal` on type objects** (135 values as built): the maintainer's decision of 2026-10-31, because Revit's newer exporters write none (RE-286 section 3).
- RE-286 section 6's other leave-outs are unchanged: a curtain panel's door type (RE-269), a ramp's, railing's and curtain member's type, and `Pset_BuildingElementProxyCommon.IsExternal` on the building.
- A `Manufacturer` on an element's own parameters: Revit's export writes none, so the instance gets none.

## 7. Reproduce

```bash
gh workflow run measure.yml --ref main -f ref=42707c3a -f base=none -f nordic=true \
  -f models="nordic-office-as-built nordic-office-permit" -f scorers=none \
  -f probe=probe_re354_door_type_blocks -f probe_scorer=-
gh workflow run measure.yml --ref main -f ref=c714d2cc -f base=main -f nordic=true \
  -f models="nordic-office-as-built nordic-office-permit" -f scorers="psets_vs_ifc qtos_vs_ifc" \
  -f ifc_flags=--revit-set-placement
```
