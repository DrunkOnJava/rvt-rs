//! #878: which element holds each IfcGUID string a Revit 2015 file stores
//! (RE-208 §3), so the storeys', project's, site's and building's GlobalIds,
//! which Revit's export writes verbatim from them, can be read.
//!
//! FACT under test: each stored string sits in the data of the element it
//! names, after that element's record frame and before the next one, so
//! the nearest frame before it gives its owner: a Level for each storey's
//! GlobalId, and the project information for the project's, site's and
//! building's.
//!
//! Output:
//!   level <id> <elevation ft> <name>
//!   frame <stream> <offset> <id> <BuiltInCategory>
//!     every element record frame: on Revit 2015 to 2023 every one
//!     `partition_element_records_2023::scan_records` reads, on later
//!     releases every first-prologue frame (its ElementId at +0x00, declared
//!     in ElemTable) of the categories below, found by its category at +0x12
//!   gid <stream> <offset> <i32 before the length> <u32 length> <string>
//!     every UTF-16 string of 22 GlobalId characters
//!   near <stream> <offset> <distance>=<id> ...
//!     the declared ElementIds (u32, any alignment) in the 4096 bytes
//!     before each such string, nearest first, up to 12
//!
//! `tools/re/stored_guids_vs_ifc.py` ties Revit's GlobalIds to them.
//!
//! Usage:
//!   cargo run --profile ci --example probe_878_stored_guids -- FILE.rvt

use rvt::RevitFile;
use rvt::research::partition_element_records as per;
use rvt::research::partition_level_records::{OST_LEVELS, recover_partition_levels};

const GID_ALPHABET: &[u8] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz_$";
const OST_PROJECT_INFORMATION: i64 = -2_003_101;

fn i32_at(buf: &[u8], at: usize) -> Option<i32> {
    Some(i32::from_le_bytes(
        buf.get(at..at.checked_add(4)?)?.try_into().ok()?,
    ))
}

fn u32_at(buf: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(
        buf.get(at..at.checked_add(4)?)?.try_into().ok()?,
    ))
}

fn main() -> rvt::Result<()> {
    let path = std::env::args().nth(1).expect("usage: FILE.rvt");
    let mut rf = RevitFile::open(&path)?;
    let version = rf.basic_file_info()?.version;
    for level in recover_partition_levels(&mut rf, version).unwrap_or_default() {
        println!(
            "level {} {:.6} {}",
            level.element_id, level.elevation_feet, level.name
        );
    }
    let declared = rvt::elem_table::declared_ids(&rvt::elem_table::parse_records(&mut rf)?);
    let categories = [
        OST_LEVELS,
        OST_PROJECT_INFORMATION,
        per::OST_WALLS,
        per::OST_FLOORS,
        per::OST_ROOFS,
        per::OST_COLUMNS,
        per::OST_STRUCTURAL_COLUMNS,
        per::OST_STRUCTURAL_FRAMING,
        per::OST_STRUCTURAL_FOUNDATION,
        per::OST_DOORS,
        per::OST_WINDOWS,
    ];
    let thirty_two = rvt::research::partition_element_records_2023::is_32bit_release(version);
    if thirty_two {
        for record in rvt::research::partition_element_records_2023::scan_records(&mut rf, version)
        {
            println!(
                "frame {} {} {} {}",
                record.stream, record.offset, record.element_id, record.builtin_category
            );
        }
    }
    let names: Vec<String> = rf
        .stream_names()
        .into_iter()
        .filter(|n| n.starts_with("Partitions/"))
        .collect();
    for name in names {
        let Ok(inflated) = rf.inflated_partition(&name) else {
            continue;
        };
        let data = inflated.bytes();
        for category in categories.iter().copied().filter(|_| !thirty_two) {
            let needle = (category as u64).to_le_bytes();
            for hit in memchr::memmem::find_iter(data, &needle) {
                let Some(start) = hit.checked_sub(per::CATEGORY_OFFSET) else {
                    continue;
                };
                if let Some(id) = u32_at(data, start).filter(|id| declared.contains(id)) {
                    println!("frame {name} {start} {id} {category}");
                }
            }
        }
        let is_gid = |b: u8| GID_ALPHABET.contains(&b);
        for parity in 0..2 {
            let mut i = parity;
            while i + 1 < data.len() {
                if !(is_gid(data[i]) && data[i + 1] == 0) {
                    i += 2;
                    continue;
                }
                let start = i;
                while i + 1 < data.len() && is_gid(data[i]) && data[i + 1] == 0 {
                    i += 2;
                }
                if (i - start) / 2 == 22 {
                    let text: String = data[start..i]
                        .iter()
                        .step_by(2)
                        .map(|&b| char::from(b))
                        .collect();
                    let param = start.checked_sub(8).and_then(|at| i32_at(data, at));
                    let length = start.checked_sub(4).and_then(|at| u32_at(data, at));
                    println!(
                        "gid {name} {start} {} {} {text}",
                        param.map_or("-".into(), |p| p.to_string()),
                        length.map_or("-".into(), |l| l.to_string())
                    );
                    let near: Vec<String> = (4..=4096usize)
                        .filter_map(|distance| {
                            let at = start.checked_sub(distance)?;
                            let id = u32_at(data, at)?;
                            (id > 1000 && declared.contains(&id))
                                .then(|| format!("{distance}={id}"))
                        })
                        .take(12)
                        .collect();
                    println!("near {name} {start} {}", near.join(" "));
                }
            }
        }
    }
    Ok(())
}
