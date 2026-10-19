# RE-190: the "Export to IFC As" values Revit's 2024 export follows on the BIM4LCA office

**Date:** 2026-10-18
**Issues:** #792; follows RE-45 (the four IFC export parameters), RE-188 (a 2024 curtain wall's value is not followed) and RE-153 (data objects).
**Artefacts:** the BIM4LCA office, concrete structure, as built and at building permit, with Revit's own IFC export of each (`research/bim4lca-measure.tsv`, CC BY-SA 4.0, read by byte range for a Measure run, never stored); the six reference models; Autodesk's sample projects.
**Probe and scorers:** `examples/probe_re45_ifc_export_parameters.rs`, `tools/re/export_parameters_vs_ifc.py`, `tools/re/class_sets_vs_ifc.py`.
**Status:** positive on both office files: 185 elements of the as-built file and 165 of the permit file now have the class Revit gives them. Measured by Measure on GitHub's runners, run 39174180136 (head e77b49d, base `main` 05cecf9).

## 1. Before

`main` follows an element's "Export to IFC As" value, or its type's, when it names `IfcShadingDevice`, `IfcSlab` or `IfcCovering` (RE-45), and not on a 2024 curtain wall (RE-188). On the as-built office, `class_sets_vs_ifc` pairs Revit's elements with rvt-rs's by Tag and finds these groups in another class than Revit's:

| Revit's class | rvt-rs's class | elements | Revit's PredefinedType |
|---|---|---:|---|
| `IfcWasteTerminal` | `IfcSanitaryTerminal` | 78 | `FLOORTRAP` 62, `ROOFDRAIN` 16 |
| `IfcElectricAppliance` | `IfcBuildingElementProxy` | 51 | `ELECTRICCOOKER` 18, `FRIDGE_FREEZER` 16, `DISHWASHER` 16, `USERDEFINED` 1 |
| `IfcFireSuppressionTerminal` | `IfcBuildingElementProxy` | 40 | `HOSEREEL` |
| `IfcSanitaryTerminal` | `IfcFurniture` | 15 | `WASHHANDBASIN` |
| `IfcAirTerminal` | `IfcBuildingElementProxy` | 1 | `USERDEFINED` |

Each element's type object in Revit's export is the type entity of its class with the same PredefinedType (`IfcWasteTerminalType FLOORTRAP` and so on).

## 2. The values

`probe_re45_ifc_export_parameters` lists every RE-45 entry, and `export_parameters_vs_ifc` compares each value with Revit's export. On both files these groups take their entity from their type's "Export Type to IFC As" (`IfcWasteTerminalType`, `IfcElectricApplianceType`, `IfcFireSuppressionTerminalType`, `IfcSanitaryTerminalType`, `IfcAirTerminalType`). Their predefined types come from the type's "Type IFC Predefined Type", the values in the table above. Revit's exporter reads an element's own value and then its type's (`ExporterUtil.GetIFCExportElementParameterInfo` in revit-ifc, CLEANROOM.md source 5), which is the order `ExportParameters::effective_export_as` already takes.

## 3. What changes

- `category_map::EXPORT_OVERRIDE_TARGETS` adds the five entities, each with `NOTDEFINED` as its default predefined type. `PREDEFINED_TYPE_ENUMERATORS` lists their IFC4 enumerators, so the type's predefined type is written only when it is one of them.
- The STEP writer gives `IfcWasteTerminal` and `IfcFireSuppressionTerminal` their type entities (`IfcWasteTerminalType`, `IfcFireSuppressionTerminalType`). Both count as distribution elements, so their ports and systems are written as for the other terminals.
- A room's space contains its waste, fire suppression and air terminals and its electric appliances, as Revit's export does. Without that, 152 elements moved from their rooms to their storeys.
- **The owner of an entry, on Revit 2024 and later.** RE-45 gave an entry to the last element-data header within 0x800 bytes before it. On the permit file that rule gave `IfcSpace` entries to doors (37), curtain walls (13), sketch lines (11) and plumbing fixtures (10). On five of the fixtures, drains, the misread `IfcSpace` hid their types' `IfcWasteTerminalType`. Their type was then written twice under one GlobalId, which is four schema-validation findings. From 2024 an element's data is a data object whose Adler-32 verifies (RE-153), so an entry now belongs to the data object whose payload holds it, and an entry in no data object is not read. Revit 2023 and earlier keep the header rule.

  The RE-45 probe prints each entry's owner under both rules:
  - **Core Interior:** the rules agree on all 48 entries, so its 21 owners are RE-45's.
  - **As-built office:** they disagree on 264 IFC values. The data objects holding them are:
    - 190 `IfcSpace` values, in rooms;
    - 40 `IfcRailing` values, in railings;
    - 34 values (29 `IfcSpace`, 5 `IfcRamp`), in declared elements of categories the probe does not name.

    The header rule had given those values to doors, mullions, sketch lines, plumbing fixtures or no element.
  - **Permit file:** 583 of its `IfcSpace` values are in rooms' data objects.

  The probe's other differing entries are parameter-group strings (`autodesk.parameter.group:…`), which are no IFC value and which the reader does not take.

## 4. Results

Run 39174180136, base `main` (05cecf9), head e77b49d:

| class (Revit's) | as built, before → after | permit, before → after |
|---|---|---|
| `IfcWasteTerminal` | 0 → 78 of 78 | 0 → 78 of 78 |
| `IfcElectricAppliance` | 0 → 51 of 51 | 0 → 31 of 51 |
| `IfcFireSuppressionTerminal` | 0 → 40 of 40 | 0 → 40 of 40 |
| `IfcSanitaryTerminal` | 129 → 144 of 144 | 129 → 144 of 144 |
| `IfcAirTerminal` | 0 → 1 of 1 | 0 → 1 of 1 |
| elements whose type entity is Revit's | 12,170 → 12,355 of 12,358 | 11,761 → 11,926 of 11,929 |

- On the permit file, 20 electric appliances stay proxies. rvt-rs joins no type to them, as on `main`, so their types' value does not reach them. On the permit file rvt-rs types no column (345), 52 doors, 43 light fixtures and these 20 appliances, where it types them on the as-built file. That is a type-join gap of the permit file, not of these values.
- Both files' exports pass IfcOpenShell's schema validation with no finding. Before the owner change the permit file had the four findings of section 3.
- Storeys and room containment are unchanged. `psets_vs_ifc` counts 15 more values rvt-rs writes and Revit does not, on each file. It does not say which.
- The six reference models' exports are byte-identical. On the samples (run 39174182276), the 147 diagnostics, schema and `rvt-info` outputs are unchanged.
- An earlier run on the owner change alone (39166053719, against the values before it) moved railing and ramp values to Revit's railings (40) and ramps (5). It also left no `IfcSpace` value on a member, a drain or a sanitary terminal.

## 5. What is not shown

- **The groups that still differ on the as-built file:**
  - 9 wall sweeps that Revit writes as proxies, though they carry `IfcCovering`;
  - 2 stairs of the Generic Models category;
  - a door that is a curtain panel;
  - a plumbing fixture Revit writes as `IfcFlowTerminal`;
  - 74 spaces: 30 areas and 44 family instances (#763).
- **The permit file's type join** for the 20 appliances above, and its untyped columns, doors and light fixtures.
- **Which 15 property values** `psets_vs_ifc` counts on each file.

## 6. Reproduce

```bash
gh workflow run measure.yml -f ref=main -f base=none -f nordic=true -f probe=probe_re45_ifc_export_parameters -f probe_scorer=export_parameters_vs_ifc
```
