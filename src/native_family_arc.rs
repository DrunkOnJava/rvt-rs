//! Anchored semicircular extrusion profile from current radius and constraints.
use super::constraints::linear;
use super::{add, cross, dot, norm, number, scale, sub, target, vector};
use crate::{native_metadata::identifier, native_parameters::ObjectGraph};
use anyhow::{Result, ensure};
use serde_json::{Value, json};
use std::collections::BTreeSet;
pub(super) fn arr(v: &Value) -> Result<&Vec<Value>> {
    v.as_array()
        .ok_or_else(|| anyhow::anyhow!("arc array missing"))
}
pub(super) fn empty(v: &Value) -> bool {
    v.as_array().is_some_and(Vec::is_empty)
}
pub(super) fn values(g: &ObjectGraph, index: usize, count: usize) -> Result<Vec<f64>> {
    let p = arr(&g.objects[index].fields["m_params"])?;
    ensure!(p.len() == count, "arc variable population");
    p.iter()
        .map(|p| {
            let v = &g.objects[target(g, index, p)?];
            ensure!(v.class_name == "VarParam", "arc variable class");
            number(&v.fields["m_val"])
        })
        .collect()
}
pub(super) fn refs(g: &ObjectGraph) -> Result<Vec<(i64, i64)>> {
    arr(&g.objects[0].fields["m_witnessRefs"])?
        .iter()
        .map(|p| {
            let o = &g.objects[target(g, 0, &p["m_pWitnessRef"])?];
            ensure!(
                matches!(o.class_name.as_str(), "GeomSegInPlaneRef" | "ArcRef"),
                "arc witness class"
            );
            let f = &o.fields["m_geomRef"];
            ensure!(
                f["m_flags"].as_i64() == Some(0)
                    && f["m_isLazyRef"].as_bool() == Some(false)
                    && f["m_famMemberIdx"].as_i64() == Some(-1)
                    && f["m_foreignElemIdRef"]["m_id64"].as_i64() == Some(-1)
                    && f["m_oNextRef"]["pointer_token"].as_u64() == Some(0)
                    && empty(&f["m_intermediateTags"])
                    && f["m_subTag"].as_i64() == Some(-1),
                "unqualified arc witness addressing"
            );
            Ok((
                identifier(&f["m_elemId"])?,
                f["m_geomTag"]
                    .as_i64()
                    .ok_or_else(|| anyhow::anyhow!("arc witness tag"))?,
            ))
        })
        .collect()
}
pub(super) fn datum(
    id: i64,
    dimensions: &[(ObjectGraph, Value)],
    origin: [f64; 3],
    x: [f64; 3],
    y: [f64; 3],
) -> Result<([f64; 2], f64, Value)> {
    let found = dimensions
        .iter()
        .filter(|(_, s)| s["local_element_id"].as_i64() == Some(id))
        .collect::<Vec<_>>();
    ensure!(found.len() == 1, "current arc datum unresolved");
    let (g, source) = found[0];
    let f = &g.objects[0].fields;
    ensure!(
        g.objects[0].class_name == "CurveElem"
            && f["m_locked"].as_bool() == Some(true)
            && f["m_referenceType"].as_i64() == Some(1)
            && f["m_detail"].as_bool() == Some(false)
            && f["m_useOffsetPos"].as_bool() == Some(false)
            && empty(&f["m_constrInfo"]),
        "arc datum must be pinned reference line"
    );
    ensure!(
        !g.objects
            .iter()
            .any(|o| o.class_name == "FamilyParametrizedElemParamsCell"),
        "parameterized arc datum unsupported"
    );
    let di = target(g, 0, &f["m_pCurveDriver"])?;
    let driver = &g.objects[di];
    ensure!(
        driver.class_name == "CurveElemDriver"
            && driver.fields["m_oControlData"]["pointer_token"].as_u64() == Some(0)
            && [
                "m_controlJoinsSet",
                "m_sideJoins",
                "m_sideAlignments",
                "m_midParams"
            ]
            .iter()
            .all(|k| empty(&driver.fields[*k])),
        "arc datum driver modified"
    );
    let ci = target(g, di, &driver.fields["m_pCrv"])?;
    let curve = &g.objects[ci];
    ensure!(curve.class_name == "GLine", "arc datum must be linear");
    let p = vector(&curve.fields["m_origin"])?;
    let d = vector(&curve.fields["m_dirVec"])?;
    ensure!(
        (norm(d) - 1.).abs() < 1e-9
            && dot(d, cross(x, y)).abs() < 1e-9
            && dot(sub(p, origin), cross(x, y)).abs() < 1e-8,
        "arc datum outside parameter plane"
    );
    let normal = [-dot(d, y), dot(d, x)];
    let offset = dot(sub(p, origin), x) * normal[0] + dot(sub(p, origin), y) * normal[1];
    Ok((
        normal,
        offset,
        json!({"datum":source,"curve_object":ci,"driver_object":di,"fields":"pinned CurveElem.referenceType; CurveElemDriver.m_pCrv -> GLine"}),
    ))
}
pub(super) fn solve(
    sketch: &ObjectGraph,
    form_id: u64,
    dimensions: &[(ObjectGraph, Value)],
    curves: &[Value],
    resolve: &mut impl FnMut(i64) -> Result<(f64, Value)>,
) -> Result<(Vec<[f64; 3]>, Value)> {
    let f = &sketch.objects[0].fields;
    ensure!(
        identifier(&f["m_userId"])? == form_id as i64,
        "arc sketch owner mismatch"
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
        "additional arc sketch controls unsupported"
    );
    for o in &sketch.objects {
        ensure!(
            !o.class_name.starts_with("VarSketch")
                || [
                    "VarSketch",
                    "VarSketchArcObj",
                    "VarSketchLineSegObj",
                    "VarSketchArcEndAngleConstrObj",
                    "VarSketchPPConstrObj",
                    "VarSketchGuess",
                    "VarSketchGuessCache"
                ]
                .contains(&o.class_name.as_str()),
            "unqualified arc constraint class {}",
            o.class_name
        );
    }
    let pi = target(sketch, 0, &f["m_oParamPlane"])?;
    let p = &sketch.objects[pi];
    ensure!(p.class_name == "Plane", "arc parameter plane class");
    let origin = vector(&p.fields["m_origin"])?;
    let x = vector(&p.fields["m_xVec"])?;
    let y = vector(&p.fields["m_yVec"])?;
    ensure!(
        (norm(x) - 1.).abs() < 1e-9 && (norm(y) - 1.).abs() < 1e-9 && dot(x, y).abs() < 1e-9,
        "arc parameter basis"
    );
    let elems = arr(&f["m_elemRecs"])?;
    ensure!(
        elems.len() == 2,
        "semicircle profile needs arc and diameter"
    );
    let ai = target(sketch, 0, &elems[0])?;
    let li = target(sketch, 0, &elems[1])?;
    let arc = &sketch.objects[ai];
    let line = &sketch.objects[li];
    ensure!(
        arc.class_name == "VarSketchArcObj"
            && line.class_name == "VarSketchLineSegObj"
            && arc.fields["m_flipped"].as_bool() == Some(false)
            && arc.fields["m_unbounded"].as_bool() == Some(false)
            && line.fields["m_unbounded"].as_bool() == Some(false),
        "qualified arc/diameter order and orientation"
    );
    let aid = identifier(&arc.fields["m_objId"])?;
    let lid = identifier(&line.fields["m_objId"])?;
    let a = values(sketch, ai, 5)?;
    let l = values(sketch, li, 4)?;
    let coef = number(&arc.fields["m_angleCoef"])?;
    ensure!(
        a[2] > 0.
            && coef > 0.
            && (a[3] / coef).abs() < 1e-9
            && (a[4] / coef - std::f64::consts::PI).abs() < 1e-9,
        "saved semicircle angular branch"
    );
    ensure!(
        (l[0] - (a[0] - a[2])).abs() < 1e-8
            && (l[1] - a[1]).abs() < 1e-8
            && (l[2] - (a[0] + a[2])).abs() < 1e-8
            && (l[3] - a[1]).abs() < 1e-8,
        "saved diameter does not close arc"
    );
    ensure!(
        curves.len() == 2
            && curves.iter().filter(|c| c["class_name"] == "GArc").count() == 1
            && curves.iter().filter(|c| c["class_name"] == "GLine").count() == 1,
        "saved form profile class population"
    );
    let c = &curves.iter().find(|c| c["class_name"] == "GArc").unwrap()["fields"];
    let saved_line = &curves.iter().find(|c| c["class_name"] == "GLine").unwrap()["fields"];
    let lo = vector(&saved_line["m_origin"])?;
    let ld = vector(&saved_line["m_dirVec"])?;
    for (index, coords) in [[l[0], l[1]], [l[2], l[3]]].iter().enumerate() {
        ensure!(
            norm(sub(
                add(lo, scale(ld, number(&saved_line["m_endParams"][index])?)),
                add(origin, add(scale(x, coords[0]), scale(y, coords[1])))
            )) < 1e-8,
            "saved line differs from current sketch"
        );
    }
    let saved_center = add(origin, add(scale(x, a[0]), scale(y, a[1])));
    ensure!(
        norm(sub(vector(&c["m_center"])?, saved_center)) < 1e-8
            && (number(&c["m_radius"])? - a[2]).abs() < 1e-8
            && norm(sub(vector(&c["m_xVec"])?, x)) < 1e-8
            && norm(sub(vector(&c["m_yVec"])?, y)) < 1e-8
            && number(&c["m_endParams"][0])?.abs() < 1e-9
            && (number(&c["m_endParams"][1])? - std::f64::consts::PI).abs() < 1e-9,
        "saved arc recipe differs from current sketch variables"
    );
    let active = arr(&f["m_constrRecs"])?;
    ensure!(active.len() == 3, "semicircle constraint population");
    let mut start_count = 0;
    let mut connections = BTreeSet::new();
    let mut constraint_sources = Vec::new();
    for pointer in active {
        let i = target(sketch, 0, pointer)?;
        let o = &sketch.objects[i];
        let v = &o.fields;
        ensure!(
            empty(&v["m_params"]),
            "arc constraint variable controls unsupported"
        );
        let es = arr(&v["m_constrElems"])?;
        let subtypes = arr(&v["m_constrSubTypes"])?;
        if o.class_name == "VarSketchArcEndAngleConstrObj" {
            ensure!(
                es.len() == 1
                    && subtypes == &vec![json!(0)]
                    && es[0]["pointer_token"].as_u64() == Some(arc.token as u64)
                    && v["m_end"].as_i64() == Some(0)
                    && number(&v["m_angle"])?.abs() < 1e-12
                    && v["m_priorityLevel"].as_i64() == Some(3),
                "arc start angle constraint profile"
            );
            start_count += 1;
        } else {
            ensure!(
                o.class_name == "VarSketchPPConstrObj"
                    && es.len() == 2
                    && subtypes.len() == 2
                    && es[0]["pointer_token"].as_u64() == Some(line.token as u64)
                    && es[1]["pointer_token"].as_u64() == Some(arc.token as u64)
                    && v["m_priorityLevel"].as_i64() == Some(0),
                "arc endpoint coincidence profile"
            );
            connections.insert((subtypes[0].as_i64(), subtypes[1].as_i64()));
        }
        constraint_sources.push(json!({"object_index":i,"class_name":o.class_name,"fields":v}));
    }
    ensure!(
        start_count == 1 && connections == BTreeSet::from([(Some(1), Some(2)), (Some(2), Some(1))]),
        "arc endpoint closure constraints incomplete"
    );
    let ids = arr(&f["m_dimIds"])?
        .iter()
        .map(identifier)
        .collect::<Result<BTreeSet<_>>>()?;
    let data = arr(&f["m_dimData"])?
        .iter()
        .map(|v| identifier(&v["first"]))
        .collect::<Result<BTreeSet<_>>>()?;
    ensure!(
        ids.len() == 4 && data.len() == 3 && data.is_subset(&ids),
        "semicircle dimension population"
    );
    let mut radius = None;
    let mut radial_id = None;
    let mut equations = Vec::new();
    let mut diameter = None;
    let mut seen = BTreeSet::new();
    let mut dimension_sources = Vec::new();
    for (g, source) in dimensions {
        let Some(id) = source["local_element_id"].as_i64() else {
            continue;
        };
        if !ids.contains(&id) {
            let relevant = g
                .objects
                .iter()
                .filter_map(|o| o.fields.get("m_geomRef"))
                .any(|reference| {
                    identifier(&reference["m_elemId"])
                        .is_ok_and(|owner| owner == aid || owner == lid || owner == form_id as i64)
                });
            ensure!(
                !relevant,
                "additional undeclared arc dimension requires solving"
            );
            continue;
        }
        ensure!(seen.insert(id), "duplicate arc dimension");
        let root = &g.objects[0];
        let d = &root.fields;
        let segs = arr(&d["m_ArrSegInfo"])?;
        ensure!(
            segs.len() == 1
                && d["m_locked"].as_bool() == Some(false)
                && empty(&d["m_constrInfo"])
                && d["m_useEqualityFormula"].as_bool() == Some(false),
            "arc dimension additional controls"
        );
        let seg = &segs[0];
        let witnesses = refs(g)?;
        if root.class_name == "RadialDim" {
            ensure!(
                seg["m_flags"].as_i64() == Some(0)
                    && witnesses == vec![(aid, 0)]
                    && !data.contains(&id),
                "radius dimension addressing"
            );
            let style = identifier(&d["m_styleSymbolId"])?;
            let styles = dimensions
                .iter()
                .filter(|(_, s)| s["local_element_id"].as_i64() == Some(style))
                .collect::<Vec<_>>();
            ensure!(
                styles.len() == 1 && styles[0].0.objects[0].class_name == "DimensionStyle",
                "radius dimension style not qualified"
            );
            let style_type = styles[0].0.objects[0].fields["m_dimensionStyleType"]
                .as_i64()
                .ok_or_else(|| anyhow::anyhow!("radial style discriminator missing"))?;
            let (value, ps) = resolve(identifier(&seg["m_paramId"])?)?;
            let r = radial_value(style_type, value)?;
            ensure!(
                r.is_finite() && r > 0. && radius.replace(r).is_none(),
                "positive unique radius required"
            );
            radial_id = Some(id);
            dimension_sources.push(json!({"dimension":source,"parameter":ps,"style":styles[0].1,"radius_feet":r,"style_type":style_type,"parameter_value":value,"rule":"qualified DimensionStyle 2 radius / 9 diameter"}));
        } else {
            ensure!(
                root.class_name == "Alignment"
                    && seg["m_flags"].as_i64() == Some(1)
                    && number(&seg["m_lockedValue"])?.abs() < 1e-12
                    && identifier(&seg["m_paramId"])? == -1
                    && witnesses.len() == 2
                    && data.contains(&id),
                "arc alignment mode"
            );
            let subject = witnesses
                .iter()
                .filter(|(owner, _)| *owner == aid || *owner == lid)
                .copied()
                .collect::<Vec<_>>();
            let datums = witnesses
                .iter()
                .filter(|(owner, _)| *owner != aid && *owner != lid)
                .copied()
                .collect::<Vec<_>>();
            ensure!(
                subject.len() == 1 && datums.len() == 1 && datums[0].1 == 0,
                "arc alignment endpoints"
            );
            let (normal, offset, ds) = datum(datums[0].0, dimensions, origin, x, y)?;
            let dir = vector(&d["m_constrDir"])?;
            let dn = add(scale(x, normal[0]), scale(y, normal[1]));
            ensure!(
                (dot(dir, dn).abs() - 1.).abs() < 1e-8,
                "arc alignment direction differs from datum normal"
            );
            if subject[0] == (aid, 1) {
                equations.push(linear::Equation {
                    terms: vec![(0, normal[0]), (1, normal[1])],
                    value: offset,
                });
            } else {
                ensure!(
                    subject[0] == (lid, 0) && diameter.replace((normal, offset)).is_none(),
                    "diameter alignment unique"
                );
            }
            dimension_sources.push(json!({"dimension":source,"datum":ds,"normal":normal,"offset":offset,"subject":subject[0]}));
        }
    }
    ensure!(
        seen == ids && radial_id.is_some() && equations.len() == 2,
        "radius/center dimension coverage incomplete"
    );
    let solved = linear::solve(2, &equations)?;
    let (dn, dd) = diameter.ok_or_else(|| anyhow::anyhow!("diameter alignment absent"))?;
    ensure!(
        dn[0].abs() < 1e-8
            && (dn[1].abs() - 1.).abs() < 1e-8
            && (dn[0] * solved.values[0] + dn[1] * solved.values[1] - dd).abs() < 1e-8,
        "diameter not through constrained center on start-angle axis"
    );
    let radius = radius.unwrap();
    let center = add(
        origin,
        add(scale(x, solved.values[0]), scale(y, solved.values[1])),
    );
    let chord_tolerance = (radius * 1e-5).min(1e-4);
    let n = (std::f64::consts::PI / (2. * (1. - chord_tolerance / radius).acos())).ceil() as usize;
    ensure!((16..=8192).contains(&n), "arc tessellation budget");
    let polygon = (0..=n)
        .map(|i| {
            let t = std::f64::consts::PI * i as f64 / n as f64;
            add(
                center,
                add(scale(x, radius * t.cos()), scale(y, radius * t.sin())),
            )
        })
        .collect();
    Ok((
        polygon,
        json!({"role":"current_profile_nonlinear_constraints","nonlinear":{"kind":"anchored_semicircle_radius","radius_feet":radius,"center":center,"basis_x":x,"basis_y":y,"start_angle_radians":0.,"end_angle_radians":std::f64::consts::PI,"center_rank":solved.rank,"diameter_alignment":true,"chord_tolerance_feet":chord_tolerance,"arc_segments":n},"analytic_profile":{"kind":"semicircle","center":center,"basis_x":x,"basis_y":y,"radius":radius,"sweep_radians":std::f64::consts::PI,"chord_tolerance_feet":chord_tolerance,"arc_segments":n},"constraints":constraint_sources,"dimensions":dimension_sources,"saved_arc_angle_coefficient":coef,"placement_rule":"two independently pinned reference-line center alignments; explicit radius and diameter/start-angle constraints; saved profile only verifies branch"}),
    ))
}

pub(super) fn radial_value(style: i64, value: f64) -> Result<f64> {
    ensure!(
        value.is_finite() && value > 0.,
        "positive radial value required"
    );
    match style {
        2 => Ok(value),
        9 => Ok(value / 2.),
        _ => anyhow::bail!("unqualified radial style {style}"),
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn pinned_datum_with_unmodeled_control_payload_is_refused() {
        use crate::native_parameters::{GraphEdge, GraphObject};
        let object = |class: &str, token, fields| GraphObject {
            class_tag: 1,
            class_name: class.into(),
            token,
            start: 0,
            fields_end: 0,
            fields,
        };
        let graph = ObjectGraph {
            consumed_bytes: 0,
            objects: vec![
                object(
                    "CurveElem",
                    1,
                    json!({"m_locked":true,"m_referenceType":1,"m_detail":false,"m_useOffsetPos":false,"m_constrInfo":[],"m_pCurveDriver":{"pointer_token":2,"offset":10}}),
                ),
                object(
                    "CurveElemDriver",
                    2,
                    json!({"m_oControlData":{"pointer_token":3},"m_controlJoinsSet":[],"m_sideJoins":[],"m_sideAlignments":[],"m_midParams":[]}),
                ),
            ],
            edges: vec![GraphEdge {
                source_object_index: 0,
                pointer_offset: 10,
                pointer_token: 2,
                target_object_index: 1,
                target_class_tag: 1,
            }],
        };
        let error = datum(
            10,
            &[(graph, json!({"local_element_id":10}))],
            [0.; 3],
            [1., 0., 0.],
            [0., 1., 0.],
        )
        .unwrap_err();
        assert!(error.to_string().contains("datum driver modified"));
    }
    #[test]
    fn saved_datum_coordinates_do_not_replace_a_missing_pin() {
        let graph = ObjectGraph {
            consumed_bytes: 0,
            objects: vec![crate::native_parameters::GraphObject {
                class_tag: 1,
                class_name: "CurveElem".into(),
                token: 1,
                start: 0,
                fields_end: 0,
                fields: json!({"m_locked":false,"m_referenceType":1,"m_detail":false,"m_useOffsetPos":false,"m_constrInfo":[]}),
            }],
            edges: vec![],
        };
        let error = datum(
            10,
            &[(graph, json!({"local_element_id":10}))],
            [0.; 3],
            [1., 0., 0.],
            [0., 1., 0.],
        )
        .unwrap_err();
        assert!(error.to_string().contains("pinned reference line"));
    }
    #[test]
    fn radius_and_diameter_are_distinct_saved_styles() {
        assert_eq!(radial_value(2, 4.5).unwrap(), 4.5);
        assert_eq!(radial_value(9, 9.).unwrap(), 4.5);
        assert_eq!(radial_value(9, 10.25).unwrap(), 5.125);
        assert!(radial_value(0, 9.).is_err());
        assert!(radial_value(9, 0.).is_err());
        assert!(radial_value(2, f64::NAN).is_err());
    }
}
