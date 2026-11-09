# RE-381: a profile wall's pocket opening carries its wall's type name and its profile's area

**Date:** 2026-11-09
**Issues:** #1193 (W7: the opening Reference is one of the values it lists); follows RE-380 (#1607, whose pairing of the terrain opening counted three of its Revit values as missing), RE-237 (the openings of a host's own geometry carry the host's Reference) and RE-321 (a wall's cut openings)
**Artefacts:** the BIM4LCA office's terrain files (Revit 2024, CC BY-SA 4.0, with Revit's IFC4 export, which has base quantities), Autodesk's `2024_Core_Interior.rvt` with Revit's export of it (base quantities off, RE-242), and Autodesk's Snowdon Towers Architectural with Revit's own IFC4 export (local only, no licence, never committed; counts and equalities only here)
**Probe and scorer:** `probes/examples/probe_1607_wall_profile_lines.rs` (commit `80808447`) and `tools/re/pocket_opening_values_vs_ifc.py`, which prints, for each opening whose Tag is one of its host wall's sketch lines, the host's `Reference`, the opening's property and quantity sets, and the area of its profile; Measure run [42648274662](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42648274662) on the five Revit 2024 files with Revit's export (the scorer on the probe's commit `7d432ab5`)
**Status:** positive for the Reference pair, on 15 openings of three files, and for the Area on one opening that is Revit's value to its full precision.

## 1. What Revit writes

| File | Host wall | Its `Pset_WallCommon.Reference` | The opening's property sets | Its quantities |
|---|---|---|---|---|
| terrain, concrete and timber | 1387731 (type `MS-2`) | `MS-2` | `Pset_OpeningElementCommon.Reference` and `Pset_EnvironmentalImpactIndicators.Reference`, both `MS-2` | `Qto_OpeningElementBaseQuantities.Area` 7.216426890207297 m2, nothing else; the scorer's area of the opening's profile is 7.216427 m2 |
| Core Interior | 55840 | none (the export's setup writes no property set on any wall) | none | none (no base quantities either) |
| Snowdon Towers Architectural | 7 walls | each its own type name | on each of the 13 openings both sets, equal to the host's (26 of 26) | none (the export writes no base quantities) |

So the Reference of a profile wall's opening is its wall's, in both sets, where Revit's export writes property sets at all. Core Interior's export, whose setup writes none on any wall or opening and no base quantities, shows neither the pair nor the Area, so it cannot say what Revit writes there. The Area is the polygon's area, the only quantity on the one opening that has quantities, where Revit's door and rectangular openings carry Depth, Width and Height (RE-244, RE-321).

## 2. What rvt-rs writes

`step_writer.rs`, in the loop of a wall's profile openings (RE-151, RE-380): the Reference of the host (`host_references`, the one RE-237 reads for an opening of a floor's sketch void) in both sets where the host has one; and, where the host wall has base quantities (a `Qto_WallBaseQuantities.Width`, the test RE-244 uses for a door's opening), `Qto_OpeningElementBaseQuantities.Area` as the area of the opening's outline. `rvt-ifc --no-base-quantities` leaves the wall's quantities out and so this one. The count of `Pset_` values the exporter reports (`decoded.parameter_value_count`) holds the two Reference values of each written profile opening. On Core Interior the opening of wall 55840 gains both Reference values and the Area, as every other element of Core gets the sets Revit's export of it does not write; the census counts them among the values Revit does not write, and the two committed Core Interior witness observations count them (`IFCPROPERTYSET` +2, `IFCELEMENTQUANTITY` +1, `IFCQUANTITYAREA` +1, `IFCRELDEFINESBYPROPERTIES` +3).

## 3. Measured

@@MEASURE@@

## 4. Not shown

- **The Area rests on one opening** (the same wall in both terrain files). The office's other `Area`-bearing openings (as built 5, all missing) are of other kinds and are not read here.
- **Volume, Width, Height and Depth** are not written on this opening by Revit's export, on that one opening; a pocket opening on another file that carried them would say that the rule is the profile's shape, not the kind.
- **The Reference where the host has none** is not shown: no export that writes property sets has a profile wall without a Reference, so that rvt-rs writes none then is its own rule.

## 5. Reproduce

```bash
gh workflow run measure.yml --ref <branch with the scorer> -f ref=7d432ab5aa7566ceac22d5841fdc1fbf05de88c8 -f base=none \
    -f nordic=true -f models="core-interior nordic-office-as-built nordic-office-permit nordic-office-terrain-*" \
    -f scorers=none -f probe=probe_1607_wall_profile_lines -f probe_scorer=pocket_opening_values_vs_ifc
```

Scoring Snowdon Towers needs the local copies of both files (RE-100): `python3 tools/re/pocket_opening_values_vs_ifc.py <probe output> "Snowdon Towers Sample Architectural_IFC4.ifc"`.
