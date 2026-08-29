//! Evidence probe for Discussion #112 / issue #152:
//! Do trailing words of ElemTable records look like owner ElementIds?
//!
//! Does not invent a decoder — prints histograms and join rates only.

use rvt::{RevitFile, elem_table};
use std::collections::{BTreeMap, BTreeSet};

fn main() {
    let paths = [
        (
            "2023-einhoven-28B",
            "/workspace/_project_corpus/Revit/Revit_IFC5_Einhoven.rvt",
        ),
        (
            "2024-core-40B",
            "/workspace/_project_corpus/Revit/2024_Core_Interior.rvt",
        ),
    ];
    for (label, path) in paths {
        println!("=== {label} ===");
        let mut rf = match RevitFile::open(path) {
            Ok(rf) => rf,
            Err(e) => {
                println!("  open error: {e}");
                continue;
            }
        };
        let header = match elem_table::parse_header(&mut rf) {
            Ok(h) => h,
            Err(e) => {
                println!("  header error: {e}");
                continue;
            }
        };
        let records = match elem_table::parse_records(&mut rf) {
            Ok(r) => r,
            Err(e) => {
                println!("  records error: {e}");
                continue;
            }
        };
        println!(
            "  header element_count={} record_count={} parsed={}",
            header.element_count,
            header.record_count,
            records.len()
        );
        if records.is_empty() {
            continue;
        }
        let stride = records[0].raw.len();
        println!("  stride={stride} B");

        let ids: BTreeSet<u32> = records.iter().map(|r| r.id_primary).collect();
        println!("  unique id_primary={}", ids.len());

        // Sample first 8 records hex
        for r in records.iter().take(8) {
            print!("  id={:<6} raw=", r.id_primary);
            for b in &r.raw {
                print!("{b:02x}");
            }
            println!();
        }

        // Interpret trailing u32 (last 4 bytes) and trailing u64 (last 8)
        let mut trail_u32 = CounterLite::default();
        let mut trail_u32_in_ids = 0usize;
        let mut trail_u32_zero = 0usize;
        let mut trail_u64_zero = 0usize;
        let mut trail_u64_hi_nonzero = 0usize;
        // Also try offset+12..+16 on 28B (first payload word after ids) etc.
        let mut payload_word_in_ids: BTreeMap<usize, usize> = BTreeMap::new();

        for r in &records {
            let raw = &r.raw;
            if raw.len() >= 4 {
                let u = u32::from_le_bytes(raw[raw.len() - 4..].try_into().unwrap());
                trail_u32.bump(u);
                if u == 0 {
                    trail_u32_zero += 1;
                } else if ids.contains(&u) {
                    trail_u32_in_ids += 1;
                }
            }
            if raw.len() >= 8 {
                let lo = u32::from_le_bytes(raw[raw.len() - 8..raw.len() - 4].try_into().unwrap());
                let hi = u32::from_le_bytes(raw[raw.len() - 4..].try_into().unwrap());
                if lo == 0 && hi == 0 {
                    trail_u64_zero += 1;
                }
                if hi != 0 {
                    trail_u64_hi_nonzero += 1;
                }
            }
            // Every aligned u32 in the record after the marker
            let marker = if stride == 28 { 4 } else if stride == 40 { 8 } else { 0 };
            let body = &raw[marker..];
            for (i, chunk) in body.chunks_exact(4).enumerate() {
                let v = u32::from_le_bytes(chunk.try_into().unwrap());
                if v != 0 && v != r.id_primary && v != r.id_secondary && ids.contains(&v) {
                    *payload_word_in_ids.entry(i).or_default() += 1;
                }
            }
        }

        let n = records.len();
        println!(
            "  trailing u32: zero={trail_u32_zero}/{} ({:.1}%), in_id_set={trail_u32_in_ids}/{} ({:.1}%)",
            n,
            100.0 * trail_u32_zero as f64 / n as f64,
            n,
            100.0 * trail_u32_in_ids as f64 / n as f64
        );
        println!("  trailing u32 top values: {}", trail_u32.top(8));
        println!(
            "  trailing u64: both_zero={trail_u64_zero}/{} hi_nonzero={trail_u64_hi_nonzero}/{}",
            n, n
        );
        println!("  payload u32 words that are other declared ids (by word index after marker):");
        for (idx, count) in &payload_word_in_ids {
            println!(
                "    word[{idx}]: {count}/{} ({:.1}%)",
                n,
                100.0 * *count as f64 / n as f64
            );
        }
        // Tree-ish check: how many distinct non-zero trailing u32s, and is there a single root (id never appears as owner)?
        let owners: BTreeSet<u32> = trail_u32
            .counts
            .keys()
            .copied()
            .filter(|v| *v != 0)
            .collect();
        let owned_as_id: usize = records
            .iter()
            .filter(|r| owners.contains(&r.id_primary))
            .count();
        println!(
            "  distinct nonzero trailing-u32 values={}; records whose id_primary appears as some trailing-u32={owned_as_id}",
            owners.len()
        );
    }
}

#[derive(Default)]
struct CounterLite {
    counts: BTreeMap<u32, usize>,
}
impl CounterLite {
    fn bump(&mut self, v: u32) {
        *self.counts.entry(v).or_default() += 1;
    }
    fn top(&self, n: usize) -> String {
        let mut items: Vec<_> = self.counts.iter().collect();
        items.sort_by(|a, b| b.1.cmp(a.1).then(a.0.cmp(b.0)));
        items
            .into_iter()
            .take(n)
            .map(|(k, v)| format!("{k:#x}:{v}"))
            .collect::<Vec<_>>()
            .join(", ")
    }
}
