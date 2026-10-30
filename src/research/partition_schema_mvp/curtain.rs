//! Curtain walls, grids, mullions and panels.

use super::*;
use crate::le_bytes;

const MODEL_AXES: [[f64; 3]; 3] = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];

/// The box of a curtain mullion: its record box, along the model's axes and
/// the axis it runs along ([`CURTAIN_MULLION_ALONG_FIELD`]), or turned or
/// tilted along its own axes ([`CURTAIN_AXES_FIELDS`], RE-106). A mullion
/// whose axis is not known has no `along`.
fn mullion_box(element: &DecodedElement) -> Option<MullionBox> {
    let id = element.id?;
    let [x0, y0, z0, x1, y1, z1] = element_record_bbox(element)?;
    let centre = [(x0 + x1) / 2.0, (y0 + y1) / 2.0, (z0 + z1) / 2.0];
    let size = [x1 - x0, y1 - y0, z1 - z0];
    let model = MullionBox {
        id,
        centre,
        axes: MODEL_AXES,
        half: size.map(|side| side / 2.0),
        along: None,
    };
    if let Some(axes) = curtain_axes_from_fields(&element.fields) {
        return Some(match crate::ifc::export_content::oriented_box(axes, size) {
            Some((axes, half)) => MullionBox {
                axes,
                half,
                along: Some(2),
                ..model
            },
            None => model,
        });
    }
    let along = element.fields.iter().find_map(|(name, value)| match value {
        InstanceField::Integer { value, .. } if name == CURTAIN_MULLION_ALONG_FIELD => {
            usize::try_from(*value).ok().filter(|axis| *axis < 3)
        }
        _ => None,
    });
    Some(MullionBox { along, ..model })
}

/// Give each curtain mullion and panel of `products` that has no
/// [`IS_EXTERNAL_FIELD`] yet its curtain wall's from `walls`, as revit-ifc's
/// `IsElementExternal` gives a family instance with no parameter and no type
/// Function its host's value (CLEANROOM.md source 5; RE-193, RE-269): the
/// curtain wall's own parameter or type Function, not the value of the property
/// set parameter that wall's `Pset_CurtainWallCommon` is written from. Nothing
/// for a part with no curtain wall or a curtain wall with no value.
// Tracker: B64.
pub(super) fn attach_curtain_member_is_external(
    walls: &[DecodedElement],
    products: &mut [DecodedElement],
) {
    let external: BTreeMap<u32, bool> = walls
        .iter()
        .filter(|wall| wall.class == CURTAIN_WALL_CLASS)
        .filter_map(|wall| {
            let value = wall.fields.iter().find_map(|(name, value)| match value {
                InstanceField::Bool(flag) if name == IS_EXTERNAL_FIELD => Some(*flag),
                _ => None,
            })?;
            Some((wall.id?, value))
        })
        .collect();
    for part in products.iter_mut() {
        if !CURTAIN_WALL_PART_CLASSES.contains(&part.class.as_str())
            || part
                .fields
                .iter()
                .any(|(name, _)| name == IS_EXTERNAL_FIELD)
        {
            continue;
        }
        let whole = part.fields.iter().find_map(|(name, value)| match value {
            InstanceField::ElementId { id, .. } if name == AGGREGATE_WHOLE_FIELD => Some(*id),
            _ => None,
        });
        if let Some(&value) = whole.and_then(|id| external.get(&id)) {
            part.fields
                .push((IS_EXTERNAL_FIELD.into(), InstanceField::Bool(value)));
        }
    }
}

/// Mark each curtain mullion that stands on a straight curtain wall with an
/// unedited profile and whose type has its own rectangular section
/// ([`crate::native_mullion_hosts::mullions_on_plain_hosts`], RE-243) with
/// [`MULLION_PLAIN_HOST_FIELD`], and give it, where the boxes of its curtain
/// wall's mullions say how many of its end faces Revit's solid keeps
/// ([`kept_ends`], RE-268), [`MULLION_KEPT_ENDS_FIELD`]. The curtain wall is
/// the one its native record names, as in the record's own grouping.
pub(super) fn attach_mullion_plain_hosts(rf: &mut RevitFile, products: &mut [DecodedElement]) {
    let mullions: BTreeSet<u64> = products
        .iter()
        .filter(|element| element.class == "CurtainWallMullion")
        .filter_map(|element| Some(u64::from(element.id?)))
        .collect();
    let Ok(found) = crate::native_mullion_hosts::mullion_hosts(rf, &mullions) else {
        return;
    };
    let mut groups: BTreeMap<u64, Vec<MullionBox>> = BTreeMap::new();
    for element in products.iter() {
        if element.class != "CurtainWallMullion" {
            continue;
        }
        let Some(host) = element.id.and_then(|id| found.host.get(&u64::from(id))) else {
            continue;
        };
        if let Some(mullion) = mullion_box(element) {
            groups.entry(*host).or_default().push(mullion);
        }
    }
    let mut kept: BTreeMap<u32, u8> = BTreeMap::new();
    for group in groups.values() {
        kept.extend(kept_ends(group));
    }
    for element in products.iter_mut() {
        let Some(id) = element.id else {
            continue;
        };
        if element.class == "CurtainWallMullion" && found.plain.contains(&u64::from(id)) {
            element
                .fields
                .push((MULLION_PLAIN_HOST_FIELD.into(), InstanceField::Bool(true)));
            if let Some(ends) = kept.get(&id) {
                element.fields.push((
                    MULLION_KEPT_ENDS_FIELD.into(),
                    InstanceField::Integer {
                        value: i64::from(*ends),
                        signed: false,
                        size: 4,
                    },
                ));
            }
        }
    }
}

/// Whether `element` is a curtain panel of the [`EMPTY_PANEL_FAMILY_NAME`]
/// family.
pub(super) fn is_empty_curtain_panel(element: &DecodedElement) -> bool {
    element.class == "CurtainWallPanel"
        && element.fields.iter().any(|(name, value)| {
            matches!(value, InstanceField::String(family) if name == FAMILY_NAME_FIELD && family == EMPTY_PANEL_FAMILY_NAME)
        })
}

/// The curtain panels left out as empty (#309): how many, and the types of
/// theirs whose material is set, which would contradict the name.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EmptyCurtainPanels {
    /// Placed panels of the [`EMPTY_PANEL_FAMILY_NAME`] family.
    pub count: usize,
    /// Their types whose material is set.
    pub types_with_material: BTreeSet<u32>,
}

/// Count the placed curtain panels of the [`EMPTY_PANEL_FAMILY_NAME`]
/// family, the ones recovery leaves out, through the same name-entry type
/// and family join ([`crate::partition_names::resolve_type`],
/// [`crate::partition_names::resolve_family`]).
pub fn scan_empty_curtain_panels(
    rf: &mut RevitFile,
    revit_version: u32,
) -> Result<EmptyCurtainPanels> {
    use crate::partition_element_records as per;
    let mut out = EmptyCurtainPanels::default();
    if !per::supports_revit_version(revit_version) {
        return Ok(out);
    }
    let declared: BTreeSet<u32> = match crate::elem_table::parse_records(rf) {
        Ok(records) => crate::elem_table::declared_ids(&records),
        Err(_) => return Ok(out),
    };
    let names = rf.memoised_element_names();
    let empty_families: BTreeSet<u32> = names
        .entries
        .iter()
        .filter(|(_, entry)| entry.name == EMPTY_PANEL_FAMILY_NAME)
        .map(|(id, _)| *id)
        .collect();
    if empty_families.is_empty() {
        return Ok(out);
    }
    let records =
        per::scan_category_records(rf, revit_version, per::OST_CURTAIN_WALL_PANELS, &declared)?;
    let options = rf.memoised_design_options();
    let phases = rf.memoised_phase_filter();
    let mut panels: BTreeSet<u32> = BTreeSet::new();
    let mut types: BTreeSet<u32> = BTreeSet::new();
    for record in records.iter().filter(|r| {
        r.is_exported_instance() && !options.excludes(r) && !phases.excludes(r) && r.has_volume()
    }) {
        let Some(type_id) = crate::partition_names::resolve_type(
            &names,
            &record.references,
            record.builtin_category,
            record.element_id,
        ) else {
            continue;
        };
        if crate::partition_names::resolve_family(&names, type_id)
            .is_some_and(|family| empty_families.contains(&family))
        {
            panels.insert(record.element_id);
            types.insert(type_id);
        }
    }
    out.count = panels.len();
    for stream in rf.partition_stream_names() {
        let Ok(inflated) = rf.inflated_partition(&stream) else {
            continue;
        };
        for (id, material) in
            crate::partition_names::find_panel_type_materials(inflated.bytes(), &types)
        {
            if material.is_some() {
                out.types_with_material.insert(id);
            }
        }
    }
    Ok(out)
}

/// The three axes [`CURTAIN_AXES_FIELDS`] record, X, Y and Z.
pub fn curtain_axes_from_fields(fields: &[(String, InstanceField)]) -> Option<[[f64; 3]; 3]> {
    let values = CURTAIN_AXES_FIELDS.map(|wanted| {
        fields.iter().find_map(|(name, value)| match value {
            InstanceField::Float { value, .. } if name == wanted => Some(*value),
            _ => None,
        })
    });
    let mut axes = [[0.0; 3]; 3];
    for (index, value) in values.into_iter().enumerate() {
        axes[index / 3][index % 3] = value?;
    }
    Some(axes)
}

/// Give each curtain mullion and panel whose transform (RE-87) turns or
/// tilts it off the model's axes its three axes (RE-106). One on the
/// model's axes keeps its record box as its body's box; a mullion there gets
/// the model axis it runs along ([`CURTAIN_MULLION_ALONG_FIELD`]).
pub(super) fn attach_curtain_axes(
    rf: &mut RevitFile,
    revit_version: u32,
    products: &mut [DecodedElement],
) {
    let ids: BTreeSet<u32> = products
        .iter()
        .filter(|element| CURTAIN_AXES_CLASSES.contains(&element.class.as_str()))
        .filter_map(|element| element.id)
        .collect();
    if ids.is_empty() {
        return;
    }
    let Ok(transforms) =
        crate::partition_instance_transforms::scan_instance_transforms(rf, revit_version, &ids)
    else {
        return;
    };
    for element in products.iter_mut() {
        let Some(transform) = element.id.and_then(|id| transforms.get(&id)) else {
            continue;
        };
        // On the model's axes: each axis has one component, the other two
        // within INSTANCE_TURN_MIN_RADIANS of zero.
        let on_axes = transform.axes.iter().all(|axis| {
            axis.iter()
                .filter(|v| v.abs() > INSTANCE_TURN_MIN_RADIANS)
                .count()
                == 1
        });
        if on_axes {
            if element.class == "CurtainWallMullion" {
                if let Some(along) = transform.axes[2]
                    .iter()
                    .position(|v| v.abs() > INSTANCE_TURN_MIN_RADIANS)
                {
                    element.fields.push((
                        CURTAIN_MULLION_ALONG_FIELD.into(),
                        InstanceField::Integer {
                            value: along as i64,
                            signed: false,
                            size: 4,
                        },
                    ));
                }
            }
            continue;
        }
        let values = transform.axes.iter().flatten().copied();
        for (name, value) in CURTAIN_AXES_FIELDS.iter().zip(values) {
            element
                .fields
                .push(((*name).into(), InstanceField::Float { value, size: 8 }));
        }
    }
}

/// Make every wall a curtain-wall mullion names a curtain wall, and give
/// each panel and mullion that names exactly one of them that wall as its
/// aggregate whole (RE-46).
///
/// A curtain wall owns its grid's mullions, and each mullion's reference
/// list names its wall. On Snowdon Towers the walls named by a mullion are
/// exactly the 42 Revit exports as `IfcCurtainWall` aggregates, and on RE1
/// Architecture the one; no other wall is named by a mullion. Walls named
/// only by a panel are basic walls used as a panel's infill, which Revit
/// exports as `IfcWall`, so a panel alone does not make a curtain wall.
///
/// Some parts name no wall at all, only their sibling mullions: on Snowdon,
/// the mullions of the "Solar Panels" curtain walls. Such a part takes the
/// one curtain wall whose record box contains its own (RE-62). That is
/// Revit's parent for all 89 such mullions and the 3 such panels that
/// Revit's export aggregates.
///
/// A door or window set in a curtain wall's grid in place of a panel is one
/// of its parts too, as Revit's export aggregates RE1 Architecture's door
/// 445975 under curtain wall 445961. It takes the curtain wall of the
/// one grid its record names. Naming a curtain wall directly is not enough:
/// on Snowdon Towers three doors name curtain wall 1506500 and Revit leaves
/// them standalone (RE-46). No record box decides a door either.
// Tracker: B64.
pub(super) fn attach_curtain_walls(
    rf: &mut RevitFile,
    revit_version: u32,
    walls: &mut [DecodedElement],
    products: &mut [DecodedElement],
    openings: [&mut Vec<DecodedElement>; 2],
) {
    let wall_ids: BTreeSet<u32> = walls.iter().filter_map(|wall| wall.id).collect();
    if wall_ids.is_empty() {
        return;
    }
    let mut references: Vec<Option<Vec<u64>>> = Vec::with_capacity(products.len());
    let mut curtain_walls: BTreeSet<u32> = BTreeSet::new();
    for product in products.iter() {
        if !CURTAIN_WALL_PART_CLASSES.contains(&product.class.as_str()) {
            references.push(None);
            continue;
        }
        let list = record_references(rf, product).map(|(list, _)| list);
        if product.class == "CurtainWallMullion" {
            for id in list.iter().flatten() {
                if let Ok(id) = u32::try_from(*id) {
                    if wall_ids.contains(&id) {
                        curtain_walls.insert(id);
                    }
                }
            }
        }
        references.push(list);
    }
    let opening_references: Vec<Vec<Option<Vec<u64>>>> = openings
        .iter()
        .map(|elements| {
            elements
                .iter()
                .map(|element| record_references(rf, element).map(|(list, _)| list))
                .collect()
        })
        .collect();
    // RE-72: a part names the curtain grid it lies on, and the grid's own
    // data names its curtain wall.
    let unrecorded: BTreeSet<u32> = references
        .iter()
        .chain(opening_references.iter().flatten())
        .flatten()
        .flatten()
        .filter_map(|&id| u32::try_from(id).ok())
        .filter(|id| !wall_ids.contains(id))
        .collect();
    let grids = scan_curtain_grid_walls(rf, revit_version, &unrecorded, &wall_ids);
    curtain_walls.extend(grids.values().copied());
    if curtain_walls.is_empty() {
        return;
    }
    for wall in walls.iter_mut() {
        if wall.id.is_some_and(|id| curtain_walls.contains(&id)) {
            wall.class = CURTAIN_WALL_CLASS.into();
        }
    }
    // RE-62: the record boxes of the curtain walls, for parts whose record
    // names none.
    let wall_boxes: Vec<(u32, [f64; 6])> = walls
        .iter()
        .filter_map(|wall| Some((wall.id?, element_record_bbox(wall)?)))
        .filter(|(id, _)| curtain_walls.contains(id))
        .collect();
    for (product, list) in products.iter_mut().zip(references) {
        let Some(list) = list else {
            continue;
        };
        let named: BTreeSet<u32> = list
            .iter()
            .filter_map(|&id| u32::try_from(id).ok())
            .filter(|id| curtain_walls.contains(id) && Some(*id) != product.id)
            .collect();
        let on_grid: BTreeSet<u32> = list
            .iter()
            .filter_map(|&id| u32::try_from(id).ok())
            .filter_map(|id| grids.get(&id).copied())
            .collect();
        let whole = match (on_grid.len(), named.len()) {
            (1, _) => on_grid.iter().next().copied(),
            (0, 1) => named.iter().next().copied(),
            (0, 0) => element_record_bbox(product).and_then(|part| {
                let mut inside = wall_boxes
                    .iter()
                    .filter(|(_, wall)| box_contains(wall, &part, CURTAIN_PART_BOX_TOLERANCE_FEET))
                    .map(|(id, _)| *id);
                let first = inside.next()?;
                inside.next().is_none().then_some(first)
            }),
            _ => None,
        };
        if let Some(whole) = whole {
            product.fields.push((
                AGGREGATE_WHOLE_FIELD.into(),
                InstanceField::ElementId { tag: 0, id: whole },
            ));
        }
    }
    for (elements, lists) in openings.into_iter().zip(opening_references) {
        for (element, list) in elements.iter_mut().zip(lists) {
            let Some(list) = list else {
                continue;
            };
            let ids: Vec<u32> = list
                .iter()
                .filter_map(|&id| u32::try_from(id).ok())
                .collect();
            let on_grid: BTreeSet<u32> =
                ids.iter().filter_map(|id| grids.get(id).copied()).collect();
            if let (1, Some(&whole)) = (on_grid.len(), on_grid.iter().next()) {
                element.fields.push((
                    AGGREGATE_WHOLE_FIELD.into(),
                    InstanceField::ElementId { tag: 0, id: whole },
                ));
            }
        }
    }
}

/// The curtain wall each id in `candidates` names as a curtain grid, by
/// the grid's ElementId (RE-72): its element data carries a wall in
/// `walls` at both [`CURTAIN_GRID_WALL_OFFSETS`]. A grid whose copies
/// disagree is left out.
pub(super) fn scan_curtain_grid_walls(
    rf: &mut RevitFile,
    revit_version: u32,
    candidates: &BTreeSet<u32>,
    walls: &BTreeSet<u32>,
) -> BTreeMap<u32, u32> {
    let Some(header) = crate::partition_names::element_data_header(revit_version) else {
        return BTreeMap::new();
    };
    let mut found: BTreeMap<u32, Option<u32>> = BTreeMap::new();
    for stream in rf.partition_stream_names() {
        let Ok(inflated) = rf.inflated_partition(&stream) else {
            continue;
        };
        let buf = inflated.bytes();
        for hit in memchr::memmem::find_iter(buf, &header) {
            let id_at = hit + header.len();
            let Some(id) = le_bytes::u64_at(buf, id_at)
                .and_then(|id| u32::try_from(id).ok())
                .filter(|id| candidates.contains(id))
            else {
                continue;
            };
            let [first, second] =
                CURTAIN_GRID_WALL_OFFSETS.map(|offset| le_bytes::u64_at(buf, id_at + 8 + offset));
            let wall = match (first, second) {
                (Some(first), Some(second)) if first == second => u32::try_from(first)
                    .ok()
                    .filter(|wall| walls.contains(wall)),
                _ => None,
            };
            let Some(wall) = wall else {
                continue;
            };
            match found.get_mut(&id) {
                None => {
                    found.insert(id, Some(wall));
                }
                Some(held) => {
                    if *held != Some(wall) {
                        *held = None;
                    }
                }
            }
        }
    }
    found
        .into_iter()
        .filter_map(|(id, wall)| wall.map(|wall| (id, wall)))
        .collect()
}

/// `inner` lies inside `outer`, both `[min x, min y, min z, max x, max y,
/// max z]`, within `tolerance` on every side.
pub(super) fn box_contains(outer: &[f64; 6], inner: &[f64; 6], tolerance: f64) -> bool {
    (0..3).all(|axis| {
        inner[axis] >= outer[axis] - tolerance && inner[axis + 3] <= outer[axis + 3] + tolerance
    })
}
