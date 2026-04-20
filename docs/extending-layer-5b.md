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
- `cargo test --release --test field_type_coverage` passes (sanity
  check that your corpus is good)

---

## The three-file change

Adding a decoder for `MyClass` touches exactly three files:

```
src/elements/mod.rs         # register the decoder
src/elements/my_class.rs    # decoder impl + typed view + tests
tests/elements.rs           # integration test against corpus
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

The `decoder_class_names_are_unique` test in `src/elements/mod.rs`
will fail at build time if you accidentally register two decoders
for the same class — a useful tripwire.

### Step 4 — Write unit tests with synthesized bytes

Every decoder gets unit tests against handcrafted bytes so the
test suite runs without the corpus. Pattern from
`src/elements/level.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::formats::{ClassEntry, FieldEntry, FieldType};

    fn synth_schema() -> ClassEntry {
        ClassEntry {
            name: "MyClass".to_string(),
            fields: vec![
                FieldEntry {
                    name: "m_name".to_string(),
                    cpp_type: Some("String".into()),
                    field_type: Some(FieldType::String),
                },
                FieldEntry {
                    name: "m_foo".to_string(),
                    cpp_type: Some("unsigned int".into()),
                    field_type: Some(FieldType::Primitive { kind: 0x05, size: 4 }),
                },
            ],
            offset: 0,
            tag: Some(0x1234),
            parent: None,
            declared_field_count: Some(2),
            was_parent_only: false,
            ancestor_tag: None,
        }
    }

    fn synth_bytes() -> Vec<u8> {
        let mut b = Vec::new();
        // m_name = "hello" — UTF-16LE length-prefixed
        let name = "hello";
        b.extend_from_slice(&(name.chars().count() as u32).to_le_bytes());
        for ch in name.encode_utf16() {
            b.extend_from_slice(&ch.to_le_bytes());
        }
        // m_foo = 42
        b.extend_from_slice(&42u32.to_le_bytes());
        b
    }

    #[test]
    fn decodes_all_fields() {
        let decoded = MyClassDecoder.decode(
            &synth_bytes(), &synth_schema(), &HandleIndex::new()
        ).unwrap();
        assert_eq!(decoded.fields.len(), 2);
    }

    #[test]
    fn typed_view() {
        let decoded = MyClassDecoder.decode(
            &synth_bytes(), &synth_schema(), &HandleIndex::new()
        ).unwrap();
        let v = MyClass::from_decoded(&decoded);
        assert_eq!(v.name.as_deref(), Some("hello"));
        assert_eq!(v.foo, Some(42));
    }

    #[test]
    fn rejects_wrong_schema() {
        let wrong = ClassEntry { name: "Wall".to_string(), ..synth_schema() };
        assert!(MyClassDecoder.decode(&[], &wrong, &HandleIndex::new()).is_err());
    }
}
```

Run them: `cargo test --lib elements::my_class`. Must pass before
moving on.

### Step 5 — Corpus integration test

The unit tests verify your decoder works on bytes shaped exactly
how you expect. Corpus tests verify it works on bytes Revit
actually writes — the real test of whether your synth matches
reality.

Add to `tests/elements.rs` (create if not present):

```rust
#[test]
#[cfg_attr(not(feature = "corpus"), ignore)]
fn myclass_decodes_on_2024_sample() {
    let path = "_corpus/examples/Autodesk/racbasicsamplefamily-2024.rfa";
    let mut rf = rvt::RevitFile::open(path).unwrap();
    let formats = rf.read_stream("Formats/Latest").unwrap();
    let formats_d = rvt::compression::inflate_at(&formats, 0).unwrap();
    let schema = rvt::formats::parse_schema(&formats_d).unwrap();
    let myclass = schema.classes.iter().find(|c| c.name == "MyClass")
        .expect("MyClass present in 2024 schema");

    // ... walk to the MyClass instance in Global/Latest (Layer 5a
    // is still under development for non-ADocument classes — for
    // now, just confirm the schema class has sane fields)
    assert!(!myclass.fields.is_empty());
}
```

### Step 6 — Document in `CHANGELOG.md`

Add to the `[Unreleased]` section:

```markdown
### Added
- **`elements::my_class::MyClassDecoder`** — Layer 5b decoder for
  Revit's `MyClass` (description). Produces
  `elements::my_class::MyClass { name, foo, ... }` as the typed
  view; generic `DecodedElement` via `decode_instance` is the
  underlying primitive. Covers N fields. Unit-tested with
  synthesized bytes; corpus integration test pending live walker
  integration (tracked in L5B-01..02).
```

### Step 7 — Commit + PR

- `cargo fmt --all`
- `cargo clippy --all-targets -- -D warnings`
- `cargo test --lib elements`
- Commit message: `feat(walker): Layer 5b MyClass decoder (#LXX)`
  — use the L5B-XX task number from `TODO-BLINDSIDE.md`
- Open PR referencing the task number

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
- Reference `TODO-BLINDSIDE.md` at the repo parent for the
  tracked L5B-XX task numbers
- Read `CONTRIBUTING.md` and `CLEANROOM.md` for the source-
  provenance rules before contributing
