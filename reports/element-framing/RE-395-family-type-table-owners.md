# RE-395: a family type table's entries in a `Family` element's block are no type's value

**Date:** 2026-11-09
**Issues:** #1185 (W7: type parameters); follows RE-354 (a family type table's Fire Rating and Manufacturer entries), RE-185 section 3.3 (a family type table's entries) and RE-77 (type text parameters)
**Artefacts:** `2024_Core_Interior.rvt` (MIT, `magnetar-io/revit-test-datasets`, pinned in `research/witness-registry.json`) with Revit's export of it; Autodesk's Snowdon Towers Sample Architectural, Revit 2024 (pinned in `research/autodesk-sample-projects.tsv`) and the Revit 2025 edition Autodesk ships with Revit 2025 (not pinned); the BIM4LCA office as built and at permit and its housing terrain file (Revit 2024, CC BY-SA 4.0, `research/bim4lca-measure.tsv`); and, local only, Golden Nugget Architektur und Ingenieurbau (Revit 2025)
**Probe:** `probes/examples/probe_re395_family_type_table_owners.rs`
**Status:** positive. Every owner whose table entries disagree, 6 on the 7 files probed, is a `Family` element, none a type. Leaving a `Family`'s entries out removes the RE-354 warning from 4 of 32 files and changes no IFC.

## 1. The question

RE-354 reads a `Fire Rating` (-1001206) or `Manufacturer` (-1010108) entry in the form of a family type table's (`u32 n · UTF-16 × n · flag · f64 · i64 -1 · i64 BuiltInParameter`) as the value of the element whose value block holds it. Where an element's entries give different texts, it writes neither and warns: "1 type text parameter(s) (Fire Rating or Manufacturer) are left out because the copies of a family type table's entry, or an entry and the id-first form, give different values (RE-354)". Core Interior gets this warning, and Revit's own export of the file has no fire rating at all. Which value was left out, and was it a value?

## 2. What the probe measures

`probe_re395_family_type_table_owners` runs RE-354's scanner (`scan_partition`) over every partition. For each element whose block holds a table entry, it prints the element's class, read from its record's class alone (`native_document` with no graph class decoded), what each partition's entries agree on, and whether the copies across partitions agree.

## 3. Core Interior

The one owner that disagrees is 17235, a `Family` (`m_name` `I_Single-Flush`), not a type. Its one block, in `Partitions/46`, holds three Fire Rating entries: `2HR`, `2HR` and `1HR`. Its five types (`FamilySymbol` with `m_familyId` 17235: 17331 `3 x 8 2 Hour`, 19270, 19271, 19272 and 132875) each hold one entry of their own in their own block, all `2HR`, and agree. The `Family` element's block holds its family type table (`m_pFamilyTypes`), so its entries are the table's rows, and rows for different types are expected to differ. Why the table has a `1HR` row that none of the five types holds was not traced.

The value left out was the family's, which no exporter writes as a type: neither rvt-rs's IFC nor Revit's has an object for 17235. The warning is false.

## 4. Elsewhere

| file | owners of table entries | of which `Family` | disagreeing | of which `Family` |
|---|---:|---:|---:|---:|
| Core Interior 2024 | 10 | 3 | 1 | 1 |
| Snowdon Towers Architectural 2024 | 48 | 1 | 1 | 1 (2159935) |
| Snowdon Towers Architectural 2025 | 48 | 1 | 1 | 1 (2159935) |
| Golden Nugget Architektur 2025 | 443 | 49 | 3 | 3 (2655864, 2660215, 2664563) |
| BIM4LCA office, as built | 45 | 19 | 0 | 0 |
| BIM4LCA office, at permit | 45 | 19 | 0 | 0 |
| BIM4LCA housing terrain | 25 | 10 | 0 | 0 |

Owners are counted per parameter. Every disagreeing owner is a `Family`. A `Family` whose rows agree (a family with one type, or with types that share a rating) gave its owner a value, which nothing wrote: a family is not exported as a type object.

## 5. The change

`type_text_parameters` reads the classes of the owners of table entries and leaves out those that are `Family` elements before the copies are checked against each other and against the id-first form. Where the classes cannot be read, every owner is kept, as before.

`rvt-ifc` at `main` (`bf54d5f4`) against the change, default, on 32 projects (Autodesk's sample projects of 2023 to 2027, its five Snowdon Towers 2024 models, Core Interior, Einhoven, the three BIM4LCA files and, local only, the seven Snowdon Towers 2025 models and the two Golden Nugget models): every IFC is the same (less `FILE_NAME` and `IfcOwnerHistory`), RE-354's six door types' `FireRating` and six family types' `Manufacturer` on the office among them. The warning goes from 1 to 0 on Core Interior and on Snowdon Towers Architectural 2024 and 2025, and from 3 to 0 on Golden Nugget Architektur; no file had it otherwise.

The class read adds no time that two runs each tell apart from the runs' own spread (Core Interior 7.7 s, Golden Nugget Architektur 50.2 to 51.7 s, Snowdon Towers Architectural 2025 44.5 to 46.6 s, the office 44.1 to 44.4 s, base and change alike).

## 6. Not shown

- Why `I_Single-Flush`'s table holds a `1HR` row that none of its five types holds.
- Whether a type's own block can hold more than one row of a table. No type on the seven files disagrees, so none was seen.

## 7. Reproduce

```
cargo run --profile ci -p rvt-probes --example probe_re395_family_type_table_owners -- 2024_Core_Interior.rvt
```
