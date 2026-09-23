# RE-48 — Revit's own IFC GlobalIds, rebuilt from the file

**Date:** 2026-09-23
**Result:**
- **Positive.** The GlobalId Revit's IFC exporter gives an element is rebuilt from the file. It is the GUID of the element's episode, from `Global/History` via the episode number in its `Global/ElemTable` record, with the element's first ElementId XORed into the last 32 bits, in IFC's 22-character base-64 form.
- rvt-rs now writes it in place of its own positional GlobalIds. Every element rvt-rs exports that Revit's export also holds has Revit's GlobalId:
  - Core Interior: 854 of 854;
  - RE1 Architecture, Mechanical, Plumbing and Electrical: 73, 73, 123 and 12;
  - `teste_export_2025`: 8; Projeto1: 2;
  - Snowdon Towers: 5,945 of 5,945.
- Spaces and storeys follow the same rule. Core Interior's 116 spaces and 15 storeys, and RE1 Architecture's 11 spaces, now carry Revit's GlobalIds. A storey takes its Level's GlobalId through the Level's name, which Revit keeps unique.
- No export has a duplicate GlobalId.
- An rvt-rs IFC file and Revit's IFC of the same model now name the same objects the same way. Diffs line up, and BCF issues and federation links written against one resolve in the other.

**Probe:** `examples/probe_re48_revit_global_ids.rs` prints each element's rebuilt GlobalId and, given Revit's export, compares them per `Tag`.

**Oracles:**
- `2024_Core_Interior.rvt` with `2024_Core_Interior_slim.ifc`.
- The RE1 models (Revit 2025, MIT) with their exports.
- Snowdon Towers Sample Architectural (local only) with Revit's IFC4 export.
- `teste_export_2025` and Projeto1 (local only) with their exports.

-----

## 1. The rule

Revit's UniqueId is `<episode GUID>-<ElementId in 8 hex digits>`. Its IFC exporter derives a GlobalId from it by XORing the ElementId into the GUID's last 32 bits, which is public prior work on the Revit API. What the file had to yield is the episode GUID of every element.

### `Global/History`

| offset | content |
|---|---|
| `+0x0e` | `u32` N, the number of episodes |
| `+0x16` | the document GUID, repeated |
| `+0x66` | `u32` M, then M `u32` values (not read) |
| then | `u32` N again, then N entries of a 16-byte GUID (Windows byte order) and the byte `0x28` |

The list runs newest first. Core has 39 episodes, RE1 Architecture 703, Snowdon 2,226. The first `u16` is `0x04ff` on the 2024 files and `0x051d` on the 2025 files. The layout after it is the same.

### `Global/ElemTable`

A 40-byte record (Revit 2024 and later) holds its element's episode number at `+0x18`. The element's episode GUID is entry `N − 1 − episode`. On Snowdon, 362 distinct episode numbers each map to exactly one list entry.

### The ElementId

The id XORed in is the record's first id. An element whose ElementId has since changed, which is a record with a second id (RE-41), keeps its original id in its UniqueId. On Snowdon all 37 such elements match Revit's GlobalId only with the first id.

## 2. Measured

Per `Tag`, rvt-rs's GlobalId against Revit's, with the element's own entity compared and not the parts Revit synthesises under it:

| file | release | same | different | rvt-rs only |
|---|---|---:|---:|---:|
| Core Interior | 2024 | 854 | 0 | 0 |
| RE1 Architecture | 2025 | 73 | 0 | 1 |
| RE1 Mechanical | 2025 | 73 | 0 | 0 |
| RE1 Plumbing | 2025 | 123 | 0 | 1 |
| RE1 Electrical | 2025 | 12 | 0 | 0 |
| `teste_export_2025` | 2025 | 8 | 0 | 0 |
| Projeto1 | 2025 | 2 | 0 | 0 |
| Snowdon Towers Architectural | 2024 | 5,945 | 0 | 100 |

Snowdon's two ramps are written by Revit as an `IfcRamp` and, first under the same `Tag`, a synthesised `IfcRampFlight` part. rvt-rs's `IfcRamp` has the GlobalId of Revit's `IfcRamp`.

"rvt-rs only" counts elements Revit's export leaves out (#309). Their GlobalIds are rebuilt the same way.

Revit also exports parts it synthesises: stair components on Snowdon, a ramp's flight, and multi-piece slabs past their first piece. Those get GlobalIds of Revit's own making, which the rule does not give. rvt-rs does not write them. For the further pieces of a sketched slab (#331), which share their element's `Tag`, only the first piece gets the rebuilt GlobalId, so none is written twice.

Types: of the 7 type GlobalIds on RE1 Architecture, 5 follow the same rule. rvt-rs writes no type objects with GlobalIds, so this is not used.

The committed Core witness observations are unchanged (they count entities), and so is the synthetic fixture, whose ElemTable is not gzip-framed.

### Spaces and storeys

Rooms are exported as element records, so they take the same rule. Neither entity has a `Tag`, so they are compared by GlobalId:

| file | spaces with Revit's GlobalId | storeys with Revit's GlobalId |
|---|---:|---:|
| Core Interior | 116 of 116 | 15 of 15 |
| RE1 Architecture | 11 of 11 | 0 (rvt-rs writes 4 storeys, Revit 2) |
| Snowdon Towers | (Revit's export has none) | 0 of 17 |

Revit's storey GlobalIds follow the rule with the Level's ElementId on all 15 Core storeys and all 18 Snowdon storeys. rvt-rs gives a storey its Level's GlobalId when the storey's name is exactly one recovered Level's.

On Snowdon, the Level names are not recovered, and the storeys are named by elevation ("Elevation -16.917 ft"), so they keep generated GlobalIds. That is a gap in Level recovery, not in this rule.

## 3. What this does not do

- Revit 2023 and earlier (28-byte ElemTable records) are not read, and rvt-rs's positional GlobalIds stay (Einhoven gives none).
- The project, site, building and relationship entities keep generated GlobalIds. Revit's project, site and building GlobalIds are derived another way.
