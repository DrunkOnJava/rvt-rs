//! RE-338, #1165: are the records of a wall cut the same in the BIM4LCA
//! office as built and at building permit?
//!
//! FACT under test: Revit's export of one office file omits an opening
//! (a curtain wall's, a cabinet's, a door's in a second wall) that its
//! export of the other file holds, and the records behind the opening are
//! byte for byte the other file's. This prints a digest of each wall's
//! native record body, of each step of its record that names a cutting
//! element (its fields), of the cutter's native record body and of the
//! partition element records of the wall and the cutter, for every such
//! step in the file, so that two files' outputs can be compared by
//! (cutter, wall).
//!
//! Output, one record per line:
//!   step <wall> <index> <step class> <cutter> <digest of the step's fields>
//!   wall <wall> <body length> <digest of the body>
//!   cutter <cutter> <class> <body length> <digest of the body>
//!   prec <id> <stream> <length> <digest of the record's bytes>
//!
//! Usage:
//!   cargo run --profile ci -p rvt-probes --example probe_re338_sibling_records -- MODEL.rvt

use std::collections::BTreeSet;

use anyhow::Result;
use rvt::RevitFile;
use rvt::research::native_document::{Options, Record, extract_graphs};
use rvt::research::partition_element_records as per;
use serde_json::Value;
use sha2::{Digest, Sha256};

const STEPS: [&str; 4] = [
    "WallEmbedWallGStep",
    "WallCutoutGStep",
    "WallRectOpeningGStep",
    "WallShadowCutoutGStep",
];

fn digest(bytes: &[u8]) -> String {
    let digits: Vec<String> = Sha256::digest(bytes)
        .iter()
        .take(10)
        .map(|b| format!("{b:02x}"))
        .collect();
    digits.join("")
}

fn probe(path: &str) -> Result<()> {
    let mut rf = RevitFile::open(path)?;
    let mut cutters: BTreeSet<u64> = BTreeSet::new();
    let mut walls: BTreeSet<u64> = BTreeSet::new();
    let options = Options {
        selected_classes: BTreeSet::from(["SWall".to_string()]),
        keep_body: true,
        ..Options::default()
    };
    extract_graphs(&mut rf, &options, |record: Record| {
        let Some(graph) = record.graph.as_ref() else {
            return Ok(());
        };
        let wall = record.identity.element_id;
        let mut named = false;
        for (index, object) in graph.objects.iter().enumerate() {
            if !STEPS.contains(&object.class_name.as_str()) {
                continue;
            }
            let Some(cutter) = object
                .fields
                .pointer("/m_instId/m_id/m_id64")
                .and_then(Value::as_u64)
            else {
                continue;
            };
            named = true;
            cutters.insert(cutter);
            println!(
                "step {wall} {index} {} {cutter} {}",
                object.class_name,
                digest(object.fields.to_string().as_bytes())
            );
        }
        if named {
            walls.insert(wall);
            let body = record.body.as_deref().unwrap_or_default();
            println!("wall {wall} {} {}", body.len(), digest(body));
        }
        Ok(())
    })?;
    let options = Options {
        selected_ids: cutters.clone(),
        keep_body: true,
        ..Options::default()
    };
    extract_graphs(&mut rf, &options, |record: Record| {
        let body = record.body.as_deref().unwrap_or_default();
        println!(
            "cutter {} {} {} {}",
            record.identity.element_id,
            record.class_name.as_deref().unwrap_or("-"),
            body.len(),
            digest(body)
        );
        Ok(())
    })?;
    let wanted: BTreeSet<u64> = cutters.union(&walls).copied().collect();
    let revit = rf.basic_file_info()?.version;
    if let Some(marker) = per::file_bbox_marker(&mut rf, revit) {
        for stream in rf.partition_stream_names() {
            let Ok(inflated) = rf.inflated_partition(&stream) else {
                continue;
            };
            let buf = inflated.bytes();
            for span in per::partition_record_chain(buf, &marker) {
                if !wanted.contains(&span.element_id) {
                    continue;
                }
                let Some(bytes) = buf.get(span.start..span.end) else {
                    continue;
                };
                println!(
                    "prec {} {stream} {} {}",
                    span.element_id,
                    bytes.len(),
                    digest(bytes)
                );
            }
        }
    }
    Ok(())
}

fn main() {
    let paths: Vec<String> = std::env::args()
        .skip(1)
        .filter(|arg| !arg.starts_with("--"))
        .collect();
    for path in &paths {
        if let Err(error) = probe(path) {
            println!("error {path}: {error:#}");
        }
    }
}
