# RE-157: a pipe's inner and outer diameters are in its own curve object

**Date:** 2026-10-03
**Issues:** #35, #96; backlog item B26
**Artefacts:** RE1 Mechanical and RE1 Plumbing (Revit 2025, MIT) with Revit's own IFC2X3 exports (Coordination View 2.0; first recorded here as IFC4). Measured on GitHub's hosted runners with the Measure workflow, run 37142956936.
**Probe:** `examples/probe_re157_pipe_inner_diameter.rs`.
**Status:** positive.

## 1. The question

Revit's IFC2X3 export gives every pipe `Pset_FlowSegmentPipeSegment.InvertElevation` (69 values on RE1, all missing from rvt-rs's export). IFC4 has no flow-segment sets: its `Pset_PipeSegmentOccurrence` holds the invert, where Revit's IFC4 export of the BIM4LCA office writes it and rvt-rs's IFC4 export does since 2026-10-19 (#822). On the RE1 exports it is:

- for a horizontal pipe (46), the height of its axis above its storey less half its inner diameter: pipe 443719's axis is at 2,850 mm and its invert 2,844.67 mm, with an outer radius of 6.35 mm;
- for a vertical pipe (23), the height of its lower end: pipe 443895 starts at 415.884 mm and its invert is 415.884 mm.

rvt-rs reads a pipe's axis (RE-131) and its nominal size (RE-134), not its inner diameter. Where is it?

## 2. Method

From the reference export the probe takes each horizontal pipe's inner diameter, twice its axis height less its invert. For each distinct inner diameter it looks, in every partition, for an `f64` within 1e-6 ft of it in feet, and prints each hit's innermost verified data object (RE-153) and the `f64`s around it.

## 3. Result

Eleven distinct inner diameters, from 7.48 mm (a 9.52 mm pipe) to 102.26 mm (a 114.3 mm pipe). For every one:

- **The pipe's own data object holds it, 549 bytes after the object's header, followed by its outer diameter.** On pipe 441731 (Mechanical) the object starts at 2,464,442 in `Partitions/78`, and at 2,464,991 lies 0.098196 ft (29.93 mm), then 0.104166667 ft (31.75 mm), the outer diameter of its box. The offset is 549 on every pipe shown, in both models. The object's class word is `0xd68`, 3432, `RbsPipeCurve`'s tag on Revit 2025 (RE-134).
- The pipe segment's size table holds the same value in a row of nominal, inner and outer diameter: in segment 191066 (Mechanical), 0.104986877 ft (32 mm nominal), then 0.098196, then 0.104166667. A pipe's own copy saves looking its size up there.

## 4. What this changes

`InvertElevation` can be written for every pipe along an axis: the lower end of a vertical pipe; the axis of a horizontal one less half the inner diameter read at byte 549 of its `RbsPipeCurve` object; each above the pipe's storey (section 6 corrects this: above the model's zero). A sloped pipe's invert is not measured here, as RE1 has none.

## 5. Reproduce

```text
gh workflow run measure.yml --ref main -f ref=0c7e71c27eacb72a35bfefec76f9804225f0e4bd -f base=none -f probe=probe_re157_pipe_inner_diameter
```

## 6. Correction, 2026-10-30: the invert is a height above the model's zero, not above the storey

**Artefacts:** the BIM4LCA office as built and at building permit (Revit 2024, CC BY-SA 4.0) with Revit's IFC4 exports. Measured on GitHub's hosted runners with the Measure workflow, run 41109551247 (`psets_vs_ifc`: `Pset_PipeSegmentOccurrence.InvertElevation` 12 values, equal 0 before and 12 after on both files; RE1 Mechanical's 6 and Plumbing's 63 stay equal, their exports byte-identical).

Every RE1 pipe is on Level 1, at elevation 0, so sections 1 and 4 could not tell a height above the storey from a height above the model's zero. The office is the first model with pipes on a storey that is not at zero. Its 12 pipes are `Pipe Types:JSV-3`, all vertical, on `Level_01` (55,500 mm in Revit's export). Each pipe's extrusion starts 179.19 mm below the storey's placement and runs up 3,679.19 mm, so its lower end is at 55,320.81 mm. `Pset_PipeSegmentOccurrence.InvertElevation` is `55320.806855210511` on all 12, in the file's millimetres: the height of the lower end above the model's zero. Above the storey it would be -179.19 mm.

rvt-rs therefore writes a pipe's invert as the height it works out in the model's frame, without taking the storey's elevation off (the step `pipe_inverts_above_storeys` took is gone). On RE1 nothing moves: the storey those 69 pipes stand on is at zero.

A vertical pipe's lower end needs no inner diameter, so it is written on a Revit 2024 pipe too, where the diameters are not read (#869). A horizontal pipe still needs the diameter, and a 2024 one still gets nothing: no model has one to measure.
