# RE-162: an MEP system is an element of its own, and its members hold its id

**Date:** 2026-10-03
**Issues:** #528; backlog item B33
**Artefacts:** RE1 Mechanical, RE1 Plumbing and RE1 Electrical (Revit 2025, MIT) with Revit's own IFC4 exports. Measured on GitHub's hosted runners with the Measure workflow, runs 37148070432, 37148641247 and 37148993751.
**Probe:** `examples/probe_re162_mep_systems.rs`.
**Status:** positive.

## 1. The question

Revit's IFC4 exports of the RE1 MEP models write one `IfcSystem` per MEP system: 5 on Mechanical (`Mechanical Supply Air 1`, `Mechanical Return Air 1`, `OTH 1` to `OTH 3`), 8 on Plumbing (`CW 1`, `HW 1`, `SAN 1` to `SAN 6`) and 13 on Electrical (circuits, three of them named `1`, and one `<unnamed>`). Each groups its members by `IfcRelAssignsToGroup`. rvt-rs writes none. Where are a system and its members stored?

## 2. Method

For each system of the reference export the probe looks for its name as `u32 n · UTF-16 × n` in every partition and takes the verified data object (RE-153) holding each hit as a candidate. For each candidate it counts how many of the system's members (by `Tag`) have a data object of their own holding the candidate's id. A second pass takes every data object of the classes the first pass found, and lists its strings, the objects holding its id and the Revit system those overlap most. A third dumps the bytes of each electrical system around its strings.

## 3. Result

**Each system is one data object of a system class.** Mechanical's are `RbsHvacSystem` (0xd52) and `RbsPipingSystem` (0xd7a), Plumbing's `RbsPipingSystem`, Electrical's `RbsElectricalSystem` (0xd40). Each has no name entry (RE-38): the name is in the object.

**Its members are the elements whose own data objects hold its id**, all 26 systems:

| System | Revit's members | holding the id | others holding it |
|---|---|---|---|
| Mechanical Supply Air 1 | 58 | 58 | none |
| Mechanical Return Air 1, OTH 1 to 3 | 8, 2, 2, 8 | all | none |
| CW 1 | 59 | 59 | a FamilyInstance, a PostedWarningElem |
| HW 1 | 52 | 52 | a PostedWarningElem |
| SAN 1 | 3 | 3 | a FamilyInstance |
| SAN 2 to 6 | 4 each | all | none |
| the 13 circuits | 9, 5, 2, 2, 2, 7, 2, 2, 2, 4, 7, 6, 4 | all | none |

A warning element is not exported, so restricting members to exported elements leaves the two FamilyInstances as the only possible over-assignments.

**Names.** In an HVAC or piping system the first string of printable ASCII is its name: `Mechanical Supply Air 1` at byte 291 of 428159, `OTH 1` at 143 of 441733, `CW 1` at 233 of 443721; 13 of 13. Shorter strings before it, such as a count of 1 followed by half an ElementId, are not ASCII.

An electrical system's name is its circuit number, which is not its first string on every circuit. After the first `ff` × 8 `01` in the object, 20 bytes on, comes a `u32` count of 32-byte entries, then the number as `u32 n · UTF-16 × n`:

```text
930277 (circuit 1, no entry):   183 00000000            187 01000000 3100              "1"
460097 (circuit 1, one entry):  183 01000000 24080000…  219 01000000 3100              "1"
490333 (circuit 3,5):           183 01000000 2b080000…  219 03000000 33002c003500      "3,5"
1021814 (no number):            167 01000000 16080000…  203 00000000                   ""
```

13 of 13. Circuit 1021814's number is empty, and Revit's export names it `<unnamed>`, the name Revit gives a circuit with no number. Its load name, `Other Room RE-4, RE-3`, is its first string, which is why the first-string rule fails on electrical systems. Later strings are the load name (`Lighting Room RE-6, RE-12, …`, `AHU`), the panel's circuit and its rating.

## 4. What this changes

Every MEP system can be written: find the objects of `RbsHvacSystem`, `RbsPipingSystem` and `RbsElectricalSystem`, name each as above, and group in it the exported elements whose objects hold its id. Two FamilyInstances on Plumbing hold the id of a system Revit does not put them in, or are not exported; the export test against Revit's (`tests/mep_systems.rs`) decides.

## 5. Reproduce

```text
gh workflow run measure.yml --ref re/re162-mep-systems -f base=none -f probe=probe_re162_mep_systems
```
