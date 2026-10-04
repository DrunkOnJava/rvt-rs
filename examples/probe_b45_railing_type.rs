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

fn main() -> anyhow::Result<()> {
    let path = PathBuf::from(std::env::args().nth(1).expect("model path"));
    if !path.to_string_lossy().contains("Architecture") {
        println!("not RE1 Architecture");
        return Ok(());
    }
    let mut rf = RevitFile::open(&path)?;
    let classes = rf.schema_classes()?;
    let class_name = |class: u32| {
        classes
            .classes
            .iter()
            .find(|c| u32::from(c.tag) == class & 0xffff)
            .map_or_else(|| format!("{class:#x}"), |c| c.name.clone())
    };
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
                "{whose} object in {stream} at {p}: class {}, {} bytes, the other's id at {at:?}",
                class_name(object.class),
                bytes.len()
            );
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
