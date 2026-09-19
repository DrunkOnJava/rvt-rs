//! Level ElementId binding from a partition element record's counted
//! reference list (#219, RE-27).
//!
//! RE-23 read the slot *before* a record's own id in the counted list
//! at `+0x88` to find a door's host wall, and RE-25 read the last slot
//! of the *second* list to find a sketch line's owner. This reads the
//! same first list for a different fact: a placed instance names the
//! Revit `Level` that hosts it, as a plain ElementId slot.
//!
//! The rule is **exactly one**: a record whose reference list names no
//! recovered Level, or names two or more, resolves to `None`. That is
//! not a tuning choice — a Revit column or wall carries both a base
//! constraint and a top constraint, and on
//! `2024_Core_Interior.rvt` every `OST_Columns` record that names a
//! Level at all names two of them. Picking one of the pair by slot
//! position would be a guess, so the element keeps whatever the
//! elevation join gives it (which for those columns is already exact).
//!
//! See `reports/element-framing/RE-27-level-reference-storey-bind.md`.

use std::collections::BTreeSet;

/// Field carrying the recovered host `Level` ElementId on a
/// record-backed element.
pub const LEVEL_REFERENCE_FIELD: &str = "m_levelId";

/// Value recorded for where [`LEVEL_REFERENCE_FIELD`] came from.
pub const LEVEL_REFERENCE_SOURCE: &str = "partition_element_record_reference_list";

/// Property the emitted element carries [`LEVEL_REFERENCE_FIELD`] on.
pub const LEVEL_ELEMENT_ID_PROPERTY: &str = "LevelElementId";

/// The single recovered `Level` ElementId a record's counted
/// reference list names, or `None`.
///
/// Fail closed on both edges: nothing named (the list did not decode,
/// or holds no Level) and more than one distinct Level named.
pub fn unique_level_reference(references: &[u64], level_ids: &BTreeSet<u32>) -> Option<u32> {
    let mut found: Option<u32> = None;
    for slot in references {
        let Ok(id) = u32::try_from(*slot) else {
            continue;
        };
        if !level_ids.contains(&id) {
            continue;
        }
        match found {
            None => found = Some(id),
            Some(seen) if seen == id => {}
            // Two distinct Levels: base and top constraint, with
            // nothing in the bytes saying which is which.
            Some(_) => return None,
        }
    }
    found
}

#[cfg(test)]
mod tests {
    use super::*;

    fn levels(ids: &[u32]) -> BTreeSet<u32> {
        ids.iter().copied().collect()
    }

    #[test]
    fn one_named_level_resolves() {
        assert_eq!(
            unique_level_reference(&[9001, 20307, 71_299], &levels(&[20307, 20308])),
            Some(20307)
        );
    }

    #[test]
    fn the_same_level_named_twice_is_still_one_level() {
        assert_eq!(
            unique_level_reference(&[20307, 55, 20307], &levels(&[20307])),
            Some(20307)
        );
    }

    #[test]
    fn two_distinct_levels_fail_closed() {
        assert_eq!(
            unique_level_reference(&[20307, 20308], &levels(&[20307, 20308])),
            None
        );
    }

    #[test]
    fn no_named_level_resolves_to_none() {
        assert_eq!(unique_level_reference(&[1, 2, 3], &levels(&[20307])), None);
        assert_eq!(unique_level_reference(&[], &levels(&[20307])), None);
    }

    #[test]
    fn slots_outside_the_elementid_range_are_skipped() {
        let out_of_range = u64::from(u32::MAX) + 1;
        assert_eq!(
            unique_level_reference(&[out_of_range, 20307], &levels(&[20307])),
            Some(20307)
        );
    }
}
