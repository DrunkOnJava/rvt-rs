# RE-189 — An ElemTable record's first id names no project element where it differs from its second

**Date:** 2026-10-18
**Issues:** #760 (from #737)
**Result:**
- Where a 40-byte `Global/ElemTable` record's two ids differ, the second is the element's ElementId (RE-41) and the first is the id it was created with, which its UniqueId keeps (RE-48). No element record of the project names such a first id unless that id is also some other element's current id.
- On Snowdon Towers Architectural (84 records with two ids), 43 first ids are named only inside loaded families' own documents, past each partition's leading record chain (RE-135), and in `Global/ContentDocuments`; 41 are named nowhere. On Snowdon Towers Structural (27), 7 first ids are other elements' current ids and stay declared through them; of the other 20, 2 are named only in loaded families' documents and 18 nowhere.
- So `elem_table::declared_ids` now declares each record's current id only: the second on the 40-byte layout when it is not `0`, the first otherwise. That drops 84 ids on Architectural and 20 on Structural that named no element, and leaves every other file's declared set as it was.

| Measure run 39158195119 (`-f samples=true`), main against this change | main | RE-189 |
|---|---:|---:|
| Snowdon Architectural: declared ids | 47,317 | 47,233 |
| Snowdon Structural: declared ids | 16,591 | 16,571 |
| CI's 6 reference models: every output (IFC hash, GLB, scores) | — | byte-identical |
| The 49 samples: `rvt-info` output and export diagnostics, the fraction below aside | — | byte-identical |

-----

## 1. What names a first id

`probe_re189_first_id_references` searches, for every record whose two ids differ, every inflated `Partitions/*` stream in one pass for both ids, as `u64` and as `u32`. It names the chain record holding each hit (RE-35), or "past the chain" for a hit in a loaded family's document. It then searches every other stream but `Global/ElemTable` itself, `Global/Latest` among them, inflated where it is compressed. Measure run [39157096050](https://github.com/DrunkOnJava/rvt-rs/actions/runs/39157096050) (`-f samples=true`):

| | Snowdon Architectural | Snowdon Structural |
|---|---:|---:|
| Records whose ids differ | 84 | 27 |
| First ids that are another element's current id | 0 | 7 |
| Other first ids named in a project element record | 0 | 0 |
| ... named only in loaded families' documents | 43 | 2 |
| ... named nowhere | 41 | 18 |
| First ids named in another stream | 43, all in `Global/ContentDocuments` | 1, `Global/Latest`: 665875, another element's current id |

The 43 Architectural first ids named in family documents are exactly the 43 that `Global/ContentDocuments` names, so those ids belong to the loaded families' id spaces, which RE-135 already keeps out of the project's elements.

The other 28 files with 40-byte records that Measure reads (Revit 2024 to 2027: the other Autodesk sample projects and Snowdon models, Core Interior and the four RE1 models) have no record whose ids differ. Records of Revit 2023 and earlier are 28 bytes, and RE-41 found their two ids equal on every record.

## 2. Two facts left unexplained

- **Structural's pairs run in chains.** Its first ids are often the previous element's current id: 666709 -> 666710, 666710 -> 666711 and so on. That is why 7 of its 27 first ids are other elements' ids. What edit produces such a chain is not known.
- **Architectural's named first ids come in runs of consecutive ids** (2185195 to 2185198, 2185331 to 2185340, ...), listed as consecutive `u64`s in a family document. Their elements' current ids are a separate consecutive run (2220516 onwards), which looks like a block of elements copied from a family document into the project. That reading is not measured.

## 3. What changes

`declared_ids` has 43 call sites in 11 modules: ownership checks, chain records, export parameters, names and element records among them. Each asks whether an id is an element of this document. A first id that names no element could only make such a check pass for a number nothing else in the project uses, so dropping it fails closed. `revit_global_ids` reads both ids from the records themselves, so an element still gets the GlobalId its first id gives it.

The corpus target that first showed this (`partition_record_chain::every_first_prologue_frame_starts_a_record_of_its_own_id`, "47149 of 47233 declared ids have a record" on Architectural) runs only where Snowdon is in `RVT_PROJECT_CORPUS_DIR`, which CI's corpus is not: Snowdon reaches CI as a Measure sample, not as a corpus target.

## 4. End-to-end measurement

Measure run [39158195119](https://github.com/DrunkOnJava/rvt-rs/actions/runs/39158195119), `-f samples=true`, `main` (b2f415e) against this change (2dd2bb8). It read CI's 6 reference models with `rvt-ifc`, `rvt-gltf` and the scorers, and the 49 Autodesk sample projects and Snowdon models Measure pins (2016 to 2027) with `rvt-info` and `rvt-ifc`'s diagnostics, which give each export's entity counts by type, storeys and warnings.

- **The 6 reference models:** every output, the IFC hash and GLB among them, is byte-identical to main's.
- **The 49 samples:** every `rvt-info` output and every diagnostics file is identical, except two `diagnostics.json` files, the two Snowdon models with records whose ids differ. In each, `decoded_element_fraction` (decoded elements over declared ids) rises as its denominator loses the stray first ids: Architectural from 0.13303886 to 0.13327546 (6,295 of 47,317, then of 47,233), Structural from 0.08191682 to 0.08201569 (20 fewer declared ids).

The ids no longer declared changed nothing an export carries, as section 1 predicts: nothing in the project named them.

```bash
gh workflow run measure.yml -f ref=<branch> -f base=main -f samples=true
gh workflow run measure.yml -f ref=<branch> -f base=none -f samples=true -f probe=probe_re189_first_id_references
```
