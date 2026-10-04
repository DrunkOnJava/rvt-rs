//! A duct type's shape, from its native record (B59, B43).
//!
//! A rigid duct type is an `AbsDuctType` record whose objects include its
//! sweep profile: `AbsSysRectSweepProfile`, `AbsSysCircSweepProfile` or
//! `AbsSysOvalSweepProfile`. Revit names a duct's system family by that
//! shape (`Rectangular Duct`, `Round Duct`, `Oval Duct`) and its export names
//! the duct `<system family>:<type>:<ElementId>`. On RE1 Mechanical the duct
//! type 53292 its 25 ducts use holds a rectangular profile, and the file's
//! other duct types a round and an oval one (Measure run 37231078920).

use crate::{RevitFile, native_document};
use std::collections::{BTreeMap, BTreeSet};

/// A duct type's shape.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DuctShape {
    Rectangular,
    Round,
    Oval,
}

impl DuctShape {
    /// The shape a sweep profile object's class names.
    fn of_profile(class: &str) -> Option<Self> {
        Some(match class {
            "AbsSysRectSweepProfile" => Self::Rectangular,
            "AbsSysCircSweepProfile" => Self::Round,
            "AbsSysOvalSweepProfile" => Self::Oval,
            _ => return None,
        })
    }

    /// The system family Revit names a duct of this shape by.
    pub fn system_family(self) -> &'static str {
        match self {
            Self::Rectangular => "Rectangular Duct",
            Self::Round => "Round Duct",
            Self::Oval => "Oval Duct",
        }
    }
}

/// The shape of each of `types` that is a rigid duct type (`AbsDuctType`)
/// with one sweep profile. Errs where the native record path does not read
/// the file's release.
pub fn duct_type_shapes(
    rf: &mut RevitFile,
    types: &BTreeSet<u64>,
) -> anyhow::Result<BTreeMap<u64, DuctShape>> {
    let mut shapes = BTreeMap::new();
    let options = native_document::Options {
        selected_ids: types.clone(),
        ..native_document::Options::default()
    };
    native_document::extract(rf, &options, |record| {
        if record.class_name.as_deref() != Some("AbsDuctType") {
            return Ok(());
        }
        let Some(graph) = record.graph.as_ref() else {
            return Ok(());
        };
        let mut found = graph
            .objects
            .iter()
            .filter_map(|object| DuctShape::of_profile(&object.class_name));
        if let (Some(shape), None) = (found.next(), found.next()) {
            shapes.insert(record.identity.element_id, shape);
        }
        Ok(())
    })?;
    Ok(shapes)
}
