//! B45 (probe): where RE1 Architecture's railing 462556 holds its type,
//! 446543 (the `Tag` of Revit's `IfcRailingType`). Prints the railing's
//! decoded fields, and every data object (RE-153) of the railing and of the
//! type with its class, size and the offsets at which the other's id occurs.
//!
//! Usage: probe_b45_railing_type <RE1-Architecture.rvt>

use rvt::RevitFile;
use rvt::partition_room_parameters::data_objects;
use std::path::PathBuf;

const RAILING: u32 = 462556;
const TYPE: u32 = 446543;

fn strings(bytes: &[u8]) -> Vec<String> {
    let mut out = Vec::new();
    let mut q = 0;
    while q + 4 <= bytes.len() {
        let n = u32::from_le_bytes(bytes[q..q + 4].try_into().expect("4 bytes")) as usize;
        if (1..=80).contains(&n) && q + 4 + 2 * n <= bytes.len() {
            let units: Vec<u16> = bytes[q + 4..q + 4 + 2 * n]
                .chunks_exact(2)
                .map(|c| u16::from_le_bytes([c[0], c[1]]))
                .collect();
            if units.iter().all(|u| (0x20..0x7f).contains(u)) {
                out.push(String::from_utf16_lossy(&units));
                q += 4 + 2 * n;
                continue;
            }
        }
        q += 1;
    }
    out
}

/// Each `u32 n · UTF-16 × n` string of printable units, with its offset.
fn strings_at(bytes: &[u8]) -> Vec<(usize, String)> {
    let mut out = Vec::new();
    let mut q = 0;
    while q + 4 <= bytes.len() {
        let n = u32::from_le_bytes(bytes[q..q + 4].try_into().expect("4 bytes")) as usize;
        if (1..=80).contains(&n) && q + 4 + 2 * n <= bytes.len() {
            let units: Vec<u16> = bytes[q + 4..q + 4 + 2 * n]
                .chunks_exact(2)
                .map(|c| u16::from_le_bytes([c[0], c[1]]))
                .collect();
            if units.iter().all(|u| (0x20..0x7f).contains(u)) {
                out.push((q, String::from_utf16_lossy(&units)));
                q += 4 + 2 * n;
                continue;
            }
        }
        q += 1;
    }
    out
}

fn main() -> anyhow::Result<()> {
    let path = PathBuf::from(std::env::args().nth(1).expect("model path"));
    if !path.to_string_lossy().contains("Architecture") {
        println!("not RE1 Architecture");
        return Ok(());
    }
    let reference = path.with_extension("ifc");
    if let Ok(step) = std::fs::read_to_string(&reference) {
        for line in step.lines() {
            if line.contains("IFCRAILINGTYPE(")
                || line.contains("'462556'")
                || line.contains("Pset_RailingCommon")
            {
                println!("revit: {}", line.chars().take(300).collect::<String>());
            }
        }
        // The values of Pset_RailingCommon's properties.
        let ids: Vec<&str> = step
            .lines()
            .filter(|l| l.contains("'Pset_RailingCommon'"))
            .flat_map(|l| {
                l.split('#')
                    .skip(2)
                    .map(|s| s.split(|c: char| !c.is_ascii_digit()).next().unwrap_or(""))
            })
            .collect();
        for line in step.lines() {
            if ids
                .iter()
                .any(|id| !id.is_empty() && line.starts_with(&format!("#{id}=")))
            {
                println!("revit property: {line}");
            }
        }
    }
    let mut rf = RevitFile::open(&path)?;
    let names = rf.element_names();
    println!(
        "name entry of {TYPE}: {:?}",
        names.entries.get(&TYPE).map(|e| e.name.clone())
    );
    let version = rf.basic_file_info()?.version;
    let wanted: std::collections::BTreeSet<u32> = [TYPE].into_iter().collect();
    if let Some(header) = rvt::partition_names::element_data_header(version) {
        for stream in rf.partition_stream_names() {
            let Ok(inflated) = rf.inflated_partition(&stream) else {
                continue;
            };
            let data =
                rvt::partition_names::find_element_data_names(inflated.bytes(), &header, &wanted);
            let railing = rvt::partition_names::find_railing_type_names(inflated.bytes(), &wanted);
            if !data.is_empty() || !railing.is_empty() {
                println!("{stream}: element data names {data:?}, railing type names {railing:?}");
            }
        }
    }
    let classes = rf.schema_classes()?;
    let class_name = |class: u32| {
        classes
            .classes
            .iter()
            .find(|c| u32::from(c.tag) == class & 0xffff)
            .map_or_else(|| format!("{class:#x}"), |c| c.name.clone())
    };
    if let Ok(result) = rvt::ifc::RvtDocExporter.export_with_diagnostics(&mut rf) {
        for line in rvt::ifc::write_step(&result.model).lines() {
            if line.contains("462556") || line.contains("446543") {
                println!("export: {}", line.chars().take(300).collect::<String>());
            }
        }
    }
    for element in rvt::walker::iter_elements(&mut rf)? {
        if element.id == Some(RAILING) {
            println!("railing class {}", element.class);
            for (name, value) in &element.fields {
                println!("  {name}: {value:?}");
            }
        }
    }
    for stream in rf.partition_stream_names() {
        let Ok(inflated) = rf.inflated_partition(&stream) else {
            continue;
        };
        let buf = inflated.bytes();
        for (p, object) in data_objects(buf) {
            let (whose, other) = match object.element_id {
                RAILING => ("railing", TYPE),
                TYPE => ("type", RAILING),
                _ => continue,
            };
            let bytes = &buf[p..object.end];
            let at: Vec<usize> = memchr::memmem::find_iter(bytes, &other.to_le_bytes()).collect();
            println!(
                "{whose} object in {stream} at {p}: class {}, {} bytes, the other's id at {at:?}, strings {:?}",
                class_name(object.class),
                bytes.len(),
                strings_at(bytes)
            );
            if whose == "type" {
                let marks: Vec<usize> =
                    memchr::memmem::find_iter(bytes, &rvt::partition_names::RAILING_TYPE_NAME_END)
                        .collect();
                println!(
                    "  marker at {marks:?}; railing_type_name_in {:?}",
                    rvt::partition_names::railing_type_name_in(bytes)
                );
                // The classes whose tags follow an `ffffffff` sentinel.
                let mut tags: Vec<u32> = Vec::new();
                for at in memchr::memmem::find_iter(bytes, &[0xff, 0xff, 0xff, 0xff]) {
                    if let Some(tag) = bytes.get(at + 4..at + 6) {
                        let tag = u32::from(u16::from_le_bytes([tag[0], tag[1]]));
                        if tag != 0xffff && !tags.contains(&tag) {
                            tags.push(tag);
                        }
                    }
                }
                for tag in tags.iter().take(24) {
                    println!("  tag {tag:#06x} after a sentinel: {}", class_name(*tag));
                }
                for tag in [0x020b_u32, 0x0220, 0x0fdb, 0x103c] {
                    println!("  tag {tag:#06x}: {}", class_name(tag));
                }
                for row in (520..bytes.len().min(640)).step_by(16) {
                    let end = (row + 16).min(bytes.len());
                    let hex: Vec<String> =
                        bytes[row..end].iter().map(|b| format!("{b:02x}")).collect();
                    println!("  +{row}: {}", hex.join(" "));
                }
            }
        }
        // Occurrences of the type id anywhere in the stream near the railing id.
        for hit in memchr::memmem::find_iter(buf, &RAILING.to_le_bytes()) {
            let window = &buf[hit.saturating_sub(64)..(hit + 512).min(buf.len())];
            if memchr::memmem::find(window, &TYPE.to_le_bytes()).is_some() {
                println!(
                    "{stream}: railing id at {hit} with the type id within the next 512 bytes"
                );
            }
        }
    }
    Ok(())
}
