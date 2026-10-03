# RE-148: a keyed block belongs to the loaded family whose GUID keys it

**Date:** 2026-10-03
**Issues:** #421, RE-135's open question. puzzbobb reported this (#421, issuecomment-5956962887).
**Artefacts:**
- Autodesk's sample projects of 2019 to 2027 (`research/autodesk-sample-projects.tsv`);
- the eleven Autodesk families of 2016 to 2026;
- Core Interior (2024), Einhoven (2023) and the four RE1 models (2025).

Measured on GitHub's hosted runners with the Measure workflow, run 37118135955.
**Probe:** `examples/probe_re148_content_keys.rs`; `native_segments::GroupSource` now carries the keyed `ContentMarker`'s count.
**Status:** positive, with one exception on RE1 Electrical, described below.

## 1. The question

RE-135 found that the records after a project's chain are the loaded families' own documents. It did not establish which family a record belongs to. puzzbobb reported two things:
- the 16-byte key of the `ContentMarker` before a block of partition groups is the GUID of one `Global/ContentDocuments` section;
- the marker's `m_nElementCount` is the block's channel-101 record count less the one record whose id is −1.

## 2. Method

`native_segments::walk` already returns each group's `content_key`. It now also returns the count of the keyed `ContentMarker` before the group (`content_element_count`). For every distinct key the probe counts the block's channel-101 records and those with id −1, and compares the stated count with their difference. It also searches the inflated `Global/ContentDocuments` bytes for each key: every gzip member, after the checksum-page strip. A byte search shows the key is in the stream, not which field holds it.

## 3. Result

| files | release | blocks | keys found in `ContentDocuments` | stated count = records − 1 | records with id −1 per block |
|---|---|---:|---:|---:|---|
| `rac_basic` samples | 2019 to 2027 | 163 each | **163** | **163** | 1 on every block |
| `rst_basic` samples | 2019 to 2027 | 52 each | **52** | **52** | 1 |
| `rac_advanced` samples | 2019 to 2027 | 121 each | **121** | **121** | 1 |
| Core Interior | 2024 | 46 | 46 | 46 | 1 |
| Einhoven | 2023 | 6 | 6 | 6 | 1 |
| RE1 Architecture, Mechanical, Plumbing | 2025 | 77, 40, 39 | all | all | 1 |
| RE1 Electrical | 2025 | 288 | **228** | 288 | 1 |
| families | 2019 to 2026 | 0 | | | |

- **puzzbobb's counts reproduce exactly on every sample project** from 2019 to 2027: 163, 52 and 121 keys, all in `Global/ContentDocuments`. On every block the stated count is the channel-101 records less the one record with id −1.
- **The count rule holds on every block of every file:** 3,520 blocks in all, 496 of them on the six reference models.
- **RE1 Electrical's 60 keys** that the byte search does not find in `Global/ContentDocuments` are the one exception. Their counts agree like the others. Where those 60 documents are listed was not examined.
- **The families have no keyed blocks.** Their `Global/ContentDocuments` inflates to 14 bytes.

So a record after a project's chain belongs to the loaded family whose GUID keys its block. That answers RE-135's open question on every file measured except RE1 Electrical's 60 blocks.

## 4. What this does not claim

- **The section structure of `Global/ContentDocuments` was not decoded.** The key is found in the stream by search, not read as a section's GUID field.
- **2016 to 2018 were not walked** in this run. 2016 and 2017 lack the `SignatureMarker` class, and 2018 hits the schema stop fixed in #561.
