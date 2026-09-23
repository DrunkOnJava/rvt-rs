//! RE-52: a straight stair run's data carries its plan sketch, and its run
//! type's serialised object carries the tread, riser and nosing sizes and
//! which parts the run has. With the stair's riser height they give the
//! run's treads and risers.
//!
//! FACT: on Autodesk's Snowdon Towers 2024 architectural sample, each
//! straight run's data holds, among its bounded lines (`04 00 08 01`, as
//! RE-49 reads for beams), its two boundary lines from the first riser to
//! the last and one line across the run at each riser, all at the run's
//! base elevation. The run type the run's reference list names has its own
//! object, `01 00 00 00 · u64 id` with the tag `db 0f` 0x47 bytes past the
//! id, holding structural depth, tread thickness, riser thickness and
//! nosing length (`f64` feet at +0x59, +0x69, +0x71, +0x79) and one-byte
//! flags (monolithic, treads, risers, slanted risers at +0xbd..=+0xc0, and
//! at +0xb5 a flag set on the one type whose treads run on under the riser
//! above), followed by the type's name.
//!
//! The probe prints each run: its type and the type's values, whether its
//! sketch reads, and whether a side view is drawn. With `--json PATH` it
//! writes each drawn run's sketch and side view, for scoring against
//! Revit's own geometry of the flight (the flight's IFC4 tessellation or
//! a VIM mesh): the report's script checks that every vertex of the drawn
//! run lies on Revit's mesh and every vertex of Revit's mesh lies on or in
//! the drawn run.
//!
//! Usage:
//!   cargo run --profile ci --example probe_re52_stair_treads -- \
//!     MODEL.rvt [--json OUT.json]

use rvt::RevitFile;
use rvt::partition_element_records::{
    OST_STAIRS_RUNS, REFERENCE_LIST_OFFSET, decode_reference_list,
};
use rvt::partition_schema_mvp::{
    STAIR_RISER_COUNT_FIELD, STAIR_RISER_HEIGHT_FIELD, recover_partition_schema_mvp,
};
use rvt::partition_stairs::{run_side_profile, run_sketch, scan_run_lines, scan_run_types};
use rvt::partition_type_records::{scan_type_records, type_definition_ids, unique_type_reference};
use rvt::walker::{InstanceField, WalkerLimits};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;

struct Run {
    id: u32,
    riser_height: Option<f64>,
    risers: Option<i64>,
    run_type: Option<u32>,
}

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
    let declared = rvt::elem_table::declared_ids(&rvt::elem_table::parse_records(&mut rf)?);
    let type_records = scan_type_records(&mut rf, version, OST_STAIRS_RUNS, &declared)?;
    let run_types = type_definition_ids(&type_records);

    let mut runs = Vec::new();
    for element in mvp.products.iter().filter(|e| e.class == "StairsRun") {
        let Some(id) = element.id else { continue };
        let (mut riser_height, mut risers, mut stream, mut offset) = (None, None, None, None);
        for (name, value) in &element.fields {
            match (name.as_str(), value) {
                (STAIR_RISER_HEIGHT_FIELD, InstanceField::Float { value, .. }) => {
                    riser_height = Some(*value);
                }
                (STAIR_RISER_COUNT_FIELD, InstanceField::Integer { value, .. }) => {
                    risers = Some(*value);
                }
                ("m_source_stream", InstanceField::String(v)) => stream = Some(v.clone()),
                ("m_source_offset", InstanceField::Integer { value, .. }) => {
                    offset = usize::try_from(*value).ok();
                }
                _ => {}
            }
        }
        let run_type = match (stream, offset) {
            (Some(stream), Some(offset)) => {
                let inflated = rf.inflated_partition(&stream)?;
                decode_reference_list(inflated.bytes(), offset + REFERENCE_LIST_OFFSET)
                    .and_then(|references| unique_type_reference(&references, &run_types))
            }
            _ => None,
        };
        runs.push(Run {
            id,
            riser_height,
            risers,
            run_type,
        });
    }
    let ids: BTreeSet<u32> = runs.iter().map(|run| run.id).collect();
    let type_ids: BTreeSet<u32> = runs.iter().filter_map(|run| run.run_type).collect();
    let lines = scan_run_lines(&mut rf, version, &ids)?;
    let types = scan_run_types(&mut rf, version, &type_ids)?;

    println!("release {version}: {} stair runs", runs.len());
    println!("run types:");
    for (id, t) in &types {
        println!(
            "  {id:>8}  depth {:>6.3} in  tread {:>6.3} in  riser {:>6.3} in  nosing {:>6.3} in  \
             monolithic {}  treads {}  risers {}  slanted {}  tread under riser {}  {:?}",
            t.structural_depth_feet * 12.0,
            t.tread_thickness_feet * 12.0,
            t.riser_thickness_feet * 12.0,
            t.nosing_length_feet * 12.0,
            t.monolithic as u8,
            t.treads as u8,
            t.risers as u8,
            t.slanted_risers as u8,
            t.tread_under_riser as u8,
            t.name.as_deref().unwrap_or("")
        );
    }

    let mut census: BTreeMap<String, usize> = BTreeMap::new();
    let mut json = String::from("[");
    for run in &runs {
        let run_type = run.run_type.and_then(|id| types.get(&id));
        let sketch = lines.get(&run.id).and_then(|lines| run_sketch(lines));
        let outcome = match (run_type, &sketch, run.riser_height, run.risers) {
            (None, _, _, _) => "no run type".to_string(),
            (_, None, _, _) => "no straight sketch".to_string(),
            (_, _, None, _) | (_, _, _, None) => "no riser height or count".to_string(),
            (Some(t), Some(sketch), Some(h), Some(n)) => {
                if sketch.risers.len() as i64 != n {
                    format!("{} riser lines for {n} risers", sketch.risers.len())
                } else if let Some(profile) = run_side_profile(sketch, t, h) {
                    if !json.ends_with('[') {
                        json.push(',');
                    }
                    let _ = write!(
                        json,
                        "{{\"id\":{},\"type\":{},\"origin\":{:?},\"climb\":{:?},\"across\":{:?},\
                         \"width\":{},\"profile\":{:?}}}",
                        run.id,
                        run.run_type.unwrap_or(0),
                        sketch.origin,
                        sketch.climb,
                        sketch.across,
                        sketch.width_feet,
                        profile
                    );
                    format!(
                        "drawn: {}",
                        if t.slanted_risers {
                            "slanted risers"
                        } else {
                            "upright risers"
                        }
                    )
                } else if t.monolithic {
                    "monolithic, not drawn".to_string()
                } else if !t.risers {
                    "no risers, not drawn".to_string()
                } else {
                    "construction not drawn".to_string()
                }
            }
        };
        println!(
            "  run {:>8}  type {:>8}  {outcome}",
            run.id,
            run.run_type.map(|t| t.to_string()).unwrap_or("-".into())
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
