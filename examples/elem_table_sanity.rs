//! ElemTable sanity-check: cross-verify the header's declared record
//! count against what parse_records_rough actually returns. Look at
//! patterns in the presumptive_u32_triple to form hypotheses.

use rvt::{RevitFile, elem_table};
use std::path::PathBuf;

fn main() -> anyhow::Result<()> {
    let sample_dir = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "samples/_phiag/examples/Autodesk".into());

    for year in [2016, 2020, 2024, 2026] {
        for filename in [
            format!("racbasicsamplefamily-{year}.rfa"),
            format!("rac_basic_sample_family-{year}.rfa"),
        ] {
            let path = PathBuf::from(&sample_dir).join(&filename);
            if !path.exists() {
                continue;
            }
            let mut rf = RevitFile::open(&path)?;
            let header = elem_table::parse_header(&mut rf)?;
            let records = elem_table::parse_records_rough(&mut rf, 100_000)?;

            println!("═══ Revit {year} ═══");
            println!("  declared element_count: {}", header.element_count);
            println!("  declared record_count:  {}", header.record_count);
            println!("  rough records returned: {}", records.len());
            println!("  decompressed bytes:     {}", header.decompressed_bytes);

            // Histogram: how often does each class-tag-like value (u32 < 0x4000)
            // appear in the presumptive_u32_triple?
            let mut tag_candidates: std::collections::BTreeMap<u32, u32> =
                std::collections::BTreeMap::new();
            for r in &records {
                for v in r.presumptive_u32_triple {
                    if v > 0 && v < 0x4000 {
                        *tag_candidates.entry(v).or_insert(0) += 1;
                    }
                }
            }
            let mut sorted: Vec<_> = tag_candidates.iter().collect();
            sorted.sort_by(|a, b| b.1.cmp(a.1));
            print!("  top 10 u32-in-tag-range values: ");
            for (v, c) in sorted.iter().take(10) {
                print!("0x{:04x}×{} ", v, c);
            }
            println!();

            // First 3 records
            for (i, r) in records.iter().take(3).enumerate() {
                println!(
                    "  record {i} @0x{:x}: [0x{:08x}, 0x{:08x}, 0x{:08x}]",
                    r.offset,
                    r.presumptive_u32_triple[0],
                    r.presumptive_u32_triple[1],
                    r.presumptive_u32_triple[2]
                );
            }
            println!();
            break;
        }
    }
    Ok(())
}
