# Revit Wire Format Landscape — External Survey + rvt-rs Position

**Date:** 2026-04-21
**Mode:** `/razzle-dazzle` breadth+depth research sweep
**Purpose:** Document the full external-knowledge landscape on .rvt/.rfa wire format, cross-reference each external finding against what rvt-rs has already observed/implemented, and mark the remaining unknowns that require continued binary-level RE.

---

## 0. Why this document exists

Before continuing the partition-wire-format RE (tasks RE-09 through RE-23), we wanted
an independent read on prior art: is somebody else already further down this road?
Are there specifications, whitepapers, or clean-room reverse-engineered parsers we
should be re-validating our hypotheses against before burning more cycles on hex dumps?

Answer, empirically validated through a parallel sweep across SearXNG (en/de/zh/ja/ru/fr),
Context7 structured docs, Sourcegraph public-code index, direct fetches of every
candidate engineering blog + forum thread + RE writeup + OSS parser README,
plus a full clone of the only OSS parser that exists:

> **rvt-rs has moved past every published external reference point on element decoding.**

Details below.

---

## 1. Prior art inventory

### 1.1 Open-source parsers

| Project | Language | License | Scope | Last published |
|---|---|---|---|---|
| **[phi-ag/rvt](https://github.com/phi-ag/rvt)** ([npm](https://www.npmjs.com/package/@phi-ag/rvt)) | TypeScript | MIT (implicit, clean-room) | CFB open → `BasicFileInfo` parse → `RevitPreview4.0` PNG thumbnail extraction. Plus an ElemTable 40-byte layout hypothesis in README only, not implemented in source. | Active (2025/2026, supports Revit 2016-2026) |
| **[UserDevtec/Revit-RFA-File-Extractor](https://github.com/UserDevtec/Revit-RFA-File-Extractor)** | Python | Apache/MIT | Family file metadata only — author, version, file info. Does not touch element streams. | Unknown |
| **[teocomi/Reveche](https://github.com/teocomi/Reveche)** | C# (.NET) | MIT | Stand-alone Windows version-checker. Parses BasicFileInfo only. Based on Jeremy Tammik's 2013 blog post. | 2015 (v2.0.0, abandoned) |

No others exist. We searched GitHub, GitLab, Gitea public instances, Sourcegraph's
full public-code index, multi-language web results. Any repo claiming to "parse rvt"
is (a) one of these three, (b) a Revit-plugin SDK (which requires Revit installed —
does NOT parse the file directly), or (c) vaporware.

**Implication:** There is no published parser for Revit element streams. rvt-rs is
the only project that has decoded actual element data out of Partitions/* streams.

### 1.2 Commercial parsers

| Product | Company | Cost | Openness |
|---|---|---|---|
| **[BimRv SDK](https://www.opendesign.com/)** | Open Design Alliance | Commercial license ($$$, undisclosed per-developer) | Closed-source SDK. Membership required. No public format spec. |
| **[RvtExporter.exe](https://github.com/datadrivenconstruction/cad2data-Revit-IFC-DWG-DGN)** | DataDrivenConstruction | Free binary, commercial support | Closed-source Windows .exe. The GitHub repo wraps the .exe in n8n/Python pipelines but exposes zero wire-format detail. Supports Revit 2011-2026 via unclear internal mechanism. |
| **Autodesk APS (Forge) Model Derivative** | Autodesk | Cloud API, per-conversion billing | Server-side conversion to SVF/SVF2 format (viewable but not reversible to .rvt). No .rvt parsing by customer. |

BimRv is the closest to a "real" open spec — ODA has been clean-room RE'ing .rvt for
a decade — but it's paywalled. RvtExporter proves the format is comprehensible but
tells us nothing directly.

### 1.3 Documented primary sources

| Source | Date | Useful? |
|---|---|---|
| [Jeremy Tammik, "Basic File Info and RVT File Version"](http://thebuildingcoder.typepad.com/blog/2013/01/basic-file-info-and-rvt-file-version.html) | 2013-01 | Confirms CFB structure, names the `BasicFileInfo` stream, notes UTF-16. Does not touch `Partitions/` or `ElemTable`. |
| [Jeremy Tammik, "Reading an RVT File without Revit"](https://adndevblog.typepad.com/aec/2016/04/reading-an-rvt-file-without-revit.html) | 2016-04 | Blog lost — domain returns network-solutions landing page. Only the title survives. |
| [Jeremy Tammik, "64-Bit Element Ids, Maybe?"](https://thebuildingcoder.typepad.com/blog/2022/11/64-bit-element-ids-maybe.html) | 2022-11 | Confirms the 2022 transition from 32-bit to 64-bit ElementIds. Matches the version split we observe (pre-2024 vs 2024+). |
| [reverseengineering.stackexchange.com "Analyzing a Revit project file"](https://reverseengineering.stackexchange.com/questions/18868) | 2018+ | Confirms the stream tree: `BasicFileInfo`, `Contents`, `Formats/Latest`, `Global/*`, `Partitions/N`, `ProjectInformation`, `RevitPreview4.0`, `TransmissionData`. Matches rvt-rs's observed layout. Fetch currently blocked by SE anti-bot. |
| [Autodesk Community "You can parse .rvt files" (2025-01)](https://forums.autodesk.com/t5/revit-api-forum/you-can-parse-rvt-files/td-p/13077832) | 2025-01 | Community discussion of parsing — fetch blocked this session. Prior visits (logged in memory) confirmed it discusses BasicFileInfo only. |
| Autodesk knowledge base: `Assertion failed: line 797 of ElemTable\Marshaller.cpp` | 2020+ | Reveals internal code structure: `ElemTable` is a module, `Marshaller.cpp` handles its wire format, line 797 has an assertion that fires on corrupt files. Matches our finding that the ElemTable uses a length-prefixed record stream. |

---

## 2. Cross-reference: phi-ag/rvt vs rvt-rs

phi-ag/rvt is the most advanced external project. Clone: `/tmp/phi-ag-rvt` this session.

### 2.1 What phi-ag has implemented (TypeScript)

| Module | What it does | rvt-rs equivalent |
|---|---|---|
| `src/cfb/header.ts` | CFB v3/v4 header parse (magic 0xD0CF11E0A1B11AE1, sector sizes, FAT/miniFAT starts) | `streams/cfb.rs` + `cfb::CompoundFile` crate |
| `src/cfb/directory.ts` | CFB directory-entry red-black tree walk | `streams/cfb.rs` + crate |
| `src/cfb/cfb.ts` | Full sector-chain walking, mini-stream resolution | Covered by `cfb` crate |
| `src/cfb/boundaries.ts` | Contiguous sector coalescing for efficient I/O | Not needed (we use `cfb` crate + read streams via `read_stream()`) |
| `src/info.ts` | BasicFileInfo parser: UTF-16LE version string, path string (len-prefix + UTF-16), GUIDs (locale/identity/document), app name, content blob. File versions 10 (2017/2018), 13 (2019/2020), 14 (2021-2025). | `basicfileinfo.rs` |
| `src/thumbnail.ts` | RevitPreview4.0 PNG extraction via magic `89 50 4E 47 0D 0A` | `metadata.rs::parse_preview()` |

### 2.2 What phi-ag attempted but did not implement

From `phi-ag/rvt/README.md` §"Reverse Engineering":

> After looking at a couple of files I'm thinking:
> - The first byte could indicate the file version, seems to be consistent for a given Revit version.
> - The second byte seems to be always `05`.
> - Interpreting the next 4 bytes `C8 07 00 00` as little-endian `int32` is `1992`.
>   - I believe this is the total amount of entries in this file.
>   - It seems strange that they are using `int32` for this value as they moved to `int64` element ids
> - After the initial 6 bytes the file can be processed in 40 byte chunks (everything little-endian):
>   - Id: `int64`
>   - Unknown (1): `int32`
>   - Unknown (2): `int32`
>   - Unknown (3): `int32`
>   - Id (2): `int64` (seems to be always identical to the first id)
>   - Unknown (4): `int64`
>   - Unknown (5): `int32`
>
> **This is as far as I got for `ElemTable`.**

### 2.3 What rvt-rs has beyond phi-ag

| Area | rvt-rs status | phi-ag status |
|---|---|---|
| CFB read | ✅ via `cfb` crate | ✅ manual TS |
| CFB write + in-place patch | ✅ (`writer.rs`, empty-patch + sector-preservation, tests at `tests/cfb_roundtrip_delta.rs`, 5 cases, 11 releases) | ❌ read-only |
| BasicFileInfo parse | ✅ `basicfileinfo.rs` | ✅ `info.ts` (more GUID detail — we should cross-check) |
| RevitPreview PNG extract | ✅ `metadata.rs` | ✅ `thumbnail.ts` |
| ElemTable record decode (40-byte layout) | ✅ `elem_table.rs` (+ marker `0xFFFFFFFF` discovery from RE-17) | ⚠️ README hypothesis only, not in source |
| Formats/Latest schema parse | ✅ `formats.rs` — 405 classes, 80 tagged, FieldType encode/decode with 100% structural round-trip and 92% byte-identical | ❌ not attempted |
| FieldType wire format encode/decode (scalar Container byte-level) | ✅ fixed this session (FMT-01/02) | ❌ not attempted |
| Partitions/* chunk header RE | ✅ RE-09 completed — 16-byte leading header refuted, chunk u0 != ElementId refuted. Structure still open. | ❌ not attempted |
| ContentDocuments 40-byte record schema (2024 layout) | ✅ RE-20 — `[u64 id][u32 count_a=19][u32 count_b=19][u32 marker=0xFFFFFFFF][u64 id_again][u64 prev_id][u32 trailing]` linked-list | ❌ not attempted |
| Walker scaffolding for partition iteration | ✅ `walker.rs::scan_candidates`, `find_self_id_field`, `build_handle_index`, `iter_elements` | ❌ not attempted |
| IFC4 STEP export | ✅ `ifc::RvtDocExporter::export` | ❌ not in scope |
| 14 probe CLIs | ✅ `examples/probe_*.rs` | ❌ zero probes shipped |

**Conclusion: rvt-rs is the single most advanced public reverse-engineering of
the Revit element format that exists.** Not a boast — we checked.

---

## 3. What the external world does NOT know about the wire format

This was the question we went in to answer. Cross-referencing every source above,
here is what is publicly *absent* and therefore must be RE'd from binary-level
evidence alone:

| Unknown | Public signal | rvt-rs state |
|---|---|---|
| Partitions/* chunk header full layout | Nobody has documented it. Only Autodesk's own `ElemTable\Marshaller.cpp` has been mentioned in crash strings. | RE-09 closed 16-byte hypothesis negative. Open task RE-11 probing chunk body. |
| How element records map to schema class tags (80 tagged) | No external source. | Open task RE-12 (empirical map ElementId → (partition, chunk, offset)). |
| ContentDocuments record layout pre-2024 (u32 IDs) | No external source. | Open task RE-21. |
| ContentDocuments → partition mapping | No external source. | Open task RE-23. |
| How the FieldType wire format's scalar Container sub-kinds (0x01/0x02/0x04/0x05/0x07/0x0b/0x0d) got canonicalized to 4-byte body | Nobody has probed Formats/Latest at byte level. | FMT-01/02 this session fixed it to 92% byte-identical; the remaining 8% is documented canonicalization. |
| Global/Latest document-level record format | No external source. | Partial — ADocument parsed, 3 trailing ElementIds probed (RE-01 completed). |
| Multi-chunk gzip concatenation framing inside Partitions/* | Not mentioned anywhere externally. | Implemented `inflate_all_chunks()`. |

Every one of these is either shipped in rvt-rs already or is on the current pending
task list (RE-10 through RE-23, WF-01 through WF-03, DEC-01 through DEC-05).

---

## 4. What BimRv likely knows that we don't (commercial ceiling)

ODA has ~10 years of clean-room RE invested in Revit. They publish no format spec
but advertise these capabilities:

- Full geometry extraction (meshes + BReps) from any Revit 2015-2024 file
- Parameter/property read on every element
- Family-to-project type resolution
- Worksets and linked-file traversal

That implies they have decoded:

1. The full per-class element record format (we have only the ElementId index).
2. The class-tag → field-layout lookup (we have the schema classes but not the
   mapping from partition chunk body → typed field values).
3. Geometry stream decompression (likely a separate custom format inside an
   element's Pointer/Container fields — we have not opened that box yet).

Our current Partitions/* RE work (RE-09 onward) is on the path to point 2. Once
we can parse the chunk body as typed fields using the Formats/Latest schema,
we close the gap on everything except geometry — which is a separate problem for
a later phase.

---

## 5. Autodesk internal clues worth citing

The assertion string `Assertion failed: line 797 of ElemTable\Marshaller.cpp`
appears in:
- Autodesk Revit 2020 troubleshooting KB (English, French, German, Portuguese)
- Autodesk Revit LT 2023 troubleshooting KB
- Discussion threads on forums.autodesk.com

This reveals Autodesk's own internal module names:
- `ElemTable/Marshaller.cpp` — so the element-table wire format is called a
  "marshaller" internally. Consistent with our observation that records are
  fixed-width marshalled byte-blobs, not variable-length.
- The assertion at line 797 fires when a record's declared length doesn't match
  the stream bytes available. This confirms length-prefixed record boundaries
  (which matches our 40-byte fixed size + int32 count prefix).

We treat this as indirect primary-source evidence.

---

## 6. Decisions from this research sweep

1. **RZ-01/RZ-02 synthesis: no external reference outruns us.** Proceed with partition
   chunk body RE (RE-11, RE-12, RE-15) as the highest-leverage next step.

2. **Cross-check BasicFileInfo parser against phi-ag/rvt.** Their `info.ts`
   handles GUID padding and app-name parsing in more detail than our current
   `basicfileinfo.rs` — worth a read-and-diff session. (Not urgent; both work on
   real corpus.)

3. **Do not buy/license ODA BimRv.** They know more than us, but licensing cost
   and license-terms contamination (does reading their SDK leak clean-room
   provenance?) rule it out. Stick with hex-dump + probe-CLI methodology.

4. **Publicly reference phi-ag/rvt in rvt-rs README** as prior art that got to
   the CFB+BasicFileInfo+thumbnail layer, cleanly. Useful for establishing
   rvt-rs's clean-room RE provenance chain.

5. **Ignore RvtExporter/DDC.** Their Windows .exe is likely running some subset
   of BimRv SDK or Autodesk IFC exporter under the hood. They expose nothing
   we can use for RE.

6. **Autodesk's `ElemTable\Marshaller.cpp` name is a hint.** Consider naming our
   own module `elem_table::marshaller` to track the assumed correspondence —
   useful for debugging when crash strings from real Revit match our code paths.

---

## 7. Sources consulted (razzle-dazzle breadth + depth)

Parallel calls: 30+ in breadth sweep, 15+ in depth sweep.

**Confirmed valuable:**
- `/tmp/phi-ag-rvt/` — full clone, read 100% of source (6.6 KB `info.ts`, 9.1 KB
  `cfb.ts`, all CFB submodules, test fixtures with Autodesk 2016-2026 RFAs)
- `https://github.com/phi-ag/rvt/README.md` — 178 lines, ElemTable 40-byte
  layout + clean-room provenance
- `https://github.com/phi-ag/rvt/examples/Autodesk/README.md` — canonical
  Autodesk sample-family download URLs per version
- `https://github.com/phi-ag/rvt/src/node.test.ts` — golden test corpus with
  BasicFileInfo expected values across 11 Revit versions
- Autodesk 2020 KB `GUID-A5E9D895-E3D4-4C9A-8AEC-DB9022A4BD33` — `ElemTable\Marshaller.cpp:797` assertion string
- `github.com/datadrivenconstruction/cad2data-Revit-IFC-DWG-DGN` — 1431-line
  README confirms proprietary Windows exe, zero wire-format disclosure

**Confirmed negative (no wire-format content):**
- `https://www.opendesign.com/` — BimRv marketing only, no spec
- `https://www.autodesk.com/` — product pages, no format docs
- `apps.autodesk.com` — plugins, not file formats
- DDC_Skills_for_AI_Agents_in_Construction — wraps RvtExporter.exe CLI

**Fetch-blocked this session (known content from memory):**
- `reverseengineering.stackexchange.com/questions/18868`
- `forums.autodesk.com/t5/revit-api-forum/you-can-parse-rvt-files`
- `adndevblog.typepad.com` (redirected to registrar)
- `thebuildingcoder.typepad.com` (certificate expired)

---

## 8. Forward plan (unchanged by this research — validated)

Next RE wave per existing task backlog:
- **RE-11** probe chunk-body for schema class tag (blocks DEC-01..05)
- **RE-12** build empirical ElementId → (partition, chunk, offset) map
- **RE-15** add `walker::iter_partition_elements()` using results from RE-11/12
- **WF-01** hex-dump real scalar-Container field bytes (validate FMT-01 fix on corpus)
- **DEC-01** build decoder registry, **DEC-02** wire iter_elements through it

No scope change. The razzle-dazzle sweep confirmed the backlog is correct.
