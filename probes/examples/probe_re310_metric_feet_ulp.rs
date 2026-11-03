//! RE-310: how a metric length is turned into the feet the parameter store
//! holds (#155).
//!
//! FACT under test, as reported in Discussion #112: Revit computes internal
//! feet from millimetres as `mm / 25.4 / 12`, which can differ from
//! `mm / 304.8` by one unit in the last place, so an exact-bit search has to
//! try both. This reads every `f64 feet · ff×8 · i64 parameter` entry of the
//! type value blocks (RE-93, `partition_length_entries`), takes those whose
//! value is a whole or tenth millimetre to within 1e-6 mm, and for each one
//! where the candidate conversions give different bits counts which of them
//! the stored value equals: `div` (`mm / 304.8`), `chain` (`mm / 25.4 / 12`),
//! `mul` (`mm * (1 / 304.8)`), `inch` (`(mm / 25.4) / 12` computed as
//! `mm * (1 / 25.4) / 12`) or `none`.
//!
//! Output:
//!   streams_not_read <n>          partitions the large-model limits still refused
//!   census entries=<n> whole_mm=<n> tenth_mm=<n> agree=<n> disagree=<n>
//!   disagree <candidate set> <n>  per set of candidates the stored value equals
//!   sample owner=<id> parameter=<id> mm=<mm> bits=<hex> ulps_from_div=<n> matches=<set>
//!                                 the first 25 entries where the candidates disagree
//!
//! A file whose `disagree` lines name only `chain`, or only `div`, settles
//! which conversion that release used for the values it holds; one with
//! both names neither.
//!
//! ```text
//! cargo run --profile ci -p rvt-probes --example probe_re310_metric_feet_ulp -- MODEL.rvt
//! ```

use std::collections::{BTreeMap, BTreeSet};

use rvt::RevitFile;
use rvt::research::partition_type_parameters::partition_length_entries;

fn candidates(mm: f64) -> [(&'static str, f64); 4] {
    [
        ("div", mm / 304.8),
        ("chain", mm / 25.4 / 12.0),
        ("mul", mm * (1.0 / 304.8)),
        ("inch", mm * (1.0 / 25.4) / 12.0),
    ]
}

fn ulps(a: f64, b: f64) -> i64 {
    (a.to_bits() as i64).wrapping_sub(b.to_bits() as i64)
}

fn main() -> anyhow::Result<()> {
    let Some(path) = std::env::args().nth(1) else {
        anyhow::bail!("usage: probe_re310_metric_feet_ulp MODEL.rvt");
    };
    let mut rf = RevitFile::open_with_limits(&path, rvt::cli::open_limits(None, None, true))?;
    let declared = rvt::elem_table::declared_ids(&rvt::elem_table::parse_records(&mut rf)?);
    let mut entries = 0usize;
    let mut whole = 0usize;
    let mut tenth = 0usize;
    let mut agree = 0usize;
    let mut disagree: BTreeMap<String, usize> = BTreeMap::new();
    let mut samples = Vec::new();
    let mut seen: BTreeSet<(u32, i64, u64)> = BTreeSet::new();
    for stream in rf.partition_stream_names() {
        let Ok(inflated) = rf.inflated_partition(&stream) else {
            continue;
        };
        for (owner, values) in partition_length_entries(inflated.bytes(), &declared) {
            for (parameter, value) in values {
                if value == 0.0 || !seen.insert((owner, parameter, value.to_bits())) {
                    continue;
                }
                entries += 1;
                let raw = value * 304.8;
                let mm = if (raw - raw.round()).abs() < 1e-6 {
                    whole += 1;
                    raw.round()
                } else if (raw * 10.0 - (raw * 10.0).round()).abs() < 1e-5 {
                    tenth += 1;
                    (raw * 10.0).round() / 10.0
                } else {
                    continue;
                };
                let all = candidates(mm);
                if all.iter().all(|(_, c)| c.to_bits() == all[0].1.to_bits()) {
                    agree += 1;
                    continue;
                }
                let matched: Vec<&str> = all
                    .iter()
                    .filter(|(_, c)| c.to_bits() == value.to_bits())
                    .map(|(name, _)| *name)
                    .collect();
                let key = if matched.is_empty() {
                    "none".to_string()
                } else {
                    matched.join("+")
                };
                *disagree.entry(key.clone()).or_default() += 1;
                if samples.len() < 25 {
                    samples.push(format!(
                        "sample owner={owner} parameter={parameter} mm={mm} bits={:016x} ulps_from_div={} matches={key}",
                        value.to_bits(),
                        ulps(value, all[0].1)
                    ));
                }
            }
        }
    }
    println!("streams_not_read {}", rf.unread_streams().len());
    println!(
        "census entries={entries} whole_mm={whole} tenth_mm={tenth} agree={agree} disagree={}",
        disagree.values().sum::<usize>()
    );
    for (key, n) in &disagree {
        println!("disagree {key} {n}");
    }
    for line in samples {
        println!("{line}");
    }
    Ok(())
}
