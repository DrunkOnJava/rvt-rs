//! The layers of walls and slabs, and the materials they name.

use super::*;

/// The layers [`WALL_LAYERS_FIELD`] records on a wall with no
/// [`WALL_EXTERIOR_FIELDS`]: its type's, where its data does not place it
/// (RE-88).
pub fn unplaced_wall_layers_from_fields(
    fields: &[(String, InstanceField)],
) -> Option<Vec<crate::ifc::LayerBand>> {
    if fields
        .iter()
        .any(|(name, _)| WALL_EXTERIOR_FIELDS.contains(&name.as_str()))
    {
        return None;
    }
    fields
        .iter()
        .find_map(|(name, value)| (name == WALL_LAYERS_FIELD).then_some(value))
        .and_then(layer_bands_from_field)
}

/// The total width of the layers [`WALL_LAYERS_FIELD`] records on a wall,
/// its type's thickness, feet: Revit's `Wall.Width`, which revit-ifc's
/// `CreateWallBaseQuantities` writes as the wall's `Width` (CLEANROOM.md
/// source 5) whether or not the wall is drawn from its centreline (#1016).
pub fn wall_layers_width_from_fields(fields: &[(String, InstanceField)]) -> Option<f64> {
    let bands = fields
        .iter()
        .find_map(|(name, value)| (name == WALL_LAYERS_FIELD).then_some(value))
        .and_then(layer_bands_from_field)?;
    let width: f64 = bands.iter().map(|band| band.width_feet).sum();
    (width.is_finite() && width > 0.0).then_some(width)
}

/// The layers of a [`WALL_LAYERS_FIELD`] or [`SLAB_LAYERS_FIELD`] value.
pub(super) fn layer_bands_from_field(value: &InstanceField) -> Option<Vec<crate::ifc::LayerBand>> {
    let InstanceField::Vector(items) = value else {
        return None;
    };
    let bands = items
        .iter()
        .map(|item| match item {
            InstanceField::Vector(parts) => match parts.as_slice() {
                [
                    InstanceField::Float { value: width, .. },
                    InstanceField::Integer { value: colour, .. },
                    InstanceField::Float {
                        value: transparency,
                        ..
                    },
                    rest @ ..,
                ] => Some(crate::ifc::LayerBand {
                    width_feet: *width,
                    color_packed: u32::try_from(*colour).ok(),
                    transparency: *transparency,
                    name: match rest {
                        [InstanceField::String(name)] => Some(name.clone()),
                        _ => None,
                    },
                }),
                _ => None,
            },
            _ => None,
        })
        .collect::<Option<Vec<_>>>()?;
    (!bands.is_empty()).then_some(bands)
}

/// The field form of a type's layers, each with its material's shading and,
/// where it is read, its name (RE-58); membranes, which have no width, are
/// left out.
pub(super) fn layer_bands_field(
    layers: &[crate::partition_compound_structure::CompoundLayer],
    appearances: &BTreeMap<u32, crate::partition_materials::MaterialAppearance>,
    names: &BTreeMap<u32, String>,
) -> InstanceField {
    InstanceField::Vector(
        layers
            .iter()
            .filter(|layer| layer.width_feet > 0.0)
            .map(|layer| {
                let appearance = layer.material.and_then(|m| appearances.get(&m));
                let mut band = vec![
                    InstanceField::Float {
                        value: layer.width_feet,
                        size: 8,
                    },
                    InstanceField::Integer {
                        value: appearance.map_or(-1, |a| i64::from(a.color_packed())),
                        signed: true,
                        size: 8,
                    },
                    InstanceField::Float {
                        value: appearance.map_or(0.0, |a| f64::from(a.transparency)),
                        size: 8,
                    },
                ];
                if let Some(name) = layer.material.and_then(|m| names.get(&m)) {
                    band.push(InstanceField::String(name.clone()));
                }
                InstanceField::Vector(band)
            })
            .collect(),
    )
}

/// The layers [`WALL_LAYERS_FIELD`] and [`WALL_EXTERIOR_FIELDS`], or
/// [`SLAB_LAYERS_FIELD`], record.
pub fn element_layers_from_fields(
    fields: &[(String, InstanceField)],
) -> Option<crate::ifc::ElementLayers> {
    let field = |wanted: &str| {
        fields
            .iter()
            .find_map(|(name, value)| (name == wanted).then_some(value))
    };
    if let Some(layers) = field(SLAB_LAYERS_FIELD).and_then(layer_bands_from_field) {
        return Some(crate::ifc::ElementLayers {
            exterior_normal: [0.0, 0.0],
            layers,
            stacked: true,
            system_family: None,
        });
    }
    let float = |wanted: &str| match field(wanted) {
        Some(InstanceField::Float { value, .. }) => Some(*value),
        _ => None,
    };
    let [x, y] = WALL_EXTERIOR_FIELDS.map(float);
    let layers = field(WALL_LAYERS_FIELD).and_then(layer_bands_from_field)?;
    Some(crate::ifc::ElementLayers {
        exterior_normal: [x?, y?],
        layers,
        stacked: false,
        system_family: None,
    })
}

/// The materials layers may name and their names, on `revit_version`:
/// the `OST_Materials` type records and RE-58's names on 2024 and 2025, and
/// RE-113's material objects and names on 2023.
pub(super) fn materials_and_names(
    rf: &mut RevitFile,
    revit_version: u32,
    declared: &BTreeSet<u32>,
) -> Option<(BTreeSet<u32>, BTreeMap<u32, String>)> {
    if crate::partition_element_records_2023::is_32bit_release(revit_version) {
        return Some(crate::partition_materials::scan_materials_32(
            rf,
            revit_version,
            declared,
        ));
    }
    let materials = crate::partition_type_records::scan_type_records(
        rf,
        revit_version,
        crate::partition_type_records::OST_MATERIALS,
        declared,
    )
    .unwrap_or_default()
    .iter()
    .map(|record| record.element_id)
    .collect();
    let names =
        crate::partition_materials::scan_material_names(rf, revit_version, declared).ok()?;
    Some((materials, names))
}

/// Give each floor, building pad, roof and ceiling its type's layers, top
/// first, each with its material's shading (RE-57), where they add up to its
/// record box's height: the plate is then exactly its layers. A roof drawn
/// along its slope (RE-56) is left out.
pub(super) fn attach_slab_layers(
    rf: &mut RevitFile,
    revit_version: u32,
    mut elements: Vec<&mut DecodedElement>,
) {
    use crate::partition_compound_structure as pcs;
    if pcs::layer_layout(revit_version).is_none() {
        return;
    }
    let type_of = |element: &DecodedElement| {
        element.fields.iter().find_map(|(name, value)| match value {
            InstanceField::ElementId { id, .. } if name == TYPE_ID_FIELD => Some(*id),
            _ => None,
        })
    };
    elements.retain(|element| {
        STACKED_LAYER_CLASSES.contains(&element.class.as_str())
            && type_of(element).is_some()
            && roof_slope_from_fields(&element.fields).is_none()
    });
    if elements.is_empty() {
        return;
    }
    let types: BTreeSet<u32> = elements
        .iter()
        .filter_map(|element| type_of(element))
        .collect();
    let declared: BTreeSet<u32> = match crate::elem_table::parse_records(rf) {
        Ok(records) => crate::elem_table::declared_ids(&records),
        Err(_) => return,
    };
    let Some((materials, names)) = materials_and_names(rf, revit_version, &declared) else {
        return;
    };
    let (Ok(layers), Ok(appearances)) = (
        pcs::scan_type_layers(rf, revit_version, &types, &materials, &declared),
        crate::partition_materials::scan_material_appearances(rf, revit_version, &declared),
    ) else {
        return;
    };
    // RE-115: a layer that takes its category's material takes the one the
    // document's object styles give the element's category, as a wall's
    // does (RE-91), where it is set and named.
    let mut category_materials: BTreeMap<&str, Option<u32>> = BTreeMap::new();
    for (class, category) in [
        ("Floor", crate::partition_element_records::OST_FLOORS),
        ("Roof", crate::partition_element_records::OST_ROOFS),
        ("Ceiling", crate::partition_element_records::OST_CEILINGS),
    ] {
        let material =
            crate::partition_materials::scan_category_material(rf, revit_version, category)
                .ok()
                .flatten()
                .filter(|id| names.contains_key(id));
        category_materials.insert(class, material);
    }
    for element in elements {
        let Some(type_layers) = type_of(element).and_then(|id| layers.get(&id)) else {
            continue;
        };
        let category_material = category_materials
            .get(element.class.as_str())
            .copied()
            .flatten();
        let type_layers: Vec<pcs::CompoundLayer> = type_layers
            .iter()
            .map(|layer| pcs::CompoundLayer {
                material: layer.material.or(category_material),
                ..*layer
            })
            .collect();
        let type_layers = &type_layers;
        // B41: a ceiling's finish, its finish layers' materials. It is the
        // type's, so it does not wait on the height check below.
        if element.class == "Ceiling" {
            if let Some(finish) = finish_of_layers(type_layers, &names) {
                element
                    .fields
                    .push((COVERING_FINISH_FIELD.into(), InstanceField::String(finish)));
            }
        }
        let height = element.fields.iter().find_map(|(name, value)| match value {
            InstanceField::Float { value, .. } if name == "m_bboxHeight" => Some(*value),
            _ => None,
        });
        let total: f64 = type_layers.iter().map(|layer| layer.width_feet).sum();
        // RE-295: a floor's volume is its area times that total, whatever
        // its box, which a sloped floor's is deeper than.
        if element.class == "Floor" && total.is_finite() && total > 0.0 {
            let count = type_layers
                .iter()
                .filter(|layer| layer.width_feet > 0.0)
                .count();
            element.fields.push((
                SLAB_TYPE_THICKNESS_FIELD.into(),
                InstanceField::Float {
                    value: total,
                    size: 8,
                },
            ));
            element.fields.push((
                SLAB_TYPE_LAYER_COUNT_FIELD.into(),
                InstanceField::Integer {
                    value: count as i64,
                    signed: false,
                    size: 4,
                },
            ));
        }
        if !height.is_some_and(|h| (h - total).abs() <= SLAB_LAYER_HEIGHT_TOLERANCE_FEET) {
            continue;
        }
        let bands = layer_bands_field(type_layers, &appearances, &names);
        if matches!(&bands, InstanceField::Vector(items) if !items.is_empty()) {
            element.fields.push((SLAB_LAYERS_FIELD.into(), bands));
        }
    }
}

/// The names of the materials of `layers`' finish layers (functions 4 and
/// 5, [`crate::partition_compound_structure`]), each followed by `;`, as
/// Revit's export writes `Pset_CoveringCommon.Finish` (6 of 6 RE1 ceilings,
/// one finish layer each). `None` when there is no finish layer, or one
/// whose material's name is not read.
pub(super) fn finish_of_layers(
    layers: &[crate::partition_compound_structure::CompoundLayer],
    names: &BTreeMap<u32, String>,
) -> Option<String> {
    let finishes: Vec<&crate::partition_compound_structure::CompoundLayer> = layers
        .iter()
        .filter(|layer| matches!(layer.function, 4 | 5))
        .collect();
    if finishes.is_empty() {
        return None;
    }
    let mut out = String::new();
    for layer in finishes {
        out.push_str(names.get(&layer.material?)?);
        out.push(';');
    }
    Some(out)
}
