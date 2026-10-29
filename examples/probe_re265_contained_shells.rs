//! RE-265 (#1103): what the contained-shell rule sees on the four BIM4LCA
//! office doors whose saved mesh holds a shell inside another.
//!
//! For each door it groups the saved mesh's triangles into shells joined
//! through vertices within a tolerance (1e-6, 2e-5, 1e-4 and 1e-3 ft) and
//! prints each shell's triangle count and signed volume, then, for the
//! smallest shell enclosing positive volume, how many of the points 1e-4 ft
//! inside its faces each larger shell holds by ray parity along two
//! directions. Last, each primitive's object index, face tag, render style,
//! material and owner, and the shell (at 2e-5 ft) its triangles fall in;
//! each saved-graphics object's signed volume; then each render style's record: its category (`m_categoryId`,
//! a negative id being a built-in category) and its scalar fields.
//!
//! Usage:
//!   cargo run --profile ci --example probe_re265_contained_shells -- MODEL.rvt

use std::collections::BTreeMap;

use rvt::RevitFile;
use rvt::research::native_document::{self, Options};
use rvt::research::native_saved_scene::extract_at_detail;

type Point = [f64; 3];

const DOORS: [u64; 4] = [1382481, 1623922, 3448563, 3444654];

fn sub(a: Point, b: Point) -> Point {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
fn dot(a: Point, b: Point) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
fn cross(a: Point, b: Point) -> Point {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn crosses(o: Point, d: Point, [a, b, c]: [Point; 3]) -> bool {
    let (e1, e2) = (sub(b, a), sub(c, a));
    let h = cross(d, e2);
    let det = dot(e1, h);
    if det.abs() < 1e-12 {
        return false;
    }
    let s = sub(o, a);
    let u = dot(s, h) / det;
    if !(0.0..=1.0).contains(&u) {
        return false;
    }
    let q = cross(s, e1);
    let v = dot(d, q) / det;
    v >= 0.0 && u + v <= 1.0 && dot(e2, q) / det > 1e-9
}

fn root(parent: &mut [usize], mut i: usize) -> usize {
    while parent[i] != i {
        parent[i] = parent[parent[i]];
        i = parent[i];
    }
    i
}

fn shells(triangles: &[[Point; 3]], tolerance: f64) -> Vec<Vec<usize>> {
    let mut cells: BTreeMap<[i64; 3], usize> = BTreeMap::new();
    let ids: Vec<[usize; 3]> = triangles
        .iter()
        .map(|t| {
            t.map(|v| {
                let next = cells.len();
                *cells
                    .entry(v.map(|c| (c / tolerance).round() as i64))
                    .or_insert(next)
            })
        })
        .collect();
    let mut parent: Vec<usize> = (0..cells.len()).collect();
    for (cell, &id) in &cells {
        for dx in -1..=1 {
            for dy in -1..=1 {
                for dz in -1..=1 {
                    if let Some(&other) = cells.get(&[cell[0] + dx, cell[1] + dy, cell[2] + dz]) {
                        let (a, b) = (root(&mut parent, id), root(&mut parent, other));
                        parent[a] = b;
                    }
                }
            }
        }
    }
    for [a, b, c] in &ids {
        let r = root(&mut parent, *a);
        for x in [*b, *c] {
            let rx = root(&mut parent, x);
            parent[rx] = r;
        }
    }
    let mut groups: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
    for (i, [a, _, _]) in ids.iter().enumerate() {
        let r = root(&mut parent, *a);
        groups.entry(r).or_default().push(i);
    }
    groups.into_values().collect()
}

fn volume(triangles: &[[Point; 3]], shell: &[usize]) -> f64 {
    shell
        .iter()
        .map(|&i| {
            let [a, b, c] = triangles[i];
            dot(a, cross(b, c)) / 6.0
        })
        .sum()
}

fn main() -> anyhow::Result<()> {
    let path = std::env::args().nth(1).expect("usage: MODEL.rvt");
    let mut rf = RevitFile::open(&path)?;
    let options = Options {
        selected_ids: DOORS.into_iter().collect(),
        max_graph_values: 1_000_000,
        max_graph_objects: 1_000_000,
        ..Default::default()
    };
    let scene = extract_at_detail(&mut rf, &options, 3)?;
    let mut styles = std::collections::BTreeSet::new();
    for element in &scene.elements {
        let id = element.identity.element_id;
        if !DOORS.contains(&id) {
            continue;
        }
        let mut triangles: Vec<[Point; 3]> = Vec::new();
        let mut primitive_of: Vec<usize> = Vec::new();
        for (index, p) in element.meshes.primitives.iter().enumerate() {
            for t in &p.triangles {
                if let [Some(a), Some(b), Some(c)] = t.map(|i| p.vertices.get(i as usize).copied())
                {
                    triangles.push([a, b, c]);
                    primitive_of.push(index);
                }
            }
        }
        println!(
            "door {id}: {} primitives, {} triangles, diagnostics {}",
            element.meshes.primitives.len(),
            triangles.len(),
            element.meshes.diagnostics.len()
        );
        for tolerance in [1e-6, 2e-5, 1e-4, 1e-3] {
            let groups = shells(&triangles, tolerance);
            let mut volumes: Vec<(f64, usize, usize)> = groups
                .iter()
                .enumerate()
                .map(|(g, s)| (volume(&triangles, s), s.len(), g))
                .collect();
            volumes.sort_by(|a, b| b.0.abs().total_cmp(&a.0.abs()));
            let listed: Vec<String> = volumes
                .iter()
                .take(6)
                .map(|(v, n, _)| format!("{v:.4}/{n}"))
                .collect();
            println!(
                "  tolerance {tolerance:e}: {} shells, volume/triangles {}",
                groups.len(),
                listed.join(" ")
            );
            let Some(&(small_volume, _, small)) = volumes
                .iter()
                .filter(|(v, n, _)| *v > 0.0 && *n >= 4)
                .min_by(|a, b| a.0.total_cmp(&b.0))
            else {
                continue;
            };
            let samples: Vec<Point> = groups[small]
                .iter()
                .filter_map(|&i| {
                    let [a, b, c] = triangles[i];
                    let n = cross(sub(b, a), sub(c, a));
                    let length = dot(n, n).sqrt();
                    (length > 1e-12).then(|| {
                        let k = 1e-4 / length;
                        [
                            (a[0] + b[0] + c[0]) / 3.0 - n[0] * k,
                            (a[1] + b[1] + c[1]) / 3.0 - n[1] * k,
                            (a[2] + b[2] + c[2]) / 3.0 - n[2] * k,
                        ]
                    })
                })
                .collect();
            for (v, _, g) in &volumes {
                if *g == small || *v <= small_volume {
                    continue;
                }
                let shell: Vec<[Point; 3]> = groups[*g].iter().map(|&i| triangles[i]).collect();
                let inside = |d: Point| {
                    samples
                        .iter()
                        .filter(|&&p| shell.iter().filter(|&&t| crosses(p, d, t)).count() % 2 == 1)
                        .count()
                };
                println!(
                    "    smallest positive shell {small_volume:.4}: of {} inset points, inside shell {v:.4}: {} along (0.48,0.57,0.66), {} along (0.13,-0.71,0.69)",
                    samples.len(),
                    inside([0.4817, 0.5743, 0.6619]),
                    inside([0.1301, -0.7127, 0.6893])
                );
            }
        }
        let mut by_object: BTreeMap<usize, (usize, f64)> = BTreeMap::new();
        for (t, &owner) in primitive_of.iter().enumerate() {
            let [a, b, c] = triangles[t];
            let entry = by_object
                .entry(element.meshes.primitives[owner].object_index)
                .or_default();
            entry.0 += 1;
            entry.1 += dot(a, cross(b, c)) / 6.0;
        }
        for (object, (count, signed)) in &by_object {
            println!("  object {object}: {count} triangles, signed volume {signed:.4} cubic ft");
        }
        let groups = shells(&triangles, 2e-5);
        let mut shell_of = vec![0usize; triangles.len()];
        for (g, shell) in groups.iter().enumerate() {
            for &i in shell {
                shell_of[i] = g;
            }
        }
        for (index, p) in element.meshes.primitives.iter().enumerate() {
            let in_shells: std::collections::BTreeSet<usize> = primitive_of
                .iter()
                .enumerate()
                .filter(|(_, owner)| **owner == index)
                .map(|(t, _)| shell_of[t])
                .collect();
            styles.insert(p.render_style_id);
            println!(
                "  primitive {index}: object {} face {} style {} material {:?} owner {:?} triangles {} shell {:?}",
                p.object_index,
                p.face_tag,
                p.render_style_id,
                p.material_id,
                p.source_owner_id,
                p.triangles.len(),
                in_shells
            );
        }
    }
    let wanted: std::collections::BTreeSet<u64> = styles
        .iter()
        .filter_map(|&id| u64::try_from(id).ok())
        .collect();
    let options = Options {
        selected_ids: wanted.clone(),
        ..Default::default()
    };
    native_document::extract(&mut rf, &options, |record| {
        if !wanted.contains(&record.identity.element_id) {
            return Ok(());
        }
        let Some(graph) = &record.graph else {
            return Ok(());
        };
        for (n, object) in graph.objects.iter().enumerate().take(4) {
            let scalars: Vec<String> = object
                .fields
                .as_object()
                .into_iter()
                .flatten()
                .filter(|(_, v)| !v.is_object() && !v.is_array())
                .map(|(k, v)| format!("{k}={v}"))
                .collect();
            let ids: Vec<String> = object
                .fields
                .as_object()
                .into_iter()
                .flatten()
                .filter_map(|(k, v)| v.pointer("/m_id/m_id64").map(|id| format!("{k}={id}")))
                .collect();
            println!(
                "style {} object {n} {}: {} {}",
                record.identity.element_id,
                object.class_name,
                ids.join(" "),
                scalars.join(" ")
            );
        }
        Ok(())
    })?;
    Ok(())
}
