//! #1864: which elements of two saves of one project differ in their body or
//! profile, what each save's export writes for them, and what each save's
//! element record holds.
//!
//! FACT under test: between Autodesk's 2025 and 2026 saves of `rac_advanced`
//! and `rac_basic`, `rvt-diff --elements` (#1843) lists 4 and 2 elements whose
//! body differs, one of them with a profile that the 2025 save gives
//! (`ProfileSource`, `ProfileVertexCount`, `ProfileVoidCount`) and the 2026
//! save does not, and cannot say whether the model changed between the saves
//! or rvt-rs reads the two saves of one element differently. This probe names
//! those elements, prints each side's body as the export writes it (the
//! extrusion's size and profile kind, the solid's kind, the location and
//! rotation) with the properties that say how it was chosen, and prints what
//! each save's element record holds for them: its box, container and placement
//! kind. Equal records with different bodies are a reading that differs (or
//! sketch data the record does not hold); records that differ are a change in
//! the model.
//!
//! The probe:
//! - with one file, compares its export with a second export of the same file;
//! - with two (Measure's `probe_pair_release`: a sample, then the same project
//!   saved by another release), compares their exports.
//!
//! Output (the samples are Autodesk's and redistributable; the held-out models
//! are not run with this probe, which prints names and ElementIds):
//!   body_diff <IFC entity> tag=<ElementId> name=<name> fields=<field>,...
//!   body_side side=<left|right> tag=<ElementId> ext=<w>x<d>x<h> profile=<kind|none>
//!     solid=<kind|none> location=<x,y,z> rotation=<radians> BodySource=<..>
//!     ProfileSource=<..> ProfileVertexCount=<..> ProfileVoidCount=<..> ProfileResolved=<..>
//!     (feet; `absent` where the export writes no such property)
//!   body_diff_elements <n> of <changed> changed
//!   body_record tag=<ElementId> side=<left|right> record=<category> stream=<name>
//!     offset=<n> container=<raw> placement=<raw> box=<6 feet> refs=<n>
//!     (one line per record found; `record=none` where the side holds none)
//!
//! Usage:
//!   cargo run --profile ci -p rvt-probes --example probe_1864_body_diff -- A.rvt [B.rvt]

use std::collections::{BTreeMap, BTreeSet};

use rvt::RevitFile;
use rvt::ifc::element_diff::{ElementState, diff_elements, element_states};
use rvt::ifc::entities::IfcEntity;
use rvt::research::partition_element_records as per;
use serde_json::Value;

/// The set the body and profile properties live in, as an element's properties are keyed.
const SET: &str = "RvtElementRecordGeometry.";

/// The properties that say how an element's body and profile were chosen.
const BODY_PROPERTIES: [&str; 5] = [
    "BodySource",
    "ProfileSource",
    "ProfileVertexCount",
    "ProfileVoidCount",
    "ProfileResolved",
];

/// One save's export: the element states, and each element's body as text.
struct Side {
    states: Vec<ElementState>,
    shapes: BTreeMap<String, String>,
}

fn open(path: &str) -> anyhow::Result<RevitFile> {
    Ok(RevitFile::open_with_limits(
        path,
        rvt::cli::open_limits(None, None, false),
    )?)
}

/// The name of an enum value from its `Debug` text, without its fields.
fn variant(debug: String) -> String {
    debug
        .chars()
        .take_while(|c| c.is_alphanumeric() || *c == '_')
        .collect()
}

fn exported(path: &str) -> anyhow::Result<Side> {
    let mut rf = open(path)?;
    let run = rvt::ifc::run_export(&mut rf, &rvt::ifc::ExportOptions::default())?;
    let mut shapes = BTreeMap::new();
    for entity in &run.model.entities {
        let IfcEntity::BuildingElement {
            type_guid: Some(tag),
            extrusion,
            solid_shape,
            location_feet,
            rotation_radians,
            ..
        } = entity
        else {
            continue;
        };
        let ext = extrusion.as_ref().map_or("none".to_string(), |e| {
            format!(
                "{:.4}x{:.4}x{:.4} profile={}",
                e.width_feet,
                e.depth_feet,
                e.height_feet,
                e.profile_override
                    .as_ref()
                    .map_or("none".to_string(), |p| variant(format!("{p:?}")))
            )
        });
        let solid = solid_shape
            .as_ref()
            .map_or("none".to_string(), |s| variant(format!("{s:?}")));
        let at = (*location_feet).map_or("-".to_string(), |l| {
            format!("{:.4},{:.4},{:.4}", l[0], l[1], l[2])
        });
        let turn = (*rotation_radians).map_or("-".to_string(), |r| format!("{r:.6}"));
        shapes.entry(tag.clone()).or_insert(format!(
            "ext={ext} solid={solid} location={at} rotation={turn}"
        ));
    }
    Ok(Side {
        states: element_states(&run.model),
        shapes,
    })
}

/// A property value as text: `absent` where the element has none, otherwise
/// its value (`PropertyValue` serialises as `{"kind", "value"}`).
fn show(value: Option<&Value>) -> String {
    match value {
        None | Some(Value::Null) => "absent".to_string(),
        Some(other) => match other.get("value") {
            Some(Value::String(text)) => text.clone(),
            Some(inner) => inner.to_string(),
            None => other.to_string(),
        },
    }
}

/// What one file's element records hold for `tags`.
fn records(side: &str, path: &str, tags: &BTreeSet<u32>) -> anyhow::Result<()> {
    let mut rf = open(path)?;
    let version = rf.basic_file_info()?.version;
    let declared = rvt::elem_table::declared_ids(&rvt::elem_table::parse_records(&mut rf)?);
    let categories: Vec<i64> = per::BUILTIN_CATEGORY_NAMES
        .iter()
        .map(|(id, _)| *id)
        .chain(per::PRODUCT_RECORD_CATEGORIES.iter().map(|(id, _)| *id))
        .collect::<BTreeSet<i64>>()
        .into_iter()
        .collect();
    let found = per::scan_category_records_multi(&mut rf, version, &categories, &declared)?;
    for tag in tags {
        let mut any = false;
        for record in found.iter().filter(|record| record.element_id == *tag) {
            any = true;
            let b = record.bbox_feet;
            println!(
                "body_record tag={tag} side={side} record={} stream={} offset={} container={:#x} placement={:#x} box={:.4},{:.4},{:.4},{:.4},{:.4},{:.4} refs={}",
                per::builtin_category_name(record.builtin_category).unwrap_or("?"),
                record.stream,
                record.offset,
                record.container,
                record.placement_kind,
                b[0],
                b[1],
                b[2],
                b[3],
                b[4],
                b[5],
                record.references.len()
            );
        }
        if !any {
            println!("body_record tag={tag} side={side} record=none");
        }
    }
    Ok(())
}

fn main() -> anyhow::Result<()> {
    // Measure passes a single sample `--records` too.
    let files: Vec<String> = std::env::args()
        .skip(1)
        .filter(|arg| !arg.starts_with("--"))
        .collect();
    let (left_path, right_path) = match files.as_slice() {
        [one] => (one, one),
        [first, second, ..] => (first, second),
        [] => anyhow::bail!("usage: probe_1864_body_diff A.rvt [B.rvt]"),
    };
    let (left, right) = (exported(left_path)?, exported(right_path)?);
    let diff = diff_elements(&left.states, &right.states);
    let mut tags: BTreeSet<u32> = BTreeSet::new();
    let mut listed = 0usize;
    for change in &diff.changed {
        let about_body = change.changes.iter().any(|field| {
            field.field == "body"
                || BODY_PROPERTIES
                    .iter()
                    .any(|name| field.field == format!("property:{SET}{name}"))
        });
        if !about_body {
            continue;
        }
        listed += 1;
        let tag = change.tag.clone().unwrap_or_else(|| "-".to_string());
        println!(
            "body_diff {} tag={tag} name={} fields={}",
            change.ifc_type,
            change.name,
            change
                .changes
                .iter()
                .map(|field| field.field.replace("property:", ""))
                .collect::<Vec<_>>()
                .join(",")
        );
        for (side, from) in [("left", &left), ("right", &right)] {
            let state = from
                .states
                .iter()
                .find(|state| state.global_id == change.global_id);
            let properties = BODY_PROPERTIES
                .iter()
                .map(|name| {
                    let value = state.and_then(|s| s.properties.get(&format!("{SET}{name}")));
                    format!("{name}={}", show(value))
                })
                .collect::<Vec<_>>()
                .join(" ");
            println!(
                "body_side side={side} tag={tag} {} {properties}",
                from.shapes.get(&tag).map_or("no body", String::as_str)
            );
        }
        if let Ok(id) = tag.parse::<u32>() {
            tags.insert(id);
        }
    }
    println!(
        "body_diff_elements {listed} of {} changed",
        diff.changed.len()
    );
    if !tags.is_empty() {
        records("left", left_path, &tags)?;
        records("right", right_path, &tags)?;
    }
    Ok(())
}
