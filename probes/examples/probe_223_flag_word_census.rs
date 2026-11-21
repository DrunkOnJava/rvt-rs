//! #223: a census of the element record's flag word at `+0x46`, bit by bit,
//! by BuiltInCategory and by whether Revit's own IFC4 export of the same
//! model writes the element.
//!
//! FACT under test: a bit of `+0x46` marks a property Revit's export writes,
//! so within one category the bit is set exactly on the elements the export
//! carries. RE-128, RE-219 and RE-261 measured three bits that way (7, 12 and
//! 0); the rest of the word is unattributed (#223). This probe decides no
//! bit: it prints the counts the decision is read from, so a maintainer can
//! run it over the reference models.
//!
//! Input: one .rvt, and optionally a text file of the ElementIds Revit's
//! export writes as `Tag`, one per line. From an IFC4 export,
//! `rg -o -r '$1' ",'([0-9]+)',(\.|[0-9]|\))" revit.ifc` prints them: a Tag is
//! the quoted ElementId before the entity's predefined type, its overall size,
//! or the closing bracket. A space's Tag is its name, so a space named `001`
//! adds id 1 to the set and no space is paired by its Tag (RE-261 pairs those
//! through their GlobalId instead); a numeric string that is not an ElementId
//! is dropped when it does not parse as a `u32`.
//!
//! Output, one line per bit and category:
//!   bit <bit> <category> <set> <clear> <in_set> <in_clear> <out_set> <out_clear>
//! `in_` counts the records whose ElementId the export tags, `out_` the rest;
//! the four are `-` when no tag file is given. And one line per distinct word:
//!   word <hex> <category> <count>
//!
//! One record per ElementId: the first frame in stream order, as RE-261 counts
//! them, so a second-prologue frame of an id already seen is not counted twice.
//!
//! ```text
//! cargo run --profile ci -p rvt-probes --example probe_223_flag_word_census -- model.rvt [tags.txt]
//! ```
//!
//! The model is the first argument or `RVT_MODEL`, the tag file the second or
//! `RVT_IFC_TAGS`.

use std::collections::{BTreeMap, BTreeSet};

use rvt::RevitFile;
use rvt::research::partition_element_records as per;

/// The flag word's offset in the record prologue (RE-128).
const FLAG_OFFSET: usize = 0x46;
/// Every bit of the word is counted, set or clear.
const BITS: usize = 32;

#[derive(Default)]
struct BitCounts {
    set: usize,
    clear: usize,
    in_set: usize,
    in_clear: usize,
    out_set: usize,
    out_clear: usize,
}

struct CategoryCensus {
    words: BTreeMap<u32, usize>,
    bits: [BitCounts; BITS],
}

impl CategoryCensus {
    fn new() -> Self {
        Self {
            words: BTreeMap::new(),
            bits: std::array::from_fn(|_| BitCounts::default()),
        }
    }
}

fn u32_at(buf: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(buf.get(at..at + 4)?.try_into().ok()?))
}

fn read_tags(path: &str) -> anyhow::Result<BTreeSet<u32>> {
    let text = std::fs::read_to_string(path)?;
    let mut tags = BTreeSet::new();
    let mut skipped = 0usize;
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        match line.parse::<u32>() {
            Ok(id) => {
                tags.insert(id);
            }
            Err(_) => skipped += 1,
        }
    }
    if skipped > 0 {
        eprintln!("{skipped} lines of {path} are not an ElementId and were skipped");
    }
    if tags.is_empty() {
        anyhow::bail!("{path} holds no ElementId: one per line, as the export's Tag");
    }
    Ok(tags)
}

fn main() -> anyhow::Result<()> {
    let mut args = std::env::args().skip(1);
    let Some(model) = args.next().or_else(|| std::env::var("RVT_MODEL").ok()) else {
        anyhow::bail!("usage: probe_223_flag_word_census MODEL.rvt [TAGS.txt]");
    };
    let tags_path = args.next().or_else(|| std::env::var("RVT_IFC_TAGS").ok());
    let tags = match &tags_path {
        Some(path) => Some(read_tags(path)?),
        None => None,
    };

    let mut rf = RevitFile::open(&model)?;
    let version = rf.basic_file_info()?.version;
    let declared = rvt::elem_table::declared_ids(&rvt::elem_table::parse_records(&mut rf)?);
    let Some(marker) = per::file_bbox_marker(&mut rf, version) else {
        anyhow::bail!("no bbox marker for Revit {version}");
    };
    let assigned_all = rf.second_prologue_ids();
    let none = BTreeMap::new();
    let mut seen = BTreeSet::new();
    let mut exported = 0usize;
    let mut census: BTreeMap<String, CategoryCensus> = BTreeMap::new();
    for stream in rf.partition_stream_names() {
        let inflated = rf.inflated_partition(&stream)?;
        let buf = inflated.bytes();
        let assigned = assigned_all.get(&stream).unwrap_or(&none);
        for hit in memchr::memmem::find_iter(buf, &marker) {
            let Some(offset) = hit.checked_sub(per::BBOX_MARKER_OFFSET) else {
                continue;
            };
            let record = match per::decode_at_with_marker(&stream, buf, offset, &declared, &marker)
            {
                Some(record) => record,
                None => match assigned
                    .get(&offset)
                    .and_then(|&id| per::decode_frame_as(&stream, buf, offset, id, &marker))
                {
                    Some(record) => record,
                    None => continue,
                },
            };
            if !seen.insert(record.element_id) {
                continue;
            }
            let Some(word) = u32_at(buf, offset + FLAG_OFFSET) else {
                continue;
            };
            let category = match per::builtin_category_name(record.builtin_category) {
                Some(name) => name.to_string(),
                None => record.builtin_category.to_string(),
            };
            let in_export = tags.as_ref().map(|tags| tags.contains(&record.element_id));
            if in_export == Some(true) {
                exported += 1;
            }
            let entry = census.entry(category).or_insert_with(CategoryCensus::new);
            *entry.words.entry(word).or_default() += 1;
            for (bit, counts) in entry.bits.iter_mut().enumerate() {
                let set = word & (1u32 << bit) != 0;
                if set {
                    counts.set += 1;
                } else {
                    counts.clear += 1;
                }
                match in_export {
                    Some(true) if set => counts.in_set += 1,
                    Some(true) => counts.in_clear += 1,
                    Some(false) if set => counts.out_set += 1,
                    Some(false) => counts.out_clear += 1,
                    None => {}
                }
            }
        }
    }
    for (category, entry) in &census {
        for (bit, counts) in entry.bits.iter().enumerate() {
            let export = if tags.is_some() {
                format!(
                    "{} {} {} {}",
                    counts.in_set, counts.in_clear, counts.out_set, counts.out_clear
                )
            } else {
                "- - - -".to_string()
            };
            println!(
                "bit {bit} {category} {} {} {export}",
                counts.set, counts.clear
            );
        }
        for (word, count) in &entry.words {
            println!("word {word:#010x} {category} {count}");
        }
    }
    eprintln!(
        "{} records in {} categories, {exported} of them in the export, {} tags read",
        seen.len(),
        census.len(),
        tags.as_ref().map_or(0, |tags| tags.len())
    );
    Ok(())
}
