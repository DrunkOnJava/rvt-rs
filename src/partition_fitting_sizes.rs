//! A pipe fitting's nominal size, from the parameter values its own data
//! object holds (RE-165, B44, #35).
//!
//! A fitting's own data object (RE-153) holds a list of its family's
//! parameter values. Each value after the first is an entry of 38 bytes:
//!
//! ```text
//! ff × 8     a separator
//! u32        the parameter's key, which differs from family to family
//! 00 × 18
//! f64        the value, feet
//! ```
//!
//! A fitting's nominal diameter is an entry whose value is twice the value
//! just before it, its nominal radius. On RE1 Plumbing (Revit 2025, MIT) that
//! pair is the `Pset_PipeFittingTypeCommon.NominalDiameter` Revit's own
//! export writes for all 42 fittings: elbows, tees, a cap and a vent tee, 10
//! to 50 mm, the diameter 654 bytes into the object on 40 of them, 610 on the
//! cap and 838 on the vent tee. Most fittings also hold one record per
//! connector with its radius and diameter, but the cap holds none. A fitting
//! whose pairs disagree has no single nominal size and is given none.

use crate::partition_room_parameters::{DATA_OBJECT_HEADER, verified_data_object};
use crate::{Result, RevitFile};
use std::collections::{BTreeMap, BTreeSet};

/// Bytes of an entry before its value: the separator, the key and the zeros.
const ENTRY_HEAD: usize = 30;
/// Bytes from one value to the next.
const ENTRY_STRIDE: usize = ENTRY_HEAD + 8;
/// Smallest and largest nominal diameter read, feet.
const DIAMETER_FEET: std::ops::Range<f64> = 0.001..20.0;

/// Data objects are read on Revit 2024 and later (RE-153).
pub fn supports_revit_version(revit_version: u32) -> bool {
    revit_version >= 2024
}

fn f64_at(buf: &[u8], at: usize) -> Option<f64> {
    Some(f64::from_le_bytes(
        buf.get(at..at.checked_add(8)?)?.try_into().ok()?,
    ))
}

/// The diameter of the entry whose value starts at `at` in `payload`, when
/// it is a parameter entry whose value is twice the value before it.
fn pair_diameter(payload: &[u8], at: usize) -> Option<f64> {
    let head = payload.get(at.checked_sub(ENTRY_HEAD)?..at)?;
    let (separator, rest) = head.split_at(8);
    let (key, zeros) = rest.split_at(4);
    if separator.iter().any(|&b| b != 0xff)
        || key.iter().all(|&b| b == 0)
        || zeros.iter().any(|&b| b != 0)
    {
        return None;
    }
    let diameter = f64_at(payload, at)?;
    let radius = f64_at(payload, at.checked_sub(ENTRY_STRIDE)?)?;
    (diameter.is_normal() && DIAMETER_FEET.contains(&diameter) && diameter == 2.0 * radius)
        .then_some(diameter)
}

/// The diameter of each radius and diameter pair in `payload`, in order.
pub fn nominal_diameters(payload: &[u8]) -> Vec<f64> {
    (ENTRY_STRIDE..payload.len())
        .filter_map(|at| pair_diameter(payload, at))
        .collect()
}

/// The nominal diameter, feet, of each of `fittings` whose own data object
/// holds radius and diameter pairs that all give one diameter.
pub fn scan_fitting_nominal_diameters(
    rf: &mut RevitFile,
    revit_version: u32,
    fittings: &BTreeSet<u32>,
) -> Result<BTreeMap<u32, f64>> {
    if !supports_revit_version(revit_version) || fittings.is_empty() {
        return Ok(BTreeMap::new());
    }
    let mut found: BTreeMap<u32, Option<f64>> = BTreeMap::new();
    for stream in rf.partition_stream_names() {
        let Ok(inflated) = rf.inflated_partition(&stream) else {
            continue;
        };
        let buf = inflated.bytes();
        for &id in fittings {
            for start in memchr::memmem::find_iter(buf, &id.to_le_bytes()) {
                let Some(object) = verified_data_object(buf, start) else {
                    continue;
                };
                if object.element_id != id {
                    continue;
                }
                let diameters = nominal_diameters(&buf[start + DATA_OBJECT_HEADER..object.end]);
                let Some(&first) = diameters.first() else {
                    continue;
                };
                let size = diameters.iter().all(|&d| d == first).then_some(first);
                match found.get(&id) {
                    None => {
                        found.insert(id, size);
                    }
                    Some(held) if *held != size => {
                        found.insert(id, None);
                    }
                    Some(_) => {}
                }
            }
        }
    }
    Ok(found
        .into_iter()
        .filter_map(|(id, size)| Some((id, size?)))
        .collect())
}
