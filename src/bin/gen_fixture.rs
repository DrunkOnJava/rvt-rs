//! `gen-fixture` — synthesize a minimal Revit-file-alike CFB fixture for tests.
//!
//! This binary writes a CFB (OLE2) container populated with the 12 invariant
//! streams `rvt::RevitFile` expects (see `reader::missing_required_streams`)
//! plus a single `Partitions/NN` stream. The streams are NOT captured from a
//! real Revit file — they are hand-built by this tool so the rvt-rs decoders
//! (schema parser, walker's `read_field_by_type`, BasicFileInfo regex,
//! compression pipeline) have deterministic, license-free inputs to chew on.
//!
//! # Approach: Option A (synthetic CFB)
//!
//! Option A (a real CFB container with synthetic stream contents) was feasible
//! because the project already has all the primitives:
//!
//! - `cfb` crate for the outer container
//! - `compression::truncated_gzip_encode` + `truncated_gzip_encode_with_prefix8`
//!   for the exact Revit framing conventions
//! - `formats.rs` documents the on-disk schema wire format explicitly, so the
//!   writer can emit bytes the `parse_schema` decoder happily re-reads
//!
//! Option B (decoded-form JSON) was the fallback if synthesizing the CFB layer
//! had proven fragile, but it would have skipped the walker and compression
//! pipelines — both of which are exactly what test fixtures should exercise.
//!
//! # Synthetic vs. real Revit files
//!
//! These fixtures are deliberately NOT byte-compatible with Autodesk Revit's
//! own writer. Specifically:
//!
//! - `BasicFileInfo` carries a plausible version/build string but no real GUID
//!   or original file path
//! - `Formats/Latest` declares a minimal class schema (3–5 classes); the full
//!   Revit schema is ~600 classes and would take real Revit to produce
//! - `Global/Latest` holds a few fabricated element instance records whose
//!   field payloads match the schema classes' FieldTypes, positioned after
//!   the mandatory 8-byte zero prefix
//! - Other required streams (`Contents`, `Global/ContentDocuments`,
//!   `RevitPreview4.0`, …) are present as empty / placeholder streams so the
//!   `has_revit_signature()` check passes
//!
//! The file will NOT open in Revit. It WILL parse cleanly through the rvt-rs
//! reader, walker, and compression layers — exactly the surface a test fixture
//! needs to hit.
//!
//! # Usage
//!
//! ```text
//! cargo run --bin gen-fixture -- minimal \
//!   --output tests/fixtures/synthetic-minimal.rvt
//! cargo run --bin gen-fixture -- structural \
//!   --classes Wall,Level,Project,Column \
//!   --element-count 25 \
//!   --seed 42
//! ```

use clap::Parser;
use rvt::compression;
use std::io::Write;
use std::path::PathBuf;
use std::process::ExitCode;

#[derive(Parser, Debug)]
#[command(
    name = "gen-fixture",
    version,
    about = "Synthesize a minimal Revit-file-alike CFB fixture for tests"
)]
struct Cli {
    /// Short fixture name (used in BasicFileInfo + log output).
    name: String,

    /// Output path. Defaults to `tests/fixtures/synthetic-<name>.rvt`
    /// under `CARGO_MANIFEST_DIR`.
    #[arg(short = 'o', long = "output")]
    output: Option<PathBuf>,

    /// Seed for the deterministic synthetic content. Same seed + same
    /// flags → byte-identical output.
    #[arg(long = "seed", default_value_t = 0u64)]
    seed: u64,

    /// Comma-separated class names to emit in the synthetic
    /// `Formats/Latest` schema. All names must start uppercase and use
    /// only `[A-Za-z0-9_]`.
    #[arg(long = "classes", default_value = "Wall,Level,Project")]
    classes: String,

    /// Number of synthetic element instance records to write into
    /// `Global/Latest`. Each record picks a class from `--classes`
    /// round-robin.
    #[arg(long = "element-count", default_value_t = 10usize)]
    element_count: usize,

    /// Revit release year to embed in `BasicFileInfo`. Controls the
    /// `Partitions/NN` stream name via `streams::partition_for_year`.
    #[arg(long = "year", default_value_t = 2024u32)]
    year: u32,
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match run(cli) {
        Ok(path) => {
            eprintln!("wrote {}", path.display());
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::from(1)
        }
    }
}

fn run(cli: Cli) -> anyhow::Result<PathBuf> {
    // Validate + parse class list.
    let classes: Vec<String> = cli
        .classes
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    for c in &classes {
        if !is_valid_class_name(c.as_bytes()) {
            anyhow::bail!(
                "invalid class name {c:?}: must start uppercase ASCII and \
                 contain only [A-Za-z0-9_]"
            );
        }
    }
    if classes.is_empty() {
        anyhow::bail!("--classes must name at least one class");
    }

    let out_path = resolve_output_path(&cli)?;
    if let Some(parent) = out_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let spec = FixtureSpec {
        name: cli.name.clone(),
        seed: cli.seed,
        classes,
        element_count: cli.element_count,
        year: cli.year,
    };

    write_fixture(&out_path, &spec)?;
    Ok(out_path)
}

fn resolve_output_path(cli: &Cli) -> anyhow::Result<PathBuf> {
    if let Some(p) = &cli.output {
        return Ok(p.clone());
    }
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| std::env::current_dir().unwrap());
    Ok(manifest_dir
        .join("tests")
        .join("fixtures")
        .join(format!("synthetic-{}.rvt", cli.name)))
}

/// Parameters that fully determine the synthesized fixture's bytes.
struct FixtureSpec {
    name: String,
    seed: u64,
    classes: Vec<String>,
    element_count: usize,
    year: u32,
}

fn write_fixture(path: &std::path::Path, spec: &FixtureSpec) -> anyhow::Result<()> {
    // Build every stream's final on-disk bytes up front.
    let basic_file_info = build_basic_file_info(spec);
    let formats_latest = build_formats_latest(spec)?;
    let global_latest = build_global_latest(spec)?;
    let revit_preview = build_revit_preview();
    let part_atom = build_part_atom();

    // Partition year → NN mapping. Falls back to 67 (2024) if the
    // requested year is out of the table's range.
    let partition_nn = rvt::streams::partition_for_year(spec.year).unwrap_or(67);
    let partitions_stream = format!("Partitions/{partition_nn}");

    let streams: Vec<(&str, Vec<u8>)> = vec![
        (rvt::streams::BASIC_FILE_INFO, basic_file_info),
        (rvt::streams::CONTENTS, vec![]),
        (rvt::streams::FORMATS_LATEST, formats_latest),
        (rvt::streams::GLOBAL_CONTENT_DOCUMENTS, vec![]),
        (rvt::streams::GLOBAL_DOC_INCREMENT_TABLE, vec![]),
        (rvt::streams::GLOBAL_ELEM_TABLE, vec![]),
        (rvt::streams::GLOBAL_HISTORY, vec![]),
        (rvt::streams::GLOBAL_LATEST, global_latest),
        (rvt::streams::GLOBAL_PARTITION_TABLE, vec![]),
        (rvt::streams::PART_ATOM, part_atom),
        (rvt::streams::REVIT_PREVIEW_4_0, revit_preview),
        (rvt::streams::TRANSMISSION_DATA, vec![]),
        (&partitions_stream, vec![]),
    ];

    let out_file = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(true)
        .open(path)?;
    let mut out =
        cfb::CompoundFile::create(out_file).map_err(|e| anyhow::anyhow!("cfb create: {e}"))?;

    // Pre-create parent storages (`/Formats`, `/Global`, `/Partitions`).
    let mut created: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    for (name, _) in &streams {
        let norm = format!("/{}", name.trim_start_matches('/'));
        let parts: Vec<&str> = norm.split('/').filter(|s| !s.is_empty()).collect();
        for n in 1..parts.len() {
            let parent = format!("/{}", parts[..n].join("/"));
            if created.insert(parent.clone()) {
                out.create_storage(&parent)
                    .map_err(|e| anyhow::anyhow!("create_storage {parent}: {e}"))?;
            }
        }
    }

    for (name, data) in streams {
        let path = format!("/{}", name.trim_start_matches('/'));
        let mut s = out
            .create_stream(&path)
            .map_err(|e| anyhow::anyhow!("create_stream {path}: {e}"))?;
        s.write_all(&data)
            .map_err(|e| anyhow::anyhow!("write_all {path}: {e}"))?;
    }
    out.flush().map_err(|e| anyhow::anyhow!("cfb flush: {e}"))?;
    Ok(())
}

// ─── BasicFileInfo ───────────────────────────────────────────────────

/// Emit a UTF-16LE byte buffer that `basic_file_info::extract_version`
/// + `extract_build` can parse. The format matches the 2019+ pattern:
///   `<year>  YYYYMMDD_HHMM(x64)Z C:\synthetic\<name>.rfa`.
fn build_basic_file_info(spec: &FixtureSpec) -> Vec<u8> {
    let mut s = String::new();
    s.push_str(&format!("{}  20230101_0000(x64)Z ", spec.year));
    s.push_str(&format!(
        "C:\\synthetic\\rvt-rs\\fixtures\\{}.rfa ENU",
        spec.name
    ));
    // Synthetic file GUID (stable for a given name + seed — downstream
    // tests can treat it as a deterministic value).
    let guid = synthetic_guid(&spec.name, spec.seed);
    s.push(' ');
    s.push_str(&guid);
    utf16le(&s)
}

fn utf16le(s: &str) -> Vec<u8> {
    let mut out = Vec::with_capacity(s.len() * 2);
    for unit in s.encode_utf16() {
        out.extend_from_slice(&unit.to_le_bytes());
    }
    out
}

/// Deterministic pseudo-GUID from `(name, seed)`. Not a real UUID —
/// just a placeholder shaped like one so `extract_guid` can recover
/// it.
fn synthetic_guid(name: &str, seed: u64) -> String {
    let mut h = seed;
    for b in name.bytes() {
        h = h.wrapping_mul(1_000_003).wrapping_add(b as u64);
    }
    let a = (h & 0xffff_ffff) as u32;
    let b = ((h >> 16) & 0xffff) as u16;
    let c = ((h >> 8) & 0xffff) as u16;
    let d = (h & 0xffff) as u16;
    let e = h.wrapping_mul(0x9e37_79b9) & 0xffff_ffff_ffff;
    format!("{a:08x}-{b:04x}-{c:04x}-{d:04x}-{e:012x}")
}

// ─── Formats/Latest ──────────────────────────────────────────────────

/// Build a decompressed `Formats/Latest` payload with the requested
/// class schema, then truncated-gzip encode it from offset 0 (per the
/// `StreamFraming::RawGzipFromZero` convention).
///
/// Wire format (mirrors `src/formats.rs`):
///
/// ```text
/// [u16 LE name_len][name_len ASCII class name]
/// [u16 LE (tag | 0x8000)]                       // tagged class flag
/// [u16 LE 0x0000]                               // pad
/// [u16 LE parent_len][parent_len ASCII parent]
/// [u16 LE 0x0000]                               // ancestor flag
/// [u32 LE field_count]                          // declared field count
/// [u32 LE field_count]                          // duplicate
/// per field:
///   [u32 LE field_name_len][field_name]
///   [4-byte field type encoding]
/// ```
fn build_formats_latest(spec: &FixtureSpec) -> anyhow::Result<Vec<u8>> {
    let mut body: Vec<u8> = Vec::new();

    // Lead-in padding so the schema's first class record doesn't start
    // at offset 0 — the decoder is tolerant but real Revit files do
    // carry a small preamble here.
    body.extend_from_slice(&[0u8; 8]);

    // Assign stable tags. Start at 0x100 so we don't collide with the
    // canonical `ElementId` tag (0x0014).
    for (idx, class_name) in spec.classes.iter().enumerate() {
        let tag = 0x0100u16 + (idx as u16);
        emit_class_record(
            &mut body,
            class_name,
            tag,
            "Element",
            &synthesize_fields(class_name),
        )?;
    }

    // Pad the body out so the schema parser's "at least 64 bytes
    // before bailing" heuristic has headroom.
    body.resize(body.len() + 64, 0u8);

    compression::truncated_gzip_encode(&body).map_err(|e| anyhow::anyhow!("truncated_gzip: {e}"))
}

/// Write a single tagged class record. See the wire-format comment
/// above `build_formats_latest`.
fn emit_class_record(
    body: &mut Vec<u8>,
    class_name: &str,
    tag: u16,
    parent_name: &str,
    fields: &[SynthField],
) -> anyhow::Result<()> {
    let name_bytes = class_name.as_bytes();
    if name_bytes.len() > u16::MAX as usize {
        anyhow::bail!("class name too long");
    }
    body.extend_from_slice(&(name_bytes.len() as u16).to_le_bytes());
    body.extend_from_slice(name_bytes);
    // Tag with 0x8000 flag set — marks this as a top-level serializable
    // class so `parse_schema` reads the parent + field-count preamble.
    body.extend_from_slice(&(tag | 0x8000).to_le_bytes());
    // 2-byte pad.
    body.extend_from_slice(&[0u8; 2]);
    // Parent name.
    let parent_bytes = parent_name.as_bytes();
    body.extend_from_slice(&(parent_bytes.len() as u16).to_le_bytes());
    body.extend_from_slice(parent_bytes);
    // Ancestor-flag slot (0 = no ancestor_tag).
    body.extend_from_slice(&[0u8; 2]);
    // Declared field-count (duplicated u32s).
    let fc = fields.len() as u32;
    body.extend_from_slice(&fc.to_le_bytes());
    body.extend_from_slice(&fc.to_le_bytes());
    for f in fields {
        let fname = f.name.as_bytes();
        body.extend_from_slice(&(fname.len() as u32).to_le_bytes());
        body.extend_from_slice(fname);
        body.extend_from_slice(&f.encoding);
    }
    Ok(())
}

struct SynthField {
    name: &'static str,
    /// 4-byte type encoding as documented in `FieldType::decode`.
    encoding: [u8; 4],
    /// Wire-format bytes for a single field value (must match the
    /// encoding's declared size). Used when writing instance payloads
    /// into `Global/Latest`.
    value_bytes: Vec<u8>,
}

/// Fixed field set per class. Keeps the fixture tight + the wire
/// layout trivially round-trippable.
fn synthesize_fields(class_name: &str) -> Vec<SynthField> {
    // Common fields every element carries on the wire: a bool flag, a
    // u32 id, and a GUID. Extra fields depend on the class name.
    let mut out: Vec<SynthField> = vec![
        SynthField {
            name: "m_flag",
            encoding: [0x01, 0x00, 0x00, 0x00],
            value_bytes: vec![0x01],
        },
        SynthField {
            name: "m_id",
            encoding: [0x05, 0x00, 0x00, 0x00],
            value_bytes: 42u32.to_le_bytes().to_vec(),
        },
        SynthField {
            name: "m_guid",
            encoding: [0x09, 0x00, 0x00, 0x00],
            value_bytes: (0u8..16u8).collect(),
        },
    ];
    // Wall / Level / Column-style classes get an f64 height field.
    if matches!(class_name, "Wall" | "Level" | "Column" | "Beam" | "Slab") {
        out.push(SynthField {
            name: "m_height",
            encoding: [0x07, 0x00, 0x00, 0x00],
            value_bytes: 10.0_f64.to_le_bytes().to_vec(),
        });
    }
    // Project gets an i64 version stamp.
    if class_name == "Project" {
        out.push(SynthField {
            name: "m_versionStamp",
            encoding: [0x0b, 0x00, 0x00, 0x00],
            value_bytes: 2024_i64.to_le_bytes().to_vec(),
        });
    }
    out
}

// ─── Global/Latest ───────────────────────────────────────────────────

/// Build a decompressed `Global/Latest` payload: a short magic header,
/// then `element_count` synthetic element instance records. Each
/// record's field layout matches the schema class picked (round-robin
/// over `--classes`). The result is truncated-gzip encoded with the
/// 8-byte zero prefix (`StreamFraming::CustomPrefix8`).
fn build_global_latest(spec: &FixtureSpec) -> anyhow::Result<Vec<u8>> {
    let mut body: Vec<u8> = Vec::new();

    // Lead-in — mimics the `cfb` wrapper padding Real Revit places
    // between the 8-byte prefix and the meaningful payload.
    body.extend_from_slice(&[0u8; 0x20]);

    // Seed-seeded LCG so per-element id fields are unique but
    // deterministic.
    let mut rng = spec.seed.wrapping_mul(0x9e37_79b9).wrapping_add(1);
    for idx in 0..spec.element_count {
        let class_name = &spec.classes[idx % spec.classes.len()];
        let fields = synthesize_fields(class_name);
        let element_id = (idx as u32).wrapping_add(1);

        // Record header: [u32 element_id][u32 class_tag_index].
        body.extend_from_slice(&element_id.to_le_bytes());
        body.extend_from_slice(&((idx % spec.classes.len()) as u32).to_le_bytes());

        for f in &fields {
            // Override m_id with a per-element unique value so tests
            // that scan for duplicates find them distinguishable.
            if f.name == "m_id" {
                let fresh = rng.wrapping_mul(1_103_515_245).wrapping_add(12345);
                rng = fresh;
                let v = (fresh >> 16) as u32;
                body.extend_from_slice(&v.to_le_bytes());
            } else {
                body.extend_from_slice(&f.value_bytes);
            }
        }
    }

    // Pad so the compression layer has something to bite on.
    body.resize(body.len() + 64, 0u8);

    compression::truncated_gzip_encode_with_prefix8(&body)
        .map_err(|e| anyhow::anyhow!("truncated_gzip prefix8: {e}"))
}

// ─── Other required streams ──────────────────────────────────────────

/// Emit a minimal `RevitPreview4.0` stream: 16-byte Revit-specific
/// wrapper then a 1×1 transparent PNG. `RevitFile::preview_png`
/// locates the PNG via the standard magic, so the wrapper bytes are
/// opaque filler — any value works.
fn build_revit_preview() -> Vec<u8> {
    let mut out = Vec::new();
    // Revit wrapper magic (from the reader doc comment) followed by
    // 12 bytes of zero filler. The PNG magic is what preview_png
    // scans for; wrapper contents don't matter.
    out.extend_from_slice(&[0x62, 0x19, 0x22, 0x05, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
    out.extend_from_slice(&MIN_PNG_1X1);
    out
}

/// Minimal 1×1 fully-transparent PNG. IHDR + IDAT + IEND, zlib-
/// compressed single-pixel RGBA sample. Verified decoded by `png`
/// crate and browsers; synthesized offline so we don't pull a PNG
/// encoder dep into this binary.
const MIN_PNG_1X1: [u8; 67] = [
    0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, // PNG magic
    0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44, 0x52, // IHDR chunk (length=13)
    0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, // 1×1
    0x08, 0x06, 0x00, 0x00, 0x00, // 8-bit RGBA, deflate, no filter, non-interlaced
    0x1F, 0x15, 0xC4, 0x89, // IHDR CRC
    0x00, 0x00, 0x00, 0x0A, 0x49, 0x44, 0x41, 0x54, // IDAT (length=10)
    0x78, 0x9C, 0x63, 0x00, 0x01, 0x00, 0x00, 0x05, 0x00, 0x01, // zlib body
    0x0D, 0x0A, 0x2D, 0xB4, // IDAT CRC
    0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82, // IEND
];

/// A minimal PartAtom XML document. Gives `PartAtom::from_bytes`
/// enough structure to parse — titles + categories are synthetic but
/// shaped right.
fn build_part_atom() -> Vec<u8> {
    let xml = r#"<?xml version="1.0" encoding="utf-8"?>
<entry xmlns="http://www.w3.org/2005/Atom" xmlns:a="urn:schemas-autodesk-com:partatom">
  <title>synthetic fixture</title>
  <id>urn:synthetic:rvt-rs:fixture</id>
  <updated>2026-04-20T00:00:00Z</updated>
  <category term="23.40.20.00" scheme="OmniClass"/>
</entry>
"#;
    xml.as_bytes().to_vec()
}

// ─── Helpers ─────────────────────────────────────────────────────────

/// Same rule the schema parser applies: first byte uppercase ASCII,
/// rest alphanumeric or underscore.
fn is_valid_class_name(bytes: &[u8]) -> bool {
    if bytes.is_empty() {
        return false;
    }
    if !bytes[0].is_ascii_uppercase() {
        return false;
    }
    bytes[1..]
        .iter()
        .all(|c| c.is_ascii_alphanumeric() || *c == b'_')
}
