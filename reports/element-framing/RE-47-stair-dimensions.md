# RE-47 — Stair and flight riser and tread dimensions

**Date:** 2026-09-23
**Result:**
- **Positive.** A stair's serialised element data carries its riser height, tread depth and number of risers, and each run's data carries the run's number of risers.
- rvt-rs writes them as `Pset_StairCommon` on the stair and `Pset_StairFlightCommon` on each flight: `NumberOfRiser` (`IfcCountMeasure`), `RiserHeight` and `TreadLength` (`IfcPositiveLengthMeasure`). These are the sets and measure types Revit's own export uses.
- On Snowdon Towers every value Revit writes for a stair is equal, 26 of 26. For flights, riser height and tread length are equal on 43 of 43 and the number of risers on 37 of 43.
- The other 6 are both flights of three stairs, where Revit writes the stair's total on each flight and rvt-rs writes the run's own count (13 and 11, which make the stair's 24).
- To carry a second property set per element, the IFC model gains `IfcEntity::ElementPropertySet`, and the viewer's element panel shows such sets as further groups.

**Probe:** `examples/probe_re47_stair_dimensions.rs` lists each stair's dimensions and its runs' counts.

**Oracle:** Snowdon Towers Sample Architectural (local only, `.rvt` sha256 `33271010…`) with Revit's IFC4 export (`ecfcb04e…`), `Pset_StairCommon` and `Pset_StairFlightCommon`. No licensed file in CI has a stair.

-----

## 1. The stair

In a stair's element data, after the bytes

```text
ff ff ff ff fb 03 ff ff ff ff fb 03 00 00 00 00
```

come:

| offset | type | value on stair 620883 |
|---:|---|---|
| +16 | f64, feet | riser height 0.578947 (11/19 ft, 0.17646 m) |
| +24 | f64, feet | tread depth 0.916667 (11 in, 0.2794 m) |
| +56 | f64, feet | 11.0 |
| +88 | u32 | number of risers 19 |

The bytes sit 0x63 bytes into the data on 10 stairs and 0x6b on 16, because an optional field comes first. They are found within the first 0x100 bytes, not assumed.

The f64 at +56 is the stair's height on 23 of 26 stairs, but not on the other 3, where riser height times count differs from it. It is not used. Riser height, tread depth and count equal Revit's on all 26, including those three.

## 2. The runs

A run's data carries its number of risers right after

```text
fc ff ff ff ff ff ff ff 00 00 00 02 00 00 00
```

at 0x42 on every Snowdon run. A list of tread nosing distances follows (`u32 index · f64 feet`, one tread depth apart). The same data further on holds the run's path points: the start line's ends and one centre point per tread.

The runs of a stair add up to its number of risers on 23 of 26 stairs: 15 with two runs, 1 with three, and 7 with one. The other 3 each have a single run that reads one more than the stair (3 against 2, and 13 against 12), probably through a "begin or end with a riser" setting that is not read. So a flight's count is:

- a single run: the stair's count;
- several runs that add up to the stair's: their own counts;
- otherwise: none (none on Snowdon).

## 3. Measured

Each value rvt-rs writes against the same property of the same Tag in Revit's export (tolerance 5e-6):

| | NumberOfRiser | RiserHeight | TreadLength |
|---|---:|---:|---:|
| stairs (`Pset_StairCommon`) | 26 of 26 | 26 of 26 | 26 of 26 |
| flights (`Pset_StairFlightCommon`) | 37 of 43 | 43 of 43 | 43 of 43 |

- Every stair and flight Revit gives these properties also has them from rvt-rs.
- The one Snowdon stair without them is 1603717, a stair family placed on its own. It has neither the data nor a `Pset_StairCommon` in Revit's export.
- The 6 flights that differ belong to stairs 621690, 701090 and 882250, each 24 risers in two runs of 13 and 11. Revit's export writes 24 on both flights, and rvt-rs writes 13 and 11. These three are also the only Snowdon stairs Revit's export decomposes with two `IfcRelAggregates` instead of one, so its flight values go with that split, not with the runs.

IFC schema arity and IfcOpenShell 0.8.5 validation pass on the Snowdon export (0 issues).

## 4. What this does not do

- **`NumberOfTreads` is not written.** For a stair it is risers minus runs on 23 of 26 Snowdon stairs. The other 3 are the three single-run stairs whose run reads one riser more, and they have as many treads as risers. Both point to the same begin- or end-with-riser setting, which is not read.
- **No stair geometry changes.** Revit's Snowdon flights are thin tread and riser plates (flight 621141 is 0.033 m³, its treads 0.0208 ft thick). Drawing them needs the tread and riser thicknesses from the stair's type, which are not read yet. A solid sawtooth would misstate the volume by orders of magnitude, so runs keep their record boxes.
- It reads Revit 2024 only. No 2025 file in the corpus has a stair.
