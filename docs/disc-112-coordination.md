# Discussion #112 coordination notes

Public coordination record for
[Discussion #112 — Set of findings](https://github.com/DrunkOnJava/rvt-rs/discussions/112)
([@STE1200](https://github.com/STE1200)).

Automation could not post these notes as discussion comments: the GitHub App /
`gh` integration lacks `addDiscussionComment` / `updateSubscription` permission
(`Resource not accessible by integration`). This file is the durable fallback.
Mirror into the discussion thread when a maintainer account can post.

## Finding issues (index)

| Issue | Title |
|------|--------|
| [#151](https://github.com/DrunkOnJava/rvt-rs/issues/151) | RE: Reproduce checksum-paged stream decompression and silent corruption |
| [#152](https://github.com/DrunkOnJava/rvt-rs/issues/152) | RE: Validate Global/ElemTable body as a versioned ownership tree |
| [#153](https://github.com/DrunkOnJava/rvt-rs/issues/153) | RE: Validate ElementHeader framing for ElementId and class-tag recovery |
| [#154](https://github.com/DrunkOnJava/rvt-rs/issues/154) | RE: Reproduce complete Formats/Latest parsing and serialization-tag assignment |
| [#155](https://github.com/DrunkOnJava/rvt-rs/issues/155) | RE: Decode parameter-store wire formats and exact Revit unit encodings |
| [#156](https://github.com/DrunkOnJava/rvt-rs/issues/156) | RE: Reproduce sketch-to-solid geometry reconstruction pipeline |

## Audit P0 complete / P1 next (2026-08-29)

Credit: [@STE1200](https://github.com/STE1200) —
[Discussion #112](https://github.com/DrunkOnJava/rvt-rs/discussions/112).

**P0 (merged to `main`):**

| Control | PR |
|---------|----|
| SEC-001 / GOV-002 security hygiene | [#166](https://github.com/DrunkOnJava/rvt-rs/pull/166) |
| PARSE-001 Formats/Latest multipage integrity diagnostics | [#168](https://github.com/DrunkOnJava/rvt-rs/pull/168) |
| Support-matrix scaffold (honesty ceilings) | [#167](https://github.com/DrunkOnJava/rvt-rs/pull/167) |
| Changelog / 0.2.0 plan / install honesty / publish SHA pins (partial) | [#165](https://github.com/DrunkOnJava/rvt-rs/pull/165) |

Formats/Latest production strip remains **disabled**. Findings
[#152](https://github.com/DrunkOnJava/rvt-rs/issues/152)–[#156](https://github.com/DrunkOnJava/rvt-rs/issues/156)
stay **open** for evidence-only research (triage hygiene only in #166).

**P1 next (honest order):**

1. Viewer deploy / browser-regression baseline drift (material_count) + residual CI chores
2. Confirm remaining Actions tag pins outside `publish.yml`
3. Export `source_coverage` real fractions (stub from #167 — no invented %)
4. Evidence-only research on [#152](https://github.com/DrunkOnJava/rvt-rs/issues/152) / [#153](https://github.com/DrunkOnJava/rvt-rs/issues/153) / [#23](https://github.com/DrunkOnJava/rvt-rs/issues/23) — do **not** invent Door/Window/Level ElementId successes (RE-19 / RE-20 negatives stand)
5. Re-baseline issues [#81](https://github.com/DrunkOnJava/rvt-rs/issues/81)–[#96](https://github.com/DrunkOnJava/rvt-rs/issues/96)
6. crates.io / docs.rs / GitHub Releases publish decisions (human)

Hard walls unchanged: Door/Window discriminator, schema-field Wall, Level ElementIds on magnetar, AProperty host joins.

---

## Finding 1 progress (2026-08-29)

Credit: [@STE1200](https://github.com/STE1200) and team —
[Discussion #112](https://github.com/DrunkOnJava/rvt-rs/discussions/112).
Canonical issue: [#151](https://github.com/DrunkOnJava/rvt-rs/issues/151).

Status keys: **reported** (Steffen) · **independently reproduced** (maintainers
on redistributable corpus) · **merged** (on `main`).

| Item | Status |
|------|--------|
| Paging layout **65,249 = 64,896 + 353** | **Independently reproduced** |
| Gated strip-before-inflate on `Partitions/*` and `Global/*` | **Merged** — [#160](https://github.com/DrunkOnJava/rvt-rs/pull/160), narrowed by [#162](https://github.com/DrunkOnJava/rvt-rs/pull/162) / [#161](https://github.com/DrunkOnJava/rvt-rs/pull/161) / [#163](https://github.com/DrunkOnJava/rvt-rs/pull/163) |
| `Formats/Latest` strip | **Excluded** from the production gate (`class_names` regression **9579→8575** on Core Interior). Reported Formats ~**48%** schema loss was **not reproduced** on the redistributable corpus (structured class/field counts hold) |
| Writer / `read_stream` | Stay **stored-byte accurate**; **no paged encoder** yet |
| Stream-evidence harness | **Merged** on `main` — [#158](https://github.com/DrunkOnJava/rvt-rs/pull/158) |
| Evidence matrix | [`docs/recon/wave2-checksum-page-evidence-matrix-2026-08-29.md`](recon/wave2-checksum-page-evidence-matrix-2026-08-29.md) (+ Cloud Agent artifacts). Strongest oracle: `Partitions/46` chunks **925→935** |
| Findings [#152](https://github.com/DrunkOnJava/rvt-rs/issues/152)–[#156](https://github.com/DrunkOnJava/rvt-rs/issues/156) | Still **open** / awaiting independent reproduction — no Finding 2–6 implementation in this update |

Optional decompression PR from Steffen remains welcome for review against the
**narrow** gate (Partitions/Global only; Formats/Latest excluded).

## Optional decompression PR

Steffen’s optional container-layer PR remains welcome. Maintainers will not
duplicate or overwrite a contributed PR — coordinate on [#151](https://github.com/DrunkOnJava/rvt-rs/issues/151).

## Product Q&A → issues

No separate product-improvement issues yet: Steffen has not answered Griffin’s
product questions in Discussion #112. File credited, concrete requests only
after those answers land (search for duplicates first).

## Maintainer notifications (Discussion #112)

Agents cannot change personal or org watch settings with the integration token.

**Enable and verify yourself:**

1. Open [Discussion #112](https://github.com/DrunkOnJava/rvt-rs/discussions/112) → Subscribe (bell) → confirm Subscribed/Custom.
2. Repository → Watch → All activity, or Custom including **Discussions**.
3. GitHub → Settings → Notifications → Discussions / Participating on for email/web.
4. Sanity check: a test reply should produce a notification for subscribers.
