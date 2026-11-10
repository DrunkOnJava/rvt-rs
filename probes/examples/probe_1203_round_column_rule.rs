//! #1203 (RE-411): RvtIO's acceptance rule for a round column, run on the saved mesh of
//! every column beside RE-353's rule, which reads the caption `Diameter` of the
//! column's type.
//!
//! FACT under test: RvtIO (db6c5053, discussion 1241, `RoundCylinderCurrent`) takes a
//! column for a vertical cylinder when exactly two of its faces are planes; each is
//! one loop with no hole and at least eight boundary points, horizontal, on a circle
//! of one centre and one radius with the other, at elevations at least 1e-9 ft apart;
//! each cap's area is within 0.5% of pi r^2; and every point of every face lies on that
//! cylinder, between the caps, to `max(1e-6 r, 8 ULP32(largest coordinate))`. The rule
//! is RvtIO's on the faces its reader decodes; here it is run on the faces of the mesh
//! Revit saved for the column (one primitive for each face, its tessellation), so a cap's
//! points are the saved mesh's, and a cap's area is that of its triangles. RE-353 takes a
//! column as round where its type's length parameters are `Diameter` and at most the
//! round column family's two zero offsets (`m_column_round_diameter`).
//!
//! Output, one line for each column of a class `Column` or `StructuralColumn`:
//!   column <id> <class> re353 <round|no> rvtio <round|no: why> [r <feet> height <feet>
//!   cap points <a>/<b> cap area ratio <a>/<b>] planar <n> of <faces> faces
//! and then `tally <release>: ...`, the columns by what each rule says.
//!
//! Usage:
//!   cargo run --profile ci -p rvt-probes --example probe_1203_round_column_rule -- MODEL.rvt

use std::collections::{BTreeMap, BTreeSet};

use rvt::RevitFile;
use rvt::research::native_document::Options;
use rvt::research::native_saved_mesh::Primitive;
use rvt::research::native_saved_scene::extract_at_detail;
use rvt::research::partition_schema_mvp::recover_partition_schema_mvp;
use rvt::walker::{InstanceField, WalkerLimits};

const COLUMN_CLASSES: [&str; 2] = ["Column", "StructuralColumn"];
const ROUND_DIAMETER_FIELD: &str = "m_column_round_diameter";

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

fn norm(a: Vec3) -> f64 {
    (a[0] * a[0] + a[1] * a[1] + a[2] * a[2]).sqrt()
}

/// The spacing of single-precision numbers at `x`, feet.
fn ulp32(x: f64) -> f64 {
    let magnitude = x.abs().max(f64::from(f32::MIN_POSITIVE));
    2f64.powi(magnitude.log2().floor() as i32 - 23)
}

/// Whether every vertex of `face` lies within `tolerance` of the plane of its first
/// triangle that has an area.
fn is_planar(face: &Primitive, tolerance: f64) -> bool {
    let point = |i: u32| face.vertices.get(i as usize).copied();
    let plane = face.triangles.iter().find_map(|t| {
        let [a, b, c] = [point(t[0])?, point(t[1])?, point(t[2])?];
        let normal = cross(sub(b, a), sub(c, a));
        let length = norm(normal);
        (length > 1e-12).then(|| (a, normal.map(|n| n / length)))
    });
    let Some((origin, normal)) = plane else {
        return false;
    };
    face.vertices.iter().all(|&p| {
        let d = sub(p, origin);
        (d[0] * normal[0] + d[1] * normal[1] + d[2] * normal[2]).abs() <= tolerance
    })
}

/// What one cap of a cylinder is.
struct Cap {
    /// The boundary points, model feet.
    ring: Vec<Vec3>,
    /// The area of the cap's triangles, square feet.
    area: f64,
}

/// A cap's boundary and area, or why the face is not a cap without a hole.
fn cap_of(face: &Primitive) -> Result<Cap, String> {
    let mut ids: BTreeMap<[i64; 3], usize> = BTreeMap::new();
    let mut points: Vec<Vec3> = Vec::new();
    let mut id_of = |p: Vec3| {
        *ids.entry(p.map(|c| (c * 1e7).round() as i64))
            .or_insert_with(|| {
                points.push(p);
                points.len() - 1
            })
    };
    let mut edges: BTreeMap<(usize, usize), usize> = BTreeMap::new();
    let mut area = 0.0;
    for t in &face.triangles {
        let corners = t.map(|i| face.vertices.get(i as usize).copied());
        let [Some(a), Some(b), Some(c)] = corners else {
            return Err("a triangle names a missing vertex".into());
        };
        let (ia, ib, ic) = (id_of(a), id_of(b), id_of(c));
        for (x, y) in [(ia, ib), (ib, ic), (ic, ia)] {
            *edges.entry((x.min(y), x.max(y))).or_default() += 1;
        }
        area += 0.5 * norm(cross(sub(b, a), sub(c, a)));
    }
    let mut degree: BTreeMap<usize, usize> = BTreeMap::new();
    let mut next: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
    for (&(x, y), _) in edges.iter().filter(|(_, n)| **n == 1) {
        *degree.entry(x).or_default() += 1;
        *degree.entry(y).or_default() += 1;
        next.entry(x).or_default().push(y);
        next.entry(y).or_default().push(x);
    }
    if degree.is_empty() {
        return Err("the cap has no boundary".into());
    }
    if degree.values().any(|&d| d != 2) {
        return Err("the cap's boundary is not one simple loop".into());
    }
    // One loop: walking the boundary from any point reaches every boundary point.
    let start = *degree.keys().next().expect("a boundary point");
    let mut seen = BTreeSet::from([start]);
    let mut stack = vec![start];
    while let Some(at) = stack.pop() {
        for &to in &next[&at] {
            if seen.insert(to) {
                stack.push(to);
            }
        }
    }
    if seen.len() != degree.len() {
        return Err("the cap has a hole or several loops".into());
    }
    Ok(Cap {
        ring: degree.keys().map(|&i| points[i]).collect(),
        area,
    })
}

/// The circle through `ring`'s points in plan, least squares: centre and radius.
fn fit_circle(ring: &[Vec3]) -> Option<([f64; 2], f64)> {
    let n = ring.len() as f64;
    let mean = [
        ring.iter().map(|p| p[0]).sum::<f64>() / n,
        ring.iter().map(|p| p[1]).sum::<f64>() / n,
    ];
    // x^2 + y^2 + D x + E y + F = 0 on the points measured from their mean.
    let (mut sxx, mut sxy, mut syy, mut sx, mut sy) = (0.0, 0.0, 0.0, 0.0, 0.0);
    let (mut sxz, mut syz, mut sz) = (0.0, 0.0, 0.0);
    for p in ring {
        let (x, y) = (p[0] - mean[0], p[1] - mean[1]);
        let z = x * x + y * y;
        sxx += x * x;
        sxy += x * y;
        syy += y * y;
        sx += x;
        sy += y;
        sxz += x * z;
        syz += y * z;
        sz += z;
    }
    let a = [[sxx, sxy, sx], [sxy, syy, sy], [sx, sy, n]];
    let b = [-sxz, -syz, -sz];
    let det = |m: [[f64; 3]; 3]| {
        m[0][0] * (m[1][1] * m[2][2] - m[1][2] * m[2][1])
            - m[0][1] * (m[1][0] * m[2][2] - m[1][2] * m[2][0])
            + m[0][2] * (m[1][0] * m[2][1] - m[1][1] * m[2][0])
    };
    let d = det(a);
    if d.abs() < 1e-18 {
        return None;
    }
    let with = |column: usize| {
        let mut m = a;
        for (row, value) in b.iter().enumerate() {
            m[row][column] = *value;
        }
        det(m) / d
    };
    let (dd, ee, ff) = (with(0), with(1), with(2));
    let centre = [-dd / 2.0, -ee / 2.0];
    let radius_squared = centre[0] * centre[0] + centre[1] * centre[1] - ff;
    (radius_squared > 0.0).then(|| {
        (
            [centre[0] + mean[0], centre[1] + mean[1]],
            radius_squared.sqrt(),
        )
    })
}

/// What RvtIO's rule finds of a round column.
struct Round {
    radius: f64,
    height: f64,
    cap_points: [usize; 2],
    cap_area_ratio: [f64; 2],
}

fn rvtio_rule(faces: &[&Primitive]) -> Result<Round, String> {
    let all: Vec<Vec3> = faces
        .iter()
        .flat_map(|face| face.vertices.iter().copied())
        .collect();
    if all.is_empty() {
        return Err("no vertices".into());
    }
    let largest = all
        .iter()
        .flat_map(|p| p.iter().map(|c| c.abs()))
        .fold(0.0, f64::max);
    let ulp = 8.0 * ulp32(largest);
    let planes: Vec<&&Primitive> = faces.iter().filter(|f| is_planar(f, ulp)).collect();
    if planes.len() != 2 {
        return Err(format!("{} planar faces, not 2", planes.len()));
    }
    let mut caps = Vec::new();
    for face in planes {
        let cap = cap_of(face)?;
        if cap.ring.len() < 8 {
            return Err(format!(
                "a cap has {} boundary points, not 8",
                cap.ring.len()
            ));
        }
        let z = cap.ring[0][2];
        if cap.ring.iter().any(|p| (p[2] - z).abs() > ulp) {
            return Err("a cap is not horizontal".into());
        }
        let (centre, radius) = fit_circle(&cap.ring).ok_or("a cap's points have no circle")?;
        let tolerance = (1e-6 * radius).max(ulp);
        let off = cap
            .ring
            .iter()
            .map(|p| ((p[0] - centre[0]).hypot(p[1] - centre[1]) - radius).abs())
            .fold(0.0, f64::max);
        if off > tolerance {
            return Err(format!("a cap's points are {off:.6} ft off their circle"));
        }
        caps.push((cap, centre, radius, z));
    }
    let [(first, c0, r0, z0), (second, c1, r1, z1)] = [caps.remove(0), caps.remove(0)];
    let tolerance = (1e-6 * r0).max(ulp);
    if (c0[0] - c1[0]).abs() > tolerance
        || (c0[1] - c1[1]).abs() > tolerance
        || (r0 - r1).abs() > tolerance
    {
        return Err("the caps are not on one circle".into());
    }
    if (z1 - z0).abs() < 1e-9 {
        return Err("the caps are at one elevation".into());
    }
    let area_ratio = |cap: &Cap, radius: f64| cap.area / (std::f64::consts::PI * radius * radius);
    let ratios = [area_ratio(&first, r0), area_ratio(&second, r1)];
    if ratios.iter().any(|ratio| (ratio - 1.0).abs() > 0.005) {
        return Err(format!(
            "a cap's area is {:.5} and {:.5} of pi r squared",
            ratios[0], ratios[1]
        ));
    }
    let (low, high) = (z0.min(z1), z0.max(z1));
    for p in &all {
        let off = ((p[0] - c0[0]).hypot(p[1] - c0[1]) - r0).abs();
        if off > tolerance || p[2] < low - tolerance || p[2] > high + tolerance {
            return Err(format!(
                "a point is {off:.6} ft off the cylinder or beyond a cap"
            ));
        }
    }
    Ok(Round {
        radius: r0,
        height: high - low,
        cap_points: [first.ring.len(), second.ring.len()],
        cap_area_ratio: ratios,
    })
}

fn main() -> anyhow::Result<()> {
    let path = std::env::args().nth(1).expect("usage: MODEL.rvt");
    let mut rf = RevitFile::open(&path)?;
    let version = rf.basic_file_info()?.version;
    let mvp = recover_partition_schema_mvp(&mut rf, version, WalkerLimits::default())?;
    // Each column by id: its class, and whether RE-353 takes it as round.
    let mut columns: BTreeMap<u64, (String, bool)> = BTreeMap::new();
    for element in mvp.columns.iter().chain(mvp.products.iter()) {
        let Some(id) = element.id else { continue };
        if !COLUMN_CLASSES.contains(&element.class.as_str()) {
            continue;
        }
        let round = element.fields.iter().any(|(name, value)| {
            name == ROUND_DIAMETER_FIELD && matches!(value, InstanceField::Float { .. })
        });
        columns.insert(u64::from(id), (element.class.clone(), round));
    }
    let options = Options {
        selected_ids: columns.keys().copied().collect(),
        max_graph_values: 1_000_000,
        max_graph_objects: 1_000_000,
        ..Options::default()
    };
    let scene = if columns.is_empty() {
        None
    } else {
        match extract_at_detail(&mut rf, &options, 3) {
            Ok(scene) => Some(scene),
            Err(error) => {
                println!("saved graphics: {error}");
                None
            }
        }
    };
    let mut saved: BTreeMap<u64, Vec<&Primitive>> = BTreeMap::new();
    if let Some(scene) = &scene {
        for element in &scene.elements {
            saved.insert(
                element.id,
                element
                    .meshes
                    .primitives
                    .iter()
                    .filter(|p| !p.hidden && !p.filtered)
                    .collect(),
            );
        }
    }
    let mut tally: BTreeMap<&'static str, usize> = BTreeMap::new();
    for (id, (class, re353)) in &columns {
        let faces = saved.get(id).map(Vec::as_slice).unwrap_or_default();
        let (verdict, detail) = if faces.is_empty() {
            ("no saved graphics".to_string(), String::new())
        } else {
            match rvtio_rule(faces) {
                Ok(round) => (
                    "round".to_string(),
                    format!(
                        " r {:.5} height {:.5} cap points {}/{} cap area ratio {:.5}/{:.5}",
                        round.radius,
                        round.height,
                        round.cap_points[0],
                        round.cap_points[1],
                        round.cap_area_ratio[0],
                        round.cap_area_ratio[1]
                    ),
                ),
                Err(why) => (format!("no: {why}"), String::new()),
            }
        };
        let planar = faces.iter().filter(|f| is_planar(f, 1e-4)).count();
        println!(
            "column {id} {class} re353 {} rvtio {verdict}{detail} planar {planar} of {} faces",
            if *re353 { "round" } else { "no" },
            faces.len()
        );
        let both = match (*re353, verdict.as_str()) {
            (_, "no saved graphics") => "columns with no saved graphics",
            (true, "round") => "round by both",
            (true, _) => "round by RE-353 only",
            (false, "round") => "round by RvtIO's rule only",
            (false, _) => "round by neither",
        };
        *tally.entry(both).or_default() += 1;
    }
    for (what, count) in &tally {
        println!("tally {version}: {what} {count}");
    }
    println!("tally {version}: columns {}", columns.len());
    Ok(())
}
