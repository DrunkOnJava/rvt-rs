//! Partition UTF-16LE display-name candidates (RE-15 / #86).
//!
//! Adapted from the RE-15 geometry-probes branch (`cursor/re15-geometry-probes-44c9`,
//! PR #117). Pure string heuristics over
//! [`crate::object_graph::string_records_from_partitions`] — no ElementId
//! binding yet. Callers that need storey names should further filter with
//! [`is_building_storey_name`] before attaching labels to elevations.

use std::collections::BTreeSet;

/// Coarse buckets for partition display-name candidates.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum NameBucket {
    MaterialLike,
    LevelLike,
    SpaceLike,
}

/// True when `s` looks like a human-facing display name rather than a
/// Forge URI, Uniformat/OmniClass code, or asset path.
pub fn is_display_name(s: &str) -> bool {
    let t = s.trim();
    if t.len() < 2 || t.len() > 80 {
        return false;
    }
    if t.starts_with("autodesk.")
        || t.starts_with("http://")
        || t.starts_with("https://")
        || t.contains("://")
        || t.starts_with("Uniformat")
        || t.starts_with("OmniClass")
    {
        return false;
    }
    let hexish: String = t.chars().filter(|c| c.is_ascii_hexdigit()).collect();
    if hexish.len() >= 32 {
        return false;
    }
    if !t.chars().any(|c| c.is_alphabetic()) {
        return false;
    }
    if t.chars()
        .all(|c| c.is_ascii_digit() || " .-_/\\'\"".contains(c))
    {
        return false;
    }
    true
}

/// Classify a single string into a name bucket, or `None` when it is
/// not a usable display-name candidate.
pub fn classify_name(s: &str) -> Option<NameBucket> {
    if !is_display_name(s) {
        return None;
    }
    let lower = s.to_ascii_lowercase();
    if lower.contains("concrete")
        || lower.contains("gypsum")
        || lower.contains("glass")
        || lower.contains("steel")
        || lower.contains("aluminum")
        || lower.contains("aluminium")
        || lower.contains("brick")
        || lower.contains("insulation")
        || lower.contains("hardwood")
        || lower.contains("masonry")
        || (lower.contains("wood") && !lower.contains("hardwoodschema"))
    {
        if lower.contains("mats/") || lower.contains("mats\\") || lower.ends_with(".xml") {
            return None;
        }
        return Some(NameBucket::MaterialLike);
    }
    if lower.starts_with("level ")
        || lower == "level 1"
        || lower == "ground floor"
        || lower == "roof"
        || lower.contains("first floor")
        || lower.contains("second floor")
    {
        return Some(NameBucket::LevelLike);
    }
    if (lower.contains("conference")
        || lower.contains("corridor")
        || lower.contains("lobby")
        || lower.contains("office")
        || lower.contains("classroom")
        || lower.starts_with("space "))
        && s.len() <= 48
    {
        return Some(NameBucket::SpaceLike);
    }
    None
}

/// Collect unique `(bucket, name)` pairs from an iterator of strings.
pub fn collect_name_candidates<'a, I>(strings: I) -> BTreeSet<(NameBucket, String)>
where
    I: IntoIterator<Item = &'a str>,
{
    let mut out = BTreeSet::new();
    for s in strings {
        if let Some(bucket) = classify_name(s) {
            out.insert((bucket, s.trim().to_string()));
        }
    }
    out
}

/// Stricter filter for names that can label an `IfcBuildingStorey`.
///
/// Drops view/family noise that the broad [`NameBucket::LevelLike`]
/// bucket still admits (e.g. `"Level Head - Upgrade"`, layout views).
pub fn is_building_storey_name(s: &str) -> bool {
    let lower = s.trim().to_ascii_lowercase();
    if lower.is_empty() {
        return false;
    }
    if lower.contains("head")
        || lower.contains("layout")
        || lower.contains("upgrade")
        || lower.contains("view")
        || lower.contains("plan")
    {
        return false;
    }
    if matches!(
        lower.as_str(),
        "ground floor" | "groundfloor" | "roof" | "mezzanine" | "basement" | "podium"
    ) {
        return true;
    }
    if lower == "first floor" || lower == "second floor" || lower == "third floor" {
        return true;
    }
    if let Some(rest) = lower.strip_prefix("level ") {
        // Accept pure "Level N" only — not "Level 3 - Wall Layouts 1".
        return !rest.is_empty()
            && rest
                .chars()
                .all(|c| c.is_ascii_digit() || c.is_whitespace());
    }
    false
}

/// Level-like partition strings filtered to building-storey labels.
pub fn building_storey_name_candidates<'a, I>(strings: I) -> Vec<String>
where
    I: IntoIterator<Item = &'a str>,
{
    let mut names: Vec<String> = collect_name_candidates(strings)
        .into_iter()
        .filter(|(bucket, name)| *bucket == NameBucket::LevelLike && is_building_storey_name(name))
        .map(|(_, name)| name)
        .collect();
    names.sort();
    names.dedup();
    names
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_known_material_and_level_names() {
        assert_eq!(classify_name("Concrete"), Some(NameBucket::MaterialLike));
        assert_eq!(classify_name("Level 1"), Some(NameBucket::LevelLike));
        assert_eq!(classify_name("Lobby"), Some(NameBucket::SpaceLike));
    }

    #[test]
    fn rejects_forge_and_asset_paths() {
        assert!(classify_name("autodesk.unit.unit:meters-1.0.0").is_none());
        assert!(classify_name("Mats/Hardwood/Generic.xml").is_none());
    }

    #[test]
    fn building_storey_filter_keeps_level_and_roof() {
        assert!(is_building_storey_name("Level 1"));
        assert!(is_building_storey_name("Roof"));
        assert!(is_building_storey_name("Ground floor"));
        assert!(!is_building_storey_name("Level Head - Upgrade"));
        assert!(!is_building_storey_name("Level 3 - Wall Layouts 1"));
    }
}
