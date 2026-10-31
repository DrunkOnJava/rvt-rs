//! Door and window openings, sizes, cut loops, facings, original symbols and instance axes.

use super::*;
use crate::le_bytes;

/// Give each door and window whose transform (RE-87) is found its facing
/// ([`OPENING_FACING_FIELDS`]).
pub(super) fn attach_opening_facings(
    rf: &mut RevitFile,
    revit_version: u32,
    groups: [&mut Vec<DecodedElement>; 2],
) {
    let ids: BTreeSet<u32> = groups
        .iter()
        .flat_map(|elements| elements.iter())
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
    for element in groups.into_iter().flat_map(|elements| elements.iter_mut()) {
        let Some(transform) = element.id.and_then(|id| transforms.get(&id)) else {
            continue;
        };
        let [x, y, _] = transform.axes[1];
        for (name, value) in OPENING_FACING_FIELDS.iter().zip([x, y]) {
            element
                .fields
                .push(((*name).into(), InstanceField::Float { value, size: 8 }));
        }
    }
}

/// The plan facing [`OPENING_FACING_FIELDS`] record.
pub fn opening_facing_from_fields(fields: &[(String, InstanceField)]) -> Option<[f64; 2]> {
    let float = |wanted: &str| {
        fields.iter().find_map(|(name, value)| match value {
            InstanceField::Float { value, .. } if name == wanted => Some(*value),
            _ => None,
        })
    };
    let [x, y] = OPENING_FACING_FIELDS.map(float);
    Some([x?, y?])
}

/// Give each family instance ([`REVIT_CLASS_FIELD`] `FamilyInstance`, and
/// each curtain mullion and panel) its original symbol ([`ORIGINAL_SYMBOL_FIELD`], RE-167): the ElementId its
/// `GElement` data object holds [`ORIGINAL_SYMBOL_FROM_END`] bytes before its
/// end, when the copies in the highest-numbered partition stream holding one
/// give the same one and it is an [`ORIGINAL_SYMBOL_CLASSES`] element's, or
/// [`INSTANCE_GEOMETRY`]. On the RE1 models that is the element whose GlobalId Revit
/// gives the type of every one of 162 family instances; it is the symbol
/// itself for most, another FamilySymbol (often the instance's id plus one)
/// for many furniture, fittings, mullions and panels. Revit 2024 and later.
pub(super) fn attach_original_symbols<const N: usize>(
    rf: &mut RevitFile,
    revit_version: u32,
    groups: [&mut Vec<DecodedElement>; N],
) {
    if revit_version < 2024 {
        return;
    }
    // Curtain mullions and panels are family instances too, of classes of
    // their own.
    let is_instance = |element: &DecodedElement| {
        CURTAIN_AXES_CLASSES.contains(&element.class.as_str())
            || element.fields.iter().any(|(name, value)| {
                name == REVIT_CLASS_FIELD
                    && matches!(value, InstanceField::String(class) if class == "FamilyInstance")
            })
    };
    let instances: BTreeSet<u32> = groups
        .iter()
        .flat_map(|elements| elements.iter())
        .filter(|element| is_instance(element))
        .filter_map(|element| element.id)
        .collect();
    if instances.is_empty() {
        return;
    }
    let Ok(classes) = rf.memoised_schema_classes() else {
        return;
    };
    let tag_of = |name: &str| {
        classes
            .classes
            .iter()
            .find(|class| class.name == name)
            .map(|class| u32::from(class.tag))
    };
    let Some(element_tag) = tag_of("GElement") else {
        return;
    };
    let symbol_tags: BTreeSet<u32> = ORIGINAL_SYMBOL_CLASSES
        .iter()
        .filter_map(|name| tag_of(name))
        .collect();
    // Each instance's GElement values, by the number of the partition stream
    // holding the copy.
    let mut held: BTreeMap<u32, BTreeMap<u32, BTreeSet<u32>>> = BTreeMap::new();
    let mut symbols: BTreeSet<u32> = BTreeSet::new();
    for stream in rf.partition_stream_names() {
        let Ok(inflated) = rf.inflated_partition(&stream) else {
            continue;
        };
        let partition = stream
            .rsplit('/')
            .next()
            .and_then(|number| number.parse::<u32>().ok())
            .unwrap_or(0);
        let buf = inflated.bytes();
        let Ok(objects) = rf.memoised_partition_data_objects(&stream) else {
            continue;
        };
        for &(p, object) in objects.iter() {
            let class = object.class & 0xffff;
            if symbol_tags.contains(&class) {
                symbols.insert(object.element_id);
            } else if class == element_tag && instances.contains(&object.element_id) {
                let Some(at) = object.end.checked_sub(ORIGINAL_SYMBOL_FROM_END) else {
                    continue;
                };
                if at < p {
                    continue;
                }
                let Some(value) = le_bytes::u32_at(buf, at) else {
                    continue;
                };
                held.entry(object.element_id)
                    .or_default()
                    .entry(partition)
                    .or_default()
                    .insert(value);
            }
        }
    }
    for element in groups.into_iter().flat_map(|elements| elements.iter_mut()) {
        // Copies in two partitions can disagree; the copy in the highest-
        // numbered one is the one Revit's export goes by (Core Interior's
        // doors: Partitions/59 over 46 and 51).
        let Some((_, values)) = element
            .id
            .and_then(|id| held.get(&id))
            .and_then(|by_partition| by_partition.iter().next_back())
        else {
            continue;
        };
        let mut values = values.iter();
        if let (Some(&original), None) = (values.next(), values.next()) {
            if symbols.contains(&original) || original == INSTANCE_GEOMETRY {
                element.fields.push((
                    ORIGINAL_SYMBOL_FIELD.into(),
                    InstanceField::ElementId {
                        tag: 0,
                        id: original,
                    },
                ));
            }
        }
    }
}

/// Give each family instance ([`REVIT_CLASS_FIELD`] `FamilyInstance`) with
/// a vertical axis that is turned off the model's axes by at least
/// [`INSTANCE_TURN_MIN_RADIANS`] its plan direction
/// ([`crate::partition_instance_transforms::InstanceTransform::plan_axis`],
/// RE-87, RE-90). Any other keeps its record box as it is.
pub(super) fn attach_instance_axes(
    rf: &mut RevitFile,
    revit_version: u32,
    groups: [&mut Vec<DecodedElement>; 3],
) {
    let is_instance = |element: &DecodedElement| {
        element.fields.iter().any(|(name, value)| {
            name == REVIT_CLASS_FIELD
                && matches!(value, InstanceField::String(class) if class == "FamilyInstance")
        })
    };
    let ids: BTreeSet<u32> = groups
        .iter()
        .flat_map(|elements| elements.iter())
        .filter(|element| is_instance(element))
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
    for element in groups.into_iter().flat_map(|elements| elements.iter_mut()) {
        let Some((transform, [x, y])) = element
            .id
            .and_then(|id| transforms.get(&id))
            .and_then(|transform| Some((transform, transform.plan_axis()?)))
        else {
            continue;
        };
        let turn = y.abs().atan2(x.abs());
        if turn.min(std::f64::consts::FRAC_PI_2 - turn) < INSTANCE_TURN_MIN_RADIANS {
            continue;
        }
        let [ox, oy, _] = transform.origin;
        for (name, value) in INSTANCE_X_AXIS_FIELDS
            .iter()
            .zip([x, y])
            .chain(INSTANCE_ORIGIN_FIELDS.iter().zip([ox, oy]))
        {
            element
                .fields
                .push(((*name).into(), InstanceField::Float { value, size: 8 }));
        }
    }
}

/// A door's or window's opening as [`FILLER_OPENING_FIELDS`] records it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FillerOpening {
    /// Plan point the opening is centred on, model feet.
    pub centre: [f64; 2],
    /// Unit plan direction of its width.
    pub axis: [f64; 2],
    /// Elevation of its bottom, model feet.
    pub base_feet: f64,
    /// Feet.
    pub width_feet: f64,
    /// Feet.
    pub height_feet: f64,
}

/// Give each upright window whose type's Width, Height and Default Sill
/// Height are read its opening (RE-93,
/// [`crate::partition_type_parameters::type_window_openings`]).
pub(super) fn attach_window_openings(
    rf: &mut RevitFile,
    revit_version: u32,
    windows: &mut [DecodedElement],
) {
    let type_of = |element: &DecodedElement| {
        element.fields.iter().find_map(|(name, value)| match value {
            InstanceField::ElementId { id, .. } if name == TYPE_ID_FIELD => Some(*id),
            _ => None,
        })
    };
    let types: BTreeSet<u32> = windows.iter().filter_map(type_of).collect();
    let openings =
        crate::partition_type_parameters::type_window_openings(rf, revit_version, &types);
    if openings.is_empty() {
        return;
    }
    let ids: BTreeSet<u32> = windows
        .iter()
        .filter(|element| type_of(element).is_some_and(|id| openings.contains_key(&id)))
        .filter_map(|element| element.id)
        .collect();
    let Ok(transforms) =
        crate::partition_instance_transforms::scan_instance_transforms(rf, revit_version, &ids)
    else {
        return;
    };
    for element in windows.iter_mut() {
        let Some((opening, transform)) = type_of(element)
            .and_then(|id| openings.get(&id))
            .zip(element.id.and_then(|id| transforms.get(&id)))
        else {
            continue;
        };
        push_filler_opening(
            element,
            transform,
            0.0,
            opening.default_sill_feet,
            opening.width_feet,
            opening.height_feet,
        );
    }
}

pub(super) fn attach_door_openings(
    rf: &mut RevitFile,
    revit_version: u32,
    doors: &mut [DecodedElement],
) {
    let type_of = |element: &DecodedElement| {
        element.fields.iter().find_map(|(name, value)| match value {
            InstanceField::ElementId { id, .. } if name == TYPE_ID_FIELD => Some(*id),
            _ => None,
        })
    };
    let box_height = |element: &DecodedElement| {
        element.fields.iter().find_map(|(name, value)| match value {
            InstanceField::Float { value, .. } if name == "m_bboxHeight" => Some(*value),
            _ => None,
        })
    };
    let types: BTreeSet<u32> = doors.iter().filter_map(type_of).collect();
    let openings = crate::partition_type_parameters::type_door_openings(rf, revit_version, &types);
    // A door whose type draws no geometry (RE-84) is an opening alone, which
    // Revit does not cut at its rough size: RE1 Architecture's cased
    // opening is 1.0 m wide against a Rough Width of 1.1 m.
    let rough = |element: &DecodedElement| {
        if element
            .fields
            .iter()
            .any(|(name, _)| name == TYPE_WITHOUT_GEOMETRY_FIELD)
        {
            return None;
        }
        let opening = type_of(element).and_then(|id| openings.get(&id))?;
        let height = box_height(element)?;
        ((height - opening.rough_height_feet).abs() <= DOOR_ROUGH_HEIGHT_TOLERANCE_FEET)
            .then_some(*opening)
    };
    let ids: BTreeSet<u32> = doors
        .iter()
        .filter(|element| rough(element).is_some())
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
    for element in doors.iter_mut() {
        let Some((opening, transform)) =
            rough(element).zip(element.id.and_then(|id| transforms.get(&id)))
        else {
            continue;
        };
        push_filler_opening(
            element,
            transform,
            0.0,
            0.0,
            opening.rough_width_feet,
            opening.rough_height_feet,
        );
    }
}

/// The Width and Height [`DRAWN_SIZE_FIELDS`] record, feet.
pub fn drawn_size_from_fields(fields: &[(String, InstanceField)]) -> Option<[f64; 2]> {
    let [width, height] = DRAWN_SIZE_FIELDS.map(|wanted| {
        fields.iter().find_map(|(name, value)| match value {
            InstanceField::Float { value, .. } if name == wanted => Some(*value),
            _ => None,
        })
    });
    Some([width?, height?])
}

/// The size [`DOOR_TYPE_SIZE_FIELDS`] record.
pub fn door_type_size_from_fields(
    fields: &[(String, InstanceField)],
) -> Option<crate::partition_type_parameters::DoorTypeSize> {
    let [width, height] = DOOR_TYPE_SIZE_FIELDS.map(|wanted| {
        fields.iter().find_map(|(name, value)| match value {
            InstanceField::Float { value, .. } if name == wanted => Some(*value),
            _ => None,
        })
    });
    Some(crate::partition_type_parameters::DoorTypeSize {
        width_feet: width?,
        height_feet: height?,
    })
}

/// Give each door whose type's Width and Height are read
/// ([`crate::partition_type_parameters::type_door_sizes`]) that size (RE-187),
/// whether or not its opening is read. A door that is an opening alone
/// (RE-84) is written as that opening, and gets none.
pub(super) fn attach_door_sizes(
    rf: &mut RevitFile,
    revit_version: u32,
    doors: &mut [DecodedElement],
) {
    let type_of = |element: &DecodedElement| {
        element.fields.iter().find_map(|(name, value)| match value {
            InstanceField::ElementId { id, .. } if name == TYPE_ID_FIELD => Some(*id),
            _ => None,
        })
    };
    let types: BTreeSet<u32> = doors.iter().filter_map(type_of).collect();
    let sizes = crate::partition_type_parameters::type_door_sizes(rf, revit_version, &types);
    for element in doors.iter_mut() {
        if element
            .fields
            .iter()
            .any(|(name, _)| name == TYPE_WITHOUT_GEOMETRY_FIELD)
        {
            continue;
        }
        let Some(size) = type_of(element).and_then(|id| sizes.get(&id)).copied() else {
            continue;
        };
        for (name, value) in DOOR_TYPE_SIZE_FIELDS
            .iter()
            .zip([size.width_feet, size.height_feet])
        {
            element
                .fields
                .push(((*name).into(), InstanceField::Float { value, size: 8 }));
        }
    }
}

/// The width and height [`INSTANCE_BOX_SIZE_FIELDS`] record, feet.
pub fn instance_box_size_from_fields(fields: &[(String, InstanceField)]) -> Option<[f64; 2]> {
    let [width, height] = INSTANCE_BOX_SIZE_FIELDS.map(|wanted| {
        fields.iter().find_map(|(name, value)| match value {
            InstanceField::Float { value, .. } if name == wanted => Some(*value),
            _ => None,
        })
    });
    Some([width?, height?])
}

/// The extents [`CUT_SOLID_SIZE_FIELDS`] record, feet.
pub fn cut_solid_size_from_fields(fields: &[(String, InstanceField)]) -> Option<[f64; 3]> {
    let [width, depth, height] = CUT_SOLID_SIZE_FIELDS.map(|wanted| {
        fields.iter().find_map(|(name, value)| match value {
            InstanceField::Float { value, .. } if name == wanted => Some(*value),
            _ => None,
        })
    });
    Some([width?, depth?, height?])
}

/// The width and height [`CUT_LOOP_SIZE_FIELDS`] record, feet.
pub fn cut_loop_size_from_fields(fields: &[(String, InstanceField)]) -> Option<[f64; 2]> {
    let [width, height] = CUT_LOOP_SIZE_FIELDS.map(|wanted| {
        fields.iter().find_map(|(name, value)| match value {
            InstanceField::Float { value, .. } if name == wanted => Some(*value),
            _ => None,
        })
    });
    Some([width?, height?])
}

/// Give each door and window the extents of the outline the symbol that
/// draws it cuts in its host
/// ([`crate::native_symbol_cuts::instance_cut_loops`]), which Revit's export
/// writes as its `OverallWidth` and `OverallHeight` (RE-215). A door that is
/// an opening alone (RE-84) is written as that opening, and gets none.
pub(super) fn attach_cut_loop_sizes(
    rf: &mut RevitFile,
    revit_version: u32,
    groups: [&mut Vec<DecodedElement>; 2],
    walls: &BTreeSet<u32>,
    plain_walls: &BTreeSet<u32>,
) {
    if !CUT_LOOP_REVIT_VERSIONS.contains(&revit_version) {
        return;
    }
    let mut elements: Vec<&mut DecodedElement> = groups
        .into_iter()
        .flat_map(|group| group.iter_mut())
        .filter(|element| {
            !element
                .fields
                .iter()
                .any(|(name, _)| name == TYPE_WITHOUT_GEOMETRY_FIELD)
        })
        .collect();
    let ids: BTreeSet<u64> = elements
        .iter()
        .filter_map(|element| element.id.map(u64::from))
        .collect();
    let Ok((loops, uncut, drawn_by, hosts, solids)) =
        crate::native_symbol_cuts::instance_cuts_and_symbols(rf, &ids)
    else {
        return;
    };
    // RE-245: a door or window set in a wall is set in the one its current
    // native record names, which the reference list's nearest wall
    // (`attach_host_candidates`) is not where the list names several.
    for element in elements.iter_mut() {
        let Some(native) = element
            .id
            .and_then(|id| hosts.get(&u64::from(id)))
            .and_then(|host| u32::try_from(*host).ok())
        else {
            continue;
        };
        let bound = element
            .fields
            .iter_mut()
            .find_map(|(name, value)| match value {
                InstanceField::ElementId { id, .. } if name == OPENING_HOST_FIELD => Some(id),
                _ => None,
            });
        if let Some(bound) = bound {
            if *bound != native && plain_walls.contains(&native) {
                *bound = native;
            }
        }
    }
    // RE-225: a door or window set in a wall whose symbol cuts a solid and
    // no loop carries the solid's extents, which Revit's export cuts its
    // opening to (the BIM4LCA office's windows).
    for element in elements.iter_mut() {
        let Some(id) = element.id.map(u64::from) else {
            continue;
        };
        let in_wall = hosts
            .get(&id)
            .and_then(|host| u32::try_from(*host).ok())
            .is_some_and(|host| walls.contains(&host));
        let Some(solid) = solids.get(&id).filter(|_| in_wall) else {
            continue;
        };
        for (name, value) in
            CUT_SOLID_SIZE_FIELDS
                .iter()
                .zip([solid.x_feet, solid.y_feet, solid.z_feet])
        {
            element
                .fields
                .push(((*name).into(), InstanceField::Float { value, size: 8 }));
        }
    }
    for element in elements.iter_mut() {
        let Some(cut) = element.id.and_then(|id| loops.get(&u64::from(id))) else {
            continue;
        };
        for (name, value) in CUT_LOOP_SIZE_FIELDS
            .iter()
            .zip([cut.width_feet(), cut.height_feet()])
        {
            element
                .fields
                .push(((*name).into(), InstanceField::Float { value, size: 8 }));
        }
    }
    // #1016: its base quantities' size, where the symbol that draws it and
    // its type have the same Width and Height. revit-ifc reads the
    // instance's own Width and Height parameters before its type's; where
    // its own parameters make a symbol of its own, which of them that
    // symbol's size stands for is not read (2 office doors take theirs from
    // it, 5 from their type), so those get none.
    let type_of = |element: &DecodedElement| {
        element.fields.iter().find_map(|(name, value)| match value {
            InstanceField::ElementId { id, .. } if name == TYPE_ID_FIELD => Some(*id),
            _ => None,
        })
    };
    let symbols: BTreeSet<u32> = drawn_by
        .values()
        .filter_map(|symbol| u32::try_from(*symbol).ok())
        .chain(elements.iter().filter_map(|element| type_of(element)))
        .collect();
    let sizes = crate::partition_type_parameters::type_door_sizes(rf, revit_version, &symbols);
    for element in elements.iter_mut() {
        let drawn = element
            .id
            .and_then(|id| drawn_by.get(&u64::from(id)))
            .and_then(|symbol| u32::try_from(*symbol).ok())
            .and_then(|symbol| sizes.get(&symbol));
        let typed = type_of(element).and_then(|id| sizes.get(&id));
        let Some(size) = drawn.zip(typed).and_then(|(drawn, typed)| {
            ((drawn.width_feet - typed.width_feet).abs() < 1e-9
                && (drawn.height_feet - typed.height_feet).abs() < 1e-9)
                .then_some(drawn)
        }) else {
            continue;
        };
        for (name, value) in DRAWN_SIZE_FIELDS
            .iter()
            .zip([size.width_feet, size.height_feet])
        {
            element
                .fields
                .push(((*name).into(), InstanceField::Float { value, size: 8 }));
        }
    }
    // #227: where no opening is read yet, the opening is the loop, carried
    // by the instance's transform (RE-87); across the wall it keeps the
    // host's band, which the loop's plane and its mirror bound.
    let has_opening =
        |element: &DecodedElement| filler_opening_from_fields(&element.fields).is_some();
    let open: BTreeSet<u32> = elements
        .iter()
        .filter(|element| !has_opening(element))
        .filter_map(|element| element.id)
        .filter(|id| loops.contains_key(&u64::from(*id)))
        .collect();
    // Only where the symbol cuts no outline at all: one whose outline is not
    // read (an arc, several loops) keeps the sizes it had.
    let loopless: BTreeSet<u32> = elements
        .iter()
        .filter_map(|element| element.id)
        .filter(|id| uncut.contains(&u64::from(*id)))
        .collect();
    if open.is_empty() && loopless.is_empty() {
        return;
    }
    let Ok(transforms) = crate::partition_instance_transforms::scan_instance_transforms(
        rf,
        revit_version,
        &open.union(&loopless).copied().collect(),
    ) else {
        return;
    };
    // #810: a door or window whose symbol cuts no loop is sized by its
    // record box, as Revit's export sizes it: its width along its own X
    // axis and, in a wall or a curtain wall that its element record names,
    // its height through the box: all 269 windows of the BIM4LCA office, 71
    // of its doors and RE1's curtain-wall door. Any other's height is the
    // box's extent along its own Y axis, as revit-ifc's
    // `GetDoorWindowDimensionFromSymbol` measures the height of a family not
    // hosted by a wall (CLEANROOM.md source 5): the office's four roof
    // hatches and four doors of type 3267242. An instance turned off the
    // model's axes, whose box is not along its own, gets no size.
    let mut in_wall: BTreeSet<u32> = BTreeSet::new();
    for element in elements.iter() {
        let Some(id) = element.id.filter(|id| loopless.contains(id)) else {
            continue;
        };
        if let Some((references, _)) = record_references(rf, element) {
            if references
                .iter()
                .any(|r| u32::try_from(*r).is_ok_and(|r| walls.contains(&r)))
            {
                in_wall.insert(id);
            }
        }
    }
    for element in elements.iter_mut() {
        let Some(id) = element.id.filter(|id| loopless.contains(id)) else {
            continue;
        };
        let (Some(transform), Some(bbox)) = (transforms.get(&id), element_record_bbox(element))
        else {
            continue;
        };
        if !transform.is_upright() {
            continue;
        }
        let Some([ax, ay]) = transform.plan_axis() else {
            continue;
        };
        let (across_x, across_y) = (bbox[3] - bbox[0], bbox[4] - bbox[1]);
        let (width, along_y) = if (ax.abs() - 1.0).abs() <= INSTANCE_AXIS_TOLERANCE {
            (across_x, across_y)
        } else if (ay.abs() - 1.0).abs() <= INSTANCE_AXIS_TOLERANCE {
            (across_y, across_x)
        } else {
            continue;
        };
        let height = if in_wall.contains(&id) {
            bbox[5] - bbox[2]
        } else {
            along_y
        };
        if !(width > 0.0 && height > 0.0) {
            continue;
        }
        for (name, value) in INSTANCE_BOX_SIZE_FIELDS.iter().zip([width, height]) {
            element
                .fields
                .push(((*name).into(), InstanceField::Float { value, size: 8 }));
        }
    }
    for element in elements.iter_mut() {
        let Some(id) = element.id.filter(|id| open.contains(id)) else {
            continue;
        };
        let (Some(cut), Some(transform)) = (loops.get(&u64::from(id)), transforms.get(&id)) else {
            continue;
        };
        push_filler_opening(
            element,
            transform,
            (cut.min[0] + cut.max[0]) / 2.0,
            cut.min[2],
            cut.width_feet(),
            cut.height_feet(),
        );
    }
}

/// Record on an upright door or window the opening centred `along` its X
/// axis from its origin, `base` above the origin ([`FILLER_OPENING_FIELDS`]).
pub(super) fn push_filler_opening(
    element: &mut DecodedElement,
    transform: &crate::partition_instance_transforms::InstanceTransform,
    along: f64,
    base: f64,
    width: f64,
    height: f64,
) {
    if !transform.is_upright() {
        return;
    }
    let Some([ax, ay]) = transform.plan_axis() else {
        return;
    };
    let [ox, oy, oz] = transform.origin;
    let values = [
        ox + ax * along,
        oy + ay * along,
        ax,
        ay,
        oz + base,
        width,
        height,
    ];
    for (name, value) in FILLER_OPENING_FIELDS.iter().zip(values) {
        element
            .fields
            .push(((*name).into(), InstanceField::Float { value, size: 8 }));
    }
}

/// The opening [`FILLER_OPENING_FIELDS`] record.
pub fn filler_opening_from_fields(fields: &[(String, InstanceField)]) -> Option<FillerOpening> {
    let float = |wanted: &str| {
        fields.iter().find_map(|(name, value)| match value {
            InstanceField::Float { value, .. } if name == wanted => Some(*value),
            _ => None,
        })
    };
    let [x, y, ax, ay, base, width, height] = FILLER_OPENING_FIELDS.map(float);
    Some(FillerOpening {
        centre: [x?, y?],
        axis: [ax?, ay?],
        base_feet: base?,
        width_feet: width?,
        height_feet: height?,
    })
}

/// The plan origin [`INSTANCE_ORIGIN_FIELDS`] record.
pub fn instance_origin_from_fields(fields: &[(String, InstanceField)]) -> Option<[f64; 2]> {
    let float = |wanted: &str| {
        fields.iter().find_map(|(name, value)| match value {
            InstanceField::Float { value, .. } if name == wanted => Some(*value),
            _ => None,
        })
    };
    let [x, y] = INSTANCE_ORIGIN_FIELDS.map(float);
    Some([x?, y?])
}

/// The plan X axis [`INSTANCE_X_AXIS_FIELDS`] record.
pub fn instance_x_axis_from_fields(fields: &[(String, InstanceField)]) -> Option<[f64; 2]> {
    let float = |wanted: &str| {
        fields.iter().find_map(|(name, value)| match value {
            InstanceField::Float { value, .. } if name == wanted => Some(*value),
            _ => None,
        })
    };
    let [x, y] = INSTANCE_X_AXIS_FIELDS.map(float);
    Some([x?, y?])
}
