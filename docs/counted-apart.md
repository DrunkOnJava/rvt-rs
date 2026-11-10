# What the censuses count apart

W7's two censuses compare Revit's own export of a model with rvt-rs's: `tools/re/qtos_vs_ifc.py` for quantities and `tools/re/psets_vs_ifc.py` for property values. Each value Revit's export writes on an object rvt-rs also writes is scored equal, different or missing, and each value rvt-rs writes that Revit's export does not is counted as one it does not write. Those totals are what the [1.0 criteria](release-1.0-criteria.md) quote ("different 1", "none written that Revit's export does not").

A value the censuses cannot honestly score is **counted apart**: it is in none of equal, different, missing or written-that-Revit-does-not. The comparison it would make is not one (a wall whose export names another type than its saved file, a key several objects of Revit's export hold), or Revit's export is not one it can be matched against (made with base quantities or property sets off, an IFC2X3 export), or Revit's own value is known to be wrong. Each category below was decided by name, and each is a limit on what "different 0" can say, so this page lists all of them, with who decided it and why, and how many values sit in each on every model group of a Measure run.

A value rvt-rs leaves out on purpose (a type object's `IsExternal`, a value Revit's exporters disagree on) is scored as missing, not counted apart; the criteria page names those.

## The categories

<!-- counted-apart:categories -->
| Category | What is counted apart, and why | Decided | Scorer |
|---|---|---|---|
| `qto-setup-off`: Base quantities left off in Revit's export | Revit's export of the model was made with "Export base quantities" off, so it holds no base quantity set to score against. The quantities rvt-rs writes beside it are counted (`rvt-ifc --no-base-quantities` leaves them out). | RE-242; base quantities on by default, as in Revit's built-in IFC4 setups | `qtos_vs_ifc` |
| `qto-known-wrong`: Quantities Revit writes that are known to be wrong | Mullions' `CrossSectionArea`, door openings' `Width` and `Height` (which Revit swaps) and a floor's `Perimeter` where its outline is read as feet. Revit's export writes them and rvt-rs leaves them out, so they are scored as neither missing nor different. | RE-235 §5 | `qtos_vs_ifc` |
| `qto-openings-unwritten`: Quantities on openings rvt-rs does not write | The openings a door or a wall cuts in a further host, and a curtain wall's or a cabinet's cut, which Revit's export writes and rvt-rs does not (RE-338 counts them by reason). | #1165; RE-338 | `qtos_vs_ifc` |
| `wall-two-lengths`: A wall's Length, where Revit's export may write either of two | Revit's export writes a wall's `Length` as its curve's length or as that of its curve cut back at its joins, by how it draws the wall. Where the two differ nothing in the model says which, so a missing `Length` there is counted apart. | the maintainer, 2026-10-26; RE-252 | `qtos_vs_ifc` |
| `wall-width-other-type`: A wall's Width, where the two exports name different types | Revit's export of a held-out model was made from a state its saved file does not hold (#1048). Where a wall's own record confirms the type rvt-rs gives it and Revit's export names another, the differing `Width` is counted apart, not scored as different. | the maintainer, 2026-10-27; RE-253, #1048 | `qtos_vs_ifc` |
| `wall-quantities-other-type`: A wall's volumes and side areas, where the two exports name different types | The same wall's `NetVolume`, `GrossVolume`, `NetSideArea` and `GrossSideArea`: rvt-rs takes them from the mesh Revit saved and the saved type's `Width`, so they follow the saved state, while Revit's export writes them from the state it was made from. | the maintainer, 2026-11-09; RE-253, RE-388 | `qtos_vs_ifc` |
| `qto-own-unit`: Quantities with a unit of their own | A quantity that carries a unit of its own, rather than the file's unit assignment, is not scored. | the census's definition (tools/re/qtos_vs_ifc.py) | `qtos_vs_ifc` |
| `qto-repeated-key`: Quantities on a key several objects of Revit's export hold | A wall Revit's export splits at its storeys keeps its ElementId as each piece's Tag, and two spaces can share a Name, which is how spaces are matched. rvt-rs's one value for the whole element is not a piece's, so Revit's values on such a key are scored as neither equal, different nor missing (the figure is Revit's values). | #1576 | `qtos_vs_ifc` |
| `pset-repeated-key`: Property values on a key several objects hold, which differ | The same keys as above: where the objects of Revit's export hold different values, none stands for the key, so those values are counted apart. Where they agree the value is scored against the object held. | #1576 | `psets_vs_ifc` |
| `pset-sibling-openings`: Openings of a pair of exports that disagree | An unfilled opening rvt-rs writes that Revit's export of this file lacks and its export of a sibling file (the same building in another state) holds is a pair inconsistency, counted apart and never scored as a match. | the maintainer, 2026-11-03 and 2026-11-04; RE-321 §4, RE-338 | `psets_vs_ifc` |
| `pset-known-wrong`: Property values Revit writes that are known to be wrong | The stair `WalkingLineOffset` values and the others listed in RE-235 §5: Revit's export writes them and rvt-rs leaves them out. | RE-235 §5 | `psets_vs_ifc` |
| `pset-cache-defects`: Curtain members' IsExternal in an export made from the exporter's own cache | The exporter that made the BIM4LCA office's exports (24.0.5.432) sets a curtain mullion's and panel's `IsExternal`, and their types', from its cache of element types, not from the model. rvt-rs writes the members' by their curtain wall's value, as later exporters do, and none on the types. | the maintainer, 2026-10-30; RE-269 | `psets_vs_ifc` |
| `pset-property-set-file`: Values the export's own property set file places | Where the export's user-defined property set file maps a value into a set (glazing on walls, occupancy on rooms, the `FI_` sets), the model does not store the file, so the value is out of scope. | the maintainer, 2026-10-26; RE-239 | `psets_vs_ifc` |
| `pset-openings-unwritten`: Property values on openings rvt-rs does not write | The same openings as for quantities: those a door or a wall cuts in a further host, and a curtain wall's or a cabinet's cut. | RE-338 | `psets_vs_ifc` |
| `pset-no-standard-sets`: Revit's export written with no standard property set | Core Interior's export was made with no property sets, so the values rvt-rs writes beside it cannot be scored against it. | the maintainer, 2026-10-29; #1115, RE-239 §5 | `psets_vs_ifc` |
| `pset-beyond-ifc2x3`: IFC4 properties an IFC2X3 export cannot write | IFC2X3's definitions of a set lack an IFC4 property rvt-rs writes, so Revit's IFC2X3 export cannot hold it. | the maintainer, 2026-10-29; #1118 | `psets_vs_ifc` |
| `ifc2x3-lengths-as-quantities`: IFC2X3 lengths that IFC4 holds as quantities | Revit's IFC2X3 export writes a pipe's or duct's length in a property set; the quantity census scores it as the IFC4 base quantity, so the property census leaves it out. | the census's IFC2X3 rules (tools/re/psets_vs_ifc.py) | `psets_vs_ifc` |
| `ifc2x3-values-twice`: IFC2X3 values scored in an IFC4 set already | An IFC2X3 value that also sits in an IFC4 set is scored there, not twice. | the census's IFC2X3 rules (tools/re/psets_vs_ifc.py) | `psets_vs_ifc` |
| `ifc2x3-own-set`: IFC2X3 values no IFC4 set holds | A value Revit's IFC2X3 export writes that no IFC4 set of its export holds, which rvt-rs writes in its own set, cannot be matched. | the census's IFC2X3 rules (tools/re/psets_vs_ifc.py) | `psets_vs_ifc` |
<!-- /counted-apart:categories -->

## Counts in a Measure run

<!-- counted-apart:counts -->
Counts from Measure run [42864169952](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42864169952) on `376035fb6a907b15e850ac91e8d725b0542512f0`, the quantity and property census of each model summed over its group (`-`: the scorer did not run on that group; `not kept`: the held-out count files keep no line of that category).

| Category | BIM4LCA office, as built | BIM4LCA office, at permit | office terrain pair | Core Interior | Einhoven | RE1 | Revit 2025 models | IFC2X3 files (counts only) | held-out models (counts only) |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| `qto-setup-off` | 0 | 0 | 0 | 3,124 | - | 251 | 0 | not kept | not kept |
| `qto-known-wrong` | 10,390 | 10,418 | 0 | 0 | - | 0 | 12 | not kept | not kept |
| `qto-openings-unwritten` | 188 | 217 | 0 | 0 | - | 0 | 0 | not kept | not kept |
| `wall-two-lengths` | 463 | 388 | 0 | 0 | - | 0 | 82 | 0 | 83 |
| `wall-width-other-type` | 0 | 0 | 0 | 0 | - | 0 | 0 | 0 | 1 |
| `wall-quantities-other-type` | 0 | 0 | 0 | 0 | - | 0 | 0 | 0 | 2 |
| `qto-own-unit` | 0 | 0 | 0 | 0 | - | 0 | 0 | not kept | not kept |
| `qto-repeated-key` | 10 | 10 | 20 | 0 | - | 0 | 104 | 0 | 587 |
| `pset-repeated-key` | 12 | 12 | 0 | 0 | - | 0 | 0 | 0 | 30 |
| `pset-sibling-openings` | 6 | 9 | 0 | 0 | - | 0 | 0 | not kept | not kept |
| `pset-known-wrong` | 63 | 63 | 0 | 0 | - | 0 | 15 | not kept | not kept |
| `pset-cache-defects` | 10,267 | 10,304 | 0 | 0 | - | 0 | 0 | not kept | not kept |
| `pset-property-set-file` | 2,085 | 1,999 | 28 | 0 | - | 0 | 0 | not kept | not kept |
| `pset-openings-unwritten` | 266 | 300 | 0 | 0 | - | 4 | 0 | not kept | not kept |
| `pset-no-standard-sets` | 0 | 0 | 0 | 4,241 | - | 0 | 0 | not kept | not kept |
| `pset-beyond-ifc2x3` | 0 | 0 | 0 | 0 | - | 88 | 0 | 774 | 410 |
| `ifc2x3-lengths-as-quantities` | 0 | 0 | 0 | 0 | - | 219 | 0 | 0 | 502 |
| `ifc2x3-values-twice` | 0 | 0 | 0 | 0 | - | 19 | 0 | 0 | 24 |
| `ifc2x3-own-set` | 0 | 0 | 0 | 0 | - | 42 | 0 | 0 | 0 |
<!-- /counted-apart:counts -->

## How this page is kept

- `python3 tools/re/counted_apart.py --check`, in CI's PII guard job, fails when a scorer gains a line that sets a value aside and no category names it, when a category's line has left its scorer, when the held-out models' count files (`tools/ci/scrub-holdout-results.sh`) would drop a category's line, or when the table above is not the registry's. The registry is `tools/re/counted_apart.py`.
- Every Measure run prints the table for its own results in its summary ("What the censuses counted apart"), from the `measure-head` directory.
- To refresh the counts above from a run: download its `measure-head` artifact and run `python3 tools/re/counted_apart.py --write measure-head --run <run id> --commit <sha>`. The Autodesk samples have no Revit export, so they are not in the table; the held-out models, the IFC2X3 files and BIMcollab's Revit 2015 model are counts only.
