# Viewer privacy posture (VW1-21)

The planned `rvt-view` browser viewer is a **client-side-only**
application. This document pins that posture so the Rust-side
primitives stay compatible with it as the frontend lands.

## The invariant

> An RVT file a user opens in `rvt-view` **never leaves the user's
> browser**. No upload, no cloud processing, no telemetry that
> exfiltrates file content.

This is a deliberate architectural choice, not a privacy-theatre
disclaimer. Revit projects routinely carry unreleased product
designs, occupancy plans of secured facilities, construction cost
data, and personally identifying metadata (original-file paths
expose usernames, machine names, and sometimes project-ID folder
names a Windows admin assigned). The cost of a single accidental
exfiltration is higher than the convenience of a server-side
pipeline.

## What this means for the Rust side

Every `rvt::*` primitive a frontend consumes must be callable
**without** network access. The existing data model already
respects this:

- `RevitFile::open` + `*_bytes` read from memory or disk.
- `ifc::RvtDocExporter`, `ifc::step_writer::write_step`,
  `ifc::gltf::model_to_glb`, `ifc::sheet::render_plan_svg` all
  operate on in-process `IfcModel` values — no I/O beyond the
  caller's explicit paths.
- `ifc::share::encode_to_fragment` serializes to a URL fragment
  (`#v=...`), *not* a server-uploaded share link. The URL itself
  stays in the user's browser history; pasting it elsewhere is
  an explicit user action.
- `ifc::pbr::PbrMaterial` reads Revit `MaterialInfo` values
  already decoded from the file — no external material library
  fetch.

Contributions that would break the invariant:

- A `cloud_upload(path)` function in `rvt-core`.
- A material catalogue that fetches textures from a URL.
- A tile server that a frontend would hit to get element
  geometry on demand.
- Telemetry in any Rust module that talks to an external
  endpoint.

If a future viewer wants any of these, it does them in a
**separate** crate that explicitly opts in — not in `rvt-core`.

## What this means for the WASM build (VW1-01)

When the WASM build lands, the compiled `.wasm` artifact:

- Has no `wasm-bindgen` imports that map to `fetch`, `XMLHttpRequest`,
  or the network-shaped Web APIs.
- Reads user-supplied bytes via `File` / `FileReader` only.
- Writes output through `Blob` download anchors so the user sees
  a browser download dialog — nothing is POSTed.

A CI check can assert the import list: any `wasm-bindgen` import
pointing at the network namespace should fail the build.

## What this means for the static site (VW1-18)

When the viewer deploys to GitHub Pages or a similar static host:

- No cookies, no localStorage tokens, no identifiers that survive
  page reloads beyond the explicit `#v=...` URL fragment.
- No third-party analytics SDK (Google Analytics, Plausible,
  PostHog, etc.) — these send URLs, and URL fragments in this
  viewer can contain file hashes the user hasn't chosen to
  disclose.
- A `Content-Security-Policy` header that blocks `connect-src`
  entirely except for blob URIs.

## Auditability

Three checks make the posture verifiable post-facto:

1. `cargo deny check` — no crate in the dep tree has a known
   telemetry history.
2. `wasm-objdump` on the compiled viewer — grep the import
   section for `fetch` / `XMLHttpRequest` / `WebSocket`.
3. Live test — open `rvt-view` in a browser with devtools' Network
   tab open; load a demo RVT; assert zero requests fire after the
   initial page load.

The CI job for check #2 is tracked as part of VW1-01 (WASM build
pipeline).
