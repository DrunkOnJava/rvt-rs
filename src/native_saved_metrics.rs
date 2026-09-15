//! Geometric measurements of complete, coherently oriented saved triangle shells.
//!
//! These measurements do not assign Revit's category-dependent `Area` parameter.
//! Surface area is the sum of triangle areas; volume is the oriented shell integral.
use crate::native_saved_mesh::GraphicsMeshes;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::BTreeMap;

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct SavedMetrics {
    pub volume: Option<f64>,
    /// Oriented closed-chain integral, including even-incidence nonmanifold edges.
    /// This is not a boolean-union or simple-manifold guarantee.
    pub closed_chain_volume: Option<f64>,
    pub surface_area: Option<f64>,
    pub diagnostics: Vec<String>,
    pub sources: Vec<Value>,
}
fn sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    std::array::from_fn(|i| a[i] - b[i])
}
fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    (0..3).map(|i| a[i] * b[i]).sum()
}
fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
const WELD: f64 = 1e-8;
/// Measure selected graphics, refusing volume for open, nonmanifold, or
/// inconsistently wound shells. Collinear boundary subdivisions are reconciled
/// geometrically, so a face triangulation's T-junction is not an open shell.
/// Intersecting closed shells are not boolean-unioned by this function.
pub fn compute(meshes: &GraphicsMeshes) -> SavedMetrics {
    if !meshes.diagnostics.is_empty() {
        return SavedMetrics {
            diagnostics: vec!["incomplete graphics selection".into()],
            ..Default::default()
        };
    }
    let mut carriers: BTreeMap<(Option<u64>, usize), Vec<_>> = BTreeMap::new();
    for p in &meshes.primitives {
        carriers
            .entry((p.source_owner_id, p.object_index))
            .or_default()
            .push(p.clone());
    }
    if carriers.len() <= 1 {
        return measure_shell(meshes);
    }
    let mut out = SavedMetrics {
        volume: Some(0.),
        closed_chain_volume: Some(0.),
        surface_area: Some(0.),
        ..Default::default()
    };
    for ((owner, object), primitives) in carriers {
        let m = measure_shell(&GraphicsMeshes {
            primitives,
            ..Default::default()
        });
        out.volume = out.volume.zip(m.volume).map(|(a, b)| a + b);
        out.closed_chain_volume = out
            .closed_chain_volume
            .zip(m.closed_chain_volume)
            .map(|(a, b)| a + b);
        out.surface_area = out.surface_area.zip(m.surface_area).map(|(a, b)| a + b);
        out.diagnostics.extend(
            m.diagnostics
                .into_iter()
                .map(|d| format!("carrier {owner:?}/{object}: {d}")),
        );
        out.sources.push(
            json!({"source_owner_id":owner,"geometry_object_index":object,"measurement":m.sources}),
        );
    }
    out.sources.push(
        json!({"method":"sum_individual_saved_geometry_carriers","boolean_union_performed":false}),
    );
    out
}
fn measure_shell(meshes: &GraphicsMeshes) -> SavedMetrics {
    let mut out = SavedMetrics::default();
    if !meshes.diagnostics.is_empty() {
        out.diagnostics.push("incomplete graphics selection".into());
        return out;
    }
    let mut points: Vec<[f64; 3]> = Vec::new();
    let mut triangles = Vec::new();
    let mut triangle_owners = Vec::new();
    for (primitive_index, primitive) in meshes.primitives.iter().enumerate() {
        let mut ids = Vec::new();
        for &p in &primitive.vertices {
            if !p.iter().all(|x| x.is_finite()) {
                out.diagnostics.push("nonfinite vertex".into());
                return out;
            }
            let id = points
                .iter()
                .position(|q| dot(sub(p, *q), sub(p, *q)) <= WELD * WELD)
                .unwrap_or_else(|| {
                    points.push(p);
                    points.len() - 1
                });
            ids.push(id);
        }
        for t in &primitive.triangles {
            let Some(a) = ids.get(t[0] as usize) else {
                out.diagnostics.push("triangle index out of bounds".into());
                return out;
            };
            let Some(b) = ids.get(t[1] as usize) else {
                out.diagnostics.push("triangle index out of bounds".into());
                return out;
            };
            let Some(c) = ids.get(t[2] as usize) else {
                out.diagnostics.push("triangle index out of bounds".into());
                return out;
            };
            triangles.push([*a, *b, *c]);
            triangle_owners.push(primitive_index);
        }
    }
    if triangles.is_empty() {
        out.diagnostics
            .push("no bounded triangles; not evidence of zero volume".into());
        return out;
    }
    let mut seen = std::collections::BTreeSet::new();
    for &[a, b, c] in &triangles {
        let cyclic = [[a, b, c], [b, c, a], [c, a, b]].into_iter().min().unwrap();
        if !seen.insert(cyclic) {
            out.diagnostics.push("duplicate oriented triangle".into());
            return out;
        }
    }
    let origin = points[0];
    let mut area = 0.0;
    let mut volume = 0.0;
    let mut face_edges: BTreeMap<(usize, usize, usize), (usize, i64)> = BTreeMap::new();
    for (&[a, b, c], &primitive_index) in triangles.iter().zip(&triangle_owners) {
        let n = cross(sub(points[b], points[a]), sub(points[c], points[a]));
        let twice_area = dot(n, n).sqrt();
        if twice_area <= WELD * WELD {
            out.diagnostics.push("degenerate triangle".into());
            return out;
        }
        area += twice_area * 0.5;
        volume += dot(
            sub(points[a], origin),
            cross(sub(points[b], origin), sub(points[c], origin)),
        ) / 6.0;
        for (i, j) in [(a, b), (b, c), (c, a)] {
            let d = sub(points[j], points[i]);
            let dd = dot(d, d);
            let mut splits = vec![(0.0, i), (1.0, j)];
            for (k, p) in points.iter().enumerate() {
                if k == i || k == j {
                    continue;
                }
                let q = sub(*p, points[i]);
                let t = dot(q, d) / dd;
                if t > 0.0 && t < 1.0 {
                    let residual: [f64; 3] = std::array::from_fn(|axis| q[axis] - t * d[axis]);
                    if dot(residual, residual) <= WELD * WELD {
                        splits.push((t, k));
                    }
                }
            }
            splits.sort_by(|a, b| a.0.total_cmp(&b.0));
            for pair in splits.windows(2) {
                let (u, v) = (pair[0].1, pair[1].1);
                let edge = face_edges
                    .entry((primitive_index, u.min(v), u.max(v)))
                    .or_default();
                edge.0 += 1;
                edge.1 += if u < v { 1 } else { -1 };
            }
        }
    }
    // Remove internal triangulation diagonals within each saved face before
    // testing the BRep shell boundary. Different faces can subdivide an internal
    // diagonal geometrically without turning it into a topological shell edge.
    let mut edges: BTreeMap<(usize, usize), (usize, i64)> = BTreeMap::new();
    for ((_, a, b), (count, balance)) in face_edges {
        if count == 2 && balance == 0 {
            continue;
        }
        let edge = edges.entry((a, b)).or_default();
        edge.0 += count;
        edge.1 += balance;
    }
    if !area.is_finite() || !volume.is_finite() {
        out.diagnostics.push("nonfinite shell integral".into());
        return out;
    }
    out.surface_area = Some(area);
    let bad = edges
        .values()
        .filter(|(count, balance)| *count != 2 || *balance != 0)
        .count();
    if volume > 0.0
        && edges
            .values()
            .all(|(count, balance)| *count >= 2 && count % 2 == 0 && *balance == 0)
    {
        out.closed_chain_volume = Some(volume);
    }
    if bad != 0 {
        out.diagnostics.push(format!(
            "{bad} nonmanifold/open/inconsistently oriented atomic edges"
        ));
    } else if volume <= 0.0 {
        out.diagnostics
            .push("nonpositive oriented shell volume".into());
    } else {
        out.volume = Some(volume);
    }
    out.sources.push(json!({"method":"oriented_triangle_shell_integral_v1","units":{"volume":"ft3","surface_area":"ft2"},"triangle_count":triangles.len(),"atomic_edge_count":edges.len(),"weld_tolerance_feet":WELD,"boolean_union_performed":false,"manifold":bad==0,"closed_oriented_chain":out.closed_chain_volume.is_some(),"bad_edges":edges.iter().filter(|(_, (n,b))|*n!=2 || *b!=0).take(20).map(|((a,b),(n,balance))|json!({"a":points[*a],"b":points[*b],"count":n,"balance":balance})).collect::<Vec<_>>(),"semantic_area_assigned":false}));
    out
}

/// Reproduce the native straight-wall side-area rule from saved generator history.
/// Native SWall getSideFaceId uses FaceHist keys [1,-1,-1] and [2,-1,-1];
/// VWall computeUserArea returns the larger side, including each side's regions.
/// This bounded implementation requires each side to remain a directly selected
/// face. Split/absent side representations are explicitly refused.
pub fn straight_wall_area(
    owner: &crate::native_parameters::ObjectGraph,
    graphics: &crate::native_parameters::ObjectGraph,
    meshes: &GraphicsMeshes,
) -> Result<(f64, Value), String> {
    let root = owner.objects.first().ok_or("missing wall owner")?;
    if root.class_name != "SWall" {
        return Err("semantic wall area requires SWall".into());
    }
    let list_index = crate::native_saved_mesh::pointer(owner, 0, &root.fields["m_geomSteps"])
        .map_err(|e| e.to_string())?
        .ok_or("wall generator list absent")?;
    let list = &owner.objects[list_index];
    if list.class_name != "GeomStepList" {
        return Err("wall generator list class".into());
    }
    let forms = list.fields["m_bRepFormGList"]
        .as_array()
        .ok_or("wall BRep form list missing")?;
    if forms.is_empty() {
        return Err("wall BRep form list empty".into());
    }
    let generator_index = crate::native_saved_mesh::pointer(owner, list_index, &forms[0])
        .map_err(|e| e.to_string())?
        .ok_or("wall form generator absent")?;
    let class = owner.objects[generator_index].class_name.as_str();
    if !matches!(class, "BaseWallGStep" | "ExtrusionGStep") {
        return Err("wall form generator class unsupported".into());
    }
    let (area, mut source) = generator_pair_area(owner, graphics, meshes, "SWall", class)?;
    source["generator_list_object_index"] = json!(list_index);
    source["generator_list_field"] = json!("GeomStepList.m_bRepFormGList[0]");
    Ok((area, source))
}

/// Native simple floor area is the larger original top/bottom face area.
/// This requires direct unsplit extrusion end faces; edited/missing end faces
/// remain unsupported rather than inferred from bounding dimensions.
pub fn floor_area(
    owner: &crate::native_parameters::ObjectGraph,
    graphics: &crate::native_parameters::ObjectGraph,
    meshes: &GraphicsMeshes,
) -> Result<(f64, Value), String> {
    generator_pair_area(owner, graphics, meshes, "Floor", "ExtrusionGStep")
}
fn generator_pair_area(
    owner: &crate::native_parameters::ObjectGraph,
    graphics: &crate::native_parameters::ObjectGraph,
    meshes: &GraphicsMeshes,
    owner_class: &str,
    generator_class: &str,
) -> Result<(f64, Value), String> {
    if owner.objects.first().map(|o| o.class_name.as_str()) != Some(owner_class) {
        return Err(format!("semantic area requires {owner_class}"));
    }
    if !meshes.diagnostics.is_empty() {
        return Err("incomplete graphics selection".into());
    }
    let generators: Vec<_> = owner
        .objects
        .iter()
        .enumerate()
        .filter(|(_, o)| o.class_name == generator_class)
        .collect();
    if generators.len() != 1 {
        return Err("wall area requires one base wall generator".into());
    }
    let (index, generator) = generators[0];
    let rows = generator.fields["m_faceHistTable"]
        .as_array()
        .ok_or("wall face history missing")?;
    let mut tags = Vec::new();
    let mut areas = Vec::new();
    for side in [1, 2] {
        let matches: Vec<_> = rows
            .iter()
            .filter(|r| r["m_faceHist"]["m_keys"] == json!([side, -1, -1]))
            .collect();
        if matches.len() != 1 {
            return Err("wall side history ambiguous or absent".into());
        }
        let tag = matches[0]["m_id"].as_i64().ok_or("wall side tag missing")?;
        let faces: Vec<_> = meshes
            .primitives
            .iter()
            .filter(|p| p.source_owner_id.is_none() && p.face_tag == tag)
            .collect();
        if faces.len() != 1 {
            return Err(format!(
                "wall side tag{tag} not a unique selected face; regions unqualified"
            ));
        }
        let p = faces[0];
        let geometry = graphics
            .objects
            .get(p.object_index)
            .ok_or("wall geometry source missing")?;
        if geometry.class_name != "Geometry" {
            return Err("wall side is not BRep Geometry".into());
        }
        let mut faces = Vec::new();
        for pointer in geometry.fields["m_pFaces"]
            .as_array()
            .ok_or("geometry face array")?
        {
            let fi = crate::native_saved_mesh::pointer(graphics, p.object_index, pointer)
                .map_err(|e| e.to_string())?
                .ok_or("null geometry face")?;
            let face = &graphics.objects[fi];
            if face.class_name == "Face" && face.fields["m_GInfo"]["m_tag"].as_i64() == Some(tag) {
                faces.push(face);
            }
        }
        if faces.len() != 1
            || faces[0].fields["m_faceRegions"]
                .as_array()
                .is_none_or(|a| !a.is_empty())
        {
            return Err("wall side regions unsupported".into());
        }
        let mut area = 0.;
        for t in &p.triangles {
            let a = *p.vertices.get(t[0] as usize).ok_or("wall triangle index")?;
            let b = *p.vertices.get(t[1] as usize).ok_or("wall triangle index")?;
            let c = *p.vertices.get(t[2] as usize).ok_or("wall triangle index")?;
            let n = cross(sub(b, a), sub(c, a));
            area += dot(n, n).sqrt() * 0.5;
        }
        if !area.is_finite() || area <= 0. {
            return Err("wall side has no finite positive area".into());
        }
        tags.push(tag);
        areas.push(area);
    }
    Ok((
        areas[0].max(areas[1]),
        json!({"method":"native_generator_max_opposite_face_area","owner_class":owner_class,"units":"ft2","generator_object_index":index,"source_field":format!("{generator_class}.m_faceHistTable"),"side_tags":tags,"side_areas":areas,"side_regions_supported":false}),
    ))
}

/// Ordinary non-framing families use half the active trimmed BRep face area.
/// The symbol/family must have no specialized family references; unknown
/// viewport-parameter witnesses and non-BRep graphics remain unqualified.
pub fn ordinary_family_area<'a>(
    symbol: &crate::native_parameters::ObjectGraph,
    family: &crate::native_parameters::ObjectGraph,
    symbol_category: i64,
    owner_graphics: &'a crate::native_parameters::ObjectGraph,
    resolve: &dyn Fn(u64) -> Option<&'a crate::native_parameters::ObjectGraph>,
    meshes: &GraphicsMeshes,
) -> Result<(f64, Value), String> {
    let s = symbol.objects.first().ok_or("missing family symbol")?;
    let f = family.objects.first().ok_or("missing family")?;
    if s.class_name != "FamilySymbol"
        || f.class_name != "Family"
        || symbol_category == -2001300
        || symbol_category >= 0
    {
        return Err("family area structural/reference scope unsupported".into());
    }
    if s.fields["m_refs"].as_array().is_none_or(|a| !a.is_empty())
        || f.fields["m_refs"].as_array().is_none_or(|a| !a.is_empty())
    {
        return Err("specialized family references unqualified".into());
    }
    if !meshes.diagnostics.is_empty()
        || !meshes.profile_observations.is_empty()
        || meshes.primitives.is_empty()
    {
        return Err("family metric graphics context unqualified".into());
    }
    let mut area = 0.;
    let mut witnesses = Vec::new();
    for p in &meshes.primitives {
        let g = match p.source_owner_id {
            Some(id) => resolve(id).ok_or("family graphics dependency absent")?,
            None => owner_graphics,
        };
        let carrier = g
            .objects
            .get(p.object_index)
            .ok_or("family graphics carrier absent")?;
        if carrier.class_name != "Geometry" {
            return Err("family area requires planar BRep faces".into());
        }
        let mut faces = Vec::new();
        for ptr in carrier.fields["m_pFaces"]
            .as_array()
            .ok_or("family geometry face list")?
        {
            let i = crate::native_saved_mesh::pointer(g, p.object_index, ptr)
                .map_err(|e| e.to_string())?
                .ok_or("null family face")?;
            let face = &g.objects[i];
            if face.class_name == "Face"
                && face.fields["m_GInfo"]["m_tag"].as_i64() == Some(p.face_tag)
            {
                faces.push((i, face));
            }
        }
        if faces.len() != 1 {
            return Err("family face identity ambiguous".into());
        }
        let (i, face) = faces[0];
        if face.fields["m_GInfo"]["m_flags"]
            .as_u64()
            .is_none_or(|flags| flags & 0x80000 == 0)
        {
            return Err("family selection contains inactive face".into());
        }
        if face.fields["m_faceRegions"]
            .as_array()
            .is_none_or(|a| !a.is_empty())
        {
            return Err("family face regions unqualified".into());
        }
        for t in &p.triangles {
            let a = *p
                .vertices
                .get(t[0] as usize)
                .ok_or("family triangle index")?;
            let b = *p
                .vertices
                .get(t[1] as usize)
                .ok_or("family triangle index")?;
            let c = *p
                .vertices
                .get(t[2] as usize)
                .ok_or("family triangle index")?;
            let n = cross(sub(b, a), sub(c, a));
            area += dot(n, n).sqrt() * 0.25;
        }
        witnesses.push(json!({"source_owner_id":p.source_owner_id,"geometry_object_index":p.object_index,"face_object_index":i,"face_tag":p.face_tag}));
    }
    if !area.is_finite() || area <= 0. {
        return Err("family area not finite positive".into());
    }
    Ok((
        area,
        json!({"method":"ordinary_family_half_active_face_area","units":"ft2","symbol_header_category":symbol_category,"family_refs_empty":true,"symbol_refs_empty":true,"faces":witnesses}),
    ))
}

/// Curtain-wall Area uses the saved grid face (FaceHist14), not panel sums or
/// the displaced base-wall sides. Multiple grid generators remain unsupported.
pub fn curtain_wall_area(
    owner: &crate::native_parameters::ObjectGraph,
    graphics: &crate::native_parameters::ObjectGraph,
    meshes: &GraphicsMeshes,
) -> Result<(f64, Value), String> {
    if owner.objects.first().map(|o| o.class_name.as_str()) != Some("SWall")
        || !meshes.diagnostics.is_empty()
    {
        return Err("curtain wall owner/graphics unsupported".into());
    }
    let gs: Vec<_> = owner
        .objects
        .iter()
        .enumerate()
        .filter(|(_, o)| o.class_name == "CWallGridFaceGStep")
        .collect();
    if gs.len() != 1 {
        return Err("requires one curtain-grid generator".into());
    }
    let (gi, g) = gs[0];
    let rows = g.fields["m_faceHistTable"]
        .as_array()
        .ok_or("curtain-grid face history missing")?;
    let rows: Vec<_> = rows
        .iter()
        .filter(|r| r["m_faceHist"]["m_keys"] == json!([14, -1, -1]))
        .collect();
    if rows.len() != 1 {
        return Err("curtain-grid face identity ambiguous".into());
    }
    let tag = rows[0]["m_id"].as_i64().ok_or("curtain-grid face tag")?;
    let ps: Vec<_> = meshes
        .primitives
        .iter()
        .filter(|p| p.source_owner_id.is_none() && p.face_tag == tag)
        .collect();
    if ps.len() != 1 {
        return Err("curtain-grid face not uniquely selected".into());
    }
    let p = ps[0];
    let carrier = graphics
        .objects
        .get(p.object_index)
        .ok_or("curtain-grid carrier")?;
    if carrier.class_name != "Geometry" {
        return Err("curtain-grid carrier is not BRep".into());
    }
    let mut faces = Vec::new();
    for ptr in carrier.fields["m_pFaces"]
        .as_array()
        .ok_or("curtain-grid faces")?
    {
        let i = crate::native_saved_mesh::pointer(graphics, p.object_index, ptr)
            .map_err(|e| e.to_string())?
            .ok_or("curtain-grid null face")?;
        let f = &graphics.objects[i];
        if f.class_name == "Face" && f.fields["m_GInfo"]["m_tag"].as_i64() == Some(tag) {
            faces.push((i, f));
        }
    }
    if faces.len() != 1
        || faces[0].1.fields["m_faceRegions"]
            .as_array()
            .is_none_or(|a| !a.is_empty())
    {
        return Err("curtain-grid face regions unqualified".into());
    }
    let mut area = 0.;
    for t in &p.triangles {
        let a = *p
            .vertices
            .get(t[0] as usize)
            .ok_or("curtain-grid triangle index")?;
        let b = *p
            .vertices
            .get(t[1] as usize)
            .ok_or("curtain-grid triangle index")?;
        let c = *p
            .vertices
            .get(t[2] as usize)
            .ok_or("curtain-grid triangle index")?;
        let n = cross(sub(b, a), sub(c, a));
        area += dot(n, n).sqrt() * 0.5;
    }
    if !area.is_finite() || area <= 0. {
        return Err("curtain-grid nonpositive area".into());
    }
    Ok((
        area,
        json!({"method":"native_curtain_wall_grid_face_area","units":"ft2","generator_object_index":gi,"source_field":"CWallGridFaceGStep.m_faceHistTable","history_key":[14,-1,-1],"face_tag":tag,"geometry_object_index":p.object_index,"face_object_index":faces[0].0}),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::native_saved_mesh::Primitive;
    fn cube(offset: f64) -> GraphicsMeshes {
        let vertices = vec![
            [0., 0., 0.],
            [1., 0., 0.],
            [1., 1., 0.],
            [0., 1., 0.],
            [0., 0., 1.],
            [1., 0., 1.],
            [1., 1., 1.],
            [0., 1., 1.],
        ]
        .into_iter()
        .map(|p| [p[0] + offset, p[1] + offset, p[2] + offset])
        .collect();
        GraphicsMeshes {
            primitives: vec![Primitive {
                source_owner_id: None,
                object_index: 0,
                face_tag: 0,
                render_style_id: 0,
                material_id: None,
                vertices,
                normals: vec![],
                triangles: vec![
                    [0, 2, 1],
                    [0, 3, 2],
                    [4, 5, 6],
                    [4, 6, 7],
                    [0, 1, 5],
                    [0, 5, 4],
                    [1, 2, 6],
                    [1, 6, 5],
                    [2, 3, 7],
                    [2, 7, 6],
                    [3, 0, 4],
                    [3, 4, 7],
                ],
            }],
            ..Default::default()
        }
    }
    #[test]
    fn translated_closed_shell() {
        for offset in [0., 1e7] {
            let m = compute(&cube(offset));
            assert_eq!(m.surface_area, Some(6.));
            assert!((m.volume.unwrap() - 1.).abs() < 1e-12);
        }
    }
    #[test]
    fn open_and_duplicate_shells_refused() {
        let mut m = cube(0.);
        m.primitives[0].triangles.pop();
        assert!(compute(&m).volume.is_none());
        let mut m = cube(0.);
        m.primitives.push(m.primitives[0].clone());
        assert!(compute(&m).volume.is_none());
    }
    #[test]
    fn independently_closed_carriers_can_share_an_edge() {
        let mut m = cube(0.);
        let mut second = cube(0.);
        second.primitives[0].object_index = 1;
        for p in &mut second.primitives[0].vertices {
            p[0] += 1.;
            p[1] += 1.;
        }
        m.primitives.extend(second.primitives);
        assert!((compute(&m).volume.unwrap() - 2.).abs() < 1e-12);
    }
    #[test]
    fn edge_touching_shells_have_closed_chain_but_not_manifold_volume() {
        let mut m = cube(0.);
        let mut second = cube(0.);
        for p in &mut second.primitives[0].vertices {
            p[0] += 1.;
            p[1] += 1.;
        }
        m.primitives.extend(second.primitives);
        m.primitives = m
            .primitives
            .into_iter()
            .flat_map(|p| {
                p.triangles
                    .chunks(2)
                    .map(|ts| {
                        let mut face = p.clone();
                        face.triangles = ts.to_vec();
                        face
                    })
                    .collect::<Vec<_>>()
            })
            .collect();
        let measured = compute(&m);
        assert!(measured.volume.is_none());
        assert!((measured.closed_chain_volume.unwrap() - 2.).abs() < 1e-12);
        m.primitives[0].triangles.pop();
        assert!(compute(&m).closed_chain_volume.is_none());
    }
    #[test]
    fn inward_winding_is_not_silently_fixed() {
        let mut m = cube(0.);
        for t in &mut m.primitives[0].triangles {
            t.swap(1, 2);
        }
        assert!(compute(&m).volume.is_none());
    }
    #[test]
    fn disjoint_shells_and_inner_cavity() {
        let mut m = cube(0.);
        m.primitives.extend(cube(3.).primitives);
        assert!((compute(&m).volume.unwrap() - 2.).abs() < 1e-12);
        let mut m = cube(0.);
        let mut inner = cube(0.);
        for p in &mut inner.primitives[0].vertices {
            for x in p {
                *x = *x * 0.5 + 0.25;
            }
        }
        for t in &mut inner.primitives[0].triangles {
            t.swap(1, 2);
        }
        m.primitives.extend(inner.primitives);
        assert!((compute(&m).volume.unwrap() - 0.875).abs() < 1e-12);
    }
    #[test]
    fn collinear_boundary_subdivision_closes() {
        let mut m = cube(0.);
        m.primitives[0].vertices.push([0.5, 0., 0.]);
        m.primitives[0].triangles[0] = [0, 2, 8];
        m.primitives[0].triangles.push([8, 2, 1]);
        assert!((compute(&m).volume.unwrap() - 1.).abs() < 1e-12);
    }
    #[test]
    fn qualified_excluded_alternative_does_not_invalidate_shell() {
        let mut m = cube(0.);
        m.rejected_filters = 3;
        assert!((compute(&m).volume.unwrap() - 1.).abs() < 1e-12);
    }
    #[test]
    fn missing_graphics_is_not_zero() {
        assert!(compute(&GraphicsMeshes::default()).volume.is_none());
        let mut m = cube(0.);
        m.diagnostics.push("missing external symbol".into());
        assert!(compute(&m).surface_area.is_none());
    }
}
