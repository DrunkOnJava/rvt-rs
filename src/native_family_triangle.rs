//! Qualified endpoint-dimension linear solve for a three-line sketch. The saved
//! orientation branch is retained; absolute translation requires authored locks.
use super::super::{add, cross, dot, norm, scale, sub};
use super::{linear, nonlinear, number, target, vector};
use crate::{native_metadata::identifier, native_parameters::ObjectGraph};
use anyhow::{Result, ensure};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};

pub(super) fn solve(
    sketch: &ObjectGraph,
    form_id: u64,
    dimensions: &[(ObjectGraph, Value)],
    current: &[[f64; 3]],
    resolve: &mut impl FnMut(i64) -> Result<(f64, Value)>,
) -> Result<(Vec<[f64; 3]>, Value)> {
    let root = &sketch.objects[0].fields;
    ensure!(
        identifier(&root["m_userId"])? == form_id as i64,
        "linear sketch ownership mismatch"
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
            "unqualified linear sketch field {key}"
        );
    }
    ensure!(
        root["m_pPlaneRef"]["pointer_token"].as_u64() == Some(0)
            && root["m_flipXY"].as_bool() == Some(false)
            && root["m_flipZ"].as_bool() == Some(false),
        "hosted/flipped linear sketch unqualified"
    );
    for o in &sketch.objects {
        ensure!(
            !o.class_name.starts_with("VarSketch")
                || [
                    "VarSketch",
                    "VarSketchLineSegObj",
                    "VarSketchHorVerConstrObj",
                    "VarSketchPPConstrObj",
                    "VarSketchGuess",
                    "VarSketchGuessCache"
                ]
                .contains(&o.class_name.as_str()),
            "unqualified linear constraint class {}",
            o.class_name
        );
        ensure!(
            o.class_name != "FamilyParametrizedElemParamsCell",
            "additional sketch associations unsupported"
        );
    }
    let plane_index = target(sketch, 0, &root["m_oParamPlane"])?;
    let plane = &sketch.objects[plane_index];
    ensure!(
        plane.class_name == "Plane",
        "linear sketch parameter plane class"
    );
    let origin = vector(&plane.fields["m_origin"])?;
    let x = vector(&plane.fields["m_xVec"])?;
    let y = vector(&plane.fields["m_yVec"])?;
    ensure!(
        (dot(x, x) - 1.).abs() < 1e-9 && (dot(y, y) - 1.).abs() < 1e-9 && dot(x, y).abs() < 1e-9,
        "nonorthonormal linear sketch plane"
    );
    let elems = root["m_elemRecs"]
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("linear elements absent"))?;
    ensure!(
        elems.len() == 3 && current.len() == 3,
        "qualified linear profile requires three lines"
    );
    let mut variables = BTreeMap::new();
    let mut saved = Vec::new();
    let mut lines = Vec::new();
    let mut curve_lines = BTreeMap::new();
    let mut tokens = BTreeMap::new();
    for pointer in elems {
        let index = target(sketch, 0, pointer)?;
        let o = &sketch.objects[index];
        ensure!(
            o.class_name == "VarSketchLineSegObj"
                && o.fields["m_unbounded"].as_bool() == Some(false),
            "linear profile object class"
        );
        ensure!(
            curve_lines
                .insert(identifier(&o.fields["m_objId"])?, lines.len())
                .is_none(),
            "duplicate profile curve ID"
        );
        ensure!(
            o.token > 0 && o.token < u32::MAX && tokens.insert(o.token, lines.len()).is_none(),
            "ambiguous line token"
        );
        let parameters = o.fields["m_params"]
            .as_array()
            .ok_or_else(|| anyhow::anyhow!("line variables absent"))?;
        ensure!(parameters.len() == 4, "line variable count");
        let mut indices = [0usize; 4];
        for (coordinate, p) in parameters.iter().enumerate() {
            let parameter = target(sketch, index, p)?;
            let value = &sketch.objects[parameter];
            ensure!(value.class_name == "VarParam", "nonliteral sketch variable");
            let next = variables.len();
            let variable = *variables.entry(parameter).or_insert(next);
            if variable == saved.len() {
                saved.push(number(&value.fields["m_val"])?);
            }
            indices[coordinate] = variable;
        }
        lines.push(indices);
    }
    let point = |values: &[f64], line: usize, end: usize| {
        add(
            origin,
            add(
                scale(x, values[lines[line][end * 2]]),
                scale(y, values[lines[line][end * 2 + 1]]),
            ),
        )
    };
    for i in 0..3 {
        ensure!(
            norm(sub(point(&saved, i, 1), point(&saved, (i + 1) % 3, 0))) < 1e-8,
            "linear profile ordering is not closed"
        );
    }
    ensure!(
        (0..3).any(|offset| (0..3)
            .all(|i| norm(sub(point(&saved, i, 0), current[(i + offset) % 3])) < 1e-8)),
        "saved line variables disagree with current profile"
    );
    let active_constraints = root["m_constrRecs"]
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("active sketch constraints absent"))?
        .iter()
        .map(|p| target(sketch, 0, p))
        .collect::<Result<Vec<_>>>()?;
    ensure!(
        active_constraints.len() == 5
            && active_constraints
                .iter()
                .copied()
                .collect::<BTreeSet<_>>()
                .len()
                == 5,
        "ambiguous active triangle constraint population"
    );
    ensure!(
        root["m_refPnt"]["pointer_token"].as_u64() == Some(0)
            && root["m_isHostForReferenceLines"].as_bool() == Some(false)
            && root["m_isParametric"].as_bool() == Some(true),
        "unqualified sketch reference hosting"
    );
    let mut equations = Vec::new();
    let mut sources = Vec::new();
    let mut axis_count = 0;
    let mut coincidence_count = 0;
    for (index, o) in sketch.objects.iter().enumerate().filter(|(_, o)| {
        matches!(
            o.class_name.as_str(),
            "VarSketchHorVerConstrObj" | "VarSketchPPConstrObj"
        )
    }) {
        ensure!(
            active_constraints.contains(&index),
            "constraint outside active sketch list"
        );
        ensure!(
            o.fields["m_params"].as_array().is_some_and(Vec::is_empty),
            "constraint has unqualified variables"
        );
        let refs = o.fields["m_constrElems"]
            .as_array()
            .ok_or_else(|| anyhow::anyhow!("constraint refs absent"))?;
        let indices = refs
            .iter()
            .map(|p| -> Result<usize> {
                let token = u32::try_from(
                    p["pointer_token"]
                        .as_u64()
                        .ok_or_else(|| anyhow::anyhow!("constraint token absent"))?,
                )?;
                tokens
                    .get(&token)
                    .copied()
                    .ok_or_else(|| anyhow::anyhow!("external constraint target"))
            })
            .collect::<Result<Vec<_>>>()?;
        let subtypes = o.fields["m_constrSubTypes"]
            .as_array()
            .ok_or_else(|| anyhow::anyhow!("constraint subtypes absent"))?;
        if o.class_name == "VarSketchHorVerConstrObj" {
            ensure!(
                indices.len() == 1
                    && subtypes == &vec![json!(0)]
                    && o.fields["m_priorityLevel"].as_i64() == Some(3),
                "unqualified axis constraint shape"
            );
            let axis = if o.fields["m_hor"]
                .as_bool()
                .ok_or_else(|| anyhow::anyhow!("axis flag absent"))?
            {
                1
            } else {
                0
            };
            equations.push(linear::Equation {
                terms: vec![
                    (lines[indices[0]][axis], 1.),
                    (lines[indices[0]][axis + 2], -1.),
                ],
                value: 0.,
            });
            axis_count += 1;
        } else {
            ensure!(
                indices.len() == 2
                    && subtypes.len() == 2
                    && o.fields["m_priorityLevel"].as_i64() == Some(0),
                "unqualified coincidence shape"
            );
            let ends = subtypes
                .iter()
                .map(|v| match v.as_u64() {
                    Some(1) => Ok(0),
                    Some(2) => Ok(2),
                    _ => anyhow::bail!("coincidence endpoint subtype"),
                })
                .collect::<Result<Vec<_>>>()?;
            for axis in 0..2 {
                equations.push(linear::Equation {
                    terms: vec![
                        (lines[indices[0]][ends[0] + axis], 1.),
                        (lines[indices[1]][ends[1] + axis], -1.),
                    ],
                    value: 0.,
                });
            }
            coincidence_count += 1;
        }
        sources
            .push(json!({"constraint_object":index,"class_name":o.class_name,"fields":o.fields}));
    }
    ensure!(
        axis_count == 2 && coincidence_count == 3,
        "unqualified triangle constraint population"
    );
    let ids = root["m_dimIds"]
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("dimension IDs absent"))?
        .iter()
        .map(identifier)
        .collect::<Result<BTreeSet<_>>>()?;
    let data = root["m_dimData"]
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("dimension data absent"))?
        .iter()
        .map(|v| identifier(&v["first"]))
        .collect::<Result<BTreeSet<_>>>()?;
    ensure!(data.is_subset(&ids), "linear dimension context mismatch");
    let mut seen = BTreeSet::new();
    let mut labeled = 0;
    let mut locks = 0;
    let mut distances = Vec::new();
    let mut angular_ids = BTreeSet::new();
    let mut angles = Vec::new();
    for (graph, source) in dimensions {
        let relevant = graph
            .objects
            .iter()
            .filter_map(|o| o.fields.get("m_geomRef"))
            .any(|r| {
                identifier(&r["m_elemId"])
                    .is_ok_and(|id| id == form_id as i64 || curve_lines.contains_key(&id))
            });
        if !relevant {
            continue;
        }
        let f = &graph.objects[0].fields;
        ensure!(
            graph.objects[0].class_name == "AngularDim"
                || (f["m_dirType"].as_i64() == Some(0) && vector(&f["m_fixedDir"])? == [0.; 3]),
            "unqualified dimension direction mode"
        );
        let dimension_normal = vector(&f["m_planeNormal"])?;
        ensure!(
            (norm(dimension_normal) - 1.).abs() < 1e-8
                && (dot(dimension_normal, cross(x, y)).abs() - 1.).abs() < 1e-8,
            "dimension plane differs from sketch plane"
        );

        ensure!(
            matches!(
                graph.objects[0].class_name.as_str(),
                "LinearDimString" | "Alignment" | "AngularDim"
            ) && f["m_locked"].as_bool() == Some(false),
            "unqualified linear dimension class/lock"
        );
        ensure!(
            f["m_constrInfo"].as_array().is_some_and(Vec::is_empty)
                && f["m_ArrEqualityFormulaInfo_DimEqSegInfoArr"]["m_arr"]
                    .as_array()
                    .is_some_and(Vec::is_empty)
                && f["m_ArrEqualityFormulaInfo_DimEqSegInfoArr"]["m_isLazy"].as_bool()
                    == Some(false)
                && f["m_useEqualityFormula"].as_bool() == Some(false),
            "additional/equality dimension constraints unsupported"
        );
        let id = source["local_element_id"]
            .as_i64()
            .ok_or_else(|| anyhow::anyhow!("dimension identity absent"))?;
        ensure!(
            ids.contains(&id) && seen.insert(id),
            "unexpected/duplicate linear dimension"
        );
        let segments = f["m_ArrSegInfo"]
            .as_array()
            .ok_or_else(|| anyhow::anyhow!("linear dimension segments absent"))?;
        ensure!(
            segments.len() == 1,
            "multiple dimension segments unsupported"
        );
        let segment = &segments[0];
        let refs = f["m_witnessRefs"]
            .as_array()
            .ok_or_else(|| anyhow::anyhow!("ordered dimension refs absent"))?;
        ensure!(refs.len() == 2, "dimension witness count");
        let refs = refs
            .iter()
            .map(|r| -> Result<&Value> {
                ensure!(
                    r["m_constrFlags"].as_i64() == Some(0)
                        && (graph.objects[0].class_name == "AngularDim"
                            || r["m_flipDirForAngle"].as_bool() == Some(false)),
                    "witness constraint flags unsupported"
                );
                let object = &graph.objects[target(graph, 0, &r["m_pWitnessRef"])?];
                ensure!(
                    object.class_name == "GeomSegInPlaneRef",
                    "dimension reference class"
                );
                Ok(&object.fields["m_geomRef"])
            })
            .collect::<Result<Vec<_>>>()?;
        for r in &refs {
            ensure!(
                r["m_intermediateTags"]
                    .as_array()
                    .is_some_and(Vec::is_empty)
                    && identifier(&r["m_ownerDBViewId"])? == -1
                    && r["m_foreignElemIdRef"]["m_id64"].as_i64() == Some(-1)
                    && r["m_famMemberIdx"].as_i64() == Some(-1)
                    && r["m_geomTag"].as_i64() == Some(0)
                    && r["m_flags"].as_i64() == Some(0)
                    && r["m_isLazyRef"].as_bool() == Some(false)
                    && r["m_oNextRef"]["pointer_token"].as_u64() == Some(0),
                "external/lazy/subgeometry dimension reference"
            );
        }
        let endpoints = refs
            .iter()
            .map(|r| -> Result<Option<(usize, usize)>> {
                let Some(&line) = curve_lines.get(&identifier(&r["m_elemId"])?) else {
                    return Ok(None);
                };
                let end = match r["m_subTag"].as_i64() {
                    Some(0) => 0,
                    Some(1) => 1,
                    Some(-1)
                        if matches!(
                            graph.objects[0].class_name.as_str(),
                            "Alignment" | "AngularDim"
                        ) =>
                    {
                        2
                    }
                    _ => anyhow::bail!(
                        "dimension reference not qualified endpoint or alignment line"
                    ),
                };
                Ok(Some((line, end)))
            })
            .collect::<Result<Vec<_>>>()?;
        let parameter = identifier(&segment["m_paramId"])?;
        if parameter > 0 && graph.objects[0].class_name == "AngularDim" {
            ensure!(
                segment["m_flags"].as_u64() == Some(0)
                    && endpoints.iter().all(|e| e.is_some_and(|(_, end)| end == 2)),
                "angular whole-line references required"
            );
            ensure!(
                f["m_originIsSet"].as_bool() == Some(true)
                    && f["m_sectorHasBeenSet"].as_bool() == Some(true)
                    && [
                        "m_2ArcAngle",
                        "m_parallelNonTangentRefs",
                        "m_use360ArcDimFor0",
                        "m_enableZeroLength"
                    ]
                    .iter()
                    .all(|k| f[*k].as_bool() == Some(false)),
                "angular sector mode unsupported"
            );
            ensure!(
                f["m_witnessRefs"][0]["m_flipDirForAngle"].as_bool() == Some(false)
                    && f["m_witnessRefs"][1]["m_flipDirForAngle"].as_bool() == Some(true),
                "angular witness orientation profile unqualified"
            );
            let arc = &graph.objects[target(graph, 0, &f["m_pDimArc"])?];
            ensure!(
                arc.class_name == "GArc" && arc.fields["m_bFilled"].as_bool() == Some(false),
                "angular witness arc unsupported"
            );
            let (angle, parameter_source) = resolve(parameter)?;
            ensure!(
                angle.is_finite() && angle > 1e-8 && angle < std::f64::consts::FRAC_PI_2 - 1e-8,
                "only nondegenerate acute triangle angles qualified"
            );
            let a = endpoints[0].unwrap().0;
            let b = endpoints[1].unwrap().0;
            angles.push((
                a,
                b,
                angle,
                source.clone(),
                parameter_source,
                arc.fields.clone(),
                number(&segment["m_values"][0]["m_value"])?,
            ));
            ensure!(angular_ids.insert(id), "duplicate angular dimension");
            labeled += 1;
            continue;
        }
        if parameter > 0 {
            ensure!(
                graph.objects[0].class_name == "LinearDimString"
                    && segment["m_flags"].as_u64() == Some(0)
                    && endpoints.iter().all(|e| e.is_some_and(|(_, end)| end < 2)),
                "labeled endpoint dimension scope"
            );
            let first = endpoints[0].unwrap();
            let second = endpoints[1].unwrap();
            let line = &graph.objects[target(graph, 0, &f["m_pDimLine"])?];
            ensure!(line.class_name == "GLine", "dimension axis is not linear");
            let direction = vector(&line.fields["m_dirVec"])?;
            ensure!(
                (norm(direction) - 1.).abs() < 1e-8 && dot(direction, cross(x, y)).abs() < 1e-8,
                "dimension axis outside sketch plane"
            );
            let projection = dot(
                direction,
                sub(
                    point(&saved, second.0, second.1),
                    point(&saved, first.0, first.1),
                ),
            );
            ensure!(
                projection.abs() > 1e-8,
                "ambiguous zero dimension orientation branch"
            );
            let (length, parameter_source) = resolve(parameter)?;
            ensure!(
                length > 0. && length.is_finite(),
                "nonpositive driven endpoint distance"
            );
            distances.push((
                nonlinear::Distance {
                    first: [lines[first.0][first.1 * 2], lines[first.0][first.1 * 2 + 1]],
                    second: [
                        lines[second.0][second.1 * 2],
                        lines[second.0][second.1 * 2 + 1],
                    ],
                    length,
                },
                [dot(direction, x), dot(direction, y)],
                projection.signum(),
                source.clone(),
            ));
            labeled += 1;
            sources.push(json!({"dimension":source,"parameter":parameter_source,"ordered_refs":refs,"native_dimension_axis":direction,"signed_saved_orientation_branch":projection.signum(),"rule":"retain saved oriented endpoint branch; solve authored distance"}));
        } else {
            ensure!(
                matches!(segment["m_flags"].as_u64(), Some(0 | 1))
                    && endpoints.iter().filter(|e| e.is_some()).count() == 1,
                "unqualified datum dimension"
            );
            if segment["m_flags"].as_u64() == Some(1) {
                ensure!(
                    number(&segment["m_lockedValue"])? == 0.,
                    "nonzero datum distance unqualified"
                );
                let endpoint = endpoints.iter().flatten().next().copied().unwrap();
                let datum_ref = refs
                    .iter()
                    .zip(&endpoints)
                    .find(|(_, e)| e.is_none())
                    .unwrap()
                    .0;
                ensure!(
                    datum_ref["m_subTag"].as_i64() == Some(-1),
                    "datum subreference unqualified"
                );
                let datum = identifier(&datum_ref["m_elemId"])?;
                let matches = dimensions
                    .iter()
                    .filter(|(_, s)| s["local_element_id"].as_i64() == Some(datum))
                    .collect::<Vec<_>>();
                ensure!(matches.len() == 1, "datum context ambiguous");
                let (dg, ds) = matches[0];
                let df = &dg.objects[0].fields;
                ensure!(
                    dg.objects[0].class_name == "RefPlane"
                        && df["m_locked"].as_bool() == Some(true),
                    "unfixed datum cannot anchor sketch"
                );
                let surface = &dg.objects[target(dg, 0, &df["m_pSurface"])?];
                ensure!(surface.class_name == "Plane", "datum surface nonplanar");
                let normal = cross(
                    vector(&surface.fields["m_xVec"])?,
                    vector(&surface.fields["m_yVec"])?,
                );
                ensure!((norm(normal) - 1.).abs() < 1e-8, "datum normal nonunit");
                let distance = dot(normal, sub(vector(&surface.fields["m_origin"])?, origin));
                if endpoint.1 == 2 {
                    let direction = vector(&f["m_constrDir"])?;
                    ensure!(
                        (norm(direction) - 1.).abs() < 1e-8
                            && (dot(normal, direction).abs() - 1.).abs() < 1e-8,
                        "alignment constraint direction disagrees with datum plane"
                    );
                }
                let ends = if endpoint.1 == 2 {
                    vec![0, 1]
                } else {
                    vec![endpoint.1]
                };
                for end in ends {
                    equations.push(linear::Equation {
                        terms: vec![
                            (lines[endpoint.0][end * 2], dot(normal, x)),
                            (lines[endpoint.0][end * 2 + 1], dot(normal, y)),
                        ],
                        value: distance,
                    });
                }
                locks += 1;
                sources.push(json!({"dimension":source,"locked_value":0.,"datum":ds,"ordered_refs":refs,"normal":normal,"parameter_plane_distance":distance}));
            } else {
                sources.push(
                    json!({"dimension":source,"state":"unlocked_datum_dimension_not_an_anchor"}),
                );
            }
        }
    }
    ensure!(
        seen == ids
            && labeled == 2
            && data.is_disjoint(&angular_ids)
            && data.union(&angular_ids).copied().collect::<BTreeSet<_>>() == ids,
        "incomplete endpoint dimension population"
    );
    // A cached dimension direction is only a linear equation if all authored
    // linear solutions keep the endpoint displacement parallel to that axis.
    // This prevents treating an aligned diagonal distance as a stale projection.
    loop {
        let mut progress = false;
        let mut remaining = Vec::new();
        for (distance, axis, sign, source) in distances {
            let affine = linear::affine(variables.len(), &equations)?;
            let perpendicular = |values: &[f64]| {
                let delta = [
                    values[distance.second[0]] - values[distance.first[0]],
                    values[distance.second[1]] - values[distance.first[1]],
                ];
                delta[0] * axis[1] - delta[1] * axis[0]
            };
            if perpendicular(&affine.origin).abs() < 1e-9
                && affine
                    .directions
                    .iter()
                    .all(|d| perpendicular(d).abs() < 1e-9)
            {
                let terms = (0..2)
                    .flat_map(|i| [(distance.second[i], axis[i]), (distance.first[i], -axis[i])])
                    .collect();
                equations.push(linear::Equation {
                    terms,
                    value: distance.length * sign,
                });
                sources.push(json!({"dimension":source,"rule":"true endpoint distance simplifies to signed length because authored affine constraints force zero perpendicular component","axis":axis,"saved_branch_sign":sign}));
                progress = true;
            } else {
                remaining.push((distance, axis, sign, source));
            }
        }
        distances = remaining;
        if !progress {
            break;
        }
    }
    let mut angular_checks = Vec::new();
    if !angles.is_empty() {
        ensure!(
            angles.len() == 1 && distances.is_empty(),
            "combined/multiple nonlinear dimensions unqualified"
        );
        let (a, b, angle, source, parameter_source, arc, saved_angle) = angles.remove(0);
        ensure!(a != b, "angular references same line");
        let endpoints = if (a + 1) % 3 == b {
            [
                (lines[a][2], lines[a][3], lines[a][0], lines[a][1]),
                (lines[b][0], lines[b][1], lines[b][2], lines[b][3]),
            ]
        } else {
            ensure!((b + 1) % 3 == a, "angular line adjacency");
            [
                (lines[a][0], lines[a][1], lines[a][2], lines[a][3]),
                (lines[b][2], lines[b][3], lines[b][0], lines[b][1]),
            ]
        };
        let affine = linear::affine(variables.len(), &equations)?;
        let ray = |values: &[f64], e: (usize, usize, usize, usize)| {
            [values[e.2] - values[e.0], values[e.3] - values[e.1]]
        };
        let fixed = (0..2)
            .filter(|i| {
                affine
                    .directions
                    .iter()
                    .all(|d| ray(d, endpoints[*i]).iter().all(|v| v.abs() < 1e-9))
            })
            .collect::<Vec<_>>();
        ensure!(
            fixed.len() == 1,
            "angular solve requires one independently fixed ray"
        );
        let base = ray(&affine.origin, endpoints[fixed[0]]);
        let length = base[0].hypot(base[1]);
        ensure!(length > 1e-8, "angular fixed ray degenerate");
        let unit = [base[0] / length, base[1] / length];
        let variable = endpoints[1 - fixed[0]];
        let old = ray(&saved, variable);
        let old_fixed = ray(&saved, endpoints[fixed[0]]);
        let old_cos = (old[0] * old_fixed[0] + old[1] * old_fixed[1])
            / (old[0].hypot(old[1]) * old_fixed[0].hypot(old_fixed[1]));
        ensure!(
            old_cos.is_finite() && (old_cos.clamp(-1., 1.).acos() - saved_angle).abs() < 1e-8,
            "saved angular sector differs from connected-ray angle"
        );
        let handed = unit[0] * old[1] - unit[1] * old[0];
        ensure!(handed.abs() > 1e-8, "angular saved branch degenerate");
        let target_axis = [
            unit[0] * angle.cos() - handed.signum() * unit[1] * angle.sin(),
            unit[1] * angle.cos() + handed.signum() * unit[0] * angle.sin(),
        ];
        let saved_origin = point(&saved, a, if (a + 1) % 3 == b { 1 } else { 0 });
        ensure!(
            norm(sub(vector(&arc["m_center"])?, saved_origin)) < 1e-8,
            "angular arc center differs from explicitly connected vertex"
        );
        let terms = vec![
            (variable.2, target_axis[1]),
            (variable.0, -target_axis[1]),
            (variable.3, -target_axis[0]),
            (variable.1, target_axis[0]),
        ];
        equations.push(linear::Equation { terms, value: 0. });
        let proof = json!({"kind":"acute_angle_between_connected_profile_rays","dimension":source,"parameter":parameter_source,"angle_radians":angle,"saved_angle_radians":saved_angle,"witness_flip_flags":[false,true],"fixed_ray":unit,"target_ray":target_axis,"saved_orientation_sign":handed.signum(),"ordered_line_indices":[a,b],"arc":arc,"rule":"authored fixed ray plus acute angle; retain saved oriented sector, no coordinate anchor"});
        angular_checks.push((variable, target_axis, unit, angle, proof));
    }
    let (result, mut nonlinear_proof) = if distances.is_empty() {
        (linear::solve(variables.len(), &equations)?, Value::Null)
    } else {
        ensure!(
            distances.len() == 1,
            "multiple unsolved nonlinear distances unsupported"
        );
        let (distance, _, _, source) = distances.remove(0);
        let affine = linear::affine(variables.len(), &equations)?;
        ensure!(
            affine.directions.len() == 1,
            "nonlinear distance does not fix remaining authored degrees of freedom"
        );
        let tangent = [
            affine.directions[0][distance.second[0]] - affine.directions[0][distance.first[0]],
            affine.directions[0][distance.second[1]] - affine.directions[0][distance.first[1]],
        ];
        let saved_predicate = (0..2)
            .map(|i| (saved[distance.second[i]] - saved[distance.first[i]]) * tangent[i])
            .sum::<f64>();
        ensure!(
            saved_predicate.abs() > 1e-9,
            "saved nonlinear orientation branch degenerate"
        );
        let branch = nonlinear::Branch {
            terms: (0..2)
                .flat_map(|i| {
                    [
                        (distance.second[i], tangent[i]),
                        (distance.first[i], -tangent[i]),
                    ]
                })
                .collect(),
            positive: saved_predicate > 0.,
        };
        let solved = nonlinear::solve_distance(variables.len(), &equations, &distance, &branch)?;
        let proof = json!({"dimension":source,"kind":"true_endpoint_distance","distance":distance.length,"first_variables":distance.first,"second_variables":distance.second,"linear_rank":solved.linear_rank,"final_rank":solved.final_rank,"quadratic_candidates":solved.candidate_count,"distance_residual":solved.distance_residual,"branch_terms":branch.terms,"saved_orientation_positive":branch.positive,"placement_rule":"authored affine constraints; saved coordinates choose only a nondegenerate discrete orientation branch"});
        (
            linear::Solution {
                values: solved.values,
                rank: solved.final_rank,
                maximum_scaled_residual: solved.maximum_scaled_residual,
            },
            proof,
        )
    };

    for (e, target_axis, unit, angle, mut proof) in angular_checks {
        let ray = [
            result.values[e.2] - result.values[e.0],
            result.values[e.3] - result.values[e.1],
        ];
        let length = ray[0].hypot(ray[1]);
        ensure!(
            length > 1e-8 && ray[0] * target_axis[0] + ray[1] * target_axis[1] > 1e-8,
            "angular solution reverses authored sector"
        );
        let actual = ((ray[0] * unit[0] + ray[1] * unit[1]) / length)
            .clamp(-1., 1.)
            .acos();
        ensure!(
            (actual - angle).abs() < 1e-9,
            "angular residual exceeds tolerance"
        );
        proof["angular_residual_radians"] = json!((actual - angle).abs());
        proof["final_rank"] = json!(result.rank);
        nonlinear_proof = proof;
    }
    let polygon = (0..3)
        .map(|i| point(&result.values, i, 0))
        .collect::<Vec<_>>();
    for i in 0..3 {
        ensure!(
            norm(sub(
                point(&result.values, i, 1),
                point(&result.values, (i + 1) % 3, 0)
            )) < 1e-8,
            "solved profile is not closed"
        );
    }
    Ok((
        polygon,
        json!({"role":if nonlinear_proof.is_null(){"current_profile_linear_constraints"}else{"current_profile_nonlinear_constraints"},"nonlinear":nonlinear_proof,"scope":"three-line endpoint-dimension profile in saved orientation branch","variable_objects":variables,"rank":result.rank,"variable_count":variables.len(),"equation_count":equations.len(),"absolute_lock_count":locks,"maximum_scaled_residual":result.maximum_scaled_residual,"constraints":sources,"placement_rule":"unique full-rank authored constraints; no saved coordinate used as anchor"}),
    ))
}
