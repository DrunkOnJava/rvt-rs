//! A pipe fitting's nominal size, from its own data object (RE-165, B44, #35).
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
//! It also holds its family's parameter values, each after the first an entry
//! of 38 bytes: `ff × 8`, a `u32` key that differs from family to family, 18
//! zero bytes, then the `f64` value in feet. Some entries come in pairs, a
//! radius and then a diameter twice it: the nominal size and the body's
//! outside sizes, which are larger.
//!
//! On RE1 Plumbing (Revit 2025, MIT) Revit's own export writes each of the 42
//! fittings' `Pset_PipeFittingTypeCommon.NominalDiameter` (10 to 50 mm): on
//! the fittings with connector records, the one diameter they all give; on
//! the cap, which holds none, the smallest of its pairs. So a fitting's
//! nominal diameter is its connectors' one diameter, or, where it holds no
//! connector record, its smallest pair's. A fitting whose connectors disagree
//! (a reducer) has no single nominal size and is given none.

use crate::partition_room_parameters::{DATA_OBJECT_HEADER, verified_data_object};
use crate::{Result, RevitFile};
use std::collections::{BTreeMap, BTreeSet};

/// Bytes from a connector record's start to its radius.
const CONNECTOR_RADIUS_AT: usize = 48;
/// Bytes from a connector record's start to its diameter.
const CONNECTOR_DIAMETER_AT: usize = 64;
/// Bytes of a parameter entry before its value: the separator, the key and
/// the zeros.
const ENTRY_HEAD: usize = 30;
/// Bytes from one parameter value to the next.
const ENTRY_STRIDE: usize = ENTRY_HEAD + 8;
/// Smallest and largest diameter read, feet.
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

/// `diameter` when it is a size and twice `radius`.
fn twice(radius: f64, diameter: f64) -> Option<f64> {
    (diameter.is_normal() && DIAMETER_FEET.contains(&diameter) && diameter == 2.0 * radius)
        .then_some(diameter)
}

/// The diameter of the connector record starting at `p` of `payload`, when
/// one starts there.
fn connector_diameter(payload: &[u8], p: usize) -> Option<f64> {
    let direction = [
        f64_at(payload, p)?,
        f64_at(payload, p + 8)?,
        f64_at(payload, p + 16)?,
    ];
    let length = direction.iter().map(|v| v * v).sum::<f64>().sqrt();
    let origin = [
        f64_at(payload, p + 24)?,
        f64_at(payload, p + 32)?,
        f64_at(payload, p + 40)?,
    ];
    if (length - 1.0).abs() > 1e-9 || !origin.iter().all(|v| v.is_finite()) {
        return None;
    }
    twice(
        f64_at(payload, p + CONNECTOR_RADIUS_AT)?,
        f64_at(payload, p + CONNECTOR_DIAMETER_AT)?,
    )
}

/// The diameter of each connector record in `payload`, in order.
pub fn connector_diameters(payload: &[u8]) -> Vec<f64> {
    let mut out = Vec::new();
    let mut p = 0;
    while p + CONNECTOR_DIAMETER_AT + 8 <= payload.len() {
        match connector_diameter(payload, p) {
            Some(diameter) => {
                out.push(diameter);
                p += CONNECTOR_DIAMETER_AT + 8;
            }
            None => p += 1,
        }
    }
    out
}

/// The diameter of the parameter entry whose value starts at `at` in
/// `payload`, when the value is twice the one before it.
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
    twice(
        f64_at(payload, at.checked_sub(ENTRY_STRIDE)?)?,
        f64_at(payload, at)?,
    )
}

/// The diameter of each radius and diameter parameter pair in `payload`.
pub fn pair_diameters(payload: &[u8]) -> Vec<f64> {
    (ENTRY_STRIDE..payload.len())
        .filter_map(|at| pair_diameter(payload, at))
        .collect()
}

/// What a fitting's data object `payload` says of its nominal diameter:
/// `None` when it holds neither a connector record nor a pair, `Some(None)`
/// when its connectors disagree.
fn nominal_diameter(payload: &[u8]) -> Option<Option<f64>> {
    let connectors = connector_diameters(payload);
    if let Some(&first) = connectors.first() {
        return Some(connectors.iter().all(|&d| d == first).then_some(first));
    }
    pair_diameters(payload)
        .into_iter()
        .min_by(f64::total_cmp)
        .map(Some)
}

/// The nominal diameter, feet, of each of `fittings` whose own data object
/// gives one.
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
                let Some(size) = nominal_diameter(&buf[start + DATA_OBJECT_HEADER..object.end])
                else {
                    continue;
                };
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
