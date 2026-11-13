//! #1854: every GlobalId an IFC export writes, by entity class, across two
//! saves of one project and between two projects.
//!
//! FACT under test: a GlobalId the writer derives for an entity that belongs
//! to another (an opening, a property set, a relationship, a port) is seeded
//! from that owner's written GlobalId, so it holds between two saves wherever
//! the owner's does; and it is shared between two projects only where the
//! owner's is. Before #1854 every such GlobalId was seeded from the
//! document's `Unique Document GUID`, which differs on every save.
//!
//! The probe exports each file as `rvt-ifc` does by default, writes the STEP
//! to memory and reads each entity whose first attribute is a GlobalId:
//! - one file: per class, how many, and how many GlobalIds occur twice;
//! - two (Measure's `probe_pair_release`: a sample, then the same project
//!   saved by another release): per class, how many GlobalIds both saves
//!   write, and how many only one does;
//! - a `rac_basic` sample also against the `rac_advanced` sample of its year
//!   beside it, if present: per class, how many GlobalIds the two projects
//!   share (both start from Autodesk's template).
//!
//! Output, counts only, no names or GlobalIds:
//!   file <side> entities=<n> duplicates=<n>
//!   class <IFCCLASS> both=<n> left_only=<n> right_only=<n>
//!   projects shared=<n> <IFCCLASS>=<n> ...
//!
//! Usage:
//!   cargo run --profile ci -p rvt-probes --example probe_1854_global_id_census -- A.rvt [B.rvt]

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use rvt::RevitFile;

/// Each GlobalId the STEP of `path`'s default export writes, with its class,
/// and how many GlobalIds it writes twice.
fn global_ids(path: &Path) -> anyhow::Result<(BTreeMap<String, String>, usize)> {
    let mut rf = RevitFile::open_with_limits(path, rvt::cli::open_limits(None, None, false))?;
    let run = rvt::ifc::run_export(&mut rf, &rvt::ifc::ExportOptions::default())?;
    drop(rf);
    let options = rvt::ifc::step_writer::StepOptions { timestamp: Some(0) };
    let step = rvt::ifc::step_writer::write_step_with_options(&run.model, &options);
    let mut ids = BTreeMap::new();
    let mut duplicates = 0;
    for line in step.lines() {
        // `#12=IFCWALL('2O2Fr$t4X7Zf8NOew3FLOH',...`
        let Some(rest) = line.strip_prefix('#') else {
            continue;
        };
        let Some((_, entity)) = rest.split_once('=') else {
            continue;
        };
        let Some((class, attributes)) = entity.split_once('(') else {
            continue;
        };
        let Some(gid) = attributes
            .strip_prefix('\'')
            .and_then(|quoted| quoted.get(..23))
            .and_then(|quoted| quoted.strip_suffix('\''))
        else {
            continue;
        };
        if gid.len() != 22 {
            continue;
        }
        if ids.insert(gid.to_string(), class.to_string()).is_some() {
            duplicates += 1;
        }
    }
    Ok((ids, duplicates))
}

fn by_class<'a>(ids: impl Iterator<Item = &'a String>) -> BTreeMap<&'a str, usize> {
    let mut counts = BTreeMap::new();
    for class in ids {
        *counts.entry(class.as_str()).or_default() += 1;
    }
    counts
}

fn main() -> anyhow::Result<()> {
    // Measure passes a single sample `--records` too.
    let files: Vec<String> = std::env::args()
        .skip(1)
        .filter(|arg| !arg.starts_with("--"))
        .collect();
    let Some(first) = files.first() else {
        anyhow::bail!("usage: probe_1854_global_id_census A.rvt [B.rvt]");
    };
    let (left, left_duplicates) = global_ids(Path::new(first))?;
    println!(
        "file left entities={} duplicates={left_duplicates}",
        left.len()
    );

    if let Some(second) = files.get(1) {
        let (right, right_duplicates) = global_ids(Path::new(second))?;
        println!(
            "file right entities={} duplicates={right_duplicates}",
            right.len()
        );
        let classes: BTreeSet<&str> = left
            .values()
            .chain(right.values())
            .map(String::as_str)
            .collect();
        for class in classes {
            let in_left: BTreeSet<&String> = left
                .iter()
                .filter(|(_, c)| *c == class)
                .map(|(g, _)| g)
                .collect();
            let in_right: BTreeSet<&String> = right
                .iter()
                .filter(|(_, c)| *c == class)
                .map(|(g, _)| g)
                .collect();
            let both = in_left.intersection(&in_right).count();
            println!(
                "class {class} both={both} left_only={} right_only={}",
                in_left.len() - both,
                in_right.len() - both
            );
        }
    } else {
        for (class, count) in by_class(left.values()) {
            println!("class {class} count={count}");
        }
    }

    // Two projects from one template: what they share.
    let name = Path::new(first)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default();
    if name.contains("racbasic") {
        let sibling = Path::new(first).with_file_name(name.replace("racbasic", "racadvanced"));
        if sibling.exists() {
            let (other, _) = global_ids(&sibling)?;
            let shared = by_class(
                left.iter()
                    .filter(|(gid, class)| other.get(*gid) == Some(*class))
                    .map(|(_, class)| class),
            );
            let total: usize = shared.values().sum();
            let listed: Vec<String> = shared
                .iter()
                .map(|(class, n)| format!("{class}={n}"))
                .collect();
            println!("projects shared={total} {}", listed.join(" "));
        }
    }
    Ok(())
}
