# Roadmap

Last reviewed: 2026-09-30

This roadmap is the public, contributor-facing view of where rvt-rs is headed.
The support boundary lives in [`docs/status.md`](docs/status.md) and the
machine-readable [support matrix](docs/support-matrix.json); the statuses below
are its ceilings and this page never claims more. The plan itself is the GitHub
milestones and the issues under them; a milestone is closed when its release
ships. [`TODO.md`](TODO.md) is the older task decomposition they grew out of,
and [`docs/maintaining.md`](docs/maintaining.md) says how the tracker is run.

## Product Goal

rvt-rs should become a first-class open-source utility for BIM and AEC users who
need to inspect, validate, and exchange Revit files without installing Revit or
uploading private models to a third-party service.

It is not a production Revit-to-IFC converter yet, and the matrix says so
(`converter-grade-rvt-ifc` is unsupported). It opens every Revit file from 2016
to 2026 and reads its metadata, previews and embedded schema. On Revit 2024 and
2025 project files it reads the building: each element's ElementId, category,
name, type, storey, materials, layers and GlobalId, and a body that is exact
where the element's data is decoded and its bounding box where it is not. It
writes that as IFC4, glTF, plan SVG and CSV from the CLIs, Python and the
zero-upload browser viewer. Revit 2023 projects get a narrower version of the
same, and Revit 2026 is experimental. Every recovery claim is measured against
Revit's own IFC export of the same model. What is not decoded (most parameters,
family geometry, hip and gable roofs, opening profiles, phase filtering, other
releases' element records) is listed in [`docs/status.md`](docs/status.md), and
the four levels a file can reach are in the
[supported profile](docs/supported-profile.md).

## Current Position

| Area | Status | Current state | Next decision point |
|---|---|---|---|
| Container, compression, metadata | Verified | MS-CFB container, Revit's truncated-gzip streams, PartAtom and previews open on every release from 2016 to 2026. | Maintain compatibility and bounds checks against hostile input. |
| `Formats/Latest` schema | Verified | The whole schema is read (#410: 4,126 classes on Revit 2024, where 395 were read before). A class's tag is its definition ordinal (#154), and element records name their class by it (RE-76). | Classify the 9 to 12 residual field encodings per release, only with byte evidence. |
| ADocument walker | Partial | Document-level metadata for triage; the root ADocument walk is validated on 2024 to 2026. | Expand confidence across project releases and older files. |
| Typed project elements | Partial | Elements come from their partition element records, typed by `BuiltInCategory` and matched against Revit's own exports. On Revit 2024 walls, doors, windows, columns, floor slabs and building pads match element for element (verified, RE-21); 2025 and 2023 are partial, 2026 is experimental (one local model). Schema-field walls and opening-index doors and windows stay unsupported (RE-19). | Element records of other releases (#421), parameters (0.5.0), held-out models (#408). |
| IFC writer | Partial | IFC4 with storeys, IFC type objects, material layer sets, materials and stable generated GlobalIds. Bodies are exact where the element's data is decoded and the record's bounding box elsewhere, and the diagnostics count each (#409). | The geometry issues open under the 0.4.x milestone (below). |
| Browser viewer | Partial | Zero-upload WebAssembly viewer over the same decode; its file status shows how much of a model is a stand-in box. | The viewer journey verified end to end on the reference pack (0.5.0). |
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

### 0.4.x: IFC geometry beta, remaining slices

v0.4.0 shipped part of the geometry beta. The milestone of that name is closed,
and the slices still open moved to this one. Each slice records a baseline, an
improvement and a residual against Revit's own exports on the reference pack
(#407), approximations stay labelled (#409), and a slice ships in the next
release that contains it. Open now: wall bodies thicker or thinner
than their type's layers (#358), stair flights (#357) and aggregates (#323),
sloped roofs (#356, needs a pitched-roof oracle), element materials (#355),
the opening cut from the wall location curve (#227), room boundaries whose
solid does not close (#90), beams (#94), MEP equipment and routes (#96), the
sketch-to-solid pipeline (#156), and the Snowdon slabs that are not in Revit's
export: six in the Legends phase (#328) and one more (#309).

### 0.5.0: element data (parameters) and viewer journey

Make rvt-rs a data source, not only a viewer: instance and type parameters with
units and precedence (#35, #155), the element-record fields that carry them
(#223, #228), shown consistently in Rust, Python, CLI schedules, IFC property
sets and the viewer, with the viewer journey (open, inspect, schedule, export,
diagnostics) verified end to end on the reference pack.

### 1.0.0: first-class utility

A dependable open-source Revit data reader: a new user installs it, opens a
supported model, gets correct elements, geometry and properties with every
approximation visible, and can reproduce the measurements. Held-out licensed
models are measured before fixes (#408), a reference pack and a
privacy-preserving contribution path exist (#407), and the Rust and Python APIs
are documented with runnable examples. Breadth (more releases, more categories)
never lowers measured correctness.

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
