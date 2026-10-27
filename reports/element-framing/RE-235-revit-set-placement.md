# RE-235 — Where Revit's export writes properties IFC4's definitions do not

**Date:** 2026-10-25
**Issues:** #35 (parameters), W7 of [`docs/release-1.0-criteria.md`](../../docs/release-1.0-criteria.md)
**Decision:** Griffin, 2026-10-25: the default output follows buildingSMART's IFC4 definitions; Revit's placements are written with an option, and W7 is measured with it on. A value Revit's export writes that is known to be wrong is left out.
**Artefacts:** the BIM4LCA office as built and at building permit (Revit 2024, Revit's IFC4 exports), RE1 Architecture (Revit 2025, Revit's IFC2X3 export) and puzzbobb's R2025 models (Revit 2025, Revit's IFC4 exports), as Measure fetches them
**Runs:** Measure [40243309751](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40243309751) (the entities each missing property is on), [40253819602](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40253819602) (`examples/probe_re235_room_text_parameters.rs`), [40254025441](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40254025441) (a first rule, §3) and [40259011277](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40259011277) (`--revit-set-placement` on, against `main` at ab0ff87a in [40258827136](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40258827136))
**Result:** positive: with `--revit-set-placement`, 961 more of the values Revit's export writes on the office are written, none different, and none Revit's export does not write.

## 1. Values in a set IFC4 does not hold them in

`tools/re/psets_vs_ifc.py` names the entities Revit writes each missing property on. On the office three groups sit outside the set buildingSMART's IFC4 definitions give:

| what Revit writes | where | values, as built |
|---|---|---:|
| `Pset_SpaceCommon.FloorCovering`, `WallCovering`, `CeilingCovering` | spaces | 206, 234, 206 |
| `Pset_SpaceHeaterTypeCommon.Reference` | spaces | 298 |
| `Pset_StairFlightCommon.*` | stairs | 14 each |

IFC4 moved a space's coverings to `Pset_SpaceCoveringRequirements`; `Pset_SpaceHeaterTypeCommon` applies to space heaters and `Pset_StairFlightCommon` to stair flights. revit-ifc (CLEANROOM.md source 5) matches a set to an element when one of the set's entities' names begins with the element's: `IfcSpaceHeater` begins `IfcSpace`, `IfcStairFlight` `IfcStair`.

The probe printed every text parameter of each office room: each holds project parameters named `Pset_SpaceCommon.FloorCovering`, `Pset_SpaceCommon.WallCovering` and `Pset_SpaceCommon.CeilingCovering` (RE-194's naming), and Revit writes their text in that set as `IfcText`. RE-194 reads such a parameter only where buildingSMART's definition of the set has the property.

## 2. What rvt-rs does

By default nothing changes. `rvt-ifc --revit-set-placement` (`ifc::revit_set_placement`) adds, merged into a set of the same name the element has:

- a text parameter named `<set>.<property>` whose set applies to the element and whose property buildingSMART's definition of the set lacks altogether (`StandardPropertySet::defines`), as `IfcText`, on every piece the element is written as;
- each standard set one of whose entities' names begins with the element's, with a parameter named for that set's `Reference` if the element has one, else the element's `Reference`.

## 3. A set matched by name holds only `Reference`

A first rule wrote every property of such a set the element holds under the same name in its other sets. On the office Revit's `Pset_StairFlightCommon` on a stair then differs from the stair's own values: `NumberOfTreads` on 12 of 14, `RiserHeight` on 6, `NumberOfRiser` on 1. No rule built from the stair's other values gives those, so only `Reference`, the type's name, which Revit writes the same in every set, is written.

## 4. Measured

`psets_vs_ifc` with the option on, against `main` without it:

| file | equal, off | on | different | Revit does not write |
|---|---:|---:|---:|---:|
| office as built | 43,934 | 44,895 | 56, both | 495, both |
| office at permit | 44,116 | 45,077 | 56, both | 496, both |
| R2025 Rooms_Stairs | 100 | 103 | 1, both | 0, both |

The 961: 646 coverings, 298 `Pset_SpaceHeaterTypeCommon.Reference`, one air terminal box's `Pset_AirTerminalBoxTypeCommon.Reference` and the 16 stairs' `Pset_StairFlightCommon.Reference`. The diagnostics' `parameter_value_count` counts them. RE1 Architecture's IFC2X3 export writes neither name-matched set on its 11 spaces, so on RE1 they count as values Revit does not write; rvt-rs writes IFC4.

## 5. Revit values left out

A value Revit's export writes that is known to be wrong is not written, with or without the option, and `tools/re/psets_vs_ifc.py` and `tools/re/qtos_vs_ifc.py` count it apart (`REVIT_DEFECTS`), not as missing: 63 `WalkingLineOffset` values and 10,003 mullions' `CrossSectionArea` on the office as built (Measure 40306633673):

| property | Revit's value | why it is wrong |
|---|---|---|
| `Pset_StairFlightCommon.WalkingLineOffset`, `Pset_StairCommon.WalkingLineOffset` | half the run's width in feet, read as millimetres (0.00204 m on the office, 0.00164 m on the R2025 models' 1 m runs) | revit-ifc's `WalkingLineOffsetCalculator` returns feet unscaled |
| `Qto_OpeningElementBaseQuantities.Width` and `Height` on a door's opening | `Width` its vertical extent and `Height` its horizontal one, on all 187 door openings of the office as built and the R2025 door's; two of R2025's three window openings likewise (RE-244) | IFC4 defines a wall opening's `Width` as its horizontal dimension and `Height` as its vertical one; revit-ifc reads them from its profile's axes, which do not follow the opening's |
| `Qto_SlabBaseQuantities.Perimeter` (added 2026-10-26, #1016) | the slab's perimeter in feet, unscaled, read as millimetres: 0.0201 m on a slab whose outline is 6.125 m, rvt-rs's outline over Revit's value exactly 304.8 on 26 of the 31 BIM4LCA office floors compared (Measure 40441677656) | revit-ifc's `PerimeterCalculator` takes the extrusion data's `ScaledOuterPerimeter`, which on these slabs is not scaled to the file's unit |
| `Qto_MemberBaseQuantities.CrossSectionArea` on curtain mullions (`IfcMember` `.MULLION.`) | an area that grows with the mullion's length: half its surface on 4,413 of the office's 10,003 mullions as built (0.3625 m² on a 3.6 m mullion of 50 × 50 mm, whose section is 0.0025 m²), never its section's area (0 of 10,003; Measure 40298992508, `tools/re/member_qtos_list.py` at 64fc4405) | IFC4 defines it as the area of the member's cross section. revit-ifc's `CrossSectionAreaCalculator` takes a parameter, then the extrusion data of the body it writes, then Revit's computed area (`HOST_AREA_COMPUTED`), a total area by the calculator's own comment; which of them gives the mullions' value is not read |

## 6. Reproduce

```bash
gh workflow run measure.yml --ref main -f ref=main -f base=none -f nordic=true -f r2025=true \
  -f ifc_flags=--revit-set-placement
gh workflow run measure.yml --ref main -f ref=main -f base=none -f nordic=true \
  -f probe=probe_re235_room_text_parameters
```
