//! RE-155 (probe): what the slots of an element record's reference list at
//! +0x88 hold (#228).
//!
//! RE-23 found that every Revit 2024 element record carries a counted `u64`
//! reference list, whose slot before the record's own ElementId is the host
//! wall. RE-27 reads a Level from it. #228 asks what the other slots are:
//! a leading 3, and ids that look like family, type and level references.
//!
//! For every record of every partition's leading chain (RE-35) whose frame
//! decodes, the probe classifies each slot by what it equals, in this order:
//! - `three`: the value 3 in the first slot;
//! - `own`: the record's own ElementId;
//! - `type`: the element's type, as the partition name entries give it
//!   (RE-38, `iter_elements`' `m_type_id`);
//! - `level`: a Level's ElementId (RE-24);
//! - `declared`: an ElementId the ElemTable declares;
//! - `other`.
//!
//! Each slot is placed by where it sits: `slot k` counted from the start (up
//! to `slot 7 or more`), `before own` (the host slot), `own`, `after own +k`
//! (up to `+4 or more`). It prints, overall and for the commonest categories,
//! how many slots of each place hold each kind.
//!
//! Usage:
//!   cargo run --profile ci --example probe_re155_reference_slots -- MODEL.rvt ...

use rvt::RevitFile;
use rvt::partition_element_records as per;
use rvt::walker::InstanceField;
use std::collections::{BTreeMap, BTreeSet};

const CATEGORIES_SHOWN: usize = 8;

fn place(index: usize, len: usize, own: Option<usize>) -> String {
    match own {
        Some(o) if index == o => return "own".into(),
        Some(o) if index + 1 == o => return "before own".into(),
        Some(o) if index > o && index - o <= 3 => return format!("after own +{}", index - o),
        Some(o) if index > o => return "after own +4 or more".into(),
        None if index + 1 == len => return "last, no own".into(),
        _ => {}
    }
    if index <= 6 {
        format!("slot {index}")
    } else {
        "slot 7 or more".into()
    }
}

type Tally = BTreeMap<String, BTreeMap<&'static str, usize>>;

fn render(tally: &Tally) -> String {
    tally
        .iter()
        .map(|(place, kinds)| {
            let kinds: Vec<String> = kinds.iter().map(|(k, n)| format!("{k:?}:{n}")).collect();
            format!("{place:?}:{{{}}}", kinds.join(","))
        })
        .collect::<Vec<_>>()
        .join(",")
}

fn probe(path: &str) -> anyhow::Result<Vec<String>> {
    let mut rf = RevitFile::open(path)?;
    let revit = rf.basic_file_info()?.version;
    let Some(marker) = per::file_bbox_marker(&mut rf, revit) else {
        return Ok(vec![format!(
            "{{\"file\":{path:?},\"skipped\":\"no bbox marker\"}}"
        )]);
    };
    let declared = rvt::elem_table::declared_ids(&rvt::elem_table::parse_records(&mut rf)?);
    let levels: BTreeSet<u64> =
        rvt::partition_level_records::recover_partition_levels(&mut rf, revit)
            .unwrap_or_default()
            .iter()
            .map(|level| u64::from(level.element_id))
            .collect();
    let mut types: BTreeMap<u64, u64> = BTreeMap::new();
    for element in rvt::walker::iter_elements(&mut rf)? {
        let Some(id) = element.id else { continue };
        let type_id = element.fields.iter().find_map(|(name, value)| match value {
            InstanceField::ElementId { id, .. }
                if name == rvt::partition_schema_mvp::TYPE_ID_FIELD =>
            {
                Some(u64::from(*id))
            }
            _ => None,
        });
        if let Some(type_id) = type_id {
            types.insert(u64::from(id), type_id);
        }
    }
    let mut overall: Tally = BTreeMap::new();
    let mut by_category: BTreeMap<i64, (usize, Tally)> = BTreeMap::new();
    let mut records = 0usize;
    let mut seen: BTreeSet<u64> = BTreeSet::new();
    for stream in rf.partition_stream_names() {
        let Ok(inflated) = rf.inflated_partition(&stream) else {
            continue;
        };
        let buf = inflated.bytes();
        for span in per::partition_record_chain(buf, &marker) {
            if !seen.insert(span.element_id) {
                continue;
            }
            let Ok(own_id) = u32::try_from(span.element_id) else {
                continue;
            };
            let Some(frame) = per::decode_frame_as(&stream, buf, span.start, own_id, &marker)
            else {
                continue;
            };
            records += 1;
            let refs = &frame.references;
            let own = refs.iter().position(|slot| *slot == span.element_id);
            let category = by_category.entry(frame.builtin_category).or_default();
            category.0 += 1;
            for (index, slot) in refs.iter().enumerate() {
                let kind = if index == 0 && *slot == 3 {
                    "three"
                } else if *slot == span.element_id {
                    "own"
                } else if types.get(&span.element_id) == Some(slot) {
                    "type"
                } else if levels.contains(slot) {
                    "level"
                } else if u32::try_from(*slot).is_ok_and(|id| declared.contains(&id)) {
                    "declared"
                } else {
                    "other"
                };
                let at = place(index, refs.len(), own);
                *overall
                    .entry(at.clone())
                    .or_default()
                    .entry(kind)
                    .or_default() += 1;
                *category.1.entry(at).or_default().entry(kind).or_default() += 1;
            }
        }
    }
    let typed = types.len();
    let mut out = vec![format!(
        "{{\"file\":{path:?},\"revit\":{revit},\"records\":{records},\"elements_with_type\":{typed},\
         \"levels\":{},\"overall\":{{{}}}}}",
        levels.len(),
        render(&overall)
    )];
    let mut categories: Vec<(&i64, &(usize, Tally))> = by_category.iter().collect();
    categories.sort_by(|a, b| b.1.0.cmp(&a.1.0));
    for (category, (count, tally)) in categories.into_iter().take(CATEGORIES_SHOWN) {
        out.push(format!(
            "{{\"category\":{category},\"records\":{count},\"slots\":{{{}}}}}",
            render(tally)
        ));
    }
    Ok(out)
}

fn main() {
    // Measure passes flags such as `--records` after the paths.
    let paths: Vec<String> = std::env::args()
        .skip(1)
        .filter(|arg| !arg.starts_with("--"))
        .collect();
    for path in &paths {
        match probe(path) {
            Ok(lines) => lines.iter().for_each(|l| println!("{l}")),
            Err(error) => println!("{{\"file\":{path:?},\"error\":{:?}}}", format!("{error:#}")),
        }
    }
}
