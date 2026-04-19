<!-- Please keep this PR template short and complete each checkbox. -->

## Summary

<!-- One sentence. What does this PR do? -->

## Changes

<!-- Bulleted list of notable changes. -->

- 
- 

## Testing

<!-- How did you verify the change? -->

- [ ] `cargo test --release` passes on my machine
- [ ] If new behavior: a unit test pins it in the relevant `_test` module
- [ ] If a perf change: rerun `tools/bench.sh` and attach the before/after
- [ ] If a new RE finding: added a probe under `examples/` AND an
      addendum to `docs/rvt-moat-break-reconnaissance.md`

## Legal / contribution hygiene

- [ ] I agree this work is licensable under Apache-2.0 (see CONTRIBUTING.md).
- [ ] This PR contains no Autodesk proprietary source / decompiled
      internals / NDA'd SDK content.
- [ ] Any new test fixtures use synthetic values (`testuser`, `111111`,
      `FY-20XX`, etc.) and contain no real-world PII.
