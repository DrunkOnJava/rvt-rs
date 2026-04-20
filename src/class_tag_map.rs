//! Cross-version class-tag lookup (L5B-10).
//!
//! Revit assigns each top-level serializable class a `u16` tag in
//! the `Formats/Latest` schema header. The tag identifies the class
//! in serialized `Partitions/NN` records — `[u16 tag][record body]`.
//! The tag value for any given class **drifts across releases**
//! (`Wall` is `0x0123` in 2016, `0x012a` in 2021, `0x0131` in 2024)
//! as Autodesk inserts new classes into the alphabetical ordering.
//!
//! rvt-rs ships a baseline drift dataset in
//! `docs/data/tag-drift-2016-2026.csv` — 122 classes × 11 releases.
//! This module compiles that CSV into a queryable lookup table so
//! walkers that need to cross-reference tags across versions (for
//! forward-port tooling, diff analysis, or scoring heuristics)
//! don't have to re-read the file at runtime.
//!
//! # Usage
//!
//! ```no_run
//! use rvt::class_tag_map::{tag_for_class, class_for_tag, REVIT_VERSIONS};
//!
//! // Look up a class's tag in Revit 2024.
//! let tag = tag_for_class("Wall", 2024);   // Some(0x0131) — hypothetical
//!
//! // Iterate supported releases.
//! assert!(REVIT_VERSIONS.contains(&2024));
//!
//! // Resolve a tag back to a class name in a specific release.
//! let name = class_for_tag(0x0131, 2024);  // Some("Wall")
//! ```
//!
//! # Coverage scope
//!
//! The shipped dataset is the *baseline*, not exhaustive — 122 classes
//! sampled from the 395-class schema. Classes missing from the CSV
//! return `None`; callers needing the full cross-version map should
//! regenerate the CSV with `examples/tag_drift.rs` against a local
//! 11-release corpus.

/// The Revit releases represented in the compiled lookup table.
/// Kept in ascending order to match CSV column order.
pub const REVIT_VERSIONS: &[u16] = &[
    2016, 2017, 2018, 2019, 2020, 2021, 2022, 2023, 2024, 2025, 2026,
];

const TAG_DRIFT_CSV: &str = include_str!("../docs/data/tag-drift-2016-2026.csv");

/// Look up the serialization tag for `class_name` in Revit `version`.
/// Returns `None` when the class isn't in the baseline dataset OR
/// when the specific version didn't ship that class (e.g. a class
/// introduced in 2022 won't have a 2016 entry).
pub fn tag_for_class(class_name: &str, version: u16) -> Option<u16> {
    let col = version_column(version)?;
    for line in TAG_DRIFT_CSV.lines().skip(1) {
        let mut parts = line.splitn(2, ',');
        let name = parts.next()?;
        if name != class_name {
            continue;
        }
        let rest = parts.next()?;
        let cell = rest.split(',').nth(col - 1)?;
        return parse_tag(cell);
    }
    None
}

/// Reverse lookup: find the class name that carried the given tag
/// in a specific Revit release. Returns `None` when no class in the
/// baseline dataset had that tag in that version, or when `version`
/// isn't in [`REVIT_VERSIONS`].
pub fn class_for_tag(tag: u16, version: u16) -> Option<&'static str> {
    let col = version_column(version)?;
    for line in TAG_DRIFT_CSV.lines().skip(1) {
        let mut parts = line.splitn(2, ',');
        let name = parts.next()?;
        let rest = parts.next()?;
        if let Some(cell) = rest.split(',').nth(col - 1) {
            if let Some(t) = parse_tag(cell) {
                if t == tag {
                    return Some(name);
                }
            }
        }
    }
    None
}

/// Number of distinct classes in the baseline dataset. Useful for
/// sanity-checking test expectations against the shipped CSV.
pub fn dataset_size() -> usize {
    TAG_DRIFT_CSV.lines().skip(1).count()
}

fn version_column(version: u16) -> Option<usize> {
    REVIT_VERSIONS
        .iter()
        .position(|&v| v == version)
        .map(|i| i + 1)
}

/// Parse a hex tag cell like `"0x0123"`. Empty cells (class didn't
/// exist in that release) return `None`.
fn parse_tag(cell: &str) -> Option<u16> {
    let c = cell.trim();
    if c.is_empty() {
        return None;
    }
    let hex = c.strip_prefix("0x").unwrap_or(c);
    u16::from_str_radix(hex, 16).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn revit_versions_covers_2016_to_2026() {
        assert_eq!(REVIT_VERSIONS.len(), 11);
        assert_eq!(REVIT_VERSIONS[0], 2016);
        assert_eq!(REVIT_VERSIONS[10], 2026);
    }

    #[test]
    fn dataset_is_non_trivial() {
        // The CSV ships with >100 classes per repo policy.
        assert!(
            dataset_size() >= 100,
            "tag-drift dataset dropped below 100 classes: {}",
            dataset_size()
        );
    }

    #[test]
    fn tag_for_known_class_matches_csv() {
        // ADocWarnings is flat 0x001b across all releases per the CSV.
        assert_eq!(tag_for_class("ADocWarnings", 2016), Some(0x001b));
        assert_eq!(tag_for_class("ADocWarnings", 2026), Some(0x001b));
    }

    #[test]
    fn tag_for_unknown_class_is_none() {
        assert_eq!(tag_for_class("DefinitelyNotAClass", 2024), None);
    }

    #[test]
    fn tag_for_unknown_version_is_none() {
        // 2015 and 2027 aren't in the shipped dataset.
        assert_eq!(tag_for_class("ADocWarnings", 2015), None);
        assert_eq!(tag_for_class("ADocWarnings", 2027), None);
    }

    #[test]
    fn reverse_lookup_roundtrips() {
        // Pick a class we know — AProperties is 0x002a across all
        // releases — and verify the forward then reverse matches.
        let tag = tag_for_class("AProperties", 2024).expect("class present");
        assert_eq!(class_for_tag(tag, 2024), Some("AProperties"));
    }

    #[test]
    fn class_introduced_later_has_empty_early_cells() {
        // ATFProvenanceBaseCell appears starting 2022 per the CSV;
        // 2016-2021 cells are empty so lookup should return None.
        assert_eq!(tag_for_class("ATFProvenanceBaseCell", 2016), None);
        assert_eq!(tag_for_class("ATFProvenanceBaseCell", 2021), None);
        assert!(tag_for_class("ATFProvenanceBaseCell", 2022).is_some());
    }

    #[test]
    fn parse_tag_handles_empty_and_malformed() {
        assert_eq!(parse_tag(""), None);
        assert_eq!(parse_tag("   "), None);
        assert_eq!(parse_tag("not-hex"), None);
        assert_eq!(parse_tag("0x001b"), Some(0x001b));
        assert_eq!(parse_tag("001b"), Some(0x001b));
    }
}
