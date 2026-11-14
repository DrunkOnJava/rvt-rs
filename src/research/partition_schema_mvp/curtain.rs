//! Curtain walls, grids, mullions and panels.

use super::*;
use crate::le_bytes;

const MODEL_AXES: [[f64; 3]; 3] = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];

/// The box of a curtain mullion: its record box, along the model's axes and
/// the axis it runs along ([`CURTAIN_MULLION_ALONG_FIELD`]), or turned or
/// tilted along its own axes ([`CURTAIN_AXES_FIELDS`], RE-106). A mullion
/// whose axis is not known has no `along`. `kind` is the mullion's type, 0
/// where it is not known (RE-403).
fn mullion_box(element: &DecodedElement, kind: u64) -> Option<MullionBox> {
    let id = element.id?;
    let [x0, y0, z0, x1, y1, z1] = element_record_bbox(element)?;
    let centre = [(x0 + x1) / 2.0, (y0 + y1) / 2.0, (z0 + z1) / 2.0];
    let size = [x1 - x0, y1 - y0, z1 - z0];
    let model = MullionBox {
        id,
        kind,
        centre,
        axes: MODEL_AXES,
        half: size.map(|side| side / 2.0),
        along: None,
        odd: false,
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

/// The curtain walls a part's reference list `list` names (not the part
/// itself), and the walls the grids it names belong to.
fn part_walls(
    own: Option<u32>,
    list: &[u64],
    curtain_walls: &BTreeSet<u32>,
    grids: &BTreeMap<u32, u32>,
) -> (BTreeSet<u32>, BTreeSet<u32>) {
    let ids = || list.iter().filter_map(|&id| u32::try_from(id).ok());
    let named = ids()
        .filter(|id| curtain_walls.contains(id) && Some(*id) != own)
        .collect();
    let on_grid = ids().filter_map(|id| grids.get(&id).copied()).collect();
    (named, on_grid)
}

/// How many distinct walls of `wall_ids`, curtain or not, a part's reference
/// list `list` names (not the part itself).
fn walls_named(own: Option<u32>, list: &[u64], wall_ids: &BTreeSet<u32>) -> usize {
    list.iter()
        .filter_map(|&id| u32::try_from(id).ok())
        .filter(|id| wall_ids.contains(id) && Some(*id) != own)
        .collect::<BTreeSet<u32>>()
        .len()
}

/// The one wall of `named` that the element data of the ids in `list` also
/// names, from `in_data` ([`scan_walls_in_element_data`]); none where no wall
/// of `named` is in any of them or two are.
fn data_confirmed_wall(
    own: Option<u32>,
    list: &[u64],
    named: &BTreeSet<u32>,
    in_data: &BTreeMap<u32, BTreeSet<u32>>,
) -> Option<u32> {
    let confirmed: BTreeSet<u32> = list
        .iter()
        .filter_map(|&id| u32::try_from(id).ok())
        .filter(|id| Some(*id) != own)
        .filter_map(|id| in_data.get(&id))
        .flatten()
        .copied()
        .filter(|wall| named.contains(wall))
        .collect();
    let mut walls = confirmed.into_iter();
    let wall = walls.next()?;
    walls.next().is_none().then_some(wall)
}

/// Give each curtain mullion and panel of `products` that has no
/// [`IS_EXTERNAL_FIELD`] yet its curtain wall's from `walls`, as revit-ifc's
/// `IsElementExternal` gives a family instance with no parameter and no type
/// Function its host's value (CLEANROOM.md source 5; RE-193, RE-269): the
/// curtain wall's own parameter or type Function, not the value of the property
/// set parameter that wall's `Pset_CurtainWallCommon` is written from. Nothing
/// for a part with no curtain wall or a curtain wall with no value; and a part
/// that names two or more and has none of them as its wall (#1865) takes the
/// value all of them have, where they have one and agree.
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
        let value = match whole {
            Some(id) => external.get(&id).copied(),
            None => agreed_value(&part.fields, &external),
        };
        if let Some(value) = value {
            part.fields
                .push((IS_EXTERNAL_FIELD.into(), InstanceField::Bool(value)));
        }
    }
}

/// The `IsExternal` every curtain wall in [`CURTAIN_NAMED_WALLS_FIELD`] has, if
/// each has one and they are all the same.
fn agreed_value(
    fields: &[(String, InstanceField)],
    external: &BTreeMap<u32, bool>,
) -> Option<bool> {
    let named = fields.iter().find_map(|(name, value)| match value {
        InstanceField::Vector(items) if name == CURTAIN_NAMED_WALLS_FIELD => Some(items),
        _ => None,
    })?;
    let mut values = named.iter().map(|item| match item {
        InstanceField::ElementId { id, .. } => external.get(id).copied(),
        _ => None,
    });
    let first = values.next()??;
    values.all(|value| value == Some(first)).then_some(first)
}

/// Mark each curtain mullion that stands on a straight curtain wall with an
/// unedited profile and whose type has its own rectangular section
/// ([`crate::native_mullion_hosts::mullions_on_plain_hosts`], RE-243) with
/// [`MULLION_PLAIN_HOST_FIELD`], and give it, where the boxes of its curtain
/// wall's mullions say how many of its end faces Revit's solid keeps
/// ([`kept_ends_with_partners`], RE-268), [`MULLION_KEPT_ENDS_FIELD`]. The curtain wall is
/// the one its native record names, as in the record's own grouping.
pub(super) fn attach_mullion_plain_hosts(rf: &mut RevitFile, products: &mut [DecodedElement]) {
    let mullions: BTreeSet<u64> = products
        .iter()
        .filter(|element| element.class == "CurtainWallMullion")
        .filter_map(|element| Some(u64::from(element.id?)))
        .collect();
    // RE-418: the pass that draws a mullion from its saved mesh takes its length
    // along the axis its record runs it along, whatever its host.
    let axes: BTreeMap<u32, [f64; 3]> = products
        .iter()
        .filter(|element| element.class == "CurtainWallMullion")
        .filter_map(|element| {
            let mullion = mullion_box(element, 0)?;
            Some((mullion.id, mullion.axes[mullion.along?]))
        })
        .collect();
    rf.remember_mullion_axes(axes);
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
        let kind = element
            .id
            .and_then(|id| found.kind.get(&u64::from(id)))
            .copied()
            .unwrap_or(0);
        if let Some(mullion) = mullion_box(element, kind) {
            groups.entry(*host).or_default().push(mullion);
        }
    }
    let usual = usual_sections(&groups.values().flatten().collect::<Vec<_>>());
    for mullion in groups.values_mut().flatten() {
        mullion.odd = has_unusual_section(mullion, &usual);
    }
    let mut kept: BTreeMap<u32, KeptEnds> = BTreeMap::new();
    for group in groups.values() {
        kept.extend(kept_ends_with_partners(group));
    }
    // RE-404: the pass that draws a mullion from its saved mesh reads the ends of
    // those that stand on no plain host as well.
    rf.remember_mullion_kept_ends(kept.clone());
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
                        value: i64::from(ends.kept),
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

/// The releases on which a curtain panel that is the host panel of another
/// curtain wall's grid is left out (#1975): 2024, the one release a file
/// holding host panels with Revit's own export is measured on (Snowdon Towers'
/// 23, local only; RE-353).
const HOST_PANEL_REVIT_VERSIONS: &[u32] = &[2024];

/// Leave out the curtain panels that are the host panel of another curtain
/// wall's grid, as Revit's export leaves them out (#1975, RE-462). Such a
/// panel is a curtain wall of its own whose element record keeps the panel
/// category, and its wall record holds a `HostPanelInfo` naming the owner's
/// placeholder panel it fills ([`crate::native_walls::WallReads::host_panels`]).
/// Revit's exporter returns for a host panel in `WallExporter.Export` and folds
/// it into its owner's parts in `CurtainSystemExporter.GetVisiblePanelsForGrid`
/// (revit-ifc, CLEANROOM.md source 5): on Snowdon Towers Architectural none of
/// the 23 is written, and their mullions are the owner's parts. The export
/// diagnostics count what is left out, and a check that could not run is
/// reported there with every panel kept.
pub(super) fn without_host_panels(
    rf: &mut RevitFile,
    revit_version: u32,
    products: &mut Vec<DecodedElement>,
    wall_reads: &mut crate::native_walls::WallReads,
) {
    if !HOST_PANEL_REVIT_VERSIONS.contains(&revit_version) {
        return;
    }
    // A panel that holds a basic wall of a layered type (RE-64, RE-72) is
    // written as the curtain wall Revit writes for it, host panel or not: on
    // Snowdon Towers Architectural 18 such panels hold a `HostPanelInfo` and
    // all 18 are `IfcCurtainWall` in Revit's export.
    let is_panel_wall = |element: &DecodedElement| {
        element.fields.iter().any(|(name, value)| {
            name == CURTAIN_PANEL_WALL_FIELD && matches!(value, InstanceField::Bool(true))
        })
    };
    let panels: BTreeSet<u64> = products
        .iter()
        .filter(|element| element.class == "CurtainWallPanel" && !is_panel_wall(element))
        .filter_map(|element| element.id.map(u64::from))
        .collect();
    if panels.is_empty() {
        return;
    }
    let hosts = match wall_reads.host_panels(rf, &panels) {
        Ok(hosts) => hosts,
        Err(error) => {
            rf.note_host_panel_error(panels.len(), error.to_string());
            return;
        }
    };
    if hosts.is_empty() {
        return;
    }
    let mut classes: BTreeMap<String, Vec<crate::partition_element_records::LeftOutElement>> =
        BTreeMap::new();
    products.retain(|element| {
        let left_out = element.class == "CurtainWallPanel"
            && !is_panel_wall(element)
            && element.id.is_some_and(|id| hosts.contains(&u64::from(id)));
        if left_out {
            let framed =
                element.provenance.record_kind.as_deref() == Some("partition_element_record");
            classes.entry(element.class.clone()).or_default().push(
                crate::partition_element_records::LeftOutElement {
                    element_id: element.id,
                    class: element.class.clone(),
                    stream: element.provenance.source_stream.clone().filter(|_| framed),
                    offset: element.provenance.source_offset.filter(|_| framed),
                    detail: None,
                },
            );
        }
        !left_out
    });
    rf.note_host_panel_instances(classes);
}

/// The curtain panels left out as empty (#309): how many, the frame each was
/// read from (#1326), and the types of theirs whose material is set, which
/// would contradict the name.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EmptyCurtainPanels {
    /// Placed panels of the [`EMPTY_PANEL_FAMILY_NAME`] family.
    pub count: usize,
    /// The same panels, in ElementId order, each with the frame it was read
    /// from.
    pub panels: Vec<crate::partition_element_records::LeftOutElement>,
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
    let mut panels: BTreeMap<u32, &crate::partition_element_records::PartitionElementRecord> =
        BTreeMap::new();
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
            panels.insert(record.element_id, record);
            types.insert(type_id);
        }
    }
    out.count = panels.len();
    out.panels = panels
        .into_values()
        .map(|record| {
            crate::partition_element_records::LeftOutElement::from_record(
                record,
                "CurtainWallPanel",
            )
        })
        .collect();
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
        crate::partition_instance_transforms::scan_transforms_in_layout(rf, revit_version, &ids)
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

/// The releases on which a wall of a curtain wall type is a curtain wall by its
/// type, which 2023 reads on its own (RE-440): 2024 to 2027, measured on
/// Autodesk's samples (RE-462).
const CURTAIN_TYPE_REVIT_VERSIONS: std::ops::RangeInclusive<u32> = 2024..=2027;

/// The walls of `wall_ids` whose type, by the wall's own native record, is a
/// `NewCurtainWallType` and that are not the host panel of another curtain
/// wall (RE-462). Nothing on a release not measured, or where a read fails:
/// no wall is then made a curtain wall by its type.
fn curtain_walls_by_type(
    rf: &mut RevitFile,
    revit_version: u32,
    wall_ids: &BTreeSet<u32>,
    wall_reads: &mut crate::native_walls::WallReads,
) -> BTreeSet<u32> {
    if !CURTAIN_TYPE_REVIT_VERSIONS.contains(&revit_version) {
        return BTreeSet::new();
    }
    let ids: BTreeSet<u64> = wall_ids.iter().map(|&id| u64::from(id)).collect();
    let Ok(wall_types) = wall_reads.type_ids(rf, &ids) else {
        return BTreeSet::new();
    };
    let type_ids: BTreeSet<u64> = wall_types.values().copied().collect();
    let Ok(curtain_types) = crate::native_walls::curtain_wall_types(
        rf,
        &type_ids,
        crate::native_document::NATIVE_RELEASES,
    ) else {
        return BTreeSet::new();
    };
    if curtain_types.is_empty() {
        return BTreeSet::new();
    }
    let Ok(host_panels) = wall_reads.host_panels(rf, &ids) else {
        return BTreeSet::new();
    };
    wall_types
        .into_iter()
        .filter(|(wall, type_id)| {
            curtain_types.contains_key(type_id) && !host_panels.contains(wall)
        })
        .filter_map(|(wall, _)| u32::try_from(wall).ok())
        .collect()
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
/// A wall whose type is a curtain wall type is one too, with no mullion or
/// grid naming it, on 2024 to 2027 as on 2023 (RE-440, RE-462): Revit's
/// exporter makes a wall a curtain wall by the curtain grid it has, which
/// is its type's, and tests neither mullions nor cells
/// (`CurtainSystemExporter.IsCurtainSystem`, revit-ifc, CLEANROOM.md source
/// 5). The mullion-free single cell of `rme_basic`'s 573660 and 573664 was
/// the case no mullion or grid found. A wall that is the host panel of
/// another curtain wall's grid is not one: its record holds a
/// `HostPanelInfo`, and Revit writes none of Snowdon Towers' 23 such walls.
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
/// one grid its record names, or, with no grid found, the curtain wall its list
/// names when that is the one wall the list names (RE-453). Naming a curtain
/// wall among others is not enough: on Snowdon Towers three doors name curtain
/// wall 1506500 and their host, and Revit leaves them standalone (RE-46); such
/// a door is a part only when the element data of an id of its list names the
/// curtain wall too (RE-452). No record box decides a door either.
///
/// A part that names two or more curtain walls and finds no grid (the 79 corner
/// mullions of Autodesk's `rac_advanced` saved by Revit 2027, which name both
/// walls of their corner where the 2025 save names one: RE-449) takes the one
/// of them that the element data of one of its other ids names too; its grid
/// is such an id, and its data names the wall at several places whatever the
/// release (`scan_walls_in_element_data`). A part for which that gives no one
/// wall stands alone and keeps the walls it names
/// ([`CURTAIN_NAMED_WALLS_FIELD`]) for [`attach_curtain_member_is_external`].
// Tracker: B64.
pub(super) fn attach_curtain_walls(
    rf: &mut RevitFile,
    revit_version: u32,
    walls: &mut [DecodedElement],
    products: &mut [DecodedElement],
    openings: [&mut Vec<DecodedElement>; 2],
    narrow_references: Option<&BTreeMap<u32, Vec<u64>>>,
    wall_reads: &mut crate::native_walls::WallReads,
) {
    let read_references = |rf: &mut RevitFile, element: &DecodedElement| match narrow_references {
        Some(lists) => element.id.and_then(|id| lists.get(&id)).cloned(),
        None => record_references(rf, element).map(|(list, _)| list),
    };
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
        let list = read_references(rf, product);
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
                .map(|element| read_references(rf, element))
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
    curtain_walls.extend(curtain_walls_by_type(
        rf,
        revit_version,
        &wall_ids,
        wall_reads,
    ));
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
    // RE-449: the ids of the lists of the parts that name two or more curtain
    // walls and no grid, whose element data may name one of the walls.
    let mut crowded: BTreeSet<u32> = BTreeSet::new();
    for (product, list) in products.iter().zip(&references) {
        let Some(list) = list else {
            continue;
        };
        let (named, on_grid) = part_walls(product.id, list, &curtain_walls, &grids);
        if on_grid.is_empty() && named.len() >= 2 {
            crowded.extend(
                list.iter()
                    .filter_map(|&id| u32::try_from(id).ok())
                    .filter(|id| Some(*id) != product.id && !wall_ids.contains(id)),
            );
        }
    }
    let in_data = if crowded.is_empty() {
        BTreeMap::new()
    } else {
        scan_walls_in_element_data(rf, revit_version, &crowded, &curtain_walls)
    };
    for (product, list) in products.iter_mut().zip(references) {
        let Some(list) = list else {
            continue;
        };
        let (named, on_grid) = part_walls(product.id, &list, &curtain_walls, &grids);
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
            (0, _) => data_confirmed_wall(product.id, &list, &named, &in_data),
            _ => None,
        };
        if let Some(whole) = whole {
            product.fields.push((
                AGGREGATE_WHOLE_FIELD.into(),
                InstanceField::ElementId { tag: 0, id: whole },
            ));
        } else if named.len() >= 2 && on_grid.is_empty() {
            product.fields.push((
                CURTAIN_NAMED_WALLS_FIELD.into(),
                InstanceField::Vector(
                    named
                        .iter()
                        .map(|&id| InstanceField::ElementId { tag: 0, id })
                        .collect(),
                ),
            ));
        }
    }
    // RE-452: the ids of the lists of the openings that name exactly one curtain
    // wall and no grid, and another wall besides, whose element data may name
    // the curtain wall.
    let mut opening_crowded: BTreeSet<u32> = BTreeSet::new();
    for (elements, lists) in openings.iter().zip(&opening_references) {
        for (element, list) in elements.iter().zip(lists) {
            let Some(list) = list else {
                continue;
            };
            let (named, on_grid) = part_walls(element.id, list, &curtain_walls, &grids);
            if on_grid.is_empty()
                && named.len() == 1
                && walls_named(element.id, list, &wall_ids) > 1
            {
                opening_crowded.extend(
                    list.iter()
                        .filter_map(|&id| u32::try_from(id).ok())
                        .filter(|id| Some(*id) != element.id && !wall_ids.contains(id)),
                );
            }
        }
    }
    let opening_in_data = if opening_crowded.is_empty() {
        BTreeMap::new()
    } else {
        scan_walls_in_element_data(rf, revit_version, &opening_crowded, &curtain_walls)
    };
    for (elements, lists) in openings.into_iter().zip(opening_references) {
        for (element, list) in elements.iter_mut().zip(lists) {
            let Some(list) = list else {
                continue;
            };
            // RE-453: an opening with no grid whose list names one wall in all,
            // and that a curtain wall, is a part of it: every door Revit's export
            // aggregates into a curtain wall on the BIM4LCA offices and Snowdon
            // Towers names that wall alone, and the three Snowdon doors Revit
            // leaves standalone name a second wall, their host.
            // RE-452: one that names a curtain wall and another wall is a part of
            // it only when the element data of an id of its list names the
            // curtain wall too, as a mullion's or a panel's grid does. On Revit
            // 2027 the grid's offsets (RE-72) are not found, and the curtain
            // door 216844 of `rac_advanced` stood alone.
            let (named, on_grid) = part_walls(element.id, &list, &curtain_walls, &grids);
            let whole = match (on_grid.len(), named.len()) {
                (1, _) => on_grid.iter().next().copied(),
                (0, 1) if walls_named(element.id, &list, &wall_ids) == 1 => {
                    named.iter().next().copied()
                }
                (0, 1) => data_confirmed_wall(element.id, &list, &named, &opening_in_data),
                _ => None,
            };
            if let Some(whole) = whole {
                element.fields.push((
                    AGGREGATE_WHOLE_FIELD.into(),
                    InstanceField::ElementId { tag: 0, id: whole },
                ));
            }
        }
    }
}

/// Give each curtain mullion and panel its type and family (RE-367), on a 32-bit
/// release, where a name entry holds no category for
/// [`crate::partition_names::resolve_type`] to test.
///
/// The type is the one id in its reference list that has a name entry and is
/// no element's record, in `references`. The family is the id its type's data
/// holds at [`CURTAIN_TYPE_FAMILY_OFFSET_32`], where that has a name entry. A
/// part with no such type, or a type with no such family, takes neither.
pub(super) fn attach_curtain_part_types_32(
    rf: &mut RevitFile,
    revit_version: u32,
    references: &BTreeMap<u32, Vec<u64>>,
    products: &mut [DecodedElement],
) {
    let Ok(table) = crate::elem_table::parse_records(rf) else {
        return;
    };
    let declared = crate::elem_table::declared_ids(&table);
    let names = crate::partition_names::name_entries_2023(rf, &declared);
    let type_of = |element: &DecodedElement| -> Option<u32> {
        let own = element.id?;
        let mut named = references
            .get(&own)?
            .iter()
            .filter_map(|&id| u32::try_from(id).ok())
            .filter(|id| *id != own && names.contains_key(id) && !references.contains_key(id))
            .collect::<BTreeSet<u32>>()
            .into_iter();
        let type_id = named.next()?;
        named.next().is_none().then_some(type_id)
    };
    let is_part =
        |element: &DecodedElement| CURTAIN_WALL_PART_CLASSES.contains(&element.class.as_str());
    let types: BTreeSet<u32> = products
        .iter()
        .filter(|element| is_part(element))
        .filter_map(type_of)
        .collect();
    let Some(layout) = crate::partition_names::element_data_layout(revit_version) else {
        return;
    };
    let mut families: BTreeMap<u32, Option<u32>> = BTreeMap::new();
    for stream in rf.partition_stream_names() {
        let Ok(inflated) = rf.inflated_partition(&stream) else {
            continue;
        };
        let buf = inflated.bytes();
        for hit in memchr::memmem::find_iter(buf, &layout.header) {
            let id_at = hit + layout.header.len();
            let Some(id) = layout.id_at(buf, id_at).filter(|id| types.contains(id)) else {
                continue;
            };
            let family = le_bytes::u32_at(buf, id_at + 8 + CURTAIN_TYPE_FAMILY_OFFSET_32)
                .filter(|family| names.contains_key(family));
            families
                .entry(id)
                .and_modify(|held| {
                    if *held != family {
                        *held = None;
                    }
                })
                .or_insert(family);
        }
    }
    for element in products.iter_mut().filter(|element| is_part(element)) {
        let Some(type_id) = type_of(element) else {
            continue;
        };
        let Some(family_id) = families.get(&type_id).copied().flatten() else {
            continue;
        };
        let (Some(type_name), Some(family_name)) = (names.get(&type_id), names.get(&family_id))
        else {
            continue;
        };
        element.fields.push((
            TYPE_ID_FIELD.into(),
            InstanceField::ElementId {
                tag: 0,
                id: type_id,
            },
        ));
        element.fields.push((
            TYPE_NAME_FIELD.into(),
            InstanceField::String(type_name.clone()),
        ));
        element.fields.push((
            FAMILY_NAME_FIELD.into(),
            InstanceField::String(family_name.clone()),
        ));
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

/// The curtain walls of `walls` that the element data of each id in
/// `candidates` names (RE-449): the walls whose ElementId, as a `u64`, lies at
/// least [`ELEMENT_DATA_WALL_MIN_COPIES`] times within
/// [`ELEMENT_DATA_WALL_WINDOW`] bytes past the id's eight, where the release's
/// element-data header opens the data. A curtain grid's data names its wall
/// several times, at offsets that differ between grid kinds and releases (84
/// is common to the two measured, RE-72's 85 and 478 are another kind's), so
/// no fixed offset is read. An id with several copies of its data that do not
/// agree is left out, as in [`scan_curtain_grid_walls`].
pub(super) fn scan_walls_in_element_data(
    rf: &mut RevitFile,
    revit_version: u32,
    candidates: &BTreeSet<u32>,
    walls: &BTreeSet<u32>,
) -> BTreeMap<u32, BTreeSet<u32>> {
    let Some(header) = crate::partition_names::element_data_header(revit_version) else {
        return BTreeMap::new();
    };
    let mut found: BTreeMap<u32, Option<BTreeSet<u32>>> = BTreeMap::new();
    for stream in rf.partition_stream_names() {
        let Ok(inflated) = rf.inflated_partition(&stream) else {
            continue;
        };
        let buf = inflated.bytes();
        // The reader's memo of where the header lies, which the other readers of
        // element data fill and share: no pass of this scan's own over the
        // partition once one of them has made it.
        let Ok(hits) = rf.header_hits(&stream, &header) else {
            continue;
        };
        for &hit in hits.iter() {
            let id_at = hit + header.len();
            let Some(id) = le_bytes::u64_at(buf, id_at)
                .and_then(|id| u32::try_from(id).ok())
                .filter(|id| candidates.contains(id))
            else {
                continue;
            };
            let mut copies: BTreeMap<u32, usize> = BTreeMap::new();
            for offset in 0..ELEMENT_DATA_WALL_WINDOW {
                let wall = le_bytes::u64_at(buf, id_at + 8 + offset)
                    .and_then(|value| u32::try_from(value).ok())
                    .filter(|wall| walls.contains(wall));
                if let Some(wall) = wall {
                    *copies.entry(wall).or_default() += 1;
                }
            }
            let named: BTreeSet<u32> = copies
                .into_iter()
                .filter(|(_, times)| *times >= ELEMENT_DATA_WALL_MIN_COPIES)
                .map(|(wall, _)| wall)
                .collect();
            if named.is_empty() {
                continue;
            }
            match found.get_mut(&id) {
                None => {
                    found.insert(id, Some(named));
                }
                Some(held) => {
                    if held.as_ref() != Some(&named) {
                        *held = None;
                    }
                }
            }
        }
    }
    found
        .into_iter()
        .filter_map(|(id, named)| named.map(|named| (id, named)))
        .collect()
}

/// `inner` lies inside `outer`, both `[min x, min y, min z, max x, max y,
/// max z]`, within `tolerance` on every side.
pub(super) fn box_contains(outer: &[f64; 6], inner: &[f64; 6], tolerance: f64) -> bool {
    (0..3).all(|axis| {
        inner[axis] >= outer[axis] - tolerance && inner[axis + 3] <= outer[axis + 3] + tolerance
    })
}
