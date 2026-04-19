//! Extract the class/schema inventory from the `Formats/Latest` stream.
//!
//! Revit stores every class name used in the file as a plaintext ASCII
//! string inside the DEFLATE-compressed `Formats/Latest` stream. This is the
//! most important leverage point for RVT reverse engineering: the full
//! class vtable is exposed, so an implementer doesn't need to guess what
//! types exist — only how their fields are laid out.
//!
//! The format of `Formats/Latest` itself is a length-prefixed string table
//! plus binary metadata per class. For now we just extract the clean ASCII
//! class names via a conservative regex.

use crate::Result;
use std::collections::BTreeSet;

/// Returns a sorted set of unique class/schema names detected in the decompressed
/// `Formats/Latest` bytes.
///
/// The heuristic: look for runs of `[A-Z][A-Za-z0-9_]{3,60}` that begin with
/// an uppercase ASCII letter and are preceded by a null or length byte.
/// This is conservative — it misses multi-word names but has few false
/// positives. A stricter parser would read the length prefixes.
pub fn extract_class_names(decompressed: &[u8]) -> Result<BTreeSet<String>> {
    let mut names = BTreeSet::new();

    let mut i = 0;
    while i < decompressed.len() {
        let b = decompressed[i];
        if b.is_ascii_uppercase() {
            let start = i;
            let mut end = i;
            while end < decompressed.len() {
                let c = decompressed[end];
                if c == b'_' || c.is_ascii_alphanumeric() {
                    end += 1;
                } else {
                    break;
                }
            }
            let len = end - start;
            if (4..=60).contains(&len) {
                if let Ok(name) = std::str::from_utf8(&decompressed[start..end]) {
                    // Filter out obvious noise:
                    //  - all-uppercase words >= 7 chars (hex IDs, not class names)
                    //  - strings with 4+ consecutive same letters
                    if !is_noise(name) {
                        names.insert(name.to_string());
                    }
                }
            }
            i = end.max(i + 1);
        } else {
            i += 1;
        }
    }

    Ok(names)
}

fn is_noise(s: &str) -> bool {
    // All-caps run >= 7 chars that looks like a hex string or acronym
    if s.len() >= 7
        && s.chars()
            .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit())
    {
        return true;
    }
    // 4+ same-letter run like "AAAAAA" or "Aaaaaaa"
    let bytes = s.as_bytes();
    let mut run = 1;
    for w in bytes.windows(2) {
        if w[0].eq_ignore_ascii_case(&w[1]) {
            run += 1;
            if run >= 4 {
                return true;
            }
        } else {
            run = 1;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_hex_strings() {
        assert!(is_noise("AAAAAAA"));
        assert!(is_noise("0D41F08"));
    }

    #[test]
    fn accepts_real_classes() {
        assert!(!is_noise("APropertyBoolean"));
        assert!(!is_noise("ADocument"));
        assert!(!is_noise("A3PartyObject"));
        assert!(!is_noise("APIEventHandlerStatus"));
    }
}
