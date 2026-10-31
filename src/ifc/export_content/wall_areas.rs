//! The side areas and volumes of a wall joined to floors (#1046, RE-270).
//!
//! A wall joined to a roof, floor or footing (`JoinToRoofGStep`) is the same
//! prism as one nothing is joined to, except where the element cuts it: Revit
//! cuts the top at the underside of a floor that covers it, and the base at
//! the top of one beneath it. This pass writes the quantities of
//! `prism_quantities` and the wall's `Height`, the body's (RE-293), on such a
//! wall when all of these hold, measured on rvt-rs's own bodies:
//!
//! - its native record holds no step but the harmless ones, a roof or floor
//!   join and a clean join, and its length is confirmed by its record: it
//!   carries a `Length` ([`crate::partition_schema_mvp::WALL_LENGTH_FIELD`],
//!   RE-270), or its centreline cut back where its body meets the walls
//!   joined at its ends ([`crate::partition_schema_mvp::WALL_CUTBACK_LENGTH_FIELD`])
//!   is as long as the body's long side (RE-290);
//! - its body is its record's box cut back at its joins, a plain rectangle;
//! - every element its joins name is a slab, roof or footing drawn as a plain
//!   extrusion that lies wholly above the wall's top or wholly below its
//!   base, so that it cannot cut the wall. A named element that is anything
//!   else, or is not drawn as an extrusion (a sloped roof), leaves the wall
//!   out.
//!
//! The pass runs after `wall_tops`' (RE-267), on purpose, and reads
//! each extrusion at the height that pass left it. A wall the floor cut has
//! that floor's underside for its top, which the floor then clears, so such a
//! wall is measured like any other. Revit's export writes no side areas on
//! the walls it draws as a mesh (RE-267 §7 lists those among the cut walls
//! that have no `Height`), and nothing read tells those from the rest; none
//! of them has a confirmed length and a body from its record's box, so none
//! is taken here (Measure, RE-270 §4).

use std::collections::HashMap;

use super::{add_element_set, prism_quantities};
use crate::ifc::entities::{IfcEntity, Property, PropertySet, PropertyValue};

/// The entities a wall may be joined to that this pass can measure.
const JOINED_TYPES: [&str; 3] = ["IFCSLAB", "IFCROOF", "IFCFOOTING"];

/// How far an element may lie inside the wall's height and still clear it,
/// feet.
const CLEARANCE_FEET: f64 = 1e-3;

/// How far the cut-back length may differ from the body's long side and
/// still confirm it, feet: the agreement the record's two readings of each
/// end are held to (RE-252).
const CUTBACK_AGREE_FEET: f64 = 1e-4;

/// A wall joined to roofs, floors or footings, waiting for the pass.
pub(super) struct Pending {
    /// The wall's entity index.
    pub index: usize,
    /// The ElementIds its roof and floor joins name.
    pub named: Vec<u32>,
    /// Whether its record gives it a `Length` (RE-252).
    pub confirmed: bool,
    /// Its centreline cut back at its joins, where both ends are read
    /// (RE-290).
    pub cutback_feet: Option<f64>,
}

/// The lowest and highest point of each slab, roof and footing by ElementId,
/// `None` for one that is not drawn as plain extrusions.
fn spans(entities: &[IfcEntity]) -> HashMap<u32, Option<(f64, f64)>> {
    let mut spans: HashMap<u32, Option<(f64, f64)>> = HashMap::new();
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
        if !JOINED_TYPES.contains(&ifc_type.as_str()) {
            continue;
        }
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
        let merged = match (spans.get(&id), span) {
            (None, span) => span,
            (Some(Some((low, high))), Some((least, greatest))) => {
                Some((low.min(least), high.max(greatest)))
            }
            (Some(_), _) => None,
        };
        spans.insert(id, merged);
    }
    spans
}

/// The `Width` of the `Qto_WallBaseQuantities` already given to the entity at
/// `index`.
fn written_width(entities: &[IfcEntity], index: usize) -> Option<f64> {
    entities.iter().skip(index).find_map(|entity| match entity {
        IfcEntity::ElementPropertySet { element, set }
            if *element == index && set.name == "Qto_WallBaseQuantities" =>
        {
            set.properties
                .iter()
                .find_map(|property| match &property.value {
                    PropertyValue::PositiveLengthFeet(width) if property.name == "Width" => {
                        Some(*width)
                    }
                    _ => None,
                })
        }
        _ => None,
    })
}

/// Write the quantities of each wall of `pending` that this pass can vouch
/// for.
pub(super) fn add_bounded_wall_quantities(entities: &mut Vec<IfcEntity>, pending: &[Pending]) {
    if pending.is_empty() {
        return;
    }
    let spans = spans(entities);
    let mut found: Vec<(usize, PropertySet)> = Vec::new();
    for Pending {
        index,
        named,
        confirmed,
        cutback_feet,
    } in pending
    {
        let Some(IfcEntity::BuildingElement {
            ifc_type,
            extrusion: Some(body),
            solid_shape: None,
            representation_map_index: None,
            location_feet: Some(location),
            property_set,
            ..
        }) = entities.get(*index)
        else {
            continue;
        };
        let boxed = property_set.as_ref().is_some_and(|set| {
            set.properties.iter().any(|property| {
                property.name == "BodySource"
                    && matches!(&property.value, PropertyValue::Text(source)
                        if source == crate::element_record_wall_joins::WALL_BODY_JOIN_TRIMMED)
            })
        });
        if !(matches!(ifc_type.as_str(), "IFCWALL" | "IFCWALLSTANDARDCASE")
            && boxed
            && body.profile_override.is_none()
            && location.iter().all(|value| value.is_finite()))
        {
            continue;
        }
        let along = body.width_feet.max(body.depth_feet);
        let measured =
            cutback_feet.is_some_and(|length| (length - along).abs() <= CUTBACK_AGREE_FEET);
        if !(*confirmed || measured) {
            continue;
        }
        let (base, top) = (location[2], location[2] + body.height_feet);
        let clear = named.iter().all(|id| {
            matches!(spans.get(id), Some(Some((low, high)))
                if *low >= top - CLEARANCE_FEET || *high <= base + CLEARANCE_FEET)
        });
        if !clear {
            continue;
        }
        let Some(quantities) =
            written_width(entities, *index).and_then(|width| prism_quantities(body, width))
        else {
            continue;
        };
        // Revit's export writes a wall's `Height` on exactly the walls it
        // writes these four on (RE-270 §1), and it is the body's: rvt-rs's
        // extrusion is as tall as Revit's solid on every such wall (RE-293).
        let mut properties = Vec::from(quantities);
        properties.push(Property {
            name: "Height".into(),
            value: PropertyValue::PositiveLengthFeet(body.height_feet),
        });
        found.push((
            *index,
            PropertySet {
                name: "Qto_WallBaseQuantities".into(),
                properties,
            },
        ));
    }
    for (index, set) in found {
        add_element_set(entities, index, set);
    }
}
