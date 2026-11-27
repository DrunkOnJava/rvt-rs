//! Bounded saved-record blend reader: a `BlendElem` joins the straight-edged
//! outlines of two sketches, in parallel planes, by their corresponding
//! vertices. It uses the same sketch, plane and profile machinery as the
//! extrusion reader. Units are feet.
use super::*;

/// The blend's `Double` parameter that is the offset of one profile along its
/// sketch plane's normal, the lower of the two parameter ids.
pub const FIRST_OFFSET_PARAMETER: i64 = -1_001_804;
/// The other profile's offset.
pub const SECOND_OFFSET_PARAMETER: i64 = -1_001_805;

/// One profile of a blend: a sketch's outline, placed in its plane's frame.
#[derive(Clone, Debug)]
pub struct BlendSection {
    pub sketch_id: u64,
    pub plane_id: u64,
    pub frame: Frame,
    /// The outline's vertices in the plane's coordinates, in loop order.
    pub vertices: Vec<V2>,
    /// The distance from the plane's origin along its normal at which the
    /// profile lies.
    pub offset: f64,
}

impl BlendSection {
    /// The vertex `i` in the family's coordinates.
    pub fn world(&self, i: usize) -> V3 {
        let [x, y, z] = self.frame.axes;
        let o = self.frame.origin;
        let [u, v] = self.vertices[i];
        [
            o[0] + u * x[0] + v * y[0] + self.offset * z[0],
            o[1] + u * x[1] + v * y[1] + self.offset * z[1],
            o[2] + u * x[2] + v * y[2] + self.offset * z[2],
        ]
    }
}

/// A blend as its records describe it: two sections, in the order its
/// `m_sketchIds` lists them.
#[derive(Clone, Debug)]
pub struct Blend {
    pub id: u64,
    /// Whether it is a void.
    pub cutting: bool,
    pub sections: [BlendSection; 2],
}

fn area_vector(points: &[V3]) -> V3 {
    let mut sum = [0.0; 3];
    for (i, p) in points.iter().enumerate() {
        let q = points[(i + 1) % points.len()];
        let c = cross3(*p, q);
        sum = [sum[0] + c[0], sum[1] + c[1], sum[2] + c[2]];
    }
    [sum[0] / 2.0, sum[1] / 2.0, sum[2] / 2.0]
}

impl Blend {
    /// The same blend with the two offsets exchanged between its sections.
    /// The records do not say which offset belongs to which sketch; the
    /// pairing by order is the one [`FormDocument::blend`] returns, and a
    /// caller can compare both with a saved mesh.
    pub fn swapped(&self) -> Blend {
        let mut b = self.clone();
        let (a, c) = (self.sections[0].offset, self.sections[1].offset);
        b.sections[0].offset = c;
        b.sections[1].offset = a;
        b
    }

    /// The vertices of both sections in the family's coordinates, the first
    /// section's then the second's.
    pub fn corners(&self) -> Vec<V3> {
        (0..2)
            .flat_map(|s| (0..self.sections[s].vertices.len()).map(move |i| (s, i)))
            .map(|(s, i)| self.sections[s].world(i))
            .collect()
    }

    /// The least and greatest corner. Exact: every face is ruled between
    /// corresponding vertices.
    pub fn bounding_box(&self) -> (V3, V3) {
        let mut low = [f64::INFINITY; 3];
        let mut high = [f64::NEG_INFINITY; 3];
        for p in self.corners() {
            for k in 0..3 {
                low[k] = low[k].min(p[k]);
                high[k] = high[k].max(p[k]);
            }
        }
        (low, high)
    }

    /// Cubic feet. Each vertex of one section runs straight to the
    /// corresponding vertex of the other, so the area of the section at a
    /// fraction `t` of the way is quadratic in `t` and Simpson's rule gives
    /// the volume exactly.
    pub fn volume(&self) -> Result<f64, String> {
        let n = self.sections[0].frame.axes[2];
        let a: Vec<V3> = (0..self.sections[0].vertices.len())
            .map(|i| self.sections[0].world(i))
            .collect();
        let b: Vec<V3> = (0..self.sections[1].vertices.len())
            .map(|i| self.sections[1].world(i))
            .collect();
        let mid: Vec<V3> = a
            .iter()
            .zip(&b)
            .map(|(p, q)| {
                [
                    (p[0] + q[0]) / 2.0,
                    (p[1] + q[1]) / 2.0,
                    (p[2] + q[2]) / 2.0,
                ]
            })
            .collect();
        let area = |points: &[V3]| dot3(area_vector(points), n);
        let (a0, am, a1) = (area(&a), area(&mid), area(&b));
        if a0 * a1 <= 0.0 {
            return Err("sections are wound opposite ways".into());
        }
        let centre = |points: &[V3]| {
            let k = points.len() as f64;
            [
                points.iter().map(|p| p[0]).sum::<f64>() / k,
                points.iter().map(|p| p[1]).sum::<f64>() / k,
                points.iter().map(|p| p[2]).sum::<f64>() / k,
            ]
        };
        let h = dot3(sub3(centre(&b), centre(&a)), n).abs();
        Ok(h / 6.0 * (a0.abs() + 4.0 * am.abs() + a1.abs()))
    }
}

impl FormDocument {
    /// The saved blend records, without asserting that they are supported.
    pub fn blend_ids(&self) -> Vec<u64> {
        self.graphs
            .iter()
            .filter(|(_, g)| {
                g.objects
                    .first()
                    .is_some_and(|o| o.class_name == "BlendElem")
            })
            .map(|(id, _)| *id)
            .collect()
    }

    /// Decode a blend of two sketches whose outlines are single loops of
    /// straight edges with the same number of vertices, in parallel planes.
    /// Refuses anything else, and a plane whose frame is not orthonormal.
    pub fn blend(&self, id: u64) -> Result<Blend, String> {
        let graph = self.graphs.get(&id).ok_or("no blend record")?;
        let root = graph.objects.first().ok_or("empty graph")?;
        if root.class_name != "BlendElem" {
            return Err(format!("{} is not a blend", root.class_name));
        }
        let cutting = root
            .fields
            .get("m_cutting")
            .and_then(Fv::as_bool)
            .ok_or("no m_cutting")?;
        let ids: Vec<u64> = root
            .fields
            .get("m_sketchIds")
            .and_then(Fv::as_array)
            .ok_or("no m_sketchIds")?
            .iter()
            .map(|v| {
                identifier(v)
                    .and_then(|n| u64::try_from(n).ok())
                    .filter(|n| *n > 0)
            })
            .collect::<Option<_>>()
            .ok_or("invalid sketch id")?;
        let [first, second]: [u64; 2] = ids
            .try_into()
            .map_err(|_| "a blend has two sketches".to_string())?;
        let (mut a, mut b) = (None, None);
        for object in graph
            .objects
            .iter()
            .filter(|object| object.class_name == "ParamValueSetDouble")
        {
            let entries = object.fields.get("m_paramSet").and_then(Fv::as_array);
            for entry in entries.into_iter().flatten() {
                let value = entry.get("m_value").and_then(Fv::as_f64);
                match entry.get("m_paramId").and_then(identifier) {
                    Some(FIRST_OFFSET_PARAMETER) => a = value,
                    Some(SECOND_OFFSET_PARAMETER) => b = value,
                    _ => {}
                }
            }
        }
        let a = a.filter(|v| v.is_finite()).ok_or("no first offset")?;
        let b = b.filter(|v| v.is_finite()).ok_or("no second offset")?;
        let sections = [self.section(first, a)?, self.section(second, b)?];
        let (n0, n1) = (sections[0].frame.axes[2], sections[1].frame.axes[2]);
        if dot3(n0, n1).abs() < 1.0 - 1e-9 {
            return Err("sketch planes are not parallel".into());
        }
        if sections[0].vertices.len() != sections[1].vertices.len() {
            return Err("sections have different vertex counts".into());
        }
        Ok(Blend {
            id,
            cutting,
            sections,
        })
    }

    fn section(&self, sketch_id: u64, offset: f64) -> Result<BlendSection, String> {
        let curves = self.sketch_curves(sketch_id)?;
        let planes = curves.iter().map(|c| c.plane_id).collect::<BTreeSet<_>>();
        if planes.len() != 1 {
            return Err("curves on several sketch planes".into());
        }
        let plane_id = *planes.first().ok_or("no sketch plane id")?;
        let plane = self
            .graphs
            .get(&plane_id)
            .ok_or("sketch plane record absent")?;
        let trf = plane
            .objects
            .iter()
            .find(|object| object.class_name == "Trf")
            .ok_or("sketch plane has no transform")?;
        let rows = trf
            .fields
            .get("m_3x3")
            .and_then(Fv::as_array)
            .filter(|rows| rows.len() == 3)
            .ok_or("transform matrix")?;
        let mut matrix = [[0.0; 3]; 3];
        for (row, value) in matrix.iter_mut().zip(rows) {
            *row = triple(Some(value)).ok_or("transform row")?;
        }
        let origin = triple(trf.fields.get("m_or")).ok_or("transform origin")?;
        let flipped = plane
            .objects
            .first()
            .and_then(|object| object.fields.get("m_flipZ"))
            .and_then(Fv::as_bool)
            .ok_or("sketch plane flip not read")?;
        let frame = frame_of(matrix, origin, false, flipped, false)?;
        let planar = curves
            .iter()
            .map(|c| c.curve.to_plane(&frame))
            .collect::<Result<Vec<Curve>, String>>()?;
        let outline = profile(assemble_loops(&planar)?)?;
        let [only] = outline.loops.as_slice() else {
            return Err("a blend section is one loop".into());
        };
        if outline.depths.iter().any(|d| *d != 0) {
            return Err("a blend section has a hole".into());
        }
        let vertices = only
            .curves
            .iter()
            .map(|c| match c {
                Curve::Line { .. } => Ok(c.start_point()),
                Curve::Arc { .. } => Err("a blend section has an arc".to_string()),
            })
            .collect::<Result<Vec<V2>, String>>()?;
        if vertices.len() < 3 {
            return Err("a blend section has fewer than three vertices".into());
        }
        Ok(BlendSection {
            sketch_id,
            plane_id,
            frame,
            vertices,
            offset,
        })
    }
}
