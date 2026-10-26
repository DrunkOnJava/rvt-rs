//! #421, item 1: the element-record marker each file's schema gives, against
//! the release's measured constant.
//!
//! One JSON object per file: `release`, `schema` (the marker from
//! `Outline` and `ElementParents`, RE-80), `constant` (`bbox_marker`),
//! `header_tag` (the schema's `ElementHeader`) and `prologue`
//! (`record_prologue_constant` of the schema's marker), and `scanned`, the
//! marker `file_bbox_marker` hands the scans.
//!
//! Usage:
//!   cargo run --profile ci --example probe_421_schema_marker -- FILE.rvt ...

use rvt::RevitFile;
use rvt::research::partition_element_records as per;
use rvt::research::partition_element_records_2023::record_marker;

fn hex(marker: Option<[u8; 8]>) -> String {
    marker.map_or_else(
        || "null".into(),
        |m| format!("\"{}\"", m.map(|b| format!("{b:02x}")).join(" ")),
    )
}

fn main() {
    for path in std::env::args().skip(1) {
        let file = std::path::Path::new(&path)
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        let mut rf = match RevitFile::open(&path) {
            Ok(rf) => rf,
            Err(e) => {
                println!("{{\"file\":{file:?},\"error\":{:?}}}", e.to_string());
                continue;
            }
        };
        let release = rf.basic_file_info().map(|i| i.version).unwrap_or(0);
        let schema = record_marker(&mut rf);
        let marker = schema.map(|(m, _)| m);
        println!(
            "{{\"file\":{file:?},\"release\":{release},\"schema\":{},\"constant\":{},\
             \"header_tag\":{},\"prologue\":{},\"scanned\":{}}}",
            hex(marker),
            hex(per::bbox_marker(release)),
            schema.map_or_else(|| "null".into(), |(_, t)| t.to_string()),
            marker.map_or_else(
                || "null".into(),
                |m| per::record_prologue_constant(&m).to_string()
            ),
            hex(per::file_bbox_marker(&mut rf, release)),
        );
    }
}
