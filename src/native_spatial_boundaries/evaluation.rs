use super::*;
use crate::native_parameters::ObjectGraph;

#[derive(Debug, Clone, Serialize)]
pub struct Boundary {
    pub boundary_location: String,
    pub loops: Vec<Vec<BoundarySegment>>,
    pub status: &'static str,
    pub coordinate_space: &'static str,
    pub length_unit: &'static str,
}
#[derive(Debug, Clone, Serialize)]
pub struct BoundarySegment {
    pub element_id: i64,
    pub link_element_id: i64,
    pub start: [f64; 3],
    pub end: [f64; 3],
    pub sources: Vec<Source>,
}
#[derive(Debug, Clone, Serialize)]
pub struct Opening {
    pub owner: Identity,
    pub host_id: i64,
    pub host: Option<Identity>,
    pub rectangle: [[f64; 3]; 2],
    pub source: Source,
    pub status: &'static str,
}
struct Wall {
    type_id: i64,
    origin: [f64; 3],
    direction: [f64; 3],
    offsets: [f64; 3],
    source: Source,
}
#[derive(Default)]
pub(super) struct Builder {
    walls: BTreeMap<i64, Wall>,
    widths: BTreeMap<i64, (f64, Source)>,
    elevations: BTreeMap<i64, f64>,
    openings: Vec<Opening>,
}
fn point(v: &Value) -> Result<[f64; 3]> {
    ensure!(array(v)?.len() == 3, "point dimension");
    Ok([number(&v[0])?, number(&v[1])?, number(&v[2])?])
}

fn is_horizontal_orthonormal_basis(x: [f64; 3], y: [f64; 3]) -> bool {
    let dot = x[0] * y[0] + x[1] * y[1] + x[2] * y[2];
    let nx = (x[0] * x[0] + x[1] * x[1] + x[2] * x[2]).sqrt();
    let ny = (y[0] * y[0] + y[1] * y[1] + y[2] * y[2]).sqrt();
    x.iter().chain(y.iter()).all(|v| v.is_finite())
        && nx.is_finite()
        && ny.is_finite()
        && (nx - 1.).abs() < 1e-7
        && (ny - 1.).abs() < 1e-7
        && dot.abs() < 1e-7
        && x[2].abs() < 1e-7
        && y[2].abs() < 1e-7
}
fn target<'a>(
    g: &'a ObjectGraph,
    owner: usize,
    p: &Value,
    class: &str,
) -> Result<(usize, &'a Value)> {
    let es: Vec<_> = g
        .edges
        .iter()
        .filter(|e| {
            e.source_object_index == owner && Some(e.pointer_offset as u64) == p["offset"].as_u64()
        })
        .collect();
    ensure!(es.len() == 1, "expected owned {class} edge");
    let i = es[0].target_object_index;
    let o = g
        .objects
        .get(i)
        .ok_or_else(|| anyhow::anyhow!("edge outside graph"))?;
    ensure!(
        o.class_name == class,
        "expected {class}, found {}",
        o.class_name
    );
    Ok((i, &o.fields))
}
impl Builder {
    pub(super) fn ingest(&mut self, r: &Record) -> Result<()> {
        if !matches!(
            r.class_name.as_deref(),
            Some("SWallRectOpening" | "SWall" | "BasicWallType" | "Level")
        ) {
            return Ok(());
        }
        let g = r
            .graph
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("boundary input graph absent"))?;
        let root = g
            .objects
            .first()
            .ok_or_else(|| anyhow::anyhow!("empty boundary input graph"))?;
        let f = &root.fields;
        if root.class_name == "SWallRectOpening" {
            let (i, d) = target(g, 0, &f["m_oRectOpeningData"], "RectOpeningData")?;
            ensure!(
                array(&d["m_aPoint"])?.len() == 2,
                "opening rectangle cardinality"
            );
            self.openings.push(Opening {
                owner: source(r, 0, "").owner,
                host_id: identifier(&f["m_hostId"])?,
                host: None,
                rectangle: [point(&d["m_aPoint"][0])?, point(&d["m_aPoint"][1])?],
                source: source(r, i, "m_oRectOpeningData.m_aPoint"),
                status: "saved_straight_wall_rectangle_not_host_boolean",
            });
            return Ok(());
        }
        if root.class_name == "Level" {
            let (_, p) = target(g, 0, &f["m_pSurface"], "Plane")?;
            let x = point(&p["m_xVec"])?;
            let y = point(&p["m_yVec"])?;
            ensure!(is_horizontal_orthonormal_basis(x, y), "nonhorizontal level");
            self.elevations
                .insert(r.identity.element_id as i64, point(&p["m_origin"])?[2]);
            return Ok(());
        }
        if root.class_name == "BasicWallType" {
            let (i, c) = target(g, 0, &f["m_pCompoundStructure"], "CompoundStructure")?;
            ensure!(int(&c["m_variableLayerIdx"])? == -1, "variable wall layer");
            if c["m_oVertRegStructure"]["pointer_token"].as_u64() != Some(0) {
                let (_, v) = target(g, i, &c["m_oVertRegStructure"], "VerticalRegionsStructure")?;
                ensure!(
                    array(&v["m_wallSweeps"])?.is_empty() && array(&v["m_reveals"])?.is_empty(),
                    "wall sweeps/reveals unsupported"
                );
                ensure!(
                    array(&v["m_grid"]["m_regions"])?.len() == array(&c["m_layers"])?.len(),
                    "vertically subdivided layers unsupported"
                );
                let height = number(&v["m_sampleHeight"])?;
                for s in array(&v["m_grid"]["m_segments"])? {
                    if int(&s["m_orientation"])? == 1 {
                        let y = number(&s["m_coordinate"])?;
                        ensure!(
                            y.abs() < 1e-9 || (y - height).abs() < 1e-9,
                            "variable vertical region boundary"
                        );
                    }
                }
            }
            let mut width = 0.;
            for l in array(&c["m_layers"])? {
                let w = number(&l["m_layerWidth"])?;
                ensure!(w >= 0., "negative layer width");
                width += w;
            }
            ensure!(width > 0., "zero wall width");
            self.widths.insert(
                r.identity.element_id as i64,
                (width, source(r, i, "m_layers[*].m_layerWidth")),
            );
            return Ok(());
        }
        ensure!(
            int(&f["m_wallCrossSection"])? == 1
                && int(&f["m_wallKeyRef"])? == 0
                && number(&f["m_keyRefOffset"])? == 0.
                && number(&f["m_locLineOffset"])? == 0.,
            "wall outside vertical centered scope"
        );
        let (di, d) = target(g, 0, &f["m_pCurveDriver"], "VWallDriver")?;
        let (_, line) = target(g, di, &d["m_pCrv"], "GLine")?;
        let origin = point(&line["m_origin"])?;
        let direction = point(&line["m_dirVec"])?;
        ensure!(
            direction[2].abs() < 1e-9
                && (direction[0] * direction[0] + direction[1] * direction[1] - 1.).abs() < 1e-9,
            "wall line direction"
        );
        let refs = array(&f["m_pRefFaces"])?;
        ensure!(refs.len() == 4, "wall reference face count");
        let mut offsets = [0.; 3];
        for (j, p) in refs.iter().enumerate() {
            let (fi, face) = target(g, 0, p, "Face")?;
            let (_, plane) = target(g, fi, &face["m_pSurf"], "Plane")?;
            let po = point(&plane["m_origin"])?;
            let x = point(&plane["m_xVec"])?;
            let y = point(&plane["m_yVec"])?;
            ensure!(
                x.iter().zip(direction).all(|(a, b)| (a - b).abs() < 1e-9) && y == [0., 0., 1.],
                "wall reference plane orientation"
            );
            let offset = -(po[0] - origin[0]) * direction[1] + (po[1] - origin[1]) * direction[0];
            if j == 0 {
                ensure!(offset.abs() < 1e-9, "wall reference not centered");
            } else {
                offsets[j - 1] = offset;
            }
        }
        ensure!(
            (offsets[2] - (offsets[0] + offsets[1]) / 2.).abs() < 1e-9,
            "core center inconsistency"
        );
        self.walls.insert(
            r.identity.element_id as i64,
            Wall {
                type_id: identifier(&f["m_WallAttributesId"])?,
                origin,
                direction,
                offsets,
                source: source(r, 0, "m_pRefFaces[*].m_pSurf"),
            },
        );
        Ok(())
    }
    pub(super) fn openings(self, ids: &BTreeMap<u64, Identity>) -> Vec<Opening> {
        self.openings
            .into_iter()
            .map(|mut o| {
                o.host = u64::try_from(o.host_id)
                    .ok()
                    .and_then(|id| ids.get(&id).cloned());
                o
            })
            .collect()
    }
    pub(super) fn evaluate(&self, r: &Room, mode: &str) -> Result<Boundary> {
        ensure!(!r.carrier_loops.is_empty(), "no qualified carrier loops");
        ensure!(
            r.lower_offset.abs() < 1e-9,
            "room lower offset not qualified"
        );
        let z = *self
            .elevations
            .get(&r.level_id)
            .ok_or_else(|| anyhow::anyhow!("level plane unavailable"))?;
        let mut loops = Vec::new();
        for carrier in &r.carrier_loops {
            let mut shifted = Vec::new();
            let mut sources = Vec::new();
            for s in carrier {
                ensure!(
                    s.key.linked_element_id == -1 && s.key.major_index == 0 && s.points.len() == 2,
                    "boundary outside host straight wall scope"
                );
                let w = self
                    .walls
                    .get(&s.key.host_or_link_instance_id)
                    .ok_or_else(|| anyhow::anyhow!("wall inputs not qualified"))?;
                let dx = s.points[1][0] - s.points[0][0];
                let dy = s.points[1][1] - s.points[0][1];
                let len = dx.hypot(dy);
                ensure!(len > 1e-9, "zero carrier length");
                let d = [dx / len, dy / len];
                let dot = d[0] * w.direction[0] + d[1] * w.direction[1];
                ensure!(
                    (dot.abs() - 1.).abs() < 1e-9,
                    "carrier not parallel to saved wall"
                );
                let signed_distance = -(s.points[0][0] - w.origin[0]) * w.direction[1]
                    + (s.points[0][1] - w.origin[1]) * w.direction[0];
                ensure!(
                    signed_distance.abs() < 1e-7,
                    "carrier not on saved wall center"
                );
                let mut src = vec![s.source.clone(), w.source.clone()];
                let offset = match mode {
                    "Center" => 0.,
                    "CoreCenter" => w.offsets[2] * dot,
                    "CoreBoundary" => {
                        if dot > 0. {
                            w.offsets[0].max(w.offsets[1])
                        } else {
                            -w.offsets[0].min(w.offsets[1])
                        }
                    }
                    "Finish" => {
                        let (width, source) = self
                            .widths
                            .get(&w.type_id)
                            .ok_or_else(|| anyhow::anyhow!("constant wall width unavailable"))?;
                        src.push(source.clone());
                        width / 2.
                    }
                    _ => anyhow::bail!("unknown boundary mode"),
                };
                shifted.push((
                    [
                        s.points[0][0] - d[1] * offset,
                        s.points[0][1] + d[0] * offset,
                    ],
                    d,
                ));
                sources.push(src);
            }
            let mut vertices = Vec::new();
            for i in 0..shifted.len() {
                vertices.push(intersection(
                    shifted[(i + shifted.len() - 1) % shifted.len()],
                    shifted[i],
                )?);
            }
            let mut segments = Vec::new();
            for i in 0..carrier.len() {
                let a = vertices[i];
                let b = vertices[(i + 1) % vertices.len()];
                let d = shifted[i].1;
                ensure!(
                    (b[0] - a[0]) * d[0] + (b[1] - a[1]) * d[1] > 1e-8,
                    "offset loop collapsed/reversed"
                );
                segments.push(BoundarySegment {
                    element_id: carrier[i].key.host_or_link_instance_id,
                    link_element_id: -1,
                    start: [a[0], a[1], z],
                    end: [b[0], b[1], z],
                    sources: sources[i].clone(),
                });
            }
            loops.push(segments);
        }
        validate_loops(&loops)?;
        Ok(Boundary {
            boundary_location: mode.into(),
            loops,
            status: "evaluated_vertical_linear_wall_intersections",
            coordinate_space: "document_internal",
            length_unit: "feet",
        })
    }
}
fn validate_loops(loops: &[Vec<BoundarySegment>]) -> Result<()> {
    fn turn(a: [f64; 3], b: [f64; 3], c: [f64; 3]) -> f64 {
        (b[0] - a[0]) * (c[1] - b[1]) - (b[1] - a[1]) * (c[0] - b[0])
    }
    fn inside(p: [f64; 3], poly: &[BoundarySegment]) -> bool {
        let mut yes = false;
        for s in poly {
            let a = s.start;
            let b = s.end;
            if (a[1] > p[1]) != (b[1] > p[1])
                && p[0] < (b[0] - a[0]) * (p[1] - a[1]) / (b[1] - a[1]) + a[0]
            {
                yes = !yes;
            }
        }
        yes
    }
    fn intersects(a: &BoundarySegment, b: &BoundarySegment) -> bool {
        let x = turn(a.start, a.end, b.start);
        let y = turn(a.start, a.end, b.end);
        let z = turn(b.start, b.end, a.start);
        let w = turn(b.start, b.end, a.end);
        let bbox = (0..2).all(|i| {
            a.start[i].min(a.end[i]) <= b.start[i].max(b.end[i]) + 1e-8
                && b.start[i].min(b.end[i]) <= a.start[i].max(a.end[i]) + 1e-8
        });
        bbox && x * y <= 1e-12 && z * w <= 1e-12
    }
    ensure!(!loops.is_empty(), "empty boundary");
    for (i, poly) in loops.iter().enumerate() {
        ensure!(poly.len() >= 3, "degenerate boundary loop");
        let sign = if i == 0 { 1. } else { -1. };
        for j in 0..poly.len() {
            ensure!(
                turn(poly[j].start, poly[j].end, poly[(j + 1) % poly.len()].end) * sign > 1e-10,
                "nonconvex/incorrectly oriented offset loop outside qualified scope"
            );
        }
        for j in 0..poly.len() {
            for k in j + 1..poly.len() {
                if k != j + 1 && !(j == 0 && k == poly.len() - 1) {
                    ensure!(
                        !intersects(&poly[j], &poly[k]),
                        "self intersecting offset loop"
                    );
                }
            }
        }
        if i > 0 {
            ensure!(
                inside(poly[0].start, &loops[0]),
                "offset hole outside outer boundary"
            );
        }
        for (j, other) in loops.iter().enumerate().take(i) {
            for a in poly {
                for b in other {
                    ensure!(!intersects(a, b), "offset boundary loops cross/touch");
                }
            }
            if j > 0 {
                ensure!(
                    !inside(poly[0].start, other) && !inside(other[0].start, poly),
                    "nested/overlapping offset holes unsupported"
                );
            }
        }
    }
    Ok(())
}
fn intersection(a: ([f64; 2], [f64; 2]), b: ([f64; 2], [f64; 2])) -> Result<[f64; 2]> {
    let cross = a.1[0] * b.1[1] - a.1[1] * b.1[0];
    ensure!(
        cross.abs() > 1e-8,
        "parallel/collinear boundary corner unsupported"
    );
    let t = ((b.0[0] - a.0[0]) * b.1[1] - (b.0[1] - a.0[1]) * b.1[0]) / cross;
    let p = [a.0[0] + t * a.1[0], a.0[1] + t * a.1[1]];
    ensure!(
        p.iter().all(|x| x.is_finite()),
        "nonfinite boundary intersection"
    );
    Ok(p)
}
#[cfg(test)]
mod tests {
    use super::*;
    fn polygon(points: &[[f64; 2]]) -> Vec<BoundarySegment> {
        points
            .iter()
            .enumerate()
            .map(|(i, p)| {
                let q = points[(i + 1) % points.len()];
                BoundarySegment {
                    element_id: 1,
                    link_element_id: -1,
                    start: [p[0], p[1], 0.],
                    end: [q[0], q[1], 0.],
                    sources: Vec::new(),
                }
            })
            .collect()
    }
    #[test]
    fn validates_outer_hole_winding_and_overlap() {
        let outer = polygon(&[[0., 0.], [10., 0.], [10., 10.], [0., 10.]]);
        let hole = polygon(&[[2., 2.], [2., 4.], [4., 4.], [4., 2.]]);
        assert!(validate_loops(&[outer.clone(), hole.clone()]).is_ok());
        let crossing = polygon(&[[9., 2.], [9., 4.], [11., 4.], [11., 2.]]);
        assert!(validate_loops(&[outer.clone(), crossing]).is_err());
        assert!(validate_loops(&[outer.clone(), hole.clone(), hole]).is_err());
        let concave = polygon(&[[0., 0.], [10., 0.], [10., 10.], [5., 5.], [0., 10.]]);
        assert!(validate_loops(&[concave]).is_err());
    }
    #[test]
    fn offset_corner_uses_both_carriers_not_original_vertex() {
        let p = intersection(([0., 0.4375], [1., 0.]), ([19.8125, 0.], [0., 1.])).unwrap();
        assert_eq!(p, [19.8125, 0.4375]);
        assert!(intersection(([0., 0.], [1., 0.]), ([0., 1.], [1., 0.])).is_err());
    }
    #[test]
    fn accepts_native_near_identity_level_basis_but_rejects_tilt() {
        assert!(is_horizontal_orthonormal_basis(
            [1., -2.2075076950890276e-15, -0.],
            [0., 1., 0.]
        ));
        assert!(!is_horizontal_orthonormal_basis(
            [1., 0., 1e-4],
            [0., 1., 0.]
        ));
        assert!(!is_horizontal_orthonormal_basis([1., 0., 0.], [1., 0., 0.]));
    }
}
