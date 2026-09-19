//! Bounded floor/wall surfaces and saved material overrides, with face-tag provenance.
#[path = "native_wall_surfaces.rs"]
mod wall_surfaces;
use crate::{
    RevitFile, native_document::Record, native_equipment::Identity, native_metadata::identifier,
    native_parameters::ObjectGraph, native_scene,
};
use anyhow::{Result, ensure};
use serde::Serialize;
use serde_json::Value;
use std::collections::BTreeMap;
#[derive(Debug, Clone, Serialize)]
pub struct Source {
    pub owner: Identity,
    pub object_index: usize,
    pub field: String,
    pub stream: String,
    pub body_sha256: String,
    pub group_record_offset: usize,
}
#[derive(Debug, Serialize)]
pub struct Surface {
    pub owner: Identity,
    pub geometry_tag: i64,
    pub role: &'static str,
    pub vertices: Vec<[f64; 3]>,
    pub triangles: Vec<[u32; 3]>,
    pub normal: [f64; 3],
    pub area_square_feet: f64,
    pub base_material_id: i64,
    pub painted_material_id: Option<i64>,
    pub is_painted: bool,
    pub effective_material_id: i64,
    pub sources: Vec<Source>,
}
#[derive(Debug, Serialize)]
pub struct Inventory {
    pub format: &'static str,
    pub complete_surface_parity: bool,
    pub coordinate_space: &'static str,
    pub length_unit: &'static str,
    pub surfaces: Vec<Surface>,
    pub saved_texture_mapping: crate::native_texture_mapping::Inventory,
    pub diagnostics: Vec<String>,
    pub unresolved_scope: Vec<&'static str>,
}
struct Side {
    tag: i64,
    a: [f64; 2],
    b: [f64; 2],
    source: Source,
}
struct Floor {
    owner: Identity,
    type_id: i64,
    top: f64,
    bottom: f64,
    cap_tags: [i64; 2],
    sides: Vec<Side>,
    paint: BTreeMap<i64, i64>,
    sources: Vec<Source>,
}
#[derive(Default)]
pub struct InventoryBuilder {
    floors: Vec<Floor>,
    graphics_owner_ids: std::collections::BTreeSet<u64>,
    walls: wall_surfaces::Context,
    materials: BTreeMap<i64, (i64, Source)>,
    diagnostics: Vec<String>,
}
fn source(r: &Record, i: usize, f: &str) -> Source {
    Source {
        owner: Identity {
            element_id: r.identity.element_id,
            unique_id: r.identity.unique_id.clone(),
        },
        object_index: i,
        field: f.into(),
        stream: r.source.stream.clone(),
        body_sha256: r.source.body_sha256.clone(),
        group_record_offset: r.source.group_record_offset,
    }
}
fn target<'a>(g: &'a ObjectGraph, i: usize, p: &Value, c: &str) -> Result<(usize, &'a Value)> {
    let es: Vec<_> = g
        .edges
        .iter()
        .filter(|e| {
            e.source_object_index == i && Some(e.pointer_offset as u64) == p["offset"].as_u64()
        })
        .collect();
    ensure!(es.len() == 1, "expected owned {c} edge");
    let n = es[0].target_object_index;
    let o = g
        .objects
        .get(n)
        .ok_or_else(|| anyhow::anyhow!("surface edge out of graph"))?;
    ensure!(o.class_name == c, "expected {c}, found {}", o.class_name);
    Ok((n, &o.fields))
}
fn arr(v: &Value) -> Result<&Vec<Value>> {
    v.as_array()
        .ok_or_else(|| anyhow::anyhow!("surface array absent"))
}
fn int(v: &Value) -> Result<i64> {
    v.as_i64()
        .ok_or_else(|| anyhow::anyhow!("surface integer absent"))
}
fn num(v: &Value) -> Result<f64> {
    let n = v
        .as_f64()
        .ok_or_else(|| anyhow::anyhow!("surface number absent"))?;
    ensure!(n.is_finite(), "nonfinite surface coordinate");
    Ok(n)
}
fn side_profiles(r: &Record, g: &ObjectGraph, tags: &BTreeMap<i64, i64>) -> Result<Vec<Side>> {
    let (ci, c) = target(g, 0, &g.objects[0].fields["m_cellList"], "CellList")?;
    let mut helpers = Vec::new();
    for p in arr(&c["m_cells"])? {
        let es: Vec<_> = g
            .edges
            .iter()
            .filter(|e| {
                e.source_object_index == ci && Some(e.pointer_offset as u64) == p["offset"].as_u64()
            })
            .collect();
        ensure!(es.len() == 1, "floor cell edge");
        let i = es[0].target_object_index;
        if g.objects[i].class_name == "FloorExtrusionHelper" {
            helpers.push(i);
        }
    }
    ensure!(helpers.len() == 1, "floor extrusion helper ambiguous");
    let hi = helpers[0];
    let h = &g.objects[hi].fields;
    ensure!(
        h["m_complete"].as_bool() == Some(true),
        "incomplete floor helper"
    );
    let loops = arr(&h["m_pCurveLoops"])?;
    ensure!(
        loops.len() == 1,
        "multiple floor profile loops unqualified for side tags"
    );
    let (li, l) = target(g, hi, &loops[0], "CurveLoop")?;
    ensure!(l["m_open"].as_bool() == Some(false), "open floor loop");
    let mut result = Vec::new();
    let mut seen = std::collections::BTreeSet::new();
    let mut profile_z: Option<f64> = None;
    for p in arr(&l["m_curves"])? {
        let (i, c) = target(g, li, p, "GLine")?;
        let key = int(&c["m_GInfo"]["m_tag"])?;
        ensure!(seen.insert(key), "duplicate floor curve tag");
        let tag = *tags
            .get(&key)
            .ok_or_else(|| anyhow::anyhow!("side history does not resolve curve tag"))?;
        let mut points = [[0.; 2]; 2];
        for (j, point) in points.iter_mut().enumerate() {
            let t = num(&c["m_endParams"][j])?;
            for (k, value) in point.iter_mut().enumerate() {
                *value = num(&c["m_origin"][k])? + t * num(&c["m_dirVec"][k])?;
            }
        }
        let z = num(&c["m_origin"][2])?;
        // The horizontal sketch plane need not equal the extrusion's top.
        // A vertical element move can preserve sketch Z while updating the
        // independent cap planes. finish() verifies both actual mesh heights.
        ensure!(
            num(&c["m_dirVec"][2])?.abs() < 1e-9
                && profile_z.is_none_or(|previous| (previous - z).abs() < 1e-8),
            "floor profile is not a common horizontal plane"
        );
        profile_z = Some(z);
        result.push(Side {
            tag,
            a: points[0],
            b: points[1],
            source: source(
                r,
                i,
                "FloorExtrusionHelper.m_pCurveLoops.m_curves.m_GInfo.m_tag",
            ),
        });
    }
    ensure!(seen.len() == tags.len(), "unresolved side history entries");
    ensure!(result.len() >= 3, "insufficient profile sides");
    for i in 0..result.len() {
        let a = result[i].b;
        let b = result[(i + 1) % result.len()].a;
        ensure!(
            (a[0] - b[0]).abs() < 1e-8 && (a[1] - b[1]).abs() < 1e-8,
            "owned profile not closed in saved order"
        );
    }
    Ok(result)
}
fn side_triangles(
    mesh: &native_scene::Mesh,
    s: &Side,
    bottom: f64,
    top: f64,
) -> Result<Vec<[u32; 3]>> {
    let d = [s.b[0] - s.a[0], s.b[1] - s.a[1]];
    let len = d[0].hypot(d[1]);
    ensure!(len > 1e-9, "zero side segment");
    let triangles: Vec<_> = mesh
        .triangles
        .iter()
        .filter(|t| {
            let vertices: Vec<_> = t.iter().map(|i| mesh.vertices[*i as usize]).collect();
            let varying_z = vertices
                .iter()
                .any(|v| (v[2] - vertices[0][2]).abs() > 1e-9);
            varying_z
                && vertices.iter().all(|v| {
                    let p = [v[0] - s.a[0], v[1] - s.a[1]];
                    let u = (p[0] * d[0] + p[1] * d[1]) / (len * len);
                    (p[0] * d[1] - p[1] * d[0]).abs() / len < 1e-8
                        && (-1e-9..=1. + 1e-9).contains(&u)
                        && v[2] >= bottom - 1e-8
                        && v[2] <= top + 1e-8
                })
        })
        .copied()
        .collect();
    ensure!(
        triangles.len() == 2,
        "side not exactly an uncut extrusion rectangle"
    );
    Ok(triangles)
}
fn triangle_plane(mesh: &native_scene::Mesh, triangles: &[[u32; 3]]) -> Result<([f64; 3], f64)> {
    let mut normal = None;
    let mut area = 0.;
    for t in triangles {
        let a = mesh.vertices[t[0] as usize];
        let b = mesh.vertices[t[1] as usize];
        let c = mesh.vertices[t[2] as usize];
        let u = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
        let v = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];
        let cross = [
            u[1] * v[2] - u[2] * v[1],
            u[2] * v[0] - u[0] * v[2],
            u[0] * v[1] - u[1] * v[0],
        ];
        let norm = cross.iter().map(|x| x * x).sum::<f64>().sqrt();
        ensure!(norm > 1e-12, "zero surface triangle");
        let n = cross.map(|x| x / norm);
        if let Some(previous) = normal {
            let previous: [f64; 3] = previous;
            ensure!(
                previous.iter().zip(n).all(|(a, b)| (a - b).abs() < 1e-8),
                "surface orientation inconsistent"
            );
        } else {
            normal = Some(n);
        }
        area += norm / 2.;
    }
    Ok((
        normal.ok_or_else(|| anyhow::anyhow!("empty surface"))?,
        area,
    ))
}
impl InventoryBuilder {
    pub fn ingest(&mut self, r: &Record) -> Result<()> {
        // Saved graphics are independently readable even when operation-derived
        // surface reconstruction refuses an owner. Never select them by success.
        if r.channel == 102 && matches!(r.class_name.as_deref(), Some("Floor" | "SWall")) {
            self.graphics_owner_ids.insert(r.identity.element_id);
        }
        if let Err(error) = self.walls.ingest(r) {
            self.diagnostics
                .push(format!("{}: {error:#}", r.identity.element_id));
        }
        if r.channel == 102
            && matches!(r.class_name.as_deref(), Some("Floor" | "FloorAttributes"))
            && let Err(e) = self.project(r)
        {
            self.diagnostics
                .push(format!("{}: {e:#}", r.identity.element_id));
        }
        Ok(())
    }
    fn project(&mut self, r: &Record) -> Result<()> {
        let g = r
            .graph
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("surface graph unavailable"))?;
        let root = g
            .objects
            .first()
            .ok_or_else(|| anyhow::anyhow!("empty surface graph"))?;
        let f = &root.fields;
        if root.class_name == "FloorAttributes" {
            let (i, c) = target(g, 0, &f["m_pCompoundStructure"], "CompoundStructure")?;
            let layers = arr(&c["m_layers"])?;
            ensure!(
                layers.len() == 1 && int(&c["m_variableLayerIdx"])? == -1,
                "floor surface base material outside single constant layer scope"
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
        let (ti, t) = target(g, 0, &f["m_pGeomTable"], "GeomTable")?;
        let mut paint = BTreeMap::new();
        for p in arr(&t["m_materialMarkers"])? {
            let (_, m) = target(g, ti, p, "GeomMaterialMarker")?;
            ensure!(
                paint
                    .insert(int(&m["m_geomTag"])?, identifier(&m["m_materialId"])?)
                    .is_none(),
                "duplicate face paint tag"
            );
        }
        let (si, steps) = target(g, 0, &f["m_geomSteps"], "GeomStepList")?;
        for name in [
            "m_bRepCutOutGList",
            "m_bRepPostCutOutGList",
            "m_bRepAdjustGList",
            "m_bRepTweakGList",
        ] {
            ensure!(
                arr(&steps[name])?.is_empty(),
                "floor post-extrusion changes unsupported"
            );
        }
        let mut caps = None;
        let mut side_tags = BTreeMap::new();
        let mut src = vec![
            source(r, 0, "m_top/m_bottom"),
            source(r, ti, "m_materialMarkers"),
        ];
        for p in arr(&steps["m_bRepFormGList"])? {
            let es: Vec<_> = g
                .edges
                .iter()
                .filter(|e| {
                    e.source_object_index == si
                        && Some(e.pointer_offset as u64) == p["offset"].as_u64()
                })
                .collect();
            ensure!(es.len() == 1, "floor step edge");
            let i = es[0].target_object_index;
            let o = &g.objects[i];
            match o.class_name.as_str() {
                "ExtrusionGStep" => {
                    ensure!(caps.is_none(), "multiple floor extrusion generators");
                    let mut tags = [None, None];
                    for row in arr(&o.fields["m_faceHistTable"])? {
                        if row["m_faceHist"]["m_keys"][0].as_i64() == Some(3)
                            && row["m_faceHist"]["m_keys"][2].as_i64() == Some(-1)
                        {
                            ensure!(
                                side_tags
                                    .insert(
                                        int(&row["m_faceHist"]["m_keys"][1])?,
                                        int(&row["m_id"])?
                                    )
                                    .is_none(),
                                "duplicate side history"
                            );
                        }
                        for (j, key) in [1, 2].iter().enumerate() {
                            if row["m_faceHist"]["m_keys"] == serde_json::json!([key, -1, -1]) {
                                ensure!(
                                    tags[j].replace(int(&row["m_id"])?).is_none(),
                                    "duplicate cap history"
                                );
                            }
                        }
                    }
                    caps = Some([
                        tags[0].ok_or_else(|| anyhow::anyhow!("top cap tag absent"))?,
                        tags[1].ok_or_else(|| anyhow::anyhow!("bottom cap tag absent"))?,
                    ]);
                    src.push(source(r, i, "m_faceHistTable"));
                }
                "FloorRefPlanesGStep" => {}
                "SlabShapeEditGStep" => ensure!(
                    o.fields["m_dormant"].as_bool() == Some(true),
                    "active slab shaping unsupported"
                ),
                _ => anyhow::bail!("unsupported floor form step {}", o.class_name),
            }
        }
        let sides = side_profiles(r, g, &side_tags)?;
        self.floors.push(Floor {
            owner: source(r, 0, "").owner,
            type_id: identifier(&f["m_floorAttributesId"])?,
            top: num(&f["m_top"])?,
            bottom: num(&f["m_bottom"])?,
            cap_tags: caps.ok_or_else(|| anyhow::anyhow!("floor extrusion absent"))?,
            sides,
            paint,
            sources: src,
        });
        Ok(())
    }
    pub fn finish(mut self, file: &mut RevitFile) -> Result<Inventory> {
        let mut surfaces = Vec::new();
        if !self.floors.is_empty() || !self.walls.is_empty() {
            match native_scene::extract(file) {
                Err(e) => self
                    .diagnostics
                    .push(format!("native surface mesh unavailable: {e:#}")),
                Ok(scene) => {
                    for floor in &self.floors {
                        let result = (|| -> Result<Vec<Surface>> {
                            let e = scene
                                .elements
                                .iter()
                                .find(|e| {
                                    e.id == floor.owner.element_id
                                        && e.identity.unique_id == floor.owner.unique_id
                                })
                                .ok_or_else(|| {
                                    anyhow::anyhow!("floor current mesh owner absent")
                                })?;
                            ensure!(
                                e.status == "decoded_horizontal_polygonal_floor"
                                    && e.meshes.len() == 1,
                                "floor mesh outside qualified extrusion scope: {:?}",
                                e.diagnostics
                            );
                            let (base, mat_source) =
                                self.materials.get(&floor.type_id).ok_or_else(|| {
                                    anyhow::anyhow!("single-layer floor material unavailable")
                                })?;
                            ensure!(
                                *base > 0,
                                "ByCategory/unresolved layer material cannot establish effective material"
                            );
                            let mesh = &e.meshes[0];
                            for v in &mesh.vertices {
                                ensure!(
                                    (v[2] - floor.top).abs() < 1e-8
                                        || (v[2] - floor.bottom).abs() < 1e-8,
                                    "mesh outside saved cap heights"
                                );
                                ensure!(
                                    floor.sides.iter().any(|s| [s.a, s.b].iter().any(|p| (p[0]
                                        - v[0])
                                        .abs()
                                        < 1e-8
                                        && (p[1] - v[1]).abs() < 1e-8)),
                                    "mesh vertex outside owned profile"
                                );
                            }
                            let profile_area = (floor
                                .sides
                                .iter()
                                .map(|s| s.a[0] * s.b[1] - s.b[0] * s.a[1])
                                .sum::<f64>()
                                / 2.)
                                .abs();
                            let mut result = Vec::new();
                            for (j, (z, role, normal)) in [
                                (floor.top, "top_cap", [0., 0., 1.]),
                                (floor.bottom, "bottom_cap", [0., 0., -1.]),
                            ]
                            .into_iter()
                            .enumerate()
                            {
                                let triangles: Vec<_> = mesh
                                    .triangles
                                    .iter()
                                    .filter(|t| {
                                        t.iter().all(|i| {
                                            (mesh.vertices[*i as usize][2] - z).abs() < 1e-8
                                        })
                                    })
                                    .copied()
                                    .collect();
                                ensure!(!triangles.is_empty(), "cap mesh absent");
                                let mut area = 0.;
                                for t in &triangles {
                                    let a = mesh.vertices[t[0] as usize];
                                    let b = mesh.vertices[t[1] as usize];
                                    let c = mesh.vertices[t[2] as usize];
                                    let signed = ((b[0] - a[0]) * (c[1] - a[1])
                                        - (b[1] - a[1]) * (c[0] - a[0]))
                                        * normal[2]
                                        / 2.;
                                    ensure!(signed > 0., "cap orientation mismatch");
                                    area += signed;
                                }
                                ensure!(
                                    (area - profile_area).abs() < 1e-7,
                                    "cap mesh differs from owned profile area"
                                );
                                let tag = floor.cap_tags[j];
                                let painted = floor.paint.get(&tag).copied();
                                let mut sources = floor.sources.clone();
                                sources.push(mat_source.clone());
                                result.push(Surface {
                                    owner: floor.owner.clone(),
                                    geometry_tag: tag,
                                    role,
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
                            for side in &floor.sides {
                                let triangles =
                                    side_triangles(mesh, side, floor.bottom, floor.top)?;
                                let (normal, area) = triangle_plane(mesh, &triangles)?;
                                let painted = floor.paint.get(&side.tag).copied();
                                let mut sources = floor.sources.clone();
                                sources.push(mat_source.clone());
                                sources.push(side.source.clone());
                                result.push(Surface {
                                    owner: floor.owner.clone(),
                                    geometry_tag: side.tag,
                                    role: "side_face",
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
                            ensure!(
                                result.iter().map(|s| s.triangles.len()).sum::<usize>()
                                    == mesh.triangles.len(),
                                "unassigned/duplicate floor surface triangles"
                            );
                            Ok(result)
                        })();
                        match result {
                            Ok(s) => surfaces.extend(s),
                            Err(e) => self
                                .diagnostics
                                .push(format!("floor {}: {e:#}", floor.owner.element_id)),
                        }
                    }
                    let (wall_surfaces, wall_diagnostics) = self.walls.project(&scene);
                    surfaces.extend(wall_surfaces);
                    self.diagnostics.extend(wall_diagnostics);
                }
            }
        }
        let ids = self.graphics_owner_ids;
        let saved_texture_mapping = match crate::native_texture_mapping::read(file, ids) {
            Ok(v) => v,
            Err(e) => crate::native_texture_mapping::Inventory {
                diagnostics: vec![
                    serde_json::json!({"reason": format!("texture graphics extraction: {e:#}")}),
                ],
                ..Default::default()
            },
        };
        Ok(Inventory {
            format: "rvt-native-surfaces/v1",
            complete_surface_parity: false,
            coordinate_space: "document_internal",
            length_unit: "feet",
            surfaces,
            saved_texture_mapping,
            diagnostics: self.diagnostics,
            unresolved_scope: vec![
                "multi_loop_side_face_history_mapping",
                "multi_layer_face_materials",
                "wall_opening_face_tag_mapping_and_complex_booleans",
                "curved_and_shape_edited_surfaces",
                "nonplanar_or_nonunit_texture_placers_and_regenerated_UVs",
                "complete_texture_and_appearance_rendering",
            ],
        })
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn rectangle() -> native_scene::Mesh {
        native_scene::Mesh {
            vertices: vec![[0., 0., 0.], [3., 0., 0.], [3., 0., 2.], [0., 0., 2.]],
            triangles: vec![[0, 1, 2], [0, 2, 3]],
        }
    }
    #[test]
    fn surface_orientation_and_area_are_geometric() {
        let m = rectangle();
        let (n, a) = triangle_plane(&m, &m.triangles).unwrap();
        assert_eq!(n, [0., -1., 0.]);
        assert_eq!(a, 6.);
        let mut reversed = m.triangles.clone();
        reversed[1].swap(0, 1);
        assert!(triangle_plane(&m, &reversed).is_err());
    }
    #[test]
    fn side_selection_requires_exact_profile_support() {
        let m = rectangle();
        let src = Source {
            owner: Identity {
                element_id: 1,
                unique_id: "test".into(),
            },
            object_index: 0,
            field: "test".into(),
            stream: "test".into(),
            body_sha256: "test".into(),
            group_record_offset: 0,
        };
        let mut s = Side {
            tag: 3,
            a: [0., 0.],
            b: [3., 0.],
            source: src,
        };
        assert_eq!(side_triangles(&m, &s, 0., 2.).unwrap().len(), 2);
        s.a[1] = 1.;
        s.b[1] = 1.;
        assert!(side_triangles(&m, &s, 0., 2.).is_err());
    }
}
