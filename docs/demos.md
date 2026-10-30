# rvt-rs demo gallery

Concrete, reproducible outputs from the `rvt-rs` toolchain. Every command
below is a real `[[bin]]` target declared in `Cargo.toml`; every output
excerpt is either from a committed fixture or a CLI run against the
`rac_basic_sample_family` reference corpus (`.rfa` files shipped by
Autodesk with Revit 2016–2026). Nothing is mocked, and anything that is
not yet implemented is explicitly flagged `(not yet implemented)`.

The demos walk the format inside-out: identify the file, dump the schema,
decode one instance, and emit an IFC4 project that BlenderBIM will open.

---

## Demo 1: File identification (`rvt-info`)

Goal: open a Revit file, tell you what release authored it, what OLE
streams are inside, and what the embedded `PartAtom` XML says. No Revit
installation is needed — everything comes from the OLE/CFB container and
the `Formats/Latest` + `PartAtom` streams.

```bash
cargo build --release
./target/release/rvt-info \
    --show-classes --redact \
    samples/rac_basic_sample_family-2024.rfa
```

Real output (redacted form; raw path → `<redacted>` so it's safe to
commit):

```text
Revit file ·
  version:       2024
  build:         —
  file GUID:     —
  original path: C:\Users\<redacted>\Desktop\Revit - <redacted project id>\2024\racbasicsamplefamily.rfa
  partition:     Partitions/67
  streams:       13 (397312 total bytes)

PartAtom ·
  title:     0610 x 0915mm
  omniclass: 23.30.20.00

Schema ·
  class names (inferred): 395
  sample:
    - ADocument
    - DBView
    - HostObj
    - Symbol
    - APIAppInfo

Streams ·
  BasicFileInfo
  Contents
  Formats/Latest
  Global/ContentDocuments
  Global/DocumentIncrementTable
  Global/ElemTable
  Global/History
  Global/Latest
  Global/PartitionTable
  PartAtom
  Partitions/67
  RevitPreview4.0
  TransmissionData
```

What just happened:

- `rvt-info` opened the OLE/CFB container via the `cfb` crate, read
  `BasicFileInfo` (UTF-16LE) → `version: 2024`.
- It parsed the `PartAtom` XML → `title: "0610 x 0915mm"` and
  `omniclass: 23.30.20.00`.
- It decompressed `Formats/Latest` and counted 395 class records.
- `Partitions/67` names the release-specific bulk-data stream. Every
  release from 2016–2026 maps to a specific `Partitions/NN` (2016 = 58,
  2017 = 60, …, 2026 = 69).
- `--redact` rewrote Windows usernames + project-ID folders to
  `<redacted>` placeholders while preserving path shape.

For JSON: `./target/release/rvt-info -f json --redact my.rfa` (schema is
the `rvt::reader::Summary` struct in `src/reader.rs`). For the full
forensic report (identity + upgrade history + format anchors + schema +
tag-linkage histogram + disclosure scan) use `rvt-analyze --redact
<file>`; committed samples live at
[`docs/demo/rvt-analyze-2024-redacted.txt`](demo/rvt-analyze-2024-redacted.txt)
and [`.json`](demo/rvt-analyze-2024-redacted.json).

---

## Demo 2: Class schema enumeration (`rvt-schema`)

Goal: dump the 395-class serialization dictionary Revit ships inside
every file under `Formats/Latest`. Each class carries a name, a u16 tag,
an optional parent, a declared field count, and N field records — each
with a C++ type signature the Revit serializer uses to round-trip on
disk.

```bash
./target/release/rvt-schema \
    samples/rac_basic_sample_family-2024.rfa \
    --grep HostObj
```

Real output (HostObj family only):

```text
Schema · 395 classes
          13570 fields total
          1087 unique C++ type signatures

Classes matching /HostObj/:

  HostObj  [offset 0x03a2, 0 fields]

  HostObjAttr  [offset 0x03b4, 3 fields]
    . m_symbolInfo : class APropertyAProps<class APropertyRawInt>
    . m_renderStyleId : class ElementId
    . m_previewElemId : class ElementId

  HostObjData  [offset 0x03f1, 0 fields]

  HostObjType  [offset 0x03fc, 0 fields]
```

Sibling views: `--top 10` (classes by field count — Analytical* and
Structural* families dominate), `-f json` (full schema dump suitable for
library generators or schema differs).

Why this matters: these class names and type signatures match the
symbols exported by the public `RevitAPI.dll` NuGet package one-to-one.
The on-disk schema *is* the live type dictionary for the object graph in
`Global/Latest` — which is why the schema tags appear there at ~340× the
uniform-random rate. See README § "Findings of the first reconnaissance" and
[`docs/data/tag-drift-2016-2026.csv`](data/tag-drift-2016-2026.csv) for
the drift table across the 11-release corpus.

---

## Demo 3: Element decode (`rvt-doc`)

Goal: step past schema introspection and read an actual instance. The
`rvt-doc` CLI locates the `ADocument` entry inside `Global/Latest`,
decodes each field according to the `Formats/Latest` schema, and prints
typed values.

```bash
./target/release/rvt-doc --redact samples/rac_basic_sample_family-2024.rfa
```

Real output (first ~15 of 36 ADocument fields):

```text
path:              <redacted>/racbasicsamplefamily-2024.rfa
version:           2024
adocument entry:   0x000a40

  m_doc_version                        :: Pointer    [a=0x00000001, b=0x00000000]
  m_next_handle                        :: integer
  m_elemTable                          :: Pointer    [a=0x0002a1b0, b=0x00000006]
  m_is_workshared                      :: bool
  m_is_central                         :: bool
  m_transmission_data                  :: Container  [count=0, col_a=0, col_b=0]
  m_project_info_id                    :: ElementId  [tag=172, id=91]
  m_default_view_id                    :: ElementId  [tag=193, id=7]
  m_active_view_id                     :: ElementId  [tag=193, id=7]
  m_sun_settings_id                    :: ElementId  [tag=201, id=12]
  m_project_location_id                :: ElementId  [tag=219, id=8]
  m_devBranchInfo                      :: element_id
  …
```

Coverage boundary: `rvt-doc` is validated on Revit 2024–2026. For Revit
2016–2023 the walker reports:

```text
adocument entry:   <not located>

note: ADocument record not locatable in this release's stream
      layout. This is expected for Revit 2016–2023 today.
```

Extending the entry-point detector to every release is
*(not yet implemented)*.

Per-element typed views live under [`src/elements/`](../src/elements/) —
81 registered classes (Level, Wall, Floor, Roof, Door, Window, Column,
Beam, Stair, Railing, Room, Furniture, Rebar, Phase, DesignOption,
Workset, and more), each decoded by a `SimpleDecoder`, and typed views
of Level, Wall, Floor, Door, Window, Room and Material. A view takes the
generic `DecodedElement` produced by the walker and projects it into a
typed struct. Sample call:

```rust
use rvt::elements::SimpleDecoder;
use rvt::elements::level::Level;
use rvt::walker::ElementDecoder;

let entry    = schema.classes.iter().find(|c| c.name == "Level").unwrap();
let decoded  = SimpleDecoder::new("Level").decode(bytes, entry, &handle_index)?;
let level    = Level::from_decoded(&decoded);
// Level { name: Some("Ground Floor"), elevation_feet: Some(0.0),
//         level_type_id: Some(ElementId { tag: 201, id: 17 }),
//         is_building_story: Some(true) }
```

The `Wall` typed view is larger — `base_offset_feet`,
`top_offset_feet`, `unconnected_height_feet`, `structural_usage`
(NonBearing / Bearing / Shear / Combined), `location_line`
(WallCenterline / CoreCenterline / FinishFaceExterior / …), and the
`ElementId` references to the top level and `WallType`. Full field
table: [`src/elements/wall.rs`](../src/elements/wall.rs).

A real project's elements come from its partition element records
instead (Demo 4). Driving these schema decoders off the live
`Global/Latest` element table is *(not yet implemented)*, so they run
against synthesised schema and bytes.

---

## Demo 4: Synthetic IFC4 project (`tests/fixtures/synthetic-project.ifc`)

Goal: show what the IFC4 STEP writer produces given a set of decoded
elements + placements + extrusions. The committed fixture at
[`tests/fixtures/synthetic-project.ifc`](../tests/fixtures/synthetic-project.ifc)
is 157 lines of IFC4 generated by the `synthetic_project_emits_valid_ifc4`
test in [`tests/ifc_synthetic_project.rs`](../tests/ifc_synthetic_project.rs).
It is a unit-class target (see `tools/ci/test-targets.txt`) that CI no
longer runs; the committed file is validated by IfcOpenShell in CI.
Regenerate it by hand with:

```bash
DUMP_IFC=1 cargo test --release --test integration -- \
    ifc_synthetic_project::synthetic_project_emits_valid_ifc4
```

What the fixture contains:

| Entity kind | Count | Purpose |
|---|---:|---|
| `IfcProject` | 1 | Root, "Synthetic Test Project" |
| `IfcSite` / `IfcBuilding` | 1 / 1 | Defaults |
| `IfcBuildingStorey` | 3 | Ground (0 m), Second (3.048 m / 10 ft), Roof Deck (6.096 m / 20 ft) |
| `IfcWall` | 4 | N / S / E / W with 20 ft × 8″ × 10 ft extrusions |
| `IfcSlab` | 1 | 20 ft × 10 ft × 1 ft ground-floor slab |
| `IfcDoor` | 1 | 3 ft × 8″ × 7 ft front door hosted in the south wall |
| `IfcWindow` | 2 | North + south (placement only; geometry pending) |
| `IfcStair` | 1 | Main stair (placement only; geometry pending) |
| `IfcBuildingElementProxy` | 1 | Unknown-class fallback ("Mystery Element") |
| `IfcOpeningElement` | 1 | Door subtraction volume |
| `IfcRelVoidsElement` / `IfcRelFillsElement` | 1 / 1 | Wall → opening → door chain |
| `IfcMaterial` | 2 | Concrete (0xAAAAAA), Glass – Tinted (0xDDAA88, 60% transp.) |
| `IfcSurfaceStyle` | 2 | One per material, diffuse colour + rendering params |
| `IfcRelAssociatesMaterial` | 2 | Concrete → 7 elements, glass → 2 windows |
| `IfcPropertySet` | 6 | `Pset_BuildingStoreyCommon` on each storey, `Pset_BuildingCommon`, `RvtWall` (5 props) and `RvtWindow` (2 props): IFC4's `Pset_WallCommon` and `Pset_WindowCommon` declare none of the wall's and window's values (#827) |
| `IfcRelDefinesByProperties` | 6 | Binds each property set to its object |
| `IfcRelContainedInSpatialStructure` | 2 | 9 elements on Ground + 1 on Second |
| `IfcRelAggregates` | 3 | Project → Site, Site → Building, Building → 3 storeys |

Abridged excerpt (header + a storey + the north wall + the door
opening-chain + a property set):

```step
ISO-10303-21;
HEADER;
FILE_DESCRIPTION(('ViewDefinition [CoordinationView]'),'2;1');
FILE_NAME('Synthetic Test Project.ifc','2026-10-20T03:35:22',('rvt-rs'),('DrunkOnJava/rvt-rs'),'rvt-rs 0.6.1','rvt-rs STEP writer','');
FILE_SCHEMA(('IFC4'));
ENDSEC;
DATA;
#6=IFCSIUNIT(*,.LENGTHUNIT.,$,.METRE.);
#18=IFCPROJECT('2DidPPITwEWgzLwN9h$v59',#5,'Synthetic Test Project','End-to-end rvt-rs pipeline smoke test',$,$,$,(#17),#12);
#24=IFCBUILDINGSTOREY('0ltZd43IADNhrlf$bL6ubE',#5,'Ground Floor',$,$,#23,$,'Ground Floor',.ELEMENT.,0.000000);
#26=IFCBUILDINGSTOREY('0lbLN7pIw6BuvMw4oqhcdl',#5,'Second Floor',$,$,#25,$,'Second Floor',.ELEMENT.,3.048000);
#45=IFCMATERIAL('Concrete',$,$);
#51=IFCMATERIAL('Glass - Tinted',$,$);
#64=IFCRECTANGLEPROFILEDEF(.AREA.,$,#63,6.096000,0.203200);
#65=IFCEXTRUDEDAREASOLID(#64,#16,#14,3.048000);
#66=IFCSHAPEREPRESENTATION(#17,'Body','SweptSolid',(#65));
#67=IFCPRODUCTDEFINITIONSHAPE($,$,(#66));
#68=IFCWALL('3xlhuehGY479OIzuhukk6g',#5,'North Wall',$,$,#60,#67,'W-N-001',.NOTDEFINED.);
#128=IFCDOOR('1UrhkCDyE2zQ7NtI3QeiHF',#5,'Front Entry Door',$,$,#120,#127,'DOOR-001',$,$,.DOOR.,$,$);
#137=IFCOPENINGELEMENT('2AJs5dPAE0FvFGouvwp3Vn',#5,'Opening for Front Entry Door',$,$,#136,#135,'DOOR-001',.OPENING.);
#146=IFCRELVOIDSELEMENT('1B$ATe42Q2qu6cp8BLVgI$',#5,$,$,#80,#137);
#147=IFCRELFILLSELEMENT('0eAjMcQvI1j9WLKGmmjafA',#5,$,$,#137,#128);
#153=IFCPROPERTYSET('0NWcqATUcFPhNb7lDclogt',#5,'RvtWall',$,(#148,#149,#150,#151,#152));
#154=IFCRELDEFINESBYPROPERTIES('3XQFc66jg7_xbAwXMkpuD2',#5,$,$,(#68),#153);
#161=IFCRELCONTAINEDINSPATIALSTRUCTURE('2nQIPsGEU2LRIyF4X3hMhW',#5,$,$,(#68,#80,#104,#116,#128,#139,#141,#143),#24);
ENDSEC;
END-ISO-10303-21;
```

Notable properties:

- **SI units.** Every value is written in SI units and the
  `IfcUnitAssignment` says so: metres, square and cubic metres, radians,
  kilograms, seconds (#403). Feet → metres is done on the way through the
  writer (10 ft → 3.048 m, 20 ft → 6.096 m, 8 inches → 0.2032 m).
- **Stable GlobalIds.** An export of a real Revit 2024 or later project
  gives its elements, rooms and storeys the GlobalIds Revit's own exporter
  gives them (RE-48). Everything else, including every entity of this
  synthetic fixture, gets a GlobalId hashed from the document's identity
  and the entity's own: an element's ElementId, an opening's filling
  element, a relationship's participants (#400). None depends on where
  the entity lands in the file, so an export keeps its GlobalIds when
  unrelated geometry changes, and the fixture is byte-stable across
  rebuilds (see Demo 5). `tools/ifc_globalid_stability.py BEFORE.ifc
  AFTER.ifc --require-stable` checks that between two exports of one
  model.
- **Real opening chain.** The front door is hosted in the south wall
  (via `host_element_index` in the test), so the writer emits
  `IfcOpeningElement` + `IfcRelVoidsElement(wall, opening)` +
  `IfcRelFillsElement(opening, door)`. BlenderBIM draws a real hole
  where the door goes.
- **Materials carry surface style.** Each `IfcMaterial` is paired with
  an `IfcSurfaceStyle` that sets diffuse colour + transparency, so the
  concrete walls render grey and the tinted-glass windows render
  translucent.
- **Unknown classes degrade to a proxy, not an error.** The synthetic
  `AutodeskCustomThing` element comes out as
  `IfcBuildingElementProxy` — every input element appears in the output
  even without a dedicated decoder.

For the end-to-end `.rvt` → `.ifc` path, on the MIT-licensed
`2024_Core_Interior.rvt` from `magnetar-io/revit-test-datasets`:

```bash
./target/release/rvt-ifc 2024_Core_Interior.rvt
# rvt-ifc: wrote 1910493 bytes to 2024_Core_Interior.ifc
# rvt-ifc: 970 building element(s), 970 with geometry, on 15 storey(s)
#   360 IfcWall, 256 IfcColumn, 132 IfcDoor, 116 IfcSpace, 80 IfcSlab, 20 IfcShadingDevice, and 1 more type(s)
# rvt-ifc: readiness geometry (score 1.00). Add --diagnostics <path> for the full report
```

Revit 2024 and 2025 projects export typed elements read from their
partition element records; the README's "What rvt-rs reads from real
projects" measures them against Revit's own export. The summary also
counts what the export leaves out on purpose, as Revit's does: 2D-only
families and elements in non-primary design options. A family file
(`.rfa`) or a project from an earlier release writes the spatial tree
(Project, Site, Building, Storeys) with a scaffold readiness level.

---

## Demo 5: Round-trip verification (byte-stable IFC output)

Goal: re-running the IFC writer with the same inputs produces
byte-identical output, so a sample written at a fixed timestamp, like
`tests/fixtures/synthetic-project.ifc`, is reproducible.

The second test in
[`tests/ifc_synthetic_project.rs`](../tests/ifc_synthetic_project.rs),
`synthetic_project_is_byte_stable_under_fixed_timestamp`, asserts
exactly that:

```rust
let step_opts = StepOptions { timestamp: Some(1_700_000_000) };
let a = write_step_with_options(&model, &step_opts);
let b = write_step_with_options(&model, &step_opts);
assert_eq!(a, b, "fixed-timestamp output must be byte-stable");
```

`StepOptions::timestamp` pins the `FILE_NAME` header's build date; the
GlobalIds are derived from identity, not generated; entity and property ordering is
stable by construction. With those three in place, re-running the
writer on the same `IfcModel` twice produces byte-equal strings, and
re-running the fixture-generating test with `DUMP_IFC=1` produces a
file whose only varying line is line 4 (the timestamp).

CI writes the synthetic project and structural frame on every run and
validates them with IfcOpenShell and `tools/ci/ifc_schema_arity.py`; it
does not compare them with the committed copies, which are refreshed at
each release for the viewer's downloads. The test's assertions check
the IFC4 header, the spatial tree (one site, one building, three
storeys), which storey or building contains each element, the seven
extruded solids, the opening chain (opening, voids and fills), the
materials and their surface styles, and the property sets.

Anything that regresses any of the above breaks the test — which is
exactly what you want.

---

## What the demos do not cover

Honest boundary of what rvt-rs can demonstrate today:

- **Every project, exactly** — Revit 2023, 2024 and 2025 projects export
  typed elements measured against Revit's own exports: on 2024 and 2025
  with geometry measured per category, on 2023 as their boxes, cut back
  at wall joins and column faces (the README's "What rvt-rs reads from
  real projects"). 2015 to 2022, 2026 and
  2027 export experimentally. Where an
  element's bytes are not decoded, its body is its record's bounding box,
  and the export diagnostics say so.
- **Revit 2016–2023 ADocument walk** — validated on 2024–2026 only;
  older releases return `ADocument record not locatable`
  *(not yet implemented)*.
- **Field-level writes** — stream-level byte-preserving round-trip
  works (13/13 streams), but rewriting a specific schema field back
  into a Revit file is *(not yet implemented)*;\n  [ADR-002](decisions/ADR-002-semantic-write-api-gate.md) gates it.
- **Web viewer demos beyond two real projects** — the viewer is live at
  <https://drunkonjava.github.io/rvt-rs/>, and the gallery described in
  [`docs/viewer-demos.json`](viewer-demos.json) opens
  `Revit_IFC5_Einhoven.rvt` (2023, 913 KB) and
  `2024_Core_Interior.rvt` (2024, 33.7 MB) from the MIT
  magnetar-io/revit-test-datasets corpus, hash-verified at staging
  time. Every other gallery entry is a 20 KB synthetic or a family
  `.rfa`; a broader redistributable real-project corpus is
  *(not yet available)*.
