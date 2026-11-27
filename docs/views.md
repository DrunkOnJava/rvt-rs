# Views, sheets and placed viewports

`rvt::research::views::read` lists a project's views from the file's own
records, `rvt-views` prints the catalog as JSON, and
`probes/examples/probe_1117_views.rs` prints its counts, sheets, placed
viewports and 3D cameras. It is the reader behind #1117: before
it, a view's name, a sheet's number and a viewport's position on its sheet
could not be read through the API. It is a research module, behind the
`unstable-research` feature, so it can still change between releases: the
box-centre rule rests on one viewport (below).

## What is read

A view is a `DBView*` record on channel 102. The catalog carries its ElementId,
class, kind (`plan`, `three_d`, `section`, `drafting`, `sheet`, `legend`,
`schedule` or `other`), name, description, scale, view plane (`m_origin`,
`m_horzDir`, `m_vertDir`, `m_viewDir`), view range (plans), template and
whether it is a template. A 3D view also carries its `Viewer`'s camera: eye
(`m_boundedSpace.m_orig`), up and looking direction (`m_basis`), and the
stored target where that target lies on the looking axis. The camera is
accepted only when the basis is orthonormal and agrees with the view plane's
own axes, so a set of fields that does not describe one frame is not reported
as one.

A sheet is a `DBViewDrafting` record with an `m_sheetNumber`: its number, name,
date, drawn-by, checked-by, designed-by, approved-by and issue-date fields,
whether it is scheduled, and the viewport ids its record lists.

A viewport is a `Viewport` record. Every view carries one internally; a
viewport placed on a sheet also carries `m_ownerDBViewId` (the sheet's view)
and `m_dbViewId` (the view it shows). The catalog reports its detail number,
offset, scale, orientation, label origin and length, the view's crop box, the
title settings of its `ViewportAttributes` type, and its box centre where one
is measured:

```text
centre = m_offset + m_scale * ((crop.x_min + crop.x_max) / 2,
                               (crop.y_min + crop.y_max) / 2)
```

`m_offset` is where the view's origin lands on the sheet. One viewport of a
contributor's Revit 2024 model was compared with the Revit API's
`Viewport.GetBoxCenter()` and agreed to the five decimals it printed (#1117).
A viewport turned on its sheet (orientation other than 0), a view whose crop
region is off and a viewport whose crop box is not read have no centre here:
the turn and the uncropped extent are not measured.

## Usage

```rust
// Cargo.toml: rvt = { version = "...", features = ["unstable-research"] }
use rvt::RevitFile;
use rvt::research::views::ViewCatalog;

let mut file = RevitFile::open("project.rvt")?;
let catalog = ViewCatalog::read(&mut file)?;
for sheet in &catalog.sheets {
    println!("{} {}", sheet.number, sheet.name);
    for viewport in catalog.viewports_on(sheet.id) {
        println!("  view {} at {:?}", viewport.view_id, viewport.center);
    }
}
```

```bash
cargo install rvt --features unstable-research
rvt-views project.rvt
rvt-views large-project.rvt --large-model > views.json
```

`probes/examples/probe_1117_views.rs` prints the same catalog as counts and
one line per sheet, placed viewport and 3D camera:

```bash
cargo run --profile ci -p rvt-probes --example probe_1117_views -- project.rvt
```

## Evidence boundary

- The box-centre rule is measured on **one** viewport of one Revit 2024
  project, against the Revit API (#1117). It is not re-measured on the office
  model's 21 viewports (on 18 sheets), because no Revit API dump of that model
  is here.
- The reader is measured on a local Revit 2024 project (`rmeadvancedsampleproject`):
  165 views (75 plans, 39 sections, 19 3D views, 14 schedules, 8 sheets,
  7 legends, 2 drafting views and 1 report), 8 sheets, 161 viewport records,
  22 of them placed on sheets. Seven placed viewports have an active crop and
  a centre; the others are uncropped, schedules or legends. Twelve 3D views
  have a camera; nine have a stored target on the looking axis. On all twelve
  the `Viewer` basis equals the view plane's `m_horzDir`, `m_vertDir` and
  reversed `m_viewDir`.
- Revit 2015, 2018 and 2020 projects were opened for views with the same
  reader; the 2020 and 2018 projects read their view records and no sheets.
  The reader follows the native record framing on 2015 to 2027; the element
  data around it is only measured on 2023 to 2025.
- The synthetic `corpus/tier1/` files are writer-generated and hold no native
  partition records, so `read` returns their stream error rather than an empty
  catalog. A test uses the office model under `RVT_NORDIC_LCA_DIR` instead.
- Not read: view graphics and visibility overrides (#1973), per-view filters,
  the view title's printed position, a turned viewport's placement, the
  uncropped view's extent, sheet size, and anything that needs Revit to
  print. The browser viewer, Python bindings and MCP server do not expose the
  catalog yet.
