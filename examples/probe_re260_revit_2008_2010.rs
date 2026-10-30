//! RE-260 probe: Revit 2008 to 2010 element records and material names,
//! read in the 32-bit layout of Revit 2011 to 2023 (RE-180, RE-181,
//! RE-259).
//!
//! FACT (to be measured): Revit 2009 and 2010 frame their element records
//! as 2011 does, a declared `u32` ElementId 52 bytes before the record
//! marker. Revit 2008 puts it 48 bytes before: its envelope has 12 bytes
//! between the category and the placement kind where 2009 to 2023 have 16,
//! all `ff`. A Revit 2008 to 2011 material object frames its name as
//! `u32 n · UTF-16 × n` with three equal `f64`s right after it, and no
//! `PhysicalParamSet` tag ends it. A Revit 2008 to 2012 Level's elevation
//! follows `ff ff ff ff` and the `Plane` tag after its name, one reference
//! slot and 16 bytes, and is written twice, 24 bytes apart.
//!
//! For each file it prints, for every record marker of the file's own
//! schema (`Outline`, `ff`×4, `ElementParents`), the distance back to a
//! declared `u32` ElementId followed 8 bytes on by `ElementHeader`'s tag,
//! and the bytes between that record's category and its placement kind.
//! It then prints how many material objects
//! `partition_materials::scan_materials_32` finds and names, the Levels
//! `partition_level_records::recover_partition_levels` gives, and the
//! production elements (`walker::iter_elements`) by class.
//!
//! The 2008 to 2011 files measured are closed source and measured locally
//! only.
//!
//! ```bash
//! cargo run --release --example probe_re260_revit_2008_2010 -- file.rvt [more.rvt ...]
//! ```

use rvt::RevitFile;
use rvt::research::{
    partition_element_records_2023 as per32, partition_level_records, partition_materials,
};
use std::collections::BTreeMap;

/// How far before a marker an ElementId is looked for.
const ID_SEARCH: std::ops::RangeInclusive<usize> = 9..=120;
/// Bytes from the ElementId to the category, on every release measured.
const CATEGORY_AFTER_ID: usize = 14;
/// Bytes from the placement kind to the marker, on every release measured.
const KIND_BEFORE_MARKER: usize = 14;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let paths: Vec<String> = std::env::args().skip(1).collect();
    if paths.is_empty() {
        return Err("usage: probe_re260_revit_2008_2010 <file.rvt> [more.rvt ...]".into());
    }
    for path in &paths {
        let mut rf = RevitFile::open(path)?;
        let version = rf.basic_file_info()?.version;
        println!("file {path}\nrelease {version}");
        envelopes(&mut rf)?;
        let declared = rvt::elem_table::declared_ids(&rvt::elem_table::parse_records(&mut rf)?);
        let (tagged, names) = partition_materials::scan_materials_32(&mut rf, version, &declared);
        println!(
            "materials {} named {} e.g. {:?}",
            tagged.len(),
            names.len(),
            names.values().take(4).collect::<Vec<_>>()
        );
        let levels: Vec<String> =
            partition_level_records::recover_partition_levels(&mut rf, version)?
                .iter()
                .map(|level| {
                    format!(
                        "{} {:?} {} ft",
                        level.element_id, level.name, level.elevation_feet
                    )
                })
                .collect();
        println!("levels {levels:?}");
        let mut classes: BTreeMap<String, usize> = BTreeMap::new();
        for element in rvt::walker::iter_elements(&mut rf)? {
            *classes.entry(element.class).or_default() += 1;
        }
        println!("elements {classes:?}");
    }
    Ok(())
}

/// For each record marker, the distance back to its ElementId and the bytes
/// between its category and its placement kind.
fn envelopes(rf: &mut RevitFile) -> Result<(), Box<dyn std::error::Error>> {
    let Some((marker, header)) = per32::record_marker(rf) else {
        println!("framing: schema gives no record marker");
        return Ok(());
    };
    let declared = rvt::elem_table::declared_ids(&rvt::elem_table::parse_records(rf)?);
    let (mut markers, mut none) = (0usize, 0usize);
    let mut at_distance: BTreeMap<usize, usize> = BTreeMap::new();
    let mut gaps: BTreeMap<String, usize> = BTreeMap::new();
    for stream in rf.partition_stream_names() {
        let inflated = rf.inflated_partition(&stream)?;
        let buf = inflated.bytes();
        let u32_at = |at: usize| {
            buf.get(at..at + 4)
                .map(|b| u32::from_le_bytes(b.try_into().expect("4 bytes")))
        };
        let u16_at = |at: usize| {
            buf.get(at..at + 2)
                .map(|b| u16::from_le_bytes(b.try_into().expect("2 bytes")))
        };
        for hit in memchr::memmem::find_iter(buf, &marker) {
            markers += 1;
            let found = ID_SEARCH.clone().find(|&back| {
                hit.checked_sub(back).is_some_and(|at| {
                    u32_at(at).is_some_and(|id| declared.contains(&id))
                        && u16_at(at + 8) == Some(header)
                })
            });
            let Some(back) = found else {
                none += 1;
                continue;
            };
            *at_distance.entry(back).or_default() += 1;
            let from = hit - back + CATEGORY_AFTER_ID + 8;
            let to = hit - KIND_BEFORE_MARKER;
            let gap = buf
                .get(from..to)
                .map(|bytes| {
                    if bytes.iter().all(|&b| b == 0xff) {
                        format!("{} x ff", bytes.len())
                    } else {
                        format!("{} bytes, not all ff", bytes.len())
                    }
                })
                .unwrap_or_default();
            *gaps.entry(gap).or_default() += 1;
        }
    }
    let distances: Vec<String> = at_distance
        .iter()
        .map(|(back, n)| format!("-{back}={n}"))
        .collect();
    println!(
        "framing markers {markers} id-before-marker {} none {none}",
        distances.join(" ")
    );
    println!("category-to-kind {gaps:?}");
    Ok(())
}
