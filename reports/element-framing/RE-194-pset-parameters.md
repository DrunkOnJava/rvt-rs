# RE-194 — Parameters named for a standard property set's property

**Date:** 2026-10-19
**Issues:** #35; follows RE-193 (`IsExternal`), RE-156 (project and shared parameter definitions), RE-158 (Yes/No and integer entries) and RE-47 (stair dimensions).
**Artefacts:** the BIM4LCA office, as built and at building permit, with Revit's own IFC export of each (`research/bim4lca-measure.tsv`); the reference models; Autodesk's sample projects (`research/autodesk-sample-projects.tsv`).
**Probe and scorer:** `examples/probe_re194_pset_parameters.rs` and `tools/re/pset_parameters_vs_ifc.py`, measured in runs [39195062180](https://github.com/DrunkOnJava/rvt-rs/actions/runs/39195062180), [39196264528](https://github.com/DrunkOnJava/rvt-rs/actions/runs/39196264528), [39202359909](https://github.com/DrunkOnJava/rvt-rs/actions/runs/39202359909) and [39211032010](https://github.com/DrunkOnJava/rvt-rs/actions/runs/39211032010).
**Status:** positive for booleans, text, counts, lengths and thermal transmittance in the common set of each entity rvt-rs writes, the glazing set, and a space's covering and occupancy sets. What Revit's export of the office takes from its user-defined property set file is not reproduced (§6).

## 1. Before

A project or shared parameter can be named for a property of a standard property set, such as `Pset_WallCommon.FireRating`. RE-193 wrote `IsExternal` from such parameters. rvt-rs wrote none of the other properties they name.

## 2. Revit's rule

Read in revit-ifc `IFC_v24.3.40` (CLEANROOM.md source 5; read and cited, no code copied):

- **The parameter comes first.** The pset definitions create each entry as `PropertySetEntry("<set>.<property>", "<property>")`. `PropertySetEntryMap.CreatePropertyFromElementOrSymbol` reads the element's parameter named `<set>.<property>` before the plain name, the type's value and the entry's calculator.
- **A common set goes where IFC4 applies it.** `ExporterUtil.GetApplicablePropertySets` gives an element the sets whose definition lists its entity, or a supertype of it. The definitions are IFC4's (buildingSMART IFC4 ADD2 TC1): `Pset_WallCommon` applies to `IfcWall`, `Pset_DoorWindowGlazingType` to `IfcDoor` and `IfcWindow`, `Pset_SpaceOccupancyRequirements` to `IfcSpace`.
- **A view leaves some sets out.** `IFCCertifiedEntitiesAndPSets.json` lists the common sets an export may create for each model view. IFC4 Reference View lists neither `Pset_DoorWindowGlazingType` nor `Pset_SpaceOccupancyRequirements`.
- **A user-defined property set file adds sets** with its own entities, properties and value types.

## 3. Where the values are

The probe lists each project and shared parameter whose name starts with `Pset_` (RE-156's definitions). For every element of the partition decode, it prints that parameter's entry in the element's own data object (RE-153), read every way a value has been seen stored. On the office the definitions name fifteen property sets, and the values sit beside the parameter's ElementId (runs 39195062180 and 39196264528):

- a Yes/No or integer as the `u32` after it (RE-158);
- a text as `u32 n · UTF-16 × n` after it (RE-156);
- a number as the `f64` immediately before it: a length in feet (a stair's `RiserHeight`), a thermal transmittance in W/(m²·K) as Revit's export writes it.

The probe reads the element's own entry, and that is where Revit's export takes the value from (§4). Where an element's copies in several partitions hold one, the latest partition's is taken (#800). Of the models measured, only the office defines such parameters.

On the as-built file (run [39211032010](https://github.com/DrunkOnJava/rvt-rs/actions/runs/39211032010)), each reading equals Revit's value wherever Revit's export writes the property on the element carrying the parameter:

| parameters | read as | equal to Revit's |
|---|---|---:|
| `IsLaminated`, `IsTempered` (`Pset_DoorWindowGlazingType`) | `u32` | 1,381 each |
| `LoadBearing` (walls, columns, slabs, beams, stairs, ramps) | `u32` | 949 |
| `HandicapAccessible` (doors, spaces, ramps), `FireExit` (windows, ramps) | `u32` | 738 |
| `AcousticRating` (walls, curtain walls, doors, windows, slabs) | text | 627 |
| `CeilingCovering`, `FloorCovering`, `WallCovering` (`Pset_SpaceCommon`) | text | 646 |
| `FireRating` (walls, curtain walls, doors, windows, slabs) | text | 141 |
| `ThermalTransmittance` (windows, walls, doors, curtain walls, slabs) | `f64` before, W/(m²·K) | 337 |
| `OccupancyNumber`, `NumberOfRiser`, `NumberOfTreads` | `u32` | 55, 13, 13 |
| `RiserHeight`, `TreadLength` | `f64` before, feet | 13, 13 |

Where an element carries a text parameter whose property Revit's export does not write on it, the text is empty: all 939 such entries on the as-built file. A Yes/No Revit's export does not write is a stored 0 or 1, left out for the reasons of §4.

## 4. Where Revit's export writes them

Every value Revit's export writes from such a parameter is on the element itself, in the set the parameter names; none is only on the element's type (run 39202359909). Which elements get a set follows two rules:

- **A common set goes on the entities IFC4 gives it.** A wall carries `Pset_WallCommon.IsExternal` and `Pset_CurtainWallCommon.IsExternal`. Revit's export writes the first on the 605 walls, and the second on none of them. `Pset_WallCommon` is not written on the 220 curtain walls or the 21 walls exported as coverings that carry its parameters, nor `Pset_RoofCommon` on the 12 coverings that carry its `IsExternal`.
- **An empty text is not written.**

The office's export is IFC4 Reference View 1.2, by Revit 2024's own exporter (IFC 24.0.5). It also carries what only a user-defined property set file gives:

- Finnish sets (`FI_Kohde`, `FI_RakennuksenSijainti`, ...);
- `Pset_SpaceCommon.CeilingCovering`, `FloorCovering` and `WallCovering` as `IfcText`, though IFC4's `Pset_SpaceCommon` has no such properties;
- `Pset_SpaceOccupancyRequirements.OccupancyNumber` as `IfcInteger`, which IFC4 declares an `IfcCountMeasure`;
- `Pset_DoorWindowGlazingType` with `IsLaminated` and `IsTempered` alone. It is on every element carrying those parameters (605 walls, 220 curtain walls, 21 coverings, 266 doors and 269 windows), and on 115 openings.

The Reference View certifies neither the glazing set nor the occupancy set (§2). Both therefore come from that file, and so does the glazing set's absence of `IsWired`: every one of the 1,337 elements carrying `Pset_DoorWindowGlazingType.IsWired` holds 0, the value `IsLaminated` holds where Revit's export writes No, and the export writes `IsWired` on none. So the export cannot say whether Revit takes that 0 as No or as no value.

## 5. What changes

- `ifc::standard_property_sets` lists IFC4's property sets for these parameters: the common set of each entity rvt-rs writes, `Pset_DoorWindowGlazingType`, and a space's `Pset_SpaceCoveringRequirements` and `Pset_SpaceOccupancyRequirements`. For each set it gives the entities it applies to, with their subtypes, and its properties of the kinds read here: boolean, identifier, label, text, count, length, positive length and thermal transmittance. A parameter named for another IFC4 set is not written (#817).
- `partition_room_parameters::scan_parameter_entries` reads each parameter's value by how it is stored (`EntryKind`). It searches only the payloads of the elements' own data objects, an entry being its innermost object's. `scan_integer_parameters` (RE-193) is now a call to it.
- `partition_schema_mvp::attach_named_parameters` gives each element a field for each such parameter it carries. A Yes/No is taken only as 0 or 1, a positive length only above 0, and any number only when finite and not subnormal. An empty text is the latest copy's value too: it hides an older copy's text and gives nothing. `IsExternal` stays RE-193's.
- The export writes each value in its set on the element, when the set applies to the entity the element is written as, as the IFC4 type of the property. The value replaces rvt-rs's own, as Revit's exporter reads the parameter first: a stair's `RiserHeight` from the parameter instead of RE-47's record. The sets are given once the entity an element is written as is final: a door or window written as its opening alone (RE-84) gets none, and one left a door because its host is not found keeps them.
- A set given late merges into the element's set of the same name. A stair's `IsExternal`, which RE-193 wrote in a second `Pset_StairCommon` beside RE-47's, now joins it.
- `PropertyValue::ThermalTransmittance` writes `IfcThermalTransmittanceMeasure`. A model with one adds kelvin and W/(m²·K) to its unit assignment, as Revit's export declares them.

## 6. What is not written

- **What the office's user-defined file adds** (§4): the glazing set on walls, curtain walls and coverings, and the covering properties in `Pset_SpaceCommon`. `OccupancyNumber` is written as the `IfcCountMeasure` IFC4 declares, which the scorer counts as different from the export's `IfcInteger`. `IsWired`, a stored 0 on every door and window, is written there as No; the export writes none (§4).
- **Values Revit takes from elsewhere when an element carries no such parameter.** On the as-built file Revit's export writes these on elements whose own data object holds no entry for the parameter: `FireRating` on 26 walls and 2 doors, `LoadBearing` on 2 walls and a column, and `NumberOfTreads` on a stair. They come from the exporter's next sources: a built-in parameter, the type, or a calculator. None is written here.
- **Other kinds of values:** plane angles, areas, ratios and enumerations (`Status`) are not read.
- **Other IFC4 sets** the entities rvt-rs writes take, such as `Pset_SpaceFireSafetyRequirements` or `Pset_ConcreteElementGeneral`, are not listed, so a parameter named for one is not written (#817). The office defines none.
- **A parameter on a type** named for a set never occurs in these files and is not read.
- **Releases.** The parameter definitions are read from Revit 2024 on (RE-156); only Revit 2024 files define such parameters among those measured.

## 7. Results

Measure A/B against `main` (8087e60): the office files and the reference models in runs [39211032010](https://github.com/DrunkOnJava/rvt-rs/actions/runs/39211032010) and, at this change's last code commit, [39223687653](https://github.com/DrunkOnJava/rvt-rs/actions/runs/39223687653); Autodesk's samples in run [39223693806](https://github.com/DrunkOnJava/rvt-rs/actions/runs/39223693806). All property values against Revit's export (`psets_vs_ifc.py`), the same in both runs:

| | main | RE-194 |
|---|---:|---:|
| As built: Revit's 57,391 values equal | 14,531 | 18,374 |
| As built: different | 7 | 55 |
| At building permit: Revit's 57,570 values equal | 14,501 | 18,373 |
| At building permit: different | 7 | 55 |

- **Where Revit's export has a value, the one written is Revit's, but for one property's type.** On both files each property the parameters give is equal wherever both write it: for example `Pset_WallCommon.LoadBearing` 605 of 607 as built, `Pset_WindowCommon.ThermalTransmittance` 265 of 265, `Pset_DoorCommon.HandicapAccessible` 266 of 266, `Pset_SpaceCommon.HandicapAccessible` 190 of 190. `Pset_DoorWindowGlazingType.IsLaminated` and `IsTempered` are equal on 535 of Revit's 1,381 elements as built, the doors and windows; the other 846 are the walls, curtain walls and coverings of §6. The 55 differences are `OccupancyNumber`, `IfcCountMeasure` against the export's `IfcInteger` with the same number.
- **The 7 values that differed on `main` are now Revit's.** All were stairs: 6 `RiserHeight` and 1 `NumberOfRiser` from RE-47's record, which the parameters replace.
- **Values Revit does not write** rise by 536 as built and 533 at permit (`psets_vs_ifc` now counts them by set and property):
  - `IsWired`, on 494 and 490 doors and windows (§6);
  - `HandicapAccessible`, on 42 rooms whose number and name rvt-rs does not read. The scorer keys a space by its name, so it cannot pair them with Revit's; paired by GlobalId, the probe finds all 42 equal;
  - at permit, `LoadBearing` on a column Revit's export leaves out.

  A commit of this change also wrote 80 texts at permit that Revit's export does not have: `FireRating` on 74 columns, and an acoustic or fire rating on 6 walls. Each was an older partition's copy of a text the latest copy holds empty. An empty text now counts as the latest copy's value, and nothing is written for it (run [39216007512](https://github.com/DrunkOnJava/rvt-rs/actions/runs/39216007512); the values equal to Revit's are unchanged).
- **The reference models' exports are byte-identical.** Core Interior, Einhoven and the four RE1 models define no such parameters.
- **Schema.** The office files are the only exports with a thermal transmittance and its unit. Both pass IfcOpenShell's validation with no finding, and the attribute-count check, as on `main` (run [39219085980](https://github.com/DrunkOnJava/rvt-rs/actions/runs/39219085980), where Measure now validates the reference models' exports).
- **Samples.** Of 64 hashed exports, 60 are identical. The other four are Revit 2024 samples (rac_advanced, rac_basic, rst_basic and Snowdon Towers' architectural model), where 5, 2, 1 and 26 stairs carry one `Pset_StairCommon` instead of two (§5). Their property values are unchanged.
- **Time.** On one runner, the office exports take 1.01 times as long as `main`'s (13.5 s against 13.4 s as built, 14.7 s against 14.5 s at permit), and the reference models' 1.00 to 1.04 times (run [39226661835](https://github.com/DrunkOnJava/rvt-rs/actions/runs/39226661835), whose timing job now includes the office files). The A/B's separate runners had shown 2 to 17 % longer office exports from one run to the next.

## 8. Reproduce

```bash
gh workflow run measure.yml -f ref=main -f base=none -f nordic=true -f probe=probe_re194_pset_parameters -f probe_scorer=pset_parameters_vs_ifc
```
