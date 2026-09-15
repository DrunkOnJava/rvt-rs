//! Through-wall rectangular openings from owned native opening records.
use super::{Mesh, mesh};
use crate::{
    RevitFile, geometry::native_2027::NativeWallGeometry, native_document,
    native_metadata::identifier,
};
use anyhow::{Result, ensure};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};
#[derive(Debug, Serialize)]
pub(super) struct Cut {
    pub element_id: u64,
    pub unique_id: String,
    pub host_id: u64,
    pub corners: [[f64; 3]; 2],
    pub stream: String,
    pub group_record_offset: usize,
    pub body_sha256: String,
    pub source_object: usize,
}
pub(super) fn read(file: &mut RevitFile, ids: BTreeSet<u64>) -> Result<BTreeMap<u64, Cut>> {
    let mut cuts = BTreeMap::new();
    if ids.is_empty() {
        return Ok(cuts);
    }
    let options = native_document::Options {
        selected_ids: ids.clone(),
        ..Default::default()
    };
    native_document::extract(file, &options, |r| {
        ensure!(
            r.class_name.as_deref() == Some("SWallRectOpening"),
            "cut owner class mismatch"
        );
        let g = r
            .graph
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("opening graph unavailable"))?;
        let root = g
            .objects
            .first()
            .ok_or_else(|| anyhow::anyhow!("opening graph empty"))?;
        let p = &root.fields["m_oRectOpeningData"];
        let es: Vec<_> = g
            .edges
            .iter()
            .filter(|e| {
                e.source_object_index == 0 && Some(e.pointer_offset as u64) == p["offset"].as_u64()
            })
            .collect();
        ensure!(es.len() == 1, "opening data ownership ambiguous");
        let i = es[0].target_object_index;
        let o = &g.objects[i];
        ensure!(o.class_name == "RectOpeningData", "opening data class");
        let values = o.fields["m_aPoint"]
            .as_array()
            .ok_or_else(|| anyhow::anyhow!("opening points absent"))?;
        ensure!(values.len() == 2, "opening point cardinality");
        let mut corners = [[0.; 3]; 2];
        for j in 0..2 {
            ensure!(
                values[j].as_array().is_some_and(|a| a.len() == 3),
                "opening point dimension"
            );
            for k in 0..3 {
                corners[j][k] = values[j][k]
                    .as_f64()
                    .filter(|v| v.is_finite())
                    .ok_or_else(|| anyhow::anyhow!("opening coordinate invalid"))?;
            }
        }
        let cut = Cut {
            element_id: r.identity.element_id,
            unique_id: r.identity.unique_id.clone(),
            host_id: u64::try_from(identifier(&root.fields["m_hostId"])?)?,
            corners,
            stream: r.source.stream.clone(),
            group_record_offset: r.source.group_record_offset,
            body_sha256: r.source.body_sha256.clone(),
            source_object: i,
        };
        ensure!(
            cuts.insert(cut.element_id, cut).is_none(),
            "duplicate current opening"
        );
        Ok(())
    })?;
    ensure!(
        cuts.keys().copied().collect::<BTreeSet<_>>() == ids,
        "missing current opening"
    );
    Ok(cuts)
}
pub(super) fn validate_hosts(
    file: &mut RevitFile,
    expected: &BTreeMap<u64, BTreeSet<u64>>,
) -> Result<BTreeMap<u64, Option<String>>> {
    if expected.is_empty() {
        return Ok(BTreeMap::new());
    }
    let mut seen = BTreeMap::new();
    let options = native_document::Options {
        selected_ids: expected.keys().copied().collect(),
        ..Default::default()
    };
    native_document::extract(file, &options, |r| {
        let outcome = (|| -> Result<()> {
            ensure!(
                r.class_name.as_deref() == Some("SWall"),
                "rectangular cut host must be straight wall"
            );
            let g = r
                .graph
                .as_ref()
                .ok_or_else(|| anyhow::anyhow!("cut host graph unavailable"))?;
            let f = &g.objects[0].fields;
            ensure!(
                f["m_wallCrossSection"].as_i64() == Some(1),
                "slanted/tapered host unsupported"
            );
            let target = |i: usize, p: &serde_json::Value| -> Result<usize> {
                let es: Vec<_> = g
                    .edges
                    .iter()
                    .filter(|e| {
                        e.source_object_index == i
                            && Some(e.pointer_offset as u64) == p["offset"].as_u64()
                    })
                    .collect();
                ensure!(es.len() == 1, "host step ownership ambiguous");
                Ok(es[0].target_object_index)
            };
            let si = target(0, &f["m_geomSteps"])?;
            let steps = &g.objects[si];
            ensure!(steps.class_name == "GeomStepList", "host step list class");
            let mut actual = BTreeSet::new();
            for name in [
                "m_nonBRepGList",
                "m_bRepFormGList",
                "m_bRepAdjustGList",
                "m_bRepCutOutGList",
                "m_bRepPostCutOutGList",
                "m_bRepTweakGList",
            ] {
                for p in steps.fields[name]
                    .as_array()
                    .ok_or_else(|| anyhow::anyhow!("host step array absent"))?
                {
                    let o = &g.objects[target(si, p)?];
                    let v = &o.fields;
                    match o.class_name.as_str() {
                        "WallRefPlanesGStep" | "BaseWallGStep" => {}
                        "VerticalExtensionOfLayersGStep" => ensure!(
                            ["m_otherId0", "m_otherId1"]
                                .iter()
                                .all(|k| v[*k].as_array().is_some_and(Vec::is_empty)),
                            "wall layer extensions unsupported"
                        ),
                        "WallJoinTweakGStep" => ensure!(
                            ["m_idGeomJoined", "m_healingParents", "m_affectedEdgesTags"]
                                .iter()
                                .all(|k| v[*k].as_array().is_some_and(Vec::is_empty)),
                            "wall join modifications unsupported"
                        ),
                        "VCWSplitFaceGStep" => ensure!(
                            v["m_faceHistTable"].as_array().is_some_and(Vec::is_empty),
                            "split wall faces unsupported"
                        ),
                        "WallRectOpeningGStep" => {
                            ensure!(
                                v["m_bCutSomething"].as_i64() == Some(1)
                                    && v["m_removedWholeGeometry"].as_bool() == Some(false)
                                    && v["m_bUseUnattachedVoids"].as_bool() == Some(false)
                                    && v["m_bIsTrueBoolean"].as_bool() == Some(false),
                                "unqualified rectangular opening operation"
                            );
                            ensure!(
                                identifier(&v["m_infillIdBefore"])? == -1
                                    && identifier(&v["m_infillIdAfter"])? == -1
                                    && v["m_midEndsIds"] == serde_json::json!([-1, -1]),
                                "opening phase infill or edge modification unsupported"
                            );
                            ensure!(
                                actual.insert(u64::try_from(identifier(&v["m_instId"])?)?),
                                "duplicate opening step"
                            );
                        }
                        _ => anyhow::bail!("unqualified host geometry step {}", o.class_name),
                    }
                }
            }
            ensure!(
                expected.get(&r.identity.element_id) == Some(&actual),
                "saved wall steps and opening owners disagree"
            );
            Ok(())
        })();
        ensure!(
            seen.insert(
                r.identity.element_id,
                outcome.err().map(|e| format!("{e:#}"))
            )
            .is_none(),
            "duplicate current wall host"
        );
        Ok(())
    })?;
    ensure!(
        seen.keys().copied().collect::<BTreeSet<_>>() == expected.keys().copied().collect(),
        "missing current host graph"
    );
    Ok(seen)
}
fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a.iter().zip(b).map(|(a, b)| a * b).sum()
}
fn sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    std::array::from_fn(|i| a[i] - b[i])
}
/// Subtract the union of rectangles using exact shared grid coordinates.
/// Only boundary edges survive; overlapping cutters do not create interior faces.
fn rectangle_difference(domain: [f64; 4], rectangles: &[[f64; 4]]) -> Result<Vec<Vec<[f64; 2]>>> {
    ensure!(
        rectangles.len() <= 128,
        "rectangular opening arrangement budget exceeded"
    );
    let [u0, v0, u1, v1] = domain;
    ensure!(
        domain.into_iter().all(f64::is_finite) && u1 > u0 && v1 > v0,
        "invalid wall domain"
    );
    let mut xs = vec![u0, u1];
    let mut ys = vec![v0, v1];
    for &[a, b, c, d] in rectangles {
        ensure!(
            [a, b, c, d].into_iter().all(f64::is_finite) && c - a > 1e-8 && d - b > 1e-8,
            "degenerate opening rectangle"
        );
        ensure!(
            a >= u0 && b >= v0 && c <= u1 && d <= v1,
            "opening outside wall domain unsupported"
        );
        xs.extend([a, c]);
        ys.extend([b, d]);
    }
    xs.sort_by(f64::total_cmp);
    xs.dedup();
    ys.sort_by(f64::total_cmp);
    ys.dedup();
    ensure!(
        xs.windows(2)
            .chain(ys.windows(2))
            .all(|p| p[1] - p[0] > 1e-8),
        "opening boundaries too close to resolve"
    );
    let nx = xs.len() - 1;
    let ny = ys.len() - 1;
    let mut solid = vec![false; nx * ny];
    for x in 0..nx {
        for y in 0..ny {
            let u = xs[x] + (xs[x + 1] - xs[x]) * 0.5;
            let v = ys[y] + (ys[y + 1] - ys[y]) * 0.5;
            solid[x * ny + y] = !rectangles
                .iter()
                .any(|r| u > r[0] && u < r[2] && v > r[1] && v < r[3]);
        }
    }
    let mut edges = Vec::new();
    for x in 0..nx {
        for y in 0..ny {
            if solid[x * ny + y] {
                let a = [xs[x], ys[y], 0.];
                let b = [xs[x + 1], ys[y], 0.];
                let c = [xs[x + 1], ys[y + 1], 0.];
                let d = [xs[x], ys[y + 1], 0.];
                if y == 0 || !solid[x * ny + y - 1] {
                    edges.push((a, b));
                }
                if x + 1 == nx || !solid[(x + 1) * ny + y] {
                    edges.push((b, c));
                }
                if y + 1 == ny || !solid[x * ny + y + 1] {
                    edges.push((c, d));
                }
                if x == 0 || !solid[(x - 1) * ny + y] {
                    edges.push((d, a));
                }
            }
        }
    }
    let mut rings = mesh::loops(edges)?;
    // Cap triangulation and side faces must share the same non-collinear edges.
    // Remove grid subdivisions so triangulation cannot leave boundary T-junctions.
    for ring in &mut rings {
        let old = ring.clone();
        *ring = (0..old.len())
            .filter_map(|i| {
                let a = old[(i + old.len() - 1) % old.len()];
                let b = old[i];
                let c = old[(i + 1) % old.len()];
                if (a[0] == b[0] && b[0] == c[0]) || (a[1] == b[1] && b[1] == c[1]) {
                    None
                } else {
                    Some(b)
                }
            })
            .collect();
        ensure!(ring.len() >= 3, "degenerate wall remainder");
    }
    Ok(rings)
}
pub(super) fn wall(g: &NativeWallGeometry, cuts: &[&Cut]) -> Result<Mesh> {
    ensure!(!cuts.is_empty(), "cut wall needs openings");
    let s = &g.surfaces[0];
    let x = s.basis_x;
    let y = s.basis_y;
    let n = [
        x[1] * y[2] - x[2] * y[1],
        x[2] * y[0] - x[0] * y[2],
        x[0] * y[1] - x[1] * y[0],
    ];
    ensure!(
        s.radius.is_none()
            && (dot(x, x) - 1.).abs() < 1e-9
            && (dot(y, y) - 1.).abs() < 1e-9
            && dot(x, y).abs() < 1e-9,
        "cut wall frame not orthonormal planar"
    );
    let [u0, v0, u1, v1] = s.domain;
    let mut rectangles = Vec::new();
    for cut in cuts {
        let mut uv = [[0.; 2]; 2];
        for (i, p) in cut.corners.iter().enumerate() {
            let delta = sub(*p, s.origin);
            ensure!(
                dot(delta, n).abs() < 1e-7,
                "opening corners outside wall reference plane"
            );
            uv[i] = [dot(delta, x), dot(delta, y)];
        }
        let a = [uv[0][0].min(uv[1][0]), uv[0][1].min(uv[1][1])];
        let b = [uv[0][0].max(uv[1][0]), uv[0][1].max(uv[1][1])];
        rectangles.push([a[0], a[1], b[0], b[1]]);
    }
    let rings = rectangle_difference([u0, v0, u1, v1], &rectangles)?;
    let a = &g.surfaces[1];
    let b = &g.surfaces[2];
    ensure!(
        a.basis_x == x && b.basis_x == x && a.basis_y == y && b.basis_y == y,
        "wall side frames differ"
    );
    let delta = sub(b.origin, a.origin);
    let width = dot(delta, n);
    ensure!(
        width.abs() > 1e-8 && dot(delta, x).abs() < 1e-8 && dot(delta, y).abs() < 1e-8,
        "wall thickness not normal to profile"
    );
    let mut mesh = mesh::floor(&rings, 0f64.min(width), 0f64.max(width))?;
    for p in &mut mesh.vertices {
        let q = *p;
        *p = std::array::from_fn(|i| a.origin[i] + x[i] * q[0] + y[i] * q[1] + n[i] * q[2]);
    }
    Ok(mesh)
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::native_2027::{NativeCurve, NativeSurface};
    fn geometry() -> NativeWallGeometry {
        let s = NativeSurface {
            domain: [0., 0., 12., 9.],
            origin: [0., 0., 0.],
            basis_x: [1., 0., 0.],
            basis_y: [0., 0., 1.],
            basis_z: None,
            radius: None,
        };
        let mut a = s.clone();
        a.origin[1] = 0.25;
        let mut b = s.clone();
        b.origin[1] = -0.25;
        NativeWallGeometry {
            curve: NativeCurve {
                parameters: [0., 12.],
                origin: [0., 0., 0.],
                basis_x: [1., 0., 0.],
                basis_y: None,
                radius: None,
            },
            surfaces: [s.clone(), a, b, s],
        }
    }
    fn cut() -> Cut {
        Cut {
            element_id: 2,
            unique_id: "cut".into(),
            host_id: 1,
            corners: [[3., 0., 2.], [6., 0., 4.]],
            stream: "test".into(),
            group_record_offset: 0,
            body_sha256: "test".into(),
            source_object: 0,
        }
    }
    #[test]
    fn rectangular_cut_changes_volume_and_retains_closed_mesh() {
        let g = geometry();
        let c = cut();
        let m = wall(&g, &[&c]).unwrap();
        let mut volume = 0.;
        for t in &m.triangles {
            let a = m.vertices[t[0] as usize];
            let b = m.vertices[t[1] as usize];
            let c = m.vertices[t[2] as usize];
            volume += dot(
                a,
                [
                    b[1] * c[2] - b[2] * c[1],
                    b[2] * c[0] - b[0] * c[2],
                    b[0] * c[1] - b[1] * c[0],
                ],
            ) / 6.;
        }
        assert!((volume - 51.).abs() < 1e-8);
    }
    #[test]
    fn edge_and_duplicate_cuts_form_one_boundary() {
        let g = geometry();
        let mut c = cut();
        c.corners[0][2] = 0.;
        assert!(wall(&g, &[&c]).is_ok());
        let c = cut();
        assert_eq!(
            wall(&g, &[&c]).unwrap().vertices,
            wall(&g, &[&c, &c]).unwrap().vertices
        );
    }
    #[test]
    fn overlapping_rectangles_union_and_disconnected_remnants_refuse() {
        let rings =
            rectangle_difference([0., 0., 12., 9.], &[[3., 2., 6., 4.], [5., 3., 8., 6.]]).unwrap();
        assert_eq!(rings.len(), 2);
        assert_eq!(rings[1].len(), 8);
        assert!(rectangle_difference([0., 0., 12., 9.], &[[3., 0., 6., 9.]]).is_err());
        assert!(rectangle_difference([0., 0., 12., 9.], &[[3., -1., 6., 4.]]).is_err());
    }
    #[test]
    fn union_caps_and_sides_are_closed_and_oriented() {
        for rectangles in [
            vec![[3., 2., 6., 4.], [5., 3., 8., 6.]],
            vec![[3., 0., 6., 4.]],
            vec![[3., 2., 6., 4.], [6., 2., 8., 4.]],
        ] {
            let rings = rectangle_difference([0., 0., 12., 9.], &rectangles).unwrap();
            let mesh = mesh::floor(&rings, 0., 0.5).unwrap();
            let mut edges = BTreeMap::<(u32, u32), (usize, i32)>::new();
            for t in &mesh.triangles {
                for (a, b) in [(t[0], t[1]), (t[1], t[2]), (t[2], t[0])] {
                    let e = edges.entry((a.min(b), a.max(b))).or_default();
                    e.0 += 1;
                    e.1 += if a < b { 1 } else { -1 };
                }
            }
            assert!(
                edges
                    .values()
                    .all(|&(count, direction)| count == 2 && direction == 0)
            );
        }
    }
}
