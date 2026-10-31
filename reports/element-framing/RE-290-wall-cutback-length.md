# RE-290 — A wall's cut-back centreline confirms the box's length

**Date:** 2026-10-30
**Issues:** #1046 (quantities Revit computes from its joined geometry, step 1: walls), #1016 (quantities), W7 of [`docs/release-1.0-criteria.md`](../../docs/release-1.0-criteria.md); follows RE-252 (a wall's cut-back centreline), RE-267 (the top cut at a covering floor) and RE-270 (a wall's side areas and volumes)
**Artefacts:** the BIM4LCA office as built and at building permit (Revit 2024, Revit 24.0.5's IFC4 exports, CC BY-SA 4.0, `research/bim4lca-measure.tsv`) and puzzbobb's R2025 models (Revit 2025, Apache-2.0, `research/r2025-models.tsv`), as Measure fetches them
**Probe:** `probes/examples/probe_re290_wall_cutbacks.rs`. **Scorer:** `tools/re/wall_quantities_vs_ifc.py` (RE-270)
**Runs:** Measure [41136551451](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41136551451) (`main` at f73908bf: the scorer and `probe_1067_native_dump` on both offices and the R2025 models), [41138696192](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41138696192) (this change as first pushed, 46e7dacf, against `main` at f73908bf: the scorer, `probe_1067_native_dump` and `probe_re290_wall_cutbacks`), [41146179183](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41146179183) (this change at 48589c62 against `main` at 9637d5ff, every model and scorer), [41182651693](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41182651693) (this change at 790f3bfe, `main` merged in, against `main` at 3bc5e21c, every model and scorer) and [41180458307](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41180458307) (this change at a07a850d against `main` at aa917438, with the held-out models)
**Result:** positive. A wall's `Length` in Revit's export, the length of its cut-back centreline or of its curve, is also the length of the longest vertical face of its solid, and rvt-rs reads that cut-back length wherever both ends of the wall are read twice alike. On a wall whose body is the record's box and joined to roofs or floors, the box is that long on every wall for which the length is read, and on none else it is known to be right: the second RE-270 rule now takes a wall whose cut-back length is its box's long side, as well as one whose record confirms its `Length`. It adds 29 office walls as built and 36 at permit to RE-270's 67 and 70, Revit's on all four quantities on every one, none different and none Revit's export leaves out.

## 1. Revit's face is its cut-back length

`wall_quantities_vs_ifc.py` lists, per wall, Revit's quantities and the longest face of Revit's and rvt-rs's bodies. On the record-box walls with a `Height` whose record holds no step but the harmless ones, a roof or floor join and a clean join (the pool of RE-270's second rule: 132 as built, 141 at permit), the longest vertical face of Revit's solid is as long as its exported `Length`:

| | as built | at permit |
|---|---:|---:|
| the face is as long as Revit's `Length` | 128 of 132 | 127 of 141 |
| `GrossSideArea` = `NetSideArea` = `Length` × `Height`, `GrossVolume` and `NetVolume` = `Width` × `Length` × `Height` | 124 of 132 | 123 of 141 |

Of the walls whose `Length` is not the curve's (65 as built, 67 at permit), the face is that `Length` too on 64 and 60: it is the `Axis` Revit exports, the centreline cut back where the wall meets the walls joined at its ends (RE-252). So the quantities of a plain wall are fixed by the cut-back length, which RE-252 reads from the record and the join reaches (`native_walls::trimmed_ends`). RE-270's second rule used it only where it equalled the curve's length, the wall's `Length` (88 of the office's walls as built get one); the other walls were left out for want of a way to tell the box's length right.

## 2. The cut-back length tells it

`probe_re290_wall_cutbacks.rs` prints each wall's curve length, its cut-back length (new field `m_wall_cutback_length`, set wherever `trimmed_ends` reads both ends), its `Length` and its record box. Against the walls of §1:

| record-box wall joined to roofs or floors | as built | at permit |
|---|---:|---:|
| the cut-back length is read | 55 | 68 |
| ... and is the box's long side (1e-4 ft) | 55 | 68 |
| ... and is Revit's longest face | 55 | 67 |
| the cut-back length is not read | 30 | 25 |
| ... the box is Revit's face | 21 | 21 |
| ... the box is not (or Revit writes no `Height`) | 9 | 4 |

Where it is read, the cut-back length is the box's long side on every wall of these files, so the box and the record's join reaches agree with a second reading; and where the box is not Revit's face the cut-back length is not read. That also means the 1e-4 ft comparison with the box's long side has never rejected a wall on the corpus: what limits the rule is that the cut-back length is read at all (both ends of the wall read twice alike, RE-252) and the other conditions of §3.

The one wall at permit where the cut-back length is read and Revit's face differs from the box (2601954, a box 18.86 ft long whose face is 12.74 ft in Revit's solid, so that its `GrossSideArea` is 156.7 ft² where the box gives 232.1) is kept out by the join gate, not by the cut-back length: its record confirms its `Length` (18.86 ft, so RE-270's first condition would take it) but its top join names slab 1451421 and ceiling 2094524 (`Compound Ceiling:AK-3`, an `IfcCovering`), and the pass measures only slabs, roofs and footings (`wall_areas::JOINED_TYPES`), so a wall joined to anything else is left out. It carries none of the four, and Revit's solid is the shorter one because the ceiling cuts it.

The two Revit-exact readings (the box and the cut-back length) fail together or not at all on these walls, so the cut-back length is a confirmation, not a source: the rule still writes the box's values (`prism_quantities`), after RE-267's top cut, on walls whose joins clear the wall (RE-270 §4).

## 3. The rule

RE-270's second rule takes a wall of its pool when all of these hold: a body that is the record's box cut back at its joins, a plain rectangle; a `Length` confirmed by its record, **or a cut-back length within 1e-4 ft of the box's long side**; every element its roof and floor joins name is a slab, roof or footing drawn as a plain extrusion lying wholly above its top or wholly below its base (`wall_areas::JOINED_TYPES`: a join naming a ceiling or any other element leaves the wall out).

## 4. Measured

This change at 790f3bfe against `main` at 3bc5e21c, every model and every scorer (run [41182651693](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41182651693), `-f base=main -f nordic=true -f r2025=true`). Only `ifc.sha256`, `qtos_vs_ifc` and Core Interior's witness observations move; the other 1,465 scorer outputs are byte-identical to `main`'s.

| | office as built | at permit |
|---|---:|---:|
| walls carrying the four quantities, before | 67 | 70 |
| ... after | 96 | 106 |
| the four equal to Revit's, of those | 96 | 106 |
| quantities equal, before | 28,162 | 28,076 |
| ... after | 28,278 | 28,220 |
| different, before and after | 1 (the mullion) | 1 |
| written that Revit's export omits, before and after | 0 | 0 |

By quantity (equal, of the walls Revit writes it on), `GrossSideArea`, `NetSideArea` and `GrossVolume` 67 to 96 of 579 as built and 70 to 106 of 565 at permit, `NetVolume` 67 to 96 of 607 and 70 to 106 of 589; none different. The R2025 models stay at 775 of 1,515 quantities, RE1 Mechanical and Plumbing at 31 of 31 and 63 of 63. Core Interior's and RE1 Architecture's exports were made with base quantities off (RE-242): the 24 quantities rvt-rs now writes more on Core Interior (6 walls, four quantities each) are counted apart (2,268 to 2,292), RE1 Architecture's 86 are as they were, and Core Interior's witness observations gain 12 `IFCQUANTITYAREA` and 12 `IFCQUANTITYVOLUME`; both verdicts replay PASS, their files unchanged.

Held-out models (run 41180458307, this change at a07a850d against `main` at aa917438, scrubbed counts): the scrubbed `counts.txt` of the AA house (846 quantities, 127 equal, 0 different, 719 missing), modelo_bim (220, 7, 0, 213) and teste_export_2025 (44, 10, 0, 34) is identical to `main`'s on all three, so the rule writes nothing on them. The AA house has 23 of its 59 walls where Revit writes only a `NetVolume`; none is among the walls this rule takes. The 2 quantities rvt-rs writes there that Revit's export does not are on `main` already (the slab `Width`, from RE-295), not this change's.

## 5. What stays unwritten

- Walls whose cut-back length is not read (30 record-box walls joined to roofs as built, 25 at permit): 21 and 21 have Revit's face for the box's, and are the next to take once their length is read some other way; the others include the office's 2 walls Revit writes only a `NetVolume` on that are joined to roofs (1815832 and 3063733 as built).
- Walls with cutouts (a `WallCutoutGStep`), which RE-266 and RE-270 left out: the best test the record gives is a cut-back length that is the box's long side and as many cutout steps with `m_bCutSomething` 1 as openings rvt-rs cuts in the wall. On the walls that pass it, rvt-rs's body (the box less its openings) gives Revit's four quantities on 31 of 37 (9 of 11 as built, 22 of 26 at permit). The other 6 are not separable by anything the record holds: 5 are among the walls Revit writes only a `NetVolume` on (3012017 and 3020445 as built, 3 at permit), whose side areas a rule would write although Revit omits them, and 1 at permit is wrong. The `NetVolume` alone is right on 35 of the 37, and two walls are wrong. So the cutout walls stay out until a reading tells them apart.
- Revit writes only a `NetVolume`, with no side areas, on 28 walls as built and 24 at permit (the walls its exporter does not draw as an extrusion with an axis, `PropertyUtil.CreateWallBaseQuantities` not being reached): 5 and 9 of them have openings, 23 and 15 none, and nothing in the record tells them apart.
- Walls drawn from their centreline (`partition_wall_centreline`) and joined to roofs: the cut-back length is read on 23 as built and is Revit's face on 22 of them, but the one that Revit writes only a `NetVolume` on is among the 23, so a rule on the cut-back length alone writes values Revit omits.
- Layered walls: the longest face is Revit's `Length` times its `Height` on 2 of 107 as built, so the cut-back length says nothing of it.
- `GrossFootprintArea`: a slice of its own, on the walls Revit draws as an extrusion.

## 6. Reproduce

```bash
gh workflow run measure.yml --ref main -f ref=790f3bfee608771dd8e49a1aaecc2ea840e19728 -f base=main \
  -f nordic=true -f r2025=true -f models="nordic-office-* r2025-*" -f scorers="qtos_vs_ifc wall_quantities_vs_ifc" \
  -f probe="probe_1067_native_dump probe_re290_wall_cutbacks" -f probe_scorer="- -"
gh workflow run measure.yml --ref main -f ref=790f3bfee608771dd8e49a1aaecc2ea840e19728 -f base=main -f nordic=true -f r2025=true
```

The scorer and `probe_1067_native_dump` are `main`'s; join the probe's `wall <id> classes` lines with the scorer's `wall` lines and `probe_re290_wall_cutbacks`'s `wallcut` lines by ElementId.
