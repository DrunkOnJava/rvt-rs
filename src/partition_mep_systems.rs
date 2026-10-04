//! MEP systems and their members (RE-162, B33, #528).
//!
//! An MEP system is an element of its own, a data object (RE-153) of class
//! `RbsHvacSystem`, `RbsPipingSystem` or `RbsElectricalSystem`. An HVAC or
//! piping system's name is the first string of printable ASCII in its
//! payload; an electrical system's is its circuit number
//! (RE-162 §3). Its members are the elements whose own data objects
//! hold the system's ElementId. On the RE1 Mechanical, Plumbing and
//! Electrical models (Revit 2025, MIT), against Revit's own IFC: every one of
//! Revit's 26 `IfcSystem`s is one such object of the same name, and every
//! element Revit groups in it holds its id.

use crate::partition_room_parameters::{DATA_OBJECT_HEADER, verified_data_object};
use crate::{Result, RevitFile};
use std::collections::{BTreeMap, BTreeSet};

/// The schema classes of MEP systems.
pub const SYSTEM_CLASSES: [&str; 3] = ["RbsHvacSystem", "RbsPipingSystem", "RbsElectricalSystem"];
/// Longest system name read, in UTF-16 units.
const MAX_NAME_UNITS: usize = 256;

/// One MEP system.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MepSystem {
    /// Its ElementId.
    pub id: u32,
    /// Its schema class, one of [`SYSTEM_CLASSES`].
    pub class: String,
    /// Its name, where its object holds one.
    pub name: Option<String>,
    /// The elements of `elements` (see [`scan_mep_systems`]) whose own data
    /// objects hold its id.
    pub members: BTreeSet<u32>,
}

/// Data objects are read on Revit 2024 and later (RE-153).
pub fn supports_revit_version(revit_version: u32) -> bool {
    revit_version >= 2024
}

/// The `u32 n · UTF-16 × n` string at `q` of `bytes`, `n` at most
/// [`MAX_NAME_UNITS`].
fn string_at(bytes: &[u8], q: usize) -> Option<Vec<u16>> {
    let n = u32::from_le_bytes(bytes.get(q..q.checked_add(4)?)?.try_into().ok()?) as usize;
    if n > MAX_NAME_UNITS {
        return None;
    }
    Some(
        bytes
            .get(q + 4..q + 4 + 2 * n)?
            .chunks_exact(2)
            .map(|c| u16::from_le_bytes([c[0], c[1]]))
            .collect(),
    )
}

/// The first non-empty `u32 n · UTF-16 × n` string in `payload` of printable
/// ASCII: an HVAC or piping system's name. Shorter strings of other units
/// come first, a `1` followed by half an ElementId reading as one.
fn first_string(payload: &[u8]) -> Option<String> {
    (0..payload.len()).find_map(|q| {
        let units = string_at(payload, q)?;
        (!units.is_empty() && units.iter().all(|u| (0x20..0x7f).contains(u)))
            .then(|| String::from_utf16_lossy(&units))
    })
}

/// The marker an electrical system's circuit number is found from:
/// `0xff` × 8 then `01`.
const CIRCUIT_MARKER: [u8; 9] = [0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0x01];
/// Bytes from the marker's last byte to the count of the entries before the
/// circuit number.
const CIRCUIT_LIST_OFFSET: usize = 20;
/// Bytes of each of those entries.
const CIRCUIT_ENTRY_BYTES: usize = 32;
/// The name Revit gives a circuit with no circuit number.
pub const UNNAMED_CIRCUIT: &str = "<unnamed>";

/// An electrical system's name, its circuit number (RE-162): after the first
/// [`CIRCUIT_MARKER`] in its object, [`CIRCUIT_LIST_OFFSET`] bytes on, a
/// `u32` count of [`CIRCUIT_ENTRY_BYTES`]-byte entries, then the number as `u32
/// n · UTF-16 × n`. An empty number is Revit's [`UNNAMED_CIRCUIT`].
fn circuit_number(object: &[u8]) -> Option<String> {
    let marker = memchr::memmem::find(object, &CIRCUIT_MARKER)? + CIRCUIT_MARKER.len() - 1;
    let list = marker + CIRCUIT_LIST_OFFSET;
    let count = u32::from_le_bytes(object.get(list..list + 4)?.try_into().ok()?) as usize;
    let units = string_at(object, list + 4 + count.checked_mul(CIRCUIT_ENTRY_BYTES)?)?;
    if units.is_empty() {
        return Some(UNNAMED_CIRCUIT.into());
    }
    units
        .iter()
        .all(|u| (0x20..0x7f).contains(u))
        .then(|| String::from_utf16_lossy(&units))
}

/// Every MEP system in the file, with its members among `elements`. A system
/// whose objects in two streams disagree on its name keeps no name. Empty for
/// a release before 2024 or a schema with none of [`SYSTEM_CLASSES`].
pub fn scan_mep_systems(
    rf: &mut RevitFile,
    revit_version: u32,
    elements: &BTreeSet<u32>,
) -> Result<Vec<MepSystem>> {
    if !supports_revit_version(revit_version) {
        return Ok(Vec::new());
    }
    let classes = rf.schema_classes()?;
    let tags: BTreeMap<u32, String> = classes
        .classes
        .iter()
        .filter(|class| SYSTEM_CLASSES.contains(&class.name.as_str()))
        .map(|class| (u32::from(class.tag), class.name.clone()))
        .collect();
    if tags.is_empty() {
        return Ok(Vec::new());
    }
    let mut systems: BTreeMap<u32, MepSystem> = BTreeMap::new();
    let mut names: BTreeMap<u32, Option<String>> = BTreeMap::new();
    let streams = rf.partition_stream_names();
    for stream in &streams {
        let Ok(inflated) = rf.inflated_partition(stream) else {
            continue;
        };
        let buf = inflated.bytes();
        for (&tag, class) in &tags {
            for hit in memchr::memmem::find_iter(buf, &tag.to_le_bytes()) {
                let Some(p) = hit.checked_sub(16) else {
                    continue;
                };
                let Some(object) = verified_data_object(buf, p) else {
                    continue;
                };
                if object.class & 0xffff != tag {
                    continue;
                }
                let name = if class == "RbsElectricalSystem" {
                    circuit_number(&buf[p..object.end])
                } else {
                    first_string(&buf[p + DATA_OBJECT_HEADER..object.end - 4])
                };
                match names.get_mut(&object.element_id) {
                    None => {
                        names.insert(object.element_id, name);
                    }
                    Some(held) => {
                        if *held != name {
                            *held = None;
                        }
                    }
                }
                systems
                    .entry(object.element_id)
                    .or_insert_with(|| MepSystem {
                        id: object.element_id,
                        class: class.clone(),
                        name: None,
                        members: BTreeSet::new(),
                    });
            }
        }
    }
    // B70: one pass per stream over the objects of `elements`, looking in each
    // payload for a system's id, instead of a backward search for the object
    // around every place a system's id occurs.
    let (Some(&low), Some(&high)) = (systems.keys().next(), systems.keys().next_back()) else {
        return Ok(Vec::new());
    };
    for stream in &streams {
        let Ok(inflated) = rf.inflated_partition(stream) else {
            continue;
        };
        let buf = inflated.bytes();
        let Ok(objects) = rf.partition_data_objects(stream) else {
            continue;
        };
        for &(p, object) in objects.iter() {
            if !elements.contains(&object.element_id) {
                continue;
            }
            for word in buf[p + DATA_OBJECT_HEADER..object.end].windows(4) {
                let id = u32::from_le_bytes(word.try_into().expect("4 bytes"));
                if !(low..=high).contains(&id) || id == object.element_id {
                    continue;
                }
                if let Some(system) = systems.get_mut(&id) {
                    system.members.insert(object.element_id);
                }
            }
        }
    }
    for system in systems.values_mut() {
        system.name = names.remove(&system.id).flatten();
    }
    Ok(systems.into_values().collect())
}
