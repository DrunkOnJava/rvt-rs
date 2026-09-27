//! FACT (#410, Discussion #112 F1): `Formats/Latest` is checksum-paged like
//! the other database streams. Inflated without the page strip it drifts
//! after the first page boundary (~140-180 KB out) while still ending on a
//! clean BFINAL; with the strip, every file of one Revit release yields the
//! byte-identical schema, family or project.
//!
//! Verify: run over several files of one release and compare the `sha256`
//! column of the `stripped` rows (identical) with the `raw` rows (differ
//! between a family and a project):
//!
//!   cargo run --profile ci --example probe_schema_page_strip -- A.rvt B.rfa ...
//!
//! Each row also reports what `formats::parse_schema_with_scan_limit` gets
//! from the first 64 KB (the historical `SCHEMA_SCAN_LIMIT`) and from the
//! whole stream: classes, fields, and fields still `FieldType::Unknown`.
use rvt::compression;
use rvt::formats;
use sha2::{Digest, Sha256};
use std::env;
use std::io::Read;

fn main() {
    for path in env::args().skip(1) {
        let mut file = cfb::open(&path).expect("open");
        let mut raw = Vec::new();
        file.open_stream("Formats/Latest")
            .expect("Formats/Latest")
            .read_to_end(&mut raw)
            .expect("read");
        let stripped = compression::strip_revit_page_checksums(&raw);
        let name = path.rsplit('/').next().unwrap_or(&path);
        for (label, bytes) in [("raw", raw.as_slice()), ("stripped", stripped.as_slice())] {
            let d = compression::inflate_at(bytes, 0).expect("inflate");
            let hash: String = Sha256::digest(&d)[..8]
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect();
            let mut cells = Vec::new();
            for limit in [formats::SCHEMA_SCAN_LIMIT, usize::MAX] {
                let s = formats::parse_schema_with_scan_limit(&d, limit).expect("schema");
                let fields: usize = s.classes.iter().map(|c| c.fields.len()).sum();
                let unknown = s
                    .classes
                    .iter()
                    .flat_map(|c| &c.fields)
                    .filter(|f| matches!(f.field_type, Some(formats::FieldType::Unknown { .. })))
                    .count();
                cells.push(format!("{}/{fields}/{unknown}", s.classes.len()));
            }
            if label == "stripped" && env::var_os("SHOW_UNKNOWN").is_some() {
                let s = formats::parse_schema_with_scan_limit(&d, usize::MAX).expect("schema");
                for c in &s.classes {
                    for f in &c.fields {
                        if let Some(formats::FieldType::Unknown { bytes }) = &f.field_type {
                            let hex: String = bytes.iter().map(|b| format!("{b:02x}")).collect();
                            println!("  unknown {}::{} {hex}", c.name, f.name);
                        }
                    }
                }
            }
            println!(
                "{name}\t{label}\tinflated={}\tsha256={hash}\t64KB classes/fields/unknown={}\tfull={}",
                d.len(),
                cells[0],
                cells[1]
            );
        }
    }
}
