# Email draft — thanks to phi-ag for the `rac_basic_sample_family` corpus

This is a draft. Before sending, fill in `{{recipient_name}}`, pick a
recipient address for the To: line, and confirm the sign-off. The body
is written to be sent as-is once those three substitutions are made.

---

**To:** `{{recipient_email}}`
**Subject:** Thank you for the Revit sample corpus — rvt-rs project

Hi {{recipient_name}},

I'm Griffin Long, author of rvt-rs — an Apache-2.0 Rust library for
reading Autodesk Revit files (.rvt, .rfa, .rte, .rft) without a Revit
installation. I wanted to write directly and say thank you for
publishing the `rac_basic_sample_family` corpus through the phi-ag/rvt
repository. Your 11-release set of RFAs spanning Revit 2016 through
2026 has been the single most important piece of test material in the
project, and I don't think rvt-rs would be where it is without it.

Concretely, the corpus is what made the following possible:

- **Cross-version testing across 11 releases.** Every integration test
  in `tests/samples.rs` iterates the 2016–2026 range, so every change
  to the container parser, compression layer, or schema walker is
  validated against all eleven versions on every push.

- **Schema-drift detection.** Having the same family saved by every
  Revit release turned `Formats/Latest` from a single-file observation
  into a longitudinal dataset. It's how we produced the 122-class ×
  11-release tag-drift table (`docs/data/tag-drift-2016-2026.csv`),
  catalogued 395 classes with 13,570 fields at 100% type classification
  across the whole corpus, and enforced that zero-`Unknown` property
  as a CI regression gate.

- **BasicFileInfo extractor validation against real variants.** The
  build-tag format shifted across releases; every time we thought we
  had the extractor right, one of the older years disagreed. The
  corpus kept us honest. `rvt-history` can now reconstruct upgrade
  chains like the 2026 sample's path back to Revit 2018.

- **Maturing the Python bindings.** The pyo3 + maturin bindings
  shipped alongside the Rust library, with a Jupyter quickstart that
  walks through files from the corpus. Having representative RFAs for
  every release made it possible to write Python-side tests that
  exercise the parser end-to-end rather than round-tripping a single
  mock file.

As a result of work grounded in that corpus, rvt-rs now ships nine
CLIs (`rvt-analyze`, `rvt-info`, `rvt-schema`, `rvt-history`, `rvt-diff`,
`rvt-corpus`, `rvt-dump`, `rvt-doc`, `rvt-ifc`), 54 per-class decoders,
and an IFC4 STEP export pipeline that produces a valid spatial tree
with per-element entities, opening / fill relationships, and extruded
solids. The committed synthetic fixture opens cleanly in BlenderBIM and
IfcOpenShell. Python bindings are published to PyPI as `rvt`; the
crates.io release is in the final pre-publish queue, and I'll send a
follow-up once it lands.

Source is public under Apache-2.0 at
`https://github.com/DrunkOnJava/rvt-rs`. rvt-rs does not redistribute
any of your corpus files — the integration tests pull them from
phi-ag/rvt via Git LFS and skip cleanly when the corpus is absent, so
the attribution and distribution posture is entirely yours. If you'd
like to be credited in the project's acknowledgements or in
`CITATION.cff`, please let me know the form you'd prefer and I'll add
it directly. If there are additional file variants you'd like to see
covered — password-protected models, linked-model fixtures, pre-2016
content, MEP-heavy families, larger project RVTs — I'd also like to
hear which directions you see as most useful.

Thank you again. The phi-ag corpus is a real public-goods contribution
to openBIM, and the project is better for it.

Best,
{{sender_signoff | default: "Griffin Long"}}
https://github.com/DrunkOnJava/rvt-rs
