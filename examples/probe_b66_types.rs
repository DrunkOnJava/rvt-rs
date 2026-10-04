//! B66 follow-up: each curtain wall mullion's and panel's type in rvt-rs
//! against Revit's export. Prints the element, the type id rvt-rs reads for
//! it, the schema class of that id's data object, and the `Tag` of the type
//! Revit's export relates it to (the model's `.ifc` beside it), with the
//! class of that id's data object.
//!
//! Usage: probe_b66_types <model.rvt>

use rvt::RevitFile;
use rvt::partition_room_parameters::verified_data_object;
use rvt::partition_schema_mvp::TYPE_ID_FIELD;
use rvt::walker::InstanceField;
use std::collections::BTreeMap;
use std::path::PathBuf;

fn entities(step: &str) -> BTreeMap<u64, (String, String)> {
    let mut out = BTreeMap::new();
    for line in step.lines() {
        let Some(rest) = line.strip_prefix('#') else {
            continue;
        };
        let Some((id, body)) = rest.split_once('=') else {
            continue;
        };
        let Some((entity, args)) = body.split_once('(') else {
            continue;
        };
        if let Ok(id) = id.trim().parse() {
            out.insert(id, (entity.trim().to_string(), args.to_string()));
        }
    }
    out
}

fn refs(args: &str) -> Vec<u64> {
    args.split('#')
        .skip(1)
        .filter_map(|s| {
            s.chars()
                .take_while(char::is_ascii_digit)
                .collect::<String>()
                .parse()
                .ok()
        })
        .collect()
}

fn tag(args: &str) -> Option<u32> {
    args.rsplit('\'').nth(1)?.parse().ok()
}

fn main() -> anyhow::Result<()> {
    let path = PathBuf::from(std::env::args().nth(1).expect("model path"));
    let reference = path.with_extension("ifc");
    if !reference.exists() {
        println!("no reference export");
        return Ok(());
    }
    let ents = entities(&std::fs::read_to_string(&reference)?);
    let mut revit_type: BTreeMap<u32, (String, Option<u32>)> = BTreeMap::new();
    for (entity, args) in ents.values() {
        if entity != "IFCRELDEFINESBYTYPE" {
            continue;
        }
        let all = refs(args);
        let Some((&ty, related)) = all.split_last() else {
            continue;
        };
        let Some((ty_entity, ty_args)) = ents.get(&ty) else {
            continue;
        };
        for element in related.iter().skip(1) {
            let Some((e, a)) = ents.get(element) else {
                continue;
            };
            if e != "IFCMEMBER" && e != "IFCPLATE" {
                continue;
            }
            if let Some(t) = tag(a) {
                revit_type.insert(t, (ty_entity.clone(), tag(ty_args)));
            }
        }
    }
    let mut rf = RevitFile::open(&path)?;
    let classes = rf.schema_classes()?;
    let class_name = |class: u32| {
        classes
            .classes
            .iter()
            .find(|c| u32::from(c.tag) == class & 0xffff)
            .map_or_else(|| format!("{class:#x}"), |c| c.name.clone())
    };
    let mut ours: BTreeMap<u32, (String, Option<u32>)> = BTreeMap::new();
    for element in rvt::walker::iter_elements(&mut rf)? {
        if element.class != "CurtainWallPanel" && element.class != "CurtainWallMullion" {
            continue;
        }
        let type_id = element.fields.iter().find_map(|(name, value)| match value {
            InstanceField::ElementId { id, .. } if name == TYPE_ID_FIELD => Some(*id),
            _ => None,
        });
        if let Some(id) = element.id {
            ours.insert(id, (element.class.clone(), type_id));
        }
    }
    let mut object_class: BTreeMap<u32, String> = BTreeMap::new();
    let wanted: Vec<u32> = ours
        .values()
        .filter_map(|(_, t)| *t)
        .chain(revit_type.values().filter_map(|(_, t)| *t))
        .collect();
    for stream in rf.partition_stream_names() {
        let Ok(inflated) = rf.inflated_partition(&stream) else {
            continue;
        };
        let b = inflated.bytes();
        for &id in &wanted {
            for start in memchr::memmem::find_iter(b, &id.to_le_bytes()) {
                if let Some(object) = verified_data_object(b, start) {
                    if object.element_id == id {
                        object_class.insert(id, class_name(object.class));
                    }
                }
            }
        }
    }
    let describe = |id: Option<u32>| {
        id.map_or("-".to_string(), |id| {
            format!(
                "{id} ({})",
                object_class.get(&id).map_or("no object", String::as_str)
            )
        })
    };
    for (id, (class, type_id)) in &ours {
        let theirs = revit_type.get(id);
        println!(
            "{id} {class}: rvt-rs type {} | Revit {} {}",
            describe(*type_id),
            theirs.map_or("-", |(e, _)| e.as_str()),
            describe(theirs.and_then(|(_, t)| *t))
        );
    }
    Ok(())
}
