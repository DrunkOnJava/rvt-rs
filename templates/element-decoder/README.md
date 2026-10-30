# Element decoder template

A [`cargo-generate`](https://cargo-generate.github.io/cargo-generate/)
template for scaffolding a new per-class element decoder under
`src/elements/`.

Adding a decoder for a Revit class is always the same three-file
change (register in `mod.rs`, implement the decoder, add tests). This
template generates the decoder file + test module so you can focus on
the fields that are actually class-specific.

## Usage

From the repo root:

```sh
cargo install cargo-generate       # one-time
cargo generate --path templates/element-decoder --name dimension
```

`--name` is informational — cargo-generate uses it as the generated
project name but it does not control the output file. What controls
the output file is the `class_name` prompt: the template lands at
`src/elements/<snake_case_of_class_name>.rs`.

### Prompts

| Prompt | What to type | Example |
|---|---|---|
| `class_name` | Revit class name in PascalCase | `Dimension` |
| `struct_description` | One-line description for the module docstring | `Revit's dimension annotation element (linear, radial, angular, arc-length).` |
| `class_name_snake` | Auto-derived, press enter to accept | `dimension` |

Only `class_name` and `struct_description` need manual input — the
snake-case form is derived from `class_name` via the liquid
`snake_case` filter.

## After generation

The generated file is a typed view, `{{class_name}}` with a single
`name` field, over what `SimpleDecoder` decodes. It is a **scaffold**,
not a finished decoder. Three follow-up steps:

1. **Register it** in `src/elements/mod.rs`:

   ```rust
   pub mod dimension;

   pub fn all_decoders() -> Vec<Box<dyn ElementDecoder>> {
       vec![
           // ... existing decoders
           Box::new(SimpleDecoder::new("Dimension")),
       ]
   }
   ```

2. **Fill in the fields.** Dump the schema from a known-good fixture
   with `cargo run --release --bin rvt-schema -- _corpus/<file>.rfa`
   and record the real field list in the module docstring + the
   typed struct. Replace the `TODO` markers left by the template.

3. **Check it on a real file.** Run
   `cargo run --release --bin rvt-elements -- <file>` on a corpus file
   that holds the class and compare the fields with Revit's own export.
   Unit tests are not accepted (`CONTRIBUTING.md`): put the evidence in
   the pull request.

The contributor guide at `docs/extending-layer-5b.md` has the full
walkthrough including field-type dispatch, schema discovery, and
corpus integration tests.

## Style notes

- Generated files follow `level.rs` conventions: 4-space indent,
  registration through `SimpleDecoder`, and `normalise_field_name`
  imported from the `level` module.
- `cargo fmt --all` and `cargo clippy --all-targets -- -D warnings`
  must both be clean after you finish populating the decoder.
- Do not delete the `TODO` markers without replacing them with real
  content; the guide grep's for them to find decoders that were
  scaffolded but never filled in.
