//! #1603, RE-479: does a Revit 2015 to 2023 project hold the own documents of
//! the families its doors and windows come from, and do their forms decode?
//!
//! FACT to check: RE-204 to RE-206 read, from a loaded family's own document
//! (a keyed block of the project's partitions, named by the 16-byte key of its
//! `Family` record, RE-163), the solid forms' `m_materialId`, `m_cutting` and
//! stored `Visible`, to tell when Revit writes `<Unnamed>` and which materials
//! only hidden forms hold. `native_family_forms` reads them on Revit 2023 to
//! 2027 and takes an ElementId in the 2024 shape `{"m_id": {"m_id64": n}}`
//! only; a save before 2024 holds `{"m_id": {"m_id": n}}` (RE-280). RE-478
//! found that the type map alone is wrong on 389 of 992 doors and windows for
//! want of these rules. Does the older release hold the documents, and in
//! which id shape do the forms hold their materials and their `Visible`?
//!
//! Output, counts only (the held-out scrub keeps the `family forms pre2024:`
//! lines by their fixed wording):
//!   family forms pre2024: release R families F with a document D forms T nested instances N
//!   family forms pre2024: class C forms n material in the 2015 to 2023 shape a in the 2024 shape b
//!       unset c none d cutting e visible stored f visible bound g
//!
//! The families are those of the doors and windows the export writes (the
//! master symbol of each instance's record, and that symbol's `m_familyId`).
//!
//! Usage:
//!   cargo run --profile ci -p rvt-probes --example probe_1603_family_forms_pre2024 -- MODEL.rvt

use std::collections::{BTreeMap, BTreeSet};

use rvt::RevitFile;
use rvt::ifc::RvtDocExporter;
use rvt::ifc::entities::IfcEntity;
use rvt::research::graph_value::Fv;
use rvt::research::native_document::{NATIVE_RELEASES, Options, extract_graphs};
use rvt::research::native_family_forms::family_records_in;

const CLASSES: [&str; 2] = ["IFCWINDOW", "IFCDOOR"];

/// The form classes `native_family_forms` reads, and the nested instances.
const FORM_CLASSES: [&str; 7] = [
    "ExtrusionElem",
    "RevolutionElem",
    "SweepElem",
    "BlendElem",
    "SweptBlendElem",
    "FreeFormElement",
    "Text3dElem",
];

/// `IS_VISIBLE_PARAM`.
const VISIBLE_PARAMETER: i64 = -1_006_205;

/// The ElementId in `value`: `(2015 to 2023 shape, 2024 shape)`.
fn shapes(value: Option<&Fv>) -> (Option<i64>, Option<i64>) {
    let Some(value) = value else {
        return (None, None);
    };
    (
        value.pointer("/m_id/m_id").and_then(Fv::as_i64),
        value.pointer("/m_id/m_id64").and_then(Fv::as_i64),
    )
}

/// The id of `value` in either shape.
fn id_in_either(value: Option<&Fv>) -> Option<i64> {
    let (older, newer) = shapes(value);
    older.or(newer)
}

/// Counts of one form class.
#[derive(Default)]
struct Counts {
    forms: usize,
    older: usize,
    newer: usize,
    unset: usize,
    none: usize,
    cutting: usize,
    stored: usize,
    bound: usize,
}

fn main() -> anyhow::Result<()> {
    let path = std::env::args().nth(1).expect("usage: MODEL.rvt");
    let mut rf = RevitFile::open(&path)?;
    let version = rf.basic_file_info()?.version;
    let model = RvtDocExporter.export_with_diagnostics(&mut rf)?.model;
    let mut elements: BTreeSet<u64> = BTreeSet::new();
    for entity in &model.entities {
        if let IfcEntity::BuildingElement {
            ifc_type,
            type_guid: Some(tag),
            ..
        } = entity
        {
            if CLASSES.contains(&ifc_type.to_ascii_uppercase().as_str()) {
                if let Ok(id) = tag.parse() {
                    elements.insert(id);
                }
            }
        }
    }

    // The master symbols of the instances, then their families.
    let mut masters: BTreeSet<u64> = BTreeSet::new();
    extract_graphs(
        &mut rf,
        &Options {
            selected_ids: elements,
            releases: NATIVE_RELEASES,
            ..Options::default()
        },
        |record| {
            if let Some(master) = record
                .graph
                .as_ref()
                .and_then(|graph| graph.objects.first())
                .and_then(|root| id_in_either(root.fields.get("m_masterSymbolId")))
                .and_then(|id| u64::try_from(id).ok())
                .filter(|id| *id != 0)
            {
                masters.insert(master);
            }
            Ok(())
        },
    )?;
    let mut families: BTreeSet<u64> = BTreeSet::new();
    if !masters.is_empty() {
        extract_graphs(
            &mut rf,
            &Options {
                selected_ids: masters,
                releases: NATIVE_RELEASES,
                ..Options::default()
            },
            |record| {
                if let Some(family) = record
                    .graph
                    .as_ref()
                    .and_then(|graph| graph.objects.first())
                    .and_then(|root| id_in_either(root.fields.get("m_familyId")))
                    .and_then(|id| u64::try_from(id).ok())
                    .filter(|id| *id != 0)
                {
                    families.insert(family);
                }
                Ok(())
            },
        )?;
    }

    let mut classes: BTreeSet<String> = FORM_CLASSES.iter().map(ToString::to_string).collect();
    classes.insert("FamilyInstance".into());
    let documents = family_records_in(&mut rf, &families, &classes, &NATIVE_RELEASES, None);
    let documents = match documents {
        Ok(documents) => documents,
        Err(_) => {
            println!("family forms pre2024: release {version} read failed");
            return Ok(());
        }
    };

    let mut by_class: BTreeMap<String, Counts> = BTreeMap::new();
    let (mut forms, mut nested) = (0usize, 0usize);
    for records in documents.values() {
        for (_, graph) in records {
            let Some(root) = graph.objects.first() else {
                continue;
            };
            if root.class_name == "FamilyInstance" {
                nested += 1;
                continue;
            }
            forms += 1;
            let counts = by_class.entry(root.class_name.clone()).or_default();
            counts.forms += 1;
            let (older, newer) = shapes(root.fields.get("m_materialId"));
            match (older, newer) {
                (Some(-1), _) | (_, Some(-1)) => counts.unset += 1,
                (Some(_), _) => counts.older += 1,
                (_, Some(_)) => counts.newer += 1,
                _ => counts.none += 1,
            }
            counts.cutting += usize::from(
                root.fields
                    .get("m_cutting")
                    .and_then(Fv::as_bool)
                    .unwrap_or(false),
            );
            let visible = |entry: &Fv, field: &str| id_in_either(entry.get(field));
            counts.stored += usize::from(
                graph
                    .objects
                    .iter()
                    .filter(|object| object.class_name == "ParamValueSetInt")
                    .filter_map(|object| object.fields.get("m_paramSet")?.as_array())
                    .flatten()
                    .any(|entry| visible(entry, "m_paramId") == Some(VISIBLE_PARAMETER)),
            );
            counts.bound += usize::from(
                graph
                    .objects
                    .iter()
                    .filter(|object| object.class_name == "FamilyParametrizedElemParamsCell")
                    .filter_map(|object| object.fields.get("m_paramDrivenData")?.as_array())
                    .flatten()
                    .any(|entry| visible(entry, "m_elemPropId") == Some(VISIBLE_PARAMETER)),
            );
        }
    }
    println!(
        "family forms pre2024: release {version} families {} with a document {} forms {forms} nested instances {nested}",
        families.len(),
        documents.len()
    );
    for (class, counts) in &by_class {
        println!(
            "family forms pre2024: class {class} forms {} material in the 2015 to 2023 shape {} in the 2024 shape {} unset {} none {} cutting {} visible stored {} visible bound {}",
            counts.forms,
            counts.older,
            counts.newer,
            counts.unset,
            counts.none,
            counts.cutting,
            counts.stored,
            counts.bound
        );
    }
    Ok(())
}
