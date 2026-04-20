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
uniform-random rate. See README § "Phase D findings" and
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

Extending the entry-point detector to every release is task
[L5B-11](../TODO-BLINDSIDE.md) *(not yet implemented)*.

Per-element typed views live under [`src/elements/`](../src/elements/) —
54 decoders (Level, Wall, Floor, Roof, Door, Window, Column, Beam,
Stair, Railing, Room, Furniture, Rebar, Phase, DesignOption, Workset,
and more). Each takes the generic `DecodedElement` produced by the
walker and projects it into a typed struct. Sample call:

```rust
use rvt::elements::level::{Level, LevelDecoder};
use rvt::walker::ElementDecoder;

let entry    = schema.classes.iter().find(|c| c.name == "Level").unwrap();
let decoded  = LevelDecoder.decode(bytes, entry, &handle_index)?;
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

Driving this loop off the live `Global/Latest` element table — so a
Revit file's full Level / Wall / Door inventory falls out automatically
— is task [L5B-01](../TODO-BLINDSIDE.md) *(not yet implemented)*; today
the walker exposes `ADocument`, and per-element decoders run against
synthesised schema+bytes fixtures.

---

## Demo 4: Synthetic IFC4 project (`tests/fixtures/synthetic-project.ifc`)

Goal: show what the IFC4 STEP writer produces given a set of decoded
elements + placements + extrusions. The committed fixture at
[`tests/fixtures/synthetic-project.ifc`](../tests/fixtures/synthetic-project.ifc)
is 157 lines of IFC4 generated by the `synthetic_project_emits_valid_ifc4`
test in [`tests/ifc_synthetic_project.rs`](../tests/ifc_synthetic_project.rs).
Regenerate with:

```bash
DUMP_IFC=1 cargo test --release --test ifc_synthetic_project \
    synthetic_project_emits_valid_ifc4
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
| `IfcPropertySet` | 2 | `Pset_WallCommon` (5 props) + `Pset_WindowCommon` (2 props) |
| `IfcRelDefinesByProperties` | 2 | Binds each property set to its element |
| `IfcRelContainedInSpatialStructure` | 2 | 9 elements on Ground + 1 on Second |
| `IfcRelAggregates` | 3 | Project → Site, Site → Building, Building → 3 storeys |

Abridged excerpt (header + a storey + the north wall + the door
opening-chain + a property set):

```step
ISO-10303-21;
HEADER;
FILE_DESCRIPTION(('ViewDefinition [CoordinationView]'),'2;1');
FILE_NAME('Synthetic Test Project.ifc','2026-04-20T07:51:29',('rvt-rs'),('DrunkOnJava/rvt-rs'),'rvt-rs 0.1.x','rvt-rs STEP writer','');
FILE_SCHEMA(('IFC4'));
ENDSEC;
DATA;
#6=IFCSIUNIT(*,.LENGTHUNIT.,.MILLI.,.METRE.);
#16=IFCPROJECT('0rvtrs000000000000000G',#5,'Synthetic Test Project','End-to-end rvt-rs pipeline smoke test',$,$,$,(#15),#10);
#22=IFCBUILDINGSTOREY('0rvtrs000000000000000M',#5,'Ground Floor',$,$,#21,$,'Ground Floor',.ELEMENT.,0);
#24=IFCBUILDINGSTOREY('0rvtrs000000000000000O',#5,'Second Floor',$,$,#23,$,'Second Floor',.ELEMENT.,3.048);
#30=IFCMATERIAL('Concrete',$,$);
#36=IFCMATERIAL('Glass - Tinted',$,$);
#49=IFCRECTANGLEPROFILEDEF(.AREA.,$,#48,6.096000,0.203200);
#50=IFCEXTRUDEDAREASOLID(#49,#44,#12,3.048000);
#51=IFCSHAPEREPRESENTATION(#15,'Body','SweptSolid',(#50));
#52=IFCPRODUCTDEFINITIONSHAPE($,$,(#51));
#53=IFCWALL('0rvtrs000000000000000r',#5,'North Wall',$,$,#45,#52,'W-N-001');
#113=IFCDOOR('0rvtrs000000000000001n',#5,'Front Entry Door',$,$,#105,#112,'DOOR-001',$,$);
#122=IFCOPENINGELEMENT('0rvtrs000000000000001w',#5,'Opening for Front Entry Door',$,$,#121,#120,$,.OPENING.);
#131=IFCRELVOIDSELEMENT('0rvtrs0000000000000023',#5,$,$,#65,#122);
#132=IFCRELFILLSELEMENT('0rvtrs0000000000000024',#5,$,$,#122,#113);
#138=IFCPROPERTYSET('0rvtrs000000000000002A',#5,'Pset_WallCommon',$,(#133,#134,#135,#136,#137));
#139=IFCRELDEFINESBYPROPERTIES('0rvtrs000000000000002B',#5,$,$,(#53),#138);
#146=IFCRELCONTAINEDINSPATIALSTRUCTURE('0rvtrs000000000000002I',#5,$,$,(#53,#65,#89,#101,#113,#124,#126,#128,#130),#22);
ENDSEC;
END-ISO-10303-21;
```

Notable properties:

- **Real SI units.** Length in millimetres, area / volume SI. Feet →
  metres is done on the way through the writer (10 ft → 3.048 m,
  20 ft → 6.096 m, 8 inches → 0.2032 m).
- **Deterministic GlobalIds.** Every IFC entity ID has the form
  `0rvtrs…` — produced by a seeded counter in the model builder, not
  random bytes, so the fixture is byte-stable across rebuilds
  (see Demo 5).
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

<!-- TODO: add screenshot of tests/fixtures/synthetic-project.ifc loaded in BlenderBIM -->

For the end-to-end `.rvt` → `.ifc` path:

```bash
./target/release/rvt-ifc samples/rac_basic_sample_family-2024.rfa
# rvt-ifc: wrote 1847 bytes to samples/rac_basic_sample_family-2024.ifc
```

Today that path emits a valid IFC4 spatial tree (Project + Site +
Building + Storey) but per-element branches are sparse — the
end-to-end Revit walker that drives every decoded `Wall` / `Floor` /
`Door` into `build_ifc_model` the way the synthetic test does is
[L5B-01](../TODO-BLINDSIDE.md) + a handful of IFC tasks
*(not yet implemented)*.

---

## Demo 5: Round-trip verification (byte-stable IFC output)

Goal: re-running the IFC writer with the same inputs produces
byte-identical output. This is what lets
`tests/fixtures/synthetic-project.ifc` be treated as a regression
artefact that `diff` can assert against.

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
GlobalId counter is deterministic; entity and property ordering is
stable by construction. With those three in place, re-running the
writer on the same `IfcModel` twice produces byte-equal strings, and
re-running the fixture-generating test with `DUMP_IFC=1` produces a
file whose only varying line is line 4 (the timestamp).

This matters for CI: if someone changes `src/ifc/step_writer.rs` and
the diff of `tests/fixtures/synthetic-project.ifc` isn't what they
expected, the failure surfaces in a pull request as a concrete textual
diff rather than a flaky integration test. The 157-line fixture pins
the IFC4 header, the spatial-tree topology (1 site → 1 building → 3
storeys), the seven `IFCEXTRUDEDAREASOLID` chain (4 walls + 1 slab + 1
door + 1 opening clone), the opening-element chain, two materials with
colour + transparency, the two property sets, and per-element
containment into the correct storey.

Anything that regresses any of the above breaks the test — which is
exactly what you want.

---

## What the demos do not cover

Honest boundary of what rvt-rs can demonstrate today:

- **Full `.rvt` → `.ifc` with real geometry** — `rvt-ifc` produces a
  valid spatial tree + classification, but per-element geometry
  extraction from the Revit file's native surface / solid / mesh
  encoding lives in the [GEO-27 … GEO-34] backlog
  *(not yet implemented)*. The synthetic fixture demonstrates what the
  *writer* does with decoded input; the live `.rvt` → decoded-input
  bridge is the next frontier.
- **Revit 2016–2023 ADocument walk** — validated on 2024–2026 only;
  older releases return `ADocument record not locatable`
  *(not yet implemented)*.
- **Field-level writes** — stream-level byte-preserving round-trip
  works (13/13 streams), but rewriting a specific schema field back
  into a Revit file is the [WRT-01 … WRT-14] task family
  *(not yet implemented)*.
- **Web viewer** — the [VW1-*] tasks (WASM build, Three.js viewer,
  glTF export) are *(not yet implemented)*.

Each of these lands as individual commits on `main` with linked task
IDs (`git log --oneline | grep -E "(IFC|GEO|L5B|WRT)-"`).
