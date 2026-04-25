# ADR-003 — Treat Revit-openability as an optional validation tier

- **Status**: Accepted (2026-04-25)
- **Tickets**: #59
- **Author**: Griffin Long

## Context

`rvt-rs` can validate modified files structurally: CFB opens, streams
round-trip, hashes match expectations, schema streams parse, and
GUID/history invariants hold. Those checks are necessary for any write
path, but they do not prove Autodesk Revit will open the modified file
without repair prompts or hidden model corruption.

The obvious stronger test is "open the file in Revit." That is not a
normal open-source CI primitive:

- Revit is proprietary and requires a licensed Windows installation.
- License terms may restrict unattended automation.
- Most contributors do not have Revit installed.
- Public GitHub-hosted runners cannot legally or practically include
  Revit.
- A Revit-opened file can still need human observation: repair dialogs,
  upgrade prompts, worksharing warnings, and model audit results are
  user-interface state, not just process exit codes.

The project needs a validation policy that values Revit-openability
without blocking every contributor on proprietary infrastructure.

## Decision

Revit-in-the-loop validation is in scope as an optional high-confidence
validation tier. It is not required for baseline contribution, public
CI, or stream-level patch work.

The project will use three validation levels:

| Level | Runs in public CI | Requirement | Claim allowed |
|---|---:|---|---|
| Structural | Yes | CFB opens, patched streams verify, unchanged streams match, schema/metadata parse where applicable. | The file is structurally consistent with the known wire format. |
| Semantic | Yes, when decoders exist | Edited elements decode after write, intended values match, references resolve, GUID/history invariants pass. | The edit round-trips through `rvt-rs`'s supported semantic model. |
| Revit-openability | No, optional external tier | A licensed Revit instance opens the file without repair/corruption prompts under a documented checklist. | The file passed an external Revit smoke check for the tested Revit version. |

Production-safe semantic write claims require Revit-openability evidence
for the supported profile. Experimental prototypes may stop at
structural or semantic validation, but docs must say that clearly.

## Community Validation Path

Community validation is allowed when a contributor has legal access to
Revit and permission to open the test file.

Minimum checklist:

1. Record Revit version and build.
2. Open the source file once and note whether it already prompts for
   upgrade, repair, audit, missing links, or worksharing issues.
3. Open the modified file in the same Revit version.
4. Record whether any additional prompt appears.
5. If the file opens, save a copy to a temporary location.
6. Run `rvt-inspect` on the saved copy and attach the redacted JSON
   diagnostics to the issue or PR.
7. Do not publish proprietary model files unless the owner has approved
   redistribution.

Validation reports should include screenshots only when they do not
disclose private model content.

## Self-Hosted Runner Path

A self-hosted Windows runner with Revit is worth exploring only if all
of these are true:

- the license explicitly permits automated use in CI;
- the runner is private and access-controlled;
- test files are redistributable or owned by the project/operator;
- jobs run on isolated temporary workspaces;
- no proprietary model output is uploaded as an artifact by default;
- failures redact paths and user/project identifiers.

Until those conditions are verified, the project must not require or
advertise Revit-in-the-loop CI.

## Structural Substitute

When Revit is unavailable, the substitute gate is:

- CFB open on source and output;
- stream inventory equivalence except intentionally changed streams;
- patched stream verification with framing-specific decompression;
- unchanged stream byte equality;
- `BasicFileInfo` GUID preservation unless explicitly edited;
- document-history preservation unless explicitly edited;
- schema parse on `Formats/Latest`;
- element decode after write for any edited semantic target;
- deterministic diagnostic report from `rvt-inspect`.

This substitute is a lower-confidence gate. It can prevent many classes
of corruption, but it cannot prove Revit acceptance.

## Consequences

### Positive

- Contributors can keep working with fully open tooling.
- The project can accept high-value community Revit validation evidence
  without making it a hard dependency.
- User-facing claims stay precise: structural validity, semantic
  round-trip, and Revit-openability are distinct.

### Negative / mitigated

- Some production-readiness claims will remain blocked until volunteers
  or compliant infrastructure can run Revit. Mitigated by keeping
  semantic writes experimental until that evidence exists.
- Manual validation reports can be inconsistent. Mitigated by using the
  checklist above and attaching redacted `rvt-inspect` diagnostics.
- A self-hosted Revit runner may never be legally or operationally
  viable. Mitigated by treating it as optional, not as a roadmap
  dependency for read-only features.

## Alternatives Considered

1. **Require Revit validation for every write-path PR.** Rejected.
   This would exclude most contributors and cannot run on public CI.
2. **Never use Revit validation.** Rejected. It is the only practical
   external oracle for production openability.
3. **Use structural checks as proof of Revit-openability.** Rejected.
   Structural checks are necessary but not sufficient, especially for
   semantic edits.

## Verification

This ADR is documentation-only. Enforcement remains in the existing open
test gates:

- `Writer patch corpus` for stream-level writer invariants;
- semantic-write prototypes must follow ADR-002 and remain behind
  `experimental-semantic-write`;
- any future Revit-openability report must follow the checklist in this
  ADR before being cited in release notes or support claims.
