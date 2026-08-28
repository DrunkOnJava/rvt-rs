//! Partition UTF-16LE display-name candidates for RE-15-06 (#86).
use std::collections::BTreeSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum NameBucket {
    MaterialLike,
    LevelLike,
    SpaceLike,
}

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
    if t.chars().all(|c| c.is_ascii_digit() || " .-_/\\'\"".contains(c)) {
        return false;
    }
    true
}

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
}
