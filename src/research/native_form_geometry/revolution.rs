//! Bounded saved-record revolution reader, using the shared line/arc profile
//! machinery (RE-391 and the local RE-392 fixtures). No fixture dimensions or
//! native API oracle values participate in decoding.
use super::*;

const START_ANGLE: i64 = -1_001_802;
const END_ANGLE: i64 = -1_001_803;
const MAX_VERTICES: usize = 1_000_000;
const MAX_PROFILE_POINTS: usize = 4096;

/// A positive revolution whose profile is in one radial half-plane, clear of
/// its directed axis. Units are feet and radians. Currently one convex loop;
/// holes, voids and flipped records are refused. Six 2020 fixtures are measured.
#[derive(Clone, Debug)]
pub struct Revolution {
    pub id: u64,
    pub sketch_id: u64,
    pub plane_id: u64,
    pub curve_ids: Vec<u64>,
    pub axis_curve_id: u64,
    pub axis_origin: V3,
    pub axis_direction: V3,
    pub radial_direction: V3,
    /// Coordinates are (radius, distance along the axis), not the saved plane.
    pub profile: Profile,
    pub start: f64,
    pub end: f64,
}

/// A closed triangulation from decoded profile curves and sweep parameters.
#[derive(Clone, Debug)]
pub struct RevolutionMesh {
    pub vertices: Vec<V3>,
    pub triangles: Vec<[usize; 3]>,
}

fn one<'a>(graph: &'a ObjectGraph, class: &str) -> Result<&'a GraphObject, String> {
    let mut objects = graph.objects.iter().filter(|o| o.class_name == class);
    let object = objects.next().ok_or_else(|| format!("missing {class}"))?;
    if objects.next().is_some() {
        return Err(format!("ambiguous {class}"));
    }
    Ok(object)
}

fn flag(object: &GraphObject, name: &str) -> Result<bool, String> {
    object
        .fields
        .get(name)
        .and_then(Fv::as_bool)
        .ok_or_else(|| format!("missing boolean {name}"))
}

fn id_field(object: &GraphObject, name: &str) -> Result<u64, String> {
    object
        .fields
        .get(name)
        .and_then(identifier)
        .and_then(|v| u64::try_from(v).ok())
        .filter(|v| *v > 0)
        .ok_or_else(|| format!("invalid {name}"))
}

impl FormDocument {
    /// The saved revolution records, without asserting that they are supported.
    pub fn revolution_ids(&self) -> Vec<u64> {
        self.graphs
            .iter()
            .filter(|(_, g)| {
                g.objects
                    .first()
                    .is_some_and(|o| o.class_name == "RevolutionElem")
            })
            .map(|(id, _)| *id)
            .collect()
    }

    /// Decode an unflipped positive revolution through the shared sketch reader.
    /// Repeated referenced records are refused rather than selecting a revision.
    pub fn revolution(&self, id: u64) -> Result<Revolution, String> {
        let graph = self.graphs.get(&id).ok_or("no revolution record")?;
        let root = graph.objects.first().ok_or("empty revolution record")?;
        if root.class_name != "RevolutionElem" {
            return Err("not a revolution".into());
        }
        if flag(root, "m_cutting")? {
            return Err("void revolution unsupported".into());
        }
        let sketch_id = id_field(root, "m_sketchId")?;
        let sketch = self
            .graphs
            .get(&sketch_id)
            .ok_or("missing revolution sketch")?;
        let sketch = one(sketch, "VarSketch")?;
        if flag(sketch, "m_flipXY")? || flag(sketch, "m_flipZ")? {
            return Err("flipped sketch unsupported".into());
        }
        let plane_id = id_field(sketch, "m_sketchPlaneId")?;
        let plane = self.graphs.get(&plane_id).ok_or("missing sketch plane")?;
        if flag(one(plane, "SketchPlane")?, "m_flipZ")? {
            return Err("flipped plane unsupported".into());
        }
        let mut angles = BTreeMap::new();
        let params = one(graph, "ParamValueSetDouble")?
            .fields
            .get("m_paramSet")
            .and_then(Fv::as_array)
            .ok_or("angle parameter set")?;
        for p in params {
            if let Some(key @ (START_ANGLE | END_ANGLE)) = p.get("m_paramId").and_then(identifier) {
                let value = p
                    .get("m_value")
                    .and_then(Fv::as_f64)
                    .filter(|x| x.is_finite())
                    .ok_or("invalid angle")?;
                if angles.insert(key, value).is_some() {
                    return Err("duplicate angle parameter".into());
                }
            }
        }
        let start = *angles.get(&START_ANGLE).ok_or("missing start angle")?;
        let end = *angles.get(&END_ANGLE).ok_or("missing end angle")?;
        if !(0.0 <= start && start < end && end <= TAU + 1e-12) {
            return Err("unsupported angular range".into());
        }
        let curves = self.sketch_curves(sketch_id)?;
        let mut profile_curves = Vec::new();
        let mut curve_ids = Vec::new();
        let mut axis = None;
        let mut referenced = vec![id, sketch_id, plane_id];
        for c in curves {
            referenced.push(c.id);
            if c.plane_id != plane_id {
                return Err("curve plane mismatch".into());
            }
            let graph = self.graphs.get(&c.id).ok_or("missing curve")?;
            if id_field(one(graph, "SketchMembership")?, "m_groupId")? != sketch_id {
                return Err("curve membership mismatch".into());
            }
            let (_, driver) = curve_driver(graph)?;
            if flag(driver, "m_flip")? {
                return Err("flipped curve unsupported".into());
            }
            let data = one(graph, "RevolutionCurveData")?;
            if flag(data, "m_bOldAxis")? {
                return Err("old axis unsupported".into());
            }
            if flag(data, "m_axis")? {
                let Curve3::Line { from, to } = c.curve else {
                    return Err("axis is not a line".into());
                };
                if axis.replace((c.id, from, to)).is_some() {
                    return Err("multiple axes".into());
                }
            } else {
                let (driver_index, _) = curve_driver(graph)?;
                let (_, object) = linked_object(graph, driver_index, "m_pCrv")?;
                if object.class_name == "GArc" && flag(object, "m_bFilled")? {
                    return Err("filled arc unsupported".into());
                }
                profile_curves.push(c.curve);
                curve_ids.push(c.id);
            }
        }
        if referenced.iter().any(|id| self.repeated.contains(id)) {
            return Err("repeated referenced record".into());
        }
        let (axis_curve_id, axis_origin, axis_end) = axis.ok_or("missing marked axis")?;
        let direction = sub3(axis_end, axis_origin);
        let length = norm3(direction);
        if !length.is_finite() || length < TOLERANCE {
            return Err("degenerate axis".into());
        }
        let axis_direction = direction.map(|v| v / length);
        let first = profile_curves
            .first()
            .ok_or("empty revolution profile")?
            .point_at(0.0);
        let relative = sub3(first, axis_origin);
        let radial = sub3(
            relative,
            axis_direction.map(|v| v * dot3(relative, axis_direction)),
        );
        let radius = norm3(radial);
        if !radius.is_finite() || radius < TOLERANCE {
            return Err("profile touches axis".into());
        }
        let radial_direction = radial.map(|v| v / radius);
        let frame = Frame {
            origin: axis_origin,
            axes: [
                radial_direction,
                axis_direction,
                cross3(radial_direction, axis_direction),
            ],
        };
        let planar = profile_curves
            .iter()
            .map(|c| c.to_plane(&frame))
            .collect::<Result<Vec<_>, _>>()?;
        if planar.iter().any(|c| {
            !c.range_along([1.0, 0.0]).0.is_finite() || c.range_along([1.0, 0.0]).0 <= TOLERANCE
        }) {
            return Err("profile crosses axis or opposite half-plane".into());
        }
        let loops = assemble_loops(&planar)?;
        if loops.len() != 1 || loops[0].curves.len() != planar.len() {
            return Err("requires one closed loop without repaired gaps".into());
        }
        // The shared assembler tolerates small gaps at internal joins as well
        // as the closing join. Revolutions must not silently bridge either.
        let curves = &loops[0].curves;
        for (i, curve) in curves.iter().enumerate() {
            if dist2(
                curve.end_point(),
                curves[(i + 1) % curves.len()].start_point(),
            ) > 1e-9
            {
                return Err("profile curves do not meet without repaired gaps".into());
            }
        }
        let result = Revolution {
            id,
            sketch_id,
            plane_id,
            curve_ids,
            axis_curve_id,
            axis_origin,
            axis_direction,
            radial_direction,
            profile: profile(loops)?,
            start,
            end,
        };
        // Validate the bounded polygon before exposing it as a supported solid.
        result.polygon()?;
        Ok(result)
    }
}

impl Revolution {
    /// Evaluate a decoded profile point at a sweep angle, in family coordinates.
    pub fn point_at(&self, point: V2, angle: f64) -> V3 {
        let (sin, cos) = angle.sin_cos();
        let side = cross3(self.axis_direction, self.radial_direction);
        std::array::from_fn(|i| {
            self.axis_origin[i]
                + self.axis_direction[i] * point[1]
                + point[0] * (cos * self.radial_direction[i] + sin * side[i])
        })
    }

    fn step(&self) -> f64 {
        if self
            .profile
            .loops
            .iter()
            .flat_map(|l| &l.curves)
            .any(|c| matches!(c, Curve::Arc { .. }))
        {
            PI / 240.0
        } else {
            PI / 720.0
        }
    }

    fn polygon(&self) -> Result<Vec<V2>, String> {
        let [loop_] = self.profile.loops.as_slice() else {
            return Err("requires one profile loop".into());
        };
        let mut points = Vec::new();
        for curve in &loop_.curves {
            let n = match curve {
                Curve::Line { .. } => 1,
                Curve::Arc { start, end, .. } => {
                    ((end - start).abs() / self.step()).ceil() as usize
                }
            };
            if n == 0 || n > MAX_PROFILE_POINTS.saturating_sub(points.len()) {
                return Err("profile point budget".into());
            }
            for i in 0..n {
                points.push(curve.point_at(i as f64 / n as f64));
            }
        }
        if points.len() < 3 || points.iter().flatten().any(|x| !x.is_finite()) {
            return Err("degenerate profile polygon".into());
        }
        if loop_.signed_area() < 0.0 {
            points.reverse();
        }
        // A convex loop makes a cap fan valid. Check every point against every
        // directed edge, which also refuses self-crossing star polygons.
        for (i, &a) in points.iter().enumerate() {
            let b = points[(i + 1) % points.len()];
            if dist2(a, b) < 1e-10 {
                return Err("zero-length profile edge".into());
            }
            if points.iter().any(|&p| orient(a, b, p) < -1e-10) {
                return Err("nonconvex or crossing profile unsupported".into());
            }
        }
        Ok(points)
    }

    /// Closed mesh for the supported convex loop. Arc and sweep steps are at
    /// most 0.75 degrees (0.25 for a line-only profile), with an allocation cap.
    pub fn mesh(&self) -> Result<RevolutionMesh, String> {
        let polygon = self.polygon()?;
        if !(self.start.is_finite()
            && self.end.is_finite()
            && 0.0 <= self.start
            && self.start < self.end
            && self.end <= TAU + 1e-12)
        {
            return Err("unsupported angular range".into());
        }
        let sweep = self.end - self.start;
        let n = (sweep / self.step()).ceil() as usize;
        let closed = (sweep - TAU).abs() < 1e-10;
        let rings = if closed { n } else { n + 1 };
        let size = polygon.len();
        if n == 0 || rings.checked_mul(size).is_none_or(|v| v > MAX_VERTICES - 2) {
            return Err("revolution mesh budget".into());
        }
        let mut vertices = Vec::with_capacity(rings * size + 2);
        for j in 0..rings {
            let angle = self.start + sweep * j as f64 / n as f64;
            vertices.extend(polygon.iter().map(|&p| self.point_at(p, angle)));
        }
        let mut triangles = Vec::with_capacity(2 * n * size + 2 * size);
        for j in 0..n {
            let k = (j + 1) % rings;
            for i in 0..size {
                let h = (i + 1) % size;
                triangles.push([j * size + i, k * size + i, k * size + h]);
                triangles.push([j * size + i, k * size + h, j * size + h]);
            }
        }
        if !closed {
            let centre = polygon.iter().fold([0.0; 2], |a, p| {
                [a[0] + p[0] / size as f64, a[1] + p[1] / size as f64]
            });
            let cap = vertices.len();
            vertices.push(self.point_at(centre, self.start));
            vertices.push(self.point_at(centre, self.end));
            for i in 0..size {
                let h = (i + 1) % size;
                triangles.push([cap, i, h]);
                triangles.push([cap + 1, n * size + h, n * size + i]);
            }
        }
        // The radial/axial profile's positive winding sweeps inward; orient it
        // by signed volume, measured near the axis to avoid translation loss.
        let volume6: f64 = triangles
            .iter()
            .map(|t| {
                dot3(
                    sub3(vertices[t[0]], self.axis_origin),
                    cross3(
                        sub3(vertices[t[1]], self.axis_origin),
                        sub3(vertices[t[2]], self.axis_origin),
                    ),
                )
            })
            .sum();
        if !volume6.is_finite() || volume6.abs() < 1e-12 {
            return Err("degenerate revolution volume".into());
        }
        if volume6 < 0.0 {
            for t in &mut triangles {
                t.swap(1, 2);
            }
        }
        Ok(RevolutionMesh {
            vertices,
            triangles,
        })
    }
}
