//! Standalone family bodies from the shared native revolution reader.
//!
//! The measured scope is one visible positive revolution in a Revit 2020
//! family, without another model form or nested instance. Project instances
//! need their type evaluation and placement checked separately. No saved
//! display mesh or external oracle supplies this body's coordinates.
use super::IfcModel;
use super::entities::{
    CurveSegment, IfcEntity, ProfileDef, Property, PropertySet, PropertyValue, SolidShape,
};
use crate::control::WalkerControl;
use crate::native_form_geometry::{self, FormDocument};
use crate::native_parameters::{self, ObjectGraph};
use anyhow::{Result, ensure};
use std::collections::BTreeSet;

pub(super) const BODY_SOURCE: &str = "family_revolution";

/// Retain the decoded lines and arcs as an analytic IFC profile. The
/// display mesh is deliberately not used to construct the exported solid.
fn analytic_shape(revolution: &native_form_geometry::Revolution) -> Result<SolidShape> {
    let [outline] = revolution.profile.loops.as_slice() else {
        anyhow::bail!("requires one analytic profile loop");
    };
    ensure!(
        !outline
            .curves
            .iter()
            .any(|c| matches!(c, native_form_geometry::Curve::Arc { .. }))
            || (revolution.end - revolution.start - std::f64::consts::TAU).abs() < 1e-10,
        "partial revolution with an arc profile lacks native export confirmation"
    );
    let curve = super::family_extrusions::indexed_curve(outline, true, 1.0)
        .ok_or_else(|| anyhow::anyhow!("unwritable revolution profile"))?;
    let original_lines = outline
        .curves
        .iter()
        .filter(|c| matches!(c, native_form_geometry::Curve::Line { .. }))
        .count();
    ensure!(
        curve
            .segments
            .iter()
            .filter(|s| matches!(s, CurveSegment::Line(_)))
            .count()
            == original_lines,
        "arc collapses at IFC precision"
    );
    let rounded: Vec<[f64; 2]> = curve
        .points
        .iter()
        .map(|&(x, y)| [x, y].map(|v| format!("{:.6}", v * 0.3048).parse().unwrap_or(f64::NAN)))
        .collect();
    let mut points = BTreeSet::new();
    for p in &rounded {
        ensure!(
            p.iter().all(|v| v.is_finite()),
            "nonfinite written profile point"
        );
        ensure!(
            points.insert(p.map(|v| if v == 0.0 { 0 } else { v.to_bits() })),
            "profile points coincide at IFC precision"
        );
    }
    for segment in &curve.segments {
        if let CurveSegment::Arc(indices) = segment {
            let [a, b, c] = indices.map(|i| rounded[i]);
            let cross = (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0]);
            ensure!(
                cross.is_finite() && cross.abs() > 1e-16,
                "collinear arc at IFC precision"
            );
        }
    }
    let axis = revolution.axis_direction;
    let radial = revolution.radial_direction;
    let side = [
        axis[1] * radial[2] - axis[2] * radial[1],
        axis[2] * radial[0] - axis[0] * radial[2],
        axis[0] * radial[1] - axis[1] * radial[0],
    ];
    let (s, c) = revolution.start.sin_cos();
    Ok(SolidShape::PlacedRevolvedArea {
        profile: ProfileDef::IndexedPolyCurve {
            points: curve.points,
            segments: curve.segments,
        },
        origin_feet: revolution.axis_origin,
        axis_direction: axis,
        radial_direction: std::array::from_fn(|i| c * radial[i] + s * side[i]),
        angle_radians: revolution.end - revolution.start,
    })
}

fn visible(graph: &ObjectGraph) -> bool {
    let values: Vec<_> = graph
        .objects
        .iter()
        .filter(|o| o.class_name == "ParamValueSetInt")
        .filter_map(|o| o.fields.get("m_paramSet")?.as_array())
        .flatten()
        .filter(|p| {
            p.get("m_paramId")
                .and_then(crate::native_json::identifier_i64)
                == Some(-1_006_205)
        })
        .map(|p| p.get("m_value").and_then(crate::graph_value::Fv::as_i64))
        .collect();
    values == [Some(1)]
}

/// Append one fully decoded family body, atomically. Other documents return
/// zero. An unsupported candidate returns a reason with the model unchanged.
pub(super) fn attach(
    file: &mut crate::RevitFile,
    model: &mut IfcModel,
    control: &WalkerControl,
) -> Result<usize> {
    let streams = file.stream_names();
    if !streams.iter().any(|s| s == "PartAtom")
        || streams.iter().any(|s| s == "ProjectInformation")
        || file.layout_release()? != 2020
    {
        return Ok(0);
    }
    control.check()?;
    let source = native_form_geometry::open(file)?;
    // Discover candidates without decoding supporting sketch graphs. In
    // particular, a family without revolutions cannot exhaust their budget.
    let mut candidates = Vec::new();
    native_form_geometry::walk(file, &source, &[101, 102, 103], |entry| {
        control.check()?;
        if entry.class == Some("RevolutionElem") {
            candidates.push(entry.id);
            ensure!(
                candidates.len() <= 2000,
                "family revolution record budget exceeded"
            );
        }
        Ok(())
    })?;
    if candidates.is_empty() {
        return Ok(0);
    }
    let mut document = FormDocument::default();
    let mut forms = Vec::new();
    let mut unsupported = false;
    let mut records = 0usize;
    let limits = native_parameters::GraphLimits {
        max_values: 200_000,
        max_objects: 100_000,
        max_depth: 128,
    };
    let walked = native_form_geometry::walk(file, &source, &[101, 102, 103], |entry| {
        control.check()?;
        if matches!(
            entry.class,
            Some(
                "ExtrusionElem"
                    | "SweepElem"
                    | "BlendElem"
                    | "SweptBlendElem"
                    | "FreeFormElement"
                    | "Text3dElem"
                    | "FamilyInstance"
                    | "ImportInstance"
            )
        ) {
            unsupported = true;
        }
        if !matches!(
            entry.class,
            Some("RevolutionElem" | "CurveElem" | "SketchPlane" | "VarSketch")
        ) {
            return Ok(());
        }
        records += 1;
        ensure!(records <= 2000, "family revolution record budget exceeded");
        let graph =
            native_parameters::decode_graph_with_limits(entry.body, &source.registry, &limits)?;
        ensure!(
            graph.consumed_bytes == entry.body.len(),
            "incomplete family form record"
        );
        if entry.class == Some("RevolutionElem") {
            forms.push((entry.id, visible(&graph)));
        }
        document.insert(entry.id, graph);
        Ok(())
    })?;
    if forms.is_empty() {
        return Ok(0);
    }
    ensure!(walked.unframed_groups == 0, "unframed family groups");
    ensure!(
        !unsupported,
        "family contains other model forms or nested instances"
    );
    ensure!(forms.len() == 1, "requires one standalone revolution");
    ensure!(forms[0].1, "revolution visibility is not explicitly on");
    ensure!(
        !model
            .entities
            .iter()
            .any(|e| matches!(e, IfcEntity::BuildingElement { .. })),
        "family already has exported elements"
    );
    control.check()?;
    let revolution = document
        .revolution(forms[0].0)
        .map_err(anyhow::Error::msg)?;
    let shape = analytic_shape(&revolution)?;
    control.check()?;
    model.entities.push(IfcEntity::BuildingElement {
        // The form's geometry is known; a building-product category is not.
        ifc_type: "IFCBUILDINGELEMENTPROXY".into(),
        name: format!("Revolution {}", revolution.id),
        type_guid: Some(revolution.id.to_string()),
        predefined_type: None,
        storey_index: None,
        material_index: None,
        property_set: Some(PropertySet {
            name: "RvtFamilyFormGeometry".into(),
            properties: vec![
                Property {
                    name: "BodySource".into(),
                    value: PropertyValue::Text(BODY_SOURCE.into()),
                },
                Property {
                    name: "RevitClass".into(),
                    value: PropertyValue::Text("RevolutionElem".into()),
                },
            ],
        }),
        location_feet: Some([0.0; 3]),
        rotation_radians: None,
        extrusion: None,
        host_element_index: None,
        material_layer_set_index: None,
        material_profile_set_index: None,
        solid_shape: Some(shape),
        representation_map_index: None,
    });
    Ok(1)
}
