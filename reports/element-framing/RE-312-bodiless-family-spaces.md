# RE-312 — Revit's export leaves out a family instance that has no 3D body, so 92 of 94 parking bays are not spaces

**Date:** 2026-11-03
**Issues:** #1262 (91 "Parking Space" family instances written as `IfcSpace` on the BIM4LCA terrain office, where Revit's export has 9 `IfcSpace`); builds on RE-45, RE-190 and RE-204
**Artefacts:** the BIM4LCA terrain office at building permit, in concrete and in timber (Revit 2024, Revit 24.0.5's IFC4 exports, CC BY-SA 4.0, VTT; `research/bim4lca-measure.tsv`, read by byte range for a Measure run with `-f nordic=true`, never stored), the two office files of the earlier runs, the reference models and the R2025 models
**Probe and scorer:** `probes/examples/probe_re312_parking_spaces.rs`, `tools/re/parking_spaces_vs_ifc.py`
**Runs:** Measure [41652870443](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41652870443) (the probe at eef185da, terrain pair), [41655209746](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41655209746) (the probe at b9bc0dbc, all four office files), and [41656473590](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41656473590) (the fix at 6eae7dd2, base `main`, `nordic`, `r2025`, `holdout`)
**Result:** positive. See section 5 for what changed.

## 1. The question

#1262 reports that `rvt` 0.12.0 writes 98 `IfcSpace` on the terrain office, 91 of them family instances named `Parking Space:PP:<id>` and `Parking Space:AP:<id>`, where Revit's export has 9 `IfcSpace` and no parking bays, and expects a proxy.

Revit's export holds 9 `IfcSpace`: 7 rooms and two `Parking Space:AP:<id>` family instances, as `IFCSPACE(…, .PARKING.)` typed by an `IfcSpaceType` named `AP`. So Revit does export a parking-space instance as `IfcSpace`, as the reporter's file asks it to ("Export to IFC As" `IfcSpace`, predefined type `PARKING`); it writes 2 of the 94 instances. The other 92 are not in its export under any class (no entity carries their ElementId as a `Tag` or a name suffix). Both terrain files, concrete and timber, give the same 94 and 2.

## 2. What does not tell the two groups apart

`probe_re312_parking_spaces` prints, for every element whose own "Export to IFC As" is `IfcSpace`, its partition record, every scalar of its native record and of its type symbol's record, and the model forms of its family. `parking_spaces_vs_ifc` sets each field's values against whether Revit's export holds the element. On the terrain office (94 instances, 2 written):

| candidate | the 2 written | the 92 left out |
|---|---|---|
| "Export to IFC As", "IFC Predefined Type" (the element's, RE-45) | `IfcSpace`, `PARKING` | `IfcSpace`, `PARKING` |
| its type's "Export Type to IFC As" | none | none |
| phase created, demolished | 86961, none | 86961, none |
| design option (record `+0x2a`, native base) | none | none |
| container, placement kind | unset, the same | unset, the same |
| host (`m_hostId`) | 1353095 | 1353095 for 24, 1366580 for 68 |
| `m_invisible`, `m_famElemVisibility`, `m_roomBounding`, workplane flags | the same | the same |
| the family and its type symbol's structure | one family (1371167), the same fields | the same |

The two groups differ in their type (`AP`, 5500 by 2500 mm, and `PP`, 2000 by 500 mm), in orientation (`m_flippedX`, `m_RefDir`: a correlate of the two instances, not a cause) and in one parameter entry, below. Revit's exporter also reads the integer "Export to IFC" of the element and its type (`ElementFilteringUtil.GetExportElementState` in revit-ifc, CLEANROOM.md source 5); no instance or type object of either type carries such an entry (the data-object bytes of `AP` and `PP` differ only in ids, dimensions and orientation, and this entry would appear in that difference).

## 3. What does

Revit's `FamilyInstanceExporter.ExportFamilyInstanceElement` (revit-ifc) builds the instance's 3D body and returns with nothing written when it has none and the element is not a door or a window ("We will allow a door or window to be exported without any geometry … Anything else doesn't really make sense").

The "Parking Space" family document, embedded in the project, holds one model form, an extrusion (the parking stripe). Its `Visible` is bound to the family parameter "Stripe Y/N" (`FamilyParametrizedElemParamsCell`, the mechanism RE-206 reads for nested families):

- both types hold "Stripe Y/N" as 1;
- the 2 car bays hold no value of their own, so the stripe is drawn, and Revit's body is that one stripe (an extruded 5500 by 101.6 mm profile, 3.175 mm deep, material "Parking Stripe");
- all 92 bicycle bays hold "Stripe Y/N" as 0 in their own serialised data (a parameter entry the car bays lack), so no form of their family is drawn.

The same holds on the office files: of the 78 family instances whose value is `IfcSpace` on the as-built file, the 42 "Turning Free Space" instances (all written by Revit) have a family (3439998) with one drawn form; the 34 that Revit leaves out are of two types of a family (3509968) whose one form is a void.

## 4. The rule

`native_family_forms::instances_without_body` reads, for the instances given, the symbol and the parameter values of the instance and its type, the captions of those parameters, and the model forms of the symbol's family document (`family_solids`). A form is drawn unless it is a void or its `Visible` is 0, where `Visible` is the value of the parameter it is bound to (the instance's, else the type's, matched to the family document's by caption) or else its stored value. An instance with no drawn form has no body.

`partition_schema_mvp::without_bodiless_family_spaces` drops those of them whose "Export to IFC As" is `IfcSpace`. It leaves an instance in, as before, where its family's document is not found, holds no form, nests a family instance, or has a form or an instance that does not decode, and for any Revit release outside 2023 to 2027. Other categories are not touched.

## 5. Measured

The probe's prediction, before the rule (run 41655209746, 107 `IfcSpace`-valued family instances per office file, 94 on the terrain files; rooms and areas, which Revit names by number, are counted apart):

| file | drawn form, Revit wrote | drawn form, Revit left out | no drawn form, Revit wrote | no drawn form, Revit left out | not decided |
|---|---:|---:|---:|---:|---:|
| terrain, concrete and timber (each) | 2 | 0 | 0 | 92 | 0 |
| office as built and at permit (each) | 42 | 0 | 0 | 34 | 2 written (the two VOLUME masses, whose family documents the probe did not read), and 29 areas (class `RoomElem`, not family instances) |

So the prediction is exact on 4 files: no instance Revit writes is without a drawn form, and every one it leaves out that was decided has none. The office's 34 belong to a family whose one form is a void.

With the rule (run 41656473590, head 6eae7dd2 against `main`, `nordic`, `r2025`, `holdout`, every scorer):

- The exports of 45 of the 47 models measured are byte-identical to `main`'s: the six reference models (Core Interior, Einhoven, RE1's four), the two concrete office files, the 24 R2025 models and the 13 held-out models (their counts only).
- The two that change are the terrain files, and each changes the same way:

| `class_sets_vs_ifc` / `global_ids_vs_ifc` / `psets_vs_ifc` / `room_names_vs_ifc` | before | after |
|---|---:|---:|
| `IfcSpace` rvt-rs writes that Revit's export does not | 89 | 0 |
| `IfcSpace` rvt-rs writes that equal one of Revit's | 9 of 98 | 9 of 9 |
| `IfcSpace` in Revit's export that rvt-rs has under the same Tag or name | 9 of 9 | 9 of 9 |
| property values rvt-rs writes that Revit does not | 180 | 2 |
| property values of Revit's that rvt-rs writes equal / different / missing | 82 / 0 / 29 | 82 / 0 / 29 |
| type objects | 16 | 15 |

- No element Revit exports is dropped: every `class_sets_vs_ifc` row that was `same` is still `same`.
- `main` wrote 91 of the 94 instances (the reporter's count, 2 car bays and 89 bicycle bays); the rule removes the 89 and leaves the 2 car bays. The other 3 bicycle bays were already left out, for a reason not examined here.
- Schema validation: one finding (`IfcRelAssociatesMaterial.AllowedElements`) on each side, unchanged.

## 6. What is not shown

- **Why the bicycle bays carry 0.** Their "Stripe Y/N" is 0 in their data. Whether the model's authors hid the stripe on purpose is not something the file says.
- **That "no drawn form" is Revit's whole test.** Revit's body also needs geometry that survives its tessellation and the export view's visibility (`ElementFilteringUtil.IsElementVisible`: hidden elements, categories and filters of the export view). Two instances of one family can differ in either. The rule covers what the file decides: forms, voids, and the parameter values that show them. An element Revit's export view hides is not decided.
- **Other categories.** Revit's rule is for every family instance that is not a door or a window. This change applies it to the family instances written as `IfcSpace` through their "Export to IFC As", the measured case, and to no other. A family whose document nests a family instance, holds no form, or has a form that does not decode (imported or direct geometry, model lines, other form classes) is left as it was. Applying the test to lighting fixtures, furniture or generic models needs its own measurement against Revit's export.
- **Revit before 2023.** Family documents are read for 2023 to 2027 only.
- **The terrain office's other gap.** Revit's export holds 104 `IfcGeographicElement` that rvt-rs does not write: 83 "U2BicyclePost", 8 "Park Bench", 7 "Pavement Stripe", 3 Toposolids and 3 Sub-Divisions. `class_sets_vs_ifc` pairs 31 of Revit's 141 elements with rvt-rs's. That is a separate finding, not touched here.

## 7. Reproduce

```bash
gh workflow run measure.yml --ref main -f ref=<sha> -f base=none -f nordic=true -f models="nordic-office-*" -f scorers=none -f probe=probe_re312_parking_spaces -f probe_scorer=parking_spaces_vs_ifc
```
