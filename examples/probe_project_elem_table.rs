//! Probe Global/ElemTable on real project files — data capture for
//! the L5B-03 dispatch + from_decoded → IFC wiring research thread.

use rvt::{RevitFile, elem_table};

fn main() {
    let files = [
        "/private/tmp/rvt-corpus-probe/magnetar/Revit/Revit_IFC5_Einhoven.rvt",
        "/private/tmp/rvt-corpus-probe/magnetar/Revit/2024_Core_Interior.rvt",
        "/Users/griffin/Developer/re/rvt-recon-2026-04-19/samples/racbasicsamplefamily-2024.rfa",
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
