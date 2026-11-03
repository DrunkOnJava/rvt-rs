# RE-306 — Why a beam keeps its record box

**Date:** 2026-11-03
**Issues:** #1247; follows RE-49 and RE-216
**Run:** Measure [41627751687](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41627751687) (`probe_re306_beam_copies` at cd3aff05, Autodesk's 2024 sample projects), after [41624852827](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41624852827) at bdc797ea (Core Interior and the BIM4LCA office as built)
**Result:** partial. On every file measured here, a beam drawn as its record box has one location line that `partition_beam_axes::beam_body` does not fit to its box, or no line; none has copies that disagree. Whether #1247's 341 boxed beams do is open: the probe is for that file.

## 1. The question

A structural-framing element runs along its location line only where `partition_beam_axes::scan_bounded_lines` finds one line for it in its element data and `beam_body` builds a solid from that line and its record box (RE-49). Since RE-216, an element whose copies of that data in different partitions hold different lines (`Copies::Agreeing`) gets none, and keeps its box. #1247 reports 341 beams of a Revit 2024 project drawn as boxes that v0.6.1, before RE-216, drew along their lines. That rule is the first suspect.

## 2. The probe

`probe_re306_beam_copies` exports the file, takes every `IfcBeam` whose `BodySource` is `partition_element_record_bbox`, and sorts it by what the line scan finds, from `partition_beam_axes::line_copies`, which returns every copy without choosing:

- `no_data`: no partition's element data names it;
- `no_line`: its element data holds no bounded line;
- `one_line`: its copies hold one line, so `beam_body` rejected the line against the box;
- `disagree`: its copies hold different lines, and RE-216 drops it. For these it counts the beams whose current partition (`native_document::current_partitions`) holds one line, and the largest distance between two copies' line ends.

It opens the file with `--large-model`'s limits, since at the default limits a partition over them is not read and its beams would count as `no_data`, and prints how many streams were still not read whole. It prints counts only.

## 3. Measured

| file | boxed beams | one line | no line | copies disagree | streams not read |
|---|---:|---:|---:|---:|---:|
| Snowdon Towers structural (2024 sample) | 19 | 19 | 0 | 0 | 0 |
| rst basic (2024 sample) | 25 | 25 | 0 | 0 | 0 |
| rac advanced (2024 sample) | 9 | 8 | 1 | 0 | 0 |
| Core Interior, the BIM4LCA office as built, the other 2024 samples | 0 | | | | |

Snowdon's 19 are the beams RE-49 left as boxes (923 of its 942 run along their line). No file here has a boxed beam whose copies disagree, so on these files RE-216 boxes no beam.

## 4. Not claimed

- That RE-216's rule is or is not the cause of #1247's 341: the user's file is the only one known to show the loss.
- Why `beam_body` rejects the 52 lines above; RE-49 describes the box check.

## 5. Reproduce

```bash
gh workflow run measure.yml --ref main -f ref=cd3aff05 -f base=none -f probe=probe_re306_beam_copies \
  -f samples=true -f sample_years="2024 2024" -f scorers=none \
  -f models="sample-2024-snowdon_towers_sample_structural sample-2024-rstbasicsampleproject sample-2024-racadvancedsampleproject"
```
