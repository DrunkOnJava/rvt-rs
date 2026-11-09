# RE-378: a profile-edited wall ends at its profile's highest point, below its record box

**Date:** 2026-11-09
**Issues:** #1600 (split off #1067's last Measure by #1596, RE-375); follows RE-151 (the openings an edited elevation profile cuts) and RE-358 (the profile's line carried on past its end)
**Artefacts:** the BIM4LCA office's terrain files at building permit, concrete (`ARK_NordicLCA_Office_Terrain_Concrete_BuildingPermit.rvt`, 7,634,944 bytes, sha256 `12477111d81ffca78d87a22a64d5e1f69ff1f8d027148c749e7c2d570087ecad`) and timber (`ARK_NordicLCA_Office_Terrain_Timber_BuildingPermit.rvt`, 7,749,632 bytes, sha256 `838118fd85da2451e47448dd4dae1b1dac9b3a74dbaedfbae4afce9ca915c57c`), Revit 2024, each with Revit's own IFC4 export (`research/bim4lca-measure.tsv`; VTT's ARCH.zip, CC BY-SA 4.0)
**Probe:** `probes/examples/probe_1067_native_dump.rs`
**Scorers:** `tools/re/wall_heights_vs_ifc.py`, `tools/re/qtos_vs_ifc.py`, `tools/re/unfilled_openings_vs_ifc.py`
**Status:** positive on one wall, in two files. **Open:** the unfilled opening Revit writes on that wall and the two rvt-rs writes on the terrain files in its place (section 5, #1607).

## 1. What the wall is

Each terrain file holds two walls. The last Measure of #1067's work (run 42598250689) scored one of them, 1387731, as the only wall whose top is not Revit's: rvt-rs's highest point is 200.1988 ft and Revit's 196.5223 ft, 3.6765 ft lower, in the same storey and with the same base (182.0866 ft), and Revit's body is a Tessellation. Its `Qto_WallBaseQuantities.Height` is the one `different` of the files' two Heights: rvt-rs writes 5.520605 m and Revit 4.4 m (14.4357 ft). No floor, roof or footing in either file overlaps its plan box, so the top pass of RE-267 and RE-320 had no cutter to read.

The probe (Measure run 42608767799, `main` at `b969296e`; the wall records are the same in both files) shows what the wall is. Its record holds no top join, no cut pair and no cutout step. Its `GLine`s are a closed elevation profile of seven lines in the wall's vertical plane over the whole of its length, x 122.4038 to 296.2603 ft: the base at 182.0866 ft, a right end up to 183.0898 ft, a left end up to 196.5223 ft, and the top between them, 196.5223 ft at x 122.4038, 195.6988 at x 138.808, 192.4075 at x 171.6386, level to x 215.944, and then down to 183.0898 at x 296.2603. The wall's saved Unconnected Height (-1001101) is 18.1122 ft, so its record box tops out at 200.1988 ft. The profile's highest point, 196.5223 ft, is Revit's top.

The other wall, 1351524, has a profile of the same kind whose highest point is its box's top (193.2415 ft), which is why its top reads as Revit's.

## 2. What rvt-rs drew

A wall with an edited profile is read as its record box less openings (RE-151): `elevation_profile_cuts` takes each run of the profile's lines that leaves the loop's bounding rectangle and voids the region between the run and the rectangle's edge. The rectangle is the loop's own, and here its top is 196.5223 ft, so the 3.6765 ft above it, up to the box's top, stayed solid over the whole wall, and the wall's `Height`, the extrusion's, was the box's. rvt-rs writes an opening for each wall (Tag 1351630 in 1351524 and Tag 1389150 in 1387731), where Revit's export has one unfilled opening in each file and it is neither of those (matched by Tag and host).

On Revit's side the profile is the wall: a wall's body is the region its sketch encloses, not its rectangle less what the sketch leaves out, and Unconnected Height is a parameter the edit leaves as it was.

## 3. The rule

`wall_profile::take_top_at_profile` lowers the extrusion of a wall to the profile's highest point, before the quantities that take the body's height, and marks the wall with `ProfileTopSource` (`wall_top_at_profile_top`), counted in `exported.wall_profile_top_sources`. All of these must hold, and the rule rests on the one wall measured, so each is the narrowest that wall allows:

- Revit 2024, where a profile's cuts are read and where `m_profile_extent` and `m_profile_height` are kept with the wall's openings;
- the wall is a straight wall whose record names nothing joined to or cutting it (`m_wall_uncut`), so that no other pass decides its top or base, and it is one plain extrusion with a record property set;
- the profile spans the wall: its lowest and highest points along its own axis lie at the wall's two ends, within a thousandth of a foot. A wall that goes on past its profile has a box that holds the profile's line carried on, and its top is not the profile's: `rac_basic`'s wall 493612 is 3.529048 m high for a profile of 3.5 m, the 0.029048 m being the line's slope over the 0.14 m past the profile's end (RE-358). That wall, and the eight office walls that RE-358 carries on, go on past their profiles, so the rule leaves them as they are;
- the profile's lowest line is the wall's base, within a thousandth of a foot;
- the profile's highest point lies more than a thousandth of a foot below the top of the box.

A wall of which any of this is not known keeps the box's top, as before, and is not counted. The pure part, `top_at_profile`, has unit tests of its failure modes next to it.

## 4. Result

Measure run [42614195315](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42614195315) (the rule at `8a5b2225` against `main` at `b969296e`, the two terrain files only, `wall_heights_vs_ifc`, `qtos_vs_ifc`, `wall_quantities_vs_ifc`, `unfilled_openings_vs_ifc` and `wall_bodies_vs_ifc`), on each of the two files:

| | before | now |
|---|---:|---:|
| walls whose base and top are Revit's (`wall_heights_vs_ifc`) | 1 of 2 | 2 of 2 |
| wall 1387731's highest point | 200.1988 ft | 196.5223 ft, Revit's |
| `Qto_WallBaseQuantities.Height`, equal / different (`qtos_vs_ifc`) | 1 / 1 | 2 / 0 |
| walls whose body volume equals Revit's (`wall_quantities_vs_ifc`, NetVolume and GrossVolume candidates) | 1 of 2 | 2 of 2 |
| walls whose side areas and footprint equal Revit's (the same scorer) | 1 of 2 | 2 of 2 |

Wall 1387731's body volume is now Revit's, 755.389 ft³, and its `Height` is 14.4357 ft, Revit's 4.4 m. That is the whole shape, not only the highest point: the profile's openings cut the region under the top as before, and the strip above the profile is gone. In `qtos_vs_ifc` the quantities equal rise from 15 to 16 and the different fall by one (from 1 to 0 on the concrete file, from 2 to 1 on the timber file, whose remaining different value is a space's `NetVolume`, which does not move). `unfilled_openings_vs_ifc` and `wall_bodies_vs_ifc` do not move, so the two openings rvt-rs writes are as before. `diagnostics.json` differs only by `exported.wall_profile_top_sources`, `wall_top_at_profile_top` 1 on each file, and `wall rules: profile top sources` says the same.

The unfiltered run, Measure run [42614629885](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42614629885) (the rule at `8a5b2225` against `main` at `b969296e`, with `nordic`, `r2025`, `samples`, `racadv`, `bimcollab` and `holdout` on and every scorer), covers 118 models. Only the two terrain exports moved (`ifc.sha256`); no other model's IFC differs by a byte, in any year of the samples. `diagnostics.json` differs on all 93 models that keep one by `exported.wall_profile_top_sources` alone (`{}` on 91, `wall_top_at_profile_top` 1 on the two terrain files), their log by the diagnostics' size and the run's milliseconds, and `mcp-dump.json` on 34 models by the same key (on the terrain files also by wall 1387731's `ProfileTopSource` and its `Height`, 5.520605 to 4.4). The other 25 models, the held-out, BIMcollab and `rac_advanced` ones, keep a scrubbed `counts.txt`, identical on both sides, with `wall rules: profile top sources none`. The scorers differ on the two terrain files only (`wall_heights_vs_ifc`, `qtos_vs_ifc`, `mcp_answers_vs_ifc` and `wall_rules`); the MCP answers' quantity values different from Revit's fall from 1 to 0 on each. `same-project gates` passed on the measured commit (15 samples held to their earlier saves, and the Areas on 4 saves and on the 2021 save), and the advisory `semantic diff` lists the four changes of the two files, the marker twice, its count and `Height`, and nothing else.

What the run exercises. Revit's own export of a Revit 2024 file scores every one of its walls' tops, so a wall that the rule's conditions left out and that stood below its box would show as a miss. The five Revit 2024 files with Revit's export hold 1,560 walls (Core Interior 360, the office as built 607 and at permit 589, the terrain files 2 each) and `wall_heights_vs_ifc` gives base and top Revit's on all 1,560 (1,558 before). The profile-edited walls whose profile line RE-358 carries on past its end, 19 in four models (the two offices, `rac_basic` 2024 and Snowdon's architectural sample), go on past their profiles and the rule leaves every one as it was. The rule fires on one wall, in two files.

## 5. Not shown

- **The openings.** Revit's export writes one unfilled opening of no cutter on each terrain file, Tag 1389018 on wall 1387731: the first three lines of the wall's top closed by a straight line back to the first, x 122.404 to 215.944 ft and height 192.408 to 196.522 ft, above the body and outside it. rvt-rs writes two others, Tag 1389150 on the same wall (all four lines of the top, closed by the rectangle's corner) and Tag 1351630 on wall 1351524, on which Revit's export writes none; the rule leaves both in place. In the office files rvt-rs also writes 8 openings that Revit's export does not, on RE-358's eight walls, and RE-151's rule matches Revit on one wall only, Core Interior's 55840. Why Revit's opening takes three of the four lines is not shown ([#1607](https://github.com/DrunkOnJava/rvt-rs/issues/1607)).
- **Any other wall.** No other wall of any measured model is changed by the rule, and none is known to need it. A profile that is a plain rectangle shorter than the wall's unconnected height produces no cut and is not read.
- **Revit 2025 and later**, where no model has been measured with such a wall.

## 6. Reproduce

```bash
# The two terrain files only (section 4's table); base is the commit main was at when the rule was cut:
gh workflow run measure.yml --ref main -f ref=8a5b22257522a38d426d3449fa5b46aa9486efab \
    -f base=b969296e46fd86e712b92a6c93cc1791b2d3cc1b -f nordic=true -f models="nordic-office-terrain-*" \
    -f scorers="wall_heights_vs_ifc qtos_vs_ifc wall_quantities_vs_ifc unfilled_openings_vs_ifc wall_bodies_vs_ifc"
# Every model, the held-out ones included (section 4's unfiltered run):
gh workflow run measure.yml --ref main -f ref=8a5b22257522a38d426d3449fa5b46aa9486efab \
    -f base=b969296e46fd86e712b92a6c93cc1791b2d3cc1b -f nordic=true -f r2025=true -f samples=true \
    -f racadv=true -f bimcollab=true -f holdout=true
```

`base=main` is `main` on the day it runs, so a later run also shows what `main` has changed since `b969296e`; the commit names the same side. The measured commit is archived under `refs/archive/2026-11-09/cited/8a5b22257522` because this pull request is squashed. The probe of section 1 is `probe_1067_native_dump` with `-f models="nordic-office-terrain-*"`.
