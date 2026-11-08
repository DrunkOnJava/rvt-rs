# RE-364: Revit 2015 to 2023 walls along their lines, joined by their join lists

**Date:** 2026-11-06
**Issues:** #1190 (geometry beyond the box on 2015 to 2023, 2026 and 2027); follows RE-49 and RE-53 (a wall's line and layers), RE-70 (join lists), RE-114 and RE-120 (2023 walls keep their box, cut at their joins), RE-180 and RE-181 (no cross-section word before 2021), RE-280 (native records on 2015 to 2022).
**Artefacts:** Autodesk's own sample projects, pinned in `research/autodesk-sample-projects.tsv` and fetched by `tools/fetch-autodesk-samples.sh`, never stored.
**Scorer:** `tools/re/save_pair_bodies.py`, with its new `--no-openings`; `probes/examples/probe_re364_wall_join_lists_32.rs`.
**Status:** positive against the same samples saved by a later release. **No wall of a 2015 to 2023 file is scored here against a Revit export**, and a later save cannot catch an error it shares. RE-120's measurement, every wall of two 2023 projects as Revit's, was of the cut-back box, which a wall along a model axis keeps on the narrowed head described next. The `racadv-*` pairs and BIMcollab's Revit 2015 example have such exports; they are for Measure, which reports counts only, and none was opened for this work.


**Narrowed in review, 2026-11-08 (the maintainer).** Measure run 42516794247 on the first head scored this rule against Revit's own exports and found two problems the later saves share. On Warmond (Revit 2021, held out, counts only), the wall quantities it added were 91 equal against 45 different for `Height`, 0 against 45 for `NetSideArea`, 22 against 23 for `NetVolume` and 26 against 11 for `GrossFootprintArea`. On Vitoria `ALD26 Before` (Revit 2018), axis-parallel walls with both ends within 0.001 ft of Revit's fell from 160 to 125 of 199, while angled walls' faces rose from 1 to 33 of 48. So on Revit 2015 to 2023: a wall along a model axis keeps its record box cut back at its joins, an angled wall is drawn along its line, a wall's `Height`, areas and volumes are not written (the record is read without its control joins, so what reads as uncut is not known to be), and on the releases with no cross-section word an angled body must be the rectangle its box closes on. The figures in the sections below were measured before this narrowing, on the contributor's first head.

**What the narrowed head does, measured on `430a2606`** (Measure run 42522812941, main's side `ab5f8e1c`, unfiltered with samples, held-out models, `racadv` and BIMcollab; the code is that of the landing head, which adds only main's commits and text). 100 of 118 exports are byte for byte main's and 18 move, and `same-project gates` passes.

- Warmond's wall quantities are main's: equal 272, different 5, missing 3,135, values Revit does not write 5.
- On Vitoria `ALD26 Before`, axis-parallel walls return to 160 of 199 with both ends within 0.001 ft. **Its angled walls stay as on main**, faces 1 of 48 and ends 4 of 48, and none takes a line body. The first head's 33 of 48 is not kept: 2018 has no cross-section word, and the check that a body must be the rectangle its box closes on refuses the joint-to-joint fallback those walls took. Between the two heads it is the only change that touches an angled wall's body. No value read on 2015 to 2020 marks a slanted wall, so the check stays.
- A line body reaches 24 walls in 12 sample exports (`rac_basic` of 2015 to 2023, `rst_basic` of 2021 to 2023), and none on Vitoria `ALD26 Before`, `modelo_bim`, `Exemplo_data` or the `racadv` pairs. **So no Revit export read here scores this rule.** Five other held-out exports move (Warmond, the Vitoria project, Frigento, Gdynia and bim-whale), layer sets on three of them, and are not attributed: they keep no count of body sources, and Measure run 42525743981 (`wall_bodies_vs_ifc` on the five) left no `wall bodies:` line for them, so whether any of their walls takes a line body is not measured.
- The `racadv` pairs of 2015 to 2020 are byte for byte main's. So are RE-120's two projects, in Measure run 42525247402 (head `9bef6e08`, main's side `60cb1a81`, `wall_bodies_vs_ifc`, counts only): `Exemplo_data` 17 of 17 and `modelo_bim` 4 of 4 on faces and on ends within 0.001 ft on both sides, so RE-120's scores stand.
- Layer sets gain on three held-out models (counts only): Frigento 10 to 12, all 12 whole; Gdynia 108 to 109, 107 of 108 whole; bim-whale 43 to 302, 300 of 300 whole.

## 1. Before

A wall on 2015 to 2023 kept its record box, cut back at its joins (RE-120), because `attach_wall_layers` turned centreline bodies off for every 32-bit release. RE-114's reason was that "2023's join lists are not read, and a line without them would lose the corners the box already has". On `main`, `rac_advanced`'s walls of `exported.body_sources`:

| | 2023 | 2024 |
|---|---:|---:|
| `partition_wall_centreline_layer_joins` | 0 | 118 |
| `partition_wall_centreline` | 0 | 1 |
| `partition_element_record_join_trimmed` | 176 | 58 |

## 2. The join list's 32-bit layout

`scan_wall_join_partners` failed closed on these releases. The list is there. Wall 148769 of `rac_advanced`, whose partner is `0x0235c5`:

```
2024  07000000 7d9f0b00 02000000 | count 01000000 | ac000000 c5350200 00000000 05000000
2023  07000000 7d9f0b00 02000000 07000000 + 13 zero bytes | count 01000000 | ac000000 c5350200 05000000
```

The count sits 29 bytes past the opening word, and an entry is `u32 · u32 ElementId · u32`, 12 bytes. The tag is the same on 2015 to 2024. What the `u32` at +12 holds is not decoded; the reader requires the 13 bytes after it to be zero.

Every wall's partners against its 2024 save, by `probe_re364_wall_join_lists_32`: `rac_advanced` 146 of 146 on each of its nine saves of 2015 to 2023, 57 of them naming a partner, and `rst_basic` 9 of 9 on 2016 to 2023. `rac_basic` is 46 of 47 on 2016 to 2023, where wall 493879 names no partner and its 2024 save names 627729, and 45 of 47 on 2015, where wall 849032 also names 977133, which its 2024 save does not. `rst_basic` 2015 is another model (RE-181).

## 3. The change

- `partition_compound_structure`: `wall_join_lists_32`, `WALL_JOIN_32BIT_REVIT_VERSIONS` (2015 to 2023). `wall_join_count_offset` and the 64-bit reader are unchanged.
- `attach_wall_layers`: `WALL_CENTRELINE_REVIT_VERSIONS` (2015 to 2027) replaces the 32-bit test. 2008 to 2014 stay as they were.
- 2015 to 2020 have no cross-section word (RE-180, RE-181), so nothing marks a slanted wall there and each is drawn vertical. That is not proved separately: the walls these samples hold are the ones scored below.
- `native_walls_and_top_joins_into` names `NATIVE_RELEASES`, as RE-280's readers do, so 2015 to 2022 take the facts 2023 takes.
- The export's warning says so on 2015 to 2023.

## 4. Bodies against a later save

Each wall the change draws along its line, paired by `Tag`, to 3 mm on the world box and 0.1% on the volume. "No openings" meshes the wall without its voids; door and window cuts are read on 2024 and not on these releases, so "with cuts" also counts that gap, which this change does not touch.

| save | reference | no openings | with cuts |
|---|---|---:|---:|
| `rac_advanced` 2015 to 2020 (each) | 2024 | 107 of 118 | 73 of 118 |
| `rac_advanced` 2021 | 2024 | 99 of 103 | 70 of 103 |
| `rac_advanced` 2022, 2023 (each) | 2024 | 118 of 118 | 80 of 118 |
| `rst_basic` 2016 to 2023 (each) | 2024 | 2 of 2 | 2 of 2 |
| `rac_basic` 2015 to 2022 (each) | 2023 | 17 of 22 | 17 of 22 |
| `rac_basic` 2023 | 2024 | 5 of 18 | 5 of 18 |
| `rme_basic`, `rme_advanced` 2015, 2019 (each) | 2024 | 1 of 1 | 1 of 1 |

`rst_advanced` 2015 and 2019, and `rst_basic` 2015, have no wall drawn along its line by this change.

On `main` the same walls of `rac_advanced` 2023 match on 1 of 118 with no openings, and those of `rst_basic` and `rac_basic` on none.

**The 11 on `rac_advanced` 2015 to 2020** are within 8 mm on the box and 0.21% on the volume. The saves' raw boxes differ from 2024's on those walls too; why is not decoded.

**The 4 on `rac_advanced` 2021** are 15 to 31 mm off, and their raw boxes differ as well. That save also draws 103 walls along their lines, not 118; the other 15 keep their cut-back box, and why is not decoded.

**`rac_basic` is scored against its 2023 save.** 13 of its walls move by up to 3.19 m in the 2024 save, in the raw boxes (`main`'s export) as much as here. Of the 5 that differ from the 2023 save, up to 0.18 m on the box and 2.64% on the volume, 4 are ones that save leaves as cut-back boxes; the fifth, 906885, is drawn along its line on both and is 13 mm off, which is not decoded.

**Two walls end farther from the later save's box than on `main`**: 160150 and 160284 of `rac_advanced` 2015 to 2020, 8 mm where `main` has 6 mm. No wall that matched on `main` stops matching.

## 5. What else changes

Across the 31 sample saves of 2015 to 2023 that have a reference:

- **Nothing outside walls.** No other element changes a property, a storey or a GlobalId, and no element appears or goes.
- **Thickness and materials.** On `rac_advanced` 2023, `Thickness` goes from 84 to 93 of the 93 changed walls that equal 2024's, and the 8 parapets take the layer set usage 2024 writes. `main` gives `rac_basic`'s wall 493790 a thickness of 9.75 where 2024 gives 0.28.
- **Quantities gained.** `Height` on 784 walls, 781 equal to 2024's and 3 not in it. `GrossSideArea` and `NetSideArea` on 481, 462 equal and 19 where 2024 writes none. `GrossFootprintArea`, `GrossVolume` and `NetVolume` the same, except on wall 418079 of `rac_basic` 2015, where `GrossVolume` is 2.42 and 2024's 1.75; that save's own box differs, and why is not decoded. No quantity or property is lost or changes value.
- **One storey.** `rst_basic`'s wall 627729, 2016 to 2023, moves from a storey made from its base elevation to `Default Building`, as the 2024 save has it. No Revit export says which is right.
- **`Length` is left out.** A 2015 to 2023 record is read without its control joins (`joins`, `side_joins`, `exposed`, also `shells` and `type_id`, all empty on wall 143858 of 2023, all present on 2024). `attach_wall_lengths` takes an end with none as joined to nothing, and wrote a `Length` on 44 walls of `rac_advanced` 2023 where 2024 writes none, so it does not run on these releases.
- **Byte identity.** Of 206 local project files of 2008 to 2027, 170 export IFC and diagnostics byte for byte as on `main` (`SOURCE_DATE_EPOCH=1700000000`). The other 36 are files of 2017 to 2023: 32 change their IFC, and 4 only the warning in their diagnostics. No file of 2024 or later, or before 2015, changes.
- **Cost.** No difference above the noise of single runs on this machine, where one export varied between 3 and 6 s.

## 6. Not measured, not read

- Against a Revit export: see the status above.
- RE-127's joined-wall lists: two walls of `rac_advanced` hold the pattern, with count 0 on 2024, so there is nothing to measure the layout against and it is not read. The L joints those lists decide take what the join lists give.
- A record's control joins on 2015 to 2023, and with them `Length`: why the graph holds none is not decoded (#1158).
- A wall's profile openings and end steps (each its own list, 2024), its type's face angles (2024, 2025).
- Door and window cuts.

## 7. Reproduce

```bash
tools/fetch-autodesk-samples.sh samples 2015 2024
for f in samples/*.rvt; do rvt-ifc "$f" -o "${f%.rvt}.ifc"; done
S=partition_wall_centreline,partition_wall_centreline_layer_joins
python3 tools/re/save_pair_bodies.py samples/2023-racadvancedsampleproject.ifc samples/2024-racadvancedsampleproject.ifc --sources $S --no-openings
# Section 2: the wall ids are the `Tag`s of the 2024 export's IfcWalls, one file for both saves.
cargo run --profile ci -p rvt-probes --example probe_re364_wall_join_lists_32 -- samples/2023-racadvancedsampleproject.rvt 2023 walls.txt > 2023.txt
cargo run --profile ci -p rvt-probes --example probe_re364_wall_join_lists_32 -- samples/2024-racadvancedsampleproject.rvt 2024 walls.txt > 2024.txt
diff 2023.txt 2024.txt
```
