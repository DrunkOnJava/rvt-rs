# RE-490: a blend rebuilt from its records

**Date:** 2026-11-17. **Follows:** RE-327 (family extrusions), RE-391 and RE-392 (revolutions).

## Question

`native_family_forms` lists `BlendElem` among the forms a family holds, but `native_form_geometry` rebuilds extrusions and revolutions only. Does a blend rebuild from its records as an extrusion does, and does the result match the mesh Revit saved for it?

## What was read

A `BlendElem` record holds, observed in `Column_Square_Tollerance.rfa` (Revit family, one blend) and `test__System__FamilyDocument__familyDocumentTests.rfa` (four):

- `m_sketchIds`: two element ids, the `VarSketch` of each profile.
- `m_cutting`, as an extrusion's.
- A `ParamValueSetDouble` with `m_paramId` -1001804 and -1001805, the offset of each profile along its sketch plane's normal (feet). The extrusion's pair is -1001800 and -1001801.
- Each sketch's curves are the `CurveElem` records whose `SketchMembership.m_groupId` names it, and its plane the `SketchPlane` they name, as for an extrusion. The plane's `Trf` (`m_3x3`, `m_or`) and `m_flipZ` give the frame, read as `Convention::COLUMNS`.

## Measured

Five blends, two families, all with a complete saved mesh (volume in cubic millimetres, converted from feet).

| Family | Blend | Vertices | Volume rebuilt | Saved mesh | Relative difference | Bounding box |
|---|---|---|---|---|---|---|
| Column_Square_Tollerance | 3926 | 4 + 4 | 26,984,032,053.3 | 26,984,032,053.3 | 1.2e-16 | exact |
| familyDocumentTests | 273498 | 4 + 4 | 1,209,459.9 | 1,209,459.9 | 4.9e-16 | exact |
| familyDocumentTests | 273517 | 4 + 4 | 1,209,459.9 | 1,209,459.9 | 1.6e-16 | exact |
| familyDocumentTests | 273536 | 4 + 4 | 1,209,459.9 | 1,209,459.9 | 3.2e-16 | exact |
| familyDocumentTests | 273555 | 4 + 4 | 1,209,459.9 | 1,209,459.9 | 2.8e-15 | exact |

The volume is exact for straight edges between corresponding vertices: the area of the section a fraction `t` of the way along is a quadratic in `t`, so Simpson's rule on `t` = 0, 1/2, 1 is the integral.

Which offset belongs to which sketch is not in the record's field names. Volume and bounding box do not tell the two pairings apart (both give the same). The corners do: with the first sketch at -1001804 and the second at -1001805 every corner lies on a vertex of the saved mesh (largest distance 0.00000 ft), and with the pairing exchanged the corners miss by 0.47 ft on the column and 0.12 ft on the four others.

## Not decided

- Five blends from two families, every section a four-vertex outline of straight edges, in planes parallel to a base plane. Whether the pairing by order holds for a blend whose sketches were created in the other order is not measured.
- The correspondence is vertex `i` to vertex `i` in the loops' assembled order. Where the two loops start at different vertices the volume would differ; no such blend is in the sample, and the reader does not choose a start.
- A section with an arc, a hole, a different vertex count or a plane not parallel to the other is refused.
- `SweptBlendElem` is not read.
- Both families are Revit 2020 standalone families: no blend of another release is measured, and none in a project's family.

## Change

`FormDocument::blend`, `blend_ids`; `Blend`, `BlendSection`, `FIRST_OFFSET_PARAMETER`, `SECOND_OFFSET_PARAMETER`; `FormDocument::insert` keeps `BlendElem` records. A test of the volume on a constructed frustum always runs; the saved-mesh agreement reads two files from `RVT_FAMILY_BLEND_DIR` and skips where it is unset.
