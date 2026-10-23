//! RE-209 (#879): what a Revit 2015 footing and air terminal record say
//! about their Level, and what sets the three air terminals Revit's export
//! leaves out apart.
//!
//! FACT under test: on BIMcollab's Revit 2015 structural example rvt-rs
//! writes 16 `IfcFooting` on no storey, where Revit's export has each in a
//! storey, and 3 `IfcAirTerminal` (124771, 148100, 221553) that Revit's
//! export leaves out. This probe prints every decoded field of those
//! elements, of the file's Levels, and of the first structural columns for
//! contrast; `tools/re/footing_levels_vs_ifc.py` joins them by Tag with the
//! storeys of Revit's export.
//!
//! Output:
//!   el <class> <id> <field>=<value>; ...      one element per line
//!   level <id> <elevation ft> <name>          the Levels rvt-rs recovers
//!   ref <id> <stream> <offset> <text> <hex>   where each id under 100000 that
//!     a footing's or air terminal's reference list names appears as a u64,
//!     with the first UTF-16 text in the 160 bytes after it (up to 6 places)
//!   near <class> <id> <level id> <offset from the record> <u32|u64>
//!     each recovered Level's id within 512 bytes before and 2048 after a
//!     footing's or column's record
//!   bytes <class> <id> +300 <hex>   a footing record's bytes +300 to +420
//!
//! Usage:
//!   cargo run --profile ci --example probe_re209_footing_levels_2015 -- FILE.rvt

use rvt::RevitFile;
use rvt::research::partition_level_records::recover_partition_levels;
use rvt::walker::{InstanceField, iter_elements};
use std::collections::BTreeSet;

fn show(value: &InstanceField) -> String {
    let text = match value {
        InstanceField::ElementId { id, .. } => format!("#{id}"),
        InstanceField::Integer { value, .. } => value.to_string(),
        InstanceField::Float { value, .. } => format!("{value:.4}"),
        InstanceField::String(s) => format!("{s:?}"),
        other => format!("{other:?}"),
    };
    text.chars().take(160).collect()
}

/// The first run of at least 3 printable UTF-16LE characters in `bytes`.
fn utf16_text(bytes: &[u8]) -> String {
    let mut best = String::new();
    for parity in 0..2 {
        let mut run = String::new();
        let mut i = parity;
        while i + 1 < bytes.len() {
            let (lo, hi) = (bytes[i], bytes[i + 1]);
            if hi == 0 && (0x20..0x7f).contains(&lo) {
                run.push(char::from(lo));
            } else {
                if run.len() >= 3 && best.is_empty() {
                    best = run.clone();
                }
                run.clear();
            }
            i += 2;
        }
        if run.len() >= 3 && best.is_empty() {
            best = run;
        }
    }
    best.replace(' ', "_")
}

fn main() {
    let path = std::env::args_os()
        .nth(1)
        .expect("usage: probe_re209_footing_levels_2015 FILE.rvt");
    let mut rf = RevitFile::open(&path).expect("open");
    let mut columns = 0;
    let mut targets = BTreeSet::new();
    let mut records: Vec<(String, u32, String, usize)> = Vec::new();
    for element in iter_elements(&mut rf).expect("elements") {
        if matches!(
            element.class.as_str(),
            "StructuralFoundation" | "DuctTerminal"
        ) {
            for (name, value) in &element.fields {
                if name == "m_referenceIds" {
                    for id in format!("{value:?}").split("id: ").skip(1) {
                        if let Ok(id) = id
                            .split(|c: char| !c.is_ascii_digit())
                            .next()
                            .unwrap_or("")
                            .parse::<u64>()
                        {
                            if id < 100_000 {
                                targets.insert(id);
                            }
                        }
                    }
                }
            }
        }
        let wanted = match element.class.as_str() {
            "StructuralFoundation" | "DuctTerminal" | "Level" => true,
            "StructuralColumn" => {
                columns += 1;
                columns <= 6
            }
            _ => false,
        };
        if !wanted {
            continue;
        }
        if matches!(
            element.class.as_str(),
            "StructuralFoundation" | "StructuralColumn"
        ) {
            let stream = element
                .fields
                .iter()
                .find_map(|(n, v)| match (n.as_str(), v) {
                    ("m_source_stream", InstanceField::String(s)) => Some(s.clone()),
                    _ => None,
                });
            let offset = element
                .fields
                .iter()
                .find_map(|(n, v)| match (n.as_str(), v) {
                    ("m_source_offset", InstanceField::Integer { value, .. }) => {
                        usize::try_from(*value).ok()
                    }
                    _ => None,
                });
            if let (Some(stream), Some(offset), Some(id)) = (stream, offset, element.id) {
                records.push((element.class.clone(), id, stream, offset));
            }
        }
        let fields: Vec<String> = element
            .fields
            .iter()
            .map(|(name, value)| format!("{name}={}", show(value)))
            .collect();
        println!(
            "el {} {} {}",
            element.class,
            element.id.map_or("-".into(), |id| id.to_string()),
            fields.join("; ")
        );
    }
    let version = rf.basic_file_info().map(|info| info.version).unwrap_or(0);
    let levels = recover_partition_levels(&mut rf, version).unwrap_or_default();
    for level in &levels {
        println!(
            "level {} {:.4} {}",
            level.element_id, level.elevation_feet, level.name
        );
    }
    for (class, id, stream, offset) in &records {
        let Ok(inflated) = rf.inflated_partition(stream) else {
            continue;
        };
        let data = inflated.bytes();
        if class == "StructuralFoundation" {
            let window = &data[(offset + 300).min(data.len())..(offset + 420).min(data.len())];
            let hex: String = window.iter().map(|b| format!("{b:02x}")).collect();
            println!("bytes {class} {id} +300 {hex}");
        }
        let start = offset.saturating_sub(512);
        let end = (offset + 2048).min(data.len());
        for level in &levels {
            let narrow = level.element_id.to_le_bytes();
            let wide = u64::from(level.element_id).to_le_bytes();
            for at in start..end.saturating_sub(4) {
                if data[at..at + 4] == narrow {
                    let width = if data.get(at..at + 8) == Some(&wide[..]) {
                        "u64"
                    } else {
                        "u32"
                    };
                    println!(
                        "near {class} {id} {} {} {width}",
                        level.element_id,
                        at as i64 - *offset as i64
                    );
                }
            }
        }
    }
    let partitions: Vec<String> = rf
        .stream_names()
        .into_iter()
        .filter(|n| n.starts_with("Partitions/"))
        .collect();
    for id in targets {
        let needle = id.to_le_bytes();
        let mut shown = 0;
        for name in &partitions {
            let Ok(inflated) = rf.inflated_partition(name) else {
                continue;
            };
            let data = inflated.bytes();
            let mut at = 0;
            while shown < 6 {
                let Some(found) = data[at..].windows(8).position(|w| w == needle) else {
                    break;
                };
                let start = at + found;
                let after = &data[start + 8..(start + 168).min(data.len())];
                let hex: String = after.iter().take(40).map(|b| format!("{b:02x}")).collect();
                println!("ref {id} {name} {start} {} {hex}", utf16_text(after));
                shown += 1;
                at = start + 1;
            }
        }
    }
}
