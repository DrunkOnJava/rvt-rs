//! #1489: why two `Single-Flush` doors of `rac_basic`'s 2027 save write no
//! `Qto_DoorBaseQuantities` Width and Height.
//!
//! FACT under test: the quantities are written where the symbol that draws an
//! instance and the instance's type hold the same Width and Height
//! (`type_door_sizes` needs every copy of an entry to agree). RE-318's probe
//! shows doors 431064 and 485679 are drawn by symbol 431054 of type 232780 on
//! the 2025, 2026 and 2027 saves alike, so the symbol is not re-resolved on
//! 2027. This prints every partition's Width and Height entries for those
//! elements, with two other symbols of the type (490150 and 235155), whose
//! doors keep their quantities, as the control.
//!
//! Output, one entry per line:
//!   entry <partition> <owner> width=<feet|-> height=<feet|->
//!   sizes <owner> <width,height|none>   (what `type_door_sizes` returns)
//!
//! Usage:
//!   cargo run --profile ci -p rvt-probes --example probe_1489_door_size_entries -- MODEL.rvt

use std::collections::BTreeSet;

use rvt::RevitFile;
use rvt::elem_table;
use rvt::research::partition_type_parameters::{
    VALUE_BLOCK_MARK, WINDOW_HEIGHT_PARAMETER, WINDOW_WIDTH_PARAMETER, scan_partition_lengths,
    type_door_sizes,
};

const OWNERS: [u32; 4] = [431054, 490150, 235155, 232780];

fn main() -> anyhow::Result<()> {
    let path = std::env::args().nth(1).expect("usage: MODEL.rvt");
    let mut rf = RevitFile::open(&path)?;
    let records = elem_table::parse_records(&mut rf)?;
    let declared = elem_table::declared_ids(&records);
    let parameters = BTreeSet::from([WINDOW_WIDTH_PARAMETER, WINDOW_HEIGHT_PARAMETER]);
    let owners: BTreeSet<u32> = OWNERS.into_iter().collect();
    let version = rf.basic_file_info().map(|i| i.version).unwrap_or(0);
    for stream in rf.partition_stream_names() {
        let Ok(inflated) = rf.inflated_partition(&stream) else {
            println!("unread {stream}");
            continue;
        };
        for (owner, held) in scan_partition_lengths(inflated.bytes(), &declared, &parameters) {
            if !owners.contains(&owner) {
                continue;
            }
            let value = |parameter: i64| {
                held.get(&parameter)
                    .map_or("-".to_string(), |feet| format!("{feet}"))
            };
            println!(
                "entry {stream} {owner} width={} height={}",
                value(WINDOW_WIDTH_PARAMETER),
                value(WINDOW_HEIGHT_PARAMETER)
            );
        }
    }
    for owner in OWNERS {
        println!("declared {owner} {}", declared.contains(&owner));
    }
    // Where each owner's id and the Width parameter sit in each partition,
    // to tell an entry that is missing from one the scan does not attribute.
    let width = WINDOW_WIDTH_PARAMETER.to_le_bytes();
    for stream in rf.partition_stream_names() {
        let Ok(inflated) = rf.inflated_partition(&stream) else {
            continue;
        };
        let bytes = inflated.bytes();
        let widths: Vec<usize> = memchr::memmem::find_iter(bytes, &width).collect();
        for owner in OWNERS {
            for at in memchr::memmem::find_iter(bytes, &owner.to_le_bytes()) {
                let next = widths.iter().find(|&&w| w > at).map(|&w| w - at);
                if next.is_some_and(|gap| gap < 4096) {
                    let gap = next.unwrap_or(0);
                    println!("near {stream} {owner} id@{at} width+{gap}");
                    // The bytes from just before the id to just past the
                    // Width parameter's value, for the closest few, to see
                    // what separates an attributed entry from one that is not.
                    if gap < 400 {
                        let from = at.saturating_sub(16);
                        let to = (at + gap + 24).min(bytes.len());
                        let hex: Vec<String> =
                            bytes[from..to].iter().map(|b| format!("{b:02x}")).collect();
                        println!("bytes {stream} {owner} @{from} {}", hex.join(""));
                    }
                }
            }
        }
    }
    // Every place a value entry holds the type's Width (a f64, eight 0xff and
    // the parameter), and the owner the scan gives that place, if any: an entry
    // that exists but is attributed to another owner, or to none, is the gap.
    let widths: BTreeSet<i64> = BTreeSet::from([WINDOW_WIDTH_PARAMETER]);
    for stream in rf.partition_stream_names() {
        let Ok(inflated) = rf.inflated_partition(&stream) else {
            continue;
        };
        let bytes = inflated.bytes();
        let placed: Vec<(u32, usize)> =
            rvt::research::partition_type_parameters::scan_partition_length_places(
                bytes, &declared, &widths, true,
            )
            .into_iter()
            .flat_map(|(owner, held)| held.into_values().map(move |(_, at)| (owner, at)))
            .collect();
        let mut needle = 2.624_671_916_010_498_8_f64.to_le_bytes().to_vec();
        needle.extend_from_slice(&[0xff; 8]);
        needle.extend_from_slice(&WINDOW_WIDTH_PARAMETER.to_le_bytes());
        for at in memchr::memmem::find_iter(bytes, &needle) {
            let param_at = at + 16;
            let owner = placed
                .iter()
                .find(|(_, place)| place.abs_diff(param_at) <= 16 || place.abs_diff(at) <= 16)
                .map_or("none".to_string(), |(owner, _)| owner.to_string());
            let from = at.saturating_sub(96);
            let hex: Vec<String> = bytes[from..(at + 24).min(bytes.len())]
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect();
            println!("width {stream} @{at} owner={owner} {}", hex.join(""));
            // The block marks before the entry, nearest first, with the u64 in
            // the eight bytes before each and whether it is a declared id: the
            // owner is the nearest mark with a declared id or a zero, and a
            // mark of any other id is passed over.
            let marks: Vec<usize> = memchr::memmem::find_iter(&bytes[..at], &VALUE_BLOCK_MARK)
                .filter(|&mark| mark >= 8)
                .collect();
            for &mark in marks.iter().rev().take(4) {
                let id = u64::from_le_bytes(bytes[mark - 8..mark].try_into().unwrap_or([0; 8]));
                let is_declared = u32::try_from(id).is_ok_and(|id| declared.contains(&id));
                println!(
                    "  mark {stream} @{mark} back={} id={id} declared={is_declared} before=0x{:02x}",
                    at - mark,
                    bytes[mark - 1]
                );
            }
        }
    }
    let sizes = type_door_sizes(&mut rf, version, &owners);
    for owner in OWNERS {
        match sizes.get(&owner) {
            Some(size) => println!("sizes {owner} {},{}", size.width_feet, size.height_feet),
            None => println!("sizes {owner} none"),
        }
    }
    Ok(())
}
