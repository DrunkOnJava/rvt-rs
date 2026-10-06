# Revit 2023: the container reference and the Building Story setting in their 32-bit forms

**Date:** 2026-10-06
**Status:** positive.
**Artefacts:** Autodesk's `rac_basic`, `rst_basic` and `rac_advanced` sample projects of 2023 and 2024 (pinned in `research/autodesk-sample-projects.tsv`, fetched with `tools/fetch-autodesk-samples.sh`).
**Answer key:** the 2024 copy of each model, joined on ElementId. ElementIds and the UniqueId-derived GlobalIds survive Autodesk's re-save. This is agreement with the admitted 2024 reader of the same model, not with a Revit export of the 2023 files.
**Probe:** `examples/probe_32bit_container.rs`.

## 1. The container reference

A 2023 element record's container reference is a `u32` 16 bytes after the category (22 before the marker), followed by `ff ff ff ff`. The `u64` read at 2024's place relative to the marker (8 bytes after the category) is another field.

| 2023 project | records shared with 2024 | in a container on 2024 | old `u64` agrees | `u32` agrees |
|---|---:|---:|---:|---:|
| `rac_basic` | 1,201 | 3 | 1,192 | 1,201 |
| `rst_basic` | 5,858 | 0 | 1,675 | 5,858 |
| `rac_advanced` | 8,901 | 32 | 8,806 | 8,901 |

`find_level_records_2023` reads the same field. `rac_basic`'s Level 800333 is a container member on 2024 (container 800214), and the old read took it for a seventh standalone Level. That failed `recovered_levels_are_a_storey_set`, so no Level of the project was read.

## 2. The Building Story setting

After a 2023 Level's name the frame is `ff ff ff ff 00 01`, an `f64` (0), a `u32` ElementId (305, the Level type, on `rac_basic`) and the setting byte. The `u64` frame `BUILDING_STORY_ANCHOR` searches for finds an unrelated run there. Read in the 32-bit form, every Level's setting equals the 2024 copy's:
- `rac_basic`: 6 Levels, all stories.
- `rac_advanced`: 5 Levels, all stories.
- `rst_basic`: 9 Levels, none a story.

## 3. Export

Storey agreement of the 2023 exports with the 2024 copy's, per element:

| project | before | after |
|---|---:|---:|
| `rac_basic` | 15.5 % | 81.7 % |
| `rst_basic` | 91.9 % | 95.2 % |
| `rac_advanced` | 90.0 % | 97.8 % |

`rac_basic`'s one element that 2024 does not export, a container member, is no longer exported.

The 2024 exports of the three projects are byte-identical apart from timestamps.

## Not measured

There is no Revit export of these 2023 files. The admitted 2023 reference projects were not available here, so the Measure workflow on them is the check that remains.
