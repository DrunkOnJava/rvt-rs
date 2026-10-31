# RE-277: three BIM4LCA office properties the file does not decide

**Date:** 2026-10-30
**Issues:** W7 ([release 1.0 criteria](../../docs/release-1.0-criteria.md))
**Artefacts:** the BIM4LCA office as built and at building permit (Revit 2024, CC BY-SA 4.0) with Revit's own IFC4 exports. Read on GitHub's hosted runners with the Measure workflow, runs 41111710319 and 41113408569 (the probe), and run 41104131261 (the census).
**Probe:** `probes/examples/probe_w7_office_gaps.rs`.
**Status:** negative for three gaps. RE-275 and RE-276 are the positive findings of the same pass.

The pass looked for the source of every property Revit's IFC4 export writes on the office and rvt-rs did not. Beams' `Slope`, `Span` and `Roll`, storeys' `Reference` and `EntranceLevel`, and pipes' `InvertElevation` have one (RE-275, RE-276, RE-157 section 6). Three do not. Each stays unwritten, since a value is written only where it equals Revit's on every object it is written on.

## 1. Columns' `Pset_ColumnCommon.Slope`

Revit writes it on 73 of the 279 columns as built and 101 of the 345 at permit, 0.0 on all, and on no other. revit-ifc's `SlopeCalculator` (CLEANROOM.md source 5) has, as read, nothing to give a vertical column before its last step: no beam elevation parameters, no ramp slope, no parameter named `Slope` on these families, an extrusion slope of 0, and a vertical location line that gives no angle. That step takes the slope of the largest top-facing face of the last object of the element's geometry, where an object that is a `GeometryInstance` (the symbol's own geometry) gives nothing. So the property would exist on the columns whose geometry, as Revit's API returns it when the exporter runs, ends in a solid with a face that looks up, and its value is 0 for a flat top, as it is on all of them. That is the state of Revit's geometry at export, which the `.rvt` stores only as the steps that make it.

The probe lists each column's native record (its `ColumnAttachment`, `CutPairGStep` and `CleanPairGStep` objects, `GeomStepList`, `FamilyInstance`, `InstanceInfo` and parameters). Over the 622 columns of the two files whose record is read, by what the column is attached to (its `ColumnAttachment`) and whether one of its clean (join) steps is user-set:

| the column is attached to | a clean step is user-set | with the property | without |
|---|---|---:|---:|
| nothing | no | 0 | 55 |
| a beam | no | 0 | 16 |
| a roof | yes | 0 | 20 |
| a roof | no | 102 | 65 |
| a floor | no | 31 | 176 |
| a floor | yes | 25 | 73 |
| nothing | yes | 16 | 43 |

Only the cells with no column on one side are pure, and none is a rule for the property: that needs a cell where every column has it. The nearest, a roof's column with no user-set join, splits by how far the attachment's z extent differs from the column's own height: -0.33 ft on 62 of 62 columns, 0 on 40 of 46, and -0.5 or 2.4 to 6.5 ft on none of 59. The six columns it misses are the same six that gain the property at permit:

- **Identical records, opposite outcomes.** Columns 2472052, 2472522, 2472594, 2472785, 2472828 and 2473198 (`Rectangular and Square Hollow Sections-Column`, PI-4, Level_01) have a native record that is identical in the two files, object by object and field by field, and Revit's export gives them no `Slope` as built and `Slope` at permit. No rule over a column's own record can be exact on both files.
- **Identical neighbours.** The 12 `Concrete-Rectangular-Column` PI-3 columns on Level_01 have the same box, the same type, level and attachment to the same roof; 5 have the property and 7 do not. The 7 each have one clean step with another element that is user-set (on one of them, the wall 1621552), and the 5 have only automatic ones. That separates these twelve but not the rest: the floor-attached columns with a user-set clean step have the property on 25 of 98.

What would decide it is the geometry the exporter sees, a roof's and a floor's cut of each column and the joins' effect, which rvt-rs reproduces only for some columns (RE-178, RE-202).

## 2. The building's `Reference`

Revit writes `Project Information` in `Pset_BuildingCommon`, `Pset_BuildingStoreyCommon`, `Pset_BuildingSystemCommon` and `Pset_BuildingElementProxyCommon` on the building of the office (as built and at permit) and of each R2025 model. revit-ifc's `ReferenceCalculator` gives an element with no type its own name, and the building's is the project information element's. Whether Revit localizes that name is not shown by the exports measured, which are all English. The string is not in either office file: no stream holds it as UTF-16 text, and the only UTF-8 hits are two RTF help-text lines in the as-built file. Writing the English text would be a guess about the exporting session, not a read of the file.

## 3. Door openings' `Pset_OpeningElementCommon.Reference` and `FireExit`

The census counted 140 `Reference` values and 31 `FireExit` values on the office as built (147 and 37 at permit) missing on `IfcOpeningElement`s. Keyed by `Tag`, which is the ElementId of the element an opening belongs to, they collapsed several openings into one (the census scorer now keys an opening by whether something fills it, #1161). Revit's export writes, for a door, several openings under its one `Tag`:

- the opening its door fills (`O-4:O-4.8x21:1383546:1`, host an `IfcWall`, filled by the `IfcDoor`): 187 on the office as built, none of them with a `Pset_OpeningElementCommon`;
- for each further wall the door cuts, or door it voids, an opening with no filling, named after that host (`Basic Wall:VS-2:1352055`), holding the door's type name as `Reference` and `FireExit` false: 39 in walls and 4 in doors with both. The same kind of opening under a curtain wall's or a recessed fire hose cabinet's `Tag` holds `Reference` alone: 141 in walls.

rvt-rs writes the first kind with no `Pset_OpeningElementCommon` either (192 filled door openings), which is Revit's. It writes the openings of a floor's or roof's own sketch with their `Reference`, equal to Revit's (RE-237). It does not write the second kind: the openings a door cuts in other walls are a geometry task, which needs the walls the door passes through and the cut's shape. So the 31 and 140 values are on openings rvt-rs does not write, not missing properties on openings it does, and writing them on the door's own opening would add values Revit's export omits there.

## 4. Reproduce

```text
gh workflow run measure.yml --ref main -f ref=8193128ea56b7fb098c1472eacb0536b9fe09c32 -f base=none -f nordic=true -f r2025=true -f models="nordic-office-* r2025-*" -f scorers=none -f probe=probe_w7_office_gaps -f probe_scorer="-"
```
