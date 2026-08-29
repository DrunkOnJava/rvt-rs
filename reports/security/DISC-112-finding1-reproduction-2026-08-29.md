# Discussion #112 / issue #151 — Finding 1 (Wave 2 narrowed production)

Date: 2026-08-29  
Credit: reported by [@STE1200](https://github.com/STE1200) / Steffen's team in
[Discussion #112](https://github.com/DrunkOnJava/rvt-rs/discussions/112).  
Judge: Wave 1 independent verdict **narrow**.

## Reported vs independently reproduced

| Claim (reported) | Independent result |
|---|---|
| Full page = 64,896 payload + 353 trailer | **Confirmed** |
| Drift / silent success without strip | **Confirmed** (synthetic + partition member recovery) |
| ~48% Formats schema loss | **Not reproduced** on redistributable corpus; naive strip regresses `class_names` (e.g. 9579→8575) while SchemaTable stays flat |
| Affects streams ≳190 KiB | **Narrowed** — tracks full-page / multi-member desync |

## Production contract (narrowed)

Order: CFB stored → page-strip **when gated** → truncated-gzip / DEFLATE.

- **Gated strip:** `Partitions/*`, listed `Global/*` DB streams via
  `inflate_stream_*` / `inflate_all_chunks_for_stream`.
- **Not gated by default:** `Formats/Latest` (and metadata / previews).
- **Never strip inside** raw `inflate_at` / `inflate_all_chunks`.
- **`read_stream`** remains stored-byte accurate.
- **Writer verify** uses bare `inflate_at` (writer emits strip-clean gzip until
  a paged encoder exists).

## Evidence highlights

### Synthetic

`synthetic_multipage_partitions_require_strip_before_inflate` and
`tests/checksum_page_framing.rs`: bare inflate ≠ payload; gated Partitions /
Global strip == payload; Formats prepare is identity.

### Redistributable corpus (Wave 1 judge / Worker C)

`2024_Core_Interior.rvt` `Partitions/46`:
control `inflate_all_chunks` **925** ok → strip **935** ok (+814 893 bytes).

`Formats/Latest` on same file: SchemaTable **395/1114** both ways;
`class_names` **9579→8575** after naive strip — reason Formats stays ungated.

## Confidence

**High** for partition / Global gated strip-before-inflate.  
**Low** for reporter Formats ~48% on our samples — deliberately not claimed.

## Follow-ons

#152–#156 deferred. Paged writer encoder coordinated with Worker D.
