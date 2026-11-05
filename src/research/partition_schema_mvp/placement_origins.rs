//! The origin of a door that is set in no wall (RE-361, #1494).

use super::*;

/// Fields holding the origin of the `IfcLocalPlacement` of a door in Revit's
/// own export, model feet (RE-361): the origin of its stored transform
/// (RE-87), for a door rvt-rs binds to no host wall (no `OPENING_HOST_FIELD`)
/// whose stored transform is upright. Revit's export gives such a door no
/// opening, and its origin is the stored one: the offset a door set in a wall
/// has from it is the corner of its opening (#1465), and there is none.
pub const OPENING_PLACEMENT_ORIGIN_FIELDS: [&str; 3] = [
    "m_opening_placement_origin_x",
    "m_opening_placement_origin_y",
    "m_opening_placement_origin_z",
];

/// The origin [`OPENING_PLACEMENT_ORIGIN_FIELDS`] record.
pub fn opening_placement_origin_from_fields(
    fields: &[(String, InstanceField)],
) -> Option<[f64; 3]> {
    let float = |wanted: &str| {
        fields.iter().find_map(|(name, value)| match value {
            InstanceField::Float { value, .. } if name == wanted => Some(*value),
            _ => None,
        })
    };
    let [x, y, z] = OPENING_PLACEMENT_ORIGIN_FIELDS.map(float);
    Some([x?, y?, z?])
}

/// Give each door with no host wall ([`OPENING_HOST_FIELD`]) whose stored
/// transform is read, and upright, the origin of that transform
/// ([`OPENING_PLACEMENT_ORIGIN_FIELDS`], RE-361). A door whose type draws no
/// geometry ([`TYPE_WITHOUT_GEOMETRY_FIELD`]) is written as its opening alone
/// and is left as it is. Run after `attach_cut_loop_sizes`, which may rebind
/// the host (RE-245).
pub(super) fn attach_opening_placement_origins(
    rf: &mut RevitFile,
    revit_version: u32,
    doors: &mut [DecodedElement],
) {
    let wanted = |door: &DecodedElement| {
        !door.fields.iter().any(|(name, value)| {
            name == OPENING_HOST_FIELD
                || (name == TYPE_WITHOUT_GEOMETRY_FIELD
                    && matches!(value, InstanceField::Bool(true)))
        })
    };
    let ids: BTreeSet<u32> = doors
        .iter()
        .filter(|door| wanted(door))
        .filter_map(|door| door.id)
        .collect();
    if ids.is_empty() {
        return;
    }
    let Ok(transforms) =
        crate::partition_instance_transforms::scan_instance_transforms(rf, revit_version, &ids)
    else {
        return;
    };
    for door in doors.iter_mut().filter(|door| wanted(door)) {
        let Some(transform) = door.id.and_then(|id| transforms.get(&id)) else {
            continue;
        };
        if !transform.is_upright() || !transform.origin.iter().all(|value| value.is_finite()) {
            continue;
        }
        for (name, value) in OPENING_PLACEMENT_ORIGIN_FIELDS.iter().zip(transform.origin) {
            door.fields
                .push(((*name).into(), InstanceField::Float { value, size: 8 }));
        }
    }
}
