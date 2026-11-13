# Viewer privacy posture

The browser viewer (`viewer/`, deployed at
<https://drunkonjava.github.io/rvt-rs/>) is a **client-side-only**
application. This document pins that posture, so the Rust-side
primitives and the viewer stay compatible with it, and says what each
check below covers.

## The invariant

> An RVT file a user opens in the viewer **never leaves the user's
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
- A shared viewer state would be a URL fragment (`#v=...`) that
  `ifc::share::decode_from_fragment` reads, *not* a server-uploaded
  share link; nothing shipped reads or writes one yet, the viewer's
  own URL options being query parameters. The URL itself stays in the
  user's browser history; pasting it elsewhere is an explicit user
  action.
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

## What this means for the WASM build

The compiled `.wasm` artifact, built from the `rvt-wasm` crate:

- Has no `wasm-bindgen` imports that map to `fetch`, `XMLHttpRequest`,
  or the network-shaped Web APIs.
- Reads user-supplied bytes via the `File` / `Blob` API
  (`file.arrayBuffer()`) only.
- Writes output through `Blob` download anchors so the user sees
  a browser download dialog — nothing is POSTed.

CI asserts the import list on every deploy (check 2 under
[Auditability](#auditability)): an import that reaches the network fails
the build.

## What this means for the static site

The viewer is deployed to GitHub Pages, a static host, and keeps to these
rules:

- No cookies, no localStorage tokens, no identifiers that survive
  page reloads beyond its own query parameters; the `#v=...` share
  fragment is a primitive the page does not bind.
- No third-party analytics SDK (Google Analytics, Plausible,
  PostHog, etc.) — these send URLs, and URL fragments in this
  viewer can contain file hashes the user hasn't chosen to
  disclose.
- A `Content-Security-Policy` `<meta>` element in
  `viewer/index.html` that keeps `connect-src`
  restricted to `'self'` and `blob:`. The same-origin allowance
  exists only because the `wasm-pack` JavaScript loader fetches the
  static `rvt_bg.wasm` bundle from the viewer origin; `blob:` is
  needed for Three.js to read the in-memory GLB object URL generated
  from local WASM output. Application data requests are not allowed.
- The same policy's `worker-src` is `'self' blob:`. `blob:` was added on
  2026-11-05 (Griffin: "Keep the claim, no request") so that the page's two
  workers, the one that opens the file and the one that reads an element's
  saved parameters, can be bundled into the page's own script and started from
  blob URLs, which name bytes the page made itself and are no request. The
  alternative, a worker script fetched when it was first needed, would have been
  a request after the file was open (the first selection, or opening the file
  again with another phase or the saved-mesh toggle), and the claim that nothing
  is requested once the file is open would no longer have been true.
  `script-src`, `connect-src` and every other directive are unchanged, and a
  worker started from a blob URL takes the policy of the page that made it.

Allowed browser requests are limited to initial same-origin static
assets needed to boot the viewer:

- the HTML document;
- Vite-generated JavaScript chunks. Both workers are inside the page's own
  bundle and start from blob URLs, and so is the decoder's JavaScript glue (a
  static import, not a chunk fetched at the first export), so none of them is a
  request; the first page load carries them, which made it heavier by the
  amount `docs/viewer-api.md` states;
- the generated `rvt_bg.wasm` WebAssembly binary, fetched once, by the worker
  of the first open, which compiles it and hands the compiled module to the
  page. The page keeps it for good: the next open (another phase, the
  saved-mesh toggle, the large-model retry, another file), a worker that reads
  an element's saved parameters and the main thread's exports start from that
  module and fetch nothing. So selecting an element, exporting IFC, plan or
  glTF, and opening the file again make no request, and a page taken offline
  after it has loaded still does all of them;
- optional same-origin source maps or favicon requests from the
  browser tooling/runtime;
- optional same-origin `/demos/` catalog JSON, thumbnails, and
  redistributable demo binaries (fetched only at boot for the catalog /
  when the user clicks a gallery entry).

`blob:` and `data:` URLs are local browser object URLs, not network
requests; the viewer uses them for GLB rendering and user-initiated
downloads.

## Auditability

Three checks make the posture verifiable post-facto:

1. `cargo deny check` — `deny.toml`'s RustSec advisories and yanked
   crates, license allowlist, duplicate and wildcard bans, and
   crates.io-only sources. It has no telemetry knowledge base, so a
   crate with a telemetry history is caught only if someone listed it.
2. `viewer/scripts/audit-wasm-imports.mjs` on the compiled viewer —
   every import of the `.wasm`, by name and by the body of its shim
   in the wasm-bindgen glue, checked for `fetch`, `XMLHttpRequest`,
   `WebSocket`, `EventSource`, `sendBeacon`, `WebTransport` and
   `RTCPeerConnection`. A constructor such as `new WebSocket` imports
   as `__wbg_new_<hash>`, so the name alone would miss it. Before
   October 2026 this check grepped `wasm-objdump` output for quoted
   names that wabt never prints, so it could not fail.
3. Playwright browser test
   (`viewer/tests/no-network.spec.ts` + `viewer/tests/wasm-no-network-imports.spec.ts`) — load the built viewer,
   open a sample (`RVT_VIEWER_SAMPLE`; in the deploy job, the
   MIT-licensed `Revit_IFC5_Einhoven.rvt` project), allow only
   same-origin static boot assets, then assert zero HTTP/WebSocket
   requests after the model is loaded, after an element is selected (the
   selection step fails the test when the sample has no element to select),
   after an IFC, a plan and a glTF export are made, and after the file is opened
   again. `viewer/tests/parameters-row.spec.ts` adds that a page taken offline
   after the load still does all of those, and that no request of any kind, a
   worker's included, is made while it does.

The CI viewer deploy job runs checks #2 and #3. Browser-network
failures print the request URL, method, phase, resource type, and
Chrome DevTools Protocol initiator so regressions point directly
at the offending code path.

What the checks cover: the WebAssembly module's imports (check 2), and
the requests the page makes while booting, opening that sample and
afterwards (check 3), with the page's Content-Security-Policy limiting
`connect-src` to `'self'` and `blob:`. They are checks of those paths, not
a proof that no other code path, browser extension or modified copy of
the page could make a request.
