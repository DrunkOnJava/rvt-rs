//! RE-92: a stair run's data carries its StairsRun record, whose last flag
//! is Revit's "End with Riser" run setting. A run that ends with a tread
//! has one riser line more than risers, and a monolithic run is drawn from
//! its riser lines, nosing and structural depth.
//!
//! FACT: on Autodesk's Snowdon Towers 2024 architectural sample, each
//! straight run's data holds, within its first 4 KB, the StairsRun class's
//! fields in schema order: seven `f64` (bottom elevation, top elevation,
//! extend below base, extend below tread base, run width, left and right
//! stringer width), a `u32` top riser index, then one-byte flags centre
//! mark visible, begin with riser and end with riser. The first such
//! record whose bottom elevation and width equal the run sketch's (RE-52)
//! is the run's. End with riser is 0 on exactly the three runs whose riser
//! lines outnumber their risers (1563804, 1647253, 2234674) and 1 on the
//! other 40.
//!
//! The probe prints each run's type, flags, riser lines and risers, and
//! whether a side view is drawn. With `--json PATH` it writes each drawn
//! run's sketch and side view, for `tools/re/stair_runs_vs_ifc.py`.
//!
//! Usage:
//!   cargo run --profile ci --example probe_re92_run_ends -- \
//!     MODEL.rvt [--json OUT.json]

use rvt::RevitFile;
use rvt::partition_schema_mvp::{
    STAIR_RISER_COUNT_FIELD, TYPE_ID_FIELD, recover_partition_schema_mvp,
    stair_run_body_from_fields,
};
use rvt::partition_stairs::{run_sketch, scan_run_ends, scan_run_lines};
use rvt::walker::{InstanceField, WalkerLimits};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;

fn main() -> rvt::Result<()> {
    let mut args = std::env::args().skip(1);
    let path = args.next().expect("usage: MODEL.rvt [--json OUT.json]");
    let json_path = match (args.next().as_deref(), args.next()) {
        (Some("--json"), Some(out)) => Some(out),
        _ => None,
    };
    let mut rf = RevitFile::open(&path)?;
    let version = rf.basic_file_info()?.version;
    let mvp = recover_partition_schema_mvp(&mut rf, version, WalkerLimits::default())?;
    let runs: Vec<_> = mvp
        .products
        .iter()
        .filter(|e| e.class == "StairsRun" && e.id.is_some())
        .collect();
    let ids: BTreeSet<u32> = runs.iter().filter_map(|e| e.id).collect();
    let sketches: BTreeMap<_, _> = scan_run_lines(&mut rf, version, &ids)?
        .iter()
        .filter_map(|(&id, lines)| Some((id, run_sketch(lines)?)))
        .collect();
    let ends = scan_run_ends(&mut rf, version, &sketches)?;

    println!(
        "release {version}: {} stair runs, {} sketches, {} with their ends read",
        runs.len(),
        sketches.len(),
        ends.len()
    );
    let mut census: BTreeMap<String, usize> = BTreeMap::new();
    let mut json = String::from("[");
    for element in &runs {
        let id = element.id.expect("filtered");
        let (mut risers, mut run_type) = (None, None);
        for (name, value) in &element.fields {
            match (name.as_str(), value) {
                (STAIR_RISER_COUNT_FIELD, InstanceField::Integer { value, .. }) => {
                    risers = Some(*value)
                }
                (TYPE_ID_FIELD, InstanceField::ElementId { id, .. }) => run_type = Some(*id),
                _ => {}
            }
        }
        let flags = ends.get(&id).map_or("--".to_string(), |e| {
            format!("{}{}", e.begin_with_riser as u8, e.end_with_riser as u8)
        });
        let lines = sketches.get(&id).map(|s| s.risers.len());
        let body = stair_run_body_from_fields(&element.fields);
        if let Some(body) = &body {
            if !json.ends_with('[') {
                json.push(',');
            }
            let _ = write!(
                json,
                "{{\"id\":{id},\"type\":{},\"origin\":{:?},\"climb\":{:?},\"across\":{:?},\
                 \"width\":{},\"profile\":{:?}}}",
                run_type.unwrap_or(0),
                body.origin,
                body.climb,
                body.across,
                body.width_feet,
                body.profile
            );
        }
        let end = match ends.get(&id) {
            Some(e) if e.end_with_riser => "ends with a riser",
            Some(_) => "ends with a tread",
            None => "ends not read",
        };
        let outcome = format!(
            "{end}, {}",
            if body.is_some() { "drawn" } else { "not drawn" }
        );
        println!(
            "  run {id:>8}  type {:>8}  begin/end {flags}  riser lines {:>3}  risers {:>3}  {outcome}",
            run_type.map_or("-".into(), |t| t.to_string()),
            lines.map_or("-".into(), |n| n.to_string()),
            risers.map_or("-".into(), |n| n.to_string()),
        );
        *census.entry(outcome).or_default() += 1;
    }
    json.push(']');
    println!("census:");
    for (what, count) in &census {
        println!("  {count:>4}  {what}");
    }
    if let Some(out) = json_path {
        std::fs::write(&out, json).expect("write json");
        println!("wrote {out}");
    }
    Ok(())
}
