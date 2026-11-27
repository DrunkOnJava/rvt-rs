# RE-489: a standalone family of Revit 2014 frames and its extrusions rebuild, as on 2015 to 2027

**Date:** 2026-11-17
**Issues:** #1189 (W3: families read like projects); follows RE-327 (family extrusions) and RE-280 (the releases the native framing holds on).
**Artefacts:** the 8 families of Revit 2013 and 2014 among DynamoRevit's test suite (Apache-2.0, not redistributed here): the Revit 2014 ones are `test__revit__Family__GetCurvesFromFamilyInstance`, `test__System__AdaptiveComponent__AdaptiveComponent.0002`, `test__System__DividedCurve__DividedCurve`, `test__System__Samples__AllCurves.0007` and `.0008`, `test__System__Selection__Selection.0001` and `.0002`; the Revit 2013 one is `test__System__AdaptiveComponent__AdaptiveComponent.0001`.
**Probe:** `probe_re327_family_extrusions`, unchanged, run on the three files that hold extrusions.
**Status:** positive for Revit 2014, with 7 files; negative for Revit 2013.

## 1. The question

`native_form_geometry::open` refused every release outside `native_document::NATIVE_RELEASES` (2015 to 2027) with "native record framing is unvalidated for Revit 2014". Discussion #112 reports a Revit 2014 family whose extrusions rebuild from their records. Do a standalone Revit 2014 family's plain groups frame, and do its extrusions match what Revit saved?

## 2. Measured

With the release check widened to 2014 and nothing else changed:

| File | Release | Plain groups | Unframed | Extrusions built | Match their complete saved mesh |
| --- | ---: | ---: | ---: | ---: | ---: |
| `DividedCurve` | 2014 | 8 | 0 | 6 | 6 of 6 |
| `Selection.0001` | 2014 | 18 | 0 | 3 (1 void) | 1 of 1 (2 have no mesh, 1 of them the void) |
| `Selection.0002` | 2014 | 19 | 0 | 3 (1 void) | 1 of 1 (2 have no mesh, 1 of them the void) |

The other four Revit 2014 files frame with no unframed group and hold no extrusion.

"Match" is the probe's: volume within 1% and the chord loss, bounding box, every mesh vertex and samples of the solid's surface within 0.01 ft, under the default convention (`tally matches under default`). 8 of the 8 extrusions with a complete mesh match; the 4 others have no mesh, so nothing scores them.

## 3. Not decided

- **Revit 2013:** `AdaptiveComponent.0001` still fails to open, with "schema declared field budget at 58"; the range stays at 2014.
- **`VarSketch`:** the probe's `refused VarSketch` tally is 18 on `DividedCurve` and 42 on each `Selection` file, on Revit 2014 only (the 2016 to 2027 families measured, 12 files, refuse none). The extrusions still build, so those records are not needed for them; what they hold on 2014 is not read here.
- **Not measured:** a Revit 2014 family with a revolution, a sweep or a blend; any Revit 2014 project, where the project readers keep `NATIVE_RELEASES`.
- **Corpus:** three files, one author's test suite. No other Revit 2014 family was available.

## 4. Change

`native_form_geometry::FAMILY_RELEASES` (2014 to 2027) replaces `NATIVE_RELEASES` in `open`. `tests/family_2014_extrusions.rs` reads the three files from `RVT_FAMILY_2014_DIR` and skips where it is unset.
