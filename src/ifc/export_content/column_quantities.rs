//! The base quantities of a column joined to slabs and walls (#1046, RE-291).
//!
//! Revit's export draws a column as an extrusion, with its
//! `Qto_ColumnBaseQuantities` from the extrusion's profile and length, unless
//! the column is cut where it meets another element: a slab whose body lies
//! across one end of it (its `CutPairGStep` names the slab) makes Revit's
//! export draw a cut mesh, which has a `CrossSectionArea` and a `NetVolume`
//! that are not the box's and no `Length`, `OuterSurfaceArea` or
//! `GrossVolume`. A wall that cuts it (`CutPairGStep`) changes its section.
//! On the BIM4LCA office's 345 columns at building permit, 18 columns are
//! drawn as a mesh and every one has a cut slab that straddles its top; no
//! other column has (RE-291).
//!
//! This pass writes a column's `CrossSectionArea`, `OuterSurfaceArea`,
//! `GrossVolume` and `NetVolume` as its record box's, on a column drawn as
//! that box and joined by a `CleanPairGStep` (those no clean step joins are
//! the ones `add_body_quantities` covers), when every element its
//! cut and clean pairs name is exported and
//!
//! - is a slab lying wholly above or wholly below the column;
//! - is a wall, covering or column, if it is named by a clean pair only.
//!
//! Only these are accepted because they are the only pairs the office's
//! box-drawn columns hold (RE-291 §2). Any other pair (a beam, roof or
//! footing, say), a wall or covering that cuts it, or an element rvt-rs does
//! not export, leaves the column out.

use std::collections::HashMap;

use super::{RECORD_BBOX_BODY_SOURCE, add_element_set};
use crate::ifc::entities::{IfcEntity, Property, PropertySet, PropertyValue};

/// How far an element may reach into the column's span and still lie clear
/// of it, feet.
const EDGE_FEET: f64 = 5e-3;

/// A column joined by a clean pair, waiting for the pass.
pub(super) struct Pending {
    /// The column's entity index.
    pub index: usize,
    /// The ElementIds its cut pairs name.
    pub cut: Vec<u32>,
    /// The ElementIds its clean pairs name.
    pub clean: Vec<u32>,
}

/// An element's entity type and, where it is drawn as one extrusion, its
/// lowest and highest point.
type Partner = (String, Option<(f64, f64)>);

/// Each exported element's entity and extent, by ElementId. An element
/// written in pieces of different extents has none.
fn partners(entities: &[IfcEntity]) -> HashMap<u32, Partner> {
    let mut out: HashMap<u32, Partner> = HashMap::new();
    for entity in entities {
        let IfcEntity::BuildingElement {
            ifc_type,
            type_guid: Some(guid),
            extrusion,
            solid_shape,
            representation_map_index,
            location_feet,
            ..
        } = entity
        else {
            continue;
        };
        let Ok(id) = guid.parse::<u32>() else {
            continue;
        };
        let span = match (
            extrusion,
            solid_shape,
            representation_map_index,
            location_feet,
        ) {
            (Some(body), None, None, Some(location))
                if body.height_feet.is_finite()
                    && body.height_feet > 0.0
                    && location.iter().all(|value| value.is_finite()) =>
            {
                Some((location[2], location[2] + body.height_feet))
            }
            _ => None,
        };
        let kind = ifc_type.to_ascii_uppercase();
        let merged = match out.get(&id) {
            None => (kind, span),
            Some((_, Some((low, high)))) => match span {
                Some((least, greatest)) => (kind, Some((low.min(least), high.max(greatest)))),
                None => (kind, None),
            },
            Some(_) => (kind, None),
        };
        out.insert(id, merged);
    }
    out
}

/// Whether a span lies wholly above or wholly below `base..top`.
fn lies_clear(span: Option<(f64, f64)>, base: f64, top: f64) -> bool {
    matches!(span, Some((low, high)) if high <= base + EDGE_FEET || low >= top - EDGE_FEET)
}

/// Whether nothing the column's pairs name cuts it or crosses an end of it.
fn unaffected(partners: &HashMap<u32, Partner>, column: &Pending, base: f64, top: f64) -> bool {
    let cut = column.cut.iter().all(|id| match partners.get(id) {
        Some((kind, span)) => kind == "IFCSLAB" && lies_clear(*span, base, top),
        None => false,
    });
    let clean = column.clean.iter().all(|id| match partners.get(id) {
        Some((kind, span)) => match kind.as_str() {
            "IFCSLAB" => lies_clear(*span, base, top),
            "IFCWALL" | "IFCCOVERING" | "IFCCOLUMN" => true,
            _ => false,
        },
        None => false,
    });
    cut && clean
}

/// Write the quantities of each column of `pending` this pass can vouch for.
pub(super) fn add_joined_column_quantities(entities: &mut Vec<IfcEntity>, pending: &[Pending]) {
    if pending.is_empty() {
        return;
    }
    let partners = partners(entities);
    let mut found: Vec<(usize, PropertySet)> = Vec::new();
    for column in pending {
        let Some(IfcEntity::BuildingElement {
            ifc_type,
            extrusion: Some(body),
            solid_shape: None,
            representation_map_index: None,
            location_feet: Some(location),
            property_set,
            ..
        }) = entities.get(column.index)
        else {
            continue;
        };
        let boxed = property_set.as_ref().is_some_and(|set| {
            set.properties.iter().any(|property| {
                property.name == "BodySource"
                    && matches!(&property.value, PropertyValue::Text(source)
                        if source == RECORD_BBOX_BODY_SOURCE)
            })
        });
        let sizes = [body.width_feet, body.depth_feet, body.height_feet];
        if !(ifc_type.eq_ignore_ascii_case("IFCCOLUMN")
            && boxed
            && body.profile_override.is_none()
            && location.iter().all(|value| value.is_finite())
            && sizes.iter().all(|size| size.is_finite() && *size > 0.0))
        {
            continue;
        }
        let (base, top) = (location[2], location[2] + body.height_feet);
        if !unaffected(&partners, column, base, top) {
            continue;
        }
        let [width, depth, height] = sizes;
        let area = width * depth;
        found.push((
            column.index,
            PropertySet {
                name: "Qto_ColumnBaseQuantities".into(),
                properties: vec![
                    Property {
                        name: "CrossSectionArea".into(),
                        value: PropertyValue::AreaSquareFeet(area),
                    },
                    Property {
                        name: "OuterSurfaceArea".into(),
                        value: PropertyValue::AreaSquareFeet(2.0 * (width + depth) * height),
                    },
                    Property {
                        name: "GrossVolume".into(),
                        value: PropertyValue::VolumeCubicFeet(area * height),
                    },
                    Property {
                        name: "NetVolume".into(),
                        value: PropertyValue::VolumeCubicFeet(area * height),
                    },
                ],
            },
        ));
    }
    for (index, set) in found {
        add_element_set(entities, index, set);
    }
}
