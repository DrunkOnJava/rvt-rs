//! Stable wall surface history joined to owned planes and qualified wall meshes.
use super::{Source, Surface, arr, int, num, source, target, triangle_plane};
use crate::{
    native_document::Record, native_equipment::Identity, native_metadata::identifier, native_scene,
};
use anyhow::{Result, ensure};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
#[derive(Clone)]
struct FacePlane {
    tag: i64,
    normal: [f64; 3],
    point: [f64; 3],
    role: &'static str,
    bounds: Option<[f64; 4]>,
}
struct Wall {
    owner: Identity,
    type_id: i64,
    origin: [f64; 3],
    x: [f64; 3],
    y: [f64; 3],
    openings: Vec<(i64, [i64; 4], Source)>,
    planes: Vec<FacePlane>,
    cut_face_tags: Option<BTreeSet<i64>>,
    paint: BTreeMap<i64, i64>,
    sources: Vec<Source>,
}
struct JoinedWall {
    owner: Identity,
    type_id: i64,
    paint: BTreeMap<i64, i64>,
    sources: Vec<Source>,
}
#[derive(Default)]
pub(super) struct Context {
    walls: Vec<Wall>,
    joined: Vec<JoinedWall>,
    openings: BTreeMap<i64, (i64, [[f64; 3]; 2], Source)>,
    materials: BTreeMap<i64, (i64, Source)>,
}
fn vector(v: &Value) -> Result<[f64; 3]> {
    ensure!(arr(v)?.len() == 3, "wall plane vector size");
    Ok([num(&v[0])?, num(&v[1])?, num(&v[2])?])
}
fn add(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    std::array::from_fn(|i| a[i] + b[i])
}
fn sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    std::array::from_fn(|i| a[i] - b[i])
}
fn scale(a: [f64; 3], v: f64) -> [f64; 3] {
    a.map(|a| a * v)
}
fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a.iter().zip(b).map(|(a, b)| a * b).sum()
}
fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
impl Context {
    pub(super) fn is_empty(&self) -> bool {
        self.walls.is_empty() && self.joined.is_empty()
    }
    pub(super) fn ingest(&mut self, r: &Record) -> Result<()> {
        if r.channel != 102
            || !matches!(
                r.class_name.as_deref(),
                Some("SWall" | "BasicWallType" | "SWallRectOpening")
            )
        {
            return Ok(());
        }
        let g = r
            .graph
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("wall surface graph unavailable"))?;
        let root = &g.objects[0].fields;
        if r.class_name.as_deref() == Some("SWallRectOpening") {
            let (i, data) = target(g, 0, &root["m_oRectOpeningData"], "RectOpeningData")?;
            let corners = arr(&data["m_aPoint"])?;
            ensure!(corners.len() == 2, "opening corners cardinality");
            ensure!(
                self.openings
                    .insert(
                        r.identity.element_id as i64,
                        (
                            identifier(&root["m_hostId"])?,
                            [vector(&corners[0])?, vector(&corners[1])?],
                            source(r, i, "RectOpeningData.m_aPoint;root.m_hostId")
                        )
                    )
                    .is_none(),
                "duplicate opening identity"
            );
            return Ok(());
        }
        if r.class_name.as_deref() == Some("BasicWallType") {
            let (i, c) = target(g, 0, &root["m_pCompoundStructure"], "CompoundStructure")?;
            let layers = arr(&c["m_layers"])?;
            ensure!(
                layers.len() == 1 && int(&c["m_variableLayerIdx"])? == -1,
                "wall material requires one constant layer"
            );
            self.materials.insert(
                r.identity.element_id as i64,
                (
                    identifier(&layers[0]["m_materialId"])?,
                    source(r, i, "m_layers[0].m_materialId"),
                ),
            );
            return Ok(());
        }
        ensure!(
            root["m_wallCrossSection"].as_i64() == Some(1)
                && root["m_isFlipped"].as_bool() == Some(false),
            "wall surface orientation outside unflipped vertical scope"
        );
        // Stage identity/material metadata for the separate operation-qualified
        // join evaluator. No face is published until its final topology and mesh
        // have been qualified together by that evaluator and the checks below.
        if g.objects.iter().any(|o| {
            matches!(
                o.class_name.as_str(),
                "JoinEndGStep" | "CutPairGStep" | "CleanPairGStep"
            ) || (o.class_name == "VWallDriver"
                && ["m_controlJoinsSet", "m_sideJoins"]
                    .iter()
                    .any(|k| o.fields[*k].as_array().is_some_and(|v| !v.is_empty())))
        }) {
            let (ti, t) = target(g, 0, &root["m_pGeomTable"], "GeomTable")?;
            let mut paint = BTreeMap::new();
            let mut sources = vec![source(r, 0, "SWall.m_WallAttributesId/m_pGeomTable")];
            for p in arr(&t["m_materialMarkers"])? {
                let (i, m) = target(g, ti, p, "GeomMaterialMarker")?;
                let material = identifier(&m["m_materialId"])?;
                ensure!(
                    material > 0 && paint.insert(int(&m["m_geomTag"])?, material).is_none(),
                    "invalid/duplicate joined wall paint"
                );
                sources.push(source(r, i, "GeomMaterialMarker.m_geomTag/m_materialId"));
            }
            self.joined.push(JoinedWall {
                owner: source(r, 0, "").owner,
                type_id: identifier(&root["m_WallAttributesId"])?,
                paint,
                sources,
            });
            return Ok(());
        }
        let (steps_index, steps) = target(g, 0, &root["m_geomSteps"], "GeomStepList")?;
        for name in [
            "m_nonBRepGList",
            "m_bRepFormGList",
            "m_bRepAdjustGList",
            "m_bRepCutOutGList",
            "m_bRepPostCutOutGList",
            "m_bRepTweakGList",
        ] {
            for pointer in arr(&steps[name])? {
                let edges = g
                    .edges
                    .iter()
                    .filter(|e| {
                        e.source_object_index == steps_index
                            && Some(e.pointer_offset as u64) == pointer["offset"].as_u64()
                    })
                    .collect::<Vec<_>>();
                ensure!(edges.len() == 1, "wall geometry step ownership ambiguous");
                let step = &g.objects[edges[0].target_object_index];
                let fields = &step.fields;
                match step.class_name.as_str() {
                    "WallRefPlanesGStep" | "BaseWallGStep" => {}
                    "VerticalExtensionOfLayersGStep" => ensure!(
                        ["m_otherId0", "m_otherId1"]
                            .iter()
                            .all(|k| fields[*k].as_array().is_some_and(Vec::is_empty)),
                        "wall layer extensions unqualified"
                    ),
                    "WallJoinTweakGStep" => ensure!(
                        ["m_idGeomJoined", "m_healingParents", "m_affectedEdgesTags"]
                            .iter()
                            .all(|k| fields[*k].as_array().is_some_and(Vec::is_empty)),
                        "wall joined surfaces unqualified"
                    ),
                    "VCWSplitFaceGStep" => ensure!(
                        arr(&fields["m_faceHistTable"])?.is_empty(),
                        "split wall face history unqualified"
                    ),
                    "WallRectOpeningGStep" => ensure!(
                        fields["m_bCutSomething"].as_i64() == Some(1)
                            && fields["m_removedWholeGeometry"].as_bool() == Some(false)
                            && fields["m_bUseUnattachedVoids"].as_bool() == Some(false)
                            && fields["m_bIsTrueBoolean"].as_bool() == Some(false)
                            && identifier(&fields["m_infillIdBefore"])? == -1
                            && identifier(&fields["m_infillIdAfter"])? == -1,
                        "opening boolean/phase operation unqualified"
                    ),
                    other => anyhow::bail!("unqualified wall surface step {other}"),
                }
            }
        }
        let refs = arr(&root["m_pRefFaces"])?;
        ensure!(refs.len() == 4, "wall reference plane population");
        let mut planes = Vec::new();
        let mut sources = Vec::new();
        for p in refs {
            let (fi, f) = target(g, 0, p, "Face")?;
            let (pi, p) = target(g, fi, &f["m_pSurf"], "Plane")?;
            ensure!(
                p["m_orientFlag"].as_bool() == Some(true),
                "wall reference plane orientation"
            );
            planes.push((
                vector(&p["m_origin"])?,
                vector(&p["m_xVec"])?,
                vector(&p["m_yVec"])?,
                p["m_Envelope"]["m_corners"].clone(),
            ));
            sources.push(source(r, pi, "m_pRefFaces[].m_pSurf.Plane"));
        }
        let (origin, x, y, domain) = &planes[0];
        let n = cross(*x, *y);
        ensure!(
            (dot(*x, *x) - 1.).abs() < 1e-8
                && (dot(*y, *y) - 1.).abs() < 1e-8
                && dot(*x, *y).abs() < 1e-8,
            "wall plane basis not orthonormal"
        );
        for p in &planes {
            ensure!(
                p.1 == *x && p.2 == *y && p.3 == *domain,
                "wall reference planes disagree"
            );
        }
        let u0 = num(&domain[0][0])?;
        let v0 = num(&domain[0][1])?;
        let u1 = num(&domain[1][0])?;
        let v1 = num(&domain[1][1])?;
        ensure!(u1 > u0 && v1 > v0, "wall profile domain invalid");
        ensure!(
            dot(sub(planes[1].0, *origin), n) < 0. && dot(sub(planes[2].0, *origin), n) > 0.,
            "wall cap side ordering unqualified"
        );
        let mut histories = BTreeMap::new();
        for (i, o) in g
            .objects
            .iter()
            .enumerate()
            .filter(|(_, o)| o.class_name == "BaseWallGStep")
        {
            ensure!(histories.is_empty(), "multiple base wall generators");
            for row in arr(&o.fields["m_faceHistTable"])? {
                let keys = arr(&row["m_faceHist"]["m_keys"])?
                    .iter()
                    .map(int)
                    .collect::<Result<Vec<_>>>()?;
                ensure!(
                    histories.insert(keys, int(&row["m_id"])?).is_none(),
                    "duplicate wall face history"
                );
            }
            sources.push(source(r, i, "BaseWallGStep.m_faceHistTable"));
        }
        ensure!(histories.len() == 6, "wall base history needs six faces");
        let mut tags = BTreeSet::new();
        let mut faces = Vec::new();
        for (key, point, normal, role) in [
            (
                vec![1, -1, -1],
                planes[1].0,
                scale(n, -1.),
                "wall_positive_side",
            ),
            (vec![2, -1, -1], planes[2].0, n, "wall_negative_side"),
            (
                vec![3, 0, -1],
                add(*origin, scale(*x, u0)),
                scale(*x, -1.),
                "wall_start",
            ),
            (vec![3, 1, -1], add(*origin, scale(*x, u1)), *x, "wall_end"),
            (
                vec![3, 2, -1],
                add(*origin, scale(*y, v0)),
                scale(*y, -1.),
                "wall_bottom",
            ),
            (vec![3, 3, -1], add(*origin, scale(*y, v1)), *y, "wall_top"),
        ] {
            let tag = *histories
                .get(&key)
                .ok_or_else(|| anyhow::anyhow!("wall face key missing"))?;
            ensure!(tags.insert(tag), "duplicate wall surface tag");
            faces.push(FacePlane {
                tag,
                point,
                normal,
                role,
                bounds: None,
            });
        }
        let (ti, t) = target(g, 0, &root["m_pGeomTable"], "GeomTable")?;
        let mut paint = BTreeMap::new();
        for p in arr(&t["m_materialMarkers"])? {
            let (i, m) = target(g, ti, p, "GeomMaterialMarker")?;
            let material = identifier(&m["m_materialId"])?;
            ensure!(material > 0, "invalid wall paint material");
            ensure!(
                paint.insert(int(&m["m_geomTag"])?, material).is_none(),
                "duplicate wall paint tag"
            );
            sources.push(source(r, i, "GeomMaterialMarker.m_geomTag/m_materialId"));
        }
        let mut openings = Vec::new();
        for (i, o) in g
            .objects
            .iter()
            .enumerate()
            .filter(|(_, o)| o.class_name == "WallRectOpeningGStep")
        {
            let ids = arr(&o.fields["m_OpeningFacesIds"])?
                .iter()
                .map(int)
                .collect::<Result<Vec<_>>>()?;
            ensure!(ids.len() == 4, "opening face ID count");
            openings.push((
                identifier(&o.fields["m_instId"])?,
                [ids[0], ids[1], ids[2], ids[3]],
                source(r, i, "WallRectOpeningGStep.m_instId/m_OpeningFacesIds"),
            ));
        }
        let mut cut_face_tags = None;
        if !openings.is_empty() {
            for field in ["m_bRepFormSnapshot", "m_bRepAdjustSnapshot"] {
                let (si, snapshot) = target(g, steps_index, &steps[field], "SnapshotData")?;
                let tops = arr(&snapshot["m_geomTops"])?;
                ensure!(tops.len() == 1, "wall snapshot topology population");
                let (ti, topology) = target(g, si, &tops[0], "GeometryTopology")?;
                ensure!(
                    topology["m_complete"].as_bool() == Some(true),
                    "incomplete wall topology"
                );
                let entries = arr(&topology["m_faces"])?;
                let tags = entries
                    .iter()
                    .map(|f| int(&f["m_id"]))
                    .collect::<Result<BTreeSet<_>>>()?;
                ensure!(
                    tags.len() == entries.len() && tags.iter().all(|t| *t > 0),
                    "invalid wall topology face identities"
                );
                if let Some(previous) = &cut_face_tags {
                    ensure!(previous == &tags, "wall form/adjust topology differs");
                }
                cut_face_tags = Some(tags);
                sources.push(source(r, ti, "GeomStepList form/adjust SnapshotData.m_geomTops; GeometryTopology.m_complete/m_faces.m_id"));
            }
        }
        self.walls.push(Wall {
            origin: *origin,
            x: *x,
            y: *y,
            openings,
            owner: source(r, 0, "").owner,
            type_id: identifier(&root["m_WallAttributesId"])?,
            planes: faces,
            cut_face_tags,
            paint,
            sources,
        });
        Ok(())
    }
    pub(super) fn project(&self, scene: &native_scene::Scene) -> (Vec<Surface>, Vec<String>) {
        let mut surfaces = Vec::new();
        let mut diagnostics = Vec::new();
        for wall in &self.walls {
            let result = (|| -> Result<(Vec<Surface>, usize)> {
                let element = scene
                    .elements
                    .iter()
                    .find(|e| {
                        e.id == wall.owner.element_id
                            && e.identity.unique_id == wall.owner.unique_id
                    })
                    .ok_or_else(|| anyhow::anyhow!("wall current mesh absent"))?;
                ensure!(
                    matches!(
                        element.status.as_str(),
                        "decoded_wall_with_rectangular_cuts" | "decoded_uncut_wall_surface_model"
                    ) && element.meshes.len() == 1,
                    "wall mesh outside qualified scope: {:?}",
                    element.diagnostics
                );
                let (base, mat_source) = self
                    .materials
                    .get(&wall.type_id)
                    .ok_or_else(|| anyhow::anyhow!("single-layer wall material unavailable"))?;
                ensure!(*base > 0, "ByCategory wall material unresolved");
                let mesh = &element.meshes[0];
                let mut result = Vec::new();
                let mut assigned = BTreeSet::new();
                let mut opening_planes = Vec::new();
                let mut opening_sources = Vec::new();
                let mut seen_tags = wall.planes.iter().map(|p| p.tag).collect::<BTreeSet<_>>();
                for (id, tags, step_source) in &wall.openings {
                    let (host, corners, opening_source) = self
                        .openings
                        .get(id)
                        .ok_or_else(|| anyhow::anyhow!("opening face owner unresolved"))?;
                    ensure!(
                        *host == wall.owner.element_id as i64,
                        "opening surface host mismatch"
                    );
                    let points = corners.map(|p| {
                        let delta = sub(p, wall.origin);
                        [dot(delta, wall.x), dot(delta, wall.y)]
                    });
                    let bounds = [
                        points[0][0].min(points[1][0]),
                        points[0][1].min(points[1][1]),
                        points[0][0].max(points[1][0]),
                        points[0][1].max(points[1][1]),
                    ];
                    for (index, point, normal, role) in [
                        (
                            0,
                            add(wall.origin, scale(wall.y, bounds[1])),
                            wall.y,
                            "opening_sill",
                        ),
                        (
                            1,
                            add(wall.origin, scale(wall.y, bounds[3])),
                            scale(wall.y, -1.),
                            "opening_lintel",
                        ),
                        (
                            2,
                            add(wall.origin, scale(wall.x, bounds[0])),
                            wall.x,
                            "opening_start_jamb",
                        ),
                        (
                            3,
                            add(wall.origin, scale(wall.x, bounds[2])),
                            scale(wall.x, -1.),
                            "opening_end_jamb",
                        ),
                    ] {
                        ensure!(
                            tags[index] == -1 || (tags[index] > 0 && seen_tags.insert(tags[index])),
                            "invalid/duplicate opening face tag"
                        );
                        opening_planes.push(FacePlane {
                            tag: tags[index],
                            point,
                            normal,
                            role,
                            bounds: Some(bounds),
                        });
                    }
                    opening_sources.extend([step_source.clone(), opening_source.clone()]);
                }
                if let Some(current) = &wall.cut_face_tags {
                    ensure!(
                        current.is_subset(&seen_tags)
                            && wall.planes.iter().all(|p| current.contains(&p.tag)),
                        "wall history/current topology face populations differ"
                    );
                    opening_planes = merge_opening_planes(&opening_planes, current)?;
                    seen_tags = current.clone();
                }
                ensure!(
                    wall.paint.keys().all(|tag| seen_tags.contains(tag)),
                    "paint refers to an unprojected wall surface"
                );
                for face in wall.planes.iter().chain(&opening_planes) {
                    let triangles = mesh
                        .triangles
                        .iter()
                        .enumerate()
                        .filter(|(_, t)| {
                            t.iter().all(|i| {
                                let p = mesh.vertices[*i as usize];
                                dot(sub(p, face.point), face.normal).abs() < 1e-8
                                    && face.bounds.is_none_or(|b| {
                                        let d = sub(p, wall.origin);
                                        let u = dot(d, wall.x);
                                        let v = dot(d, wall.y);
                                        u >= b[0] - 1e-8
                                            && u <= b[2] + 1e-8
                                            && v >= b[1] - 1e-8
                                            && v <= b[3] + 1e-8
                                    })
                            })
                        })
                        .collect::<Vec<_>>();
                    ensure!(!triangles.is_empty(), "wall face plane has no triangles");
                    for (i, _) in &triangles {
                        ensure!(
                            assigned.insert(*i),
                            "wall triangle assigned to multiple face tags"
                        );
                    }
                    let triangles = triangles.into_iter().map(|(_, t)| *t).collect::<Vec<_>>();
                    let (normal, area) = triangle_plane(mesh, &triangles)?;
                    ensure!(
                        dot(normal, face.normal) > 1. - 1e-8,
                        "wall face orientation mismatch"
                    );
                    let painted = wall.paint.get(&face.tag).copied();
                    let mut sources = wall.sources.clone();
                    sources.push(mat_source.clone());
                    sources.extend(opening_sources.clone());
                    result.push(Surface {
                        owner: wall.owner.clone(),
                        geometry_tag: face.tag,
                        role: face.role,
                        vertices: mesh.vertices.clone(),
                        triangles,
                        normal,
                        area_square_feet: area,
                        base_material_id: *base,
                        painted_material_id: painted,
                        is_painted: painted.is_some(),
                        effective_material_id: painted.unwrap_or(*base),
                        sources,
                    });
                }
                Ok((result, mesh.triangles.len() - assigned.len()))
            })();
            match result {
                Ok((found, remaining)) => {
                    surfaces.extend(found);
                    if remaining > 0 {
                        diagnostics.push(format!("wall {}: {remaining} opening-surface triangles have unresolved native face-tag association",wall.owner.element_id));
                    }
                }
                Err(e) => diagnostics.push(format!("wall {}: {e:#}", wall.owner.element_id)),
            }
        }
        for wall in &self.joined {
            match self.project_joined(wall, scene) {
                Ok(found) => surfaces.extend(found),
                Err(e) => diagnostics.push(format!("wall {}: {e:#}", wall.owner.element_id)),
            }
        }
        (surfaces, diagnostics)
    }
    fn project_joined(
        &self,
        wall: &JoinedWall,
        scene: &native_scene::Scene,
    ) -> Result<Vec<Surface>> {
        let element = scene
            .elements
            .iter()
            .find(|e| e.id == wall.owner.element_id && e.identity.unique_id == wall.owner.unique_id)
            .ok_or_else(|| anyhow::anyhow!("joined wall mesh absent"))?;
        ensure!(
            element.status == "decoded_joined_wall" && element.meshes.len() == 1,
            "joined wall mesh unqualified: {:?}",
            element.diagnostics
        );
        let geometry = element
            .geometry
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("joined wall geometry missing"))?;
        let planes = arr(&geometry["face_planes"])?;
        ensure!(!planes.is_empty(), "joined wall face history unresolved");
        let (base, material_source) = self
            .materials
            .get(&wall.type_id)
            .ok_or_else(|| anyhow::anyhow!("joined wall material unavailable"))?;
        ensure!(*base > 0, "ByCategory joined wall material unresolved");
        let mesh = &element.meshes[0];
        let mut sources = wall.sources.clone();
        sources.push(material_source.clone());
        for witness in arr(&geometry["sources"])? {
            let identity = &witness["identity"];
            let record = &witness["record"];
            let text = |v: &Value| -> Result<String> {
                Ok(v.as_str()
                    .ok_or_else(|| anyhow::anyhow!("join source string absent"))?
                    .into())
            };
            sources.push(Source {
                owner: Identity {
                    element_id: identity["element_id"]
                        .as_u64()
                        .ok_or_else(|| anyhow::anyhow!("join source owner missing"))?,
                    unique_id: text(&identity["unique_id"])?,
                },
                object_index: usize::try_from(
                    witness["geometry_steps_object"]
                        .as_u64()
                        .ok_or_else(|| anyhow::anyhow!("join source object missing"))?,
                )?,
                field: "owned join operations, reference planes and complete current face topology"
                    .into(),
                stream: text(&record["stream"])?,
                body_sha256: text(&record["body_sha256"])?,
                group_record_offset: usize::try_from(
                    record["group_record_offset"]
                        .as_u64()
                        .ok_or_else(|| anyhow::anyhow!("join source offset missing"))?,
                )?,
            });
        }
        let mut assigned = BTreeSet::new();
        let mut tags = BTreeSet::new();
        let mut out = Vec::new();
        for face in planes {
            let tag = int(&face["tag"])?;
            ensure!(
                tag > 0 && tags.insert(tag),
                "duplicate/invalid joined face tag"
            );
            let point = vector(&face["point"])?;
            let normal = vector(&face["normal"])?;
            ensure!(
                (dot(normal, normal) - 1.).abs() < 1e-8,
                "joined face normal not normalized"
            );
            let role = match face["role"].as_str() {
                Some("wall_positive_side") => "wall_positive_side",
                Some("wall_negative_side") => "wall_negative_side",
                Some("wall_start") => "wall_start",
                Some("wall_end") => "wall_end",
                Some("wall_bottom") => "wall_bottom",
                Some("wall_top") => "wall_top",
                Some("wall_explicit_cut") => "wall_explicit_cut",
                _ => anyhow::bail!("unqualified joined face role"),
            };
            let selected = mesh
                .triangles
                .iter()
                .enumerate()
                .filter(|(_, t)| {
                    t.iter()
                        .all(|i| dot(sub(mesh.vertices[*i as usize], point), normal).abs() < 1e-8)
                })
                .collect::<Vec<_>>();
            ensure!(!selected.is_empty(), "joined face has no triangles");
            for (i, _) in &selected {
                ensure!(
                    assigned.insert(*i),
                    "joined triangle has ambiguous face identity"
                );
            }
            let triangles = selected.into_iter().map(|(_, t)| *t).collect::<Vec<_>>();
            let (measured, area) = triangle_plane(mesh, &triangles)?;
            ensure!(
                dot(measured, normal) > 1. - 1e-8,
                "joined face orientation differs"
            );
            let painted = wall.paint.get(&tag).copied();
            out.push(Surface {
                owner: wall.owner.clone(),
                geometry_tag: tag,
                role,
                vertices: mesh.vertices.clone(),
                triangles,
                normal: measured,
                area_square_feet: area,
                base_material_id: *base,
                painted_material_id: painted,
                is_painted: painted.is_some(),
                effective_material_id: painted.unwrap_or(*base),
                sources: sources.clone(),
            });
        }
        ensure!(
            assigned.len() == mesh.triangles.len(),
            "unassigned joined wall triangles"
        );
        ensure!(
            wall.paint.keys().all(|t| tags.contains(t)),
            "joined paint references unprojected face"
        );
        Ok(out)
    }
}

/// Extend a retained plane only through a touching coplanar opening whose face
/// was explicitly omitted from the complete native topology. No tag is invented.
fn merge_opening_planes(planes: &[FacePlane], current: &BTreeSet<i64>) -> Result<Vec<FacePlane>> {
    let mut out = planes
        .iter()
        .filter(|p| current.contains(&p.tag))
        .cloned()
        .collect::<Vec<_>>();
    for face in &mut out {
        let mut included = BTreeSet::new();
        loop {
            let count = included.len();
            for (i, candidate) in planes.iter().enumerate() {
                if current.contains(&candidate.tag)
                    || included.contains(&i)
                    || face.role != candidate.role
                    || dot(face.normal, candidate.normal) < 1. - 1e-10
                    || dot(sub(candidate.point, face.point), face.normal).abs() > 1e-8
                {
                    continue;
                }
                let a = face
                    .bounds
                    .ok_or_else(|| anyhow::anyhow!("opening bounds missing"))?;
                let b = candidate
                    .bounds
                    .ok_or_else(|| anyhow::anyhow!("opening candidate bounds missing"))?;
                // Same plane fixes one profile coordinate. The interval along
                // that plane must touch/overlap; disconnected faces cannot merge.
                let axis = if matches!(face.role, "opening_sill" | "opening_lintel") {
                    0
                } else {
                    1
                };
                if a[axis] <= b[axis + 2] + 1e-8 && b[axis] <= a[axis + 2] + 1e-8 {
                    face.bounds = Some([
                        a[0].min(b[0]),
                        a[1].min(b[1]),
                        a[2].max(b[2]),
                        a[3].max(b[3]),
                    ]);
                    included.insert(i);
                }
            }
            if included.len() == count {
                break;
            }
        }
    }
    Ok(out)
}

#[cfg(test)]
mod opening_history_tests {
    use super::*;
    fn sill(tag: i64, bounds: [f64; 4]) -> FacePlane {
        FacePlane {
            tag,
            normal: [0., 0., 1.],
            point: [0., 0., 2.],
            role: "opening_sill",
            bounds: Some(bounds),
        }
    }
    #[test]
    fn omitted_touching_face_extends_retained_tag_but_disconnected_does_not() {
        let planes = vec![
            sill(23, [1., 2., 4., 6.]),
            sill(-1, [4., 2., 7., 6.]),
            sill(-1, [9., 2., 12., 6.]),
        ];
        let merged = merge_opening_planes(&planes, &BTreeSet::from([23])).unwrap();
        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].tag, 23);
        assert_eq!(merged[0].bounds, Some([1., 2., 7., 6.]));
    }
}
