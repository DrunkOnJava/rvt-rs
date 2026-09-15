//! Per-material measurements from selected saved geometry and qualified native IDs.
//! Callers select the category-specific material quantity detail profile. Render
//! equivalence alone never supplies a native material identity.
use crate::native_saved_mesh::{GraphicsMeshes, Primitive};
use serde::Serialize;
use serde_json::{Value, json};
use std::collections::BTreeMap;

#[derive(Debug, Default, Serialize)]
pub struct MaterialQuantities {
    pub materials: BTreeMap<i64, crate::native_saved_metrics::SavedMetrics>,
    pub unresolved_primitives: Vec<usize>,
    pub diagnostics: Vec<String>,
    pub sources: Vec<Value>,
}

pub fn compute(
    meshes: &GraphicsMeshes,
    resolve: &impl Fn(usize, &Primitive) -> Option<i64>,
) -> MaterialQuantities {
    let mut out = MaterialQuantities::default();
    if !meshes.diagnostics.is_empty() {
        out.diagnostics
            .push("incomplete saved geometry selection".into());
        return out;
    }
    let mut groups: BTreeMap<i64, Vec<Primitive>> = BTreeMap::new();
    for (i, p) in meshes.primitives.iter().enumerate() {
        if let Some(id) = resolve(i, p).filter(|id| *id > 0) {
            groups.entry(id).or_default().push(p.clone());
        } else {
            out.unresolved_primitives.push(i);
        }
    }
    // An unresolved face can belong to any material: do not publish incomplete
    // per-material areas or a misleading closed subset volume.
    if !out.unresolved_primitives.is_empty() {
        out.diagnostics
            .push("unresolved native material identity".into());
        return out;
    }
    for (id, primitives) in groups {
        let metrics = crate::native_saved_metrics::compute(&GraphicsMeshes {
            primitives,
            ..Default::default()
        });
        out.materials.insert(id, metrics);
    }
    out.sources.push(json!({"method":"per_native_material_saved_geometry_measurement","area":"full selected face area; host semantic area requires separate qualified owner generator","volume":"closed oriented geometry carriers; open material subsets refuse volume","units":{"area":"ft2","volume":"ft3"}}));
    out
}

/// Reproduce the installed native host material report's single-layer rule.
/// This deliberately remains separate from physical closed-shell volume: the
/// native implementation uses 1,000 left-endpoint slices when side areas differ.
/// The caller must qualify one constant-width compound layer and supply its
/// native width source; arbitrary or varying-width layers are outside this API.
pub fn single_layer_wall_report(
    owner: &crate::native_parameters::ObjectGraph,
    graphics: &crate::native_parameters::ObjectGraph,
    meshes: &GraphicsMeshes,
    width: f64,
    width_source: Value,
) -> Result<(f64, f64, Value), String> {
    if !width.is_finite() || width <= 0. || width_source.is_null() {
        return Err("positive native constant layer width and provenance required".into());
    }
    let (_, side_source) =
        crate::native_saved_metrics::straight_wall_area(owner, graphics, meshes)?;
    let end = side_source["side_areas"][0]
        .as_f64()
        .ok_or("first side area")?;
    let start = side_source["side_areas"][1]
        .as_f64()
        .ok_or("second side area")?;
    let volume = host_layer_volume(start, end, width);
    Ok((
        start,
        volume,
        json!({"method":"material_area_volume_single_constant_layer","area_start_face_history":[2,-1,-1],"area_end_face_history":[1,-1,-1],"native_integration":"1000 left-endpoint slices when absolute side-area difference exceeds 1e-10","physical_closed_shell_volume":false,"side_source":side_source,"width":width,"width_source":width_source}),
    ))
}
fn host_layer_volume(start: f64, end: f64, width: f64) -> f64 {
    if (start - end).abs() <= 1e-10 {
        return start * width;
    }
    let step = width / 1000.;
    let mut volume = 0.;
    for i in 0..1000 {
        volume += (start - (i as f64 / 1000.) * (start - end)) * step;
    }
    volume
}

/// Native PatternHelper material-area report for complete direct-shape BReps.
/// This exposes no Volume: a successful area read does not establish volume
/// applicability or a closed, consistently oriented native solid.
pub fn direct_shape_type_areas(
    owner: &crate::native_parameters::ObjectGraph,
    graphics: &crate::native_parameters::ObjectGraph,
    meshes: &GraphicsMeshes,
) -> Result<(BTreeMap<i64, f64>, Value), String> {
    use std::collections::{HashMap, HashSet};
    if owner.objects.first().map(|o| o.class_name.as_str()) != Some("DirectShapeType") {
        return Err("not a current DirectShapeType".into());
    }
    let owner_id = owner.objects[0].fields["m_id"]["m_id"]["m_id64"]
        .as_u64()
        .filter(|id| *id > 0)
        .ok_or("invalid direct-shape owner ID")?;
    if graphics.objects.first().is_none_or(|o| {
        o.class_name != "GElement" || o.fields["m_elementId"].as_u64() != Some(owner_id)
    }) {
        return Err("direct-shape graphics owner mismatch".into());
    }
    if !meshes.diagnostics.is_empty()
        || !meshes.profile_observations.is_empty()
        || meshes.unbounded_faces != 0
        || meshes.rejected_filters != 0
        || meshes.excluded_visibility_branches != 0
    {
        return Err("incomplete direct-shape material-area representation".into());
    }
    let mut edges = HashMap::new();
    for e in &graphics.edges {
        let key = (
            e.source_object_index,
            e.pointer_offset,
            u64::from(e.pointer_token),
        );
        edges
            .entry(key)
            .and_modify(|v| *v = None)
            .or_insert(Some(e));
    }
    let mut expected = HashMap::new();
    for (gi, g) in graphics
        .objects
        .iter()
        .enumerate()
        .filter(|(_, o)| o.class_name == "Geometry")
    {
        for ptr in g.fields["m_pFaces"]
            .as_array()
            .ok_or("missing direct BRep face list")?
        {
            let key = (
                gi,
                ptr["offset"].as_u64().ok_or("face pointer offset")? as usize,
                ptr["pointer_token"].as_u64().ok_or("face pointer token")?,
            );
            let e = edges
                .get(&key)
                .and_then(|e| *e)
                .ok_or("ambiguous or missing face edge")?;
            let face = graphics
                .objects
                .get(e.target_object_index)
                .ok_or("face index out of bounds")?;
            if face.class_name != "Face"
                || face.class_tag != e.target_class_tag
                || ptr["class_tag"].as_u64() != Some(u64::from(face.class_tag))
            {
                return Err("direct face class mismatch".into());
            }
            let f = &face.fields;
            if f["m_GInfo"]["m_flags"]
                .as_u64()
                .is_none_or(|v| v & 0x80000 == 0)
                || f["m_faceRegions"].as_array().is_none_or(|v| !v.is_empty())
                || f["m_pFirstLoop"]["pointer_token"]
                    .as_u64()
                    .is_none_or(|v| v == 0)
            {
                return Err("inactive, regioned, or untrimmed direct face".into());
            }
            let tag = f["m_GInfo"]["m_tag"].as_i64().ok_or("face tag")?;
            let material = f["m_renderStyleId"]["m_id"]["m_id64"]
                .as_i64()
                .filter(|v| *v > 0)
                .ok_or("positive face material identity required")?;
            if expected.insert((gi, tag), material).is_some() {
                return Err("duplicate direct face identity".into());
            }
        }
    }
    let mut seen = HashSet::new();
    let mut areas = BTreeMap::new();
    for p in &meshes.primitives {
        let key = (p.object_index, p.face_tag);
        let material = *expected
            .get(&key)
            .ok_or("unexpected direct-shape mesh face")?;
        if p.source_owner_id.is_some() || p.render_style_id != material || !seen.insert(key) {
            return Err("external, mismatched, or duplicated direct-shape face".into());
        }
        let area = areas.entry(material).or_insert(0.);
        for t in &p.triangles {
            let mut v = [[0.; 3]; 3];
            for k in 0..3 {
                v[k] = *p.vertices.get(t[k] as usize).ok_or("triangle index")?;
            }
            if v.iter().flatten().any(|v| !v.is_finite()) {
                return Err("nonfinite triangle".into());
            }
            let u: [f64; 3] = std::array::from_fn(|i| v[1][i] - v[0][i]);
            let w: [f64; 3] = std::array::from_fn(|i| v[2][i] - v[0][i]);
            let cross: [f64; 3] = std::array::from_fn(|i| {
                u[(i + 1) % 3] * w[(i + 2) % 3] - u[(i + 2) % 3] * w[(i + 1) % 3]
            });
            *area += cross.iter().map(|x| x * x).sum::<f64>().sqrt() * 0.5;
        }
    }
    if seen.len() != expected.len()
        || seen.is_empty()
        || areas.values().any(|v| !v.is_finite() || *v <= 0.)
    {
        return Err("incomplete or invalid direct-shape area".into());
    }
    Ok((
        areas,
        json!({"method":"native_PatternHelper_calculateMaterialAreaGeometry",
        "rule":"sum all active trimmed direct BRep faces matching positive renderStyle material ID",
        "face_count":seen.len(),"units":"ft2","volume_applicability":"not established by area"}),
    ))
}

/// Qualified constant-layer floor reports with equal opposite cap areas.
/// Layer IDs and widths must come from the owner's resolved compound type.
/// Native reports add an area entry for each matching layer, even when a
/// material occurs repeatedly; this is not exposed surface area or mesh volume.
pub type LayerQuantities = BTreeMap<i64, (f64, f64)>;
pub fn constant_layers_floor_report(
    owner: &crate::native_parameters::ObjectGraph,
    graphics: &crate::native_parameters::ObjectGraph,
    meshes: &GraphicsMeshes,
    layers: &[(i64, f64, Value)],
) -> Result<(LayerQuantities, Value), String> {
    if layers.is_empty()
        || layers
            .iter()
            .any(|(id, w, s)| *id <= 0 || !w.is_finite() || *w <= 0. || s.is_null())
    {
        return Err("positive resolved native materials and constant layer widths required".into());
    }
    let (_, cap_source) = crate::native_saved_metrics::floor_area(owner, graphics, meshes)?;
    let first = cap_source["side_areas"][0]
        .as_f64()
        .ok_or("first cap area")?;
    let second = cap_source["side_areas"][1]
        .as_f64()
        .ok_or("second cap area")?;
    if (first - second).abs() > 1e-10 {
        return Err(
            "unequal floor cap areas require qualified layer order and interpolation".into(),
        );
    }
    let mut result = BTreeMap::new();
    for (id, width, _) in layers {
        let (area, volume) = result.entry(*id).or_insert((0., 0.));
        *area += first;
        *volume += first * width;
    }
    Ok((
        result,
        json!({"method":"material_area_volume_equal_caps_constant_layers","cap_source":cap_source,"layers":layers,"physical_closed_shell_volume":false,"repeated_material_areas_accumulate_per_layer":true}),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn triangle() -> Primitive {
        Primitive {
            source_owner_id: None,
            object_index: 0,
            face_tag: 0,
            render_style_id: 1,
            material_id: Some(1),
            vertices: vec![[0., 0., 0.], [2., 0., 0.], [0., 3., 0.]],
            normals: vec![[0., 0., 1.]; 3],
            triangles: vec![[0, 1, 2]],
        }
    }
    #[test]
    fn native_host_report_keeps_discrete_integration_separate_from_true_volume() {
        let reported = host_layer_volume(696.75, 694.25, 0.5);
        let exact = (696.75 + 694.25) * 0.5 / 2.;
        assert!((reported - 347.750625).abs() < 1e-10);
        assert!((reported - exact - 0.000625).abs() < 1e-10);
        assert_eq!(host_layer_volume(12., 12., 0.5), 6.);
        assert!((host_layer_volume(694.25, 696.75, 0.5) - 347.749375).abs() < 1e-10);
    }
    #[test]
    fn open_material_surface_has_area_but_no_invented_volume() {
        let m = compute(
            &GraphicsMeshes {
                primitives: vec![triangle()],
                ..Default::default()
            },
            &|_, p| p.material_id,
        );
        assert_eq!(m.materials[&1].surface_area, Some(3.));
        assert_eq!(m.materials[&1].volume, None);
    }
    #[test]
    fn unresolved_face_prevents_partial_material_claims() {
        let m = compute(
            &GraphicsMeshes {
                primitives: vec![triangle(), triangle()],
                ..Default::default()
            },
            &|i, _| if i == 0 { Some(1) } else { None },
        );
        assert!(m.materials.is_empty());
        assert_eq!(m.unresolved_primitives, vec![1]);
    }
}
