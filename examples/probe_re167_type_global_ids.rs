//! RE-167 (probe): how Revit's exporter makes a type object's GlobalId
//! (B60).
//!
//! Revit's open-source exporter (revit-ifc, `GUIDUtil.CreateInternal`) gives
//! a family type the GlobalId of its symbol, unless the type's key needs a
//! hash: then the GlobalId is MD5 of
//! `<symbol GlobalId> + "Sub-element:" + <key>` read as a GUID, where the key
//! is `"Flipped: <bool>"` for a door or window, then `" Material: <id>"` when
//! the instance draws in a single material, then `" InAssembly: <bool>"`.
//!
//! For each type object of Revit's export beside the model, the probe tries
//! the symbol's own GlobalId and every such key, with the symbol taken as the
//! type's `Tag`, rvt-rs's type id for its elements, or an element itself, and
//! every material of the file, and prints which reproduces Revit's GlobalId.
//!
//! Usage: probe_re167_type_global_ids <model.rvt>

use md5::{Digest, Md5};
use rvt::RevitFile;
use rvt::partition_schema_mvp::TYPE_ID_FIELD;
use rvt::revit_global_ids::{canonical_guid, compress_ifc_guid, revit_global_ids};
use rvt::walker::InstanceField;
use std::collections::{BTreeMap, BTreeSet};
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

/// The first quoted string: an entity's GlobalId.
fn first_quoted(args: &str) -> Option<String> {
    Some(args.split('\'').nth(1)?.to_string())
}

/// The `Tag` of a type object: the quoted string after the 7 attributes
/// before it (`GlobalId, OwnerHistory, Name, Description, ApplicableOccurrence,
/// HasPropertySets, RepresentationMaps`), and of an element its last quoted.
fn quoted_numbers(args: &str) -> Vec<u32> {
    args.split('\'')
        .skip(1)
        .step_by(2)
        .filter_map(|s| s.parse().ok())
        .collect()
}

fn hashed(key: &str) -> String {
    let digest = Md5::digest(key.as_bytes());
    let bytes: [u8; 16] = digest.as_slice().try_into().expect("16 bytes");
    compress_ifc_guid(canonical_guid(bytes))
}

fn main() -> anyhow::Result<()> {
    let path = PathBuf::from(std::env::args().nth(1).expect("model path"));
    let reference = path.with_extension("ifc");
    if !reference.exists() {
        println!("no reference export");
        return Ok(());
    }
    let ents = entities(&std::fs::read_to_string(&reference)?);
    let mut rf = RevitFile::open(&path)?;
    let version = rf.basic_file_info()?.version;
    let ids = revit_global_ids(&mut rf)?;
    let declared: BTreeSet<u32> = rvt::elem_table::declared_element_ids(&mut rf)?
        .into_iter()
        .collect();
    let materials: Vec<u32> =
        rvt::partition_materials::scan_material_names(&mut rf, version, &declared)?
            .into_keys()
            .collect();
    let levels: Vec<u32> =
        rvt::partition_level_records::recover_partition_levels(&mut rf, version)?
            .into_iter()
            .map(|level| level.element_id)
            .collect();
    println!("levels {levels:?}, materials {}", materials.len());
    let mut our_type: BTreeMap<u32, u32> = BTreeMap::new();
    for element in rvt::walker::iter_elements(&mut rf)? {
        let type_id = element.fields.iter().find_map(|(name, value)| match value {
            InstanceField::ElementId { id, .. } if name == TYPE_ID_FIELD => Some(*id),
            _ => None,
        });
        if let (Some(id), Some(type_id)) = (element.id, type_id) {
            our_type.insert(id, type_id);
        }
    }
    let mut summary: BTreeMap<String, usize> = BTreeMap::new();
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
        let Some(revit_gid) = first_quoted(ty_args) else {
            continue;
        };
        let type_tag = quoted_numbers(ty_args).last().copied();
        let elements: Vec<u32> = related
            .iter()
            .skip(1)
            .filter_map(|e| ents.get(e))
            .filter_map(|(_, a)| quoted_numbers(a).last().copied())
            .collect();
        let mut symbols: BTreeSet<u32> = type_tag.into_iter().collect();
        symbols.extend(elements.iter().filter_map(|e| our_type.get(e)));
        symbols.extend(elements.iter().copied());
        let mut matched: Vec<String> = Vec::new();
        for &symbol in &symbols {
            let Some(symbol_gid) = ids.get(&symbol) else {
                continue;
            };
            let whose = if Some(symbol) == type_tag {
                "type tag"
            } else if elements.contains(&symbol) {
                "element"
            } else {
                "our type"
            };
            if symbol_gid == &revit_gid {
                matched.push(format!("{whose} {symbol}: its own GlobalId"));
                continue;
            }
            for flip in [None, Some(false), Some(true)] {
                for index in [None, Some(1), Some(2)] {
                    for level in std::iter::once(None).chain(levels.iter().map(Some)) {
                        for copy in [None, Some(1), Some(2)] {
                            for material in std::iter::once(None).chain(materials.iter().map(Some))
                            {
                                for assembly in [false, true] {
                                    let mut key = String::new();
                                    if let Some(flip) = flip {
                                        key += if flip {
                                            "Flipped: True"
                                        } else {
                                            "Flipped: False"
                                        };
                                    }
                                    if let Some(index) = index {
                                        key += &format!(" Index: {index}");
                                    }
                                    if let Some(level) = level {
                                        key += &format!(" Level: {level}");
                                    }
                                    if let Some(copy) = copy {
                                        key += &format!(" Copy: {copy}");
                                    }
                                    if let Some(material) = material {
                                        key += &format!(" Material: {material}");
                                    }
                                    key += if assembly {
                                        " InAssembly: True"
                                    } else {
                                        " InAssembly: False"
                                    };
                                    if hashed(&format!("{symbol_gid}Sub-element:{key}"))
                                        == revit_gid
                                    {
                                        matched.push(format!("{whose} {symbol}: hash of {key:?}"));
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        let form = matched
            .first()
            .map(|m| {
                let mut shape = String::new();
                for part in [
                    "Flipped",
                    "Index",
                    "Level",
                    "Copy",
                    "Material",
                    "InAssembly: True",
                    "own GlobalId",
                ] {
                    if m.contains(part) {
                        shape += part;
                        shape += " ";
                    }
                }
                shape
            })
            .unwrap_or_else(|| "none".into());
        *summary.entry(format!("{ty_entity}: {form}")).or_default() += 1;
        println!(
            "{ty_entity} tag {type_tag:?} {revit_gid} elements {elements:?}: {}",
            if matched.is_empty() {
                "no key reproduces it".to_string()
            } else {
                matched.join(" | ")
            }
        );
    }
    println!("summary: {summary:?}");
    Ok(())
}
