//! A pipe fitting's nominal size, from the connectors its own data object
//! holds (RE-165, B44, #35).
//!
//! A fitting's own data object (RE-153) holds one record per connector:
//!
//! ```text
//! +0   f64 × 3   the connector's direction, a unit vector
//! +24  f64 × 3   its origin, model feet
//! +48  f64       its radius, feet
//! +56  f64       a further length
//! +64  f64       its diameter, twice the radius
//! ```
//!
//! On RE1 Plumbing (Revit 2025, MIT) the connectors of each of the 42 fittings
//! give the diameter Revit's own export writes as the fitting's
//! `Pset_PipeFittingTypeCommon.NominalDiameter`: elbows, tees, a cap and a
//! vent tee, 10 to 50 mm. A record is recognised by that shape alone: a unit
//! direction, a finite origin, and a diameter that is twice a positive radius.
//! A fitting whose connectors disagree (a reducer) has no single nominal size
//! and is given none.

use crate::partition_room_parameters::{DATA_OBJECT_HEADER, verified_data_object};
use crate::{Result, RevitFile};
use std::collections::{BTreeMap, BTreeSet};

/// Bytes from a connector record's start to its diameter.
const DIAMETER_AT: usize = 64;
/// Bytes from a connector record's start to its radius.
const RADIUS_AT: usize = 48;
/// Largest radius read, feet.
const MAX_RADIUS_FEET: f64 = 10.0;

/// Data objects are read on Revit 2024 and later (RE-153).
pub fn supports_revit_version(revit_version: u32) -> bool {
    revit_version >= 2024
}

fn f64_at(buf: &[u8], at: usize) -> Option<f64> {
    Some(f64::from_le_bytes(
        buf.get(at..at.checked_add(8)?)?.try_into().ok()?,
    ))
}

/// The diameter of the connector record starting at `p` of `payload`, when
/// one starts there.
fn record_diameter(payload: &[u8], p: usize) -> Option<f64> {
    let direction = [
        f64_at(payload, p)?,
        f64_at(payload, p + 8)?,
        f64_at(payload, p + 16)?,
    ];
    let length = direction.iter().map(|v| v * v).sum::<f64>().sqrt();
    let unit = (length - 1.0).abs() <= 1e-9;
    let origin = [
        f64_at(payload, p + 24)?,
        f64_at(payload, p + 32)?,
        f64_at(payload, p + 40)?,
    ];
    let radius = f64_at(payload, p + RADIUS_AT)?;
    let diameter = f64_at(payload, p + DIAMETER_AT)?;
    (unit
        && origin.iter().all(|v| v.is_finite())
        && radius > 0.0
        && radius < MAX_RADIUS_FEET
        && (diameter - 2.0 * radius).abs() <= 1e-9 * diameter.max(1.0))
    .then_some(diameter)
}

/// The diameter of each connector record in `payload`, in order.
pub fn connector_diameters(payload: &[u8]) -> Vec<f64> {
    let mut out = Vec::new();
    let mut p = 0;
    while p + DIAMETER_AT + 8 <= payload.len() {
        match record_diameter(payload, p) {
            Some(diameter) => {
                out.push(diameter);
                p += DIAMETER_AT + 8;
            }
            None => p += 1,
        }
    }
    out
}

/// The nominal diameter, feet, of each of `fittings` whose own data object
/// holds connector records that all give one diameter.
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
                let diameters = connector_diameters(&buf[start + DATA_OBJECT_HEADER..object.end]);
                let Some(&first) = diameters.first() else {
                    continue;
                };
                let one_size = diameters
                    .iter()
                    .all(|d| (d - first).abs() <= 1e-9 * first.max(1.0));
                let size = one_size.then_some(first);
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
