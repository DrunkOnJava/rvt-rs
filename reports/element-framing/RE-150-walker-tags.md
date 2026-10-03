# RE-150: the walker's base-tag lookup changes nothing it finds

**Date:** 2026-10-03
**Issues:** #154 (follows RE-146); backlog item B24
**Artefacts:** Core Interior (2024), Einhoven (2023), the four RE1 models (2025) and the eleven Autodesk families of 2016 to 2026. Measured on GitHub's hosted runners with the Measure workflow, run 37120815506.
**Probe:** `examples/probe_re150_walker_tags.rs`.
**Status:** negative. The defect is real, but it has no effect on output.

## 1. The question

RE-146 showed that `ClassEntry::tag`, which `formats::parse_schema` keeps on each class, is the tag of the class's base, not its own. The schema-directed walker indexes classes by that word when it scans `Global/Latest` for instance headers (`walker::scan_candidates_with_control`), so it looks for each class under its base's tag. The export calls this walker, through `iter_elements`, before merging the partition-record elements. Does using each class's own tag (its definition ordinal, `formats::schema_classes`) change what the walker finds?

## 2. Method

On each file's `Global/Latest`, the probe runs the candidate scan twice at the production threshold (`PRODUCTION_ELEMENT_MIN_SCORE`, 80):
1. with the schema as `parse_schema` reads it;
2. with every tagged class given its own tag.

For each run it prints the candidates, the commonest classes, and how many candidates' header tag is the own tag of the class they decode as.

## 3. Result

| file | `Global/Latest` bytes | candidates, base tags | candidates, own tags |
|---|---:|---:|---:|
| Core Interior | 1,008,506 | 0 | 0 |
| Einhoven | 884,529 | 0 | 0 |
| RE1 Architecture | 1,243,990 | 0 | 0 |
| every other RE1 model and every family | | 0 | 0 |

Neither tag set yields a single candidate that scores 80 on any file. The walker's `Global/Latest` scan contributes nothing to the export on the reference models. The elements come from the partition records (RE-21 onward), so the base-tag lookup changes no output.

## 4. What this changes

Nothing in the code. `ClassEntry::tag` has documented what it is since RE-146. Moving the walker to own tags would change no measured output, so B24 is closed as measured. Whether the `Global/Latest` scan should stay at all is a separate question; it is noted in the backlog and not decided here.
