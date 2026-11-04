# RE-323 — The residual Formats/Latest field encodings, and what Revit 2027's flagged fields hold

**Date:** 2026-11-03
**Issues:** #1196 (classify the residual field encodings and the 2027 flagged fields from byte evidence); builds on RE-264 (the flagged encodings), #410 (the whole-schema gate) and #1101
**Artefacts:** the `phi-ag/rvt` family corpus at bec25470413fbe659c3ab30d4cfd029b73d63fc3 (`examples/Autodesk/`), sha256:

| File | sha256 |
|---|---|
| `rac_basic_sample_family-2016.rfa` | fde80383b9ba0f2ee620a35bb0e938f1be3af858044c596ff1af00431e5e9601 |
| `rac_basic_sample_family-2017.rfa` | fce912633160e782c79ba1d6065c9d2f13ede646cd2b22379de5acfc697f36ea |
| `rac_basic_sample_family-2018.rfa` | 301fd82d5ee48158015e36a87246f32477f6c1f8d078f7e8acccfc5c1f89688f |
| `rac_basic_sample_family-2019.rfa` | 163824383cce459c1af5c6365602e90003498caf71ece85b8b739b171d1beaeb |
| `racbasicsamplefamily-2020.rfa` | a16624fa9e2a376d45c0c54f38cc9159d85ed5f317ac9369f917fdf294c5b629 |
| `racbasicsamplefamily-2021.rfa` | acbf8b0532f0ff3513a3bc2705630a1cf46ceb2667e2f6e28bb0861ae5369907 |
| `racbasicsamplefamily-2022.rfa` | e6373e7632a738bee3f39a1dc8a43aad4ef5eef26cf2d681f005434a9939befa |
| `racbasicsamplefamily-2023.rfa` | 1cdff09b3fcf6aa574059c679a0ca5bf7595e4e22f44ad8109131a49473bc6bf |
| `racbasicsamplefamily-2024.rfa` | fe9ffd26cec4fa660b1d65a0b629a93ac7a994225746377e693c315d26c0aa5e |
| `racbasicsamplefamily-2025.rfa` | 609b370aaa6bbb55a390cfd5833a6005177aaf0f8143ce3d568b12d835b72e51 |
| `racbasicsamplefamily-2026.rfa` | 00425e9d0766c3d520df344189375970c5219bd5f48e58b399484132fdb41e8d |
| `racbasicsamplefamily-2027.rfa` | 2df5452e0a468559b99abb3cb89791c7283bb8fd286e36709c762b7912fd84f4 |

and the maker family `stertil-closed-pit-sp-xp-pp-2014.rfa` (Revit 2014, `research/public-families.tsv`, sha256 b3b46ded5fc26bfa9e554442f089b51e6331f2fe89c86188b5ed41ec5a98de0c), and Autodesk's 58 sample projects of Revit 2015 to 2027 that Measure fetched (`research/autodesk-sample-projects.tsv` holds each file's sha256): 6, 3, 3, 3, 6, 3, 3, 3, 3, 10, 5, 5 and 5 projects of 2015, 2016, 2017, 2018, 2019, 2020, 2021, 2022, 2023, 2024, 2025, 2026 and 2027.
**Runs:** Measure 41754444445 (families), 41754940721 (samples 2015 to 2020), 41754944550 (2021 to 2026), 41754936505 (2027), all of probe `94541d10` (the classification as it was before this change); 41756271989 and 41756275764 (§5, two readings of the one unread modifier, 2015 to 2017); 41757339031, 41757342653, 41757346003, 41757349044 (the same probe on the head, §6); 41757335670 (identity, §8).
**Result:** positive for 11 of the 14 residual names, measured negative for 3. Revit 2027's 339 flagged fields occupy no instance bytes on every instance measured.

## 1. Two readings of the schema

`formats::parse_schema` scans for class names and files each field under the last class it saw. `schema_registry::parse` (the forward-only grammar of `schema_classes`) reads the same stream exactly: a class is its name, base, version and fields, and a class defined inline inside another's base or field list ends where its own fields end. The probe reads each file both ways and decodes the 32 bytes at every grammar field's descriptor with `FieldType::decode`, which is what the scan does. They agree on the bytes and differ in the class a field is filed under wherever an inline definition interrupts the outer class's field list (the three `m_pFace`, `m_next` and `m_prev` arrays are the same bytes in the same order under either name):

| Scan's name (`RESIDUAL`) | The class the grammar says declares it | Descriptor bytes |
|---|---|---|
| `GInfo::m_pFace`, `m_next`, `m_prev` | `GEdge` (base `GEdgeBase`) | `0e 13 00 00 02 00 00 00` |
| `VarFunction::m_refCt` | `VarExpr` | `04 00 04 00` |
| `VarSketchObj::m_params` | `VarSketchObj` (same); `VarFunction::m_args` has `0e 54` too and no scan entry | `0e 54 00 00` |
| `VarSketchObj::m_refPts` | `VarSketchDimObj` (base `VarSketchConstrObj`) | `0e 54 00 00` |
| `VarSketchObj::m_constrElems` | `VarSketchConstrObj` (base `VarSketchObj`) | `0e 53 00 00` |
| `VarSketchObj::m_dimValue` | `VarSketchDimObj` | `0e 04 00 00` |
| `VarParam::m_pSubst` | `VarParam` (same) | `0e 04 00 00` |
| `LFormatDefaults::m_unit`, `m_defaultMetricFormatOptions`, `m_defaultImperialFormatOptions` | same | `0e 53 00 00` |
| `ClassDefinitionRef::m_ref` | same | `0a 00 00 00` |
| `ZoningAppInfo::first` | `std::pair< SymbolCategoryPair, int >` | `0e 00 00 00 8e 10` (a class reference), cut to `0e 00` by the scan (§3) |

The fields are in these releases' schemas (grammar names; `*` is the 2014 maker family): `GEdge` x3, `VarExpr::m_refCt`, `VarFunction::m_args`, `VarSketchDimObj` x2, `VarSketchConstrObj::m_constrElems`, `VarSketchObj::m_params` and `ClassDefinitionRef::m_ref` in 2014* and every release 2016 to 2027; `VarParam::m_pSubst` in 2014*, 2016 and 2017; `LFormatDefaults` x3 in 2019 and 2020 only. That is 14 grammar fields with an undecoded descriptor, and one more the scan leaves undecoded for another reason (§3). The scan names 13 of the 14 (it has no entry for `VarFunction::m_args`) and the clipped one: 14 names.

## 2. Per-field evidence

The instance reader is the native graph decoder (`native_document::extract_graphs`): a record's body decodes only if the whole of it is consumed, so a field read with a wrong width or layout fails the record. The family files cannot be read by it (`invalid increment episode range`), so the instances come from Autodesk's sample projects, 2015 to 2027. For each residual field the probe holds every instance to the layout below: the value has the shape the layout gives, and a pointer token is null or the token of an object in the same graph.

Records decoded, by release, over all the projects (`complete` is a record whose body the graph consumed to its last byte):

| Release | Projects | Records | Complete | Not complete |
|---|---:|---:|---:|---:|
| 2015 | 6 | 121,147 | 119,527 | 1,620 |
| 2016 | 3 | 39,363 | 39,040 | 323 |
| 2017 | 3 | 39,473 | 39,150 | 323 |
| 2018 | 3 | 39,668 | 39,658 | 10 |
| 2019 | 6 | 109,087 | 106,783 | 2,304 |
| 2020 | 3 | 39,101 | 39,091 | 10 |
| 2021 | 3 | 38,951 | 38,948 | 3 |
| 2022 | 3 | 39,208 | 39,205 | 3 |
| 2023 | 3 | 39,205 | 39,202 | 3 |
| 2024 | 10 | 218,367 | 218,246 | 121 |
| 2025 | 5 | 96,842 | 96,837 | 5 |
| 2026 | 5 | 97,189 | 97,184 | 5 |
| 2027 | 5 | 97,005 | 97,000 | 5 |

The records that are not complete fail on an entity-schema catalog the project does not hold (`ESEntityCell`, 2015 to 2020), a size budget (`ARasterImage` 124, `TriangulationDriverMemento`, `GeomGenerator`, a table style: 2021 to 2027), `AreaLoad.m_planeNorm` (a non-finite double, 2015 to 2020, 5 records) or `VarParam.m_pSubst` (1,125 records of the 2015 to 2017 projects, §5); none fails at a field this report classifies.

| Field (grammar name) | Layout tested | Instances | Where | Layout failures, unresolved pointers | Verdict |
|---|---|---:|---|---|---|
| `GEdge::m_pFace`, `m_next`, `m_prev` | array of 2 `u32` tokens, no class tag | 111,985 each (2,861 records) | 57 of 58 projects, 2015 to 2027 (13 releases) | 0, 0 (`m_pFace`: 35,582 null tokens, 188,388 resolved of 223,970; the other two 39,207 and 184,763) | `FieldType::PointerArray { modifier: 0x13, count: 2 }` |
| `VarExpr::m_refCt` | `i32`, 4 bytes | 138,477 (4,820 records) | 46 projects, 2018 to 2027 (10 releases) | 0, - (the value is 1 in every instance) | `Flagged { flags: 4, base: Primitive { kind: 4, size: 4 } }` |
| `VarSketchConstrObj::m_constrElems` | `u32` count, then that many `u32` tokens, no class tag | 58,876 (4,652 records), 1 to 4 items | same 46 projects | 0, 0 (91,055 tokens, all resolved) | `PointerContainer { modifier: 0x53 }` |
| `VarSketchObj::m_params` | `u32` count, then per item a `u32` token and, where it is not 0, a `u16` class tag | 93,092 (4,820 records), 0 to 7 items | same 46 projects | 0, 0 (138,477 items, all to class `VarParam`) | `PointerContainer { modifier: 0x54 }` |
| `LFormatDefaults::m_unit` and 2 more | (as `0e 53`) | none | 2019, 2020: no project instance | - | `PointerContainer { modifier: 0x53 }` by its header alone |
| `VarFunction::m_args`, `VarSketchDimObj::m_refPts` | (as `0e 54`) | none | - | - | `PointerContainer { modifier: 0x54 }` by its header alone |
| `ClassDefinitionRef::m_ref` | - | none | - | - | still `Unknown` |
| `VarParam::m_pSubst` | - | 31,705 | 12 projects, 2015 to 2017 | see §5 | still `Unknown` |
| `VarSketchDimObj::m_dimValue` | - | none | - | - | still `Unknown` |

Two checks that do not depend on the layout code: the 138,477 items of `VarSketchObj::m_params` equal the 138,477 instances of `VarExpr::m_refCt` (the base class of `VarParam`) in every one of the 46 projects, so every parameter is owned by exactly one sketch; and the number of empty `m_params` (58,876) equals the number of `VarSketchConstrObj::m_constrElems` instances (58,876), the class that derives from `VarSketchObj`.

A header-only classification is a claim about the declared type, not about instance bytes: the two `0e 54` and the three `0e 53` fields are classified because every other field with the same header has instances that decode, and the report says so rather than counting them as measured. `ClassDefinitionRef::m_ref`'s header (`0a`) is shared with no other field, and nothing reads an instance of it, so it stays `Unknown`.

## 3. The scan's window

`ZoningAppInfo::first` (2026) is `Unknown` with two bytes, `0e 00`. The grammar's field is `std::pair< SymbolCategoryPair, int >::first`, descriptor offset 491768 in the inflated stream, bytes `0e 00 00 00 8e 10 06 00 00 00 73 65`: a by-value class reference to tag 0x108e (4238), which `FieldType::decode` reads as `ElementIdRef`. The scan files it under `ZoningAppInfo`, whose name starts at 487659; the scan stops a class's fields 4,096 bytes after the end of its name header (487659 + 2 + 13 + 4096 = 491770) and cut the window there, 2 bytes after the field name. The same cut gives the 2014 maker family `SymbolIdMgr::m_defAssemblyTypeId` (class at 330231, 2 + 11 + 4096 = 334340, descriptor at 334338, bytes `0e 00 00 00 14 00`: an `ElementId`). The scan now reads the 32 bytes from the stream, not from the class's window; the gate lists no scan residual that is a clip any more.

## 4. Revit 2027's flagged fields

The grammar lists 339 fields with bit 1 of the descriptor's flags word set in the 2027 family: 315 with flags 0x0002, 15 with 0x0012 and 9 with 0x0022 (RE-264 counted 301, 15 and 9 through the scan, which lists 325 of them and misses the 14 the grammar adds, ten field names among them: `BasePoint::m_oldInternalPosition`, `CircuitGroupAppInfo::m_arrSystemElemIds`, `SketchEditMgr::m_lastUserId`, `ViewportInfo::m_pDocument`, ...). By header they are `bool` 79, `u32` 45 (`04 00` 44, `05 00` 1), `u32` containers (`04 50`) 3, `f64` 7, `vector<f64>` 8, string 7, `u64` 1, class references (`0e 00`) 64, pointers (`0e 01` to `0e 03`) 42 and pointer containers (`0e 50`, `0e 51`) 83.

The native decoder skips every field whose flags have bit 1 set (the rule that was written for `VWall`), reading no bytes. Five 2027 projects hold 97,005 records; 97,000 of them decode completely, so with every flagged field skipped the bodies end at their last byte, and the 5 that do not (a raster image's compressed bytes, a site surface's triangulation, a stair's edge history: size budgets) are not at a flagged field. 110 flagged fields, of 51 classes (`DBView` 25, `AssemblyTracker` 9, `ViewDisplayMgr` 5, `RvtLinkSymbol` 5, `Viewer` 4, `VWall` 3, ...), have the class instantiated in those records: 103 with flags 0x0002, 4 with 0x0012, 3 with 0x0022; by header `bool` 40, pointers (`0e 01` to `0e 03`) 21, pointer containers (`0e 50`, `0e 51`) 15, `u32` 13, `u32` containers 2, class references 11, `vector<f64>` 6, string 2. Across the five projects the 110 fields' classes are instantiated 1,165,791 times. If a flagged field held bytes in an instance, skipping it would shift every field after it; none shows that. The other 229 fields' classes occur in no record of the five projects, and the family corpus cannot be read by the native decoder. The schemas of the 53 sample projects of 2015 to 2026 carry no descriptor with bit 1 set (0 fields in each); all five 2027 projects list the same 339.

So the flagged fields are zero-width on every instance measured, and a base type does not apply to them: `Flagged`'s `base` is the type the header names, not a value stored. The walker keeps reading them as `Unknown`.

## 5. The modifier `0e 04`

The only residual header no instance in the corpus fits is `0e 04 00 00`: `VarParam::m_pSubst` (2014*, 2016, 2017) and `VarSketchDimObj::m_dimValue` (every release). The native decoder had no reader for modifier 4, so every `VarSketch` record holding a `VarParam` of the 2015 to 2017 projects failed (1,125 records in 12 projects). To read them two layouts were tried on branches (`refs/archive/2026-11-03/re323-modifier4-owning`: a pointer as modifier 1 and 2 are, token and a class tag where the token is not 0; `refs/archive/2026-11-03/re323-modifier4-external`: modifier 3's, a token alone). Both decode all 1,125 records and change nothing else (Measure 41756271989 and 41756275764: of the 199,983 records of the 12 projects, 198,842 are complete against 197,717 before; the 1,141 left are the entity-schema, size-budget and non-finite cases above). They agree because every one of the 31,705 `m_pSubst` instances (19,311 in 2015, 6,197 in 2016, 6,197 in 2017) is a null token: a pointer with a token of 0 has no class tag in either layout. So the null case is measured and the non-null one is not, and the bytes do not choose between the layouts. The field stays `Unknown`; `VarSketchDimObj::m_dimValue` has no instance in any file at all. Neither branch is merged.

## 6. What changed

- `FieldType::PointerArray { modifier, count }` and `FieldType::PointerContainer { modifier }` are appended to the non-exhaustive enum; `decode` reads `0e 13 00 00 <count>` and `0e 53`/`0e 54` as them, and `encode` writes the same bytes back. `Flagged` accepts byte 2 = 0x04. The scan decodes from the stream, not from the class's 4,096-byte window.
- The walker reads the new variants as `Unknown` (its record shapes were not measured), so no walk changes.
- `docs/schemas/schema-table.schema.json` has the two new `FieldType` branches.
- `tests/field_type_coverage.rs`: `RESIDUAL` is `ClassDefinitionRef::m_ref`, `VarParam::m_pSubst` and `VarSketchObj::m_dimValue` (the scan's name for `VarSketchDimObj::m_dimValue`), down from 14 names. The test also reads each schema with `schema_registry` and holds the fields the exact grammar leaves `Unknown` to a second list under the declaring classes (`RESIDUAL_GRAMMAR`), and fails on an entry of either list that no longer decodes `Unknown`.
- After (Measure 41757339031, the probe on the head): every family of 2016 to 2027 and the 2014 maker family has 2 `Unknown` fields in the scan and in the grammar, 3 in 2016, 2017 and the 2014 family with `VarParam::m_pSubst`, and no clipped window.

The family corpus gate is otherwise unchanged at 100% of the remainder.

## 7. Reproduce

The classification before this change is the commit `94541d10` (the probe on `main` at 8da11dab), kept at `refs/archive/2026-11-03/re323-probe-before`:

```sh
gh workflow run measure.yml --ref main -f ref=94541d1057362c12be931f073ce96d168a5959c9 -f base=none \
  -f families=true -f models=none -f scorers=none -f probe=probe_re323_residual_field_encodings -f probe_scorer=-
gh workflow run measure.yml --ref main -f ref=94541d1057362c12be931f073ce96d168a5959c9 -f base=none \
  -f samples=true -f sample_years="2015 2020" -f models=none -f scorers=none \
  -f probe=probe_re323_residual_field_encodings -f probe_scorer=-
# the same with sample_years="2021 2026" and sample_years=2027
```

The probe prints, per file, the residual fields' grammar names, descriptors and 32-byte windows (`residual`), the scan's unknowns (`old-unknown`) and clipped windows (`clipped`), the descriptor shapes (`shape`), and each residual field's instances with their layout verdict (`instance`), and for 2027 the flagged fields whose classes occur (`flagged-instances`, `flagged-total`). On the head, the same command with `-f ref=<head sha>` lists only the three remaining residual fields.

## 8. Identity

Measure 41757335670, `-f base=main -f nordic=true -f r2025=true -f samples=true -f families=true -f scorers=none`, head 1a8427d5: *results below*.
