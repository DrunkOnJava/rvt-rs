//! Wave 1 evidence probe for Discussion #112 / issue #151 (checksum-paged streams).
//!
//! Compares **control** (bare inflate on stored CFB bytes) vs **experiment**
//! (strip each full 65_249-byte page's 353-byte trailer, then inflate).
//!
//! Does not change production inflate paths — probe-only.
//!
//! Usage:
//! ```text
//! cargo run --release --example probe_checksum_page_evidence -- \
//!   path.rvt Formats/Latest [--json]
//! cargo run --release --example probe_checksum_page_evidence -- \
//!   path.rvt --all-paged [--json]
//! ```
//!
//! Credit: reported layout from @STE1200 (Discussion #112).

use std::env;
use std::io::Read;
use std::path::PathBuf;

use rvt::compression::{
    self, REVIT_PAGE_CHECKSUM_BYTES, REVIT_PAGE_PAYLOAD_BYTES, REVIT_STORED_PAGE_BYTES,
    is_checksum_paged_stream, strip_revit_page_checksums,
};
use rvt::formats;
use serde_json::{Value, json};

fn main() {
    let mut args: Vec<String> = env::args().skip(1).collect();
    let json_out = args.iter().any(|a| a == "--json");
    args.retain(|a| a != "--json");
    let all_paged = args.iter().any(|a| a == "--all-paged");
    args.retain(|a| a != "--all-paged");

    let path = PathBuf::from(
        args.first()
            .expect("usage: PATH [STREAM|--all-paged] [--json]"),
    );
    let mut file = cfb::open(&path).expect("open cfb");

    let streams: Vec<String> = if all_paged {
        let mut names = Vec::new();
        for entry in file.walk() {
            if !entry.is_stream() {
                continue;
            }
            let clean = entry
                .path()
                .display()
                .to_string()
                .replace('\\', "/")
                .trim_start_matches('/')
                .to_string();
            if is_checksum_paged_stream(&clean) {
                names.push(clean);
            }
        }
        names.sort();
        names
    } else {
        vec![
            args.get(1)
                .cloned()
                .unwrap_or_else(|| "Formats/Latest".to_string()),
        ]
    };

    let mut reports = Vec::new();
    for stream in &streams {
        match probe_one(&mut file, &path, stream) {
            Ok(v) => reports.push(v),
            Err(e) => reports.push(json!({
                "file": path.display().to_string(),
                "stream": stream,
                "error": e,
            })),
        }
    }

    if json_out {
        println!("{}", serde_json::to_string_pretty(&reports).unwrap());
    } else {
        for r in &reports {
            print_human(r);
        }
    }
}

fn probe_one(
    file: &mut cfb::CompoundFile<std::fs::File>,
    path: &std::path::Path,
    stream: &str,
) -> Result<Value, String> {
    let mut raw = Vec::new();
    file.open_stream(stream)
        .map_err(|e| format!("open stream: {e}"))?
        .read_to_end(&mut raw)
        .map_err(|e| format!("read: {e}"))?;

    let full_pages = raw.len() / REVIT_STORED_PAGE_BYTES;
    let rem = raw.len() % REVIT_STORED_PAGE_BYTES;
    let stripped = strip_revit_page_checksums(&raw);

    let control = inflate_bundle(&raw);
    let experiment = inflate_bundle(&stripped);

    let mut schema = Value::Null;
    if stream.eq_ignore_ascii_case("Formats/Latest") {
        schema = json!({
            "control": schema_metrics(&raw),
            "experiment": schema_metrics(&stripped),
        });
    }

    let ok_delta = experiment["inflate_all_chunks_ok"].as_u64().unwrap_or(0) as i64
        - control["inflate_all_chunks_ok"].as_u64().unwrap_or(0) as i64;
    let bytes_delta = experiment["inflate_all_chunks_total"].as_u64().unwrap_or(0) as i64
        - control["inflate_all_chunks_total"].as_u64().unwrap_or(0) as i64;

    Ok(json!({
        "file": path.display().to_string(),
        "stream": stream,
        "is_checksum_paged_path": is_checksum_paged_stream(stream),
        "stored_len": raw.len(),
        "stripped_len": stripped.len(),
        "removed": raw.len() - stripped.len(),
        "full_pages": full_pages,
        "remainder": rem,
        "page_constants": {
            "stored_page": REVIT_STORED_PAGE_BYTES,
            "payload": REVIT_PAGE_PAYLOAD_BYTES,
            "checksum_tail": REVIT_PAGE_CHECKSUM_BYTES,
        },
        "control": control,
        "experiment_stripped": experiment,
        "ok_chunk_delta": ok_delta,
        "inflated_bytes_delta": bytes_delta,
        "schema": schema,
        "notes": [
            "control = bare inflate on stored CFB bytes (current main callers)",
            "experiment = strip_revit_page_checksums then inflate (Wave 2 candidate)",
            "Positive ok_chunk_delta on multi-member partitions is the primary oracle",
            "Formats/Latest length alone is a weak oracle — prefer schema field parity + synthetic round-trip",
        ],
    }))
}

fn inflate_bundle(bytes: &[u8]) -> Value {
    let at0 = match compression::inflate_at(bytes, 0) {
        Ok(o) => json!({"ok": true, "len": o.len()}),
        Err(e) => json!({"ok": false, "err": e.to_string()}),
    };
    let auto = match compression::inflate_at_auto(bytes) {
        Ok((off, o)) => json!({"ok": true, "offset": off, "len": o.len()}),
        Err(e) => json!({"ok": false, "err": e.to_string()}),
    };
    let chunks = compression::inflate_all_chunks(bytes);
    let total: usize = chunks.iter().map(|c| c.len()).sum();
    json!({
        "inflate_at_0": at0,
        "inflate_at_auto": auto,
        "inflate_all_chunks_ok": chunks.len(),
        "inflate_all_chunks_total": total,
    })
}

fn schema_metrics(stored: &[u8]) -> Value {
    let Ok(decomp) = compression::inflate_at(stored, 0) else {
        return json!({"inflate": "err"});
    };
    let names = rvt::class_index::extract_class_names(&decomp)
        .map(|s| s.len())
        .unwrap_or(0);
    match formats::parse_schema(&decomp) {
        Ok(schema) => {
            let fields: usize = schema.classes.iter().map(|c| c.fields.len()).sum();
            json!({
                "inflate_len": decomp.len(),
                "class_names": names,
                "schema_classes": schema.classes.len(),
                "fields": fields,
                "skipped_records": schema.skipped_records,
            })
        }
        Err(e) => json!({
            "inflate_len": decomp.len(),
            "class_names": names,
            "schema_err": e.to_string(),
        }),
    }
}

fn print_human(r: &Value) {
    if let Some(err) = r.get("error") {
        println!("file={} stream={} ERROR {}", r["file"], r["stream"], err);
        return;
    }
    println!(
        "file={} stream={} stored={} pages={} rem={} removed={} ok_delta={} bytes_delta={}",
        r["file"],
        r["stream"],
        r["stored_len"],
        r["full_pages"],
        r["remainder"],
        r["removed"],
        r["ok_chunk_delta"],
        r["inflated_bytes_delta"],
    );
    println!(
        "  control:   chunks_ok={} total={}",
        r["control"]["inflate_all_chunks_ok"], r["control"]["inflate_all_chunks_total"]
    );
    println!(
        "  stripped:  chunks_ok={} total={}",
        r["experiment_stripped"]["inflate_all_chunks_ok"],
        r["experiment_stripped"]["inflate_all_chunks_total"]
    );
    if !r["schema"].is_null() && r["schema"]["control"].is_object() {
        println!(
            "  schema control:    classes={} fields={} names={}",
            r["schema"]["control"]["schema_classes"],
            r["schema"]["control"]["fields"],
            r["schema"]["control"]["class_names"]
        );
        println!(
            "  schema experiment: classes={} fields={} names={}",
            r["schema"]["experiment"]["schema_classes"],
            r["schema"]["experiment"]["fields"],
            r["schema"]["experiment"]["class_names"]
        );
    }
}
