//! RE-298: doors and windows whose type draws all its geometry with no
//! material (#1247, RE-84, RE-149).
//!
//! FACT under test: RE-84 writes a door or window whose type has a value
//! block with no geometry-material map as its opening alone, as Revit's
//! export does for a family that only cuts its host. RE-82's map reader
//! takes a map only where some value is a material, so a type whose every
//! map value is unset (`0xff` x 8: geometry drawn with no material of its
//! own, RE-149) reads as having no map too. RE-149 gives such a column or
//! product `<Unnamed>` and clears RE-84's flag; a door or window keeps the
//! flag and is written as an opening. If the all-unset map means the type
//! draws geometry, Revit exports these doors and windows as the elements.
//!
//! Prints each door and window record, with the five columns of
//! `probe_re84_void_only_families` (ElementId, class, type ElementId, the
//! type's map material count or `-`/`?`, family), then:
//! - `u` where the type's value block opens with an all-unset map
//!   (`partition_type_materials::unset_material_types`), else `-`;
//! - the triangles of the element's saved graphics at fine detail, `-` where
//!   it has none;
//! - its family's `shows_unset` (`native_family_forms`, RE-204): `t`, `f`,
//!   `?` where undecided, `-` where the family is not read.
//!
//! `tools/re/void_only_vs_ifc.py` reads the first five columns, and counts
//! the sixth where present.
//!
//! ```text
//! cargo run --profile ci -p rvt-probes --example probe_re298_unset_map_doors -- model.rvt
//! ```

use std::collections::{BTreeMap, BTreeSet};

use rvt::research::native_document::{Options, extract_graphs};
use rvt::research::native_family_forms::family_forms;
use rvt::research::native_saved_scene::extract_at_detail;
use rvt::research::partition_schema_mvp::{FAMILY_NAME_FIELD, TYPE_ID_FIELD};
use rvt::research::partition_type_materials::{type_material_names, unset_material_types};
use rvt::walker::InstanceField;

fn main() -> anyhow::Result<()> {
    let Some(path) = std::env::args().nth(1) else {
        anyhow::bail!("usage: probe_re298_unset_map_doors FILE.rvt");
    };
    let mut rf = rvt::RevitFile::open(&path)?;
    let version = rf.basic_file_info()?.version;
    let maps = type_material_names(&mut rf, version);
    let unset = unset_material_types(&mut rf, version);
    let mvp = rvt::research::partition_schema_mvp::recover_partition_schema_mvp(
        &mut rf,
        version,
        rvt::walker::WalkerLimits::default(),
    )?;
    let elements: Vec<_> = mvp.doors.iter().chain(&mvp.windows).collect();
    let type_of = |element: &rvt::walker::DecodedElement| {
        element.fields.iter().find_map(|(name, value)| match value {
            InstanceField::ElementId { id, .. } if name == TYPE_ID_FIELD => Some(*id),
            _ => None,
        })
    };

    let types: BTreeSet<u64> = elements
        .iter()
        .filter_map(|e| type_of(e))
        .map(u64::from)
        .collect();
    let mut family_of: BTreeMap<u64, u64> = BTreeMap::new();
    let options = Options {
        selected_ids: types,
        ..Options::default()
    };
    if let Err(e) = extract_graphs(&mut rf, &options, |record| {
        let family = record
            .graph
            .as_ref()
            .and_then(|graph| graph.objects.first())
            .and_then(|root| root.fields.get("m_familyId"))
            .and_then(|id| id.pointer("/m_id/m_id64"))
            .and_then(serde_json::Value::as_i64)
            .and_then(|id| u64::try_from(id).ok());
        if let Some(family) = family {
            family_of.insert(record.identity.element_id, family);
        }
        Ok(())
    }) {
        eprintln!("type records not read: {e}");
    }
    let families: BTreeSet<u64> = family_of.values().copied().collect();
    let forms = family_forms(&mut rf, &families).unwrap_or_else(|e| {
        eprintln!("family forms not read: {e}");
        BTreeMap::new()
    });

    let ids: BTreeSet<u64> = elements
        .iter()
        .filter_map(|e| e.id)
        .map(u64::from)
        .collect();
    let options = Options {
        selected_ids: ids,
        max_graph_values: 1_000_000,
        max_graph_objects: 1_000_000,
        ..Default::default()
    };
    let mut triangles: BTreeMap<u64, usize> = BTreeMap::new();
    match extract_at_detail(&mut rf, &options, 3) {
        Ok(scene) => {
            for element in &scene.elements {
                let n: usize = element
                    .meshes
                    .primitives
                    .iter()
                    .map(|p| p.triangles.len())
                    .sum();
                *triangles.entry(element.identity.element_id).or_default() += n;
            }
        }
        Err(e) => eprintln!("saved graphics not read: {e}"),
    }

    for element in elements {
        let type_id = type_of(element);
        let family = element.fields.iter().find_map(|(name, value)| match value {
            InstanceField::String(text) if name == FAMILY_NAME_FIELD => Some(text.as_str()),
            _ => None,
        });
        let materials = match type_id {
            None => "?".to_string(),
            Some(id) => maps.get(&id).map_or("-".into(), |m| m.len().to_string()),
        };
        let unset_map = if type_id.is_some_and(|id| unset.contains(&id)) {
            "u"
        } else {
            "-"
        };
        let saved = element
            .id
            .and_then(|id| triangles.get(&u64::from(id)))
            .filter(|n| **n > 0)
            .map_or("-".to_string(), |n| n.to_string());
        let shows = type_id
            .and_then(|id| family_of.get(&u64::from(id)))
            .and_then(|family| forms.get(family))
            .map_or("-", |forms| match forms.shows_unset {
                Some(true) => "t",
                Some(false) => "f",
                None => "?",
            });
        println!(
            "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
            element.id.map_or(String::new(), |id| id.to_string()),
            element.class,
            type_id.map_or(String::new(), |id| id.to_string()),
            materials,
            family.unwrap_or(""),
            unset_map,
            saved,
            shows,
        );
    }
    Ok(())
}
