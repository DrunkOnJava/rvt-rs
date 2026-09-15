//! Bounded planar wall end joins and height-overlapping cuts from current native relationships.
use crate::{
    RevitFile,
    native_document::{self, Record},
    native_metadata::identifier,
    native_parameters::ObjectGraph,
    native_scene::{Mesh, mesh},
};
use anyhow::{Result, ensure};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};

#[derive(serde::Serialize)]
pub struct JoinResult {
    #[serde(skip)]
    pub mesh: Option<Mesh>,
    pub geometry: Value,
    pub diagnostic: Option<String>,
}
struct Wall {
    id: u64,
    type_id: u64,
    origin: [f64; 2],
    x: [f64; 2],
    n: [f64; 2],
    length: [f64; 2],
    sides: [f64; 2],
    bottom: f64,
    top: f64,
    joins: Vec<Value>,
    source: Value,
    face_history: BTreeMap<Vec<i64>, i64>,
    joined_face: Option<i64>,
    current_faces: BTreeSet<i64>,
    explicit: Option<(u64, bool)>,
    cut_history: BTreeMap<Vec<i64>, i64>,
}
fn arr(v: &Value) -> Result<&Vec<Value>> {
    v.as_array()
        .ok_or_else(|| anyhow::anyhow!("join array missing"))
}
fn num(v: &Value) -> Result<f64> {
    v.as_f64()
        .filter(|n| n.is_finite())
        .ok_or_else(|| anyhow::anyhow!("join finite number missing"))
}
fn vec3(v: &Value) -> Result<[f64; 3]> {
    ensure!(arr(v)?.len() == 3, "join vector size");
    Ok([num(&v[0])?, num(&v[1])?, num(&v[2])?])
}
fn dot(a: [f64; 2], b: [f64; 2]) -> f64 {
    a[0] * b[0] + a[1] * b[1]
}
fn sub(a: [f64; 2], b: [f64; 2]) -> [f64; 2] {
    [a[0] - b[0], a[1] - b[1]]
}
fn add(a: [f64; 2], b: [f64; 2]) -> [f64; 2] {
    [a[0] + b[0], a[1] + b[1]]
}
fn scale(a: [f64; 2], s: f64) -> [f64; 2] {
    [a[0] * s, a[1] * s]
}
fn target(g: &ObjectGraph, i: usize, p: &Value, class: &str) -> Result<usize> {
    let e = g
        .edges
        .iter()
        .filter(|e| {
            e.source_object_index == i && Some(e.pointer_offset as u64) == p["offset"].as_u64()
        })
        .collect::<Vec<_>>();
    ensure!(e.len() == 1, "join pointer ownership");
    let t = e[0].target_object_index;
    ensure!(
        g.objects[t].class_name == class,
        "join pointer class expected {class}"
    );
    Ok(t)
}
fn empty(v: &Value) -> bool {
    v.as_array().is_some_and(Vec::is_empty)
}
fn parse(r: &Record) -> Result<Wall> {
    let g = r
        .graph
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("join graph unavailable"))?;
    let root = &g.objects[0].fields;
    ensure!(
        root["m_wallCrossSection"].as_i64() == Some(1)
            && root["m_isFlipped"].as_bool() == Some(false),
        "join wall orientation unsupported"
    );
    let driver = g
        .objects
        .iter()
        .enumerate()
        .filter(|(_, o)| o.class_name == "VWallDriver")
        .collect::<Vec<_>>();
    ensure!(driver.len() == 1, "join wall driver population");
    let (di, d) = driver[0];
    ensure!(
        ["m_sideAlignments", "m_midParams", "m_noJoinAtMidEnds"]
            .iter()
            .all(|k| empty(&d.fields[*k])),
        "join wall additional controls unsupported"
    );
    let mut joins = arr(&d.fields["m_controlJoinsSet"])?
        .iter()
        .map(|j| j["m_JoinInfo"].clone())
        .collect::<Vec<_>>();
    for p in arr(&d.fields["m_sideJoins"])? {
        let j = target(g, di, p, "CDJoinInfo")?;
        joins.push(g.objects[j].fields.clone());
    }
    ensure!(joins.len() <= 1, "multiple automatic joins unsupported");
    if !joins.is_empty() {
        let j = &joins[0];
        let members = arr(&j["m_info"])?;
        ensure!(
            members.len() == 2 && empty(&j["m_cleanData"]["m_data"]),
            "join pair/cleanup scope"
        );
        ensure!(
            members
                .iter()
                .all(|m| m["m_bTangent"].as_bool() == Some(false)),
            "tangent join unsupported"
        );
        ensure!(
            matches!(j["m_wallJoinType"].as_i64(), Some(0 | 1)),
            "join mode unsupported"
        );
    }
    let si = target(g, 0, &root["m_geomSteps"], "GeomStepList")?;
    let mut end_steps = Vec::new();
    let mut face_history = BTreeMap::new();
    let mut joined_face = None;
    let mut clean_peer = None;
    let mut cut_peer = None;
    let mut cut_history = BTreeMap::new();
    for field in [
        "m_nonBRepGList",
        "m_bRepFormGList",
        "m_bRepAdjustGList",
        "m_bRepCutOutGList",
        "m_bRepPostCutOutGList",
        "m_bRepTweakGList",
    ] {
        for p in arr(&g.objects[si].fields[field])? {
            let es = g
                .edges
                .iter()
                .filter(|e| {
                    e.source_object_index == si
                        && Some(e.pointer_offset as u64) == p["offset"].as_u64()
                })
                .collect::<Vec<_>>();
            ensure!(es.len() == 1, "join step ownership");
            let o = &g.objects[es[0].target_object_index];
            let f = &o.fields;
            match o.class_name.as_str() {
                "WallRefPlanesGStep" => {}
                "BaseWallGStep" => {
                    for row in arr(&f["m_faceHistTable"])? {
                        let key = arr(&row["m_faceHist"]["m_keys"])?
                            .iter()
                            .map(|v| {
                                v.as_i64()
                                    .ok_or_else(|| anyhow::anyhow!("wall face history key"))
                            })
                            .collect::<Result<Vec<_>>>()?;
                        let id = row["m_id"]
                            .as_i64()
                            .ok_or_else(|| anyhow::anyhow!("wall face history ID"))?;
                        ensure!(
                            face_history.insert(key, id).is_none(),
                            "duplicate wall face history"
                        );
                    }
                }
                "JoinEndGStep" => {
                    let history = arr(&f["m_faceHistTable"])?;
                    ensure!(
                        history.len() == 1
                            && arr(&history[0]["m_faceHist"]["m_keys"])?.len() == 3
                            && history[0]["m_faceHist"]["m_keys"][2]
                                .as_i64()
                                .is_some_and(|edge| edge > 0)
                            && history[0]["m_faceHist"]["m_keys"][0].as_i64() == Some(12)
                            && history[0]["m_faceHist"]["m_keys"][1].as_i64() == Some(-1),
                        "joined face history profile"
                    );
                    ensure!(
                        joined_face
                            .replace(
                                history[0]["m_id"]
                                    .as_i64()
                                    .ok_or_else(|| anyhow::anyhow!("joined face ID"))?
                            )
                            .is_none(),
                        "multiple joined faces"
                    );
                    end_steps.push(
                        f["m_end"]
                            .as_i64()
                            .ok_or_else(|| anyhow::anyhow!("join step end missing"))?,
                    );
                    ensure!(
                        f["m_version"].as_i64() == Some(7)
                            && f["m_flags"].as_i64() == Some(761725)
                            && empty(&f["m_geomJoined"])
                            && empty(&f["m_realJoined"])
                            && f["m_hasRealJoined"].as_bool() == Some(false),
                        "join end operation profile unsupported"
                    );
                }
                "VerticalExtensionOfLayersGStep" => ensure!(
                    empty(&f["m_otherId0"]) && empty(&f["m_otherId1"]),
                    "join layer extensions unsupported"
                ),
                "WallJoinTweakGStep" => ensure!(
                    empty(&f["m_idGeomJoined"]) && empty(&f["m_healingParents"]),
                    "additional join healing unsupported"
                ),
                "VCWSplitFaceGStep" => ensure!(
                    empty(&f["m_faceHistTable"]),
                    "joined split face unsupported"
                ),
                "CleanPairGStep" => {
                    ensure!(
                        joins.is_empty()
                            && f["m_version"].as_i64() == Some(12)
                            && f["m_flags"].as_i64() == Some(761727)
                            && f["m_userSet"].as_bool() == Some(true)
                            && f["m_doNothing"].as_bool() == Some(false)
                            && f["m_isAssociatedWithColJoinStep"].as_bool() == Some(false)
                            && empty(&f["m_healingParents"])
                            && empty(&f["m_faceHistTable"]),
                        "explicit cleanup profile unsupported"
                    );
                    ensure!(
                        clean_peer
                            .replace(u64::try_from(identifier(&f["m_otherId"])?)?)
                            .is_none(),
                        "multiple explicit cleanup peers"
                    );
                }
                "CutPairGStep" => {
                    ensure!(
                        joins.is_empty()
                            && f["m_version"].as_i64() == Some(12)
                            && f["m_flags"].as_i64() == Some(761725)
                            && f["m_userSet"].as_bool() == Some(true)
                            && [
                                "m_allDetailLevels",
                                "m_bOutlinesOverlapWithoutInt",
                                "m_isAssociatedWithColJoinStep",
                                "m_isTop",
                                "m_offsetCutter"
                            ]
                            .iter()
                            .all(|k| f[*k].as_bool() == Some(false))
                            && empty(&f["m_mergedFaceInfo"]),
                        "explicit cutting profile unsupported"
                    );
                    ensure!(
                        cut_peer
                            .replace(u64::try_from(identifier(&f["m_otherId"])?)?)
                            .is_none(),
                        "multiple explicit cutters"
                    );
                    for row in arr(&f["m_faceHistTable"])? {
                        let key = arr(&row["m_faceHist"]["m_keys"])?
                            .iter()
                            .map(|v| v.as_i64().ok_or_else(|| anyhow::anyhow!("cut face key")))
                            .collect::<Result<Vec<_>>>()?;
                        let id = row["m_id"]
                            .as_i64()
                            .ok_or_else(|| anyhow::anyhow!("cut face ID"))?;
                        ensure!(
                            cut_history.insert(key, id).is_none(),
                            "duplicate cut history"
                        );
                    }
                }
                c => anyhow::bail!("unqualified joined wall step {c}"),
            }
        }
    }
    ensure!(face_history.len() == 6, "join base face population");
    let mut current_faces = BTreeSet::new();
    let snapshot_fields = if cut_peer.is_some() {
        vec!["m_bRepPostCutOutSnapshot"]
    } else {
        vec!["m_bRepFormSnapshot", "m_bRepAdjustSnapshot"]
    };
    for field in snapshot_fields {
        let sn = target(g, si, &g.objects[si].fields[field], "SnapshotData")?;
        let tops = arr(&g.objects[sn].fields["m_geomTops"])?;
        ensure!(tops.len() == 1, "join snapshot topology population");
        let ti = target(g, sn, &tops[0], "GeometryTopology")?;
        let top = &g.objects[ti].fields;
        ensure!(
            top["m_complete"].as_bool() == Some(true),
            "join topology incomplete"
        );
        let ids = arr(&top["m_faces"])?
            .iter()
            .map(|v| {
                v["m_id"]
                    .as_i64()
                    .ok_or_else(|| anyhow::anyhow!("join current face ID"))
            })
            .collect::<Result<BTreeSet<_>>>()?;
        ensure!(
            (if cut_peer.is_some() {
                (8..=10).contains(&ids.len())
            } else {
                ids.len() == 6
            }) && ids.iter().all(|id| *id > 0)
                && ids.len() == arr(&top["m_faces"])?.len(),
            "join current face population"
        );
        if !current_faces.is_empty() {
            ensure!(
                current_faces == ids,
                "join snapshot face populations differ"
            );
        }
        current_faces = ids;
    }
    let explicit = if joins.is_empty() {
        let peer = clean_peer.ok_or_else(|| anyhow::anyhow!("explicit cleanup peer missing"))?;
        ensure!(
            end_steps.is_empty()
                && cut_peer.is_none_or(|id| id == peer)
                && d.fields["m_noJoinAtEnd"] == json!([true, true]),
            "explicit join end/control mismatch"
        );
        Some((peer, cut_peer.is_some()))
    } else {
        ensure!(
            clean_peer.is_none() && cut_peer.is_none(),
            "mixed automatic and explicit joins"
        );
        let members = arr(&joins[0]["m_info"])?;
        let own = members
            .iter()
            .filter(|m| identifier(&m["m_elemId"]).ok() == Some(r.identity.element_id as i64))
            .collect::<Vec<_>>();
        ensure!(own.len() == 1, "join owner membership");
        let end = own[0]["m_joinedEnd"]
            .as_i64()
            .ok_or_else(|| anyhow::anyhow!("joined end missing"))?;
        ensure!(
            (0..=3).contains(&end)
                && end_steps.len() == usize::from(end < 2)
                && end_steps.iter().all(|step_end| *step_end == end),
            "join end step population"
        );
        if end < 2 {
            ensure!(
                d.fields["m_noJoinAtEnd"][end as usize].as_bool() == Some(false),
                "join end disabled"
            );
        }
        None
    };
    let refs = arr(&root["m_pRefFaces"])?;
    ensure!(refs.len() == 4, "join reference face population");
    let mut planes = Vec::new();
    for p in refs {
        let f = target(g, 0, p, "Face")?;
        let pi = target(g, f, &g.objects[f].fields["m_pSurf"], "Plane")?;
        planes.push(&g.objects[pi].fields);
    }
    let o = vec3(&planes[0]["m_origin"])?;
    let x = vec3(&planes[0]["m_xVec"])?;
    let y = vec3(&planes[0]["m_yVec"])?;
    ensure!(
        x[2].abs() < 1e-10 && (x[0] * x[0] + x[1] * x[1] - 1.).abs() < 1e-10 && y == [0., 0., 1.],
        "join wall reference basis"
    );
    let origin = [o[0], o[1]];
    let x = [x[0], x[1]];
    let n = [-x[1], x[0]];
    let corners = arr(&planes[0]["m_Envelope"]["m_corners"])?;
    ensure!(corners.len() == 2, "join plane domain");
    let length = [num(&corners[0][0])?, num(&corners[1][0])?];
    let bottom = o[2] + num(&corners[0][1])?;
    let top = o[2] + num(&corners[1][1])?;
    ensure!(
        length[1] > length[0] && top > bottom,
        "join wall positive extent"
    );
    let mut sides = [0.; 2];
    for i in 0..4 {
        ensure!(
            planes[i]["m_Envelope"] == planes[0]["m_Envelope"]
                && planes[i]["m_xVec"] == planes[0]["m_xVec"]
                && planes[i]["m_yVec"] == planes[0]["m_yVec"]
                && planes[i]["m_orientFlag"].as_bool() == Some(true),
            "join reference planes disagree"
        );
        let p = vec3(&planes[i]["m_origin"])?;
        ensure!(
            (p[2] - o[2]).abs() < 1e-10 && dot(sub([p[0], p[1]], origin), x).abs() < 1e-10,
            "join side plane offset"
        );
        if i == 1 || i == 2 {
            sides[i - 1] = dot(sub([p[0], p[1]], origin), n);
        }
    }
    sides.sort_by(f64::total_cmp);
    ensure!(
        sides[0] < 0. && sides[1] > 0. && (sides[0] + sides[1]).abs() < 1e-9,
        "join centered constant width required"
    );
    Ok(Wall {
        id: r.identity.element_id,
        type_id: u64::try_from(identifier(&root["m_WallAttributesId"])?)?,
        origin,
        x,
        n,
        length,
        sides,
        bottom,
        top,
        joins,
        source: json!({"identity":r.identity,"record":r.source,"driver_object":di,"geometry_steps_object":si,"fields":["VWallDriver.m_controlJoinsSet/m_sideJoins","BaseWallGStep.m_faceHistTable","JoinEndGStep.m_faceHistTable/m_end","CutPairGStep.m_otherId/m_faceHistTable; CleanPairGStep.m_otherId","GeomStepList.m_bRepFormSnapshot/m_bRepAdjustSnapshot -> GeometryTopology.m_complete/m_faces","SWall.m_pRefFaces -> Plane"]}),
        face_history,
        joined_face,
        current_faces,
        explicit,
        cut_history,
    })
}
fn point(w: &Wall, u: f64, v: f64) -> [f64; 2] {
    add(w.origin, add(scale(w.x, u), scale(w.n, v)))
}
fn clip(poly: &[[f64; 2]], n: [f64; 2], d: f64) -> Vec<[f64; 2]> {
    let mut out = Vec::new();
    for i in 0..poly.len() {
        let a = poly[i];
        let b = poly[(i + 1) % poly.len()];
        let av = dot(a, n) - d;
        let bv = dot(b, n) - d;
        if av <= 1e-10 {
            out.push(a);
        }
        if (av < -1e-10 && bv > 1e-10) || (av > 1e-10 && bv < -1e-10) {
            out.push(add(a, scale(sub(b, a), av / (av - bv))));
        }
    }
    out
}
fn evaluate(
    w: &Wall,
    all: &BTreeMap<u64, Wall>,
    types: &BTreeMap<u64, (i64, f64)>,
) -> Result<JoinResult> {
    ensure!(
        types.contains_key(&w.type_id),
        "join type must be one constant material layer"
    );
    if w.explicit.is_some() {
        return evaluate_explicit(w, all, types);
    }
    let j = &w.joins[0];
    let members = arr(&j["m_info"])?;
    let first = u64::try_from(identifier(&members[0]["m_elemId"])?)?;
    let second = u64::try_from(identifier(&members[1]["m_elemId"])?)?;
    let own_index = usize::from(w.id == second);
    ensure!(w.id == first || w.id == second, "join owner absent");
    let other_id = if own_index == 0 { second } else { first };
    let other = all
        .get(&other_id)
        .ok_or_else(|| anyhow::anyhow!("join peer unavailable/unsupported"))?;
    ensure!(other.joins[0] == *j, "mutual join relationship differs");
    ensure!(
        types.contains_key(&other.type_id),
        "join peer material layer unsupported"
    );
    ensure!(
        dot(w.x, other.n).abs() > 1e-6 && (w.bottom - other.bottom).abs() < 1e-9,
        "automatic join requires transverse walls on a common base"
    );
    ensure!(
        (w.sides[1] - other.sides[1]).abs() < 1e-9,
        "unequal-width join unqualified"
    );
    let own_layer = types[&w.type_id];
    let peer_layer = types[&other.type_id];
    ensure!(
        own_layer.0 == peer_layer.0
            && (own_layer.1 - (w.sides[1] - w.sides[0])).abs() < 1e-8
            && (peer_layer.1 - (other.sides[1] - other.sides[0])).abs() < 1e-8,
        "join layer material/width differs from qualified reference planes"
    );
    let own_end = members[own_index]["m_joinedEnd"].as_i64().unwrap();
    let peer_end = members[1 - own_index]["m_joinedEnd"].as_i64().unwrap();
    let mode = j["m_wallJoinType"].as_i64().unwrap();
    let mut length = w.length;
    let mut plane = None;
    if own_end < 2 {
        let end = own_end as usize;
        let junction = point(w, w.length[end], 0.);
        let delta = sub(junction, other.origin);
        ensure!(
            dot(delta, other.n).abs() < 1e-8,
            "joined endpoint not on peer centerline"
        );
        let peer_u = dot(delta, other.x);
        if peer_end < 2 {
            ensure!(
                (peer_u - other.length[peer_end as usize]).abs() < 1e-8,
                "joined endpoints do not coincide"
            );
        } else {
            let clearance =
                (w.sides[1] + other.sides[1] * dot(w.x, other.x).abs()) / dot(w.x, other.n).abs();
            ensure!(
                peer_u > other.length[0] + clearance && peer_u < other.length[1] - clearance,
                "side join is not interior"
            );
        }
        let outward = scale(w.x, if end == 0 { -1. } else { 1. });
        let boundary = if mode == 1 {
            ensure!(peer_end < 2, "side miter unsupported");
            let peer_out = scale(other.x, if peer_end == 0 { -1. } else { 1. });
            let normal = sub(outward, peer_out);
            (normal, dot(junction, normal))
        } else {
            let extend = own_index == 0 && peer_end < 2;
            let normal = scale(other.n, dot(outward, other.n).signum());
            (
                normal,
                dot(junction, normal)
                    + if extend {
                        other.sides[1]
                    } else {
                        -other.sides[1]
                    },
            )
        };
        let denominator = dot(outward, boundary.0);
        ensure!(
            denominator > 1e-8,
            "join boundary does not face owned endpoint"
        );
        let reach = ((boundary.1 - dot(junction, boundary.0)).abs()
            + w.sides[1] * dot(w.n, boundary.0).abs())
            / denominator;
        ensure!(
            reach.is_finite() && reach < w.length[1] - w.length[0],
            "join would reach the opposite endpoint"
        );
        length[end] += if end == 0 { -reach } else { reach };
        plane = Some(boundary);
    } else {
        ensure!(
            own_index == 0 && peer_end < 2 && mode == 0,
            "side join priority unsupported"
        );
    }
    ensure!(length[1] > length[0], "join removes complete wall");
    let mut profile = vec![
        point(w, length[0], w.sides[0]),
        point(w, length[1], w.sides[0]),
        point(w, length[1], w.sides[1]),
        point(w, length[0], w.sides[1]),
    ];
    if let Some((n, d)) = plane {
        profile = clip(&profile, n, d);
    }
    ensure!(profile.len() >= 3, "join profile empty");
    let mesh = mesh::floor(&[profile.clone()], w.bottom, w.top)?;
    let face_planes = face_planes(w, &profile, own_end, &plane)?;
    Ok(JoinResult {
        mesh: Some(mesh),
        geometry: json!({"kind":"qualified_planar_automatic_wall_join","length_unit":"feet","coordinate_space":"project_internal","finished_width":w.sides[1]-w.sides[0],"face_planes":face_planes,"profile":profile,"bottom":w.bottom,"top":w.top,"join":j,"sources":[w.source,other.source],"scope":"common-base equal-width centered single-layer transverse pair with independently retained heights; operation-derived side-plane/bisector profile, not cached final BRep"}),
        diagnostic: None,
    })
}
pub fn read(file: &mut RevitFile, ids: BTreeSet<u64>) -> Result<BTreeMap<u64, JoinResult>> {
    if ids.is_empty() {
        return Ok(BTreeMap::new());
    }
    let version = file.basic_file_info()?.version;
    let mut walls = BTreeMap::new();
    let mut results = BTreeMap::new();
    let options = native_document::Options {
        selected_ids: ids,
        ..Default::default()
    };
    native_document::extract(file, &options, |r| {
        if r.class_name.as_deref() != Some("SWall") {
            return Ok(());
        }
        let active = r.graph.as_ref().is_some_and(|g| {
            g.objects.iter().any(|o| {
                matches!(
                    o.class_name.as_str(),
                    "JoinEndGStep" | "CutPairGStep" | "CleanPairGStep"
                ) || (o.class_name == "VWallDriver"
                    && (!empty(&o.fields["m_controlJoinsSet"]) || !empty(&o.fields["m_sideJoins"])))
            })
        });
        if !active {
            return Ok(());
        }
        let parsed = (|| {
            ensure!(
                version == 2027,
                "wall join version outside qualified 2027 profile"
            );
            parse(&r)
        })();
        match parsed {
            Ok(w) => {
                walls.insert(w.id, w);
            }
            Err(e) => {
                results.insert(
                    r.identity.element_id,
                    JoinResult {
                        mesh: None,
                        geometry: Value::Null,
                        diagnostic: Some(format!("{e:#}")),
                    },
                );
            }
        }
        Ok(())
    })?;
    let mut types = BTreeMap::new();
    let type_ids = walls.values().map(|w| w.type_id).collect::<BTreeSet<_>>();
    if !type_ids.is_empty() {
        native_document::extract(
            file,
            &native_document::Options {
                selected_ids: type_ids,
                ..Default::default()
            },
            |r| {
                if let Some(g) = &r.graph {
                    if r.class_name.as_deref() == Some("BasicWallType") {
                        let valid = (|| -> Result<(i64, f64)> {
                            let i = target(
                                g,
                                0,
                                &g.objects[0].fields["m_pCompoundStructure"],
                                "CompoundStructure",
                            )?;
                            let f = &g.objects[i].fields;
                            ensure!(
                                arr(&f["m_layers"])?.len() == 1
                                    && f["m_variableLayerIdx"].as_i64() == Some(-1),
                                "nonuniform join type"
                            );
                            let layer = &f["m_layers"][0];
                            let material = identifier(&layer["m_materialId"])?;
                            let width = num(&layer["m_layerWidth"])?;
                            ensure!(material > 0 && width > 0., "join material/width unresolved");
                            Ok((material, width))
                        })();
                        if let Ok(layer) = valid {
                            types.insert(r.identity.element_id, layer);
                        }
                    }
                }
                Ok(())
            },
        )?;
    }
    for (&id, w) in &walls {
        let result = evaluate(w, &walls, &types).unwrap_or_else(|e| JoinResult {
            mesh: None,
            geometry: Value::Null,
            diagnostic: Some(format!("{e:#}")),
        });
        results.insert(id, result);
    }
    Ok(results)
}

fn face_planes(
    w: &Wall,
    profile: &[[f64; 2]],
    end: i64,
    miter: &Option<([f64; 2], f64)>,
) -> Result<Vec<Value>> {
    let u0 = profile
        .iter()
        .map(|p| dot(sub(*p, w.origin), w.x))
        .fold(f64::INFINITY, f64::min);
    let u1 = profile
        .iter()
        .map(|p| dot(sub(*p, w.origin), w.x))
        .fold(f64::NEG_INFINITY, f64::max);
    let mut planes = Vec::new();
    let mut tags = BTreeSet::new();
    for (key, p, n, role) in [
        (
            vec![1, -1, -1],
            point(w, 0., w.sides[1]),
            w.n,
            "wall_positive_side",
        ),
        (
            vec![2, -1, -1],
            point(w, 0., w.sides[0]),
            scale(w.n, -1.),
            "wall_negative_side",
        ),
        (
            vec![3, 0, -1],
            point(w, u0, 0.),
            scale(w.x, -1.),
            "wall_start",
        ),
        (vec![3, 1, -1], point(w, u1, 0.), w.x, "wall_end"),
    ] {
        let replaced = key == vec![3, end, -1] && end < 2;
        let tag = if replaced {
            w.joined_face
                .ok_or_else(|| anyhow::anyhow!("joined face absent"))?
        } else {
            *w.face_history
                .get(&key)
                .ok_or_else(|| anyhow::anyhow!("base face history absent"))?
        };
        let (p, n) = if replaced {
            if let Some((n, d)) = miter {
                let norm = dot(*n, *n).sqrt();
                (scale(*n, *d / (norm * norm)), scale(*n, 1. / norm))
            } else {
                (p, n)
            }
        } else {
            (p, n)
        };
        ensure!(tags.insert(tag), "duplicate joined face tag");
        planes.push(
            json!({"tag":tag,"point":[p[0],p[1],w.bottom],"normal":[n[0],n[1],0.],"role":role}),
        );
    }
    for (key, z, n, role) in [
        (vec![3, 2, -1], w.bottom, -1., "wall_bottom"),
        (vec![3, 3, -1], w.top, 1., "wall_top"),
    ] {
        let tag = *w
            .face_history
            .get(&key)
            .ok_or_else(|| anyhow::anyhow!("base cap history absent"))?;
        ensure!(tags.insert(tag), "duplicate joined cap tag");
        planes.push(
            json!({"tag":tag,"point":[w.origin[0],w.origin[1],z],"normal":[0.,0.,n],"role":role}),
        );
    }
    ensure!(
        (if w.explicit.is_some() {
            tags.is_subset(&w.current_faces)
        } else {
            tags == w.current_faces
        }),
        "derived joined face IDs differ from complete current topology"
    );
    Ok(planes)
}

fn evaluate_explicit(
    w: &Wall,
    all: &BTreeMap<u64, Wall>,
    types: &BTreeMap<u64, (i64, f64)>,
) -> Result<JoinResult> {
    let (peer_id, cut) = w
        .explicit
        .ok_or_else(|| anyhow::anyhow!("explicit relation absent"))?;
    let other = all
        .get(&peer_id)
        .ok_or_else(|| anyhow::anyhow!("explicit peer unavailable"))?;
    ensure!(
        other.explicit == Some((w.id, !cut)),
        "explicit cleanup/cutter reciprocity differs"
    );
    let a = types
        .get(&w.type_id)
        .ok_or_else(|| anyhow::anyhow!("explicit type unsupported"))?;
    let b = types
        .get(&other.type_id)
        .ok_or_else(|| anyhow::anyhow!("explicit peer type unsupported"))?;
    ensure!(
        a.0 == b.0
            && (a.1 - (w.sides[1] - w.sides[0])).abs() < 1e-8
            && (b.1 - (other.sides[1] - other.sides[0])).abs() < 1e-8
            && (a.1 - b.1).abs() < 1e-8,
        "explicit equal-width material scope"
    );
    ensure!(
        dot(w.x, other.n).abs() > 1e-6 && w.bottom.max(other.bottom) < w.top.min(other.top) - 1e-8,
        "explicit crossing requires transverse walls with positive height overlap"
    );
    let whole = vec![
        point(w, w.length[0], w.sides[0]),
        point(w, w.length[1], w.sides[0]),
        point(w, w.length[1], w.sides[1]),
        point(w, w.length[0], w.sides[1]),
    ];
    let low = dot(other.origin, other.n) + other.sides[0];
    let high = dot(other.origin, other.n) + other.sides[1];
    let removed = clip(&clip(&whole, other.n, high), scale(other.n, -1.), -low);
    ensure!(
        removed.len() == 4
            && removed.iter().all(|p| {
                let own_u = dot(sub(*p, w.origin), w.x);
                let peer_u = dot(sub(*p, other.origin), other.x);
                own_u > w.length[0] + 1e-8
                    && own_u < w.length[1] - 1e-8
                    && peer_u > other.length[0] + 1e-8
                    && peer_u < other.length[1] - 1e-8
            }),
        "explicit crossing must pass through the interior of both walls"
    );
    let profiles = if cut {
        vec![
            clip(&whole, other.n, low),
            clip(&whole, scale(other.n, -1.), -high),
        ]
    } else {
        vec![whole.clone()]
    };
    let mut mesh = Mesh {
        vertices: Vec::new(),
        triangles: Vec::new(),
    };
    for profile in &profiles {
        ensure!(
            profile.len() == 4,
            "crossing strip must leave two bounded quadrilateral components"
        );
        let part = mesh::floor(std::slice::from_ref(profile), w.bottom, w.top)?;
        let shift = u32::try_from(mesh.vertices.len())?;
        mesh.vertices.extend(part.vertices);
        mesh.triangles
            .extend(part.triangles.into_iter().map(|t| t.map(|i| i + shift)));
    }
    let mut layers = Vec::new();
    if cut {
        let lo = w.bottom.max(other.bottom);
        let hi = w.top.min(other.top);
        if lo > w.bottom {
            layers.push(crate::native_scene::layered_prism::Layer {
                bottom: w.bottom,
                top: lo,
                profiles: vec![whole.clone()],
            });
        }
        layers.push(crate::native_scene::layered_prism::Layer {
            bottom: lo,
            top: hi,
            profiles: profiles.clone(),
        });
        if hi < w.top {
            layers.push(crate::native_scene::layered_prism::Layer {
                bottom: hi,
                top: w.top,
                profiles: vec![whole.clone()],
            });
        }
        if layers.len() > 1 {
            mesh = crate::native_scene::layered_prism::build(&layers)?;
        }
    }
    let mut planes = face_planes(w, &whole, 2, &None)?;
    if cut {
        // The saved history can retain non-current cap IDs, or omit a cap
        // outside the cut wall height. Current faces are established below
        // from required plane intersections and the complete topology.
        let allowed_history = [
            vec![1, -1, -1],
            vec![2, -1, -1],
            vec![3, 2, -1],
            vec![3, 3, -1],
        ]
        .iter()
        .map(|key| {
            other
                .face_history
                .get(key)
                .map(|tag| vec![5, *tag, 0])
                .ok_or_else(|| anyhow::anyhow!("cutter source face history missing"))
        })
        .collect::<Result<BTreeSet<_>>>()?;
        ensure!(
            w.cut_history
                .keys()
                .all(|key| allowed_history.contains(key))
                && w.cut_history.values().all(|tag| *tag > 0)
                && w.cut_history.values().collect::<BTreeSet<_>>().len() == w.cut_history.len(),
            "explicit cut history contains unqualified source or duplicate face IDs"
        );
        for (key, offset, outward) in [
            (vec![1, -1, -1], other.sides[1], other.n),
            (vec![2, -1, -1], other.sides[0], scale(other.n, -1.)),
        ] {
            let peer_tag = *other
                .face_history
                .get(&key)
                .ok_or_else(|| anyhow::anyhow!("cutter side face missing"))?;
            let tag = *w
                .cut_history
                .get(&vec![5, peer_tag, 0])
                .ok_or_else(|| anyhow::anyhow!("cut face missing cutter-face provenance"))?;
            let p = point(other, 0., offset);
            let n = scale(outward, -1.);
            planes.push(json!({"tag":tag,"point":[p[0],p[1],w.bottom],"normal":[n[0],n[1],0.],"role":"wall_explicit_cut","cutter_element_id":other.id,"cutter_face_tag":peer_tag}));
        }
        for (key, z, normal) in [
            (vec![3, 2, -1], other.bottom, 1.),
            (vec![3, 3, -1], other.top, -1.),
        ] {
            if z > w.bottom + 1e-8 && z < w.top - 1e-8 {
                let peer_tag = *other
                    .face_history
                    .get(&key)
                    .ok_or_else(|| anyhow::anyhow!("cutter cap identity missing"))?;
                let tag = *w
                    .cut_history
                    .get(&vec![5, peer_tag, 0])
                    .ok_or_else(|| anyhow::anyhow!("cut cap history missing"))?;
                planes.push(json!({"tag":tag,"point":[w.origin[0],w.origin[1],z],"normal":[0.,0.,normal],"role":"wall_explicit_cut","cutter_element_id":other.id,"cutter_face_tag":peer_tag}));
            }
        }
    }
    let tags = planes
        .iter()
        .map(|p| p["tag"].as_i64().unwrap())
        .collect::<BTreeSet<_>>();
    ensure!(
        tags.len() == planes.len() && tags == w.current_faces,
        "explicit evaluated plane/current topology identities differ"
    );
    Ok(JoinResult {
        mesh: Some(mesh),
        geometry: json!({"kind":"qualified_planar_explicit_wall_join","length_unit":"feet","coordinate_space":"project_internal","finished_width":a.1,"profiles":profiles,"height_layers":layers.iter().map(|l| json!({"bottom":l.bottom,"top":l.top,"profiles":l.profiles})).collect::<Vec<_>>(),"bottom":w.bottom,"top":w.top,"face_planes":planes,"join":{"kind":"explicit_geometry_join","owner_id":w.id,"peer_id":other.id,"is_cutting_element":!cut,"cutting_element_id":if cut{other.id}else{w.id},"cut_element_id":if cut{w.id}else{other.id}},"sources":[w.source,other.source],"scope":"interior transverse equal-width single-layer pair; CutPair side/cap-plane operation and reciprocal CleanPair, independent height overlap and closed components preserved"}),
        diagnostic: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn pair(mode: i64) -> BTreeMap<u64, Wall> {
        let j = json!({"m_info":[{"m_elemId":{"m_id":{"m_id64":1}},"m_joinedEnd":1},{"m_elemId":{"m_id":{"m_id64":2}},"m_joinedEnd":0}],"m_wallJoinType":mode});
        BTreeMap::from([
            (
                1,
                Wall {
                    id: 1,
                    type_id: 3,
                    origin: [0., 0.],
                    x: [1., 0.],
                    n: [0., 1.],
                    length: [0., 10.],
                    sides: [-0.375, 0.375],
                    bottom: 0.,
                    top: 10.,
                    joins: vec![j.clone()],
                    source: Value::Null,
                    face_history: BTreeMap::from([
                        (vec![1, -1, -1], 5),
                        (vec![2, -1, -1], 6),
                        (vec![3, 0, -1], 18),
                        (vec![3, 1, -1], 10),
                        (vec![3, 2, -1], 7),
                        (vec![3, 3, -1], 14),
                    ]),
                    joined_face: Some(27),
                    current_faces: BTreeSet::from([5, 6, 7, 14, 18, 27]),
                    explicit: None,
                    cut_history: BTreeMap::new(),
                },
            ),
            (
                2,
                Wall {
                    id: 2,
                    type_id: 3,
                    origin: [10., 0.],
                    x: [0., 1.],
                    n: [-1., 0.],
                    length: [0., 10.],
                    sides: [-0.375, 0.375],
                    bottom: 0.,
                    top: 10.,
                    joins: vec![j],
                    source: Value::Null,
                    face_history: BTreeMap::from([
                        (vec![1, -1, -1], 5),
                        (vec![2, -1, -1], 6),
                        (vec![3, 0, -1], 18),
                        (vec![3, 1, -1], 10),
                        (vec![3, 2, -1], 7),
                        (vec![3, 3, -1], 14),
                    ]),
                    joined_face: Some(27),
                    current_faces: BTreeSet::from([5, 6, 7, 10, 14, 27]),
                    explicit: None,
                    cut_history: BTreeMap::new(),
                },
            ),
        ])
    }
    fn area(p: &Value) -> f64 {
        let a = p.as_array().unwrap();
        (0..a.len())
            .map(|i| {
                let b = &a[(i + 1) % a.len()];
                a[i][0].as_f64().unwrap() * b[1].as_f64().unwrap()
                    - a[i][1].as_f64().unwrap() * b[0].as_f64().unwrap()
            })
            .sum::<f64>()
            .abs()
            / 2.
    }
    #[test]
    fn ordered_butt_and_miter_partition_corner_without_double_volume() {
        let types = BTreeMap::from([(3, (99, 0.75))]);
        for mode in [0, 1] {
            let walls = pair(mode);
            let a = evaluate(&walls[&1], &walls, &types).unwrap();
            let b = evaluate(&walls[&2], &walls, &types).unwrap();
            assert!(
                (area(&a.geometry["profile"]) + area(&b.geometry["profile"]) - 15.).abs() < 1e-10
            );
            if mode == 0 {
                assert!((area(&a.geometry["profile"]) - 7.78125).abs() < 1e-10);
            } else {
                assert!((area(&a.geometry["profile"]) - 7.5).abs() < 1e-10);
            }
        }
    }
    #[test]
    fn independent_heights_preserved_and_asymmetric_pair_refused() {
        let types = BTreeMap::from([(3, (99, 0.75))]);
        let mut walls = pair(0);
        walls.get_mut(&2).unwrap().top = 9.;
        assert_eq!(
            evaluate(&walls[&1], &walls, &types).unwrap().geometry["top"],
            json!(10.)
        );
        assert_eq!(
            evaluate(&walls[&2], &walls, &types).unwrap().geometry["top"],
            json!(9.)
        );
        walls.get_mut(&2).unwrap().top = 10.;
        walls.get_mut(&2).unwrap().joins[0]["m_wallJoinType"] = json!(1);
        assert!(evaluate(&walls[&1], &walls, &types).is_err());
    }
    #[test]
    fn transverse_butt_and_miter_keep_volume_partition() {
        let types = BTreeMap::from([(3, (99, 0.75))]);
        for degrees in [15.0_f64, 30., 37., 60., 120., 150., 165.] {
            for mode in [0, 1] {
                let mut walls = pair(mode);
                let peer = walls.get_mut(&2).unwrap();
                let angle = degrees.to_radians();
                peer.x = [angle.cos(), angle.sin()];
                peer.n = [-angle.sin(), angle.cos()];
                let a = evaluate(&walls[&1], &walls, &types).unwrap();
                let b = evaluate(&walls[&2], &walls, &types).unwrap();
                assert!(
                    (area(&a.geometry["profile"]) + area(&b.geometry["profile"]) - 15.).abs()
                        < 1e-8
                );
            }
        }
    }
    fn explicit_fixture() -> BTreeMap<u64, Wall> {
        let mut walls = pair(0);
        for (&id, w) in &mut walls {
            w.joins.clear();
            w.joined_face = None;
            w.explicit = Some((if id == 1 { 2 } else { 1 }, id == 2));
            w.current_faces = BTreeSet::from([5, 6, 7, 10, 14, 18]);
        }
        let cut = walls.get_mut(&2).unwrap();
        cut.origin = [5., -5.];
        cut.cut_history = BTreeMap::from([
            (vec![5, 5, 0], 23),
            (vec![5, 6, 0], 28),
            (vec![5, 7, 0], 33),
            (vec![5, 14, 0], 36),
        ]);
        cut.current_faces.extend([23, 28]);
        walls
    }

    #[test]
    fn partial_height_cut_requires_only_current_cap_planes() {
        let types = BTreeMap::from([(3, (99, 0.75))]);
        let mut walls = explicit_fixture();
        walls.get_mut(&1).unwrap().bottom = 2.25;
        walls.get_mut(&1).unwrap().top = 7.25;
        walls.get_mut(&2).unwrap().current_faces.extend([33, 36]);
        let result = evaluate(&walls[&2], &walls, &types).unwrap();
        assert_eq!(result.geometry["face_planes"].as_array().unwrap().len(), 10);
        let mut m = result.mesh.unwrap();
        mesh::validate(&mut m).unwrap();
        walls
            .get_mut(&2)
            .unwrap()
            .cut_history
            .remove(&vec![5, 14, 0]);
        assert!(evaluate(&walls[&2], &walls, &types).is_err());
    }

    #[test]
    fn absent_inactive_history_is_valid_but_unknown_provenance_is_not() {
        let types = BTreeMap::from([(3, (99, 0.75))]);
        let mut walls = explicit_fixture();
        walls.get_mut(&2).unwrap().top = 7.25;
        walls
            .get_mut(&2)
            .unwrap()
            .cut_history
            .remove(&vec![5, 14, 0]);
        assert!(evaluate(&walls[&2], &walls, &types).is_ok());
        walls
            .get_mut(&2)
            .unwrap()
            .cut_history
            .insert(vec![5, 999, 0], 99);
        assert!(evaluate(&walls[&2], &walls, &types).is_err());
    }

    #[test]
    fn explicit_cut_order_preserves_two_closed_components_and_reverses_cutter_faces() {
        let mut walls = pair(0);
        for (&id, w) in &mut walls {
            w.joins.clear();
            w.joined_face = None;
            w.explicit = Some((if id == 1 { 2 } else { 1 }, id == 2));
            w.current_faces = BTreeSet::from([5, 6, 7, 10, 14, 18]);
        }
        walls.get_mut(&2).unwrap().origin = [5., -5.];
        let cut = walls.get_mut(&2).unwrap();
        cut.cut_history = BTreeMap::from([
            (vec![5, 5, 0], 23),
            (vec![5, 6, 0], 28),
            (vec![5, 7, 0], 33),
            (vec![5, 14, 0], 36),
        ]);
        cut.current_faces.extend([23, 28]);
        let types = BTreeMap::from([(3, (99, 0.75))]);
        let a = evaluate(&walls[&1], &walls, &types).unwrap();
        let b = evaluate(&walls[&2], &walls, &types).unwrap();
        assert_eq!(a.geometry["profiles"].as_array().unwrap().len(), 1);
        assert_eq!(b.geometry["profiles"].as_array().unwrap().len(), 2);
        let total = b.geometry["profiles"]
            .as_array()
            .unwrap()
            .iter()
            .map(area)
            .sum::<f64>();
        assert!((total - 6.9375).abs() < 1e-10);
        assert_eq!(b.geometry["face_planes"].as_array().unwrap().len(), 8);
        walls.get_mut(&1).unwrap().explicit = Some((2, true));
        assert!(evaluate(&walls[&2], &walls, &types).is_err());
    }
}
