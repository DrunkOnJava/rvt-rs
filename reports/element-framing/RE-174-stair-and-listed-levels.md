# RE-174 — Stair Base and Top Level, and the Level named in an element's id lists

**Date:** 2026-10-13
**Follows:** RE-172.
**Artefacts:** Autodesk's public Revit 2024 samples with Revit's own IFC4 export as the oracle.
**Clean-room provenance:** public Autodesk Revit 2024 sample files, Revit's own IFC4 export as the oracle, and exported symbol names only. No SDK source and no decompiled or disassembled code.
**Result:**
- **Stairs.** A stair's association record (`d3 02 01 00 00 00 · u32 id · u32 0 · u32 0xffffffff · …`) holds its Base Level as a `u32` at ElementId + 115, 121, 129 or 137 (record variant) and its Top Level exactly 32 bytes later. Unlike other records, the `u32` after the level field is not zero. Written as `Base Level` and `Top Level` only when exactly one variant offset gives two different recovered Levels. Spike 255: 44 of 44.
- **Beams, braces, MEP terminals and other elements whose association level is `0xffffffff`.** The Level is a member of a sorted counted-`u64` id list of the element record: the second list after the list that holds the element's own id for a free-standing beam, the first list for a beam hosted on a beam system. The existing reference-list reader is used (`decode_reference_list` at the record's `+0x88`, then the list that follows). The Level is the second list's if it names any recovered Level, else the first's, and is written only when exactly one is named. Two candidates (the record also stores the level below) write nothing: the highest-elevation tiebreak of the spike is not applied. Beams and members taken this way are `Reference Level`; other classes `Level`.
- **Additive.** Storeys, `LevelElementId` and every property RE-172 wrote are unchanged on all files (checked).

Measured against Revit's IFC4 export on the six Autodesk samples that main exports, per class (written / Revit has it / equal to Revit); every written value that Revit also has is equal:

| file | IfcStair Base + Top Level | IfcBeam Reference Level | other (Level) |
|---|---:|---:|---:|
| racadvanced | 5 / 5 / 5 | 35 / 35 / 35 | proxies 40 / 40 / 40 |
| racbasic | 3 / 3 / 3 | | proxy 1, light 2, sanitary 3, all equal |
| rstadvanced | | 395 / 395 / 395 | |
| rstbasic | 1 / 1 / 1 | 339 / 339 / 339 | |
| rmeadvanced | 0 of 4 | | proxies 512, lights 307, air terminals 28, sanitary 11, all equal |
| Technicalschool | | 35 / 35 / 35 | proxies 40, all equal |

Not written although Revit has the property (fail closed, or not decoded): beams racadvanced 2 of 37, rstadvanced 2 of 375, rstbasic 31 of 370, Technicalschool 2 of 37; rmeadvanced stairs 4 of 4. Spike 255 found 2, 2 and 17 beams with two candidate Levels in racadvanced, rstadvanced and rstbasic; the rest of rstbasic's are beams whose Level is not a recovered one.

Where rvt-rs writes a property Revit does not have: racbasic 2 railings `Base Level` and 1 roof `Base Level`.

**Limits.** Revit 2024 only. DACH and Snowdon export no elements on main. The list carrier is read at the fixed `+0x88` offset, so records of another layout (DACH's) are not covered.
