//! RE-79: shared nested MEP components are element records Revit's export
//! leaves out (#96).
//!
//! FACT: in the lighting-device, electrical-fixture and electrical-equipment
//! categories, a shared nested family instance (the symbol a switch or
//! receptacle family nests) is a placed element record of its own, in its
//! parent's category. The parent and the nested instance each list the other
//! in their first reference list, and the nested one has the larger
//! ElementId. Revit's own export writes the parent and leaves the nested one
//! out.
//!
//! Verify on RE1 Electrical (Revit 2025, `Drshelden/IFC-ECS`, MIT), whose
//! IFC export sits next to it:
//!
//! ```text
//! cargo run --profile ci --example probe_re79_nested_mep_components -- \
//!     RE1-Electrical.rvt RE1-Electrical.ifc
//! ```
//!
//! It prints one line per mutual pair with whether each side is a `Tag` of
//! the export; on RE1 Electrical all 21 pairs are "parent exported, nested
//! not exported".

use std::collections::{BTreeMap, BTreeSet};

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let (Some(rvt), Some(ifc)) = (args.get(1), args.get(2)) else {
        anyhow::bail!("usage: probe_re79_nested_mep_components FILE.rvt EXPORT.ifc");
    };
    // Every quoted all-digit STEP string: the Tags, plus a few names that
    // cannot collide with an ElementId of a placed device.
    let text = std::fs::read_to_string(ifc)?;
    let tags: BTreeSet<u32> = text
        .split('\'')
        .skip(1)
        .step_by(2)
        .filter(|s| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit()))
        .filter_map(|s| s.parse().ok())
        .collect();
    let mut rf = rvt::RevitFile::open(rvt)?;
    let version = rf.basic_file_info()?.version;
    let declared = rvt::elem_table::declared_ids(&rvt::elem_table::parse_records(&mut rf)?);
    let categories = rvt::partition_element_records::NESTED_COMPONENT_CATEGORIES;
    let records = rvt::partition_element_records::scan_category_records_multi(
        &mut rf,
        version,
        &categories,
        &declared,
    )?;
    let placed: BTreeMap<u32, _> = records
        .iter()
        .filter(|r| r.is_exported_instance())
        .map(|r| (r.element_id, r))
        .collect();
    let (mut pairs, mut as_revit) = (0, 0);
    for child in placed.values() {
        for parent in child
            .references
            .iter()
            .filter_map(|&r| u32::try_from(r).ok())
            .filter(|&p| p < child.element_id)
            .filter_map(|p| placed.get(&p))
            .filter(|p| {
                p.builtin_category == child.builtin_category
                    && p.references.contains(&u64::from(child.element_id))
            })
        {
            pairs += 1;
            let (p, c) = (
                tags.contains(&parent.element_id),
                tags.contains(&child.element_id),
            );
            if p && !c {
                as_revit += 1;
            }
            println!(
                "category {} parent {} ({}) nested {} ({})",
                child.builtin_category,
                parent.element_id,
                if p { "exported" } else { "not exported" },
                child.element_id,
                if c { "exported" } else { "not exported" },
            );
        }
    }
    println!("{pairs} mutual pairs, {as_revit} with the parent exported and the nested one not");
    Ok(())
}
