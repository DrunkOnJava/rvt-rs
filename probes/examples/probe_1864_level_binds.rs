//! #1864: which elements of two saves of one project differ in how their
//! Level was chosen, from what to what, and what each save's record names.
//!
//! FACT under test: the export's `RvtElementRecordGeometry` set says, for an
//! element whose record names a Level, which Level (`LevelElementId`), whether
//! one was found (`LevelBindResolved`) and by which rule it was chosen
//! (`LevelBindSource`: the record's single reference, the base constraint of
//! two, a host, the Level of the objects it names, its base elevation, ...,
//! `rvt::research::element_record_level_refs`). Between Autodesk's 2025 and
//! 2026 saves of `rst_basic`, `rvt-diff --elements` (#1843) lists 12 elements
//! that differ in these three properties and cannot say whether the model
//! changed between the saves or rvt-rs reads the two saves of one element
//! differently. This probe names those elements, prints the values on each
//! side, and prints what each save's element record holds for them: the
//! Levels the record's reference list names, the other elements it names, and
//! the record's box. Equal records with different binds are a reading that
//! differs; records that differ are a change in the model.
//!
//! The probe:
//! - with one file, compares its export with a second export of the same file;
//! - with two (Measure's `probe_pair_release`: a sample, then the same project
//!   saved by another release), compares their exports.
//!
//! Output (the samples are Autodesk's and redistributable; the held-out models
//! are not run with this probe, which prints names and ElementIds):
//!   level_bind <IFC entity> tag=<ElementId> | <property> <left> -> <right> | ... | name=<name>
//!   level_bind_elements <n> of <changed> changed
//!   level_bind_source <left source or absent> -> <right source or absent>: <n>
//!   levels side=<left|right> n=<count> <id>:<name>:<elevation feet>:<story>|...
//!     (<story> is `story` or `not-story` by the Level's Building Story
//!     setting, `story-unread` where the save's Levels do not say)
//!   level_refs tag=<ElementId> side=<left|right> record=<category> stream=<name> offset=<n>
//!     container=<raw> placement=<raw> base=<feet> top=<feet> refs=<slots>
//!     levels=<id>:<name>:<elevation feet>:<story>|... others=<ElementIds>
//!     (one line per record found; `record=none` where the side holds none)
//!
//! Usage:
//!   cargo run --profile ci -p rvt-probes --example probe_1864_level_binds -- A.rvt [B.rvt]

use std::collections::{BTreeMap, BTreeSet};

use rvt::RevitFile;
use rvt::ifc::element_diff::{ElementState, diff_elements, element_states};
use rvt::research::element_record_level_refs::named_levels;
use rvt::research::partition_element_records as per;
use serde_json::Value;

/// The set the Level properties live in, as `ElementDiff` keys a property.
const PREFIX: &str = "property:RvtElementRecordGeometry.";

/// The properties that say how an element's Level was chosen.
const LEVEL_PROPERTIES: [&str; 3] = ["LevelBindSource", "LevelElementId", "LevelBindResolved"];

/// The other elements a record's list names, as far as this lists them.
const OTHERS_SHOWN: usize = 16;

fn open(path: &str) -> anyhow::Result<RevitFile> {
    Ok(RevitFile::open_with_limits(
        path,
        rvt::cli::open_limits(None, None, false),
    )?)
}

fn states(path: &str) -> anyhow::Result<Vec<ElementState>> {
    let mut rf = open(path)?;
    let run = rvt::ifc::run_export(&mut rf, &rvt::ifc::ExportOptions::default())?;
    Ok(element_states(&run.model))
}

/// A property value as text: `absent` where the element has none, otherwise
/// its value (`PropertyValue` serialises as `{"kind", "value"}`).
fn show(value: &Value) -> String {
    match value {
        Value::Null => "absent".to_string(),
        other => match other.get("value") {
            Some(Value::String(text)) => text.clone(),
            Some(inner) => inner.to_string(),
            None => other.to_string(),
        },
    }
}

/// What one file's element records hold for `tags`: the Levels the file has,
/// then per record the Levels and other elements its reference list names.
fn records(side: &str, path: &str, tags: &BTreeSet<u32>) -> anyhow::Result<()> {
    let mut rf = open(path)?;
    let version = rf.basic_file_info()?.version;
    let declared = rvt::elem_table::declared_ids(&rvt::elem_table::parse_records(&mut rf)?);
    let levels =
        rvt::research::partition_level_records::recover_partition_levels(&mut rf, version)?;
    let by_id: BTreeMap<u32, (String, f64, Option<bool>)> = levels
        .iter()
        .map(|level| {
            (
                level.element_id,
                (
                    level.name.clone(),
                    level.elevation_feet,
                    level.building_story,
                ),
            )
        })
        .collect();
    let level_text = |id: &u32| -> String {
        match by_id.get(id) {
            Some((name, feet, story)) => {
                let story = match story {
                    Some(true) => "story",
                    Some(false) => "not-story",
                    None => "story-unread",
                };
                format!("{id}:{name}:{feet:.4}:{story}")
            }
            None => format!("{id}:?:?:?"),
        }
    };
    println!(
        "levels side={side} n={} {}",
        by_id.len(),
        by_id.keys().map(level_text).collect::<Vec<_>>().join("|")
    );
    let level_ids: BTreeSet<u32> = by_id.keys().copied().collect();
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
            let named = named_levels(&record.references, &level_ids);
            let mut others: Vec<u32> = Vec::new();
            for slot in &record.references {
                let Ok(id) = u32::try_from(*slot) else {
                    continue;
                };
                if id != record.element_id && !level_ids.contains(&id) && !others.contains(&id) {
                    others.push(id);
                }
            }
            let shown: Vec<String> = others
                .iter()
                .take(OTHERS_SHOWN)
                .map(u32::to_string)
                .collect();
            println!(
                "level_refs tag={tag} side={side} record={} stream={} offset={} container={:#x} placement={:#x} base={:.4} top={:.4} refs={} levels={} others={}{}",
                per::builtin_category_name(record.builtin_category).unwrap_or("?"),
                record.stream,
                record.offset,
                record.container,
                record.placement_kind,
                record.bbox_feet[2],
                record.bbox_feet[5],
                record.references.len(),
                named.iter().map(level_text).collect::<Vec<_>>().join("|"),
                shown.join(","),
                if others.len() > OTHERS_SHOWN {
                    format!(",+{}", others.len() - OTHERS_SHOWN)
                } else {
                    String::new()
                }
            );
        }
        if !any {
            println!("level_refs tag={tag} side={side} record=none");
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
        [] => anyhow::bail!("usage: probe_1864_level_binds A.rvt [B.rvt]"),
    };
    let diff = diff_elements(&states(left_path)?, &states(right_path)?);
    let mut by_source: BTreeMap<(String, String), usize> = BTreeMap::new();
    let mut tags: BTreeSet<u32> = BTreeSet::new();
    let mut listed = 0usize;
    for change in &diff.changed {
        let mut parts = Vec::new();
        let (mut before, mut after) = ("absent".to_string(), "absent".to_string());
        for field in &change.changes {
            let Some(name) = field.field.strip_prefix(PREFIX) else {
                continue;
            };
            if !LEVEL_PROPERTIES.contains(&name) {
                continue;
            }
            if name == "LevelBindSource" {
                before = show(&field.left);
                after = show(&field.right);
            }
            parts.push(format!(
                "{name} {} -> {}",
                show(&field.left),
                show(&field.right)
            ));
        }
        if parts.is_empty() {
            continue;
        }
        *by_source.entry((before, after)).or_default() += 1;
        listed += 1;
        if let Some(tag) = change.tag.as_deref().and_then(|tag| tag.parse().ok()) {
            tags.insert(tag);
        }
        println!(
            "level_bind {} tag={} | {} | name={}",
            change.ifc_type,
            change.tag.as_deref().unwrap_or("-"),
            parts.join(" | "),
            change.name
        );
    }
    println!(
        "level_bind_elements {listed} of {} changed",
        diff.changed.len()
    );
    for ((before, after), count) in &by_source {
        println!("level_bind_source {before} -> {after}: {count}");
    }
    if !tags.is_empty() {
        records("left", left_path, &tags)?;
        records("right", right_path, &tags)?;
    }
    Ok(())
}
