//! RE-11 main pass — per-tag signal-over-noise analysis for partition chunks.
//!
//! Prior probe `probe_partition_tag_density` showed ~20 tag hits/KB in
//! concatenated partition chunks on real files. RE-09 F13 concluded
//! this is noisy (most hits are random u16 coincidences), but F14
//! flagged `HostObjAttr` (0x006b) as a legitimate high-frequency
//! signal (5600 hits in Einhoven Partitions/0).
//!
//! This probe distinguishes signal from noise systematically:
//!
//! - For each schema class with a tag, compute:
//!
//!   ```text
//!   observed = count of u16-LE positions in partition where bytes match the tag
//!   expected = partition_u16_position_count / 65536
//!   ratio    = observed / expected
//!   ```
//!
//! - Tags with ratio >> 1.0 are non-random. They likely appear as
//!   part of a real on-disk structure (class markers, record
//!   headers, element-type discriminators).
//!
//! - Tags with ratio ≈ 1.0 are byte-coincidence noise.
//!
//! - Tags with ratio << 1.0 either don't exist in the partition
//!   or appear at specific excluded positions — also information.
//!
//! The output is a ranked table per partition: tag, class name,
//! observed count, expected count, signal ratio, log10 ratio. The
//! top of the table is where real element-type discriminators live.

use rvt::{RevitFile, compression, formats, streams};
use std::collections::BTreeMap;

fn main() {
    let project_dir = std::env::var("RVT_PROJECT_CORPUS_DIR")
        .unwrap_or_else(|_| "/private/tmp/rvt-corpus-probe/magnetar/Revit".into());

    let targets: Vec<(&str, Vec<&str>)> = vec![
        (
            "Revit_IFC5_Einhoven.rvt",
            vec!["Partitions/0", "Partitions/5"],
        ),
        ("2024_Core_Interior.rvt", vec!["Partitions/46"]),
    ];

    for (file, partition_list) in &targets {
        let path = format!("{project_dir}/{file}");
        let Ok(mut rf) = RevitFile::open(&path) else {
            continue;
        };
        let Ok(formats_raw) = rf.read_stream(streams::FORMATS_LATEST) else {
            continue;
        };
        let Ok(formats_d) = compression::inflate_at(&formats_raw, 0) else {
            continue;
        };
        let Ok(schema) = formats::parse_schema(&formats_d) else {
            continue;
        };
        let tag_to_name: BTreeMap<u16, &str> = schema
            .classes
            .iter()
            .filter_map(|c| c.tag.map(|t| (t, c.name.as_str())))
            .collect();

        println!("\n=== {file}: {} tagged classes ===", tag_to_name.len());

        for s in partition_list {
            let Ok(raw) = rf.read_stream(s) else {
                continue;
            };
            let chunks = compression::inflate_all_chunks(&raw);
            let concat: Vec<u8> = chunks.into_iter().flatten().collect();
            if concat.len() < 4 {
                continue;
            }

            // Count every u16-LE position.
            let positions = concat.len().saturating_sub(1);
            let mut counts: BTreeMap<u16, usize> = BTreeMap::new();
            for i in 0..positions {
                let v = u16::from_le_bytes([concat[i], concat[i + 1]]);
                *counts.entry(v).or_insert(0) += 1;
            }

            // Baseline = positions / 65536 (uniform-random expectation
            // for any specific u16).
            let expected = (positions as f64) / 65536.0;

            // Build signal-over-noise table for tagged classes only.
            let mut rows: Vec<(u16, &str, usize, f64, f64)> = Vec::new();
            let mut total_tag_hits = 0usize;
            for (&tag, &name) in &tag_to_name {
                let observed = counts.get(&tag).copied().unwrap_or(0);
                total_tag_hits += observed;
                let ratio = if expected > 0.0 {
                    observed as f64 / expected
                } else {
                    0.0
                };
                let log10_ratio = if observed > 0 {
                    (observed as f64 / expected).log10()
                } else {
                    f64::NEG_INFINITY
                };
                rows.push((tag, name, observed, expected, log10_ratio));
                let _ = ratio;
            }
            // Sort: highest log10_ratio first (strongest signal first).
            rows.sort_by(|a, b| b.4.partial_cmp(&a.4).unwrap_or(std::cmp::Ordering::Equal));

            println!(
                "\n  {s}: {} B decomp, {} u16 positions, random-expected {:.1}/tag, \
                 {} total tag hits across all tagged classes",
                concat.len(),
                positions,
                expected,
                total_tag_hits
            );

            // Top signal:
            println!(
                "\n    {:^8} {:<30} {:>8} {:>10} {:>10}",
                "tag", "class", "observed", "expected", "log10(o/e)"
            );
            println!("    {}", "-".repeat(72));
            for (tag, name, observed, exp, lg) in rows.iter().take(15) {
                if *observed == 0 {
                    continue;
                }
                println!(
                    "    0x{tag:04x}   {name:<30} {observed:>8} {:>10.1} {:>10.2}",
                    exp, lg
                );
            }

            // Noise-floor: tags with ratio ≈ 1.0
            let noise_floor_count = rows
                .iter()
                .filter(|(_, _, obs, exp, _)| {
                    *obs > 0 && (*obs as f64 / *exp).abs() > 0.5 && (*obs as f64 / *exp).abs() < 2.0
                })
                .count();
            let strong_signal_count = rows
                .iter()
                .filter(|(_, _, obs, exp, _)| *obs > 0 && (*obs as f64 / *exp) > 10.0)
                .count();
            let absent_count = rows.iter().filter(|(_, _, obs, _, _)| *obs == 0).count();
            println!(
                "\n    Signal census: {strong_signal_count} strong (>10× random), \
                 {noise_floor_count} noise (0.5×-2× random), {absent_count} absent"
            );
        }
    }
}
