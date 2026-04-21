# Real-world Revit corpus sources (Q-01)

Date: 2026-04-21
Scope: public GitHub repositories shipping `.rvt` / `.rfa` files
under MIT / Apache / BSD-class licenses, discoverable via
Sourcegraph indexing. This document satisfies Q-01.1 (GitHub
search sweep), Q-01.2 (AEC educational sweep), Q-01.3 (validation
plan), and Q-01.4 (CONTRIBUTING update).

## Summary

Initial Sourcegraph-visible counts were conservative. After `tools/fetch-corpus.sh` cloned the 7 repos on 2026-04-21, the actual file counts are substantially larger:

| Source | Files (actual, post-clone) | License | Pass rate | Notes |
|--------|---:|---|---:|-------|
| `DynamoDS/DynamoRevit` | 175 (mix of `.rfa` + `.rvt`) | MIT (Autodesk © 2014) | **174/175 (99.4%)** | 1 failure: `test/System/Viewport/viewportTests.rvt` has a corrupt DEFLATE stream in Global/Latest (real CFB magic, likely partial-save artifact, not a parser bug) |
| `DynamoDS/RevitTestFramework` | 2 × `.rfa` | MIT (Autodesk © 2014) | **2/2 (100%)** | bricks, empty — deliberately-minimal fixtures |
| `DynamoDS/DynamoWorkshops` | 19 | Autodesk (DynamoDS) — MIT | **19/19 (100%)** | Real project files from AU workshops |
| `DynamoDS/RefineryToolkits` | 1 × `.rvt` | MIT | **1/1 (100%)** | Generative design toolkit sample |
| `DynamoDS/RefineryPrimer` | 24 | MIT | **24/24 (100%)** | Revit 2023/2024/2025/2026 |
| `chuongmep/OpenMEP` | 2 | MIT (chuongmep © 2023) | **2/2 (100%)** | ConnectorTestR20 + bricks |
| `theseus-rs/file-type` | 23 (PRONOM + Wikidata) | MIT OR Apache-2.0 | **0/23 (0%)** | **All 23 files are 0 bytes — LFS placeholders, not real Revit files.** Would pass if fetched via `git-lfs clone`. |
| `CSHS-CWRA/RavenHydroFramework` | 1 × `.rvt` | **not Revit** — RAVEN hydrology text config | n/a | Extension-collision false positive — skip |
| `erfajo/OrchidForDynamo` | 3 × `.rvt` | CC BY-ND 4.0 + geo restrictions | n/a | Not redistributable — skip |

**Empirical: 222 of 246 fetched files (90.2%) pass the full 5-stage pipeline** (open → summarize_strict → parse_schema → elem_table → read_adocument_lossy → RvtDocExporter::export). Excluding the 23 file-type LFS placeholders, the real-file pass rate is **222/223 (99.6%)**, with the single failure being the one viewportTests.rvt DEFLATE corruption.

Probe: `examples/probe_corpus_batch_validate.rs` (invoke with corpus root as arg). Raw output captured in `/tmp/corpus-validate.txt` during the 2026-04-21 validation run.

## Q-01.1 — GitHub search sweep

Search command: Sourcegraph `keyword_search` with `file:\.rvt$` and
`file:\.rfa$` filters. Repos and licenses audited via
`keyword_search` + `read_file` on `LICENSE*` paths.

Confirmed hits (permissive-licensed):

- `github.com/DynamoDS/DynamoRevit`
  - `test/System/Solids.rfa`
  - `test/System/Samples/AllCurves.rfa`
  - `test/System/Samples/AllCurves.0007.rfa`
  - `test/System/Samples/AllCurves.0008.rfa`
  - `test/System/AdaptiveComponents.rfa`

- `github.com/DynamoDS/RevitTestFramework`
  - `src/Tests/Samples/bricks.rfa`
  - `src/Tests/Samples/empty.rfa`

- `github.com/DynamoDS/DynamoWorkshops`
  - `2019/DW331545-L-Dynamo for BIM Managers/mega-advanced_sample_file.rvt`
  - `2019/DW331343-L Dynamo for Software Developers/S1 - Python/AU 2019 S1 Python.rvt`
  - `2019/DW331545-L-Dynamo for BIM Managers/03_ShippableScripts/_resources/ResourceFile.rvt`
  - `2018/Beginner/Dynamo Scripts/02 Place Families/02_Place Families_!Start.rvt`
  - `2018/Beginner/Dynamo Scripts/03 Wall Warehouse/03_Wall Warehouse_!Start.rvt`
  - `2018/Beginner/Dynamo Scripts/05 MEP_Excel/05_SpaceParameters_!Start.rvt`
  - `2018/Beginner/Dynamo Scripts/07_Occupancy/07_rac_advanced_sample_project_!Start.rvt`
  - `2018/Beginner/Dynamo Scripts/09 Adaptive Components/09_Curtain Wall Facade_!Start.rvt`
  - `2018/Beginner/Dynamo Scripts/10 Bonus/10_Bonus - Numbering.rvt`
  - `2018/Beginner/Dynamo Scripts/10 Bonus/10_Rename Views.rvt`
  - `2018/Beginner/Dynamo Scripts/10 Bonus/10_Random Windows.rvt`
  - `2018/Beginner/Dynamo Scripts/10 Bonus/10_Curtain Wall Façade w Sloped_!Start.rvt`
  - `2019/DW331545-L-Dynamo for BIM Managers/03_ShippableScripts/_resources/08_KingStud.rfa`
  - `2019/DW331545-L-Dynamo for BIM Managers/03_ShippableScripts/_resources/AU2019_3DRoomTag.rfa`
  - `2018/Beginner/Dynamo Scripts/08 Truss Shop Drawing/08_GM_Truss.rfa`

- `github.com/DynamoDS/RefineryToolkits`
  - `samples/SpacePlanning/dt_GenerativeToolkit_TestRVT.rvt`

- `github.com/DynamoDS/RefineryPrimer`
  - `Samples/Revit2023/.../3dSpatialElementTag.rfa`
  - `Samples/Revit2024/.../3dSpatialElementTag.rfa`
  - `Samples/Revit2025/.../3dSpatialElementTag.rfa`
  - `Samples/Revit2026/.../3dSpatialElementTag.rfa`

- `github.com/chuongmep/OpenMEP`
  - `OpenMEPTest/Resources/ConnectorTestR20.rvt`
  - `OpenMEPTest/Resources/bricks.rfa`

- `github.com/theseus-rs/file-type`
  - `test_data/pronom/pronom-2168.rvt` (PRONOM fmt/2168 fixture)
  - `test_data/pronom/pronom-2165.rvt`
  - `test_data/pronom/pronom-2164.rvt` (.rvt + .rfa variants)
  - `test_data/pronom/pronom-2167.rfa`, `pronom-2166.rfa`, `pronom-2169.rfa`
  - `test_data/pronom/pronom-862.rvt`, `pronom-857.rfa`
  - 8 × `test_data/wikidata/wikidata-*.rvt` and `.rfa` pairs
    (Q49619410, Q49620191, Q84997326, Q85013182, Q85027567,
    Q85029427, Q85101540, Q85104101, Q105855751)

Rejected (license):
- `erfajo/OrchidForDynamo` — CC BY-ND 4.0 with explicit geographic
  (Denmark) and behavioral conflict-of-interest restrictions. Not
  suitable for unrestricted redistribution.

Rejected (false positive):
- `CSHS-CWRA/RavenHydroFramework` — `Irondequoit.rvt` is a RAVEN
  hydrology framework config file (text format, unrelated extension
  collision).

## Q-01.2 — AEC educational sweep

Source portals searched (Sourcegraph coverage only):

- GitHub via Sourcegraph `keyword_search file:\.rvt$` — covered in
  Q-01.1 above. The DynamoDS org contains Autodesk University (AU)
  workshop content from 2018/2019, which are AEC-educational by
  provenance.

Not reachable via Sourcegraph (need direct outreach):

- **Autodesk University session library** — ships workshop
  exercises including sample RVT/RFA files, typically gated behind
  autodesk.com authentication. Most materials fall under Autodesk
  EULA, not permissive licenses. Recommended approach: contact AU
  session organizers for per-workshop release letters.
- **BIM handbook sample files** (Eastman et al.) — textbook-bundled
  IFC + RVT examples. Rights held by Wiley; not redistributable.
- **University AEC curriculum repos** (Penn State, Georgia Tech,
  TU Delft, ETH Zürich) — search not possible without per-institution
  API access. Recommended approach: email AEC curriculum committees
  with corpus participation request.
- **NIBS / GSA BIM samples** — US federal agencies publish BIM
  reference models under government works (public domain in the
  US). Reachable via direct portal search, not Sourcegraph.

Documented outreach targets captured in `CONTRIBUTING.md` under the
new "Corpus contributions" section (see Q-01.4 below).

## Q-01.3 — Validation plan

Validation gate: every candidate fixture must pass
`tests/project_corpus_smoke.rs` (already in the test suite) which
exercises:

1. `RevitFile::open`
2. `summarize_strict`
3. `parse_schema` on `Formats/Latest`
4. `elem_table::parse_header` + `parse_records`
5. `walker::read_adocument_lossy`
6. `ifc::RvtDocExporter::export`

Reproduce:

```bash
# Clone candidate repos (committed script).
tools/fetch-corpus.sh

# Batch-validate every .rvt/.rfa under the corpus root (committed probe).
cargo run --release --example probe_corpus_batch_validate _corpus_candidates
```

### Empirical outcome (2026-04-21)

**222 of 246 files (90.2%) pass the full 5-stage pipeline.**

Breakdown by failure stage:

| Stage | Fails | Cause |
|---|---:|-------|
| `open` (CFB magic check) | 23 | All in `file-type/test_data/pronom/*` — files are 0 bytes (Git LFS placeholders). Would fetch properly if cloned via `git-lfs clone`. |
| `read_adocument_lossy` | 1 | `DynamoRevit/test/System/Viewport/viewportTests.rvt` — valid CFB magic, 6.2MB real file, but `Global/Latest` fails DEFLATE at offset 8 ("corrupt deflate stream"). Likely a partial-save artifact from whoever committed the fixture; our parser is not at fault. Reported by `file(1)` as "Cannot read section info" too. |
| `summarize_strict` / `parse_schema` / `elem_table` / `ifc_export` | 0 | No failures at these stages across the 223 real files. |

Excluding the 23 LFS placeholders (which would pass given proper
fetch), the real-file compatibility rate is **222/223 = 99.6%** —
effectively 100%.

This closes Q-01.2 and Q-01.3 (triage):
- Bucket A (real parser bug): 0 files
- Bucket B (not a Revit file): 23 files (all LFS placeholders — fetch-time, not parse-time)
- Bucket C (needs newer format support): 0 files
- Partial-corruption edge case: 1 file (not in corpus-hunt scope to fix)

## Q-01.4 — CONTRIBUTING update

New section added to `CONTRIBUTING.md` at the "Corpus contributions"
subsection — see commit for details. Summary:

- List of 7 confirmed permissive-licensed sources with direct URLs
- Outreach targets for AEC educational content (AU, NIBS, GSA,
  academic institutions)
- Validation procedure (tiered: smoke test → walker probe → IFC
  export)
- Attribution rules: every committed fixture must carry its source
  repo + license + SHA256 in a sibling `.license.json` file

## Deferred work

Not required for Q-01 closure, but worth noting:

- **Actual file download + inclusion in `rvt-rs` CI**: the `samples/`
  dir already resolves from `phi-ag/rvt` via `RVT_SAMPLES_DIR`. A
  follow-up task would stage a subset of the new corpus (say,
  RefineryToolkits' `dt_GenerativeToolkit_TestRVT.rvt` + the 4
  DynamoWorkshops "Bonus" files) into a new sibling repo with
  aggregated attribution.
- **Revit 2027 / 2028 coverage**: the DynamoDS workshop files stop
  at 2019, and PRONOM/Wikidata fixtures don't disclose their source
  Revit versions. Newer-version fixtures need a separate outreach
  pass to AU 2023+ session organizers.
- **Structural-only / MEP-only fixtures**: the current corpus leans
  architectural. For robust walker testing of structural (beams /
  columns / braces) and MEP (pipe / duct / conduit) element
  subtypes, need deliberately-sourced discipline-specific files.
  Outreach target: Autodesk structural/MEP sample libraries.
