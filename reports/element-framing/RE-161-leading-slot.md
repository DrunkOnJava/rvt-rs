# RE-161: the leading 3 of a reference list is ElementId 3, not a workset or a parameter

**Date:** 2026-10-03
**Issues:** #228; backlog item B39
**Artefacts:** Core Interior (Revit 2024) and the four RE1 models (Revit 2025). Measured on GitHub's hosted runners with the Measure workflow, runs 37146990738 and 37147380469.
**Probe:** `examples/probe_re161_leading_slot.rs`.
**Status:** narrowed; not attributed.

## 1. The question

RE-155 found that an element record's reference list starts with 3 on 79% of Core Interior's records and on most RE1 MEP records, and never on RE1 Architecture's, whose first slot is a Level on 90 of 152. What is the 3?

## 2. Method

For every record whose frame decodes, the probe takes the first slot. It looks, in the element's own verified data objects (RE-153), for a BuiltInParameter id whose next `u32` or `u64` equals that slot. It also prints the file's worksharing state, and for the commonest first-slot values, whether the ElemTable declares them and their name entry (RE-38).

## 3. Result

| model | records | worksharing | first slot (records) |
|---|---:|---|---|
| Core Interior | 22,337 | Not enabled | 3 (17,544), 660 (1,410), ... |
| RE1 Architecture | 152 | Not enabled | 311 (90), 8482 (9, the `Rectangular Mullion` type), ... |
| RE1 Mechanical | 50 | Not enabled | 3 (42) |
| RE1 Plumbing | 136 | Not enabled | 3 (126) |
| RE1 Electrical | 13 | Not enabled | 1 (3), 3 (2) |

- **Not a workset.** No file is workshared, so no element carries a workset id, yet the 3 is in most lists.
- **Not a parameter value.** No BuiltInParameter entry in any element's own object holds the first slot's value, except Room Number (-1006901) on 95 of 286 rooms. That match is a coincidence: a three-character room number's length prefix is 3.
- **ElementId 3 is declared by the ElemTable on every file and has no name entry.** It is one of the document's first internal elements, the same id in all five files.

So where the first slot is 3, it most likely refers to ElementId 3, an internal element whose class is not read here. RE1 Architecture's records put a Level or a type in that slot instead, which suggests the slot's meaning depends on the record's class (RE-155: the slots follow the class's fields).

## 4. What this changes

Nothing in code. #228's remaining open part is which internal element ElementId 3 is. Reading its class needs its own record, which is not in the partitions' leading chain.

## 5. Reproduce

```text
gh workflow run measure.yml --ref re/re161-leading-slot -f base=none -f probe=probe_re161_leading_slot
```
