//! FMT-01 — schema round-trip byte-delta audit.
//!
//! Two checks:
//!   1. Structural round-trip: decode(field_type.encode()) == field_type
//!      Must be 100% or there's a real round-trip regression.
//!   2. Byte-identical check (best-effort): scan source for field name,
//!      compare encoded vs source bytes at the post-name position.
//!      Known to have gaps from canonicalization (String alt form,
//!      Vector/Container alt sub) — documented as intentional in
//!      FieldType::encode doc. Also limited by the probe's naive
//!      name-search when field names collide across classes.
//!
//! The primary L5B-09-related concern is whether scalar-base Container
//! encoding (now always 4 B after the fix) matches source. The probe's
//! per-kind breakdown makes this visible.

use rvt::{RevitFile, compression, formats, streams};
use std::collections::BTreeMap;
use std::path::PathBuf;

/// Find the next occurrence of `field_name` (with its u32-LE length
/// prefix) at or after `start` in `source`. Returns the position of
/// the length-prefix byte.
fn find_field_offset_from(source: &[u8], field_name: &str, start: usize) -> Option<usize> {
    let name_bytes = field_name.as_bytes();
    let len_le = (name_bytes.len() as u32).to_le_bytes();
    let needle: Vec<u8> = len_le.iter().chain(name_bytes.iter()).copied().collect();
    if start >= source.len() {
        return None;
    }
    source[start..]
        .windows(needle.len())
        .position(|w| w == needle)
        .map(|p| p + start)
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let targets: Vec<PathBuf> = if args.is_empty() {
        let project_dir = std::env::var("RVT_PROJECT_CORPUS_DIR")
            .unwrap_or_else(|_| "/private/tmp/rvt-corpus-probe/magnetar/Revit".into());
        vec![
            PathBuf::from(format!("{project_dir}/Revit_IFC5_Einhoven.rvt")),
            PathBuf::from(format!("{project_dir}/2024_Core_Interior.rvt")),
            PathBuf::from("../../samples/rac_basic_sample_family-2016.rfa"),
            PathBuf::from("../../samples/racbasicsamplefamily-2024.rfa"),
            PathBuf::from("../../samples/racbasicsamplefamily-2026.rfa"),
        ]
    } else {
        args.into_iter().map(PathBuf::from).collect()
    };

    for path in &targets {
        if !path.exists() {
            continue;
        }
        let Ok(mut rf) = RevitFile::open(path) else {
            continue;
        };
        let Ok(raw) = rf.read_stream(streams::FORMATS_LATEST) else {
            continue;
        };
        let Ok(decomp) = compression::inflate_at(&raw, 0) else {
            continue;
        };
        let Ok(schema) = formats::parse_schema(&decomp) else {
            continue;
        };

        let name = path.file_name().unwrap().to_string_lossy().into_owned();
        let mut total_fields = 0usize;
        let mut structural_ok = 0usize;
        let mut byte_ident_ok = 0usize;
        let mut byte_ident_fail = 0usize;
        let mut unfindable_name = 0usize;
        let mut by_kind: BTreeMap<String, (usize, usize)> = BTreeMap::new();
        let mut fail_samples: Vec<(String, String, Vec<u8>, Vec<u8>)> = Vec::new();

        // Build a name-frequency map so we can restrict byte-identical
        // comparison to fields whose names are globally unique. That
        // avoids duplicate-name ambiguity (many classes reuse "first",
        // "second", "m_id").
        let mut name_freq: BTreeMap<&str, usize> = BTreeMap::new();
        for cls in &schema.classes {
            for f in &cls.fields {
                *name_freq.entry(f.name.as_str()).or_insert(0) += 1;
            }
        }

        for cls in &schema.classes {
            for field in &cls.fields {
                total_fields += 1;
                let Some(ft) = &field.field_type else {
                    continue;
                };
                let encoded = ft.encode();

                // 1. Structural round-trip.
                let redecoded = formats::FieldType::decode(&encoded);
                if redecoded == *ft {
                    structural_ok += 1;
                }

                // 2. Byte-identical: ONLY for globally-unique names.
                if name_freq.get(field.name.as_str()) != Some(&1) {
                    continue;
                }
                let Some(name_pos) = find_field_offset_from(&decomp, &field.name, 0) else {
                    unfindable_name += 1;
                    continue;
                };
                let type_enc_start = name_pos + 4 + field.name.len();
                let type_enc_end = (type_enc_start + encoded.len()).min(decomp.len());
                let source_slice = &decomp[type_enc_start..type_enc_end];
                let matches = source_slice == encoded.as_slice();
                let kind_label = match ft {
                    formats::FieldType::Primitive { kind, .. } => {
                        format!("Primitive 0x{kind:02x}")
                    }
                    formats::FieldType::String => "String".to_string(),
                    formats::FieldType::Guid => "Guid".to_string(),
                    formats::FieldType::ElementId => "ElementId".to_string(),
                    formats::FieldType::ElementIdRef { .. } => "ElementIdRef".to_string(),
                    formats::FieldType::Pointer { .. } => "Pointer".to_string(),
                    formats::FieldType::Vector { kind, .. } => format!("Vector 0x{kind:02x}"),
                    formats::FieldType::Container { kind, .. } => {
                        format!("Container 0x{kind:02x}")
                    }
                    formats::FieldType::Unknown { .. } => "Unknown".to_string(),
                };
                let entry = by_kind.entry(kind_label.clone()).or_insert((0, 0));
                if matches {
                    byte_ident_ok += 1;
                    entry.0 += 1;
                } else {
                    byte_ident_fail += 1;
                    entry.1 += 1;
                    if fail_samples.len() < 10 {
                        fail_samples.push((
                            cls.name.clone(),
                            field.name.clone(),
                            source_slice.to_vec(),
                            encoded.clone(),
                        ));
                    }
                }
            }
        }

        println!("\n=== {name} ===");
        println!(
            "  Total fields: {total_fields}, structural_ok={structural_ok}, \
             byte_ident_ok={byte_ident_ok}, byte_ident_fail={byte_ident_fail}, \
             unfindable_names={unfindable_name}"
        );
        let total_compared = byte_ident_ok + byte_ident_fail;
        if total_compared > 0 {
            println!(
                "  Byte-identical rate: {:.2}% ({byte_ident_ok}/{total_compared})",
                100.0 * (byte_ident_ok as f64) / (total_compared as f64)
            );
        }
        println!("  Per-kind (ok / fail):");
        for (kind, (ok, fail)) in &by_kind {
            println!("    {kind:<20}  ok={ok:>5}  fail={fail:>5}");
        }
        if !fail_samples.is_empty() {
            println!("  Sample byte-identical failures (first 5):");
            for (cls, field, src, enc) in fail_samples.iter().take(5) {
                println!(
                    "    {cls}::{field}\n      src: {:?}\n      enc: {:?}",
                    src.iter()
                        .map(|b| format!("{b:02x}"))
                        .collect::<Vec<_>>()
                        .join(" "),
                    enc.iter()
                        .map(|b| format!("{b:02x}"))
                        .collect::<Vec<_>>()
                        .join(" ")
                );
            }
        }
    }
}
