# RE-235 — Where Revit's export writes properties IFC4's definitions do not

**Date:** 2026-10-25
**Issues:** #35 (parameters), W7 of [`docs/release-1.0-criteria.md`](../../docs/release-1.0-criteria.md)
**Decision:** Griffin, 2026-10-25: the default output follows buildingSMART's IFC4 definitions; Revit's placements are written with an option, and W7 is measured with it on. A value Revit's export writes that is known to be wrong is left out.
**Artefacts:** the BIM4LCA office as built and at building permit (Revit 2024, Revit's IFC4 exports), RE1 Architecture (Revit 2025, Revit's IFC2X3 export) and puzzbobb's R2025 models (Revit 2025, Revit's IFC4 exports), as Measure fetches them
**Runs:** Measure [40243309751](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40243309751) (the entities each missing property is on), [40253819602](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40253819602) (`examples/probe_re235_room_text_parameters.rs`), [40254025441](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40254025441) (a first rule, §3) and [40255914190](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40255914190) (`--revit-set-placement` on)
**Result:** positive: with `--revit-set-placement`, 947 more of the values Revit's export writes on the office are written, none different, and none Revit's export does not write.

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

- a text parameter named `<set>.<property>` whose set applies to the element and whose property buildingSMART's definition of the set lacks altogether (`StandardPropertySet::defines`), as `IfcText`;
- each standard set one of whose entities' names begins with the element's, with the element's `Reference`.

## 3. A set matched by name holds only `Reference`

A first rule wrote every property of such a set the element holds under the same name in its other sets. On the office Revit's `Pset_StairFlightCommon` on a stair then differs from the stair's own values: `NumberOfTreads` on 12 of 14, `RiserHeight` on 6, `NumberOfRiser` on 1. No rule built from the stair's other values gives those, so only `Reference`, the type's name, which Revit writes the same in every set, is written.

## 4. Measured

`psets_vs_ifc` with the option on, against the same code without it:

| file | equal, off | on | different | Revit does not write |
|---|---:|---:|---:|---:|
| office as built | 43,906 | 44,853 | 56, both | 495, both |
| office at permit | 44,088 | 45,035 | 56, both | 496, both |
| R2025 Rooms_Stairs | 98 | 100 | 1, both | 0, both |

The 947: 646 coverings, 298 `Pset_SpaceHeaterTypeCommon.Reference`, one air terminal box's `Pset_AirTerminalBoxTypeCommon.Reference` and two stairs' `Pset_StairFlightCommon.Reference` (14 once stairs carry their `Reference`). RE1 Architecture's IFC2X3 export writes neither name-matched set on its 11 spaces, so on RE1 they count as values Revit does not write; rvt-rs writes IFC4.

## 5. Revit values left out

A value Revit's export writes that is known to be wrong is not written, with or without the option:

| property | Revit's value | why it is wrong |
|---|---|---|
| `Pset_StairFlightCommon.WalkingLineOffset`, `Pset_StairCommon.WalkingLineOffset` | half the run's width in feet, read as millimetres (0.00204 m on the office, 0.00164 m on the R2025 models' 1 m runs) | revit-ifc's `WalkingLineOffsetCalculator` returns feet unscaled |

## 6. Reproduce

```bash
gh workflow run measure.yml --ref main -f ref=main -f base=none -f nordic=true -f r2025=true \
  -f ifc_flags=--revit-set-placement
gh workflow run measure.yml --ref main -f ref=main -f base=none -f nordic=true \
  -f probe=probe_re235_room_text_parameters
```
