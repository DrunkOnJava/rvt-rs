//! Saved planar texture coordinates from current channel-103 graphics ownership.
//! This projection does not regenerate graphics after parameter changes. It is
//! separate from the material's subsequent bitmap sampling transform.
use crate::{
    RevitFile,
    native_document::{self, Record},
    native_metadata::identifier,
    native_parameters::ObjectGraph,
};
use anyhow::{Context, Result, ensure};
use serde::Serialize;
use serde_json::Value;
use std::collections::BTreeSet;

#[derive(Debug, Serialize)]
pub struct Mapping {
    pub element_id: u64,
    pub unique_id: String,
    pub geometry_tag: i64,
    pub material_id: i64,
    /// Row-major affine map: world feet `[x, y, z, 1]` -> raw exporter UV.
    pub world_to_uv: [[f64; 4]; 2],
    pub plane_origin: [f64; 3],
    pub plane_normal: [f64; 3],
    pub source: Value,
}
impl Mapping {
    pub fn evaluate(&self, point: [f64; 3]) -> Result<[f64; 2]> {
        ensure!(
            point.iter().all(|x| x.is_finite()),
            "nonfinite texture mapping point"
        );
        let distance: f64 = (0..3)
            .map(|i| (point[i] - self.plane_origin[i]) * self.plane_normal[i])
            .sum();
        ensure!(
            distance.abs() <= 1e-7,
            "texture point outside saved face plane"
        );
        Ok(self
            .world_to_uv
            .map(|r| r[3] + (0..3).map(|i| r[i] * point[i]).sum::<f64>()))
    }
}
#[derive(Debug, Default, Serialize)]
pub struct Inventory {
    pub mappings: Vec<Mapping>,
    pub diagnostics: Vec<Value>,
    pub excluded_graphics_groups: Vec<Value>,
}
fn target(g: &ObjectGraph, owner: usize, p: &Value, class: &str) -> Result<usize> {
    let edges = g
        .edges
        .iter()
        .filter(|e| {
            e.source_object_index == owner && Some(e.pointer_offset as u64) == p["offset"].as_u64()
        })
        .collect::<Vec<_>>();
    ensure!(edges.len() == 1, "texture mapping pointer ownership");
    let i = edges[0].target_object_index;
    ensure!(
        g.objects.get(i).is_some_and(|o| o.class_name == class),
        "texture mapping expected {class}"
    );
    Ok(i)
}
fn array<const N: usize>(v: &Value) -> Result<[f64; N]> {
    let a: [f64; N] = v
        .as_array()
        .context("texture mapping vector absent")?
        .iter()
        .map(|v| v.as_f64().context("texture mapping number absent"))
        .collect::<Result<Vec<_>>>()?
        .try_into()
        .map_err(|_| anyhow::anyhow!("texture mapping vector length"))?;
    ensure!(
        a.iter().all(|x| x.is_finite()),
        "texture mapping nonfinite vector"
    );
    Ok(a)
}
fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    (0..3).map(|i| a[i] * b[i]).sum()
}
fn face(r: &Record, g: &ObjectGraph, fi: usize) -> Result<Mapping> {
    let f = &g.objects[fi].fields;
    ensure!(
        f["m_faceRegions"].as_array().is_some_and(Vec::is_empty),
        "texture face regions unqualified"
    );
    let bi = target(g, fi, &f["m_pGFilling"], "GFilling")?;
    let b = &g.objects[bi].fields;
    ensure!(
        b["m_pGFace"]["pointer_token"].as_u64() == Some(u64::from(g.objects[fi].token)),
        "texture filling owner mismatch"
    );
    ensure!(
        b["m_flags"].as_i64() == Some(18)
            && b["m_data"]["pointer_token"].as_u64() == Some(0)
            && identifier(&b["m_patternId"])? == -1,
        "texture filling profile unqualified"
    );
    let placer = &b["m_placer"];
    ensure!(
        placer["m_isMirrored"].as_bool() == Some(false)
            && placer["m_placedDraft"].as_bool() == Some(false)
            && placer["m_scale"].as_f64() == Some(1.)
            && array::<2>(&placer["m_uvScale"]["m_uv"])? == [1., 1.],
        "texture placer mirror/draft/scale unqualified"
    );
    let d = array::<2>(&placer["m_dir"])?;
    let offset = array::<2>(&placer["m_origin"])?;
    ensure!(
        (d[0] * d[0] + d[1] * d[1] - 1.).abs() < 1e-10,
        "texture placer direction not unit"
    );
    let pi = target(g, fi, &f["m_pSurf"], "Plane")?;
    let p = &g.objects[pi].fields;
    let origin = array::<3>(&p["m_origin"])?;
    let x = array::<3>(&p["m_xVec"])?;
    let y = array::<3>(&p["m_yVec"])?;
    ensure!(
        (dot(x, x) - 1.).abs() < 1e-10 && (dot(y, y) - 1.).abs() < 1e-10 && dot(x, y).abs() < 1e-10,
        "texture plane basis not orthonormal"
    );
    let axes = [[d[0], d[1]], [-d[1], d[0]]];
    let world_to_uv = axes.map(|a| {
        let v: [f64; 3] = std::array::from_fn(|i| a[0] * x[i] + a[1] * y[i]);
        [
            v[0],
            v[1],
            v[2],
            -dot(v, origin) - a[0] * offset[0] - a[1] * offset[1],
        ]
    });
    Ok(Mapping {
        element_id: r.identity.element_id,
        unique_id: r.identity.unique_id.clone(),
        geometry_tag: f["m_GInfo"]["m_tag"]
            .as_i64()
            .context("texture face tag absent")?,
        material_id: identifier(&f["m_renderStyleId"]).context("texture material absent")?,
        world_to_uv,
        plane_origin: origin,
        plane_normal: [
            x[1] * y[2] - x[2] * y[1],
            x[2] * y[0] - x[0] * y[2],
            x[0] * y[1] - x[1] * y[0],
        ],
        source: serde_json::json!({"channel":103,"stream":r.source.stream,"body_sha256":r.source.body_sha256,"group_record_offset":r.source.group_record_offset,"face_object":fi,"filling_object":bi,"plane_object":pi,"fields":["Face.m_renderStyleId","GFilling.m_placer","Plane.m_origin","Plane.m_xVec","Plane.m_yVec"],"semantics":"saved_graphics_raw_uv_not_regenerated_or_material_transformed"}),
    })
}
fn parse(r: &Record) -> Result<Inventory> {
    ensure!(
        r.status == "complete_bounded_graph",
        "texture graphics graph incomplete"
    );
    let g = r.graph.as_ref().context("texture graphics graph absent")?;
    ensure!(
        g.consumed_bytes == r.source.body_bytes,
        "texture graphics EOF mismatch"
    );
    let root = &g.objects.first().context("texture graphics root absent")?;
    ensure!(
        root.class_name == "GElement"
            && root.fields["m_elementId"].as_u64() == Some(r.identity.element_id),
        "texture graphics root owner mismatch"
    );
    let mut out = Inventory::default();
    let mut tags = BTreeSet::new();
    for p in root.fields["m_subNodes"]
        .as_array()
        .context("texture graphics subnodes absent")?
    {
        if p["class_tag"].as_u64() == Some(2248) {
            let gi = target(g, 0, p, "GGroup")?;
            out.excluded_graphics_groups.push(serde_json::json!({"element_id":r.identity.element_id,"object_index":gi,"reason":"auxiliary grouped graphics; mapping only direct body Geometry"}));
            continue;
        }
        let gi = target(g, 0, p, "Geometry")?;
        for p in g.objects[gi].fields["m_pFaces"]
            .as_array()
            .context("texture geometry faces absent")?
        {
            let fi = target(g, gi, p, "Face")?;
            let tag = g.objects[fi].fields["m_GInfo"]["m_tag"].clone();
            match face(r,g,fi) {
                Ok(m)=>{ensure!(tags.insert(m.geometry_tag),"duplicate texture face tag");out.mappings.push(m)},
                Err(e)=>out.diagnostics.push(serde_json::json!({"element_id":r.identity.element_id,"geometry_tag":tag,"reason":format!("{e:#}")})),
            }
        }
    }
    Ok(out)
}
/// Selected current saved graphics only; unsupported owners/faces are explicit.
pub fn read(file: &mut RevitFile, ids: BTreeSet<u64>) -> Result<Inventory> {
    let mut result = Inventory::default();
    if ids.is_empty() {
        return Ok(result);
    }
    ensure!(
        file.basic_file_info()?.version == 2027,
        "texture graphics version outside qualified 2027 profile"
    );
    let mut seen = BTreeSet::new();
    native_document::extract(
        file,
        &native_document::Options {
            selected_ids: ids.clone(),
            channels: BTreeSet::from([103]),
            ..Default::default()
        },
        |r| {
            seen.insert(r.identity.element_id);
            match parse(&r) {
            Ok(mut v)=>{result.mappings.append(&mut v.mappings);result.diagnostics.append(&mut v.diagnostics);result.excluded_graphics_groups.append(&mut v.excluded_graphics_groups)},
            Err(e)=>result.diagnostics.push(serde_json::json!({"element_id":r.identity.element_id,"reason":format!("{e:#}")})),
        }
            Ok(())
        },
    )?;
    for id in ids.difference(&seen) {
        result
            .diagnostics
            .push(serde_json::json!({"element_id":id,"reason":"current graphics channel absent"}))
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn affine_mapping_refuses_off_plane_and_nonfinite_points() {
        let m = Mapping {
            element_id: 1,
            unique_id: "test".into(),
            geometry_tag: 2,
            material_id: 3,
            world_to_uv: [[0., 1., 0., -8.], [-1., 0., 0., 4.]],
            plane_origin: [0., 0., 7.],
            plane_normal: [0., 0., 1.],
            source: Value::Null,
        };
        assert_eq!(m.evaluate([2., 9., 7.]).unwrap(), [1., 2.]);
        assert!(m.evaluate([2., 9., 7.01]).is_err());
        assert!(m.evaluate([f64::NAN, 9., 7.]).is_err());
    }
}
