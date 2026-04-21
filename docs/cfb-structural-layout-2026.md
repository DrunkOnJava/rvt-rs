# Revit OLE/CFB sector layout across the 11-release corpus (WRT-10)

Date: 2026-04-21
Probe: `examples/probe_cfb_sector_layout.rs`
Corpus: `rac_basic_sample_family-{2016..2026}.rfa`

## Summary

The `cfb` crate (v0.11) defaults to **CFB v4 + 4KB sectors**, which
matches Revit's file format. So WRT-10 is NOT about rewriting the
version/sector-size defaults — those already agree. The divergence
surfaces in *intra-file ordering*: which sector index each stream's
data lands at, where the mini-FAT lives, and the specific padding /
DIFAT patterns.

## Baseline header dump (all 11 releases)

Every file in the corpus shares these header fields:

| Field | Value | Semantics |
|-------|-------|-----------|
| Magic | `D0 CF 11 E0 A1 B1 1A E1` | OLE Compound File |
| Major Version | 4 | CFB v4 (4KB sectors) |
| Minor Version | 62 (0x3E) | Per MS-CFB §2.2 default for v4 |
| Byte Order | 0xFFFE | Little-endian |
| Sector Shift | 12 | 2^12 = **4096** bytes/sector |
| Mini-Sector Shift | 6 | 2^6 = **64** bytes/mini-sector |
| Directory Sector Count | 1 | Single 4KB directory sector |
| FAT Sector Count | 1 | Single 4KB FAT sector |
| First Directory Sector | 1 | Directory at sector 1 |
| First DIFAT Sector | 0xFFFFFFFE | ENDOFCHAIN (no DIFAT sectors — all FAT sector refs fit inline) |
| DIFAT Sector Count | 0 | Consistent with 1stDI=ENDOFCHAIN |

**Release-specific divergence found:**

| Release | First Mini-FAT Sector |
|---------|----------------------|
| 2016    | 2                    |
| 2017    | 2                    |
| 2018    | 2                    |
| 2019    | 2                    |
| 2020    | 2                    |
| 2021    | 2                    |
| 2022    | 2                    |
| 2023    | 2                    |
| 2024    | 2                    |
| **2025** | **4**                |
| 2026    | 2                    |

Only Revit 2025 places its mini-FAT at sector 4 instead of sector 2.
2024 and 2026 both use sector 2 — this is a 2025-specific anomaly,
not a trend. Likely a side-effect of whatever internal allocator
change Revit 2025 shipped; 2026 reverted to the earlier layout.

## What the `cfb` crate does by default

`cfb::CompoundFile::create` (src/lib.rs:89) delegates to
`create_with_version(Version::V4, …)` — so the baseline header
values above are already the correct target. The bug surface for
byte-identical round-trip is therefore the *allocation order*: when
`write_with_patches` re-creates a CFB container and writes streams
in a specific order, the sector indices it picks for each stream may
not match the source file's.

Concrete consequence today: a round-tripped family file has the
same streams, same contents, same CFB v4 header — but the bytes
on disk are not byte-identical because (for example) `FormatsLatest`
might land at sector 10 in the source and sector 12 in the rewrite.

## Next (WRT-10.3 / 10.4 / 10.5)

1. **WRT-10.3** — implement a sector-reordering pass that, given the
   source's observed (stream → starting sector index) map, makes
   the rewriter allocate streams at matching indices. The `cfb`
   crate exposes no such hook as of v0.11; the pass likely needs to
   operate at the byte level *after* `cfb` has finished, moving
   4KB blocks around and rewriting FAT entries to reflect the new
   chain heads.

2. **WRT-10.4** — byte-identical round-trip test. For each of the
   11 releases, copy the family file through `write_with_patches`
   with an empty patch set and assert `src == dst` byte-for-byte.
   Current state: streams + header correct, physical sector order
   may differ → test fails without the WRT-10.3 reordering pass.

3. **WRT-10.5** — wire the reordering pass behind an opt-in flag
   on `write_with_patches` (call it `preserve_sector_layout: bool`).
   Default `false` so existing callers (who don't care about
   bit-identical rewrites) aren't slowed down by the extra pass.
   Downstream tools that DO care (corpus re-emission, canonical
   form checks, git-friendly diffing) flip the flag.

Estimated scope: WRT-10.3 is a multi-hour RE task on top of `cfb`
crate internals. Blocker is whether upstream `cfb` can expose a
"construct with explicit FAT layout" API, or whether we patch over
its output. Worth checking with `cfb` upstream before investing —
the API question is general enough that other downstream crates
may want the same hook.

## Evidence paths

- Probe script: `examples/probe_cfb_sector_layout.rs`
- Corpus header dumps (reproducible): re-run the probe
- `cfb` crate source: `~/.cargo/registry/src/…/cfb-0.11.0/src/lib.rs`
