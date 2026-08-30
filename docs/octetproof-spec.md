# Technical Specification: OctetProof — A License-Free Verification Protocol for Undocumented Binary Formats

**Version:** 1.0.0
**Date:** 2026-08-30
**Status:** Specification — 1.0.0. Supersedes the draft received from the project owner on 2026-08-30, which is retained verbatim at [`docs/octetproof-spec-draft.md`](octetproof-spec-draft.md) with its reviewer notes. The corrections applied here are listed in Section 19.
**License of this document:** CC-BY-4.0
**Reference implementation:** rvt-rs (Apache-2.0) — in-repo instance; umbrella repository planned
**Primary domain:** Building Information Modeling (BIM) closed formats, with generalization to any undocumented binary container format

-----

## 1. Abstract

This specification defines **OctetProof**, a public, license-free protocol for verifying the correctness of readers for undocumented or proprietary binary file formats. It replaces the current industry default — trusting a single closed reader or a single undocumented implementation — with a **cross-witness agreement model**.

A format is treated as verified not because it matches a published specification (none exists for the formats in scope), but because **N independent, independently-implemented witnesses** produce **semantically equivalent observations** on the same **golden artifact**, and those observations are recorded with **cryptographic provenance**.

The protocol is designed so that:

- No proprietary SDK, license, or vendor binary is required after the initial artifact generation.
- Any reader — open or commercial — can participate as a witness.
- Disagreement is a first-class, machine-checkable event.
- The entire chain is replayable by a stranger with no special hardware.

The first concrete instance targets Autodesk Revit (`.rvt` / `.rfa`) and AutoCAD DWG (`.dwg`) files. The protocol is format-agnostic and is intended to generalize to STEP, IGES, DGN, Navisworks, glTF, E57, and beyond.

-----

## 2. Motivation and Problem Statement

### 2.1 The silent-corruption problem

Undocumented binary formats have no external oracle. A reader can decode bytes, produce plausible output, and be completely wrong — and the caller has no way to know. This is not theoretical. In the reference implementation `rvt-rs`, a checksum-paged stream decompression bug caused approximately **48% schema loss** on streams larger than ~190 KB, with the decoder terminating cleanly and reporting success. The output looked valid. It was not.

This class of failure is worse than a hard crash: it produces **confident, plausible garbage**.

### 2.2 Why existing approaches fail

|Approach                                |Failure mode                                                         |
|----------------------------------------|---------------------------------------------------------------------|
|Single closed reader (ODA BimRv, Teigha)|Vendor is both author and judge; no independent check                |
|Single open reader                      |No external ground truth; self-consistency is not correctness        |
|Published spec                          |Does not exist for RVT; DWG spec is reverse-engineered and incomplete|
|Human review                            |Does not scale; cannot catch silent drift                            |
|Statistical testing                     |Wrong frame for deterministic byte formats                           |

### 2.3 The core insight

Correctness for an undocumented format is not "matches the spec." It is **agreement across independent implementations on the same input**, where each implementation was derived from different evidence (different samples, different reverse-engineering sessions, different languages, different authors).

A single implementation can be wrong in a correlated way. Two independent implementations agreeing is evidence. Three is strong evidence. Agreement across a **format boundary** (e.g., RVT → DWG → parse) is the strongest form, because the two witnesses never shared a codebase or a sample set.

-----

## 3. Definitions

- **Source format (S):** the undocumented or proprietary format under test (e.g., RVT).
- **Bridge format (B):** a documented or independently-specified format reachable from S via an export or conversion path (e.g., DWG, IFC, DXF).
- **Golden artifact (G):** a committed, immutable file in format B, generated from a known source in S, with full provenance.
- **Witness (W):** an independently implemented reader that can parse G and emit a normalized observation.
- **Observation (O):** a canonical, witness-emitted JSON document describing the semantic content of G as that witness understands it.
- **Agreement:** two or more observations on the same G are equivalent under the defined diff function for the claimed semantic surface.
- **Disagreement:** observations differ on a field within the claimed surface; this is a protocol event, not a bug report.
- **Provenance record (P):** the signed, hash-chained metadata binding G to its source, generator, and witnesses.
- **Semantic surface (Σ):** the subset of meaning a witness claims to recover; agreement is only required within Σ.
- **Faithful export surface:** the subset of bridge-format outputs where the export from S to B is lossless enough that cross-witness agreement is meaningful.

-----

## 4. Threat Model

### 4.1 In scope

- Silent corruption in a single reader (the 48% schema-loss class).
- Circular oracles: a reader checked against files it (or a correlated tool) generated.
- Correlated implementation error: two readers sharing a common reverse-engineering error.
- License contamination: GPL or commercial SDK leaking into an Apache-2.0 tree.
- Provenance forgery: a golden artifact claimed to come from a source it did not.
- Replay non-determinism: a witness producing different observations on the same bytes across runs.

### 4.2 Out of scope (explicit non-goals)

- Proving that the *original* source file in S is semantically correct as authored by a human. The protocol verifies readers, not authors.
- Defending against a malicious witness that deliberately lies. Witnesses are trusted to the degree of their independence; a colluding set of witnesses can agree on a lie. Mitigation is diversity of origin, not cryptography.
- Real-time or streaming verification. All checks are batch, offline, CI-gated.
- Formal verification of decoder logic. The protocol is empirical, not proof-carrying.

### 4.3 Assumptions

- At least one licensed seat of the source application (e.g., Revit) is available for a bounded, recorded generation session.
- Bridge formats have at least two independent, non-colluding readers.
- Golden artifacts are immutable once committed; updates create new artifacts with new IDs.
- Witnesses are deterministic given identical input bytes and identical configuration.

-----

## 5. Architecture

The system has five layers. Each layer has a single responsibility and a single owner.

### 5.1 Layer 1 — Protocol Specification

A versioned, citable document (this document) defining:

- The artifact schema.
- The observation schema.
- The diff function.
- The provenance record format.
- The CI gate rules.
- The witness registration process.

This layer is the scholarly contribution. It is published, versioned, and reviewable independently of any implementation.

### 5.2 Layer 2 — Golden Artifact Corpus

A directory of immutable artifacts, each in its own subdirectory, each with a `manifest.json`.

```
corpus/
├── README.md
├── MANIFEST_INDEX.json
└── artifacts/
    ├── g-2026-0001/
    │   ├── manifest.json
    │   ├── source.rvt.sha256
    │   ├── bridge.dwg
    │   ├── bridge.dwg.sha256
    │   ├── export_recording.mp4 (optional, redacted)
    │   ├── observations/
    │   │   ├── acadsharp.json
    │   │   ├── jdwgparser.json
    │   │   ├── libredwg.json
    │   │   └── rvt-rs-via-dwg.json
    │   └── verdict.json
    ├── g-2026-0002/
    └── ...
```

Each artifact is content-addressed. The `manifest.json` binds the bridge file hash, the source file hash, the generator identity, the export settings, and the semantic surface claimed.

The layout above is the umbrella form. The in-repo instance (Section 16.3) carries the same content under `research/witness/<artifact_id>/{observations/*.json, verdict.json}`, with the manifest living beside the decoder's own count fixtures and the artifact bytes fetched by hash from the upstream dataset rather than vendored. Directory layout is not normative; the manifest, observation, and verdict shapes are.

### 5.3 Layer 3 — Witness Registry

A machine-readable index (`registry.yaml`) of every participating reader.

```yaml
witnesses:
  - id: acadsharp
    language: C#
    license: MIT
    versions: [R14, R2000, R2004, R2007, R2010, R2013, R2018]
    coverage: undeclared
    ci_eligible: true
    copyleft: none
  - id: jdwgparser
    language: Java
    license: GPL-3.0
    versions: [R2000, R2004, R2007, R2010, R2013, R2018]
    coverage: "project's own claim: 74/74 entity types, 92.2% sample pass rate"
    ci_eligible: true
    copyleft: strong   # secondary, isolated-process witness only
  - id: libredwg
    language: C
    license: GPL-3.0
    versions: [R13, R14, R2000, R2004, R2007, R2010, R2013, R2018]
    coverage: "project's own claim: ~99% read"
    ci_eligible: true
    copyleft: strong   # secondary, isolated-process witness only
  - id: rvt-rs
    language: Rust
    license: Apache-2.0
    versions: [2016, 2017, 2018, 2019, 2020, 2021, 2022, 2023, 2024, 2025, 2026]
    coverage: "schema 100%, typed extraction partial"
    ci_eligible: true
    copyleft: none
    role: primary_source_witness
  - id: ifc-openshell
    language: C++/Python
    license: LGPL-3.0
    versions: [IFC2x3, IFC4, IFC4x3]
    ci_eligible: true
    copyleft: weak     # dynamic link only, per the Section 9.2 row
  - id: ifc-lite
    language: Rust
    license: MPL-2.0      # LTplus-AG/ifc-lite; crate ifc-lite-core, pinned =7.1.1
    versions: [IFC2x3, IFC4, IFC4x3, IFC5]
    ci_eligible: true
    copyleft: weak
```

`copyleft` is a three-valued declaration — `none`, `weak`, `strong` — not a boolean. Weak copyleft (LGPL, MPL, CDDL) permits the Section 9.2 dynamic-link treatment; strong copyleft (GPL, AGPL) permits only the isolated-process treatment. A witness whose license this registry has not verified against its upstream declares `license: unverified` and `copyleft: unknown`, and cannot be one half of a sole agreeing pair until it is verified.

Coverage figures quoted from a project's own README are recorded as that project's claim, not as a registry finding, until an agreement recorded under this protocol reproduces them. A witness whose coverage no agreement has reproduced declares `coverage: undeclared`.

The registry is the single source of truth for what the CI gate tests. A witness not in the registry is not tested.

#### 5.3.1 Normative mapping to the in-repo registry

The reference implementation carries the registry as `research/witness-registry.json`, whose field names differ from the `registry.yaml` vocabulary above. The correspondence below is normative: a conforming implementation MAY use either encoding, and these are the fields that MUST carry the same meaning across both.

|Spec field (`registry.yaml`, §5.3 / §9.4)|`research/witness-registry.json`|State                                                                                                                                 |
|-----------------------------------------|--------------------------------|--------------------------------------------------------------------------------------------------------------------------------------|
|`id`                                     |`id`                            |Implemented                                                                                                                           |
|`language`                               |`language`                      |Implemented; may be `null` for an authoring witness                                                                                   |
|`license`                                |`license`                       |Implemented as an SPDX identifier, `commercial (<vendor>)`, `none declared`, or `unverified`                                           |
|`copyleft`                               |derived from `license`          |Implemented in the gate, not stored: `witness-verdict.py` reads `GPL*` / `AGPL*` as strong copyleft and `commercial` as gate-ineligible|
|§9.1 repository reference                |`repo`                          |Implemented as `owner/name`, or `null` where the witness has no public repository                                                     |
|§9.3 lineage rule                        |`lineage`                       |Implemented — names the witness this one is built on; the gate counts a witness and its lineage parent as one lineage                  |
|`role: primary_source_witness`           |derived from the observation    |Implemented differently: role is per-verdict, taken from the observation's `input_role` (`source` / `bridge`) matched against the manifest hashes, not declared once in the registry|
|—                                        |`kind` (`reader` / `author`)    |Extension: the spec's witness model covers readers only; the in-repo registry also names the authoring witness that produced an edge   |
|—                                        |`node`                          |Extension: the format the witness reads, keyed to the registry's `nodes` list                                                          |
|—                                        |`status` (`adopted` / `candidate` / `rejected`)|Extension: survey state, not a gate input                                                                               |
|—                                        |`priority`                      |Extension: survey triage only, never normative                                                                                        |
|—                                        |`checked`                       |Extension: the date this registry verified the entry against the upstream API. Absent means unverified.                               |
|—                                        |`notes`                         |Extension; the registry's `candidate_claims` rule applies — any figure quoted here is the project's own claim                          |
|`ci_eligible`                            |not implemented                 |**Umbrella scope.** The in-repo gate runs a fixed witness set named by the CI job rather than filtering the registry.                  |
|`coverage` / §9.4 coverage declaration   |not implemented                 |**Umbrella scope.** The per-run claim lives in the observation's `semantic_surface_covered`; there is no registry-level declaration to check it against.|
|`versions`                               |not implemented                 |**Umbrella scope.** `node` names the format, not the supported version range.                                                          |
|§9.5 determinism attestation             |not implemented                 |**Umbrella scope.** The observation carries a per-run `deterministic` flag; there is no cross-OS, cross-architecture attestation.      |
|§9.6 exact version pinning               |not implemented                 |**Umbrella scope.** The observation carries `witness_version`; CI pins IfcOpenShell to a version range, not a commit.                  |

Fields marked **umbrella scope** are required by this specification but are not enforceable inside a single decoder repository, because they describe a multi-repository gate. They become mandatory when the umbrella repository is created (Section 16.3).

### 5.4 Layer 4 — Cross-Witness CI Gate

A GitHub Actions workflow (or equivalent) that, on every PR touching a decoder or the corpus:

1. Loads the registry.
2. For each golden artifact whose claimed surface intersects the changed code:
- Runs every `ci_eligible` witness that claims the bridge format.
- Normalizes each observation to canonical JSON (RFC 8785 / JCS).
- Applies the diff function.
- Emits a `verdict.json`.
3. Fails the build on any disagreement within the claimed semantic surface.
4. Publishes the full observation set and verdict as a build artifact for human review.

The gate is **fail-closed**: absence of a witness is not agreement. If only one witness runs, the verdict is `INSUFFICIENT_WITNESSES`, not `PASS`.

### 5.5 Layer 5 — Decoder Repositories

`rvt-rs`, `dwg-rs`, and future readers remain separate repositories. They:

- Consume the protocol (Layer 1) as a dependency or documented contract.
- Emit observations in the canonical schema when run in witness mode.
- Are checked by Layer 4; they do not self-certify.
- Ship independently; the meta-repo never contains their source.

This separation is load-bearing. It prevents the meta-repo from becoming a monorepo that recreates the original coordination problem at larger scale.

-----

## 6. Data Schemas

Every example in this section is valid JSON and matches the reference implementation's emitted shape. The observation and verdict schemas are published as machine-checkable JSON Schema 2020-12 documents:

- [`docs/schemas/witness-observation.schema.json`](schemas/witness-observation.schema.json)
- [`docs/schemas/witness-verdict.schema.json`](schemas/witness-verdict.schema.json)

The worked example throughout is the first committed artifact, `magnetar-2024-core-interior`: the RVT → IFC edge, with `rvt-rs` as the source witness and IfcOpenShell as the bridge witness. The files quoted below are the real committed ones under `research/witness/magnetar-2024-core-interior/`.

### 6.1 Golden Artifact Manifest

The manifest binds the source and bridge hashes and declares, per category, whether that category is inside the claimed semantic surface. The v1 shape in use is the project-count manifest (`tests/fixtures/project-counts/2024-core-interior.json`), abbreviated here to three of its thirteen categories:

```json
{
  "schema_version": 1,
  "id": "magnetar-2024-core-interior",
  "project_file": "2024_Core_Interior.rvt",
  "reference_ifc_file": "2024_Core_Interior.ifc",
  "source": {
    "repo": "magnetar-io/revit-test-datasets",
    "license": "MIT",
    "revit_release": 2024,
    "rvt_sha256": "c805df445d613b408e37337765572021265e3f5dfdc7d1fa53b22ba1600b8014",
    "rvt_bytes": 33718272,
    "reference_ifc_sha256": "d07c7462aee22640661faed5262cf802ce0fcbc663f312961a39be92bf857050",
    "reference_ifc_bytes": 20392,
    "notes": "The paired IFC export is the only authoritative count source currently available for this project. It appears to be an element-export fixture, not a full project schedule."
  },
  "counts": {
    "walls": {
      "status": "known",
      "expected": 0,
      "source": "paired Revit IFC export",
      "source_ifc_type": "IFCWALL",
      "tolerance": 0,
      "decoder_metric": "diagnostics.exported.by_ifc_type.IFCWALL",
      "decoder_expected": 0,
      "decoder_tolerance": 0
    },
    "units": {
      "status": "known",
      "expected": 1,
      "source": "paired Revit IFC export",
      "source_ifc_type": "IFCUNITASSIGNMENT",
      "tolerance": 0,
      "decoder_metric": "step.IFCUNITASSIGNMENT",
      "decoder_expected": 1,
      "decoder_tolerance": 0
    },
    "materials": {
      "status": "known_gap",
      "expected": 1,
      "source": "paired Revit IFC export",
      "source_ifc_type": "IFCMATERIAL",
      "tolerance": 0,
      "decoder_metric": "diagnostics.exported.material_count",
      "decoder_expected": 102,
      "decoder_tolerance": 5,
      "tracking_issue": 34,
      "unsupported_feature": "revit_compound_assemblies_and_walltype_widths",
      "notes": "Paired IFC expects only 1; compound assemblies and WallType widths remain open."
    }
  }
}
```

Normative reading of the fields the gate consumes:

- `id` — the artifact identifier carried into every observation and the verdict.
- `source.rvt_sha256` / `source.reference_ifc_sha256` — the two accepted input hashes. An observation whose `input_hash_sha256` matches the first has `input_role: "source"`; the second, `input_role: "bridge"`; neither, `REJECTED_INPUT`.
- `counts.<category>.source_ifc_type` — the field name compared across witnesses; a category without it is not part of the surface at all.
- `counts.<category>.status` — `known` puts `entity_counts.<source_ifc_type>` inside the claimed surface; `known_gap` and `unsupported` exclude it first-class (Section 7.1 rule 3), and the exclusion is recorded in the verdict with its `tracking_issue` and `unsupported_feature`.
- `counts.<category>.tolerance` — the per-category count tolerance (Section 7.2). Absent means exact.
- The `decoder_*` fields are the decoder repository's own regression baseline, not a cross-witness input. The gate ignores them.

**Planned extension — DWG bridge (not yet implemented).** When the RVT → DWG edge is recorded, the manifest gains a bridge block naming the export that produced it. This block is specified here so implementers can write against it; no manifest in the reference implementation carries it today:

```json
{
  "bridge": {
    "format": "DWG",
    "dwg_version": "AC1032",
    "export_mode": "2D_plan_view",
    "export_settings_hash": "0000000000000000000000000000000000000000000000000000000000000000",
    "file": "2024_Core_Interior_plan.dwg",
    "file_hash_sha256": "0000000000000000000000000000000000000000000000000000000000000000",
    "faithful_surface": [
      "entity_counts",
      "layer_topology",
      "linework",
      "bounding_boxes",
      "xdata_fields",
      "text_content"
    ],
    "explicitly_excluded": [
      "3d_solids",
      "meshes",
      "bim_parameters",
      "materials",
      "element_categories"
    ]
  }
}
```

`faithful_surface` and `explicitly_excluded` draw exclusively from the Section 9.4 controlled vocabulary. Witness identifiers never appear there; the witness set is derived from the observations present, not declared in the manifest.

### 6.2 Observation (`observations/<witness>.json`)

An observation is what one witness saw in one input, plus enough provenance to replay it. Every key below is required.

|Key                        |Type            |Meaning                                                                                                     |
|---------------------------|----------------|------------------------------------------------------------------------------------------------------------|
|`schema_version`           |string          |`"1.0.0"` for this specification                                                                            |
|`witness_id`               |string          |Registry id; must resolve in the registry when the gate runs with one                                        |
|`witness_version`          |string          |The exact version that produced this observation                                                            |
|`artifact_id`              |string          |The manifest `id`                                                                                           |
|`input_role`               |`source`/`bridge`|Which side of the edge this witness read                                                                    |
|`input_file`               |string          |File name as read                                                                                           |
|`input_hash_sha256`        |string          |SHA-256 of the input bytes; must match the manifest hash for the declared role                              |
|`deterministic`            |boolean         |The witness's attestation for this run; anything but `true` rejects the witness                              |
|`semantic_surface_covered` |array of string |Section 9.4 vocabulary; a witness is compared only on what it declares                                        |
|`observation`              |object          |The payload — the only part that is hashed and diffed                                                        |
|`observation_hash_sha256`  |string          |SHA-256 over the canonicalized `observation` payload (Section 7.3)                                            |
|`unsupported_entities`     |array of string |What the witness could not read; recorded, never a disagreement (Section 7.1 rule 4)                          |
|`warnings`                 |array of string |Non-fatal notes from the run                                                                                 |

The bridge witness, IfcOpenShell reading the Revit-authored IFC — the complete committed file:

```json
{
  "artifact_id": "magnetar-2024-core-interior",
  "deterministic": true,
  "input_file": "2024_Core_Interior.ifc",
  "input_hash_sha256": "d07c7462aee22640661faed5262cf802ce0fcbc663f312961a39be92bf857050",
  "input_role": "bridge",
  "observation": {
    "entity_counts": {
      "IFCBEAM": 0,
      "IFCCOLUMN": 0,
      "IFCDOOR": 0,
      "IFCFLOWTERMINAL": 0,
      "IFCMATERIAL": 1,
      "IFCPROPERTYSET": 25,
      "IFCROOF": 0,
      "IFCSHADINGDEVICE": 1,
      "IFCSPACE": 0,
      "IFCUNITASSIGNMENT": 1,
      "IFCWALL": 0,
      "IFCWINDOW": 0
    },
    "ifc_schema": "IFC4"
  },
  "observation_hash_sha256": "8cf4046509bb788406e93261f0dcb10708fbf33ae195d12f8ddc99b41a93398f",
  "schema_version": "1.0.0",
  "semantic_surface_covered": [
    "entity_counts"
  ],
  "unsupported_entities": [],
  "warnings": [],
  "witness_id": "ifcopenshell",
  "witness_version": "0.8.5"
}
```

The source witness, rvt-rs reading the `.rvt` directly — the same file, abbreviated in the `entity_counts`, `unsupported_entities`, and `warnings` arrays only:

```json
{
  "artifact_id": "magnetar-2024-core-interior",
  "deterministic": true,
  "input_file": "2024_Core_Interior.rvt",
  "input_hash_sha256": "c805df445d613b408e37337765572021265e3f5dfdc7d1fa53b22ba1600b8014",
  "input_role": "source",
  "observation": {
    "building_elements_with_geometry": 0,
    "entity_counts": {
      "IFCBUILDINGSTOREY": 12,
      "IFCMATERIAL": 102,
      "IFCPROPERTYSET": 64,
      "IFCSLAB": 64,
      "IFCSPACE": 18,
      "IFCUNITASSIGNMENT": 1
    },
    "exported_building_elements": {
      "IFCSLAB": 64,
      "IFCSPACE": 18
    },
    "material_count": 102,
    "storey_count": 12
  },
  "observation_hash_sha256": "b6d9b67c10c3350b69b58cbd2c6caeca405f63ce182e7291feba3e3ff10f3e00",
  "schema_version": "1.0.0",
  "semantic_surface_covered": [
    "entity_counts"
  ],
  "unsupported_entities": [
    "real_file_element_geometry",
    "floor_slab_extrusion_thickness"
  ],
  "warnings": [
    "No exported building elements include decoded geometry; see skipped diagnostics for missing curve/profile/dimension data."
  ],
  "witness_id": "rvt-rs",
  "witness_version": "0.1.2"
}
```

The committed hash `b6d9b6…` is over the full payload, not the abbreviation above; the abbreviated block is illustrative of shape only. Every other example in this section is byte-exact.

Two properties of the payload are load-bearing. First, the two witnesses share only `entity_counts`; everything else in a payload is witness-specific and is not diffed. Second, a witness may report a type the other never emits (`IFCSHADINGDEVICE` here) — the diff is driven by the manifest's declared surface, not by the union of the payload keys, and a type absent from a payload counts as zero.

Observations are **canonicalized** with RFC 8785 (JSON Canonicalization Scheme) before hashing or diffing. Key ordering, number formatting, and Unicode normalization are fixed. This guarantees bit-identical hashes across languages and platforms.

### 6.3 Verdict (`verdict.json`)

The verdict is the gate's output for one artifact. It records the whole decision, not just the outcome: which witnesses were compared, which inputs they read, which fields were inside the surface, which were excluded and why, any diffs, whether the independence set was satisfied, and whether the replay matched.

|Key                     |Required|Meaning                                                                                                |
|------------------------|--------|--------------------------------------------------------------------------------------------------------|
|`schema_version`        |yes     |`"1.0.0"`                                                                                              |
|`artifact_id`           |yes     |The manifest `id`                                                                                       |
|`status`                |yes     |One of the Section 10.5 vocabulary                                                                      |
|`witnesses_compared`    |yes     |Sorted witness ids remaining in the gate after commercial witnesses are dropped                          |
|`inputs`                |yes     |Per witness: `input_hash_sha256`, resolved `role`, `witness_version`                                     |
|`semantic_surface`      |yes     |The fields actually compared, as `entity_counts.<TYPE>`                                                  |
|`excluded`              |yes     |Fields removed from the surface first-class, with reason and tracking issue                              |
|`diffs`                 |yes     |Pairwise disagreements inside the surface                                                                |
|`insufficient_witnesses`|yes     |True when fewer than two observations were present                                                       |
|`independence`          |when a registry is supplied|The Section 9.3 evaluation                                                            |
|`replay`                |when a committed set is supplied|Per witness: `match`, `missing`, or a drift description                           |
|`rejected`              |when non-empty|Witnesses whose input hash or determinism flag failed                                              |
|`manifest_errors`       |when non-empty|Why the manifest and the observations do not fit together                                          |
|`verdict_hash_sha256`   |yes     |SHA-256 over the canonicalized verdict excluding `timestamp`                                             |
|`timestamp`             |optional|ISO-8601. Omitted by default so the verdict is byte-reproducible.                                        |

The committed passing verdict for the worked example, complete:

```json
{
  "artifact_id": "magnetar-2024-core-interior",
  "diffs": [],
  "excluded": [
    {
      "category": "floors",
      "field": "entity_counts.IFCSHADINGDEVICE",
      "reason": "known_gap",
      "tracking_issue": 31,
      "unsupported_feature": "floor_slab_extrusion_thickness"
    },
    {
      "category": "rooms_spaces",
      "field": "entity_counts.IFCSPACE",
      "reason": "known_gap",
      "tracking_issue": 33,
      "unsupported_feature": "typed_door_window_discrimination_and_host_binding"
    },
    {
      "category": "materials",
      "field": "entity_counts.IFCMATERIAL",
      "reason": "known_gap",
      "tracking_issue": 34,
      "unsupported_feature": "revit_compound_assemblies_and_walltype_widths"
    },
    {
      "category": "property_sets",
      "field": "entity_counts.IFCPROPERTYSET",
      "reason": "known_gap",
      "tracking_issue": 35,
      "unsupported_feature": "typed_door_window_discrimination_and_host_binding"
    }
  ],
  "independence": {
    "commercial_dropped": [],
    "lineages": [
      "ifcopenshell",
      "rvt-rs"
    ],
    "roles": [
      "bridge",
      "source"
    ],
    "satisfied": true,
    "strong_copyleft": []
  },
  "inputs": {
    "ifcopenshell": {
      "input_hash_sha256": "d07c7462aee22640661faed5262cf802ce0fcbc663f312961a39be92bf857050",
      "role": "bridge",
      "witness_version": "0.8.5"
    },
    "rvt-rs": {
      "input_hash_sha256": "c805df445d613b408e37337765572021265e3f5dfdc7d1fa53b22ba1600b8014",
      "role": "source",
      "witness_version": "0.1.2"
    }
  },
  "insufficient_witnesses": false,
  "schema_version": "1.0.0",
  "semantic_surface": [
    "entity_counts.IFCWALL",
    "entity_counts.IFCROOF",
    "entity_counts.IFCDOOR",
    "entity_counts.IFCWINDOW",
    "entity_counts.IFCCOLUMN",
    "entity_counts.IFCBEAM",
    "entity_counts.IFCFLOWTERMINAL",
    "entity_counts.IFCUNITASSIGNMENT"
  ],
  "status": "PASS",
  "verdict_hash_sha256": "3df3ba73bb3f86fd3fab63c4a4db5fb7074fa18036056f01bba45d0c0b43f53a",
  "witnesses_compared": [
    "ifcopenshell",
    "rvt-rs"
  ]
}
```

This is what an honest thin verdict looks like: eight fields inside the surface, of which seven are zero counts, and four categories excluded as tracked decoder gaps. The verdict says so rather than rounding up to "verified".

On disagreement, `status` becomes `DISAGREE` and `diffs` carries one entry per disagreeing pair per field. `tolerance_applied` records the tolerance the comparison actually used, taken from the manifest category (`0` for an exact field):

```json
{
  "artifact_id": "magnetar-2024-core-interior",
  "diffs": [
    {
      "field": "entity_counts.IFCWALL",
      "tolerance_applied": 0,
      "value_a": 0,
      "value_b": 3,
      "witness_a": "ifcopenshell",
      "witness_b": "rvt-rs"
    }
  ],
  "excluded": [],
  "insufficient_witnesses": false,
  "schema_version": "1.0.0",
  "semantic_surface": [
    "entity_counts.IFCWALL"
  ],
  "status": "DISAGREE",
  "verdict_hash_sha256": "0000000000000000000000000000000000000000000000000000000000000000",
  "witnesses_compared": [
    "ifcopenshell",
    "rvt-rs"
  ],
  "inputs": {}
}
```

On replay, the gate re-hashes each fresh payload against the committed observation of the same witness and records the outcome per witness. Any value other than `match` sets `REPLAY_DRIFT`. The two keys that change are shown below; the rest of the verdict is as above:

```json
{
  "replay": {
    "ifcopenshell": "match",
    "rvt-rs": "drift (committed b6d9b67c10c3350b69b58cbd2c6caeca405f63ce182e7291feba3e3ff10f3e00, fresh 0000000000000000000000000000000000000000000000000000000000000000)"
  },
  "status": "REPLAY_DRIFT"
}
```

The full status vocabulary is defined in Section 10.5.

-----

## 7. The Diff Function

The diff function is the load-bearing definition of the protocol. It is the single most important artifact in this specification.

### 7.1 Principles

1. **Agreement is scoped to the claimed semantic surface.** A witness is not penalized for fields it explicitly excludes.
2. **Tolerances are explicit and per-field.** Geometry uses a relative epsilon with an absolute floor; counts use exact equality; strings use normalized comparison.
3. **Excluded fields are first-class.** A field marked `known_gap` or `unsupported` in the manifest must not appear in a disagreement. If it does, the manifest is wrong, not the witness.
4. **Unsupported entities are not disagreements.** If witness A reports `ACAD_PROXY_ENTITY` as unsupported and witness B skips it silently, that is recorded but not a fail, provided both agree on the supported surface.
5. **Determinism is required.** A witness that produces different observations on identical bytes is itself a protocol violation.

### 7.2 Per-field rules

|Field class         |Comparison                                       |Tolerance                                                                              |
|--------------------|-------------------------------------------------|-----------------------------------------------------------------------------------------|
|Entity counts       |Exact integer equality                           |None, unless the manifest states a per-category integer tolerance with a written reason  |
|Layer names         |Case-sensitive string equality after trim        |None                                                                                     |
|Layer color/linetype|Exact                                            |None                                                                                     |
|Bounding box        |Component-wise                                   |Relative `1e-6` with an absolute floor stated by the manifest in the artifact's model units|
|Coordinates         |Component-wise                                   |Same as bounding box                                                                     |
|XDATA fields        |Exact for known app names; normalized for unknown|None                                                                                     |
|Text content        |Unicode NFC normalization, trim                  |None                                                                                     |
|3D solids / meshes  |Deferred to later surface                        |Not applicable in v1                                                                     |

Two components `a` and `b` agree when

```
abs(a - b) <= max(1e-6 * max(abs(a), abs(b)), floor_abs)
```

The relative term is dimensionless and carries no unit. `floor_abs` is the only quantity with a unit: the manifest states it in the artifact's model units (millimetres for a metric Revit export, feet for an imperial one), and it exists so that components near zero, where the relative term collapses, still have a defined agreement band. A manifest that omits `floor_abs` for a geometric surface is a `MANIFEST_ERROR`.

Counts are exact. A manifest may relax a category to an integer tolerance, but only with a written reason recorded in that category's `notes`, and changing a tolerance is a reviewed change like any other — the tolerance is the only place slack is allowed anywhere in the protocol, which is why it is the one place a prose justification is mandatory.

### 7.3 Canonicalization

Before diffing, every observation is passed through a canonicalizer that:

- Sorts object keys lexicographically.
- Renders numbers in a fixed format (no scientific notation for integers; fixed 6 decimal places for floats in geometry).
- Normalizes all strings to Unicode NFC.
- Removes all `null` and empty-array fields that the schema marks as optional-absent.
- Computes a SHA-256 over the canonical bytes.

Two observations are **bit-equivalent** if their canonical hashes match. They are **semantically equivalent** if the diff function returns no diffs within the claimed surface.

The reference implementation's canonicalizer is RFC 8785 restricted to the value space it actually emits: sorted keys, no insignificant whitespace, UTF-8, integers and ASCII strings only. That restriction is byte-identical to full JCS for those payloads, and it is what makes a Rust witness and a Python witness produce the same `observation_hash_sha256`. A witness that emits floats must implement the full RFC 8785 number rules.

-----

## 8. Provenance Model

### 8.1 Provenance record

Every golden artifact carries a provenance record binding:

1. **Source identity:** hash of the original file in S, plus the Revit (or other) build that authored it.
2. **Generation event:** who or what ran the export, when, with what settings, and a content hash of the export settings.
3. **Bridge identity:** hash of the committed DWG (or other bridge file).
4. **Witness set:** the exact witness IDs and versions that produced the committed observations.
5. **Verdict:** the recorded pass/fail at commit time.

### 8.2 Recording

For the first generation of each artifact class, a screen recording (or structured event log) of the export session is retained. This is the human-auditable proof that the bridge file was produced by the claimed source application, not synthesized. Recordings may be redacted for sensitive project data but must retain the export dialog, settings, and resulting file hash.

### 8.3 Immutability

Once an artifact is committed with a passing verdict, it is immutable. Corrections create a new artifact with a new ID, a link to the superseded artifact, and a reason. This preserves the audit trail.

### 8.4 Replay

Any stranger can:

1. Clone the corpus.
2. Run any registered witness against any artifact.
3. Recompute the canonical observation hash.
4. Compare against the committed observation.
5. Re-run the diff function.
6. Confirm the verdict.

No special hardware, no vendor license, no network access after the initial clone.

-----

## 9. Witness Participation Rules

### 9.1 Registration

A witness registers by submitting a PR to the registry that includes:

- Language, license, and copyleft status (`none` / `weak` / `strong`, per Section 5.3).
- Supported formats and versions.
- A coverage declaration: either a figure this registry reproduced through a recorded agreement, or the project's own claim marked as such, or `undeclared`.
- A CI-eligible flag (false if commercial; true but secondary if strong copyleft).
- A reference to a public repository or binary.

A license the registry has not verified against the upstream repository is recorded as `unverified`, not guessed from a README or a package index.

### 9.2 License policy

|License                          |CI-eligible            |Notes                                                  |
|---------------------------------|-----------------------|-------------------------------------------------------|
|Apache-2.0, MIT, BSD, MPL-2.0    |Yes                    |Preferred                                              |
|LGPL-2.1/3, CDDL (weak copyleft) |Yes (dynamic link only)|Must not be statically linked into Apache-2.0 artifacts|
|GPL-2/3, AGPL (strong copyleft)  |Yes as secondary only  |Never linked into primary tree; run as isolated process|
|Commercial (ODA, DATAKIT)        |No                     |May be used for offline research, not CI gate          |
|Unverified or undeclared         |No                     |Cannot participate until the license is verified       |

The dynamic-link row is a ceiling, not a floor: running a weak-copyleft witness as a separate process, as the reference implementation does with IfcOpenShell, is stricter than the row requires and is always permitted.

### 9.3 Independence requirement

For an artifact to receive a `PASS` verdict, the set of witnesses that produced agreeing observations must satisfy all of the following:

- **At least two** `ci_eligible` witnesses, drawn from **distinct implementation lineages** (different primary author, different language, different evidence base). A witness and the implementation it is built on count as one lineage, not two — a Rust FFI wrapper over LibreDWG, a workbench that runs IfcOpenShell underneath, a GDAL driver that wraps dgnlib, and a toolkit whose mesh path is web-ifc are each the same witness as their base.
- **No shared reverse-engineering session.** Witnesses that were trained on, or copied from, the same sample set or the same reverse-engineering notes are correlated and do not satisfy the requirement, even if their codebases differ.
- **At least one witness must be a bridge-format reader** (e.g., ACadSharp, jDwgParser, LibreDWG) that never reads the source format S directly. This is the cross-boundary check that breaks correlated error.
- **At least one witness must be a source-format reader** (e.g., rvt-rs) that never reads the bridge format B. This prevents the bridge reader from being the sole source of truth.
- **Strong-copyleft witnesses** (LibreDWG, jDwgParser) may participate as secondary witnesses but **never** as the sole agreeing pair. They run as isolated processes; their code is never linked into an Apache-2.0 or MIT primary tree.
- **Commercial SDKs** (ODA, DATAKIT) are excluded from the CI gate entirely. They may be used offline to generate reference observations for research, but a commercial witness cannot certify an open-source reader.

If the independence set cannot be satisfied, the verdict is `INSUFFICIENT_INDEPENDENT_WITNESSES`, regardless of whether the observations agree.

Lineage is declared in the registry, not inferred. Where the registry carries a `lineage` field naming another witness, the gate collapses the pair to one lineage before counting.

-----

### 9.4 Coverage declaration

Every registered witness must declare, in the registry, the **semantic surface** it claims to cover, using the controlled vocabulary defined here:

- `entity_counts`
- `layer_topology`
- `linework`
- `bounding_boxes`
- `xdata_fields`
- `text_content`
- `3d_solids` (deferred in v1)
- `meshes` (deferred in v1)
- `bim_parameters` (explicitly excluded for DWG bridge in v1)

The same vocabulary is what an observation's `semantic_surface_covered` array and a manifest's `faithful_surface` / `explicitly_excluded` arrays draw from. Witness identifiers are never members of it.

A witness may only be compared on fields it declares. Declaring a field it cannot actually parse is a registration violation and grounds for removal.

-----

### 9.5 Determinism attestation

Each witness must attest, in its registration, that it produces **bit-identical canonical observations** on identical input bytes across:

- At least two operating systems (e.g., Linux x86_64 and Windows x86_64).
- At least two CPU architectures where applicable (x86_64 and aarch64).
- At least two runs with no intervening state.

The CI gate re-runs every witness on a fixed golden artifact at least once per release cycle. A witness that fails this re-run is marked `non_deterministic` and removed from the agreeing set until the cause is fixed and re-attested.

Pending the umbrella (Section 5.3.1), the per-run `deterministic` flag in the observation is the only attestation the reference implementation carries; a witness that sets it to anything but `true` is rejected outright rather than merely excluded.

-----

### 9.6 Version pinning

Witnesses are pinned by **exact version** (git commit SHA or release tag) in the registry. Floating version ranges are forbidden. When a witness updates, the registry PR must include:

- The new version.
- A diff of its observation output on the existing corpus.
- A justification for any change in declared coverage.
- Re-computation of all affected verdicts.

Silent witness upgrades are a protocol violation.

-----

## 10. CI Gate Specification

### 10.1 Trigger conditions

The gate runs on every pull request or push to `main` that modifies:

- Any file under the corpus.
- Any file in a registered decoder repository (detected via submodule or path filter).
- The registry.
- The diff function implementation.
- The canonicalizer.

### 10.2 Execution model

```
for each artifact A in corpus:
    surface = A.manifest categories with status "known" and a source type
    witnesses = registry.ci_eligible
                .filter(w => w.covers(surface))
                .filter(w => w.claims(A.bridge.format))
    if len(witnesses) < 2:
        verdict = INSUFFICIENT_WITNESSES
        continue
    observations = []
    for w in witnesses:
        o = w.run(A.bridge_file)   # isolated process, pinned version
        o = canonicalize(o)         # RFC 8785
        observations.append(o)
    verdict = diff_function(observations, surface)
    write verdict.json
    if verdict != PASS:
        fail_build()
```

### 10.3 Isolation

Each witness runs in an **isolated container** (Docker or equivalent) with:

- No network access after image build.
- A read-only mount of the corpus.
- A fixed, pinned version of the witness binary or source.
- A resource limit (CPU, memory, wall-clock) to prevent runaway processes.
- A deterministic locale (`C.UTF-8`) and timezone (`UTC`).

Strong-copyleft witnesses run in separate containers from Apache-2.0/MIT witnesses. No shared filesystem, no shared process namespace.

### 10.4 Caching

Witness containers and their dependency layers are cached by content hash. A cache hit is valid only if the witness version, the input file hash, and the container image hash are unchanged. Cache poisoning is mitigated by re-computing the canonical observation hash on every run and comparing against the committed hash; a mismatch invalidates the cache entry and re-runs.

### 10.5 Failure semantics

|Condition                                                        |Verdict                              |Build             |
|-----------------------------------------------------------------|-------------------------------------|------------------|
|≥2 independent witnesses agree on Σ                              |`PASS`                               |Pass              |
|≥2 independent witnesses disagree on Σ                           |`DISAGREE`                           |Fail              |
|<2 witnesses produced an observation                             |`INSUFFICIENT_WITNESSES`             |Fail (fail-closed)|
|Witnesses present but the Section 9.3 independence set unsatisfied|`INSUFFICIENT_INDEPENDENT_WITNESSES`|Fail              |
|Witness read bytes the manifest does not name, or failed its determinism flag|`REJECTED_INPUT`         |Fail              |
|Fresh observation hash differs from the committed one            |`REPLAY_DRIFT`                       |Fail              |
|Witness non-deterministic across runs, platforms, or architectures|`NON_DETERMINISTIC`                 |Fail              |
|Witness crashes or times out                                     |`WITNESS_ERROR`                      |Fail              |
|Manifest claims a surface no witness declares, or a witness is absent from the registry|`MANIFEST_ERROR`|Fail          |

`REJECTED_INPUT` is the input-provenance guard: an observation whose `input_hash_sha256` matches neither the manifest's source hash nor its bridge hash is a witness that read something else, which is a stronger failure than disagreeing. `REPLAY_DRIFT` is the Section 8.4 guard: the fresh run reproduces the diff but not the committed bytes, which means either the witness or the artifact moved.

`NON_DETERMINISTIC` and `WITNESS_ERROR` require, respectively, the cross-platform attestation harness of Section 9.5 and container-level supervision of Section 10.3. Both are umbrella scope (Section 5.3.1); the reference implementation's in-repo gate collapses a failed `deterministic` flag into `REJECTED_INPUT` and lets a crashed witness fail the CI job directly rather than emitting a verdict. A conforming umbrella gate MUST emit them.

The gate is **fail-closed by default**. A missing witness is never treated as agreement.

### 10.6 Artifact publication

On every successful run, the full set of canonical observations, the verdict, and the diff report are published as a GitHub Actions artifact, retained for at least 90 days. This allows human review of disagreements without re-running the gate.

-----

## 11. Faithful Export Surface (v1)

Based on the empirical behavior of Autodesk's Revit-to-DWG export (ACadSharp's ACIS reader for 3DSOLID / REGION / BODY, merged as PR #1139 and released in v3.6.51 on 2026-07-29, and the known lossy flattening of BIM parameters), the v1 faithful surface is restricted to:

**Included (lossless or near-lossless):**

- `entity_counts` for 2D primitives: LINE, LWPOLYLINE, ARC, CIRCLE, ELLIPSE, SPLINE, TEXT, MTEXT.
- `layer_topology`: layer names, colors, linetypes, and the set of entities per layer.
- `bounding_boxes` for 2D entities (component-wise, relative 1e-6 with the manifest's absolute floor in model units).
- `xdata_fields` for the `REVIT` application group, limited to: category code, element ID, and level elevation (where present).
- `text_content` for TEXT and MTEXT entities (Unicode NFC normalized).

**Explicitly excluded in v1:**

- `3d_solids`, `meshes`, `regions`, `bodies` — Revit flattens these; ACadSharp reads the ACIS payload but semantic reconstruction is not yet reliable across witnesses, and no second witness reads it at all.
- `bim_parameters`, `materials`, `element_categories` beyond the XDATA subset — stripped by the export.
- `hatches`, `dimensions`, `leaders`, `tables` — export behavior is version-dependent and not yet characterized.

**v1.1 target (pending a second independent ACIS reader and dwg-rs coverage above 40%):**

- Add `3d_solids` via ACIS payload comparison. ACadSharp reads the payload since v3.6.51; MESH remains unimplemented there, and dwg-rs has no ACIS path, so this edge has one reader and therefore no agreement.
- Add `meshes` once two independent MESH readers exist.

The faithful surface is versioned. Expanding it requires a new corpus class, new witness coverage declarations, and a new diff-function rule set, all via PR.

-----

## 12. Provenance Chain (Detailed)

### 12.1 Record structure

```
P = (S_hash, S_build, G_event, G_settings_hash, B_hash, W_set, V)
```

where:

- `S_hash`: SHA-256 of the original source file in format S.
- `S_build`: exact build string of the source application (e.g., `Revit 2026.1.0.0 (x64)`).
- `G_event`: ISO-8601 timestamp of the generation, plus generator identity (human, agent, or script) and a content hash of the full export settings.
- `G_settings_hash`: SHA-256 of the canonicalized export settings JSON.
- `B_hash`: SHA-256 of the committed bridge file.
- `W_set`: ordered list of `(witness_id, witness_version, observation_canonical_hash)` tuples.
- `V`: the verdict at commit time (`PASS` or `DISAGREE` with full diff).

### 12.2 Hash chaining

Each provenance record includes `prev_hash`, the SHA-256 of the previous record in the corpus. This creates a tamper-evident chain: altering any historical record invalidates all subsequent hashes. The chain root is signed by the maintainer's key (Ed25519) and published in `MANIFEST_INDEX.json`.

### 12.3 Recording requirement

For the **first** artifact of each export-mode class (e.g., first 2D plan view, first 3D wireframe), a structured recording is required:

- Screen recording (MP4, H.264) or structured event log (JSONL of mouse/keyboard/API calls).
- Must capture: the source file open, the export dialog with all settings visible, the export execution, and the resulting file hash.
- May be redacted for project-sensitive geometry, but the export settings panel, file path, and resulting hash must remain visible.
- Stored at `export_recording.<ext>` alongside the manifest; referenced by URI in the provenance record.

Subsequent artifacts of the same class may reference the first recording by ID rather than re-recording, provided the export settings hash is identical.

-----

## 13. Replay Protocol

Any third party can independently verify any artifact:

1. `git clone` the corpus repository.
2. Verify the chain root signature in `MANIFEST_INDEX.json`.
3. For artifact `g-2026-0001`:
- Read `manifest.json`; confirm `B_hash` matches `sha256sum bridge.dwg`.
- For each witness in `W_set`:
  - Check out the pinned witness version.
  - Run `witness --input bridge.dwg --output obs.json` in an isolated container.
  - Canonicalize `obs.json` per RFC 8785.
  - Compare canonical hash to the committed `observation_canonical_hash`.
- Re-run `diff_function` on the fresh observations.
- Confirm the verdict matches `verdict.json`.

No network, no vendor license, no special hardware. The only dependency is a container runtime (Docker or Podman) and the witness images, which are published alongside the corpus.

-----

## 14. Generalization Beyond BIM

The protocol is format-agnostic. To add a new source format S' with bridge format B':

1. Define the faithful export surface for S' → B' (Section 11 pattern).
2. Register at least two independent, non-colluding witnesses for B' (Section 9).
3. Generate the first golden artifact with a recorded export session (Section 12).
4. Extend the diff function with any new field classes (Section 7).
5. Add the new corpus class to the CI gate (Section 10).

Candidate next instances, ordered by witness availability:

|Source (S')   |Bridge (B')|Existing witnesses             |Status                    |
|--------------|-----------|-------------------------------|--------------------------|
|RVT           |IFC        |IfcOpenShell, ifc-lite, web-ifc|Recorded (first instance)|
|DWG           |DXF        |ezdxf, dxf-rs, IxMilia.Dxf     |Ready                     |
|STEP          |(self)     |STEPcode, cadmpeg-codec-step   |Ready                     |
|IGES          |(self)     |pyiges, IGESio                 |Partial                   |
|DGN           |(self)     |dgnlib (C, MIT)                |Needs Rust port           |
|Navisworks NWD|IFC        |None open                      |Greenfield                |
|E57 / LAS     |(self)     |libE57Format, las-rs           |Ready                     |

The RVT→IFC edge is the first recorded instance because IfcOpenShell provides a mature, LGPL, independently-implemented witness that never shares code with rvt-rs. The RVT→DWG edge is the next one, and it is the edge that turns a single dataset into a method, because it crosses a format boundary with three candidate bridge readers rather than one.

-----

## 15. Security Considerations

### 15.1 Supply chain

Witness containers are built from pinned source. A compromised witness upstream is detected by the determinism attestation (Section 9.5) and the cross-witness comparison (a single compromised witness will disagree with the others). The gate fails on disagreement, so a single bad witness cannot produce a false PASS.

### 15.2 Provenance forgery

Altering a golden artifact's bytes changes `B_hash`, which invalidates the provenance chain (Section 12.2) and is caught earlier still by `REJECTED_INPUT` (Section 10.5), because a witness reading the altered bytes reports a hash the manifest does not name. Altering a committed observation changes its canonical hash, which the replay protocol (Section 13) detects as `REPLAY_DRIFT`. The chain root signature prevents retroactive insertion of artifacts.

### 15.3 Denial of service

A malicious or buggy witness that hangs or consumes excessive resources is bounded by the container resource limits (Section 10.3). A witness that consistently times out is marked `WITNESS_ERROR` and excluded from the agreeing set.

### 15.4 Information leakage

Golden artifacts may contain sensitive project data. The protocol supports redaction: the source file hash is retained, but the source file itself may be replaced with a hash-verified excerpt or omitted entirely, provided the bridge file and its hash remain. Recordings may be redacted per Section 12.3.

-----

## 16. Conformance and Versioning

### 16.1 Spec versioning

This specification follows semantic versioning:

- **Major** (1.x → 2.x): breaking changes to the observation schema, diff function, or provenance record that require corpus migration.
- **Minor** (1.0 → 1.1): additive changes (new field classes, new faithful surface entries) that are backward-compatible.
- **Patch** (1.0.0 → 1.0.1): clarifications, typo fixes, non-semantic corrections.

### 16.2 Corpus compatibility

A corpus generated under spec version X is replayable under any version ≥ X. A corpus generated under version X is **not** guaranteed to produce identical verdicts under version Y < X if Y changed the diff function or canonicalizer.

### 16.3 Reference implementation

The reference implementation is `rvt-rs` (Apache-2.0), as an in-repo instance. The umbrella repository — vision, golden corpus, cross-witness CI gate, protocol spec, with decoders linked and never parsing a byte — is planned and earns its existence the day the second edge is recorded and gated.

What the in-repo instance provides today:

|Requirement                          |Where                                                                                |
|-------------------------------------|---------------------------------------------------------------------------------------|
|Canonicalizer (Section 7.3)          |`tools/ci/witness-verdict.py`; the same rules in `src/bin/rvt_ifc.rs` for the Rust witness|
|Diff function (Section 7.2)          |`tools/ci/witness-verdict.py`, exact counts with per-category manifest tolerance         |
|Observation emitters (Section 6.2)   |`rvt-ifc --observation PATH --artifact-id ID` (source); `tools/ci/witness-ifcopenshell.py --observation` (bridge)|
|Verdict and independence (Sections 6.3, 9.3)|`tools/ci/witness-verdict.py --registry research/witness-registry.json`           |
|Replay (Sections 8.4, 13)            |`tools/ci/witness-verdict.py --compare-committed`; pinned by `tests/witness_verdict.rs`  |
|Registry (Section 5.3)               |`research/witness-registry.json`; consistency enforced by `tests/witness_registry.rs`    |
|CI gate (Section 10)                 |the `ifcopenshell-validate` job, observations and verdict published as build artifacts   |
|Machine-checkable schemas (Section 6)|`docs/schemas/witness-observation.schema.json`, `docs/schemas/witness-verdict.schema.json`|

What it does not provide, all of it umbrella scope (Section 5.3.1): containers (Section 10.3), hash chaining and the Ed25519 chain root (Section 12.2), the recording requirement (Section 12.3), the cross-platform determinism attestation (Section 9.5), exact version pinning of external witnesses (Section 9.6), and the `ci_eligible` / coverage-declaration registry fields.

The reference implementation is not normative; any conforming implementation may be used. Conformance is tested by validating observations and verdicts against the published schemas and by running the reference implementation's test suite against the candidate.

-----

## 17. Open Questions and Future Work

1. **Statistical significance of agreement.** With deterministic witnesses, "agreement" is binary, not probabilistic. However, the *number* of independent witnesses required for a claim of "verified" may warrant a formal threshold (e.g., 3 for "strong," 5 for "robust"). This is deferred to v1.1.
2. **Adversarial witness detection.** A witness that deliberately produces plausible-but-wrong observations to force agreement is not defended against by the current model. Mitigation requires diversity of origin and, potentially, a reputation system. Deferred.
3. **Incremental corpus growth.** As new Revit versions ship, new golden artifacts must be generated. The protocol supports this via new artifact IDs, but the cost of maintaining a licensed Revit seat for each new version is non-trivial. A sponsored or donated seat program is the intended mitigation.
4. **Formal methods integration.** The empirical cross-witness model could be augmented with formal verification of individual decoders (e.g., using Kani or Prusti for Rust). This is a research direction, not a v1 requirement.
5. **Standardization.** If the protocol gains adoption, submission to a standards body (ISO, buildingSMART, or IETF) is the natural next step. The current draft is intentionally self-published to allow rapid iteration.
6. **Thin surfaces.** The first recorded artifact agrees on eight fields of which seven are zero counts, because the only paired export available is a 20 KB element-export fixture. A protocol that treats such a verdict as `PASS` is honest but weak. Whether a minimum non-trivial surface should be a gate condition, rather than a matter of reviewer judgement, is open.

-----

## 18. References

- RFC 8785 — JSON Canonicalization Scheme (JCS).
- Open Design Alliance DWG Specification v5.4.1 (basis for jDwgParser and dwg-rs).
- ACadSharp (MIT, C# DWG/DXF reader; `DomCR/ACadSharp`). Current release v3.7.1, 2026-08-18. ACIS payload reading for 3DSOLID / REGION / BODY since v3.6.51, released 2026-07-29 (PR #1139, "Read the ACIS payload of 3DSOLID, REGION and BODY from DXF and DWG"); MESH still unimplemented.
- jDwgParser (GPL-3.0, Java; `ebandal/jDwgParser`, last pushed 2026-05-21). The project claims 74/74 entity types and a 92.2% sample pass rate through R2018; those are its own figures, not reproduced under this protocol.
- LibreDWG 0.14 (GPL-3.0, C). The project claims ~99% read coverage through R2018; its own figure.
- IfcOpenShell (LGPL-3.0, mature IFC engine; the first third-party reading witness in the reference implementation).
- IFClite (`LTplus-AG/ifc-lite`; crates.io crate `ifc-lite-core` 7.1.1, published 2026-08-27). **MPL-2.0**, asserted from the crate metadata and the repository LICENSE, both read on 2026-08-30 (Section 19, note 4). A Rust STEP/IFC parser with its own byte-level scanner; the project verifies its geometry kernel against IfcOpenShell, which is a comparison, not a shared lineage. Third reading witness on the IFC node in the reference implementation.
- STEPcode (BSD-3-Clause on file, NOASSERTION on GitHub; NIST-rooted STEP parser).
- libE57Format (C++, ASTM E57 point cloud).
- rvt-rs (Apache-2.0, clean-room RVT reader; reference implementation for this protocol).
- dwg-rs (Apache-2.0, clean-room DWG reader; sibling project).

Every version, date, and license above was checked against the GitHub API on 2026-08-30, except where explicitly marked unverified.

-----

## 19. Changes from the 2026-08-30 draft

This section records every difference between this document and `docs/octetproof-spec-draft.md`. No change alters the protocol's substance; they are corrections, schema repairs, and reconciliation with the reference implementation.

**Factual corrections (verified against the GitHub API on 2026-08-30):**

1. **jDwgParser license.** The draft's §5.3 carried `license: (TBD — verify)` and `copyleft: false`. `ebandal/jDwgParser` is **GPL-3.0**, Java, last pushed 2026-05-21. It is therefore a secondary, isolated-process witness under §9.2 and §9.3, and never one half of a sole agreeing pair. Corrected in §5.3, §9.3, and §18.
2. **ACadSharp coverage.** The draft's §5.3 gave ACadSharp `coverage_pct: 92`. That figure is jDwgParser's self-reported sample pass rate, not ACadSharp's. ACadSharp's coverage is undeclared and is recorded as such; §5.3 and §9.1 now define `undeclared` and require a reproduced agreement before a figure is registered.
3. **LGPL is weak copyleft.** The draft marked IfcOpenShell `copyleft: false`. LGPL-3.0 is weak copyleft; the §9.2 dynamic-link row is the correct treatment. `copyleft` becomes a three-valued field (`none` / `weak` / `strong`) in §5.3, the §9.2 table names the three tiers explicitly, and §9.3 and §10.3 now say "strong copyleft" where the draft said "GPL/AGPL".
4. **ifc-lite license.** The draft listed MPL-2.0; the in-repo registry carries MIT. Neither was verifiable: the registry entry names no upstream owner, and a GitHub search on 2026-08-30 returns candidates that disagree — `LTplus-AG/ifc-lite` (MPL-2.0, TypeScript-primary with Rust crates), `zahmadsaleem/ifc-lite-headless` (MPL-2.0, Rust), `spookylukey/ifc-lite-python` (no license), among others. The entry was therefore marked `license: unverified`, `copyleft: unknown`, and §9.2 makes an unverified license gate-ineligible until resolved. **Resolved the same day**, before the witness was adopted: the crate the reference implementation actually pins is `ifc-lite-core` 7.1.1 (crates.io, published 2026-08-27), whose `license` field is **MPL-2.0** and whose `repository` is `https://github.com/LTplus-AG/ifc-lite`, which the GitHub API reports as MPL-2.0. `zahmadsaleem/ifc-lite-headless` is a different project and is not what is adopted. §5.3 and §18 now carry `MPL-2.0` / `copyleft: weak`, and the in-repo registry names the exact upstream and the exact pinned version per §9.6.
5. **ACIS support attribution.** The draft's §11 attributed ACIS payload support to "2026-07-29" and §18 to "ACadSharp v3.7.1". Both are right in part: the ACIS reader for 3DSOLID / REGION / BODY landed in **v3.6.51, released 2026-07-29** (PR #1139); **v3.7.1** is the current release, 2026-08-18. Corrected in §11 and §18.
6. **jDwgParser and LibreDWG coverage figures.** The draft's §18 stated "74/74 entity types, 92.2% sample pass rate" and "~99% read coverage" as facts. Both are the projects' own claims, unreproduced under this protocol, and are now labelled as such in §5.3 and §18.
7. **Unverified port example removed.** §9.3's illustration of a port that counts as one lineage cited `@node-projects/acad-ts`, which no GitHub search on 2026-08-30 resolves. It is replaced with the four lineage pairs the in-repo registry actually declares: a Rust FFI wrapper over LibreDWG, a workbench running IfcOpenShell, a GDAL driver wrapping dgnlib, and a toolkit whose mesh path is web-ifc.

**Schema and example repairs:**

8. **§5.3 registry YAML.** The draft's block was not parseable YAML (`formats: versions: [...]` collapsed two keys, `coverage_pct: 100_entity_types` mixed a number and a label, one entry had `formats: versions:` with no value). Rewritten as valid YAML with `versions` and `coverage` as separate keys.
9. **§6.1 manifest.** The draft's `faithful_surface` was a malformed JSON array — missing commas, witness ids (`acadsharp`, `jdwgparser`) mixed into the surface vocabulary, and no closing brace for `bridge`. §6.1 now shows the v1 manifest shape the reference implementation actually consumes, with the normative reading of each field the gate uses, and gives the DWG-bridge block separately and explicitly labelled as a planned extension. §9.4 states that witness identifiers are never members of the surface vocabulary.
10. **§6.2 observation.** The draft had empty values for `semantic_surface_covered` and `unsupported_entities`, and a `layers` array corrupted by pasted bounding-box and XDATA fragments. Replaced with the field table and the two real committed observations for `magnetar-2024-core-interior`, including the keys the draft omitted: `input_role`, `input_file`, and `observation_hash_sha256`.
11. **§6.3 verdict.** The draft had empty values for `witnesses_compared` and `semantic_surface`. Replaced with the field table and the real committed `PASS` verdict, plus a `DISAGREE` and a `REPLAY_DRIFT` example. `inputs`, `excluded`, `independence`, `replay`, and `verdict_hash_sha256` — all emitted by the reference gate and all absent from the draft — are specified.
12. **Machine-checkable schemas.** `docs/schemas/witness-observation.schema.json` and `docs/schemas/witness-verdict.schema.json` (JSON Schema 2020-12) are published alongside this document and validate the committed files under `research/witness/magnetar-2024-core-interior/`.

**Normative clarifications:**

13. **§7.2 tolerance.** The draft's bounding-box row read `abs(a-b) <= 1e-6 * max(abs(a),abs(b),1.0)` mm — a relative tolerance carries no unit, and the `1.0` floor silently assumed a unit system. The rule is now stated as relative `1e-6` with an absolute floor that the manifest states in the artifact's model units; a geometric surface without a stated floor is a `MANIFEST_ERROR`. The count row now says exact unless the manifest states a per-category tolerance with a written reason.
14. **§10.5 and §6.3 status vocabulary.** `REJECTED_INPUT` (a witness read bytes the manifest does not name, or failed its determinism flag) and `REPLAY_DRIFT` (the fresh observation hash differs from the committed one) are added; both are emitted by the reference gate and were missing from the draft. `INSUFFICIENT_INDEPENDENT_WITNESSES` gains its own row. `NON_DETERMINISTIC` and `WITNESS_ERROR` are retained and marked as requiring the attestation harness and container supervision that are umbrella scope.
15. **§5.3.1 registry mapping.** A new normative table maps the spec's `registry.yaml` vocabulary onto `research/witness-registry.json` (`id`, `kind`, `node`, `repo`, `license`, `language`, `status`, `priority`, `lineage`, `checked`, `notes`) and names the four spec requirements that are not implemented in a single decoder repository — `ci_eligible`, the coverage declaration, the determinism attestation, and exact version pinning — as umbrella scope.
16. **§16.3 reference implementation.** Rewritten from a list of what a reference implementation would provide into a table of what the in-repo instance provides today and an explicit list of what it does not.
17. **§14 status column.** The RVT→IFC row read "Ready (LGPL/MPL)"; that edge is recorded and gated, so it now reads "Recorded (first instance)".
18. **§17 open question 6.** Added: the first recorded artifact's surface is eight fields of which seven are zero counts, and whether a minimum non-trivial surface should be a gate condition is open.

-----

*End of specification. This document is the citable artifact of Layer 1. All other layers derive from it.*
