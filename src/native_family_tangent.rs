//! Qualified capsule: independently pinned centers and four explicit tangent joins.
use super::{add, arc, constraints::linear, cross, dot, norm, number, scale, sub, target, vector};
use crate::{native_metadata::identifier, native_parameters::ObjectGraph};
use anyhow::{Result, ensure};
use arc::{arr, empty};
use serde_json::{Value, json};
use std::collections::BTreeSet;
pub(super) fn solve(
    g: &ObjectGraph,
    form: u64,
    dimensions: &[(ObjectGraph, Value)],
    curves: &[Value],
    resolve: &mut impl FnMut(i64) -> Result<(f64, Value)>,
) -> Result<(Vec<[f64; 3]>, Value)> {
    let f = &g.objects[0].fields;
    ensure!(
        identifier(&f["m_userId"])? == form as i64,
        "tangent sketch owner"
    );
    ensure!(
        [
            "m_pointRecs",
            "m_pointObjIdxMap",
            "m_weakDimIds",
            "m_clineIds",
            "m_customDatumPlanes",
            "m_constrInfo"
        ]
        .iter()
        .all(|k| empty(&f[*k]))
            && f["m_pPlaneRef"]["pointer_token"].as_u64() == Some(0)
            && f["m_flipXY"].as_bool() == Some(false)
            && f["m_flipZ"].as_bool() == Some(false),
        "additional tangent sketch controls"
    );
    let plane = &g.objects[target(g, 0, &f["m_oParamPlane"])?];
    ensure!(plane.class_name == "Plane", "tangent parameter plane");
    let origin = vector(&plane.fields["m_origin"])?;
    let x = vector(&plane.fields["m_xVec"])?;
    let y = vector(&plane.fields["m_yVec"])?;
    ensure!(
        (norm(x) - 1.).abs() < 1e-9 && (norm(y) - 1.).abs() < 1e-9 && dot(x, y).abs() < 1e-9,
        "tangent plane basis"
    );
    let elems = arr(&f["m_elemRecs"])?;
    ensure!(elems.len() == 4, "capsule four elements required");
    let mut objects = Vec::new();
    let mut vars = Vec::new();
    let mut ids = Vec::new();
    for (i, p) in elems.iter().enumerate() {
        let index = target(g, 0, p)?;
        let o = &g.objects[index];
        let is_arc = i % 2 == 0;
        ensure!(
            o.class_name
                == if is_arc {
                    "VarSketchArcObj"
                } else {
                    "VarSketchLineSegObj"
                }
                && o.fields["m_unbounded"].as_bool() == Some(false),
            "capsule ordered arc/line elements"
        );
        if is_arc {
            ensure!(
                o.fields["m_flipped"].as_bool() == Some(false),
                "capsule flipped arc"
            );
        }
        vars.push(arc::values(g, index, if is_arc { 5 } else { 4 })?);
        ids.push(identifier(&o.fields["m_objId"])?);
        objects.push(o);
    }
    ensure!(
        ids.iter().collect::<BTreeSet<_>>().len() == 4,
        "duplicate capsule owner"
    );
    for i in [0, 2] {
        let v = &vars[i];
        let coef = number(&objects[i].fields["m_angleCoef"])?;
        let start = if i == 0 {
            std::f64::consts::FRAC_PI_2
        } else {
            -std::f64::consts::FRAC_PI_2
        };
        ensure!(
            coef > 0.
                && v[2] > 0.
                && (v[3] / coef - start).abs() < 1e-8
                && (v[4] / coef - start - std::f64::consts::PI).abs() < 1e-8,
            "capsule saved semicircle branch"
        );
    }
    ensure!(
        vars[0][0] < vars[2][0]
            && (vars[0][1] - vars[2][1]).abs() < 1e-8
            && (vars[0][2] - vars[2][2]).abs() < 1e-8,
        "capsule saved orientation"
    );
    let endpoint = |i: usize, end: usize| -> [f64; 2] {
        let v = &vars[i];
        if i % 2 == 0 {
            let angle = v[3 + end] / number(&objects[i].fields["m_angleCoef"]).unwrap();
            [v[0] + v[2] * angle.cos(), v[1] + v[2] * angle.sin()]
        } else {
            [v[end * 2], v[end * 2 + 1]]
        }
    };
    for i in 0..4 {
        let a = endpoint(i, 1);
        let b = endpoint((i + 1) % 4, 0);
        ensure!(
            (a[0] - b[0]).hypot(a[1] - b[1]) < 1e-8,
            "capsule saved closure"
        );
    }
    // The extrusion recipe and current sketch must describe the same four curves.
    ensure!(curves.len() == 4, "capsule recipe population");
    let mut matched = BTreeSet::new();
    for c in curves {
        let cf = &c["fields"];
        let candidates = (0..4)
            .filter(|&i| {
                if i % 2 == 0 {
                    c["class_name"] == "GArc"
                        && vector(&cf["m_center"]).is_ok_and(|p| {
                            norm(sub(
                                p,
                                add(origin, add(scale(x, vars[i][0]), scale(y, vars[i][1]))),
                            )) < 1e-8
                        })
                        && number(&cf["m_radius"]).is_ok_and(|r| (r - vars[i][2]).abs() < 1e-8)
                        && vector(&cf["m_xVec"]).is_ok_and(|v| norm(sub(v, x)) < 1e-8)
                        && vector(&cf["m_yVec"]).is_ok_and(|v| norm(sub(v, y)) < 1e-8)
                        && (0..2).all(|end| {
                            number(&cf["m_endParams"][end]).is_ok_and(|angle| {
                                (angle
                                    - vars[i][3 + end]
                                        / number(&objects[i].fields["m_angleCoef"]).unwrap())
                                .abs()
                                    < 1e-8
                            })
                        })
                } else {
                    if c["class_name"] != "GLine" {
                        return false;
                    }
                    let Ok(o) = vector(&cf["m_origin"]) else {
                        return false;
                    };
                    let Ok(d) = vector(&cf["m_dirVec"]) else {
                        return false;
                    };
                    (0..2).all(|e| {
                        number(&cf["m_endParams"][e]).is_ok_and(|t| {
                            let p = endpoint(i, e);
                            norm(sub(
                                add(o, scale(d, t)),
                                add(origin, add(scale(x, p[0]), scale(y, p[1]))),
                            )) < 1e-8
                        })
                    })
                }
            })
            .collect::<Vec<_>>();
        ensure!(
            candidates.len() == 1 && matched.insert(candidates[0]),
            "capsule recipe/current mismatch"
        );
    }
    let expected = BTreeSet::from([(1, 1, 0, 2), (2, 1, 1, 2), (3, 1, 2, 2), (3, 2, 0, 1)]);
    let mut pp = BTreeSet::new();
    let mut tangent = BTreeSet::new();
    let mut horizontal = 0;
    let mut sources = Vec::new();
    let active = arr(&f["m_constrRecs"])?;
    ensure!(
        active.len() == 9,
        "capsule requires four explicit tangent locks"
    );
    for p in active {
        let index = target(g, 0, p)?;
        let o = &g.objects[index];
        let v = &o.fields;
        ensure!(empty(&v["m_params"]), "capsule extra constraint variables");
        let es = arr(&v["m_constrElems"])?;
        let subs = arr(&v["m_constrSubTypes"])?;
        let object_indices = es
            .iter()
            .map(|p| {
                objects
                    .iter()
                    .position(|o| Some(o.token as u64) == p["pointer_token"].as_u64())
                    .ok_or_else(|| anyhow::anyhow!("capsule foreign constraint element"))
            })
            .collect::<Result<Vec<_>>>()?;
        match o.class_name.as_str() {
            "VarSketchPPConstrObj" | "VarSketchTangentJoinConstrObj" => {
                ensure!(
                    es.len() == 2 && subs.len() == 2 && v["m_priorityLevel"].as_i64() == Some(0),
                    "capsule join mode"
                );
                let is_tangent = o.class_name == "VarSketchTangentJoinConstrObj";
                if is_tangent {
                    ensure!(
                        v["m_bAngleIsPI"].as_bool() == Some(true),
                        "capsule tangent branch"
                    );
                }
                let subtract = if is_tangent { 20 } else { 0 };
                let key = (
                    object_indices[0],
                    subs[0].as_i64().unwrap_or(-1) - subtract,
                    object_indices[1],
                    subs[1].as_i64().unwrap_or(-1) - subtract,
                );
                ensure!(
                    if is_tangent {
                        tangent.insert(key)
                    } else {
                        pp.insert(key)
                    },
                    "duplicate capsule join"
                );
            }
            "VarSketchHorVerConstrObj" => {
                ensure!(
                    object_indices == vec![1]
                        && subs == &vec![json!(0)]
                        && v["m_hor"].as_bool() == Some(true)
                        && v["m_priorityLevel"].as_i64() == Some(3),
                    "capsule horizontal constraint"
                );
                horizontal += 1;
            }
            _ => anyhow::bail!("unsupported capsule constraint {}", o.class_name),
        }
        sources.push(json!({"object_index":index,"class_name":o.class_name,"fields":v}));
    }
    ensure!(
        pp == expected && tangent == expected && horizontal == 1,
        "capsule complete endpoint/tangency graph"
    );
    let dimids = arr(&f["m_dimIds"])?
        .iter()
        .map(identifier)
        .collect::<Result<BTreeSet<_>>>()?;
    let data = arr(&f["m_dimData"])?
        .iter()
        .map(|v| identifier(&v["first"]))
        .collect::<Result<BTreeSet<_>>>()?;
    ensure!(
        dimids.len() == 5 && data.len() == 4 && data.is_subset(&dimids),
        "capsule dimension population"
    );
    let mut equations = Vec::new();
    let mut seen = BTreeSet::new();
    let mut radius = None;
    let mut dimension_sources = Vec::new();
    let mut center_counts = [0; 2];
    for (dg, source) in dimensions {
        let Some(id) = source["local_element_id"].as_i64() else {
            continue;
        };
        if !dimids.contains(&id) {
            let relevant = dg
                .objects
                .iter()
                .filter_map(|o| o.fields.get("m_geomRef"))
                .any(|v| {
                    identifier(&v["m_elemId"])
                        .is_ok_and(|id| ids.contains(&id) || id == form as i64)
                });
            ensure!(!relevant, "undeclared capsule dimension");
            continue;
        }
        ensure!(seen.insert(id), "duplicate capsule dimension");
        let root = &dg.objects[0];
        let d = &root.fields;
        let segs = arr(&d["m_ArrSegInfo"])?;
        ensure!(
            segs.len() == 1
                && d["m_locked"].as_bool() == Some(false)
                && empty(&d["m_constrInfo"])
                && d["m_useEqualityFormula"].as_bool() == Some(false),
            "capsule dimension extra controls"
        );
        let seg = &segs[0];
        let refs = arc::refs(dg)?;
        if root.class_name == "RadialDim" {
            ensure!(
                refs == vec![(ids[0], 0)]
                    && !data.contains(&id)
                    && seg["m_flags"].as_i64() == Some(0),
                "capsule radius reference"
            );
            let style = identifier(&d["m_styleSymbolId"])?;
            let styles = dimensions
                .iter()
                .filter(|(_, s)| s["local_element_id"].as_i64() == Some(style))
                .collect::<Vec<_>>();
            ensure!(
                styles.len() == 1 && styles[0].0.objects[0].class_name == "DimensionStyle",
                "capsule radial style"
            );
            let style = styles[0].0.objects[0].fields["m_dimensionStyleType"]
                .as_i64()
                .unwrap_or(-1);
            let (value, ps) = resolve(identifier(&seg["m_paramId"])?)?;
            let r = arc::radial_value(style, value)?;
            ensure!(radius.replace(r).is_none(), "multiple capsule radii");
            dimension_sources
                .push(json!({"dimension":source,"parameter":ps,"radius":r,"style_type":style,"style_source":styles[0].1}));
        } else {
            ensure!(
                root.class_name == "Alignment"
                    && data.contains(&id)
                    && refs.len() == 2
                    && seg["m_flags"].as_i64() == Some(1)
                    && identifier(&seg["m_paramId"])? == -1
                    && number(&seg["m_lockedValue"])?.abs() < 1e-12,
                "capsule hard center alignment"
            );
            let centers = refs
                .iter()
                .filter(|(id, tag)| (*id == ids[0] || *id == ids[2]) && *tag == 1)
                .collect::<Vec<_>>();
            ensure!(centers.len() == 1, "capsule center addressing");
            let c = if centers[0].0 == ids[0] { 0 } else { 1 };
            let datums = refs
                .iter()
                .filter(|(id, tag)| !ids.contains(id) && *tag == 0)
                .collect::<Vec<_>>();
            ensure!(datums.len() == 1, "capsule datum addressing");
            let (normal, offset, ds) = arc::datum(datums[0].0, dimensions, origin, x, y)?;
            let dir = vector(&d["m_constrDir"])?;
            ensure!(
                norm(cross(dir, add(scale(x, normal[0]), scale(y, normal[1])))) < 1e-8
                    && (norm(dir) - 1.).abs() < 1e-8,
                "capsule alignment normal"
            );
            equations.push(linear::Equation {
                terms: vec![(2 * c, normal[0]), (2 * c + 1, normal[1])],
                value: offset,
            });
            center_counts[c] += 1;
            dimension_sources.push(json!({"dimension":source,"datum":ds,"normal":normal,"offset":offset,"center_index":c}));
        }
    }
    ensure!(
        seen == dimids && center_counts == [2, 2] && radius.is_some(),
        "capsule radius/anchor coverage"
    );
    let solved = linear::solve(4, &equations)?;
    let centers = [
        [solved.values[0], solved.values[1]],
        [solved.values[2], solved.values[3]],
    ];
    let distance = centers[1][0] - centers[0][0];
    ensure!(
        distance > 1e-8 && (centers[1][1] - centers[0][1]).abs() < 1e-8,
        "capsule centers must lie on constrained horizontal axis"
    );
    let radius = radius.unwrap();
    // Horizontal lower tangent and equal center heights imply both radii are equal.
    // Four tangent/endpoint joins determine the complementary upper segment.
    let tolerance = (radius * 1e-5).min(1e-4);
    let n = (std::f64::consts::PI / (2. * (1. - tolerance / radius).acos())).ceil() as usize;
    ensure!((16..=8192).contains(&n), "capsule tessellation budget");
    let mut polygon = Vec::new();
    let mut worldcenters = Vec::new();
    for (i, c) in centers.iter().enumerate() {
        let center = add(origin, add(scale(x, c[0]), scale(y, c[1])));
        worldcenters.push(center);
        let start = if i == 0 {
            std::f64::consts::FRAC_PI_2
        } else {
            -std::f64::consts::FRAC_PI_2
        };
        for j in 0..=n {
            let a = start + std::f64::consts::PI * j as f64 / n as f64;
            polygon.push(add(
                center,
                add(scale(x, radius * a.cos()), scale(y, radius * a.sin())),
            ));
        }
    }
    Ok((
        polygon,
        json!({"role":"current_profile_nonlinear_constraints","nonlinear":{"kind":"anchored_capsule_tangent_joins","center_rank":solved.rank,"propagated_equal_radius":radius,"center_distance":distance,"branch":"two outward semicircles and four antiparallel endpoint tangents"},"analytic_profile":{"kind":"capsule","center":worldcenters[0],"second_center":worldcenters[1],"basis_x":x,"basis_y":y,"radius":radius,"center_distance":distance,"chord_tolerance_feet":tolerance,"arc_segments":n},"constraints":sources,"dimensions":dimension_sources}),
    ))
}
