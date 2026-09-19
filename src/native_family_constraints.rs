//! Qualified sketch evaluation: explicit linear constraints or uniquely matched
//! saved rectangle solutions. No last/most-used cache or implicit anchor choice.
use super::{number, target, vector};
use crate::{native_metadata::identifier, native_parameters::ObjectGraph};
use anyhow::{Result, ensure};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};
#[path = "native_family_linear.rs"]
pub(super) mod linear;
#[path = "native_family_nonlinear.rs"]
mod nonlinear;
#[path = "native_family_triangle.rs"]
mod triangle;
pub(super) fn reuse(
    sketch: &ObjectGraph,
    form: &ObjectGraph,
    form_id: u64,
    dimensions: &[(ObjectGraph, Value)],
    current: &[[f64; 3]],
    resolve: &mut impl FnMut(i64) -> Result<(f64, Value)>,
) -> Result<(Vec<[f64; 3]>, Value)> {
    if current.len() == 3 {
        return triangle::solve(sketch, form_id, dimensions, current, resolve);
    }
    let root = &sketch.objects[0].fields;
    for object in &sketch.objects {
        ensure!(
            !object.class_name.starts_with("VarSketch")
                || [
                    "VarSketch",
                    "VarSketchLineSegObj",
                    "VarSketchHorVerConstrObj",
                    "VarSketchPPConstrObj",
                    "VarSketchGuess",
                    "VarSketchGuessCache"
                ]
                .contains(&object.class_name.as_str()),
            "unqualified saved solution constraint class {}",
            object.class_name
        );
        ensure!(
            object.class_name != "FamilyParametrizedElemParamsCell",
            "additional sketch associations unsupported"
        );
    }
    ensure!(
        root["m_pPlaneRef"]["pointer_token"].as_u64() == Some(0),
        "externally hosted sketch unsupported"
    );
    ensure!(
        identifier(&root["m_userId"])? == form_id as i64,
        "sketch ownership mismatch"
    );
    for key in [
        "m_pointRecs",
        "m_pointObjIdxMap",
        "m_weakDimIds",
        "m_clineIds",
        "m_customDatumPlanes",
        "m_constrInfo",
    ] {
        ensure!(
            root[key].as_array().is_some_and(Vec::is_empty),
            "unsupported sketch field {key}"
        );
    }
    ensure!(
        root["m_flipXY"].as_bool() == Some(false) && root["m_flipZ"].as_bool() == Some(false),
        "flipped sketch solution unsupported"
    );
    let plane_index = target(sketch, 0, &root["m_oParamPlane"])?;
    let plane = &sketch.objects[plane_index];
    ensure!(
        plane.class_name == "Plane",
        "solution parameter plane class"
    );
    let origin = vector(&plane.fields["m_origin"])?;
    let x = vector(&plane.fields["m_xVec"])?;
    let y = vector(&plane.fields["m_yVec"])?;
    ensure!(
        (super::dot(x, x) - 1.0).abs() < 1e-9
            && (super::dot(y, y) - 1.0).abs() < 1e-9
            && super::dot(x, y).abs() < 1e-9,
        "nonorthonormal solution parameter plane"
    );
    let elems = root["m_elemRecs"]
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("solution elements absent"))?;
    ensure!(
        elems.len() == 4 && current.len() == 4,
        "only four-edge saved solutions qualified"
    );
    let mut saved = Vec::new();
    let mut element_indices = BTreeMap::new();
    let mut curve_ids = BTreeSet::new();
    let mut curve_indices = BTreeMap::new();
    for p in elems {
        let index = target(sketch, 0, p)?;
        let object = &sketch.objects[index];
        element_indices.insert(index, element_indices.len());
        curve_ids.insert(identifier(&object.fields["m_objId"])?);
        curve_indices.insert(
            identifier(&object.fields["m_objId"])?,
            element_indices.len() - 1,
        );
        ensure!(
            object.class_name == "VarSketchLineSegObj"
                && object.fields["m_unbounded"].as_bool() == Some(false),
            "nonlinear/unbounded solution element"
        );
        let params = object.fields["m_params"]
            .as_array()
            .ok_or_else(|| anyhow::anyhow!("line parameters absent"))?;
        ensure!(params.len() == 4, "line parameter count");
        for p in params {
            let parameter = &sketch.objects[target(sketch, index, p)?];
            ensure!(
                parameter.class_name == "VarParam",
                "nonliteral sketch variable"
            );
            saved.push(number(&parameter.fields["m_val"])?);
        }
    }
    let mut axis_constraints = Vec::new();
    let mut point_constraints = Vec::new();
    for object in &sketch.objects {
        if !["VarSketchHorVerConstrObj", "VarSketchPPConstrObj"]
            .contains(&object.class_name.as_str())
        {
            continue;
        }
        let pointers = object.fields["m_constrElems"]
            .as_array()
            .ok_or_else(|| anyhow::anyhow!("constraint elements missing"))?;
        let indices = pointers
            .iter()
            .map(|p| {
                element_indices
                    .get(&{
                        let token = p["pointer_token"]
                            .as_u64()
                            .ok_or_else(|| anyhow::anyhow!("constraint reference token absent"))?;
                        ensure!(
                            token > 0 && token < u32::MAX as u64,
                            "constraint reference must be named object token"
                        );
                        let targets = sketch
                            .objects
                            .iter()
                            .enumerate()
                            .filter(|(_, o)| o.token == token as u32)
                            .map(|(i, _)| i)
                            .collect::<Vec<_>>();
                        ensure!(
                            targets.len() == 1,
                            "constraint named token ambiguous or absent"
                        );
                        targets[0]
                    })
                    .copied()
                    .ok_or_else(|| anyhow::anyhow!("constraint targets external sketch element"))
            })
            .collect::<Result<Vec<_>>>()?;
        let subtypes = object.fields["m_constrSubTypes"]
            .as_array()
            .ok_or_else(|| anyhow::anyhow!("constraint subtypes missing"))?;
        if object.class_name == "VarSketchHorVerConstrObj" {
            ensure!(
                indices.len() == 1 && subtypes == &vec![json!(0)],
                "unqualified axis constraint layout"
            );
            axis_constraints.push((
                indices[0],
                object.fields["m_hor"]
                    .as_bool()
                    .ok_or_else(|| anyhow::anyhow!("axis flag absent"))?,
            ));
        } else {
            ensure!(
                indices.len() == 2 && subtypes.len() == 2,
                "point constraint layout"
            );
            let mut endpoints = Vec::new();
            for (i, t) in indices.iter().zip(subtypes) {
                let endpoint = match t.as_u64() {
                    Some(1) => 0,
                    Some(2) => 2,
                    _ => anyhow::bail!("point subtype unsupported"),
                };
                endpoints.push(i * 4 + endpoint);
            }
            point_constraints.push((endpoints[0], endpoints[1]));
        }
    }
    ensure!(
        axis_constraints.len() == 4 && point_constraints.len() == 4,
        "bounded rectangle constraint population"
    );
    let map = |v: &[f64]| -> Result<Vec<[f64; 3]>> {
        ensure!(
            v.len() == 16 && v.iter().all(|v| v.is_finite()),
            "saved solution variable count/value"
        );
        for (i, horizontal) in &axis_constraints {
            let axis = if *horizontal { 1 } else { 0 };
            ensure!(
                (v[i * 4 + axis] - v[i * 4 + 2 + axis]).abs() < 1e-8,
                "candidate violates current orthogonality constraint"
            );
        }
        for (a, b) in &point_constraints {
            ensure!(
                (v[*a] - v[*b]).abs() < 1e-8 && (v[a + 1] - v[b + 1]).abs() < 1e-8,
                "candidate violates current endpoint constraint"
            );
        }
        for i in 0..4 {
            let n = (i + 1) % 4;
            ensure!(
                (v[i * 4 + 2] - v[n * 4]).abs() < 1e-8
                    && (v[i * 4 + 3] - v[n * 4 + 1]).abs() < 1e-8,
                "saved solution endpoint discontinuity"
            );
        }
        Ok((0..4)
            .map(|i| std::array::from_fn(|a| origin[a] + x[a] * v[i * 4] + y[a] * v[i * 4 + 1]))
            .collect())
    };
    let saved_polygon = map(&saved)?;
    ensure!(
        (0..4).any(|shift| saved_polygon
            .iter()
            .enumerate()
            .all(|(i, a)| super::norm(super::sub(*a, current[(i + shift) % 4])) < 1e-8)),
        "variable order/parameter-plane mapping does not match current recipe"
    );
    let steps = form
        .objects
        .iter()
        .filter(|o| o.class_name == "ExtrusionGStep")
        .collect::<Vec<_>>();
    ensure!(steps.len() == 1, "ambiguous extrusion face history");
    let mut faces = BTreeMap::new();
    for row in steps[0].fields["m_faceHistTable"]
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("face history absent"))?
    {
        let keys = row["m_faceHist"]["m_keys"]
            .as_array()
            .ok_or_else(|| anyhow::anyhow!("face keys absent"))?;
        if keys.len() == 3 && keys[0].as_i64() == Some(3) && keys[2].as_i64() == Some(-1) {
            let i = keys[1]
                .as_u64()
                .ok_or_else(|| anyhow::anyhow!("face edge index absent"))?
                as usize;
            ensure!(i < 4, "face index outside profile");
            ensure!(
                faces
                    .insert(
                        row["m_id"]
                            .as_i64()
                            .ok_or_else(|| anyhow::anyhow!("face tag absent"))?,
                        i
                    )
                    .is_none(),
                "duplicate extrusion face history tag"
            );
        }
    }
    let auto_ids = root["m_dimIds"]
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("sketch dimension IDs missing"))?
        .iter()
        .map(identifier)
        .collect::<Result<BTreeSet<_>>>()?;
    let data_ids = root["m_dimData"]
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("sketch dimension data missing"))?
        .iter()
        .map(|v| identifier(&v["first"]))
        .collect::<Result<BTreeSet<_>>>()?;
    ensure!(
        auto_ids.len() == 4 && auto_ids == data_ids,
        "unqualified auto-dimension population"
    );
    let mut seen_auto = BTreeSet::new();
    let mut anchors = Vec::new();
    let mut constraints = Vec::new();
    let mut sources = Vec::new();
    for (graph, source) in dimensions {
        let refs = graph
            .objects
            .iter()
            .filter_map(|o| o.fields.get("m_geomRef"))
            .collect::<Vec<_>>();
        if !refs.iter().any(|r| {
            identifier(&r["m_elemId"])
                .is_ok_and(|id| id == form_id as i64 || curve_ids.contains(&id))
        }) {
            continue;
        }
        ensure!(
            graph.objects[0].class_name == "LinearDimString"
                && graph.objects[0].fields["m_locked"].as_bool() == Some(false),
            "relevant locked or unrecognized dimension unsupported"
        );
        ensure!(
            graph.objects[0].fields["m_constrInfo"]
                .as_array()
                .is_some_and(Vec::is_empty),
            "additional dimension constraint info unsupported"
        );
        let rows = graph.objects[0].fields["m_ArrSegInfo"]
            .as_array()
            .ok_or_else(|| anyhow::anyhow!("dimension segments absent"))?;
        ensure!(
            rows.len() == 1 && matches!(rows[0]["m_flags"].as_u64(), Some(0 | 1)),
            "unrecognized dimension segment flags"
        );
        if !rows
            .iter()
            .any(|r| identifier(&r["m_paramId"]).is_ok_and(|p| p > 0))
        {
            let id = source["local_element_id"]
                .as_i64()
                .ok_or_else(|| anyhow::anyhow!("auto-dimension identity absent"))?;
            ensure!(
                auto_ids.contains(&id) && seen_auto.insert(id) && refs.len() == 2,
                "unknown/duplicate unlabeled profile dimension"
            );
            let reference_ids = refs
                .iter()
                .map(|r| identifier(&r["m_elemId"]))
                .collect::<Result<Vec<_>>>()?;
            ensure!(
                reference_ids
                    .iter()
                    .filter(|id| curve_ids.contains(id))
                    .count()
                    == 1,
                "auto-dimension must reference one profile curve"
            );
            let datum = reference_ids
                .iter()
                .find(|id| !curve_ids.contains(id))
                .unwrap();
            let datum_graphs = dimensions
                .iter()
                .filter(|(_, s)| s["local_element_id"].as_i64() == Some(*datum))
                .collect::<Vec<_>>();
            ensure!(
                datum_graphs.len() == 1
                    && datum_graphs[0].0.objects[0].class_name == "RefPlane"
                    && datum_graphs[0].0.objects[0].fields["m_locked"].as_bool() == Some(true),
                "auto-dimension datum context unqualified"
            );
            if rows[0]["m_flags"].as_u64() == Some(1) {
                ensure!(
                    number(&rows[0]["m_lockedValue"])? == 0.0,
                    "nonzero locked datum distance not qualified"
                );
                let curve_ref = refs
                    .iter()
                    .find(|r| identifier(&r["m_elemId"]).is_ok_and(|id| curve_ids.contains(&id)))
                    .unwrap();
                ensure!(
                    curve_ref["m_geomTag"].as_i64() == Some(0)
                        && curve_ref["m_famMemberIdx"].as_i64() == Some(-1),
                    "locked curve reference scope"
                );
                let endpoint = curve_ref["m_subTag"]
                    .as_u64()
                    .ok_or_else(|| anyhow::anyhow!("locked curve endpoint absent"))?
                    as usize;
                ensure!(endpoint < 2, "locked reference is not curve endpoint");
                let vertex = (curve_indices[&identifier(&curve_ref["m_elemId"])?] + endpoint) % 4;
                let dg = &datum_graphs[0].0;
                let df = &dg.objects[0].fields;
                let surface = &dg.objects[target(dg, 0, &df["m_pSurface"])?];
                ensure!(surface.class_name == "Plane", "locked datum is not planar");
                let normal = super::cross(
                    vector(&surface.fields["m_xVec"])?,
                    vector(&surface.fields["m_yVec"])?,
                );
                ensure!(
                    (super::norm(normal) - 1.0).abs() < 1e-8,
                    "locked datum normal nonunit"
                );
                let distance = super::dot(normal, vector(&surface.fields["m_origin"])?);
                anchors.push((vertex,(normal,distance),json!({"dimension":source,"segment_flags":1,"locked_value":0.0,"datum":datum_graphs[0].1,"curve_id":identifier(&curve_ref["m_elemId"] )?,"endpoint":endpoint})));
            }
            continue;
        }
        ensure!(
            rows[0]["m_flags"].as_u64() == Some(0),
            "locked labeled dimension not qualified"
        );
        ensure!(
            rows.len() == 1 && refs.len() == 2,
            "multisegment labeled dimension unsupported"
        );
        let parameter = identifier(&rows[0]["m_paramId"])?;
        let (value, value_source) = resolve(parameter)?;
        ensure!(value > 0.0, "nonpositive labeled distance");
        let indices = refs
            .iter()
            .map(|r| -> Result<usize> {
                ensure!(
                    identifier(&r["m_elemId"])? == form_id as i64
                        && r["m_famMemberIdx"].as_i64() == Some(-1)
                        && r["m_subTag"].as_i64() == Some(-1),
                    "external/subentity dimension reference unsupported"
                );
                faces
                    .get(
                        &r["m_geomTag"]
                            .as_i64()
                            .ok_or_else(|| anyhow::anyhow!("dimension face tag absent"))?,
                    )
                    .copied()
                    .ok_or_else(|| anyhow::anyhow!("dimension face history unresolved"))
            })
            .collect::<Result<Vec<_>>>()?;
        ensure!(
            (indices[0] + 2) % 4 == indices[1],
            "dimension faces not opposite profile edges"
        );
        constraints.push((indices[0], indices[1], value));
        sources.push(json!({"dimension":source,"parameter_local_id":parameter,"parameter_value":value_source,"face_profile_indices":indices,"distance":value}));
    }
    ensure!(
        seen_auto == auto_ids,
        "missing current auto-dimension evidence"
    );
    ensure!(
        constraints.len() == 2 && constraints[0].0 % 2 != constraints[1].0 % 2,
        "two independent labeled profile dimensions required"
    );
    if !anchors.is_empty() {
        ensure!(
            anchors.len() == 2 && anchors[0].0 == anchors[1].0,
            "two hard datum constraints on one profile vertex required"
        );
        let normal = super::cross(x, y);
        let mut lengths = [0.0; 2];
        for (edge, _, distance) in &constraints {
            lengths[(edge + 1) % 2] = *distance;
        }
        let polygon = anchored_rectangle(
            &saved_polygon,
            anchors[0].0,
            [
                anchors[0].1,
                anchors[1].1,
                (normal, super::dot(normal, saved_polygon[0])),
            ],
            lengths,
        )?;
        return Ok((
            polygon,
            json!({"role":"evaluated_hard_datum_rectangle","constraints":sources,"anchors":anchors.iter().map(|a|&a.2).collect::<Vec<_>>(),"parameter_plane_object":plane_index,"method":"intersection of two independently locked zero-distance datum planes and sketch plane, plus two labeled opposite-face distances; no saved guess required"}),
        ));
    }
    let cache_index = target(sketch, 0, &root["m_oGuessCache"])?;
    let cache = &sketch.objects[cache_index];
    ensure!(
        cache.class_name == "VarSketchGuessCache" && cache.fields["m_nPar"].as_u64() == Some(16),
        "solution cache layout"
    );
    let mut candidates = vec![(
        saved_polygon,
        json!({"role":"current_varparams","objects":element_indices.keys().collect::<Vec<_>>()}),
    )];
    for pointer in cache.fields["m_guessArr"]
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("solution candidates absent"))?
    {
        let index = target(sketch, cache_index, pointer)?;
        let object = &sketch.objects[index];
        ensure!(
            object.class_name == "VarSketchGuess",
            "solution candidate class"
        );
        let values = object.fields["m_values"]
            .as_array()
            .ok_or_else(|| anyhow::anyhow!("solution values absent"))?
            .iter()
            .map(number)
            .collect::<Result<Vec<_>>>()?;
        let polygon = map(&values)?;
        candidates.push((polygon, json!({"role":"saved_guess","object":index})));
    }
    let mut matches: Vec<(Vec<[f64; 3]>, Vec<Value>)> = Vec::new();
    for (polygon, locator) in candidates {
        if constraints.iter().all(|(a, b, d)| {
            let edge = super::sub(polygon[(*a + 1) % 4], polygon[*a]);
            let other = super::sub(polygon[(*b + 1) % 4], polygon[*b]);
            let len = super::norm(edge);
            len > 1e-9
                && super::norm(super::cross(edge, other)) < 1e-8
                && ((super::norm(super::cross(edge, super::sub(polygon[*b], polygon[*a]))) / len)
                    - d)
                    .abs()
                    < 1e-8
        }) {
            if let Some((_, locators)) = matches.iter_mut().find(|(p, _)| p == &polygon) {
                locators.push(locator);
            } else {
                matches.push((polygon, vec![locator]));
            }
        }
    }
    ensure!(
        matches.len() == 1,
        "saved constraint solution absent or ambiguous ({} candidates)",
        matches.len()
    );
    let (polygon, locators) = matches.pop().unwrap();
    Ok((
        polygon,
        json!({"role":"uniquely_constraint_qualified_saved_solution","solution_locators":locators,"cache_object":cache_index,"parameter_plane_object":plane_index,"constraints":sources,"anchoring_semantics":"serialized candidate positions preserved; no new underconstrained placement solved","selection_rule":"exactly one native candidate satisfies both labeled opposite-face distances; no order or usecount preference"}),
    ))
}

fn anchored_rectangle(
    current: &[[f64; 3]],
    anchor_vertex: usize,
    planes: [([f64; 3], f64); 3],
    edge_lengths: [f64; 2],
) -> Result<Vec<[f64; 3]>> {
    ensure!(
        current.len() == 4 && anchor_vertex < 4,
        "anchored rectangle vertex scope"
    );
    ensure!(
        edge_lengths.iter().all(|v| v.is_finite() && *v > 0.0),
        "invalid driven rectangle length"
    );
    let [a, b, c] = planes;
    let determinant = super::dot(a.0, super::cross(b.0, c.0));
    ensure!(
        determinant.is_finite() && determinant.abs() > 1e-8,
        "datum constraints do not establish unique anchor"
    );
    let anchor = std::array::from_fn(|i| {
        (super::cross(b.0, c.0)[i] * a.1
            + super::cross(c.0, a.0)[i] * b.1
            + super::cross(a.0, b.0)[i] * c.1)
            / determinant
    });
    let edge0 = super::sub(current[1], current[0]);
    let edge1 = super::sub(current[2], current[1]);
    let l0 = super::norm(edge0);
    let l1 = super::norm(edge1);
    ensure!(l0 > 1e-8 && l1 > 1e-8, "degenerate rectangle");
    let u = super::scale(edge0, 1.0 / l0);
    let v = super::scale(edge1, 1.0 / l1);
    ensure!(
        super::dot(u, v).abs() < 1e-8
            && super::norm(super::add(super::sub(current[3], current[2]), edge0)) < 1e-8
            && super::norm(super::add(super::sub(current[0], current[3]), edge1)) < 1e-8,
        "profile is not an oriented rectangle"
    );
    let e0 = super::scale(u, edge_lengths[0]);
    let e1 = super::scale(v, edge_lengths[1]);
    let offsets = [[0.0; 3], e0, super::add(e0, e1), e1];
    let base = super::sub(anchor, offsets[anchor_vertex]);
    Ok(offsets.iter().map(|p| super::add(base, *p)).collect())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn independent_hard_datums_determine_absolute_rectangle_without_cache() {
        let current = vec![[0., 0., 0.], [4., 0., 0.], [4., 2., 0.], [0., 2., 0.]];
        let out = anchored_rectangle(
            &current,
            0,
            [([1., 0., 0.], 3.), ([0., 1., 0.], -2.), ([0., 0., 1.], 5.)],
            [7., 8.],
        )
        .unwrap();
        assert_eq!(
            out,
            vec![[3., -2., 5.], [10., -2., 5.], [10., 6., 5.], [3., 6., 5.]]
        );
        assert!(
            anchored_rectangle(
                &current,
                0,
                [([1., 0., 0.], 3.), ([1., 0., 0.], 3.), ([0., 0., 1.], 5.)],
                [7., 8.]
            )
            .is_err()
        );
    }
}
