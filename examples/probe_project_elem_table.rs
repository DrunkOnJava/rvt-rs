//! Probe Global/ElemTable on real project files — data capture for
//! the L5B-03 dispatch + from_decoded → IFC wiring research thread.

use rvt::{RevitFile, elem_table};

fn main() {
    // Paths resolved via env vars so the probe doesn't leak any
    // contributor's home directory into the repo (CI PII guard).
    let project_dir = std::env::var("RVT_PROJECT_CORPUS_DIR")
        .unwrap_or_else(|_| "/private/tmp/rvt-corpus-probe/magnetar/Revit".into());
    let family_path = std::env::var("RVT_FAMILY_2024").unwrap_or_else(|_| {
        format!(
            "{}/samples/racbasicsamplefamily-2024.rfa",
            std::env::var("RVT_SAMPLES_DIR").unwrap_or_else(|_| "../../samples".into())
        )
    });
    let project_2023 = format!("{project_dir}/Revit_IFC5_Einhoven.rvt");
    let project_2024 = format!("{project_dir}/2024_Core_Interior.rvt");
    let files = [
        project_2023.as_str(),
        project_2024.as_str(),
        family_path.as_str(),
    ];
    for path in files {
        let mut rf = RevitFile::open(path).unwrap();
        let name = path.rsplit('/').next().unwrap();
        let hdr = match elem_table::parse_header(&mut rf) {
            Ok(h) => h,
            Err(e) => {
                println!("{}: header error: {}", name, e);
                continue;
            }
        };
        println!("=== {} ===", name);
        println!(
            "  header: {} elements, {} records, flag=0x{:04x}, {} decomp bytes",
            hdr.element_count, hdr.record_count, hdr.header_flag, hdr.decompressed_bytes
        );
        let records = elem_table::parse_records_rough(&mut rf, 5000).unwrap_or_default();
        println!("  rough records parsed: {}", records.len());
        if !records.is_empty() {
            let sample: Vec<_> = records.iter().take(5).collect();
            for (i, r) in sample.iter().enumerate() {
                println!(
                    "    [{}] off=0x{:04x} triple=[{:10}, {:10}, {:10}]",
                    i,
                    r.offset,
                    r.presumptive_u32_triple[0],
                    r.presumptive_u32_triple[1],
                    r.presumptive_u32_triple[2]
                );
            }
        }
    }
}
