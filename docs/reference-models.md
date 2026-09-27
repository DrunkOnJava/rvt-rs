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
  cargo test --profile ci --test revit_global_ids --test element_records_2025
```

The script downloads each redistributable file from its original source and
checks it against the registry; nothing is vendored in this repository. It
lists the local-only files with their source and hash, and never fetches them.

## The files

| Model | Revit | Licence | Reference export | What it measures |
|---|---|---|---|---|
| magnetar Core Interior | 2024 | MIT, redistributable | IFC4 ReferenceView 1.2, Revit 24.0.20.20 (the full `_slim` export, and a 20 KB element fixture) | 360 walls, 132 doors, 6 windows, 256 columns, 80 slabs, 15 storeys; Revit's GlobalId on 854 elements, 116 rooms and 15 storeys. Run in CI. |
| magnetar Einhoven | 2023 | MIT, redistributable | none | the 2023 arc-wall path and the metadata/scaffold profile |
| IFC-ECS RE1 Architecture, Mechanical, Plumbing, Electrical | 2025 | MIT, redistributable | IFC2X3 CoordinationView 2.0, Revit 26.2.0.20 | 2025 element records; Architecture: 7 walls, 2 slabs, 11 rooms; 2 storeys each; Revit's GlobalIds 73 / 73 / 123 / 12. Run in CI. |
| Autodesk Snowdon Towers Architectural | 2024 | none declared, local only | IFC4 ReferenceView 1.2, Revit 24.0.20.20 | 6,021 exported, 5,945 of them in Revit's export; 1,078 walls; 18 storeys; wall joins, layers, curtain walls, stairs, roofs |
| Autodesk Snowdon Towers Structural | 2024 | none declared, local only | VIM | beams along their axes, structural categories |
| MIT 4.567 tutorial house, 2024 and 2025 saves | 2024, 2025 | copyright reserved, local only | none | the same model and ElementIds in two releases; the one shed roof |
| 255ribeiro Projeto1, `teste_export_2025` | 2025 | none declared, local only | IFC4X3 CoordinationView; IFC4 DesignTransferView 1.0, Revit 25.4 (PTB) | metric projects; IFC export overrides |

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
`rvt-inspect model.rvt --json` (paths are redacted by default) and
`rvt-ifc model.rvt -o /tmp/out.ifc --diagnostics diag.json` (post the
`diag.json`, not the IFC). They hold counts, Revit class and category names,
and a few sample names such as storeys and materials, but no geometry; read
both before posting and remove any name you would not publish.

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
