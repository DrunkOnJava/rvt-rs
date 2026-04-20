# Threat Model

rvt-rs is a parser for a complex, proprietary, adversary-reachable
binary format. This document lists the threats we defend against, the
mitigations we ship, and the ones we explicitly don't cover so callers
can make informed deployment decisions.

## Attacker capabilities assumed

An attacker can:

- Hand-craft a malicious `.rvt` / `.rfa` / `.rte` / `.rft` file.
- Upload that file to a service that uses rvt-rs to parse it
  (SaaS file-scanning, archival ingest, compliance checking,
  openBIM pipelines, etc.).
- Submit crafted files to command-line users who run
  `rvt-analyze file.rfa` without inspecting the input first.

An attacker cannot (outside scope):

- Force rvt-rs to execute code it wasn't compiled with. rvt-rs is
  pure parser — no `eval`, no dynamic loading, no plugin API.
- Tamper with rvt-rs's source code or binary artifacts once
  published (supply-chain attacks are their own threat model; see
  §Supply chain).

## Threats and mitigations

### T1 — Remote code execution via parser bug

**Threat:** A malformed binary triggers a memory-safety bug in the
parser, letting the attacker hijack control flow.

**Mitigations:**

- **Pure safe Rust.** The library crate compiles with
  `#![forbid(unsafe_code)]` on the pure-parser tier (see Phase 1
  workspace split in TODO-BLINDSIDE.md). All parsing logic —
  CFB opening, gzip decompression, schema parsing, walker — runs
  with safe-Rust bounds checks.
- **Panic-safe short inputs.** `FieldType::decode`, `find_chunks`,
  `gzip_header_len`, `inflate_at`, and the other byte-level
  primitives handle arbitrarily short / truncated input by
  returning `None` or `Result::Err`, never by panicking or
  indexing out of bounds. Property tests + corpus-driven fuzzing
  (see SEC-14..25) enforce this.
- **No C dependencies.** `flate2` uses `miniz_oxide` (pure Rust) as
  its default backend. `cfb` is pure Rust. No FFI to C parsers.

**Residual risk:** Pyo3 FFI layer in `src/python.rs` is the one
place where safe-Rust forbid doesn't apply (pyo3 macros expand to
unsafe function bodies). We isolate this to `rvt-py` (a separate
crate) post–SEC-12 so the core parser stays forbid-unsafe.

### T2 — Denial-of-service via unbounded memory

**Threat:** A small compressed file expands to gigabytes during
decompression, or a claimed stream size forces a multi-GB
allocation, starving the host of memory.

**Mitigations:**

- **`compression::InflateLimits`.** Every `inflate_at_with_limits`
  call caps output bytes. Default is 256 MiB/call. Legitimate
  files parse; compressed bombs (1 KB → 1 GB zeros) fail with
  `Error::DecompressLimitExceeded`.
- **`reader::OpenLimits`.** `RevitFile::open_with_limits` stats
  the file before reading; refuses >2 GiB by default. Protects
  against a claimed huge file in upload scenarios.
- **`read_stream_with_limit`.** Per-stream cap (default 256 MiB)
  checked against CFB directory entry + enforced during chunked
  read so malformed CFB "stream is huge" claims don't slip
  through.

**Residual risk:** An attacker inside the 256 MiB/stream limit can
still legitimately force 256 MiB allocations. Callers expecting
many small files should compose with their own total-memory cap.

### T3 — Denial-of-service via algorithmic complexity

**Threat:** A crafted file makes the parser iterate an unbounded
number of times (e.g. walker scans all possible entry offsets).

**Mitigations:**

- **`WalkerLimits` (pending API-11).** Caps max_scan_bytes +
  max_trial_offsets on the brute-force ADocument entry-point
  detector.
- **`find_chunks` bounded.** The gzip-magic scanner over raw bytes
  is linear in input length.
- **`parse_schema` scan cap.** Schema parsing scans at most
  64 KiB from the start of decompressed `Formats/Latest`. Returns
  `scan_was_capped: true` diagnostic if it hits the cap (pending
  API-10).

### T4 — Information disclosure / PII exposure

**Threat:** A user shares a `.rvt` or tool output not realizing it
contains Windows usernames, Autodesk-internal paths, document
GUIDs, or other identifying info.

**Mitigations:**

- **`--redact` flag on every CLI.** Scrubs usernames (C:\Users\X
  → C:\Users\<user>), Autodesk-internal paths (Apollo build
  servers), and project-ID folder names. Replaces with
  placeholder tokens while preserving path shape so claims
  remain verifiable.
- **PII guard CI job.** Scans the repo's committed files for
  PII-shaped patterns on every push. Blocks commits that contain
  them.
- **No embedded user emails, no account info.** The reader
  extracts path + GUID from Revit's own metadata streams; it
  doesn't invent or enrich PII.

### T5 — Supply chain

**Threat:** A transitively-included crate adds a malicious
dependency that the parser pulls in automatically.

**Mitigations:**

- **`cargo-deny check` on every push (pending SEC-27).**
  License allowlist (no GPL/proprietary), advisory deny, source
  allowlist (crates.io only, no arbitrary git).
- **`cargo-audit` on every push (pending SEC-28).** Fails CI on
  known RustSec advisories.
- **`actions/*` SHA-pinned (partial; pending SEC-29).** Common
  GitHub Actions pinned by commit hash to prevent
  tag-retargeting attacks.
- **OIDC-based PyPI publishing.** No long-lived API token in
  repo secrets. Token scope is per-release via Trusted Publisher.

## Out of scope

- **Side-channel timing attacks.** rvt-rs is not a cryptographic
  library. Timing variations on file parsing are not in the
  threat model.
- **Attacks requiring code modification.** We assume the library
  binary is what the crate authors published. Binary tampering
  is a supply-chain problem handled by cargo's own integrity
  mechanisms (checksums, signatures via `cargo sparse`).
- **Bypassing rvt-rs's validation with a second parser.** If
  downstream code re-parses the RVT with a different library and
  that library has bugs, those bugs are out of scope for
  rvt-rs's threat model.

## Reporting vulnerabilities

See [`SECURITY.md`](SECURITY.md) for the private reporting
channel. Do not file public issues for parser-safety bugs until
the fix has shipped in a release.
