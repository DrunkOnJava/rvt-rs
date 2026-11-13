# RE-299 — A door or window only cuts its host where its type's native map is empty

**Date:** 2026-11-02
**Issues:** #1247 (73 doors of a Revit 2024 project written as `IfcOpeningElement`), #309, #227; refines RE-84
**Probe:** `probes/examples/probe_re299_unset_map_doors.rs`; scoring `tools/re/void_only_vs_ifc.py` (now counts the probe's further columns)
**Run:** Measure [41538550670](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41538550670), the probe at 1fcad647 (then named `probe_re298_unset_map_doors`, the same code), every reference model, the BIM4LCA office, puzzbobb's R2025 models, Autodesk's 36 sample projects and the held-out models
**Result:** positive. RE-84 decides that a door's or window's type draws no geometry from a byte scan of the type's value block, which misses a map whose values it cannot name. The type's native `m_geomTag2MaterialId` is the map itself: it is empty on every door and window Revit exports as its opening alone, so a type whose native map holds entries is no longer taken as drawing nothing.

## 1. The miss

RE-84 writes a hosted door or window as its opening alone where its type has a parameter value block and RE-82's scan of it finds no geometry-material map. The scan (`partition_type_materials::map_at`) takes a map only where every value is either unset or a material whose name `scan_material_names` read, and at least one is a material. Two kinds of map fail that and leave the type with an empty list, which `attach_type_materials` reads as "draws no geometry":

- a map whose every value is unset (−1): a family whose geometry takes no material of its own, as one drawn entirely in its category's or subcategories' materials does (RE-149);
- a map naming a material the name scan dropped, as it drops one whose copies in different partitions hold different names.

RE-149 already lifts the flag from columns and products whose block opens with an all-unset map (`unset_material_types`), and gives them `<Unnamed>`. Doors and windows were left out of that on purpose, and the probe shows why that byte signal cannot decide them either: it fires on RE1's CasedOpening and on Snowdon Towers' Door-Opening, which Revit exports as openings.

## 2. Measured

`probe_re299_unset_map_doors` prints every door and window record with its type's scanned map count (RE-84's column), whether the type's block opens with an all-unset map, its saved-graphics triangles, its family's `shows_unset` (RE-204) and its native map as entries and unset entries. Every door and window whose type the scan reads as having no map:

| file | elements | Revit's export | all-unset block (RE-149) | saved graphics | native map |
|---|---:|---|---|---|---|
| RE1 Architecture (2025): CasedOpening | 1 | opening only | yes | none | 0 entries |
| Snowdon Towers architectural (2024): Door-Opening | 9 | opening only (RE-84, local export) | yes | none | 0 entries |
| Snowdon Towers architectural (2024): Schematic Opening Cut | 38 | opening only (RE-84, local export) | no | none | 0 entries |

Snowdon Towers is read here as one of Autodesk's sample projects (`research/autodesk-sample-projects.tsv`); Revit's export of it is local only, and RE-84 measured these 47 against it. No other door or window on Core Interior (138), the office as built (537) and at permit (533), RE1's four models, the R2025 models or the other 35 sample projects has a type the scan reads as having no map, so none is moved: every one of them is exported as the element, as in Revit's exports where there is one (Core Interior 138 of 138, office 537 of 537 and 533 of 533, R2025 24 of 24, RE1 Architecture 5 of 5).

The native map holds entries on every door and window type Revit exports as the element (Core Interior's 138, all naming materials only; the office's 431 and 425 that the scan leaves out, whose maps mix materials and unset values), and on none whose instances it exports as openings. The saved graphics agree: none of the 48 openings-only has any.

## 3. What rvt-rs does

`refine_type_materials` already reads each column, door, window and product type's native record (RE-203); `native_type_materials::geometry_maps` now also returns the types whose `m_geomTag2MaterialId` holds any entry, with whether every value is −1 and the type's family. A type among them that the scan left with an empty list is dropped from the list instead, so it is a type whose materials are not read rather than one that draws nothing. Its doors and windows are written as the elements, sized and named as other doors and windows are (the flag also gated RE-215's cut-loop sizes, the opening-index sizes and RE-154's type name), with no type material: what Revit's export gives such a door or window is not measured, so nothing is written for it.

One case keeps the flag: a map whose every value is −1, of a family whose own document shows none of that geometry (`native_family_forms`, RE-204: every such form hidden or void, or no form and no nested instance). Such a type draws nothing visible, as the scan read it. No measured file holds one. Where the native record is not read, the scan decides, as before; where the family is not read, a map with entries is geometry and the type draws (the correlation of section 2; no measured file holds an all -1 map).

Every export of the corpora above is unchanged: the 48 openings-only keep an empty native map, and no other door or window had the flag. The positive case is #1247's project, which is not public.

## 4. Not claimed

- What material Revit's export gives a door or window whose map is all unset (RE-149 measured `<Unnamed>` on products only).
- Whether #1247's 73 doors have entries in their native map: the reporter's run of the probe decides it.
- The all −1, hidden-or-void case of §3 is reasoned from RE-204, not measured: no file in the corpora has such a door or window type.
- That a type with an all -1 map and an unread family draws is reasoned from section 2, not measured.

## 5. Reproduce

```bash
gh workflow run measure.yml --ref research/1247-unset-map-doors -f ref=1fcad647 -f base=none \
  -f probe=probe_re298_unset_map_doors -f probe_scorer=void_only_vs_ifc \
  -f nordic=true -f r2025=true -f samples=true -f holdout=true -f scorers=none
```

Where the branch is gone, push 1fcad647 to a branch first. The probe has the name `probe_re299_unset_map_doors` from 43bcb6b5 on; once this lands, `--ref main` carries the scorer and `-f ref=<the merge commit>` the probe.
