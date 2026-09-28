# RE-85 — A door's or window's host is the wall its reference list names

**Date:** 2026-09-27
**Issue:** #439 (host mismatches), #222 (RE-23 host binding)
**Probe:** `examples/probe_re85_opening_host_candidates.rs`; scoring `tools/re/opening_hosts_vs_ifc.py`

**Result:** positive. On Snowdon Towers, 192 of the 194 doors and windows Revit fills an opening for now take the wall that opening voids (107 before), and none takes a wall Revit does not cut. Core Interior and RE1 are unchanged.

## What RE-23 missed

RE-23 bound a door or window to the reference slot immediately before its own ElementId in its record's first counted list (`+0x88`), accepted when that slot is an exported wall. On Core Interior that is exact (138 of 138). On Snowdon Towers only 107 of 110 matched, and 84 more doors and windows that Revit fills got no host.

The list is in ascending ElementId order. The slot before the record's own id is only the nearest smaller id, so:

- a host whose ElementId is larger than its door's comes after the door's own id (the 63 Snowdon doors and windows, e.g. window 631497 lists `…, 593176, 631497, 674522, …`, host 674522);
- families that list their type, such as Door-Opening and Schematic Opening Cut, put the type between the host and their own id (door 1290306: `…, 787288, 787328, 1290267, 1290306, …`, host 787288).

## The rule

The host candidates are the exported walls the list names, other than the record itself: the nearest below the record's own id first, then the nearest above. After curtain walls are identified (RE-46, RE-72), the first candidate that is not a curtain wall is the host. Revit's export cuts no opening for a door or window whose only candidate is a curtain wall, so such an element keeps no host.

## Measured

Per door and window record, against Revit's own export (`IfcRelFillsElement` then `IfcRelVoidsElement`):

| file | records naming one wall: Revit's host | naming several: the wall Revit fills | another wall Revit cuts | a wall Revit does not cut |
|---|---:|---:|---:|---:|
| Snowdon Towers 2024 (local only) | 235 of 235 | 3 | 3 | 0 |
| Core Interior 2024 | 273 of 273 | — | — | 0 |
| RE1 Architecture 2025 | 5 of 5 | — | — | 0 |

The export, `tools/re/opening_hosts_vs_ifc.py` on Snowdon Towers:

| | RE-23 | now |
|---|---:|---:|
| doors and windows in the wall Revit fills | 107 | 192 |
| in another wall Revit also cuts | 1 | 2 (1309129, 1528884) |
| in a wall Revit does not cut | 3 | 0 |
| no host, where Revit fills an opening | 84 | 0 |
| openings with no filler (RE-84), voiding Revit's wall | 36 | 47 of 47 |
| doors and windows not in Revit's export | 11 | 0 |

Filled openings in both exports: 110 before, 194 of Revit's 194 now. `tools/re/openings_vs_ifc.py` puts 124 of them within 0.26 ft of Revit's opening (RE-83's residuals apply).

## Not claimed

- Which wall Revit fills when an element cuts several: 2 Snowdon elements take another wall Revit also cuts.
- Revit's further openings in the other walls such an element cuts.
- Revit 2023 records (RE-81) keep their own host path.
