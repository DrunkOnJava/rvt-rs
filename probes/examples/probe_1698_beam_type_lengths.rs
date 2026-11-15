//! #1698: where is the profile of a beam drawn as a hollow tube?
//!
//! FACT under test: Revit's export draws the BIM4LCA office's 8 beams of type
//! `Putkipalkki - suorakaide:PA-1` as an extrusion of a 200 mm by 300 mm tube
//! with a 4 mm wall, its outer corners rounded 8 mm and its inner ones 4 mm
//! (`IfcArbitraryProfileDefWithVoids`), where rvt-rs draws a solid box
//! (`partition_beam_axis`). The columns of three hollow-section types of the
//! same office are already drawn from their type's family parameters, captioned
//! `b`, `Ht`, `t` and `kr` (`type_hollow_sections`, W6). If the beam type holds
//! the same four captions, the same reader gives the beam's profile and the six
//! base quantities follow from it (#1698, RE-442).
//!
//! The probe prints, for every type a structural-framing element names, its
//! family and type name, how many beams use it, every length parameter the type
//! holds by caption (`type_named_lengths`), and the hollow section those make
//! (`hollow_section`) or `-` where they make none. Lengths are in feet.
//!
//! Output (one model per run):
//!   type <type id> <family>:<type name> beams <n>
//!     lengths <caption>=<feet> ...
//!     hollow <width> <height> <wall> <outer radius> | -
//!
//! ```text
//! cargo run --profile ci -p rvt-probes --example probe_1698_beam_type_lengths -- model.rvt
//! ```

use std::collections::{BTreeMap, BTreeSet};

use rvt::RevitFile;
use rvt::research::partition_schema_mvp::{
    FAMILY_NAME_FIELD, TYPE_ID_FIELD, TYPE_NAME_FIELD, recover_partition_schema_mvp,
};
use rvt::research::partition_type_parameters::{hollow_section, type_named_lengths};
use rvt::walker::{DecodedElement, InstanceField, WalkerLimits};

fn text(element: &DecodedElement, wanted: &str) -> Option<String> {
    element.fields.iter().find_map(|(name, value)| match value {
        InstanceField::String(text) if name == wanted => Some(text.clone()),
        _ => None,
    })
}

fn type_of(element: &DecodedElement) -> Option<u32> {
    element.fields.iter().find_map(|(name, value)| match value {
        InstanceField::ElementId { id, .. } if name == TYPE_ID_FIELD => Some(*id),
        _ => None,
    })
}

fn main() -> anyhow::Result<()> {
    let Some(path) = std::env::args().nth(1) else {
        anyhow::bail!("usage: probe_1698_beam_type_lengths FILE.rvt");
    };
    let mut rf = RevitFile::open(&path)?;
    let version = rf.basic_file_info()?.version;
    let mvp = recover_partition_schema_mvp(&mut rf, version, WalkerLimits::default())?;
    let mut by_type: BTreeMap<u32, (String, usize)> = BTreeMap::new();
    for element in mvp
        .products
        .iter()
        .filter(|element| element.class == "StructuralFraming")
    {
        let Some(type_id) = type_of(element) else {
            continue;
        };
        let family = text(element, FAMILY_NAME_FIELD).unwrap_or_default();
        let name = text(element, TYPE_NAME_FIELD).unwrap_or_default();
        let held = by_type
            .entry(type_id)
            .or_insert_with(|| (format!("{family}:{name}").replace(' ', "_"), 0));
        held.1 += 1;
    }
    let types: BTreeSet<u32> = by_type.keys().copied().collect();
    let lengths = type_named_lengths(&mut rf, version, &types);
    println!(
        "release {version}: {} structural-framing types, {} with named lengths",
        types.len(),
        lengths.len()
    );
    for (type_id, (name, beams)) in &by_type {
        println!("type {type_id} {name} beams {beams}");
        let Some(by_name) = lengths.get(type_id) else {
            println!("  lengths -");
            println!("  hollow -");
            continue;
        };
        let shown: Vec<String> = by_name
            .iter()
            .map(|(caption, feet)| format!("{caption}={feet:.9}"))
            .collect();
        println!("  lengths {}", shown.join(" "));
        match hollow_section(by_name) {
            Some(section) => println!(
                "  hollow {:.9} {:.9} {:.9} {:.9}",
                section.width_feet,
                section.height_feet,
                section.wall_feet,
                section.outer_radius_feet
            ),
            None => println!("  hollow -"),
        }
    }
    Ok(())
}
