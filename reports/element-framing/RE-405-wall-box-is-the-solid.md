# RE-405: a wall's box gives its quantities and its Height only where Revit's solid is that box, which the wall's saved mesh says

**Date:** 2026-11-10
**Issues:** #1636 (the second-project measurement), #1046 (base quantities Revit computes from its joined geometry, W7), #1016 (quantities); follows RE-266 and RE-293 (a wall's `Height`), RE-270 (its areas and volumes from its box), RE-388 (the same from its saved mesh) and RE-402 (the first reading of these rules on the BIM4LCA housing and timber office)
**Artefacts:** the BIM4LCA housing, concrete as built (Revit 2024; Revit 24.0.5's IFC4 export, CC BY-SA 4.0, as Measure fetches it), and rvt-rs's export of it by `main` at `237d6ef6`, which Measure [42906770416](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42906770416) kept (`keep_ifc`); the other BIM4LCA files, the Revit 2025 models and the samples through Measure's census
**Runs:** Measure 42906770416 for section 2; the unfiltered run of this change for section 4
**Status:** In progress: section 2 is read from the one pair that was kept; the unfiltered Measure of this change is running, and sections 4 and 5 are written when it reads.

## 1. What RE-402 left open

On the four large housing files RE-402 found the wall rules of RE-266, RE-270 and RE-293 writing values that Revit's export does not: the wall `Height` differs on some walls, `NetVolume` and the areas on others, and `GrossFootprintArea` is written on walls whose export Revit writes none for (RE-402 section 3). Each of those rules is measured equal on every wall of the BIM4LCA office, whose walls are mostly one layer, square at their ends and cut by nothing that Revit trims the wall's body for; the housing's are not. The counts before and after this change are in section 4.

## 2. What the walls that differ are

`qtos_vs_ifc` of the concrete as built lists 9 `NetVolume` and 3 `Height` values that differ. The pair of exports (rvt-rs's, kept by run 42906770416, and Revit's) says by `Tag` what each wall is.

**`Height`.** Revit's export writes a `Height` on 357 of the 369 walls both files hold: 198 are equal, 3 differ and 156 are missing in rvt-rs's. Of the 159 that are not equal, 136 are walls whose body in rvt-rs's export is an extrusion as high as Revit's `Height` (RE-266 writes a `Height` only where the wall's record names no step that joins or cuts it, and these records do), 19 are extrusions of another height, by 0.03 to 8.3 m in either direction, and 1 has no extrusion. The 3 that differ are three `VS-1` walls 1.65 m long (1695251, 1695217 and 1695563): rvt-rs's extrusion is 3.1 m (2.905 m for the third) from the wall's level, and Revit's body is a box of 8 vertices 1.0 m (0.805 m) high that starts 2.1 m above the level; both exports hold an `IfcOpeningElement` named like each wall, with no filling. The other 12 walls have no `Height` in Revit's export, nor `GrossSideArea`, nor `GrossFootprintArea`.

**`NetVolume` and the areas (the 9).** The 9 are walls whose box is not Revit's solid, of three kinds:

| kind | walls | rvt-rs's box | Revit's body |
|---|---|---|---|
| the box is longer than the wall | 1949400 | 0.728 x 0.088 m | 0.458 x 0.088 m |
| layered walls | 1346715, 1346580, 1346782 | one rectangle 0.54 m thick (on 1346715 its volume, 2.626 m3, is Revit's `GrossVolume`) | five extrusions, one per layer, whose profiles have 5 or 6 corners (Revit's `NetVolume` of 1346715 is 2.199 m3) |
| one layer, a profile of 5 to 8 corners | 1507225, 1881697, 1427008, 1881696, 1972381 | a rectangle | an arbitrary profile of 5, 5, 6, 5 and 8 corners (on 1507225 a footprint of 0.35 m2 against the rectangle's 0.38) |

**`GrossFootprintArea`.** Revit's export writes it on all 204 walls of one layer that carry a `Height`, and on none of the 153 walls of several layers, whose quantity sets hold one complex quantity per layer. rvt-rs writes it on 36 of those 153.

## 3. The change

`saved_meshes::add_wall_quantities`, in the pass that reads the mesh Revit saved for a wall (RE-388), reads two more things from a mesh that is whole (every face flat, one closed piece, one way: `wall_mesh_quantities`):

- **The box against the mesh.** Where the wall holds a `NetVolume` that is not the mesh's by more than 0.1% of it, the box is not Revit's solid, and the quantities taken from the box (`NetVolume`, `NetSideArea`, `GrossSideArea`, `GrossVolume`, `GrossFootprintArea`) are dropped; the mesh gives the ones RE-388 reads from it (`NetVolume`, and the areas of a wall of no more than four vertical faces). A wall whose box is its solid keeps its values.
- **`Height`.** The mesh's vertical span confirms the height of the wall's extrusion where it is within 0.0005 ft of it. The extrusion's height is then written as `Height` for a wall of no more than four vertical faces that holds none: the walls RE-388 gives areas, since Revit's export gives some walls of five or more vertical faces a volume and nothing else. Where the span is not the extrusion's height, a `Height` the wall holds is dropped. The span is not written itself: the saved corners are single precision, which moves it by up to 1e-4 ft (RE-388).

## 4. Measured

The unfiltered run of this change against `main` (every Measure model) is dispatched from `main`, whose workflow holds the housing and the timber office; its readings go here when it finishes.

## 5. Not shown

- **`GrossFootprintArea` on layered walls.** Revit omits it on 153 of 153 in the one file read; rvt-rs writes it on 36 of them. The rule that stops it is its own change.
- **Walls with a mesh that is not whole.** A mesh with a face that is not a plane, two pieces or an open edge gives no volume and no `Height`, and the wall keeps the values its box gave.
