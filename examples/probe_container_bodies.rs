//! L5B-09.2 — hex-dump `Container.body` bytes for one exemplar per
//! kind.
//!
//! Feeds L5B-09.3's wire-format reverse-engineering work.
//! `Container.body` holds the bytes *after* the 4-byte outer header,
//! so what prints here is the actual scalar-base container payload
//! for kinds 0x01 / 0x02 / 0x04 / 0x05 / 0x07 / 0x0d — the
//! non-0x0e paths that `read_field_by_type` does not yet decode.
//!
//! Picks the first occurrence seen per kind in schema-parse order
//! (deterministic — same file, same class.field, same body). If a
//! body is empty (some Container variants embed only a header with
//! no payload), this is reported so L5B-09.3 knows that kind's
//! layout is "nothing after the outer kind/sub header."

use rvt::{RevitFile, compression, formats, streams};
use std::collections::BTreeMap;

fn main() {
    let mut paths: Vec<String> = Vec::new();

    let family_dir = std::env::var("RVT_SAMPLES_DIR").unwrap_or_else(|_| "../../samples".into());
    if let Ok(entries) = std::fs::read_dir(&family_dir) {
        for e in entries.flatten() {
            let p = e.path();
            if p.extension().is_some_and(|x| x == "rfa" || x == "rvt") {
                paths.push(p.to_string_lossy().into_owned());
            }
        }
    }
    if let Ok(proj) = std::env::var("RVT_PROJECT_CORPUS_DIR") {
        if let Ok(entries) = std::fs::read_dir(&proj) {
            for e in entries.flatten() {
                let p = e.path();
                if p.extension().is_some_and(|x| x == "rvt") {
                    paths.push(p.to_string_lossy().into_owned());
                }
            }
        }
    }
    paths.sort();

    if paths.is_empty() {
        eprintln!(
            "no corpus files found. Set RVT_SAMPLES_DIR or \
             RVT_PROJECT_CORPUS_DIR."
        );
        return;
    }

    // Target kinds — the non-0x0e variants whose wire format is
    // undocumented in src/formats.rs beyond the outer kind byte.
    let targets: &[u8] = &[0x01, 0x02, 0x04, 0x05, 0x07, 0x0d];
    let mut seen: BTreeMap<u8, (String, String, String, Vec<u8>, Option<String>)> = BTreeMap::new();

    for path in &paths {
        let Ok(mut rf) = RevitFile::open(path) else {
            continue;
        };
        let Ok(raw) = rf.read_stream(streams::FORMATS_LATEST) else {
            continue;
        };
        let Ok((_, d)) = compression::inflate_at_auto(&raw) else {
            continue;
        };
        let Ok(schema) = formats::parse_schema(&d) else {
            continue;
        };
        let file_name = path.rsplit('/').next().unwrap_or(path).to_string();

        for cls in &schema.classes {
            for f in &cls.fields {
                if let Some(formats::FieldType::Container {
                    kind,
                    cpp_signature,
                    body,
                }) = &f.field_type
                {
                    if targets.contains(kind) && !seen.contains_key(kind) {
                        seen.insert(
                            *kind,
                            (
                                file_name.clone(),
                                cls.name.clone(),
                                f.name.clone(),
                                body.clone(),
                                cpp_signature.clone(),
                            ),
                        );
                    }
                }
            }
        }
        if targets.iter().all(|k| seen.contains_key(k)) {
            break;
        }
    }

    println!("=== Container body exemplars ===");
    for kind in targets {
        match seen.get(kind) {
            Some((file, class, field, body, sig)) => {
                println!(
                    "\nkind 0x{kind:02x} @ {file} :: {class}.{field}  (body={} B, sig={:?})",
                    body.len(),
                    sig
                );
                if body.is_empty() {
                    println!("  <empty body>");
                } else {
                    hexdump(body, 48);
                }
            }
            None => {
                println!("\nkind 0x{kind:02x} — no exemplar in scanned corpus");
            }
        }
    }
}

fn hexdump(bytes: &[u8], max_total: usize) {
    let limit = bytes.len().min(max_total);
    let mut i = 0usize;
    while i < limit {
        let end = (i + 16).min(limit);
        let hex: String = bytes[i..end]
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<Vec<_>>()
            .join(" ");
        let ascii: String = bytes[i..end]
            .iter()
            .map(|&b| {
                if (0x20..0x7f).contains(&b) {
                    b as char
                } else {
                    '.'
                }
            })
            .collect();
        println!("  {i:04x}  {hex:<48}  |{ascii}|");
        i = end;
    }
    if bytes.len() > max_total {
        println!("  ... {} more bytes elided", bytes.len() - max_total);
    }
}
