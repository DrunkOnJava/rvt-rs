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
//!
//! A record that names exactly two Levels names its base and top
//! constraint (RE-59). The list does not order them. Revit's own exports
//! always contain the element in the higher of the two that lies at or
//! below the record's base, or in the lower one when neither does: the
//! element's base constraint ([`base_constraint_level`]).
//!
//! A record that names no Level keeps its reference list
//! ([`REFERENCES_FIELD`], RE-60). A railing then takes the Level of the one
//! stair or ramp it names. Any other element takes a Level only where two
//! readings agree: the Level of the objects it names (each object
//! `01 00 00 00 · u64 id · u64 Level id`), and the highest Level at or below
//! its base ([`level_at_or_below`]).

use std::collections::{BTreeMap, BTreeSet};

/// Field carrying the recovered host `Level` ElementId on a
/// record-backed element.
pub const LEVEL_REFERENCE_FIELD: &str = "m_levelId";

/// Value recorded for where [`LEVEL_REFERENCE_FIELD`] came from.
pub const LEVEL_REFERENCE_SOURCE: &str = "partition_element_record_reference_list";

/// Property the emitted element carries [`LEVEL_REFERENCE_FIELD`] on.
pub const LEVEL_ELEMENT_ID_PROPERTY: &str = "LevelElementId";

/// Field carrying the two Levels a record names, in list order, when it
/// names exactly two (RE-59).
pub const CONSTRAINT_LEVELS_FIELD: &str = "m_constraintLevelIds";

/// Field recording how [`LEVEL_REFERENCE_FIELD`] was chosen when it is not
/// the single Level a record names.
pub const LEVEL_BIND_SOURCE_FIELD: &str = "m_levelBindSource";

/// Value of [`LEVEL_BIND_SOURCE_FIELD`] for a Level chosen by
/// [`base_constraint_level`].
pub const BASE_CONSTRAINT_SOURCE: &str = "partition_element_record_base_constraint";

/// Field carrying the reference list of a record that names no Level,
/// less its own id (RE-60).
pub const REFERENCES_FIELD: &str = "m_referenceIds";

/// Value of [`LEVEL_BIND_SOURCE_FIELD`] for a railing given the Level of
/// the stair or ramp its record names (RE-60).
pub const HOST_SOURCE: &str = "partition_element_record_host";

/// Value of [`LEVEL_BIND_SOURCE_FIELD`] for an element given the Level
/// the objects its record names carry, where that is also the highest
/// Level at or below its base (RE-60).
pub const LEVEL_OBJECT_SOURCE: &str = "partition_element_record_level_object";

/// Value of [`LEVEL_BIND_SOURCE_FIELD`] for a stair or ramp whose base sits
/// exactly at one Level's elevation (RE-68).
pub const BASE_AT_LEVEL_SOURCE: &str = "partition_element_record_base_at_level";

/// Value of [`LEVEL_BIND_SOURCE_FIELD`] for an element given the Level of
/// the one element of its own class its record names, such as a part of a
/// nested light-fixture family (RE-68).
pub const SAME_CLASS_HOST_SOURCE: &str = "partition_element_record_same_class_host";

/// Value of [`LEVEL_BIND_SOURCE_FIELD`] for an element placed by its base
/// elevation (RE-68, [`elevation_band_level`]).
pub const BASE_ELEVATION_SOURCE: &str = "partition_element_record_base_elevation";

/// How far below a Level an element's base may sit and still be on that
/// Level (RE-68). On Snowdon Towers Revit puts on the Level just above
/// their base 5 elements whose base is 0.013 to 0.233 ft below it.
pub const BELOW_LEVEL_TOLERANCE_FEET: f64 = 0.25;

/// How far below the next Level up an element's base must sit to be on the
/// Level at or below it (RE-68). The nearest such base on Snowdon Towers is
/// 0.333 ft below the next Level. Between this and
/// [`BELOW_LEVEL_TOLERANCE_FEET`] no Level is chosen.
pub const CLEAR_OF_LEVEL_FEET: f64 = 0.5;

/// The Level an element sits on by its base elevation (RE-68), within a
/// fail-closed band:
/// - the Level just above `base_feet` when the base is at most
///   [`BELOW_LEVEL_TOLERANCE_FEET`] below it;
/// - the Level at or below the base ([`level_at_or_below`]) when the next
///   Level up is at least [`CLEAR_OF_LEVEL_FEET`] above it, or there is none;
/// - `None` in between, or when two Levels share the elevation that decides.
pub fn elevation_band_level(elevations: &BTreeMap<u32, f64>, base_feet: f64) -> Option<u32> {
    if !base_feet.is_finite() {
        return None;
    }
    let above = elevations
        .values()
        .copied()
        .filter(|at| *at > base_feet + 1e-3)
        .fold(None, |low: Option<f64>, at| {
            Some(low.map_or(at, |l| l.min(at)))
        });
    match above {
        Some(up) if up - base_feet <= BELOW_LEVEL_TOLERANCE_FEET => {
            let mut at_up = elevations.iter().filter(|(_, at)| **at == up);
            let (id, _) = at_up.next()?;
            at_up.next().is_none().then_some(*id)
        }
        Some(up) if up - base_feet < CLEAR_OF_LEVEL_FEET => None,
        _ => level_at_or_below(elevations, base_feet),
    }
}

/// The highest Level whose elevation is at or below `base_feet` (within
/// 1e-3 ft), or `None` when none is, or when two Levels share that
/// elevation.
pub fn level_at_or_below(elevations: &BTreeMap<u32, f64>, base_feet: f64) -> Option<u32> {
    if !base_feet.is_finite() {
        return None;
    }
    let mut best: Option<(u32, f64)> = None;
    let mut tied = false;
    for (&id, &at) in elevations {
        if at > base_feet + 1e-3 {
            continue;
        }
        match best {
            Some((_, seen)) if at < seen => {}
            Some((_, seen)) if at == seen => tied = true,
            _ => {
                best = Some((id, at));
                tied = false;
            }
        }
    }
    if tied { None } else { best.map(|(id, _)| id) }
}

/// The distinct recovered `Level` ElementIds a record's counted reference
/// list names, in list order.
pub fn named_levels(references: &[u64], level_ids: &BTreeSet<u32>) -> Vec<u32> {
    let mut named = Vec::new();
    for slot in references {
        if let Ok(id) = u32::try_from(*slot) {
            if level_ids.contains(&id) && !named.contains(&id) {
                named.push(id);
            }
        }
    }
    named
}

/// The base constraint of an element whose record names exactly two
/// Levels (RE-59): the higher of the two whose elevation is at or below the
/// record's base `base_feet` (within 1e-3 ft), else the lower of the two.
/// `None` unless exactly two Levels are named, both have an elevation and
/// the elevations differ.
///
/// Measured against the storey Revit's own IFC export contains the element
/// in, over every record naming two Levels: Snowdon Towers 650 of 650,
/// 2024_Core_Interior 482 of 482, RE1 Architecture 7 of 7.
pub fn base_constraint_level(
    named: &[u32],
    elevations: &BTreeMap<u32, f64>,
    base_feet: f64,
) -> Option<u32> {
    let [a, b] = named else {
        return None;
    };
    let (ea, eb) = (*elevations.get(a)?, *elevations.get(b)?);
    // Two Levels at one elevation: nothing tells base from top.
    if ea == eb || !base_feet.is_finite() {
        return None;
    }
    let (low, high, high_at) = if ea < eb { (*a, *b, eb) } else { (*b, *a, ea) };
    Some(if high_at <= base_feet + 1e-3 {
        high
    } else {
        low
    })
}

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
