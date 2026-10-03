# RE-160: an element's phases are not parameter entries in its data object

**Date:** 2026-10-03
**Issues:** #328; backlog items B38 and B46
**Artefacts:** Core Interior (Revit 2024) and the four RE1 models (Revit 2025), with Revit's own IFC4 exports. Measured on GitHub's hosted runners with the Measure workflow, run 37146818151.
**Probe:** `examples/probe_re160_phase_entries.rs`.
**Status:** negative.

## 1. The question

rvt-rs writes RE1 Plumbing's `224200-FIXT-Rough In Water Closet Tank` 442378, a sanitary terminal on Level 1, which Revit's IFC4 export of the same file does not hold. Revit exports the model's other six rough-in fixtures. Snowdon's #328 slabs are left out by phase. Do the elements Revit leaves out differ in their Phase Created or Phase Demolished?

## 2. Method

For every record of every partition's leading chain (RE-35) whose frame decodes, the probe reads the `u32` after the BuiltInParameter ids `PHASE_CREATED` (-1012100) and `PHASE_DEMOLISHED` (-1012101) in the element's own verified data objects (RE-153). It tallies the pairs by category and by whether Revit's export holds the element's `Tag`.

## 3. Result

- **No record on any model carries either entry**: 22,339 on Core Interior and 152, 50, 136 and 13 on the RE1 models. An element's phases are not parameter entries in its data object. They are the native element fields `m_createdPhaseId` and `m_demolishedPhaseId` (`native_element`, `native_metadata`), which rvt-rs reads only on Revit 2023, 2024 and 2027.
- **442378 is not a record of the partitions' leading chains.** Of RE1 Plumbing's 136 chain records, 9 have a `Tag` Revit exports (6 pipe fittings and 3 pipes); the fixtures, 442378 among them, reach rvt-rs's export by another path.

## 4. What this changes

Why Revit leaves 442378 out stays open. Testing the phase hypothesis needs the native element fields on Revit 2025 (B46). The water closet tank is the one rough-in fixture Revit leaves out, which also fits a design-option or view-visibility cause; neither is measured here.

## 5. Reproduce

```text
gh workflow run measure.yml --ref re/re160-phase-entries -f base=none -f probe=probe_re160_phase_entries
```
