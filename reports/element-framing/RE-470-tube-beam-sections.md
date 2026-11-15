# RE-470: a beam whose type is a rectangular tube is drawn as that tube, with its six base quantities

**Date:** 2026-11-15
**Issues:** #1698 (a beam body that is not Revit's solid: the office's 8 hollow tubes); follows RE-221 (hollow-section columns, the same profile), RE-423 (a beam's six base quantities) and RE-461 (release gates).
**Artefacts:** the BIM4LCA office as built and at permit (Revit 2024), with Revit's IFC4 export of each.
**Scorer:** `tools/re/beam_tubes_in_ifc.py`; probe `probe_1698_beam_type_lengths` (commit `45c22b0abed2`, archived under `refs/archive/2026-11-15/cited/45c22b0abed2`).
**Status:** positive on the 8 beams of each office file. Measured on one project and one release (the rule is gated to Revit 2024, `TUBE_BEAM_REVIT_VERSIONS`).

## 1. Before

The office's 8 beams of type `Putkipalkki - suorakaide:PA-1` were drawn as solid boxes (`IfcRectangleProfileDef`, `partition_beam_axis`) and carried none of `Qto_BeamBaseQuantities`' `CrossSectionArea`, `OuterSurfaceArea`, `GrossSurfaceArea`, `NetSurfaceArea`, `GrossVolume` and `NetVolume`: 48 values missing on each file.

## 2. Where the profile is

Revit's export draws each as an `IfcExtrudedAreaSolid` over an `IfcArbitraryProfileDefWithVoids`: a 200 by 300 mm outline and a 192 by 292 mm void, each an `IfcIndexedPolyCurve` of 12 points with four lines and four three-point arcs alternating (outer corners rounded 8 mm, inner 4 mm), extruded along the beam for 5,685 mm, which is the `Length` (the scorer prints depth minus `Length`: 0 on all 8). The areas of the two rings are 59,925.0 and 56,045.3 mm2.

The beam's type holds those numbers as named lengths. `probe_1698_beam_type_lengths` prints them: `bf` 200 mm and `d` 300 are the outside, `tf` and `tw` 4 the wall, `ri` 8 the outside corner radius and `ra` 4 the inside one. They are not the column family's `b`, `Ht`, `t` and `kr` (RE-221), so the columns' reader cannot read them.

## 3. The change

- `partition_type_parameters::tube_beam_section` reads `TUBE_BEAM_PARAMETER_NAMES` (`bf`, `d`, `tf`, `tw`, `ri`, `ra`) and gives the section only where the wall is the same on both sides (`tf` = `tw`) and the outside radius is the inside radius plus the wall (`ri` = `ra` + `tf`, to 1e-9 ft), which is what the hollow section's one wall and one outside radius can say. Anything else leaves the beam its box.
- `attach_column_sections` reads it on Revit 2024 structural framing types (`TUBE_BEAM_REVIT_VERSIONS`), under its own reader name, `type_tube_section`, so that `tests/value_sources_offsets.rs` holds the six captions it is made of; the column reader's four were refused six (Corpus health caught it on the first head).
- `export_content` draws the beam as the tube (`beam_tube_solid`, the profile of RE-221's columns, extruded along the centreline as an I beam is) where its body along its line is as wide and as deep as the tube's outside, and writes the six quantities from that profile and the length (`add_tube_beam_quantities`). A beam turned so that its body is not that wide keeps its box.
- `SectionShape` is `RectangleHollow` and `SectionWallThickness` the wall, and the provenance note keeps the source `type_tube_section`.

## 4. Measured

Measure run 43667663717 (head `3666499531d7`, the two office files, `beam_tubes_in_ifc`, `beam_quantities_vs_ifc`, `qtos_vs_ifc`, `psets_vs_ifc`, `column_bodies_vs_ifc`, `keep_ifc`) and the unfiltered run 43667655898 (the merge base `63d06f42`, with samples, the BIM4LCA office, the Revit 2025 models and the held-out models). The rows were registered in the pull request before either was read.

- **The body is Revit's, up to where the ring starts and the digits.** All 8 beams of both files are an extrusion over `IfcArbitraryProfileDefWithVoids`, as Revit's are. Outer and void have 12 points, the same alternation of lines and arcs, the outer counter-clockwise and the void clockwise, and areas 59,925.0 and 56,045.2 mm2 where Revit's are 59,925.0 and 56,045.3. **The registered row (the same indices, points equal to 1e-6 mm) is not met**: rvt-rs starts its outer ring one vertex and its void ring two vertices before Revit's, and writes its points to six decimals of a metre, so after aligning the start they differ from Revit's by at most 0.000146 mm (outer) and 0.000427 mm (void). The row was tighter than the writer's resolution (1e-3 mm); the bound that was measured is these. Two beams 6,000.000 mm apart in Revit's export are 6,000.000 mm apart in rvt-rs's, turned by the office's site rotation (40.33 degrees), and rvt-rs's profile sits at Revit's height (77,320 mm plus 150 mm).
- **Quantities.** `CrossSectionArea`, `OuterSurfaceArea`, `GrossSurfaceArea`, `NetSurfaceArea`, `GrossVolume` and `NetVolume` are written on all 8 beams of each file and equal Revit's: 48 values per file, none different, none missing, and `Length` stays equal on 8 of 8. In the unfiltered run `Qto_BeamBaseQuantities` goes from 0 to 8 equal on each of those six, and the file's quantities on objects rvt-rs writes from 37,509 to 37,557 equal (as built) and 37,882 to 37,930 (permit), `different` 1 on both sides and values Revit does not write 0.
- **Nothing else moves.** 118 of 120 exports are byte for byte the merge base's, the two that are not being the office files; all 102 `diagnostics.json` are, so no `body_sources` count changes; `same-project gates` pass (15 samples held to their earlier saves). The Autodesk samples, Core Interior, Einhoven, RE1, the 23 Revit 2025 models and the held-out models hold no such type.
- **Cost.** One export of the as-built office took 21.3 s on the head and 23.7 s on the base (single runs; `attach_column_sections` now reads the type parameters of every Revit 2024 structural framing type, which costs nothing above the noise here).

## 5. Not measured, not read

- A tube on another release: the rule is gated to Revit 2024, the one release a file with such a type was found for. No Revit 2025 to 2027 model local holds one.
- A tube of another shape (round, or with a wall that differs on two sides): a beam whose type does not fit the six captions as above keeps its box.
- A tube beam turned so that its body is not as wide and deep as the tube's outside keeps its box.
- The structural framing sections #94 still lists (channels, angles, one HSS column, 17 I beams) are not changed here.

## 6. Reproduce

```bash
gh workflow run measure.yml --ref main -f ref=3666499531d701e8d5310d1c8947b8afdec477d5 -f base=63d06f42c9f8fe1f8f9964ea544f169018a98833 \
  -f nordic=true -f r2025=true -f samples=true -f holdout=true
gh workflow run measure.yml --ref main -f ref=3666499531d701e8d5310d1c8947b8afdec477d5 -f base=none -f keep_ifc=true \
  -f models="nordic-office-as-built nordic-office-permit" \
  -f scorers="beam_tubes_in_ifc beam_quantities_vs_ifc qtos_vs_ifc psets_vs_ifc column_bodies_vs_ifc"
# the ring's points: the scorer on rvt-rs's kept file against itself lists rvt-rs's curves
python3 tools/re/beam_tubes_in_ifc.py model.ifc model.ifc --list 8
```

The measured commit is archived under `refs/archive/2026-11-15/cited/3666499531d7` because the pull request is squashed.
