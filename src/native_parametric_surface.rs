//! Typed evaluators for the non-planar surfaces observed in saved graphs.

use crate::native_parameters::ObjectGraph;
use anyhow::{Context, Result, bail, ensure};
use serde_json::Value;

pub type Point = [f64; 3];
fn p(v: &Value, n: &str) -> Result<Point> {
    let a = v
        .as_array()
        .with_context(|| format!("{n} must be an array"))?;
    ensure!(a.len() == 3, "{n} must have three coordinates");
    let r = [
        a[0].as_f64().context("coordinate")?,
        a[1].as_f64().context("coordinate")?,
        a[2].as_f64().context("coordinate")?,
    ];
    ensure!(
        r.iter().all(|x| x.is_finite()),
        "{n} has nonfinite coordinate"
    );
    Ok(r)
}
fn dot(a: Point, b: Point) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
fn add(a: Point, b: Point) -> Point {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}
fn sub(a: Point, b: Point) -> Point {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
fn mul(a: Point, s: f64) -> Point {
    [a[0] * s, a[1] * s, a[2] * s]
}
fn len(a: Point) -> f64 {
    dot(a, a).sqrt()
}
fn range(v: &Value, n: &str) -> Result<[f64; 2]> {
    let a = v
        .as_array()
        .with_context(|| format!("{n} must be an array"))?;
    ensure!(a.len() == 2, "{n} must have two values");
    let r = [
        a[0].as_f64().context("range")?,
        a[1].as_f64().context("range")?,
    ];
    ensure!(
        r.iter().all(|x| x.is_finite()) && r[1] > r[0],
        "invalid {n}"
    );
    Ok(r)
}
fn corner(v: &Value) -> Result<[f64; 2]> {
    let a = v.as_array().context("envelope corner")?;
    ensure!(a.len() == 2, "envelope corner must have two coordinates");
    let r = [
        a[0].as_f64().context("envelope coordinate")?,
        a[1].as_f64().context("envelope coordinate")?,
    ];
    ensure!(
        r.iter().all(|x| x.is_finite()),
        "nonfinite envelope coordinate"
    );
    Ok(r)
}

fn target(g: &ObjectGraph, source: usize, v: &Value) -> Result<usize> {
    crate::native_saved_mesh::pointer(g, source, v)?.context("missing surface pointer target")
}

fn basis(f: &Value) -> Result<(Point, Point, Point)> {
    let x = p(&f["m_xVec"], "surface x basis")?;
    let y = p(&f["m_yVec"], "surface y basis")?;
    let z = p(&f["m_zVec"], "surface z basis")?;
    ensure!(
        (len(x) - 1.).abs() <= 1e-6 && (len(y) - 1.).abs() <= 1e-6 && (len(z) - 1.).abs() <= 1e-6,
        "surface basis is not unit"
    );
    ensure!(
        dot(x, y).abs() <= 1e-6 && dot(x, z).abs() <= 1e-6 && dot(y, z).abs() <= 1e-6,
        "surface basis is not orthogonal"
    );
    Ok((x, y, z))
}

pub fn surface_of_revolution_point(g: &ObjectGraph, si: usize, u: f64, v: f64) -> Result<Point> {
    let surface = ParametricSurface::read(g, si)?;
    ensure!(
        matches!(surface, ParametricSurface::SurfRev { .. }),
        "not SurfRev"
    );
    surface.evaluate(u, v)
}

pub fn ruled_surface_point(g: &ObjectGraph, si: usize, u: f64, v: f64) -> Result<Point> {
    let surface = ParametricSurface::read(g, si)?;
    ensure!(
        matches!(surface, ParametricSurface::RuledSurf { .. }),
        "not RuledSurf"
    );
    surface.evaluate(u, v)
}

#[derive(Debug, Clone)]
pub enum Curve {
    Line {
        origin: Point,
        direction: Point,
        range: [f64; 2],
    },
    Arc {
        center: Point,
        x: Point,
        y: Point,
        radius: f64,
        range: [f64; 2],
    },
}

impl Curve {
    pub fn line(origin: Point, direction: Point, range: [f64; 2]) -> Self {
        Self::Line {
            origin,
            direction,
            range,
        }
    }
    fn read(g: &ObjectGraph, i: usize) -> Result<Self> {
        let o = g.objects.get(i).context("profile object")?;
        match o.class_name.as_str() {
            "GLine" => {
                let direction = p(&o.fields["m_dirVec"], "line direction")?;
                ensure!(len(direction) > 1e-12, "singular line direction");
                Ok(Self::Line {
                    origin: p(&o.fields["m_origin"], "line origin")?,
                    direction,
                    range: range(&o.fields["m_endParams"], "line range")?,
                })
            }
            "GArc" => {
                let x = p(&o.fields["m_xVec"], "arc x basis")?;
                let y = p(&o.fields["m_yVec"], "arc y basis")?;
                let radius = o.fields["m_radius"].as_f64().context("arc radius")?;
                ensure!(
                    radius.is_finite()
                        && radius > 0.
                        && (len(x) - 1.).abs() <= 1e-6
                        && (len(y) - 1.).abs() <= 1e-6
                        && dot(x, y).abs() <= 1e-6,
                    "invalid arc basis"
                );
                Ok(Self::Arc {
                    center: p(&o.fields["m_center"], "arc center")?,
                    x,
                    y,
                    radius,
                    range: range(&o.fields["m_endParams"], "arc range")?,
                })
            }
            name => bail!("unsupported parametric profile curve {name}"),
        }
    }
    fn eval(&self, t: f64) -> Point {
        match self {
            Self::Line {
                origin, direction, ..
            } => add(*origin, mul(*direction, t)),
            Self::Arc {
                center,
                x,
                y,
                radius,
                ..
            } => add(
                *center,
                mul(add(mul(*x, t.cos()), mul(*y, t.sin())), *radius),
            ),
        }
    }
    fn derivative(&self, t: f64) -> Point {
        match self {
            Self::Line { direction, .. } => *direction,
            Self::Arc { x, y, radius, .. } => {
                mul(add(mul(*x, -t.sin()), mul(*y, t.cos())), *radius)
            }
        }
    }
}

#[derive(Debug, Clone)]
pub enum ParametricSurface {
    SurfRev {
        center: Point,
        x: Point,
        y: Point,
        z: Point,
        envelope: [[f64; 2]; 2],
        orient: bool,
        profile: Curve,
    },
    RuledSurf {
        envelope: [[f64; 2]; 2],
        orient: bool,
        profile1: Curve,
        profile2: Curve,
    },
}

impl ParametricSurface {
    pub fn read(g: &ObjectGraph, si: usize) -> Result<Self> {
        let s = g.objects.get(si).context("surface index")?;
        let corners = s.fields["m_Envelope"]["m_corners"]
            .as_array()
            .context("surface envelope")?;
        ensure!(corners.len() == 2, "surface envelope corners");
        let a = corner(&corners[0])?;
        let b = corner(&corners[1])?;
        ensure!(
            b[0] > a[0] && b[1] > a[1],
            "surface envelope is not ordered"
        );
        let envelope = [a, b];
        match s.class_name.as_str() {
            "SurfRev" => {
                let profile = Curve::read(g, target(g, si, &s.fields["m_pProfileCurve"])?)?;
                let (x, y, z) = basis(&s.fields)?;
                Ok(Self::SurfRev {
                    center: p(&s.fields["m_center"], "surface center")?,
                    x,
                    y,
                    z,
                    envelope,
                    orient: s.fields["m_orientFlag"]
                        .as_bool()
                        .context("surface orientation")?,
                    profile,
                })
            }
            "RuledSurf" => Ok(Self::RuledSurf {
                envelope,
                orient: s.fields["m_orientFlag"]
                    .as_bool()
                    .context("surface orientation")?,
                profile1: Curve::read(g, target(g, si, &s.fields["m_pProfileCurve1"])?)?,
                profile2: Curve::read(g, target(g, si, &s.fields["m_pProfileCurve2"])?)?,
            }),
            name => bail!("unsupported parametric surface {name}"),
        }
    }
    pub fn bounds(&self) -> [[f64; 2]; 2] {
        match self {
            Self::SurfRev { envelope, .. } | Self::RuledSurf { envelope, .. } => *envelope,
        }
    }
    pub fn orientation(&self) -> bool {
        match self {
            Self::SurfRev { orient, .. } | Self::RuledSurf { orient, .. } => *orient,
        }
    }
    pub fn evaluate(&self, u: f64, v: f64) -> Result<Point> {
        let e = self.bounds();
        ensure!(
            u >= e[0][0] - 1e-9
                && u <= e[1][0] + 1e-9
                && v >= e[0][1] - 1e-9
                && v <= e[1][1] + 1e-9,
            "surface coordinates outside envelope"
        );
        Ok(match self {
            Self::SurfRev {
                center,
                x,
                y,
                z,
                profile,
                ..
            } => {
                let q = profile.eval(v);
                let local = [
                    q[0] * u.cos() - q[1] * u.sin(),
                    q[0] * u.sin() + q[1] * u.cos(),
                    q[2],
                ];
                add(
                    *center,
                    add(add(mul(*x, local[0]), mul(*y, local[1])), mul(*z, local[2])),
                )
            }
            Self::RuledSurf {
                envelope: _,
                profile1,
                profile2,
                ..
            } => {
                let t = |c: &Curve| c_range(c)[0] + u * (c_range(c)[1] - c_range(c)[0]);
                add(
                    mul(profile1.eval(t(profile1)), 1. - v),
                    mul(profile2.eval(t(profile2)), v),
                )
            }
        })
    }
    pub fn normal(&self, u: f64, v: f64) -> Result<Point> {
        let e = self.bounds();
        ensure!(
            u.is_finite()
                && v.is_finite()
                && u >= e[0][0] - 1e-9
                && u <= e[1][0] + 1e-9
                && v >= e[0][1] - 1e-9
                && v <= e[1][1] + 1e-9,
            "coordinates outside envelope"
        );
        if let Self::SurfRev {
            x,
            y,
            z,
            profile,
            orient,
            ..
        } = self
        {
            let q = profile.eval(v);
            let dq = profile.derivative(v);
            let cu = u.cos();
            let su = u.sin();
            let ru = [-q[0] * su - q[1] * cu, q[0] * cu - q[1] * su, 0.];
            let rv = [dq[0] * cu - dq[1] * su, dq[0] * su + dq[1] * cu, dq[2]];
            let transform = |q: Point| add(add(mul(*x, q[0]), mul(*y, q[1])), mul(*z, q[2]));
            let tu = transform(ru);
            let tv = transform(rv);
            let mut n = cross(tu, tv);
            ensure!(
                n.iter().all(|value| value.is_finite()),
                "nonfinite surface normal"
            );
            if len(n) <= 1e-12 {
                // At a revolution pole, the U derivative collapses.  Use the
                // limiting radial/axial profile tangent instead.
                ensure!(
                    q[0].abs() <= 1e-9 && q[1].abs() <= 1e-9 && dq[1].abs() <= 1e-9,
                    "unsupported nonmeridional parametric singularity"
                );
                // Serialized meridian profiles have their radial tangent in
                // local X.  At the rotation axis the limiting normal is
                // continuous in U; projecting dq onto the current radial
                // direction would spuriously change the pole sign.
                let pole_du = transform([-su, cu, 0.]);
                let pole_dv = transform([dq[0] * cu, dq[0] * su, dq[2]]);
                n = cross(pole_du, pole_dv);
                ensure!(len(n) > 1e-12, "surface normal is singular");
            }
            return Ok(mul(n, if *orient { 1. / len(n) } else { -1. / len(n) }));
        }
        let Self::RuledSurf {
            profile1,
            profile2,
            orient,
            ..
        } = self
        else {
            unreachable!()
        };
        let r1 = c_range(profile1);
        let r2 = c_range(profile2);
        let t1 = r1[0] + u * (r1[1] - r1[0]);
        let t2 = r2[0] + u * (r2[1] - r2[0]);
        let p1 = profile1.eval(t1);
        let p2 = profile2.eval(t2);
        let du = add(
            mul(profile1.derivative(t1), (1. - v) * (r1[1] - r1[0])),
            mul(profile2.derivative(t2), v * (r2[1] - r2[0])),
        );
        let dv = sub(p2, p1);
        let n = cross(du, dv);
        ensure!(
            n.iter().all(|value| value.is_finite()),
            "nonfinite surface normal"
        );
        let l = len(n);
        ensure!(l > 1e-12, "surface normal is singular");
        Ok(mul(n, if *orient { 1. / l } else { -1. / l }))
    }
}

fn cross(a: Point, b: Point) -> Point {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn c_range(c: &Curve) -> [f64; 2] {
    match c {
        Curve::Line { range, .. } | Curve::Arc { range, .. } => *range,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::native_parameters::{GraphEdge, GraphObject};
    fn o(name: &str, tag: u16, token: u32, fields: Value) -> GraphObject {
        GraphObject {
            class_tag: tag,
            class_name: name.into(),
            token,
            start: 0,
            fields_end: 0,
            fields,
        }
    }
    fn e(source: usize, offset: usize, token: u32, target: usize, tag: u16) -> GraphEdge {
        GraphEdge {
            source_object_index: source,
            pointer_offset: offset,
            pointer_token: token,
            target_object_index: target,
            target_class_tag: tag,
        }
    }
    fn g(objects: Vec<GraphObject>, edges: Vec<GraphEdge>) -> ObjectGraph {
        ObjectGraph {
            consumed_bytes: 1,
            objects,
            edges,
        }
    }
    #[test]
    fn surface_revolution_uses_edge_target_not_token_index() {
        let s = o(
            "SurfRev",
            4283,
            u32::MAX,
            serde_json::json!({"m_Envelope":{"m_corners":[[0.,0.],[1.,1.]]},"m_center":[0.,0.,0.],"m_xVec":[1.,0.,0.],"m_yVec":[0.,1.,0.],"m_zVec":[0.,0.,1.],"m_orientFlag":true,"m_pProfileCurve":{"class_tag":1973,"offset":8,"pointer_token":55}}),
        );
        let filler = o("Plane", 634, 55, serde_json::json!({}));
        let line = o(
            "GLine",
            1973,
            77,
            serde_json::json!({"m_origin":[2.,0.,0.],"m_dirVec":[0.,0.,1.],"m_endParams":[0.,1.]}),
        );
        let mut graph = g(vec![s, filler, line], vec![e(0, 8, 55, 2, 1973)]);
        assert_eq!(
            surface_of_revolution_point(&graph, 0, 0.0, 0.5).unwrap(),
            [2.0, 0.0, 0.5]
        );
        let surface = ParametricSurface::read(&graph, 0).unwrap();
        assert_eq!(surface.normal(0.0, 0.5).unwrap(), [1.0, 0.0, 0.0]);
        graph.objects[0].fields["m_orientFlag"] = Value::Bool(false);
        let reversed = ParametricSurface::read(&graph, 0).unwrap();
        assert_eq!(reversed.normal(0.0, 0.5).unwrap(), [-1.0, 0.0, 0.0]);
    }
    #[test]
    fn ruled_surface_blends_profiles() {
        let s = o(
            "RuledSurf",
            3859,
            u32::MAX,
            serde_json::json!({"m_Envelope":{"m_corners":[[0.,0.],[1.,1.]]},"m_orientFlag":true,"m_pProfileCurve1":{"class_tag":1973,"offset":8,"pointer_token":53},"m_pProfileCurve2":{"class_tag":1973,"offset":12,"pointer_token":54}}),
        );
        let a = o(
            "GLine",
            1973,
            1,
            serde_json::json!({"m_origin":[0.,0.,0.],"m_dirVec":[1.,0.,0.],"m_endParams":[0.,1.]}),
        );
        let b = o(
            "GLine",
            1973,
            2,
            serde_json::json!({"m_origin":[0.,0.,1.],"m_dirVec":[0.,1.,0.],"m_endParams":[0.,2.]}),
        );
        let graph = g(
            vec![s, a, b],
            vec![e(0, 8, 53, 1, 1973), e(0, 12, 54, 2, 1973)],
        );
        assert_eq!(
            ruled_surface_point(&graph, 0, 0.5, 0.5).unwrap(),
            [0.25, 0.5, 0.5]
        );
    }

    #[test]
    fn ruled_surface_false_orientation_reverses_normal() {
        let s = o(
            "RuledSurf",
            3859,
            u32::MAX,
            serde_json::json!({"m_Envelope":{"m_corners":[[0.,0.],[1.,1.]]},"m_orientFlag":true,"m_pProfileCurve1":{"class_tag":1973,"offset":8,"pointer_token":53},"m_pProfileCurve2":{"class_tag":1973,"offset":12,"pointer_token":54}}),
        );
        let a = o(
            "GLine",
            1973,
            1,
            serde_json::json!({"m_origin":[0.,0.,0.],"m_dirVec":[1.,0.,0.],"m_endParams":[0.,1.]}),
        );
        let b = o(
            "GLine",
            1973,
            2,
            serde_json::json!({"m_origin":[0.,0.,1.],"m_dirVec":[0.,1.,0.],"m_endParams":[0.,2.]}),
        );
        let mut graph = g(
            vec![s, a, b],
            vec![e(0, 8, 53, 1, 1973), e(0, 12, 54, 2, 1973)],
        );
        let positive = ParametricSurface::read(&graph, 0)
            .unwrap()
            .normal(0.5, 0.5)
            .unwrap();
        graph.objects[0].fields["m_orientFlag"] = Value::Bool(false);
        let negative = ParametricSurface::read(&graph, 0)
            .unwrap()
            .normal(0.5, 0.5)
            .unwrap();
        assert_eq!(negative, [-positive[0], -positive[1], -positive[2]]);
    }

    #[test]
    fn revolution_uses_local_profile_and_rotated_basis() {
        let s = o(
            "SurfRev",
            4283,
            u32::MAX,
            serde_json::json!({"m_Envelope":{"m_corners":[[0.,0.],[1.,1.]]},"m_center":[10.,20.,30.],"m_xVec":[0.,1.,0.],"m_yVec":[-1.,0.,0.],"m_zVec":[0.,0.,1.],"m_orientFlag":true,"m_pProfileCurve":{"class_tag":1973,"offset":8,"pointer_token":55}}),
        );
        let line = o(
            "GLine",
            1973,
            77,
            serde_json::json!({"m_origin":[2.,0.,0.],"m_dirVec":[0.,0.,1.],"m_endParams":[0.,1.]}),
        );
        let graph = g(vec![s, line], vec![e(0, 8, 55, 1, 1973)]);
        assert_eq!(
            surface_of_revolution_point(&graph, 0, 0.0, 0.5).unwrap(),
            [10., 22., 30.5]
        );
    }

    #[test]
    fn sphere_pole_normal_has_one_continuous_limit() {
        let surface = ParametricSurface::SurfRev {
            center: [0., 0., 0.],
            x: [1., 0., 0.],
            y: [0., 1., 0.],
            z: [0., 0., 1.],
            envelope: [
                [0., -std::f64::consts::FRAC_PI_2],
                [std::f64::consts::TAU, std::f64::consts::FRAC_PI_2],
            ],
            orient: true,
            profile: Curve::Arc {
                center: [0., 0., 0.],
                x: [1., 0., 0.],
                y: [0., 0., 1.],
                radius: 1.,
                range: [-std::f64::consts::FRAC_PI_2, std::f64::consts::FRAC_PI_2],
            },
        };
        for u in [
            0.,
            std::f64::consts::FRAC_PI_2,
            std::f64::consts::PI,
            3. * std::f64::consts::FRAC_PI_2,
        ] {
            let north = surface.normal(u, std::f64::consts::FRAC_PI_2).unwrap();
            let south = surface.normal(u, -std::f64::consts::FRAC_PI_2).unwrap();
            assert!((north[2] - 1.).abs() < 1e-12, "u={u}: {north:?}");
            assert!((south[2] + 1.).abs() < 1e-12, "u={u}: {south:?}");
        }
    }

    #[test]
    fn ruled_normal_blends_profile_derivatives_by_v() {
        let surface = ParametricSurface::RuledSurf {
            envelope: [[0., 0.], [1., 1.]],
            orient: true,
            profile1: Curve::line([0., 0., 0.], [1., 0., 0.], [0., 2.]),
            profile2: Curve::line([0., 1., 1.], [0., 1., 0.], [0., 3.]),
        };
        let n = surface.normal(0.5, 0.5).unwrap();
        let norm = 19.25_f64.sqrt();
        let expected = [1.5 / norm, -1. / norm, 4. / norm];
        assert!((0..3).all(|i| (n[i] - expected[i]).abs() < 1e-3), "{n:?}");
        assert!(
            surface
                .normal(0.5, 0.)
                .unwrap()
                .iter()
                .all(|x| x.is_finite())
        );
        assert!(
            surface
                .normal(0.5, 1.)
                .unwrap()
                .iter()
                .all(|x| x.is_finite())
        );
    }

    #[test]
    fn nonmeridional_pole_singularity_is_rejected() {
        let surface = ParametricSurface::SurfRev {
            center: [0., 0., 0.],
            x: [1., 0., 0.],
            y: [0., 1., 0.],
            z: [0., 0., 1.],
            envelope: [[0., 0.], [std::f64::consts::TAU, 1.]],
            orient: true,
            profile: Curve::line([0., 0., 0.], [0., 1., 0.], [0., 1.]),
        };
        let error = surface.normal(0.5, 0.).unwrap_err().to_string();
        assert!(error.contains("nonmeridional"), "{error}");
    }

    #[test]
    fn left_handed_sphere_pole_matches_near_pole_normal() {
        let surface = ParametricSurface::SurfRev {
            center: [0., 0., 0.],
            x: [1., 0., 0.],
            y: [0., -1., 0.],
            z: [0., 0., 1.],
            envelope: [
                [0., -std::f64::consts::FRAC_PI_2],
                [std::f64::consts::TAU, std::f64::consts::FRAC_PI_2],
            ],
            orient: true,
            profile: Curve::Arc {
                center: [0., 0., 0.],
                x: [1., 0., 0.],
                y: [0., 0., 1.],
                radius: 1.,
                range: [-std::f64::consts::FRAC_PI_2, std::f64::consts::FRAC_PI_2],
            },
        };
        let pole = surface.normal(0.7, std::f64::consts::FRAC_PI_2).unwrap();
        let near = surface
            .normal(0.7, std::f64::consts::FRAC_PI_2 - 1e-6)
            .unwrap();
        assert!(pole.iter().zip(near).map(|(a, b)| a * b).sum::<f64>() > 1. - 1e-8);
    }

    #[test]
    fn revolution_rotates_both_local_profile_coordinates() {
        let surface = ParametricSurface::SurfRev {
            center: [0., 0., 0.],
            x: [1., 0., 0.],
            y: [0., 1., 0.],
            z: [0., 0., 1.],
            envelope: [[0., 0.], [std::f64::consts::TAU, 1.]],
            orient: true,
            profile: Curve::line([1., 2., 0.], [0., 0., 1.], [0., 1.]),
        };
        let u = 0.5;
        let point = surface.evaluate(u, 0.25).unwrap();
        let expected = [u.cos() - 2. * u.sin(), u.sin() + 2. * u.cos(), 0.25];
        assert!((0..3).all(|i| (point[i] - expected[i]).abs() < 1e-12));
        assert!(
            surface
                .normal(u, 0.25)
                .unwrap()
                .iter()
                .all(|x| x.is_finite())
        );
    }
}
