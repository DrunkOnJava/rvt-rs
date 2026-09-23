# RE-33 — Thirteen more product categories from element records, and the second prologue keeps the instance rule

**Date:** 2026-09-22
**Result:**
- **Positive** on precision: thirteen more `BuiltInCategory` values decode under the unchanged RE-21 instance rule, and every ElementId they select is a `Tag` in Revit's own export of the same file. There are **zero false positives** across four files and two releases.
- **Positive** on the second prologue: frames with no ElementId at `+0x00` (RE-30) still carry the container reference at `+0x32` and the placement kind at `+0x42`. With the instance rule applied to them, the unattributed count equals the number of elements Revit exported that rvt-rs cannot, exactly for the MEP categories on RE1 and for mullions, columns, railings, ceilings and furniture on Snowdon.
- **Partial** on recall: as before, most Snowdon frames use the second prologue.
- **Measured negative** carried over: one record-backed slab on Snowdon (ElementId 2062318) is not in Revit's export (§4).

**Oracles:**

| file | Revit | IFC exporter | licence | sha256 (`.rvt` / `.ifc`) |
|---|---|---|---|---|
| `RE1-Architecture` (`Drshelden/IFC-ECS`, `data/RE1/`, commit `9e8b371`) | 2025 | Revit 26.2.0.20, IFC2x3 CV2.0 | **MIT** | `4a78bdd6…` / `a9b5d366…` |
| `RE1-Mechanical` (same) | 2025 | same | **MIT** | `d5597201…` / `9e56cf72…` |
| `RE1-Plumbing` (same) | 2025 | same | **MIT** | `03689df2…` / `294490b2…` |
| Snowdon Towers Sample Architectural (`bldrs-ai/test-models`) | 2024 | Revit 24.0.20.20, IFC4 | none (local only) | `33271010…` / `ecfcb04e…` |
| `2024_Core_Interior` (magnetar) | 2024 | reference `2024_Core_Interior_slim.ifc` | MIT | `c805df44…` |

CI tier 2 now fetches the three RE1 models by pinned commit and hash, and `tests/element_records_2025.rs` checks them.

**Probe:** `examples/probe_re33_category_census.rs` anchors on the release's bbox marker, decodes every frame whose `+0x00` ElementId is declared, and prints per category the frames, the RE-21 instances and (with `--ids`) their ElementIds. It also prints the second-prologue frames per category, in total and as instance-like.

-----

## 1. The census

Every decodable record's `BuiltInCategory` was histogrammed and each category's instance set scored against the export's `Tag`s. This is the method that established `OST_Rooms` (RE-29). Category names are the public `BuiltInCategory` enum members.

| category | class | IFC4 entity written | RE1 Arch | RE1 Mech | RE1 Plumb | Snowdon | FP |
|---|---|---|---:|---:|---:|---:|---:|
| `OST_Furniture` −2000080 | Furniture | `IfcFurniture` | 13 | – | – | 1 | 0 |
| `OST_Casework` −2001000 | Casework | `IfcFurniture` | 10 | – | – | – | 0 |
| `OST_PlumbingFixtures` −2001160 | PlumbingFixture | `IfcSanitaryTerminal` | 7 | – | – | – | 0 |
| `OST_SpecialityEquipment` −2001350 | SpecialtyEquipment | `IfcBuildingElementProxy` | 9 | – | – | – | 0 |
| `OST_Ceilings` −2000038 | Ceiling | `IfcCovering` `.CEILING.` | 6 | – | – | – | 0 |
| `OST_CurtainWallMullions` −2000171 | CurtainWallMullion | `IfcMember` `.MULLION.` | 10 | – | – | – | 0 |
| `OST_CurtainWallPanels` −2000170 | CurtainWallPanel | `IfcPlate` `.CURTAIN_PANEL.` | 2 | – | – | – | 0 |
| `OST_StairsRailing` −2000126 | Railing | `IfcRailing` | 1 | – | – | – | 0 |
| `OST_Cornices` −2000181 | WallSweep | `IfcBuildingElementProxy` `.NOTDEFINED.` | – | – | – | 36 | 0 |
| `OST_DuctCurves` −2008000 | Duct | `IfcDuctSegment` | – | 13 | – | – | 0 |
| `OST_DuctFitting` −2008010 | DuctFitting | `IfcDuctFitting` | – | 14 | – | – | 0 |
| `OST_PipeCurves` −2008044 | Pipe | `IfcPipeSegment` | – | 3 | 3 | – | 0 |
| `OST_PipeFitting` −2008049 | PipeFitting | `IfcPipeFitting` | – | 3 | 6 | – | 0 |

The categories whose instance sets are *not* in any export are all non-product categories: `OST_SketchLines`, `OST_Lines`, `OST_RoomSeparationLines`, `OST_CurtainGridsWall`, `OST_CoordinateSystem`, `OST_ProjectBasePoint`, `OST_SharedBasePoint`, `OST_AreaSchemeLines`, `OST_InsulationLines`, `OST_RailingSupport`, `OST_AnalyticalPanel`, `OST_AnalyticalNodes`, `OST_Topography`, and `OST_PipeCurvesCenterLine` / `OST_PipeFittingCenterLine`. The last two carry 63 and 54 instances on RE1 Plumbing, the same counts as the export's pipe segments and fittings, but none of their ElementIds is a `Tag`. They are separate elements, not the pipes.

On Core Interior no record of the thirteen categories decodes, and its export is unchanged.

End to end, `rvt-ifc` writes these sets, and each equals the export's set for the entity Revit chose. The reference exports are IFC2x3 (RE1) and IFC4 (Snowdon), so Revit's own entity differs from the IFC4 one rvt-rs writes in three places: furniture and casework (`IfcFurnishingElement` in IFC2x3), plumbing fixtures (`IfcFlowTerminal`), and ducts and pipes (`IfcFlowSegment` / `IfcFlowFitting`).

`category_map` choices follow Revit's own IFC4 export where it shows them:
- `.MULLION.` on all 1,425 Snowdon mullions;
- `.CURTAIN_PANEL.`;
- `.CEILING.`;
- `.NOTDEFINED.` on the wall-sweep proxies.

`SpecialtyEquipment`, `FurnitureSystem` and `Mass` previously mapped to `USERDEFINED` with no `ObjectType`, which IFC4's `CorrectPredefinedType` rule forbids. They now write `$`.

## 2. Recall

| file | exported by Revit, of these categories | recovered | missing |
|---|---:|---:|---:|
| RE1 Architecture | 58 | 58 | 0 |
| RE1 Mechanical | 73 (segments, fittings, terminals) | 33 | 40 |
| RE1 Plumbing | 123 | 9 | 114 |
| Snowdon Architectural | thousands | 37 | most |

Most missing elements are second-prologue frames (§3). Air terminals and plumbing-fixture terminals on the MEP models are not among the decodable records at all.

## 3. The second prologue keeps the instance rule

RE-30 established that most Snowdon frames have no ElementId at `+0x00`. Counting them all overstated what was missing. On RE1 Architecture every furniture, casework, plumbing-fixture and specialty-equipment element is recovered, yet 47 second-prologue frames of those categories were counted.

Reading the RE-21 fields on those frames at the same offsets shows they keep them:
- the container reference at `+0x32` (`0xFF`×8 when none);
- the placement kind at `+0x42` (`0xffffef7f` for a placed instance).

Filtering by them:

| file | category | second-prologue frames | instance-like | exported by Revit, not by rvt-rs |
|---|---|---:|---:|---:|
| RE1 Architecture | furniture, casework, fixtures, specialty equipment | 47 | **0** | **0** |
| RE1 Architecture | doors | 17 | 6 | 5 |
| RE1 Architecture | walls | 1 | 1 | 1 (the curtain wall) |
| RE1 Mechanical | ducts + pipes | 15 | **15** | **15** segments |
| RE1 Mechanical | duct fittings | 38 | **18** | **18** |
| RE1 Plumbing | pipes | 60 | **60** | **60** |
| RE1 Plumbing | pipe fittings | 73 | **48** | **48** |
| RE1 Plumbing | plumbing fixtures | 19 | 13 | 6 |
| Snowdon | mullions | 1,757 | **1,425** | **1,425** |
| Snowdon | columns | 180 | **118** | **118** |
| Snowdon | railings | 137 | **131** | **131** |
| Snowdon | ceilings | 74 | **68** | **68** |
| Snowdon | furniture + casework | 455 | **344** | **344** |
| Snowdon | doors | 245 | 141 | 132 |
| Snowdon | windows | 150 | 106 | 68 |
| Snowdon | panels | 650 | 527 | 462 |

The overcounts that remain (doors, windows and panels on Snowdon) are plausibly curtain-wall infill that Revit exports under another entity. That is not established.

`count_unattributed_frames_with_marker` now counts only instance-like frames. Over the twenty recovered categories, the `element_record_without_element_id` item and `confidence.unexported_element_records` move as follows (before this change they covered only the seven core categories: 2,043 on Snowdon Architectural and 18 on RE1 Architecture):

| file | all second-prologue frames | instance-like, now counted |
|---|---:|---:|
| Snowdon Architectural | 6,019 | 4,886 |
| RE1 Architecture | 65 | 7 |
| RE1 Mechanical | 56 | 33 |
| RE1 Plumbing | 152 | 121 |

## 4. Measured negative: Snowdon slab 2062318

The one `OST_Floors` record on Snowdon Architectural that decodes under the first prologue is ElementId 2062318. It is a 23.05 × 1.54 ft plate, 0.33 ft thick, at internal z 59.67 ft, and it is not in Revit's IFC4 export under any entity.

The nearest exported elements, after the internal-to-shared transform, are "Slab Edge:Bleacher Seating" proxies and 4-inch "Concrete Slab" steps with other `Tag`s, 0.9 to 7 ft away. It is one bleacher step that Revit did not export, for example one in a non-primary design option or a demolished phase. No field decoded so far separates it, so rvt-rs still exports it. This is the only false positive among the 43 first-prologue instances on Snowdon, and it predates this change. Tracked in #309.

## 5. What this does not claim

- Recall is partial everywhere except RE1 Architecture's thirteen-category set.
- Entity choices are IFC4 and do not reproduce the IFC2x3 entity of the RE1 references.
- Bodies are the record bounding box, as for columns before RE-29.
- Levels bind only when the record's reference list names exactly one.
- 2023 and 2026 stay unsupported, as in RE-32.
