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

## The files

| Model | Revit | Licence | Reference export | What it measures |
|---|---|---|---|---|
| magnetar Core Interior | 2024 | MIT, redistributable | IFC4 ReferenceView 1.2, Revit 24.0.20.20 (the full `_slim` export, and a 20 KB element fixture) | 360 walls, 132 doors, 6 windows, 256 columns, 80 slabs, 15 storeys; Revit's GlobalId on 854 elements, 116 rooms and 15 storeys. Run in CI. |
| magnetar Einhoven | 2023 | MIT, redistributable | none | 2023 element records with no export to score against (walls, windows, materials), the 2023 arc-wall path; pinned by the project-count fixture and the viewer's browser test |
| IFC-ECS RE1 Architecture, Mechanical, Plumbing, Electrical | 2025 | MIT, redistributable | IFC2X3 CoordinationView 2.0, Revit 26.2.0.20 | 2025 element records; Architecture: 7 walls, 2 slabs, 11 rooms; 2 storeys each; Revit's GlobalIds 73 / 73 / 123 / 12. Run in CI. |
| Autodesk Snowdon Towers Architectural | 2024 | none declared, local only | IFC4 ReferenceView 1.2, Revit 24.0.20.20 | 5,974 exported, 5,945 of them in Revit's export; 1,078 walls; 18 storeys; wall joins, layers, curtain walls, stairs, roofs |
| Autodesk Snowdon Towers Structural | 2024 | none declared, local only | VIM | beams along their axes, structural categories |
| Autodesk Snowdon Towers Plumbing, HVAC, Electrical | 2024 | Autodesk sample, local only; downloaded from Autodesk's "Revit Sample Project Files" page (sha256 `4f80526b…`, `58538599…`, `e50e1b71…`) | VIM (2027 edition) | **held out**: never used to develop the decoder, measured once as they are (`reports/validation/held-out-snowdon-mep-2026-09-27.md`) |
| MIT 4.567 tutorial house, 2024 and 2025 saves | 2024, 2025 | copyright reserved, local only | none | the same model and ElementIds in two releases; the one shed roof |
| 255ribeiro Projeto1, `teste_export_2025` | 2025 | none declared, local only | IFC4X3 CoordinationView; IFC4 DesignTransferView 1.0, Revit 25.4 (PTB) | metric projects; IFC export overrides; files saved in `PTB` (RE-123) |
| 255ribeiro `Exemplo_data`, `modelo_bim` | 2023 | none declared, local only | IFC4 ReferenceView 1.2, Revit 23.1 (`Exemplo_data`); IFC4 DesignTransferView 1.0, Revit 26.2 PTB (`modelo_bim`, exported from the 2023 file by a later Revit) | Revit 2023 element records (RE-81 to RE-122): 37 of 37 and 37 of 45 elements, storeys, types, layers, materials, wall joins, beams; files saved in `PTB` |
| BIMcollab example, structural model (`02_BIMcollab_Example_STR`) | 2015 | none stated, local only; BIMcollab's public example project (`research/bimcollab-models.tsv`, fetched by Measure with `-f bimcollab=true`, STE1200's pointer) | IFC2X3 CoordinationView 2.0, Revit Architecture 2015, 2014-10-08, rewritten by Solibri's IFC Optimizer; the model was renamed after it | **held out**, measured once before any fix (RE-198): all 241 exported elements by ElementId and class, 3 more written, GlobalIds not in the file (#878, RE-208), 16 footings on no storey (#879); wall faces and layer sets as Revit's |

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

There is no Revit export of any of them, so they measure the record envelope
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
`ARCH.zip`, is 1.58 GB, so CI's tier-two job reads only the five files its
targets need, by byte range (`tools/ci/fetch-bim4lca.py`), and checks each
against [`research/bim4lca-models.tsv`](../research/bim4lca-models.tsv):
four `.rvt` models and Revit's IFC export of one. Locally, point
`RVT_NORDIC_LCA_DIR` at a folder holding them
([`corpus/tier2/README.md`](../corpus/tier2/README.md)). Measure scores the
concrete office, as built and at building permit, against Revit's exports of
both with `-f nordic=true`
([`research/bim4lca-measure.tsv`](../research/bim4lca-measure.tsv), #763).

## Revit 2025 models made for one feature each, fetched for a run

A contributor, puzzbobb, made 23 small Revit 2025 projects. Each isolates
one feature: roofs of one and two slopes, hips and an L-shaped hip, slope
arrows, rafter cuts and a fascia, a roof by extrusion, rooms with a door
and windows, stairs with and without risers, monolithic and spiral, and
wall joins. All but one come with Revit's own IFC4 Reference View export.
He released the `.rvt` and `.ifc` files under Apache-2.0 on #356. Measure
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
  types (#309, #328); rvt-rs follows it where the bytes say which.
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
