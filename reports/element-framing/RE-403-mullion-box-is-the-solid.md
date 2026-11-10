# RE-403: a curtain mullion's box gives its quantities only where Revit's solid is that box, and an overlap counts only where the box of the mullion over it is its solid

**Date:** 2026-11-10
**Issues:** #1046 (base quantities Revit computes from its joined geometry, W7), #1016 (quantities); follows RE-243, RE-268, RE-387, RE-400 and RE-402 section 8
**Artefacts:** the BIM4LCA housing (concrete as built, timber at permit) and timber office (as built and at permit), and the concrete office (Revit 2024, Revit 24.0.5's IFC4 exports, CC BY-SA 4.0, as Measure fetches them); revit-ifc is not read again here
**Runs:** Measure [42898163164](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42898163164), [42900917460](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42900917460), [42901978537](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42901978537), [42902864917](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42902864917) and [42903713917](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42903713917) (scratch scorers `mullion_wrong_detail.py` and `mullion_profile_probe.py`, commits `7b5bdc87e6ae` and `f2202acccac3`, saved meshes off; sections 2 and 3), [42909452814](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42909452814) (`probe_1046_mullion_joins`, the types of the timber office at permit) and [42910979704](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42910979704) (its export with `--no-saved-meshes`, kept), [42904014676](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42904014676) (the first form of this change, section 5), and the unfiltered run of the change as it stands (section 5)
**Status:** In progress: sections 1 to 4 are measured on the files named; the unfiltered Measure of the change as it stands is running and section 5 is written when it reads.

## 1. What RE-402 left open

RE-402 measured the shipped mullion rules on the BIM4LCA housing and the timber office, which no run had held when RE-400 merged. On the housing 48, 40, 48 and 48 mullions of one type have a wrong `NetVolume` and a wrong `NetSurfaceArea`, 0.005429 m2 above Revit's whatever their ends. On the timber office at permit RE-400 adds 186 `NetSurfaceArea` values and 24 are not Revit's (22 on mullions with another lying over an end, 2 on mullions that meet a short one). The box formula of RE-243 and RE-268 (`NetVolume` the box's two sides times its length; `NetSurfaceArea` its perimeter times its length plus the section's area for each end Revit's solid keeps) is right wherever the record box is Revit's solid. The two causes below are the places it is not.

## 2. The box is not the solid

`mullion_profile_probe.py` lists, per mullion, Revit's body, its mesh and its quantities against rvt-rs's box and its values.

**The housing's 48.** Revit draws the 156 mullions of `Liukulasituksen yläprofiili` as an extrusion on 108 and as an `IfcPolygonalFaceSet` (8 vertices, 12 triangles) on the 48 that are wrong. They are the prism of a curtain-wall corner with a 45 degree mitre across the 50 mm side. For tag 1504271 (concrete as built):

| | Revit | rvt-rs's box formula |
|---|---|---|
| `Length` | 680.507 mm | 680.507 mm |
| solid | 50 x 100 mm section, mean length 655.507 mm (the long edge less half the 50 mm) | 50 x 100 x 680.507 mm |
| `NetVolume` | 0.0032775365 m3 = 0.05 x 0.1 x 0.655507 | 0.003403 m3 |
| `NetSurfaceArea` | 0.2037232603 m2 = 0.3 x 0.655507 + root 2 x 0.005 | 0.209152 m2 = 0.3 x 0.680507 + 0.005 |

Revit's value is the lateral area at the mean length plus the mitre face, which is root 2 times the section, where the flat end is hidden by its partner; the sums agree to 1e-15. The box's error is the perimeter times 25 mm less (root 2 - 1) times the section, 0.0075 - 0.002071 = 0.005429 m2, for one flat and one mitred end whichever of them its neighbours hide, which is why the 48 differ by one constant. The saved mesh of each is a closed prism of flat faces whose volume is Revit's `NetVolume` and 2.6 to 3.7% below the box's (14 listed: 0.9633 to 0.9740).

**The timber office at permit.** Run 42903713917 gives each of the 49 mullions with a wrong `NetSurfaceArea` (the 25 RE-268 already got wrong, 22 overlapped ones and 2 that meet a short mullion, run 42898163164) the volume of Revit's solid against rvt-rs's box. On 28 the box is not the solid: a record box 1.19 times smaller than Revit's extruded solid (7 mullions of `50 x 150 mm`: 0.02100 against 0.02502 m3), 2.85 times smaller (4 of `120 x 30 mm puu`: 0.0053125 against 0.01512 m3, a 50 x 50 mm box for a 60 x 120 mm solid), 12 uprights of one curtain wall 0.875 times (0.0048775 against 0.0055750 m3) and 5 more between 0.976 and 0.991. They are 20 of RE-268's 25, 6 of the 22 overlapped and the 2 that meet a short one. On the other 21 the box is the solid (ratio 1.0000): 16 are the overlaps of section 3, and 5 are RE-268's own wrong values, two 100 mm pieces among them, which neither guard touches.

## 3. The box is the solid and a mullion whose box is not its solid lies over an end

RE-400 reads an overlapped mullion on its own: the mullion over its end counts for nothing, because its box says where its solid is. At permit 126 mullions are read that way and 22 are wrong; as built all 118 are right. Run 42898163164 labelled what lies over each end by the types' names in rvt-rs's export, which put the 22 with another section or another type over them (20 and 2) against 0 wrong where the mullion over it had the same name and section (140 right). The names mislead: the probe of run 42909452814 reads each mullion's type as the `m_masterSymbolId` of its native record, and every mullion that lies over an end in a wrong case is of the type of the mullion it lies over (3624382 for 20 of the 22, 3639808 for 2). The 16 mullions of 3624382 whose box is odd (below) have no name in rvt-rs's export, which the labels of run 42898163164 read as another type.

What differs is the box. The boxes of the file's 7,873 mullions, read from its export with `--no-saved-meshes` (run 42910979704), hold one section for almost every mullion of a type and another for a few:

| type | mullions | the type's usual section | boxes with another |
|---|---:|---|---:|
| 3639808 | 5,431 | 50 x 50 mm | 19, of 9 other sections (4 at 70.7 x 700 mm and several at 50 x 105 to 108 mm among them) |
| 3624382 (`120 x 30 mm puu`) | 792 | 60 x 120 mm | 16, all 50 x 50 mm |
| 8486 | 180 | 60 x 150 mm | 14, all 50 x 150 mm |
| 3682535 | 174 | 80 x 80 mm | 2, both 50 x 50 mm |
| 1907093, 3630838 | 1,292 and 4 | 50 x 150 mm, 120 x 120 mm | 0 |

A type has one section, so a box with another is not the mullion's solid: section 2 reads 4 of the 50 x 50 mm boxes of 3624382 against Revit's 60 x 120 mm solids, and 7 of the 50 x 150 mm boxes of 8486 against solids 1.19 times larger. Of the 170 overlapped mullions at permit (22 wrong, 148 right), 16 of the 22 that are wrong have a mullion with such a box lying over one of their ends, and 1 of the 148 that are right does. Where every mullion over an end has its type's usual section the overlap is Revit's on 147 of 153: the 6 that are wrong are 4 mullions whose own box is odd (a box that is not their solid, which section 2's guard covers) and 2 of type 3639808 that no feature tells from the 147.

## 4. What is written

- `saved_meshes::follow_mesh_over_box`, in the pass that draws a body from the mesh Revit saved: a curtain mullion whose `NetVolume` the box gave and whose closed flat mesh has another volume (by more than 0.1%, as a fraction of the mesh's) is not an extrusion of its box. It takes the mesh's volume, which is Revit's (RE-387), and loses its `NetSurfaceArea`, which assumed the box. A mullion with no complete mesh, and an export made with `--no-saved-meshes`, keep the box's values as before.
- `partition_schema_mvp::usual_sections` and `has_unusual_section`: the section more than half of the boxes of each type have, by the `m_masterSymbolId` of the native record (read with its host in `native_mullion_hosts`), and the boxes that have another. `kept_ends` leaves an overlapped mullion out, as RE-268 left every one out, where a mullion with such a box lies over one of its ends, and it stays a reason to leave its neighbours out. `tools/re/mullion_ends_vs_boxes.py` mirrors it by the members' `ObjectType`.

## 5. Measured

The first form of this change (42904014676, `c4ee0f30`) left the overlapped mullions out where a mullion of another type lay over an end, which on the timber office at permit changed nothing: the rule had no case. It moved 14 exports of 126 (the four large housing files, the timber office at permit and nine of Autodesk's samples) and left 112 byte-identical; the mesh guard took every `NetVolume` that differed to equal on all five files (233 values: 48, 40, 48, 48 and 49) and every `NetSurfaceArea` that differed on the four housing files to missing (48, 40, 48 and 48), and the timber office's `NetSurfaceArea` from 49 different to 21. The unfiltered run of the change as it stands against `main` is dispatched from `main`, whose workflow holds the housing and the timber office; its readings go here when it finishes.

## 6. Not shown

- **The mitred mullions' own `NetSurfaceArea`.** It is Revit's mesh area less the end faces its neighbours hide, with a mitre face counting root 2 times the section; reading it needs the end faces from the mesh and which of them are hidden at a mitre, and the 48 stay out until it is read (RE-404).
- **A cause for the values RE-268 already got wrong** where the box is the solid and no mullion with an odd box lies over an end (5 at permit, two 100 mm pieces among them), and for the 2 mullions of type 3639808 whose overlap reads wrong with a mullion of the usual section over them.
- **A section read from the type.** The usual section is the one more than half of a type's boxes have, in the one file; a type's own section would be read from its record (`SysMullionFamSym`'s profile), and is not.
- **Another project.** The rule rests on the timber office's one state and the concrete office; the mesh guard is the general one, and holds wherever a closed flat mesh is saved.
