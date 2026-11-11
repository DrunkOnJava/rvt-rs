# RE-418: a curtain mullion's `Length` and `Span` are the extent of its saved mesh along its own axis where its record box overruns the solid

**Date:** 2026-11-10
**Issues:** #1046 (curtain mullion quantities), #1636 (second-project measurement); follows RE-403 (the box is not the solid), RE-387 (the mesh's volume) and RE-268 (kept end faces)
**Artefacts:** the BIM4LCA office at building permit, in concrete and in timber, and the housing in its four large files (Revit 2024, Revit's IFC4 exports), as Measure fetches them. Never stored.
**Runs:** Measure [42943892070](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42943892070) (the scorer `tools/re/mullion_profiles_vs_ifc.py` at `e24fe0903abe134ec10719e458a6b7b94d7307ff`, on `main` at `9bf2ccf801e1bf9eec6967f4d3b46a35818af4ee`) for section 1
**Status:** pending the change's own measurement (section 3); the finding of sections 1 and 2 is measured.

## 1. What the scorer found

`tools/re/mullion_profiles_vs_ifc.py` pairs every curtain mullion (`IfcMember`, `PredefinedType` MULLION) of rvt-rs's export with Revit's by `Tag`, reads each side's `Length`, `Span`, `NetVolume` and `NetSurfaceArea` in their own units (Revit's export declares millimetres, square metres and cubic metres), and measures the body's mesh along the axis it is a prism along: the planes of its triangles, at most two of them (its ends) not parallel to the axis, and where several axes do that (a box has three), the one whose extent is nearest the side's `Length`.

- **Revit's `Length` is the extent of its own body.** On the 10,031 mullions of the office at permit it is the mesh's extent along its axis on every one but one (a mullion whose body the scorer does not read as a prism), and the same on the timber office's 7,872 and on the housing's 354 to 444 per file.
- **rvt-rs's body is Revit's on most.** Its mesh has the same extent and area as Revit's on 9,847 of the office's 10,031 mullions at permit and 7,681 of the timber office's 7,872; the others are cut differently or are not prisms the scorer reads.
- **The written `Length` is the record box's extent along the mullion, and it is Revit's on all but 71 of the timber office's 7,872 mullions** (19 of the concrete office's 10,031; none on the housing's). Of the 71, **49 are mullions whose box overruns the solid: rvt-rs's mesh is Revit's and its extent along its axis is Revit's `Length`**; 18 have no axis the scorer can read as theirs and 4 differ with the mesh's extent too (3 flat-ended, 1 mitred), and none of those 22 is a change this report makes.

| mullions of the timber office at permit | count |
|---|---:|
| `Length` and `Span` written equal to Revit's, and the mesh's extent equal | 7,773 + 28 |
| written different, the mesh's extent equal to Revit's | 49 |
| written different, the mesh's extent different too | 3 + 1 |
| written different, no axis read | 18 |

## 2. The rule

`saved_meshes::follow_mesh_over_box` (RE-403) already replaces a curtain mullion's box `NetVolume` with its mesh's where the two differ by more than 0.1%, and takes out the box's `NetSurfaceArea`. Where it does, the mesh is not an extrusion of the box, and the box's length is not the solid's either: the mullion's `Length` becomes the one extent of its mesh, along the axes it is a prism along (`mesh_volume::prism_extents`), that lies within 35% of the written `Length`; where there are two such, or none, the written `Length` stays. A `Span` follows its `Length`, as it does at the record. A difference under 1e-4 ft (the mesh's corners are single precision) leaves the written value alone.

## 3. Measured: the change

Pending: the unfiltered Measure of the change against `main`, which this section reports.

## 4. Sources

RE-243, RE-268, RE-387, RE-403. revit-ifc's `PropertyUtil.CreateBeamColumnBaseQuantities` as RE-353 read it (CLEANROOM.md source 5). No Autodesk source was read.
