# RE-245 — The wall a door or window is set in

**Date:** 2026-10-26
**Issues:** #227 (openings), #1016 (their quantities); follows RE-23, RE-85 and #439 (the host from the reference list), RE-215 (the native pass over doors and windows)
**Artefacts:** the BIM4LCA office as built and at building permit (Revit 2024, Revit's IFC4 exports), Core Interior (Revit 2024), RE1 Architecture and puzzbobb's R2025 models (Revit 2025), as Measure fetches them
**Runs:** Measure [40318608193](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40318608193) (`tools/re/opening_depth_list.py`, the hosts of rvt-rs's and Revit's openings) and [40319840390](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40319840390) (this change against `main`, unfiltered)
**Result:** positive: a door's or window's current native record names its host in `m_hostId`; where its reference list names several walls, the nearest one rvt-rs took is not always it.

## 1. Before

A door or window is set in a wall its partition element record's reference list names (RE-23): the nearest wall below its own ElementId, then the nearest above (#439, RE-85), not a curtain wall. That holds wherever the list names one wall. On the office as built 11 of the 195 doors whose openings rvt-rs writes with a host are set in another wall than the one Revit's export voids (`IfcRelVoidsElement` ∘ `IfcRelFillsElement`), none of them a stacked wall's member: door 1728679 in wall 1728013 (`Basic Wall:VS-4`, 0.2 m) where Revit's opening voids 1727582 (`VS-6`, 0.3 m), and ten more like it.

## 2. The host in the native record

The native pass RE-215 already makes over every door and window record (`native_symbol_cuts::instance_cuts_and_symbols`) reads its current record (RE-216), whose `FamilyInstance` object names the element hosting it in `m_hostId`. rvt-rs now takes that wall where it differs from the reference list's and is an exported wall other than a curtain wall; a door whose native host is not such a wall keeps the reference list's.

## 3. Measured

This change against `main` at 8bc87a5b, unfiltered (run 40319840390, `-f nordic=true -f r2025=true`, every scorer). Only the two office exports move:

| office | doors and windows set in the wall Revit fills (`opening_hosts_vs_ifc`) | in another wall | openings within 0.01 ft of Revit's (`openings_vs_ifc`) | over 1 ft | type GlobalIds agreeing (`type_objects_vs_ifc`) |
|---|---:|---:|---:|---:|---:|
| as built, `main` | 452 | 11 | 174 | 19 | 12,281 of 12,603 |
| as built, this change | 463 | 0 | 185 | 8 | 12,288 |
| at permit, `main` | 446 | 11 | 174 | 17 | 12,265 of 12,648 |
| at permit, this change | 457 | 0 | 185 | 6 | 12,267 |

The 11 openings are now Revit's within 0.001 ft (`opening_boxes_vs_ifc`), and 7 more doors' type GlobalIds agree, since a door's type GlobalId hangs on its flip against its host wall (B72). Every other scorer's output is as on `main`, and Core Interior (138 of 138), RE1 Architecture (4 of 4) and the R2025 models export byte for byte as before, their doors and windows already in Revit's walls.

## 4. Reproduce

```bash
gh workflow run measure.yml --ref main -f ref=main -f base=none -f nordic=true \
  -f models="nordic-office-*" -f scorers="opening_hosts_vs_ifc openings_vs_ifc opening_boxes_vs_ifc"
```
