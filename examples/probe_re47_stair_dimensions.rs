//! RE-47: a stair's riser height, tread depth and number of risers, and
//! each run's number of risers, are in their serialised data.
//!
//! FACT: on Autodesk's Snowdon Towers 2024 architectural sample, a stair's
//! element data carries, past the bytes
//! `ff ff ff ff fb 03 ff ff ff ff fb 03 00 00 00 00`, its riser height (f64
//! feet, +16), tread depth (f64 feet, +24) and number of risers (u32, +88).
//! These equal the `RiserHeight`, `TreadLength` and `NumberOfRiser` Revit's
//! own IFC4 export writes in `Pset_StairCommon`, on all 26 stairs it gives
//! them. A run's data carries its number of risers past
//! `fc ff ff ff ff ff ff ff 00 00 00 02 00 00 00`. A stair's runs add up to
//! its count, except for a single run, which reads one more.
//!
//! The probe prints, for each placed stair, its dimensions and its runs'
//! counts, as rvt-rs's partition MVP resolves them. Compare them with
//! `Pset_StairCommon` / `Pset_StairFlightCommon` of the same Tags.
//!
//! Usage:
//!   cargo run --profile ci --example probe_re47_stair_dimensions -- FILE.rvt

use rvt::RevitFile;
use rvt::partition_schema_mvp::{
    AGGREGATE_WHOLE_FIELD, STAIR_RISER_COUNT_FIELD, STAIR_RISER_HEIGHT_FIELD,
    STAIR_TREAD_DEPTH_FIELD, recover_partition_schema_mvp,
};
use rvt::walker::{DecodedElement, InstanceField, WalkerLimits};
use std::path::PathBuf;

fn integer(element: &DecodedElement, wanted: &str) -> Option<i64> {
    element.fields.iter().find_map(|(name, value)| match value {
        InstanceField::Integer { value, .. } if name == wanted => Some(*value),
        _ => None,
    })
}

fn feet(element: &DecodedElement, wanted: &str) -> Option<f64> {
    element.fields.iter().find_map(|(name, value)| match value {
        InstanceField::Float { value, .. } if name == wanted => Some(*value),
        _ => None,
    })
}

fn main() -> rvt::Result<()> {
    let path = PathBuf::from(std::env::args().nth(1).expect("usage: FILE.rvt"));
    let mut rf = RevitFile::open(&path)?;
    let version = rf.basic_file_info()?.version;
    let mvp = recover_partition_schema_mvp(&mut rf, version, WalkerLimits::default())?;
    let stairs: Vec<&DecodedElement> = mvp
        .products
        .iter()
        .filter(|element| element.class == "Stair")
        .collect();
    let read = stairs
        .iter()
        .filter(|stair| integer(stair, STAIR_RISER_COUNT_FIELD).is_some())
        .count();
    println!(
        "release {version}: {} stairs, {read} with dimensions",
        stairs.len()
    );
    for stair in stairs {
        let Some(id) = stair.id else {
            continue;
        };
        let Some(risers) = integer(stair, STAIR_RISER_COUNT_FIELD) else {
            println!("  stair {id:>9}: no dimensions");
            continue;
        };
        let riser = feet(stair, STAIR_RISER_HEIGHT_FIELD).unwrap_or(f64::NAN);
        let tread = feet(stair, STAIR_TREAD_DEPTH_FIELD).unwrap_or(f64::NAN);
        let runs: Vec<String> = mvp
            .products
            .iter()
            .filter(|element| element.class == "StairsRun")
            .filter(|run| {
                run.fields.iter().any(|(name, value)| {
                    name == AGGREGATE_WHOLE_FIELD
                        && matches!(value, InstanceField::ElementId { id: whole, .. } if *whole == id)
                })
            })
            .map(|run| {
                let count = integer(run, STAIR_RISER_COUNT_FIELD)
                    .map_or_else(|| "?".to_string(), |n| n.to_string());
                format!("{}: {count}", run.id.unwrap_or(0))
            })
            .collect();
        println!(
            "  stair {id:>9}: {risers:>3} risers of {:.4} m, treads {:.4} m; runs {}",
            riser * 0.3048,
            tread * 0.3048,
            runs.join(", ")
        );
    }
    Ok(())
}
