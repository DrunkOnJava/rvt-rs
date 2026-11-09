# The W10 re-run: the pages' numbers against the documented command

W10 of the [1.0 criteria](release-1.0-criteria.md) asks that the numbers on the README and the [supported profile](supported-profile.md) that rest on models anyone can fetch can be re-run with one documented command. [#1555](https://github.com/DrunkOnJava/rvt-rs/issues/1555) is the first time anyone ran it and compared. This page records that run, what was compared, each difference and what the command does not produce. Run it again, and keep the result here, before a release that changes a number on those pages.

## The run

```bash
gh workflow run measure.yml --ref main -f ref=df44583fd1fcc76bbec7872b763d3fc5a06a612e -f base=none \
  -f nordic=true -f r2025=true -f samples=true
```

[Measure run 42596736395](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42596736395), 2026-11-09, on `main` at `df44583f`: the command of [re-run the numbers](reference-models.md#re-run-the-numbers) as written. It succeeded, and its `same-project gates` check run read `pass: 15 samples held to their earlier saves, and the Areas on 4 saves and on the 2021 save`. The pages are those of that commit.

## What was compared

`tools/ci/measured_numbers.py` reads each page into clauses and names the model each number stands beside. The README and the supported profile hold 87 clauses with a number. Of them 52 (197 numbers) rest on a model the [model index](reference-models.md#model-index) marks `redistributable` or `fetched by Measure`. The other 35 rest on `local only` or `not shared` models, or are marked as no measurement (`<!-- w10: ... -->`); a run cannot reproduce them, and the pages already say so.

Each of the 52 was read against the scorer output of the run's `measure-head` artifact (the files under each model's directory), and counted by one of three verdicts: **equal** (the run prints the page's value for the same measure), **different** (it prints another value: both are given), **not produced** (no scorer of the command prints it: it needs a flag, a probe or a scorer that is not registered, or the clause is not a count of the run). A clause that is partly equal and partly not produced is listed as equal with its remainder.

| verdict | clauses |
|---|---:|
| equal | 29 |
| different | 3 |
| not produced by the command | 20 |

## The three differences

All three are corrected on the pages in the change that records this run.

| page, clause | the page said | the run prints | why |
|---|---|---|---|
| supported profile, the table of formats (Revit 2024 and 2025 project) | one plumbing fixture on RE1 Plumbing is not in Revit's export | none: `class_sets_vs_ifc` lists no element rvt-rs writes that Revit's export lacks, on any of the four RE1 models | RE-363 stopped writing the fixture. `evidence.md` said the same and is corrected too |
| supported profile, doors, windows, openings | Revit's overall size on all 269 windows and 266 of 269 doors of the BIM4LCA office | as built 269 of 269 windows and 268 of 269 doors; at permit 265 of 265 and 268 of 269 (`door_window_sizes_vs_ifc`); the door left is 0 by 0 in Revit's export | RE-318 |
| supported profile, rooms | the BIM4LCA office's 1 room (permit) keeps its box | `edge_outline_spaces` 1 and `bounding_box_spaces` 44 at permit (the 44 family-instance spaces); the room takes `stored_solid_edges`, its area within 0.1% of Revit's on 1 of 1 and its outline within 0.01 ft on 0 of 1 | RE-374 (#1587) |

## What the run confirms

| clause | the page's number | where the run prints it |
|---|---|---|
| README, phase rule | `rac_basic` 14 rooms in Working Drawings | `diagnostics.json`: `room_phase_fallback` 14, on the 2024 and 2025 saves |
| README and supported profile, no 2025 beam, landing or arc | 28 models (24 and RE1's 4) | the run exports 24 and 4 models, and none writes an `IfcBeam` or a tread landing; the arcs are a probe's (not produced) |
| walls | Core Interior 360 of 360; both ends on 516 of 607 (as built) and 439 of 589 (permit) | `wall_bodies_vs_ifc`, `wall_ends_list` |
| floors | Core Interior 80 of 80; the office's 63 of 63 slabs within 0.01 ft on each file | `plan_profiles_vs_ifc` |
| a floor drawn with chords on 2025 | `rac_advanced`'s floor now equals its 2024 save's | `bodies_across_saves`: the 2025 and 2024 saves hold the same volume and vertices for it |
| roofs | 21 of 22 with every vertex within 0.001 ft, on puzzbobb's 24 models | `roof_bodies_vs_ifc`, summed over the 24 |
| beams | 923 of 942 of Snowdon Towers' drawn along their line; 34 more as a box along it (17, 9, 8); the 391 of `rac_advanced` and `rst_basic` of 2025 are their 2024 saves' | `diagnostics.json` body sources; `bodies_across_saves` (37 and 354) |
| columns | 79 hollow-section columns (as built) and 55 (permit) | `column_bodies_vs_ifc` |
| pipes | 69 on RE1 Mechanical and Plumbing (6 and 63) and the office's 12, radius and ends to 0.00001 ft | `pipe_bodies_vs_ifc`, worst 0.00000 ft |
| stairs | the office's 35 flights (34 straight, 1 spiral), 21 landings at 100.4% to 104.7% of Revit's volume; `rac_advanced`'s 5 tread landings; every flight of puzzbobb's models 13 to 16 | `stair_bodies_vs_ifc`, `predefined_types_vs_ifc`, `diagnostics.json` |
| openings | host Revit's on 138 of 138 (Core Interior) and 463 (the office as built); 138 of 138 and 4 of 4 within 0.001 ft of Revit's (the scorer's finest bin is 0.01 ft: 138 and 4, largest difference 0.00 ft) | `opening_hosts_vs_ifc`, `openings_vs_ifc` |
| door and window sizes | 137 of 137 on Core Interior and RE1; 138 of 138 on Core Interior | `door_window_sizes_vs_ifc` (132 and 5 doors, 6 windows) |
| rooms | 116 and 11 (Core Interior, RE1 Architecture); 220 of the office's rooms read by the exact walk as built and 221 of 222 at permit; 9 of 9 `stored_solid_faces`; 10 of 15 `stored_solid_approximate`; 30 of 30 area spaces on each file | `space_profiles_vs_ifc`, `space_overlaps_vs_ifc` |
| Areas | 25 Areas and 25 bodies of `rac_advanced` on a save | `areas_across_saves_2021.txt`; the gates' verdict above |
| saved meshes | the office's 265 of 269 doors and 269 of 269 windows within 0.001 ft; all 269 doors drawn from their saved meshes; Core Interior's 138 | `mesh_bodies_vs_ifc`, `saved_meshes_vs_ifc` |

## What the command does not produce

| clauses | why, and the dispatch that does |
|---|---|
| BIMcollab's 91 beams (README, supported profile) | the model needs `-f bimcollab=true` |
| Vitoria's `ALD26 Before`: 1,314 of 1,321 elements written by ElementId, 1,251 on Revit's storey, 102 conduits and 74 fittings | the models need `-f holdout=true` |
| wall tops Revit's on 64 of 65 (as built) and 70 of 70 (permit), and over the whole plan on 583 of 607 and 571 of 589 | `wall_top_cover_vs_ifc.py`, which is not registered: `-f scorers=wall_top_cover_vs_ifc` (RE-320). The run's `wall_heights_vs_ifc` prints another measure, the top and base Revit's on 606 of 607 (as built) and 589 of 589 (permit) |
| Core Interior's 256 columns exact | no scorer of the command prints it |
| the beams' saved meshes (25, 16, 9) and the 9 on a model axis | a probe (RE-328) |
| no arc in a floor, ceiling or roof of 25 models | a probe (RE-345) |
| the doors and windows drawn from every saved mesh (Core Interior 138 and RE1 228 within 0.01 ft; the office 1,554 of 1,650 and 1,638 of 1,738, RE1 148 of 228); the bodies replaced and the arcs scored | `-f ifc_flags=--saved-meshes` and `--family-extrusions` (RE-331) |
| the family corpus and the 6,172 rebuilt extrusions | `-f families=true`, and a probe (RE-327) |
| 6 of 53 exports move; the default export identical on 34 of 34 and 111 of 111 | a comparison with another commit: `-f base=main` |
| Core Interior's 17 MB partition against an 8 MiB limit | the limits are tested under `tests/` |
| a Revit 2018 export that a user posted, Snowdon Towers (local only) and the Areas' `m_area` tolerance | local only, or a tolerance and not a count |

## Doing it again

Dispatch the command above at the commit to be released, read each of the 52 clauses against the run's scorer files (the first table lists where), correct a page that differs in the same change, and add the new run to this page. A number that moves between two such runs without a change that names it is a regression until shown otherwise.
