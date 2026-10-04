# RE-163: a keyed block is listed when the family at the top of its chain is declared

**Date:** 2026-10-04
**Issues:** #421; backlog item B48
**Artefacts:** the six reference models (Core Interior, Einhoven, the four RE1 models) and Autodesk's 36 sample projects of 2016 to 2027. Measured on GitHub's hosted runners with the Measure workflow, run 37209219989.
**Probe:** `examples/probe_re163_content_documents.rs`.
**Status:** positive. puzzbobb's rule (#421, 2026-10-03) holds both ways on all 42 files.

## 1. The question

RE-148 found that every keyed block's 16-byte key occurs in `Global/ContentDocuments`, except 60 of RE1 Electrical's 288. puzzbobb proposed why on #421:

- a loaded family is a `Family` element of the project plus its own document, embedded as a keyed block;
- a `Family` record names that block by its key;
- the documents of families nested in it are named by `Family` records inside its block;
- a block is listed exactly when the `Family` at the top of that chain is an id `Global/ElemTable` declares.

RE1 Electrical's 60 blocks are three copies of a deleted `Bell Wall` family and the families nested in them.

## 2. Method

The probe is written from that statement, with rvt-rs's own walkers. It reads every partition with `native_segments::walk`, splits each group on channels 101 to 103 into records, and keeps every record whose class tag is `Family`, with the block it sits in. For each block key it lists the `Family` records whose body holds the key's 16 bytes. It then follows the first of them up through the blocks to the first `Family` outside every block, and checks that id against the declared ids. "Listed" is RE-148's test: the key's bytes occur in the inflated `ContentDocuments` stream.

## 3. Result

| file | blocks | listed, top `Family` declared | unlisted, top `Family` undeclared | other |
|---|---:|---:|---:|---:|
| RE1 Electrical (2025) | 288 | 228 | 60 | 0 |
| RE1 Architecture, Mechanical, Plumbing | 77, 40, 39 | all | 0 | 0 |
| Core Interior (2024) | 46 | 46 | 0 | 0 |
| Einhoven (2023) | 6 | 6 | 0 | 0 |
| `rac_basic`, 2016 to 2027 | 163 each | all | 0 | 0 |
| `rac_advanced`, 2016 to 2027 | 121 each | all | 0 | 0 |
| `rst_basic`, 2016 to 2027 | 52 each (54 in 2016) | all | 0 | 0 |

- **Every block's key is held by a `Family` record** on every file, so no block has an unknown owner. On the samples, 3 keys are held by two or three records, on RE1 Electrical 2: copies of one `Family` in several partitions, as puzzbobb noted.
- **No block is listed under an undeclared family, and none is left out under a declared one.** No group failed to split into records, and no chain failed to reach a `Family` outside every block.
- **RE1 Electrical's 60** go up to three undeclared `Family` elements, 937132, 950798 and 957600, the ids puzzbobb gives. The largest of these blocks holds 1,079 channel-101 records, and their nested blocks 241 to 252 each.

## 4. What this changes

A reader that reports a project's loaded families from its keyed blocks must take only the blocks `ContentDocuments` lists. An unlisted block belongs to a family the project deleted, and reading it would report three `Bell Wall`s and their 57 nested families on RE1 Electrical.

rvt-rs reads no keyed block today: `native_document` skips every group with a content key (`src/native_document.rs:412`), so nothing it writes includes a deleted family, and no code changes here.

## 5. Reproduce

```text
gh workflow run measure.yml --ref re/re163-content-documents -f base=none \
  -f probe=probe_re163_content_documents -f samples=true
```
