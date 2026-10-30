//! Type and instance materials, and the material list.

use super::*;
use crate::partition_name_candidates::{NameBucket, classify_name};

/// Give each element the materials its type's geometry uses (RE-82).
pub(super) fn attach_type_materials(
    materials: &BTreeMap<u32, Vec<String>>,
    elements: &mut [DecodedElement],
) {
    for element in elements.iter_mut() {
        let type_id = element.fields.iter().find_map(|(name, value)| match value {
            InstanceField::ElementId { id, .. } if name == TYPE_ID_FIELD => Some(*id),
            _ => None,
        });
        let Some(names) = type_id.and_then(|id| materials.get(&id)) else {
            continue;
        };
        if names.is_empty() {
            element.fields.push((
                TYPE_WITHOUT_GEOMETRY_FIELD.into(),
                InstanceField::Bool(true),
            ));
        }
        for name in names {
            element.fields.push((
                TYPE_MATERIAL_FIELD.into(),
                InstanceField::String(name.clone()),
            ));
        }
    }
}

/// Replace each family type's scanned materials with those its native
/// `m_geomTag2MaterialId` names ([`crate::native_type_materials`], RE-203)
/// where that names any and every one is named, for the types of columns,
/// doors, windows and products. The scanned order is kept where the two
/// name the same set. Returns the replaced types that also map geometry to
/// no material that their family shows ([`crate::native_family_forms`]),
/// whose instances take their own material parameters' or
/// `<Unnamed>` for it ([`attach_partly_unset_materials`]). Left as scanned
/// where the native record path or the material names are not read.
pub(super) fn refine_type_materials(
    rf: &mut RevitFile,
    revit_version: u32,
    type_materials: &mut BTreeMap<u32, Vec<String>>,
    out: &PartitionSchemaMvp,
) -> BTreeSet<u32> {
    let mut partly_unset = BTreeSet::new();
    if type_materials.is_empty() {
        return partly_unset;
    }
    let types: BTreeSet<u64> = out
        .columns
        .iter()
        .chain(&out.doors)
        .chain(&out.windows)
        .chain(&out.products)
        .filter_map(|element| {
            element.fields.iter().find_map(|(name, value)| match value {
                InstanceField::ElementId { id, .. } if name == TYPE_ID_FIELD => {
                    Some(u64::from(*id))
                }
                _ => None,
            })
        })
        .collect();
    let Ok(records) = crate::elem_table::parse_records(rf) else {
        return partly_unset;
    };
    let declared = crate::elem_table::declared_ids(&records);
    let Ok(names) = crate::partition_materials::scan_material_names(rf, revit_version, &declared)
    else {
        return partly_unset;
    };
    let materials: BTreeSet<u32> = names.keys().copied().collect();
    let Ok(found) = crate::native_type_materials::geometry_materials(rf, &types, &materials) else {
        return partly_unset;
    };
    // A family whose geometry with no material is all hidden or void has
    // no `<Unnamed>` in Revit's export (RE-204), and a material only its
    // hidden forms have is not exported (RE-205).
    let families: BTreeSet<u64> = found.values().filter_map(|found| found.family).collect();
    let forms = crate::native_family_forms::family_forms(rf, &families).unwrap_or_default();
    for (type_id, found) in found {
        let Ok(type_id) = u32::try_from(type_id) else {
            continue;
        };
        let family = found.family.and_then(|family| forms.get(&family));
        let ids: Vec<u32> = found
            .materials
            .iter()
            .copied()
            .filter(|id| !family.is_some_and(|family| family.hidden_only.contains(id)))
            .collect();
        let shown = family.and_then(|family| family.shows_unset).unwrap_or(true);
        if ids.is_empty() {
            // Only hidden forms had materials: what is shown is the
            // geometry with none (RE-206).
            if found.unset && shown {
                type_materials.insert(
                    type_id,
                    vec![crate::partition_type_materials::UNNAMED_MATERIAL.to_string()],
                );
            }
            continue;
        }
        let resolved: Vec<String> = ids.iter().filter_map(|id| names.get(id).cloned()).collect();
        if resolved.len() != ids.len() {
            continue;
        }
        let scanned = type_materials.entry(type_id).or_default();
        let same: BTreeSet<&String> = scanned.iter().collect();
        if found.unset && shown {
            partly_unset.insert(type_id);
        }
        if found.unset || same != resolved.iter().collect::<BTreeSet<_>>() {
            *scanned = resolved;
        }
    }
    partly_unset
}

/// Give each instance of a type in `partly_unset`, which maps some geometry
/// to no material, the materials its own material parameters name
/// (`instance_materials`, RE-200), or else `<Unnamed>` (RE-149), alongside
/// its type's (RE-203): Revit's export does, on Snowdon Towers' 31 "Sink
/// Vanity-Round", 21 refrigerators, 21 washer-dryers and RE1
/// Architecture's microwave.
pub(super) fn attach_partly_unset_materials(
    partly_unset: &BTreeSet<u32>,
    instance_materials: &BTreeMap<u32, Vec<String>>,
    elements: &mut [DecodedElement],
) {
    for element in elements.iter_mut() {
        let type_id = element.fields.iter().find_map(|(name, value)| match value {
            InstanceField::ElementId { id, .. } if name == TYPE_ID_FIELD => Some(*id),
            _ => None,
        });
        if !type_id.is_some_and(|id| partly_unset.contains(&id)) {
            continue;
        }
        let names = element
            .id
            .and_then(|id| instance_materials.get(&id))
            .cloned()
            .unwrap_or_else(|| vec![crate::partition_type_materials::UNNAMED_MATERIAL.into()]);
        for name in names {
            let present = element.fields.iter().any(|(field, value)| {
                field == TYPE_MATERIAL_FIELD
                    && matches!(value, InstanceField::String(s) if *s == name)
            });
            if !present {
                element
                    .fields
                    .push((TYPE_MATERIAL_FIELD.into(), InstanceField::String(name)));
            }
        }
    }
}

/// The material names the instance parameters of each column and product
/// whose type is one of `unset_types` give it, by the instance's ElementId
/// ([`crate::native_instance_materials`], RE-200). Empty where the native
/// record path or the material names are not read.
pub(super) fn instance_parameter_material_names(
    rf: &mut RevitFile,
    revit_version: u32,
    unset_types: &BTreeSet<u32>,
    out: &PartitionSchemaMvp,
) -> BTreeMap<u32, Vec<String>> {
    let instances: BTreeSet<u64> = out
        .columns
        .iter()
        .chain(&out.products)
        .filter(|element| {
            element.fields.iter().any(|(name, value)| {
                matches!(value, InstanceField::ElementId { id, .. }
                    if name == TYPE_ID_FIELD && unset_types.contains(id))
            })
        })
        .filter_map(|element| element.id.map(u64::from))
        .collect();
    if instances.is_empty() {
        return BTreeMap::new();
    }
    let Ok(records) = crate::elem_table::parse_records(rf) else {
        return BTreeMap::new();
    };
    let declared = crate::elem_table::declared_ids(&records);
    let Ok(names) = crate::partition_materials::scan_material_names(rf, revit_version, &declared)
    else {
        return BTreeMap::new();
    };
    let materials: BTreeSet<u32> = names.keys().copied().collect();
    let Ok(found) =
        crate::native_instance_materials::instance_parameter_materials(rf, &instances, &materials)
    else {
        return BTreeMap::new();
    };
    found
        .into_iter()
        .filter_map(|(instance, ids)| {
            let instance = u32::try_from(instance).ok()?;
            let resolved: Vec<String> =
                ids.iter().filter_map(|id| names.get(id).cloned()).collect();
            // Every value named, or none taken: a set missing a material
            // would be a wrong set, where `<Unnamed>` is Revit's fallback.
            (resolved.len() == ids.len()).then_some((instance, resolved))
        })
        .collect()
}

/// Give each element whose type draws its geometry with no material of its
/// own, and that has no type material yet, the materials its own material
/// parameters name (`instance_materials`, RE-200), or else the material
/// Revit's export writes for such geometry,
/// [`crate::partition_type_materials::UNNAMED_MATERIAL`] (RE-149). Doors,
/// windows, walls and slabs are left alone: a door or window type without a
/// material map is written as its opening (RE-84).
pub(super) fn attach_unnamed_materials(
    unset_types: &BTreeSet<u32>,
    instance_materials: &BTreeMap<u32, Vec<String>>,
    elements: &mut [DecodedElement],
) {
    for element in elements.iter_mut() {
        if element
            .fields
            .iter()
            .any(|(name, _)| name == TYPE_MATERIAL_FIELD)
        {
            continue;
        }
        let type_id = element.fields.iter().find_map(|(name, value)| match value {
            InstanceField::ElementId { id, .. } if name == TYPE_ID_FIELD => Some(*id),
            _ => None,
        });
        if type_id.is_some_and(|id| unset_types.contains(&id)) {
            // The type draws geometry, so it is not a type without
            // geometry (RE-84), which `attach_type_materials` took its empty
            // map to mean; a hosted instance would otherwise be written as
            // its opening alone (reported by Cursor Bugbot on #580).
            element
                .fields
                .retain(|(name, _)| name != TYPE_WITHOUT_GEOMETRY_FIELD);
            let names = element
                .id
                .and_then(|id| instance_materials.get(&id))
                .cloned()
                .unwrap_or_else(|| vec![crate::partition_type_materials::UNNAMED_MATERIAL.into()]);
            for name in names {
                element
                    .fields
                    .push((TYPE_MATERIAL_FIELD.into(), InstanceField::String(name)));
            }
        }
    }
}

/// Give each curtain wall mullion and panel ([`CURTAIN_AXES_CLASSES`]) that
/// has no type material yet the material its type holds
/// ([`crate::partition_curtain_materials`], RE-166), by name, as Revit's
/// export associates it.
pub(super) fn attach_curtain_materials(
    rf: &mut RevitFile,
    revit_version: u32,
    products: &mut [DecodedElement],
) {
    use crate::partition_curtain_materials as pcm;
    let is_part = |element: &DecodedElement| CURTAIN_AXES_CLASSES.contains(&element.class.as_str());
    if !pcm::supports_revit_version(revit_version) || !products.iter().any(is_part) {
        return;
    }
    let Ok(records) = crate::elem_table::parse_records(rf) else {
        return;
    };
    let declared = crate::elem_table::declared_ids(&records);
    let Ok(names) = crate::partition_materials::scan_material_names(rf, revit_version, &declared)
    else {
        return;
    };
    let materials: BTreeSet<u32> = names.keys().copied().collect();
    let Ok(type_materials) = pcm::scan_curtain_type_materials(rf, revit_version, &materials) else {
        return;
    };
    for element in products.iter_mut().filter(|element| is_part(element)) {
        // A part whose type's map holds only unset values has `<Unnamed>`
        // (#580); its type's own material replaces it (RE-201).
        if element.fields.iter().any(|(name, value)| {
            name == TYPE_MATERIAL_FIELD
                && !matches!(value, InstanceField::String(s) if s == crate::partition_type_materials::UNNAMED_MATERIAL)
        }) {
            continue;
        }
        let type_id = element.fields.iter().find_map(|(name, value)| match value {
            InstanceField::ElementId { id, .. } if name == TYPE_ID_FIELD => Some(*id),
            _ => None,
        });
        let Some(name) = type_id
            .and_then(|id| type_materials.get(&id))
            .and_then(|material| names.get(material))
        else {
            continue;
        };
        element
            .fields
            .retain(|(field, _)| field != TYPE_MATERIAL_FIELD);
        element.fields.push((
            TYPE_MATERIAL_FIELD.into(),
            InstanceField::String(name.clone()),
        ));
    }
}

/// Whether `element`'s only type material is
/// [`crate::partition_type_materials::UNNAMED_MATERIAL`].
pub(super) fn has_unnamed_material_only(element: &DecodedElement) -> bool {
    use crate::partition_type_materials::UNNAMED_MATERIAL;
    let mut materials = element
        .fields
        .iter()
        .filter(|(name, _)| name == TYPE_MATERIAL_FIELD)
        .peekable();
    materials.peek().is_some()
        && materials
            .all(|(_, value)| matches!(value, InstanceField::String(s) if s == UNNAMED_MATERIAL))
}

/// Replace `element`'s type materials with `names`.
pub(super) fn set_type_materials(element: &mut DecodedElement, names: Vec<String>) {
    element
        .fields
        .retain(|(name, _)| name != TYPE_MATERIAL_FIELD);
    for name in names {
        element
            .fields
            .push((TYPE_MATERIAL_FIELD.into(), InstanceField::String(name)));
    }
}

/// A column with only `<Unnamed>` takes a layer of the wall it was first
/// joined to (RE-202, #355; [`crate::native_column_joins`]): the first join
/// among the walls that overlap it ([`COLUMN_JOINED_WALL_FIELD`]) where any
/// do, so that a foundation wall below it is passed over (RE-205). Its
/// exterior layer where the column stands out past both of the wall's faces
/// or its box centre is on the wall's exterior side, the interior layer
/// otherwise. Revit's export gives all 108 of Snowdon Towers' off-centre
/// columns that layer. The rule for a column past both faces rests on one
/// column whose wall's two faces differ (1366814): the other 50 such columns
/// are in walls with the same material on both faces. A column whose wall is
/// not straight, or whose layer on that side has no named material, is left
/// alone.
///
/// [`COLUMN_JOINED_WALL_FIELD`]: crate::element_record_column_cuts::COLUMN_JOINED_WALL_FIELD
pub(super) fn attach_first_join_materials(
    rf: &mut RevitFile,
    joined: Option<&BTreeMap<u64, Vec<u64>>>,
    columns: &mut [DecodedElement],
    walls: &[DecodedElement],
) {
    use crate::element_record_column_cuts::COLUMN_JOINED_WALL_FIELD;
    use crate::element_record_wall_joins::WALL_THICKNESS_FIELD;
    use crate::native_column_joins as joins;
    let float = |element: &DecodedElement, wanted: &str| {
        element.fields.iter().find_map(|(name, value)| match value {
            InstanceField::Float { value, .. } if name == wanted => Some(*value),
            _ => None,
        })
    };
    let Some(joined) = joined else {
        return;
    };
    let first: BTreeMap<u64, u64> = columns
        .iter()
        .filter(|column| has_unnamed_material_only(column))
        .filter_map(|column| {
            let id = u64::from(column.id?);
            let order = joined.get(&id)?;
            let overlapping: BTreeSet<u64> = column
                .fields
                .iter()
                .filter_map(|(name, value)| match value {
                    InstanceField::ElementId { id, .. } if name == COLUMN_JOINED_WALL_FIELD => {
                        Some(u64::from(*id))
                    }
                    _ => None,
                })
                .collect();
            let wall = order
                .iter()
                .find(|wall| overlapping.contains(wall))
                .or_else(|| order.first())?;
            Some((id, *wall))
        })
        .collect();
    // Each wall's layers, thickness and centreline point (RE-54).
    struct WallData {
        layers: Vec<Option<String>>,
        thickness: Option<f64>,
        axis: Option<[f64; 2]>,
    }
    let wall_data: BTreeMap<u64, WallData> = walls
        .iter()
        .filter_map(|wall| {
            let id = u64::from(wall.id?);
            if !first.values().any(|joined| *joined == id) {
                return None;
            }
            let bands = wall
                .fields
                .iter()
                .find_map(|(name, value)| (name == WALL_LAYERS_FIELD).then_some(value))
                .and_then(layer_bands_from_field)?;
            let names: Vec<Option<String>> = bands.into_iter().map(|band| band.name).collect();
            let [ax, ay] = WALL_AXIS_START_FIELDS.map(|name| float(wall, name));
            let axis = ax.zip(ay).map(|(x, y)| [x, y]);
            (!names.is_empty()).then_some((
                id,
                WallData {
                    layers: names,
                    thickness: float(wall, WALL_THICKNESS_FIELD),
                    axis,
                },
            ))
        })
        .collect();
    let Ok(sides) = joins::wall_sides(rf, &wall_data.keys().copied().collect()) else {
        return;
    };
    for column in columns.iter_mut() {
        let Some(id) = column.id.map(u64::from) else {
            continue;
        };
        let Some(wall) = first.get(&id) else {
            continue;
        };
        let (Some(x), Some(y)) = (float(column, "m_locationX"), float(column, "m_locationY"))
        else {
            continue;
        };
        let (Some(data), Some(side)) = (wall_data.get(wall), sides.get(wall)) else {
            continue;
        };
        let (layers, thickness, axis) = (&data.layers, data.thickness, data.axis);
        // How far the column's box reaches each way across the wall.
        let reach = match (float(column, "m_bboxWidth"), float(column, "m_bboxDepth")) {
            (Some(width), Some(depth)) => {
                (side.exterior[0].abs() * width + side.exterior[1].abs() * depth) / 2.0
            }
            _ => 0.0,
        };
        let centre = side.offset([x, y], axis.unwrap_or(side.origin));
        let past_both = thickness.is_some_and(|thickness| {
            axis.is_some() && centre - reach < -thickness / 2.0 && centre + reach > thickness / 2.0
        });
        let layer = if past_both || side.is_exterior([x, y]) {
            layers.first()
        } else {
            layers.last()
        };
        if let Some(Some(layer)) = layer {
            set_type_materials(column, vec![layer.clone()]);
        }
    }
}

/// An instance of rendered content (a family whose
/// `Family.m_eRenderModelType` is 4: RPC people and plants, Snowdon Towers'
/// Honey-Locust trees) has the one material `<Unnamed>`, as Revit's export
/// writes it (RE-202, #355; [`crate::native_rendered_content`]).
pub(super) fn attach_rendered_content_materials(
    rf: &mut RevitFile,
    products: &mut [DecodedElement],
) {
    use crate::partition_type_materials::UNNAMED_MATERIAL;
    let type_of = |element: &DecodedElement| {
        element.fields.iter().find_map(|(name, value)| match value {
            InstanceField::ElementId { id, .. } if name == TYPE_ID_FIELD => Some(u64::from(*id)),
            _ => None,
        })
    };
    let types: BTreeSet<u64> = products
        .iter()
        .filter(|element| {
            element
                .fields
                .iter()
                .any(|(name, _)| name == TYPE_MATERIAL_FIELD)
        })
        .filter_map(type_of)
        .collect();
    let Ok(rendered) = crate::native_rendered_content::rendered_content_types(rf, &types) else {
        return;
    };
    for element in products.iter_mut() {
        if type_of(element).is_some_and(|id| rendered.contains(&id))
            && element
                .fields
                .iter()
                .any(|(name, _)| name == TYPE_MATERIAL_FIELD)
        {
            set_type_materials(element, vec![UNNAMED_MATERIAL.to_string()]);
        }
    }
}

/// A column whose type draws with no material and that walls are joined to
/// takes the joined walls' material (#355): Revit's export gives the 149
/// columns of `2024_Core_Interior.rvt` that walls are joined to "Default
/// Wall", the material of those walls, and `<Unnamed>` to the 107 no wall
/// joins. Only when every joined wall's layers name the same one set of
/// materials; otherwise the column keeps `<Unnamed>`.
pub(super) fn attach_joined_wall_materials(
    columns: &mut [DecodedElement],
    walls: &[DecodedElement],
) {
    use crate::element_record_column_cuts::COLUMN_JOINED_WALL_FIELD;
    use crate::partition_type_materials::UNNAMED_MATERIAL;

    let wall_materials: BTreeMap<u32, BTreeSet<String>> = walls
        .iter()
        .filter_map(|wall| {
            let layers = element_layers_from_fields(&wall.fields)?;
            let names: Option<BTreeSet<String>> =
                layers.layers.iter().map(|band| band.name.clone()).collect();
            Some((wall.id?, names.filter(|n| !n.is_empty())?))
        })
        .collect();
    for column in columns.iter_mut() {
        let unnamed_only = column
            .fields
            .iter()
            .filter(|(name, _)| name == TYPE_MATERIAL_FIELD)
            .all(|(_, value)| matches!(value, InstanceField::String(s) if s == UNNAMED_MATERIAL));
        let has_material = column
            .fields
            .iter()
            .any(|(name, _)| name == TYPE_MATERIAL_FIELD);
        if !(unnamed_only && has_material) {
            continue;
        }
        let joined: Vec<u32> = column
            .fields
            .iter()
            .filter_map(|(name, value)| match value {
                InstanceField::ElementId { id, .. } if name == COLUMN_JOINED_WALL_FIELD => {
                    Some(*id)
                }
                _ => None,
            })
            .collect();
        let sets: Vec<&BTreeSet<String>> = joined
            .iter()
            .filter_map(|id| wall_materials.get(id))
            .collect();
        let Some(first) = sets.first() else {
            continue;
        };
        if sets.len() != joined.len() || sets.iter().any(|set| set != first) {
            continue;
        }
        let names = (*first).clone();
        column
            .fields
            .retain(|(name, _)| name != TYPE_MATERIAL_FIELD);
        for name in names {
            column
                .fields
                .push((TYPE_MATERIAL_FIELD.into(), InstanceField::String(name)));
        }
    }
}

/// Every material the file declares, from its `OST_Materials` record
/// (RE-28), with the name (RE-58) and shading colour (RE-53) read from its
/// own object, in ElementId order; on Revit 2023 from its material objects
/// (RE-113, RE-116). A record whose name is not found is left out. `None`
/// on a release those layouts are not measured on, where the partition
/// display-name strings stand in ([`materials_from_names`]).
pub(super) fn materials_from_records(
    rf: &mut RevitFile,
    revit_version: u32,
) -> Option<Vec<DecodedElement>> {
    use crate::partition_materials as pm;
    let revit_2023 = crate::partition_element_records_2023::is_32bit_release(revit_version);
    if !pm::MATERIALS_SUPPORTED_REVIT_VERSIONS.contains(&revit_version) && !revit_2023 {
        return None;
    }
    let declared = crate::elem_table::declared_ids(&crate::elem_table::parse_records(rf).ok()?);
    // Revit 2023 has no material records; its materials are the objects
    // carrying the 2023 material tag (RE-113).
    let (ids, names) = if revit_2023 {
        pm::scan_materials_32(rf, revit_version, &declared)
    } else {
        let ids: BTreeSet<u32> = crate::partition_type_records::scan_type_records(
            rf,
            revit_version,
            crate::partition_type_records::OST_MATERIALS,
            &declared,
        )
        .ok()?
        .iter()
        .map(|record| record.element_id)
        .collect();
        (
            ids,
            pm::scan_material_names(rf, revit_version, &declared).ok()?,
        )
    };
    let appearances = pm::scan_material_appearances(rf, revit_version, &declared).ok()?;
    Some(
        ids.into_iter()
            .filter_map(|id| {
                let name = names.get(&id)?;
                let mut fields = vec![("m_name".into(), InstanceField::String(name.clone()))];
                if let Some(appearance) = appearances.get(&id) {
                    fields.push((
                        "m_color".into(),
                        InstanceField::Integer {
                            value: i64::from(appearance.color_packed()),
                            signed: false,
                            size: 4,
                        },
                    ));
                    fields.push((
                        "m_transparency".into(),
                        InstanceField::Float {
                            value: f64::from(appearance.transparency),
                            size: 4,
                        },
                    ));
                }
                fields.push((
                    "m_source".into(),
                    InstanceField::String("partition_materials".into()),
                ));
                Some(DecodedElement {
                    id: Some(id),
                    class: "Material".into(),
                    fields,
                    byte_range: 0..0,
                    provenance: ElementProvenance::partition(
                        "partition",
                        0,
                        "partition_materials",
                        "partition_materials::material_record",
                        0.9,
                        None::<String>,
                    ),
                })
            })
            .collect(),
    )
}

pub(super) fn materials_from_names(
    name_set: &BTreeSet<(NameBucket, String)>,
) -> Vec<DecodedElement> {
    name_set
        .iter()
        .filter(|(b, n)| *b == NameBucket::MaterialLike && is_strict_material_name(n))
        .enumerate()
        .map(|(i, (_, name))| {
            DecodedElement {
                id: None,
                class: "Material".into(),
                fields: vec![
                    ("m_name".into(), InstanceField::String(name.clone())),
                    (
                        "m_source".into(),
                        InstanceField::String("partition_schema_mvp".into()),
                    ),
                ],
                byte_range: i..i,
                provenance: Default::default(),
            }
            .with_provenance(ElementProvenance::partition(
                "partition",
                i,
                "partition_schema_mvp",
                "partition_schema_mvp::material_name",
                0.6,
                None::<String>,
            ))
        })
        .collect()
}

/// Reject schedule / schema / compound material path noise.
pub fn is_strict_material_name(s: &str) -> bool {
    let t = s.trim();
    if t.len() < 3 || t.len() > 48 {
        return false;
    }
    if t.contains(':') || t.contains('/') || t.contains('\\') || t.contains('.') {
        return false;
    }
    let lower = t.to_ascii_lowercase();
    if lower.contains("schema") || lower.ends_with(" material") || lower.contains("default") {
        return false;
    }
    // Keep classify_name's material bucket; just tighten.
    classify_name(t) == Some(NameBucket::MaterialLike)
}
