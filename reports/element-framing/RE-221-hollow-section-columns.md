# RE-221: A square hollow-section column's body from its family's parameters

**Date:** 2026-10-25
**Issues:** W6, the shape census of every body rvt-rs writes as its record box (`tools/re/box_bodies_vs_ifc.py`, #977). Follows RE-93 (a type's lengths as `f64 · ff×8 · i64` entries), RE-103 and RE-104 (a steel member's I section from its type), and RE-38 (family and type names).
**Artefacts:**

| file | sha256 | licence |
|---|---|---|
| `ARK_NordicLCA_Office_Concrete_As-Built.rvt` (Revit 2024) | `747120005b4b35c7cb66ec61158aab50ded95e052e172cb387b86cd15b31d906` | CC BY-SA 4.0, VTT (`research/bim4lca-measure.tsv`) |
| `ARK_NordicLCA_Office_Concrete_As-Built_Revit.ifc` (Revit's IFC4 export) | `b2f530eba8b4424c4542b98e9a2c7bbcc726367f21b2740f3074e4a6ce917382` | CC BY-SA 4.0 |
| `ARK_NordicLCA_Office_Concrete_BuildingPermit.rvt` (Revit 2024) | `1c12c2058128315edbd65a8bf97f60da971aee8625f21233b8477d9c6165c1d1` | CC BY-SA 4.0 |
| `ARK_NordicLCA_Office_Concrete_BuildingPermit_Revit.ifc` (Revit's IFC4 export) | `bc3bd3366a3c413ce74840300a590f59102b740118ef37898e906253b6b7c08d` | CC BY-SA 4.0 |

Measure fetches all of them and stores none.
**Probe:** `examples/probe_hss_sections.rs`. **Scorers:** `tools/re/body_forms_list.py` (Revit's extrusion profiles, point by point), `tools/re/column_bodies_vs_ifc.py` (new in Measure).
**Runs:**
- [40184775856](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40184775856): the probe's first pass: the three hollow-section types, their columns, and Revit's bodies for them;
- [40185531601](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40185531601): every length entry of the three types;
- [40186485900](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40186485900): the parameter elements those entries are stored under, and Revit's profile coordinates;
- [40187355859](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40187355859): this change against `main` (3a3b50ad).

**Status:** positive. Every hollow-section column on both office files is now Revit's body: 79 of 79 as built, 55 of 55 at building permit.

## 1. What was missing

The W6 census puts each body rvt-rs writes as its record box against Revit's body for the same element. On the office as built, 79 of its 278 such columns fill well under half of their box in Revit's export. All 79 are of one family, `Rectangular and Square Hollow Sections-Column`: 37 of type PI-4, 24 of PI-2 and 18 of PI-5. On the building-permit file the same family's 55 columns (37 PI-4, 18 PI-5) do the same.

Revit writes each as an `IfcExtrudedAreaSolid` of an `IfcArbitraryProfileDefWithVoids` (`tools/re/body_forms_list.py`). The outer curve and the one inner curve are each an `IfcIndexedPolyCurve` of 12 points: four lines and four three-point arcs. The outer curve runs counter-clockwise and the inner one clockwise. rvt-rs drew a solid square.

## 2. The family's parameters

The types hold no BuiltInParameter for a section. Their value blocks hold `f64 · ff×8 · i64` length entries (RE-93) under four parameters whose ids are ElementIds of the file, 2218981 to 2218984. Each is a `ParamElemFamily` element whose `ParamDefValue.m_caption` names it: `kr`, `t`, `Ht` and `b`. Run 40186485900 prints all four records. Every partition that holds a type's value holds the same one.

| type | columns as built | `b` | `Ht` | `t` | `kr` |
|---|---:|---:|---:|---:|---:|
| PI-4 (2219001) | 37 | 0.328084 ft (100 mm) | 0.328084 | 0.016404 (5 mm) | 0.032808 (10 mm) |
| PI-5 (3141950) | 18 | 0.656168 (200 mm) | 0.656168 | 0.016404 | 0.032808 |
| PI-2 (3513104) | 24 | 0.590551 (180 mm) | 0.590551 | 0.026247 (8 mm) | 0.052493 (16 mm) |

Revit's profile is exactly these values, to the six decimals its export writes:
- The outside is `b` by `Ht`, centred. Each corner is a quarter arc of radius `kr`; for PI-4 the arcs' centres are at ±0.131234 ft, which is `b/2 − kr`.
- The inside is `t` in from the outside. Its corners are quarter arcs about the same centres, of radius `kr − t`; for PI-4 the inside half-width is 0.147638 ft, which is `b/2 − t`.
- Each arc's middle point lies on the corner's diagonal, at `centre + radius/√2` along each axis.

The extrusion's depth is the column's height, and its placement is the column's own. The office export is in shared coordinates turned about 40° (axes 0.7623, 0.6472); in the file's own coordinates every one of these columns stands along the model's axes.

## 3. What rvt-rs does

`partition_type_parameters::type_hollow_sections` reads, for each column type, the length entries its value blocks hold under every parameter, keeping a value only where every partition agrees. It names the parameters by their `ParamElemFamily` records' captions, in one pass of `extract_graphs` over the ids the entries name. A type is a hollow section where it holds all four of `b`, `Ht`, `t` and `kr`, and they make a wall with rounded corners:
- `t` is positive and under half of the narrower side;
- `kr` is over `t` and at most half of the narrower side.

`partition_schema_mvp::attach_hollow_sections` gives that section to each structural or architectural column of such a type that has no I section (RE-104), as the fields `m_hollow_section_width`, `_height`, `_wall` and `_radius`.

The exporter draws such a column as its section where the section is square (`b` and `Ht` within 0.001 ft) and the record box's width and depth are `b` and `Ht` within 0.001 ft. The profile is the outside and inside described in §2, extruded through the box's height. The body source is `family_type_hollow_section`. Requiring the box to be the section's outside keeps out a column turned off the model's axes, whose box is wider than its section; such a column keeps its box.

The IFC writer gains a profile kind for it. `ProfileDef::IndexedPolyCurveWithVoids` holds an outer `IndexedCurve` and its voids, each a list of points and line and arc segments. It is written as `IfcArbitraryProfileDefWithVoids(.AREA., $, outer, (inner…))` over `IfcIndexedPolyCurve`s, in Revit's form, so IfcOpenShell meshes both files alike. Both the new variant and `IndexedCurve` are additions; `IndexedCurve` is `#[non_exhaustive]`.

## 4. Measured

`column_bodies_vs_ifc` meshes each column whose rvt-rs `BodySource` is a section, and Revit's body for the same `Tag`, with IfcOpenShell. It compares the volume and how far each of Revit's vertices lies from rvt-rs's nearest vertex and from rvt-rs's surface. Run [40187355859](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40187355859), this change against `main`:

| model | drawn as a hollow section | in Revit's export | vertices within 0.001 ft and volume within 0.0001 | every Revit vertex on rvt-rs's surface |
|---|---:|---:|---:|---:|
| BIM4LCA office as built | 79 | 79 | 79 | 79 |
| BIM4LCA office at building permit | 55 | 55 | 55 | 55 |

The W6 census of record-box bodies falls from 11,296 to 11,217 as built and from 11,388 to 11,333 at building permit. As built, every column left in the census is a box in Revit's export too. At building permit 269 of the 290 left are; 18 concrete PI-1 columns fill at least 90% of theirs, and two concrete PI-2 columns less. No other scorer's output changes. Core Interior, Einhoven, the four RE1 models and the 23 R2025 models export byte-identical IFC: none has a column of this family.

## 5. What it does not cover

- A rectangular hollow section (`b` ≠ `Ht`). Which side runs along the column's own X axis is not read, and none is on a measured file, so such a column keeps its box.
- A column turned off the model's axes.
- A hollow section whose family names its parameters otherwise, and round hollow sections. The names are this family's; another family keeps its box until a file with one is measured.
