---
name: Bug report
about: rvt-rs misbehaves on a specific input or environment
title: "[bug] "
labels: bug
---

<!--
  For security-sensitive issues (crashes on hostile input, silent data
  corruption, information disclosure) please use the flow in
  SECURITY.md instead of opening a public issue.
-->

## What happened

<!-- One sentence. What did you see that was wrong? -->

## Minimal reproducer

<!-- Smallest command + input that triggers the issue. -->

```
$ ./target/release/rvt-analyze <file>
...
```

## Expected behavior

<!-- What would have been correct output? -->

## Environment

- `rvt-rs` version (`cargo pkgid rvt` or commit hash):
- OS + version:
- Rust toolchain (`rustc --version`):
- Input file kind: `.rvt` / `.rfa` / `.rte` / `.rft`
- Revit release that wrote the file (if known):

## Input-file disclosure

- [ ] I've confirmed the input file contains no PII that would prevent
      sharing (or I've scrubbed a copy via `rvt-analyze --redact`).
- [ ] I can share the input if requested (or a synthetic reproducer).
