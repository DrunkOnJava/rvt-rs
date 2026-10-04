//! B54 (probe): where a family instance keeps a connector nothing is joined
//! to (#528).
//!
//! Revit's export writes a port for every connector of RE1 Electrical's
//! lights, devices and panels, nothing connected, and for the open connectors
//! of Plumbing's fixtures. For each element Revit writes ports for (its
//! `Port_<ElementId>_<index>` names), this prints the classes and sizes of the
//! element's own data objects (RE-153), and each place in them where a unit
//! direction (`f64` × 3) is followed by an origin (`f64` × 3) within 10 ft of
//! the element's record location, with the eight `f64`s after it, beside the
//! connector indices Revit's ports name.
//!
//! Usage: probe_b54_connectors <RE1-*.rvt>

use rvt::RevitFile;
use rvt::partition_room_parameters::data_objects;
use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

const ELEMENTS: usize = 10;

fn f64_at(buf: &[u8], at: usize) -> Option<f64> {
    Some(f64::from_le_bytes(buf.get(at..at + 8)?.try_into().ok()?))
}

fn main() -> anyhow::Result<()> {
    let path = PathBuf::from(std::env::args().nth(1).expect("model path"));
    let name = path.to_string_lossy().to_string();
    if !(name.contains("Electrical") || name.contains("Plumbing")) {
        println!("not RE1 Electrical or Plumbing");
        return Ok(());
    }
    let step = std::fs::read_to_string(path.with_extension("ifc"))?;
    // Each element's port indices, from Revit's port names.
    let mut ports: BTreeMap<u32, BTreeSet<u32>> = BTreeMap::new();
    for line in step.lines() {
        if !line.contains("IFCDISTRIBUTIONPORT(") {
            continue;
        }
        let Some(port) = line.split('\'').nth(3) else {
            continue;
        };
        let rest = port
            .trim_start_matches("InPort_")
            .trim_start_matches("OutPort_")
            .trim_start_matches("Port_");
        if let Some((id, index)) = rest.split_once('_') {
            if let (Ok(id), Ok(index)) = (id.parse(), index.parse()) {
                ports.entry(id).or_default().insert(index);
            }
        }
    }
    let mut rf = RevitFile::open(&path)?;
    let classes = rf.schema_classes()?;
    let class_name = |class: u32| {
        classes
            .by_tag((class & 0xffff) as u16)
            .map_or_else(|| format!("{class:#x}"), |c| c.name.clone())
    };
    let mut location: BTreeMap<u32, [f64; 3]> = BTreeMap::new();
    for element in rvt::walker::iter_elements(&mut rf)? {
        let Some(id) = element.id else { continue };
        let field = |wanted: &str| {
            element.fields.iter().find_map(|(name, value)| match value {
                rvt::walker::InstanceField::Float { value, .. } if name == wanted => Some(*value),
                _ => None,
            })
        };
        if let (Some(x), Some(y), Some(z)) = (
            field("m_locationX"),
            field("m_locationY"),
            field("m_locationZ"),
        ) {
            location.insert(id, [x, y, z]);
        }
    }
    let wanted: BTreeSet<u32> = ports.keys().take(ELEMENTS).copied().collect();
    let mut objects: BTreeMap<u32, Vec<(String, String, Vec<u8>)>> = BTreeMap::new();
    for stream in rf.partition_stream_names() {
        let Ok(inflated) = rf.inflated_partition(&stream) else {
            continue;
        };
        let buf = inflated.bytes();
        for (p, object) in data_objects(buf) {
            if wanted.contains(&object.element_id) {
                objects.entry(object.element_id).or_default().push((
                    stream.clone(),
                    class_name(object.class),
                    buf[p..object.end].to_vec(),
                ));
            }
        }
    }
    println!(
        "{} elements with Revit ports; showing {}",
        ports.len(),
        wanted.len()
    );
    for id in &wanted {
        let at = location.get(id);
        println!(
            "element {id}: Revit port indices {:?}, record location {at:?}",
            ports[id]
        );
        for (stream, class, bytes) in objects.get(id).into_iter().flatten() {
            println!("  object {class} in {stream}, {} bytes", bytes.len());
            for q in 0..bytes.len().saturating_sub(48) {
                let Some(d) = (0..3)
                    .map(|k| f64_at(bytes, q + 8 * k))
                    .collect::<Option<Vec<f64>>>()
                else {
                    continue;
                };
                let norm = d.iter().map(|v| v * v).sum::<f64>().sqrt();
                if (norm - 1.0).abs() > 1e-9 {
                    continue;
                }
                let Some(o) = (0..3)
                    .map(|k| f64_at(bytes, q + 24 + 8 * k))
                    .collect::<Option<Vec<f64>>>()
                else {
                    continue;
                };
                let near = at.is_some_and(|at| {
                    o.iter()
                        .zip(at)
                        .all(|(a, b)| a.is_finite() && (a - b).abs() < 10.0)
                });
                if !near {
                    continue;
                }
                let after: Vec<String> = (0..8)
                    .filter_map(|k| f64_at(bytes, q + 48 + 8 * k))
                    .map(|v| format!("{v:.4}"))
                    .collect();
                let ints: Vec<u32> = (0..6)
                    .filter_map(|k| {
                        bytes
                            .get(q.checked_sub(24)? + 4 * k..q.checked_sub(24)? + 4 * k + 4)
                            .map(|b| u32::from_le_bytes(b.try_into().expect("4 bytes")))
                    })
                    .collect();
                println!(
                    "    +{q}: direction {d:.3?} origin {o:.3?}; then {after:?}; u32s before {ints:?}"
                );
            }
        }
    }
    Ok(())
}
