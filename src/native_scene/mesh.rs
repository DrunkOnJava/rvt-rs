use super::Mesh;
use crate::geometry::native_2027::{NativeWallGeometry, WallCurveKind};
use anyhow::{Result, ensure};
use std::collections::BTreeMap;
fn quad(t: &mut Vec<[u32; 3]>, a: u32, b: u32, c: u32, d: u32) {
    t.extend([[a, b, c], [a, c, d]]);
}
pub fn wall(g: &NativeWallGeometry, kind: WallCurveKind) -> Result<Mesh> {
    let [lo, base, hi, top] = g.surfaces[0].domain;
    let n = if kind == WallCurveKind::Line {
        1
    } else {
        ((hi - lo) / std::f64::consts::TAU * 1024.).ceil() as usize
    };
    ensure!((1..=4096).contains(&n), "wall tessellation budget exceeded");
    let mut vertices = Vec::new();
    let mut triangles = Vec::new();
    for i in 0..=n {
        let u = lo + (hi - lo) * i as f64 / n as f64;
        for v in [base, top] {
            for s in &g.surfaces[1..3] {
                let point = std::array::from_fn(|j| {
                    if let Some(r) = s.radius {
                        s.origin[j]
                            + r * (u.cos() * s.basis_x[j] + u.sin() * s.basis_y[j])
                            + v * s.basis_z.unwrap()[j]
                    } else {
                        s.origin[j] + u * s.basis_x[j] + v * s.basis_y[j]
                    }
                });
                vertices.push(point);
            }
        }
    }
    for i in 0..n as u32 {
        let a = 4 * i;
        let b = a + 4;
        quad(&mut triangles, a, b, b + 1, a + 1);
        quad(&mut triangles, a + 2, a + 3, b + 3, b + 2);
        quad(&mut triangles, a, a + 2, b + 2, b);
        quad(&mut triangles, a + 1, b + 1, b + 3, a + 3);
    }
    quad(&mut triangles, 0, 1, 3, 2);
    let a = 4 * n as u32;
    quad(&mut triangles, a, a + 2, a + 3, a + 1);
    let mut m = Mesh {
        vertices,
        triangles,
    };
    validate(&mut m)?;
    Ok(m)
}
fn cross(a: [f64; 2], b: [f64; 2], c: [f64; 2]) -> f64 {
    (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0])
}
fn area(p: &[[f64; 2]]) -> f64 {
    p.iter()
        .zip(p.iter().cycle().skip(1))
        .take(p.len())
        .map(|(a, b)| a[0] * b[1] - a[1] * b[0])
        .sum::<f64>()
        / 2.
}
fn near(a: [f64; 2], b: [f64; 2]) -> bool {
    (a[0] - b[0]).hypot(a[1] - b[1]) < 1e-8
}
fn inside(p: [f64; 2], ring: &[[f64; 2]]) -> bool {
    let mut yes = false;
    for (a, b) in ring
        .iter()
        .zip(ring.iter().cycle().skip(1))
        .take(ring.len())
    {
        if (a[1] > p[1]) != (b[1] > p[1])
            && p[0] < (b[0] - a[0]) * (p[1] - a[1]) / (b[1] - a[1]) + a[0]
        {
            yes = !yes;
        }
    }
    yes
}
fn on(a: [f64; 2], b: [f64; 2], p: [f64; 2]) -> bool {
    cross(a, b, p).abs() < 1e-9
        && p[0] >= a[0].min(b[0]) - 1e-9
        && p[0] <= a[0].max(b[0]) + 1e-9
        && p[1] >= a[1].min(b[1]) - 1e-9
        && p[1] <= a[1].max(b[1]) + 1e-9
}
fn intersect(a: [f64; 2], b: [f64; 2], c: [f64; 2], d: [f64; 2]) -> bool {
    (cross(a, b, c) * cross(a, b, d) < 0. && cross(c, d, a) * cross(c, d, b) < 0.)
        || on(a, b, c)
        || on(a, b, d)
        || on(c, d, a)
        || on(c, d, b)
}
pub fn loops(mut edges: Vec<([f64; 3], [f64; 3])>) -> Result<Vec<Vec<[f64; 2]>>> {
    ensure!(edges.len() <= 8192, "profile edge budget exceeded");
    let mut result = Vec::new();
    while let Some((a, b)) = edges.pop() {
        let mut ring = vec![[a[0], a[1]], [b[0], b[1]]];
        ensure!(!near(ring[0], ring[1]), "zero length profile edge");
        while !near(*ring.last().unwrap(), ring[0]) {
            let end = *ring.last().unwrap();
            let matches: Vec<_> = edges
                .iter()
                .enumerate()
                .filter_map(|(i, (a, b))| {
                    if near([a[0], a[1]], end) {
                        Some((i, [b[0], b[1]]))
                    } else if near([b[0], b[1]], end) {
                        Some((i, [a[0], a[1]]))
                    } else {
                        None
                    }
                })
                .collect();
            ensure!(matches.len() == 1, "open or ambiguous profile graph");
            let (i, p) = matches[0];
            edges.remove(i);
            ring.push(p);
        }
        ring.pop();
        ensure!(ring.len() >= 3, "degenerate profile");
        result.push(ring);
    }
    ensure!(!result.is_empty(), "empty profile");
    result.sort_by(|a, b| area(b).abs().total_cmp(&area(a).abs()));
    for (r, ring) in result.iter().enumerate() {
        ensure!(
            ring.iter().flatten().all(|x| x.is_finite()) && area(ring).abs() > 1e-10,
            "nonfinite or zero profile area"
        );
        for i in 0..ring.len() {
            for j in i + 1..ring.len() {
                if j == i + 1 || (i == 0 && j == ring.len() - 1) {
                    continue;
                }
                ensure!(
                    !intersect(
                        ring[i],
                        ring[(i + 1) % ring.len()],
                        ring[j],
                        ring[(j + 1) % ring.len()]
                    ),
                    "self intersecting profile"
                );
            }
        }
        if r > 0 {
            ensure!(
                inside(ring[0], &result[0]),
                "disjoint floor profiles unsupported"
            );
            for earlier in &result[..r] {
                for i in 0..ring.len() {
                    for j in 0..earlier.len() {
                        ensure!(
                            !intersect(
                                ring[i],
                                ring[(i + 1) % ring.len()],
                                earlier[j],
                                earlier[(j + 1) % earlier.len()]
                            ),
                            "touching/intersecting rings"
                        );
                    }
                }
            }
            for earlier in &result[1..r] {
                ensure!(
                    !inside(ring[0], earlier) && !inside(earlier[0], ring),
                    "nested holes unsupported"
                );
            }
        }
    }
    for (i, ring) in result.iter_mut().enumerate() {
        if (area(ring) > 0.) != (i == 0) {
            ring.reverse();
        }
    }
    Ok(result)
}
pub fn floor(rings: &[Vec<[f64; 2]>], bottom: f64, top: f64) -> Result<Mesh> {
    ensure!(
        top.is_finite() && bottom.is_finite() && top > bottom,
        "invalid floor elevations"
    );
    let mut points = Vec::new();
    let mut holes = Vec::<u32>::new();
    let mut boundaries = Vec::new();
    for (i, r) in rings.iter().enumerate() {
        if i > 0 {
            holes.push(points.len() as u32);
        }
        let start = points.len();
        points.extend(r);
        boundaries.push(start..points.len());
    }
    ensure!(points.len() <= 8192, "profile vertex budget exceeded");
    let mut cap = Vec::<u32>::new();
    earcut::Earcut::new().earcut(points.iter().copied(), &holes, &mut cap);
    ensure!(
        !cap.is_empty() && cap.len() % 3 == 0,
        "triangulation failed"
    );
    let expected = area(&rings[0]).abs() - rings[1..].iter().map(|r| area(r).abs()).sum::<f64>();
    let mut sum = 0.;
    for t in cap.chunks_exact_mut(3) {
        ensure!(
            t.iter().all(|i| (*i as usize) < points.len()),
            "invalid triangulation index"
        );
        let a = points[t[0] as usize];
        let b = points[t[1] as usize];
        let c = points[t[2] as usize];
        let signed = cross(a, b, c) / 2.;
        ensure!(signed.abs() > 1e-14, "degenerate cap triangle");
        sum += signed.abs();
        if signed < 0. {
            t.swap(1, 2);
        }
        let p = [(a[0] + b[0] + c[0]) / 3., (a[1] + b[1] + c[1]) / 3.];
        ensure!(
            inside(p, &rings[0]) && !rings[1..].iter().any(|r| inside(p, r)),
            "triangulation enters void/outside profile"
        );
    }
    ensure!(
        expected > 0. && (sum - expected).abs() <= expected.max(1.) * 1e-9,
        "triangulation area mismatch"
    );
    let n = points.len() as u32;
    let mut vertices = Vec::new();
    for z in [bottom, top] {
        vertices.extend(points.iter().map(|p| [p[0], p[1], z]));
    }
    let mut triangles = Vec::new();
    for t in cap.chunks_exact(3) {
        triangles.push([t[0], t[2], t[1]]);
        triangles.push([t[0] + n, t[1] + n, t[2] + n]);
    }
    for range in boundaries {
        for i in range.clone() {
            let j = if i + 1 == range.end {
                range.start
            } else {
                i + 1
            };
            quad(
                &mut triangles,
                i as u32,
                j as u32,
                j as u32 + n,
                i as u32 + n,
            );
        }
    }
    let mut mesh = Mesh {
        vertices,
        triangles,
    };
    validate(&mut mesh)?;
    Ok(mesh)
}
pub fn validate(mesh: &mut Mesh) -> Result<()> {
    ensure!(
        mesh.vertices.iter().flatten().all(|v| v.is_finite()),
        "nonfinite mesh vertex"
    );
    let mut edges = BTreeMap::<(u32, u32), (u32, i32)>::new();
    let mut volume = 0.;
    let origin = *mesh
        .vertices
        .first()
        .ok_or_else(|| anyhow::anyhow!("empty mesh"))?;
    for t in &mesh.triangles {
        ensure!(
            t.iter().all(|&i| (i as usize) < mesh.vertices.len()),
            "invalid mesh index"
        );
        let [a, b, c] =
            t.map(|i| std::array::from_fn::<_, 3, _>(|j| mesh.vertices[i as usize][j] - origin[j]));
        let cross = [
            b[1] * c[2] - b[2] * c[1],
            b[2] * c[0] - b[0] * c[2],
            b[0] * c[1] - b[1] * c[0],
        ];
        volume += a.iter().zip(cross).map(|(a, b)| a * b).sum::<f64>() / 6.;
        for (a, b) in [(t[0], t[1]), (t[1], t[2]), (t[2], t[0])] {
            ensure!(a != b, "degenerate triangle edge");
            let item = edges.entry((a.min(b), a.max(b))).or_default();
            item.0 += 1;
            item.1 += if a < b { 1 } else { -1 };
        }
    }
    ensure!(
        volume.is_finite() && volume.abs() > 1e-12,
        "zero or nonfinite solid volume"
    );
    ensure!(
        edges.values().all(|v| *v == (2, 0)),
        "mesh is not consistently closed"
    );
    if volume < 0. {
        for t in &mut mesh.triangles {
            t.swap(1, 2);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn edges(rings: &[Vec<[f64; 2]>]) -> Vec<([f64; 3], [f64; 3])> {
        rings
            .iter()
            .flat_map(|r| {
                r.iter()
                    .zip(r.iter().cycle().skip(1))
                    .take(r.len())
                    .map(|(a, b)| ([a[0], a[1], 0.], [b[0], b[1], 0.]))
            })
            .collect()
    }
    #[test]
    fn multiple_holes_produce_closed_solid() {
        let rings = vec![
            vec![[0., 0.], [14., 0.], [14., 12.], [0., 12.]],
            vec![[2., 2.], [4., 2.], [4., 5.], [2., 5.]],
            vec![[8., 6.], [10., 6.], [10., 9.], [8., 9.]],
        ];
        let ordered = loops(edges(&rings)).unwrap();
        let mut m = floor(&ordered, 0., 0.75).unwrap();
        validate(&mut m).unwrap();
        assert_eq!(m.vertices.len(), 24);
    }
    #[test]
    fn invalid_profiles_are_not_triangulated() {
        let bowtie = vec![vec![[0., 0.], [2., 2.], [0., 2.], [2., 0.]]];
        assert!(loops(edges(&bowtie)).is_err());
        let touching = vec![
            vec![[0., 0.], [4., 0.], [4., 4.], [0., 4.]],
            vec![[0., 1.], [1., 1.], [1., 2.], [0., 2.]],
        ];
        assert!(loops(edges(&touching)).is_err());
        let disconnected = vec![
            vec![[0., 0.], [1., 0.], [1., 1.], [0., 1.]],
            vec![[5., 5.], [6., 5.], [6., 6.], [5., 6.]],
        ];
        assert!(loops(edges(&disconnected)).is_err());
    }
    #[test]
    fn open_mesh_is_rejected() {
        let mut m = Mesh {
            vertices: vec![[0., 0., 0.], [1., 0., 0.], [0., 1., 0.]],
            triangles: vec![[0, 1, 2]],
        };
        assert!(validate(&mut m).is_err());
    }
}
