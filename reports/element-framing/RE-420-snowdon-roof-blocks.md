# RE-420: why Snowdon Towers Architectural's roofs stay level blocks: six have one decoder diagnostic each and the other fourteen are Revit's volume

**Date:** 2026-11-11
**Issues:** #1630 (closed by this report), #1683 (the six decoders); follows RE-389 (#1612, the roofs drawn from their saved meshes), RE-408 (instances forbidden the targets 10) and RE-415 (the slope's cosine)
**Artefacts:** Autodesk's Snowdon Towers Sample Architectural (Revit 2024, as Measure fetches the sample) and Revit's IFC4 export of it (local only: percentages only here, no Revit-side volumes); rvt-rs's export of it by `main`, which Measure kept (`keep_ifc`)
**Probe and scorer:** `probes/examples/probe_re389_roof_blocks.rs`, `tools/re/roof_bodies_vs_ifc.py`
**Runs:** Measure [42941250988](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42941250988) (the probe, `main` at `56821a14aec98f9fc8c4f1bba2d35c2f91738ad2`, the 2024 samples) and [42943220826](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42943220826) (`keep_ifc`, `main` at `6bb024499a5866ea6b4fafa9e00de3e5ac52fd0e`, the 2024 samples)
**Status:** Positive for the question; three of the five rows registered on #1630 missed, as section 5 says. Six roofs keep their level block, each with exactly one decoder diagnostic and no mesh; the other fourteen are drawn from their saved meshes and read within 0.061% of Revit's volume.

## 1. The question

#1630 (2026-11-09) found that #1612 draws 10 of the model's 20 roofs from their saved meshes and leaves the other 10 as the level block of their sketch outline, 4.9% to 133% above Revit's volume, and did not know why those 10 do not take their mesh.

## 2. What the scene holds for each roof

`probe_re389_roof_blocks` exports the model without the saved-mesh pass and prints, for each of the 20 `IfcRoof`, the saved scene's record (`native_saved_scene` at detail 3). All 20 read `partial_saved_graphics`, because the status counts an unbounded face as well as a diagnostic, and the pass reads only the diagnostics (`saved_meshes.rs`: a decoder diagnostic that is not a view filter leaves a mesh partial; an element with no primitive or no triangle keeps its box). By that rule:

| roofs | primitives | triangles | unbounded faces | diagnostics |
|---|---:|---:|---:|---|
| 14 | 6 to 47 | 12 to 668 | 1 to 27 | none |
| 964559 | 0 | 0 | 1 | `multiple cylinder trim loops unsupported` |
| 1916596 | 0 | 0 | 0 | `shared curved edge changes its fixed parameter` |
| 1926040, 1975810, 1985275, 2112673 | 0 | 0 | 1, 0, 0, 0 | `malformed ruled-surface edge is connected to face #` |

The unbounded-face count does not separate the two sets (the fourteen have 1 to 27); the diagnostic, with its empty mesh, does.

## 3. What the export did

`RvtElementRecordGeometry.BodySource` and `SavedMeshRefusal` of each roof in the kept export (`tools/re/roof_bodies_vs_ifc.py` loads the same file): 14 roofs are `saved_graphics_mesh` with no refusal, and the six above are `partition_element_record_bbox` with their diagnostic as the refusal text, `partial: ` and the kind. So each block now says why it is one (#1645, #1647).

## 4. Against Revit's export

`roof_bodies_vs_ifc.py` on the kept export and the local IFC4, all 20 roofs paired by `Tag`:

| roofs | volume over Revit's | every vertex of Revit's body from rvt-rs's surface |
|---|---|---|
| 759489, 882818, 1916295, 1916467, 1916546, 1916562, 1916578 | 0.0610% to 0.0612% (the cosine of their 2.0 degree slope, RE-415) | 1.8e-6 ft at most |
| 754197, 754672, 754785, 1924305, 1924316, 1924325, 1935345 | 6e-7 or less | 3.7e-6 ft at most |
| 1916596 (shared curved edge) | 74% | 0.0005 ft |
| 964559 (multi-loop cylinder), 1975810, 1985275 (ruled surface) | 42%, 40%, 42% | 0.3 to 0.4 ft |
| 1926040 (ruled surface) | 33% | 0.4 ft |
| 2112673 (ruled surface) | 133% | 0.4 ft |

The six that stay blocks are the six largest of the ten #1630 listed. The four that read 4.9% to 5.8% over then are among the fourteen now, equal to Revit's body to 1e-6: they are the roofs with 5 to 27 unbounded faces.

## 5. The rows registered on #1630, as they came out

- R1 (10 roofs stay blocks, each with a diagnostic or an unbounded face and a triangle) **missed**: 6 stay blocks, and each has no triangle.
- R2 (the diagnostics are kinds other classes already show) held for two roofs and **missed for four**: `malformed ruled-surface edge is connected to face #` is on 4 elements of this model's saved-mesh census and on these four roofs alone, a new kind (`ruled_surface_component`, `src/research/native_parametric_mesh.rs`: one edge of a ruled-surface component that does not resolve to two `Face` objects through `m_pFace` refuses the whole component).
- R3 (the 10 that take their mesh have no unbounded face and no diagnostic) **missed**: they have 1 to 27 unbounded faces; the diagnostic separates the sets, not the unbounded count.
- N1 (the 14 without a diagnostic are meshes within 1% of Revit's volume) held, bar its clause "0.01 ft of its box", which the scorer does not read (its surface test is stricter) and which is not measured. Its vertex-to-vertex line reads `differs` on 8 of the 14, because Revit's tessellation of a roof has more vertices than the mesh Revit saved (32 against 8 on the seven slope roofs).
- N2 (the other 6 keep their box and carry the refusal text) held.

## 6. Not shown

- **Why four more roofs are drawn than on 2026-11-09.** They are the ones with many unbounded faces, and RE-408 landed in between; no run isolates it.
- **The cause of the ruled-surface refusal.** It is the guard that refuses, not a decoded fault: which edge fails to resolve, and to what, is not read (the next step, #1683).
- **Revit's volumes.** The Revit IFC4 of this model is local only, so every Revit-side figure is a percentage.
- **Another project's blocked roofs.** One model; the housing's 5 roofs per file are drawn.

## 7. Reproduce

```bash
gh workflow run measure.yml --ref main -f ref=56821a14aec98f9fc8c4f1bba2d35c2f91738ad2 -f base=none -f samples=true -f sample_years="2024 2024" \
  -f scorers=none -f probe=probe_re389_roof_blocks -f probe_scorer=-
gh workflow run measure.yml --ref main -f ref=6bb024499a5866ea6b4fafa9e00de3e5ac52fd0e -f base=none -f samples=true -f sample_years="2024 2024" \
  -f scorers=none -f keep_ifc=true
python3 tools/re/roof_bodies_vs_ifc.py <kept>/sample-2024-snowdon_towers_sample_architectural/model.ifc <Snowdon Revit IFC4>
```

## 8. Sources

RE-389, RE-408, RE-415, #1612, #1645. No Autodesk source was read.
