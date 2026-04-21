//! L5B-09.1 — find corpus examples of `FieldType::Container` with
//! kinds 0x04, 0x07, 0x0d (and any other non-0x0e variants).
//!
//! `src/formats.rs` only tests the 0x0e (reference-typed) Container
//! decode path. The generalised 2-column decoder (L5B-09) needs real
//! wire samples for the scalar-base container kinds. This probe
//! scans every schema in the corpus and reports a histogram of
//! distinct `Container.kind` values, with the class + field that
//! exhibits each kind so the follow-up hex-dump probe (L5B-09.2)
//! knows where to look.
//!
//! Resolves paths via `RVT_SAMPLES_DIR` (family corpus) +
//! `RVT_PROJECT_CORPUS_DIR` (project corpus, optional).

use rvt::{RevitFile, compression, formats, streams};
use std::collections::BTreeMap;

fn main() {
    let mut paths: Vec<String> = Vec::new();

    // Family corpus via RVT_SAMPLES_DIR or default relative path.
    let family_dir = std::env::var("RVT_SAMPLES_DIR").unwrap_or_else(|_| "../../samples".into());
    if let Ok(entries) = std::fs::read_dir(&family_dir) {
        for e in entries.flatten() {
            let p = e.path();
            if p.extension().is_some_and(|x| x == "rfa" || x == "rvt") {
                paths.push(p.to_string_lossy().into_owned());
            }
        }
    }

    // Project corpus, if set.
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

    if paths.is_empty() {
        eprintln!(
            "no corpus files found. Set RVT_SAMPLES_DIR or \
             RVT_PROJECT_CORPUS_DIR to point at a .rvt/.rfa directory."
        );
        return;
    }

    // kind -> Vec<(file, class, field)>.
    let mut by_kind: BTreeMap<u8, Vec<(String, String, String)>> = BTreeMap::new();

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
                if let Some(formats::FieldType::Container { kind, .. }) = &f.field_type {
                    by_kind.entry(*kind).or_default().push((
                        file_name.clone(),
                        cls.name.clone(),
                        f.name.clone(),
                    ));
                }
            }
        }
    }

    println!(
        "=== Container kind histogram across {} files ===",
        paths.len()
    );
    for (kind, examples) in &by_kind {
        println!(
            "\nkind 0x{kind:02x}: {} occurrences, {} distinct (file, class, field) triples",
            examples.len(),
            {
                let mut unique: std::collections::BTreeSet<(String, String, String)> =
                    std::collections::BTreeSet::new();
                unique.extend(examples.iter().cloned());
                unique.len()
            }
        );
        // Show up to 5 examples per kind.
        for (file, class, field) in examples.iter().take(5) {
            println!("  {file} → {class}.{field}");
        }
        if examples.len() > 5 {
            println!("  … {} more", examples.len() - 5);
        }
    }
}
