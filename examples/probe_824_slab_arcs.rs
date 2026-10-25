//! #824: how the sketch lines of the BIM4LCA office's slabs whose outline
//! still differs from Revit's read, each both as a line and as an arc.
//!
//! FACT under test: roof 1461758's two corner arcs are sketch lines whose
//! record reads as an arc (RE-96) with every point in the line's record
//! box, and the line reading also lies in it, so RE-96 rejects both and the
//! outline closes without them; and roof 2991742's and slab 1384047's holes
//! are sketch lines of their own, closing loops inside the outline.
//!
//! Output, per owner in `RVT_PROBE_OWNERS` (default 1461758, 2991742,
//! 1384047, 3058084) that owns sketch lines:
//!   owner <id> lines <n>
//!   copy <id> <stream> box <min x y z> <max x y z> current <partition|none>
//!     every record copy of a line with several, and its routed partition
//!   line <owner> <id> box <min x y z> <max x y z>
//!     as-line <start> <end> in-box <bool> | as-line none
//!     as-arc centre <c> radius <r> angles <a0> <a1> start <p> end <p> in-box <bool> | as-arc none
//!
//! `tools/re/slab_outlines_list.py` prints Revit's outline of each owner.
//!
//! Usage:
//!   cargo run --profile ci --example probe_824_slab_arcs -- MODEL.rvt

use rvt::RevitFile;
use rvt::research::element_record_plan_profiles::{VERTEX_EPS_FEET, arc_points};
use rvt::research::partition_beam_axes::scan_sketch_curves;
use rvt::research::partition_element_records as per;
use std::collections::{BTreeMap, BTreeSet};

fn fmt(p: [f64; 3]) -> String {
    format!("{:.6} {:.6} {:.6}", p[0], p[1], p[2])
}

fn in_box(points: &[[f64; 3]], b: &[f64; 6]) -> bool {
    let eps = VERTEX_EPS_FEET;
    points
        .iter()
        .all(|p| (0..3).all(|axis| p[axis] >= b[axis] - eps && p[axis] <= b[axis + 3] + eps))
}

fn main() -> rvt::Result<()> {
    let path = std::env::args().nth(1).expect("usage: MODEL.rvt");
    let wanted: BTreeSet<u32> = std::env::var("RVT_PROBE_OWNERS")
        .unwrap_or_else(|_| "1461758,2991742,1384047,3058084".into())
        .split(',')
        .filter_map(|id| id.trim().parse().ok())
        .collect();
    let mut rf = RevitFile::open(&path)?;
    let version = rf.basic_file_info()?.version;
    let declared = rvt::elem_table::declared_ids(&rvt::elem_table::parse_records(&mut rf)?);
    let records =
        per::scan_category_records_multi(&mut rf, version, &[per::OST_SKETCH_LINES], &declared)?;
    let mut owners: BTreeMap<u32, BTreeMap<u32, [f64; 6]>> = BTreeMap::new();
    for record in &records {
        if let Some(owner) = record
            .owner_reference
            .filter(|owner| wanted.contains(owner))
        {
            owners
                .entry(owner)
                .or_default()
                .entry(record.element_id)
                .or_insert(record.bbox_feet);
        }
    }
    let ids: BTreeSet<u32> = owners.values().flat_map(|l| l.keys().copied()).collect();
    // Every record copy of each line, and the partition its current record
    // routes to (RE-216).
    let mut copies: BTreeMap<u32, Vec<(String, [f64; 6])>> = BTreeMap::new();
    let mut holders: BTreeMap<u32, BTreeSet<u32>> = BTreeMap::new();
    for record in records.iter().filter(|r| ids.contains(&r.element_id)) {
        copies
            .entry(record.element_id)
            .or_default()
            .push((record.stream.clone(), record.bbox_feet));
        if let Some(p) = record
            .stream
            .strip_prefix("Partitions/")
            .and_then(|n| n.parse::<u32>().ok())
        {
            holders.entry(record.element_id).or_default().insert(p);
        }
    }
    let current = rvt::research::native_document::current_partitions(&mut rf, &holders);
    for (id, list) in &copies {
        if list.len() > 1 {
            for (stream, b) in list {
                println!(
                    "copy {id} {stream} box {} {} current {}",
                    fmt([b[0], b[1], b[2]]),
                    fmt([b[3], b[4], b[5]]),
                    current.get(id).map_or("none".into(), |p| p.to_string())
                );
            }
        }
    }
    let curves = scan_sketch_curves(&mut rf, version, &ids)?;
    for (owner, lines) in &owners {
        println!("owner {owner} lines {}", lines.len());
        for (id, b) in lines {
            println!(
                "line {owner} {id} box {} {}",
                fmt([b[0], b[1], b[2]]),
                fmt([b[3], b[4], b[5]])
            );
            let curve = curves.get(id);
            match curve.and_then(|c| c.line.as_ref()) {
                Some(line) => {
                    let at = |t: f64| {
                        [
                            line.origin[0] + line.direction[0] * t,
                            line.origin[1] + line.direction[1] * t,
                            line.origin[2] + line.direction[2] * t,
                        ]
                    };
                    let (s, e) = (at(line.start_parameter), at(line.end_parameter));
                    println!(
                        "  as-line {} {} in-box {}",
                        fmt(s),
                        fmt(e),
                        in_box(&[s, e], b)
                    );
                }
                None => println!("  as-line none"),
            }
            match curve.and_then(|c| c.arc.as_ref()) {
                Some(arc) => {
                    let inside = arc_points(arc).is_some_and(|points| in_box(&points, b));
                    println!(
                        "  as-arc centre {} radius {:.6} angles {:.6} {:.6} start {} end {} in-box {}",
                        fmt(arc.centre),
                        arc.radius,
                        arc.start_angle,
                        arc.end_angle,
                        fmt(arc.point(arc.start_angle)),
                        fmt(arc.point(arc.end_angle)),
                        inside
                    );
                }
                None => println!("  as-arc none"),
            }
        }
    }
    Ok(())
}
