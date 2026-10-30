# RE-239 — Values a project's own property set file places

**Date:** 2026-10-26
**Issues:** #35 (parameters), W7 of [`docs/release-1.0-criteria.md`](../../docs/release-1.0-criteria.md); follows RE-194 §4, RE-235
**Decision:** Griffin, 2026-10-26: a value that a project's export-time user-defined property set file maps into a set is out of W7's scope, and is listed per model with the evidence. W7 stays reachable on every reference model.
**Artefacts:** the BIM4LCA office as built and at building permit (Revit 2024, Revit's IFC4 exports), RE1 Architecture, Mechanical, Plumbing and Electrical (Revit 2025, Revit's IFC2X3 exports), Core Interior (Revit 2024) and puzzbobb's R2025 models (Revit 2025, Revit's IFC4 exports), as Measure fetches them
**Runs:** Measure [40272461335](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40272461335) (`tools/re/psets_vs_ifc.py`, listing every space missing `OccupancyType`, on `main` at f0369be3), [40276486381](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40276486381) (`examples/probe_re239_mapped_parameters.rs`, the stream scan, on f0498735), [40278960117](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40278960117) (the same probe with `tools/re/mapped_parameters_vs_psets.py` matching spaces by number) and [40278970448](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40278970448) (`tools/re/set_census.py`, occurrences and type objects)
**Result:** positive: every value the office's file places is a saved parameter of the element, and the model stores the parameter and the export setting but not the file. As built, 2,013 values rvt-rs does not write as Revit does are placed that way, 1,938 at permit; no other reference model has one.

## 1. The office's file

RE-194 §4 found that the office's export, IFC4 Reference View 1.2 by Revit 2024's own exporter, carries what only a user-defined property set file gives: revit-ifc's list of the sets a Reference View export may create (`IFCCertifiedEntitiesAndPSets.json`, CLEANROOM.md source 5) holds neither `Pset_DoorWindowGlazingType` nor `Pset_SpaceOccupancyRequirements`, yet the export writes both, and it writes sets buildingSMART does not define (`FI_Kohde`, `FI_RakennuksenSijainti`, ...). A user-defined property set file lists, after the keyword `PropertySet:`, a set's name and the entities it is written on, then each property with its value type and the parameter it is read from; revit-ifc installs one, `IFC-SG Property Mapping Export.txt`.

What the file places, by `psets_vs_ifc` and `set_census` on objects rvt-rs writes. Type objects hold sets too (`HasPropertySets`): 1,618 values as built and 1,696 at permit, in the common sets of their own entities and `Pset_ManufacturerTypeInformation`, none of them the glazing, occupancy or `FI_` sets; `psets_vs_ifc` scores occurrences only (#1018).

| what Revit writes | where | as built | at permit | rvt-rs |
|---|---|---:|---:|---|
| `Pset_DoorWindowGlazingType.IsLaminated`, `IsTempered` | walls, curtain walls, the openings curtain walls cut, coverings | 1,692 | 1,618 | not written |
| `Pset_SpaceOccupancyRequirements.OccupancyType` | spaces | 234 | 234 | not written |
| `Pset_SpaceOccupancyRequirements.OccupancyNumber` as `IfcInteger` | spaces | 55 | 55 | the same number, as the `IfcCountMeasure` IFC4 declares |
| `Pset_SpaceCommon.FloorCovering`, `WallCovering`, `CeilingCovering` as `IfcText` | spaces | 646 | 646 | written with `--revit-set-placement` (RE-235) |
| `FI_RH1`, `FI_Kiinteistö`, `FI_Suunnittelija`, `FI_Kohde`, `FI_MallinSijainti`, `FI_RakennuksenSijainti` | the building and the site | 32 | 31 | not written |
| `Pset_DoorWindowGlazingType.IsLaminated`, `IsTempered` | doors and windows | 1,070 | 1,062 | written: IFC4 gives doors and windows the set |

An unfilled opening takes its cutting element's properties (RE-237 §1), so the openings curtain walls cut carry their glazing. `psets_vs_ifc` keys an element by its `Tag`, and such an opening has its curtain wall's, so as built its 846 keys for each glazing property (604 last written as walls, 151 as curtain walls, 70 as openings, 21 as coverings) stand for 918 objects: the probe matches 607 walls, 290 curtain walls and openings, and 21 coverings (§2). At permit they are 809 keys on 588 walls, 149 curtain walls and 72 openings. W7 is counted in that census's keys. The rooms' covering properties and the doors' and windows' glazing are listed for completeness; rvt-rs already writes them as Revit does, so they leave W7's count unchanged.

## 2. The parameters behind them

The probe prints every Yes/No and text value each wall, curtain wall, covering, member, plate and room saves (RE-207), and the scorer counts, for each property Revit writes, which parameter holds the same value. Elements are matched by `Tag`, a space by its number, and the 43 rooms without one by their ElementId:

| property | held by | as built | at permit |
|---|---|---:|---:|
| glazing, on walls | parameters 2189854 and 2189858, named `Pset_DoorWindowGlazingType.IsLaminated` and `.IsTempered` | 607 of 607 | 591 of 591 |
| glazing, on curtain walls and their openings | the same two parameters | 290 of 290 | 292 of 292 |
| glazing, on coverings (walls Revit exports as coverings) | the same two parameters | 21 of 21 | none written |
| `OccupancyType` | parameter 2065807, `RoomTypeCode` (values such as `0400-64-03`) | 234 of 234 | 234 of 234 |

## 3. Only the file says where they go

The parameters' names do not decide it:

- every wall Revit writes glazing on also holds parameter 1882432, `Pset_CurtainWallCommon.IsExternal`, and every curtain wall parameter 1877411, `Pset_WallCommon.IsExternal`. Revit's export writes no `Pset_CurtainWallCommon` on a wall and no `Pset_WallCommon` on a curtain wall (RE-194 §4), yet writes `Pset_DoorWindowGlazingType` on both. Writing every parameter named `<set>.<property>` wherever an element holds it would add at least 827 values as built (607 walls, 220 curtain walls) that Revit's export does not write;
- revit-ifc's own definitions (`ExporterInitializer_PsetDef.cs`) write `Pset_DoorWindowGlazingType` on `IfcDoor` and `IfcWindow` only, and read `OccupancyType` from a parameter named `OccupancyType`, `Pset_SpaceOccupancyRequirements.OccupancyType` or a localised name such as `Occupancy Type`; the `OccupancyTypeCalculator` they also look up is not in revit-ifc's source. No office room has a parameter by any of those names. The codes are in `RoomTypeCode`, a name revit-ifc's source does not contain.

## 4. The model does not store the file

The probe counts, in every stream of each model as stored and inflated, the keyword `PropertySet:` that opens each set of such a file, the name of one set only the office's file defines (`FI_Kohde`), and revit-ifc's names for its export setup (`IFCExportConfiguration`) and for the setting that turns the file on and names it (`ExportUserDefinedPsets`, `ExportUserDefinedPsetsFileName`), in UTF-8 and UTF-16:

| model | `PropertySet:` | `FI_Kohde` | `IFCExportConfiguration` | `ExportUserDefinedPsets` | the setting |
|---|---:|---:|---:|---:|---|
| office as built | 0 | 0 | 1 (`Global/Latest`) | 4 (`Partitions/425`) | `true`, a file named `LCA_Properties_export_….txt` |
| office at permit | 0 | 0 | 1 | 4 (`Partitions/336`, `Partitions/404`) | the same |
| Core Interior | 0 | 0 | 1 | 4 (`Partitions/65`) | `false` |
| RE1, R2025, Einhoven | 0 | 0 | 0 | 0 | none saved |

The office's model keeps the export setup that was used, with the user-defined property sets turned on and the file's name (the probe prints the text after each `ExportUserDefinedPsets`, a path cut to its file name), but not the file: no set definition, and no name of a set only the file defines. Which parameter goes to which set on which entity is in a text file beside the model, not in it.

## 5. What rvt-rs does

Nothing changes. These values are out of W7's scope and listed here per model:

| model | values out of scope | which |
|---|---:|---|
| office as built | 2,013 | §1: 1,692 glazing, 234 `OccupancyType`, 55 `OccupancyNumber` types, 32 in the FI_ sets |
| office at permit | 1,938 | §1: 1,618 glazing, 234, 55, 31 |
| RE1 Architecture, Mechanical, Plumbing, Electrical | 0 | each property their exports write and rvt-rs does not is on an entity revit-ifc's IFC2X3 definitions give its set, or on an unfilled opening, which takes its cutting element's (RE-237 §1) |
| Core Interior | 0 | its reference export writes no property sets |
| R2025 models | 0 | each such property is on an entity revit-ifc's definitions give its set, in a set matched by the start of its entity's name (RE-235 §1), or on an unfilled opening |

Since 2026-10-30 `tools/re/psets_vs_ifc.py` counts these values apart rather than as missing (`export_file_values`): in an IFC4 Reference View export, `Pset_DoorWindowGlazingType` on an element that is not a door or a window, and `Pset_SpaceOccupancyRequirements`, since revit-ifc's certified list for that view creates neither set (§1). The `FI_` sets are not standard sets, so the census never scored them. On the same exports of `main` at b09310a6 (Measure runs 41129217725 and 41129221110 before, 41129224669 and 41129228138 after), the census sets aside 1,981 values as built (1,692 glazing, 234 `OccupancyType`, 55 `OccupancyNumber`) and 1,907 at permit; equal is unchanged on every model, different goes from 56 to 1 on both office files, and with `--revit-set-placement` missing goes from 2,381 to 455 as built and from 2,304 to 452 at permit. The glazing set on doors and windows is still scored: 535 and 531 values each, all equal.

The rest of what the office's export writes and rvt-rs does not (`Pset_MemberCommon.IsExternal` on 10,003 mullions, `Pset_PlateCommon.IsExternal` on 260 panels, sloped slabs' `PitchAngle`, beams' `Span`, `Slope` and `Roll`) is placed by revit-ifc's own rules and stays in W7. No file maps a value onto the mullions and panels: the probe finds no saved Yes/No or text parameter on any of the 10,003 mullions, and none but the two empty IFC export parameters (-1019016, -1019014) on the panels, so their `IsExternal` is revit-ifc's calculation, whose host rule RE-226 §2 measured negative. Quantities are not counted here or in W7's census yet (#1016).

## 6. Reproduce

```bash
gh workflow run measure.yml --ref main -f ref=main -f base=none -f nordic=true -f r2025=true \
  -f probe=probe_re239_mapped_parameters -f probe_scorer=mapped_parameters_vs_psets
gh workflow run measure.yml --ref main -f ref=main -f base=none -f nordic=true -f r2025=true \
  -f probe=basics_inspect -f probe_scorer=set_census
```

The listing of §1 is `psets_vs_ifc.txt` in any Measure run with `nordic=true`; `PSETS_MISSING` (#1013) names the properties it lists object by object.

## 7. The values the census sets aside, checked on IFC4 exports (2026-10-29)

`tools/re/psets_vs_ifc.py` sets aside what rvt-rs writes where Revit's reference export cannot have written it: Core Interior's export has no property sets (§5, #1115), and RE1's are IFC2X3, which defines no place for some IFC4 properties (#1118). That is 4,210 values in 25 properties. Each property was checked against Revit's IFC4 exports of the BIM4LCA office (as built, at permit) and the R2025 models (Measure [40981313275](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40981313275) and, before either change, [40977753002](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40977753002)):

| | properties | values set aside |
|---|---:|---:|
| written by Revit's IFC4 exports and equal there, never different or extra | 19 | 2,506 |
| equal and never different there, with an extra only on elements Revit's export leaves out (column 1927742, 17 openings) | 5 | 1,684 |
| in no IFC4 export: `Pset_ShadingDeviceCommon.Reference` | 1 | 20 |

The middle row's properties are `Pset_EnvironmentalImpactIndicators.Reference`, `Pset_ColumnCommon.IsExternal` and `Reference`, `Pset_ReinforcementBarPitchOfColumn.Reference` and `Pset_OpeningElementCommon.Reference`: their extras are elements Revit's export does not hold (W4, RE-262), not values. No reference export holds an `IfcShadingDevice`, so nothing confirms that Revit writes `Pset_ShadingDeviceCommon.Reference`. The maintainer chose to keep both set-asides and to leave that property out until a Revit IFC4 export confirms it (2026-10-29), and then, for every set, to write `Reference` only where a Revit export confirms it. rvt-rs can write `Reference` in 44 standard sets for the entities it exports. 34 are confirmed: 31 equal on the office's and the R2025 models' IFC4 exports and never written where Revit's export leaves them out, and the duct fitting, duct segment and pipe fitting types on RE1's IFC2X3 exports. They are `CONFIRMED_REFERENCE_SETS` (`src/ifc/export_content.rs`). The others get no `Reference`:

- `Pset_ShadingDeviceCommon`: in no reference export;
- `Pset_AlarmTypeCommon`: Revit's IFC2X3 export of RE1 Electrical writes its 5 alarms as `IfcDistributionControlElement`s, which the set does not apply to, so no export can confirm it (their type name is in `Pset_QuantityTakeOff`, which rvt-rs writes as `Pset_EnvironmentalImpactIndicators`);
- `Pset_FootingCommon`, `Pset_ReinforcementBarCountOfIndependentFooting`, `Pset_ReinforcementBarPitchOfContinuousFooting`, `Pset_ReinforcingBarCommon`, `Pset_ElementComponentCommon`, `Pset_TransportElementCommon`, `Pset_RampFlightCommon`: no reference export confirms them (only BIMcollab's 2015 structural model has footings, and its results are counts only);
- `Pset_BuildingStoreyCommon`: a storey is never given `Reference`.

A set joins the list when a reference export confirms it.

