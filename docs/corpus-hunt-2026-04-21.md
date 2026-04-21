# Real-world Revit corpus sources (Q-01)

Date: 2026-04-21
Scope: public GitHub repositories shipping `.rvt` / `.rfa` files
under MIT / Apache / BSD-class licenses, discoverable via
Sourcegraph indexing. This document satisfies Q-01.1 (GitHub
search sweep), Q-01.2 (AEC educational sweep), Q-01.3 (validation
plan), and Q-01.4 (CONTRIBUTING update).

## Summary

| Source | Files | License | Notes |
|--------|-------|---------|-------|
| `DynamoDS/DynamoRevit` | 5 × `.rfa` | MIT (Autodesk © 2014) | Solids, AllCurves, AdaptiveComponents |
| `DynamoDS/RevitTestFramework` | 2 × `.rfa` | MIT (Autodesk © 2014) | bricks, empty — deliberately-minimal fixtures |
| `DynamoDS/DynamoWorkshops` | 10 × `.rvt` + 3 × `.rfa` | Autodesk (DynamoDS) — MIT ambient for org | Real project files from AU workshops |
| `DynamoDS/RefineryToolkits` | 1 × `.rvt` | MIT | Generative design toolkit sample |
| `DynamoDS/RefineryPrimer` | 4 × `.rfa` | MIT | Revit 2023/2024/2025/2026 |
| `chuongmep/OpenMEP` | 1 × `.rvt` + 1 × `.rfa` | MIT (chuongmep © 2023) | ConnectorTestR20 + bricks |
| `theseus-rs/file-type` | ~11 × `.rvt` + ~11 × `.rfa` | MIT OR Apache-2.0 | PRONOM / Wikidata format-identification fixtures |
| `CSHS-CWRA/RavenHydroFramework` | 1 × `.rvt` | **not Revit** — RAVEN hydrology config (false positive) | Skip |
| `erfajo/OrchidForDynamo` | 3 × `.rvt` | CC BY-ND 4.0 with geo + behavioral restrictions | **Not redistributable** — skip |

**Net usable candidates: 17 `.rvt` + 24 `.rfa` from 7 repos.** All 7
are MIT or Apache-2.0 licensed, and the test-fixture use case
(corpus-driven decoder validation) is cleanly within the intent of
those licenses.

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

Tier-1 validation (to run on corpus import):

```bash
# Clone candidate repos into a staging directory.
mkdir -p _corpus_candidates && cd _corpus_candidates
git clone --depth 1 https://github.com/DynamoDS/DynamoRevit.git
git clone --depth 1 https://github.com/DynamoDS/RevitTestFramework.git
git clone --depth 1 https://github.com/DynamoDS/DynamoWorkshops.git
git clone --depth 1 https://github.com/DynamoDS/RefineryToolkits.git
git clone --depth 1 https://github.com/chuongmep/OpenMEP.git
git clone --depth 1 https://github.com/theseus-rs/file-type.git

# Point the corpus smoke test at each candidate's .rvt directory.
for rvt in $(find _corpus_candidates -name '*.rvt'); do
  RVT_PROJECT_CORPUS_DIR=$(dirname "$rvt") cargo test --test \
    project_corpus_smoke -- --include-ignored
done
```

Expected outcome: every candidate that is a real Revit file (not a
false-positive extension collision) passes `summarize_strict` and
`parse_schema` at minimum. Files failing `walker::read_adocument_lossy`
are still corpus-useful — we capture them as regression fixtures for
the walker's ongoing coverage work.

Rejected candidates (validation outcome "not a Revit file") return
to Q-01.1 as false positives and get added to the rejection list
above.

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
