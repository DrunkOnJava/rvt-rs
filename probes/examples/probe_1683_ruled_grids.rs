//! #1683: the grid of each ruled face a saved graph holds, and what sets its cells.
//!
//! FACT under test: after the per-face grid (b4e2df58) and the counts along and
//! across the profiles (bb4e21dc) the four roofs of Snowdon Towers Architectural
//! still have 761,635 triangles (Measure 42968296323), against 1,259 in Revit's
//! own body of one of them. `native_parametric_mesh::face_grid` gives a ruled face
//! the knots of its formula (the arcs' angular steps and the twist's cells) and
//! then adds every coordinate of the trim loop's points to both knot vectors, so a
//! face whose edges the graph samples finely has a lattice of its own, and the
//! formula's own count is only part of it.
//!
//! This reads every record the saved-mesh pass reads (the owners of channel 103
//! and the shared symbols they name, a level at a time), runs `face_grid` on each
//! `Face`, and keeps those on a `RuledSurf`. For each it prints the formula's
//! counts (replayed here from the profiles and the corners, as `face_grid` makes
//! them), the final knots, how many distinct coordinates the trim points add to
//! each axis, the twist and whether the face is flat. It says how many cells the
//! faces hold in all, in how many of them the final knots are 4 times the
//! formula's cells or more (the trim lattice) and in how many the formula's own
//! cells are the most, and lists the owners and faces with the most.
//!
//! Usage:
//!   cargo run --profile ci -p rvt-probes --example probe_1683_ruled_grids -- MODEL.rvt

use std::collections::{BTreeMap, BTreeSet};

use rvt::RevitFile;
use rvt::ifc::entities::IfcEntity;
use rvt::ifc::{ExportQualityMode, RvtDocExporter};
use rvt::research::native_document::{self, Record};
use rvt::research::native_parametric_mesh::face_grid;
use rvt::research::native_parametric_surface::{Curve, ParametricSurface};

/// `native_parametric_mesh::face_grid`'s chord bound, feet.
const CHORD: f64 = 0.001;

fn angular(r: f64) -> f64 {
    (4. * (CHORD / (2. * r.max(CHORD))).min(1.).sqrt().asin()).min(std::f64::consts::FRAC_PI_2)
}

fn range_steps(curve: &Curve) -> f64 {
    let r = curve.range();
    match curve {
        Curve::Line { .. } => 1.,
        Curve::Arc { radius, .. } => ((r[1] - r[0]).abs() / angular(*radius)).ceil(),
        Curve::Ellipse { x_len, y_len, .. } => {
            ((r[1] - r[0]).abs() / angular(x_len.max(*y_len))).ceil()
        }
    }
}

fn kind(curve: &Curve) -> String {
    match curve {
        Curve::Line { .. } => "line".into(),
        Curve::Arc { radius, range, .. } => {
            format!(
                "arc r {radius:.2} over {:.3} rad",
                (range[1] - range[0]).abs()
            )
        }
        Curve::Ellipse {
            x_len,
            y_len,
            range,
            ..
        } => format!(
            "ellipse {x_len:.2} by {y_len:.2} over {:.3} rad",
            (range[1] - range[0]).abs()
        ),
    }
}

fn distinct(values: impl Iterator<Item = f64>) -> usize {
    let mut all: Vec<f64> = values.collect();
    all.sort_by(f64::total_cmp);
    all.dedup_by(|a, b| (*a - *b).abs() < 1e-12);
    all.len()
}

/// The pass's `instance_symbols`, with `native_metadata::identifier` copied.
fn identifier(value: &rvt::research::graph_value::Fv) -> Option<i64> {
    if let Some(n) = value.as_i64() {
        return Some(n);
    }
    ["m_id", "m_id64"]
        .into_iter()
        .find_map(|key| value.get(key))
        .and_then(identifier)
}

fn instance_symbols(record: &Record) -> Vec<u64> {
    let Some(graph) = &record.graph else {
        return Vec::new();
    };
    graph
        .objects
        .iter()
        .filter(|object| object.class_name == "InstanceInfo")
        .filter_map(|object| {
            identifier(&object.fields["m_symbolId"])
                .and_then(|id| u64::try_from(id).ok())
                .filter(|id| *id > 0)
        })
        .collect()
}

/// One ruled face's grid.
struct Grid {
    owner: u64,
    face: usize,
    profiles: String,
    twist: f64,
    flat: bool,
    formula: (f64, f64),
    knots: (usize, usize),
    trim: (usize, usize),
    trim_points: usize,
}

impl Grid {
    fn cells(&self) -> usize {
        self.knots.0.saturating_sub(1) * self.knots.1.saturating_sub(1)
    }
    fn formula_cells(&self) -> f64 {
        self.formula.0 * self.formula.1
    }
}

/// The formula `face_grid` gives a ruled face, replayed: the cells along and across,
/// the twist and whether the face is flat.
fn formula(surface: &ParametricSurface) -> Option<((f64, f64), f64, bool)> {
    let ParametricSurface::RuledSurf {
        profile1, profile2, ..
    } = surface
    else {
        return None;
    };
    let env = surface.bounds();
    let corner = |u: f64, v: f64| surface.evaluate(u, v).ok();
    let c00 = corner(env[0][0], env[0][1])?;
    let c10 = corner(env[1][0], env[0][1])?;
    let c01 = corner(env[0][0], env[1][1])?;
    let c11 = corner(env[1][0], env[1][1])?;
    let length = |p: [f64; 3]| p.iter().map(|x| x * x).sum::<f64>().sqrt();
    let twist: [f64; 3] = std::array::from_fn(|k| c00[k] - c10[k] - c01[k] + c11[k]);
    let a: [f64; 3] = std::array::from_fn(|k| c10[k] - c00[k]);
    let b: [f64; 3] = std::array::from_fn(|k| c01[k] - c00[k]);
    let normal = [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ];
    let normal_length = length(normal);
    let flat = normal_length > 0.
        && (twist.iter().zip(normal).map(|(t, n)| t * n).sum::<f64>() / normal_length).abs() < 1e-8;
    let twisted = if flat { 0. } else { length(twist) };
    let cells = twisted / CHORD;
    let n_u = cells
        .sqrt()
        .ceil()
        .max(range_steps(profile1))
        .max(range_steps(profile2))
        .max(1.);
    let n_v = (cells / n_u).ceil().max(1.);
    Some(((n_u, n_v), length(twist), flat))
}

fn scan(
    owner: u64,
    record: &Record,
    grids: &mut Vec<Grid>,
    refused: &mut BTreeMap<String, usize>,
    faces: &mut usize,
) {
    let Some(graph) = &record.graph else {
        return;
    };
    for (fi, object) in graph.objects.iter().enumerate() {
        if object.class_name != "Face" {
            continue;
        }
        *faces += 1;
        match face_grid(graph, fi) {
            Ok(grid) => {
                let Some((formula, twist, flat)) = formula(&grid.surface) else {
                    continue;
                };
                let ParametricSurface::RuledSurf {
                    profile1, profile2, ..
                } = &grid.surface
                else {
                    continue;
                };
                grids.push(Grid {
                    owner,
                    face: fi,
                    profiles: format!("{} | {}", kind(profile1), kind(profile2)),
                    twist,
                    flat,
                    formula,
                    knots: (grid.u.len(), grid.v.len()),
                    trim: (
                        distinct(grid.trim.iter().map(|p| p[0])),
                        distinct(grid.trim.iter().map(|p| p[1])),
                    ),
                    trim_points: grid.trim.len(),
                });
            }
            Err(error) => {
                let message: String = format!("{error:#}").chars().take(70).collect();
                *refused.entry(message).or_default() += 1;
            }
        }
    }
}

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let path = args
        .iter()
        .find(|a| !a.starts_with("--"))
        .ok_or_else(|| anyhow::anyhow!("usage: probe_1683_ruled_grids MODEL.rvt"))?;
    let mut rf = RevitFile::open_with_limits(path, rvt::cli::open_limits(None, None, false))?;
    let model = RvtDocExporter.export_with_mode_and_limits(
        &mut rf,
        ExportQualityMode::Scaffold,
        rvt::walker::WalkerLimits::default(),
    )?;
    let ids: BTreeSet<u64> = model
        .entities
        .iter()
        .filter_map(|entity| match entity {
            IfcEntity::BuildingElement {
                type_guid: Some(tag),
                ..
            } => tag.parse().ok(),
            _ => None,
        })
        .collect();
    let mut options = native_document::Options {
        selected_ids: ids,
        channels: BTreeSet::from([103]),
        max_graph_values: 1_000_000,
        max_graph_objects: 1_000_000,
        record_extras: true,
        ..Default::default()
    };
    let mut grids: Vec<Grid> = Vec::new();
    let mut refused: BTreeMap<String, usize> = BTreeMap::new();
    let mut faces = 0usize;
    let mut owner_ids = BTreeSet::<u64>::new();
    let mut wanted = BTreeSet::<u64>::new();
    native_document::extract_graphs(&mut rf, &options, |r| {
        let id = r.identity.element_id;
        owner_ids.insert(id);
        wanted.extend(instance_symbols(&r));
        scan(id, &r, &mut grids, &mut refused, &mut faces);
        Ok(())
    })?;
    let mut attempted = BTreeSet::<u64>::new();
    wanted.retain(|id| !owner_ids.contains(id));
    for _ in 0..128 {
        if wanted.is_empty() {
            break;
        }
        attempted.extend(&wanted);
        options.selected_ids = std::mem::take(&mut wanted);
        let mut found = Vec::new();
        native_document::extract_graphs(&mut rf, &options, |r| {
            found.extend(instance_symbols(&r));
            scan(
                r.identity.element_id,
                &r,
                &mut grids,
                &mut refused,
                &mut faces,
            );
            Ok(())
        })?;
        wanted.extend(
            found
                .into_iter()
                .filter(|id| !owner_ids.contains(id) && !attempted.contains(id)),
        );
    }

    let cells: usize = grids.iter().map(Grid::cells).sum();
    let formula_cells: f64 = grids.iter().map(Grid::formula_cells).sum();
    println!(
        "faces {faces} | ruled faces with a grid {} | refused {} | cells {cells} | formula cells {formula_cells:.0}",
        grids.len(),
        refused.values().sum::<usize>()
    );
    let lattice: Vec<&Grid> = grids
        .iter()
        .filter(|grid| grid.cells() as f64 >= 4.0 * grid.formula_cells())
        .collect();
    let lattice_cells: usize = lattice.iter().map(|grid| grid.cells()).sum();
    let formula_led = grids.len() - lattice.len();
    println!(
        "faces whose final cells are 4 times the formula's or more: {} with {lattice_cells} cells ({:.1}% of all); the other {formula_led} hold {}",
        lattice.len(),
        if cells > 0 {
            100.0 * lattice_cells as f64 / cells as f64
        } else {
            0.0
        },
        cells - lattice_cells
    );
    let twisted: Vec<&Grid> = grids.iter().filter(|grid| !grid.flat).collect();
    println!(
        "faces not flat: {} with {} cells | flat: {} with {} cells",
        twisted.len(),
        twisted.iter().map(|grid| grid.cells()).sum::<usize>(),
        grids.len() - twisted.len(),
        grids
            .iter()
            .filter(|grid| grid.flat)
            .map(Grid::cells)
            .sum::<usize>()
    );
    let mut by_owner: BTreeMap<u64, (usize, usize)> = BTreeMap::new();
    for grid in &grids {
        let entry = by_owner.entry(grid.owner).or_default();
        entry.0 += 1;
        entry.1 += grid.cells();
    }
    let mut owners: Vec<(&u64, &(usize, usize))> = by_owner.iter().collect();
    owners.sort_by_key(|(_, (_, cells))| std::cmp::Reverse(*cells));
    for (owner, (count, owner_cells)) in owners.iter().take(10) {
        println!("  record {owner}: {count} ruled faces, {owner_cells} cells");
    }
    let mut sorted: Vec<&Grid> = grids.iter().collect();
    sorted.sort_by_key(|grid| std::cmp::Reverse(grid.cells()));
    for top in [1usize, 5, 25] {
        let share: usize = sorted.iter().take(top).map(|grid| grid.cells()).sum();
        println!(
            "largest {top} faces: {share} cells, {:.1}% of all",
            if cells > 0 {
                100.0 * share as f64 / cells as f64
            } else {
                0.0
            }
        );
    }
    for grid in sorted.iter().take(15) {
        println!(
            "  record {} face {}: cells {} = knots {} x {} (formula {} x {}), trim points {} distinct {} u {} v, twist {:.4} ft {}, {}",
            grid.owner,
            grid.face,
            grid.cells(),
            grid.knots.0 - 1,
            grid.knots.1 - 1,
            grid.formula.0,
            grid.formula.1,
            grid.trim_points,
            grid.trim.0,
            grid.trim.1,
            grid.twist,
            if grid.flat { "flat" } else { "twisted" },
            grid.profiles
        );
    }
    let mut reasons: Vec<(&String, &usize)> = refused.iter().collect();
    reasons.sort_by_key(|(_, count)| std::cmp::Reverse(**count));
    for (reason, count) in reasons.iter().take(6) {
        println!("  refused {count}: {reason}");
    }
    Ok(())
}
