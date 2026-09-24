# RE-68 — Storeys for the elements whose record names no Level

**Date:** 2026-09-24
**Result:**
- After RE-59 and RE-60, 102 Snowdon Towers elements were on no storey. 97 of them now sit on the storey Revit's own IFC export gives them, none on a wrong one. The other 5 stay unplaced by design (§4).
- Three rules, applied in order to record-backed elements still without a Level:
  1. **A stair or ramp whose base sits exactly at one Level takes that Level.** A multistory stair names three Levels, so RE-59 declined it. Its railings then follow it through RE-60's rule.
  2. **An element takes the Level of the one element of its own class that its record names.** These are the parts of a nested light-fixture family, which follow the chandelier that holds them.
  3. **Slab edges, light fixtures, wall sweeps, generic models, hardscape and stairs take the Level their base elevation gives,** within a fail-closed band (`element_record_level_refs::elevation_band_level`):
     - the Level just above the base when the base is at most 0.25 ft below it;
     - the Level at or below the base when the next Level up is at least 0.5 ft above;
     - no Level in between.
- Measured against every copy Revit writes of each element: it writes a multistory stair and its railings once per storey under one `Tag`. `tools/re/storeys_vs_ifc.py --any-copy` is the new flag for that.

| Snowdon Towers | before | after |
|---|---:|---:|
| on Revit's storey | 5,847 | 5,944 |
| on no storey | 102 | 5 |
| on a different storey | 2 | 2 |

- No other element moves. The 13 other local files export byte-identical IFC apart from the timestamp. On Snowdon a per-element comparison finds only:
  - the 97 newly placed elements;
  - 3 stairs and 2 stringers that gain a Level on the storey they already had.
- Measured on Snowdon only (Revit 2024). The other files have no element these rules reach.

-----

## 1. What the 102 were

| elements | what their record names | Revit's storey |
|---:|---|---|
| 11 railings | a multistory stair (621690, 701090 or 882250) | the stair's: L3, and its L4 copy |
| 16 light fixtures | the chandelier 1612603, which has a Level | the chandelier's, L3 |
| 31 slab edges, 24 light fixtures, 6 wall sweeps, 7 generic models, 2 hardscape, 1 stair | Level objects (RE-60) that name a Level other than the one at or below their base, or none | by elevation (§2) |
| 4 plumbing fixtures | a Level object naming L1 - Block 35 | L1 - Block 35, although their base is up to 3.2 ft away from it |

## 2. The elevation band

For the 71 elevation-placed elements, the gap between the base and the next Level up separates Revit's choices:

| Revit's storey | distance from the base up to the next Level |
|---|---|
| the Level just above (5: 3 model texts, 2 stair nosings) | 0.013, 0.013, 0.213, 0.213, 0.233 ft |
| the Level at or below (66) | 0.333 ft or more |

The rule takes the Level above within 0.25 ft, and the Level below from 0.5 ft. Between 0.25 and 0.5 ft it takes neither: the measured gap is only 0.1 ft wide, so the band stays closed on its uncertain part. Slab edge 2060948, 0.333 ft below the next Level, therefore stays unplaced, although Revit puts it on the Level below.

## 3. Why not the Level objects

RE-60 already used the Level that named objects carry, where it agreed with the base elevation. For the elements left over, it doesn't agree:
- For 26 slab edges, 38 light fixtures and 6 sweeps, the objects name a Level other than Revit's, typically the storey above.
- Only for the 4 plumbing fixtures is it Revit's. The elevation band would put them wrongly: the floor sinks are 0.57 ft below their Level, and the faucet is 3.2 ft above.

Plumbing fixtures are therefore not among the elevation-placed classes, and they stay unplaced.

## 4. What stays

- **4 plumbing fixtures.** They follow their Level object in Revit's export. Nothing here separates their objects from the light fixtures' ones, which don't name Revit's Level.
- **Slab edge 2060948,** in the band's closed part.
- **The 2 elements on a different storey** predate this and are #370's: curtain panel 1593696 (a nested curtain wall) and railing 1646280.

## 5. End-to-end measurement

| file | sha256 |
|---|---|
| Snowdon Towers Sample Architectural.rvt (local only) | `33271010919acb2a0d0d040851393a511d86ea7fd9a1f4c054797ae03e5e44a1` |
| Snowdon Towers Sample Architectural_IFC4.ifc (local only) | `ecfcb04e3e818090cf818873fe76fba8b447742bbf4d19dad6d590f09cbb985a` |

```bash
cargo run --profile ci --example probe_re68_remaining_levels -- MODEL.rvt > rows.tsv
./target/ci/rvt-ifc MODEL.rvt -o model.ifc
python3 tools/re/storeys_vs_ifc.py --any-copy -v model.ifc REVIT_EXPORT.ifc
```

The witness observations do not change.
