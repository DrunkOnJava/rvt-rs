//! Closed boundary of contiguous vertical slabs with convex planar occupancy.
//!
//! All slabs share one planar arrangement. Boundary edges are globally split,
//! so changing profiles cannot leave a hanging vertex or an internal cap.
use super::{Mesh, mesh};
use anyhow::{Result, ensure};
use std::collections::BTreeMap;

type Point = [f64; 2];
const CELL_LIMIT: usize = 4096;
const POINT_LIMIT: usize = 32768;

pub(crate) struct Layer {
    pub bottom: f64,
    pub top: f64,
    /// Disjoint interiors, each ring counterclockwise and convex.
    pub profiles: Vec<Vec<Point>>,
}

fn cross(a: Point, b: Point, c: Point) -> f64 {
    (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0])
}
fn area(p: &[Point]) -> f64 {
    let o = p[0];
    (1..p.len() - 1)
        .map(|i| cross(o, p[i], p[i + 1]))
        .sum::<f64>()
        / 2.
}
fn near(a: Point, b: Point, eps: f64) -> bool {
    (a[0] - b[0]).hypot(a[1] - b[1]) <= eps
}
fn center(p: &[Point]) -> Point {
    let o = p[0];
    std::array::from_fn(|k| o[k] + p.iter().map(|v| v[k] - o[k]).sum::<f64>() / p.len() as f64)
}
fn edges(p: &[Point]) -> impl Iterator<Item = (Point, Point)> + '_ {
    p.iter()
        .copied()
        .zip(p.iter().copied().cycle().skip(1))
        .take(p.len())
}
fn valid_ring(p: &[Point], eps: f64) -> Result<()> {
    ensure!(
        (3..=64).contains(&p.len()),
        "layered prism ring vertex budget"
    );
    ensure!(
        area(p) > eps * eps,
        "layered prism requires positive CCW ring"
    );
    for (i, a) in p.iter().enumerate() {
        ensure!(
            p[i + 1..].iter().all(|b| !near(*a, *b, eps)),
            "layered prism repeated vertex"
        );
    }
    for (a, b) in edges(p) {
        let length = (b[0] - a[0]).hypot(b[1] - a[1]);
        ensure!(
            p.iter().all(|v| cross(a, b, *v) >= -eps * length),
            "layered prism nonconvex profile"
        );
    }
    Ok(())
}
fn signed(p: Point, line: [f64; 3]) -> f64 {
    line[0] * p[0] + line[1] * p[1] - line[2]
}
fn clipped(p: &[Point], line: [f64; 3], sign: f64, eps: f64) -> Vec<Point> {
    let mut out = Vec::new();
    for (a, b) in edges(p) {
        let da = sign * signed(a, line);
        let db = sign * signed(b, line);
        if da >= -eps {
            out.push(a);
        }
        if (da > eps && db < -eps) || (da < -eps && db > eps) {
            let t = da / (da - db);
            out.push([a[0] + t * (b[0] - a[0]), a[1] + t * (b[1] - a[1])]);
        }
    }
    out.dedup_by(|a, b| near(*a, *b, eps));
    if out.len() > 1 && near(out[0], *out.last().unwrap(), eps) {
        out.pop();
    }
    out
}
fn intern(points: &mut Vec<Point>, p: Point, eps: f64) -> Result<usize> {
    if let Some(i) = points.iter().position(|v| near(*v, p, eps)) {
        return Ok(i);
    }
    ensure!(points.len() < POINT_LIMIT, "layered prism point budget");
    points.push(p);
    Ok(points.len() - 1)
}

/// Construct only occupied/unoccupied interfaces; never concatenate closed prisms.
/// Input lengths use the caller's common coordinate system and units.
pub(crate) fn build(layers: &[Layer]) -> Result<Mesh> {
    ensure!(
        !layers.is_empty() && layers.len() <= 16,
        "layered prism layer budget"
    );
    ensure!(
        layers.iter().all(|l| l.bottom.is_finite()
            && l.top.is_finite()
            && l.top > l.bottom
            && !l.profiles.is_empty()
            && l.profiles.len() <= 16),
        "layered prism invalid slab"
    );
    ensure!(
        layers.windows(2).all(|w| w[0].top == w[1].bottom),
        "layered prism slabs must be exactly contiguous and ordered"
    );
    let inputs: Vec<Point> = layers
        .iter()
        .flat_map(|l| l.profiles.iter().flatten().copied())
        .collect();
    ensure!(
        !inputs.is_empty()
            && inputs.len() <= 1024
            && inputs.iter().flatten().all(|x| x.is_finite()),
        "layered prism input coordinate budget"
    );
    let low: Point =
        std::array::from_fn(|k| inputs.iter().map(|p| p[k]).fold(f64::INFINITY, f64::min));
    let high: Point = std::array::from_fn(|k| {
        inputs
            .iter()
            .map(|p| p[k])
            .fold(f64::NEG_INFINITY, f64::max)
    });
    let extent = (high[0] - low[0]).max(high[1] - low[1]);
    let eps = extent.max(1.) * 1e-10;
    ensure!(
        high[0] - low[0] > eps && high[1] - low[1] > eps,
        "layered prism degenerate bounds"
    );
    let mut lines: Vec<[f64; 3]> = Vec::new();
    for profile in layers.iter().flat_map(|l| &l.profiles) {
        valid_ring(profile, eps)?;
        for (a, b) in edges(profile) {
            let length = (b[0] - a[0]).hypot(b[1] - a[1]);
            let mut line = [-(b[1] - a[1]) / length, (b[0] - a[0]) / length, 0.];
            if line[0] < 0. || (line[0] == 0. && line[1] < 0.) {
                line[0] = -line[0];
                line[1] = -line[1];
            }
            line[2] = line[0] * a[0] + line[1] * a[1];
            if !lines.iter().any(|l| {
                (l[0] - line[0]).abs() < 1e-12
                    && (l[1] - line[1]).abs() < 1e-12
                    && (l[2] - line[2]).abs() <= eps
            }) {
                lines.push(line);
            }
        }
    }
    ensure!(lines.len() <= 128, "layered prism line budget");
    let mut cells = vec![vec![low, [high[0], low[1]], high, [low[0], high[1]]]];
    for line in lines {
        let mut next = Vec::new();
        for p in cells {
            if p.iter().any(|v| signed(*v, line) > eps) && p.iter().any(|v| signed(*v, line) < -eps)
            {
                for side in [1., -1.] {
                    let c = clipped(&p, line, side, eps);
                    ensure!(
                        c.len() >= 3 && area(&c) > eps * eps,
                        "layered prism unstable planar split"
                    );
                    next.push(c);
                }
            } else {
                next.push(p);
            }
        }
        ensure!(next.len() <= CELL_LIMIT, "layered prism arrangement budget");
        cells = next;
    }
    let mut occupied = vec![vec![false; cells.len()]; layers.len()];
    for (li, layer) in layers.iter().enumerate() {
        for (ci, cell) in cells.iter().enumerate() {
            let c = center(cell);
            let count = layer
                .profiles
                .iter()
                .filter(|p| {
                    edges(p).all(|(a, b)| cross(a, b, c) >= -eps * (b[0] - a[0]).hypot(b[1] - a[1]))
                })
                .count();
            ensure!(count <= 1, "layered prism overlapping profile interiors");
            occupied[li][ci] = count == 1;
        }
        ensure!(
            occupied[li].iter().any(|x| *x),
            "layered prism empty occupancy"
        );
    }
    let mut points = Vec::new();
    for p in inputs.iter().chain(cells.iter().flatten()) {
        intern(&mut points, *p, eps)?;
    }
    let mut rings = Vec::new();
    // Split every edge at every shared collinear point, including authored vertices.
    for cell in &cells {
        let mut ring = Vec::new();
        for (a, b) in edges(cell) {
            let dx = b[0] - a[0];
            let dy = b[1] - a[1];
            let length2 = dx * dx + dy * dy;
            let mut along: Vec<(f64, usize)> = points
                .iter()
                .enumerate()
                .filter_map(|(i, p)| {
                    let t = ((p[0] - a[0]) * dx + (p[1] - a[1]) * dy) / length2;
                    (t >= -eps / length2.sqrt()
                        && t < 1. - eps / length2.sqrt()
                        && cross(a, b, *p).abs() <= eps * length2.sqrt())
                    .then_some((t, i))
                })
                .collect();
            along.sort_by(|a, b| a.0.total_cmp(&b.0));
            ring.extend(along.into_iter().map(|(_, i)| i));
        }
        ensure!(ring.len() >= 3, "layered prism incomplete cell ring");
        rings.push(ring);
    }
    let mut neighbors: BTreeMap<(usize, usize), Vec<usize>> = BTreeMap::new();
    for (ci, ring) in rings.iter().enumerate() {
        for (&a, &b) in ring
            .iter()
            .zip(ring.iter().cycle().skip(1))
            .take(ring.len())
        {
            neighbors.entry((a.min(b), a.max(b))).or_default().push(ci);
        }
    }
    ensure!(
        neighbors.values().all(|v| v.len() <= 2),
        "layered prism nonmanifold planar arrangement"
    );
    let mut heights = vec![layers[0].bottom];
    heights.extend(layers.iter().map(|l| l.top));
    let mut vertices = Vec::new();
    let mut ids = BTreeMap::new();
    let mut triangles = Vec::new();
    {
        let mut vertex = |point: usize, level: usize| -> u32 {
            *ids.entry((point, level)).or_insert_with(|| {
                let i = vertices.len() as u32;
                vertices.push([points[point][0], points[point][1], heights[level]]);
                i
            })
        };
        for (li, active) in occupied.iter().enumerate() {
            for (ci, ring) in rings.iter().enumerate().filter(|(ci, _)| active[*ci]) {
                for (&a, &b) in ring
                    .iter()
                    .zip(ring.iter().cycle().skip(1))
                    .take(ring.len())
                {
                    if !neighbors[&(a.min(b), a.max(b))]
                        .iter()
                        .any(|other| *other != ci && active[*other])
                    {
                        let [a, b, c, d] = [
                            vertex(a, li),
                            vertex(b, li),
                            vertex(b, li + 1),
                            vertex(a, li + 1),
                        ];
                        triangles.extend([[a, b, c], [a, c, d]]);
                    }
                }
            }
        }
    }
    for (ci, ring) in rings.iter().enumerate() {
        let c = center(&cells[ci]);
        for level in 0..heights.len() {
            let below = level > 0 && occupied[level - 1][ci];
            let above = level < layers.len() && occupied[level][ci];
            if below == above {
                continue;
            }
            let center_id = vertices.len() as u32;
            vertices.push([c[0], c[1], heights[level]]);
            for (&a, &b) in ring
                .iter()
                .zip(ring.iter().cycle().skip(1))
                .take(ring.len())
            {
                let mut get = |p: usize| {
                    *ids.entry((p, level)).or_insert_with(|| {
                        let id = vertices.len() as u32;
                        vertices.push([points[p][0], points[p][1], heights[level]]);
                        id
                    })
                };
                let a = get(a);
                let b = get(b);
                triangles.push(if below {
                    [center_id, a, b]
                } else {
                    [center_id, b, a]
                });
            }
        }
    }
    let mut result = Mesh {
        vertices,
        triangles,
    };
    mesh::validate(&mut result)?;
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn rect(a: f64, b: f64) -> Vec<Point> {
        vec![[a, 0.], [b, 0.], [b, 2.], [a, 2.]]
    }
    fn fixture() -> Vec<Layer> {
        vec![
            Layer {
                bottom: 0.,
                top: 2.,
                profiles: vec![rect(0., 10.)],
            },
            Layer {
                bottom: 2.,
                top: 5.,
                profiles: vec![rect(0., 4.), rect(6., 10.)],
            },
            Layer {
                bottom: 5.,
                top: 8.,
                profiles: vec![rect(0., 10.)],
            },
        ]
    }
    fn volume(m: &Mesh) -> f64 {
        m.triangles
            .iter()
            .map(|t| {
                let [a, b, c] = t.map(|i| m.vertices[i as usize]);
                (a[0] * (b[1] * c[2] - b[2] * c[1])
                    + a[1] * (b[2] * c[0] - b[0] * c[2])
                    + a[2] * (b[0] * c[1] - b[1] * c[0]))
                    / 6.
            })
            .sum()
    }
    #[test]
    fn floating_cut_is_closed_without_internal_caps() {
        let mut m = build(&fixture()).unwrap();
        mesh::validate(&mut m).unwrap();
        assert!((volume(&m) - 148.).abs() < 1e-9);
        let mut cap_area = 0.;
        for t in &m.triangles {
            let p = t.map(|i| m.vertices[i as usize]);
            if p.iter().all(|v| v[2] == 2.) || p.iter().all(|v| v[2] == 5.) {
                cap_area +=
                    cross([p[0][0], p[0][1]], [p[1][0], p[1][1]], [p[2][0], p[2][1]]).abs() / 2.;
            }
        }
        assert!((cap_area - 8.).abs() < 1e-9);
    }
    #[test]
    fn rotated_cut_preserves_volume_and_collinear_vertices() {
        let mut layers = fixture();
        layers[0].profiles[0].insert(1, [3., 0.]);
        for p in layers
            .iter_mut()
            .flat_map(|l| l.profiles.iter_mut().flatten())
        {
            let [x, y] = *p;
            let (s, c) = 0.37_f64.sin_cos();
            *p = [100. + c * x - s * y, -70. + s * x + c * y];
        }
        let mut m = build(&layers).unwrap();
        mesh::validate(&mut m).unwrap();
        assert!((volume(&m) - 148.).abs() < 1e-8);
    }
    #[test]
    fn oblique_floating_strip_is_closed() {
        let mut layers = fixture();
        layers[1].profiles = vec![
            vec![[0., 0.], [4., 0.], [6., 2.], [0., 2.]],
            vec![[6., 0.], [10., 0.], [10., 2.], [8., 2.]],
        ];
        let mut m = build(&layers).unwrap();
        mesh::validate(&mut m).unwrap();
        assert!((volume(&m) - 148.).abs() < 1e-9);
    }

    #[test]
    fn invalid_layers_fail_without_partial_mesh() {
        let mut layers = fixture();
        layers[1].bottom = 2.1;
        assert!(build(&layers).is_err());
        let mut layers = fixture();
        layers[1].profiles.push(rect(3., 7.));
        assert!(build(&layers).is_err());
        let mut layers = fixture();
        layers[0].profiles[0].reverse();
        assert!(build(&layers).is_err());
    }
}
