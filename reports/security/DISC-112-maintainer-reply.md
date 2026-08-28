# Draft reply for Discussion #112

(Automation token lacked `discussions: write`. Paste into
https://github.com/DrunkOnJava/rvt-rs/discussions/112 )

---

Thanks Steffen — this is exactly the kind of clean-room evidence we want, and filing as separate issues (with confidence + repro) per CONTRIBUTING.md is the right flow. A PR for the container/paging layer is welcome once we have a shared oracle.

### Item 1 (checksum-paged streams) — what we checked today

We agree the **constants and public framing story are credible**:

- full stored page **65,249** = **64,896** payload + **353** trailer
- matches ahzs645/reviter’s ODA loader notes (`PagedStreamImplReader<…, 65249u>`, `stripRevitPageChecksums`)

We added the strip helpers + probes (see `src/compression.rs` and `reports/security/DISC-112-page-checksum-probe-2026-08-28.md`), but we **have not flipped strip on by default**.

On our redistributable project corpus (`Revit_IFC5_Einhoven.rvt`, `2024_Core_Interior.rvt`), a naive 0-aligned strip **does not** increase `Formats/Latest` inflate yield; it slightly *decreases* it and reduces opportunistic `class_names` hits while structured `SchemaTable` class counts stay flat. So enabling it as the default inflate path would be a regression on those files without your multi-page partition oracle.

**Ask:** if you can share a redistributable repro (or a minimized stored-stream dump + expected inflated length / schema counts / per-chunk gzip trailer checks), we will re-run the probe and gate the default path on that evidence. Until then we are treating this as a high-priority open format question, not yet a declared vulnerability.

### Other items

ElemTable ownership tree, element-header framing, Formats grammar, parameter store, and extrusion geometry sound extremely useful — please open one issue per item with the evidence packs you described. We will review them in that form and can take dated reconnaissance addenda + example probes under Apache-2.0.

Appreciate you asking before flooding the tracker.
