# DISC-112 / #151 — Wave 1 Worker C decompression investigation (2026-08-29)

Credit: [@STE1200](https://github.com/STE1200) (Discussion #112).  
Scope: reproduce / narrow / refute checksum-paged decompression; **no production inflate wiring**.

## Reported vs independently reproduced

| Item | Reported (@STE1200) | Independently reproduced here? |
|---|---|---|
| Page layout 65,249 = 64,896 + 353 | Yes | **Yes** (constants + real stream page alignment; matches reviter) |
| Must strip before inflate | Yes | **Yes** on large partitions; synthetic fixtures require strip for round-trip |
| Silent clean terminate + drift | Yes | **Yes** (synthetic); partitions show failed members under control that succeed after strip |
| Formats/Latest ~48% schema loss | Yes (reporter sample) | **No** on redistributable corpus — schema classes/fields unchanged |
| ~190 KB threshold | Yes | **Narrowed** — effect tracks full-page count / multi-member desync, not a single size cliff |
| 209/209 gzip trailer oracle | Yes (two reporter files) | **Not re-run** (fixtures/methodology not shared in-repo) |

**This is not a “fixed” claim.** `main` still calls bare `inflate_at` / `inflate_all_chunks` at production sites. Strip helpers exist but are opt-in / probe-facing.

## Strongest independent evidence

Redistributable `2024_Core_Interior.rvt` (magnetar-io / project corpus):

| Stream | Stored | Pages | Control chunks ok | Stripped chunks ok | Δ ok | Δ inflated bytes |
|---|---:|---:|---:|---:|---:|---:|
| Partitions/46 | 17,072,209 | 261 | 925† | 935 | **+10**† / **+73**‡ | **+~8.0 MiB** |
| Partitions/53 | 5,692,783 | 87 | (fail>0)‡ | all ok‡ | **+17**‡ | **+~2.1 MiB** |
| Partitions/61 | 2,809,648 | 43 | (fail>0)‡ | all ok‡ | **+13**‡ | **+~1.6 MiB** |

† `inflate_all_chunks` (Rust probe; skips failed magics).  
‡ Python magic-scan oracle counting ok vs fail (see `chunk_oracle_matrix.json`).

Also: `bricks.rfa` `Partitions/0` — magic-scan fail 1→0 after strip.

## Formats/Latest on same corpus (weak / non-reproducing for 48% claim)

| File | Control inflate | Stripped inflate | Schema classes | Fields | class_names |
|---|---:|---:|---:|---:|---:|
| 2024_Core_Interior.rvt | 472,791 | 470,502 | 395=395 | 1114=1114 | 9579→8575 |
| Revit_IFC5_Einhoven.rvt | 464,741 | 462,765 | 405=405 | 1161=1161 | 10495→8474 |
| I_Single-Flush.rfa | 440,627 | 438,250 | 437=437 | 1282=1282 | 9688→8066 |

Structured schema metrics do **not** show a 48% loss under control. Inflate-length-only comparison is a **poor oracle** for Formats (strip slightly shortens output while partition chunk recovery improves).

## Synthetic fixture

`tests/checksum_page_framing.rs` injects 353-byte tails every 64,896 payload bytes into truncated-gzip buffers:

- Bare `inflate_at` does **not** round-trip the payload.
- `strip_revit_page_checksums` + `inflate_at` restores it exactly.
- Multi-member concat recovers expected member count only after strip.

## Prior maintainer note (2026-08-28) — updated interpretation

The 2026-08-28 probe correctly observed Formats length regression under naive strip and deferred default enablement. Wave 1 adds the **partition chunk oracle**, which independently confirms Steffen/reviter on redistributable data. Default wiring remains a Wave 2 change under the framing contract.

## Artifacts

- `/opt/cursor/artifacts/wave1_worker_c_decompress/chunk_oracle_matrix.json`
- `/opt/cursor/artifacts/wave1_worker_c_decompress/probe_runs.log`
- Contract draft: `docs/re/wave1-checksum-paged-framing-contract-2026-08-29.md`

## Confidence summary

- **High** that checksum paging is real and that strip-before-inflate is required for large paged partitions.
- **High** that current `main` callers are exposed on those streams.
- **Low** that the reporter’s Formats ~48% figure applies to our redistributable samples (different oracle / different file).
- **Medium** on exact fail-closed rules for short-page tails (retain-full matches reviter; ECC still unknown).
