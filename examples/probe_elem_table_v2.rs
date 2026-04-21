//! Verify the corrected `parse_records` against the 3-file corpus.

use rvt::{RevitFile, elem_table};

fn main() {
    let files = [
        (
            "family-2024",
            "/private/tmp/rvt-corpus-probe/racbasicsamplefamily-2024.rfa",
        ),
        (
            "project-2023",
            "/private/tmp/rvt-corpus-probe/magnetar/Revit/Revit_IFC5_Einhoven.rvt",
        ),
        (
            "project-2024",
            "/private/tmp/rvt-corpus-probe/magnetar/Revit/2024_Core_Interior.rvt",
        ),
    ];

    for (label, path) in files {
        let mut rf = match RevitFile::open(path) {
            Ok(rf) => rf,
            Err(e) => {
                println!("{label}: open error — {e}");
                continue;
            }
        };
        let header = match elem_table::parse_header(&mut rf) {
            Ok(h) => h,
            Err(e) => {
                println!("{label}: header error — {e}");
                continue;
            }
        };
        let records = match elem_table::parse_records(&mut rf) {
            Ok(r) => r,
            Err(e) => {
                println!("{label}: records error — {e}");
                continue;
            }
        };
        println!(
            "{label}: header element_count={}, record_count={}, decompressed={} B",
            header.element_count, header.record_count, header.decompressed_bytes
        );
        println!(
            "  parsed {} records, first 5 ids: {:?}",
            records.len(),
            records
                .iter()
                .take(5)
                .map(|r| (r.id_primary, r.id_secondary, r.offset))
                .collect::<Vec<_>>()
        );
        println!(
            "  last 3 records: {:?}",
            records
                .iter()
                .rev()
                .take(3)
                .map(|r| (r.id_primary, r.id_secondary, r.offset))
                .collect::<Vec<_>>()
        );
        let id_primaries: Vec<u32> = records.iter().map(|r| r.id_primary).collect();
        let monotonic = id_primaries.windows(2).all(|w| w[1] >= w[0]);
        let unique: std::collections::HashSet<_> = id_primaries.iter().collect();
        println!(
            "  id_primary monotonic={monotonic}, unique_count={} / {}",
            unique.len(),
            records.len()
        );
    }
}
