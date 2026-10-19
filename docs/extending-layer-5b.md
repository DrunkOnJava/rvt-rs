# Extending Layer 5b — adding a new element decoder

This guide walks through adding a decoder for a Revit element class
(Wall, Floor, Door, Window, Column, etc.) to `rvt-rs`. Layer 5a is
already done (ADocument walker); Layer 5b is "per-class decoders
that produce typed element values from the object graph."

**Time required per class**: 30–90 minutes for an experienced Rust
programmer plus access to the `phi-ag/rvt` corpus via `_corpus/`
(fetched by CI with LFS; can be cloned locally the same way).

**Prerequisites**:

- Read `src/walker.rs` to understand `InstanceField` + `decode_instance`
- Read `src/elements/level.rs` — it's the reference example this
  guide walks through
- Clone the corpus: `git clone --recurse-submodules
  https://github.com/phi-ag/rvt _corpus` in the repo root
- `cargo test --release --test integration -- field_type_coverage::` passes (sanity
  check that your corpus is good)

---

## The three-file change

Adding a decoder for `MyClass` touches these files:

```
src/elements/my_class.rs                    # decoder impl + typed view
src/elements/mod.rs                         # register the decoder, and route production through it
src/elements/typed_json.rs                  # its typed JSON for the CLI and Python
tests/python/test_decoded_api.py            # the MVP class list it pins
docs/schemas/decoded-elements.schema.json   # the typed projection's class list
tests/myclass_corpus.rs                     # the corpus check (Step 5), with its line in
tests/integration/main.rs                   #   the one test binary
tools/ci/test-targets.txt                   #   and its class in the CI manifest
```

### Step 1 — Check the schema

Before writing any code, confirm the class exists in Revit's
schema and inspect its fields:

```bash
cargo run --release --bin rvt-schema -- _corpus/examples/Autodesk/racbasicsamplefamily-2024.rfa \
    | grep -A 20 "class MyClass"
```

Output lists the fields in schema order with their decoded
`FieldType`. Record the field list — you'll use it for the
`synth_schema` helper in the tests.

If `MyClass` doesn't appear, that's useful: it means the class is
out of scope for family files and you may need a project `.rvt` in
the corpus. Check the discovery scripts under `tools/`.

### Step 2 — Create the decoder file

Copy `src/elements/level.rs` as the starting template. Rename
everything (`LevelDecoder` → `MyClassDecoder`, `Level` → `MyClass`,
`"Level"` → `"MyClass"`). Shape:

```rust
//! MyClass — one-line description of the Revit class.
//!
//! | Field | Type | Semantics |
//! |---|---|---|
//! | m_name | String | display name |
//! | m_foo  | Primitive u32 | some value |
//! | ... | ... | ... |

use crate::formats;
use crate::walker::{DecodedElement, ElementDecoder, HandleIndex, InstanceField};
use crate::{Error, Result};

pub struct MyClassDecoder;

impl ElementDecoder for MyClassDecoder {
    fn class_name(&self) -> &'static str { "MyClass" }

    fn decode(
        &self,
        bytes: &[u8],
        schema: &formats::ClassEntry,
        _index: &HandleIndex,
    ) -> Result<DecodedElement> {
        if schema.name != "MyClass" {
            return Err(Error::BasicFileInfo(format!(
                "MyClassDecoder received wrong schema: {}", schema.name
            )));
        }
        Ok(crate::walker::decode_instance(bytes, 0, schema))
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct MyClass {
    pub name: Option<String>,
    pub foo: Option<i64>,
    // ... typed fields you want to expose
}

impl MyClass {
    pub fn from_decoded(decoded: &DecodedElement) -> Self {
        let mut out = Self { name: None, foo: None };
        for (field_name, value) in &decoded.fields {
            let n = crate::elements::level::normalise_field_name(field_name);
            match (n.as_str(), value) {
                ("name", InstanceField::String(s)) => out.name = Some(s.clone()),
                ("foo",  InstanceField::Integer { value, .. }) => out.foo = Some(*value),
                _ => {}
            }
        }
        out
    }
}
```

### Step 3 — Register in the dispatch table

```rust
// src/elements/mod.rs
pub mod level;
pub mod my_class;     // <-- add this

pub fn all_decoders() -> Vec<Box<dyn ElementDecoder>> {
    vec![
        Box::new(level::LevelDecoder),
        Box::new(my_class::MyClassDecoder),   // <-- and this
    ]
}
```

`decoder_for_class` takes the first decoder registered for a class
name, so check that `MyClass` has none yet
(`grep -rn '"MyClass"' src/elements/`).

Production decoding (`walker::iter_elements`) uses a typed decoder only
for a class listed in `elements::MVP_TYPED_CLASSES`, so add `MyClass`
there too. Production then decodes each schema instance of it with your
decoder, marked `provenance.decoder == "MyClassDecoder"`, and skips one
the decoder rejects rather than decoding it generically (fail closed):
Step 5's check is what keeps that from dropping real instances. A class
in that list is also advertised as typed, so the same change:

- adds its arm to `typed_json::mvp_typed_view`, or the CLI and Python
  give it `typed: null`;
- adds it wherever the typed classes are listed: the module doc of
  `src/elements/mod.rs`, the doc comment in `rvt-py/src/lib.rs`, the
  stub `python/rvt/__init__.pyi`, `docs/python.md`, the description in
  `docs/schemas/decoded-elements.schema.json`, the list
  `tests/python/test_decoded_api.py` pins, and the README's count of
  them. `git grep -n -E 'Level.{1,6}Wall.{1,6}Floor' -- . ':!CHANGELOG.md' ':!reports'`
  finds them, and any later copy.

### Step 4 — Write down how it can fail

Unit tests are banned in this repository, synthesized-byte tests
included. Before writing the decoder, list the ways it could go wrong
on real files: a field that is absent on one release, a record of
another class that happens to parse, a length that runs past the
stream, a value that is plausible but wrong. Each one becomes a check
in the corpus step below, against a real file and Revit's own export
of it, and the decoder returns an `Error` or nothing (fail closed) in
each case.

### Step 5 — Corpus integration test

Corpus tests verify it works on bytes Revit actually writes — the real test of whether your synth matches
reality.

Every `tests/*.rs` file is a module of one test binary, `integration`
(C6), and CI runs only the files `tools/ci/test-targets.txt` classes as
real-file, CLI, contract or hardening checks. So a new check takes three
steps:

1. Write `tests/myclass_corpus.rs`. Use the files whose oracle holds
   `MyClass`: the family corpus (`tests/common`, `RVT_SAMPLES_DIR`), or
   project files from `RVT_PROJECT_CORPUS_DIR` with Revit's export of
   each beside them, never a hard-coded path. Skip a missing file, but
   fail on one when `RVT_REQUIRE_CORPUS` is set, as CI sets it. Require
   an instance only where the oracle has one, and every instance it has:
   compare the set of decoded ElementIds with the oracle's, since
   production skips an instance the decoder rejects. Check that the instances
   decoded from the schema (`record_kind` `schema_instance`) came from
   your decoder, not the generic one: `iter_elements` also returns
   partition records of some classes, which other decoders read. Then
   check the typed view the CLI and Python expose,
   `typed_json::mvp_typed_view`, field by field against the oracle's
   value for the same element: the wire fields alone do not show a wrong
   mapping in `MyClass::from_decoded`.

   The example covers a class every family release holds:

   ```rust
   mod common;

   use common::{ALL_YEARS, sample_for_year};

   fn require_family_corpus() -> bool {
       std::env::var("RVT_REQUIRE_CORPUS").is_ok_and(|v| v == "1" || v == "true")
   }

   #[test]
   fn myclass_decodes_on_every_release() {
       let mut missing = Vec::new();
       for year in ALL_YEARS {
           let path = sample_for_year(year);
           if !path.exists() {
               missing.push(year);
               continue;
           }
           let mut rf = rvt::RevitFile::open(&path).unwrap();
           let from_schema: Vec<_> = rvt::walker::iter_elements(&mut rf)
               .unwrap()
               .filter(|e| e.class == "MyClass")
               .filter(|e| e.provenance.record_kind.as_deref() == Some("schema_instance"))
               .collect();
           // Every instance the oracle holds, not only some: production skips
           // an instance the decoder rejects, so a missing id is a dropped row.
           let decoded: std::collections::BTreeSet<u32> =
               from_schema.iter().filter_map(|e| e.id).collect();
           assert_eq!(decoded, oracle_myclass_ids(year), "{year}: MyClass ElementIds");
           for e in &from_schema {
               assert_eq!(e.provenance.decoder.as_deref(), Some("MyClassDecoder"), "{year}");
               let typed = rvt::elements::typed_json::mvp_typed_view(e)
                   .unwrap_or_else(|| panic!("{year}: MyClass has no typed view"));
               // Each field of `typed`, against the oracle's value for this
               // element (by its ElementId, `e.id`). `oracle_myclass_ids` and the
               // field values come from Revit's export of the same file.
               let _ = typed;
           }
       }
       assert!(
           missing.is_empty() || !require_family_corpus(),
           "RVT_REQUIRE_CORPUS is set and releases {missing:?} are missing"
       );
   }
   ```

   For a class only project files hold, loop over those files instead,
   resolving each as `tests/elem_table_corpus.rs`'s `project_file`
   does: it skips an absent file, and fails when `RVT_PROJECT_CORPUS_DIR`
   is set but the file is not there.

2. Add it to `tests/integration/main.rs`, in name order:

   ```rust
   #[path = "../myclass_corpus.rs"]
   mod myclass_corpus;
   ```

3. Class it `real` in `tools/ci/test-targets.txt`. A file the manifest
   does not class fails `tools/ci/verify-real-files.sh`, so a new check
   cannot go unrun.

CI fetches the family corpus and sets `RVT_REQUIRE_CORPUS=1`, so the
check above fails there on a missing release. Locally,
`RVT_SAMPLES_DIR=<corpus>/examples/Autodesk cargo test --profile ci --test integration myclass_corpus`
runs it.

### Step 6 — Document in `CHANGELOG.md`

Add to the `[Unreleased]` section:

```markdown
### Added
- **`elements::my_class::MyClassDecoder`** — Layer 5b decoder for
  Revit's `MyClass` (description). Produces
  `elements::my_class::MyClass { name, foo, ... }` as the typed
  view. Covers N fields, measured on <files> against <oracle>
  (<counts>, RE-NN); `tests/myclass_corpus.rs` checks it.
```

### Step 7 — Commit + PR

- `cargo fmt --all`
- `cargo clippy --all-targets -- -D warnings`
- `tools/check-local.sh` (fmt, clippy, rustdoc, build, real-file checks)
- Commit message: `feat(elements): a MyClass decoder (#NNN) (RE-NN)`
  — reference the GitHub issue (CLASS-xx / M3-xx in `TODO.md`)
- Open PR referencing the issue

---

## Field-name normalisation

Revit uses `m_` prefixes + varies between camelCase / snake_case /
UPPERCASE across versions. `src/elements/level.rs` exposes a
helper (`normalise_field_name`) you should reuse:

```
m_LevelTypeId        → leveltypeid
m_level_type_id      → leveltypeid
levelTypeId          → leveltypeid
LEVEL_TYPE_ID        → leveltypeid
```

Match against the normalised form in `from_decoded`. This is one
of the few places where "be generous in what you accept" is the
right posture.

---

## Type-dispatch cheat sheet

Field schema types → `InstanceField` variants:

| Schema field | Wire bytes | `InstanceField` variant |
|---|---|---|
| `Primitive { kind: 0x01, size: 1 }` | 1-byte padded bool | `Bool(bool)` |
| `Primitive { kind: 0x02, size: 2 }` | u16 LE | `Integer { signed: false, size: 2, .. }` |
| `Primitive { kind: 0x04/0x05, size: 4 }` | u32/i32 LE | `Integer { signed: ?, size: 4, .. }` |
| `Primitive { kind: 0x06, size: 4 }` | f32 LE | `Float { size: 4, .. }` |
| `Primitive { kind: 0x07, size: 8 }` | f64 LE | `Float { size: 8, .. }` |
| `Primitive { kind: 0x0b, size: 8 }` | i64 LE | `Integer { signed: true, size: 8, .. }` |
| `String` | u32 count + UTF-16LE chars | `String(String)` |
| `Guid` | 16 raw bytes | `Guid([u8; 16])` |
| `ElementId` / `ElementIdRef` | u32 tag + u32 id | `ElementId { tag, id }` |
| `Pointer { .. }` | u32 + u32 pointer pair | `Pointer { raw: [u32; 2] }` |
| `Vector { .. }` / `Container { .. }` | u32 count + elements | `Vector(..)` or `Bytes(..)` |

---

## When the decoder doesn't fit the generic pattern

Some classes need custom handling beyond `decode_instance`'s
field-by-field walk. Examples:

- **CurtainWall + CurtainGrid**: entity is a graph — the generic
  walker can decode the root but following the grid-mullion-panel
  references needs `HandleIndex` lookups.
- **FamilyInstance**: the field layout varies by the referenced
  Symbol's family. Requires type-aware dispatch.
- **View subtypes** (FloorPlan, Section, etc.): polymorphic — one
  base class with several concrete subclasses.

For these, override `decode` and call into `decode_instance` for
the common fields, then manually walk the `fields` vec + do
handle resolution for the specific relationships.

---

## Questions / unknowns

Document them explicitly in the decoder's module docstring, not
buried in comments. Example:

```rust
//! **Known open questions:**
//!
//! - `m_phaseCreated` semantics on R2026 — appears to be a
//!   PhaseRef but the ID space is different. Tracked as
//!   `docs/rvt-moat-break-reconnaissance.md` §Q7.X.
//! - `m_designOption` is `0xFFFFFFFF` on all files in the sample
//!   corpus; unclear what non-default value looks like. Need a
//!   project `.rvt` with multiple design options for validation.
```

This way future contributors can see what's uncertain without
reading git blame.

---

## Priority classes for initial adoption

If you're picking which class to tackle, the value-ordered list:

1. **Level** — already done, the reference
2. **Grid** + **ReferencePlane** — similar shape, small surface
3. **Material** + **FillPattern** + **LinePattern** — used by every visible element
4. **Category** + **Subcategory** — foundation for IFC category mapping
5. **View** subtypes (FloorPlan, Section, etc.) — required for sheet rendering
6. **Wall** + **WallType** — first "real geometry" class; unlocks IFC emission
7. **Floor** + **Roof** + **Ceiling** — boundary-sketch-based elements
8. **FamilyInstance** — parent of Door, Window, Furniture, Equipment
9. **Door** / **Window** — host-void semantics, essential for IFC
10. **Column** + **StructuralFraming** — beam/column IFC
11. **Dimension** + **Tag** + **TextNote** — annotation layer

Each unblocks downstream work. The first 6 give you a barebones
exporter + basic viewer; the rest fill in per-category coverage.

---

## Getting help

- Open a GitHub Discussion for "how do I…" questions
- File an Issue with the `class-decoder` label if you hit a
  blocker that looks like a bug in the scaffold
- `TODO.md` and `ROADMAP.md` carry the tracked decoder work (the
  historical L5B-XX planning ids were never checked in)
- Read `CONTRIBUTING.md` and `CLEANROOM.md` for the source-
  provenance rules before contributing
