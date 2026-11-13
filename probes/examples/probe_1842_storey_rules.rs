//! #1842, #1864: which storey does Revit's export put an element in when its
//! record names two or more Levels, and which Level does that come from?
//!
//! FACT under test: an element record that names exactly one Level is bound to
//! it (RE-27), one that names exactly two to its base constraint, the higher of
//! the two at or below the record's box base, else the lower (RE-59), and one
//! that names three or more is left unbound. RE-175 found that a beam's or
//! brace's Reference Level is the highest Level its record lists (83 of 83 on the
//! timber office), and Revit's storey for such a beam may be that Level and not
//! the base constraint: on the two timber office files 63 beams are in a
//! different storey from Revit's, and a confidential Revit 2027 structural model
//! reports 424 beams one to six storeys lower than Revit's and 73 columns and
//! beams on no storey (HWT-25 on #1842).
//!
//! The probe prints, for every placed instance whose record names two or more
//! Levels, the Levels it names with the record's box base and top, and the
//! storey rvt-rs's export puts it in. It does not compare them with Revit's
//! export: `tools/re/storey_rules_vs_ifc.py` does, and scores each rule that
//! picks one of the named Levels. It prints ElementIds and Level names, so for a
//! model fetched without a licence to redistribute it the run's scrub drops its
//! output (`tools/ci/scrub-holdout-results.sh` keeps only the count lines it
//! names).
//!
//! Output (one model per run):
//!   file revit=<release> levels=<n> storeys=<n> records=<n>
//!   level id=<ElementId> elev=<feet> story=<yes|no|?> name=<the rest of the line>
//!   storey index=<n> elev=<feet> name=<the rest of the line>
//!   named tag=<ElementId> cat=<category, spaces as _> class=<IFC entity|-> n=<Levels named>
//!     levels=<ElementIds in list order, or -> base=<feet> top=<feet> ours=<storey index|->
//!     second=<ElementIds, or ->
//!     (one line per placed instance whose record names two or more Levels, and
//!     for the structural framing, columns and foundations, floors and stairs also
//!     those that name none or one: `levels` are the Levels the record's first
//!     counted reference list names, `second` those its second list names (RE-174,
//!     RE-175: the list a beam's Reference Level is read from), `base` and `top`
//!     the record box's lowest and highest z, `ours` the storey the element itself
//!     is contained in)
//!
//! Usage:
//!   cargo run --profile ci -p rvt-probes --example probe_1842_storey_rules -- MODEL.rvt

use std::collections::{BTreeSet, HashMap};

use rvt::RevitFile;
use rvt::ifc::entities::IfcEntity;
use rvt::ifc::{ExportOptions, run_export};
use rvt::research::element_record_level_refs::named_levels;
use rvt::research::partition_element_records as per;

/// The Levels the record's second counted reference list names, in list order.
/// The first list is `record.references`; the second follows it, framed alike.
fn second_levels(
    rf: &mut RevitFile,
    record: &per::PartitionElementRecord,
    level_ids: &BTreeSet<u32>,
) -> Vec<u32> {
    let Ok(inflated) = rf.inflated_partition(&record.stream) else {
        return Vec::new();
    };
    let buf = inflated.bytes();
    let Some(at) = record.offset.checked_add(per::REFERENCE_LIST_OFFSET) else {
        return Vec::new();
    };
    let Some(first) = per::decode_reference_list(buf, at) else {
        return Vec::new();
    };
    let second = at
        .checked_add(4 + first.len() * 8)
        .and_then(|at| per::decode_reference_list(buf, at))
        .unwrap_or_default();
    named_levels(&second, level_ids)
}

fn main() -> anyhow::Result<()> {
    let path = std::env::args()
        .skip(1)
        .find(|arg| !arg.starts_with("--"))
        .ok_or_else(|| anyhow::anyhow!("usage: probe_1842_storey_rules MODEL.rvt"))?;
    let mut rf = RevitFile::open_with_limits(&path, rvt::cli::open_limits(None, None, false))?;
    let version = rf.basic_file_info()?.version;
    let declared = rvt::elem_table::declared_ids(&rvt::elem_table::parse_records(&mut rf)?);
    let levels =
        rvt::research::partition_level_records::recover_partition_levels(&mut rf, version)?;
    let level_ids: BTreeSet<u32> = levels.iter().map(|level| level.element_id).collect();
    let model = run_export(&mut rf, &ExportOptions::default())?.model;
    // Each exported element by its Tag: its IFC entity and its storey.
    let mut exported: HashMap<u32, (String, Option<usize>)> = HashMap::new();
    for entity in &model.entities {
        let IfcEntity::BuildingElement {
            ifc_type,
            type_guid,
            storey_index,
            ..
        } = entity
        else {
            continue;
        };
        let Some(tag) = type_guid.as_deref().and_then(|tag| tag.parse::<u32>().ok()) else {
            continue;
        };
        exported
            .entry(tag)
            .or_insert_with(|| (ifc_type.clone(), *storey_index));
    }
    let categories: Vec<i64> = per::BUILTIN_CATEGORY_NAMES
        .iter()
        .map(|(id, _)| *id)
        .chain(per::PRODUCT_RECORD_CATEGORIES.iter().map(|(id, _)| *id))
        .collect::<BTreeSet<i64>>()
        .into_iter()
        .collect();
    let records = per::scan_category_records_multi(&mut rf, version, &categories, &declared)?;
    println!(
        "file revit={version} levels={} storeys={} records={}",
        levels.len(),
        model.building_storeys.len(),
        records.len()
    );
    for level in &levels {
        let story = match level.building_story {
            Some(true) => "yes",
            Some(false) => "no",
            None => "?",
        };
        println!(
            "level id={} elev={:.4} story={story} name={}",
            level.element_id, level.elevation_feet, level.name
        );
    }
    for (index, storey) in model.building_storeys.iter().enumerate() {
        println!(
            "storey index={index} elev={:.4} name={}",
            storey.elevation_feet, storey.name
        );
    }
    let mut seen: BTreeSet<u32> = BTreeSet::new();
    for record in &records {
        if !record.is_exported_instance() || seen.contains(&record.element_id) {
            continue;
        }
        let named = named_levels(&record.references, &level_ids);
        let category = per::builtin_category_name(record.builtin_category)
            .unwrap_or("unnamed")
            .replace(' ', "_");
        // A record that names none or one Level is listed for the categories whose
        // elements #1842 reports on no storey or on a storey below Revit's.
        let structural = matches!(
            category.as_str(),
            "Structural_Framing"
                | "Structural_Foundations"
                | "Structural_Columns"
                | "Floors"
                | "Stairs"
                | "Stairs:_Runs"
                | "Stairs:_Landings"
        );
        if named.len() < 2 && !structural {
            continue;
        }
        seen.insert(record.element_id);
        let (class, ours) = exported
            .get(&record.element_id)
            .map_or(("-".to_string(), None), |(class, storey)| {
                (class.clone(), *storey)
            });
        let listed: Vec<String> = named.iter().map(u32::to_string).collect();
        let second: Vec<String> = second_levels(&mut rf, record, &level_ids)
            .iter()
            .map(u32::to_string)
            .collect();
        println!(
            "named tag={} cat={category} class={class} n={} levels={} base={:.4} top={:.4} ours={} second={}",
            record.element_id,
            named.len(),
            if listed.is_empty() {
                "-".to_string()
            } else {
                listed.join(",")
            },
            record.bbox_feet[2],
            record.bbox_feet[5],
            ours.map_or("-".to_string(), |index| index.to_string()),
            if second.is_empty() {
                "-".to_string()
            } else {
                second.join(",")
            }
        );
    }
    Ok(())
}
