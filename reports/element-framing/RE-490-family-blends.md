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

## Corpus run (2026-11-27)

`probe_re490_family_blends` reads every blend of 83 files with `FormDocument::blend`: 18 files of the family corpus, Autodesk's 59 sample projects of Revit 2015 to 2027 (the families each project has loaded) and six reference models. 48 of them hold a blend; the family corpus holds none. Counts only (Measure run 46029833703 at `3aed43e6`). The family corpus's Revit 2014 file could not be fetched, as its host now answers 404. The first run (44483325377 at `808c2a3e`, 2026-11-19) read it and refused it as a Revit 2014 file, so it held no blend that run counted, and every count below is the same in both runs; whether it holds a blend, now that 2014 families open (RE-489), is not measured.

| What the reader does | Blends |
|---|---:|
| Found | 1,260 |
| Refused: a section has an arc | 604 |
| Refused: open loop | 104 |
| Read | 552 |

Of the 552 read, 484 have a complete saved mesh and 68 none.

| Against the saved mesh | Blends |
|---|---:|
| Volume agrees, corners on the mesh with the offsets paired as read | 343 |
| Volume agrees, corners on the mesh with either pairing | 13 |
| Volume differs (by 10% or more on 46), corners on the mesh as read | 48 |
| No volume to compare, corners on the mesh as read | 25 |
| No volume to compare, corners on the mesh with either pairing | 1 |
| Corners more than a foot off the mesh with either pairing (40 with a volume that differs, 14 with none) | 54 |

- **The pairing holds:** no blend fits only with the offsets exchanged, 0 of 552.
- **Agreement in full:** 356 of the 484 with a complete mesh agree in volume and corners.
- **Saves repeat:** the sample projects are a few projects saved by thirteen releases, so a blend is counted once in each save. By project, family and ElementId there are 147 distinct blends: 96 agree in full, 15 have their corners on the mesh and another volume, 10 have no volume to compare, 6 have their corners off the mesh and 20 have no saved mesh. Each comes out the same way in every save.
- **Not explained:** the 54 whose corners are off the mesh, the 48 whose corners fit and whose volume does not (the vertex correspondence, or a cut by another form), and the 26 whose corners are on the mesh and that have no volume to compare (14 more with no volume have their corners off it and are among the 54).

Reproduce: `gh workflow run measure.yml --ref main -f ref=3aed43e6f933a194732ec0a8492c584bd161d78c -f base=none -f families=true -f samples=true -f scorers=none -f probe=probe_re490_family_blends`.

Kept output: `refs/evidence/46029833703-probes` (commit `f3266325`), one `probe.txt` per file.

## Not decided

- The first measurement was five blends of two families, every section a four-vertex outline. The corpus run above reads 552 blends and finds none that fits only with the offsets exchanged; where neither pairing can be checked (no saved mesh, or corners off the mesh with both), the pairing by order is not measured.
- The correspondence is vertex `i` to vertex `i` in the loops' assembled order. Where the two loops start at different vertices the volume would differ, and the reader does not choose a start. Whether the corpus run's 48 blends whose corners fit and whose volume does not are such blends is not read.
- A section with an arc, a hole, a different vertex count or a plane not parallel to the other is refused.
- `SweptBlendElem` is not read.
- The first five are in Revit 2020 standalone families. The corpus run reads blends in the families loaded in Autodesk's sample projects of every release from 2015 to 2027 and in four reference models; the family corpus holds none, so no standalone family of another release is measured.

## Change

`FormDocument::blend`, `blend_ids`; `Blend`, `BlendSection`, `FIRST_OFFSET_PARAMETER`, `SECOND_OFFSET_PARAMETER`; `FormDocument::insert` keeps `BlendElem` records. A test of the volume on a constructed frustum always runs; the saved-mesh agreement reads two files from `RVT_FAMILY_BLEND_DIR` and skips where it is unset.
