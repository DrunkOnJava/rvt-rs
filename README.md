# rvt-rs

**Apache-2.0 clean-room Rust/Python toolkit for reading Autodesk Revit files (`.rvt`, `.rfa`, `.rte`, `.rft`) without a Revit installation.** It opens any Revit file from 2016 to 2026 and reports its metadata, previews and embedded schema, and on Revit 2024 and 2025 project files it reads the building itself: walls, floors, roofs, doors, windows, stairs, curtain walls, furniture, MEP and structure, with the names, types, storeys, materials and GlobalIds Revit gives them, written out as IFC4, glTF, plan SVG or CSV. A zero-upload browser viewer does the same in a tab.

**It is a reader, not a converter for every model.** What rvt-rs recovers is measured element by element against Revit's own IFC export of the same file, on the handful of real models listed in [What rvt-rs reads from real projects](#what-rvt-rs-reads-from-real-projects). Those numbers are the claim; other models of the same releases are expected to behave alike but are not measured. Geometry is exact where the bytes say so and approximated elsewhere (see [What does not work yet](#what-does-not-work-yet)), and most element parameters are not read yet.

### What you can rely on, by input

| Input | Open, metadata, previews, schema | Elements | Geometry | Properties |
|---|---|---|---|---|
| Revit 2024 and 2025 projects | yes | typed IFC entities from each element's own record; the ElementId set equals Revit's export on every measured model | measured per category: exact joins, profiles and cuts where decoded, the element's bounding box where not | names, family and type names, storey, materials and layers, Revit's GlobalId, stair dimensions, IFC export overrides; most other parameters not read |
| Revit 2026 projects | yes | **on `main`, experimental:** typed IFC entities from each element's record (RE-124), on one 2026 project 113 of the 115 elements of Revit's export, none outside it | the element's bounding box, walls cut back at their joins (wall thickness Revit's on 59 of 59, length on 35) | Revit's GlobalIds (113 of 113), storeys with Revit's names and elevations (4 of 4), rooms with Revit's number and name (13 of 13), doors and windows in their host walls, family and type names, wall and floor layer sets as thick as Revit's, walls joined by their join lists; roof slopes, stairs and IFC export overrides not read |
| Revit 2023 projects | yes | typed IFC entities (since 0.4.0) from each element's record (RE-81): on two 2023 projects every element of Revit's export (37 of 37) or all but the 8 window trims nested in windows (37 of 45), none outside it | the element's bounding box, walls cut back by the walls they join (Exemplo_data 17 of 17 and modelo_bim 4 of 4 as Revit's, RE-120), beams stopped at the faces of the columns they frame into (modelo_bim 8 of 8, RE-122); rooms their outline (RE-102) | ElementId, category, each door's and window's host wall (30 of 30 as Revit's, RE-85), and the storeys of Revit's own Levels with each element on the one its record names, a beam on the one at its top (Exemplo_data 37 of 37 and modelo_bim 37 of 37 in Revit's storey, RE-107, RE-119), each family instance's family and type name (50 of 50 as Revit names them, RE-109), each wall's, floor's and roof's type and system family (24 of 24 with Revit's type, RE-111, RE-112), family instances' materials (44 of 45 as Revit's, RE-113), and layer sets, named by their own or their category's material (RE-114, RE-115): Exemplo_data 36 of 37 and modelo_bim 31 of 37 elements with Revit's materials, and materials with Revit's shading colours (20 of 21, none different, RE-116); no layered or angled joins, design options or IFC export overrides, and the diagnostics say so |
| Revit 2022 and earlier projects | yes | not decoded; IFC export is the spatial scaffold plus diagnostics | none | metadata only |
| Families (`.rfa`) and templates | yes | family metadata, OmniClass, previews; no family geometry | none | none |

**Typical uses:** inventory and audit folders of Revit files without Revit (`rvt-info`); open a 2024 or 2025 model in a browser or in an IFC viewer to check its layout, storeys, types and materials; take element and room schedules into a spreadsheet (`rvt-schedule`); feed IFC or glTF into a coordination, quantity or visualisation pipeline, checking the per-export diagnostics first; and research the format itself with the probes under `examples/`.

For the non-technical workflow, start with the [`docs/user-guide.md`](docs/user-guide.md). Installation paths live in [`docs/install.md`](docs/install.md). The machine-readable capability list, with the evidence behind each entry and an honest status ceiling, is [`docs/support-matrix.json`](docs/support-matrix.json) (`rvt-capabilities --matrix -f text` prints it); [`docs/status.md`](docs/status.md) and [`docs/supported-profile.md`](docs/supported-profile.md) summarise it. This README describes `main`. The latest release is [v0.4.0](https://github.com/DrunkOnJava/rvt-rs/releases/tag/v0.4.0) (2026-09-28), on crates.io, PyPI, GitHub Releases and ghcr.io; [what `main` adds](#release-040-and-main) is listed below and under `[Unreleased]` in [`CHANGELOG.md`](CHANGELOG.md).

### Release 0.4.0 and main

`cargo install rvt --locked`, `pip install rvt`, the release archives and the container image give you 0.4.0, and the browser viewer runs it. 0.4.0 added Revit 2023 projects' typed elements, room outlines with Revit's numbers and names, door and window openings from their types, turned family instances, steel sections, IFC type objects and locale-aware system family names; upgrading from 0.3 is covered under "Upgrading from 0.3" in the [changelog](CHANGELOG.md). On `main` since 0.4.0:

- **The element-record marker** is confirmed from each file's own schema before a scan (#421). Output is unchanged.

To use `main` before the next release, build from source ([`docs/install.md`](docs/install.md#build-from-source)).

Rust 2024 edition (MSRV 1.85). **Nineteen CLIs ship** (`rvt-analyze`, `rvt-info`, `rvt-inspect`, `rvt-schema`, `rvt-history`, `rvt-diff`, `rvt-corpus`, `rvt-dump`, `rvt-doc`, `rvt-ifc`, `rvt-ifc-compare`, `rvt-write`, `rvt-gltf`, `rvt-sheet`, `rvt-elem-table`, `rvt-elements`, `rvt-capabilities`, `rvt-schedule`, `gen-fixture`) plus the reproducible probes under `examples/`, one or more per format finding. Python bindings via pyo3+maturin in the `rvt-py` workspace member (SEC-12/13 — the core `rvt` crate is unconditionally `#![forbid(unsafe_code)]`) — `pip install rvt`.

## What works today

| Layer | Status | Notes |
|---|---|---|
| OLE/CFB container open | ✓ | No Revit required |
| Truncated-gzip stream decode | ✓ | |
| `BasicFileInfo` metadata | ✓ | Version, build, GUID, original path, plus the `Key: value` block every release 2016-2026 writes: worksharing state, central model path, last-saved-by user, save counter, single-user-cloud flag (`rvt-info`, `rvt::metadata`) |
| `PartAtom` XML | ✓ | Title, OmniClass code, taxonomies |
| Stream preview extraction | ✓ | Clean PNG, wrapper stripped |
| `Formats/Latest` schema parse | ✓ | the whole page-stripped schema: 4,126 classes and 11,562 fields on Revit 2024, 122,107 fields across the 11 releases (#410) |
| Field-type classification | ✓ | 9 to 12 fields per release unclassified out of 10,000 to 12,000, all listed in the CI gate (`tests/field_type_coverage.rs`, #410); the 100% figure before #410 covered only the first 64 KB of each schema |
| Cross-release tag-drift table | ✓ | First public 122×11 dataset |
| Layer 5a ADocument walker | partial | Reliable on Revit 2024–2026; 2016–2023 entry-point detection pending |
| Stream-level modifying writer | ✓ | 13/13 streams byte-preserving; `rvt-write` CLI + JSON patch manifests |
| Field-level semantic writer | pending | Gated by [ADR-002](docs/decisions/ADR-002-semantic-write-api-gate.md); no stable API until decoder/validation evidence is strong enough |
| Layer 5b per-class decoders | partial | **81 decoder structs exist in `elements::all_decoders()`**, verified on synthesized bytes. Real project elements come from partition element records instead (next section); schema-field decoding of Wall/Floor/Door/Window from `Global/Latest` stays open (RE-01, RE-19). |
| IFC4 STEP export — spatial tree | ✓ | `IfcProject` + `IfcSite` + `IfcBuilding` + `IfcBuildingStorey` + OmniClass classifications |
| IFC4 STEP export — elements | partial | Revit 2024 and 2025 projects: typed entities from element records, measured against Revit's own export ([next section](#what-rvt-rs-reads-from-real-projects)). Other releases: the spatial scaffold (2023 arc walls aside). Low-confidence `HostObjAttr` candidates are never exported as proxies. |
| IFC4 STEP export — geometry | partial | Extrusions of arbitrary closed profiles with voids, sloped extrusions, stepped stair flights and B-rep solids. What each category gets on real files, and how exact it is, is in [the next section](#what-rvt-rs-reads-from-real-projects); anything not decoded is its record's bounding box. |
| IFC4 STEP export — materials | ✓ | Single-material via `IfcMaterial` + `IfcRelAssociatesMaterial`; compound assemblies via `IfcMaterialLayerSet` + `IfcMaterialLayerSetUsage` (IFC-28/29). On Revit 2024 and 2025 files, walls, floors, roofs and ceilings carry their type's layers with material names and thicknesses (RE-58). |
| IFC4 STEP export — properties | ✓ | `IfcPropertySet` + `IfcPropertySingleValue` with typed values (`IfcText`, `IfcInteger`, `IfcReal`, `IfcBoolean`, `IfcLengthMeasure`, `IfcPlaneAngleMeasure`, `IfcAreaMeasure`, `IfcVolumeMeasure`, `IfcCountMeasure`, `IfcTimeMeasure`, `IfcMassMeasure`) wired via `IfcRelDefinesByProperties`. |
| IFC4 STEP export — openings | partial | `IfcOpeningElement` + `IfcRelVoidsElement` + `IfcRelFillsElement`: each door and window cuts its host wall. The host is the wall Revit fills the opening in for 138 of 138 on Core Interior and 192 of 194 on Snowdon Towers, where the other 2 take another wall Revit also cuts (RE-23, RE-85). The opening is as deep as the host wall and otherwise the door's or window's box: within 0.25 ft of Revit's on all 138 Core Interior openings and within 0.26 ft on 124 of Snowdon's 194 (RE-83). A window whose type stores its Width, Height and Default Sill Height takes Revit's opening from them, centred on its origin, and its `OverallWidth` / `OverallHeight`: 56 of Snowdon's 68 windows, each within 0.001 ft of Revit's (RE-93). A door whose body is its type's Rough Height tall takes its type's rough opening the same way: 57 of Snowdon's 126 doors, 54 within 0.001 ft of Revit's (RE-94). Otherwise it is not Revit's opening profile, and in walls that are not straight extrusions it stays the element's box (#227). |
| Geometry extraction | partial | See the next section: walls from their centreline, joins and arcs; slabs and roofs from their sketches; shed roofs along their slope; columns cut by walls; beams along their axes; stair flights as treads and risers. Doors, windows, furniture and most families are boxes. |
| glTF 2.0 binary export | ✓ | `model_to_glb()` produces a valid `.glb` file that loads in Three.js's `GLTFLoader` (VW1-04), each element drawn as its IFC body: rotated, with its sketched or steel profile, or as its swept, revolved or brep solid. `rvt-gltf` CLI. |
| CSV schedules | partial | `rvt-schedule`, the viewer's Schedule panel and `RevitFile.schedule_csv()` write element and room schedules for Excel / Sheets; rows are exactly what decodes (typed on Revit 2024 projects with element records). |
| 2D plan-view SVG export | ✓ | `render_plan_svg()` produces per-category-coloured SVG (walls black, doors amber, columns red, …) (VW1-11), each element drawn as its body's plan outline, rotated and with holes open, with slabs beneath the walls. `rvt-sheet` CLI. |
| Browser viewer | ✓ | Live at <https://drunkonjava.github.io/rvt-rs/>. WebAssembly build of the core library + Three.js + Vite. Zero-upload, in-tab parse, Export glTF/IFC/SVG buttons, URL-based share via `share::ViewerState`. (VW1-01 through VW1-24 shipped.) |
| Fuzz-regression harness | ✓ | 9 libFuzzer targets + 38 synthetic adversarial regression cases under `tests/fuzz_regressions.rs`. Caught a real `gzip_header_len` bounds bug on 9-byte truncated headers (Q-04). |

## What rvt-rs reads from real projects

On Revit 2024 and 2025 project files, rvt-rs reads each element from its partition record: its ElementId, `BuiltInCategory`, bounding box and references, for a family instance its family and type, and for a wall, floor, ceiling, roof or railing its type's name. Every number below compares rvt-rs's IFC with Revit's own IFC export of the same file, element by element (the IFC `Tag` is the ElementId).

| Elements | IFC | Against Revit's own export |
|---|---|---|
| Walls | `IfcWall`, runs trimmed at their joins; a 2024 or 2025 wall its type's thickness either side of its centreline, running through or stopping at a butt join as its join lists say, layer by layer where both walls are layered alike, and stopping layer by layer where it ends part way along another wall, on a slanted line where the walls meet at an angle; a curved wall the ring sector along its arc; a tapered wall its cross-section, leaning on its exterior face, along its line | Core Interior 360 of 360 (351 world-exact); Snowdon Towers 1,078 of 1,078, faces exactly Revit's on 1,030 of them (RE-54, RE-75, RE-86), both ends on 724 (RE-70 to RE-75; curved walls scored by radius and angle) |
| Doors, windows | `IfcDoor`, `IfcWindow`, each filling an opening in its host wall | Core Interior 132 and 6; Snowdon Towers 132 and 68 |
| Floors, building pads | `IfcSlab` with its sketched plan profile | Core Interior 80 of 80, every one with its sketched outline, and its 20 shading-device plates with Revit's outline (RE-95); Snowdon Towers 176 of 199 with their outline, curved edges and shaft openings included, 154 with Revit's area (RE-96 to RE-100) |
| Columns | `IfcColumn`, minus what the walls cut | Core Interior 256 of 256, world-exact |
| Roofs, ceilings | `IfcRoof` with its sketched outline, a shed roof along its slope; `IfcCovering` with its sketched outline (RE-98) | Snowdon Towers 20 and 68; 13 roofs carry their outline, 11 with the area of Revit's own roof (RE-50); a shed roof's slope, thickness and outline reproduce its recorded height to 1e-9 ft (RE-56); 66 of 68 Snowdon ceilings and 6 of 6 RE1 ceilings with Revit's outline (RE-98) |
| Stairs | `IfcStair` aggregating its flights, landings and stringers, with `Pset_StairCommon` / `Pset_StairFlightCommon` riser count, riser height and tread length; straight flights with separate treads and risers, and monolithic ones, drawn as their steps | Snowdon Towers 27 stairs, 43 flights, 17 landings, every aggregate relation one Revit's has; every stair's riser and tread values are Revit's (26 of 26), and flights' on 37 of 43 (RE-47); 38 flights drawn as their steps, 33 of them equal to Revit's own geometry (RE-52, RE-92) |
| Curtain walls | `IfcCurtainWall` aggregating its panels and mullions, with no body of its own; a panel that holds a basic wall is an `IfcCurtainWall` with its body | Snowdon Towers 60 of 60 and RE1 1 of 1; all 2,075 of Snowdon's panel and mullion relations are Revit's own (RE-46, RE-62, RE-72); 1,019 turned or tilted mullions and panels are the box along their own axes, Revit's extents on every one (RE-106) |
| Railings, curtain panels, mullions | `IfcRailing`, `IfcPlate`, `IfcMember` | Snowdon Towers 131 railings, every mullion |
| Furniture, casework, plumbing, lighting, equipment | `IfcFurniture`, `IfcSanitaryTerminal`, `IfcLightFixture`, `IfcElectricAppliance`, proxies | RE1 models and Snowdon Towers, every one Revit exports |
| Ducts, pipes, fittings, air terminals | `IfcDuctSegment`, `IfcPipeSegment`, their fittings, `IfcAirTerminal` | RE1 Mechanical and Plumbing, every one |
| Electrical and mechanical devices and equipment | `IfcAlarm` (fire alarm devices), `IfcElectricAppliance` (data devices), proxies (switches, sensors, receptacles, junction boxes, panels, mechanical equipment) | RE1 Electrical and Mechanical, every one Revit exports and nothing else; the symbols nested in switches and receptacles left out, as Revit leaves them out (RE-79) |
| Structural framing, columns, foundations | `IfcBeam` along its location line, `IfcColumn`, `IfcFooting` | Snowdon structural sample: all 1,078 in the category its VIM export gives them; 923 of 942 beams run along their line, with Revit's own section on 613 of the 839 a later edition kept unchanged, and on 179 more less the top a floor join cuts (RE-49); 377 steel beams and 40 steel columns are their type's I section, Revit's own on all 271 beams and 24 columns the VIM scores (RE-103, RE-104) |
| Site elements, generic models, slab edges, elevators, ramps | proxies, `IfcTransportElement`, `IfcRamp` | Snowdon Towers, every one Revit exports |
| Rooms | `IfcSpace` named as Revit's export names it, `Name` the room number and `LongName` the room name, with the room's ElementId as the `ElementId` property and the outline of the solid Revit stores for the room (RE-101) | Core Interior all 116 and RE1 Architecture all 11 at Revit's own area and outline; Snowdon Towers 45 of 54 at the area its VIM export gives; on Revit 2023, Exemplo_data all 9 (RE-102); every room with Revit's number and name, Core Interior 116, Snowdon Towers 54 (against the VIM), RE1 11 and Exemplo_data 9 (RE-117); a room whose solid does not close keeps its box |
| Storeys | `IfcBuildingStorey` for each Revit Level that is a building story, with its name and elevation | Core Interior 15, Snowdon Towers 18, every RE1 model 2; name, elevation and GlobalId all Revit's (RE-24, RE-51); Snowdon Towers Structural 12 of its 19 Levels, its VIM export's building stories, 208 of the 210 elements on them in their storey (RE-118) |
| Storey containment | each element in the storey of the Level its record names, of its base constraint where it names two, and, where it names none, of a railing's host, of the Level its work planes carry, or of its base elevation | Core Interior all 854 in Revit's storey; Snowdon Towers 5,949 in Revit's storey, 1 in another and 1 on none (RE-27, RE-59, RE-60, RE-68) |
| Names | `Family:Type:ElementId`, `ObjectType` `Family:Type`, for family instances and, since RE-63 to RE-65, walls, curtain walls, floors, ceilings, roofs, railings, slab edges, ramps, curtain panels that are walls, and stairs with their runs, landings and supports | 4,397 family instances named exactly as Revit names them; Snowdon's 1,077 walls, 20 roofs and 68 ceilings and Core Interior's 460 system-family elements all named as Revit names them (RE-63); Snowdon's 18 panel walls and 59 slab edges too (RE-64), its 256 stair elements (RE-65), all 131 of its railings (RE-66), its 7 model texts (RE-67) and its 258 wall sweeps (RE-69) |
| GlobalIds | the GlobalId Revit's own exporter gives each element, room and storey; every other entity one derived from the document and its own identity, never its place in the file (#400) | every element rvt-rs exports that Revit's export holds: Core Interior 854 (plus 116 rooms and 15 storeys), the RE1 models 319, Snowdon Towers 5,945 (RE-48); on Revit 2023, Exemplo_data 37 (plus 9 rooms and 2 storeys) and modelo_bim 37 (plus 4 storeys) (RE-108); Snowdon Towers exported before and after RE-75 keeps all 20,149 GlobalIds (#400) |
| Wall, floor, roof and ceiling layers and colours | glTF and the viewer: a wall drawn as its layers (2024; 2025 since RE-55), and a floor, roof or ceiling as its layers stacked top first (RE-57), each in its material's shading colour | Snowdon Towers 883 of the 1,054 walls whose layers are read; every layer within 0.001 ft of Revit's on 851 of 862 walls, and all 1,368 layer colours Revit's (RE-53); floors, roofs and ceilings: every layer's bottom and top Revit's on Snowdon's 236 and Core Interior's 68 (RE-57) |
| Material names and IFC layer sets | each material's name, by ElementId; `IfcMaterialLayerSetUsage` over an `IfcMaterialLayerSet` for layered walls, floors, roofs and ceilings, each layer with its material and thickness | materials named: Snowdon 220 of 220, Core Interior 86 of 86 (RE-125), the Revit 2026 house 148 of 148, RE1, Projeto1, `teste_export_2025` and the tutorial house all; layer names Revit's, in Revit's order, on 3,321 of Snowdon's 3,322 layers and all of Core Interior's and RE1's; sets on Revit's bodies within 0.001 ft on 1,196 of 1,207 Snowdon elements, 403 of 403 Core Interior (RE-91), 15 of 15 RE1 (RE-58); a wall whose body is not a layer set gets Revit's `IfcMaterial` or constituent set of its layers, 95 of Snowdon's 111 (RE-88) |
| Family instances' materials | `IfcMaterialConstituentSet` of the materials the type's geometry uses, from its geometry-material map | the same set as Revit's on 1,331 of Snowdon's 1,371 family instances with a map, Core Interior's 132 doors, all 43 on RE1 Architecture; differences are nested components and planting (RE-82). In the GLB and viewer, an instance drawn in one material takes its colour: Revit's on all 290 on Snowdon and 38 on RE1. On Revit 2023, Revit's set on 17 of Exemplo_data's 18 and all 27 of modelo_bim's (RE-113) |
| Type parameters | `Type Mark`, `Description` and `Fire Rating` of each element's type, in its property set | Snowdon Towers: Type Mark on 1,257 of the 1,276 elements its VIM export checks (19 empty there), Description 70 of 70, Fire Rating 696 of 696 against Revit's IFC4 export; RE1 Fire Rating 7 of 7 (RE-77) |
| System-family type names | `TypeName` in the property set of walls, floors, ceilings, roofs and railings | 2,028 of 2,029 equal to Revit's across Core Interior, Snowdon Towers, RE1, Projeto1 and `teste_export_2025`. The other is a wall that export shows in a later state of the model (RE-44). On Revit 2023, walls', floors' and roofs' 24 of 24 (RE-111). The family in `Family:Type` names follows the locale the file was saved in, as Revit's export does: `Parede básica`, `Piso` and `Telhado básico` on the four files saved in Brazilian Portuguese, 32 of 32 (RE-123) |
| Type objects | `IfcWallType`, `IfcDoorType`, `IfcColumnType` and the rest, one per Revit type, named `Family:Type` with the type's ElementId as `Tag`, joined to their elements by `IfcRelDefinesByType` | every element Revit's export types, typed by the type Revit names: Core Interior 854 of 854, Snowdon Towers 5,803 of 5,812, RE1 Architecture 72 of 73; on Revit 2023, all 37 of Exemplo_data and all 37 of modelo_bim (RE-111, RE-112). Revit's exporter often splits one type into several type objects, one per column for instance, so type GlobalIds are Revit's on Core Interior's 438 of 854 (RE-110) |
| IFC export overrides | the entity and predefined type set by Revit's "Export to IFC As" and "IFC Predefined Type", the element's own or its type's | Core Interior 20 shading devices and 2 `.ROOF.` slabs; `teste_export_2025` 3 walls as `IfcCovering .CLADDING.`; each as in Revit's export (RE-45) |

Across Autodesk's Snowdon Towers architectural sample (Revit 2024), rvt-rs exports 5,974 elements. 5,945 of them are in Revit's own export, which is every element Revit exports in these entities. The other 29 are elements Revit's exporter leaves out: 23 curtain panels with no resolved type and 6 slabs in the Legends phase (#309, #328). Doors and windows whose type draws no geometry are written as their opening alone, as Revit writes them (RE-84). Elements in a design option set's non-primary options, 2D-only families and empty curtain panels are left out, as Revit leaves them out, and the export diagnostics count them.

Core Interior and the RE1 models are licensed test files and run in CI. Snowdon Towers has no licence and is measured locally only.

All of the above were used to develop the decoder. Three models that were not, Autodesk's Snowdon Towers Plumbing, HVAC and Electrical samples (2024), were measured once as they are against the VIM export of the sample: every exported element the VIM holds is in the expected category (10,385), every VIM instance both hold is exported, and all storeys and GlobalIds agree ([held-out report](reports/validation/held-out-snowdon-mep-2026-09-27.md), #408). Their bodies are bounding boxes.

## What does not work yet

| Gap | Status | Evidence |
|---|---|---|
| Element extraction from real `.rvt` project files | **partial** | Revit 2024 and 2025 projects decode their element records into typed IFC elements; see [What rvt-rs reads from real projects](#what-rvt-rs-reads-from-real-projects). Revit 2023 projects decode their elements' identity, category and box (RE-81). Earlier releases open and report metadata and schema; their element records are not decoded. The per-finding history, RE-21 to RE-43, is in [`docs/status.md`](docs/status.md) and `reports/element-framing/`. |
| 81 per-class decoders wired into walker | partial | `MVP_TYPED_CLASSES` are preferred via typed decoders in `iter_elements` (fail closed). The remaining registry entries still use generic `decode_instance`. ArcWall uses a separate partition decoder. |
| Element geometry beyond what is listed above | partial | Where the bytes are not decoded, a body is the element record's bounding box, and the export diagnostics say so. Still boxes: doors, windows and loadable families (no family geometry is read), turned with the instance where it is turned (RE-87); openings across and in height, which are the door's or window's box cut to the wall's depth (#227). Not decoded: hip and gable roofs (#356, no licensed example yet), monolithic, riserless and spiral stair flights (#357), edited wall profiles and some wall ends (#358: both ends match Revit on 716 of Snowdon's 1,078 walls), the boundaries of 7 of Snowdon's 54 rooms, whose stored solids do not close at the floor (#90), and phase filtering (#328). A slab sketched as separate pieces exports one element per piece, as Revit's export does (#331). |
| Community corpus open/scaffold verification | executed (scaffold only) | `tools/fetch-corpus.sh` + `examples/probe_corpus_batch_validate.rs` reported **222/223** real files passing open → schema → scaffold IFC (`docs/corpus-hunt-2026-04-21.md`). That measures container/schema/scaffold health, not typed element extraction. |
| Parameters and system-family type names | partial | Partition records are read through their record chain (RE-35), their frame prologue and reference lists, and the name entries of loadable families and types (RE-38, RE-42). The type names of system families (walls, floors, ceilings, roofs, railings) come from the type's own serialised data (RE-44, #322). The rest of that data is not walked yet, so most parameters are not read. |
| Scalar-Container wire format on real bytes | assumption only | L5B-09 fix assumes Vector-equivalent layout for kinds 0x01/0x02/0x04/0x05/0x07/0x0b/0x0d. Round-trip tests use synthesized bytes; no real-.rvt round-trip has been exercised. Tracked as WF-01..03. |
| Patched CFB roundtrip for grow/shrink cases | covered | Family corpus tests cover identity, grow, shrink, multi-stream, and missing-stream patches; project-corpus tests cover identity/grow/shrink/multi while preserving unpatched streams plus GUID/history. |

## Why the schema matters

The openBIM community — anchored by [buildingSMART International](https://www.buildingsmart.org/) and the IFC standard — has spent years working on Revit interoperability. Autodesk's own [revit-ifc](https://github.com/Autodesk/revit-ifc) exporter runs **inside** Revit using the Revit API, so it can only emit what the API surfaces. Real-world IFC exports from Revit are described, routinely and publicly, as *"very limited"* (thinkmoult.com), *"data loss"* (Reddit r/bim), and *"out of the box, just crap"* (the [OSArch Wiki's guide to Revit for openBIM](https://wiki.osarch.org/index.php?title=Revit_setup_for_OpenBIM)).

The schema work here — decoding the whole of `Formats/Latest` and classifying all but about ten field encodings per release across 11 Revit releases — is the dictionary a byte-level reader needs. Once the partition-stream decoder work in [`TODO.md`](TODO.md) lands, the resulting IFC export can carry more than what the Revit API chooses to expose. That is the thesis. It is not yet the delivered product.

If you're building BIM/AEC tooling and want an Apache-2 Revit reader to compose into your stack:

- **Any release, 2016 to 2026** — OLE/CFB open, truncated-gzip decode, metadata, previews, schema introspection (the whole schema, all but about ten field encodings per release classified), stream-level byte-preserving writes.
- **Revit 2024 and 2025 projects** — typed elements with Revit's ElementIds, names, types, storeys, materials, layers and GlobalIds, as IFC4, glTF 2.0, plan SVG and CSV, each export with a diagnostics sidecar naming what was approximated or left out.
- **Revit 2023 projects** — typed elements with Revit's ElementIds and categories, each drawn as its bounding box (RE-81), walls cut back at their joins (RE-120), beams at their columns (RE-122), rooms as their outline (RE-102), on the storeys of Revit's own Levels (RE-107), family instances named as Revit names them (RE-109), walls, floors and roofs with their type, read from its layers (RE-111, RE-112).
- **Not yet** — element records of 2022 and earlier and of 2026, most parameters, family geometry, and semantic editing of a Revit file.

[`tests/fixtures/synthetic-project.ifc`](tests/fixtures/synthetic-project.ifc) is a committed IFC from synthesized inputs, useful for testing a consumer without a Revit file. The browser viewer at <https://drunkonjava.github.io/rvt-rs/> runs the same pipeline in the tab: drop a 2023, 2024 or 2025 project and Export IFC writes its typed elements; other releases give the metadata and spatial scaffold. The viewer is built from `main`, so it already has what 0.3.0 does not (Revit 2023 elements among them).

## Quick demo

One command produces the full forensic picture — identity, upgrade history, format anchors, schema table, Phase D link histogram, content metadata, and a disclosure scan:

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

Drop a `.rvt` / `.rfa` / `.rte` / `.rft` at <https://drunkonjava.github.io/rvt-rs/> — nothing leaves the tab. The viewer compiles the core library to WebAssembly (`wasm-pack build --target web --features wasm`), runs the parse in a dedicated worker, and renders 3D via Three.js. One-click buttons export the model as **glTF 2.0 binary**, **IFC4 STEP**, or **plan-view SVG**. URL state (camera pose + category filters) is shareable via the hash fragment. Multi-megabyte projects fit inside wasm32's 4 GiB linear-memory ceiling: each gzip member reserves at most 1 MiB before decoding and the decoded buffer is trimmed (#256), which is what lets the 33.7 MB Core Interior demo finish at 419 MB of linear memory instead of growing to 4057 MB in 139 ms and trapping. A Rust panic in the wasm build is reported through `console.error` with its message and source location instead of a bare `RuntimeError: unreachable`.

The landing dropzone also includes a **demo gallery** staged from [`docs/viewer-demos.json`](docs/viewer-demos.json) (license/provenance + expected quality labels). It opens two real projects, `Revit_IFC5_Einhoven.rvt` (2023) and `2024_Core_Interior.rvt` (2024) from the MIT-licensed [magnetar-io/revit-test-datasets](https://github.com/magnetar-io/revit-test-datasets), hash-verified at staging time, alongside the tier1 synthetics. Einhoven is 913 KB and opens in about half a second; Core Interior is 33.7 MB and decodes in about 3 seconds on the deployed site since #266 (about 7 seconds including the 32 MB download), reporting 889 entities and 854 elements carrying geometry. Two Playwright tests gate both cards, and staging refuses a file whose sha256 does not match the catalog. Demo bytes are same-origin static assets only.

Privacy posture is CI-enforced: the deploy workflow (`.github/workflows/deploy-viewer.yml`) runs `wasm-objdump -j Import` on every build and fails if the compiled `.wasm` imports `fetch`, `XMLHttpRequest`, or `WebSocket`. See [`docs/viewer-privacy-posture.md`](docs/viewer-privacy-posture.md).

### Supported MVP workflow

The supported end-to-end shell (issue M11-02) is intentionally honest about partial decode:

1. Open a supported Revit file locally (drop, file picker, or a redistributable gallery demo).
2. Read the File status / confidence panel before trusting geometry.
3. Inspect decoded entities in the scene tree or by picking in the 3-D view.
4. Export IFC / glTF / plan only after checking the export-quality label.
5. Download diagnostics when the export is scaffold-only or partial.

What still depends on decoder work: element records outside Revit 2023 to 2025, and on 2023 geometry beyond each element's box, wall joins and beam cuts, layered or angled joins, design options and IFC export overrides; hip and gable roofs, family geometry and opening profiles; most element parameters; phase filtering (#328); compound layers of elements drawn whole or sloped, which get no layer set; and joining family instances to their materials (#34). **RE-19 / RE-20 (2026-08-29) closed negative** on the magnetar corpora: there is no Door vs Window discriminator *in the opening-index bytes* and no recoverable Level ElementId map there, so do not re-probe those without a new corpus or signal. Typed categories come from each element record's `BuiltInCategory` instead (RE-21 onward). On releases other than 2024 and 2025, treat IFC export as scaffold plus diagnostics; see [`docs/status.md`](docs/status.md) and [`docs/supported-profile.md`](docs/supported-profile.md).

**Sample output** (all pre-scrubbed with `--redact`, committed for review):

- **One-screen teaser**: [`docs/demo/rvt-analyze-2024-teaser.txt`](docs/demo/rvt-analyze-2024-teaser.txt) — the four highlight sections fit in one terminal screen (identity, format anchors, Phase D linkage, disclosure scan)
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

Every class and field name that `rvt-schema` extracts was cross-checked against the public `RevitAPI.dll` NuGet package's exported C++ symbol list. All top-level tagged class names we've inspected (ADocument, DBView, HostObj, LoadBCBase, Symbol, APIAppInfo, APropertyDouble3, ElementId, and the rest) appear in that export with their decorated signatures (e.g. `__cdecl NotNull<class ADocument *,void>::NotNull(class ADocument *)`), confirming the on-disk schema names match the compiled symbols one-to-one.

A build-server path also appears in C++ assertion strings inside the same DLL; it is mentioned in the recon report for completeness and does not represent anything the reader extracts from .rvt / .rfa files.

### Performance on large projects

Partition streams are inflated once per file and cached on the `RevitFile` handle (#266). On the 33.7 MB `2024_Core_Interior.rvt`, `rvt-ifc --mode geometry` went from **26.07 s / 2641.4 MiB** peak RSS to **1.69 s / 490.1 MiB** (Apple Silicon, `/usr/bin/time -l`, best of three), and the exported IFC is byte-identical once the writer's two wall-clock stamps are normalised. In the browser the same file decodes in **about 3 s** on the deployed site, where it used to take about 28. Every `tools/perf_budget.py --require-category medium` row now passes with an order of magnitude of headroom, without any budget being loosened.

## Phase D findings (what makes this project different)

Six reproducible discoveries, all documented in [`docs/rvt-moat-break-reconnaissance.md`](docs/rvt-moat-break-reconnaissance.md) and reproducible from `examples/`:

1. **The schema indexes the data.** Class names do not appear as ASCII in `Global/Latest`; class tags from `Formats/Latest` (u16 after class name, with 0x8000 flag set) occur ~340× the uniform-random rate. The top tag, `AbsCurveGStep`, appears 19,415 times in 938 KB of decompressed Global/Latest. [`examples/link_schema.rs`]

2. **Tags drift across releases** but are stable-sort-assigned. `ADocWarnings` = 0x001b 2016→2026 because no class sorted alphabetically before it has ever been added. `AbsCurveGStep` shifted 0x0053 → 0x0066 across the decade as 19 new A-class entries were inserted. Full 122-class × 11-release drift table: [`docs/data/tag-drift-2016-2026.csv`](docs/data/tag-drift-2016-2026.csv), visualised in [`docs/data/tag-drift-heatmap.svg`](docs/data/tag-drift-heatmap.svg). First publicly-available version of this data. [`examples/tag_drift.rs`]

3. **Revit 2021 was a major undocumented format transition.** Global/Latest grew 27× (~26 KB → ~715 KB) while simultaneously the Forge Design Data Schema namespaces (`autodesk.unit.*`, `autodesk.spec.*`) debuted in Partitions/NN. Two symptoms, one event. Any reader built for 2016-2020 silently drops 30× more data when pointed at 2021+.

4. **Parameter-group namespace shipped separately in Revit 2024.** `autodesk.parameter.group.*` identifiers appear in 2024+ only — three releases after units/specs. Dating the Forge schema rollout from on-disk bytes: [`examples/tag_drift.rs`](examples/tag_drift.rs), [`src/object_graph.rs`](src/object_graph.rs).

5. **A stable Revit format-identifier GUID in family files.** `Global/PartitionTable` is 167 bytes decompressed in `.rfa` family files, and **165 of those bytes are byte-for-byte identical across every Revit release 2016-2026** (98.8% invariant). The invariant region contains a never-before-published UUIDv1: `3529342d-e51e-11d4-92d8-0000863f27ad`. The MAC suffix `0000863f27ad` matches a known Autodesk-dev-workstation signature from circa 2000. Useful for family-file detection. **Scope correction (2026-04-21):** this invariant is a *family-file* anchor, not a universal Revit-file anchor. Three real `.rvt` project files we probed carry three different GUIDs (`6a6261fd-...` on Revit 2023, `552368c6-...` on 2024, all-zero on 2025) in a shorter 87-byte `PartitionTable`. File-type sniffers using the family GUID will correctly reject non-family files but can't identify them. See [`docs/project-file-corpus-probe-2026-04-21.md`](docs/project-file-corpus-probe-2026-04-21.md). [`examples/partition_full.rs`]

6. **Tagged class record structure decoded.** Every class declaration in `Formats/Latest` carries an explicit tag (u16 with 0x8000 flag), optional parent class, and declared field count, followed by N field records each with name + C++ type encoding. `HostObjAttr` now resolves to `{tag=107, parent=Symbol, declared_field_count=3}` with all three field names (`m_symbolInfo`, `m_renderStyleId`, `m_previewElemId`) extracted byte-for-byte. [`examples/record_framing.rs`, `src/formats.rs`]

Three unintended disclosure patterns also surfaced in Autodesk's shipped reference content — the specific values are withheld from this README to avoid re-broadcasting them; they are documented in [`docs/rvt-moat-break-reconnaissance.md`](docs/rvt-moat-break-reconnaissance.md) for security-research reproducibility:

- A customer-facing OneDrive path that leaks the directory structure of an Autodesk employee's personal sample-authoring workflow.
- A build-server path baked into C++ assertion strings inside the public `RevitAPI.dll`.
- A creator-name field inside the `Contents` stream that travels with every copy of the sample family, preserving the name of one of Revit's original 1997 developers.

**Downstream safety:** the `rvt-analyze` CLI ships with a `--redact` flag (on by default for any of the committed demo output in this repo) that rewrites creator paths, Autodesk-internal paths, and build-server paths to `<redacted>` markers while preserving the surrounding structure. Any tool consuming rvt-rs output and displaying it publicly should do the same.

---

## Library surface

All modules compile under both the default build and the `wasm` feature flag. See `src/` for type docs:

| Module | What it does |
|---|---|
| `reader` | Open any Revit file with `OpenLimits`, enumerate every OLE stream, fetch raw stream bytes, bounded reads |
| `compression` | Truncated-gzip decode (`inflate_at`, `inflate_at_auto`, `inflate_at_with_limits`) + multi-chunk (`inflate_all_chunks_with_limits`) + truncated-gzip encoder for write-back (`truncated_gzip_encode`) |
| `basic_file_info` | Version, build tag, GUID, creator path, locale — read path + byte-back encoder (`BasicFileInfo::encode`) |
| `part_atom` | Atom XML with Autodesk `partatom` namespace — title, OmniClass, taxonomies — read + encode |
| `formats` | Parse + encode `Formats/Latest` with `FieldType` classification (100 % over the 11-release corpus) |
| `walker` | Schema-directed instance walker + generic `decode_instance` + `detect_adocument_start` entry-point finder (does **not** dispatch through the 81-decoder registry) |
| `elements` | 81 `ElementDecoder` registry entries in `all_decoders()` (Wall, Floor, Door, Window, Column, Beam, Stair, Railing, Rebar, Room, Furniture, …) — synthesized-fixture unit tests only; not reached on real project files |
| `geometry` | Curve / Face / Solid variants (Line, Arc, Ellipse, NURBS, Hermite, Ruled, Revolved, Extrusion, Sweep, Blend, SweptBlend, Boolean, Mesh, PointCloud) |
| `object_graph` | `DocumentHistory`, string-record extractor for Global/Latest + Partitions/NN |
| `class_index` | Quick class-name inventory (BTreeSet) |
| `corpus` | Cross-version byte-delta classifier |
| `elem_table` | `Global/ElemTable` header parser + rough record enumeration |
| `partitions` | Partitions/NN 44-byte header decoder + gzip-chunk splitter |
| `writer` | Byte-preserving round-trip `copy_file` + `write_with_patches` (atomic temp-file rename, stream-hash verification) + GUID + history preservation |
| `round_trip` | Per-class encoder round-trip verification (`verify_instance_round_trip`) |
| `ifc` | Full IFC4 spatial tree + elements + materials + properties + openings + extrusion geometry + glTF 2.0 binary (`gltf::model_to_glb`) + plan-view SVG (`sheet::render_plan_svg`) + viewer data model (`scene_graph`, `camera`, `clipping`, `sheet`, `share`, `measure`, `annotation`, `pbr`) |
| `streams` | Named constants for every invariant OLE stream in a Revit file |
| `redact` | Shared PII scrubbers for all CLIs (`--redact` flag) |
| `wasm` | `#[cfg(feature = "wasm")]` — 14 JS-callable wasm-bindgen bindings powering the browser viewer |
| `error` | Structured error type (`Error` / `Result`) |

Runtime capabilities:

- Open any Revit file from disk (magic `D0 CF 11 E0 A1 B1 1A E1`)
- Enumerate every OLE stream; find the version-specific `Partitions/NN`
- Decompress any stream (truncated-gzip format — standard gzip header, no trailing CRC/ISIZE)
- Parse `BasicFileInfo`, `PartAtom`, extract preview PNG
- Extract **395 class records** from `Formats/Latest` with tag + parent + ancestor-tag + declared field count for every tagged class
- Decode the 167-byte `Global/PartitionTable` structure including the stable Revit format-identifier GUID
- Decode the 307-byte `Contents` stream including the embedded UTF-16LE metadata chunk
- Produce a byte-for-byte round-trip copy of any `.rfa` / `.rvt` file
- Run across the full 11-release corpus in < 500 ms per file (release build)

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

# Hex-dump every decompressed stream (for Phase D work)
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

# Cross-version corpus analysis (11 releases in one pass; 3 or more files)
./target/release/rvt-corpus /path/to/corpus-dir/*.rfa

# Triage a folder of files into failure buckets
./target/release/rvt-corpus doctor /path/to/corpus-dir --json
```

Thirty-six reproducible probes live in `examples/` — one per FACT in the recon report:

```bash
cargo build --release --examples

# --- schema ↔ data linkage (Phase D) ---
./target/release/examples/probe_link              <file>           # null-hypothesis: class names absent from Global/Latest
./target/release/examples/tag_bytes               <file>           # hex around known class names in Formats/Latest
./target/release/examples/tag_dump                <file>           # statistical sweep of post-name u16 patterns
./target/release/examples/link_schema             <file>           # tag-frequency histogram in Global/Latest (340× non-uniformity)
./target/release/examples/tag_drift               <sample-dir> <out.csv>   # per-class drift table 2016-2026
./target/release/examples/tag_drift_svg           <in.csv> <out.svg>       # render drift table as colour-coded SVG heatmap

# --- record framing (Phase 4c) ---
./target/release/examples/record_framing          <file>           # dump bytes at tagged-class defs + first tag occurrence
./target/release/examples/elem_table_probe        <sample-dir>     # Global/ElemTable structural sweep across releases
./target/release/examples/partitions_header_probe <sample-dir>     # 44-byte Partitions/NN header + chunk offsets
./target/release/examples/contents_probe          <file>           # Contents stream decoder (creator name + build tag)

# --- stable anchors ---
./target/release/examples/partition_invariant     <sample-dir>     # find 165-byte invariant in Global/PartitionTable
./target/release/examples/partition_diff          <sample-dir>     # show the 2 varying bytes per release
./target/release/examples/partition_full          <file>           # full annotated hex dump + UUID decode

# --- write path (Phase 6) ---
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
│                                 NN = 58, 60-69 for Revit 2016-2026
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

| Layer | Description | Status |
|---|---|---|
| 1 · Container | OLE2 / Microsoft Compound File ([MS-CFB]) | **Done** |
| 2 · Compression | Truncated gzip → raw DEFLATE | **Done** |
| 3 · Stream framing | Per-stream custom headers, `Partitions/NN` chunk layout, `Contents` / `Preview` / `PartitionTable` wrappers | **Done** — 165/167 bytes of `PartitionTable` invariant; 44-byte `Partitions/NN` header decoded; `62 19 22 05` wrapper magic confirmed on `Contents` + `RevitPreview4.0` |
| 4a · Schema table | Class names + fields + C++ type signatures from `Formats/Latest`; per-class tag + parent + declared field count; cross-release tag-drift map | **Done** |
| 4b · Schema→data link | Tags from `Formats/Latest` occur at ~340× the noise rate in `Global/Latest`; schema IS the live type dictionary for the object graph | **Done** |
| 4c.1 · Record framing | Tagged class records in `Formats/Latest` parse into structured records: `{tag, parent, ancestor_tag, declared_field_count}`; HostObjAttr → `{tag=107, parent=Symbol, ancestor_tag=0x0025 → APIVSTAMacroElem, declared_field_count=3}` | **Done** |
| 4c.2 · Field-body decoding | `FieldType` enum classifies all but 9 to 12 schema fields per release across 8 variants (Primitive, String, Guid, ElementId, ElementIdRef, Pointer, Vector, Container). 11 discriminator bytes mapped, including generalized scalar-base Vector/Container (`{kind} 0x10 ...` / `{kind} 0x50 ...`) and the `0x0d` point-type base. | **Done (all but 9 to 12 of 10,000 to 12,000 fields per release since #410; the earlier 100.00% on 13,570 fields covered the first 64 KB of each schema; zero `Unknown`)** |
| 4d · ElemTable | `Global/ElemTable` header parser + rough record enumeration; record semantics remain unresolved pending per-element schema lookup | **Partial** |
| 5 · IFC4 export | Full spatial tree + per-element IFC entities + `IfcLocalPlacement` + `IfcExtrudedAreaSolid` + compound material layers + typed property sets + `IfcOpeningElement`/`IfcRelVoidsElement`/`IfcRelFillsElement` for doors and windows. Deterministic ISO-10303-21 output. IfcOpenShell + BlenderBIM verified. | **Done** (rectangular profiles; swept / revolved / BRep fallbacks ship but use rectangular in the default emission path — IFC-17/24 is the remaining refinement) |
| 6 · Write path | Byte-preserving copy for unchanged files; stream-level patching for named OLE streams with atomic temp-file rename, per-stream verification, grow/shrink/multi-stream coverage, and GUID/history preservation checks. Field-level semantic patching is Phase 7. | **Done (stream-level); field-level pending** |
| 7 · Browser viewer | WebAssembly build of the core + Three.js + Vite + Pages deploy. Zero-upload, in-tab parse, export buttons for glTF/IFC/SVG, URL-state share. Live at <https://drunkonjava.github.io/rvt-rs/>. | **Done** (VW1-01..24) |

All 5 original P0 research questions (Q4-Q7) are **resolved**. Layer 4c.2 classified 100.00% of the 13,570 fields in the first 64 KB of each schema; #410 found the rest of `Formats/Latest` was being inflated without its page strip, and on the whole schema (122,107 fields across 11 releases) 9 to 12 fields per release remain unclassified. IFC4 emission, glTF export, 2D plan view, and the browser viewer all ship. The next frontier is real-world project-file corpus validation (Q-01) — one `.rvt` probe already caught a `gzip_header_len` bounds bug that family files never hit.

Key findings from this phase:

- **Q4** The u16 "flag" word in each tagged-class preamble is a **class-tag reference** (ancestor / mixin / protocol). 9/9 non-zero values resolve to named classes in the same schema.
- **Q5** Each field's `type_encoding` is `[byte category][u16 sub_type][optional body]`. 9 category bytes mapped (`0x01` bool, `0x02` u16, `0x04/0x05` u32, `0x06` f32, `0x07` f64, `0x08` string, `0x09` GUID, `0x0b` u64, `0x0e` reference/container).
- **Q5.1** Coverage extended to 84% of fields.
- **Q5.2** Coverage reaches **100%** of the fields in the first 64 KB of each schema (13,570 across 11 releases; #410 later read the whole schema). Generalized `{scalar_base} 0x10 ...` / `{scalar_base} 0x50 ...` as vector/container modifiers; added `0x0d` point-type base; added `0x08 0x60 ...` alternate string encoding; added `ElementIdRef { referenced_tag, sub }` for references that carry a specific target-class tag; added deprecated `0x03` i32-alias seen only in 2016–2018. See `docs/rvt-moat-break-reconnaissance.md` §Q5.2.
- **Q6** `Global/Latest` is **not** an index + heap — it's a flat TLV stream.
- **Q6.1** Instance data is **schema-directed** (tag-less, protobuf-style). Decoding requires schema-first sequential walk from a known entry point.
- **Q7** `Partitions/NN` trailer u32 fields are **not** per-chunk offsets. Gzip-magic scan remains correct.

The full analysis narrative with 12 dated addenda lives in [`docs/rvt-moat-break-reconnaissance.md`](docs/rvt-moat-break-reconnaissance.md). Session-length synthesis in [`docs/rvt-phase4c-session-2026-04-19.md`](docs/rvt-phase4c-session-2026-04-19.md).

## Sample corpus

Integration tests run against 11 versions of Autodesk's public
`rac_basic_sample_family` RFA fixture (one per Revit release from 2016
through 2026). These are distributed via Git LFS in the `phi-ag/rvt`
repository. To pull them:

```bash
cd /path/to/rvt-recon/samples
git clone https://github.com/phi-ag/rvt.git _phiag
cd _phiag && git lfs pull
cd .. && cp _phiag/examples/Autodesk/*.rfa .
```

The integration tests in `tests/samples.rs` skip any year whose RFA file
is absent, so partial corpora are okay — you'll just see
`skipping 2024: sample not present` messages.

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

rvt-rs is a clean-room reader for Revit files: an Apache-2.0 Rust core with
Python bindings and a WebAssembly viewer, with no Revit install or Autodesk
SDK at build or run time. Contributing does not need private files either —
the checked-in `corpus/tier1/` synthetic fixtures drive the default gates, and
corpus-backed tests skip themselves while `RVT_PROJECT_CORPUS_DIR` is unset.

- **Build:** stable Rust 1.85 or newer, then `cargo build`. Viewer:
  `cd viewer && pnpm install --frozen-lockfile`. Python bindings: [`docs/python.md`](docs/python.md).
- **Gate:** `tools/check-local.sh` runs what CI requires — `cargo fmt --check`,
  `cargo clippy -D warnings`, rustdoc with `-D warnings`, and the workspace
  tests (1,074 as of 2026-08-30). `--viewer`, `--corpus`, `--deny`, `--audit`
  add the optional gates.
- **Where the tests live:** unit tests next to the code in `src/`; integration
  tests in `tests/` (corpus-gated ones skip without `RVT_PROJECT_CORPUS_DIR`);
  `tests/fuzz_regressions.rs` replays crash-shaped inputs on stable Rust;
  libFuzzer targets in `fuzz/`; Playwright browser tests in `viewer/tests/`;
  Python tests in `tests/python/`.
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

- **Code**: Apache License 2.0. See [`LICENSE`](LICENSE) for the full
  text and [`NOTICE`](NOTICE) for attribution detail.
- **Trademarks**: "Autodesk" and "Revit" are registered trademarks of
  Autodesk, Inc. This project is **not affiliated with, endorsed by,
  or sponsored by Autodesk**. References to "Autodesk" and "Revit" in
  this project identify the file format this reader parses and are
  nominative fair use.
- **Interoperability basis**: reverse engineering for the purpose of
  creating an independently-developed interoperable program is
  recognised as lawful fair use under *Sega Enterprises v. Accolade*,
  977 F.2d 1510 (9th Cir. 1992) and *Sony Computer Entertainment v.
  Connectix*, 203 F.3d 596 (9th Cir. 2000) in the United States, and
  under Article 6 of the EU Software Directive 2009/24/EC in the
  European Union. File formats themselves are not copyrightable
  subject matter (*Baker v. Selden*, 101 U.S. 99 (1879); *Lotus
  Development v. Borland*, 516 U.S. 233 (1996)).
- **No Autodesk proprietary code** is used, referenced, or
  redistributed by this project. All file-format observations were
  made by inspecting the bytes of publicly-shipped Autodesk sample
  content and by parsing the public `RevitAPI.dll` NuGet package's
  exported symbol list. See [`NOTICE`](NOTICE).
