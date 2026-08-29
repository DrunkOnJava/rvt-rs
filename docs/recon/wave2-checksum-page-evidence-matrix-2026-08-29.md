# Wave 2 evidence matrix — stream-evidence harness (#151)

Date: 2026-08-29
Harness: `stream-evidence` (#158)
Credit: [@STE1200](https://github.com/STE1200)
Judge: **narrow** — SchemaTable / member-ok preferred; do not claim Formats ~48%.
Artifacts: `/opt/cursor/artifacts/wave2_evidence_matrix/`

## Corpus provenance

| Sample | sha256 | Provenance |
|---|---|---|
| `2024_Core_Interior.rvt` | `c805df445d613b408e37337765572021265e3f5dfdc7d1fa53b22ba1600b8014` | magnetar-io MIT |
| `Revit_IFC5_Einhoven.rvt` | `d3a0c6d37d3f47a1726bc5aa7fe3880ed3c13bbe819b5e64680f6710b15aa948` | magnetar-io MIT |
| `empty.rfa` | `5f194a9a70a2ec1490bbdc61da03f63884da97e073917b6f096c1a198d8afd90` | in-repo fixtures |
| `architectural-2024.rvt` | `3f50ce849ff022eb1c28ebfd91fb450d4a36af72fd19105d93caffb861e9560b` | in-repo tier1 synthetic |

## Key streams (control = bare inflate, experimental = page-strip then inflate)

| Sample | Stream | Stored | Pages | Chunks ctrl→exp | Bytes ctrl→exp | Δ chunks | First-member equal | Schema classes/fields |
|---|---|---:|---:|---|---|---:|---|---|
| `2024_Core_Interior.rvt` | `Formats/Latest` | 173,230 | 2+42732 | 1→1 | 472,791→470,502 | 0 | False | 395→395 / 1114→1114 (names 9579→8575) |
| `2024_Core_Interior.rvt` | `Global/ContentDocuments` | 243,084 | 3+47337 | 1→1 | 1,463,023→1,456,498 | 0 | False | — |
| `2024_Core_Interior.rvt` | `Global/ElemTable` | 125,892 | 1+60643 | 1→1 | 1,059,812→1,057,030 | 0 | False | — |
| `2024_Core_Interior.rvt` | `Global/Latest` | 78,810 | 1+13561 | 1→1 | 1,014,963→1,008,506 | 0 | False | — |
| `2024_Core_Interior.rvt` | `Partitions/46` | 17,072,209 | 261+42220 | 925→935 | 97,957,200→98,772,093 | 10 | True | — |
| `2024_Core_Interior.rvt` | `Partitions/48` | 2,388,625 | 36+39661 | 138→139 | 17,094,328→17,142,729 | 1 | True | — |
| `2024_Core_Interior.rvt` | `Partitions/51` | 598,771 | 9+11530 | 28→28 | 3,334,591→3,315,208 | 0 | True | — |
| `2024_Core_Interior.rvt` | `Partitions/53` | 5,692,783 | 87+16120 | 246→251 | 31,588,102→32,170,195 | 5 | True | — |
| `2024_Core_Interior.rvt` | `Partitions/55` | 2,695,935 | 41+20726 | 103→103 | 13,140,081→13,074,355 | 0 | True | — |
| `2024_Core_Interior.rvt` | `Partitions/59` | 1,598,473 | 24+32497 | 75→76 | 9,162,283→9,239,919 | 1 | True | — |
| `2024_Core_Interior.rvt` | `Partitions/61` | 2,809,648 | 43+3941 | 102→106 | 12,924,846→13,440,880 | 4 | True | — |
| `2024_Core_Interior.rvt` | `Partitions/65` | 94,163 | 1+28914 | 6→6 | 444,843→443,218 | 0 | True | — |
| `Revit_IFC5_Einhoven.rvt` | `Formats/Latest` | 169,647 | 2+39149 | 1→1 | 464,741→462,765 | 0 | False | 405→405 / 1161→1161 (names 10495→8474) |
| `Revit_IFC5_Einhoven.rvt` | `Global/ElemTable` | 10,709 | 0+10709 | 1→1 | 73,245→73,245 | 0 | True | — |
| `Revit_IFC5_Einhoven.rvt` | `Global/Latest` | 64,737 | 0+64737 | 1→1 | 884,529→884,529 | 0 | True | — |
| `Revit_IFC5_Einhoven.rvt` | `Partitions/0` | 464,357 | 7+7614 | 33→33 | 3,015,590→3,002,973 | 0 | True | — |
| `Revit_IFC5_Einhoven.rvt` | `Partitions/5` | 100,304 | 1+35055 | 10→10 | 587,060→584,425 | 0 | True | — |
| `architectural-2024.rvt` | `Formats/Latest` | 122 | 0+122 | 1→1 | 453→453 | 0 | True | 6→6 / 17→17 (names 6→6) |
| `architectural-2024.rvt` | `Global/Latest` | 309 | 0+309 | 1→1 | 901→901 | 0 | True | — |
| `empty.rfa` | `Formats/Latest` | 100,816 | 1+35567 | 1→1 | 335,736→334,630 | 0 | False | 652→652 / 1835→1835 (names 7622→6952) |
| `empty.rfa` | `Global/ElemTable` | 11,505 | 0+11505 | 1→1 | 80,049→80,049 | 0 | True | — |
| `empty.rfa` | `Global/Latest` | 16,250 | 0+16250 | 1→1 | 35,944→35,944 | 0 | True | — |
| `empty.rfa` | `Partitions/62` | 220,567 | 3+24820 | 12→12 | 1,482,555→1,475,107 | 0 | True | — |

## Acceptance callouts (narrow contract)

- **Partitions/46** (`2024_Core_Interior.rvt`): chunks **925→935** (Δ10), inflated bytes **97,957,200→98,772,093** (Δ814,893). Matches judge/`inflate_all_chunks` directional oracle.
- **Formats/Latest** Core Interior: first-member equal=False, length_delta=-2289, pages=2. Structured schema in parser field (see JSON); do **not** treat class_names as success.
  Parser control keys snapshot: `{'kind': 'formats_schema', 'ok': True, 'class_count': 395, 'field_count': 1114, 'skipped_records': 0, 'class_name_count': 9579, 'error': None, 'failure_offset': None}`
- **tier1 synthetics**: below one full page → strip is identity (no regression).
- Production strip helper matches harness experimental strip: see `production_strip_matches_experimental` in JSON.

## Raw reports

- `core_interior_all_paged.json` (233327 bytes)
- `core_interior_formats.json` (4849 bytes)
- `core_interior_global_latest.json` (4105 bytes)
- `core_interior_partitions_46.json` (104042 bytes)
- `einhoven_all_paged.json` (47319 bytes)
- `einhoven_formats.json` (4854 bytes)
- `empty_rfa_all_paged.json` (26565 bytes)
- `tier1_architectural_all_paged.json` (7143 bytes)
