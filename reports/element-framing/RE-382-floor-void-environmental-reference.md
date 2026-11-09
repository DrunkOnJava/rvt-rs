# RE-382: the opening of a host's own geometry carries its host's type name in both Reference sets

**Date:** 2026-11-09
**Issues:** #1193 (W7: the opening `Reference` is one of the values it lists); follows RE-237 (which openings carry `Pset_OpeningElementCommon`, and its `Reference`), RE-321 and RE-338 (a wall's cut openings, both sets) and RE-381 (a wall's profile openings, both sets)
**Artefacts:** the BIM4LCA office as built and at building permit (Revit 2024, Revit 24.0.5's IFC4 exports, CC BY-SA 4.0, as Measure fetches them), Autodesk's Snowdon Towers Architectural with Revit's own IFC4 export (local only, no licence, never committed; counts and equalities only here), Autodesk's `2024_Core_Interior.rvt` with Revit's export of it (no property sets, its setup)
**Runs:** Measure [42656294752](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42656294752) (the property census of the two office files with Revit's set placement, `main` at `060e8f0a`), which names the 37 and 36 values; the unfilled-openings scorer of Measure [42649417280](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42649417280) and `psets_vs_ifc` with `PSETS_MISSING=Pset_EnvironmentalImpactIndicators.Reference` on Snowdon Towers, run locally on the CI-built binaries of that run; the change measured by the unfiltered Measure [42657527737](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42657527737) (section 3) and read in rvt-rs's own export of the office by Measure [42662314183](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42662314183)
**Status:** Positive. Every opening of a floor's or roof's own geometry that Revit's export holds on the BIM4LCA office now has both reference sets in rvt-rs's, equal: 37 values as built and 36 at permit (the census holds one value per key), none different, and the census's count of values Revit does not write does not move on either file. On Core Interior, whose Revit export writes no property set, the 62 sets are values not scored. The rule rests on one project's two states and on Snowdon Towers, all Revit 2024; no export of a later release holds such an opening, and no held-out model exercises it.

## 1. What Revit writes

The openings Revit's export holds that are of the host's own geometry, with no cutter, are a floor's sketch void and the opening a face-hosted door or window cuts in its host (RE-237, RE-151, #824). RE-237 measured that each has `Pset_OpeningElementCommon.Reference`, the host's type name, and rvt-rs wrote that. The property census of the office with Revit's set placement then leaves 37 values missing as built and 36 at permit, all `Pset_EnvironmentalImpactIndicators.Reference` on `IfcOpeningElement`:

| File | Openings in a floor or roof | Their `Pset_OpeningElementCommon.Reference` | Their `Pset_EnvironmentalImpactIndicators.Reference` in Revit's export |
|---|---|---|---|
| office as built | 36 floor voids and the roof's 4 hatch openings, which share one key (the roof's ElementId): 37 keys | `VP-1` x36, `YP-1` x1, equal to Revit's (RE-237) | `VP-1` x36, `YP-1` x1, none written by rvt-rs |
| office at permit | 35 floor voids and the same 4 hatch openings: 36 keys | `VP-1` x35, `YP-1` x1, equal | `VP-1` x35, `YP-1` x1, none written |
| Snowdon Towers Architectural | 12 openings in floors that the census lists as missing the Environmental set, on 2 host types | the host's type name, equal | the host's type name, none written |
| Core Interior | 62 floor voids (42 in slabs, 20 in shading devices), all in Revit's export | none (its export writes no property set) | none |

So an opening of a host's own geometry holds the host's type name in both sets, as the wall's cut openings (RE-321, RE-338) and its profile openings (RE-381) do. Of the openings rvt-rs writes, no other is missing the Environmental set: the 37 and 36 values are all the census lists on `IfcOpeningElement`. The census scores one value for a key that several openings of Revit's export hold, so the four hatch openings are one value there, and four sets in the file.

## 2. What rvt-rs writes

`step_writer.rs`, in the loop of a host's own openings (`IfcEntity::VoidOpening`, `IfcEntity::HostedCutOpening`): the host's `Reference` (`host_references`) in both `Pset_OpeningElementCommon` and `Pset_EnvironmentalImpactIndicators`, where the host has one, as the loops of the wall's cut openings and profile openings write it. `decoded.parameter_value_count` holds both values of each (`pset_property_value_count`). On Core Interior the 62 floor voids gain the Environmental set, as every other element of Core gets the sets Revit's export of it does not write; the census counts them among the values Revit does not write, and the two committed Core Interior witness observations count them.

## 3. Measured

**Run.** Measure [42657527737](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42657527737), unfiltered (`nordic`, `r2025`, `samples`, `racadv`, `bimcollab` and `holdout`), code `721605453b52` against `main` at `8b8892df` (before the saved-mesh default of #1610), 118 models with an export on both sides. The same-project gates pass (15 samples held to their earlier saves, and the Areas on 4 saves and the 2021 save); they check bodies and Areas across saves, which this change does not touch, so what they say is that nothing else moved.

**What moved.** 13 of the 118 exports differ and 105 are byte-identical, all 18 held-out ones among them. In the 13 only property sets differ: `decoded.parameter_value_count` moves by the number of sets gained and no other path of `diagnostics.json` differs; the scorer outputs that differ are the property census, the unfilled-openings scorer's property lines, `schema.txt`'s count of sets and, where there is one, the MCP dump's `parameter_value_count`; no body, quantity, material or identity scorer line differs on any model, and no held-out count does. The sets each gains (the Environmental set on its openings of a host's own geometry):

| Export | Sets gained |
|---|---|
| Core Interior | 62 |
| office as built | 40 |
| office at permit | 39 |
| the MEP samples, advanced and basic, each of the 2024, 2025, 2026 and 2027 saves | 9 each |
| Snowdon Towers Architectural (2024) | 12 |
| Snowdon Towers Structural (2024) | 7 |

**The census on the office.** Without Revit's set placement, `psets_vs_ifc` (before to after):

| | As built | At permit |
|---|---|---|
| equal | 44,422 to 44,459 | 44,571 to 44,607 |
| missing | 1,375 to 1,338 | 1,385 to 1,349 |
| different | 1 to 1 | 1 to 1 |
| rvt-rs values Revit does not write | 1 to 1 | 6 to 6 |
| `Pset_EnvironmentalImpactIndicators.Reference` | 12,787 of 12,824 equal to 12,824 of 12,824 | 12,819 of 12,855 to 12,855 of 12,855 |

The 37 and 36 values the census named are equal, none different, and the values counted apart (a key held by several objects of Revit's export: equal 13 to 14, missing 2 to 1; the sibling-only openings: 6 on 3) do not otherwise move. The unfilled-openings scorer lists `Pset_EnvironmentalImpactIndicators.Reference equal` for the 36 floor voids and the roof's one key as built (35 and 1 at permit), which it listed for no opening in a slab before.

**Forty sets for 37 values.** rvt-rs's own export of the two office files, kept by Measure 42662314183 on the same commit (`keep_ifc`) and read with IfcOpenShell: 40 `IfcOpeningElement` in an `IfcSlab` carry the Environmental `Reference` as built (36 `VP-1` and the roof's 4 `YP-1` hatch openings, which share the key (2991742, 2991742)) and 39 at permit (35 and the same 4); each equals the opening's `Pset_OpeningElementCommon.Reference`; and no opening of the file has the Common `Reference` without the Environmental one (164 openings with it as built, 151 at permit, 124 and 112 of them in walls).

**Core Interior.** Revit's export of it writes no property set (RE-239 section 5), so the 62 sets are counted among the values not scored (4,179 to 4,241) and the unfilled-openings scorer, which counts a property both sides lack as equal, no longer lists the 62 as equal. Both committed rvt-rs witness observations count them: `IFCPROPERTYSET` 6,516 to 6,578, `IFCPROPERTYSINGLEVALUE` 28,068 to 28,130 and `IFCRELDEFINESBYPROPERTIES` 7,045 to 7,107. They are refreshed from the run's `witness-*.json` (byte-identical to the CI artifact's), both verdicts replay PASS with the verdict files unchanged, and the `IFCPROPERTYSET` pin of `2024-core-interior.json` and `2024-core-interior-slim.json` moves 6,514 to 6,578 (it stood inside its tolerance of 8 at the 6,516 measured).

**Snowdon Towers** (local, counts only). The architectural file's census, run locally on the CI-built binaries, named 12 missing values, each the host's type name that `Pset_OpeningElementCommon.Reference` already equals; the 12 sets above are those. The structural file has no Revit export here.

**Held-out.** None of the 18 held-out exports moves, so none holds an opening of a host's own geometry whose host has a `Reference`: the held-out set does not exercise this rule, and "values Revit does not write" is unmeasured on it.

## 4. Not shown

- **A later release.** The rule rests on the office's exports (Revit 24.0.5) and Snowdon Towers' (Revit 2024); no export of a later release holds a floor void or a face-hosted cut to check it against. The run shows the sets written on the 2025, 2026 and 2027 saves of the MEP samples as on their 2024 saves (9 each), which no Revit export of those saves confirms. The export of RE1 Architecture, a Revit 2025 file, holds both sets on the 2 openings the census lists there as values on openings rvt-rs does not write, so nothing measured says a newer exporter drops the Environmental set on an opening, but Revit's newer exporters do write fewer sets on some elements (type objects' `IsExternal`, RE-354). A model of a later release with a floor void and its export would test it.
- **`FireExit` on the openings a door cuts in a second wall.** Revit's export writes `Pset_OpeningElementCommon.FireExit` (`false` on 25 of the office's door openings as built and 33 at permit) where the cutter is a door; rvt-rs writes none. Every value measured is `false` and so is every door's `Pset_DoorCommon.FireExit` (266 of 266), so the measurement cannot say whether the opening takes the door's value or a fixed one; it is left for a model with a door whose value is `true`.

## 5. Reproduce

```bash
# The census that names the values (the property census with Revit's set placement, two office files):
gh workflow run measure.yml --ref main -f ref=main -f base=none -f nordic=true \
    -f models="nordic-office-as-built nordic-office-permit" -f scorers="psets_vs_ifc qtos_vs_ifc" \
    -f ifc_flags=--revit-set-placement
# The unfiltered run of section 3 (the code commit against the main it branched from):
gh workflow run measure.yml --ref main -f ref=721605453b525128ec110cb441112a51c840ffd5 \
    -f base=8b8892df -f nordic=true -f r2025=true -f samples=true -f racadv=true -f bimcollab=true -f holdout=true
# rvt-rs's own export of the two office files, kept (the 40 and 39 sets of section 3):
gh workflow run measure.yml --ref main -f ref=721605453b525128ec110cb441112a51c840ffd5 -f base=none -f nordic=true \
    -f models="nordic-office-as-built nordic-office-permit" -f scorers=none -f keep_ifc=true
```

The kept export is each model's `model.ifc` in the `measure-head` artifact. The count is every `IfcOpeningElement` with a `Pset_EnvironmentalImpactIndicators` holding `Reference`, grouped by the class of the element it voids, the classes of what fills it and the value, with its `Tag` and the `Tag` of its host beside it (IfcOpenShell: `HasFillings`, `VoidsElements`, `IsDefinedBy`).

Snowdon Towers needs the local copies of both files (RE-100): `PSETS_MISSING=Pset_EnvironmentalImpactIndicators.Reference python3 tools/re/psets_vs_ifc.py snowdon.ifc "Snowdon Towers Sample Architectural_IFC4.ifc"` lists the 12 openings.
