//! RE-310: how a metric length is turned into the feet the parameter store
//! holds (#155).
//!
//! FACT under test, as reported in Discussion #112: Revit computes internal
//! feet from millimetres as `mm / 25.4 / 12`, which can differ from
//! `mm / 304.8` by one unit in the last place, so an exact-bit search has to
//! try both. This reads every `f64 feet · ff×8 · i64 parameter` entry of the
//! type value blocks (RE-93, `partition_length_entries`), takes those whose
//! value is a whole or tenth millimetre to within 1e-6 mm and is not a whole
//! number of 1/256 inches (an imperial value, which `div` reproduces from its
//! millimetres trivially: 304.8 mm is 1 ft exactly), and for each one where
//! the candidate conversions give different bits counts which of them
//! the stored value equals: `div` (`mm / 304.8`), `chain` (`mm / 25.4 / 12`),
//! `mul` (`mm * (1 / 304.8)`), `inch` (`(mm / 25.4) / 12` computed as
//! `mm * (1 / 25.4) / 12`) or `none`.
//!
//! Output:
//!   streams_not_read <n>          partitions the large-model limits still refused
//!   census entries=<n> whole_mm=<n> tenth_mm=<n> imperial=<n> agree=<n> disagree=<n>
//!   disagree <candidate set> <n>  per set of candidates the stored value equals
//!   sample owner=<id> parameter=<id> mm=<mm> bits=<hex> ulps_from_div=<n> matches=<set>
//!                                 the first 25 entries where the candidates disagree
//!   kind <kind> <candidate set> <n>
//!                                 the same, by what the entry's i64 is: `builtin` (a
//!                                 negative BuiltInParameter id), `declared` (a declared
//!                                 ElementId, a family or shared parameter) or `other`
//!   param <id> div_only=<n> chain_only=<n>
//!                                 per builtin or declared parameter, entries matching
//!                                 `div` and not `chain`, and the reverse; the 20 with
//!                                 most `chain_only`, then the 20 with most `div_only`
//!
//! A candidate that every differing value of a file equals would be that
//! file's conversion. Read the `kind builtin` and `kind declared` lines for
//! parameters: `kind other` entries' i64 is no parameter id.
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
    let mut imperial = 0usize;
    let mut agree = 0usize;
    let mut disagree: BTreeMap<String, usize> = BTreeMap::new();
    let mut samples = Vec::new();
    let mut seen: BTreeSet<(u32, i64, u64)> = BTreeSet::new();
    let mut by_kind: BTreeMap<(&'static str, String), usize> = BTreeMap::new();
    let mut by_param: BTreeMap<i64, [usize; 2]> = BTreeMap::new();
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
                let steps = value * 3072.0;
                if (steps - steps.round()).abs() < 1e-6 {
                    imperial += 1;
                    continue;
                }
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
                let kind = if (-10_000_000..0).contains(&parameter) {
                    "builtin"
                } else if u32::try_from(parameter).is_ok_and(|id| declared.contains(&id)) {
                    "declared"
                } else {
                    "other"
                };
                *by_kind.entry((kind, key.clone())).or_default() += 1;
                if kind != "other" {
                    let div = matched.contains(&"div");
                    let chain = matched.contains(&"chain");
                    let slot = by_param.entry(parameter).or_default();
                    if div && !chain {
                        slot[0] += 1;
                    }
                    if chain && !div {
                        slot[1] += 1;
                    }
                }
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
        "census entries={entries} whole_mm={whole} tenth_mm={tenth} imperial={imperial} agree={agree} disagree={}",
        disagree.values().sum::<usize>()
    );
    for (key, n) in &disagree {
        println!("disagree {key} {n}");
    }
    for line in samples {
        println!("{line}");
    }
    for ((kind, key), n) in &by_kind {
        println!("kind {kind} {key} {n}");
    }
    let mut params: Vec<(i64, [usize; 2])> = by_param.into_iter().collect();
    params.sort_by_key(|(id, [_, chain])| (std::cmp::Reverse(*chain), *id));
    let mut shown: BTreeSet<i64> = BTreeSet::new();
    for (id, [div, chain]) in params.iter().filter(|(_, [_, c])| *c > 0).take(20) {
        shown.insert(*id);
        println!("param {id} div_only={div} chain_only={chain}");
    }
    params.sort_by_key(|(id, [div, _])| (std::cmp::Reverse(*div), *id));
    for (id, [div, chain]) in params
        .iter()
        .filter(|(id, [d, _])| *d > 0 && !shown.contains(id))
        .take(20)
    {
        println!("param {id} div_only={div} chain_only={chain}");
    }
    Ok(())
}
