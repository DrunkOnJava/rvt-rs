# RE-442: a tube beam's saved mesh gives none of Revit's six base quantities to the census's tolerance, and the profile Revit draws gives all six

**Date:** 2026-11-11
**Issues:** #1698 (beams whose body is not Revit's solid: the office's 8 hollow tubes), #1693 (closed; RE-423 left the 8 without the six quantities), #1636 (the rules rest on the BIM4LCA models)
**Artefacts:** the BIM4LCA concrete office as built, Revit 2024, `ARK_NordicLCA_Office_Concrete_As-Built.rvt` (sha256 `747120005b4b35c7cb66ec61158aab50ded95e052e172cb387b86cd15b31d906`), with Revit's own IFC4 export of it, `ARK_NordicLCA_Office_Concrete_As-Built_Revit.ifc` (sha256 `b2f530eba8b4424c4542b98e9a2c7bbcc726367f21b2740f3074e4a6ce917382`); both in `research/bim4lca-models.tsv`
**Probe:** `probes/examples/probe_beam_tube_quantities.rs`
**Run:** Measure [43113951802](https://github.com/DrunkOnJava/rvt-rs/actions/runs/43113951802)
**Status:** Negative for the saved mesh. Nothing is written from it. The six values are those of the drawn profile and the length, to the census's tolerance, so they belong with the body #1698 asks for.

## 1. The question

Revit's export writes `Qto_BeamBaseQuantities`' `CrossSectionArea`, `OuterSurfaceArea`, `GrossSurfaceArea`, `NetSurfaceArea`, `GrossVolume` and `NetVolume` on each of the office's 8 beams, all of one type, a hollow rectangular tube. rvt-rs writes `Length` on them and none of the six. RE-423 writes a beam's six from the prism its saved mesh is, and refuses a mesh that is not one closed solid with no hole through it, which leaves the tubes out. Can a second rule read the six from a tube's saved mesh, with the lateral faces split into the outer surface and the inner one?

## 2. What the probe prints

For every structural-framing element whose saved graphics decode whole and whose element data holds a bounded line, the probe reads the mesh along that line: whether the triangles close, the pieces, the planes square to the line and their areas, the area of the faces along the line split by whether their normal points away from the line or towards it, the extent along the line and the volume of the signed tetrahedra.

All 8 beams are meshed and read, and the 8 lines are identical:

| Figure | Value |
| --- | --- |
| pieces | 1 |
| triangles close (every directed edge met once by its opposite) | no |
| planes square to the line | 2, of 0.041904762 sq ft each |
| extent along the line | 18.651574803 ft |
| lateral area, outward and inward | 60.343600152 and 58.810044852 sq ft |
| volume | 0.781589801 cu ft, the end area times the extent |

## 3. Against Revit's export

The six quantities those figures make, in square and cubic metres, against the values Revit's export writes on the same 8 beams (identical on all 8):

| Quantity | From the mesh | Revit's export | Relative difference |
| --- | ---: | ---: | ---: |
| `CrossSectionArea` | 0.003893080 | 0.003894736 | 4.3e-4 |
| `OuterSurfaceArea` | 5.606103899 | 5.606919268 | 1.5e-4 |
| `GrossSurfaceArea` | 5.613890058 | 5.614708741 | 1.5e-4 |
| `NetSurfaceArea` | 11.077522008 | 11.078748375 | 1.1e-4 |
| `GrossVolume` | 0.022132158 | 0.022141577 | 4.3e-4 |
| `NetVolume` | 0.022132158 | 0.022141819 | 4.4e-4 |

`tools/re/qtos_vs_ifc.py` reads two values equal within a relative 1e-5, or 1e-6 absolute. All six are outside it on all 8 beams: 48 values that would read different, 0 equal. Every mesh figure is short of Revit's, as a tube whose rounded corners are chords is.

## 4. What does give Revit's values

Revit's export draws each of the 8 as an `IfcExtrudedAreaSolid` over an `IfcArbitraryProfileDefWithVoids`: an outer rectangle of 200 by 300 mm with 8 mm corners, a void of 192 by 292 mm with 4 mm corners, extruded 5,685 mm. With `A` the section's area (outer less void), `Po` and `Pi` the outer and inner perimeters and `L` the length:

| Quantity | From the profile | Value | Revit's export | Difference |
| --- | --- | ---: | ---: | ---: |
| `CrossSectionArea` | `A` | 0.003894796 | 0.003894736 | 6.0e-8 |
| `OuterSurfaceArea` | `Po * L` | 5.606919268 | 5.606919268 | under 1e-9 |
| `GrossSurfaceArea` | `Po * L + 2 A` | 5.614708861 | 5.614708741 | 1.2e-7 |
| `NetSurfaceArea` | `(Po + Pi) * L + 2 A` | 11.078748495 | 11.078748375 | 1.2e-7 |
| `GrossVolume` | `A * L` | 0.022141918 | 0.022141577 | 3.4e-7 |
| `NetVolume` | `A * L` | 0.022141918 | 0.022141819 | 1.0e-7 |

All six are within the census's tolerance, `OuterSurfaceArea` to the last digit printed. Revit's `CrossSectionArea`, `GrossVolume` and `NetVolume` are not exactly the arcs' own (they differ from the closed form in the fifth significant digit, and from each other), which the absolute bound absorbs at this size; a larger tube may not be absorbed, and that is not measured here.

## 5. What this binds

- No rule writes a tube beam's six quantities from its saved mesh. A mesh that reads Revit's values to four digits is not a reason to: the census's bar is 1e-5.
- The six quantities come with the tube's body. Once #1698 finds the profile in the file (the two rectangles and their corner radii) and draws the tube as Revit does, they follow from the profile and the length. Where the profile is held is not read here.
- Measured on one type on one model: 8 beams, one section. Another tube section, or a round one, is not measured.

## 6. Reproduce

```bash
# the probe on the office as built; its output is the model's probe.txt in the run's measure-head artifact
gh workflow run measure.yml --ref main -f ref=317b41b85f0ce44b3d29dccfec6fb54c246c8bec -f base=none \
  -f nordic=true -f models="nordic-office-as-built" -f probe=probe_beam_tube_quantities -f probe_scorer=- -f scorers=none
# Revit's values: the Qto_BeamBaseQuantities of the 8 IfcBeam in ARK_NordicLCA_Office_Concrete_As-Built_Revit.ifc,
# and the IfcArbitraryProfileDefWithVoids their bodies extrude
```
