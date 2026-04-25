# ADR-002 — Gate semantic Revit writes behind evidence and an experimental API

- **Status**: Accepted (2026-04-25)
- **Tickets**: #58
- **Author**: Griffin Long

## Context

`rvt-rs` can copy a Revit CFB container byte-for-byte and can replace
complete named OLE streams through `writer::write_with_patches`.
That is a stream-level capability. It is not a semantic Revit editor.

Field-level semantic writing is materially riskier. A change such as
"set wall unconnected height to 3000 mm" crosses several boundaries:

- locating the correct element instance in `Global/Latest` or a
  partition stream;
- proving the element decoder is high-confidence for that Revit
  version and class tag;
- encoding the field with the exact schema-specific wire type;
- preserving every unrelated field, reference, id, GUID, and history
  entry;
- proving the modified file is structurally valid and ideally opens in
  Revit.

The reader is not ready for that promise on arbitrary project files.
Shipping a user-facing semantic edit path too early would create files
that look successfully written but may be corrupt, partially mutated, or
silently inconsistent.

## Decision

Do not ship a stable user-facing semantic write API until the reader has
corpus-backed typed element recovery and field encoders for the supported
profile.

Any prototype must be explicitly experimental:

- Rust API behind a Cargo feature named `experimental-semantic-write`;
- CLI commands hidden or marked experimental, requiring an explicit flag
  such as `--experimental-semantic-write`;
- Python bindings omitted until the Rust API has stabilized, or exposed
  only behind the same experimental build feature;
- docs and diagnostics must say that experimental outputs are not
  production-safe unless validated externally.

No prototype ships as part of this ADR. The only accepted public write
surface remains byte-preserving copy and stream-level patching.

## Proposed API Shape

The eventual API should be transaction-oriented rather than a set of
ad-hoc setters:

```rust
let plan = EditPlan::new()
    .require_revit_version(2024)
    .require_stream_hash("Global/Latest", expected_hash)
    .set_field(
        ElementSelector::by_unique_id(element_id),
        FieldPath::new("Wall", "Unconnected Height"),
        FieldValue::LengthMillimetres(3000.0),
    );

let report = SemanticWriter::open(input)?
    .apply(plan)?
    .validate()?
    .write_to(output)?;
```

Key properties:

- edits are collected into an `EditPlan`;
- every edit carries a selector, schema/class expectation, field path,
  typed value, and precondition;
- validation runs before any output file is written;
- writes are output-path based by default, not in-place;
- the write report lists changed streams, changed element ids,
  validation tier, warnings, and residual risks.

## Supported Field Types

Initial support should be allowlist-only:

- booleans;
- signed/unsigned integers whose width is known from `FieldType`;
- finite `f32`/`f64` values with unit metadata where applicable;
- UTF-16 strings with length-prefix validation;
- GUIDs where the field is known to be a GUID, not an opaque byte blob;
- `ElementId` references only when the target id exists and the
  relationship is explicitly supported.

Do not initially support:

- arbitrary vectors/containers without per-class encoder tests;
- raw pointer/reference fields;
- geometry payload edits;
- partition-stream structural rewrites that require unknown indexes;
- edits to history, central-model identity, worksharing, or upgrade
  provenance streams.

Unsupported values should fail before write with a typed diagnostic,
not fall back to raw bytes.

## Transaction Model

The transaction model must be pessimistic:

1. Open the source with bounded reader limits.
2. Snapshot stream hashes and document identity.
3. Decode the target elements with high-confidence decoders only.
4. Apply edits to an in-memory decoded representation.
5. Re-encode only streams whose full encoder coverage is proven.
6. Re-read the output and compare unchanged streams byte-for-byte.
7. Verify GUID/history preservation unless the edit explicitly targets
   those fields.

Partial transaction success is not allowed. If any edit fails validation,
the writer returns an error and produces no output.

## Validation

Validation has three tiers:

| Tier | Requirement | Meaning |
|---|---|---|
| Structural | CFB opens, patched streams re-read, unchanged streams match, schema parses. | Minimum requirement for any experimental output. |
| Semantic | Edited elements decode after write, field values match, referenced ids resolve, GUID/history invariants pass. | Required before exposing a prototype to users. |
| Revit-openability | A licensed Revit instance opens the file and reports no repair/corruption dialog. | Required before claiming production-safe semantic writing. |

Until Revit-openability validation exists, semantic write docs must use
phrasing such as "experimental" and "not production-safe".

## Backup And Atomic Write

The API should never overwrite the only copy of a model by default.

- `write_to(output)` writes a new file.
- `overwrite_with_backup(input)` is allowed only with an explicit backup
  path or generated timestamped backup path.
- writes use a sibling temp file and atomic rename when possible;
- every write emits a report containing source hash, output hash, backup
  path, changed streams, and validation tier.

## Revit-Openability Strategy

Revit-in-the-loop validation is valuable but cannot be assumed for every
contributor. The project should treat it as an optional higher tier:

- local/community volunteers can run a documented validation checklist;
- a self-hosted Windows runner with licensed Revit can be explored only
  if license terms permit automation;
- until then, structural and semantic invariants are necessary but not
  sufficient for production claims.

## Consequences

### Positive

- Prevents a premature API from corrupting user files.
- Gives contributors a concrete target for future encoder work.
- Keeps current `rvt-write` honest: it patches streams, not Revit
  concepts.
- Aligns CLI, Rust, Python, and docs around the same safety gate.

### Negative / mitigated

- Slower path to a user-visible editor. Mitigated by focusing the next
  milestones on decoder coverage and validation.
- Experimental feature gates add maintenance overhead. Mitigated by not
  creating the feature until prototype code exists.
- Revit-openability validation may require non-open infrastructure.
  Mitigated by documenting structural and semantic validation as
  lower-confidence tiers.

## Alternatives Considered

1. **Expose raw field setters now.** Rejected. The reader cannot yet
   prove enough project-file coverage to make writes safe.
2. **Accept stream patches as semantic edits.** Rejected. Complete stream
   replacement is useful for controlled tooling, but it has no semantic
   awareness.
3. **Build a private prototype without feature gates.** Rejected. Hidden
   prototypes tend to leak into examples and bindings. The feature gate
   makes the risk visible at compile time.

## Verification

This ADR is documentation-only. The current enforced write-path evidence
remains:

- `cargo test --test cfb_roundtrip_delta` for family/project stream patch
  coverage;
- `cargo test writer::tests:: --lib` for patch verification,
  corrupt-gzip reporting, GUID, and history helpers;
- CI `Writer patch corpus` job for real family and project fixtures.
