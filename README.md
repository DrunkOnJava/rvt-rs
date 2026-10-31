# rvt-rs

**Inspect Revit files without installing Revit.**

rvt-rs is an open-source toolkit for reading Autodesk Revit project, family and template files (`.rvt`, `.rfa`, `.rte`, `.rft`). It reads a file's metadata, preview images and embedded schema. On the Revit releases it supports, it also recovers the building's elements (walls, floors, roofs, doors, windows, stairs, curtain walls, furniture, MEP and structure) with the names, types, storeys, materials and identifiers Revit gives them, and exports them as IFC4 (the open building-model exchange format), glTF (3D), plan SVG and CSV schedules.

It comes as command-line tools, a Rust library, Python bindings and a browser viewer that reads your file inside the browser tab. It is built from public observations of the file format ([`CLEANROOM.md`](CLEANROOM.md)), licensed under the [Apache License 2.0](LICENSE), and not affiliated with Autodesk.

## Start here

| What you want to do | Where to start |
|---|---|
| See a model without installing anything | Open the [browser viewer](https://drunkonjava.github.io/rvt-rs/) and pick a demo, or drop in a file of your own. |
| Inspect or export files from the command line | The [installation guide](docs/install.md); prebuilt binaries are on [GitHub Releases](https://github.com/DrunkOnJava/rvt-rs/releases). |
| Work with files in Python | The [Python guide](docs/python.md) (`pip install rvt`). |
| Check what your file will give | [What you can rely on, by input](#what-you-can-rely-on-by-input) below, then the [user guide](docs/user-guide.md). |
| Use the Rust library | The [examples](examples/README.md) (open a file, list its elements, export it), [Library surface](#library-surface) below and the [API documentation](https://docs.rs/rvt). |
| Tell us how it went on your own models | [Discussion #982](https://github.com/DrunkOnJava/rvt-rs/discussions/982): your diagnostics are enough, never the building. |
| Ask a question, report a problem or contribute | [Discussions](https://github.com/DrunkOnJava/rvt-rs/discussions) for questions, and the [contribution guide](CONTRIBUTING.md) for everything else. |

## Support and evidence

Support depends on the Revit release, the file type and the feature. Opening a file and reading its metadata does not mean its elements or geometry can be recovered: elements are recovered from Revit 2023, 2024 and 2025 projects, and experimentally from Revit 2015 to 2022, 2026 and 2027 projects. Geometry is exact where the file's bytes are decoded and the element's bounding box elsewhere. `rvt-elements` and Python list each element's saved parameters on Revit 2023 to 2027 files, and `rvt-ifc --revit-property-sets` writes the named ones in Revit's own property sets (on Autodesk's Snowdon Towers HVAC, every one Revit's). Every export comes with diagnostics that say what was approximated or left out; check them before relying on an export for measurement or coordination.

The recovery figures in this README compare rvt-rs's export with Revit's own IFC export of the same file, element by element, on the models named in [What rvt-rs reads from real projects](#what-rvt-rs-reads-from-real-projects). Other models of the same releases are expected to behave alike but are not measured, and files saved by the same release can store their data differently. [`docs/support-matrix.json`](docs/support-matrix.json) records each capability with its evidence and its status ceiling.

This README describes the development branch, `main`, which the browser viewer is also built from. Installed packages are the latest release; [Release 0.11.0 and main](#release-0110-and-main) says what differs.

## What you can rely on, by input

| Input | Open, metadata, previews, schema | Elements | Geometry | Properties |
|---|---|---|---|---|
| Revit 2024 and 2025 projects | yes | typed IFC entities from each element's own record; by ElementId the set equals Revit's export on Core Interior, RE1 Architecture, Electrical and Mechanical, the 22 of puzzbobb's Revit 2025 models that have an export, and the BIM4LCA office as built; RE1 Plumbing and the office at building permit each write one element Revit's export leaves out, and 23 of Snowdon Towers' 5,968 are not in it (local only; details below) | measured per category: exact joins, profiles and cuts where decoded, the element's bounding box where not | names, family and type names, storey, materials and layers, Revit's GlobalId, stair dimensions, IFC export overrides; most other parameters not read |
| Revit 2026 projects | yes | **experimental (since 0.5.0):** typed IFC entities from each element's record (RE-124), on one 2026 project 113 of the 115 elements of Revit's export, none outside it | the element's bounding box, walls cut back at their joins (wall thickness Revit's on 59 of 59, length on 35) | Revit's GlobalIds (113 of 113), storeys with Revit's names and elevations (4 of 4), rooms with Revit's number and name (13 of 13), doors and windows in their host walls, family and type names, wall and floor layer sets as thick as Revit's, walls joined by their join lists; roof slopes, stairs and IFC export overrides not read |
| Revit 2027 projects | yes; 325 fields of the 2027 schema carry a flag whose meaning is not measured (RE-264) | **experimental (since 0.6.0):** typed IFC entities from each element's record, with the constants the file's schema gives (RE-179); on Autodesk's five 2027 sample projects the same ElementIds and classes as their 2026 copies (20,818 of 20,818), with **no Revit 2027 export measured** | as on 2026 | as on 2026: storeys, rooms, doors and windows in their hosts, family and type names, layer sets, wall joins and mullion materials read the same as the 2026 copies; 79 corner mullions are not put in a curtain wall |
| Revit 2023 projects | yes | typed IFC entities (since 0.4.0) from each element's record (RE-81): on two 2023 projects every element of Revit's export (37 of 37) or all but the 8 window trims nested in windows (37 of 45), none outside it | the element's bounding box, walls cut back by the walls they join (Exemplo_data 17 of 17 and modelo_bim 4 of 4 as Revit's, RE-120), beams stopped at the faces of the columns they frame into (modelo_bim 8 of 8, RE-122); rooms their outline (RE-102) | ElementId, category, each door's and window's host wall (30 of 30 as Revit's, RE-85), and the storeys of Revit's own Levels with each element on the one its record names, a beam on the one at its top (Exemplo_data 37 of 37 and modelo_bim 37 of 37 in Revit's storey, RE-107, RE-119), each family instance's family and type name (50 of 50 as Revit names them, RE-109), each wall's, floor's and roof's type and system family (24 of 24 with Revit's type, RE-111, RE-112), family instances' materials (44 of 45 as Revit's, RE-113), and layer sets, named by their own or their category's material (RE-114, RE-115): Exemplo_data 36 of 37 and modelo_bim 31 of 37 elements with Revit's materials, and materials with Revit's shading colours (20 of 21, none different, RE-116); no layered or angled joins, design options or IFC export overrides, and the diagnostics say so |
| Revit 2019 to 2022 projects | yes | **experimental (since 0.6.0):** typed IFC entities from each element's record, which 2019 to 2022 frame as 2023 does, read with the tags each release's schema gives (RE-180); on Autodesk's 2019 to 2022 sample projects every ElementId shared with the 2023 copy has the class it has there (`rac_basic`, `rst_basic` and `rac_advanced` of each release), with **no Revit export of a 2019 to 2022 file measured** | as on 2023 | as on 2023: storeys, doors and windows in their hosts, family and type names, layer sets and materials read as the 2023 copies do, with a few type names and type materials different (RE-180 §6) |
| Revit 2015 to 2018 projects | yes on 2016 to 2018; on 2015, partial: they open on Autodesk's sample projects, and previews and PartAtom are not measured | **experimental (since 0.6.0):** typed IFC entities from each element's record, which 2015 to 2018 frame as 2019 does, read with the tags each release's schema gives (RE-181); on Autodesk's 2016, 2017 and 2018 sample projects every ElementId shared with the 2019 copy has the class it has there (`rac_basic`, `rst_basic` and `rac_advanced` of each release), and on five of its six 2015 sample projects (the sixth is another model), with **Revit's own export measured on one 2015 project and on none of 2016 to 2018**: BIMcollab's structural example (RE-198, a public download, not redistributable) has all 241 of Revit's elements by ElementId, each in Revit's class or IFC4's for its IFC2X3 one, 3 air terminals Revit's export leaves out (#879), 16 footings on no storey and none of its elements' GlobalIds Revit's (#878); 2014's element records are not read, since no documented 2014 project backs them | as on 2019 | as on 2019: storeys, doors and windows in their hosts, family and type names, layer sets and materials read as the 2019 copies do, but where the files' own bytes differ (RE-181 §6) |
| Revit 2011 to 2014 projects | yes: 2011 to 2013 open since RE-259, whose schemas have no GUID lists; measured on local closed-source files only, outside the 1.0 range (2015 to 2027) | **experimental:** typed IFC entities from each element's record, which 2011 to 2014 frame as 2015 does, read with the tags each release's schema gives (RE-259); on a local copy of Autodesk's 2013 `rac_advanced` sample every ElementId shared with its 2015 copy has the class it has there and 62,071 of 62,153 fields agree, with **no Revit export of a file before 2015 measured** and no same-model pair for 2011, 2012 or 2014 | as on 2015 | as on 2015: storeys, doors and windows in their hosts, family and type names, layer sets and materials read as the 2015 copy does, but where the files' own bytes differ (RE-259 §7) |
| Revit 2008 to 2010 projects | yes: they open and report their release, which 2008 and 2009 take from their build date (RE-259); measured on local closed-source files only, outside the 1.0 range (2015 to 2027) | **experimental:** typed IFC entities from each element's record, which 2009 and 2010 frame as 2011 does and 2008 4 bytes shorter (RE-260); on one local 2009 project 8 doors and their wall, on one 2008 project one generic model, with **no Revit export and no same-model copy of any 2008 to 2010 file measured** | as on 2015 | storeys with their names and elevations (RE-260, also on 2011 and 2012), materials named and coloured; type names, layer sets and hosts as on 2015 but not measured |
| Families (`.rfa`) | yes | family metadata, OmniClass, previews; no family geometry | none | a family's types and type parameters as its `PartAtom` stream lists them (`rvt-info`, `PartAtom::family_types`) |
| Templates (`.rte`, `.rft`) | the same container code; no template file is in any corpus, so not measured | not measured | none | none |

**Typical uses:** inventory and audit folders of Revit files without Revit (`rvt-info`); open a 2024 or 2025 model in a browser or in an IFC viewer to check its layout, storeys, types and materials; take element and room schedules into a spreadsheet (`rvt-schedule`); feed IFC or glTF into a coordination, quantity or visualisation pipeline, checking the per-export diagnostics first; and research the format itself with the probes under `examples/`.

`rvt-capabilities --matrix -f text` prints the support matrix; [`docs/status.md`](docs/status.md) and [`docs/supported-profile.md`](docs/supported-profile.md) summarise it.

## Release 0.11.0 and main

The latest release is [v0.11.0](https://github.com/DrunkOnJava/rvt-rs/releases/tag/v0.11.0) (2026-10-30). `cargo install rvt --locked`, `pip install rvt`, the release archives and the container image install it, and "Upgrading from 0.10" in the [changelog](CHANGELOG.md) says what a user of 0.10 has to change: Rust code that names a removed item, scripts that expect an empty model with exit status 0 from a model over the resource limits, and anyone who wants doors and windows drawn as boxes. What 0.11.0 changed, in brief:

- **Large models ([#1075](https://github.com/DrunkOnJava/rvt-rs/issues/1075)):** a stream over the reader's resource limits is named in the export diagnostics, and `rvt-ifc` and `rvt-gltf` exit with status 3 instead of writing an empty model. `--large-model` (streams up to 2 GiB), `--max-stream-bytes` and `--max-inflate-bytes` raise the limits, `--allow-partial` writes what was read, and the viewer offers a retry with the large-model limits. Measured by lowering the limits below the reference files' streams (Core Interior's 17 MB partition against an 8 MiB limit); no model over the default limits, including the reporter's private ones, has been read with `--large-model` yet.
- **Files that kept an older schema ([#421](https://github.com/DrunkOnJava/rvt-rs/issues/421), puzzbobb's #1081):** a project saved by Revit 2024 to 2027 that carries the schema of the release it passed through is read as that release, where it read as zero elements; the diagnostics say when a file is read as another release than the one that saved it. Not yet measured on such a file: the reporter's are private, and none of the reference or sample files keeps an older schema.
- **Phases ([#328](https://github.com/DrunkOnJava/rvt-rs/issues/328), RE-184, puzzbobb's #1074):** elements that do not exist in the last phase, which Revit's export shows by default, are left out as Revit's export leaves them out; the phase order is read on Revit 2015 to 2027, and none of Autodesk's 2015 to 2023 samples has an element to leave out (RE-281).
- **Doors and windows:** drawn from the meshes Revit saved for them where the whole mesh decodes, every vertex within 0.001 ft of Revit's export on Core Interior's 132 doors and 6 windows and the BIM4LCA office's 269 windows (`--no-saved-meshes` keeps their boxes); a window whose family cuts a solid gets Revit's opening (RE-225).
- **Quantities and properties:** a wall nothing joins to or cuts carries Revit's `Height` (RE-266), and its side areas and volumes where its body is its record's box (RE-270); `IsWired`, `NumberOfStoreys` where it could differ from Revit's, and `Reference` outside the sets a Revit export confirms are no longer written; a one-storey project gets its storey.
- **Releases:** Revit 2027 is in the family corpus (RE-264). Revit 2008 to 2013 files open, and 2008 to 2014 projects export their elements experimentally (RE-259, RE-260; puzzbobb's #1076 and #1077), outside the 1.0 range of 2015 to 2027.
- **Breaking:** the stable API nothing used is removed, each item listed under "Removed" in the changelog with what to use instead. The crates.io package carries only what builds the crate, and the format probes are their own crate, `rvt-probes`.

The browser viewer is built from `main`, so it runs what `main` has; each change `main` adds after 0.11.0 is a fragment in [`changelog.d/`](changelog.d/README.md), which the next release moves into [`CHANGELOG.md`](CHANGELOG.md). To use `main` before the next release, build from source ([`docs/install.md`](docs/install.md#build-from-source)).

Rust 2024 edition (MSRV 1.87). **Nineteen CLIs ship** (`rvt-analyze`, `rvt-info`, `rvt-inspect`, `rvt-schema`, `rvt-history`, `rvt-diff`, `rvt-corpus`, `rvt-dump`, `rvt-doc`, `rvt-ifc`, `rvt-ifc-compare`, `rvt-write`, `rvt-gltf`, `rvt-sheet`, `rvt-elem-table`, `rvt-elements`, `rvt-capabilities`, `rvt-schedule`, `gen-fixture`) plus the reproducible probes under `probes/examples/`, one or more per format finding. Python bindings via pyo3+maturin in the `rvt-py` workspace member (so the core `rvt` crate can stay unconditionally `#![forbid(unsafe_code)]`) — `pip install rvt`.

## What works today

| Layer | Status | Notes |
|---|---|---|
| OLE/CFB container open | ✓ | No Revit required |
| Truncated-gzip stream decode | ✓ | |
| `BasicFileInfo` metadata | ✓ | Version, build, GUID, original path, plus the `Key: value` block every release 2016-2027 writes: worksharing state, central model path, last-saved-by user, save counter, single-user-cloud flag (`rvt-info`, `rvt::metadata`) |
| `PartAtom` XML | ✓ | Title, OmniClass code, taxonomies |
| Stream preview extraction | ✓ | Clean PNG, wrapper stripped |
| `Formats/Latest` schema parse | ✓ | the whole page-stripped schema: 4,126 classes and 11,562 fields on Revit 2024, 134,794 fields across the 12 releases (#410) |
| Field-type classification | ✓ | 9 to 12 fields per release unclassified out of 10,000 to 12,000, all listed in the CI gate (`tests/field_type_coverage.rs`, #410); the 100% figure before #410 covered only the first 64 KB of each schema |
| Cross-release tag-drift table | ✓ | First public 122×11 dataset |
| Layer 5a ADocument walker | partial | Reliable on Revit 2024–2026; 2016–2023 entry-point detection pending |
| Stream-level modifying writer | ✓ | 13/13 streams byte-preserving; `rvt-write` CLI + JSON patch manifests |
| Field-level semantic writer | pending | Gated by [ADR-002](docs/decisions/ADR-002-semantic-write-api-gate.md); no stable API until decoder/validation evidence is strong enough |
| Layer 5b per-class decoders | partial | **81 classes are registered in `elements::all_decoders()`, each through `SimpleDecoder`**; no test exercises them (their synthesized-byte suites were deleted in #573). Real project elements come from partition element records instead (next section); schema-field decoding of Wall/Floor/Door/Window from `Global/Latest` stays open (RE-01, RE-19). |
| IFC4 STEP export — spatial tree | ✓ | `IfcProject` + `IfcSite` + `IfcBuilding` + `IfcBuildingStorey` + OmniClass classifications |
| IFC4 STEP export — elements | partial | Revit 2023, 2024 and 2025 projects, and 2015 to 2022, 2026 and 2027 projects experimentally: typed entities from element records, measured against Revit's own export where there is one ([next section](#what-rvt-rs-reads-from-real-projects), and the table [by input](#what-you-can-rely-on-by-input) for 2015 to 2023, 2026 and 2027). Revit 2014 and earlier: the spatial scaffold. Low-confidence `HostObjAttr` candidates are never exported as proxies. |
| IFC4 STEP export — geometry | partial | Extrusions of arbitrary closed profiles with voids, sloped extrusions, stepped stair flights and B-rep solids. What each category gets on real files, and how exact it is, is in [the next section](#what-rvt-rs-reads-from-real-projects); anything not decoded is its record's bounding box. |
| IFC4 STEP export — materials | ✓ | Single-material via `IfcMaterial` + `IfcRelAssociatesMaterial`; compound assemblies via `IfcMaterialLayerSet` + `IfcMaterialLayerSetUsage`. On Revit 2024 and 2025 files, walls, floors, roofs and ceilings carry their type's layers with material names and thicknesses (RE-58). |
| IFC4 STEP export — properties | ✓ | `IfcPropertySet` + `IfcPropertySingleValue` with typed values (`IfcText`, `IfcInteger`, `IfcReal`, `IfcBoolean`, `IfcLengthMeasure`, `IfcPlaneAngleMeasure`, `IfcAreaMeasure`, `IfcVolumeMeasure`, `IfcCountMeasure`, `IfcTimeMeasure`, `IfcMassMeasure`) wired via `IfcRelDefinesByProperties`. |
| IFC4 STEP export — openings | partial | `IfcOpeningElement` + `IfcRelVoidsElement` + `IfcRelFillsElement`: each door and window cuts its host wall. The host is the wall Revit fills the opening in for 138 of 138 on Core Interior and 192 of 194 on Snowdon Towers, where the other 2 take another wall Revit also cuts (RE-23, RE-85); on Revit 2024 and 2025 it is the wall the element's native record names, Revit's for all 463 of the BIM4LCA office's doors and windows set in a wall (RE-245). The opening is as deep as the host wall. On Revit 2024 and 2025 it is the outline the symbol that draws the door or window cuts, placed by the instance's transform (RE-215, #227): all 138 of Core Interior's openings and RE1 Architecture's 4 are within 0.001 ft of Revit's (Measure run 40927543524). Otherwise it is the door's or window's box: within 0.26 ft on 124 of Snowdon's 194 (RE-83, local only). A window whose type stores its Width, Height and Default Sill Height takes Revit's opening from them, centred on its origin, and its `OverallWidth` / `OverallHeight`: 56 of Snowdon's 68 windows, each within 0.001 ft of Revit's (RE-93). A door whose body is its type's Rough Height tall takes its type's rough opening the same way: 57 of Snowdon's 126 doors, 54 within 0.001 ft of Revit's (RE-94). Any other door whose type stores its Width and Height carries them as its `OverallWidth` / `OverallHeight`: all 132 Core Interior doors and all 5 of RE1's, equal to Revit's (RE-187; Measure run 40927543524). Otherwise it is not Revit's opening profile, and in walls that are not straight extrusions it stays the element's box (#227). |
| Geometry extraction | partial | See the next section: walls from their centreline, joins and arcs; slabs and roofs from their sketches; sloped roofs as their planes (sheds, gables, hips, slope arrows); columns cut by walls; beams along their axes; stair flights as treads and risers. Doors, windows, furniture and most families are boxes. |
| glTF 2.0 binary export | ✓ | `model_to_glb()` produces a valid `.glb` file that loads in Three.js's `GLTFLoader`, each element drawn as its IFC body: rotated, with its sketched or steel profile, or as its swept, revolved or brep solid. `rvt-gltf` CLI. |
| CSV schedules | partial | `rvt-schedule`, the viewer's Schedule panel and `RevitFile.schedule_csv()` write element and room schedules for Excel / Sheets; rows are exactly what decodes (typed on Revit 2024 projects with element records). |
| 2D plan-view SVG export | ✓ | `render_plan_svg()` produces per-category-coloured SVG (walls black, doors amber, columns red, …), each element drawn as its body's plan outline, rotated and with holes open, with slabs beneath the walls. `rvt-sheet` CLI. |
| Browser viewer | ✓ | Live at <https://drunkonjava.github.io/rvt-rs/>. WebAssembly build of the core library + Three.js + Vite. Zero-upload, in-tab parse, Export glTF/IFC/SVG buttons, URL-based share via `share::ViewerState`. |
| Fuzz-regression harness | ✓ | 11 libFuzzer targets, run nightly, and 64 adversarial regression cases in `tests/fuzz_regressions.rs`, run by CI on every pull request. Caught a real `gzip_header_len` bounds bug on 9-byte truncated headers. |

## What rvt-rs reads from real projects

On Revit 2024 and 2025 project files, rvt-rs reads each element from its partition record: its ElementId, `BuiltInCategory`, bounding box and references, for a family instance its family and type, and for a wall, floor, ceiling, roof or railing its type's name. Every number below compares rvt-rs's IFC with Revit's own IFC export of the same file, element by element (the IFC `Tag` is the ElementId).

| Elements | IFC | Against Revit's own export |
|---|---|---|
| Walls | `IfcWall`, runs trimmed at their joins; a 2024 or 2025 wall its type's thickness either side of its centreline, running through or stopping at a butt join as its join lists say, layer by layer where both walls are layered alike, and stopping layer by layer where it ends part way along another wall, on a slanted line where the walls meet at an angle; a curved wall the ring sector along its arc; a tapered wall its cross-section, leaning on its exterior face, along its line; a wall's top at the underside of the floor, roof or footing its top join or a cut pair names, where that element covers the whole of the wall's plan | Core Interior 360 of 360, all world-exact (RE-127); the BIM4LCA office 482 of 607 with both ends Revit's, T joints, orientations and type cores read from the native records (RE-222, RE-223); Snowdon Towers 1,078 of 1,078, faces exactly Revit's on 1,030 of them (RE-54, RE-75, RE-86), both ends on 724 (RE-70 to RE-75; curved walls scored by radius and angle); the BIM4LCA office's wall tops Revit's on 64 of the 65 it cuts as built and 70 of 70 at building permit, a wall a floor covers in part (29 and 26) keeping its full height where Revit steps the top (RE-267) |
| Doors, windows | `IfcDoor`, `IfcWindow`, each filling an opening in its host wall | Core Interior 132 and 6; Snowdon Towers 132 and 68 |
| Floors, building pads | `IfcSlab` with its sketched plan profile | Core Interior 80 of 80, every one with its sketched outline, and its 20 shading-device plates with Revit's outline (RE-95); Snowdon Towers 176 of 199 with their outline, curved edges and shaft openings included, 154 with Revit's area (RE-96 to RE-100) |
| Columns | `IfcColumn`, minus what the walls cut; a square hollow steel section drawn as it, rounded corners and all | Core Interior 256 of 256, world-exact; the BIM4LCA office's 79 hollow-section columns are Revit's body (RE-221) |
| Roofs, ceilings | `IfcRoof` with its sketched outline, a sloped roof as its planes (shed, gable, hip, slope arrow, rafter cut; RE-56, RE-199), a Revit 2025 roof by extrusion as its sketch's section (RE-211); `IfcCovering` with its sketched outline (RE-98) | Snowdon Towers 20 and 68; 13 roofs carry their outline, 11 with the area of Revit's own roof (RE-50); a shed roof's slope, thickness and outline reproduce its recorded height to 1e-9 ft (RE-56); on puzzbobb's Revit 2025 models 18 of 19 roofs have Revit's body, every vertex within 0.001 ft and the volume within 1e-4 (RE-199); the 19th, a roof by extrusion, is drawn as its section, all 1,187 of Revit's vertices within 0.001 ft of it and 1.00015 times Revit's volume (RE-211); 66 of 68 Snowdon ceilings and 6 of 6 RE1 ceilings with Revit's outline (RE-98) |
| Stairs | `IfcStair` aggregating its flights, landings and stringers, with `Pset_StairCommon` / `Pset_StairFlightCommon` riser count, riser height, tread length and `TreadLengthAtOffset`, a flight's number of treads, `TreadLengthAtInnerSide`, nosing length and run type as its `Reference`, a landing's type and a stair's as their `Reference` and a stair's nosing length where its runs share one, its `TreadLengthAtInnerSide` and, with `--revit-set-placement`, its flight set from its flights (RE-287) (Revit 2024, and on Revit 2025 the same dimensions from the stair's and run's native records, RE-236; not `WalkingLineOffset`, which Revit's export gives as half the run's width in feet read as millimetres), and each stair's and flight's `PredefinedType` from its runs and landings; straight flights with separate treads and risers, and monolithic ones, drawn as their steps | Snowdon Towers 27 stairs, 43 flights, 17 landings, every aggregate relation one Revit's has; every stair's riser and tread values are Revit's (26 of 26), and flights' on 37 of 43 (RE-47); 38 flights drawn as their steps, 33 of them equal to Revit's own geometry (RE-52, RE-92); on Revit 2025, puzzbobb's R2025 stair models' two straight flights with treads and risers and their monolithic one have Revit's volume, every vertex of Revit's on rvt-rs's (RE-217); BIM4LCA office (Revit 2024), 34 of 35 monolithic flights equal to Revit's, its spiral flight drawn as its 18 treads with Revit's volume (RE-217 §11), every stair's and flight's `PredefinedType` Revit's (16 of 16 and 35 of 35, RE-217 §10, §12), and its 21 monolithic landings drawn as their slabs with Revit's top and outline, 100.4% to 104.7% of Revit's volume (RE-214), and its one landing that is not monolithic as the tread it is, Revit's body (RE-214 §7); the office's flights' number of treads Revit's on 34 of the 35 rvt-rs writes, their tread lengths, nosing length and `Reference` on 35 of 35, its stairs' nosing length, `TreadLengthAtOffset` and `TreadLengthAtInnerSide` on 14 of 14 and their flight set on 111 of the 112 values Revit's export writes (RE-287), and its 22 landings' `Reference` on all 22, none different; on the five R2025 stair models 10 or 12 more of each model's stair and flight dimensions Revit's, none different (RE-236) |
| Curtain walls | `IfcCurtainWall` aggregating its panels and mullions, with no body of its own; a panel that holds a basic wall is an `IfcCurtainWall` with its body | Snowdon Towers 60 of 60 and RE1 1 of 1; all 2,075 of Snowdon's panel and mullion relations are Revit's own (RE-46, RE-62, RE-72); 1,019 turned or tilted mullions and panels are the box along their own axes, Revit's extents on every one (RE-106) |
| Railings, curtain panels, mullions | `IfcRailing`, `IfcPlate`, `IfcMember` | Snowdon Towers 131 railings, every mullion |
| Furniture, casework, plumbing, lighting, equipment | `IfcFurniture`, `IfcSanitaryTerminal`, `IfcLightFixture`, `IfcElectricAppliance`, proxies | RE1 models and Snowdon Towers, every one Revit exports |
| Ducts, pipes, fittings, air terminals | `IfcDuctSegment`, `IfcPipeSegment` drawn as its cylinder where its ends are proven (RE-131), their fittings, `IfcAirTerminal` | RE1 Mechanical and Plumbing, every one; all 69 pipes are Revit's cylinder, its radius, axis ends and length to 0.00001 ft, and so are the BIM4LCA office's 12 (Revit 2024) |
| Electrical and mechanical devices and equipment | `IfcAlarm` (fire alarm devices), `IfcElectricAppliance` (data devices), proxies (switches, sensors, receptacles, junction boxes, panels, mechanical equipment) | RE1 Electrical and Mechanical, every one Revit exports and nothing else; the symbols nested in switches and receptacles left out, as Revit leaves them out (RE-79) |
| Structural framing, columns, foundations | `IfcBeam` along its location line, `IfcColumn`, `IfcFooting` | Snowdon structural sample: all 1,078 in the category its VIM export gives them; 923 of 942 beams run along their line, with Revit's own section on 613 of the 839 a later edition kept unchanged, and on 179 more less the top a floor join cuts (RE-49); 377 steel beams and 40 steel columns are their type's I section, Revit's own on all 271 beams and 24 columns the VIM scores (RE-103, RE-104) |
| Site elements, generic models, slab edges, elevators, ramps | proxies, `IfcTransportElement`, `IfcRamp` | Snowdon Towers, every one Revit exports |
| Rooms | `IfcSpace` named as Revit's export names it, `Name` the room number (for a room with no number, `<name>:<ElementId>`, RE-196) and `LongName` the room name, with the room's ElementId as the `ElementId` property and the outline of the solid Revit stores for the room (RE-101) | Core Interior all 116 and RE1 Architecture all 11 at Revit's own area and outline (RE-101's run of `tools/re/plan_profiles_vs_ifc.py --class IfcSpace`; neither CI nor Measure scores room outlines); Snowdon Towers 45 of 54 at the area its VIM export gives; on Revit 2023, Exemplo_data all 9 (RE-102); every room with Revit's number and name, Core Interior 116, Snowdon Towers 54 (against the VIM), RE1 11 and Exemplo_data 9 (RE-117), and the BIM4LCA office 264 of 264 as built and at building permit, 43 of them with no number (RE-196); a room whose solid does not close keeps its box |
| Areas | `IfcSpace .SPACE.` for each placed Area of a Revit 2024 file, named as Revit's export names it (`Name` the number, `LongName` the name), in its Level's storey and typed by its own `IfcSpaceType`; no body, since the Area's boundary is not decoded | the BIM4LCA office as built and at building permit: all 30 of Revit's area spaces, number, name and GlobalId as Revit's (RE-191) |
| Storeys | `IfcBuildingStorey` for each Revit Level that is a building story, with its name and elevation | Core Interior 15, Snowdon Towers 18, every RE1 model 2; name, elevation and GlobalId all Revit's (RE-24, RE-51); Snowdon Towers Structural 12 of its 19 Levels, its VIM export's building stories, 208 of the 210 elements on them in their storey (RE-118) |
| Storey containment | each element in the storey of the Level its record names, of its base constraint where it names two, and, where it names none, of a railing's host, of the Level its work planes carry, or of its base elevation | Core Interior all 854 in Revit's storey; Snowdon Towers 5,949 in Revit's storey, 1 in another and 1 on none (RE-27, RE-59, RE-60, RE-68) |
| Names | `Family:Type:ElementId`, `ObjectType` `Family:Type`, for family instances and, since RE-63 to RE-65, walls, curtain walls, floors, ceilings, roofs, railings, slab edges, ramps, curtain panels that are walls, and stairs with their runs, landings and supports | 4,397 family instances named exactly as Revit names them; Snowdon's 1,077 walls, 20 roofs and 68 ceilings and Core Interior's 460 system-family elements all named as Revit names them (RE-63); Snowdon's 18 panel walls and 59 slab edges too (RE-64), its 256 stair elements (RE-65) and every stair, flight and stringer of the five R2025 stair models (RE-65 §6), all 131 of its railings (RE-66), its 7 model texts (RE-67) and its 258 wall sweeps (RE-69); the BIM4LCA office's 220 curtain walls, their type a curtain wall type's native name (RE-230) |
| GlobalIds | the GlobalId Revit's own exporter gives each element, room and storey; every other entity one derived from the document and its own identity, never its place in the file (#400) | every element rvt-rs exports that Revit's export holds: Core Interior 854 (plus 116 rooms and 15 storeys), the RE1 models 319, Snowdon Towers 5,945 (RE-48); on Revit 2023, Exemplo_data 37 (plus 9 rooms and 2 storeys) and modelo_bim 37 (plus 4 storeys) (RE-108); Snowdon Towers exported before and after RE-75 keeps all 20,149 GlobalIds (#400) |
| Wall, floor, roof and ceiling layers and colours | glTF and the viewer: a wall drawn as its layers (2024; 2025 since RE-55), and a floor, roof or ceiling as its layers stacked top first (RE-57), each in its material's shading colour | Snowdon Towers 883 of the 1,054 walls whose layers are read; every layer within 0.001 ft of Revit's on 851 of 862 walls, and all 1,368 layer colours Revit's (RE-53); floors, roofs and ceilings: every layer's bottom and top Revit's on Snowdon's 236 and Core Interior's 68 (RE-57) |
| Material names and IFC layer sets | each material's name, by ElementId; `IfcMaterialLayerSetUsage` over an `IfcMaterialLayerSet` for layered walls, floors, roofs and ceilings, each layer with its material and thickness | materials named: Snowdon 220 of 220, Core Interior 86 of 86 (RE-125), the Revit 2026 house 148 of 148, RE1, Projeto1, `teste_export_2025` and the tutorial house all; layer names Revit's, in Revit's order, on 3,321 of Snowdon's 3,322 layers and all of Core Interior's and RE1's; sets on Revit's bodies within 0.001 ft on 1,196 of 1,207 Snowdon elements, 403 of 403 Core Interior (RE-91), 15 of 15 RE1 (RE-58); a layer taking its category's material takes the one the category's graphics styles name where the object styles leave it unset, every R2025 wall's "Default Wall" (RE-206); a wall whose body is not a layer set gets Revit's `IfcMaterial` or constituent set of its layers, 95 of Snowdon's 111 (RE-88) |
| Family instances' materials | `IfcMaterialConstituentSet` of the materials the type's geometry uses, from its geometry-material map; where the map holds no material, the materials the instance's own parameters name, else `<Unnamed>` (RE-149, RE-200) | the same set as Revit's on 1,331 of Snowdon's 1,371 family instances with a map, Core Interior's 132 doors, all 43 on RE1 Architecture; the differences RE-82 left, nested components and planting, are Revit's since RE-203 and RE-202. In the GLB and viewer, an instance drawn in one material takes its colour: Revit's on all 290 on Snowdon and 38 on RE1. On Revit 2023, Revit's set on 17 of Exemplo_data's 18 and all 27 of modelo_bim's (RE-113). Instance parameters: Revit's set on all 9 window trims and muntins of each R2025 model 12 to 17 (RE-200). Read from the native records since RE-201 to RE-206 (mullions' and panels' type material, off-centre columns' joined-wall layer, rendered content `<Unnamed>`, the type's map by name, hidden and nested forms): Revit's set on all 5,134 Snowdon elements given materials, all 71 on RE1 Architecture and 49 on RE1 Electrical |
| Type parameters | `Type Mark`, `Description` and `Fire Rating` of each element's type, in its property set | Snowdon Towers: Type Mark on 1,257 of the 1,276 elements its VIM export checks (19 empty there), Description 70 of 70, Fire Rating 696 of 696 against Revit's IFC4 export; RE1 Fire Rating 7 of 7 (RE-77) |
| IsExternal | `IsExternal` in each element's common property set, as Revit's exporter sets it: a parameter named for the set, else the type's Function or the element's class | BIM4LCA office: 2,110 of the 12,375 values Revit's export writes as built and 2,118 of 12,420 at building permit, none different, curtain mullions' and panels' counted apart (the office's exporter caches them, RE-269) and written by their curtain wall's value (RE-288); the reference and R2025 models 245 of 310, none different (RE-193), and RE1 Architecture's 12 curtain members equal. On Revit 2015 to 2023, experimentally, a wall's from its type's Function and any parameter named for it: Revit's value on all 114 walls of BIMcollab's Revit 2015 export and all 4 of modelo_bim's, none different, and the Function equal to the same project's later copy on 1,982 of 1,983 walls (RE-283) |
| Mullion span | `Span` in each curtain mullion's `Pset_MemberCommon`: its length along its own axis, as Revit's exporter takes it from the mullion's extrusion | BIM4LCA office: 9,984 of the 10,003 values Revit's export writes as built and 10,012 of 10,031 at building permit, 1 different on each; RE1 Architecture's IFC2X3 export writes none on its 10 mullions, rvt-rs's IFC4 does (RE-226) |
| Railing height | `Height` in each railing's `Pset_RailingCommon`: its type's top rail height | BIM4LCA office: all 41 values Revit's export writes as built and at building permit, RE1 Architecture's 1 and both on each of the five R2025 stair models (RE-231) |
| Walls' extension to structure | `ExtendToStructure` in each wall's `Pset_WallCommon`: whether its native record joins its top to a roof or floor | BIM4LCA office: all 607 values Revit's export writes as built and all 589 at building permit, RE1 Architecture's 7 (RE-228). On Revit 2015 to 2023, experimentally, the same record gives it: equal to the same project's later copy on all 1,983 walls of 24 pairs, and Revit's value on all 114 walls of BIMcollab's Revit 2015 export and all 4 of modelo_bim's, which are all false (RE-284) |
| Base quantities | IFC4's `Qto_` sets: each curtain mullion's `Length` and, where Revit's export draws it as an extrusion of its section, its `NetVolume` (RE-243), each door's and window's `Width`, `Height` and `Area` where the symbol that draws it and its type agree on its size, each wall's `Width` (its type's thickness), its `Length` where its curve cut back at its joins is as long as its curve (RE-252), its `Height`, its body's depth, where its native record names nothing joined to or cutting it (RE-266) and on the walls joined to roofs or floors that the rule below takes (RE-293), its `GrossSideArea`, `NetSideArea`, `GrossVolume` and `NetVolume` where, besides, no step of the record embeds or sweeps it and its body is the record's box cut back at its joins, a rectangle, or the wall is joined to roofs or floors that lie clear of it and its record confirms its length or its centreline cut back at its joins is as long as its body (RE-270, RE-290), and the `Depth` of the openings its doors and windows cut (RE-244), each curtain wall's `Length`, its curve's, each beam's `Length` where it is drawn along its location line, each pipe and duct segment's `Length` and a pipe's `GrossCrossSectionArea` and `OuterSurfaceArea` from its cylinder, each stair flight's `Length` (its riser lines' span) and `NetVolume`, and the `Length` of each column joined to nothing, which Revit's export draws as an extrusion, and on such a column drawn as its type's hollow section its `CrossSectionArea`, `OuterSurfaceArea` and `GrossVolume` (RE-247), and on a column drawn as its record's box whose type is a plain rectangle, none of whose pairs, clean or cut, is cut by or crosses an end of it (measured on Revit 2024), its `CrossSectionArea`, `OuterSurfaceArea`, `GrossVolume` and `NetVolume` (RE-291), each floor's `GrossArea`, the area of the profile it extrudes (RE-249), and, where its type has one layer as deep as its extrusion, its `Depth` and `GrossVolume`, its `NetVolume`, that area times its type's layers' total thickness, where nothing cuts it, and, of more than one layer, its `Width`, that thickness (RE-295), and each ceiling's `Width`, its layers' thickness, on a ceiling of several pieces too (RE-250, RE-297), its `GrossArea`, the area of its sketch's outline less its holes, where its native record holds no join that cuts it (RE-297), and the `Width` of each wall Revit's export writes as a covering, its type's thickness (RE-297), each space's `Height`, its room's, where the room's record holds its solid's faces (RE-255), its `NetVolume`, its outline's area less its holes times the height of its box, where every vertical face of that solid runs from the floor to the ceiling and no face is slanted, on Revit 2024 and 2025 (RE-296), and the `Length` of each stair railing whose path is one line along its stair's runs, that line climbing at the stair's slope (RE-254); on by default, as in each of Revit's built-in IFC4 export setups, and left out by `rvt-ifc --no-base-quantities`, as Revit's exporter leaves them out with "Export base quantities" off (RE-242) | BIM4LCA office as built: 29,288 of the 39,388 quantities Revit's export writes on objects rvt-rs writes (leaving out the Revit defects: 10,003 mullions' `CrossSectionArea`, 382 door openings' `Width` and `Height` and 13 floors' `Perimeter`; and the 461 walls' `Length` where the wall's curve and its curve cut back at its joins differ, either of which Revit's export may write, RE-252), 1 different (the mullion whose `Span` differs), none it does not write; a joined column's `Length` and a curtain panel's `Width` are left out, as Revit's export leaves them off objects nothing tells apart; all 94 IFC2X3 pipe and duct lengths of RE1 Mechanical and Plumbing; Core Interior's and RE1 Architecture's exports were made with base quantities off ([#1016](https://github.com/DrunkOnJava/rvt-rs/issues/1016)) |
| Parameters named for a standard set's property | each project or shared parameter named `<set>.<property>`, or the property alone where no such parameter gives it (RE-227), for a property of one of IFC4's 278 property sets that apply to an entity, from buildingSMART's own definitions (`Pset_WallCommon.FireRating`, `Pset_WindowCommon.ThermalTransmittance`, ...), in that set where IFC4 applies it to the entity written or one of its supertypes, with the predefined type a scoped set needs: booleans, labels and text, counts and integers, lengths and thermal transmittance | BIM4LCA office: 3,843 more of the values Revit's export writes as built are written and equal (3,872 at building permit), fire and acoustic ratings, load bearing, U-values, glazing and stair dimensions among them; `Pset_DoorCommon.FireExit` on all 266 doors by its bare name; none differs but 55 `OccupancyNumber`, written as IFC4's `IfcCountMeasure` where the export has `IfcInteger`. What the export takes from its user-defined property set file is not written (RE-194) |
| Element parameters | `rvt-elements` and Python's `decoded_elements()`, on Revit 2023 to 2027 files, and experimentally on Revit 2015 to 2022 ones (RE-281): each element's saved parameters from its native record, named by the file's own definitions (family, shared, project and global parameters; built-in ones by id only), a family instance's, wall's, floor's, ceiling's and roof's type parameters after its own, with the SI value of a length, area, volume or angle and the name of the material or family type a value names. Not written to IFC beyond the sets above | the BIM4LCA office: every parameter named `<set>.<property>` holds the value Revit's export writes there, 7,606 of 7,606 as built and 7,592 of 7,592 at building permit; the R2025 windows' type `Fire Rating` 18 of 18 (RE-207); on Snowdon Towers the walls', floors', ceilings' and roofs' type name, assembly code, type mark and fire rating, 4,297 of 4,314 (RE-256) |
| Revit's set placement (`--revit-set-placement`) | a text parameter named `<set>.<property>` for a property the set's IFC4 definition lacks, in that set as `IfcText` (a room's `Pset_SpaceCommon.FloorCovering`, which IFC4 moved to `Pset_SpaceCoveringRequirements`), and the `Reference` of each set whose entity's name begins with the element's (`Pset_SpaceHeaterTypeCommon` on a space, `Pset_StairFlightCommon` on a stair), as Revit's exporter writes them; the default output follows buildingSMART's IFC4 definitions | BIM4LCA office: 966 more of the values Revit's export writes, as built and at building permit, none different and none Revit's IFC4 export does not write: 646 room coverings, 298 spaces' `Pset_SpaceHeaterTypeCommon.Reference`, and the `Reference` of 16 stair flights, 5 ramp flights and one air terminal box (RE-235; Measure runs 40927543524 and 40929095759). On RE1's IFC2X3 exports the option writes 18 values the export lacks (11 on Architecture, 7 on Mechanical) |
| Revit's own property sets (`--revit-property-sets`) | each element's named saved parameters in one set per parameter group, named for the group, and its type's on its type object, as Revit's exporter writes them with "Export Revit property sets" (revit-ifc `Release_25.x.x`): Booleans, integers, texts, lengths, areas, volumes, angles, numbers, air flows, pressures, powers, voltages, currents, material names and family types | Autodesk's Snowdon Towers HVAC against Revit 2025's export with Revit property sets (measured locally): 28,906 of 28,906 properties (counted per element) in the set of the same name with Revit's value; Revit writes 79,907 more, nearly all built-ins. Snowdon Architectural: 58,346 of 58,733; Revit names 13 type objects' sets with `(Type)`, as the order of its export has it. The BIM4LCA office as built: `FireExit` in `Identity Data` equal to Revit's `Pset_DoorCommon` and `Pset_WindowCommon.FireExit` on 498 of 498 doors and windows (RE-210) |
| System-family type names | `TypeName` in the property set of walls, floors, ceilings, roofs and railings | 2,028 of 2,029 equal to Revit's across Core Interior, Snowdon Towers, RE1, Projeto1 and `teste_export_2025` (Snowdon Towers, Projeto1 and `teste_export_2025` are local only). The other is a wall that export shows in a later state of the model (RE-44). On Revit 2023, walls', floors' and roofs' 24 of 24 (RE-111). The family in `Family:Type` names follows the locale the file was saved in, as Revit's export does: `Parede básica`, `Piso` and `Telhado básico` on the four files saved in Brazilian Portuguese, 32 of 32 (RE-123) |
| Type objects | `IfcWallType`, `IfcDoorType`, `IfcColumnType` and the rest, one per Revit type, named `Family:Type` with the type's ElementId as `Tag`, joined to their elements by `IfcRelDefinesByType` | every element Revit's export types, typed by the type Revit names: Core Interior 854 of 854, Snowdon Towers 5,803 of 5,812 (local only), RE1 Architecture 73 of 73; on Revit 2023, all 37 of Exemplo_data and all 37 of modelo_bim (RE-111, RE-112). Type GlobalIds are Revit's on all 854 of Core Interior's and on every RE1 element Revit types (RE-167; Measure run 40927543524). Their common sets carry `FireRating`, `LoadBearing` and a stair or flight type's dimensions as Revit's IFC4 exports write them: 34 of the BIM4LCA office's 182 type values as built and 29 of 154 at permit, none different; no `IsExternal` on a type object, which Revit's newer exporters do not write (RE-286) |
| IFC export overrides | the entity and predefined type set by Revit's "Export to IFC As" and "IFC Predefined Type", the element's own or its type's | Core Interior 20 shading devices and 2 `.ROOF.` slabs; `teste_export_2025` 3 walls as `IfcCovering .CLADDING.`; the BIM4LCA office's waste, fire suppression, sanitary and air terminals and electric appliances (185 of 185 as built); each as in Revit's export (RE-45, RE-190) |

Across Autodesk's Snowdon Towers architectural sample (Revit 2024), rvt-rs exports 5,968 elements. 5,945 of them are in Revit's own export (local only: that export cannot be redistributed, and the figures are from #309 and #328, not a committed report), which is every element Revit exports in these entities. The other 23 are curtain panels with no resolved type, which Revit's exporter leaves out (#309). The 6 slabs in the Legends phase that rvt-rs wrote before are no longer written, because they exist in no phase after Legends (#328, RE-184). Doors and windows whose type draws no geometry are written as their opening alone, as Revit writes them (RE-84). Elements in a design option set's non-primary options, elements that do not exist in the project's last phase (RE-184), 2D-only families and empty curtain panels are left out, as Revit leaves them out, and the export diagnostics count them. Rooms are not checked against the phase (see the gap below).

Core Interior and the RE1 models are licensed test files and run in CI. The BIM4LCA office and puzzbobb's Revit 2025 models are fetched by Measure. Snowdon Towers, the 255ribeiro projects, the 2026 house and BIMcollab's 2015 example have no licence to redistribute and are measured locally (BIMcollab's and the holdout pack's summary counts leave Measure's runner), so those figures cannot be re-run by others. By ElementId the exports equal Revit's on Core Interior (970), RE1 Architecture (84), Electrical (49) and Mechanical (74), the 22 R2025 models with an export and the office as built (12,974); RE1 Plumbing writes a water closet tank (442378) and the office at building permit a column (1927742, #792) that Revit's export leaves out, and one office fixture (3095244) is in another IFC class than Revit's (#792) (Measure run 40927543524). Parts Revit's exporter splits one element into under one ElementId (a roof or slab, #323) count once.

All of the above were used to develop the decoder. Three models that were not, Autodesk's Snowdon Towers Plumbing, HVAC and Electrical samples (2024), were measured once as they are against the VIM export of the sample: every exported element the VIM holds is in the expected category (10,385), every VIM instance both hold is exported, and all storeys and GlobalIds agree ([held-out report](reports/validation/held-out-snowdon-mep-2026-09-27.md), #408). Their bodies are bounding boxes.

## What does not work yet

| Gap | Status | Evidence |
|---|---|---|
| Element extraction from real `.rvt` project files | **partial** | Revit 2024 and 2025 projects decode their element records into typed IFC elements; see [What rvt-rs reads from real projects](#what-rvt-rs-reads-from-real-projects). Revit 2023 projects decode their elements' identity, category and box (RE-81), and Revit 2026 projects their element records experimentally (since 0.5.0), measured on one project (RE-124). Revit 2015 to 2022 and 2027 projects decode their element records experimentally (since 0.6.0), measured against the same models saved by a neighbouring release, not against a Revit export (RE-179 to RE-181). Earlier releases open and report metadata and schema; their element records are not decoded. The per-finding history, RE-21 to RE-43, is in [`docs/status.md`](docs/status.md) and `reports/element-framing/`. |
| 81 registered classes wired into walker | partial | `MVP_TYPED_CLASSES` are preferred via their registered decoder in `iter_elements` (fail closed). The remaining registry entries still use generic `decode_instance`. ArcWall uses a separate partition decoder. |
| Element geometry beyond what is listed above | partial | Where the bytes are not decoded, a body is the element record's bounding box, and the export diagnostics say so. Doors and windows are drawn by default from the mesh Revit saved for display, on Revit 2023 to 2027, where the whole mesh decodes (every vertex within 0.001 ft of Revit's on all 132 of Core Interior's doors and 6 windows and all 269 of the BIM4LCA office's windows, Measure run 40932136360; `--no-saved-meshes` keeps their box). Other loadable families stay boxes by default, turned with the instance where it is turned (RE-87); `--saved-meshes` draws them from their saved mesh too (RE-78: within 0.01 ft of Revit's export for 158 of 158 on Core Interior, 207 of 214 on RE1 (Revit 2025) and 3,332 of 3,736 on Snowdon Towers); ducts and pipe fittings, though pipes are cylinders where their ends are proven (RE-131); openings across and in height, which are the door's or window's box cut to the wall's depth (#227). Not decoded: roofs by extrusion outside Revit 2025 (RE-211) and hip roofs over a T or U footprint (#884), monolithic spiral stair flights and straight riserless ones with a nosing (#357), edited wall profiles and some wall ends (#358: both ends match Revit on 716 of Snowdon's 1,078 walls), the height of 78 of the BIM4LCA office's 579 walls, which Revit's export draws cut at one end, mostly at a floor the wall's top is joined to, and rvt-rs as tall as their record box (#1067, RE-257, RE-258), and the boundaries of 7 of Snowdon's 54 rooms, whose stored solids do not close at the floor (#90). A slab sketched as separate pieces exports one element per piece, as Revit's export does (#331). |
| Rooms in an earlier phase | not applied | Revit's exporter leaves out a room whose own phase parameter (`ROOM_PHASE`) is not the export phase, and exports an area or an MEP space whatever its phase (revit-ifc `ElementFilteringUtil.IsRoomInInvalidPhase`, called from `Exporter.ExportSpatialElements`). No model here shows that parameter to be the room's created phase, so rvt-rs checks no room against the phase and writes a room created in an earlier phase than the last, which Revit's export may leave out (RE-184) |
| Community corpus open/scaffold verification | executed (scaffold only) | `tools/fetch-corpus.sh` + `probes/examples/probe_corpus_batch_validate.rs` reported **222/223** real files passing open → schema → scaffold IFC (`docs/corpus-hunt-2026-04-21.md`). That measures container/schema/scaffold health, not typed element extraction. |
| Parameters and system-family type names | partial | Partition records are read through their record chain (RE-35), their frame prologue and reference lists, and the name entries of loadable families and types (RE-38, RE-42). The type names of system families (walls, floors, ceilings, roofs, railings) come from the type's own serialised data (RE-44, #322). Since RE-207, `rvt-elements` and Python list each element's saved parameters on Revit 2023 to 2027 files, but built-in parameters have no name (the built-in catalog is not shipped, #255), a family's built-in slots (a window type's Width and Height) are not resolved, a wall's or floor's type parameters are not joined to it, and only `--revit-property-sets` writes the rest to IFC (RE-210: on Autodesk's Snowdon Towers HVAC, all 28,906 properties written (counted per element) are Revit's; #35). |
| Scalar-Container wire format on real bytes | assumption only | The fix of the Scalar-Container decoder assumes Vector-equivalent layout for kinds 0x01/0x02/0x04/0x05/0x07/0x0b/0x0d. The round-trip tests use synthesized bytes and CI does not run them; no real-.rvt round-trip has been exercised. |
| Patched CFB roundtrip for grow/shrink cases | covered | Family corpus tests cover identity, grow, shrink, multi-stream, and missing-stream patches; project-corpus tests cover identity/grow/shrink/multi while preserving unpatched streams plus GUID/history. |

## Why the schema matters

The openBIM community, around [buildingSMART International](https://www.buildingsmart.org/) and the IFC standard, has long worked on getting building data out of Revit. Autodesk's [revit-ifc](https://github.com/Autodesk/revit-ifc) exporter runs inside Revit through the Revit API, so what it writes depends on what the API exposes and on how the export is configured; the [OSArch wiki's guide to setting up Revit for openBIM](https://wiki.osarch.org/index.php?title=Revit_setup_for_OpenBIM) covers the settings that matter. rvt-rs takes an independent route: it reads the file itself, so it runs where Revit is not installed, and its output can be checked against Revit's own export of the same file.

The schema work here, decoding the whole of `Formats/Latest` and classifying all but about ten field encodings per release across 11 Revit releases, is the dictionary a byte-level reader needs. The aim is an export that can also carry data the Revit API does not surface. Today the export reaches what the tables above measure, and the rest is open work ([`ROADMAP.md`](ROADMAP.md) and the [issues](https://github.com/DrunkOnJava/rvt-rs/issues)).

If you are building BIM or AEC tooling and want an Apache-2.0 Revit reader to build on:

- **Revit 2016 to 2027 files**: container open, stream decompression, metadata, previews, schema introspection (the whole schema, all but about ten field encodings per release classified) and stream-level byte-preserving writes.
- **Revit 2024 and 2025 projects** — typed elements with Revit's ElementIds, names, types, storeys, materials, layers and GlobalIds, as IFC4, glTF 2.0, plan SVG and CSV, each export with a diagnostics sidecar naming what was approximated or left out.
- **Revit 2023 projects** — typed elements with Revit's ElementIds and categories, each drawn as its bounding box (RE-81), walls cut back at their joins (RE-120), beams at their columns (RE-122), rooms as their outline (RE-102), on the storeys of Revit's own Levels (RE-107), family instances named as Revit names them (RE-109), walls, floors and roofs with their type, read from its layers (RE-111, RE-112).
- **Revit 2026 projects, experimentally (since 0.5.0)**: typed elements, storeys, rooms, doors and windows, type names and layer sets, measured on one project (RE-124).
- **Revit 2015 to 2022 and 2027 projects, experimentally (since 0.6.0)**: typed elements, measured against the same sample projects saved by Revit 2019 (for 2015 to 2018), 2023 (for 2019 to 2022) and 2026 (for 2027); Revit's own export is measured on one 2015 project (RE-198) and on none of the others (RE-179 to RE-181).
- **Revit 2008 to 2014 projects, experimentally and outside the 1.0 range (2015 to 2027)**: typed elements, measured on a local copy of Autodesk's 2013 `rac_advanced` sample against its 2015 copy (RE-259); 2008 to 2010 only on their own files' consistency, with storeys and named materials (RE-260). The pre-2015 files are closed source and measured locally only.
- **Not yet**: element records of Revit 2007 and earlier, most parameters, family geometry, and semantic editing of a Revit file.

[`tests/fixtures/synthetic-project.ifc`](tests/fixtures/synthetic-project.ifc) is a committed IFC from synthesized inputs, useful for testing a consumer without a Revit file. The [browser viewer](https://drunkonjava.github.io/rvt-rs/) runs the same pipeline in the tab: drop a 2023, 2024 or 2025 project, or a 2015 to 2022, 2026 or 2027 one experimentally, and Export IFC writes its typed elements; earlier releases give the metadata and the spatial scaffold. The viewer is built from `main`, not from a release.

## Quick demo

One command produces the full forensic picture — identity, upgrade history, format anchors, schema table, schema-to-data link histogram, content metadata, and a disclosure scan:

```bash
cargo build --release
./target/release/rvt-analyze --redact path/to/your.rfa
```

### From Python

```python
import rvt

f = rvt.RevitFile("my-project.rfa")
print(f.version, f.part_atom_title)      # 2024 "0610 x 0915mm"
print(f.read_adocument()["fields"][-1])  # {name: m_devBranchInfo, kind: element_id, tag: 0, id: 35}
open("out.ifc", "w").write(f.write_ifc())
```

Install: `pip install rvt` — or build from source with [`maturin build --release --manifest-path rvt-py/Cargo.toml`](docs/python.md#from-source). Full API + Jupyter notebook walkthrough: [`docs/python.md`](docs/python.md) and [`docs/rvt-python-quickstart.ipynb`](docs/rvt-python-quickstart.ipynb).
See [`docs/install.md`](docs/install.md) for cargo, PyPI, source, and viewer
install/smoke-test paths, and for the prebuilt Linux / macOS / Windows CLI
archives each GitHub Release carries (from [v0.2.0](https://github.com/DrunkOnJava/rvt-rs/releases/tag/v0.2.0) on).

### In the browser

Drop a `.rvt` / `.rfa` / `.rte` / `.rft` at <https://drunkonjava.github.io/rvt-rs/>: the file is read and converted inside the tab and is not uploaded. The viewer runs the core library compiled to WebAssembly by the `rvt-wasm` crate (`wasm-pack build rvt-wasm --target web --out-dir ../viewer/pkg --out-name rvt`, as [`docs/install.md`](docs/install.md) describes), parses in a dedicated worker, and renders 3D with Three.js. One-click buttons export the model as **glTF 2.0 binary**, **IFC4 STEP**, or **plan-view SVG**. URL state (camera pose + category filters) is shareable via the hash fragment. Multi-megabyte projects fit inside wasm32's 4 GiB linear-memory ceiling: each gzip member reserves at most 1 MiB before decoding and the decoded buffer is trimmed (#256), which is what lets the 33.7 MB Core Interior demo finish at 419 MB of linear memory instead of growing to 4057 MB in 139 ms and trapping. A Rust panic in the wasm build is reported through `console.error` with its message and source location instead of a bare `RuntimeError: unreachable`.

The landing dropzone also includes a **demo gallery** staged from [`docs/viewer-demos.json`](docs/viewer-demos.json) (license/provenance + expected quality labels). It opens two real projects, `Revit_IFC5_Einhoven.rvt` (2023) and `2024_Core_Interior.rvt` (2024) from the MIT-licensed [magnetar-io/revit-test-datasets](https://github.com/magnetar-io/revit-test-datasets), hash-verified at staging time, alongside the tier1 synthetics. Einhoven is 913 KB and opens in about half a second; Core Interior is 33.7 MB and decodes in about 3 seconds on the deployed site since #266 (about 7 seconds including the 32 MB download), reporting 889 entities and 854 elements carrying geometry. Two Playwright tests gate both cards, and staging refuses a file whose sha256 does not match the catalog. Demo bytes are same-origin static assets only.

The deploy workflow (`.github/workflows/deploy-viewer.yml`) checks the viewer's privacy posture on every build, in two ways. `viewer/scripts/audit-wasm-imports.mjs` reads each import of the compiled `.wasm` and its shim in the JavaScript glue, and fails if any reaches `fetch`, `XMLHttpRequest`, `WebSocket`, `EventSource` or `sendBeacon` (the check before #735 could not fail). A Playwright test opens a real project in the built viewer and fails on any request to another origin, or on any request at all after the file is open. The page's Content-Security-Policy also limits its connections to its own origin. Together these cover the WebAssembly module's imports, the page's connection policy and the flow the test drives; they are checks, not a proof about every possible code path. [`docs/viewer-privacy-posture.md`](docs/viewer-privacy-posture.md) states the invariant and what each check covers.

### Supported MVP workflow

The supported end-to-end shell is intentionally honest about partial decode:

1. Open a supported Revit file locally (drop, file picker, or a redistributable gallery demo).
2. Read the File status / confidence panel before trusting geometry.
3. Inspect decoded entities in the scene tree or by picking in the 3-D view.
4. Export IFC / glTF / plan only after checking the export-quality label.
5. Download diagnostics when the export is scaffold-only or partial.

What still depends on decoder work: element records of Revit 2014 and earlier (2015 to 2022's, 2026's and 2027's are experimental), and on 2023 geometry beyond each element's box, wall joins and beam cuts, layered or angled joins, design options and IFC export overrides; roofs by extrusion outside Revit 2025 or over a T or U footprint, family geometry and opening profiles; most element parameters; and compound layers of elements drawn whole or sloped, which get no layer set. **RE-19 / RE-20 (2026-08-29) closed negative** on the magnetar corpora: there is no Door vs Window discriminator *in the opening-index bytes* and no recoverable Level ElementId map there, so do not re-probe those without a new corpus or signal. Typed categories come from each element record's `BuiltInCategory` instead (RE-21 onward). On Revit 2022 and earlier, treat IFC export as the spatial scaffold plus diagnostics; on 2023 elements are their boxes with the joins and properties listed [by input](#what-you-can-rely-on-by-input), and on 2026 the export is experimental. See [`docs/status.md`](docs/status.md) and [`docs/supported-profile.md`](docs/supported-profile.md).

**Sample output** (all pre-scrubbed with `--redact`, committed for review):

- **One-screen teaser**: [`docs/demo/rvt-analyze-2024-teaser.txt`](docs/demo/rvt-analyze-2024-teaser.txt) — the four highlight sections fit in one terminal screen (identity, format anchors, schema-to-data linkage, disclosure scan)
- **Full terminal report**: [`docs/demo/rvt-analyze-2024-redacted.txt`](docs/demo/rvt-analyze-2024-redacted.txt) — 130 lines of structured output
- **JSON report**:    [`docs/demo/rvt-analyze-2024-redacted.json`](docs/demo/rvt-analyze-2024-redacted.json) — machine-readable version
- **Tag-drift heatmap**: [`docs/data/tag-drift-heatmap.svg`](docs/data/tag-drift-heatmap.svg) — visual proof of class-ID drift across 11 Revit releases

The `--redact` flag (on by default in every committed artifact) scrubs Windows usernames, Autodesk-internal paths, and project-ID folder names to `<redacted>` markers while preserving path shape so claims remain verifiable. Omit the flag when running privately against your own files.

## Results at a glance

Running the shipped CLIs against one 400 KB RFA fixture:

- **Metadata**: version, build tag, creator path, file GUID, locale, worksharing state, central model path, last saved (time and user), save counter (`rvt-info`)
- **Folder inventory**: one row per Revit file under a folder — release, worksharing, last saved — as a table, CSV, JSON or JSON Lines, reading only each file's two identity streams (`rvt-info <folder> -f csv`)
- **Atom XML**: title, OmniClass code, taxonomies (`rvt-info` parses `PartAtom`)
- **Preview**: clean PNG thumbnail, 300-byte Revit wrapper stripped (`rvt-info --extract-preview`)
- **Schema**: every class and field of the embedded schema with its typed encoding, 3,490 classes on the 2016 sample to 4,285 on 2026 (`rvt-schema`)
- **History**: every Revit release that ever saved this file (`rvt-history`)
- **Bulk strings**: 3,746 length-prefixed UTF-16LE records from Partitions/NN — Autodesk unit/spec/parameter-group identifiers, OmniClass + Uniformat codes, Revit category labels, localized format strings (`rvt-history --partitions`)

Every class and field name that `rvt-schema` extracts was cross-checked against the public `RevitAPI.dll` NuGet package's exported C++ symbol list. All top-level tagged class names we've inspected (ADocument, DBView, HostObj, LoadBCBase, Symbol, APIAppInfo, APropertyDouble3, ElementId, and the rest) appear in that export with their decorated signatures (e.g. `__cdecl NotNull<class ADocument *,void>::NotNull(class ADocument *)`), consistent with the on-disk schema naming the compiled classes.

A build-server path also appears in C++ assertion strings inside the same DLL; it is mentioned in the recon report for completeness and does not represent anything the reader extracts from .rvt / .rfa files.

### Performance on large projects

Partition streams are inflated once per file and cached on the `RevitFile` handle (#266). On the 33.7 MB `2024_Core_Interior.rvt`, `rvt-ifc --mode geometry` went from **26.07 s / 2641.4 MiB** peak RSS to **1.69 s / 490.1 MiB** (Apple Silicon, `/usr/bin/time -l`, best of three), and the exported IFC is byte-identical once the writer's two wall-clock stamps are normalised. In the browser the same file decodes in **about 3 s** on the deployed site, where it used to take about 28. Every `tools/perf_budget.py --require-category medium` row now passes with an order of magnitude of headroom, without any budget being loosened.

## Findings of the first reconnaissance

Six reproducible findings from the first reconnaissance (April 2026), all documented in [`docs/rvt-moat-break-reconnaissance.md`](docs/rvt-moat-break-reconnaissance.md) and reproducible with the programs in `examples/` and `probes/examples/`:

1. **The schema indexes the data.** Class names do not appear as ASCII in `Global/Latest`; class tags from `Formats/Latest` (u16 after class name, with 0x8000 flag set) occur ~340× the uniform-random rate. The top tag, `AbsCurveGStep`, appears 19,415 times in 938 KB of decompressed Global/Latest. [`examples/link_schema.rs`]

2. **Tags drift across releases** but are stable-sort-assigned. `ADocWarnings` = 0x001b 2016→2026 because no class sorted alphabetically before it has ever been added. `AbsCurveGStep` shifted 0x0053 → 0x0066 across the decade as 19 new A-class entries were inserted. Full 122-class × 11-release drift table: [`docs/data/tag-drift-2016-2026.csv`](docs/data/tag-drift-2016-2026.csv), visualised in [`docs/data/tag-drift-heatmap.svg`](docs/data/tag-drift-heatmap.svg). [`examples/tag_drift.rs`]

3. **Revit 2021 changed the format substantially.** On the sample families, Global/Latest grew about 27× (about 26 KB to about 715 KB) from 2020 to 2021, and the Forge Design Data Schema namespaces (`autodesk.unit.*`, `autodesk.spec.*`) first appear in Partitions/NN in the same release.

4. **Parameter-group namespace shipped separately in Revit 2024.** `autodesk.parameter.group.*` identifiers appear in 2024+ only — three releases after units/specs. Dating the Forge schema rollout from on-disk bytes: [`examples/tag_drift.rs`](examples/tag_drift.rs), [`src/object_graph.rs`](src/object_graph.rs).

5. **A stable Revit format-identifier GUID in family files.** `Global/PartitionTable` is 167 bytes decompressed in `.rfa` family files, and **165 of those bytes are byte-for-byte identical across every Revit release 2016-2026** (98.8% invariant). The invariant region contains a version-1 UUID, `3529342d-e51e-11d4-92d8-0000863f27ad`, whose timestamp field decodes to 2001-01-08. Useful for family-file detection. **Scope correction (2026-04-21):** this invariant is a *family-file* anchor, not a universal Revit-file anchor. Three real `.rvt` project files we probed carry three different GUIDs (`6a6261fd-...` on Revit 2023, `552368c6-...` on 2024, all-zero on 2025) in a shorter 87-byte `PartitionTable`. File-type sniffers using the family GUID will correctly reject non-family files but can't identify them. See [`docs/project-file-corpus-probe-2026-04-21.md`](docs/project-file-corpus-probe-2026-04-21.md). [`examples/partition_full.rs`]

6. **Tagged class record structure decoded.** Every tagged class declaration in `Formats/Latest` carries a tag word (u16 with 0x8000 flag), an optional parent class and a declared field count, followed by N field records, each with a name and a C++ type encoding. `HostObjAttr` resolves to parent `Symbol` with `declared_field_count=3` and all three field names (`m_symbolInfo`, `m_renderStyleId`, `m_previewElemId`) extracted byte for byte. RE-146 later showed that the tag word is the base class's tag and the word after it the grandparent's (#154). [`examples/record_framing.rs`, `src/formats.rs`]

Revit files, Autodesk's shipped samples among them, can carry authoring metadata: the path a file was saved from, a creator name in the `Contents` stream, and build-server paths in assertion strings of the public `RevitAPI.dll`. The values found are not repeated here.

**Downstream safety:** the `rvt-analyze` CLI has a `--redact` flag (used for all committed demo output in this repository) that rewrites creator paths, Autodesk-internal paths and build-server paths to `<redacted>` markers while preserving the surrounding structure. A tool that displays rvt-rs output publicly should do the same.

---

## Library surface

Three runnable examples show the library end to end: [`basics_inspect`](examples/basics_inspect.rs) reads what a file is, [`basics_elements`](examples/basics_elements.rs) lists its building elements by IFC type with their ElementIds, and [`basics_export`](examples/basics_export.rs) writes IFC4, glTF, a CSV schedule and the export diagnostics. `cargo run --example basics_inspect -- model.rvt` runs one on your file, and CI runs all three on the synthetic project the repository ships and on a real one, Einhoven (MIT). The other programs under `examples/` are research probes ([`examples/README.md`](examples/README.md)).

All modules build for native targets and for `wasm32-unknown-unknown`, which the `rvt-wasm` crate compiles for the browser viewer. The type documentation is on [docs.rs](https://docs.rs/rvt):

| Module | What it does |
|---|---|
| `reader` | Open any Revit file with `OpenLimits`, enumerate every OLE stream, fetch raw stream bytes, bounded reads |
| `compression` | Truncated-gzip decode (`inflate_at`, `inflate_at_auto`, `inflate_at_with_limits`) + multi-chunk (`inflate_all_chunks_with_limits`) + truncated-gzip encoder for write-back (`truncated_gzip_encode`) |
| `basic_file_info` | Version, build tag, GUID, creator path, locale — read path + byte-back encoder (`BasicFileInfo::encode`) |
| `part_atom` | Atom XML with Autodesk `partatom` namespace — title, OmniClass, taxonomies — read + encode |
| `formats` | Parse + encode `Formats/Latest` with `FieldType` classification (all but 9 to 12 fields per release classified across the 12-release corpus, #410; 2027's flagged fields by their base type, RE-264) |
| `walker` | Schema-directed instance walker + generic `decode_instance` + `detect_adocument_start` entry-point finder; candidates of the 7 classes in `elements::MVP_TYPED_CLASSES` go through their typed decoder from the registry (fail closed), and every other class decodes generically |
| `elements` | 81 `ElementDecoder` registry entries in `all_decoders()`, all `SimpleDecoder` (Wall, Floor, Door, Window, Column, Beam, Stair, Railing, Rebar, Room, Furniture, …), and typed views of Level, Wall, Floor, Door, Window, Room and Material; the walker uses the 7 `MVP_TYPED_CLASSES` decoders, and no test exercises the rest (#573). Real project elements come from partition element records |
| `geometry` | `Point3`, `Curve` (Line, Arc, Circle, Ellipse, Hermite spline, NURBS, cylindrical helix) and `CurveLoop`, and `recovery`, which reads wall location curves, floor boundaries, opening hosts and level elevations from the typed views |
| `object_graph` | `DocumentHistory`, string-record extractor for Global/Latest + Partitions/NN |
| `class_index` | Quick class-name inventory (BTreeSet) |
| `corpus` | Cross-version byte-delta classifier |
| `elem_table` | `Global/ElemTable` header and records: the declared ElementIds and each record's owner, in 28-byte records through Revit 2023 and 40-byte records from 2024 (#152, RE-41) |
| `partitions` | Partitions/NN 44-byte header decoder + gzip-chunk splitter |
| `writer` | Byte-preserving round-trip `copy_file` + `write_with_patches` (atomic temp-file rename, stream-hash verification) + GUID + history preservation |
| `round_trip` | Per-class encoder round-trip verification (`verify_instance_round_trip`) |
| `ifc` | Full IFC4 spatial tree + elements + materials + properties + openings + extrusion geometry + glTF 2.0 binary (`gltf::model_to_glb`) + plan-view SVG (`sheet::render_plan_svg`) + viewer data model (`scene_graph`, `camera`, `clipping`, `sheet`, `share`, `pbr`) |
| `streams` | Named constants for every invariant OLE stream in a Revit file |
| `redact` | Shared PII scrubbers for all CLIs (`--redact` flag) |
| (`rvt-wasm` crate) | the 12 JS-callable wasm-bindgen bindings powering the browser viewer ([`docs/viewer-api.md`](docs/viewer-api.md#the-javascript-bindings)), a crate of its own so `rvt` builds as a plain `rlib` |
| `error` | Structured error type (`Error` / `Result`) |

Runtime capabilities:

- Open any Revit file from disk (magic `D0 CF 11 E0 A1 B1 1A E1`)
- Enumerate every OLE stream; find the version-specific `Partitions/NN`
- Decompress any stream (truncated-gzip format — standard gzip header, no trailing CRC/ISIZE)
- Parse `BasicFileInfo`, `PartAtom`, extract preview PNG
- Read every tagged class record in `Formats/Latest` with its tag, parent and declared field count
- Decode a family file's 167-byte `Global/PartitionTable`, including its stable format-identifier GUID
- Decode a family file's 307-byte `Contents` stream, including its embedded UTF-16LE metadata chunk
- Produce a byte-for-byte round-trip copy of a `.rfa` / `.rvt` file
- Run across the 11-release family corpus in under 500 ms per file (release build, measured in April 2026; for large projects see [Performance on large projects](#performance-on-large-projects))

**Nineteen CLIs** ship in the box:

```bash
cargo build --release

# One-shot forensic analysis — all subsystems in one report
./target/release/rvt-analyze --redact my-project.rvt
./target/release/rvt-analyze --redact --json my-project.rvt > report.json

# Quick metadata + schema summary
./target/release/rvt-info --show-classes my-project.rvt

# Machine-readable (JSON)
./target/release/rvt-info -f json my-project.rvt > meta.json

# Element and room schedules for Excel / Sheets / LibreOffice
./target/release/rvt-schedule my-project.rvt
./target/release/rvt-schedule my-project.rvt --schedule rooms --metric --excel

# Inventory every Revit file under a folder: release, worksharing, last saved
./target/release/rvt-info projects/
./target/release/rvt-info projects/ -f csv --redact > inventory.csv

# Pull the embedded thumbnail
./target/release/rvt-info --extract-preview preview.png my-project.rvt

# Plain-language file health and IFC export readiness
./target/release/rvt-inspect my-project.rvt
./target/release/rvt-inspect my-project.rvt --json

# Compare two versions of the same file (cross-version byte diff)
./target/release/rvt-diff --decompress 2018.rfa 2024.rfa

# Dump the full class schema (4,126 classes and 11,562 fields on a Revit 2024 file)
./target/release/rvt-schema my-project.rvt

# Document upgrade history (which Revit releases have opened this file)
./target/release/rvt-history my-project.rvt

# Pull every UTF-16LE string record out of Partitions/NN
# (categories, OmniClass, Uniformat, Autodesk unit identifiers, …)
./target/release/rvt-history --partitions my-project.rvt

# Hex-dump every decompressed stream (for format research)
./target/release/rvt-dump my-project.rvt

# IFC4 STEP export — everything decoded; the default mode accepts a scaffold-only result, with warnings
./target/release/rvt-ifc my-project.rvt -o out.ifc

# Require a stronger quality gate before writing IFC: exits non-zero, writing nothing,
# while the diagnostics list unsupported features (true of every reference model today)
./target/release/rvt-ifc my-project.rvt -o out.ifc --mode strict

# IFC4 export with a shareable JSON readiness/support sidecar
./target/release/rvt-ifc my-project.rvt -o out.ifc --diagnostics out.diagnostics.json

# Diagnostic IFC export — include low-confidence proxy candidates with provenance
./target/release/rvt-ifc my-project.rvt -o diagnostic.ifc --diagnostic-proxies

# Compare an rvt-rs IFC export against a Revit (or other) reference IFC
./target/release/rvt-ifc-compare out.ifc revit-reference.ifc --json /tmp/ifc-compare.json

# glTF 2.0 binary export — loads in Three.js / Blender / any glTF viewer
./target/release/rvt-gltf my-project.rvt -o out.glb

# 2D plan-view SVG — per-category colours, ready for plot/laser-cut/printing
./target/release/rvt-sheet my-project.rvt -o out.svg

# Global/ElemTable dump — declared element-ids + record layout (family 12B / project 28B/40B)
./target/release/rvt-elem-table my-project.rvt --limit 20

# Production decoded elements / class counts (JSON; mirrors Python element_counts)
./target/release/rvt-elements my-project.rvt --counts

# Stream-level write path — patch named OLE streams via JSON manifest
./target/release/rvt-write my-project.rvt --patches patches.json -o patched.rvt

# ADocument's instance fields, as text or JSON
./target/release/rvt-doc my-project.rvt > doc.txt

# Cross-version corpus analysis (12 releases in one pass; 3 or more files)
./target/release/rvt-corpus /path/to/corpus-dir/*.rfa

# Triage a folder of files into failure buckets
./target/release/rvt-corpus doctor /path/to/corpus-dir --json
```

Every format finding has a probe under [`probes/examples/`](probes/README.md), whose module documentation states the fact it proves and how to check it; the `probe_re*.rs` files carry the numbered findings in `reports/element-framing/`. The probes of the first reconnaissance:

```bash
cargo build --release --examples -p rvt -p rvt-probes

# --- schema ↔ data linkage ---
./target/release/examples/probe_link              <file>           # null-hypothesis: class names absent from Global/Latest
./target/release/examples/tag_bytes               <file>           # hex around known class names in Formats/Latest
./target/release/examples/tag_dump                <file>           # statistical sweep of post-name u16 patterns
./target/release/examples/link_schema             <file>           # tag-frequency histogram in Global/Latest (340× non-uniformity)
./target/release/examples/tag_drift               <sample-dir> <out.csv>   # per-class drift table 2016-2026
./target/release/examples/tag_drift_svg           <in.csv> <out.svg>       # render drift table as colour-coded SVG heatmap

# --- record framing ---
./target/release/examples/record_framing          <file>           # dump bytes at tagged-class defs + first tag occurrence
./target/release/examples/elem_table_probe        <sample-dir>     # Global/ElemTable structural sweep across releases
./target/release/examples/partitions_header_probe <sample-dir>     # 44-byte Partitions/NN header + chunk offsets
./target/release/examples/contents_probe          <file>           # Contents stream decoder (creator name + build tag)

# --- stable anchors ---
./target/release/examples/partition_invariant     <sample-dir>     # find 165-byte invariant in Global/PartitionTable
./target/release/examples/partition_diff          <sample-dir>     # show the 2 varying bytes per release
./target/release/examples/partition_full          <file>           # full annotated hex dump + UUID decode

# --- write path ---
./target/release/examples/roundtrip                                # copy 2024 sample, verify all 13 streams identical
```

## Format overview

Every Revit file is a Microsoft Compound File Binary (OLE2) container with
this stream layout (constant across 11 years of Revit releases):

```
<root>
├── BasicFileInfo                 UTF-16LE metadata
├── Contents                      custom 4-byte header + DEFLATE body
├── Formats/Latest                DEFLATE — class schema inventory
├── Global/
│   ├── ContentDocuments          tiny document list
│   ├── DocumentIncrementTable    DEFLATE — change tracking
│   ├── ElemTable                 DEFLATE — element ID index
│   ├── History                   DEFLATE — edit history (GUIDs)
│   ├── Latest                    DEFLATE — current object state (17:1 ratio)
│   └── PartitionTable            DEFLATE — partition metadata
├── PartAtom                      plain XML (Atom + Autodesk partatom namespace)
├── Partitions/NN                 bulk data: 5-10 concatenated DEFLATE segments
│                                 NN = 58, 60-70 for Revit 2016-2027
├── RevitPreview4.0               custom header + PNG thumbnail
└── TransmissionData              UTF-16LE transmission metadata
```

All compressed streams use a "truncated gzip" format — the standard 10-byte
gzip header (magic `1F 8B 08 ...`) followed by raw DEFLATE, but *without*
the trailing 8-byte CRC32 + ISIZE that conforming gzip writers produce.
Python's `gzip.GzipFile` and Rust's `flate2::read::GzDecoder` both refuse
these streams. The fix is to skip the 10-byte header manually and use
`flate2::read::DeflateDecoder` on the raw body.

## Reverse engineering state

The format, layer by layer, and how far each layer is understood. Some rows record the research history they come from; the current, evidence-backed status of each capability is in [`docs/support-matrix.json`](docs/support-matrix.json).

| Layer | Description | Status |
|---|---|---|
| 1 · Container | OLE2 / Microsoft Compound File ([MS-CFB]) | **Done** |
| 2 · Compression | Truncated gzip → raw DEFLATE | **Done** |
| 3 · Stream framing | Per-stream custom headers, `Partitions/NN` chunk layout, `Contents` / `Preview` / `PartitionTable` wrappers | **Done** — 165/167 bytes of `PartitionTable` invariant; 44-byte `Partitions/NN` header decoded; `62 19 22 05` wrapper magic confirmed on `Contents` + `RevitPreview4.0` |
| 4a · Schema table | Class names + fields + C++ type signatures from `Formats/Latest`; per-class tag + parent + declared field count; cross-release tag-drift map | **Done** |
| 4b · Schema→data link | Tags from `Formats/Latest` occur at ~340× the noise rate in `Global/Latest`; schema IS the live type dictionary for the object graph | **Done** |
| 4c.1 · Record framing | Tagged class records in `Formats/Latest` parse into structured records: `{tag, parent, ancestor_tag, declared_field_count}`; HostObjAttr → `{parent=Symbol, declared_field_count=3}`. The `tag` a record carries is its base class's tag and `ancestor_tag` is its grandparent's, on 53 files of 2016 to 2027 (RE-146) | **Done** |
| 4c.2 · Field-body decoding | `FieldType` enum classifies all but 9 to 12 schema fields per release across 8 variants (Primitive, String, Guid, ElementId, ElementIdRef, Pointer, Vector, Container). 11 discriminator bytes mapped, including generalized scalar-base Vector/Container (`{kind} 0x10 ...` / `{kind} 0x50 ...`) and the `0x0d` point-type base. | **Done (all but 9 to 12 of 10,000 to 12,000 fields per release since #410; the earlier 100.00% on 13,570 fields covered the first 64 KB of each schema; zero `Unknown`)** |
| 4d · ElemTable | `Global/ElemTable`: the `u32` record count at `0x02`, then records from `0x06`, 28 bytes through Revit 2023 and 40 from 2024, each with its ElementId and owner (#152, RE-41, #737) | **Done**: where a 40-byte record's two ids differ, the second is the ElementId and the first the id the element was created with, which no element record of the project names, so only the second is declared (RE-189, #760) |
| 5 · IFC4 export | Full spatial tree + per-element IFC entities + `IfcLocalPlacement` + extruded, swept, revolved and B-rep bodies + compound material layers + typed property sets + `IfcOpeningElement`/`IfcRelVoidsElement`/`IfcRelFillsElement` for doors and windows. Deterministic ISO-10303-21 output; CI validates the exports of the reference projects with IfcOpenShell. | **Partial**: what each category gets is in [What rvt-rs reads from real projects](#what-rvt-rs-reads-from-real-projects); a body that is not decoded is its element's bounding box |
| 6 · Write path | Byte-preserving copy for unchanged files; stream-level patching for named OLE streams with atomic temp-file rename, per-stream verification, grow/shrink/multi-stream coverage, and GUID/history preservation checks. Field-level semantic patching is not implemented (ADR-002). | **Done (stream-level); field-level pending** |
| 7 · Browser viewer | WebAssembly build of the core + Three.js + Vite + Pages deploy. Zero-upload, in-tab parse, export buttons for glTF/IFC/SVG, URL-state share. Live at <https://drunkonjava.github.io/rvt-rs/>. | **Done** |

The original research questions are resolved: those on field encodings, `Global/Latest` and `Partitions/NN` as first answered, and the class preamble's flag word by RE-146, after its first reading did not reproduce (RE-137). Layer 4c.2 classified 100.00% of the 13,570 fields in the first 64 KB of each schema; #410 found the rest of `Formats/Latest` was being inflated without its page strip, and on the whole schema (122,107 fields across 11 releases) 9 to 12 fields per release remain unclassified. In April 2026 the next frontier was real-world project files, where one `.rvt` probe caught a `gzip_header_len` bounds bug that family files never hit; the partition element records (RE-21 onward) and the measured tables above are where that work went.

Key findings from this phase:

- **Class preamble flag word** (superseded, RE-137) The u16 "flag" word in each tagged-class preamble was read as a class-tag reference distinct from the parent. That reading did not reproduce under the page-stripped schema grammar (#410, #154): the nine classes behind it have the bases the grammar gives (`HostObjAttr` → `Symbol`), not the "ancestor" named, on 17 files of 2016 to 2026, and every base reference resolves. RE-146 then identified the word: it is the grandparent class's tag, which an earlier addendum had looked up among base tags (#154).
- **Field type encoding** Each field's `type_encoding` is `[byte category][u16 sub_type][optional body]`. 9 category bytes mapped (`0x01` bool, `0x02` u16, `0x04/0x05` u32, `0x06` f32, `0x07` f64, `0x08` string, `0x09` GUID, `0x0b` u64, `0x0e` reference/container).
- **Field-type coverage** extended to 84% of fields,
  then reaches **100%** of the fields in the first 64 KB of each schema (13,570 across 11 releases; #410 later read the whole schema). Generalized `{scalar_base} 0x10 ...` / `{scalar_base} 0x50 ...` as vector/container modifiers; added `0x0d` point-type base; added `0x08 0x60 ...` alternate string encoding; added `ElementIdRef { referenced_tag, sub }` for references that carry a specific target-class tag; added deprecated `0x03` i32-alias seen only in 2016–2018. See `docs/rvt-moat-break-reconnaissance.md` §Q5.2.
- **`Global/Latest` layout** It is **not** an index + heap — it's a flat TLV stream.
- **Instance data** is **schema-directed** (tag-less, protobuf-style). Decoding requires schema-first sequential walk from a known entry point.
- **`Partitions/NN` trailers** Their u32 fields are **not** per-chunk offsets. Gzip-magic scan remains correct.

The full analysis narrative with 12 dated addenda lives in [`docs/rvt-moat-break-reconnaissance.md`](docs/rvt-moat-break-reconnaissance.md). Session-length synthesis in [`docs/rvt-phase4c-session-2026-04-19.md`](docs/rvt-phase4c-session-2026-04-19.md).

## Sample corpus

The real-file checks read public corpora that are fetched, never committed:

- **Families:** Autodesk's `rac_basic_sample_family`, one per Revit release from 2016 to 2027, distributed through Git LFS in [`phi-ag/rvt`](https://github.com/phi-ag/rvt). `RVT_SAMPLES_DIR` points at them.
- **Projects:** the MIT-licensed models of [`magnetar-io/revit-test-datasets`](https://github.com/magnetar-io/revit-test-datasets) (Core Interior, Einhoven) and the RE1 models of [`Drshelden/IFC-ECS`](https://github.com/Drshelden/IFC-ECS), each with Revit's own IFC export. `RVT_PROJECT_CORPUS_DIR` points at them.

A check whose files are absent skips with a message, so a partial corpus is fine. [`CONTRIBUTING.md`](CONTRIBUTING.md#corpus-env-vars) shows how to fetch both, and [`docs/reference-models.md`](docs/reference-models.md) lists every model the measurements use.

## Design choices

- **cfb crate over custom OLE parser** — the `cfb` crate is mature,
  tested against Office documents, and handles both short and regular
  sectors. Faster than writing our own.
- **flate2 over miniz_oxide direct** — `flate2` wraps both `miniz_oxide`
  (pure Rust) and libz backends. We pick the default pure-Rust build to
  avoid a C toolchain dependency.
- **quick-xml over xml-rs** — ~3x faster, zero-copy friendly, and the
  `.from_str` + event-loop pattern is closer to what Go/Python parsers do.
- **encoding_rs over stdlib** — Revit's UTF-16LE streams sometimes have
  malformed pairs at boundaries (single-byte markers get interleaved).
  `encoding_rs` recovers gracefully where stdlib panics.
- **BTreeSet for class names** — deterministic ordering in output (plus
  sorted JSON) matters for diffable CLI output.

## For contributors

Use the [Codespaces development environment](docs/codespaces.md) for remote
Linux builds and the viewer from VS Code, a terminal, Codex or Claude Code.

rvt-rs is a clean-room reader for Revit files: an Apache-2.0 Rust core with
Python bindings and a WebAssembly viewer, with no Revit install or Autodesk
SDK at build or run time. Contributing does not need private files either —
the checked-in `corpus/tier1/` synthetic fixtures drive the default gates, and
corpus-backed tests skip themselves while `RVT_PROJECT_CORPUS_DIR` is unset.

- **Build:** stable Rust 1.87 or newer, then `cargo build`. Viewer:
  `cd viewer && pnpm install --frozen-lockfile`. Python bindings: [`docs/python.md`](docs/python.md).
- **Gate:** `tools/check-local.sh` runs the checks a pull request must pass:
  `cargo fmt --check`, `cargo clippy -D warnings`, rustdoc with `-D warnings`,
  a build of every target, and the real-file, CLI and contract checks.
  `--viewer`, `--corpus`, `--deny`, `--audit` add the optional gates.
- **How changes are verified:** against real Revit files and Revit's own
  exports, through the built CLIs, and by contract checks that keep the
  published claims honest. [`tools/ci/test-targets.txt`](tools/ci/test-targets.txt)
  classes every target under `tests/` (corpus-gated ones skip without their
  corpus); `tests/fuzz_regressions.rs` replays crash-shaped inputs on every
  pull request; libFuzzer targets are in `fuzz/`, Playwright browser tests in
  `viewer/tests/` and Python tests in `tests/python/`. Unit tests written
  before this policy stay in the source but are not run, and new ones are not
  accepted ([`CONTRIBUTING.md`](CONTRIBUTING.md) says why).
- **What "done" looks like:** the gate is green, the pull-request template's
  checklist is filled in, and any change to user-visible capability updates
  [`docs/status.md`](docs/status.md) and
  [`docs/support-matrix.json`](docs/support-matrix.json) in the same PR — the
  project does not claim what its tests cannot show.
- **Start here:** [`CONTRIBUTING.md`](CONTRIBUTING.md) walks from clone to a
  first pull request; [`docs/contribution-map.md`](docs/contribution-map.md)
  maps the larger areas; small tasks carry the
  [good first issue](https://github.com/DrunkOnJava/rvt-rs/labels/good%20first%20issue)
  label.

## License and trademarks

- **Code:** Apache License 2.0. See [`LICENSE`](LICENSE) for the full
  text and [`NOTICE`](NOTICE) for attribution.
- **Trademarks:** "Autodesk" and "Revit" are registered trademarks of
  Autodesk, Inc. rvt-rs is an independent project, **not affiliated
  with, endorsed by or sponsored by Autodesk**; it uses the names only to
  identify the file format it reads.
- **Provenance:** rvt-rs contains no Autodesk source code. Its knowledge of
  the format comes from the bytes of publicly available files and from the
  exported symbol names of the public `RevitAPI.dll` NuGet package.
  [`CLEANROOM.md`](CLEANROOM.md) lists the sources contributions may use and
  the declaration a format finding carries, and [`NOTICE`](NOTICE) gives the
  project's notices. This section describes how the project is built; it is
  not legal advice.
