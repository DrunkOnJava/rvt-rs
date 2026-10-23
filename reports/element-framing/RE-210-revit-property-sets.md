# RE-210 — Revit's own property sets, one per parameter group

**Date:** 2026-10-22, measured against Revit's own sets 2026-10-23
**Issues:** #35 (M4-07, the second slice); builds on RE-207 (an element's saved parameters, named by the file's definitions).
**Artefacts:** the BIM4LCA office, as built and at building permit, and the housing on terrain, each with Revit 2024's own IFC export (VTT, CC BY-SA 4.0); puzzbobb's R2025 models (`R2025_Models.zip`, Apache-2.0, #356); Core Interior and the RE1 models (CI's project corpus); revit-ifc's `Release_23.x.x` to `Release_27.x.x` and `master` branches (LGPL, CLEANROOM.md source 5, read and cited, no code copied); web-ifc's public test file `ISSUE_159_kleine_Wohnung_R22.ifc` (a German Revit 2022 export with Revit's own property sets); Autodesk's Snowdon Towers Sample HVAC, Site and Architectural, 2025 edition (sha256 `a1756cfd…31bfe`, `43140cd8…c1ff8`, `8ac90a85…0d334`), each with Revit 2025's IFC4X3 export with "Export Revit property sets" ticked (`3505ed9e…af6d8`, `4f34da2e…7a5f`, `18df1f11…df466`). The Snowdon files are Autodesk's; they are measured locally and never committed, as in `reports/validation/held-out-snowdon-mep-2026-09-27.md`.
**Probe and scorer:** `examples/probe_re210_revit_property_sets.rs`; `tools/re/revit_property_sets_vs_ifc.py` scores an export made with the option against Revit's export with "Export Revit property sets".
**Status:** partial. On Snowdon HVAC and Site, every one of the 28,906 and 718 properties rvt-rs writes in Revit's own sets (counted per element, a type object's once for each of its elements) is in the set of the same name, on the element or type object of the same Tag, with Revit's value (§5). On Snowdon Architectural, 58,346 of 58,733 are. Of the other 387, 369 are in sets Revit names with `(Type)` on 13 type objects it does export, an artefact of the order its exporter works in, and 18 are the type category of curtain panels, which rvt-rs wrote before this change. Revit writes 79,907, 948 and 118,432 more, almost all built-in parameters, which rvt-rs does not name (#255).

## 1. Before

RE-207 read each element's saved parameters and listed them in `rvt-elements` and Python, but wrote nothing new to IFC. `rvt-ifc` wrote an element's parameters only where they are named for a standard set's property (RE-194), plus a few of Revit's own properties (Category, Family, Type, phases) in sets named `Other`, `Identity Data` and `Phasing`.

## 2. What Revit's exporter writes

With "Export Revit property sets" ticked, Revit's exporter writes an element's parameters grouped by parameter group, one set per group. This is how revit-ifc's `Release_25.x.x` branch does it:

- **Which parameters.** `ParameterUtil.CacheParametersForElement` takes the element's visible, named parameters in parameter id order and groups them by their group's type id. It leaves out the IFC group (`GroupTypeId.Ifc`), whose parameters drive the export instead. Within a group it keeps one parameter per name with its spaces removed (`NamingUtil.RemoveSpaces`), the first.
- **Instance and type.** `PropertyUtil.CreateInternalRevitPropertySets` makes the element's sets from its own parameters and, separately, its type's sets from the type's parameters. A type's sets go on its type object. A type with no type object gets its sets on its elements, each name followed by `(Type)` (`Properties.Resources.PropertySetTypeSuffix`). A family type's parameters are its type parameters, so a family's instance parameters are not on the type object.
- **`(Type)` on a type object.** Whether the type has a type object is asked when the element's sets are made (`ExporterCacheManager.ElementTypeToHandleCache.IsRegistered`), and the sets made then are kept for the type (`CreatedInternalPropertySets`). A type whose type object is created only afterwards gets `(Type)` sets, and its type object carries them too. On Snowdon Architectural, 13 of the 1,286 type objects with Revit's sets have them: 5 of the 8 curtain wall types, 6 type objects of one scupper type and 1 of a towel dispenser, and 1 wall type. Which types are hit depends on the order Revit exports elements in, which the file does not record.
- **Set names.** Each set is named by `LabelUtils.GetLabelForGroup` for its group: `Dimensions`, `Identity Data`, `Materials and Finishes`, and so on. A parameter with no group is in `Other`.
- **Values.** A Yes/No is written as `IfcBoolean`, and any other integer with a spec as `IfcInteger`. An integer with no spec is an enumeration, written as `IfcIdentifier` by its display text. A text is `IfcText` unless it is empty, which Revit leaves out. A real follows its spec (`CreateRealPropertyByType`): a length, distance, pipe size, duct size or another length-like spec is a length measure; an area, volume or angle its measure, the angle a plane angle; a number `IfcReal`. An ElementId is written by the display name of what it names (`Parameter.AsValueString`), unless it is −1, which Revit leaves out. On Snowdon Architectural that is a material's name, and a family type's `<family> : <type>`, or the type's name alone where the family has the same name (`Muntin Pattern_2x2`).

The set's GlobalId (§7) is the hash of the element's export GUID, `"Sub-element:IfcPropertySet: "` and the group's type id on every current release branch, 23.x to 27.x. revit-ifc's first Revit 2023 exporter (`de98ad5`) still used a sub-element index for the groups it knew, and so does the Revit 2022 export above: its sets' GlobalIds are the element's GlobalId with a sub-element index per group (`ID-Daten`, Identity Data, 1537; `Bemaßungen`, Dimensions, 1538; `Abhängigkeiten`, Constraints, 1556; …). Its `(Type)` sets are named `ID-Daten(Type)`, `Sonstige(Type)` and so on: the suffix is not translated.

## 3. What rvt-rs writes

`rvt-ifc --revit-property-sets` (`src/ifc/revit_property_sets.rs`) does the same from RE-207's parameters:

- An element's instance-level named parameters go in sets named by their group. A set rvt-rs already writes under that name (`Other`, `Identity Data`, `Phasing`) takes the properties it does not hold.
- Its type's parameters go once per type on the type object (`IfcModel::type_property_sets`, merged by name into the sets the type object already has). A type with no type object has its sets put on its elements with `(Type)` added. None of the measured models has one.
- Values are written as in §2 for Yes/No, integers with a spec, non-empty texts, and reals whose spec is a length, pipe size, duct size, area, volume, angle or number. Lengths, areas and volumes go through the export's unit conversion like every other measure, and angles stay in radians as `IfcPlaneAngleMeasure`.
- A real whose spec is an air flow, an HVAC or piping pressure, an apparent or electrical power, an electrical potential or a current is written as `IfcVolumetricFlowRateMeasure`, `IfcPressureMeasure`, `IfcPowerMeasure`, `IfcElectricVoltageMeasure` or `IfcElectricCurrentMeasure`, the measures `CreateRealPropertyByType` gives them. The value is converted from Revit's internal unit (ft³/s, kg/(ft·s²), kg·ft²/s³, kg·ft²/(s³·A), A) to m³/s, Pa, W, V or A. The export's `IfcUnitAssignment` then also declares those units, as Revit's export does, and only when one such value is written.
- An ElementId naming a material is written as an `IfcLabel` holding the material's name, and one naming a family type as `<family> : <type>`, or the type's name where the family has the same name.
- A type whose type object rvt-rs writes always has its sets under the group's name: the types Revit gives `(Type)` sets on their type objects follow from the order of its export.

Left out and counted (the probe gives the reasons): parameters with no name (built-ins, #255), the IFC group, enumerations, ElementIds rvt-rs cannot name, and reals of other specs (electrical frequency, demand factor and luminous intensity, HVAC temperature, currency). The group labels are Revit's English labels for the 24 groups the measured models use; a group outside `GROUP_LABELS` is left out.

Without the option, the export is unchanged.

## 4. What is written

`probe_re210_revit_property_sets MODEL.rvt` exports the model as `rvt-ifc --revit-property-sets` does:

| model | elements | type objects | properties on elements | on type objects |
|---|---:|---:|---:|---:|
| R2025 models 12 to 17, each | 13 | 5 | 55 | 32 |
| RE1 Architecture | 51 | 35 | 64 | 159 |
| RE1 Electrical | 48 | 17 | 344 | 557 |
| RE1 Mechanical | 43 | 7 | 889 | 22 |
| RE1 Plumbing | 61 | 12 | 706 | 28 |
| Core Interior | 6 | 5 | 54 | 23 |
| BIM4LCA housing on terrain | 49 | 13 | 117 | 96 |
| BIM4LCA office, as built | 2,322 | 86 | 13,600 | 1,127 |
| BIM4LCA office, building permit | 2,337 | 78 | 13,440 | 1,064 |

Over the 15 models, 32,812 properties are written. Of the named parameters left out:

- 9,920 are empty texts and 2,425 ElementIds of −1, which Revit leaves out too;
- 324 are in the IFC group;
- 408 are ElementIds rvt-rs cannot name;
- 62 are reals of other specs.

The RE1 MEP models' 56 flows, pressures, powers, voltages and currents have no Revit export with these sets to compare with. Their voltages are all nominal (120, 208 or 240 V), and `tests/revit_property_sets.rs` asserts that and that each of the five units is declared where its values are written. With the unit declaration disabled, it fails ("RE1-Electrical: 17 values of IFCPOWERMEASURE, no POWERUNIT declared").

IfcOpenShell 0.9.0's validator finds no issue in the exports of R2025 model 12, RE1 Architecture and the housing on terrain.

## 5. Measured

### Against Revit's own sets: Snowdon Towers HVAC

`rvt-ifc MODEL.rvt -o ours.ifc --revit-property-sets`, scored with `tools/re/revit_property_sets_vs_ifc.py ours.ifc REVIT.ifc` against Revit 2025's export with "Export Revit property sets". rvt-rs exports 2,596 elements and Revit's export 2,627. The other 31 are flex ducts, which rvt-rs does not export.

| level | set | written by rvt-rs | equal to Revit's | Revit's, not written by rvt-rs |
|---|---|---:|---:|---:|
| element | Dimensions | 4,014 | 4,014 | 6,891 |
| element | Mechanical - Flow | 907 | 907 | 15,668 |
| element | Other | 9,524 | 9,524 | 4,212 |
| element | Constraints, Graphics, Identity Data, Insulation, Lining, Mechanical, Phasing | 0 | — | 45,181 |
| type object | Constraints | 43 | 43 | 1,543 |
| type object | Dimensions | 4,052 | 4,052 | 0 |
| type object | Electrical - Loads | 74 | 74 | 0 |
| type object | Electrical Engineering | 37 | 37 | 0 |
| type object | Identity Data | 1,543 | 1,543 | 3,216 |
| type object | Mechanical | 223 | 223 | 1,090 |
| type object | Mechanical - Flow | 37 | 37 | 0 |
| type object | Model Properties | 74 | 74 | 0 |
| type object | Other | 8,350 | 8,350 | 2,106 |
| type object | Text | 28 | 28 | 0 |
| | **total** | **28,906** | **28,906** | **79,907** |

The counts are per element: a type object's property counts once for each element of the type. They include the Category, Family and Type values rvt-rs wrote in `Other` before this change. What `--revit-property-sets` adds is 5,772 properties on 1,469 elements and 17 type objects (the probe).

So on this model the group labels, the set each parameter lands in, the split between the element and its type object, angles as plane angles, numbers as `IfcReal`, and lengths, flows, pressures, powers and voltages in their measures are all Revit's. No property is written in a set where Revit's export does not have it.

- The scorer compares angles after each file's angle unit (Revit's export declares degrees) and allows rvt-rs's rounding to six decimals; before both, 4,281 of the values counted as differing, all of them for those two reasons.
- The 1,055 flows, pressures, powers and voltages and the 37 in `Electrical Engineering` were left out before §3's specs and that group were added.
- What Revit writes and rvt-rs does not is built-in parameters: System Classification, System Name, Phase Created, Size, Flow, the elevations, Family and Type, Mark, OmniClass and so on, which rvt-rs does not name (#255). The exception is two family parameters, `OffsetWidth` and `OffsetHeight` (43 each, on duct transitions). They are not among the parameters rvt-rs reads on those elements; why is not known.

### Against Revit's own sets: Snowdon Towers Site

The same, on the Site model: walls, floors, railings and generic models. Counted per element as above, rvt-rs writes 718 properties in Revit's own sets, and all 718 are Revit's. Most were written before this change: the elements' `Other` and the type objects' `Identity Data` and `Other`. The parameters add 8 properties, in the `Dimensions` of 2 type objects, which count 213 times on those types' elements. Revit writes 948 more, all built-in parameters: Phase Created, Area, Volume, Level, Type Mark, Assembly Code, the walls' and floors' constraints and construction, the types' analytical properties.

- Revit writes one floor as five `IfcSlab`s of the same Tag, and only one of them holds `Other`, so the scorer takes the sets of all the products of a Tag together. Before that, 5 of the floor's properties were counted as not written by Revit.
- Revit writes `(Type)` sets, `Other(Type)`, `Identity Data(Type)` and `Materials and Finishes(Type)`, on 12 of the model's 13 sites, 3 storeys and 5 openings: a Revit 2025 English export, so §2's suffix holds there too. rvt-rs exports none of these as elements, so it writes no `(Type)` set on this model.
- rvt-rs exports 64 of Revit's 74 element Tags, leaving out its 10 `IfcGeographicElement`s.

### Against Revit's own sets: Snowdon Towers Architectural

The same, on the Architectural model, the first measured with doors, windows, stairs, curtain walls, furniture and lighting. Of Revit's 5,994 element Tags, rvt-rs exports all but 49.

| level | set | written by rvt-rs | equal to Revit's | in Revit's `(Type)` set | differ | Revit's, not written by rvt-rs |
|---|---|---:|---:|---:|---:|---:|
| element | Other | 29,275 | 29,275 | 0 | 0 | 4,658 |
| element | Dimensions, Construction, Materials and Finishes, Graphics, Constraints, Data, Plumbing, Structural | 3,055 | 3,055 | 0 | 0 | 44,242 |
| type object | Other | 11,892 | 11,670 | 204 | 18 | 903 |
| type object | Identity Data | 5,490 | 5,389 | 101 | 0 | 10,612 |
| type object | Dimensions, Constraints | 4,502 | 4,438 | 64 | 0 | 9,952 |
| type object | Materials and Finishes, Construction, Electrical, Electrical - Lighting, Electrical - Loads, Electrical Engineering, Graphics, Photometrics, Text, Data | 4,519 | 4,519 | 0 | 0 | 24,350 |
| | other sets Revit writes and rvt-rs does not | 0 | — | — | — | 23,715 |
| | **total** | **58,733** | **58,346** | **369** | **18** | **118,432** |

Counted per element, as above. What `--revit-property-sets` adds is 5,885 properties on 1,068 elements and 169 type objects (the probe).

- **`(Type)` on a type object (369).** These are the 13 type objects of §2 whose sets Revit names with `(Type)`: rvt-rs writes the same values under the group's name. 66 are the parameters' (the scupper's dimensions and constraints, the towel dispenser's panel size), and 303 the Category, Family Name and Type Name rvt-rs wrote before this change.
- **The type category of curtain panels (18).** Revit exports a curtain panel of a basic wall type as an `IfcPlate`, and its type's Category is `Walls`, the type's own. rvt-rs exports it as an `IfcCurtainWall` and gave its type the panel's category, `Curtain Panels`, before this change.
- **Added on this model.** Before these were added, 337 of the values were left out: the groups `Photometrics` (82) and `Plumbing` (15, 10 of them piping pressures), the currents (42) as in §3, and the family type values (198). All 337 now equal Revit's. A family type is written `<family> : <type>`, except where both have the same name: 34 muntin patterns, such as `Muntin Pattern_2x2`, are written by the type's name alone.
- **What Revit writes and rvt-rs does not.** Of the 118,432, 186 are parameters rvt-rs names on the element or its type. 66 are the `(Type)` values above, which rvt-rs writes under the group's name. 117 are the types' `Family Thumbnail`, an ElementId Revit writes by the name of the type's preview image (`1233119_typeImage`), which rvt-rs does not name, and 3 a family type rvt-rs does not name. The rest are built-in parameters: Phase Created, Volume, Area, Level, Length, Assembly Code, Host, Thickness, the walls' constraints, the materials' thermal values, the stairs' and railings' component sets, and so on.

### Against the local exports' common sets

The local exports were made without Revit's own property sets. Where they also write a property of the same name on the same element, the values can still be compared (scratch cross-check, any set of Revit's export by Tag):

| model | properties added, per element with its type object's | same name in Revit's export | equal |
|---|---:|---:|---:|
| BIM4LCA office, as built | 24,366 | 498 | 498 |
| R2025 model 12, RE1 Architecture, Core Interior, housing on terrain | 1,979 | 0 | — |

The 498 are the Yes/No `FireExit` of 266 doors and 232 windows. rvt-rs writes it in `Identity Data` and Revit's export in `Pset_DoorCommon` and `Pset_WindowCommon`, both as `IfcBoolean`, all equal. The other values are RE-207's, which equal Revit's wherever its exports show them (7,606 of 7,606 on the office as built).

`tests/revit_property_sets.rs` asserts the 498 against the office's export. It fails, 498 of 498, when the sets are not attached.

## 6. Not measured yet

The Snowdon models have no element of a type lacking a type object that rvt-rs exports, and no enumeration that rvt-rs writes. The RE1 MEP models' currents and R2025 models 12 to 17's family type values are written as on Snowdon Architectural but have no Revit export with these sets. An IFC4 export of R2025 models 01 and 12 to 17 with "Export Revit property sets" ticked would measure them, with rooms.

## 7. Not written

- **Built-in parameters.** No name (#255).
- **Enumerations.** An integer with no spec is written by its display text, which the file does not hold.
- **Elements rvt-rs does not name.** An ElementId naming anything other than a material or a family type rvt-rs reads, such as a type's preview image (`Family Thumbnail`).
- **`(Type)` on a type object.** Which types Revit gives such sets follows from the order of its export (§2).
- **Other specs.** The electrical, HVAC, structural and currency reals not in §3 each have their own measure and unit scale; none that rvt-rs names is in a measured Revit export.
- **The sets' GlobalIds.** rvt-rs gives every property set its own GlobalId. Revit's is the hash in §2, which needs the group's type id at write time.

## 8. Cost

Reading the parameters decodes each element's native record, as for `rvt-elements` (RE-207 §8), and only with the option. With hyperfine (`-N --warmup 1 --runs 3`), on the same machine as RE-207 §8:

| model | `rvt-ifc` (s) | with `--revit-property-sets` (s) | ratio |
|---|---:|---:|---:|
| Core Interior | 6.785 ± 0.027 | 7.809 ± 0.015 | 1.15 |
| BIM4LCA office, as built | 24.262 ± 0.027 | 34.036 ± 0.206 | 1.40 |
