//! Storey elevations recovered from partition element-record bboxes.
//!
//! [`crate::partition_element_records`] decodes a Revit 2024 element
//! record's model-space bounding box as six `f64` feet at `+0x58`. The
//! `min_z` of a placed instance is a *measured* elevation, and on
//! `2024_Core_Interior.rvt` the 256 recovered `OST_Columns` records
//! carry exactly eleven distinct `min_z` values —
//! `{0, 31, 46, 61, 76, 91, 106, 121, 136, 151, 166}` ft — every one of
//! which is an `IfcBuildingStorey.Elevation` in Revit's own export of
//! the same file (#213). (#213 predicted ten; the measurement on the
//! recovered record set is eleven, with 166 ft — the export's
//! `Level 12` — the additional one.)
//!
//! That makes the bbox distribution better elevation evidence than
//! the partition Level *name* strings the exporter recovers today,
//! which arrive with no elevation at all when the file has no ArcWall
//! trailers (`levels` sits at `decoder_baseline` for exactly this
//! reason).
//!
//! # What this module claims, and what it does not
//!
//! - **Claimed:** a distinct base elevation in the record set is a
//!   storey elevation. Measured, and cross-checked against the
//!   reference export.
//! - **Not claimed:** which *named* Revit Level sits at that
//!   elevation. Level ElementId recovery is falsified on this corpus
//!   (RE-20 / #86) and the recovered name strings carry no order that
//!   would let a rank join stand up — on Core Interior there are 12
//!   name candidates against 11 measured elevations, and pairing the
//!   two by rank puts `Level 6` at 91 ft where the export puts
//!   `Level 7`. So names transfer only when the two sets are the same
//!   size, which is the same fail-closed rule
//!   [`crate::partition_arc_walls`] already applies to ArcWall
//!   elevations; otherwise every storey keeps its elevation-derived
//!   name and the unplaced names are reported as a warning.
//! - **Not claimed:** that the recovered set is complete. Eleven of
//!   the export's fifteen storeys have a column standing on them; the
//!   other four (−40, −20, 15, 185.5 ft) carry no `OST_Columns` record
//!   and are simply absent.

use crate::ifc::Storey;

/// Two base elevations within this many feet are the same storey.
///
/// Revit writes level elevations as exact doubles and the record bbox
/// echoes them, so the observed clustering is bit-exact; the window
/// exists to absorb round-tripping, not to merge nearby levels. It
/// matches [`crate::partition_arc_walls::storey_index_for_elevation`].
pub const STOREY_ELEVATION_TOLERANCE_FEET: f64 = 1e-3;

/// Fewest distinct elevations that count as a distribution.
///
/// One elevation is a single slab of elements, not evidence of a
/// storey *set*, and replacing a recovered Level list with it would
/// lose more than it adds.
pub const MIN_DISTINCT_ELEVATIONS: usize = 2;

/// Quantise to 1e-4 ft so bit-identical doubles group exactly.
fn quantise(value: f64) -> i64 {
    (value * 10_000.0).round() as i64
}

/// Distinct, ascending base elevations from a stream of measured
/// values. Non-finite values are dropped rather than propagated.
///
/// The representative of each group is the quantised value, not the
/// first raw double: bbox arithmetic leaves a `Level 1` base at
/// `-1.9e-14` ft rather than `0.0`, and 1e-4 ft (0.03 mm) is orders of
/// magnitude below anything the format expresses, so rounding there
/// recovers the number Revit stored instead of echoing the noise.
pub fn distinct_base_elevations_feet(values: impl IntoIterator<Item = f64>) -> Vec<f64> {
    let mut keys: Vec<i64> = values
        .into_iter()
        .filter(|v| v.is_finite())
        .map(quantise)
        .collect();
    keys.sort_unstable();
    keys.dedup();
    keys.into_iter().map(|key| key as f64 / 10_000.0).collect()
}

/// Name a storey after the elevation it was measured at.
///
/// Deliberately not a Level name: the file supplies the number, not
/// the label. Matches the `Elevation {:.3} ft` shape the ArcWall
/// storey recovery already emits.
pub fn elevation_storey_name(elevation_feet: f64) -> String {
    format!("Elevation {elevation_feet:.3} ft")
}

/// True when `storeys` carry no elevation evidence — empty, or every
/// entry sitting at the same elevation (which on current corpora
/// means every entry defaulted to 0.0 because the Level rows had no
/// elevation to give).
pub fn storeys_lack_elevation_evidence(storeys: &[Storey]) -> bool {
    let mut iter = storeys.iter().map(|s| quantise(s.elevation_feet));
    let Some(first) = iter.next() else {
        return true;
    };
    iter.all(|key| key == first)
}

/// Outcome of [`storeys_from_base_elevations`], kept separate from the
/// storeys themselves so callers can report *why* names did or did not
/// transfer without re-deriving it.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ElementRecordStoreyRecovery {
    /// One storey per distinct measured elevation, ascending.
    pub storeys: Vec<Storey>,
    /// Names carried over from the recovered Level strings.
    pub named_from_levels: usize,
    /// Level name candidates the recovery could not place, in the
    /// order they arrived. Non-empty means the counts disagreed.
    pub unplaced_level_names: Vec<String>,
}

/// Build one storey per distinct measured base elevation.
///
/// `level_names` are the recovered Level display names, in whatever
/// order the caller holds them. They are applied **only** when there
/// is exactly one name per measured elevation — that is the single
/// case where a rank join is not an invention, and it is the rule
/// [`crate::partition_arc_walls::recover_storeys_from_arc_walls`]
/// already uses. Otherwise every storey keeps its elevation-derived
/// name and the names are returned unplaced.
///
/// Returns an empty recovery when fewer than
/// [`MIN_DISTINCT_ELEVATIONS`] elevations were measured (fail closed).
pub fn storeys_from_base_elevations(
    elevations_feet: &[f64],
    level_names: &[String],
) -> ElementRecordStoreyRecovery {
    if elevations_feet.len() < MIN_DISTINCT_ELEVATIONS {
        return ElementRecordStoreyRecovery::default();
    }
    let mut storeys: Vec<Storey> = elevations_feet
        .iter()
        .map(|&elevation_feet| Storey {
            name: elevation_storey_name(elevation_feet),
            elevation_feet,
        })
        .collect();
    if level_names.len() == storeys.len() {
        let ordered = crate::partition_arc_walls::order_building_storey_names(level_names);
        for (storey, name) in storeys.iter_mut().zip(ordered) {
            storey.name = name;
        }
        return ElementRecordStoreyRecovery {
            named_from_levels: storeys.len(),
            storeys,
            unplaced_level_names: Vec::new(),
        };
    }
    ElementRecordStoreyRecovery {
        storeys,
        named_from_levels: 0,
        unplaced_level_names: level_names.to_vec(),
    }
}

/// Index of the storey whose elevation equals `elevation_feet`.
///
/// Fails closed on both misses and ambiguity: `None` when nothing
/// matches, and `None` when more than one storey is inside the
/// tolerance window (a storey list that dense is not one this join can
/// resolve).
pub fn unique_storey_index_for_elevation(storeys: &[Storey], elevation_feet: f64) -> Option<usize> {
    let mut hit = None;
    for (index, storey) in storeys.iter().enumerate() {
        if (storey.elevation_feet - elevation_feet).abs() < STOREY_ELEVATION_TOLERANCE_FEET {
            if hit.is_some() {
                return None;
            }
            hit = Some(index);
        }
    }
    hit
}

#[cfg(test)]
mod tests {
    use super::*;

    fn storey(name: &str, elevation_feet: f64) -> Storey {
        Storey {
            name: name.to_string(),
            elevation_feet,
        }
    }

    #[test]
    fn distinct_elevations_are_deduplicated_and_sorted() {
        let got = distinct_base_elevations_feet([76.0, 0.0, 31.0, 76.0, 0.0, 151.0]);
        assert_eq!(got, vec![0.0, 31.0, 76.0, 151.0]);
    }

    #[test]
    fn non_finite_elevations_are_dropped() {
        let got = distinct_base_elevations_feet([0.0, f64::NAN, f64::INFINITY, 15.0]);
        assert_eq!(got, vec![0.0, 15.0]);
    }

    #[test]
    fn a_single_elevation_is_not_a_distribution() {
        let recovery = storeys_from_base_elevations(&[0.0], &[]);
        assert!(recovery.storeys.is_empty());
    }

    #[test]
    fn elevations_become_storeys_named_after_their_elevation() {
        let recovery = storeys_from_base_elevations(&[0.0, 31.0, 46.0], &[]);
        assert_eq!(recovery.storeys.len(), 3);
        assert_eq!(recovery.storeys[1].name, "Elevation 31.000 ft");
        assert_eq!(recovery.storeys[1].elevation_feet, 31.0);
        assert_eq!(recovery.named_from_levels, 0);
    }

    #[test]
    fn level_names_transfer_only_when_the_counts_match() {
        let names = vec!["Level 2".to_string(), "Level 1".to_string()];
        let matched = storeys_from_base_elevations(&[0.0, 15.0], &names);
        assert_eq!(matched.named_from_levels, 2);
        assert_eq!(matched.storeys[0].name, "Level 1");
        assert_eq!(matched.storeys[1].name, "Level 2");
        assert!(matched.unplaced_level_names.is_empty());

        // 2024_Core_Interior.rvt's shape: more names than measured
        // elevations, so a rank join would mislabel. Fail closed.
        let mismatched = storeys_from_base_elevations(&[0.0, 15.0, 30.0], &names);
        assert_eq!(mismatched.named_from_levels, 0);
        assert_eq!(mismatched.storeys[0].name, "Elevation 0.000 ft");
        assert_eq!(mismatched.unplaced_level_names, names);
    }

    #[test]
    fn elevation_join_is_exact_and_fails_closed() {
        let storeys = vec![storey("A", 0.0), storey("B", 76.0)];
        assert_eq!(unique_storey_index_for_elevation(&storeys, 76.0), Some(1));
        assert_eq!(
            unique_storey_index_for_elevation(&storeys, 76.000_5),
            Some(1)
        );
        assert_eq!(unique_storey_index_for_elevation(&storeys, 75.0), None);
        // Two storeys inside the window is ambiguous, not a coin flip.
        let dense = vec![storey("A", 76.0), storey("B", 76.000_1)];
        assert_eq!(unique_storey_index_for_elevation(&dense, 76.0), None);
    }

    #[test]
    fn elevation_evidence_detection() {
        assert!(storeys_lack_elevation_evidence(&[]));
        assert!(storeys_lack_elevation_evidence(&[
            storey("A", 0.0),
            storey("B", 0.0)
        ]));
        assert!(!storeys_lack_elevation_evidence(&[
            storey("A", 0.0),
            storey("B", 15.0)
        ]));
    }
}
