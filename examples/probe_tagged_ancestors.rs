//! RE-18 — schema tagged-ancestor walker. Given a parsed schema,
//! for every untagged class resolve the first tag-carrying ancestor
//! by walking the `parent` chain.
//!
//! Why this matters (H7, confidence 0.7, from RE-09):
//!   - Schema has 405 classes but only 80 are directly tagged.
//!   - `Wall`, `Floor`, `Door`, `Window`, `Level`, `Grid`,
//!     `FamilyInstance`, `Room` are all tagless abstract parents.
//!   - Their instances on the wire presumably carry a concrete
//!     subtype tag (e.g. `ArcWall` 0x0191, `VWall` 0x0192, or a
//!     generic ancestor like `HostObjAttr` 0x006b).
//!   - RE-11's tag scan needs the full set of "tag worth scanning
//!     for if I care about Walls" — that's the ancestor map.
//!
//! Output:
//!   - Counts (total / directly-tagged / resolved-via-ancestor /
//!     unresolvable).
//!   - Per "interesting class" the ancestor + tag resolution.
//!   - Distribution of ancestor depths (how many parent hops to
//!     reach a tag).
//!   - Sample of unresolvable classes (no tag in chain) — these are
//!     likely mixins, abstract protocols, or classes whose parents
//!     live in a different serializable scope.

use rvt::{RevitFile, compression, formats, streams};
use std::collections::BTreeMap;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let project_dir = std::env::var("RVT_PROJECT_CORPUS_DIR")
        .unwrap_or_else(|_| "/private/tmp/rvt-corpus-probe/magnetar/Revit".into());
    let targets: Vec<String> = if args.is_empty() {
        vec![
            format!("{project_dir}/Revit_IFC5_Einhoven.rvt"),
            format!("{project_dir}/2024_Core_Interior.rvt"),
            "../../samples/racbasicsamplefamily-2024.rfa".to_string(),
            "../../samples/racbasicsamplefamily-2026.rfa".to_string(),
        ]
    } else {
        args
    };

    for path in targets {
        if !std::path::Path::new(&path).exists() {
            continue;
        }
        let Ok(mut rf) = RevitFile::open(&path) else {
            continue;
        };
        let Ok(raw) = rf.read_stream(streams::FORMATS_LATEST) else {
            continue;
        };
        let Ok(decomp) = compression::inflate_at(&raw, 0) else {
            continue;
        };
        let Ok(schema) = formats::parse_schema(&decomp) else {
            continue;
        };

        let fname = std::path::Path::new(&path)
            .file_name()
            .unwrap()
            .to_string_lossy();
        println!("\n=== {fname} ===");

        let total = schema.classes.len();
        let direct_tag = schema.classes.iter().filter(|c| c.tag.is_some()).count();
        let untagged = total - direct_tag;

        // Resolve every untagged class through the ancestor walker.
        let mut resolved_via_ancestor = 0usize;
        let mut unresolvable: Vec<&str> = Vec::new();
        let mut depth_distribution: BTreeMap<usize, usize> = BTreeMap::new();
        let mut ancestor_popularity: BTreeMap<&str, usize> = BTreeMap::new();

        for c in &schema.classes {
            if c.tag.is_some() {
                continue;
            }
            // Walk parent chain manually to get depth count. Using the
            // method gives the answer; we redo the walk here so we can
            // count hops.
            let mut depth = 0usize;
            let mut current = c.name.as_str();
            let mut found: Option<(&str, u16)> = None;
            while let Some(entry) = schema.classes.iter().find(|x| x.name == current) {
                if let Some(t) = entry.tag {
                    found = Some((entry.name.as_str(), t));
                    break;
                }
                match entry.parent.as_deref() {
                    Some(p) => {
                        current = p;
                        depth += 1;
                    }
                    None => break,
                }
                // Guard against pathological chains.
                if depth > 40 {
                    break;
                }
            }
            match found {
                Some((anc, _)) => {
                    resolved_via_ancestor += 1;
                    *depth_distribution.entry(depth).or_insert(0) += 1;
                    *ancestor_popularity.entry(anc).or_insert(0) += 1;
                }
                None => unresolvable.push(&c.name),
            }
        }

        println!("  Schema: {total} classes ({direct_tag} directly tagged, {untagged} untagged)");
        println!(
            "  Untagged: {resolved_via_ancestor} resolved via ancestor chain, \
             {} unresolvable",
            unresolvable.len()
        );
        println!(
            "  Coverage: {} of {total} total classes now resolvable ({:.1}%)",
            direct_tag + resolved_via_ancestor,
            100.0 * (direct_tag + resolved_via_ancestor) as f64 / total as f64,
        );

        println!("\n  Ancestor hop-count distribution (untagged classes):");
        for (depth, count) in &depth_distribution {
            println!("    {depth:>2} hops: {count} classes");
        }

        println!("\n  Top-10 most-popular tagged ancestors:");
        let mut popular: Vec<(&&str, &usize)> = ancestor_popularity.iter().collect();
        popular.sort_by_key(|(_, count)| std::cmp::Reverse(**count));
        for (anc, count) in popular.iter().take(10) {
            let tag = schema
                .classes
                .iter()
                .find(|c| &c.name == *anc)
                .and_then(|c| c.tag)
                .unwrap_or(0);
            println!("    {count:>4} classes → {anc} (tag 0x{tag:04x})");
        }

        // Interesting-class resolution: what tag does each key
        // architectural class resolve to?
        let interesting = [
            "Wall",
            "Floor",
            "Door",
            "Window",
            "Stair",
            "Column",
            "Beam",
            "Roof",
            "Ceiling",
            "Level",
            "Grid",
            "FamilyInstance",
            "Room",
            "HostObject",
            "HostObjAttr",
            "Element",
        ];
        println!("\n  Interesting-class resolution:");
        for name in interesting {
            match schema.tagged_ancestor(name) {
                Some((anc, tag)) if anc == name => {
                    println!("    {name:<18} direct tag 0x{tag:04x}");
                }
                Some((anc, tag)) => {
                    println!("    {name:<18} → {anc} (tag 0x{tag:04x})");
                }
                None => {
                    let exists = schema.classes.iter().any(|c| c.name == name);
                    if exists {
                        println!("    {name:<18} UNRESOLVABLE (in schema, no tagged ancestor)");
                    } else {
                        println!("    {name:<18} not in schema");
                    }
                }
            }
        }

        // Sample unresolvable untagged classes
        if !unresolvable.is_empty() {
            println!(
                "\n  Sample unresolvable untagged classes (first 8 of {}):",
                unresolvable.len()
            );
            for name in unresolvable.iter().take(8) {
                println!("    {name}");
            }
        }
    }
}
