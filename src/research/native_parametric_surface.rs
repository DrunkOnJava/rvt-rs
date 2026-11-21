//! Typed evaluators for the non-planar surfaces observed in saved graphs.

use crate::graph_value::Fv;
use crate::native_parameters::ObjectGraph;
use crate::vector_math::{cross, dot, sub};
use anyhow::{Context, Result, bail, ensure};

pub type Point = [f64; 3];
fn p(v: &Fv, n: &str) -> Result<Point> {
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
fn add(a: Point, b: Point) -> Point {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}
fn mul(a: Point, s: f64) -> Point {
    [a[0] * s, a[1] * s, a[2] * s]
}
fn len(a: Point) -> f64 {
    dot(a, a).sqrt()
}
fn range(v: &Fv, n: &str) -> Result<[f64; 2]> {
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
fn corner(v: &Fv) -> Result<[f64; 2]> {
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

fn target(g: &ObjectGraph, source: usize, v: &Fv) -> Result<usize> {
    crate::native_saved_mesh::pointer(g, source, v)?.context("missing surface pointer target")
}

fn basis(f: &Fv) -> Result<(Point, Point, Point)> {
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

/// One node of a saved `GHermiteSpline` profile (RE-311): the curve passes
/// through `point` at parameter `t` with derivative `tangent` per unit of
/// parameter.
#[derive(Debug, Clone, Copy)]
pub struct SplineNode {
    t: f64,
    point: Point,
    tangent: Point,
}

/// The most equal steps a spline profile is cut into to hold a chord error.
const SPLINE_STEP_CAP: usize = 128;

/// The two nodes of `nodes` the parameter `t` lies between, the last pair
/// from the last node on. `nodes` holds at least two, in rising order of their
/// parameter, so the pair is found by halving: a face's grid evaluates its
/// profile at every one of its points, and a scan of the nodes at each would
/// cost the points times the nodes the file saves.
fn spline_segment(nodes: &[SplineNode], t: f64) -> (SplineNode, SplineNode) {
    let i = nodes
        .partition_point(|node| node.t <= t)
        .saturating_sub(1)
        .min(nodes.len() - 2);
    (nodes[i], nodes[i + 1])
}

/// The point of a saved `GNurbSpline` (RE-311) at `t` in homogeneous
/// coordinates, by de Boor's algorithm. `control` holds more than `degree`
/// points and `knots` their count plus `degree + 1`, never falling (a
/// clamped spline repeats its end knots), as [`Curve::read`] checks. The knot span is found by halving, for the reason
/// [`spline_segment`] gives.
fn nurbs_homogeneous(degree: usize, knots: &[f64], control: &[[f64; 4]], t: f64) -> [f64; 4] {
    let n = control.len();
    // The last knot at or before `t`, kept within the curve's domain
    // `knots[degree]..knots[n]`.
    let span = knots[..n]
        .partition_point(|knot| *knot <= t)
        .saturating_sub(1)
        .clamp(degree, n - 1);
    let mut d: Vec<[f64; 4]> = (0..=degree).map(|j| control[span - degree + j]).collect();
    for r in 1..=degree {
        for j in (r..=degree).rev() {
            let i = span - degree + j;
            let denominator = knots[i + degree - r + 1] - knots[i];
            // A repeated knot leaves a span of no length, which takes no share.
            let a = if denominator.abs() < 1e-300 {
                0.
            } else {
                (t - knots[i]) / denominator
            };
            let previous = d[j - 1];
            for (value, before) in d[j].iter_mut().zip(previous) {
                *value = (1. - a) * before + a * *value;
            }
        }
    }
    d[degree]
}

#[derive(Debug, Clone)]
pub enum Curve {
    /// RE-443: a null ruled-profile pointer's separately saved point.
    Point(Point),
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
    /// `center + x_len cos t x + y_len sin t y`: Revit's `GEllipse`, seen as
    /// the meridian of a revolved dome (a pendant light's shade).
    Ellipse {
        center: Point,
        x: Point,
        y: Point,
        x_len: f64,
        y_len: f64,
        range: [f64; 2],
    },
    /// Revit's `GHermiteSpline` (RE-311): a cubic Hermite segment between each
    /// pair of consecutive nodes, with at least two nodes in rising order of
    /// their parameter.
    Hermite {
        nodes: Vec<SplineNode>,
        range: [f64; 2],
    },
    /// Revit's `GNurbSpline` (RE-311): a rational B-spline of degree 1 to 5,
    /// its control points held as `(w x, w y, w z, w)`.
    Nurbs {
        degree: usize,
        knots: Vec<f64>,
        control: Vec<[f64; 4]>,
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
            "GEllipse" => {
                let x = p(&o.fields["m_xVec"], "ellipse x basis")?;
                let y = p(&o.fields["m_yVec"], "ellipse y basis")?;
                let x_len = o.fields["m_xLen"].as_f64().context("ellipse x length")?;
                let y_len = o.fields["m_yLen"].as_f64().context("ellipse y length")?;
                ensure!(
                    x_len.is_finite()
                        && y_len.is_finite()
                        && x_len > 0.
                        && y_len > 0.
                        && (len(x) - 1.).abs() <= 1e-6
                        && (len(y) - 1.).abs() <= 1e-6
                        && dot(x, y).abs() <= 1e-6,
                    "invalid ellipse basis"
                );
                Ok(Self::Ellipse {
                    center: p(&o.fields["m_center"], "ellipse center")?,
                    x,
                    y,
                    x_len,
                    y_len,
                    range: range(&o.fields["m_endParams"], "ellipse range")?,
                })
            }
            "GHermiteSpline" => {
                // A periodic spline is unmeasured, so it is refused.
                ensure!(
                    o.fields["m_Periodic"].as_bool() == Some(false),
                    "periodic spline profile is unmeasured"
                );
                let nodes = o.fields["m_NodeArray"]
                    .as_array()
                    .context("spline nodes")?
                    .iter()
                    .map(|node| -> Result<SplineNode> {
                        Ok(SplineNode {
                            t: node["m_iParametr"]
                                .as_f64()
                                .context("spline node parameter")?,
                            point: p(&node["m_iPoint"], "spline node point")?,
                            tangent: p(&node["m_iTangent"], "spline node tangent")?,
                        })
                    })
                    .collect::<Result<Vec<_>>>()?;
                ensure!(
                    nodes.len() >= 2
                        && nodes.iter().all(|node| node.t.is_finite())
                        && nodes.windows(2).all(|pair| pair[1].t > pair[0].t),
                    "spline nodes are not in rising order"
                );
                let range = range(&o.fields["m_endParams"], "spline range")?;
                ensure!(
                    range[0] >= nodes[0].t - 1e-9 && range[1] <= nodes[nodes.len() - 1].t + 1e-9,
                    "spline range outside its nodes"
                );
                Ok(Self::Hermite { nodes, range })
            }
            "GNurbSpline" => {
                let degree = o.fields["m_Degree"].as_u64().context("spline degree")?;
                ensure!((1..=5).contains(&degree), "unsupported spline degree");
                let degree = degree as usize;
                // A closed spline is unmeasured, so it is refused.
                ensure!(
                    o.fields["m_Closed"].as_bool() == Some(false),
                    "closed spline profile is unmeasured"
                );
                let knots = o.fields["m_Knots"]
                    .as_array()
                    .context("spline knots")?
                    .iter()
                    .map(|k| k.as_f64().filter(|k| k.is_finite()).context("spline knot"))
                    .collect::<Result<Vec<_>>>()?;
                let points = o.fields["m_ctrlPoints"]
                    .as_array()
                    .context("spline control points")?;
                let mut control = Vec::with_capacity(points.len());
                for c in points {
                    let q = p(&c["m_Point"], "spline control point")?;
                    let w = c["m_w"].as_f64().context("spline weight")?;
                    ensure!(w.is_finite() && w > 0., "invalid spline weight");
                    control.push([q[0] * w, q[1] * w, q[2] * w, w]);
                }
                ensure!(
                    control.len() > degree && knots.len() == control.len() + degree + 1,
                    "spline knot count"
                );
                ensure!(
                    knots.windows(2).all(|w| w[1] >= w[0]),
                    "spline knots decrease"
                );
                let range = range(&o.fields["m_endParams"], "spline range")?;
                ensure!(
                    range[0] >= knots[degree] - 1e-9 && range[1] <= knots[control.len()] + 1e-9,
                    "spline range outside its knots"
                );
                Ok(Self::Nurbs {
                    degree,
                    knots,
                    control,
                    range,
                })
            }
            name => bail!("unsupported parametric profile curve {name}"),
        }
    }
    /// The point at `t`.
    pub(crate) fn eval(&self, t: f64) -> Point {
        match self {
            Self::Point(point) => *point,
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
            Self::Ellipse {
                center,
                x,
                y,
                x_len,
                y_len,
                ..
            } => add(
                *center,
                add(mul(*x, x_len * t.cos()), mul(*y, y_len * t.sin())),
            ),
            Self::Hermite { nodes, range } => {
                let t = t.clamp(range[0], range[1]);
                let (a, b) = spline_segment(nodes, t);
                let d = b.t - a.t;
                let s = (t - a.t) / d;
                let (s2, s3) = (s * s, s * s * s);
                let h00 = 2. * s3 - 3. * s2 + 1.;
                let h10 = s3 - 2. * s2 + s;
                let h01 = -2. * s3 + 3. * s2;
                let h11 = s3 - s2;
                std::array::from_fn(|k| {
                    h00 * a.point[k]
                        + h10 * d * a.tangent[k]
                        + h01 * b.point[k]
                        + h11 * d * b.tangent[k]
                })
            }
            Self::Nurbs {
                degree,
                knots,
                control,
                range,
            } => {
                let t = t.clamp(range[0], range[1]);
                // The weight is a sum of positive weights with shares that
                // sum to one, so it is never zero.
                let h = nurbs_homogeneous(*degree, knots, control, t);
                [h[0] / h[3], h[1] / h[3], h[2] / h[3]]
            }
        }
    }
    /// The parameter range.
    pub fn range(&self) -> [f64; 2] {
        match self {
            Self::Point(_) => [0., 1.],
            Self::Line { range, .. }
            | Self::Arc { range, .. }
            | Self::Ellipse { range, .. }
            | Self::Hermite { range, .. }
            | Self::Nurbs { range, .. } => *range,
        }
    }
    fn derivative(&self, t: f64) -> Point {
        match self {
            Self::Point(_) => [0.; 3],
            Self::Line { direction, .. } => *direction,
            Self::Arc { x, y, radius, .. } => {
                mul(add(mul(*x, -t.sin()), mul(*y, t.cos())), *radius)
            }
            Self::Ellipse {
                x, y, x_len, y_len, ..
            } => add(mul(*x, -x_len * t.sin()), mul(*y, y_len * t.cos())),
            Self::Hermite { nodes, range } => {
                let t = t.clamp(range[0], range[1]);
                let (a, b) = spline_segment(nodes, t);
                let d = b.t - a.t;
                let s = (t - a.t) / d;
                let s2 = s * s;
                let h00 = 6. * s2 - 6. * s;
                let h10 = 3. * s2 - 4. * s + 1.;
                let h01 = -6. * s2 + 6. * s;
                let h11 = 3. * s2 - 2. * s;
                std::array::from_fn(|k| {
                    (h00 * a.point[k] + h01 * b.point[k]) / d
                        + h10 * a.tangent[k]
                        + h11 * b.tangent[k]
                })
            }
            // A central difference over a millionth of the range, kept inside it.
            Self::Nurbs { range, .. } => {
                let [lo, hi] = *range;
                let h = (hi - lo) * 1e-6;
                let (a, b) = ((t - h).max(lo), (t + h).min(hi));
                mul(sub(self.eval(b), self.eval(a)), 1. / (b - a).max(1e-300))
            }
        }
    }
    /// How many equal parameter steps between `lo` and `hi` hold a spline's
    /// chord error to `chord` feet (one for any other curve): by the largest
    /// second difference of 64 samples, at most 128 (`SPLINE_STEP_CAP`).
    pub fn spline_steps(&self, lo: f64, hi: f64, chord: f64) -> usize {
        if !matches!(self, Self::Hermite { .. } | Self::Nurbs { .. }) {
            return 1;
        }
        const SAMPLES: usize = 64;
        let h = (hi - lo) / SAMPLES as f64;
        let mut second = 0.0_f64;
        for i in 1..SAMPLES {
            let t = lo + h * i as f64;
            let (a, b, c) = (self.eval(t - h), self.eval(t), self.eval(t + h));
            second = second.max(len(sub(add(a, c), mul(b, 2.))));
        }
        if !second.is_finite() {
            return SPLINE_STEP_CAP;
        }
        let steps = (SAMPLES as f64 * (second / (8. * chord)).sqrt()).ceil();
        (steps as usize).clamp(1, SPLINE_STEP_CAP)
    }
    /// The greatest length of the derivative at 65 equal steps from `lo` to
    /// `hi`.
    pub fn speed(&self, lo: f64, hi: f64) -> f64 {
        (0..=64)
            .map(|i| len(self.derivative(lo + (hi - lo) * f64::from(i) / 64.)))
            .fold(0., f64::max)
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
    /// A cone `apex + v (cos α z + sin α (cos u x + sin u y))`: `v` is the
    /// distance from the apex along the generator, and the radius at `v` is
    /// `v sin α` (RE-311).
    ConeSurf {
        apex: Point,
        x: Point,
        y: Point,
        z: Point,
        half_angle: f64,
        envelope: [[f64; 2]; 2],
        orient: bool,
    },
    /// A bicubic Hermite spline surface (RE-460, `HermiteSurf`): `u` and `v` are
    /// the parameters of its grid and `nodes` holds one node at each crossing, the
    /// node of `(iu, iv)` at `iv * u.len() + iu`. Each cell is the bicubic Hermite
    /// patch of its four nodes, and a parameter past the grid, which the envelope
    /// bounds, takes the nearest cell's patch continued.
    HermiteSurf {
        envelope: [[f64; 2]; 2],
        orient: bool,
        u: Vec<f64>,
        v: Vec<f64>,
        nodes: Vec<HermiteNode>,
    },
}

/// One node of a `HermiteSurf` (RE-460): the surface's point there, its tangents
/// along `u` and along `v` in feet per unit of the parameter, and its mixed
/// derivative.
#[derive(Debug, Clone, Copy)]
pub struct HermiteNode {
    pub point: Point,
    pub tangent: [Point; 2],
    pub mixed: Point,
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
            "RuledSurf" => {
                let profile = |name: &str, point_name: &str| -> Result<Curve> {
                    match crate::native_saved_mesh::pointer(g, si, &s.fields[name])? {
                        Some(index) => Curve::read(g, index),
                        None => Ok(Curve::Point(p(&s.fields[point_name], point_name)?)),
                    }
                };
                let profile1 = profile("m_pProfileCurve1", "m_Point1")?;
                let profile2 = profile("m_pProfileCurve2", "m_Point2")?;
                if matches!(profile1, Curve::Point(_)) || matches!(profile2, Curve::Point(_)) {
                    ensure!(
                        matches!(
                            (&profile1, &profile2),
                            (Curve::Point(_), Curve::Arc { .. })
                                | (Curve::Arc { .. }, Curve::Point(_))
                        ) && envelope == [[0., 0.], [1., 1.]],
                        "unqualified collapsed ruled profiles"
                    );
                }
                Ok(Self::RuledSurf {
                    envelope,
                    orient: s.fields["m_orientFlag"]
                        .as_bool()
                        .context("surface orientation")?,
                    profile1,
                    profile2,
                })
            }
            "ConeSurf" => {
                let (x, y, z) = basis(&s.fields)?;
                let half_angle = s.fields["m_halfAngle"]
                    .as_f64()
                    .context("cone half angle")?;
                ensure!(
                    half_angle.is_finite()
                        && half_angle > 1e-6
                        && half_angle < std::f64::consts::FRAC_PI_2 - 1e-6,
                    "invalid cone half angle"
                );
                // Only `v` above the apex is measured (0.81 to 0.93, RE-311):
                // a face across the apex or on the far nappe is refused.
                ensure!(envelope[0][1] > 0., "cone face not above its apex");
                Ok(Self::ConeSurf {
                    apex: p(&s.fields["m_center"], "cone apex")?,
                    x,
                    y,
                    z,
                    half_angle,
                    envelope,
                    orient: s.fields["m_orientFlag"]
                        .as_bool()
                        .context("surface orientation")?,
                })
            }
            "HermiteSurf" => {
                let params = |name: &str| -> Result<Vec<f64>> {
                    let items = s.fields[name]
                        .as_array()
                        .with_context(|| format!("{name} must be an array"))?;
                    let values = items
                        .iter()
                        .map(|x| x.as_f64().context("hermite parameter"))
                        .collect::<Result<Vec<_>>>()?;
                    ensure!(
                        values.len() >= 2
                            && values.iter().all(|x| x.is_finite())
                            && values.windows(2).all(|w| w[0] < w[1]),
                        "invalid {name}"
                    );
                    Ok(values)
                };
                let (u, v) = (params("m_uParams")?, params("m_vParams")?);
                // Neither direction is periodic on any of the 479 objects measured
                // (RE-460); a periodic one is unmeasured, so it is refused.
                let periodic = s.fields["m_periodic"]
                    .as_array()
                    .context("hermite periodic flags")?;
                ensure!(
                    periodic.len() == 2
                        && periodic.iter().all(|flag| flag.as_bool() == Some(false)),
                    "periodic HermiteSurf is unmeasured"
                );
                let list = s.fields["m_NodeArray"]
                    .as_array()
                    .context("hermite node array")?;
                ensure!(
                    u.len().checked_mul(v.len()) == Some(list.len()),
                    "hermite node count is not the grid's"
                );
                let nodes = list
                    .iter()
                    .map(|node| -> Result<HermiteNode> {
                        let tangents = node["m_iTangent"]
                            .as_array()
                            .context("hermite node tangents")?;
                        ensure!(tangents.len() == 2, "hermite node needs two tangents");
                        Ok(HermiteNode {
                            point: p(&node["m_iPoint"], "hermite node point")?,
                            tangent: [
                                p(&tangents[0], "hermite u tangent")?,
                                p(&tangents[1], "hermite v tangent")?,
                            ],
                            mixed: p(&node["m_iMixedDer"], "hermite mixed derivative")?,
                        })
                    })
                    .collect::<Result<Vec<_>>>()?;
                Ok(Self::HermiteSurf {
                    envelope,
                    orient: s.fields["m_orientFlag"]
                        .as_bool()
                        .context("surface orientation")?,
                    u,
                    v,
                    nodes,
                })
            }
            name => bail!("unsupported parametric surface {name}"),
        }
    }
    pub fn bounds(&self) -> [[f64; 2]; 2] {
        match self {
            Self::SurfRev { envelope, .. }
            | Self::RuledSurf { envelope, .. }
            | Self::ConeSurf { envelope, .. }
            | Self::HermiteSurf { envelope, .. } => *envelope,
        }
    }
    pub fn orientation(&self) -> bool {
        match self {
            Self::SurfRev { orient, .. }
            | Self::RuledSurf { orient, .. }
            | Self::ConeSurf { orient, .. }
            | Self::HermiteSurf { orient, .. } => *orient,
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
            Self::ConeSurf {
                apex,
                x,
                y,
                z,
                half_angle,
                ..
            } => {
                let (sa, ca) = half_angle.sin_cos();
                let (su, cu) = u.sin_cos();
                let radial = add(mul(*x, cu), mul(*y, su));
                add(*apex, mul(add(mul(*z, ca), mul(radial, sa)), v))
            }
            Self::HermiteSurf {
                u: grid_u,
                v: grid_v,
                nodes,
                ..
            } => hermite_eval(grid_u, grid_v, nodes, u, v).0,
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
        if let Self::ConeSurf {
            x,
            y,
            z,
            half_angle,
            orient,
            ..
        } = self
        {
            let (sa, ca) = half_angle.sin_cos();
            let (su, cu) = u.sin_cos();
            let tu = mul(add(mul(*x, -su), mul(*y, cu)), v * sa);
            let tv = add(mul(*z, ca), mul(add(mul(*x, cu), mul(*y, su)), sa));
            let n = cross(tu, tv);
            ensure!(
                n.iter().all(|value| value.is_finite()),
                "nonfinite surface normal"
            );
            let l = len(n);
            ensure!(l > 1e-12, "surface normal is singular");
            return Ok(mul(n, if *orient { 1. / l } else { -1. / l }));
        }
        if let Self::HermiteSurf {
            u: grid_u,
            v: grid_v,
            nodes,
            orient,
            ..
        } = self
        {
            let (_, tu, tv) = hermite_eval(grid_u, grid_v, nodes, u, v);
            let n = cross(tu, tv);
            ensure!(
                n.iter().all(|value| value.is_finite()),
                "nonfinite surface normal"
            );
            let l = len(n);
            ensure!(l > 1e-12, "surface normal is singular");
            return Ok(mul(n, if *orient { 1. / l } else { -1. / l }));
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
        let mut du = add(
            mul(profile1.derivative(t1), (1. - v) * (r1[1] - r1[0])),
            mul(profile2.derivative(t2), v * (r2[1] - r2[0])),
        );
        // Remove the vanishing positive factor at the collapsed end. The
        // one-sided limiting normal still depends on the arc parameter u.
        if matches!(profile1, Curve::Point(_)) {
            du = mul(profile2.derivative(t2), r2[1] - r2[0]);
        } else if matches!(profile2, Curve::Point(_)) {
            du = mul(profile1.derivative(t1), r1[1] - r1[0]);
        }
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

    /// The knots a mesh of the part of a `HermiteSurf` inside `bounds` needs along `u`
    /// and along `v` (RE-460): the grid's own parameters, where its patches meet, and
    /// each patch cut into as many pieces as keep a triangle of a piece within `chord`
    /// of it. The knots are shared by every strip of cells beside them, so a cell cut
    /// finer along one axis cuts its whole strip. `None` for any other surface; an error
    /// where a patch would need more than `HERMITE_MAX_PIECES` pieces along an axis.
    pub fn hermite_knots(
        &self,
        bounds: [[f64; 2]; 2],
        chord: f64,
    ) -> Result<Option<[Vec<f64>; 2]>> {
        let Self::HermiteSurf {
            u: grid_u,
            v: grid_v,
            nodes,
            ..
        } = self
        else {
            return Ok(None);
        };
        // The cells of one axis that `bounds` meets with the part of each inside them, the
        // end cells reaching to infinity where a patch is continued past the grid.
        let strips = |params: &[f64], low: f64, high: f64| -> Vec<(usize, f64, f64)> {
            let last = params.len() - 2;
            (0..=last)
                .filter_map(|i| {
                    let start = if i == 0 { f64::NEG_INFINITY } else { params[i] };
                    let end = if i == last {
                        f64::INFINITY
                    } else {
                        params[i + 1]
                    };
                    let (a, b) = (start.max(low), end.min(high));
                    (b > a).then_some((i, a, b))
                })
                .collect()
        };
        let cols = strips(grid_u, bounds[0][0], bounds[1][0]);
        let rows = strips(grid_v, bounds[0][1], bounds[1][1]);
        let mut along_u = vec![1usize; cols.len()];
        let mut along_v = vec![1usize; rows.len()];
        for (ci, &(i, ua, ub)) in cols.iter().enumerate() {
            for (ri, &(j, va, vb)) in rows.iter().enumerate() {
                let (du, dv) = (grid_u[i + 1] - grid_u[i], grid_v[j + 1] - grid_v[j]);
                let ranges = (
                    ((ua - grid_u[i]) / du, (ub - grid_u[i]) / du),
                    ((va - grid_v[j]) / dv, (vb - grid_v[j]) / dv),
                );
                let [m_uu, m_vv, m_uv] =
                    hermite_second_derivatives(grid_u, grid_v, nodes, (i, j), ranges);
                // A triangle of a piece `hu` by `hv` leaves the patch by at most an eighth
                // of the second difference along its sides and its diagonal.
                let (width, height) = (ub - ua, vb - va);
                let sag = |ku: usize, kv: usize| {
                    let (hu, hv) = (width / ku as f64, height / kv as f64);
                    (hu * hu * m_uu + 2. * hu * hv * m_uv + hv * hv * m_vv) / 8.
                };
                let mut best: Option<(usize, usize)> = None;
                for ku in 1..=HERMITE_MAX_PIECES {
                    let Some(kv) = (1..=HERMITE_MAX_PIECES).find(|kv| sag(ku, *kv) <= chord) else {
                        continue;
                    };
                    if best.is_none_or(|(bu, bv)| ku * kv < bu * bv) {
                        best = Some((ku, kv));
                    }
                }
                let (ku, kv) = best.with_context(|| {
                    format!("a hermite patch needs more than {HERMITE_MAX_PIECES} pieces")
                })?;
                along_u[ci] = along_u[ci].max(ku);
                along_v[ri] = along_v[ri].max(kv);
            }
        }
        let knots = |strips: &[(usize, f64, f64)], pieces: &[usize]| -> Vec<f64> {
            strips
                .iter()
                .zip(pieces)
                .flat_map(|(&(_, a, b), &k)| {
                    (0..=k).map(move |m| a + (b - a) * m as f64 / k as f64)
                })
                .collect()
        };
        Ok(Some([knots(&cols, &along_u), knots(&rows, &along_v)]))
    }
}

fn c_range(c: &Curve) -> [f64; 2] {
    c.range()
}

/// The most pieces one patch of a `HermiteSurf` is cut into along an axis (RE-460).
const HERMITE_MAX_PIECES: usize = 64;

/// The cubic Hermite basis of one end of a cell at `x`: the value, its slope, the
/// tangent's basis and its slope.
fn hermite_basis(end: usize, x: f64) -> [f64; 4] {
    if end == 0 {
        [
            2. * x * x * x - 3. * x * x + 1.,
            6. * x * x - 6. * x,
            x * x * x - 2. * x * x + x,
            3. * x * x - 4. * x + 1.,
        ]
    } else {
        [
            -2. * x * x * x + 3. * x * x,
            -6. * x * x + 6. * x,
            x * x * x - x * x,
            3. * x * x - 2. * x,
        ]
    }
}

/// The patch of cell `(i, j)` at the local coordinates `(t, s)`, 0 to 1 across the cell
/// and continued outside it: the point and its derivatives along `u` and along `v` per
/// unit of the parameter. The patch is the nodes' points, their tangents scaled by the
/// cell's width, and their mixed derivatives scaled by its area, each with its basis.
fn hermite_cell_patch(
    us: &[f64],
    vs: &[f64],
    nodes: &[HermiteNode],
    (i, j): (usize, usize),
    (t, s): (f64, f64),
) -> (Point, Point, Point) {
    let (du, dv) = (us[i + 1] - us[i], vs[j + 1] - vs[j]);
    let (mut point, mut d_u, mut d_v) = ([0.; 3], [0.; 3], [0.; 3]);
    for a in 0..2 {
        let ta = hermite_basis(a, t);
        for b in 0..2 {
            let sb = hermite_basis(b, s);
            let node = &nodes[(j + b) * us.len() + i + a];
            for k in 0..3 {
                let position = node.point[k];
                let tangent_u = node.tangent[0][k] * du;
                let tangent_v = node.tangent[1][k] * dv;
                let twist = node.mixed[k] * du * dv;
                point[k] += position * ta[0] * sb[0]
                    + tangent_u * ta[2] * sb[0]
                    + tangent_v * ta[0] * sb[2]
                    + twist * ta[2] * sb[2];
                d_u[k] += (position * ta[1] * sb[0]
                    + tangent_u * ta[3] * sb[0]
                    + tangent_v * ta[1] * sb[2]
                    + twist * ta[3] * sb[2])
                    / du;
                d_v[k] += (position * ta[0] * sb[1]
                    + tangent_u * ta[2] * sb[1]
                    + tangent_v * ta[0] * sb[3]
                    + twist * ta[2] * sb[3])
                    / dv;
            }
        }
    }
    (point, d_u, d_v)
}

/// The patch of the cell that holds `(u, v)` (the nearest one, continued, past the grid)
/// and its derivatives along `u` and `v`.
fn hermite_eval(
    us: &[f64],
    vs: &[f64],
    nodes: &[HermiteNode],
    u: f64,
    v: f64,
) -> (Point, Point, Point) {
    let cell = |params: &[f64], x: f64| {
        params
            .partition_point(|p| *p <= x)
            .saturating_sub(1)
            .min(params.len() - 2)
    };
    let (i, j) = (cell(us, u), cell(vs, v));
    let t = (u - us[i]) / (us[i + 1] - us[i]);
    let s = (v - vs[j]) / (vs[j + 1] - vs[j]);
    hermite_cell_patch(us, vs, nodes, (i, j), (t, s))
}

/// The longest second derivatives of one cell's patch over the part of it in `ranges` (local
/// coordinates, `u` then `v`): `|P_uu|`, `|P_vv|` and `|P_uv|` in feet per unit of the
/// parameter squared, found on five samples along each axis by central differences, which are
/// exact for the cubic along an axis, and a quarter more for what the samples miss.
fn hermite_second_derivatives(
    us: &[f64],
    vs: &[f64],
    nodes: &[HermiteNode],
    cell: (usize, usize),
    ranges: ((f64, f64), (f64, f64)),
) -> [f64; 3] {
    let (du, dv) = (us[cell.0 + 1] - us[cell.0], vs[cell.1 + 1] - vs[cell.1]);
    let at = |t: f64, s: f64| hermite_cell_patch(us, vs, nodes, cell, (t, s)).0;
    let h = 1e-2;
    let mut largest = [0.0f64; 3];
    for a in 0..5 {
        for b in 0..5 {
            let t = ranges.0.0 + (ranges.0.1 - ranges.0.0) * a as f64 / 4.;
            let s = ranges.1.0 + (ranges.1.1 - ranges.1.0) * b as f64 / 4.;
            let centre = at(t, s);
            let (t_up, t_down) = (at(t + h, s), at(t - h, s));
            let (s_up, s_down) = (at(t, s + h), at(t, s - h));
            let corners = [
                at(t + h, s + h),
                at(t + h, s - h),
                at(t - h, s + h),
                at(t - h, s - h),
            ];
            let uu: Point =
                std::array::from_fn(|k| (t_up[k] - 2. * centre[k] + t_down[k]) / (h * du).powi(2));
            let vv: Point =
                std::array::from_fn(|k| (s_up[k] - 2. * centre[k] + s_down[k]) / (h * dv).powi(2));
            let uv: Point = std::array::from_fn(|k| {
                (corners[0][k] - corners[1][k] - corners[2][k] + corners[3][k])
                    / (4. * h * h * du * dv)
            });
            for (slot, value) in largest.iter_mut().zip([uu, vv, uv]) {
                *slot = slot.max(len(value));
            }
        }
    }
    largest.map(|x| 1.25 * x)
}
