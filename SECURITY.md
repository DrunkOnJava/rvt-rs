# Security policy

## Supported versions

rvt-rs is pre-1.0. Only the `main` branch is security-supported at
this stage. Once `0.1.0` ships to crates.io, the two most recent
minor versions will be supported.

## Reporting a vulnerability

If you believe you've found a security issue — including anything
that would let an attacker cause rvt-rs to crash, mis-decode, or
leak data when fed a hostile input file — please **do not open a
public GitHub issue**. Instead, email:

**151978260+DrunkOnJava@users.noreply.github.com** with the subject line
`[SECURITY] rvt-rs: <one-line summary>`.

Include, if possible:

1. A minimal reproducer (the smallest input that triggers the issue).
2. The exact `rvt-rs` version (`cargo pkgid rvt`).
3. Your platform and Rust toolchain version.
4. A description of the impact (denial of service, memory safety,
   silent data corruption, information disclosure, etc.).
5. For parser crashes, rerun the failing command or API call with
   `RUST_BACKTRACE=1` and include the backtrace, the command line, and
   whether the input was passed through the CLI, Rust API, Python API,
   Wasm API, or viewer.

I will acknowledge receipt within 72 hours. We aim to patch
confirmed issues within 7 days for high-severity items and within
30 days for medium-severity items.

## Scope

In scope:

- **Malformed input parsing.** Any input file — valid RVT/RFA or
  not — should not crash the library, trigger `panic!`, or cause
  out-of-bounds reads. If you find one that does, that's a bug.
- **Information disclosure via output.** If rvt-rs's default
  output (without `--redact`) leaks more than the input file
  itself contains, that's a bug.
- **Memory safety.** The library has `#![deny(unsafe_code)]` on
  its target (no `unsafe` in our code today; any `unsafe` that
  slips in must have a safety argument).
- **Denial of service via resource exhaustion** (file-size-
  linear CPU/memory only). A file ten times the size should not
  cause a hundred-times-larger allocation.

Out of scope:

- Bugs in our runtime dependencies (`cfb`, `flate2`, etc.) that
  are upstreamed and already under their maintainers' security
  policies.
- Issues in the Revit file format itself (Autodesk's problem).
- Redistribution of Autodesk-owned sample files (please don't send
  us any — the test corpus is pulled from phi-ag/rvt at build
  time).

## Disclosure

Once a fix ships, I'll credit the reporter (if they want credit) in
the CHANGELOG and in a security advisory on the GitHub
repository's Security tab. Coordinated disclosure timelines are
negotiable — the default is fix-then-disclose within 7 days.
