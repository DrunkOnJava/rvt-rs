//! 32 (#1046): which part of a curtain mullion's record the 18 mullions of the
//! BIM4LCA office that get no `Qto_MemberBaseQuantities.Length` are missing.
//!
//! FACT to be shown by the output joined with Revit's export of the same model:
//! Revit's export writes `Qto_MemberBaseQuantities.Length` on each of the
//! 10,003 `IfcMember`s it writes a base quantity set on, rvt-rs on 9,985, and
//! `Pset_MemberCommon.Span` is missing on the same 18 (`psets_vs_ifc` counts
//! Revit's 0.7 m 6 times and 0.100848 m 12 times). `add_mullion_span` writes
//! both from the extent of the mullion's record box along the model axis its
//! transform's Z lies on (`CURTAIN_MULLION_ALONG_FIELD`), or, of a mullion
//! turned or tilted off the model's axes, from that box's extent along its own
//! Z (`CURTAIN_AXES_FIELDS`, RE-106), and writes neither where one of those is
//! missing or is not a positive number. Which one the 18 are missing decides
//! whether the body's own extent can give Revit's value at all: the axis the
//! saved-mesh pass reads a mullion's length along is `mullion_box`'s, which is
//! the record box's, so a mullion with no box has no axis either.
//!
//! Output, one line per curtain mullion the export gives no `Length`:
//!   mullion <tag> set=<true|false> span=<true|false> box=<w,d,h ft|-> along=<axis|-> axes=<true|false> own=<x;y;z axes|-> body=<dx,dy,dz ft|-> <reason>
//! where `body` is the extent of the body the export wrote, in the element's
//! own frame, and the reason is one of `no_record_box` (a box field is
//! missing), `axes_recorded` (turned or tilted: the export writes both from the
//! oriented box), `box_flat_on_its_axis` (the box's extent along the axis it
//! runs along is not positive), `no_axis` (the transform gave no axis) and
//! `box_has_its_extent` (the export should have written one). A mullion the
//! export holds no base quantity set for prints `set=false` and is also
//! counted under `no base quantity set` (Revit's export writes none on 670 of
//! the office's 10,673 `IfcMember`s, but writes one on these 18). The tally by
//! reason over every curtain mullion goes to stderr.
//!
//! Usage:
//!   cargo run --profile ci -p rvt-probes --example probe_1046_mullion_length_gaps -- MODEL.rvt

use std::collections::BTreeMap;

use rvt::RevitFile;
use rvt::ifc::RvtDocExporter;
use rvt::ifc::entities::{IfcEntity, PropertySet, SolidShape};
use rvt::research::partition_schema_mvp::{
    CURTAIN_AXES_FIELDS, CURTAIN_MULLION_ALONG_FIELD, recover_partition_schema_mvp,
};
use rvt::walker::{InstanceField, WalkerLimits};

const BOX_FIELDS: [&str; 3] = ["m_bboxWidth", "m_bboxDepth", "m_bboxHeight"];
const QUANTITY_SET: &str = "Qto_MemberBaseQuantities";
const COMMON_SET: &str = "Pset_MemberCommon";

fn float(fields: &[(String, InstanceField)], wanted: &str) -> Option<f64> {
    fields.iter().find_map(|(name, value)| match value {
        InstanceField::Float { value, .. } if name == wanted => Some(*value),
        _ => None,
    })
}

fn integer(fields: &[(String, InstanceField)], wanted: &str) -> Option<i64> {
    fields.iter().find_map(|(name, value)| match value {
        InstanceField::Integer { value, .. } if name == wanted => Some(*value),
        _ => None,
    })
}

/// What the export writes on one member: a base quantity set, its `Length` and
/// its `Span`.
#[derive(Default)]
struct Written {
    set: bool,
    length: bool,
    span: bool,
}

fn holds(set: &PropertySet, wanted: &str) -> bool {
    set.properties
        .iter()
        .any(|property| property.name == wanted)
}

/// The extent of a mesh's vertices along the three axes of the frame they are
/// written in.
fn extents(vertices: &[[f64; 3]]) -> Option<[f64; 3]> {
    let first = *vertices.first()?;
    let (mut low, mut high) = (first, first);
    for vertex in vertices {
        for axis in 0..3 {
            low[axis] = low[axis].min(vertex[axis]);
            high[axis] = high[axis].max(vertex[axis]);
        }
    }
    Some([high[0] - low[0], high[1] - low[1], high[2] - low[2]])
}

fn main() -> anyhow::Result<()> {
    let path = std::env::args().nth(1).expect("usage: MODEL.rvt");
    let mut rf = RevitFile::open(&path)?;
    let version = rf.basic_file_info()?.version;
    let model = RvtDocExporter.export_with_diagnostics(&mut rf)?.model;
    // The base quantity set, its `Length` and the `Span` the export writes on
    // each member, by Tag.
    let mut written: BTreeMap<String, Written> = BTreeMap::new();
    for entity in &model.entities {
        let IfcEntity::ElementPropertySet { element, set } = entity else {
            continue;
        };
        let Some(IfcEntity::BuildingElement {
            ifc_type,
            type_guid: Some(tag),
            ..
        }) = model.entities.get(*element)
        else {
            continue;
        };
        if !ifc_type.eq_ignore_ascii_case("IFCMEMBER") {
            continue;
        }
        let entry = written.entry(tag.clone()).or_default();
        if set.name == QUANTITY_SET {
            entry.set = true;
            if holds(set, "Length") {
                entry.length = true;
            }
        }
        if set.name == COMMON_SET && holds(set, "Span") {
            entry.span = true;
        }
    }
    // The extent of each member's written body, where the saved-mesh pass drew
    // it from the mesh Revit saved for it.
    let mut bodies: BTreeMap<String, [f64; 3]> = BTreeMap::new();
    for entity in &model.entities {
        let IfcEntity::BuildingElement {
            ifc_type,
            type_guid: Some(tag),
            solid_shape: Some(SolidShape::TriangulatedFaceSet { vertices_feet, .. }),
            ..
        } = entity
        else {
            continue;
        };
        if !ifc_type.eq_ignore_ascii_case("IFCMEMBER") {
            continue;
        }
        if let Some(found) = extents(vertices_feet) {
            bodies.insert(tag.clone(), found);
        }
    }
    let mvp = recover_partition_schema_mvp(&mut rf, version, WalkerLimits::default())?;
    let mut tally: BTreeMap<&str, usize> = BTreeMap::new();
    for element in &mvp.products {
        if element.class != "CurtainWallMullion" {
            continue;
        }
        let Some(id) = element.id else {
            continue;
        };
        let tag = id.to_string();
        let entry = written.get(&tag);
        if entry.is_some_and(|written| written.length) {
            *tally.entry("length written").or_default() += 1;
            continue;
        }
        // A mullion with no base quantity set at all is classified too: on
        // `main` after #1741 the 18 hold none, not a `NetVolume` alone.
        let set = entry.is_some_and(|written| written.set);
        if !set {
            *tally.entry("no base quantity set").or_default() += 1;
        }
        let span = entry.is_some_and(|written| written.span);
        let box_size = BOX_FIELDS.map(|name| float(&element.fields, name));
        let along = integer(&element.fields, CURTAIN_MULLION_ALONG_FIELD)
            .and_then(|value| usize::try_from(value).ok());
        let axes = CURTAIN_AXES_FIELDS
            .iter()
            .all(|name| float(&element.fields, name).is_some());
        let reason = if box_size.iter().any(Option::is_none) {
            "no_record_box"
        } else if axes {
            "axes_recorded"
        } else if let Some(axis) = along.filter(|axis| *axis < 3) {
            if box_size[axis].is_some_and(|size| size > 0.0) {
                "box_has_its_extent"
            } else {
                "box_flat_on_its_axis"
            }
        } else {
            "no_axis"
        };
        *tally.entry(reason).or_default() += 1;
        println!(
            "mullion {tag} set={set} span={span} box={} along={} axes={axes} own={} body={} {reason}",
            box_size
                .map(|size| size.map_or("-".to_string(), |value| format!("{value:.6}")))
                .join(","),
            along.map_or("-".to_string(), |axis| axis.to_string()),
            // The recorded own axes, X;Y;Z, each as three components, to say
            // which model axis the own Z lies on (RE-226's fallback).
            CURTAIN_AXES_FIELDS
                .map(|name| float(&element.fields, name)
                    .map_or("-".to_string(), |v| format!("{v:.9}")))
                .chunks(3)
                .map(|axis| axis.join(","))
                .collect::<Vec<_>>()
                .join(";"),
            bodies.get(&tag).map_or("-".to_string(), |extent| format!(
                "{:.6},{:.6},{:.6}",
                extent[0], extent[1], extent[2]
            )),
        );
    }
    eprintln!("curtain mullions of {path}, by what the record gave: {tally:?}");
    Ok(())
}
