# RE-168: the leading slot of an element record's reference list is its phase

**Issue:** #228. **Backlog:** B56. **Status:** positive on four of five files; RE1 Architecture's records lead with something else.

**Evidence:** Measure runs 37220480957 (`examples/probe_re168_element_3.rs`) and 37220915687 (`examples/probe_re168_phase_slot.rs`), on Core Interior (Revit 2024) and the four RE1 models (Revit 2025).

## 1. The question

RE-155 found that the first slot of an element record's reference list (RE-23) is 3 on 17,543 of Core Interior's 22,339 records and on most RE1 MEP records, and never on RE1 Architecture's. RE-161 ruled out a workset and a parameter value, and found ElementId 3 declared by every file's ElemTable with no name entry.

## 2. ElementId 3

Each file's data objects (RE-153) of ElementId 3:

| file | class | name |
|---|---|---|
| Core Interior | `ProjectPhase` | New Construction |
| RE1 Mechanical | `ProjectPhase` | New Construction |
| RE1 Architecture | `FillPatternElem` | Solid fill |

## 3. The phases and the first slot

The `ProjectPhase` objects of each file, and the first slot's commonest values:

| file | phases | first slot |
|---|---|---|
| Core Interior | 1 Existing, 3 New Construction | 3 on 17,544 records |
| RE1 Mechanical | 1 Existing, 3 New Construction | 3 on 42, 1 on 1 |
| RE1 Plumbing | 1 Existing, 3 New Construction | 3 on 126, 1 on 3 |
| RE1 Electrical | 1 Existing, 3 New Construction | 1 on 3, 3 on 2 |
| RE1 Architecture | 12589 Existing, 86961 and 460975 New Construction | Level 311 on 90, Existing (12589) on 3 |

Where a file's phases are 1 and 3, the leading slot is one of them on all but a handful of records: the element's phase, by all appearances its Phase Created, since no record leads with a demolition. The leading 3 is "New Construction".

RE1 Architecture, created from another template, numbers its phases 12589, 86961 and 460975, and most of its records lead with Level 311 (RE-155's finding); 3 lead with its Existing phase. Where its records keep their phase is not measured.

## 4. Not established

- Whether the slot is Phase Created or the phase the element is shown in. No file here demolishes anything, so the two are not told apart.
- Whether Revit's export leaves out elements by phase. On the RE1 MEP models none of the 7 records leading with Existing is in Revit's export, but what those 7 records are is not measured, and RE1 Plumbing's 442378 (B38), which Revit's export lacks, has no record in the partitions' leading chain, so its first slot is not read here.
- Core Interior's in-export counts: the probe did not find its slim export beside the model in Measure, so its counts are not given.
