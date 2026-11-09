# RE-378: a profile-edited wall ends at its profile's highest point, below its record box

**Date:** 2026-11-09
**Issues:** #1600 (split off #1067's last Measure by #1596, RE-375); follows RE-151 (the openings an edited elevation profile cuts) and RE-358 (the profile's line carried on past its end)
**Artefacts:** the BIM4LCA office's terrain files at building permit, concrete (`ARK_NordicLCA_Office_Terrain_Concrete_BuildingPermit.rvt`, 7,634,944 bytes, sha256 `12477111d81ffca78d87a22a64d5e1f69ff1f8d027148c749e7c2d570087ecad`) and timber (`ARK_NordicLCA_Office_Terrain_Timber_BuildingPermit.rvt`, 7,749,632 bytes, sha256 `838118fd85da2451e47448dd4dae1b1dac9b3a74dbaedfbae4afce9ca915c57c`), Revit 2024, each with Revit's own IFC4 export (`research/bim4lca-measure.tsv`; VTT's ARCH.zip, CC BY-SA 4.0)
**Probe:** `probes/examples/probe_1067_native_dump.rs`
**Scorers:** `tools/re/wall_heights_vs_ifc.py`, `tools/re/qtos_vs_ifc.py`, `tools/re/unfilled_openings_vs_ifc.py`
**Status:** positive on one wall, in two files. **Open:** how Revit's export forms the wall, and the one unfilled opening it writes on each file (section 5).

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

To be filled with the Measure's results: the filtered run [42614195315](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42614195315) on the two terrain files, and an unfiltered run of the final head.

## 5. Not shown

- **How Revit's export forms this wall.** The scorer reports a Tessellation body whose highest point is the profile's; whether Revit writes it with an opening, and which wall the one unfilled opening of each file voids, is not measured. rvt-rs writes two (Tags 1351630 and 1389150) and the rule leaves both in place.
- **Any other wall.** No other wall of any measured model is changed by the rule, and none is known to need it. A profile that is a plain rectangle shorter than the wall's unconnected height produces no cut and is not read.
- **Revit 2025 and later**, where no model has been measured with such a wall.
