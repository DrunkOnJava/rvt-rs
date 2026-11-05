//! RE-351 (#1471): what separates the 16 walls of Autodesk's `rac_basic` sample
//! project that two of its saves draw to different heights.
//!
//! FACT under test: on the 2025 and 2026 saves of `rac_basic` the 16 walls
//! 428588, 428745, 430064, 506386, 234869, 429964, 430318, 430361, 430412,
//! 430859, 497540, 506797, 768442, 937935, 938974 and 939084 hold the same
//! record box, centreline, joins and top-join elements, and the difference in
//! how tall they are drawn is the top cut `export_content::wall_tops` makes at
//! the element their top joins name, roof 243274: a record box on the saves
//! where its shape (a roof by extrusion, RE-211) is not read, where a plain
//! extrusion is taken for a floor with a level underside, and a solid, left
//! out, on the saves where it is. The probe prints each save's stored facts and
//! its written bodies so that the saves can be set side by side, wall by wall.
//!
//! It runs on `rac_basic`'s saves only (a file whose name has no `racbasic`
//! once the underscores are dropped prints one line and stops) and prints one
//! JSON object per line:
//!
//!   kind "model"          the file, the release that saved it and the release
//!                         whose layout its records take
//!   kind "levels"         every Level's ElementId, name and elevation (feet)
//!   kind "frame"          each element-record frame of the 16 walls and the
//!                         roof: stream, offset, category, bounding box (feet),
//!                         container, placement kind, class tag, flags, the
//!                         counted reference list (the type, the base and top
//!                         Levels and the joined walls), design option
//!   kind "decoded"        the 16 walls' fields the partition reader hands the
//!                         writer: the box, the centreline, the type thickness
//!                         and every `m_wall_*` field (the join partners and
//!                         reaches, the top join's elements), in order
//!   kind "native_wall"    each wall's native `SWall` record as
//!                         `native_walls_and_top_joins` reads it: curve,
//!                         joins, join types, top-join and cut-pair elements,
//!                         base join elements; or "native_walls_error"
//!   kind "native_record"  each native record of the 16 walls and the roof:
//!                         the root object's fields (base and top constraints,
//!                         offsets) and the join and cut steps
//!   kind "wall"           every wall of the export: its entity as written (the
//!                         placement, the extrusion, the profile points, the
//!                         solid shape) with a hash of the extrusion, and its
//!                         `RvtElementRecordGeometry` properties, among them
//!                         `BodySource`, `BodyHeightSource` and
//!                         `TopCoverElementId`; "listed" says it is one of the 16
//!   kind "roof"           every roof and every element a wall names as its top
//!                         cover, as written
//!
//! Long arrays print as their length and a hash. `tools/re` holds no scorer for
//! it: the saves' lines are compared with a script (`RE-351`'s report, section
//! 5).
//!
//! Usage:
//!   cargo run --profile ci -p rvt-probes --example probe_re351_wall_top_covers -- MODEL.rvt [--records]

use std::collections::{BTreeMap, BTreeSet};

use anyhow::{Context, Result};
use rvt::RevitFile;
use rvt::ifc::entities::{IfcEntity, PropertySet};
use rvt::ifc::{ExportOptions, run_export_with_limits};
use rvt::research::native_document::{NATIVE_RELEASES, Options, extract_graphs};
use rvt::research::native_walls::native_walls_and_top_joins;
use rvt::research::partition_element_records as per;
use rvt::research::partition_element_records_2023 as per32;
use rvt::research::partition_level_records::recover_partition_levels;
use rvt::research::partition_schema_mvp::recover_partition_schema_mvp;
use rvt::walker::{InstanceField, WalkerLimits};
use serde_json::{Map, Value, json};

const WALLS: [u32; 16] = [
    428588, 428745, 430064, 506386, 234869, 429964, 430318, 430361, 430412, 430859, 497540, 506797,
    768442, 937935, 938974, 939084,
];
const ROOF: u32 = 243274;
const LONG_ARRAY: usize = 48;
const CLIP: usize = 6000;
const STEP_CLASSES: [&str; 8] = [
    "JoinToRoofGStep",
    "CutPairGStep",
    "CleanPairGStep",
    "JoinEndGStep",
    "WallEmbedWallGStep",
    "BaseWallGStep",
    "CDJoinInfo",
    "GLine",
];

fn fnv(text: &str) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    for byte in text.bytes() {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0100_0000_01b3);
    }
    hash
}

/// `value`, an array longer than `LONG_ARRAY` replaced by its length and a hash.
fn compact(value: Value) -> Value {
    match value {
        Value::Array(items) if items.len() > LONG_ARRAY => {
            let hash = fnv(&Value::Array(items.clone()).to_string());
            json!({ "len": items.len(), "fnv": format!("{hash:016x}") })
        }
        Value::Array(items) => Value::Array(items.into_iter().map(compact).collect()),
        Value::Object(members) => Value::Object(
            members
                .into_iter()
                .map(|(name, member)| (name, compact(member)))
                .collect(),
        ),
        other => other,
    }
}

fn clip(value: Value) -> Value {
    let text = value.to_string();
    if text.len() <= CLIP {
        value
    } else {
        json!({ "clipped": text.len(), "head": text.chars().take(CLIP).collect::<String>() })
    }
}

/// An object's members without pointers, history tables as their length.
fn trimmed(fields: &Value) -> Value {
    let Some(members) = fields.as_object() else {
        return clip(fields.clone());
    };
    let mut kept = Map::new();
    for (name, member) in members {
        if name.ends_with("HistTable") {
            kept.insert(
                format!("{name}#"),
                Value::from(member.as_array().map_or(0, Vec::len)),
            );
        } else if member.get("pointer_token").is_none() {
            kept.insert(name.clone(), member.clone());
        }
    }
    clip(Value::Object(kept))
}

fn field_json(field: &InstanceField) -> Value {
    match field {
        InstanceField::Float { value, .. } => json!(value),
        InstanceField::Integer { value, .. } => json!(value),
        InstanceField::Bool(value) => json!(value),
        InstanceField::ElementId { id, .. } => json!(id),
        InstanceField::String(text) => json!(text),
        InstanceField::Vector(items) => Value::Array(items.iter().map(field_json).collect()),
        _ => Value::Null,
    }
}

fn tag_of(entity: &IfcEntity) -> Option<u32> {
    match entity {
        IfcEntity::BuildingElement {
            type_guid: Some(tag),
            ..
        } => tag.parse().ok(),
        _ => None,
    }
}

/// A property set's properties as a name to value object.
fn flat(set: &Option<PropertySet>) -> Map<String, Value> {
    let mut out = Map::new();
    if let Some(set) = set {
        for property in &set.properties {
            let value = serde_json::to_value(&property.value).unwrap_or_default();
            let plain = value.get("value").cloned().unwrap_or(value);
            out.insert(property.name.clone(), plain);
        }
    }
    out
}

/// The ElementIds a wall's `TopCoverElementId` and `TopCoverElementIds` name.
fn covers_of(props: &Map<String, Value>) -> Vec<u32> {
    let mut out = Vec::new();
    if let Some(id) = props
        .get("TopCoverElementId")
        .and_then(Value::as_u64)
        .and_then(|id| u32::try_from(id).ok())
    {
        out.push(id);
    }
    if let Some(text) = props.get("TopCoverElementIds").and_then(Value::as_str) {
        out.extend(text.split(',').filter_map(|id| id.parse::<u32>().ok()));
    }
    out
}

fn is_rac_basic(path: &str) -> bool {
    std::path::Path::new(path)
        .file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| {
            name.to_ascii_lowercase()
                .replace('_', "")
                .contains("racbasic")
        })
}

fn main() -> Result<()> {
    let path = std::env::args()
        .skip(1)
        .find(|arg| !arg.starts_with("--"))
        .context("usage: probe_re351_wall_top_covers MODEL.rvt")?;
    if !is_rac_basic(&path) {
        println!(
            "{}",
            json!({ "kind": "skipped", "reason": "not a rac_basic save" })
        );
        return Ok(());
    }
    let mut rf = RevitFile::open(&path)?;
    let saved = rf.basic_file_info()?.version;
    let release = rf.layout_release()?;
    println!(
        "{}",
        json!({ "kind": "model", "saved": saved, "layout": release })
    );

    match recover_partition_levels(&mut rf, release) {
        Ok(levels) => println!(
            "{}",
            json!({
                "kind": "levels",
                "levels": levels
                    .iter()
                    .map(|level| json!([level.element_id, level.name, level.elevation_feet]))
                    .collect::<Vec<Value>>(),
            })
        ),
        Err(error) => println!(
            "{}",
            json!({ "kind": "levels", "error": error.to_string() })
        ),
    }

    let wanted = |id: u32| id == ROOF || WALLS.contains(&id);
    let records: Vec<per::PartitionElementRecord> = if per32::is_32bit_release(release) {
        per32::scan_records(&mut rf, release)
    } else {
        let table = rvt::elem_table::parse_records(&mut rf)?;
        let declared = rvt::elem_table::declared_ids(&table);
        per::scan_category_records_multi(
            &mut rf,
            release,
            &[per::OST_WALLS, per::OST_ROOFS],
            &declared,
        )?
    };
    for record in records.iter().filter(|record| wanted(record.element_id)) {
        println!(
            "{}",
            json!({
                "kind": "frame",
                "id": record.element_id,
                "category": record.builtin_category,
                "stream": record.stream,
                "offset": record.offset,
                "bbox_feet": record.bbox_feet,
                "container": record.container,
                "placement_kind": record.placement_kind,
                "class_tag": record.class_tag,
                "flags": record.flags,
                "references": record.references,
                "design_option": record.design_option,
                "enclosing_id": record.id_from_enclosing_record,
            })
        );
    }

    match recover_partition_schema_mvp(&mut rf, saved, WalkerLimits::default()) {
        Ok(mvp) => {
            for wall in &mvp.walls {
                let Some(id) = wall.id.filter(|id| WALLS.contains(id)) else {
                    continue;
                };
                let fields: Vec<Value> = wall
                    .fields
                    .iter()
                    .filter(|(name, _)| {
                        name.starts_with("m_wall_")
                            || name.starts_with("m_bbox")
                            || name.starts_with("m_location")
                            || name == "m_source_stream"
                    })
                    .map(|(name, value)| json!([name, field_json(value)]))
                    .collect();
                println!(
                    "{}",
                    json!({ "kind": "decoded", "id": id, "fields": compact(Value::Array(fields)) })
                );
            }
        }
        Err(error) => println!(
            "{}",
            json!({ "kind": "decoded", "error": error.to_string() })
        ),
    }

    let ids: BTreeSet<u64> = WALLS.iter().map(|&id| u64::from(id)).collect();
    match native_walls_and_top_joins(&mut rf, &ids) {
        Ok(reads) => {
            for (id, wall) in &reads.walls {
                println!(
                    "{}",
                    json!({
                        "kind": "native_wall",
                        "id": id,
                        "start": wall.start,
                        "end": wall.end,
                        "flipped": wall.flipped,
                        "cross_section": wall.cross_section,
                        "joins": wall.joins,
                        "join_types": wall.join_types,
                        "shells": wall.shells,
                        "faces": wall.faces,
                        "exposed": wall.exposed,
                        "side_joins": wall.side_joins,
                        "partners": wall.partners,
                        "type_id": wall.type_id,
                        "top_joined": reads.top_joined.get(id),
                        "uncut": reads.uncut.contains(id),
                        "top_cutters": reads.top_cutters.get(id),
                        "base_cutters": reads.base_cutters.get(id),
                        "pair_only": reads.pair_only.get(id),
                        "whole": reads.whole.contains(id),
                        "bounded": reads.bounded.get(id),
                    })
                );
            }
        }
        Err(error) => println!(
            "{}",
            json!({ "kind": "native_walls_error", "error": error.to_string() })
        ),
    }

    let mut selected = ids.clone();
    selected.insert(u64::from(ROOF));
    let options = Options {
        selected_ids: selected,
        releases: NATIVE_RELEASES,
        ..Options::default()
    };
    let read = extract_graphs(&mut rf, &options, |record| {
        let (Some(class), Some(graph)) = (record.class_name.as_deref(), record.graph.as_ref())
        else {
            return Ok(());
        };
        let Some(root) = graph.objects.first() else {
            return Ok(());
        };
        let steps: Vec<Value> = graph
            .objects
            .iter()
            .skip(1)
            .filter(|object| STEP_CLASSES.contains(&object.class_name.as_str()))
            .map(|object| json!({ "class": object.class_name, "fields": trimmed(&object.fields) }))
            .collect();
        let mut classes: BTreeMap<&str, usize> = BTreeMap::new();
        for object in &graph.objects {
            *classes.entry(object.class_name.as_str()).or_default() += 1;
        }
        println!(
            "{}",
            json!({
                "kind": "native_record",
                "id": record.identity.element_id,
                "class": class,
                "stream": record.source.stream,
                "root": trimmed(&root.fields),
                "classes": classes,
                "steps": steps,
            })
        );
        Ok(())
    });
    if let Err(error) = read {
        println!(
            "{}",
            json!({ "kind": "native_record_error", "error": error.to_string() })
        );
    }

    let model =
        run_export_with_limits(&mut rf, &ExportOptions::default(), WalkerLimits::default())?.model;
    let mut covers: BTreeSet<u32> = BTreeSet::new();
    for entity in &model.entities {
        let IfcEntity::BuildingElement {
            ifc_type,
            property_set,
            ..
        } = entity
        else {
            continue;
        };
        if !ifc_type.to_ascii_uppercase().starts_with("IFCWALL") {
            continue;
        }
        let Some(id) = tag_of(entity) else {
            continue;
        };
        let props = flat(property_set);
        covers.extend(covers_of(&props));
        let value = serde_json::to_value(entity).unwrap_or_default();
        let listed = WALLS.contains(&id);
        let mut line = json!({
            "kind": "wall",
            "id": id,
            "listed": listed,
            "ifc_type": ifc_type,
            "location_feet": value["location_feet"],
            "rotation_radians": value["rotation_radians"],
            "height_feet": value["extrusion"]["height_feet"],
            "extrusion_fnv": format!("{:016x}", fnv(&value["extrusion"].to_string())),
            "solid_shape": compact(value["solid_shape"].clone()),
            "props": props,
        });
        if listed {
            line["entity"] = compact(value);
        }
        println!("{line}");
    }
    for entity in &model.entities {
        let IfcEntity::BuildingElement {
            ifc_type,
            property_set,
            ..
        } = entity
        else {
            continue;
        };
        let Some(id) = tag_of(entity) else {
            continue;
        };
        if !(ifc_type.to_ascii_uppercase() == "IFCROOF" || id == ROOF || covers.contains(&id)) {
            continue;
        }
        let value = serde_json::to_value(entity).unwrap_or_default();
        println!(
            "{}",
            json!({
                "kind": "roof",
                "id": id,
                "ifc_type": ifc_type,
                "props": flat(property_set),
                "entity": compact(value),
            })
        );
    }
    eprintln!(
        "{} frame(s) of the 16 walls and the roof, {} top cover(s) named",
        records
            .iter()
            .filter(|record| wanted(record.element_id))
            .count(),
        covers.len()
    );
    Ok(())
}
