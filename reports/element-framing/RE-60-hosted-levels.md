# RE-60 — Storeys for elements whose record names no Level

**Date:** 2026-09-24
**Result:**
- After RE-59, 294 of Snowdon Towers' exported elements were on no storey, because their element records name no Level. Two readings now place most of them:
  - **A railing takes its host's storey.** Its record names the stair or ramp it is hosted by, and Revit's own export contains the railing in that host's storey on 73 of 74 Snowdon railings.
  - **Other elements need two readings to agree.** Their records name objects laid out `01 00 00 00 · u64 id · u64 Level id`: work planes and views, 6,435 of them on Snowdon, each carrying one Level. Where the one Level an element's objects carry is also the highest Level at or below its base, it is Revit's storey on 161 of 161 Snowdon elements. These are 108 light fixtures, 28 slab edges, 21 generic models, 3 wall sweeps and 1 stringer.
- Each reading alone is wrong too often to use. For light fixtures, the carried Level is wrong on 38 and the elevation on 8, because pendants hang below their Level.
- Storeys in rvt-rs's IFC against Revit's, per element (`tools/re/storeys_vs_ifc.py`):

| file | same storey as Revit's, before → after | on no storey, before → after | a different storey, before → after |
|---|---:|---:|---:|
| Snowdon Towers | 5,612 → 5,805 | 294 → 102 | 45 → 44 |
| 2024_Core_Interior | 854 → 854 | 0 → 0 | 0 → 0 |
| RE1 Architecture | 34 → 34 | 0 → 0 | 0 → 0 |
| the MIT tutorial house | no oracle | 1 → 0 | no oracle |

- The host rule also corrects two Snowdon railings that the elevation match had put on the wrong storey (622060 and 1240636, now L1 - Block 35 as in Revit). Its one miss, 1646280, now sits on Parking against Revit's L1 - Block 37. That is the only new disagreement.

**Probe:** `examples/probe_re60_hosted_levels.rs` prints each record that names no Level: the stair or ramp it names, the Levels its objects carry, and the Level at or below its base.

**End-to-end check:** `tools/re/storeys_vs_ifc.py` (RE-59).

-----

## 1. The two readings

**Railings.** 74 Snowdon railings name no Level. Their reference lists name one Stair or Ramp record, alongside ids of no element-record category. Revit contains 73 of them in the host's storey.

The other, 1646280, names a stair that Revit places on Parking, while Revit puts the railing on L1 - Block 37. Nothing measured in its record separates it from the 73.

The rule applies only where the host itself has a Level.

**Level objects.** The ids light fixtures name include, per storey, objects whose data opens with their own id followed by a Level's id:
- 786626 → L2;
- 787230 → L3;
- 787279 → L4.

`partition_level_records::scan_level_objects` indexes every declared object of that shape. An id found with two different Levels is dropped; on Snowdon none is. What these objects are (work planes, views) is not established, and neither reading needs it.

The objects' Level alone gives the wrong storey on 38 of 146 light fixtures. The highest Level at or below the fixture's base alone gives the wrong storey on 8, which are pendants hung below their ceiling's Level. Where the two agree, all 161 elements are in Revit's storey.

Railings take only the host reading. Stair parts are not changed: they follow their stair through aggregation.

## 2. The change

- `element_record_level_refs`:
  - `REFERENCES_FIELD` holds the reference list of a record naming no Level;
  - `level_at_or_below` returns the highest Level at or below the base, declining ties;
  - `HOST_SOURCE` and `LEVEL_OBJECT_SOURCE` name the two bind sources.
- `partition_level_records::scan_level_objects`.
- `partition_schema_mvp::resolve_hosted_levels` runs after RE-59's base constraints, so a railing can take its stair's Level.
- The element's `LevelBindSource` records which reading bound it: `partition_element_record_host` or `partition_element_record_level_object`.

## 3. End-to-end measurement

| file | sha256 |
|---|---|
| Snowdon Towers Sample Architectural.rvt | `33271010919acb2a0d0d040851393a511d86ea7fd9a1f4c054797ae03e5e44a1` |
| Snowdon Towers Sample Architectural_IFC4.ifc | `ecfcb04e3e818090cf818873fe76fba8b447742bbf4d19dad6d590f09cbb985a` |
| 2024_Core_Interior.rvt | `c805df445d613b408e37337765572021265e3f5dfdc7d1fa53b22ba1600b8014` |
| RE1-Architecture.rvt | `4a78bdd68e7f4dd7806060db07c222da9a810e20b3f303c17a443294b91c8ce4` |
| demo-01-Source_House_2024_tn2.rvt | `763f239e70db1ac90e5aa806ec4e6a2cebdc3e8a2e902bcfda6b8de43d2fdcfb` |

```bash
cargo run --profile ci --example probe_re60_hosted_levels -- MODEL.rvt > rows.tsv
./target/ci/rvt-ifc MODEL.rvt -o model.ifc
python3 tools/re/storeys_vs_ifc.py [-v] model.ifc REVIT_EXPORT.ifc
```

On Snowdon the scorer prints:

```text
6051 elements, 102 not on a storey
     46  IfcBuildingElementProxy
     40  IfcLightFixture
     11  IfcRailing
      4  IfcSanitaryTerminal
      1  IfcStair
against Revit's export:
   5805  same storey as Revit's
    102  on no storey; Revit's is a storey
     44  a different storey from Revit's
  different, by type: IfcMember 30, IfcStairFlight 6, IfcSlab 3, IfcStair 3, IfcPlate 1, IfcRailing 1
```

- The IFC carries 63 railings bound by their host and 161 elements bound by their Level objects.
- Core Interior's IFC is byte-identical apart from its timestamp, so the witness observations do not change.
- On the MIT house, the last element on no storey, a railing, now takes its stair's storey.

## 4. What this does not do

Snowdon still has 102 elements on no storey:
- 46 proxies;
- 40 light fixtures, where the two readings disagree or one is missing;
- 11 railings whose host has no Level or which name none;
- 4 sanitary fixtures;
- 1 stair support.

Revit gives each of them a storey.
