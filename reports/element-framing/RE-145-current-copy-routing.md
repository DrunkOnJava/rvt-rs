# RE-145: where the native path takes each element's current record

**Date:** 2026-10-03
**Issues:** #548. puzzbobb reported this (#548, issuecomment-5939524953 and 5957284268); RE-143 is the earlier result.
**Artefacts:** Core Interior (2024), Einhoven (2023), the four RE1 models (2025), and Autodesk's 36 sample projects of 2016 to 2027. Measured on GitHub's hosted runners with the Measure workflow, runs 37116666890 (all files) and 37117317288 (the six models, with the elements listed).
**Probe:** `examples/probe_re145_current_copy_routing.rs`.
**Status:** positive. The route picks the latest partition's copy where copies differ. It sends some elements to partitions holding none of their records, which #565 fixed.

## 1. The question

`native_document::extract` routes every declared element through `Global/DocumentIncrementTable` by its stored revision (`native_index::route_episode`). It emits only the record in the partition the route gives, and skips the others as historical. #548 asks which copy is current when two partitions hold different records of one element. puzzbobb reported two things:
- the route picks the newer copy on 2021 `rac_advanced` (887 of 887) and on two files we do not hold;
- on 2025 files, some elements with one record route to a partition that holds none.

## 2. Method

For every declared element with a channel-101 record outside a `ContentKey` block, the probe computes the routed partition as `native_document` does and compares it with the partitions that hold the element's records:
- for single copies: whether the route lands on their partition;
- for repeated ids: whether the copies are byte-identical, and if not, whether the route takes the latest partition's copy, the earliest's, or another's;
- for every element: whether the routed partition holds any of its records.

On releases the native path admits (2023, 2024, 2027), it also runs `native_document::extract` itself on channel 101.

## 3. Result

| file | partitions | declared | repeated ids | differing | route takes the latest copy | route takes another | routed to a partition with none of its records | `extract` emitted |
|---|---:|---:|---:|---:|---:|---:|---:|---|
| Core Interior | 8 | 26,425 | 1,340 | 571 | **569** | 2 | **10** (to `Partitions/48`) | 26,415 of 26,425 |
| Einhoven | 7 | 2,615 | 14 | 6 | **5** | 1 | **6** (to `Partitions/1`) | 2,609 of 2,615 |
| RE1 Architecture | 3 | 4,668 | 4 | 3 | **3** | 0 | 0 | not admitted (2025) |
| RE1 Electrical | 5 | 10,266 | 109 | 29 | **28** | 1 | 1 | not admitted |
| RE1 Mechanical | 2 | 4,743 | 32 | 10 | **10** | 0 | 0 | not admitted |
| RE1 Plumbing | 2 | 4,827 | 31 | 14 | **14** | 0 | 0 | not admitted |
| 2021 `rac_advanced` | 2 | 16,946 | 4,037 | 887 | **887** | 0 | 0 | not admitted |
| 2019 `rac_basic`, `rst_basic` | 2 each | 7,965, 13,810 | 51, 36 | 1, 1 | **1, 1** | 0 | 0 | not admitted |
| the other 2019 to 2027 samples | 1 each | | 0 | 0 | | | 0 | 2023, 2024, 2027: all emitted |

The 2016 to 2018 samples are not walked. 2016 and 2017 lack the `SignatureMarker` class, and 2018 hits the schema stop fixed in #561 (B03, B06).

- **Where copies differ, the route takes the latest partition's copy.** That holds on 1,518 of 1,522 differing ids. On 2021 `rac_advanced` it is 887 of 887, which matches RE-143's dating and puzzbobb's count. Of the other 4:
  - two on Core Interior route to a middle partition of three that holds a copy;
  - Einhoven's 2846 and one element of RE1 Electrical route to a partition that holds neither copy.
- **The route can land on a partition that holds none of the element's records.**
  - Einhoven: the 6 elements are 2846 (copies in `Partitions/0` and `3`) and 5973, 5974, 5976, 5978 and 5979 (one copy each, in `Partitions/2`). All have stored revision 1 and route to `Partitions/1`.
  - Core Interior: 10 elements (20951 and 64158 to 64681) with identical copies in `Partitions/46` and `51` and stored revision 31 route to `Partitions/48`.
  - The native path emitted none of these 16, although each has a record. Those are exactly the elements `extract` was missing.
- **Single copies routed away:** Einhoven's 5, and none on Core Interior, RE1 or the samples. puzzbobb's 2025 counts are on files we do not hold (Golden Nugget, Snowdon Towers).

## 4. What this changed

- #565: when the routed partition holds none of an element's records, the native path takes the copy in the latest partition that holds one. The summary counts these in `current_records_outside_route`. The real-file target `tests/native_current_records.rs` requires one emitted record per declared element on Einhoven and Core Interior.
- #548's question is answered for every file measured. The current copy is the routed partition's, which is the latest partition's wherever the copies differ, and the latest holding partition's when the route finds none.

## 5. What this does not claim

- **Why the route lands on an empty partition** (stored revisions 1 and 31 here) is not established.
- **The two middle-partition routes on Core Interior** were not dated against neighbouring releases.
- **puzzbobb's modification-history comparison** (the copies' own change logs) was not reproduced here.
