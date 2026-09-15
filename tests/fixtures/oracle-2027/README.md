# Measured native 2027 parameter spans

`parameter-spans.json` contains **exact byte excerpts**, not re-encoded values,
from the `walls-v3` controlled synthetic Revit 2027 build 27.2.0.39 suite
(2026-09-12). Source SHA256 hashes, stream names, prepared member offsets, and
inflated member offsets locate each excerpt. These observations contain only
a built-in parameter id, length, and authored test label; they contain no
customer data, file paths, document identities, or user metadata.

The cases cover Mark/Type Mark, empty strings, 127/128/255/256-unit boundaries,
a Unicode value containing a surrogate pair, and two duplicate-label
occurrences. Unit tests consume these independently measured bytes and check
truncation and duplicate multiplicity. Repository Apache-2.0 terms apply to
the authored observations and test metadata.

Full native RVTs remain local under the corpus **authorized private/local
probe lane**: unlike these tiny excerpts, their BasicFileInfo includes the
execution machine's user-specific save path. They are not public corpus
fixtures and have not been modified to hide that provenance. Run the optional
full-file regression with:

```sh
RVT_ORACLE_RUN_DIR=/path/to/walls-v3 cargo test --release --test oracle_2027_native
```

That test verifies native RVT hashes against the run manifest, independently
decodes every partition's occurrences, and only then loads API snapshots to
compare multisets. It does not validate ownership or geometry.

`warning-reference-negative.json` preserves the 112-byte native context around
an accidental parameter-id match in duplicate-warning data. The following 2784
coincides with an element id; interpreting it as a string count crosses the
supplied context. The research framing does not recognize this warning data
as a supported parameter-record body; generic warning framing remains unknown. This exposed a whole-member false positive masked by
an initial control-character filter. Final extraction requires structural
bounds and preserves all valid UTF-16, rather than screening strings for
plausibility.

`wall-geometry-spans.json` and `floor-plane-spans.json` contain exact native
geometry byte excerpts from geometry-v2 and geometry-holdout-v2 (same Revit
2027 build). Each records source SHA256, stream/member, and inflated offset;
wall cases also retain the API geometry artifact hash. Six wall spans include
translated/rotated nonzero-origin cases, independent heights/thicknesses, and an
arc sweep. Six floor pairs include elevation, rotation, irregular profile, hole,
and thickness variants. The excerpts contain only geometric primitives, no
user paths or document metadata. They are compared to saved API coordinates and
dimensions, not solely to values reconstructed by the byte probe. Apache-2.0
terms apply to these authored observation files as above.

`storage-revision-rows.json` retains four exact 40-byte ElemTable rows from the
geometry holdout. Type-thickness edits have fields +28=1 and +32=0, disproving
an earlier equality hypothesis. Their tests wrap each unchanged row in a
synthetic single-row table header; source hashes and original offsets identify
the actual native evidence. These excerpts contain no user metadata.
