# RE-439: a project's display units are its `UnitsElem`'s format map

**Date:** 2026-11-11
**Issues:** follows #403 (IFC-40, the unit assignment) and IFC-39 (`ForgeUnit`); the warning the web viewer shows on Core Interior
**Artefacts:** `2024_Core_Interior.rvt` (MIT, `magnetar-io/revit-test-datasets`, pinned in `research/witness-registry.json`); Autodesk's sample projects of 2019 to 2027 (`research/autodesk-sample-projects.tsv`) and sample families of 2016 to 2026 (`phi-ag/rvt`); Snowdon Towers 2024 and 2025; and, local only, the RE1 models, the R2025 models, Golden Nugget, Autodesk's Japanese samples and Revit_IFC5_Einhoven: 131 files in all
**Probe:** `probes/examples/probe_re439_project_units.rs`
**Status:** positive. The project's display unit of every spec is read on all 103 files of Revit 2021 to 2027 the native walk reads, and on the 8 sample families of 2021 to 2027 it does not, from their string records. No IFC changes.

## 1. The question

Loading Core Interior in the viewer warns: "Unmapped Revit unit identifiers were preserved in diagnostics: autodesk.unit.unit:amperes-1.0.0, autodesk.unit.unit:britishThermalUnits-1.0.1, …". The exporter took every `autodesk.unit.*` string of the partition, kept for each of the six IFC unit types (length, area, volume, plane angle, mass, time) the identifier seen most often, and listed every other one as unmapped. On Core Interior that gave 130 unmapped identifiers, every unit of an electrical, HVAC, energy, piping or structural spec, and the six it kept were `meters`, `squareInches`, `cubicInches`, `degrees`, `poundsMass` and `seconds`. Core Interior is an imperial project: its length is feet and fractional inches. Where does Revit keep the units the Project Units dialog sets?

## 2. The record

The schema (`Formats/Latest`) has a class `UnitsElem` with one field, `m_units`, an `AUnits`. From Revit 2021 `AUnits` holds `m_formatOptionsMap`, a `std::map` from a `ForgeTypeId` (the spec, `m_typeId` a string such as `autodesk.spec.aec:length-1.0.0`) to a `FormatOptions`, whose fields are `m_symbolTypeId` and `m_unitTypeId` (both `ForgeTypeId`), `m_accuracy` (f64), `m_roundingMethod`, the suppress, grouping and plus-prefix flags, `m_bUseDefault`, `m_universalUnitSystem`, `m_decimalSymbol`, `m_digitGroupingSymbol` and `m_digitGroupingAmount`. `native_document` decodes the record's graph with no new reader: the map arrives as a list of `first`/`second` pairs.

Core Interior's one `UnitsElem` (element 15) lists 136 specs. The six the IFC assignment takes:

| spec | display unit | accuracy |
|---|---|---:|
| `autodesk.spec.aec:length` | `feetFractionalInches-1.0.0` | 1/384 ft (1/32") |
| `autodesk.spec.aec:area` | `squareFeet-1.0.1` | 0.01 |
| `autodesk.spec.aec:volume` | `cubicFeet-1.0.1` | 0.01 |
| `autodesk.spec.aec:angle` | `degrees-1.0.1` | 0.01 |
| `autodesk.spec.aec.structural:mass` | `poundsMass-1.0.1` | 0.01 |
| `autodesk.spec.aec:time` | `seconds-1.0.0` | 0.1 |

Mass is the structural spec: the map has a piping `mass` too, with the same unit on every file probed. The count read every copy of the map the partition keeps (section 5) and every other unit string, and units other specs use outnumber the six's own: Core Interior's string records hold `squareInches` 78 times against `squareFeet` 26 (three specs of its map display square inches: HVAC cross section, reinforcement area and section area; two cubic inches), `cubicInches` 52 times against `cubicFeet` 26, and `meters` 4,675 times, while `feetFractionalInches`, 135 times, was not parsed by `ForgeUnit` at all.

## 3. Which record is the project's

Snowdon Towers Architectural (2024 and 2025) holds three `UnitsElem` records. Two have `m_famId` set (1982107 and 1982387): they belong to families whose documents the project keeps, as their own `AllProjectPhases` do (RE-184), and their units differ from the project's (power in volt-amperes, area with accuracy 1). The project's has `m_famId` unset. A family file is the other way round: its one `UnitsElem` names the family the file is (the R2025 `Beam.rfa`, `m_famId` set, the only record). The reader takes the record with `m_famId` unset, and where there is none, a file's only record; two project records that disagree, or several family records and no project one, are an error. No file probed has either.

## 4. Before Revit 2021

Revit 2015 to 2020 save `AUnits.m_formatOptionsArr`: an array of 127 (2015), 130 (2019) or 132 (2020) pointers to a `FormatOptions` with an integer `m_displayUnits` and `m_unitSymbol` and no spec. The position stands for the spec. Autodesk's `rac_basic` and `rst_basic` of 2020, and `rac_advanced` and `rst_basic` of 2019, can be set against their 2021 saves: all four are metric, position 0 is the only position whose accuracy and symbol fit length in every pair, and its integer is 2 where the 2021 save says `millimeters`. Area, volume and angle at positions 1 to 3 (integers 12, 14 and 15) fit, but so do 35 to 42 other positions each, and mass and time are not placed at all. An imperial file of these releases (`Arch Link Model`, integers 4, 11, 13, 15 at positions 0 to 3) has no later save to compare. So the array is recognised and not read: such a file gives no units, as before, with the export's existing warning "No Revit display units were recovered".

## 5. When the walk does not read the file

The native walk stops before any record on Autodesk's sample families of 2016 to 2027 ("invalid increment episode range"). Their partitions still spell the map out: the string scan (`partition_string_records`) sees each entry's spec followed by its unit (it takes the symbol for the unit's record header, so the symbol is not seen). A run of string records alternating spec and unit, the specs in ascending order, with nothing between them, is a map (`string_record_maps`). A run of one pair is not taken.

Runs cannot replace the walk on a project: the partition keeps older copies of the map. Core Interior holds 47 runs of 136 specs, and only 3 equal its `UnitsElem` map. Across the 103 files the walk reads, 101 have at least one run equal to it, and Autodesk's Japanese structural sample (2019 folder and 2025) has none: its runs are 61, 17, 33, 9 and 7 entries long. So `resolve` takes the runs only where the walk gives no units and every run agrees. Each sample family of 2021 to 2027 holds exactly one run (133 to 142 specs), which gives millimetres, square and cubic metres, degrees, kilograms and seconds. The 2016 to 2020 families hold none and give no units. A file whose runs disagree gives an error, which the export reports as a warning; no file probed does once the walk reads `Beam.rfa`.

## 6. Across the corpus

`probe_re439_project_units` on 131 files:

| what is read | files | units of the six |
|---|---:|---|
| the walk, a format map of 133 to 150 specs | 103 | metric: 70 millimetres, 2 metres (Golden Nugget Architektur 2019 and 2025); imperial: 31 feet and fractional inches |
| the string records' one map | 8 | the sample families of 2021 to 2027, metric |
| the array of 2015 to 2020, not read | 11 | none |
| the walk fails and there is no map | 9 | none (families and templates of 2015 to 2020, one 2009 file) |

Every metric file gives square and cubic metres, degrees, kilograms and seconds with its length; every imperial one square and cubic feet, degrees, pounds (mass) and seconds. The read takes at most 91 ms (Snowdon Towers Architectural, `--profile ci`) and is done once per export (`RevitFile::project_units`).

## 7. What changes

- `native_project_units::read` reads the format map, `resolve` adds the string records' map, and the export takes each IFC unit type's display unit from its spec (`IFC_UNIT_SPECS`), not by count.
- The diagnostics' `recovered_unit_identifiers` are those six. `unknown_unit_identifiers` lists only those of the six that no IFC unit is known for, and the warning names them; the units of the other specs are not unmapped, and are listed in a new `project_display_units`, by spec.
- `ForgeUnit` reads Revit's identifiers `feetFractionalInches` (feet), `fractionalInches` (inches: it was read as feet), `metersCentimeters` (metres), `degreesMinutes` (degrees), `usSurveyFeet`, `tonnes`, `usTonnesMass` and `milliseconds`, and labels a kilogram `KILO GRAM` and a gram `GRAM` (they were `GRAM` and `MILLI GRAM`; a pound's factor is now to the gram it names).
- The IFC does not change: the writer writes every value in SI units and declares those (#403), whatever the project's display units are. Section 8 has the comparison.

## 8. Exports

`rvt-ifc` of `main` (`7cbbe5b3`) and of this change, `--profile ci`, on ten files. The IFC files are identical line for line but for `FILE_NAME` and `IFCOWNERHISTORY`'s timestamps, all ten. The diagnostics differ in the unit fields and `confidence.warning_count` alone: the "Unmapped Revit unit identifiers" warning goes, and no other warning comes.

| file | `main`: the six | unmapped | this change: the six | unmapped | specs | warnings |
|---|---|---:|---|---:|---:|---|
| Core Interior 2024 | metres, in², in³, degrees, lb, s | 130 | ft-frac-in, ft², ft³, degrees, lb, s | 0 | 136 | 2 → 1 |
| `rac_basic` 2024 | metres, in², in³, degrees, lb, s | 135 | mm, m², m³, degrees, kg, s | 0 | 136 | 4 → 3 |
| `rac_basic` 2027 | metres, in², in³, degrees, lb, s | 146 | mm, m², m³, degrees, kg, s | 0 | 142 | 6 → 5 |
| `rac_basic` 2020 | none | 0 | none | 0 | 0 | 5 → 5 |
| Snowdon Towers Architectural 2024 | metres, in², in³, degrees, lb, s | 144 | ft-frac-in, ft², ft³, degrees, lb, s | 0 | 136 | 5 → 4 |
| RE1 Architecture | metres, in², in³, degrees, lb, s | 132 | mm, m², m³, degrees, kg, s | 0 | 136 | 5 → 4 |
| Golden Nugget Architektur 2025 | metres, cm², cm³, degrees, kg, s | 72 | metres, m², m³, degrees, kg, s | 0 | 136 | 5 → 4 |
| sample family 2024 | metres, cm², cm³, degrees, kg, s | 69 | mm, m², m³, degrees, kg, s | 0 | 136 | 4 → 3 |
| sample family 2019 | none | 0 | none | 0 | 0 | 5 → 5 |
| R2025 `Beam.rfa` | metres, in², in³, degrees, lb, s | 81 | ft-frac-in, ft², ft³, degrees, lb, s | 0 | 136 | 3 → 2 |

The count was wrong on every file it read, the metric ones included: `rac_basic` and RE1 Architecture, metric projects, came out in square inches and pounds.

The `export_diagnostics_sidecar_reports_default_ifc_readiness` test (`tests/walker_to_ifc_integration.rs`) asserted the count's `meters` for the 2024 sample family; it now asserts `millimeters`, the family's own length unit.
