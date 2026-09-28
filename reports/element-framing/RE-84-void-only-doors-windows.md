# RE-84 — Doors and windows whose type draws no geometry export as openings

**Date:** 2026-09-27
**Issue:** #309 (elements not in Revit's export), #227 (openings), #439 (host binding)
**Probe:** `examples/probe_re84_void_only_families.rs`; scoring `tools/re/void_only_vs_ifc.py`

**Result:** positive. A door or window whose family type has a parameter value block with no geometry-material map is written by Revit's IFC export as its opening alone. rvt-rs now does the same wherever it binds the element to its host.

## The signal

RE-82 found that a family type's value block (RE-77) carries a map of the materials its geometry is drawn in. Some door and window families draw nothing: they only cut their host. Snowdon Towers has two such families, "Door-Opening" and "Schematic Opening Cut", and RE1 Architecture has one, "CasedOpening". Their types have a value block, and the block holds no map.

Revit's export writes each instance of these as an `IfcOpeningElement` that voids the host wall, carries the door's or window's ElementId as its `Tag`, and is filled by nothing. There is no `IfcDoor` or `IfcWindow`.

The rule reads the file's bytes, not family names: a type with a value block and no map draws no geometry.

## Measured

`probe_re84_void_only_families` lists every 2024/2025 door and window record with its type's map size. `void_only_vs_ifc.py` finds each ElementId in Revit's own export:

| file | release | type has a map: exported as the element | type has no map: exported as its opening only | exceptions |
|---|---|---:|---:|---:|
| Snowdon Towers (local only) | 2024 | 200 of 200 | 47 of 47 | 0 |
| Core Interior | 2024 | 138 of 138 | none | 0 |
| RE1 Architecture | 2025 | 5 of 5 | 1 of 1 | 0 |

Every record has a type and every type a value block, so no case falls outside the rule.

A map whose materials do not resolve to names is not read as "no map": such a type is left out rather than taken as drawing nothing.

## Export

A door or window whose type draws no geometry and whose host is bound (RE-23) is exported as an `IfcOpeningElement` (`PredefinedType` `.OPENING.`) with the element's ElementId as `Tag`, voiding the host (`IfcRelVoidsElement`), with no `IfcRelFillsElement` and no spatial containment, as in Revit's export. It is not drawn in the GLB or the plan SVG, and the export diagnostics list it under `by_ifc_type` but not among building elements.

Against Revit's export:

| file | openings written this way | same Tag and host as Revit's | doors and windows not in Revit's export, before | after |
|---|---:|---:|---:|---:|
| Snowdon Towers | 36 | 36 | 47 | 11 |
| RE1 Architecture | 1 | 1 | 1 | 0 |
| Core Interior | 0 | — | 0 | 0 |

Shape: each such opening takes #227's cut (RE-83), the element's box across and in height and the host wall's depth through it. Revit extrudes these openings 10 ft (3048 mm) through the wall, centred on it, so their boxes differ in depth by design; the cut they make in the wall is what matters. Of the 36 on Snowdon, 31 lie inside Revit's opening with the same sill and head heights (within 0.26 ft); the other 5, all in wall 1534218, keep the element's uncut box and stand up to 1.2 ft out of Revit's across the wall. RE1's one lies inside Revit's.

The 11 Snowdon elements left are void-only doors and windows RE-23 does not bind to a host: their host wall is in their reference list but not in the slot before their own ElementId (#439). Without a host there is no wall to cut, so they stay the element for now.

Update 2026-09-27 (RE-85): with hosts read from anywhere in the reference list, all 47 Snowdon ones are openings voiding the wall Revit's opening voids, and no door or window is left that Revit does not export.

## Not claimed

- Void-only doors and windows with no bound host (#439).
- Revit's further unfilled openings where one door or window cuts several walls (215 on Snowdon beyond the 47 above).
- Revit's opening profile, and its 10 ft depth: the opening is the element's box cut to the host's depth where #227's rule applies.
