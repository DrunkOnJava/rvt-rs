//! #1841: which framing instances Revit's export writes as `IfcMember`.
//!
//! FACT under test: Autodesk's exporter (revit-ifc,
//! `ExporterUtil.GetSpecificCategoryForElement`) classes a framing instance
//! by a sub-category of its structural usage: a brace by the bracing
//! category, which Revit's class table maps to `IfcMember`. The usage is
//! the family instance's `m_structUsage` (`InsertableInst`, a 4-byte integer
//! in the schema). What each value means is read here from the files, not
//! from the API's enumeration: the probe counts each value against the class
//! Revit's own export gives the same elements.
//!
//! The probe:
//! 1. Exports the model as `rvt-ifc` does with no flags, and takes the
//!    elements it writes as `IFCBEAM` or `IFCMEMBER` with a numeric `Tag`.
//! 2. Reads each one's native record and its `m_structUsage`.
//! 3. With Revit's IFC export of the same file, finds the class Revit gives
//!    each `Tag`.
//!
//! Output (counts only: no name, no ElementId):
//!   framing written=<n> read=<n> without_usage=<n>
//!   usage=<value> count=<n> rvt-rs <CLASS>=<n> ... [revit <CLASS>=<n> ... absent=<n>]
//!
//! Usage:
//!   cargo run --profile ci -p rvt-probes --example probe_1841_framing_usage -- MODEL.rvt [REVIT.ifc]

use std::collections::{BTreeMap, BTreeSet};

use rvt::RevitFile;
use rvt::ifc::entities::IfcEntity;
use rvt::research::native_document::{NATIVE_RELEASES, Options, extract_graphs};

fn open(path: &str) -> anyhow::Result<RevitFile> {
    Ok(RevitFile::open_with_limits(
        path,
        rvt::cli::open_limits(None, None, false),
    )?)
}

/// The attributes of a STEP entity's argument list, split at its top-level
/// commas.
fn attributes(args: &str) -> Vec<&str> {
    let (mut out, mut depth, mut quoted, mut start) = (Vec::new(), 0usize, false, 0usize);
    for (at, ch) in args.char_indices() {
        match ch {
            '\'' => quoted = !quoted,
            '(' if !quoted => depth += 1,
            ')' if !quoted => depth = depth.saturating_sub(1),
            ',' if !quoted && depth == 0 => {
                out.push(args[start..at].trim());
                start = at + 1;
            }
            _ => {}
        }
    }
    out.push(args[start..].trim());
    out
}

/// The class Revit's export gives each `Tag` of an element: the eighth
/// attribute of every IfcElement subtype (GlobalId, OwnerHistory, Name,
/// Description, ObjectType, ObjectPlacement, Representation, Tag).
fn revit_classes(path: &str) -> anyhow::Result<BTreeMap<String, String>> {
    let text = std::fs::read_to_string(path)?;
    let mut out = BTreeMap::new();
    for line in text.lines() {
        let Some(rest) = line.trim().strip_prefix('#') else {
            continue;
        };
        let Some((_, entity)) = rest.split_once('=') else {
            continue;
        };
        let entity = entity.trim();
        let Some(open) = entity.find('(') else {
            continue;
        };
        let class = &entity[..open];
        if !matches!(
            class,
            "IFCBEAM" | "IFCMEMBER" | "IFCCOLUMN" | "IFCPLATE" | "IFCBUILDINGELEMENTPROXY"
        ) {
            continue;
        }
        let args = entity[open + 1..]
            .trim_end_matches(';')
            .trim_end_matches(')');
        if let Some(tag) = attributes(args).get(7) {
            let tag = tag.trim_matches('\'');
            if tag.chars().all(|c| c.is_ascii_digit()) && !tag.is_empty() {
                out.insert(tag.to_string(), class.to_string());
            }
        }
    }
    Ok(out)
}

fn main() -> anyhow::Result<()> {
    // Measure passes a single sample `--records` too.
    let args: Vec<String> = std::env::args()
        .skip(1)
        .filter(|arg| !arg.starts_with("--"))
        .collect();
    let Some(model) = args.first() else {
        anyhow::bail!("usage: probe_1841_framing_usage MODEL.rvt [REVIT.ifc]");
    };
    let revit = args.get(1).map(|path| revit_classes(path)).transpose()?;

    let run = rvt::ifc::run_export(&mut open(model)?, &rvt::ifc::ExportOptions::default())?;
    let written: BTreeMap<u64, String> = run
        .model
        .entities
        .iter()
        .filter_map(|entity| match entity {
            IfcEntity::BuildingElement {
                ifc_type,
                type_guid: Some(tag),
                ..
            } if matches!(ifc_type.as_str(), "IFCBEAM" | "IFCMEMBER") => {
                tag.parse::<u64>().ok().map(|id| (id, ifc_type.clone()))
            }
            _ => None,
        })
        .collect();

    let options = Options {
        selected_ids: written.keys().copied().collect::<BTreeSet<u64>>(),
        releases: NATIVE_RELEASES,
        ..Options::default()
    };
    let mut usage: BTreeMap<u64, Option<u64>> = BTreeMap::new();
    extract_graphs(&mut open(model)?, &options, |record| {
        let value = record
            .graph
            .as_ref()
            .and_then(|graph| graph.objects.first())
            .and_then(|root| root.fields.get("m_structUsage"))
            .and_then(|value| value.as_u64());
        usage.insert(record.identity.element_id, value);
        Ok(())
    })?;

    let without = usage.values().filter(|value| value.is_none()).count();
    println!(
        "framing written={} read={} without_usage={}",
        written.len(),
        usage.len(),
        without
    );
    // By usage value: rvt-rs's class, and Revit's where its export is given.
    type Counts = BTreeMap<String, usize>;
    let mut by_value: BTreeMap<Option<u64>, (usize, Counts, Counts)> = BTreeMap::new();
    for (id, class) in &written {
        let value = usage.get(id).copied().flatten();
        let entry = by_value.entry(value).or_default();
        entry.0 += 1;
        *entry.1.entry(class.clone()).or_default() += 1;
        if let Some(revit) = &revit {
            let theirs = revit
                .get(&id.to_string())
                .cloned()
                .unwrap_or_else(|| "absent".into());
            *entry.2.entry(theirs).or_default() += 1;
        }
    }
    let join = |counts: &Counts| {
        counts
            .iter()
            .map(|(class, n)| format!("{class}={n}"))
            .collect::<Vec<_>>()
            .join(" ")
    };
    for (value, (count, ours, theirs)) in &by_value {
        let value = value.map_or_else(|| "none".to_string(), |v| v.to_string());
        let revit_part = if revit.is_some() {
            format!(" revit {}", join(theirs))
        } else {
            String::new()
        };
        println!(
            "usage={value} count={count} rvt-rs {}{revit_part}",
            join(ours)
        );
    }
    Ok(())
}
