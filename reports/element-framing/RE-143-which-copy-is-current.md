# RE-143: which copy of a record is current, on Autodesk's 2021 `rac_advanced` sample

**Date:** 2026-10-01
**Issues:** #548 (kept open), #421
**Artefacts:** Autodesk's `rac_advanced` sample projects of Revit 2020, 2021 and 2022 (`research/autodesk-sample-projects.tsv`, fetched for a run, no Revit export). Measured on GitHub's hosted runners with the Measure workflow (`-f samples=true`), run 36904178406.
**Probe and tool:** `examples/probe_re135_undeclared_records.rs` with `--records`, and `tools/re/copy_vs_neighbours.py` (the comparison, from the run's artifact).
**Status:** positive for this file, and a method for others. In the 2021 file the copy in the later partition is the newer revision of the element; whether that holds for Core Interior's 571 ids is not shown.

RE-136 found that an ElementId can open a record in two partitions, and RE-139 that on Core Interior the 571 such ids that differ are different revisions of the element, boxes up to a foot apart, with no Revit export to say which is current (#548). Autodesk re-saves its sample projects in every release, so the same element is in the release before and in the release after, once each. A file with two copies has two revisions of an element, and the single copies on either side date them.

## 1. The file

The 2021 `rac_advanced` project has two partitions, `Partitions/7` (16,875 records in the leading chain) and `Partitions/8` (4,112). 4,037 ids open a record in both (RE-140). For each, the records' boxes are compared (to 0.001 ft), where both carry the element marker, which is where the box is read:

| ids with a record in both partitions | 4,037 |
|---|---:|
| records without the element marker (nothing to compare) | 380 |
| boxes equal | 2,907 |
| boxes differ | 750 |

For the 750 whose boxes differ, the one record each of the same element in the neighbouring releases (every one has a single marked record in both) is compared with the two copies:

| neighbouring release | agrees with the copy in `Partitions/7` | agrees with the copy in `Partitions/8` | both | neither |
|---|---:|---:|---:|---:|
| 2020 (the release before) | 750 | 0 | 0 | 0 |
| 2022 (the release after) | 0 | 750 | 0 | 0 |

The 2021 file holds two revisions of those 750 elements: the first partition's copy is the element as the 2020 file has it, and the second partition's is the element as the 2022 file has it. The later partition's copy is the newer one.

## 2. What this does not claim

- **One file.** The other files of the corpus with more than one partition (the 2017 `rac_basic` and `rst_basic` with 63 and 37 repeated ids, the 2019 pair with 50 and 35) repeat only records without the element marker (type and definition elements), which carry no box to compare. The 2016 `rst_basic` has two partitions and no repeated id.
- **Core Interior is not decided.** Its 571 differing copies are in `Partitions/46` and mostly `59` or `55`, and no neighbouring release of that file exists. A later partition holding the newer copy would make `59` the current one there; that is a guess from this file, not a measurement. What it would be checked against is the element's own geometry in Revit's export, which the exported bodies (built from the type's layers and the axis) do not show.
- **Which copy rvt-rs reads was not changed or examined here.** The readers' choice between two records of one id is not part of this report.
- **The direction of time is assumed from the release order**: 2022's state of those elements is later than 2020's, as an upgraded sample is edited forward. The 750 are of one model in one file.
- **Not the 2,907 equal pairs, the 380 without a marker**, or what the other fields of the two records hold.
