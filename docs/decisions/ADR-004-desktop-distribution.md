# ADR-004 — Defer native desktop wrappers; ship the browser viewer first

- **Status**: Accepted (2026-08-29)
- **Tickets**: #47 (M6-06)
- **Author**: DrunkOnJava / Cursor agent

## Context

Non-technical users often ask for a “downloadable app” rather than a
URL. The viewer today is a static Vite + WASM site (`viewer/`) with a
hard privacy invariant: compiled WASM must import no network
primitives (VW1-21 / M8-05). Packaging that surface as Tauri or
Electron would change install complexity, update channels, code
signing, and the trust story without improving decode quality.

This ADR records whether a desktop wrapper is worth maintaining now.

## Options compared

| Criterion | Browser (status quo) | Tauri | Electron |
|---|---|---|---|
| Install | Open URL or self-host static files | OS installer / unsigned binary friction | Large installer (~100+ MB shell) |
| Footprint | WASM + JS only | Rust host + WebView | Chromium runtime |
| Privacy | Local file → worker; no WASM network imports | Same WASM; plus native FS APIs tempting to wire | Same; Electron often pulls auto-update/telemetry patterns |
| Auto-update | Hosted deploy (`deploy-viewer.yml`) | Need signing + update server | Need signing + update server |
| Code signing | N/A for static hosting | Apple/Microsoft certs, notarization | Same burden |
| Decode path | Shared `rvt` WASM | Same WASM or duplicate native CLI | Same |
| Maintenance | One web pipeline | Second release matrix (macOS/Windows/Linux) | Second matrix + Chromium CVE chase |

## Decision

**Do not start a Tauri or Electron wrapper until maintenance cost is
explicitly accepted and decode confidence for the supported profile is
higher than scaffold-only for redistributable demos.**

Ship and harden the browser viewer (M6-01…M6-05) and optional
self-hosted static deploy. Desktop packaging remains a parked research
item, not an active milestone.

Revisit only when all of the following are true:

1. Hosted viewer UX is accepted for the supported MVP profile (honest
   confidence labels, demo gallery, keyboard/a11y basics).
2. There is a named maintainer willing to own signing, notarization,
   and OS release breakage.
3. A concrete user cohort cannot use the browser path (for example,
   mandatory offline air-gapped install with no local static host).

If revisited, prefer **Tauri** over Electron for footprint and
alignment with the existing Rust toolchain, still keeping the same
WASM worker and no-network import audit.

## Consequences

- No `src-tauri/` / Electron scaffold lands under this ADR.
- Issue #47 is closed as completed research (decision recorded).
- README / user-guide may point at the hosted or self-hosted viewer;
  they must not advertise a native desktop app.
- Future desktop work needs a new ADR that accepts signing and update
  cost before code lands.

## Related

- `docs/viewer-privacy-posture.md` — no-network WASM invariant
- `docs/viewer-build-pipeline.md` — static deploy path
- `docs/decisions/ADR-003-revit-openability-validation.md` — optional
  proprietary tiers stay out of public CI
