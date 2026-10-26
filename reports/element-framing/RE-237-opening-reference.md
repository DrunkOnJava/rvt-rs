# RE-237 — Which openings Revit's export gives `Pset_OpeningElementCommon`

**Date:** 2026-10-25
**Issues:** #35 (parameters), W7 of [`docs/release-1.0-criteria.md`](../../docs/release-1.0-criteria.md)
**Artefacts:** the BIM4LCA office as built and at building permit (Revit 2024, Revit's IFC4 exports), as Measure fetches them
**Runs:** Measure [40257585007](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40257585007) (`tools/re/opening_psets_list.py`) and [40258827136](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40258827136) (this change against `main`)
**Result:** positive for openings of the host's own geometry; the others need openings rvt-rs does not write yet.

## 1. Revit's openings

| opening | count, as built | `Pset_OpeningElementCommon` |
|---|---:|---|
| filled by a window | 269 | never |
| filled by a door | 194 | never |
| unfilled, in a wall | 172 | always: `Reference` the cutting element's type (`US-8`, a curtain wall's; `VAR-2.PPP`, a recessed cabinet's; `O-4.8x21`, a door cutting a second wall), and `FireExit` where that is a door |
| unfilled, in a floor | 40 | always: `Reference` `VP-1`, the floor's own type |
| unfilled recess, in a wall | 8 | always: the cutting element's type |
| unfilled, in a door | 4 | always: the door's type |

revit-ifc (CLEANROOM.md source 5) creates an opening that nothing fills with the cutting element as the element its properties come from where the opening's GUID is the cutter's, and a door's or window's own opening with none. An opening of the host's own geometry (a floor's sketch void, RE-151; the hatch openings of #824, RE-225) has no cutter, and its `Reference` is the host's type: `VP-1` on the floor voids, `YP-1` on roof 2991742's hatch openings.

## 2. What rvt-rs does

The writer gives each opening of its host's own geometry (`IfcEntity::VoidOpening`, `IfcEntity::HostedCutOpening`) a `Pset_OpeningElementCommon` holding its host's `Reference`. The diagnostics count these values with the rest (`parameter_value_count`). Openings cut in a wall by a curtain wall, a cabinet or a second door, which Revit's export writes and rvt-rs does not, are left for their geometry (W4, W6).

## 3. Measured

`psets_vs_ifc`, this change against `main`:

| file | `Reference` equal | different | on openings Revit's export does not write |
|---|---:|---:|---:|
| office as built | 28 of 168 | 0 | 9 |
| office at permit | 28 of 175 | 0 | 8 |

The 28 are 27 floor voids and one hatch opening. The rest of Revit's 168 are on openings rvt-rs does not write (§2). Core Interior's 62 floor voids carry it too; its reference export writes no property sets. Einhoven, RE1 and the R2025 models export byte for byte as before.

## 4. Reproduce

```bash
gh workflow run measure.yml --ref main -f ref=main -f base=none -f nordic=true \
  -f probe=basics_inspect -f probe_scorer=opening_psets_list
```
