# Security policy

## Supported versions

rvt-rs is pre-1.0 and has one maintainer, so support is kept to what
can actually be delivered:

| Version | Security fixes |
|---|---|
| `main` | yes |
| The latest release line (see [Releases](https://github.com/DrunkOnJava/rvt-rs/releases)) | yes, in its next patch or minor release |
| Older release lines | no, unless a note here says that line is maintained |

No older line is maintained today. A fix lands on `main` first and reaches
users in the next release.

## Reporting a vulnerability

If you believe you've found a security issue — including anything
that would let an attacker cause rvt-rs to crash, mis-decode, or
leak data when fed a hostile input file — please **do not open a
public GitHub issue**.

**Primary channel:** [GitHub private vulnerability reporting](https://github.com/DrunkOnJava/rvt-rs/security/advisories/new)
(Security → Advisories → Report a vulnerability). Private vulnerability
reporting is enabled on this repository. Use it for coordinated
disclosure; do not rely on `users.noreply.github.com` addresses — they
are not a usable inbox.

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

Reports are handled on a best-effort basis: there is no guaranteed
response or fix time. You can expect the report to be read, an
acknowledgement when it has been, a candid assessment of whether it is in
scope, and a fix in the next release for a confirmed issue, with more
severe issues taken first. If a report seems to have gone unnoticed, a
short follow-up on the same advisory is welcome.

## Scope

In scope:

- **Malformed input parsing.** Any input file — valid RVT/RFA or
  not — should not crash the library, trigger `panic!`, or cause
  out-of-bounds reads. If you find one that does, that's a bug.
- **Information disclosure via output.** If rvt-rs's default
  output (without `--redact`) leaks more than the input file
  itself contains, that's a bug.
- **Memory safety.** The core library crate has
  `#![forbid(unsafe_code)]` (no `unsafe` in our parser code today;
  any `unsafe` that slips in must have a safety argument). Python
  bindings live in the separate `rvt-py` crate.
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
repository's Security tab. The disclosure timeline is agreed with
the reporter; by default the advisory is published once the fix is in a
release.
