//! Local research for #156: inventory a standalone family's revolution,
//! sketch, curve and plane records, with schema-decoded values and byte spans.
//!
//! RE-344 establishes the occurrence of -1001802/-1001803. This probe retains
//! their raw values without assuming their angular origin or handedness.
//! It does not reconstruct a solid or declare a geometric match. Run it on
//! the authored rectangular families and compare the profile/axis records
//! and angle values across controlled changes, then with Revit's export.
//!
//! Offsets are zero-based. `inflated_offset` refers to the concatenated
//! inflated partition (rvt-dump's .decomp), not to the OLE file. When absent,
//! use the prepared-stream marker and reassembled-group body offset instead.
//! Repeated saved records are retained, not silently treated as current.
//!
//! cargo run --profile ci -p rvt-probes --example probe_family_revolutions -- FILE.rfa
//! Optional --max-records N bounds selected records (default 2000).
//! JSON goes to stdout; a framing failure or resource limit exits nonzero.

use std::collections::{BTreeMap, BTreeSet};
use std::io::{Read, Write};
use std::path::Path;

use anyhow::{Context, Result, ensure};
use rvt::RevitFile;
use rvt::research::{native_form_geometry, native_parameters};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

const CLASSES: [&str; 4] = ["RevolutionElem", "CurveElem", "SketchPlane", "VarSketch"];
const LIMITS: native_parameters::GraphLimits = native_parameters::GraphLimits {
    max_values: 200_000,
    max_objects: 100_000,
    max_depth: 128,
};
const MAX_FIELD_BYTES: usize = 4096;

fn fingerprint(path: &Path) -> Result<String> {
    let mut file = std::fs::File::open(path)?;
    let mut hash = Sha256::new();
    let mut buffer = [0u8; 65536];
    loop {
        let n = file.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        hash.update(&buffer[..n]);
    }
    Ok(hex(&hash.finalize()))
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn identifier(value: &Value) -> Option<i64> {
    let inner = value.get("m_id")?;
    inner.as_i64().or_else(|| {
        inner
            .get("m_id64")
            .or_else(|| inner.get("m_id"))
            .and_then(Value::as_i64)
    })
}

fn relevant(class: &str) -> bool {
    class.contains("Revolution")
        || class.contains("Sketch")
        || class.contains("Curve")
        || matches!(
            class,
            "GLine" | "GArc" | "GEllipse" | "Plane" | "Trf" | "ParamValueSetDouble"
        )
}

fn main() -> Result<()> {
    let mut args = std::env::args().skip(1);
    let path = args
        .next()
        .context("usage: probe_family_revolutions FILE.rfa [--max-records N]")?;
    let mut max_records = 2000usize;
    if let Some(flag) = args.next() {
        ensure!(flag == "--max-records", "unknown argument: {flag}");
        max_records = args.next().context("missing record limit")?.parse()?;
    }
    ensure!(args.next().is_none(), "unexpected argument");
    ensure!(
        (1..=20_000).contains(&max_records),
        "record limit must be 1..20000"
    );
    let path = Path::new(&path);
    ensure!(
        path.extension()
            .is_some_and(|x| x.eq_ignore_ascii_case("rfa")),
        "first-stage probe accepts standalone .rfa only"
    );
    let before = fingerprint(path)?;
    let mut file = RevitFile::open_with_limits(path, rvt::cli::open_limits(None, None, true))?;
    let document = native_form_geometry::open(&mut file)?;
    let mut records = Vec::new();
    let mut counts = BTreeMap::<String, usize>::new();
    let mut revolutions = BTreeSet::new();
    let walked = native_form_geometry::walk(&mut file, &document, &[102], |entry| {
        let class = entry.class.unwrap_or("unknown");
        if !CLASSES.contains(&class) {
            return Ok(());
        }
        ensure!(
            records.len() < max_records,
            "selected record limit exceeded; narrow the input or increase --max-records"
        );
        *counts.entry(class.to_string()).or_default() += 1;
        if class == "RevolutionElem" {
            revolutions.insert(entry.id);
        }
        let mut record = json!({
            "element_id":entry.id,"class":class,"stream":entry.stream,"channel":entry.channel,
            "body_bytes":entry.body.len(),"body_sha256":hex(&Sha256::digest(entry.body)),
            "body_inflated_offset":entry.body_offset,"group_marker_offset":entry.group_marker_offset,
            "group_body_offset":entry.group_body_offset
        });
        match native_parameters::decode_graph_with_limits(entry.body, &document.registry, &LIMITS) {
            Err(error) => {
                record["decode_error"] = json!(format!("{error:#}"));
            }
            Ok(graph) => {
                let mut objects = Vec::new();
                let mut angle_values = Vec::new();
                for (index, object) in graph.objects.iter().enumerate() {
                    if object.class_name == "ParamValueSetDouble" {
                        for item in object
                            .fields
                            .get("m_paramSet")
                            .and_then(Value::as_array)
                            .into_iter()
                            .flatten()
                        {
                            if let Some(id @ (-1_001_802 | -1_001_803)) =
                                item.get("m_paramId").and_then(identifier)
                            {
                                let value = item.get("m_value").and_then(Value::as_f64);
                                angle_values.push(json!({"parameter_id":id,"value":value,"decoded_f64_le_hex":value.map(|v| hex(&v.to_le_bytes())),"object_index":index}));
                            }
                        }
                    }
                    if !relevant(&object.class_name) {
                        continue;
                    }
                    let decoded = native_parameters::decode_object_fields_spanning(
                        entry.body,
                        &document.registry,
                        object.start,
                        object.class_tag,
                        &LIMITS,
                        native_parameters::Spans::All,
                    )?;
                    let mut spans = Vec::new();
                    for span in &decoded.field_spans {
                        let bytes = entry
                            .body
                            .get(span.start..span.end)
                            .context("field span outside owning body")?;
                        let prefix = &bytes[..bytes.len().min(MAX_FIELD_BYTES)];
                        spans.push(json!({"declaring_class_tag":span.class_tag,"field":span.name,
                            "body_start":span.start,"body_end":span.end,
                            "inflated_offset":entry.body_offset.and_then(|b|b.checked_add(span.start)),
                            "raw_hex":hex(prefix),"raw_truncated":prefix.len()!=bytes.len()}));
                    }
                    objects.push(json!({"object_index":index,"class":object.class_name,
                        "class_tag":object.class_tag,"token":object.token,
                        "body_start":object.start,"fields_end":object.fields_end,
                        "fields":object.fields,"spans":spans}));
                }
                record["graph_object_index"] = json!(graph.objects.iter().enumerate().map(|(index, object)|
                    json!({"object_index":index,"class":object.class_name,"class_tag":object.class_tag,
                        "token":object.token,"body_start":object.start,"fields_end":object.fields_end})
                ).collect::<Vec<_>>());
                record["graph_edges"] = json!(graph.edges);
                record["graph_consumed_bytes"] = json!(graph.consumed_bytes);
                record["objects"] = json!(objects);
                record["angle_parameter_values"] = json!(angle_values);
                record["decode_status"] = json!("decoded");
            }
        }
        records.push(record);
        Ok(())
    })?;
    ensure!(
        walked.unframed_groups == 0,
        "{} groups did not frame; evidence is incomplete",
        walked.unframed_groups
    );
    let mut graphics = Vec::new();
    let graphics_walk = native_form_geometry::walk(&mut file, &document, &[103], |entry| {
        if entry.class == Some("GElement") && revolutions.contains(&entry.id) {
            ensure!(
                graphics.len() < max_records,
                "saved graphics record limit exceeded"
            );
            graphics.push(json!({"element_id":entry.id,"stream":entry.stream,
                "body_inflated_offset":entry.body_offset,"body_bytes":entry.body.len(),
                "group_marker_offset":entry.group_marker_offset,"group_body_offset":entry.group_body_offset,
                "body_sha256":hex(&Sha256::digest(entry.body))}));
        }
        Ok(())
    })?;
    ensure!(
        graphics_walk.unframed_groups == 0,
        "saved graphics groups did not frame"
    );
    ensure!(before == fingerprint(path)?, "source changed during probe");
    let failures = records
        .iter()
        .filter(|r| r.get("decode_error").is_some())
        .count();
    let output = json!({"schema":"family-revolution-evidence-v1",
        "file":path.file_name().unwrap_or_default().to_string_lossy(),"file_sha256":before,
        "release":document.release,"status":if failures==0 {"observed"} else {"partial_decode"},
        "scope":"standalone saved records, copies retained; no solid reconstruction or match verdict",
        "records_by_class":counts,"unique_revolution_ids":revolutions.len(),
        "decode_failures":failures,"records":records,"saved_graphics_records":graphics,
        "walk":{"groups":walked.groups,"keyed_groups_skipped":walked.keyed_groups,"unframed_groups":walked.unframed_groups,"records":walked.records},
        "solid_reconstruction":"not_attempted","geometry_match":null});
    let mut stdout = std::io::stdout().lock();
    serde_json::to_writer_pretty(&mut stdout, &output)?;
    writeln!(stdout)?;
    Ok(())
}
