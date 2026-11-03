# RE-314 — Rooms on Revit 2026 and 2027 take their stored solid's outline

**Date:** 2026-11-03
**Issues:** #1260 (reported by inevitable-co-ai); follows RE-101 (a room's outline from its stored solid's faces) and RE-102 (the same on Revit 2023)
**Artefacts:** Autodesk's `rac_basic`, `rac_advanced`, `rme_basic`, `rme_advanced` and `rst_basic` sample projects, 2025, 2026 and 2027 saves, as Measure fetches and pins them (`research/autodesk-sample-projects.tsv`); none is committed
**Probe:** `probes/examples/probe_re101_room_solids.rs` (unchanged; it calls the outline reader with no release gate)
**Runs:** Measure [41653302934](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41653302934) (the probe on `main` at 1340ca8d, the 2025 to 2027 samples), [41655496833](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41655496833) (the fix, ba28a1a1, against `main` at a98bd316) and [41668846939](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41668846939) (the held-out models scored by RE-313's `space_profiles_vs_ifc.py`, PR #1281)
**Result:** positive. The stored-solid layout of RE-101 is unchanged on Revit 2026 and 2027, and the reader was gated off there.

## 1. The report

On Revit 2026 and 2027 saves of Autodesk's `rac_basic_sample_project`, 7 of its 14 rooms came out larger than in the 2015 to 2025 saves of the same project, the Entry Hall 72.80 m² against 30.28 m², overlapping its neighbours. On flowbim's 2026 house, one corridor was 116.8 m² against 36.6 m² in Revit's IFC.

## 2. The cause

`partition_room_boundaries::ROOM_SOLID_SUPPORTED_REVIT_VERSIONS` listed 2008 to 2025. `attach_room_outlines` returns early on any other release, so on a 2026 or 2027 file every room keeps its record box. A rectangular room's box is its outline; an L-shaped or notched room's box covers more, which is the growth the report measured.

## 3. Measured

`probe_re101_room_solids` reads each room's stored solid with no release gate and prints, per room, whether its outline closes on the record box, and the outline's area. Over the four sample projects that have rooms (`rst_basic` has none):

| project | rooms | outline kept, 2025 / 2026 / 2027 | rooms whose 2026 and 2027 outline equals 2025's |
|---|---:|---|---:|
| `rac_basic` | 14 | 14 / 14 / 14 | 14 |
| `rac_advanced` | 91 | 85 / 85 / 85 | 91 |
| `rme_basic` | 76 | 26 / 26 / 26 | 76 |
| `rme_advanced` | 76 | 26 / 26 / 26 | 76 |

Equal means the same verdict and, where an outline is kept, the same area to a relative 1e-9. The rooms whose solid does not close keep their box on every release alike. Room 857191 of `rac_basic` is the report's Master Bedroom: its outline is 290.32 ft² (26.97 m², the report's 2025 figure) and its record box 380.40 ft² (35.34 m², the report's 2026 figure).

## 4. Export

Measure 41655496833 exported 60 models with the fix and with `main` (`-f samples=true -f sample_years="2025 2027" -f holdout=true -f nordic=true -f r2025=true`). 13 exports change, all of them Revit 2026 or 2027 files with rooms:

- the 2026 and 2027 saves of `rac_basic`, `rac_advanced`, `rme_basic` and `rme_advanced`;
- the five held-out 2026 and 2027 models.

The other 47 are byte-identical: Core Interior, Einhoven, RE1, the office, the 24 Revit 2025 models, every 2025 sample and `rst_basic` 2026 and 2027, which has no rooms. No scorer output on `main` moves, since none of its scorers measured a room's outline.

RE-313's space scorer (Measure 41668846939, `--ref` its branch, the fix against `main`) gives these counts on the held-out models; the rest of their output is not uploaded:

| held-out model | rooms | area within 0.1% of Revit's, `main` → fix | outline within 0.01 ft, `main` → fix |
|---|---:|---|---|
| the 2026 house | 12 | 7 → 10 | 0 → 0 |
| four 2026 and 2027 projects | 16 | 16 → 16 | 16 → 16 |

The four projects' rooms are rectangles, which their record box already drew. On the house, 3 more rooms take Revit's area, and 2 remain off. No room's outline is within 0.01 ft of Revit's either way. The house is held out, so why is not examined.

## 5. Not claimed

- That a 2026 or 2027 room's outline equals Revit's own export of it. No Revit export of a 2026 or 2027 sample is in CI. On the one held-out house, no outline is within 0.01 ft of Revit's (§4).
- A room's prism volume (RE-296) on 2026 or 2027: `ROOM_PRISM_REVIT_VERSIONS` stays 2024 and 2025.

## 6. Reproduce

```bash
gh workflow run measure.yml --ref main -f ref=1340ca8d -f base=none -f samples=true \
  -f sample_years="2025 2027" -f probe=probe_re101_room_solids -f scorers=none
```
