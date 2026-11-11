# RE-428: a beam's Length, Span, Slope and Roll are its location line's, whatever its body is drawn from

**Date:** 2026-11-11
**Issues:** #1697 (the beams a saved mesh draws carry none of the four); follows RE-49 (the location line), RE-276 (`Roll`), RE-328 (a box longer than its line) and RE-423 (a beam's six base quantities)
**Artefacts:** the BIM4LCA office and housing, concrete and timber, as built and at building permit (Revit 2024), and Revit's IFC4 exports of them (ten models, 300 beams); the 18 held-out models as counts only (the only beams among them are `modelo_bim`'s 8)
**Probe and scorer:** `probes/examples/probe_1697_beam_lines.rs`, `tools/re/beam_lines_vs_ifc.py`
**Runs:** Measure [43014215535](https://github.com/DrunkOnJava/rvt-rs/actions/runs/43014215535) (the probe, `main` at `5cf94975`) and the change [43018018669](https://github.com/DrunkOnJava/rvt-rs/actions/runs/43018018669) (unfiltered, `keep_ifc`, head `558263ef` against `main` at `5cf94975`) and [43023503554](https://github.com/DrunkOnJava/rvt-rs/actions/runs/43023503554) (the final head `2fc89466`, `same-project gates` passing)
**Status:** Positive. The location line of a beam gives Revit's `Length`, `Span`, `Slope` and `Roll` on all 291 beams that have one in the BIM4LCA office and housing; rvt-rs now writes them for every such beam, wherever its body comes from, adding 246 values in the housing's four files, each equal to Revit's, with no `different` and no value Revit does not write on any IFC4 model. 9 beams have no line and stay without.

## 1. The question

After RE-423 wrote a beam's six base quantities, what Revit's export still writes on a beam that rvt-rs does not is `Qto_BeamBaseQuantities.Length` and `Pset_BeamCommon` `Span`, `Slope` and `Roll` on the beams rvt-rs does not draw along their line: 28 of the timber housing's 41, 7 of the timber office's 71, 1 or 2 of the concrete housing's 22 (#1697). The issue found no `Axis` representation in Revit's export to read the line from, and `Length` differing from the body's long extent on 20 of the concrete housing's 22. What is each of the four a function of?

## 2. What Revit's export says

Read from Revit's IFC4 alone (IfcOpenShell): every `IfcBeam` of the four as-built files has a `Body` and an `Axis` representation, both `IfcMappedItem`s. The `Axis` representation is an `IfcIndexedPolyCurve` of two points, which the earlier scorer did not read. On all 142 beams (22 concrete housing, 41 timber housing, 8 concrete office, 71 timber office):

- `Qto_BeamBaseQuantities.Length` and `Pset_BeamCommon.Span` equal the length of that curve, to 1e-15 relative. They equal each other on every beam.
- `Slope` is the curve's angle to the horizontal, atan2(|dz|, horizontal run), equal to within 5e-7 degrees (the rounding of the stored coordinates) on the timber housing's 9 sloped beams, and 0 on the 32 level ones.
- `Roll` is 0 on the timber housing's 41.

The body does not give them: the housing's profile is extruded across the beam, and `Length` is still the line's. So the four are a function of the beam's location line, not of the solid.

## 3. What rvt-rs holds

The probe prints, per structural-framing element, the first bounded line of its element data (the record RE-49 reads), whether the exporter attaches it, whether a column cuts the beam, the rotation read, and whether the line lies inside the record box and a box along it closes. `tools/re/beam_lines_vs_ifc.py` holds each against Revit's beam by ElementId. Over the ten BIM4LCA models (Measure 43014215535):

| group | beams | Length, Span, Slope, Roll equal to Revit's | rvt-rs wrote them before |
|---|---:|---:|---|
| line attached, a box along it closes (`partition_beam_axis`) | 234 (216 level, 18 sloped) | 234 of 234 | `Length`, `Span`, `Roll`; `Slope` on the 216 level |
| line attached, the box is longer than the line (`partition_beam_box_along_axis`, RE-328) | 39 | 39 of 39 | none |
| line read, inside the record box, no box along it closes | 18 (all sloped, timber housing) | 18 of 18 | none |
| no bounded line in the element data | 9 | | none (Revit writes all four) |

291 beams have a line, and on every one the line gives Revit's four values: the length for `Length` and `Span`, the angle for `Slope` (36 sloped beams, 9 in each of the four housing files), and the Cross-Section Rotation of the native record for `Roll` (read on all 291, 0 on each). Every line lies inside its beam's record box and no beam is cut by a column in these models. The timber housing's 28 beams of #1697 are the 18 box beams, 9 not attached and the beam with no line. The 9 beams with no line (one in each housing file as built and 7 in the timber office at permit) stay without. The held-out `modelo_bim` holds 8 beams, each cut by a column with no line: nothing is read.

## 4. The rule

The line of a beam's element data that lies inside its record box is kept for the beam's properties apart from the line its body is solved along (`BEAM_LINE_START_FIELDS`, `attach_beam_axes`). `Qto_BeamBaseQuantities.Length` is that line's length wherever the body comes from (the record set's `AxisLength` where the body is a box along it, as before), `Span` is the same, `Slope` is its angle to the horizontal (0 for a level line, as before), and `Roll` is 0 where the Cross-Section Rotation is read and 0, as before. A beam with no line, and a beam whose rotation is unread or not 0 (`Roll`), gets none of the values it did not get. The bodies do not change.

## 5. The registered rows

Registered on #1636 before the probe was read (https://github.com/DrunkOnJava/rvt-rs/issues/1636#issuecomment-6794877307, rows 1 to 6; the probe read at 6795370765) and before the change's runs (rows 7 to 11, https://github.com/DrunkOnJava/rvt-rs/issues/1636#issuecomment-6795423510).

| row | the bar | result |
|---|---|---|
| 1 | At least 95% of the 142 beams of the four as-built files have a bounded line | **Held:** 140 of 142 (98.6%); the two without are one beam each of the two housing files. |
| 2 | The line's length is Revit's `Length` and `Span` on every beam with a line | **Held:** 291 of 291 across the ten BIM4LCA models, 273 attached lines and 18 read and not attached. |
| 3 | The line's angle is Revit's `Slope` | **Held:** 291 of 291, among them the 36 sloped beams. |
| 4 | Revit's `Roll` is the rotation read, where one is read | **Held for a rotation of 0:** read on all 291 and 0 on each, as is Revit's `Roll`. A rotated beam is not measured. |
| 5 | Why the 28 are not attached | **Answered:** 18 have a box longer than the line (RE-328), 9 have a line no box along it closes, 1 has no line; the box beams were excluded on the belief that "no Revit export measures a beam longer than its line", which Revit's writes the line's length on all 39 of them contradicts. |
| 6 | The held-out models with beams read the same, counts only | **Held:** `modelo_bim`'s 8 beams are each cut by a column with no line (nothing read); the IFC2X3 held-out files hold beams too: Gdynia 2021 `Slope` and `Span` equal 3 to 9 of 10, Vitoria 2018 0 to 1, none different (`Roll` is not scored in IFC2X3). |
| 7 | The four are Revit's on every beam with a line; no `different`, no value Revit does not write | **Held** (Measure 43018018669, head `558263ef`; the final head `2fc89466` reproduces it: run 43023503554 holds the same 126 exports, byte for byte, the same 42 that move, the same censuses, `same-project gates` passes on it, and the MCP answers scorer, which counted a sloped beam's `Slope` in degrees against the answers' radians in the first run, now holds 3 to 6 more property values equal to Revit's in each housing file and 1 to 2 more quantity values in the timber files, with the 5 or 6 answers per file that are not Revit's unchanged): timber housing as built 13 to 40 of 41 on each of the four and at permit 13 to 41, concrete housing as built `Length`, `Span`, `Roll` 20 to 21 and `Slope` 11 to 21 of 22 and at permit to 22 of 22; the office files, whose beams are all of the axis group and level, are unchanged. 246 values added in the four housing files, 189 property values and 57 quantities, each equal. `different` is unchanged on every model (1, 1, 1, 1 in the property census; 0, 4, 0, 0 in the quantity census) and so is every value Revit does not write on the IFC4 models. On the IFC2X3 oracle models, which count apart: Revit's export holds none of `rac_advanced` 2015 to 2020's 8 beams' `Span` and `Slope`, so 16 values per file are rvt-rs's alone. |
| 8 | No body changes; `same-project gates` pass | **Held:** `body_sources` identical on the 101 models that report them, the gates pass on the head. |
| 9 | What moves | **Held, more widely than registered:** 42 of 126 exports move (84 are byte-identical): the housing's four files, nine IFC2X3 oracle models (`rac_advanced` 2015 to 2020, Gdynia 2021, Vitoria 2018 and its spent pair), and 29 of Autodesk's samples that hold beams (`rac_advanced`, `rst_basic` and `rst_advanced` of 2015 to 2027, Snowdon Towers Structural 2024). The office files and the rest are byte-identical. On the 33 pairs that keep their exports no element is added or removed and 700 elements differ, all `IfcBeam`: 700 gain `Pset_BeamCommon` properties, 460 gain `Qto_BeamBaseQuantities.Length`, and no set loses or changes a property. |
| 10 | What stays without | **Held:** the 9 beams with no line (one in each housing file as built, 7 in the timber office at permit) and `modelo_bim`'s 8 get none. |
| 11 | No new read; time and memory within noise | **Not timed:** the line is the one `scan_bounded_lines` already reads. |

The scorer that compares the MCP server's answers with Revit's export compared the answers' radians with Revit's degrees, and counted the sampled sloped beams as different (5 to 8 and 6 to 7 on the housing files in the first run): it converts Revit's plane angles to radians now, as `psets_vs_ifc` does.

## 6. Reproduce

```bash
# the probe and scorer on the BIM4LCA models and the held-out ones (counts)
gh workflow run measure.yml --ref research/1697-beam-lines -f ref=f08181eaa4628cae9bea9f5566aed53c46212c6e -f base=none -f nordic=true -f holdout=true -f models="nordic-* holdout-*" -f scorers=none -f probe=probe_1697_beam_lines -f probe_scorer=beam_lines_vs_ifc
# the change against main
gh workflow run measure.yml --ref research/1697-beam-lines -f ref=558263ef029352150cebbc63d9cf1ddce5181938 -f base=5cf9497503de71582766b422d150038e4cda712f -f samples=true -f nordic=true -f r2025=true -f holdout=true -f bimcollab=true -f racadv=true -f keep_ifc=true
# one model
cargo run --profile ci -p rvt-probes --example probe_1697_beam_lines -- MODEL.rvt > probe.txt
python3 tools/re/beam_lines_vs_ifc.py probe.txt <revit-export.ifc> --list
```

The commits the runs measured, `af8bee58` and `f08181ea` (the probe and scorers), `558263ef` (the change) and `2fc89466` (the final head), are kept under `refs/archive/2026-11-11/cited/<first 12 hex>`.
