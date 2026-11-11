# RE-415: a sloped roof's volume in Revit's export is its saved mesh's times the cosine of its slope

**Date:** 2026-11-10
**Issues:** #1656; follows RE-389 (roofs drawn from their saved meshes), RE-401 and RE-402 section 7 (the first reading of the housing roof)
**Artefacts:** the BIM4LCA housing, concrete as built (Revit 2024; Revit 24.0.5's IFC4 export, CC BY-SA 4.0, as Measure fetches it), its roof 1627602 (`Basic Roof:KA-1`); and Autodesk's Snowdon Towers Sample Architectural 2024 with Revit's IFC4 export of it (local only, never committed)
**Probe and scorers:** `probes/examples/probe_1656_roof_primitives.rs`, `tools/re/roof_primitives_vs_ifc.py`, `tools/re/roof_volume_vs_slope.py`
**Runs:** Measure [42927588950](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42927588950), [42928680446](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42928680446) and [42930223177](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42930223177) (the probe and its scorer on the housing, the last at `8989ec36`)
**Status:** Positive: the 0.2% of #1656 is located and explained, and it is not a face. Revit's export writes a sloped roof's layers along the vertical where the saved mesh has them perpendicular to the slope, so its volume is the mesh's times the cosine of the slope: exactly, on 8 sloped roofs of two projects. Nothing in the reader changes.

## 1. The question

On each large file of the BIM4LCA housing one roof, 1627602, is drawn from its saved mesh and reads 0.2% over Revit's volume (31.677593 ft3 against 31.613562), where the other roofs match. #1656 asked where the extra 0.064 ft3 lies in the roof's saved graphics: a face or shell Revit's export does not draw, or a tessellation that differs.

## 2. The roof's saved graphics

`probe_1656_roof_primitives` prints every primitive of the element's saved graphics. The roof has 8, in 20 triangles and no diagnostic: two faces of 6 vertices and 4 triangles, 283.9803 ft2 each, whose normals are (0.0636, 0, -0.998) and (-0.0636, 0, 0.998), and six side faces of 2 triangles each (4.1989, 0.8015, 2.7489, 0.9225, 0.1210 and 1.4500 ft2). It is an L-shaped slab on a slope of 3.6436 degrees.

## 3. Revit's body

Revit's export writes the roof as 80 triangles, 40 of them distinct (every face twice, in the same winding), in two shells of 22.315456 and 9.298107 ft3 over their distinct triangles, 31.613562 in all. `roof_primitives_vs_ifc` sets each primitive and each shell against the other body:

- **The surfaces coincide.** All 120 of Revit's vertices lie within 5e-7 ft of rvt-rs's surface, and rvt-rs's 36 within 6.6e-5 ft of Revit's (`mesh_bodies_vs_ifc` reads the farthest vertex of the roof at 6.8e-5 ft; the 0.033 ft that #1656 gave is not what `main` measures today). No primitive of the mesh is off Revit's surface.
- **Revit's two shells are stacked layers with one footprint.** Both span x from -124.1503 to -115.8972 ft and y from 44.1764 to 81.7420 ft; shell 0 spans z 179.0990 to 179.7033 and shell 1 179.1779 to 179.7363. The faces where they meet are 4 triangles each of 283.9803 ft2, with opposite normals, the nearest vertex of one 0.000159 ft from the other's surface. So Revit's body is the mesh's slab cut into two layers, and the seam is not a gap.

## 4. The thicknesses

The roof's plan area is 283.4063 ft2 and the area of each sloped face 283.9803 ft2, a ratio of 0.997979, the cosine of the slope.

| | rvt-rs's saved mesh | Revit's export |
|---|---|---|
| volume | 31.677593 ft3 | 31.613562 ft3 (22.315456 + 9.298107) |
| thickness perpendicular to the slope | 34.0000 mm | 33.9313 mm |
| thickness along the vertical | 34.0689 mm | 34.0000 mm: 24.0000 mm and 10.0000 mm |

The mesh's volume over its sloped face is 34.0000 mm, the roof type's total (layers of 24 and 10 mm), measured perpendicular to the slope. Revit's two shells over the plan area are 24.0000 and 10.0000 mm: the layers, measured along the vertical. The ratio of the two volumes, 0.997979, is the cosine of the slope, 0.997979, to six digits.

## 5. Other roofs

`roof_volume_vs_slope` reads each roof that rvt-rs draws from a saved mesh against Revit's body (its distinct triangles) and the cosine of its slope, here for the 10 of Snowdon Towers Architectural (all in Revit's export):

| roofs | slope | Revit's volume over the mesh's | cosine of the slope | ratio over cosine |
|---|---|---:|---:|---:|
| 3 `Concrete - 6"` | flat | 1.00000 | 1.00000 | 1.00000 |
| 7 `Insulation on Metal Deck - EPDM` | 2.0 degrees | 0.99939 | 0.99939 | 1.00000 (each) |
| housing roof 1627602 | 3.6436 degrees | 0.997979 | 0.997979 | 1.000000 |

The housing's other roofs, which match, are flat. So the volume of a roof drawn from its saved mesh differs from Revit's by the cosine of its slope and by nothing else: 0.06% at 2 degrees, 0.2% at 3.6, 1.5% at 10, and the 1% bar of the scorers is passed up to 8.1 degrees.

## 6. What it means

- The difference is not a face, a shell or a tessellation, so there is nothing to draw differently. The saved mesh is the geometry Revit saved: its two faces are the type's 34 mm apart perpendicular to the slope. Revit's export writes the same layers along the vertical, which makes its body thinner by the cosine. rvt-rs does not scale a mesh to an exporter's convention.
- No written value follows either convention: rvt-rs writes no volume for a roof drawn from its mesh (`add_mesh_volumes` takes the mesh volume of curtain mullions, curtain panels and columns only, RE-387), and the slab quantities of a roof written from its sketch are RE-298's.
- The scorer already reads the housing roof `doubled` (equal within 0.5% once Revit's repeated faces count once), and 0.2% passes the 1% bar; nothing in Measure fails for it.

## 7. Not shown

- **Which convention Revit's own model uses.** Revit's API is not available here, and third-party API output is a lead only; the saved mesh is Revit's geometry and the export's body is what its exporter writes.
- **A roof by extrusion, or one whose layers differ in slope.** All 8 sloped roofs measured here are roofs by footprint with one slope.
- **Other releases.** Both projects are Revit 2024.

## 8. Reproduce

```bash
gh workflow run measure.yml --ref probe/1656-roof-primitives -f ref=8989ec36d23cca2d7100b8ca2a4731f8f10f383b -f base=none \
  -f nordic=true -f models="nordic-housing-concrete-as-built" \
  -f probe=probe_1656_roof_primitives -f probe_scorer=roof_primitives_vs_ifc -f scorers=none
python3 tools/re/roof_volume_vs_slope.py <rvt-rs export of Snowdon Towers Sample Architectural> <Revit's IFC4 export of it>
```

## 9. Sources

RE-387, RE-389, RE-298, RE-402. No Autodesk source was read.
