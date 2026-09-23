//! Stair and stair-run dimensions from their serialised element data
//! (RE-47).
//!
//! A stair's data (its element-data header and ElementId, see
//! [`crate::partition_names::element_data_header`]) carries, a fixed
//! distance past the bytes [`STAIR_DIMENSIONS_ANCHOR`]:
//!
//! ```text
//! +16  f64  riser height, feet
//! +24  f64  tread depth, feet
//! +88  u32  number of risers
//! ```
//!
//! The anchor sits `0x63` or `0x6b` bytes into the data on Autodesk's
//! Snowdon Towers 2024 architectural sample: an optional field before it
//! moves it, so it is found rather than assumed. All three values equal the
//! `Pset_StairCommon` `RiserHeight`, `TreadLength` and `NumberOfRiser`
//! Revit's own IFC4 export writes for the same stair, on 29 of 29 stairs.
//!
//! A stair run's data carries its own number of risers right after
//! [`STAIR_RUN_RISERS_ANCHOR`], at `0x42` on every Snowdon run. The runs of
//! a stair add up to the stair's count on 26 of its 29 stairs. The other
//! three have one run each, which reads one more than the stair, so a
//! single-run stair's flight takes the stair's count
//! ([`flight_riser_counts`]).
//!
//! Nothing here reads a release other than 2024.

use crate::{Result, RevitFile};
use std::collections::BTreeMap;

/// Releases where these layouts are measured.
pub const STAIR_DIMENSIONS_SUPPORTED_REVIT_VERSIONS: &[u32] = &[2024];

/// The bytes the stair dimensions follow: two unset `0x03fb` field frames
/// and a zero `u32`.
pub const STAIR_DIMENSIONS_ANCHOR: [u8; 16] = [
    0xff, 0xff, 0xff, 0xff, 0xfb, 0x03, 0xff, 0xff, 0xff, 0xff, 0xfb, 0x03, 0x00, 0x00, 0x00, 0x00,
];

/// Offsets past the start of [`STAIR_DIMENSIONS_ANCHOR`].
pub const RISER_HEIGHT_OFFSET: usize = 16;
/// See [`RISER_HEIGHT_OFFSET`].
pub const TREAD_DEPTH_OFFSET: usize = 24;
/// See [`RISER_HEIGHT_OFFSET`].
pub const RISER_COUNT_OFFSET: usize = 88;

/// The bytes a run's number of risers follows.
pub const STAIR_RUN_RISERS_ANCHOR: [u8; 15] = [
    0xfc, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00,
];

/// How far into an element's data an anchor is looked for.
pub const ANCHOR_WINDOW: usize = 0x100;

/// Largest number of risers accepted; a stair of this many would rise
/// hundreds of feet.
pub const MAX_RISERS: u32 = 400;

/// A stair's riser and tread dimensions (RE-47).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StairDimensions {
    /// Height of one riser, feet.
    pub riser_height_feet: f64,
    /// Depth of one tread, feet.
    pub tread_depth_feet: f64,
    /// Number of risers of the whole stair.
    pub riser_count: u32,
}

/// Whether these layouts are measured for `revit_version`.
pub fn supports_revit_version(revit_version: u32) -> bool {
    STAIR_DIMENSIONS_SUPPORTED_REVIT_VERSIONS.contains(&revit_version)
}

fn read_u32(buf: &[u8], at: usize) -> Option<u32> {
    buf.get(at..at.checked_add(4)?)
        .map(|s| u32::from_le_bytes(s.try_into().expect("4 bytes")))
}

fn read_f64(buf: &[u8], at: usize) -> Option<f64> {
    buf.get(at..at.checked_add(8)?)
        .map(|s| f64::from_le_bytes(s.try_into().expect("8 bytes")))
}

/// Where `anchor` first starts within [`ANCHOR_WINDOW`] of `data_offset`.
fn anchor_at(buf: &[u8], data_offset: usize, anchor: &[u8]) -> Option<usize> {
    let end = data_offset
        .checked_add(ANCHOR_WINDOW + anchor.len())?
        .min(buf.len());
    let window = buf.get(data_offset..end)?;
    memchr::memmem::find(window, anchor).map(|at| data_offset + at)
}

/// A length a stair can have: finite, positive and under `limit_feet`.
fn plausible(value: f64, limit_feet: f64) -> Option<f64> {
    (value.is_finite() && value > 0.0 && value < limit_feet).then_some(value)
}

/// The dimensions in the stair data that starts at `data_offset` (its
/// element-data header), or `None` when the anchor is missing or a value
/// is out of range.
pub fn stair_dimensions_at(buf: &[u8], data_offset: usize) -> Option<StairDimensions> {
    let anchor = anchor_at(buf, data_offset, &STAIR_DIMENSIONS_ANCHOR)?;
    let riser_height_feet = plausible(read_f64(buf, anchor + RISER_HEIGHT_OFFSET)?, 2.0)?;
    let tread_depth_feet = plausible(read_f64(buf, anchor + TREAD_DEPTH_OFFSET)?, 10.0)?;
    let riser_count = read_u32(buf, anchor + RISER_COUNT_OFFSET)?;
    if !(1..=MAX_RISERS).contains(&riser_count) {
        return None;
    }
    Some(StairDimensions {
        riser_height_feet,
        tread_depth_feet,
        riser_count,
    })
}

/// The number of risers in the run data that starts at `data_offset`.
pub fn run_riser_count_at(buf: &[u8], data_offset: usize) -> Option<u32> {
    let anchor = anchor_at(buf, data_offset, &STAIR_RUN_RISERS_ANCHOR)?;
    let count = read_u32(buf, anchor + STAIR_RUN_RISERS_ANCHOR.len())?;
    (1..=MAX_RISERS).contains(&count).then_some(count)
}

/// Where each id in `ids` has its element data in `buf`, first occurrence.
fn data_offsets(buf: &[u8], header: &[u8; 10], ids: &[u32]) -> BTreeMap<u32, usize> {
    let mut out = BTreeMap::new();
    for hit in memchr::memmem::find_iter(buf, header) {
        let Some(id) = buf
            .get(hit + header.len()..hit + header.len() + 8)
            .map(|s| u64::from_le_bytes(s.try_into().expect("8 bytes")))
            .and_then(|id| u32::try_from(id).ok())
        else {
            continue;
        };
        if ids.contains(&id) {
            out.entry(id).or_insert(hit);
        }
    }
    out
}

/// The dimensions of each stair in `stairs` and the riser count of each run
/// in `runs`, read from every partition. An id whose copies disagree is
/// dropped. Both maps are empty for a release these layouts are not
/// measured on.
pub fn scan_stair_dimensions(
    rf: &mut RevitFile,
    revit_version: u32,
    stairs: &[u32],
    runs: &[u32],
) -> Result<(BTreeMap<u32, StairDimensions>, BTreeMap<u32, u32>)> {
    let mut stair_out: BTreeMap<u32, Option<StairDimensions>> = BTreeMap::new();
    let mut run_out: BTreeMap<u32, Option<u32>> = BTreeMap::new();
    let header = match crate::partition_names::element_data_header(revit_version) {
        Some(header) if supports_revit_version(revit_version) => header,
        _ => return Ok((BTreeMap::new(), BTreeMap::new())),
    };
    let wanted: Vec<u32> = stairs.iter().chain(runs).copied().collect();
    for stream in rf.partition_stream_names() {
        let Ok(inflated) = rf.inflated_partition(&stream) else {
            continue;
        };
        let buf = inflated.bytes();
        for (id, offset) in data_offsets(buf, &header, &wanted) {
            if stairs.contains(&id) {
                if let Some(found) = stair_dimensions_at(buf, offset) {
                    merge(&mut stair_out, id, found);
                }
            }
            if runs.contains(&id) {
                if let Some(found) = run_riser_count_at(buf, offset) {
                    merge(&mut run_out, id, found);
                }
            }
        }
    }
    Ok((settled(stair_out), settled(run_out)))
}

fn merge<T: PartialEq>(out: &mut BTreeMap<u32, Option<T>>, id: u32, found: T) {
    match out.get_mut(&id) {
        None => {
            out.insert(id, Some(found));
        }
        Some(held) => {
            if held.as_ref() != Some(&found) {
                *held = None;
            }
        }
    }
}

fn settled<T>(map: BTreeMap<u32, Option<T>>) -> BTreeMap<u32, T> {
    map.into_iter()
        .filter_map(|(id, value)| value.map(|value| (id, value)))
        .collect()
}

/// Each run's number of risers, given the stair's and those its runs
/// record: a single run has the stair's, and several runs have their own
/// when they add up to the stair's. Otherwise nothing is claimed.
pub fn flight_riser_counts(stair_risers: u32, runs: &[(u32, Option<u32>)]) -> BTreeMap<u32, u32> {
    if let [(run, _)] = runs {
        return BTreeMap::from([(*run, stair_risers)]);
    }
    let counts: Option<Vec<u32>> = runs.iter().map(|(_, count)| *count).collect();
    match counts {
        Some(counts) if counts.iter().sum::<u32>() == stair_risers => runs
            .iter()
            .zip(counts)
            .map(|((run, _), count)| (*run, count))
            .collect(),
        _ => BTreeMap::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::partition_names::ELEMENT_DATA_HEADER;

    /// Snowdon stair 620883's data up to its riser count: 19 risers of
    /// 11/19 ft, treads of 11 in, an 11 ft stair.
    fn stair_620883(shift: usize) -> Vec<u8> {
        let mut out = ELEMENT_DATA_HEADER.to_vec();
        out.extend_from_slice(&620_883u64.to_le_bytes());
        out.extend(vec![0xff; 0x45 + shift]);
        out.extend_from_slice(&STAIR_DIMENSIONS_ANCHOR);
        out.extend_from_slice(&(11.0f64 / 19.0).to_le_bytes());
        out.extend_from_slice(&(11.0f64 / 12.0).to_le_bytes());
        out.extend(vec![0u8; 24]);
        out.extend_from_slice(&11.0f64.to_le_bytes());
        out.extend(vec![0u8; 24]);
        out.extend_from_slice(&19u32.to_le_bytes());
        out.extend_from_slice(&1u32.to_le_bytes());
        out
    }

    #[test]
    fn stair_dimensions_follow_their_anchor() {
        for shift in [0, 8] {
            let buf = stair_620883(shift);
            let found = stair_dimensions_at(&buf, 0).expect("dimensions");
            assert_eq!(found.riser_count, 19);
            assert!((found.riser_height_feet * 0.3048 - 0.176_463_157_894_736_85).abs() < 1e-12);
            assert!((found.tread_depth_feet * 0.3048 - 0.2794).abs() < 1e-12);
        }
        // No anchor, no dimensions.
        let mut buf = stair_620883(0);
        let at = memchr::memmem::find(&buf, &STAIR_DIMENSIONS_ANCHOR).expect("anchor");
        buf[at + 4] = 0;
        assert_eq!(stair_dimensions_at(&buf, 0), None);
        // An anchor past the window is not the stair's.
        assert_eq!(stair_dimensions_at(&stair_620883(ANCHOR_WINDOW), 0), None);
    }

    #[test]
    fn a_run_records_its_risers() {
        // Snowdon run 621141: 10 risers.
        let mut buf = ELEMENT_DATA_HEADER.to_vec();
        buf.extend_from_slice(&621_141u64.to_le_bytes());
        buf.extend(vec![0xff; 0x2a]);
        buf.extend_from_slice(&STAIR_RUN_RISERS_ANCHOR);
        buf.extend_from_slice(&10u32.to_le_bytes());
        assert_eq!(run_riser_count_at(&buf, 0), Some(10));
        let offsets = data_offsets(&buf, &ELEMENT_DATA_HEADER, &[621_141]);
        assert_eq!(offsets.get(&621_141), Some(&0));
    }

    #[test]
    fn flight_counts_add_up_to_the_stair_or_are_not_claimed() {
        assert_eq!(
            flight_riser_counts(19, &[(621_141, Some(10)), (621_208, Some(9))]),
            BTreeMap::from([(621_141, 10), (621_208, 9)])
        );
        // A single run is the whole stair, whatever it records.
        assert_eq!(
            flight_riser_counts(2, &[(1_563_804, Some(3))]),
            BTreeMap::from([(1_563_804, 2)])
        );
        assert!(flight_riser_counts(19, &[(1, Some(10)), (2, Some(10))]).is_empty());
        assert!(flight_riser_counts(19, &[(1, Some(10)), (2, None)]).is_empty());
    }

    #[test]
    fn only_2024_is_read() {
        assert!(supports_revit_version(2024));
        assert!(!supports_revit_version(2025));
    }
}
