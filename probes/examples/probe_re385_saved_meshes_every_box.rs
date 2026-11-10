//! RE-385: what keeps an element's saved display mesh from replacing the box
//! the export draws it as, and whether the meshes that replace it stay inside
//! that box.
//!
//! FACT under test: `saved_meshes::attach_scoped` replaces a body that is the
//! element's record box (RE-81) or a beam's box along its line (RE-328) with
//! the element's saved display mesh when that mesh decodes whole, for every
//! element on Revit 2023 to 2027 by default since #1191 (`--saved-meshes`
//! before it). On Revit 2025's Snowdon Towers Electrical sample that left
//! 300 of 2,428 elements as boxes. RE-385 found five reasons: a family instance
//! drawn as its record box turned to its angle (RE-87) was not a candidate;
//! a `GEllipse` curve was refused, as a `SurfRev` or `RuledSurf` profile; a
//! `RuledSurf` between two curves that are not both lines was refused; a
//! trim loop that repeats its first point before closing (conduit end caps)
//! was refused; and a `GInstance` whose `m_forbiddenTarget.m_targets` is 10
//! (the art-glass pendants' drops) is refused. The first four are read; the
//! fifth is not, since what the bits of `m_targets` name is not known, and
//! those pendants keep their box.
//!
//! Output, from three exports of the one file (saved meshes off, the default
//! scope, every element; the last two are the same on Revit 2023 to 2027):
//! - `scope <off|default|all> <BodySource>=<count> ...`: the bodies by source;
//! - `replaced <n> fits <n> larger <n>`: the elements whose body is a box with
//!   saved meshes off and a saved mesh in the export for every element, and
//!   of those how many have a mesh whose local extents, sorted, are each at
//!   most the box's (sorted) plus 0.02 ft, and how many do not;
//! - `larger <id> <box w d h> <mesh w d h>`: each that does not;
//! - `boxes <n> not offered <n>`: the elements still drawn as a box in the
//!   export for every element, and of those how many `attach_scoped` never
//!   offers a saved mesh (no placement, or a resolved profile: a floor);
//! - `left <message> <count>`: for the others, what their saved graphics report, digits
//!   replaced by `#`, one count per element and distinct message;
//!   `left (no saved graphics) <count>` for those with none;
//! - `unreported <id> <status> <primitives> <triangles>`: each element still
//!   a box whose saved graphics report nothing.
//!
//! Usage:
//!   cargo run --profile ci -p rvt-probes --example probe_re385_saved_meshes_every_box -- MODEL.rvt

use std::collections::{BTreeMap, BTreeSet};

use rvt::RevitFile;
use rvt::ifc::entities::{IfcEntity, PropertyValue, SolidShape};
use rvt::ifc::{ExportOptions, IfcModel, SavedMeshChoice, run_export};
use rvt::research::native_document::Options;
use rvt::research::native_saved_scene::extract_at_detail;

const BOXES: [&str; 3] = [
    "partition_element_record_bbox",
    "partition_beam_box_along_axis",
    "partition_family_instance_turned_box",
];
const SAVED: &str = "saved_graphics_mesh";
const SLACK_FEET: f64 = 0.02;

fn message_kind(message: &str) -> String {
    let text = message.split_once(": ").map_or(message, |(_, rest)| rest);
    text.chars()
        .map(|c| if c.is_ascii_digit() { '#' } else { c })
        .collect()
}

/// One element's `BodySource`, its body's sorted local extents (feet) where
/// it has an extrusion or a triangle mesh, and whether `attach_scoped` offers
/// it a saved mesh at all (it has a placement and no resolved profile).
type Body = (String, Option<[f64; 3]>, bool);

/// Each element with a `Tag` carried by no other entity, and its [`Body`].
fn bodies(model: &IfcModel) -> BTreeMap<u64, Body> {
    let wholes: BTreeSet<usize> = model
        .entities
        .iter()
        .filter_map(|entity| match entity {
            IfcEntity::Aggregate { whole, .. } => Some(*whole),
            _ => None,
        })
        .collect();
    let mut seen: BTreeMap<u64, usize> = BTreeMap::new();
    let mut out = BTreeMap::new();
    for (index, entity) in model.entities.iter().enumerate() {
        if wholes.contains(&index) {
            continue;
        }
        let IfcEntity::BuildingElement {
            type_guid: Some(tag),
            property_set: Some(set),
            extrusion,
            solid_shape,
            location_feet,
            ..
        } = entity
        else {
            continue;
        };
        let Ok(id) = tag.parse::<u64>() else {
            continue;
        };
        let source = set
            .properties
            .iter()
            .find_map(|p| match &p.value {
                PropertyValue::Text(t) if p.name == "BodySource" => Some(t.clone()),
                _ => None,
            })
            .unwrap_or_else(|| "-".into());
        let extents = match (solid_shape, extrusion) {
            (Some(SolidShape::TriangulatedFaceSet { vertices_feet, .. }), _) => {
                let mut lo = [f64::INFINITY; 3];
                let mut hi = [f64::NEG_INFINITY; 3];
                for v in vertices_feet {
                    for k in 0..3 {
                        lo[k] = lo[k].min(v[k]);
                        hi[k] = hi[k].max(v[k]);
                    }
                }
                Some([hi[0] - lo[0], hi[1] - lo[1], hi[2] - lo[2]])
            }
            (None, Some(e)) => Some([e.width_feet, e.depth_feet, e.height_feet]),
            _ => None,
        }
        .map(|mut d| {
            d.sort_by(f64::total_cmp);
            d
        });
        let resolved = set.properties.iter().any(|p| {
            p.name == "ProfileResolved" && matches!(p.value, PropertyValue::Boolean(true))
        });
        *seen.entry(id).or_default() += 1;
        out.insert(id, (source, extents, location_feet.is_some() && !resolved));
    }
    out.retain(|id, _| seen[id] == 1);
    out
}

fn export(path: &str, choice: SavedMeshChoice) -> anyhow::Result<IfcModel> {
    let mut rf = RevitFile::open(path)?;
    let options = ExportOptions::default().with_saved_meshes(choice);
    Ok(run_export(&mut rf, &options)?.model)
}

fn main() -> anyhow::Result<()> {
    let path = std::env::args().nth(1).expect("usage: MODEL.rvt");
    let mut by_scope = Vec::new();
    for (name, choice) in [
        ("off", SavedMeshChoice::Off),
        ("default", SavedMeshChoice::Default),
        ("all", SavedMeshChoice::All),
    ] {
        let found = bodies(&export(&path, choice)?);
        let mut sources: BTreeMap<&str, usize> = BTreeMap::new();
        for (source, ..) in found.values() {
            *sources.entry(source.as_str()).or_default() += 1;
        }
        let line: Vec<String> = sources.iter().map(|(s, n)| format!("{s}={n}")).collect();
        println!("scope {name} {}", line.join(" "));
        by_scope.push(found);
    }
    let (off, all) = (&by_scope[0], &by_scope[2]);

    let (mut replaced, mut fits, mut larger) = (0, 0, Vec::new());
    for (id, (source, extents, _)) in all {
        let Some((before, Some(boxed), _)) = off.get(id) else {
            continue;
        };
        if source != SAVED || !BOXES.contains(&before.as_str()) {
            continue;
        }
        let Some(mesh) = extents else {
            continue;
        };
        replaced += 1;
        if (0..3).all(|k| mesh[k] <= boxed[k] + SLACK_FEET) {
            fits += 1;
        } else {
            larger.push((*id, *boxed, *mesh));
        }
    }
    println!("replaced {replaced} fits {fits} larger {}", larger.len());
    for (id, b, m) in &larger {
        println!(
            "larger {id} {:.3} {:.3} {:.3} {:.3} {:.3} {:.3}",
            b[0], b[1], b[2], m[0], m[1], m[2]
        );
    }

    let boxes: Vec<(u64, bool)> = all
        .iter()
        .filter(|(_, (source, ..))| BOXES.contains(&source.as_str()))
        .map(|(id, (_, _, offered))| (*id, *offered))
        .collect();
    let left: BTreeSet<u64> = boxes
        .iter()
        .filter(|(_, offered)| *offered)
        .map(|(id, _)| *id)
        .collect();
    println!(
        "boxes {} not offered {}",
        boxes.len(),
        boxes.len() - left.len()
    );
    let mut rf = RevitFile::open(&path)?;
    let options = Options {
        selected_ids: left.clone(),
        max_graph_values: 1_000_000,
        max_graph_objects: 1_000_000,
        ..Default::default()
    };
    let scene = extract_at_detail(&mut rf, &options, 3)?;
    let mut reasons: BTreeMap<String, usize> = BTreeMap::new();
    let mut with_graphics = BTreeSet::new();
    for element in &scene.elements {
        let id = element.identity.element_id;
        if !left.contains(&id) {
            continue;
        }
        with_graphics.insert(id);
        let kinds: BTreeSet<String> = element
            .meshes
            .diagnostics
            .iter()
            .map(|d| message_kind(d))
            .collect();
        if kinds.is_empty() {
            *reasons
                .entry(format!("(status {})", element.status))
                .or_default() += 1;
            let triangles: usize = element
                .meshes
                .primitives
                .iter()
                .map(|p| p.triangles.len())
                .sum();
            println!(
                "unreported {id} {} {} {triangles}",
                element.status,
                element.meshes.primitives.len()
            );
        }
        for kind in kinds {
            *reasons.entry(kind).or_default() += 1;
        }
    }
    let mut reasons: Vec<_> = reasons.into_iter().collect();
    reasons.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    for (reason, n) in reasons {
        println!("left {reason} {n}");
    }
    println!(
        "left (no saved graphics) {}",
        left.len() - with_graphics.len()
    );
    Ok(())
}
