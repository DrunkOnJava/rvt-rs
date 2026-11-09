//! #1493: the Levels a curtain mullion's partition record names, on the 2025 and
//! 2027 saves of one project.
//!
//! 21 of `rac_basic`'s 144 members have another `LevelElementId` on the 2027 save
//! than on the 2025 save (311 against 515270 on 11, and against 245423 on 10).
//! The id is the one Level the record's counted reference list names
//! (`element_record_level_refs`, RE-27). This prints the file's Levels, then for
//! each mullion record its reference list's length, the Levels it names and its
//! first references, so the two saves' lists can be set side by side.
//!
//! ```bash
//! cargo run --profile ci -p rvt-probes --example probe_1493_mullion_level_refs -- \
//!     samples/2027-racbasicsampleproject.rvt
//! ```

use std::collections::{BTreeMap, BTreeSet};

use rvt::RevitFile;
use rvt::elem_table;
use rvt::research::partition_element_records as per;
use rvt::research::partition_level_records;
use serde_json::json;

fn main() -> rvt::Result<()> {
    let path = std::env::args()
        .skip(1)
        .find(|arg| !arg.starts_with("--"))
        .expect("usage: probe_1493_mullion_level_refs <file.rvt>");
    let mut rf = RevitFile::open(&path)?;
    let version = rf.basic_file_info()?.version;
    let declared: BTreeSet<u32> = elem_table::parse_records(&mut rf)?
        .into_iter()
        .map(|r| r.id_primary)
        .collect();
    let levels = partition_level_records::scan_partition_levels(&mut rf, version, &declared)?;
    let by_id: BTreeMap<u32, (&str, f64)> = levels
        .iter()
        .map(|l| (l.element_id, (l.name.as_str(), l.elevation_feet)))
        .collect();
    println!(
        "{}",
        json!({
            "release": version,
            "levels": levels
                .iter()
                .map(|l| json!({"id": l.element_id, "name": l.name, "feet": l.elevation_feet}))
                .collect::<Vec<_>>(),
        })
    );
    let records =
        per::scan_category_records(&mut rf, version, per::OST_CURTAIN_WALL_MULLIONS, &declared)?;
    let (mut none, mut one, mut many) = (0usize, 0usize, 0usize);
    for record in records.iter().filter(|r| r.is_placed_instance()) {
        let named: Vec<u32> = record
            .references
            .iter()
            .filter_map(|v| u32::try_from(*v).ok())
            .filter(|v| by_id.contains_key(v))
            .collect();
        match named.len() {
            0 => none += 1,
            1 => one += 1,
            _ => many += 1,
        }
        println!(
            "{}",
            json!({
                "id": record.element_id,
                "references": record.references.len(),
                "levels_named": named,
                "first": record.references.iter().take(14).collect::<Vec<_>>(),
            })
        );
    }
    println!(
        "{}",
        json!({"mullion_records": none + one + many, "no_level": none, "one_level": one, "several": many})
    );
    Ok(())
}
