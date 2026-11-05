# Reference models

Every recovery claim rvt-rs makes is measured against Revit's own export of a
real file. This page lists those files, where they come from, what may be
done with them, and what a run of rvt-rs on them should show. The machine
record is the `artifacts` list of
[`research/witness-registry.json`](../research/witness-registry.json): origin,
licence, sha256, byte count, Revit release, how the reference export was
made, and whether the file may be redistributed.

## Get the licensed ones

```bash
tools/fetch-reference-models.sh                 # into _project_corpus/Revit (gitignored)
RVT_PROJECT_CORPUS_DIR="$(realpath _project_corpus/Revit)" \
  cargo test --profile ci --test integration -- revit_global_ids:: element_records_2025::
```

The script downloads each redistributable file from its original source and
checks it against the registry; nothing is vendored in this repository. It
lists the local-only files with their source and hash, and never fetches them.

## Model index

The README and the [supported profile](supported-profile.md) name a model for
every number they give. This table is where those names come from: each model,
the names the pages use for it, and whether anyone else can fetch it.
`tools/ci/measured_numbers.py` reads it in CI (the PII guard job), so a number
on either page that names no model listed here, or a model whose row does not
say who can fetch it, fails the check
([how a new number keeps to the rule](maintaining.md#numbers-on-the-readme-and-the-supported-profile)).

Who can fetch it is one of four words:

- **redistributable**: licensed for redistribution. `tools/fetch-reference-models.sh`
  fetches it, and every Measure run and CI score it.
- **fetched by Measure**: a public download, pinned by hash, that a Measure run
  fetches for the run with the flag the row names and does not keep. Anyone can
  dispatch that run: the workflow uses no secret and runs on GitHub-hosted runners.
- **local only**: no documented command fetches it with Revit's export of it, so
  nobody else can re-run a number resting on it. A page says "local only" next
  to such a number.
- **not shared**: files their owner has not shared with anyone; the numbers are
  the owner's report. A page says "not shared" next to such a number.

| Model | Names the pages use | Who can fetch it | How |
|---|---|---|---|
| magnetar Core Interior, Revit 2024 | Core Interior | redistributable | `tools/fetch-reference-models.sh`; with Revit's IFC4 export; scored in every Measure run and in CI |
| magnetar Einhoven, Revit 2023 | Einhoven | redistributable | `tools/fetch-reference-models.sh`; no Revit export of it |
| IFC-ECS RE1 Architecture, Mechanical, Plumbing and Electrical, Revit 2025 | RE1 | redistributable | `tools/fetch-reference-models.sh`; with Revit 26.2's IFC2X3 exports; scored in every Measure run and in CI |
| the synthetic fixtures of `corpus/tier1/` | corpus/tier1 | redistributable | committed to this repository (Apache-2.0); no Revit export |
| VTT's BIM4LCA example buildings (a housing and an office, timber and concrete), Revit 2024 | BIM4LCA | fetched by Measure | `-f nordic=true`, [`research/bim4lca-measure.tsv`](../research/bim4lca-measure.tsv); CC BY-SA 4.0; Measure scores the concrete office as built and at building permit and the two terrain offices, each with Revit's IFC export |
| puzzbobb's 24 Revit 2025 models | puzzbobb, R2025 | fetched by Measure | `-f r2025=true`, [`research/r2025-models.tsv`](../research/r2025-models.tsv); Apache-2.0; with Revit's IFC4 export of each |
| Autodesk's sample projects of Revit 2015 to 2027 (`rac_basic`, `rst_basic`, `rac_advanced`, `rme_basic`, `rme_advanced`, `rst_advanced`) | rac_basic, rst_basic, rac_advanced, rme_basic, rme_advanced, rst_advanced, rac_basic_sample_project, rac_advanced_sample_project, sample projects | fetched by Measure | `-f samples=true`, [`research/autodesk-sample-projects.tsv`](../research/autodesk-sample-projects.tsv); no Revit export of them in a run, except the IFC2X3 export of `rac_advanced` 2015 to 2020 that `-f racadv=true` fetches from the three.js repository |
| Autodesk's family corpus (phi-ag/rvt, 12 releases of `rac_basic_sample_family`) | family corpus | fetched by Measure | `-f families=true`; CC BY-NC-SA 3.0; no Revit export of it |
| BIMcollab's example structural model, Revit 2015 | BIMcollab | fetched by Measure | `-f bimcollab=true`, [`research/bimcollab-models.tsv`](../research/bimcollab-models.tsv); no licence stated, so only its summary counts are uploaded; with Revit 2015's IFC2X3 export |
| BIM-Speed Vitoria, Frigento, Gdynia and Warmond, Revit 2018 to 2021 | Vitoria, Frigento, Gdynia, Warmond | fetched by Measure | `-f holdout=true`, [`research/holdout-archive-models.tsv`](../research/holdout-archive-models.tsv); CC BY 4.0; counts only, held out |
| BIM Whale `AdvancedProject`, Revit 2021 | BIM Whale | fetched by Measure | `-f holdout=true`; counts only, held out |
| Autodesk's Snowdon Towers sample files alone (Architectural, Structural, Plumbing, HVAC, Electrical), Revit 2024 | Snowdon Towers samples, Snowdon Towers 2024 samples | fetched by Measure | `-f samples=true`, [`research/autodesk-sample-projects.tsv`](../research/autodesk-sample-projects.tsv); a number that rests on the files alone (their records, what rvt-rs reads from them) can be re-run |
| Revit's exports of Snowdon Towers (the IFC4 export of the Architectural, the VIM of the Structural, Plumbing, HVAC and Electrical), Revit 2024 | Snowdon | local only | the exports are not available to a Measure run (the IFC4 is from bldrs-ai/test-models with no licence), so nothing scored against them can be re-run: a page that gives a Snowdon number says "local only" |
| the Revit 2018 export of `rac_basic_sample_project` that a user posted on the IFC for Revit wiki (SourceForge `ifcexporter`, 2018-01-04) | posted on the IFC for Revit wiki | local only | a forum attachment with no licence stated, used locally and never committed ([RE-326](../reports/element-framing/RE-326-room-phases.md), section 5) |
| MIT 4.567 tutorial house, 2024 and 2025 saves | MIT house | local only | copyright reserved; no download |
| 255ribeiro `Projeto1`, `teste_export_2025`, `Exemplo_data` and `modelo_bim`, Revit 2023 and 2025 | Projeto1, teste_export_2025, Exemplo_data, modelo_bim | local only | no licence declared; not downloadable |
| flowbim's `AA-SingleDwellingHouse`, Revit 2026, with its IFC | AA-SingleDwellingHouse | local only | a public download from flowbim.ee with no licence stated, and no Measure flag fetches it |
| Revit 2008 to 2014 project files | local closed-source files | local only | closed-source files, outside the 1.0 range; no download |
| the field reports' client models of Revit 2022 to 2027 (#421, #1075) | client models, client model | not shared | their reporter's files; the counts are what the reporter sent back |

## Re-run the numbers

The command that re-runs the numbers on the models anyone can fetch is a
Measure dispatch, the workflow the figures on the README and the supported
profile were made with. It uses no secret and runs on GitHub-hosted runners, so
it works from a fork with Actions enabled:

```bash
gh workflow run measure.yml --ref main -f ref=<commit to measure> -f base=none \
  -f nordic=true -f r2025=true -f samples=true
```

`ref` is the commit to measure and the scorers come from the workflow's own
ref. The run fetches each model the index above marks `redistributable` or
`fetched by Measure` (Core Interior, Einhoven and RE1 on every run; the BIM4LCA
files with `nordic`; puzzbobb's 24 with `r2025`; Autodesk's sample projects, the
Snowdon Towers files among them, with `samples`), exports each with that commit's `rvt-ifc`, and scores the export
against Revit's own with the scorers under `tools/re/` where Revit's export is
among the fetched files. The scores are in the run's summary and artifacts.

What it covers: the figures that name Core Interior, Einhoven, RE1, BIM4LCA,
puzzbobb and Autodesk's sample projects and are scored by those scorers, at the
commit you name. Add `-f families=true` for the family corpus (read, scored
against nothing), and `-f holdout=true`, `-f bimcollab=true` and `-f racadv=true`
for the other `fetched by Measure` models, which keep only their counts.

What it does not cover:

- A number on a model the index marks `local only` or `not shared` (Snowdon
  Towers' scores against Revit's exports, the MIT house, the 255ribeiro
  projects, flowbim's 2026 house, the pre-2015 files, the client models): the
  file, its export or both are not available to a run.
- A figure made by a probe (`-f probe=`, with its `-f probe_scorer=`) or by a
  comparison with another commit (`-f base=main`, the byte-identity counts): the
  report that gives it has the dispatch with those inputs in its Reproduce
  section.
- The counts the tests under `tests/` pin on the same models. Fetch the models
  with `tools/fetch-reference-models.sh` and run `tools/check-local.sh --corpus`
  for those; it also runs the whole gate first.
- Whether a figure is current. A run gives the numbers of the commit it
  measures. A figure on a page is the number of the run it names, at that run's
  head, until the pull request that moves it edits the page.

Nobody has yet re-run the pages' numbers from this command and compared the
result with them: it is the command their runs were dispatched with, not a
verified reproduction of every figure (W10 in the
[1.0 criteria](release-1.0-criteria.md)).

## The files

| Model | Revit | Licence | Reference export | What it measures |
|---|---|---|---|---|
| magnetar Core Interior | 2024 | MIT, redistributable | IFC4 ReferenceView 1.2, Revit 24.0.20.20 (the full `_slim` export, and a 20 KB element fixture) | 360 walls, 132 doors, 6 windows, 256 columns, 80 slabs, 15 storeys; Revit's GlobalId on 854 elements, 116 rooms and 15 storeys. Run in CI. |
| magnetar Einhoven | 2023 | MIT, redistributable | none | 2023 element records with no export to score against (walls, windows, materials), the 2023 arc-wall path; pinned by the project-count fixture and the viewer's browser test |
| IFC-ECS RE1 Architecture, Mechanical, Plumbing, Electrical | 2025 | MIT, redistributable | IFC2X3 CoordinationView 2.0, Revit 26.2.0.20 (the files are Revit 2025, so this is a cross-release pair) | 2025 element records; Architecture: 7 walls, 2 slabs, 11 rooms; 2 storeys each; Revit's GlobalIds 73 / 73 / 123 / 12. Run in CI. |
| Autodesk Snowdon Towers Architectural | 2024 | none declared, local only | IFC4 ReferenceView 1.2, Revit 24.0.20.20 | 5,968 exported, 5,945 of them in Revit's export (from #309 and #328, no committed report; the other 23 are curtain panels with no resolved type, and the 6 slabs in the Legends phase of #328 are no longer written, RE-184); 1,078 walls; 18 storeys; wall joins, layers, curtain walls, stairs, roofs |
| Autodesk Snowdon Towers Structural | 2024 | none declared, local only | VIM | beams along their axes, structural categories |
| Autodesk Snowdon Towers Plumbing, HVAC, Electrical | 2024 | Autodesk sample, local only; downloaded from Autodesk's "Revit Sample Project Files" page (sha256 `4f80526b…`, `58538599…`, `e50e1b71…`) | VIM (2027 edition) | **held out**: never used to develop the decoder, measured once as they are (`reports/validation/held-out-snowdon-mep-2026-09-27.md`) |
| MIT 4.567 tutorial house, 2024 and 2025 saves | 2024, 2025 | copyright reserved, local only | none | the same model and ElementIds in two releases; the one shed roof |
| 255ribeiro Projeto1, `teste_export_2025` | 2025 | none declared, local only | IFC4X3 CoordinationView; IFC4 DesignTransferView 1.0, Revit 25.4 (PTB) | metric projects; IFC export overrides; files saved in `PTB` (RE-123) |
| 255ribeiro `Exemplo_data`, `modelo_bim` | 2023 | none declared, local only | IFC4 ReferenceView 1.2, Revit 23.1 (`Exemplo_data`); IFC4 DesignTransferView 1.0, Revit 26.2 PTB (`modelo_bim`, exported from the 2023 file by a later Revit) | Revit 2023 element records (RE-81 to RE-122): 37 of 37 and 37 of 45 elements, storeys, types, layers, materials, wall joins, beams; files saved in `PTB` |
| BIMcollab example, structural model (`02_BIMcollab_Example_STR`) | 2015 | none stated, local only; BIMcollab's public example project (`research/bimcollab-models.tsv`, fetched by Measure with `-f bimcollab=true`, STE1200's pointer) | IFC2X3 CoordinationView 2.0, Revit Architecture 2015, 2014-10-08, rewritten by Solibri's IFC Optimizer; the model was renamed after it | held out when measured (RE-198); RE-208 and RE-220 have since used it, so it no longer counts as held out: all 241 exported elements by ElementId, in Revit's class or IFC4's for its IFC2X3 one, 3 more written, GlobalIds not in the file (#878, RE-208), 16 footings on no storey (#879); wall faces and layer sets as Revit's |
| Autodesk `rac_advanced_sample_project`, saves of 2015 to 2020 | 2015 to 2020 | Autodesk sample, local only (`research/autodesk-sample-projects.tsv`); the export has no licence of its own, so both are fetched by Measure with `-f racadv=true` (`research/rac-advanced-export.tsv`, models `racadv-2015` to `racadv-2020`), never stored or cached, and only their summary counts are uploaded | IFC2X3 CoordinationView 2.0 by Revit 2021 (Spanish edition, exporter string ending "IU alternativa", publisher not established), from the three.js repository at commit `7c1f4773`, sha256 `9440a72e…fe0a5`; it shows the 2015 to 2020 state and is counted apart as IFC2X3: it can confirm a reader and never removes a value (RE-330, #1354) | the one Revit export of a 2015 to 2023 project with rooms and Areas: 91 rooms and 25 Areas, 16 of the Areas within 0.1% and 0.01 ft (RE-330); the rooms (`space_profiles_vs_ifc`, #1354) |
| BIM-Speed Vitoria (`ALD26 Before`, `ALD26 Project`), Frigento, Gdynia, Warmond | 2018, 2018, 2020, 2021, 2021 | CC BY 4.0 (TU Berlin DepositOnce, doi:10.14279/depositonce-16441); fetched by Measure with `-f holdout=true` (`research/holdout-archive-models.tsv`) | each release's own export: IFC2X3 CoordinationView 2.0 (Vitoria, Gdynia), IFC4 DesignTransferView (Frigento), IFC4 passed through BIMServer.center's preprocessor (Warmond); the IFC2X3 exports and Warmond's are counted apart and never remove a value | **held out** (Griffin, 2026-11-03): measured before any fix, counts only; the first same-release pairs for 2018, 2020 and 2021 (#408, #1261). Before any fix (run 41658225355): both Vitoria saves export no element against Revit's 1,394 and 1,520. Every Autodesk 2018 sample exports its elements, so `ALD26 Before` is spent to find why (measured in full as `spent-bimspeed-vitoria-before-2018`) and `ALD26 Project` stays held out as the check. After RE-315 (frames with no ElementId of their own take their record's, run 41707934595): `ALD26 Before` writes 1,136 of Revit's 1,321 elements by Tag, 1,127 of them in Revit's class or IFC4's, with none that Revit's export lacks, on 7 storeys; `ALD26 Project` first wrote 1,534 elements and spaces against Revit's 1,520, counts only, 198 of them elements Revit's export does not hold, and writes 1,336 (1,263 elements and 73 spaces) with none, since the export phase and design options are applied to Revit 2016 to 2023 (the counts cannot tell which of the two removed them); Frigento, Gdynia and Warmond export byte for byte as before. After RE-319 (conduits, conduit fittings and gutters, run 41741114199): `ALD26 Before` writes 1,314 of Revit's 1,321 elements by Tag, 1,305 of them in Revit's class or IFC4's, with none that Revit's export lacks, 1,251 on Revit's storey; `ALD26 Project` writes 1,514 elements and spaces (1,441 elements and 73 spaces) against Revit's 1,520, counts only, none that Revit's export lacks, 1,433 of the 1,441 elements in Revit's class or IFC4's and 1,359 on Revit's storey; the other held-out pairs export byte for byte as before. Before any fix, Frigento 151 of 151 elements paired by Tag, Gdynia 199 of 203 (18 of 19 spaces), Warmond 710 of 710 (179 of 179 spaces); an element is counted as found by its Tag, whatever its class |
| BIM Whale `AdvancedProject` | 2021 | the IFC from andrewisen/bim-whale-ifc-samples (MIT); the `.rvt` from the Dropbox link its README gives, under the README's "feel free to use these files however you like"; fetched by Measure with `-f holdout=true` | IFC2X3 CoordinationView 2.0, exporter 21.1.10.26, counted apart | **held out** (Griffin, 2026-11-03): before any fix it had 1,698 of 1,714 elements paired by Tag (456 of them in another class than Revit's: 407 `IfcFurniture` for `IfcFurnishingElement`, 49 `IfcSanitaryTerminal` for `IfcFlowTerminal`) and none of its 8 spaces. Those spaces are Revit Areas of a 'Rentable' scheme, not rooms. The first rule for Revit 2015 to 2023 Areas (RE-324) wrote every placed Area, found 8 of the 8 and wrote 2 spaces Revit's export lacks (run 41757734285, counts only); an Area is now written only where its body is proven, none of its 10 had one, and it writes 0 of 8 and 0 extras (run 41764492879), which is vacuous and leaves its Areas undelivered (#1261); it is a regression check under W5. It was briefly spent on 2026-11-03 to look for the cause and restored. While spent it was measured in full in four runs (41661841411, 41668542833, 41671242397 and 41672953553, their artifacts deleted), and the RE-315 work read its record chain and frame counts beside Vitoria's; no rule or constant came from it (#1261, #1286) |

## Candidates, not yet measured here

STE1200 (Discussion #112) measured these and gave their provenance; rvt-rs
has not measured them yet. They are public downloads without a stated
redistribution licence, so they would be local-only oracles: fetched from
their source by whoever runs the measurement, never committed.

| Model | Revit | Source | sha256 | Why |
|---|---|---|---|---|
| AA-SingleDwellingHouse-RVT-CIADD, with its IFC | 2026 | [flowbim.ee](https://flowbim.ee/files/content/cci/ENG/AA-SingleDwellingHouse-RVT-CIADD.rvt) | `eeff78f3…0a654` (IFC `c9e2986c…67342`) | the first 2026 project with Revit's export: the 2026 element-record gate (#421); measured in RE-124 (88 placed instances, all in Revit's export) |
| rac_basic_sample_project | 2017 | Autodesk sample | `a1d3d077…5fefb` | the same, with no export; now fetched and measured with the other sample projects (below, RE-140) |
| Geberit VariForm washbasin PRO_1833290, two variants | 2018 family | Geberit | `28b9d111…e5e4`, `0d48b22c…fbb9` | DirectShape meshes in a family |
| Pucciplast Cassetta Eco 9/4 l (EN) | 2022 family | Pucciplast | `a610ffcc…ae00` | sketch forms and voids in a family |

STE1200's test vectors for the 2026 house, the BIMcollab model (now measured,
RE-198) and the 2017 sample (record bytes, offsets and the values read) are the
cross-check for any reader of them.

## Autodesk's sample projects, fetched for a run

Autodesk publishes `rac_basic`, `rst_basic` and `rac_advanced` sample projects
for every Revit release from 2015 to 2027 on its own download host, 39 files,
and the MEP projects `rme_basic` and `rme_advanced` and the `rst_advanced`
project. Of those, the MEP projects of 2024 to 2027 are pinned (8 files: ducts
of every shape, fittings, connectors and systems the reference models lack),
and the MEP and `rst_advanced` projects of 2015 and 2019 (6 files), which check
the readers of 2015 to 2022 files (RE-180, RE-181). Revit 2015's are under
`Docs/` on the host, not `Docs/InProd/` as for later releases. Its multi-model
2024 sample, Snowdon Towers, is pinned as well (5 files: Architectural, Structural, HVAC, Plumbing and Electrical),
so that a regression on the models #736 found is seen in a run. It is the host the sample families of the phi-ag/rvt
corpus (CC BY-NC-SA 3.0) come from; the projects' own terms were not checked. They are pinned by hash in
[`research/autodesk-sample-projects.tsv`](../research/autodesk-sample-projects.tsv)
and fetched by `tools/fetch-autodesk-samples.sh`; the Measure workflow reads
them with `-f samples=true` (`rvt-info`, the IFC export's diagnostics, and the
probe given). They are never stored in this repository.

No Revit export of any of them is available to a Measure run (Snowdon Towers Architectural's is local only), so they measure the record envelope
and what a release's reader gives on the same model, not recovery against
Revit: `reports/element-framing/RE-140-sample-projects-2016-2027.md` compares
each release's records with 2024's by ElementId. They do not admit a release.

## BIM4LCA example buildings, fetched for CI

VTT's BIM4LCA project published Revit 2024 models of a housing and an office
building, each in a timber and a concrete version, with Revit's own IFC
export of each, under CC BY-SA 4.0
([BIM files to download](https://cris.vtt.fi/en/publications/bim-files-to-download/)).
They hold what the reference models lack, and several fixes rest on them:
a table of more than 65,535 records (#745), a reference Level at a storey's
elevation (#746), columns cut by the slabs they join (#756). The archive,
`ARCH.zip`, is 1.58 GB, so CI's tier-two job reads only the six files its
targets need, by byte range (`tools/ci/fetch-bim4lca.py`), and checks each
against [`research/bim4lca-models.tsv`](../research/bim4lca-models.tsv):
five `.rvt` models (the small terrain office, concrete, among them) and
Revit's IFC export of one. Locally, point
`RVT_NORDIC_LCA_DIR` at a folder holding them
([`corpus/tier2/README.md`](../corpus/tier2/README.md)). Measure scores the
concrete office, as built and at building permit, and the small terrain
offices (concrete and timber, at building permit), against Revit's exports of
each with `-f nordic=true`
([`research/bim4lca-measure.tsv`](../research/bim4lca-measure.tsv), #763,
#1262).

## Revit 2025 models made for one feature each, fetched for a run

A contributor, puzzbobb, made 23 small Revit 2025 projects, and on #885 a 24th
(`20_T_U_Roof`, with T and U hip roofs). Each isolates
one feature: roofs of one and two slopes, hips and an L-shaped hip, slope
arrows, rafter cuts and a fascia, a roof by extrusion, rooms with a door
and windows, stairs with and without risers, monolithic and spiral, and
wall joins. All but one of the first 23 come with Revit's own IFC4 Reference
View export, and that one has the export of its property-set variant. He
released the `.rvt` and `.ifc` files under Apache-2.0 on #356 and #885. Measure
fetches the archive by its pinned hash
([`research/r2025-models.tsv`](../research/r2025-models.tsv),
`tools/ci/fetch-r2025-models.sh`) and, with `-f r2025=true`, scores each
model against its export like the reference models. They are never stored
in this repository.

## Compare like with like

The reference exports were made with different settings, and several
differences between rvt-rs and "Revit's export" are settings, not decoding:

- **View definition.** A ReferenceView export (Core Interior, Snowdon) writes
  bodies as tessellations and no material layer sets, so layer sets are scored
  against the faces its bodies style with each material. A CoordinationView
  export (RE1) writes layer sets of its own.
- **Schema.** RE1 is IFC2X3 and Projeto1 IFC4X3; entity names differ from IFC4
  (for example `IfcFlowTerminal` in IFC2X3 for what IFC4 types more finely).
- **Coordinates.** Every export here uses shared coordinates. rvt-rs writes
  Revit's internal coordinates, so compare after the export's own transform
  (Snowdon: rotate 26.0156°, translate (1370151.83, 258247.98) ft, z + 780.5 ft).
- **What is exported.** Revit's exporter leaves out non-primary design
  options, elements not in the exported phase, 2D-only families and some
  types (#309, #328); rvt-rs follows it where the bytes say which. A room is left out by its own phase, which Revit does by a room's own phase parameter (RE-326); no model here has a room outside the last phase with a Revit export made with the default setup.
- **Model state.** An export can be newer than the `.rvt` beside it:
  `teste_export_2025.ifc` shows one wall in a later state than its `.rvt`.

## Adding a model

A new reference model is most useful when its owner can licence it for
redistribution and it comes with Revit's own IFC export. Record it in the
registry's `artifacts` (id, node, sha256, bytes, source, url when fetchable,
licence, Revit release, `derived_from` and `via` for the export, an `export`
line with the view definition and exporter version, `redistributable`, and
the observations you checked) and open a pull request with the entry only.
See [`corpus-intake.md`](corpus-intake.md) for the three intake lanes.

A pair offered as held out for 1.0's W5 (#408) is pinned sealed and measured
by no run until the final 1.0 check: W5 counts a pair only if its first
measurement meets W4, W6 and W7, and no model meets W6 and W7 yet, so a pair
measured now would be used up (`docs/release-1.0-criteria.md`, W5).

If you cannot share the file, share what rvt-rs says about it:
`rvt-inspect model.rvt --json --anonymize` and
`rvt-ifc model.rvt -o /tmp/out.ifc --diagnostics diag.json --anonymize`
(post the `diag.json`, not the IFC). They hold counts, releases and Revit
class and category names, and no geometry. Without `--anonymize` they also
hold the names the project gives things: the file and project names, the
levels, a sample of the materials and of skipped elements, and, in
`rvt-inspect`'s report, the folder path Revit stored for the file (by default it
replaces only a path's `\Users\<name>\` part and Autodesk build and
project-id folders). `--anonymize`
replaces those with placeholders such as `Level 1`, keeping counts and
ElementIds; the browser viewer's Download diagnostics does the same. Read
both before posting.

## Tiny models anyone with Revit can make

Several open questions need a Revit file whose answer is known in advance.
Each recipe is a new project from Revit's default architectural template,
saved in Revit 2024 or 2025, and exported with File > Export > IFC using
"IFC4 Reference View" and default options. Contribute the `.rvt`, the `.ifc`,
and a line saying you release both under MIT or CC0.

1. **Hip roof (#356).** On Level 1, four Basic Wall "Generic - 200mm" walls
   forming a 10 m by 6 m rectangle, 3 m high. Roof by Footprint on Level 2,
   picking the four walls, every edge "Defines Slope" at 30°, roof type
   "Generic - 300mm".
2. **Gable roof (#356).** The same walls; Roof by Footprint with only the two
   long edges defining slope at 30°.
3. **Wall joins (#238, #358).** Four "Generic - 200mm" walls: an L corner
   (default join), a second L corner with the join set to Miter, a T junction,
   and one wall end with "Disallow Join".
4. **Room (#90).** The rectangle of recipe 1 without a roof, one interior
   wall splitting it 6 m / 4 m, and a Room placed in each part.
5. **Opening (#227).** One "Generic - 200mm" wall 5 m long with one
   "Single-Flush 0915 x 2134mm" door and one "Fixed 0915 x 1220mm" window.
