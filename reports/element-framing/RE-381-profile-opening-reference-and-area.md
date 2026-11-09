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

**Measure run [42649417280](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42649417280)** (unfiltered: `nordic`, `r2025`, `samples`, `racadv`, `bimcollab` and `holdout`; 118 models; the code commit `29cdfdb5` against main's `04f21f71`, scorers from the branch). The commit after it changes witness observations and docs only. Its `same-project gates` check run on `29cdfdb5` is success: 15 samples held to their earlier saves, and the Areas on 4 saves and on the 2021 save.

- **Five exports move, and no other:** Core Interior, the two terrain files, `rac_basic` and Snowdon Towers Architectural (all Revit 2024), the five files with a profile wall's pocket opening whose host's type name is read. The other 113 are byte for byte main's: the office as built and at permit, the 24 Revit 2025 models, the 2025 to 2027 saves of the samples and the 18 held-out models (their scrubbed results, `ifc.sha256` among them, are identical on both sides). Across the 118 models the only other differences are the timings, `schema.txt` (the count of property sets), `decoded.parameter_value_count` with the info dump that carries it, and the three scorers below on the three models with Revit's export. So no scorer of a wall's body, top, height, ends or layers, or of an opening's box or volume, moves on any model.
- **`decoded.parameter_value_count` moves by the Reference values written, two for each opening:** Core Interior 4180 to 4182, each terrain file 84 to 86, `rac_basic` 1603 to 1611 (4 openings), Snowdon Towers 22542 to 22568 (13 openings).
- **The terrain files, concrete and timber** (Revit's export has property sets and base quantities):

  | Scorer | Before | After |
  |---|---|---|
  | `psets_vs_ifc`: equal / different / missing / values Revit does not write | 82 / 0 / 31 / 2 | 84 / 0 / 29 / 2 |
  | `Pset_OpeningElementCommon.Reference` equal | 0 of 1 | 1 of 1 |
  | `Pset_EnvironmentalImpactIndicators.Reference` equal | 21 of 23 | 22 of 23 (the one left is a railing's) |
  | `qtos_vs_ifc`: equal / different / missing / quantities Revit does not write | 16 / 0 / 29 / 0 (timber 16 / 1 / 28 / 0) | 17 / 0 / 28 / 0 (timber 17 / 1 / 27 / 0) |
  | `Qto_OpeningElementBaseQuantities.Area` equal | 0 of 1 | 1 of 1 |
  | `unfilled_openings_vs_ifc`: the two Reference properties equal | not listed | 1 each |

  The opening's box (within 0.01 ft), volume (within 0.01 %), Name and PredefinedType pairings are as before, "different" does not move, and the timber file's one different quantity is as it was.
- **Core Interior** (Revit's export writes no property set and no base quantity on any wall or opening, its setup): the opening of wall 55840 gets both Reference values and the Area, which the scorers count among what Revit's export does not write (`psets_vs_ifc`: values not scored 4177 to 4179; `qtos_vs_ifc`: quantities rvt-rs writes 2553 to 2554, with `Qto_OpeningElementBaseQuantities.Area: 1` listed). The unfilled-openings scorer no longer prints the two Reference lines for it, because they counted two empty values as equal; its 63 openings still pair as before. The committed witness observations are this Measure's own for the code commit, byte for byte (observation hash `5afc255e…`: `IFCPROPERTYSET` 6514 to 6516, `IFCPROPERTYSINGLEVALUE` 28066 to 28068, `IFCELEMENTQUANTITY` 932 to 933, `IFCQUANTITYAREA` 642 to 643, `IFCRELDEFINESBYPROPERTIES` 7042 to 7045), and both witness replays pass in CI.
- **`rac_basic` and Snowdon Towers Architectural** have no Revit export in the corpus: their 4 and 13 pocket openings gain the pair (`schema.txt`: sets 991 to 999 on 502 to 506 objects, and 13529 to 13555 on 6265 to 6278). Locally, on this run's CI-built binaries and the local copies of Snowdon's files: its 13 openings also each gain the Area (all 13 hosts carry a `Qto_WallBaseQuantities.Width`), which no export can check because Revit's export of Snowdon writes no base quantities; against that export (counts only) the unfilled-openings scorer's two Reference properties go from 56 to 66 equal, the 10 more being the pocket openings that pair by Tag, and its 3 openings written that Revit's export does not are as before (wall 698364's, RE-380 section 5); `wall_quantities_vs_ifc` is identical on both sides for Snowdon and for Core.
- **What the gates exercise.** The rule is exercised by the 5 of 118 models that move, and scored against Revit's export on 3 of them, by one opening that has quantities (the terrain's, in two files). The same-project gates name one of the five, `rac_basic`: its 2024 save gains its 4 openings' Reference values, its 2025 to 2027 saves do not move, and the gates pass.

## 4. Not shown

- **The Area rests on one opening** (the same wall in both terrain files). It is also written on Core Interior's and Snowdon Towers' pocket openings, where Revit's exports write no base quantities, so it is checked on neither. The office's other `Area`-bearing openings (as built 5, all missing) are of other kinds and are not read here.
- **Volume, Width, Height and Depth** are not written on this opening by Revit's export, on that one opening; a pocket opening on another file that carried them would say that the rule is the profile's shape, not the kind.
- **The Reference where the host has none** is not shown: no export that writes property sets has a profile wall without a Reference, so that rvt-rs writes none then is its own rule.

## 5. Reproduce

```bash
gh workflow run measure.yml --ref <branch with the scorer> -f ref=7d432ab5aa7566ceac22d5841fdc1fbf05de88c8 -f base=none \
    -f nordic=true -f models="core-interior nordic-office-as-built nordic-office-permit nordic-office-terrain-*" \
    -f scorers=none -f probe=probe_1607_wall_profile_lines -f probe_scorer=pocket_opening_values_vs_ifc
# The unfiltered run of section 3 (scorers from the branch that holds them; the code commit against RE-380's merge):
gh workflow run measure.yml --ref <branch with the scorers> -f ref=29cdfdb57621330781cb760a10f6bcec927e16b0 \
    -f base=04f21f714c59b77714ef38cbec21e01fbf936895 -f nordic=true -f r2025=true -f samples=true -f racadv=true \
    -f bimcollab=true -f holdout=true
```

Scoring Snowdon Towers needs the local copies of both files (RE-100): `python3 tools/re/pocket_opening_values_vs_ifc.py <probe output> "Snowdon Towers Sample Architectural_IFC4.ifc"`.
