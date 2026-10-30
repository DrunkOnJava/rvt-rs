//! RE-187: which of a door's or window's type values Revit's IFC export
//! writes as its `OverallWidth` and `OverallHeight` (#759).
//!
//! rvt-rs writes those two attributes only from the opening it cuts (RE-93
//! for windows, RE-94 for doors), so a door or window whose cut is not read
//! gets `$`, even where its type's size can be read. On Core Interior and
//! RE1 that is all 143 of them. The probe prints, for every door and window
//! the element records give, the candidate sizes its type holds in its value
//! block (`f64 feet · ff × 8 · i64 parameter`, RE-77):
//!
//! - `width` and `height`: the BuiltInParameters -1001301 and -1001300, as
//!   RE-93 reads them for windows;
//! - `rough_width` and `rough_height`: -1001305 and -1001304, as RE-94 reads
//!   them for doors;
//! - `family_width` and `family_height`: family parameters named `Width` and
//!   `Height`, matched by name as RE-93 matches Default Sill Height;
//!
//! with the element's record box. `tools/re/door_window_sizes_vs_ifc.py`
//! scores each candidate against Revit's own export by `Tag`.
//!
//! Usage:
//!   cargo run --profile ci -p rvt-probes --example probe_re187_door_window_sizes -- MODEL.rvt

use rvt::RevitFile;
use rvt::research::partition_schema_mvp::{TYPE_ID_FIELD, recover_partition_schema_mvp};
use rvt::research::partition_type_parameters::{
    DOOR_ROUGH_HEIGHT_PARAMETER, DOOR_ROUGH_WIDTH_PARAMETER, WINDOW_HEIGHT_PARAMETER,
    WINDOW_WIDTH_PARAMETER, family_parameters_named, scan_partition_lengths,
};
use rvt::walker::{DecodedElement, InstanceField, WalkerLimits};
use serde_json::json;
use std::collections::{BTreeMap, BTreeSet};

fn float(element: &DecodedElement, wanted: &str) -> Option<f64> {
    element.fields.iter().find_map(|(name, value)| match value {
        InstanceField::Float { value, .. } if name == wanted => Some(*value),
        _ => None,
    })
}

fn type_of(element: &DecodedElement) -> Option<u32> {
    element.fields.iter().find_map(|(name, value)| match value {
        InstanceField::ElementId { id, .. } if name == TYPE_ID_FIELD => Some(*id),
        _ => None,
    })
}

fn main() -> rvt::Result<()> {
    let path = std::env::args().nth(1).expect("usage: MODEL.rvt");
    let mut rf = RevitFile::open(&path)?;
    let version = rf.basic_file_info()?.version;
    let mvp = recover_partition_schema_mvp(&mut rf, version, WalkerLimits::default())?;
    let elements: Vec<(&str, &DecodedElement)> = mvp
        .doors
        .iter()
        .map(|e| ("Door", e))
        .chain(mvp.windows.iter().map(|e| ("Window", e)))
        .collect();
    let types: BTreeSet<u32> = elements.iter().filter_map(|(_, e)| type_of(e)).collect();
    let declared = rvt::elem_table::declared_ids(&rvt::elem_table::parse_records(&mut rf)?);

    let streams = rf.partition_stream_names();
    let mut family_width: BTreeSet<i64> = BTreeSet::new();
    let mut family_height: BTreeSet<i64> = BTreeSet::new();
    for stream in &streams {
        if let Ok(inflated) = rf.inflated_partition(stream) {
            let buf = inflated.bytes();
            family_width.extend(
                family_parameters_named(buf, &declared, "Width")
                    .into_iter()
                    .map(i64::from),
            );
            family_height.extend(
                family_parameters_named(buf, &declared, "Height")
                    .into_iter()
                    .map(i64::from),
            );
        }
    }
    let mut parameters: BTreeSet<i64> = BTreeSet::from([
        WINDOW_WIDTH_PARAMETER,
        WINDOW_HEIGHT_PARAMETER,
        DOOR_ROUGH_WIDTH_PARAMETER,
        DOOR_ROUGH_HEIGHT_PARAMETER,
    ]);
    parameters.extend(family_width.iter().copied());
    parameters.extend(family_height.iter().copied());

    // A type's value for a parameter counts only when every partition that
    // holds it agrees, as `partition_type_parameters` reads type lengths.
    let mut held: BTreeMap<(u32, i64), Option<f64>> = BTreeMap::new();
    for stream in &streams {
        let Ok(inflated) = rf.inflated_partition(stream) else {
            continue;
        };
        for (owner, found) in scan_partition_lengths(inflated.bytes(), &declared, &parameters) {
            if !types.contains(&owner) {
                continue;
            }
            for (parameter, value) in found {
                held.entry((owner, parameter))
                    .and_modify(|seen| {
                        if *seen != Some(value) {
                            *seen = None;
                        }
                    })
                    .or_insert(Some(value));
            }
        }
    }
    let value = |owner: u32, parameter: i64| held.get(&(owner, parameter)).copied().flatten();
    let named = |owner: u32, ids: &BTreeSet<i64>| {
        let mut values = ids.iter().filter_map(|&id| value(owner, id));
        let first = values.next()?;
        values.all(|v| v == first).then_some(first)
    };

    for (class, element) in &elements {
        let type_id = type_of(element);
        let candidate = |parameter: i64| type_id.and_then(|t| value(t, parameter));
        println!(
            "{}",
            json!({
                "file": path,
                "release": version,
                "id": element.id,
                "class": class,
                "type": type_id,
                "width": candidate(WINDOW_WIDTH_PARAMETER),
                "height": candidate(WINDOW_HEIGHT_PARAMETER),
                "rough_width": candidate(DOOR_ROUGH_WIDTH_PARAMETER),
                "rough_height": candidate(DOOR_ROUGH_HEIGHT_PARAMETER),
                "family_width": type_id.and_then(|t| named(t, &family_width)),
                "family_height": type_id.and_then(|t| named(t, &family_height)),
                "box": [
                    float(element, "m_bboxWidth"),
                    float(element, "m_bboxDepth"),
                    float(element, "m_bboxHeight"),
                ],
            })
        );
    }
    Ok(())
}
