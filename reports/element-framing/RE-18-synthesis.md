# RE-18 synthesis — tagged-ancestor walker ships, but hypothesis H7 refuted by data

**Date:** 2026-04-21
**Scope:** Build `SchemaTable::tagged_ancestor()` + `tagged_ancestor_map()` API, validate against real corpus, test the load-bearing hypothesis H7 from `RE-09-synthesis.md`.

## TL;DR

Shipped: the ancestor-walker API (6 unit tests passing, probe CLI running clean).
**H7 refuted:** on actual corpus, the tagged-ancestor approach does not resolve the
classes we care about.

## H7 refuted — the empirical picture

Hypothesis H7 from RE-09 (confidence 0.7): *"Tagless classes (Wall, Floor, Door)
use their parent-in-schema-tree tag on the wire. Walk class `parent` chain in the
schema to find the first ancestor with a tag."*

Running `probe_tagged_ancestors` against Einhoven 2023 + 2024 Core Interior +
2024/2026 sample RFAs:

| File | Classes | Directly tagged | Untagged resolved via ancestor | Unresolvable |
|---|---:|---:|---:|---:|
| Revit_IFC5_Einhoven.rvt (2023) | 405 | 80 | **0** | 325 |
| 2024_Core_Interior.rvt | 395 | 79 | **0** | 316 |
| racbasicsamplefamily-2024.rfa | 395 | 79 | **0** | 316 |
| racbasicsamplefamily-2026.rfa | 349 | 60 | **0** | 289 |

Zero untagged classes resolve to a tagged ancestor — across 4 corpus files.

### Two reasons why

1. **The class-name literals we were looking for do not exist in the schema.**
   `Wall`, `Floor`, `Door`, `Window`, `Level`, `Grid`, `Column`, `Beam`, `Roof`,
   `Ceiling`, `FamilyInstance`, `Room`, `HostObject` — every one of these returns
   "not in schema" from both real .rvt files. Only `HostObjAttr` (tag 0x006b)
   and `Element` (tagless, parent-less) are present under the names we used.
2. **The parser isn't extracting `parent` links for most untagged classes.** Of
   the 325 untagged classes on Einhoven, every one has `parent.is_none()` —
   the probe logged 0 entries in the hop-count distribution, meaning there
   was nothing to walk.

Possible parent-link shortfall causes:
- The schema parser in `formats.rs` only records `parent` when a tagged class
  is followed by a `[u16 pad=0][u16 parent_name_len][parent_name]` block AND
  the `[u16 flag][u32 fc][u32 fc_dup]` preamble validates. That guard fires
  for tagged classes; tagless ones fall through the `raw_tag & 0x8000`
  branch entirely and never reach parent detection.
- Tagless classes may have their parent stored in a different record format
  that the current parser doesn't recognize. Or they may have no parent in
  the on-disk schema at all (mixins / embedded primitives like `ElementId`,
  `Identifier`, `AppInfo` genuinely have no parent — they're fundamental
  types).
- The 325 "untagged" classes include `ACDPtrWrapper`, `A3PartySECImage`,
  `ImportVocabulary`, `ADTGridTextLocation` — these look like utility /
  mixin / interop types that are embedded in other records as value
  wrappers, not top-level serializable entities. They were never candidates
  for a Wall-to-HostObjAttr chain.

### Implication for H7 itself

The premise of H7 was that `Wall` lives in the schema under its generic name
with a parent chain pointing to some tagged class. The data says the schema
uses concrete subtype names (`ArcWall` 0x0191, `VWall` 0x0192, `WallCGDriver`
0x0197, etc.) directly, not a `Wall` abstract parent with `ArcWall` as a
subtype. On the wire, a partition chunk representing an arc wall carries
tag 0x0191 — no ancestor walk needed; we just look up 0x0191 directly.

**New posture (H7'):** Classes are carried on the wire by their own concrete
tag. There is no need to walk ancestry to find the tag for `Wall` because
`Wall` is not a thing on the wire — `ArcWall`, `VWall`, etc. are.

This simplifies the tag-scan approach: the target is the 80 tagged classes
themselves. RE-11's original scan (which already included every tagged class,
not just interesting ones) is the correct methodology — the ancestor step
adds no value.

## Verified facts (new)

- **F16** — On 4 corpus files (Einhoven 2023, 2024 Core Interior, 2024 RFA,
  2026 RFA) the schema parser extracts `parent` for essentially zero
  untagged classes. Either parent data isn't present in the stream for
  them, or our parser isn't picking it up. Either way, the ancestor-walk
  approach does not materially expand the searchable tag set.
- **F17** — Class-name literals `Wall`, `Floor`, `Door`, `Window`, `Stair`,
  `Column`, `Beam`, `Roof`, `Ceiling`, `Level`, `Grid`, `FamilyInstance`,
  `Room` are **not in the schema** under those exact names in any corpus
  file. Only `HostObjAttr` (direct tag 0x006b) and `Element` (untagged,
  parent-less) exist under the names we expected. Source: probe output.
- **F18** — Across all 4 corpus files, `HostObjAttr` is the only "generic
  host container" tag, and it sits alone at the top of concrete-element
  hierarchies without a visible subtype chain in the parsed schema.

## Code shipped

- `SchemaTable::tagged_ancestor(&self, class_name: &str) -> Option<(&str, u16)>`
- `SchemaTable::tagged_ancestor_map(&self) -> BTreeMap<String, (String, u16)>`
- 6 unit tests covering: direct-tag, walked chain, no-tag-in-chain, cycle
  guard, unknown class, aggregate map
- `examples/probe_tagged_ancestors.rs` — 180-line CLI that produces the
  empirical table above
- This synthesis doc

The API is *correct* — the unit tests prove the walker works on synthesized
schema data. It just doesn't *help* against real corpus because the parent
data isn't there to walk. Keeping the API because:

1. Callers asking "does class X have a tag either directly or via ancestor?"
   get the right answer on both synthesized + real data.
2. If the schema parser improves to extract more parent links (likely
   follow-up work), this API's results will silently improve.
3. The `tagged_ancestor_map()` doubles as a sanity check for the schema
   parser — it shows what chain coverage we have.

## Decisions

**D6** — Keep the API. Mark the hypothesis that motivated it (H7) as refuted
on this corpus. Move RE-11 execution back to its original plan: scan partition
chunks for the 80 concrete tagged classes directly, not for their abstract
ancestors.

**D7** — Open follow-up: investigate why 325 untagged classes have no `parent`.
Is the schema genuinely silent on their ancestry, or does the parser skip a
branch that contains the parent data? Lower priority than finishing RE-11/15
because H7's refutation means ancestors are not load-bearing for element
location.

**D8** — Do not add a more complex "mixin / aggregation / component graph"
walker until we have evidence that elements carry mixin-tags on the wire.
The 80-tag set is the right place to start; we can extend later if scanning
reveals tags outside that set showing up in the right positions.

## Open questions

**Q8** — Of the 80 tagged classes, which ones are concrete "element"
instances (wall-like, floor-like, door-like) vs which are internal
machinery (transaction headers, index records, version stamps)? The list
from RE-09 (tag 0x0191 ArcWall, 0x0192 VWall, 0x0197 WallCGDriver, etc.)
is a start but we need to enumerate all 80 and decide which subset RE-11
should treat as "interesting."

**Q9** — For Revit 2024 where u0 in partition chunks was quasi-monotonic
(1810, 64397, 81127, 81549, …), does that quasi-monotonic value correspond
to one of the 80 tagged classes' tag values? RE-19 correlated u0 with
ElemTable IDs (negative). A fresh correlation with the 80-tag set is
worth a pass.

## Recommended next steps

1. **RE-11 proper** — scan partition chunks for all 80 tagged classes in
   their u16 LE value form. Use `tagged_ancestor_map()` output to
   additionally resolve any hits back to class names. If a chunk
   consistently begins with a specific tag from the 80, that's the
   element envelope marker.
2. **RE-11.5** — run the same scan with tag positions offset by 0, 2, 4,
   8 bytes (in case there's a length prefix or a fixed preamble before
   the tag). The RE-09 chunk-header probe was negative on 16-byte
   hypothesis but a variable-length preamble is still possible.
3. **Parent-link parser fix (defer)** — if and only if RE-11 returns
   negative, revisit whether the schema parser is dropping parent data.

Coverage of current schema data as documented by this probe:

```
Revit_IFC5_Einhoven.rvt     80/405 classes resolvable (19.8%)
2024_Core_Interior.rvt      79/395 classes resolvable (20.0%)
racbasicsamplefamily-2024   79/395 classes resolvable (20.0%)
racbasicsamplefamily-2026   60/349 classes resolvable (17.2%)
```

That ~20% is the 80 tagged classes. No ancestor walk expands it. This
is the ceiling until the parser catches more parent data — if it exists.
