# Twitter/X thread — rvt-rs

Ten-tweet thread, audit-honest framing, same tone as `hn-show-hn.md`. Each tweet is self-contained enough to survive a quote-tweet. Character counts verified against Twitter's 280-character limit (em-dash, en-dash, and ASCII punctuation each count as 1 code unit).

---

## Tweet 1 — Hook

1/ For 20 years, Autodesk .rvt files have been opaque outside Revit itself. rvt-rs cracks them open with Rust.

github.com/DrunkOnJava/rvt-rs

Apache-2.0.

(154 chars)

---

## Tweet 2 — Capability

2/ It reads .rvt, .rfa, .rte, .rft from Revit 2016 through 2026 by parsing the OLE/CFB container and decoding Revit's truncated-gzip streams.

54 per-class element decoders today: walls, floors, roofs, doors, windows, columns, beams, stairs, rooms, grids, levels, materials.

(274 chars)

---

## Tweet 3 — IFC4 export unlock

3/ The unlock: rvt-rs emits IFC4 STEP.

The committed synthetic-project fixture opens in BlenderBIM with a real spatial tree, extruded walls + slab, a front door with an opening chain in the host wall, materials with surface styles, and Pset_WallCommon / Pset_WindowCommon.

(273 chars)

---

## Tweet 4 — How it works

4/ How it works.

Parse the OLE/CFB container. Decompress Revit's truncated-gzip streams. Walk the embedded Formats/Latest schema (395 classes, 13,570 fields, 100% type-classified across 11 releases).

Then a schema-directed walker reads ADocument field by field.

(263 chars)

---

## Tweet 5 — Why it matters

5/ Why it matters.

Revit files have been opaque outside Autodesk's C# API, forcing BIM analytics, migration, archival, and research onto Windows + Revit.

rvt-rs is a byte-level reader. No Autodesk runtime, no Wine, no commercial SDK. Drop it into any Rust stack.

(264 chars)

---

## Tweet 6 — Honest gaps

6/ Honest gaps.

- No write support (stream-level round-trip only)
- No pre-2016 files
- No password-protected models
- No linked-model resolution
- MEP and annotation decoders not yet landed
- IFC profiles rectangular only; extrusion dimensions caller-supplied

(261 chars)

---

## Tweet 7 — Why Rust

7/ Why Rust.

Memory safety on an adversarial binary format. No Autodesk runtime dependency. Cross-platform (macOS, Linux, Windows). Python bindings via pyo3+maturin — pip install rvt.

2024 edition, MSRV 1.85. 9 CLIs. 30+ reproducible probes.

(243 chars)

---

## Tweet 8 — Credits

8/ Credit where it's due.

The IfcOpenShell + BlenderBIM ecosystem is what makes verification possible — you can open the output and actually see walls.

phi-ag/rvt provided the 11-release 2016–2026 sample corpus that gated cross-version schema classification.

(260 chars)

---

## Tweet 9 — Call for feedback

9/ What we want.

Issues and PRs, especially on the partial areas.

Project .rvt corpus submissions — families got us to 100% schema typing, real projects will stress-test geometry.

IFC4 schema critique from AEC folks who read STEP natively.

(242 chars)

---

## Tweet 10 — Repo + license

10/ Repo: github.com/DrunkOnJava/rvt-rs
License: Apache-2.0
Crate: rvt (Rust) / rvt (PyPI)

If you work with Revit files outside Autodesk's toolchain — what would unblock you first?

(181 chars)

---

## Posting notes

- Post as a native thread (reply chain), not a single long tweet.
- Tweet 1 is the most quote-tweetable: repo URL + one-line hook. Pin it separately if engagement takes off.
- Tweet 3 (IFC4 / BlenderBIM) is the AEC-audience hook — expect most amplification from openBIM / IfcOpenShell / OSArch accounts here.
- Tweet 6 (honest gaps) is deliberate: the Rust-OSS audience punishes overclaiming. Leaving this in is a trust signal.
- Tweet 10 ends with a question to invite replies rather than passive impressions.
- Do not add hashtags inline — they burn chars and read as spam on this audience. If needed, reply to your own thread with tags at the end.
