//! Serialised objects keyed by their ElementId, found in one pass.
//!
//! Many objects in a `Partitions/*` stream open with `01 00 00 00` and the
//! object's `u64` ElementId: run, stair, landing, support and railing types
//! (RE-52, RE-65, RE-66), Level name blocks (RE-51), materials (RE-58).
//! Looking for each wanted id with its own search costs one pass over the
//! partition per id. [`find_id_objects`] makes one pass for any number of
//! ids: it visits every `01 00 00 00` and keeps those followed by a wanted
//! id. `01 00 00 00` cannot overlap itself, so it finds exactly the
//! occurrences a search per id finds.

use std::collections::BTreeSet;

/// The bytes an object's ElementId follows.
pub const OBJECT_PREFIX: [u8; 4] = [1, 0, 0, 0];

/// Every `(id, offset)` in `buf` where `01 00 00 00 · u64 id` starts for an
/// id in `ids`; `offset` is where the `u64` id starts. In buffer order.
pub fn find_id_objects(buf: &[u8], ids: &BTreeSet<u32>) -> Vec<(u32, usize)> {
    let mut out = Vec::new();
    let (Some(&low), Some(&high)) = (ids.first(), ids.last()) else {
        return out;
    };
    for at in memchr::memmem::find_iter(buf, &OBJECT_PREFIX) {
        let id_at = at + OBJECT_PREFIX.len();
        let Some(raw) = buf.get(id_at..id_at + 8) else {
            continue;
        };
        if raw[4..] != [0, 0, 0, 0] {
            continue;
        }
        let id = u32::from_le_bytes([raw[0], raw[1], raw[2], raw[3]]);
        if (low..=high).contains(&id) && ids.contains(&id) {
            out.push((id, id_at));
        }
    }
    out
}
