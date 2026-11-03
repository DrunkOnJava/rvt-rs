//! RE-307 (probe, #885): the slope arrows of a footprint roof sloped by
//! several of them, and where each one's plane applies.
//!
//! For each placed roof whose sketch holds two or more slope arrows (sketch
//! lines carrying `SlopeCurveData`, RE-199) and no defining edge, the probe
//! prints:
//!
//! - each arrow: its tail and head, run, tail and head heights, slope (rise
//!   over run), and the boundary line its tail lies on and is square to;
//! - the outline: each reflex vertex, the edge the extension of each of its
//!   two edges meets first, and whether that meeting lies inside the edge or
//!   at a vertex;
//! - each sloped edge so met inside, with the edge beyond each of its ends
//!   and whether the corner is convex and the neighbour sloped (the hip the
//!   wedges of RE-199 would draw);
//! - what the export read: the roof's marker, edges, angles and rafter cut.
//!
//! Usage: probe_re307_arrow_roofs <model.rvt>

use rvt::RevitFile;
use rvt::research::element_record_plan_profiles::{
    plan_profile_from_fields, scan_sketch_curve_lists,
};
use rvt::research::partition_beam_axes::scan_sketch_bounded_lines;
use rvt::research::partition_element_records::{OST_SKETCH_LINES, scan_category_records_multi};
use rvt::research::partition_roof_slopes::{scan_arrow_heights, scan_edge_slopes};
use rvt::research::partition_schema_mvp::{
    recover_partition_schema_mvp, roof_arrow_planes_from_fields, roof_rafter_cut_from_fields,
    roof_slopes_from_fields,
};
use rvt::walker::WalkerLimits;
use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

type Point = (f64, f64);

fn main() -> anyhow::Result<()> {
    let path = PathBuf::from(std::env::args().nth(1).expect("usage: FILE.rvt"));
    let mut rf = RevitFile::open(&path)?;
    let version = rf.basic_file_info()?.version;
    let mvp = recover_partition_schema_mvp(&mut rf, version, WalkerLimits::default())?;
    let roofs: Vec<_> = mvp.products.iter().filter(|e| e.class == "Roof").collect();
    let roof_ids: BTreeSet<u32> = roofs.iter().filter_map(|e| e.id).collect();
    let declared: BTreeSet<u32> =
        rvt::elem_table::declared_ids(&rvt::elem_table::parse_records(&mut rf)?);
    let sketch_lines =
        scan_category_records_multi(&mut rf, version, &[OST_SKETCH_LINES], &declared)?;
    let all_lines: BTreeSet<u32> = sketch_lines.iter().map(|l| l.element_id).collect();
    let lists = scan_sketch_curve_lists(&mut rf, version, &all_lines);
    let ids: BTreeSet<u32> = lists
        .iter()
        .filter(|(roof, _)| roof_ids.contains(roof))
        .flat_map(|(_, curves)| curves.iter().copied())
        .collect();
    let slopes = scan_edge_slopes(&mut rf, version, &ids)?;
    let unblocked: BTreeSet<u32> = ids
        .iter()
        .filter(|id| !slopes.contains_key(id))
        .copied()
        .collect();
    let arrows = scan_arrow_heights(&mut rf, version, &unblocked)?;
    let lines = scan_sketch_bounded_lines(&mut rf, version, &ids)?;
    println!("release {version}: {} roofs", roofs.len());
    for roof in roofs {
        let Some(id) = roof.id else {
            continue;
        };
        let Some(curves) = lists.get(&id) else {
            continue;
        };
        let (blocked, rest): (Vec<u32>, Vec<u32>) = curves
            .iter()
            .copied()
            .partition(|line| slopes.contains_key(line));
        let rest: Vec<u32> = rest
            .into_iter()
            .filter(|line| arrows.contains_key(line))
            .collect();
        if rest.len() < 2 {
            continue;
        }
        let defining = blocked.iter().filter(|l| slopes[*l].defines_slope).count();
        println!(
            "roof {id}: {} boundary lines, {} arrows, {defining} defining edges",
            blocked.len(),
            rest.len()
        );
        let mut sloped_edges: BTreeSet<u32> = BTreeSet::new();
        for arrow in &rest {
            let (Some(line), Some(heights)) = (lines.get(arrow), arrows.get(arrow)) else {
                println!("  arrow {arrow}: no bounded line");
                continue;
            };
            let (tail, head) = (line.start(), line.end());
            let run = (head[0] - tail[0]).hypot(head[1] - tail[1]);
            let (ux, uy) = ((head[0] - tail[0]) / run, (head[1] - tail[1]) / run);
            println!(
                "  arrow {arrow}: tail ({:.5}, {:.5}) head ({:.5}, {:.5}) run {run:.5} ft; heights tail {:.5} head {:.5} ft, angle {:.6} rad, by angle {}; rise over run {:.6}",
                tail[0],
                tail[1],
                head[0],
                head[1],
                heights.tail_feet,
                heights.head_feet,
                heights.angle_radians,
                heights.by_angle,
                heights.rise_over_run(run),
            );
            for edge_id in &blocked {
                let Some(edge) = lines.get(edge_id) else {
                    continue;
                };
                let (a, b) = (edge.start(), edge.end());
                let length = (b[0] - a[0]).hypot(b[1] - a[1]);
                let (ex, ey) = ((b[0] - a[0]) / length, (b[1] - a[1]) / length);
                let along = (tail[0] - a[0]) * ex + (tail[1] - a[1]) * ey;
                let off = (tail[0] - a[0]) * -ey + (tail[1] - a[1]) * ex;
                if off.abs() < 1e-4 && (-1e-4..=length + 1e-4).contains(&along) {
                    println!(
                        "    tail on boundary line {edge_id} ({:.5}, {:.5}) to ({:.5}, {:.5}), {along:.5} ft along its {length:.5}; arrow . line = {:.3e}",
                        a[0],
                        a[1],
                        b[0],
                        b[1],
                        ux * ex + uy * ey,
                    );
                    sloped_edges.insert(*edge_id);
                }
            }
        }
        let Some(profile) = plan_profile_from_fields(&roof.fields) else {
            println!("  no plan profile");
            continue;
        };
        println!("  outline {} vertices", profile.outer_xy.len());
        extensions(&profile.outer_xy, &lines, &sloped_edges);
        let read = roof_slopes_from_fields(&roof.fields);
        println!(
            "  export read: arrow planes {}, {} edges, rafter cut {:?}",
            roof_arrow_planes_from_fields(&roof.fields),
            read.len(),
            roof_rafter_cut_from_fields(&roof.fields),
        );
        for (index, slope) in read.iter().enumerate() {
            println!(
                "    edge {index}: ({:.5}, {:.5}) to ({:.5}, {:.5}), {:.6} rad, thickness {:.5} ft",
                slope.edge_start[0],
                slope.edge_start[1],
                slope.edge_end[0],
                slope.edge_end[1],
                slope.angle_radians,
                slope.thickness_feet,
            );
        }
    }
    Ok(())
}

/// Print, for each reflex vertex of `ring`, what the extension of each of its
/// two edges meets, and the sloped edges met inside with their corners.
fn extensions(
    ring: &[Point],
    lines: &BTreeMap<u32, rvt::research::partition_beam_axes::BoundedLine>,
    sloped_lines: &BTreeSet<u32>,
) {
    let n = ring.len();
    let near = |a: Point, b: Point| (a.0 - b.0).abs() < 1e-4 && (a.1 - b.1).abs() < 1e-4;
    // Outline edge k runs from ring[k] to ring[k + 1]; it is sloped when it
    // is a sloped boundary line.
    let sloped: Vec<usize> = (0..n)
        .filter(|&k| {
            let (a, b) = (ring[k], ring[(k + 1) % n]);
            sloped_lines.iter().any(|id| {
                lines.get(id).is_some_and(|line| {
                    let (s, e) = (line.start(), line.end());
                    let (s, e) = ((s[0], s[1]), (e[0], e[1]));
                    (near(a, s) && near(b, e)) || (near(a, e) && near(b, s))
                })
            })
        })
        .collect();
    println!("  sloped outline edges: {sloped:?}");
    let area: f64 = (0..n)
        .map(|i| {
            let (a, b) = (ring[i], ring[(i + 1) % n]);
            a.0 * b.1 - b.0 * a.1
        })
        .sum::<f64>()
        / 2.0;
    let turn = |i: usize| {
        let (p, c, q) = (ring[(i + n - 1) % n], ring[i], ring[(i + 1) % n]);
        let (u, v) = ((c.0 - p.0, c.1 - p.1), (q.0 - c.0, q.1 - c.1));
        let scale = u.0.hypot(u.1) * v.0.hypot(v.1);
        if scale > 0.0 {
            (u.0 * v.1 - u.1 * v.0) * area.signum() / scale
        } else {
            0.0
        }
    };
    let mut inside: BTreeSet<usize> = BTreeSet::new();
    for i in (0..n).filter(|&i| turn(i) < -1e-9) {
        let (p, c, q) = (ring[(i + n - 1) % n], ring[i], ring[(i + 1) % n]);
        println!("  reflex vertex {i} ({:.5}, {:.5})", c.0, c.1);
        for (name, d) in [
            ("edge before", (c.0 - p.0, c.1 - p.1)),
            ("edge after", (c.0 - q.0, c.1 - q.1)),
        ] {
            let mut nearest: Option<(f64, usize, f64)> = None;
            for k in (0..n).filter(|&k| k != i && k != (i + n - 1) % n) {
                let (a, b) = (ring[k], ring[(k + 1) % n]);
                let e = (b.0 - a.0, b.1 - a.1);
                let denom = d.0 * e.1 - d.1 * e.0;
                if denom.abs() < 1e-12 {
                    continue;
                }
                let ac = (a.0 - c.0, a.1 - c.1);
                let t = (ac.0 * e.1 - ac.1 * e.0) / denom;
                let s = (ac.0 * d.1 - ac.1 * d.0) / denom;
                if t > 1e-9
                    && (-1e-9..=1.0 + 1e-9).contains(&s)
                    && nearest.is_none_or(|(best, _, _)| t < best - 1e-12)
                {
                    nearest = Some((t, k, s * e.0.hypot(e.1)));
                }
            }
            match nearest {
                None => println!("    extension of the {name}: meets nothing"),
                Some((t, k, along)) => {
                    let (a, b) = (ring[k], ring[(k + 1) % n]);
                    let length = (b.0 - a.0).hypot(b.1 - a.1);
                    let hit = (c.0 + t * d.0, c.1 + t * d.1);
                    let kind = if along <= 1e-6 || length - along <= 1e-6 {
                        "a vertex"
                    } else {
                        inside.insert(k);
                        "inside the edge"
                    };
                    println!(
                        "    extension of the {name} meets edge {k} ({}) at ({:.5}, {:.5}), {kind}",
                        if sloped.contains(&k) {
                            "sloped"
                        } else {
                            "upright"
                        },
                        hit.0,
                        hit.1,
                    );
                }
            }
        }
    }
    for k in inside.into_iter().filter(|k| sloped.contains(k)) {
        println!("  sloped edge {k} is met inside:");
        for (vertex, other) in [(k, (k + n - 1) % n), ((k + 1) % n, (k + 1) % n)] {
            println!(
                "    corner {vertex}: sine of the turn {:.3}, neighbour edge {other} {}",
                turn(vertex),
                if sloped.contains(&other) {
                    "sloped"
                } else {
                    "upright"
                },
            );
        }
    }
}
