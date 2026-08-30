# Technical Specification: OctetProof — A License-Free Verification Protocol for Undocumented Binary Formats

**Version:** 1.0.0 (draft for review)
**Date:** 2026-08-30
**Status:** Proposed specification — received from the project owner on 2026-08-30 and committed verbatim for review; reviewer notes from the rvt-rs side are appended at the end
**License of this document:** CC-BY-4.0
**Reference implementation:** to be published under Apache-2.0
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

### 5.3 Layer 3 — Witness Registry

A machine-readable index (`registry.yaml`) of every participating reader.

```yaml
witnesses:
  - id: acadsharp
    language: C#
    license: MIT
    formats: versions: [R14, R2000, R2004, R2007, R2010, R2013, R2018]
    coverage_pct: 92
    ci_eligible: true
    copyleft: false
  - id: jdwgparser
    language: Java
    license: (TBD — verify)
    formats: versions: [R2000..R2018]
    coverage_pct: 100_entity_types
    ci_eligible: true
    copyleft: false
  - id: libredwg
    language: C
    license: GPL-3
    formats: versions: coverage_pct: 99
    ci_eligible: true
    copyleft: true   # secondary witness only; never linked into Apache-2.0 tree
  - id: rvt-rs
    language: Rust
    license: Apache-2.0
    formats: versions: [2016..2026]
    coverage_pct: (schema 100%, typed extraction partial)
    ci_eligible: true
    copyleft: false
    role: primary_source_witness
  - id: ifc-openshell
    language: C++/Python
    license: LGPL-3
    formats: [IFC2x3, IFC4, IFC4x3]
    ci_eligible: true
    copyleft: false  # LGPL allows dynamic linking
  - id: ifc-lite
    language: Rust
    license: MPL-2.0
    formats: [IFC2x3, IFC4, IFC4x3, IFC5]
    ci_eligible: true
    copyleft: false
```

The registry is the single source of truth for what the CI gate tests. A witness not in the registry is not tested.

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

### 6.1 Golden Artifact Manifest (`manifest.json`)

```json
{
  "schema_version": "1.0.0",
  "artifact_id": "g-2026-0001",
  "created": "2026-08-30T12:00:00Z",
  "source": {
    "format": "RVT",
    "revit_build": "2026.1.0.0",
    "source_hash_sha256": "...",
    "source_license": "redistributable_sample",
    "generator": "agent-driven_revit_session",
    "recording_uri": "optional://redacted"
  },
  "bridge": {
    "format": "DWG",
    "dwg_version": "AC1032",
    "export_mode": "2D_plan_view",
    "export_settings_hash": "...",
    "file_hash_sha256": "...",
    "faithful_surface": ["layer_topology", "linework", "xdata_room_boundaries" "entity_counts", "layer_names", "bounding_boxes", "xdata_fields" "bim_parameters", "materials", "element_categories", "3d_solids" "acadsharp", "jdwgparser"],
  "verdict": "PASS"
}
```

### 6.2 Observation (`observations/<witness>.json`)

```json
{
  "schema_version": "1.0.0",
  "witness_id": "acadsharp",
  "witness_version": "2.1.0",
  "artifact_id": "g-2026-0001",
  "input_hash_sha256": "...",
  "deterministic": true,
  "semantic_surface_covered": ,
  "observation": {
    "entity_counts": {
      "LINE": 142,
      "LWPOLYLINE": 38,
      "CIRCLE": 0,
      "ARC": 7,
      "INSERT": 3,
      "HATCH": 0,
      "3DFACE": 0,
      "3DSOLID": 0,
      "BODY": 0,
      "MESH": 0
    },
    "layers": [
      {"name": "A-WALL", "color": 1, "linetype": "CONTINUOUS"},
      {"name": "A-DOOR", "color": 3, "linetype": "DASHED"} 0,0,0 12000, 8000, 0 {"app_name": "REVIT", "handle": "1A2B", "fields": {"category": "OST_Walls"}}
    ]
  },
  "unsupported_entities": ,
  "warnings": []
}
```

Observations are **canonicalized** with RFC 8785 (JSON Canonicalization Scheme) before hashing or diffing. Key ordering, number formatting, and Unicode normalization are fixed. This guarantees bit-identical hashes across languages and platforms.

### 6.3 Verdict (`verdict.json`)

```json
{
  "artifact_id": "g-2026-0001",
  "status": "PASS",
  "witnesses_compared": ,
  "semantic_surface": ,
  "diffs": [],
  "insufficient_witnesses": false,
  "timestamp": "2026-08-30T12:05:00Z"
}
```

On disagreement:

```json
{
  "status": "DISAGREE",
  "diffs": [
    {
      "field": "entity_counts.LWPOLYLINE",
      "witness_a": "acadsharp",
      "value_a": 38,
      "witness_b": "jdwgparser",
      "value_b": 41,
      "tolerance_applied": false
    }
  ]
}
```

-----

## 7. The Diff Function

The diff function is the load-bearing definition of the protocol. It is the single most important artifact in this specification.

### 7.1 Principles

1. **Agreement is scoped to the claimed semantic surface.** A witness is not penalized for fields it explicitly excludes.
2. **Tolerances are explicit and per-field.** Geometry uses absolute or relative epsilon; counts use exact equality; strings use normalized comparison.
3. **Excluded fields are first-class.** A field marked `explicitly_excluded` in the manifest must not appear in a disagreement. If it does, the manifest is wrong, not the witness.
4. **Unsupported entities are not disagreements.** If witness A reports `ACAD_PROXY_ENTITY` as unsupported and witness B skips it silently, that is recorded but not a fail, provided both agree on the supported surface.
5. **Determinism is required.** A witness that produces different observations on identical bytes is itself a protocol violation.

### 7.2 Per-field rules

|Field class         |Comparison                                       |Tolerance                                     |
|--------------------|-------------------------------------------------|----------------------------------------------|
|Entity counts       |Exact integer equality                           |None                                          |
|Layer names         |Case-sensitive string equality after trim        |None                                          |
|Layer color/linetype|Exact                                            |None                                          |
|Bounding box        |Component-wise                                   |`abs(a-b) <= 1e-6 * max(abs(a),abs(b),1.0)` mm|
|Coordinates         |Component-wise                                   |Same as bounding box                          |
|XDATA fields        |Exact for known app names; normalized for unknown|None                                          |
|Text content        |Unicode NFC normalization, trim                  |None                                          |
|3D solids / meshes  |Deferred to later surface                        |N/A in v1                                     |

### 7.3 Canonicalization

Before diffing, every observation is passed through a canonicalizer that:

- Sorts object keys lexicographically.
- Renders numbers in a fixed format (no scientific notation for integers; fixed 6 decimal places for floats in geometry).
- Normalizes all strings to Unicode NFC.
- Removes all `null` and empty-array fields that the schema marks as optional-absent.
- Computes a SHA-256 over the canonical bytes.

Two observations are **bit-equivalent** if their canonical hashes match. They are **semantically equivalent** if the diff function returns no diffs within the claimed surface.

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

- Language, license, and copyleft status.
- Supported formats and versions.
- A self-reported coverage percentage with methodology.
- A CI-eligible flag (false if copyleft or commercial).
- A reference to a public repository or binary.

### 9.2 License policy

|License                      |CI-eligible            |Notes                                                  |
|-----------------------------|-----------------------|-------------------------------------------------------|
|Apache-2.0, MIT, BSD, MPL-2.0|Yes                    |Preferred                                              |
|LGPL-3                       |Yes (dynamic link only)|Must not be statically linked into Apache-2.0 artifacts|
|GPL-2/3, AGPL                |Yes as secondary only  |Never linked into primary tree; run as isolated process|
|Commercial (ODA, DATAKIT)    |No                     |May be used for offline research, not CI gate          |

### 9.3 Independence requirement

For an artifact to receive a `PASS` verdict, the set of witnesses that produced agreeing observations must satisfy all of the following:

- **At least two** `ci_eligible` witnesses, drawn from **distinct implementation lineages** (different primary author, different language, different evidence base). A witness and its direct port (e.g., ACadSharp and `@node-projects/acad-ts`) count as one lineage, not two.
- **No shared reverse-engineering session.** Witnesses that were trained on, or copied from, the same sample set or the same reverse-engineering notes are correlated and do not satisfy the requirement, even if their codebases differ.
- **At least one witness must be a bridge-format reader** (e.g., ACadSharp, jDwgParser, LibreDWG) that never reads the source format S directly. This is the cross-boundary check that breaks correlated error.
- **At least one witness must be a source-format reader** (e.g., rvt-rs) that never reads the bridge format B. This prevents the bridge reader from being the sole source of truth.
- **GPL/AGPL witnesses** (e.g., LibreDWG) may participate as secondary witnesses but **never** as the sole agreeing pair. They run as isolated processes; their output is compared but their code is never linked into an Apache-2.0 or MIT primary tree.
- **Commercial SDKs** (ODA, DATAKIT) are excluded from the CI gate entirely. They may be used offline to generate reference observations for research, but a commercial witness cannot certify an open-source reader.

If the independence set cannot be satisfied, the verdict is `INSUFFICIENT_INDEPENDENT_WITNESSES`, regardless of whether the observations agree.

-----

### 9.4 Coverage declaration

Every registered witness must declare, in `registry.yaml`, the **semantic surface** it claims to cover, using the controlled vocabulary defined in Section 3:

- `entity_counts`
- `layer_topology`
- `linework`
- `bounding_boxes`
- `xdata_fields`
- `text_content`
- `3d_solids` (deferred in v1)
- `meshes` (deferred in v1)
- `bim_parameters` (explicitly excluded for DWG bridge in v1)

A witness may only be compared on fields it declares. Declaring a field it cannot actually parse is a registration violation and grounds for removal.

-----

### 9.5 Determinism attestation

Each witness must attest, in its registration, that it produces **bit-identical canonical observations** on identical input bytes across:

- At least two operating systems (e.g., Linux x86_64 and Windows x86_64).
- At least two CPU architectures where applicable (x86_64 and aarch64).
- At least two runs with no intervening state.

The CI gate re-runs every witness on a fixed golden artifact at least once per release cycle. A witness that fails this re-run is marked `non_deterministic` and removed from the agreeing set until the cause is fixed and re-attested.

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

- Any file under `corpus/`.
- Any file in a registered decoder repository (detected via submodule or path filter).
- `registry.yaml`.
- The diff function implementation.
- The canonicalizer.

### 10.2 Execution model

```
for each artifact A in corpus:
    surface = A.manifest.faithful_surface
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
    if verdict == DISAGREE:
        fail_build()
```

### 10.3 Isolation

Each witness runs in an **isolated container** (Docker or equivalent) with:

- No network access after image build.
- A read-only mount of the corpus.
- A fixed, pinned version of the witness binary or source.
- A resource limit (CPU, memory, wall-clock) to prevent runaway processes.
- A deterministic locale (`C.UTF-8`) and timezone (`UTC`).

GPL witnesses run in separate containers from Apache-2.0/MIT witnesses. No shared filesystem, no shared process namespace.

### 10.4 Caching

Witness containers and their dependency layers are cached by content hash. A cache hit is valid only if the witness version, the input file hash, and the container image hash are unchanged. Cache poisoning is mitigated by re-computing the canonical observation hash on every run and comparing against the committed hash; a mismatch invalidates the cache entry and re-runs.

### 10.5 Failure semantics

|Condition                                   |Verdict                 |Build             |
|--------------------------------------------|------------------------|------------------|
|≥2 independent witnesses agree on Σ         |`PASS`                  |Pass              |
|≥2 independent witnesses disagree on Σ      |`DISAGREE`              |Fail              |
|<2 ci_eligible witnesses available          |`INSUFFICIENT_WITNESSES`|Fail (fail-closed)|
|Witness non-deterministic                   |`NON_DETERMINISTIC`     |Fail              |
|Witness crashes or times out                |`WITNESS_ERROR`         |Fail              |
|Manifest claims surface witness cannot cover|`MANIFEST_ERROR`        |Fail              |

The gate is **fail-closed by default**. A missing witness is never treated as agreement.

### 10.6 Artifact publication

On every successful run, the full set of canonical observations, the verdict, and the diff report are published as a GitHub Actions artifact, retained for at least 90 days. This allows human review of disagreements without re-running the gate.

-----

## 11. Faithful Export Surface (v1)

Based on the empirical behavior of Autodesk's Revit-to-DWG export (confirmed via ACadSharp's ACIS payload support added 2026-07-29 and the known lossy flattening of BIM parameters), the v1 faithful surface is restricted to:

**Included (lossless or near-lossless):**

- `entity_counts` for 2D primitives: LINE, LWPOLYLINE, ARC, CIRCLE, ELLIPSE, SPLINE, TEXT, MTEXT.
- `layer_topology`: layer names, colors, linetypes, and the set of entities per layer.
- `bounding_boxes` for 2D entities (component-wise, 1e-6 relative tolerance in mm).
- `xdata_fields` for the `REVIT` application group, limited to: category code, element ID, and level elevation (where present).
- `text_content` for TEXT and MTEXT entities (Unicode NFC normalized).

**Explicitly excluded in v1:**

- `3d_solids`, `meshes`, `regions`, `bodies` — Revit flattens these; ACadSharp reads the ACIS payload but semantic reconstruction is not yet reliable across witnesses.
- `bim_parameters`, `materials`, `element_categories` beyond the XDATA subset — stripped by the export.
- `hatches`, `dimensions`, `leaders`, `tables` — export behavior is version-dependent and not yet characterized.

**v1.1 target (pending ACadSharp MESH + 3DSOLID stabilization and dwg-rs coverage > 40%):**

- Add `3d_solids` via ACIS payload comparison.
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

|Source (S')   |Bridge (B')|Existing witnesses             |Status          |
|--------------|-----------|-------------------------------|----------------|
|RVT           |IFC        |IfcOpenShell, ifc-lite, web-ifc|Ready (LGPL/MPL)|
|DWG           |DXF        |ezdxf, dxf-rs, IxMilia.Dxf     |Ready           |
|STEP          |(self)     |STEPcode, cadmpeg-codec-step   |Ready           |
|IGES          |(self)     |pyiges, IGESio                 |Partial         |
|DGN           |(self)     |dgnlib (C, MIT)                |Needs Rust port |
|Navisworks NWD|IFC        |None open                      |Greenfield      |
|E57 / LAS     |(self)     |libE57Format, las-rs           |Ready           |

The RVT→IFC edge is the highest-value next addition because IfcOpenShell provides a mature, LGPL, independently-implemented witness that never shares code with rvt-rs.

-----

## 15. Security Considerations

### 15.1 Supply chain

Witness containers are built from pinned source. A compromised witness upstream is detected by the determinism attestation (Section 9.5) and the cross-witness comparison (a single compromised witness will disagree with the others). The gate fails on disagreement, so a single bad witness cannot produce a false PASS.

### 15.2 Provenance forgery

Altering a golden artifact's bytes changes `B_hash`, which invalidates the provenance chain (Section 12.2). Altering a committed observation changes its canonical hash, which the replay protocol (Section 13) detects. The chain root signature prevents retroactive insertion of artifacts.

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

The reference implementation (Apache-2.0) provides:

- A canonicalizer conforming to RFC 8785 with the additional normalization rules in Section 7.3.
- A diff function implementing Section 7.2.
- A CI gate workflow template (Section 10).
- A provenance chain tool (Section 12).
- A replay CLI (Section 13).

The reference implementation is not normative; any conforming implementation may be used. Conformance is tested by running the reference implementation's test suite against the candidate.

-----

## 17. Open Questions and Future Work

1. **Statistical significance of agreement.** With deterministic witnesses, "agreement" is binary, not probabilistic. However, the *number* of independent witnesses required for a claim of "verified" may warrant a formal threshold (e.g., 3 for "strong," 5 for "robust"). This is deferred to v1.1.
2. **Adversarial witness detection.** A witness that deliberately produces plausible-but-wrong observations to force agreement is not defended against by the current model. Mitigation requires diversity of origin and, potentially, a reputation system. Deferred.
3. **Incremental corpus growth.** As new Revit versions ship, new golden artifacts must be generated. The protocol supports this via new artifact IDs, but the cost of maintaining a licensed Revit seat for each new version is non-trivial. A sponsored or donated seat program is the intended mitigation.
4. **Formal methods integration.** The empirical cross-witness model could be augmented with formal verification of individual decoders (e.g., using Kani or Prusti for Rust). This is a research direction, not a v1 requirement.
5. **Standardization.** If the protocol gains adoption, submission to a standards body (ISO, buildingSMART, or IETF) is the natural next step. The current draft is intentionally self-published to allow rapid iteration.

-----

## 18. References

- RFC 8785 — JSON Canonicalization Scheme (JCS).
- Open Design Alliance DWG Specification v5.4.1 (basis for jDwgParser and dwg-rs).
- ACadSharp v3.7.1 (MIT, C# DWG/DXF reader; ACIS payload support added 2026-07-29).
- jDwgParser (Korean Java DWG reader; 74/74 entity types, 92.2% sample pass rate through R2018).
- LibreDWG 0.14 (GPL-3, ~99% read coverage through R2018).
- IfcOpenShell (LGPL-3, mature IFC engine).
- ifc-lite (Rust, MPL-2.0, verified against IfcOpenShell).
- STEPcode (BSD, NIST-rooted STEP parser).
- libE57Format (C++, ASTM E57 point cloud).
- rvt-rs (Apache-2.0, clean-room RVT reader; reference implementation for this protocol).
- dwg-rs (Apache-2.0, clean-room DWG reader; sibling project).

-----

*End of specification. This document is the citable artifact of Layer 1. All other layers derive from it.*

-----

## Appendix A — rvt-rs reviewer notes (2026-08-30)

Recorded by the maintaining session on receipt. None of these change the
protocol's substance; they are the corrections needed before the draft is
published as 1.0.0.

**Factual corrections (verified against the GitHub API on 2026-08-30):**

1. §5.3 lists jDwgParser as `license: (TBD — verify)`, `copyleft: false`.
   `ebandal/jDwgParser` is **GPL-3.0** (Java, last pushed 2026-05-21). Under
   §9.2/§9.3 it is a secondary, isolated-process witness — never one half of a
   sole agreeing pair with another GPL witness.
2. §5.3 gives ACadSharp `coverage_pct: 92` — that figure is jDwgParser's
   self-reported sample pass rate, not ACadSharp's. ACadSharp's coverage is
   undeclared; per the registry's candidate-claims rule it stays that way
   until an agreement reproduces a number.
3. §5.3 marks IfcOpenShell `copyleft: false`. LGPL-3.0 is weak copyleft; the
   §9.2 row ("dynamic link only") is the correct treatment. rvt-rs runs it as
   a separate Python process, which is stricter than the row requires.
4. §5.3 lists ifc-lite as MPL-2.0; the registry carries it as MIT from the
   earlier survey. Neither was verified this session — `checked` is absent in
   the registry entry on purpose.
5. §11 attributes the ACIS payload support to "2026-07-29" and §18 to
   "ACadSharp v3.7.1". Both are right in part: the ACIS reader for
   3DSOLID/REGION/BODY landed in **v3.6.51 (2026-07-29, PR #1139)**; v3.7.1
   is the current release (2026-08-18).
6. §18's jDwgParser figures ("74/74 entity types, 92.2%") are the project's
   own claims; the registry records them as such.

**Editorial defects in the received draft (reproduced verbatim above):**

7. §6.1 `faithful_surface` is a malformed JSON array (missing commas, mixes
   witness ids `acadsharp`/`jdwgparser` into the surface vocabulary, and the
   closing brace for `bridge` is missing).
8. §6.2 `semantic_surface_covered` and `unsupported_entities` have empty
   values; the `layers` array is corrupted by what look like pasted
   bounding-box/XDATA fragments.
9. §6.3 `witnesses_compared` and `semantic_surface` have empty values.
10. §7.2 uses `1e-6` relative tolerance "mm" — a relative tolerance has no
    unit; either drop "mm" or make the floor absolute.
11. §9.3's example port `@node-projects/acad-ts` was not verified.

**Where rvt-rs already implements this (pre-umbrella, inside the decoder repo):**

- Observation (§6.2): `rvt-ifc --observation PATH --artifact-id ID` (source
  witness) and `tools/ci/witness-ifcopenshell.py --observation PATH` (bridge
  witness). Both carry `input_hash_sha256`, `input_role`
  (`source`/`bridge`), `deterministic`, `semantic_surface_covered`, the
  `observation` payload, and `observation_hash_sha256` over the canonical
  payload (sorted keys, no whitespace, UTF-8 — JCS-equivalent for the integer
  and ASCII-string payloads used).
- Verdict and diff function (§6.3, §7, §9.3, §10.5):
  `tools/ci/witness-verdict.py` — statuses `PASS`, `DISAGREE`,
  `INSUFFICIENT_WITNESSES`, `INSUFFICIENT_INDEPENDENT_WITNESSES`,
  `REJECTED_INPUT`, `MANIFEST_ERROR`, `REPLAY_DRIFT`. Excluded fields are
  first-class: manifest categories with status `known_gap`/`unsupported`
  are listed with their tracking issue and never diffed.
- Replay (§8.4, §13): `--compare-committed` re-hashes fresh observations
  against the committed ones under `research/witness/<artifact>/`.
- Registry (§5.3, §9.3): `research/witness-registry.json` with `lineage`
  for derived witnesses; `checked` dates where verified.
- Not implemented here: containers (§10.3), hash chaining and Ed25519 root
  signature (§12.2), the recording requirement (§12.3), determinism
  attestation across OS/arch (§9.5), exact version pinning of external
  witnesses (§9.6 — CI pins IfcOpenShell to a range). These belong to the
  umbrella once the second edge exists.

**Honesty note on the first artifact:** the RVT → IFC edge recorded here
(`magnetar-2024-core-interior`) is thin — the Revit-authored reference IFC
is a 20 KB element-export fixture, so the agreeing surface is eight
categories of which seven are zero counts, and four categories are excluded
as known decoder gaps (floors, rooms/spaces, materials, property sets —
issues tracked in the manifest). The verdict says so. That is the point.
