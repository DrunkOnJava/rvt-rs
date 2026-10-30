# RE-193 — Every element's IsExternal, as Revit's exporter sets it

**Date:** 2026-10-18
**Issues:** #35; follows RE-158 (a door's and a floor's type Function) and RE-185 (a wall type's native Function).
**Artefacts:** the BIM4LCA office, as built and at building permit, with Revit's own IFC export of each (`research/bim4lca-measure.tsv`); RE1 Architecture; the R2025 models (`research/r2025-models.tsv`); Core Interior.
**Probe and scorer:** `examples/probe_re193_is_external.rs` and `tools/re/is_external_vs_ifc.py`, measured in runs [39186463760](https://github.com/DrunkOnJava/rvt-rs/actions/runs/39186463760), [39188815941](https://github.com/DrunkOnJava/rvt-rs/actions/runs/39188815941) and [39190678462](https://github.com/DrunkOnJava/rvt-rs/actions/runs/39190678462).
**Status:** positive for walls, curtain walls, doors, windows, slabs, roofs, columns, beams, coverings, railings, ramps, stairs, proxies and spaces. Negative for curtain mullions and panels, which keep no value: members of one type on the same external curtain walls are 8,460 times not external and 397 times external in Revit's export.

## 1. Before

Revit's export of the office writes `IsExternal` in the common property set of 12,472 elements of the as-built file. rvt-rs wrote it on doors and floors only, from their type's Function (RE-158).

## 2. Revit's rule

Read in revit-ifc `IFC_v24.3.40` (CLEANROOM.md source 5; read and cited, no code copied):

- Its pset definitions create the entry as `PropertySetEntry("Pset_WallCommon.IsExternal", "IsExternal")`, and so for every common set: `PropertySetEntryMap.CreatePropertyFromElementOrSymbol` reads a parameter named `<set>.IsExternal` on the element first, then one named `IsExternal`, and only then the entry's calculator.
- The calculator, `CategoryUtil.IsElementExternal`, takes the first of: a Yes/No parameter named `IsExternal` on the element or its type; the type's `FUNCTION_PARAM`, external unless Interior; a roof is external; a family instance takes its host's value; otherwise not external.

A wall type holds no `FUNCTION_PARAM` entry in its data object (RE-185). Its Function is the type's native `m_function`, which `native_wall_types` reads through the wall's `m_WallAttributesId`.

## 3. The parameters

The office defines 12 project parameters named for a set's `IsExternal`, from `Pset_WallCommon.IsExternal` and `Pset_CurtainWallCommon.IsExternal` to `Pset_RoofCommon.IsExternal`, among 63 named `Pset_*` (RE-156's definitions). A wall carries both `Pset_WallCommon.IsExternal` and `Pset_CurtainWallCommon.IsExternal`, often with different values, and Revit's export writes each in its own set: the wall's in `Pset_WallCommon`, a curtain wall's in `Pset_CurtainWallCommon`. On every element where the parameter of the set Revit writes is read, it is Revit's value. No type carries one; only the element's is read.

## 4. Measured

`is_external_vs_ifc` scores each step against Revit's export, by `Tag` and, for a space, by GlobalId (run 39188815941):

| entity | as built | at building permit | R2025 and reference models |
|---|---|---|---|
| `IfcWall` | 607 of 607: 605 by parameter, 2 by native Function | 589 of 589 | 124 of 124 by native Function |
| `IfcCurtainWall` | 220 of 220 by parameter | 220 of 220 | 1 of 1 |
| `IfcDoor` | 268 of 268: 266 by parameter, 2 by Function; and 1 curtain-panel door | 267 by parameter or Function, 1 with neither; and 2 curtain-panel doors | 11 of 11 by Function |
| `IfcWindow` | 269 of 269 by parameter | 265 of 265 | 6 of 18 by the host wall |
| `IfcSlab` | 41 by parameter; 22 stair landings, equal to their stair's Function | the same | 2 by Function, 13 roofs written as slabs |
| `IfcRoof` | none | none | 6 of 6, as roofs |
| `IfcColumn`, `IfcBeam`, `IfcRamp`, `IfcStair` | 279, 8, 5 and 16, all equal | 345, 8, 5 and 16 | — |
| `IfcCovering`, `IfcRailing` | 111 and 41, all equal | 75 and 41 | 11 railings |
| `IfcSpace`, `IfcBuildingElementProxy` | 276 and 12, all not external, by default | the same | 12 and 54 |
| `IfcMember` | 1,423 of 10,003 by the curtain wall | 1,453 of 10,031 | 16 of 16: RE1's 10 mullions of an interior curtain wall, and 6 stair stringers |
| `IfcPlate` | 227 of 260 by the curtain wall | 237 of 269 | 2 of 2 |

Apart from curtain mullions and panels, no step differs from Revit's on any element; the one member decided by the last step differs too.

## 5. What changes

- `attach_type_functions` reads every element's type Function, and `attach_wall_type_functions` a wall's, and a curtain panel wall's (RE-72), from its native type record (2024 and 2025).
- `attach_is_external` attaches each parameter named `IsExternal` or `<set>.IsExternal` an element carries, and the value Revit's calculator gives (`m_is_external`). A class whose value comes from its type's Function (walls, curtain walls, curtain panels that are walls, doors, windows, floors, building pads) gets none where neither a parameter nor the Function is read. A window does not take its host's value, which is Revit's on 6 of 18 windows of the R2025 models. Curtain mullions and panels get none (§6). Every other class is not external, as the calculator's last step is: on each office file 12 proxies, 276 spaces, 23 railings and 2 stairs take it, with 78 coverings and a column as built and 75 coverings at permit, and on the R2025 and reference models 54 proxies, 12 spaces, 11 railings, 5 stairs and 6 stair stringers written as `IfcMember`, none differing.
- The export writes `IsExternal` in the common set of the entity the element is written as: the parameter named for that set, else the calculated value. A space's goes in the `Pset_SpaceCommon` it already writes.
- The parameters and a type's Function are read on every release from 2024, as RE-158's door and floor Function was. A wall's native Function, and the last step's not external, apply on 2024 and 2025 only, the releases measured; on a 2026 or 2027 file a wall, or an element no parameter or Function decides, gets no value.

## 6. What is not shown

- **Curtain mullions and panels.** On the office Revit's export writes 9,505 of 10,003 members as not external, among them the mullions of curtain walls it writes as external. Taking the curtain wall's value is Revit's on 1,423. (Since 2026-10-30 rvt-rs writes that value on the members, RE-288, RE-269 §5.) Neither the curtain wall nor the member's type decides: mullions of type 1907093 on curtain walls whose `Pset_CurtainWallCommon.IsExternal` and native Function are both external are not external 8,460 times and external 397 times (run 39190678462). Plates split the same way. Neither class gets a value.
- **Windows without a parameter or Function.** On the R2025 models their type's Function is not read, so their 18 windows get no value.
- **Stair landings.** Revit writes the office's 22 as `IfcSlab`, with the value their stair's Function gives. They get the last step's value here, which equals it only where the stair is interior.
- **A parameter on a type** named for a set never occurs in these files and is not read.
- **Core Interior.** Revit's export of it writes no property values at all, so every `IsExternal` rvt-rs writes there is a value Revit's does not have.

## 7. Results

Measure A/B against `main` (f612f97): office files, reference and R2025 models in run [39194883495](https://github.com/DrunkOnJava/rvt-rs/actions/runs/39194883495), an earlier commit of this change against 3b49d17 in run [39191635759](https://github.com/DrunkOnJava/rvt-rs/actions/runs/39191635759) with the same values; Autodesk's samples in run [39192700482](https://github.com/DrunkOnJava/rvt-rs/actions/runs/39192700482). `IsExternal` values against Revit's export (`psets_vs_ifc.py`):

| | main | RE-193 |
|---|---:|---:|
| As built: Revit's 12,375 values equal | 266 | 2,110 |
| As built: different | 29 | 0 |
| At building permit: Revit's 12,420 values equal | 268 | 2,118 |
| At building permit: different | 27 | 0 |
| Reference and R2025 models: Revit's 310 values equal | 13 | 245 |
| Reference and R2025 models: different | 0 | 0 |

On the office every set's values are Revit's where written: walls 607 of 607 (permit 589), curtain walls 220, doors 268 of 269, windows 269 (265), columns 279 (345), beams 8, coverings 111 (75), railings 41, ramps 5, slabs 63, spaces 211, stairs 16 and proxies 12 of 13. The missing ones are the curtain-panel door and a proxy. Members and plates stay missing (§6). On the reference and R2025 models: walls 124 of 124, roofs 19 of 19, doors 11, railings 11, spaces 12, stairs 5, a curtain wall, 54 of 76 proxies, and 6 of the 16 members, which are stair stringers; RE1's 10 curtain mullions, the windows and the plates get none (§6).

- **All property values.** As built, values equal to Revit's go from 12,687 to 14,531 and different ones from 36 to 7; at permit from 12,651 to 14,501 and from 34 to 7. The 29 and 27 `IsExternal` values that differed on `main` came from RE-158's type Function on doors and floors, which the office's parameters override.
- **Values Revit does not write** rise by 85 on the as-built file and 86 at permit: `IsExternal` on objects Revit's export holds under another set or name, such as the 42 Turning Free Space instances rvt-rs writes as proxies where Revit writes spaces (#763). On the reference and R2025 models they rise by 789, 732 of them on Core Interior (§6).
- **Samples.** Of 64 hashed exports, 42 are identical. The 2024 and 2025 samples gain `IsExternal` values (for example Snowdon Towers' architectural model 2,722, its structural model 1,165). Two 2026 and two 2027 samples gain 2 to 6 from their types' Function. None loses one: on 2026 and 2027 the door and floor values RE-158 wrote stay.
- **Curtain panel walls.** Taking a curtain panel wall's value from its native Function rather than the last step (run [39199638398](https://github.com/DrunkOnJava/rvt-rs/actions/runs/39199638398) and run [39199640105](https://github.com/DrunkOnJava/rvt-rs/actions/runs/39199640105), against the commit before) changes one export: Snowdon Towers' architectural sample, whose panel walls have no Revit export on the runners. The office files and the reference and R2025 models are byte-identical.
- **Time.** On one runner, Core Interior's export takes 4.0 s against `main`'s 3.7 s, and the RE1 models' 2 to 7 % longer (run [39194885426](https://github.com/DrunkOnJava/rvt-rs/actions/runs/39194885426)). That comparison does not include the office files. On the A/B's separate runners the as-built export took 10.3 s against `main`'s 12.5 s. The first commit of this change, which searched back from every parameter entry, took 25.0 s against 9.0 s; timing checkpoints put that commit's parameter read at 8.2 s, the walls' native Function at 1.0 s and the type Function at 0.4 s (run [39193583320](https://github.com/DrunkOnJava/rvt-rs/actions/runs/39193583320)). The parameters are now read in one pass over each stream's data objects.

## 8. Reproduce

```bash
gh workflow run measure.yml -f ref=main -f base=none -f nordic=true -f r2025=true -f probe=probe_re193_is_external -f probe_scorer=is_external_vs_ifc
```
