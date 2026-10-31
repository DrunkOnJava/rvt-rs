# Roadmap

Last reviewed: 2026-10-30

This roadmap is the public, contributor-facing view of where rvt-rs is headed.
The support boundary lives in [`docs/status.md`](docs/status.md) and the
machine-readable [support matrix](docs/support-matrix.json); the statuses below
are its ceilings and this page never claims more. The plan itself is the GitHub
milestones and the issues under them; a milestone is closed when its release
ships. [`docs/archive/TODO.md`](docs/archive/TODO.md) is the older task decomposition they grew out of,
and [`docs/maintaining.md`](docs/maintaining.md) says how the tracker is run.

## Product Goal

rvt-rs should become a first-class open-source utility for BIM and AEC users who
need to inspect, validate, and exchange Revit files without installing Revit or
uploading private models to a third-party service.

It is not a production Revit-to-IFC converter yet, and the matrix says so
(`converter-grade-rvt-ifc` is unsupported). It opens every Revit file from 2016
to 2026 and reads its metadata, previews and embedded schema, and Revit 2015
and 2027 files too, measured on Autodesk's sample projects. On Revit 2024 and
2025 project files it reads the building: each element's ElementId, category,
name, type, storey, materials, layers and GlobalId, and a body that is exact
where the element's data is decoded and its bounding box where it is not. It
writes that as IFC4, glTF, plan SVG and CSV from the CLIs, Python and the
zero-upload browser viewer. Revit 2023 projects get a narrower version of the
same, and Revit 2015 to 2022, 2026 and 2027 projects are experimental. Every
recovery claim is measured against the oracle its report names: Revit's own
IFC export of the same model wherever one is measured, and for 2015 to 2018,
2019 to 2022 and 2027, which have no measured same-release export (except one
2015 project, RE-198), the same
sample projects saved by Revit
2019, 2023 and 2026 (but 2015's `rst_basic`, another model than its 2019 copy).
What is not decoded
(most parameters in IFC, family geometry, roofs by extrusion outside
Revit 2025, openings whose symbol cuts no outline, the phase check of rooms) is listed in [`docs/status.md`](docs/status.md), and
the four levels a file can reach are in the
[supported profile](docs/supported-profile.md).

## Current Position

| Area | Status | Current state | Next decision point |
|---|---|---|---|
| Container, compression, metadata | Verified | MS-CFB container, Revit's truncated-gzip streams, PartAtom and previews open on every release from 2016 to 2027; 2015 project files open on Autodesk's sample projects, where PartAtom and previews are not measured. | Maintain compatibility and bounds checks against hostile input. |
| `Formats/Latest` schema | Verified | The whole schema is read (#410: 4,126 classes on Revit 2024, where 395 were read before). A class's tag is its definition ordinal (#154), and element records name their class by it (RE-76). | Classify the 9 to 12 residual field encodings per release, only with byte evidence. |
| ADocument walker | Partial | Document-level metadata for triage; the root ADocument walk is validated on 2024 to 2026. | Expand confidence across project releases and older files. |
| Typed project elements | Partial | Elements come from their partition element records, typed by `BuiltInCategory` and matched against Revit's own exports. On Revit 2024 walls, doors, windows, columns, floor slabs and building pads match element for element (verified, RE-21); 2025 and 2023 are partial; 2026 is experimental (one local model), and so are 2015 to 2022 and 2027, measured against the same sample projects saved by Revit 2019, 2023 and 2026 (RE-181, RE-180, RE-179), but 2015's `rst_basic`, another model. Schema-field walls and opening-index doors and windows stay unsupported (RE-19). | A Revit export of a 2016 to 2022 or 2027 project to measure against, and a second 2015 one (#421, #878, #879), instance and type parameters in IFC (#35, #155, 0.11.0), held-out models (#408). |
| IFC writer | Partial | IFC4 with storeys, IFC type objects, material layer sets, materials and stable generated GlobalIds. Bodies are exact where the element's data is decoded and the record's bounding box elsewhere, and the diagnostics count each (#409). | The geometry issues open under the 0.11.0 milestone (below). |
| Browser viewer | Partial | Zero-upload WebAssembly viewer over the same decode; its file status shows how much of a model is a stand-in box. | Keep the journey test, which since 0.7.0 opens, inspects, schedules, exports and checks the diagnostics of the six redistributable reference models, green as the viewer grows. |
| Python and CLI | Partial | The `rvt` wheel on PyPI and prebuilt CLI archives on GitHub Releases, sharing the core's honesty bounds. | Stabilize JSON schemas and the one-shot inspect workflow. |
| Write path | Partial | Stream-level patching only. Field-level semantic writes are unsupported and gated by [ADR-002](docs/decisions/ADR-002-semantic-write-api-gate.md). | Stay gated until openability can be proven. |

## Milestones

### Released

- **0.2.0, 2026-09-23: audit-clean alpha.** Repository hygiene, honest public
  status, issue workflow and enforceable quality gates; prebuilt CLIs and the
  Python package. An inspection-focused alpha with experimental export.
- **0.3.0, 2026-09-27: trustworthy Revit 2024/2025 export.** IFC and glTF that
  read like Revit's own export: walls, floors, roofs and ceilings as their
  layers and materials, elements named and placed on Revit's storeys, stable
  generated GlobalIds (#400), declared units (#403), the whole schema (#410),
  and diagnostics that count the stand-in boxes (#409).
- **0.4.0, 2026-09-28.** Revit 2023 projects export typed elements as 2024
  and 2025 do; rooms take their real outline, number and name; doors and windows
  cut the opening their type specifies; turned family instances are drawn
  turned; steel members carry their I section; every typed element is related
  to an IFC type object of its Revit type.
- **0.5.0, 2026-10-18.** Revit 2026 projects export typed elements,
  experimentally; MEP models gain ports, port-to-port connections and systems,
  and pipes are drawn as the cylinders Revit draws; exports carry each
  element's levels, its category, family, type and phase, and more of the
  common property sets; rooms are aggregated into their storeys; Autodesk's
  Snowdon Towers samples export their elements again. The first release with
  outside contributors' pull requests.
- **0.5.1, 2026-10-18.** An export is named by the model's file name rather
  than the path it was saved at, which could carry the author's account and
  folder names (#782), and the README names the release it ships in.
- **0.6.0, 2026-10-19.** The standard property sets are IFC4's: those 0.5
  took from Revit's IFC2X3 exports moved to where IFC4 and Revit's own IFC4
  export put them, checked against buildingSMART's published IFC4 property
  set definitions on every measured export. Exports carry parameters named
  for a standard set's property, `IsExternal` (curtain mullions' and panels'
  by their curtain wall's, RE-288), type Fire
  Ratings, door sizes, Revit 2024 Areas as spaces, and port types, with
  electrical ports' direction (HVAC and piping directions are not read yet,
  #834); Revit 2015 to 2022 and 2027 projects export their elements,
  experimentally.
- **0.6.1, 2026-10-19.** 0.6.0 with an export panic on 32-bit targets fixed
  (#836) and a README whose support table says what is measured, built
  from 0.6.0 because `main` holds a breaking change (#841).
- **0.7.0, 2026-10-24.** Gable, hip and slope-arrow roofs drawn as their
  planes, Revit 2025 roofs with their slopes and roofs by extrusion as their
  sketch's section; Revit 2025's tapered walls; doors' and windows' overall
  sizes and openings from the outline their symbol cuts, on Revit 2024 and
  2025; ports placed, with
  Revit's flow directions, names and descriptions on HVAC and piping ports;
  Revit's materials on family instances, mullions, columns and walls; each
  element's saved parameters in `rvt-elements` and Python; reproducible
  exports; and the stable API 1.0 keeps, with the research readers behind
  `unstable-research`.
- **0.8.0, 2026-10-25.** Revit 2025 stair runs drawn as their treads and
  risers from their native records, spiral, riserless and monolithic runs
  as their steps or treads, and landings as their slabs or their one tread,
  with Revit's predefined types, names and type objects; walls that end
  where Revit's do at T joints (482 of 607 on the BIM4LCA office, from
  412); square hollow-section columns as their section; door and window
  sizes where the symbol cuts no outline; Revit 2024 pipes as cylinders;
  the project's, site's, building's and storeys' stored GlobalIds; Revit's
  property sets (`--revit-property-sets`); and `--anonymize` for
  diagnostics posted publicly.
- **0.9.0, 2026-10-26.** IFC4 base quantities as Revit's export writes
  them, on by default (20,157 equal on the BIM4LCA office as built, none
  that Revit's export leaves out): curtain mullions' `Length` and
  `NetVolume`, doors' and windows' sizes, walls' `Width` and their
  openings' `Depth`, stair flights', pipes' and unjoined columns'; curtain
  mullions' `Span`, walls' `ExtendToStructure`, railings' `Height`, level
  slabs' `PitchAngle`, stair flights' treads and nosing and more
  `Reference` values; types and hosts from native records; and
  `--revit-set-placement`.
- **0.10.0, 2026-10-27.** More of Revit's base quantities (21,065 equal on
  the BIM4LCA office as built, none that Revit's export leaves out): walls'
  `Length` and every wall's `Width`, curtain walls' and beams' `Length`,
  floors' `GrossArea`, `Depth` and `GrossVolume`, ceilings' `Width`,
  hollow-section columns' areas and volume, spaces' `Height` and stair
  railings' `Length`; and system-family elements' type parameters
  (puzzbobb, #1065).
- **0.11.0, 2026-10-30.** A model over the reader's resource limits is
  reported instead of exporting empty, with `--large-model` and a viewer
  retry (#1075); files that kept an older release's schema read as that
  release (puzzbobb, #421), neither measured yet on the reporter's private
  files; elements outside the export phase left out
  (puzzbobb, #328); doors and windows drawn from their saved meshes by
  default; windows' openings from their cut solids; walls' `Height` where
  nothing joins or cuts them; Revit 2008 to 2014 experimental (puzzbobb);
  and the unused stable API removed.

### 0.12.0: IFC geometry and element data

The geometry and element-data work that continues after 0.11.0, moved from the
0.11.0 milestone when 0.11.0 shipped. Each
slice records a baseline, an improvement and a residual against Revit's own
exports on the reference pack (#407), approximations stay labelled (#409), and
a slice ships in the next release that contains it. Geometry open now: wall
bodies thicker or thinner than their type's layers (#358) and the tops of walls Revit
steps over a floor that covers them in part (#1067), stair flights
(#357), roofs Revit splits into slab parts (#323), hip roofs over T and U footprints and roof cases
with no oracle (#884, #885), the openings on releases other than 2024 and 2025
(#227), room boundaries whose solid does not
close (#90), beams (#94), MEP equipment and routes (#96) and ports and pipes outside
Revit 2024 and 2025 (#869), the sketch-to-solid pipeline (#156), and the Snowdon slab
that is not in Revit's export (#309). Element data: instance and type parameters with units and precedence
(#35, #155) and the element-record fields that carry them (#223), written to
IFC property sets and shown in the viewer; door and window sizes where the
symbol cuts no outline, two doors still wide (#810); and ports' system classification (#834).

### 1.0.0: first-class utility

A dependable open-source Revit data reader: a new user installs it, opens a
model, gets correct elements, geometry and properties, and can reproduce the
measurements. 1.0 supports every Revit release from 2015 to 2027 and ships with
no gaps or limitations: every body is Revit's own shape, every property
Revit's export writes is written, and no release is experimental. The
requirements, each with its bar, measured value and evidence, are in
[`docs/release-1.0-criteria.md`](docs/release-1.0-criteria.md), which is the
release decision; the milestone's completion percentage is not. They include
held-out models measured before fixes (#408), every release with a Revit
export of its own (#421), the surfaces the stability commitment covers (#873),
APIs documented with examples CI runs (#872), and practitioners' reports
(#874). Breadth (more releases, more categories) never lowers measured
correctness.

## Contribution Priorities

Start with [`docs/contribution-map.md`](docs/contribution-map.md). The highest
leverage work is:

1. Openly licensed models with a Revit export of the same file, and a
   held-out check on them (#407, #408).
2. Partition-stream probes that turn byte observations into falsifiable decoder
   hypotheses: one example that states the fact it proves and how to verify it
   against the corpus, and a report with measured counts.
3. Fixture assertions that prevent decoders from claiming false positives.
4. Documentation that keeps user-facing support boundaries honest.
5. Viewer diagnostics that explain what the tool could and could not decode.

## Out of Scope

rvt-rs will not:

- Use Autodesk proprietary SDK internals, leaked documents, or decompiled
  proprietary implementation code. See [`CLEANROOM.md`](CLEANROOM.md).
- Claim production RVT-to-IFC conversion before real project typed elements and
  geometry are corpus-proven.
- Provide a Revit API-compatible surface.
- Edit Revit model data at the field level before the ADR-002 gate opens.
- Resolve cloud-worksharing, licensing, or external linked-model semantics in
  the near-term product.
