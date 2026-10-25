# RE-220: The project's, site's, building's and storeys' GlobalIds a file stores

**Date:** 2026-10-25
**Issues:** #878 (Revit 2015's GlobalIds); follows RE-48 (an element's GlobalId from its episode GUID and ElementId) and RE-208 (Revit 2015's stored IfcGUID strings).
**Status:** positive where the file stores the values; negative for deriving the project's base GUID.
**Artefacts:**

| file | sha256 | licence |
|---|---|---|
| `02_BIMcollab_Example_STR.rvt` (Revit 2015) | `8cd1f122e751c33ea69e6f16c23ed4cd572ded3d0d04dfab50aaf0973e38e193` | no licence stated, local only, fetched by Measure (`-f bimcollab=true`, RE-198) |
| `02_BIMcollab_Example_STR_optimized.ifc` (Revit 2015's export) | `1d0d047a5519a2ea143b9d07625dcc94bd3a1c3b01cb86607b16d850f0a5081d` | as above |
| `ARK_NordicLCA_Office_Concrete_As-Built.rvt` (Revit 2024) and Revit's IFC4 export | `747120005b4b35c7cb66ec61158aab50ded95e052e172cb387b86cd15b31d906`, `b2f530eba8b4424c4542b98e9a2c7bbcc726367f21b2740f3074e4a6ce917382` | CC BY-SA 4.0, VTT (`research/bim4lca-measure.tsv`) |
| `ARK_NordicLCA_Office_Concrete_BuildingPermit.rvt` (Revit 2024) and Revit's IFC4 export | `1c12c2058128315edbd65a8bf97f60da971aee8625f21233b8477d9c6165c1d1`, `bc3bd3366a3c413ce74840300a590f59102b740118ef37898e906253b6b7c08d` | CC BY-SA 4.0 |
| `2024_Core_Interior.rvt` and the RE1 and R2025 models with Revit's exports | as RE-48 and `research/r2025-models.tsv` | MIT, Apache-2.0 |

**Probes:** `examples/probe_878_stored_guids.rs`, `examples/probe_878_project_guids.rs`, `examples/probe_878_text_guids.rs`. **Scorers:** `tools/re/stored_guids_vs_ifc.py`, `tools/re/project_guids_vs_ifc.py`, `tools/re/text_guids_vs_ifc.py`, `tools/re/global_ids_vs_ifc.py`.

## 1. How Revit's export names the project, site and building

revit-ifc's `GUIDUtil.CreateProjectLevelGUID` (CLEANROOM.md source 5) takes the project's, building's or site's GlobalId from the project information's `IFC_PROJECT_GUID`, `IFC_BUILDING_GUID` or `IFC_SITE_GUID` parameter where it holds a valid GlobalId, and otherwise from `ExportUtils.GetExportId` of a fake ElementId: -16 for the project, -15 for the building and -14 for the site (`GUIDUtil.ProjectLevelGUIDType`). An export with "store IFC GUID" writes the values it used back into those parameters.

Every model measured agrees with the fake ids: in Revit's export the site's GlobalId is the project's with 2 XORed into its last bits and the building's with 1, which is what -16, -15 and -14 (0xFFFFFFF0, 0xFFFFFFF1, 0xFFFFFFF2) give when XORed into one GUID's last 32 bits as RE-48 XORs an element's id. The GUID they are XORed into is not found:

| candidate base | models where it gives Revit's project GlobalId |
|---|---:|
| an element's RE-48 GlobalId, any XOR of the last 8 bits | 0 of 30 |
| a `Global/History` episode GUID (high 96 bits) | 0 of 30; the nearest differs in 30 to 42 bits |
| BasicFileInfo's `Unique Document GUID` | 0 of 30 |
| any GUID the file holds as text (8-4-4-4-12 hex, ASCII or UTF-16, in any stream, raw or inflated), read in text or .NET byte order | 0 of 30; 10 to 354 such GUIDs per file |

All 23 R2025 models share one project GlobalId, so the base is constant across files saved from one template. Measure runs [40171223280](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40171223280), [40172100995](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40172100995), [40175317537](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40175317537) and [40176521028](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40176521028).

## 2. Where a file stores them

Each stored value is a GlobalId in UTF-16, preceded by an `i32` parameter id and the length 22 (RE-208 §3). Measure run [40172644920](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40172644920) names the element whose data holds each one, from the nearest element-data header before it:

- **BIMcollab 2015:** the project's, site's and building's under -1019002, -1019004 and -1019003 (`IFC_PROJECT_GUID`, `IFC_SITE_GUID`, `IFC_BUILDING_GUID`), all in the data of element 1075, each Revit's export exactly. They are not related by the XOR above: they were set, not derived.
- **BIMcollab 2015 storeys:** five strings under -1019000 (`IFC_GUID`) equal Revit's five storey GlobalIds. They are not in the Levels' own data but in elements 393104 (four) and 407507 (one). Before each, its Level's id stands twice, 58 bytes apart, 240 to 320 bytes back.
- **BIMcollab 2015 elements:** 229 strings under -1019000, each in its own element's data, hold 126 of the 128 bits of the element's GlobalId (RE-208); they are not used.
- **BIM4LCA office (both files):** three strings under the parameter id -1, in the data of element 49504, that are Revit's project, site and building GlobalIds, related by the XOR above.

Core Interior, Einhoven, the RE1 models and the R2025 models store none of them.

## 3. What rvt-rs does

`revit_global_ids::stored_global_ids` reads them: the project's, site's and building's under their own parameters, each where the file holds one value; else, among the values stored under -1, the one triple related as Revit's export relates them, the project's being the value whose last bit flipped is another stored value (the building's) and whose bit before it flipped is a third (the site's). A storey takes the `IFC_GUID` value before which its Level's id stands twice, 58 bytes apart, within 512 bytes; a Level given two values gets none. `ifc::RevitGlobalIds` carries the project's, site's and building's (`project`, `site`, `building`), and the STEP writer writes them in place of its generated ones. A stored storey value is taken before RE-48's.

## 4. Measured

Measure run [40176601924](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40176601924), this change against `main` (fa7e56b6), default models plus `-f nordic=true -f r2025=true -f bimcollab=true`, `global_ids_vs_ifc`:

| model | project, site, building equal to Revit's | storeys with one of Revit's GlobalIds |
|---|---|---|
| BIMcollab 2015 | 0 → 3 of 3 | 0 → 5 of 5 |
| BIM4LCA office as built | 0 → 3 of 3 | 10 of 10, unchanged |
| BIM4LCA office at building permit | 0 → 3 of 3 | 10 of 10, unchanged |
| Core Interior, Einhoven, RE1, R2025 | 0 of 3, unchanged | unchanged |

Only those three files' IFC changes. The BIMcollab storeys pair with Revit's by GlobalId and not by name: the model's Levels were renamed after its export (RE-198).

## 5. What it does not cover

The project's, site's and building's GlobalIds of a file that stores none keep rvt-rs's generated ones, since the base GUID of `GetExportId` for the fake ids is not found (§1). The storeys' stored-value rule is measured on one Revit 2015 file.

## 6. Reproduce

```bash
gh workflow run measure.yml --ref <branch> -f ref=<branch> -f base=main -f nordic=true -f r2025=true -f bimcollab=true
gh workflow run measure.yml --ref <branch> -f ref=<branch> -f base=none -f bimcollab=true -f nordic=true \
  -f probe=probe_878_stored_guids -f probe_scorer=stored_guids_vs_ifc
gh workflow run measure.yml --ref <branch> -f ref=<branch> -f base=none -f bimcollab=true -f nordic=true -f r2025=true \
  -f probe=probe_878_text_guids -f probe_scorer=text_guids_vs_ifc
```
