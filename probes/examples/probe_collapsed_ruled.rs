//! RE-443: a zero RuledSurf profile pointer selects its separately saved point.
//!
//! Reads the seven source symbols in the public Revit 2023 basic MEP sample.
//! Checks the pointer and point bytes, evaluates every admitted patch including
//! its pole, meshes its real face, and refuses mutations of those real graphs.
//! JSON names the file hash, stream, record, body-relative offsets and bytes.
//!
//! cargo run --profile ci -p rvt-probes --example probe_collapsed_ruled -- <sample.rvt>
//! Compare the JSON and exported bodies with tools/re/replay_collapsed_ruled.py.
use anyhow::{Context, Result, ensure};
use rvt::RevitFile;
use rvt::research::{
    graph_value::Fv,
    native_document::{Options, extract_graphs},
    native_parameters::ObjectGraph,
    native_parametric_mesh::face_grid,
    native_parametric_surface::ParametricSurface,
    native_saved_mesh::{face, pointer},
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

const OWNERS: [u64; 7] = [417088, 492340, 539304, 543268, 717794, 718444, 718635];

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn nonrectangular_control(g: &ObjectGraph, fi: usize) -> Result<String> {
    for (ei, edge) in g.objects.iter().enumerate() {
        if edge.class_name != "Edge" {
            continue;
        }
        for (side, value) in edge.fields["m_pFace"]
            .as_array()
            .context("edge faces")?
            .iter()
            .enumerate()
        {
            if pointer(g, ei, value)? != Some(fi) {
                continue;
            }
            let mut changed = g.clone();
            let mut fields = edge.fields.to_json();
            let mut sample = fields["m_firstAndLastEdgePnts"][0].clone();
            sample["uv"][side] = json!([0.37, 0.41]);
            fields["m_interiorEdgePnts"] = json!([sample]);
            changed.objects[ei].fields = Fv::from_value(&fields);
            let error = face_grid(&changed, fi)
                .err()
                .context("accepted nonrectangular collapsed trim")?
                .to_string();
            ensure!(
                error == "unqualified collapsed ruled trim",
                "unexpected trim refusal: {error}"
            );
            return Ok(error);
        }
    }
    anyhow::bail!("no face edge for trim control")
}

fn controls(g: &ObjectGraph, si: usize, side: usize, partner: usize) -> Result<Vec<Value>> {
    let mut out = Vec::new();
    for case in [
        "missing point",
        "invalid point",
        "unresolved pointer",
        "both null",
        "clipped envelope",
        "point-line",
        "point-ellipse",
    ] {
        let mut changed = g.clone();
        let mut fields = changed.objects[si].fields.to_json();
        match case {
            "missing point" => {
                fields
                    .as_object_mut()
                    .context("fields")?
                    .remove(&format!("m_Point{side}"));
            }
            "invalid point" => fields[format!("m_Point{side}")] = json!([0, "invalid", 0]),
            "unresolved pointer" => {
                fields[format!("m_pProfileCurve{side}")]["pointer_token"] = json!(4294967294_u64)
            }
            "both null" => {
                fields[format!("m_pProfileCurve{}", 3 - side)]["pointer_token"] = json!(0)
            }
            "clipped envelope" => fields["m_Envelope"]["m_corners"][1][1] = json!(0.9),
            "point-line" => {
                changed.objects[partner].class_name = "GLine".into();
                changed.objects[partner].fields = Fv::from_value(
                    &json!({"m_origin":[0,0,0],"m_dirVec":[1,0,0],"m_endParams":[0,1]}),
                );
            }
            _ => {
                changed.objects[partner].class_name = "GEllipse".into();
                let mut f = changed.objects[partner].fields.to_json();
                f["m_xLen"] = f["m_radius"].clone();
                f["m_yLen"] = f["m_radius"].clone();
                changed.objects[partner].fields = Fv::from_value(&f);
            }
        }
        changed.objects[si].fields = Fv::from_value(&fields);
        let error = ParametricSurface::read(&changed, si)
            .err()
            .with_context(|| format!("accepted {case}"))?;
        out.push(json!({"case":case,"refusal":error.to_string()}));
    }
    Ok(out)
}

// Use the real pinned records, but deliberately swap their export classes.
// Admission must follow the source BuiltInCategory, not these IFC labels.
fn category_controls(file: &mut RevitFile) -> Result<Value> {
    use rvt::ifc::{
        RvtDocExporter,
        entities::{IfcEntity, PropertyValue},
        saved_meshes,
    };
    let pin: Value = serde_json::from_str(include_str!("../../research/collapsed-ruled.json"))?;
    let targets: BTreeSet<&str> = pin["targets"]
        .as_array()
        .context("targets")?
        .iter()
        .map(|v| v.as_str().context("target tag"))
        .collect::<Result<_>>()?;
    let refusals: BTreeSet<&str> = pin["refusal_controls"]
        .as_array()
        .context("refusals")?
        .iter()
        .map(|v| v["tag"].as_str().context("refused tag"))
        .collect::<Result<_>>()?;
    ensure!(
        targets.len() == 105 && refusals.len() == 2 && targets.is_disjoint(&refusals),
        "category control selection"
    );
    let mut model =
        RvtDocExporter.export_with_limits(file, rvt::walker::WalkerLimits::default())?;
    for entity in &mut model.entities {
        if let IfcEntity::BuildingElement {
            ifc_type,
            type_guid: Some(tag),
            ..
        } = entity
        {
            if targets.contains(tag.as_str()) {
                *ifc_type = "IFCBUILDINGELEMENTPROXY".into();
            }
            if refusals.contains(tag.as_str()) {
                *ifc_type = "IFCDUCTFITTING".into();
            }
        }
    }
    saved_meshes::attach(file, &mut model)?;
    let mut admitted = BTreeSet::new();
    let mut refused = BTreeSet::new();
    for entity in &model.entities {
        let IfcEntity::BuildingElement {
            type_guid: Some(tag),
            property_set: Some(set),
            ..
        } = entity
        else {
            continue;
        };
        let text = |name: &str| {
            set.properties.iter().find_map(|p| match &p.value {
                PropertyValue::Text(value) if p.name == name => Some(value.as_str()),
                _ => None,
            })
        };
        if targets.contains(tag.as_str()) {
            ensure!(
                text("BodySource") == Some("saved_graphics_mesh"),
                "overridden fitting refused: {tag}"
            );
            ensure!(admitted.insert(tag.as_str()), "duplicate fitting: {tag}");
        }
        if refusals.contains(tag.as_str()) {
            ensure!(
                text("BodySource") == Some("partition_element_record_bbox"),
                "overridden terminal admitted: {tag}"
            );
            let expected = pin["refusal_controls"]
                .as_array()
                .unwrap()
                .iter()
                .find(|row| row["tag"] == *tag)
                .unwrap();
            ensure!(
                text("SavedMeshRefusal") == expected["reason"].as_str(),
                "terminal refusal: {tag}"
            );
            ensure!(refused.insert(tag.as_str()), "duplicate terminal: {tag}");
        }
    }
    ensure!(
        admitted == targets && refused == refusals,
        "missing category controls"
    );
    Ok(
        json!({"fittings_with_proxy_class_admitted":admitted.len(),"terminals_with_duct_class_refused":refused.len()}),
    )
}

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    ensure!(args.len() == 2, "usage: probe_collapsed_ruled <sample.rvt>");
    let digest = hex(&Sha256::digest(std::fs::read(&args[1])?));
    let mut file = RevitFile::open_with_limits(&args[1], rvt::cli::open_limits(None, None, true))?;
    let options = Options {
        selected_ids: BTreeSet::from(OWNERS),
        channels: BTreeSet::from([103]),
        record_extras: true,
        keep_body: true,
        ..Options::default()
    };
    let mut rows = Vec::new();
    let mut seen = BTreeSet::new();
    extract_graphs(&mut file, &options, |record| {
        let owner = record.identity.element_id;
        ensure!(seen.insert(owner), "duplicate source record");
        let g = record.graph.as_ref().context("source graph refused")?;
        let body = record.body.as_ref().context("source body")?;
        for (si, object) in g.objects.iter().enumerate() {
            if object.class_name != "RuledSurf" {
                continue;
            }
            let f = &object.fields;
            let nulls: Vec<usize> = [1, 2]
                .into_iter()
                .filter(|s| f[format!("m_pProfileCurve{s}")]["pointer_token"].as_u64() == Some(0))
                .collect();
            if nulls.is_empty() {
                continue;
            }
            ensure!(nulls.len() == 1, "both profiles null in witness");
            let side = nulls[0];
            let partner = pointer(g, si, &f[format!("m_pProfileCurve{}", 3 - side)])?
                .context("other profile")?;
            let pointer_offset = f[format!("m_pProfileCurve{side}")]["offset"]
                .as_u64()
                .context("pointer offset")? as usize;
            let point_offset = object
                .fields_end
                .checked_sub((3 - side) * 24)
                .context("point offset")?;
            let pointer_bytes = body
                .get(pointer_offset..pointer_offset + 4)
                .context("pointer bytes")?;
            ensure!(pointer_bytes == [0; 4], "null pointer bytes differ");
            let point_bytes = body
                .get(point_offset..point_offset + 24)
                .context("point bytes")?;
            let point: [f64; 3] = std::array::from_fn(|i| {
                f64::from_le_bytes(point_bytes[i * 8..i * 8 + 8].try_into().unwrap())
            });
            ensure!(
                json!(point) == f[format!("m_Point{side}")].to_json(),
                "point bytes differ"
            );
            let surface = ParametricSurface::read(g, si)?;
            for u in [0., 0.5, 1.] {
                for v in [0., 0.5, 1.] {
                    ensure!(
                        surface
                            .evaluate(u, v)?
                            .iter()
                            .chain(surface.normal(u, v)?.iter())
                            .all(|x| x.is_finite()),
                        "nonfinite evaluation"
                    );
                }
            }
            let mut faces = Vec::new();
            for (fi, obj) in g.objects.iter().enumerate() {
                if obj.class_name == "Face" && pointer(g, fi, &obj.fields["m_pSurf"])? == Some(si) {
                    let mesh = face(g, fi)?.context("missing face mesh")?;
                    let grid = face_grid(g, fi)?;
                    faces.push(json!({"face":fi,"vertices":mesh.vertices.len(),"triangles":mesh.triangles.len(),"u_knots":grid.u.len(),"v_knots":grid.v.len(),"bounds":grid.bounds,"rectangular":grid.rectangular,"nonrectangular_refusal":nonrectangular_control(g,fi)?}));
                }
            }
            ensure!(!faces.is_empty(), "no face for surface");
            rows.push(json!({"owner":owner,"stream":record.source.stream,"group_record_offset":record.source.group_record_offset,"body_sha256":record.source.body_sha256,"surface":si,"surface_start":object.start,"null_side":side,"profile_class":g.objects[partner].class_name,"pointer_offset":pointer_offset,"pointer_hex":hex(pointer_bytes),"point_offset":point_offset,"point_hex":hex(point_bytes),"point_feet":point,"faces":faces,"controls":controls(g,si,side,partner)?}));
        }
        Ok(())
    })?;
    ensure!(seen == BTreeSet::from(OWNERS), "missing source records");
    ensure!(
        rows.len() == 76,
        "expected 76 measured surfaces, got {}",
        rows.len()
    );
    let categories = category_controls(&mut file)?;
    println!(
        "{}",
        serde_json::to_string(
            &json!({"file_sha256":digest,"records":seen.len(),"surfaces":rows.len(),"rows":rows,"category_controls":categories})
        )?
    );
    Ok(())
}
