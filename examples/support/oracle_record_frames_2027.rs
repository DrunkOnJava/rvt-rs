//! Research-only bounds for native 2027 oracle experiments. Not a production
//! owner decoder: the repeated identity is used solely to validate boundaries.
use std::ops::Range;

pub fn record_bodies(bytes: &[u8], version: u32) -> Vec<(usize, Range<usize>)> {
    if version != 2027 {
        return Vec::new();
    }
    let mut frames = Vec::new();
    for (carrier, window) in bytes.windows(6).enumerate() {
        if window != [0x10, 0x03, 1, 0, 0, 0] {
            continue;
        }
        let Some(id_bytes) = bytes.get(carrier + 6..carrier + 14) else {
            continue;
        };
        let id = u64::from_le_bytes(id_bytes.try_into().unwrap());
        if id == 0 || id > i64::MAX as u64 {
            continue;
        }
        let mut candidate = None;
        let mut ambiguous = false;
        for outer in carrier.saturating_sub(256)..carrier.saturating_sub(15) {
            if bytes.get(outer..outer + 8) != Some(id_bytes) {
                continue;
            }
            let Some(length_bytes) = bytes.get(outer + 12..outer + 16) else {
                continue;
            };
            let length = u32::from_le_bytes(length_bytes.try_into().unwrap()) as usize;
            if !(2..=16 * 1024 * 1024).contains(&length) {
                continue;
            }
            let start = outer + 16;
            let Some(end) = start.checked_add(length) else {
                continue;
            };
            if end < carrier + 14 || bytes.get(end..end + 4) != Some(length_bytes) {
                continue;
            }
            if candidate.is_some() {
                ambiguous = true;
                break;
            }
            candidate = Some((outer, start..end));
        }
        if !ambiguous {
            if let Some(frame) = candidate {
                frames.push(frame);
            }
        }
    }
    frames
}
