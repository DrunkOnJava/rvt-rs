//! #1046 (RE-388): what the saved mesh of a wall gives of Revit's wall quantities.
//!
//! FACT under test: the mesh Revit saved for a wall is its solid as Revit last
//! drew it, joined to the walls beside it and cut by what it hosts, so the
//! volume of the mesh is `NetVolume`, the span of its heights is `Height`, and
//! its vertical faces give `GrossSideArea` (the largest, over the vertical
//! faces, of the area inside a face's outer loop) and `NetSideArea` (the area
//! of the largest vertical face, its holes taken out), as revit-ifc's
//! `PropertyUtil.CreateWallBaseQuantities` takes them from the solid it writes
//! (CLEANROOM.md source 5, RE-270). `tools/re/wall_mesh_quantities_vs_ifc.py`
//! scores the lines against Revit's `Qto_WallBaseQuantities`.
//!
//! For every wall it prints, in feet and the document frame:
//!
//!   wall tag:<id> status <status> primitives <n> triangles <n> unbounded <n> diagnostics <n>
//!   wall tag:<id> volume <ft3> z <low> <high> vfaces <n>
//!   wall tag:<id> closed <0|1> open <n> over <n> shells <n>
//!   wall tag:<id> shape up <n> down <n> sloped <n> curved <n>
//!   wall tag:<id> vface net <ft2> gross <ft2> normal <nx> <ny>
//!
//! `closed` says whether the mesh is a closed oriented surface (every directed edge
//! has its reverse once; `open` counts the edges that do not, `over` those the mesh
//! holds more than once) and `shells` how many connected pieces it has, corners
//! welded to 1e-6 ft.
//!
//! One `vface` line for each of the four largest flat vertical faces (a primitive
//! whose normals are one horizontal direction), `gross` the area inside its
//! outer loop and `net` its area less its holes; `gross` is `nan` where its
//! boundary does not close into loops one way each.
//!
//! Usage:
//!   cargo run --profile ci -p rvt-probes --example probe_1046_wall_mesh_quantities -- MODEL.rvt

use anyhow::{Context, Result};
use rvt::RevitFile;
use rvt::research::native_document::{Options, Record, extract_graphs};
use rvt::research::native_saved_scene::extract_at_detail;
use std::collections::{BTreeSet, HashMap, HashSet};

const WALLS: [&str; 2] = ["SWall", "ArcWall"];
const DETAIL: i64 = 3;
const WELD: f64 = 1e-6;
const SHOWN_FACES: usize = 4;

type Vec3 = [f64; 3];

fn sub(a: Vec3, b: Vec3) -> Vec3 {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn cross(a: Vec3, b: Vec3) -> Vec3 {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn dot(a: Vec3, b: Vec3) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn wall_ids(record: Record, ids: &mut BTreeSet<u64>) -> Result<()> {
    if record
        .class_name
        .as_deref()
        .is_some_and(|class| WALLS.contains(&class))
    {
        ids.insert(record.identity.element_id);
    }
    Ok(())
}

/// One triangle of a primitive wound as its normals point, or none where it
/// has no area.
fn oriented(
    vertices: &[Vec3],
    normals: &[Vec3],
    triangle: &[u32; 3],
) -> Option<(Vec3, Vec3, Vec3, Vec3)> {
    let corners = triangle.map(|i| vertices.get(i as usize).copied());
    let [Some(a), Some(mut b), Some(mut c)] = corners else {
        return None;
    };
    let mut normal = cross(sub(b, a), sub(c, a));
    if dot(normal, normal).sqrt() < 1e-12 {
        return None;
    }
    if normals.len() == vertices.len() {
        let outward = triangle.iter().fold([0.0; 3], |sum, i| {
            let n = normals.get(*i as usize).copied().unwrap_or([0.0; 3]);
            [sum[0] + n[0], sum[1] + n[1], sum[2] + n[2]]
        });
        if dot(normal, outward) < 0.0 {
            std::mem::swap(&mut b, &mut c);
            normal = [-normal[0], -normal[1], -normal[2]];
        }
    }
    Some((a, b, c, normal))
}

/// The area inside the outer loop of the boundary of `triangles` (wound one
/// way, corners welded), seen along the horizontal unit normal `n`, or nan
/// where the boundary is not made of loops, each vertex left once.
fn outer_loop_area(triangles: &[(Vec3, Vec3, Vec3)], n: [f64; 2]) -> f64 {
    let mut ids: HashMap<[i64; 3], u32> = HashMap::new();
    let mut points: Vec<Vec3> = Vec::new();
    let mut edges: HashSet<(u32, u32)> = HashSet::new();
    for (a, b, c) in triangles {
        let mut corner = |p: &Vec3| {
            let key = (*p).map(|x| (x / WELD).round() as i64);
            *ids.entry(key).or_insert_with(|| {
                points.push(*p);
                (points.len() - 1) as u32
            })
        };
        let (ia, ib, ic) = (corner(a), corner(b), corner(c));
        for edge in [(ia, ib), (ib, ic), (ic, ia)] {
            edges.insert(edge);
        }
    }
    let mut next: HashMap<u32, u32> = HashMap::new();
    for &(from, to) in &edges {
        if !edges.contains(&(to, from)) && next.insert(from, to).is_some() {
            return f64::NAN;
        }
    }
    let along = [-n[1], n[0]];
    let plane = |i: u32| {
        let p = points[i as usize];
        (p[0] * along[0] + p[1] * along[1], p[2])
    };
    let mut seen: HashSet<u32> = HashSet::new();
    let mut outer = 0.0f64;
    let starts: Vec<u32> = next.keys().copied().collect();
    for start in starts {
        if seen.contains(&start) {
            continue;
        }
        let mut sum = 0.0;
        let mut at = start;
        loop {
            seen.insert(at);
            let Some(&to) = next.get(&at) else {
                return f64::NAN;
            };
            let ((u0, v0), (u1, v1)) = (plane(at), plane(to));
            sum += u0 * v1 - u1 * v0;
            at = to;
            if at == start {
                break;
            }
            if seen.contains(&at) {
                return f64::NAN;
            }
        }
        outer = outer.max(sum.abs() / 2.0);
    }
    if outer == 0.0 { f64::NAN } else { outer }
}

/// (open edges, over-full edges, shells) of the triangles, corners welded: an edge is open
/// where its reverse is not in the mesh the same number of times, over-full where the mesh
/// holds it more than once.
fn closedness(triangles: &[(Vec3, Vec3, Vec3)]) -> (usize, usize, usize) {
    let mut ids: HashMap<[i64; 3], usize> = HashMap::new();
    let mut corner = |p: &Vec3| {
        let key = (*p).map(|x| (x / WELD).round() as i64);
        let next = ids.len();
        *ids.entry(key).or_insert(next)
    };
    let mut directed: HashMap<(usize, usize), usize> = HashMap::new();
    let mut parent: Vec<usize> = Vec::new();
    let mut find = |parent: &mut Vec<usize>, mut i: usize| {
        while parent[i] != i {
            parent[i] = parent[parent[i]];
            i = parent[i];
        }
        i
    };
    for (a, b, c) in triangles {
        let (ia, ib, ic) = (corner(a), corner(b), corner(c));
        while parent.len() <= ia.max(ib).max(ic) {
            parent.push(parent.len());
        }
        for edge in [(ia, ib), (ib, ic), (ic, ia)] {
            *directed.entry(edge).or_insert(0) += 1;
            let (ra, rb) = (find(&mut parent, edge.0), find(&mut parent, edge.1));
            if ra != rb {
                parent[ra] = rb;
            }
        }
    }
    let open = directed
        .iter()
        .filter(|((from, to), n)| directed.get(&(*to, *from)).copied().unwrap_or(0) != **n)
        .count();
    let over = directed.values().filter(|n| **n > 1).count();
    let shells = (0..parent.len())
        .filter(|&i| find(&mut parent, i) == i)
        .count();
    (open, over, shells)
}

fn main() -> Result<()> {
    let path = std::env::args()
        .skip(1)
        .find(|arg| !arg.starts_with("--"))
        .context("usage: probe_1046_wall_mesh_quantities MODEL.rvt")?;
    let mut rf = RevitFile::open(&path)?;
    let mut ids: BTreeSet<u64> = BTreeSet::new();
    let walls = Options {
        selected_classes: WALLS.iter().map(|name| (*name).to_string()).collect(),
        ..Options::default()
    };
    extract_graphs(&mut rf, &walls, |record| wall_ids(record, &mut ids))?;
    println!("walls {}", ids.len());
    if ids.is_empty() {
        return Ok(());
    }
    let options = Options {
        selected_ids: ids,
        max_graph_values: 1_000_000,
        max_graph_objects: 1_000_000,
        ..Options::default()
    };
    let scene = extract_at_detail(&mut rf, &options, DETAIL)?;
    for element in &scene.elements {
        let id = element.id;
        let meshes = &element.meshes;
        let triangles: usize = meshes.primitives.iter().map(|p| p.triangles.len()).sum();
        println!(
            "wall tag:{id} status {} primitives {} triangles {triangles} unbounded {} diagnostics {}",
            element.status,
            meshes.primitives.len(),
            meshes.unbounded_faces,
            meshes.diagnostics.len()
        );
        let mut volume = 0.0;
        let (mut low, mut high) = (f64::INFINITY, f64::NEG_INFINITY);
        let mut faces: Vec<(f64, f64, [f64; 2])> = Vec::new();
        let mut every: Vec<(Vec3, Vec3, Vec3)> = Vec::new();
        // Distinct heights of the flat faces that face up and down (to 1e-4 ft), the flat faces
        // that are neither vertical nor horizontal, and the faces that are not flat.
        let (mut up, mut down): (BTreeSet<i64>, BTreeSet<i64>) = (BTreeSet::new(), BTreeSet::new());
        let (mut sloped, mut curved) = (0usize, 0usize);
        for primitive in &meshes.primitives {
            let mut flat: Vec<(Vec3, Vec3, Vec3)> = Vec::new();
            let mut net = 0.0;
            let mut direction: Option<Vec3> = None;
            let mut vertical = true;
            let mut flat_face = true;
            let mut height_sum = 0.0;
            let mut corners = 0.0;
            for triangle in &primitive.triangles {
                let Some((a, b, c, normal)) =
                    oriented(&primitive.vertices, &primitive.normals, triangle)
                else {
                    continue;
                };
                volume += dot(a, cross(b, c)) / 6.0;
                every.push((a, b, c));
                for p in [a, b, c] {
                    low = low.min(p[2]);
                    high = high.max(p[2]);
                }
                let length = dot(normal, normal).sqrt();
                let unit = [normal[0] / length, normal[1] / length, normal[2] / length];
                match direction {
                    None => direction = Some(unit),
                    Some(first) if dot(first, unit) < 1.0 - 1e-9 => {
                        vertical = false;
                        flat_face = false;
                    }
                    Some(_) => {}
                }
                for p in [a, b, c] {
                    height_sum += p[2];
                    corners += 1.0;
                }
                if unit[2].abs() > 1e-6 {
                    vertical = false;
                }
                net += length / 2.0;
                flat.push((a, b, c));
            }
            if let Some(unit) = direction.filter(|_| !flat.is_empty()) {
                if !flat_face {
                    curved += 1;
                } else if unit[2].abs() >= 1.0 - 1e-9 {
                    let level = (height_sum / corners / 1e-4).round() as i64;
                    if unit[2] > 0.0 {
                        up.insert(level);
                    } else {
                        down.insert(level);
                    }
                } else if unit[2].abs() > 1e-6 {
                    sloped += 1;
                }
            }
            let Some(unit) = direction.filter(|_| vertical && !flat.is_empty()) else {
                continue;
            };
            let horizontal = unit[0].hypot(unit[1]);
            let n = [unit[0] / horizontal, unit[1] / horizontal];
            faces.push((net, outer_loop_area(&flat, n), n));
        }
        if triangles == 0 || !volume.is_finite() {
            continue;
        }
        println!(
            "wall tag:{id} volume {:.4} z {low:.4} {high:.4} vfaces {}",
            f64::abs(volume),
            faces.len()
        );
        let (open, over, shells) = closedness(&every);
        println!(
            "wall tag:{id} closed {} open {open} over {over} shells {shells}",
            u8::from(open == 0 && over == 0)
        );
        println!(
            "wall tag:{id} shape up {} down {} sloped {sloped} curved {curved}",
            up.len(),
            down.len()
        );
        faces.sort_by(|x, y| y.0.total_cmp(&x.0));
        for (net, gross, n) in faces.iter().take(SHOWN_FACES) {
            println!(
                "wall tag:{id} vface net {net:.4} gross {gross:.4} normal {:.4} {:.4}",
                n[0], n[1]
            );
        }
    }
    Ok(())
}
