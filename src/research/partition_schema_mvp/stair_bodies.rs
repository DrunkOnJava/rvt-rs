//! Stair run, spiral run, landing and tread bodies.

use super::*;

/// A stair run's treads and risers as [`STAIR_RUN_PROFILE_FIELD`] and its
/// companions record them (RE-52).
#[derive(Debug, Clone, PartialEq)]
pub struct StairRunBody {
    /// Where the run's sketch starts, model feet.
    pub origin: [f64; 3],
    /// Unit plan direction the run climbs in.
    pub climb: [f64; 2],
    /// Unit plan direction across the run.
    pub across: [f64; 2],
    /// Feet.
    pub width_feet: f64,
    /// Side view, `[along, up]` feet from `origin`, counter-clockwise.
    pub profile: Vec<[f64; 2]>,
    /// The side views of the run's further solids, where it is several
    /// (a riserless run's treads, #357); empty for one.
    pub pieces: Vec<Vec<[f64; 2]>>,
}

/// The run body recorded in `fields`, when every part of it is.
pub fn stair_run_body_from_fields(fields: &[(String, InstanceField)]) -> Option<StairRunBody> {
    let float = |wanted: &str| {
        fields.iter().find_map(|(name, value)| match value {
            InstanceField::Float { value, .. } if name == wanted => Some(*value),
            _ => None,
        })
    };
    let ring = |points: &[InstanceField]| {
        points
            .iter()
            .map(|point| match point {
                InstanceField::Vector(pair) => match pair.as_slice() {
                    [
                        InstanceField::Float { value: u, .. },
                        InstanceField::Float { value: z, .. },
                    ] => Some([*u, *z]),
                    _ => None,
                },
                _ => None,
            })
            .collect::<Option<Vec<[f64; 2]>>>()
    };
    let profile = fields.iter().find_map(|(name, value)| match value {
        InstanceField::Vector(points) if name == STAIR_RUN_PROFILE_FIELD => ring(points),
        _ => None,
    })?;
    let pieces = match fields
        .iter()
        .find(|(name, _)| name == STAIR_RUN_PIECES_FIELD)
    {
        Some((_, InstanceField::Vector(rings))) => rings
            .iter()
            .map(|r| match r {
                InstanceField::Vector(points) => ring(points).filter(|p| p.len() >= 3),
                _ => None,
            })
            .collect::<Option<Vec<_>>>()?,
        Some(_) => return None,
        None => Vec::new(),
    };
    let [ox, oy, oz] = STAIR_RUN_ORIGIN_FIELDS.map(float);
    let [cx, cy] = STAIR_RUN_CLIMB_FIELDS.map(float);
    let [ax, ay] = STAIR_RUN_ACROSS_FIELDS.map(float);
    Some(StairRunBody {
        origin: [ox?, oy?, oz?],
        climb: [cx?, cy?],
        across: [ax?, ay?],
        width_feet: float(STAIR_RUN_WIDTH_FIELD)?,
        profile: (profile.len() >= 3).then_some(profile)?,
        pieces,
    })
}

/// Give each straight stair run with separate treads and risers its side
/// view, from the plan sketch in its own data, its run type and its
/// stair's riser height (RE-52, [`crate::partition_stairs`]), and each run
/// whose run type reads that type's id and name. A run ending with a tread
/// has one riser line more than risers (RE-92); a run whose riser lines do
/// not number its risers so is left to its record box.
pub(super) fn attach_stair_run_bodies(
    rf: &mut RevitFile,
    revit_version: u32,
    products: &mut [DecodedElement],
) {
    use crate::partition_stairs as ps;
    use crate::partition_type_records as ptr;
    if !ps::supports_revit_version(revit_version) {
        if crate::native_stairs::NATIVE_RUN_REVIT_VERSIONS.contains(&revit_version) {
            attach_native_stair_run_bodies(rf, revit_version, products);
        }
        return;
    }
    let float = |element: &DecodedElement, wanted: &str| {
        element.fields.iter().find_map(|(name, value)| match value {
            InstanceField::Float { value, .. } if name == wanted => Some(*value),
            _ => None,
        })
    };
    let integer = |element: &DecodedElement, wanted: &str| {
        element.fields.iter().find_map(|(name, value)| match value {
            InstanceField::Integer { value, .. } if name == wanted => Some(*value),
            _ => None,
        })
    };
    let run_indices: Vec<usize> = products
        .iter()
        .enumerate()
        .filter(|(_, element)| element.class == "StairsRun" && element.id.is_some())
        .map(|(index, _)| index)
        .collect();
    if run_indices.is_empty() {
        return;
    }
    let declared: BTreeSet<u32> = match crate::elem_table::parse_records(rf) {
        Ok(records) => crate::elem_table::declared_ids(&records),
        Err(_) => return,
    };
    let type_records = ptr::scan_type_records(
        rf,
        revit_version,
        crate::partition_element_records::OST_STAIRS_RUNS,
        &declared,
    )
    .unwrap_or_default();
    let run_types = ptr::type_definition_ids(&type_records);
    let mut type_of: BTreeMap<usize, u32> = BTreeMap::new();
    for &index in &run_indices {
        if let Some((references, _)) = record_references(rf, &products[index]) {
            if let Some(type_id) = ptr::unique_type_reference(&references, &run_types) {
                type_of.insert(index, type_id);
            }
        }
    }
    let run_ids: BTreeSet<u32> = run_indices
        .iter()
        .filter_map(|&index| products[index].id)
        .collect();
    let type_ids: BTreeSet<u32> = type_of.values().copied().collect();
    let (Ok(lines), Ok(types)) = (
        ps::scan_run_lines(rf, revit_version, &run_ids),
        ps::scan_run_types(rf, revit_version, &type_ids),
    ) else {
        return;
    };
    let sketches: BTreeMap<u32, ps::RunSketch> = lines
        .iter()
        .filter_map(|(&id, lines)| Some((id, ps::run_sketch(lines)?)))
        .collect();
    let ends = ps::scan_run_ends(rf, revit_version, &sketches).unwrap_or_default();
    // Where each run's underside is cut, from its native record (#357); a
    // run that does not give it is cut at its base, as RE-92 measured.
    let base_cuts = crate::native_stairs::native_run_base_cuts(
        rf,
        &sketches.keys().map(|&id| u64::from(id)).collect(),
    )
    .unwrap_or_default();
    for index in run_indices {
        let element = &mut products[index];
        let Some(run_type) = type_of.get(&index).and_then(|id| types.get(id)) else {
            continue;
        };
        let has_type_name = element
            .fields
            .iter()
            .any(|(name, _)| name == TYPE_NAME_FIELD);
        if let (Some(name), false) = (&run_type.name, has_type_name) {
            element.fields.push((
                TYPE_ID_FIELD.into(),
                InstanceField::ElementId {
                    tag: 0,
                    id: type_of[&index],
                },
            ));
            element
                .fields
                .push((TYPE_NAME_FIELD.into(), InstanceField::String(name.clone())));
        }
        element.fields.push((
            STAIR_NOSING_LENGTH_FIELD.into(),
            InstanceField::Float {
                value: run_type.nosing_length_feet,
                size: 8,
            },
        ));
        let (Some(riser_height), Some(risers)) = (
            float(element, STAIR_RISER_HEIGHT_FIELD),
            integer(element, STAIR_RISER_COUNT_FIELD),
        ) else {
            continue;
        };
        let run_ends = element.id.and_then(|id| ends.get(&id)).copied();
        if let Some(run_ends) = run_ends {
            let treads = risers - 1
                + i64::from(!run_ends.begin_with_riser)
                + i64::from(!run_ends.end_with_riser);
            element.fields.push((
                STAIR_TREAD_COUNT_FIELD.into(),
                InstanceField::Integer {
                    value: treads,
                    signed: false,
                    size: 4,
                },
            ));
        }
        let end_with_riser = run_ends.map(|ends| ends.end_with_riser);
        let riser_lines = risers + i64::from(end_with_riser == Some(false));
        let Some(mut sketch) = element
            .id
            .and_then(|id| sketches.get(&id))
            .filter(|sketch| i64::try_from(sketch.risers.len()) == Ok(riser_lines))
            .cloned()
        else {
            continue;
        };
        if let Some(cut) = element.id.and_then(|id| base_cuts.get(&u64::from(id))) {
            sketch.base_cut_feet = *cut;
        }
        let Some(pieces) = ps::run_side_pieces(&sketch, run_type, riser_height, end_with_riser)
        else {
            continue;
        };
        push_stair_run_body(element, &sketch, &pieces);
    }
}

/// Give each stair the nosing length every one of its runs has, where they
/// all have one and it is the same: revit-ifc's `NosingLengthCalculator`
/// (CLEANROOM.md source 5) takes a stair's from the type of its first run.
pub(super) fn attach_stair_nosing_lengths(products: &mut [DecodedElement]) {
    let float = |element: &DecodedElement, wanted: &str| {
        element.fields.iter().find_map(|(name, value)| match value {
            InstanceField::Float { value, .. } if name == wanted => Some(*value),
            _ => None,
        })
    };
    let whole_of = |element: &DecodedElement| {
        element.fields.iter().find_map(|(name, value)| match value {
            InstanceField::ElementId { id, .. } if name == AGGREGATE_WHOLE_FIELD => Some(*id),
            _ => None,
        })
    };
    let mut lengths: BTreeMap<u32, Vec<Option<f64>>> = BTreeMap::new();
    for element in products.iter().filter(|e| e.class == "StairsRun") {
        if let Some(stair) = whole_of(element) {
            lengths
                .entry(stair)
                .or_default()
                .push(float(element, STAIR_NOSING_LENGTH_FIELD));
        }
    }
    for element in products.iter_mut().filter(|e| e.class == "Stair") {
        let Some(found) = element.id.and_then(|id| lengths.get(&id)) else {
            continue;
        };
        let Some(Some(first)) = found.first().copied() else {
            continue;
        };
        if found.iter().all(|length| *length == Some(first)) {
            element.fields.push((
                STAIR_NOSING_LENGTH_FIELD.into(),
                InstanceField::Float {
                    value: first,
                    size: 8,
                },
            ));
        }
    }
}

/// Record a run's sketch and side view in its fields, where
/// [`stair_run_body_from_fields`] reads them (RE-52).
pub(super) fn push_stair_run_body(
    element: &mut DecodedElement,
    sketch: &crate::partition_stairs::RunSketch,
    pieces: &[Vec<[f64; 2]>],
) {
    let scalar = |value: f64| InstanceField::Float { value, size: 8 };
    let ring = |points: &[[f64; 2]]| {
        InstanceField::Vector(
            points
                .iter()
                .map(|&[u, z]| InstanceField::Vector(vec![scalar(u), scalar(z)]))
                .collect(),
        )
    };
    let Some((profile, rest)) = pieces.split_first() else {
        return;
    };
    for (name, value) in STAIR_RUN_ORIGIN_FIELDS.iter().zip(sketch.origin) {
        element.fields.push(((*name).into(), scalar(value)));
    }
    for (name, value) in STAIR_RUN_CLIMB_FIELDS.iter().zip(sketch.climb) {
        element.fields.push(((*name).into(), scalar(value)));
    }
    for (name, value) in STAIR_RUN_ACROSS_FIELDS.iter().zip(sketch.across) {
        element.fields.push(((*name).into(), scalar(value)));
    }
    element
        .fields
        .push((STAIR_RUN_WIDTH_FIELD.into(), scalar(sketch.width_feet)));
    element
        .fields
        .push((STAIR_RUN_PROFILE_FIELD.into(), ring(profile)));
    if let Some(&last) = sketch.risers.last() {
        element
            .fields
            .push((STAIR_RUN_LENGTH_FIELD.into(), scalar(last)));
    }
    if !rest.is_empty() {
        element.fields.push((
            STAIR_RUN_PIECES_FIELD.into(),
            InstanceField::Vector(rest.iter().map(|r| ring(r)).collect()),
        ));
    }
}

/// Give each Revit 2025 stair run the treads and risers its native records
/// describe (#357, RE-217): its own run type, the one `OST_StairsRuns` type
/// definition its element record's reference list names (RE-28), as
/// [`attach_stair_run_bodies`] reads it on Revit 2024; its sketch and that
/// type's construction from [`crate::native_stairs::native_runs`]; drawn as
/// RE-92 draws a 2024 run ([`crate::partition_stairs::run_side_profile`]). A
/// run whose record names no one run type, whose records do not give its
/// sketch, or whose type the profile does not draw, keeps its box.
pub(super) fn attach_native_stair_run_bodies(
    rf: &mut RevitFile,
    revit_version: u32,
    products: &mut [DecodedElement],
) {
    use crate::partition_type_records as ptr;
    let declared: BTreeSet<u32> = match crate::elem_table::parse_records(rf) {
        Ok(records) => crate::elem_table::declared_ids(&records),
        Err(_) => return,
    };
    let type_records = ptr::scan_type_records(
        rf,
        revit_version,
        crate::partition_element_records::OST_STAIRS_RUNS,
        &declared,
    )
    .unwrap_or_default();
    let run_types = ptr::type_definition_ids(&type_records);
    let mut runs: BTreeMap<u64, u64> = BTreeMap::new();
    for element in products
        .iter()
        .filter(|element| element.class == "StairsRun")
    {
        let (Some(id), Some((references, _))) = (element.id, record_references(rf, element)) else {
            continue;
        };
        if let Some(type_id) = ptr::unique_type_reference(&references, &run_types) {
            runs.insert(u64::from(id), u64::from(type_id));
        }
    }
    let Ok(native) = crate::native_stairs::native_runs(rf, &runs) else {
        return;
    };
    for element in products
        .iter_mut()
        .filter(|element| element.class == "StairsRun")
    {
        let Some(run) = element.id.and_then(|id| native.get(&u64::from(id))) else {
            continue;
        };
        element.fields.push((
            STAIR_NOSING_LENGTH_FIELD.into(),
            InstanceField::Float {
                value: run.run_type.nosing_length_feet,
                size: 8,
            },
        ));
        let Some(pieces) = crate::partition_stairs::run_side_pieces(
            &run.sketch,
            &run.run_type,
            run.riser_height_feet,
            run.end_with_riser,
        ) else {
            continue;
        };
        let has_type_name = element
            .fields
            .iter()
            .any(|(name, _)| name == TYPE_NAME_FIELD);
        if let (Some(name), false, Ok(type_id)) = (
            &run.run_type.name,
            has_type_name,
            u32::try_from(run.type_id),
        ) {
            element.fields.push((
                TYPE_ID_FIELD.into(),
                InstanceField::ElementId {
                    tag: 0,
                    id: type_id,
                },
            ));
            element
                .fields
                .push((TYPE_NAME_FIELD.into(), InstanceField::String(name.clone())));
        }
        push_stair_run_body(element, &run.sketch, &pieces);
    }
}

/// One tread of a [`SpiralRunBody`]: its bottom and top, feet, the angle of
/// the riser line it starts at, below, and the angles at which its other edge
/// meets the outer and inner circles, radians about the centre.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpiralTreadBody {
    pub bottom_feet: f64,
    pub top_feet: f64,
    pub down_angle: f64,
    pub up_angles: [f64; 2],
}

/// A spiral run's treads as [`SPIRAL_RUN_TREADS_FIELD`] and its companions
/// record them (#357).
#[derive(Debug, Clone, PartialEq)]
pub struct SpiralRunBody {
    pub centre: [f64; 2],
    /// Inner and outer radius, feet.
    pub radii_feet: [f64; 2],
    pub treads: Vec<SpiralTreadBody>,
}

/// The spiral run body recorded in `fields`, when every part of it is.
pub fn spiral_run_body_from_fields(fields: &[(String, InstanceField)]) -> Option<SpiralRunBody> {
    let float = |wanted: &str| {
        fields.iter().find_map(|(name, value)| match value {
            InstanceField::Float { value, .. } if name == wanted => Some(*value),
            _ => None,
        })
    };
    let treads = fields.iter().find_map(|(name, value)| match value {
        InstanceField::Vector(treads) if name == SPIRAL_RUN_TREADS_FIELD => Some(treads),
        _ => None,
    })?;
    let treads = treads
        .iter()
        .map(|tread| {
            let InstanceField::Vector(values) = tread else {
                return None;
            };
            let values: Vec<f64> = values
                .iter()
                .map(|value| match value {
                    InstanceField::Float { value, .. } => Some(*value),
                    _ => None,
                })
                .collect::<Option<_>>()?;
            match values.as_slice() {
                [bottom, top, down, outer, inner] => Some(SpiralTreadBody {
                    bottom_feet: *bottom,
                    top_feet: *top,
                    down_angle: *down,
                    up_angles: [*outer, *inner],
                }),
                _ => None,
            }
        })
        .collect::<Option<Vec<_>>>()?;
    let [cx, cy] = SPIRAL_RUN_CENTRE_FIELDS.map(float);
    let [r0, r1] = SPIRAL_RUN_RADII_FIELDS.map(float);
    (!treads.is_empty()).then_some(SpiralRunBody {
        centre: [cx?, cy?],
        radii_feet: [r0?, r1?],
        treads,
    })
}

/// The angle past a riser line, about a spiral's centre, at which a line
/// parallel to it at `offset` feet meets the circle of `radius`.
pub(super) fn offset_angle(radius: f64, offset: f64) -> Option<f64> {
    (radius > offset && offset >= 0.0).then(|| (offset / radius).asin())
}

/// Give each spiral stair run of a type with treads and no risers, not
/// monolithic, its treads (#357), from its native records
/// ([`crate::native_stairs::native_spiral_runs`]) and its own run type, the
/// one `OST_StairsRuns` type definition its element record names (RE-28).
/// Each tread is an annular sector between the run's two radii, from the
/// riser line it starts at to the next, its top the tread's own, its type's
/// tread thickness deep. On the BIM4LCA office's run 3057703, a mirrored run
/// climbing clockwise, Revit's export runs each tread on past the next riser
/// line, under the tread above, to a line parallel to it at the type's
/// nosing length; a run with a nosing that is not measured so keeps its box.
pub(super) fn attach_spiral_run_bodies(
    rf: &mut RevitFile,
    revit_version: u32,
    products: &mut [DecodedElement],
) {
    use crate::partition_type_records as ptr;
    if !crate::native_stairs::NATIVE_SPIRAL_RUN_REVIT_VERSIONS.contains(&revit_version) {
        return;
    }
    let unbodied: Vec<usize> = products
        .iter()
        .enumerate()
        .filter(|(_, element)| {
            element.class == "StairsRun"
                && element.id.is_some()
                && !element
                    .fields
                    .iter()
                    .any(|(name, _)| name == STAIR_RUN_PROFILE_FIELD)
        })
        .map(|(index, _)| index)
        .collect();
    if unbodied.is_empty() {
        return;
    }
    let declared: BTreeSet<u32> = match crate::elem_table::parse_records(rf) {
        Ok(records) => crate::elem_table::declared_ids(&records),
        Err(_) => return,
    };
    let type_records = ptr::scan_type_records(
        rf,
        revit_version,
        crate::partition_element_records::OST_STAIRS_RUNS,
        &declared,
    )
    .unwrap_or_default();
    let run_types = ptr::type_definition_ids(&type_records);
    let mut runs: BTreeMap<u64, u64> = BTreeMap::new();
    for &index in &unbodied {
        let element = &products[index];
        let (Some(id), Some((references, _))) = (element.id, record_references(rf, element)) else {
            continue;
        };
        if let Some(type_id) = ptr::unique_type_reference(&references, &run_types) {
            runs.insert(u64::from(id), u64::from(type_id));
        }
    }
    let Ok(spirals) = crate::native_stairs::native_spiral_runs(rf, &runs) else {
        return;
    };
    let scalar = |value: f64| InstanceField::Float { value, size: 8 };
    for index in unbodied {
        let element = &mut products[index];
        let Some(run) = element.id.and_then(|id| spirals.get(&u64::from(id))) else {
            continue;
        };
        let kind = &run.run_type;
        if !(kind.treads && !kind.risers && !kind.monolithic && kind.tread_thickness_feet > 0.0) {
            continue;
        }
        // The way round the run climbs: towards smaller angles when the
        // second tread lies at smaller angles than the first.
        let clockwise = run.treads.windows(2).next().is_some_and(|pair| {
            let turn = std::f64::consts::TAU;
            let d = pair[1].angles[0] - pair[0].angles[0];
            d - turn * (d / turn).round() < 0.0
        });
        let nosing = kind.nosing_length_feet;
        if nosing > 0.0 && !(run.mirrored && clockwise) {
            continue;
        }
        let [r0, r1] = run.radii_feet;
        let (Some(inner), Some(outer)) = (offset_angle(r0, nosing), offset_angle(r1, nosing))
        else {
            continue;
        };
        let sign = if clockwise { -1.0 } else { 1.0 };
        let treads: Vec<InstanceField> = run
            .treads
            .iter()
            .map(|tread| {
                let [a0, a1] = tread.angles;
                let (down, up) = if clockwise { (a1, a0) } else { (a0, a1) };
                InstanceField::Vector(vec![
                    scalar(tread.top_feet - kind.tread_thickness_feet),
                    scalar(tread.top_feet),
                    scalar(down),
                    scalar(up + sign * outer),
                    scalar(up + sign * inner),
                ])
            })
            .collect();
        let has_type_name = element
            .fields
            .iter()
            .any(|(name, _)| name == TYPE_NAME_FIELD);
        if let (Some(name), false, Ok(type_id)) =
            (&kind.name, has_type_name, u32::try_from(run.type_id))
        {
            element.fields.push((
                TYPE_ID_FIELD.into(),
                InstanceField::ElementId {
                    tag: 0,
                    id: type_id,
                },
            ));
            element
                .fields
                .push((TYPE_NAME_FIELD.into(), InstanceField::String(name.clone())));
        }
        for (name, value) in SPIRAL_RUN_CENTRE_FIELDS.iter().zip(run.centre) {
            element.fields.push(((*name).into(), scalar(value)));
        }
        for (name, value) in SPIRAL_RUN_RADII_FIELDS.iter().zip(run.radii_feet) {
            element.fields.push(((*name).into(), scalar(value)));
        }
        element.fields.push((
            SPIRAL_RUN_TREADS_FIELD.into(),
            InstanceField::Vector(treads),
        ));
    }
}

/// A tread landing as [`LANDING_TREAD_OUTLINE_FIELD`] and its companions
/// record it (#824).
#[derive(Debug, Clone, PartialEq)]
pub struct LandingTreadBody {
    pub outline: Vec<crate::native_stairs::LandingCurve>,
    pub bottom_feet: f64,
    pub top_feet: f64,
}

/// The tread landing body recorded in `fields`, when every part of it is.
pub fn landing_tread_body_from_fields(
    fields: &[(String, InstanceField)],
) -> Option<LandingTreadBody> {
    use crate::native_stairs::LandingCurve;
    let float = |wanted: &str| {
        fields.iter().find_map(|(name, value)| match value {
            InstanceField::Float { value, .. } if name == wanted => Some(*value),
            _ => None,
        })
    };
    let curves = fields.iter().find_map(|(name, value)| match value {
        InstanceField::Vector(curves) if name == LANDING_TREAD_OUTLINE_FIELD => Some(curves),
        _ => None,
    })?;
    let outline = curves
        .iter()
        .map(|curve| {
            let InstanceField::Vector(values) = curve else {
                return None;
            };
            let values: Vec<f64> = values
                .iter()
                .map(|value| match value {
                    InstanceField::Float { value, .. } => Some(*value),
                    _ => None,
                })
                .collect::<Option<_>>()?;
            match values.as_slice() {
                [kind, ax, ay, bx, by] if *kind == 0.0 => {
                    Some(LandingCurve::Line([*ax, *ay], [*bx, *by]))
                }
                [kind, ax, ay, mx, my, bx, by] if *kind == 1.0 => {
                    Some(LandingCurve::Arc([*ax, *ay], [*mx, *my], [*bx, *by]))
                }
                _ => None,
            }
        })
        .collect::<Option<Vec<_>>>()?;
    let (bottom_feet, top_feet) = (
        float(LANDING_TREAD_BOTTOM_FIELD)?,
        float(LANDING_TREAD_TOP_FIELD)?,
    );
    (outline.len() >= 3 && top_feet > bottom_feet).then_some(LandingTreadBody {
        outline,
        bottom_feet,
        top_feet,
    })
}

/// The plan outline a landing's own lines at `top_feet` make (#824): the
/// chains of lines that meet end to end, each joined to the chain with the
/// end nearest where the outline has reached by a straight edge, and closed
/// back to the first point. On the
/// BIM4LCA office a landing's lines at its top stop where its flights meet
/// it, and Revit's outline runs straight across there: the outline is
/// Revit's within 0.001 ft on all 21 monolithic landings. `None` for fewer
/// than three lines, or an outline that crosses itself or has no area.
pub(super) fn landing_outline(
    lines: &[crate::partition_beam_axes::BoundedLine],
    top_feet: f64,
) -> Option<Vec<[f64; 2]>> {
    const TOL: f64 = LANDING_LINE_TOLERANCE_FEET;
    let gap = |a: [f64; 2], b: [f64; 2]| (a[0] - b[0]).hypot(a[1] - b[1]);
    let mut remaining: Vec<([f64; 2], [f64; 2])> = lines
        .iter()
        .filter_map(|line| {
            let (s, e) = (line.start(), line.end());
            ((s[2] - top_feet).abs() <= TOL && (e[2] - top_feet).abs() <= TOL)
                .then_some(([s[0], s[1]], [e[0], e[1]]))
        })
        .collect();
    if remaining.len() < 3 {
        return None;
    }
    // The lines that meet end to end, as chains of points.
    let mut chains: Vec<Vec<[f64; 2]>> = Vec::new();
    while !remaining.is_empty() {
        let (a, b) = remaining.remove(0);
        let mut chain = vec![a, b];
        loop {
            let tail = *chain.last()?;
            let head = chain[0];
            if let Some(i) = remaining
                .iter()
                .position(|(a, b)| gap(*a, tail) <= TOL || gap(*b, tail) <= TOL)
            {
                let (a, b) = remaining.remove(i);
                chain.push(if gap(a, tail) <= TOL { b } else { a });
            } else if let Some(i) = remaining
                .iter()
                .position(|(a, b)| gap(*a, head) <= TOL || gap(*b, head) <= TOL)
            {
                let (a, b) = remaining.remove(i);
                chain.insert(0, if gap(b, head) <= TOL { a } else { b });
            } else {
                break;
            }
        }
        chains.push(chain);
    }
    // Each chain after the first is the one with an end nearest where the
    // outline has reached, joined by a straight edge across the gap.
    let mut points = chains.remove(0);
    while !chains.is_empty() {
        let here = *points.last()?;
        let (index, reverse, _) = chains
            .iter()
            .enumerate()
            .flat_map(|(i, chain)| {
                [
                    (i, false, gap(chain[0], here)),
                    (i, true, gap(*chain.last().unwrap_or(&chain[0]), here)),
                ]
            })
            .min_by(|x, y| x.2.total_cmp(&y.2))?;
        let mut chain = chains.remove(index);
        if reverse {
            chain.reverse();
        }
        if gap(chain[0], here) <= TOL {
            chain.remove(0);
        }
        points.extend(chain);
    }
    if points.len() > 1 && gap(points[0], *points.last()?) <= TOL {
        points.pop();
    }
    let n = points.len();
    if n < 3 {
        return None;
    }
    let area: f64 = (0..n)
        .map(|i| {
            let (a, b) = (points[i], points[(i + 1) % n]);
            a[0] * b[1] - b[0] * a[1]
        })
        .sum::<f64>()
        / 2.0;
    if area.abs() <= TOL {
        return None;
    }
    let cross = |o: [f64; 2], a: [f64; 2], b: [f64; 2]| {
        (a[0] - o[0]) * (b[1] - o[1]) - (a[1] - o[1]) * (b[0] - o[0])
    };
    for i in 0..n {
        for j in (i + 1)..n {
            if j == i + 1 || (i == 0 && j == n - 1) {
                continue;
            }
            let (p, q) = (points[i], points[(i + 1) % n]);
            let (r, s) = (points[j], points[(j + 1) % n]);
            let (d1, d2) = (cross(r, s, p), cross(r, s, q));
            let (d3, d4) = (cross(p, q, r), cross(p, q, s));
            let crosses = if d1 == 0.0 && d2 == 0.0 {
                // On one line: they meet only where their extents overlap.
                (0..2).all(|k| {
                    p[k].min(q[k]).max(r[k].min(s[k])) <= p[k].max(q[k]).min(r[k].max(s[k]))
                })
            } else {
                d1 * d2 <= 0.0 && d3 * d4 <= 0.0
            };
            if crosses {
                return None;
            }
        }
    }
    Some(points)
}

/// Give each stair run the shape its treads' native records give
/// ([`STAIR_RUN_SHAPE_FIELD`], #323).
pub(super) fn attach_stair_run_shapes(
    rf: &mut RevitFile,
    revit_version: u32,
    products: &mut [DecodedElement],
) {
    if !crate::native_stairs::NATIVE_RUN_SHAPE_REVIT_VERSIONS.contains(&revit_version) {
        return;
    }
    let runs: BTreeSet<u64> = products
        .iter()
        .filter(|element| element.class == "StairsRun")
        .filter_map(|element| element.id.map(u64::from))
        .collect();
    let Ok(shapes) = crate::native_stairs::native_run_shapes(rf, &runs) else {
        return;
    };
    for element in products
        .iter_mut()
        .filter(|element| element.class == "StairsRun")
    {
        if let Some(shape) = element.id.and_then(|id| shapes.get(&u64::from(id))) {
            element.fields.push((
                STAIR_RUN_SHAPE_FIELD.into(),
                InstanceField::Integer {
                    value: shape.run_type,
                    signed: true,
                    size: 4,
                },
            ));
        }
    }
    // Each stair's type from its runs, in ElementId order, and its landings
    // (#323).
    let whole = |element: &DecodedElement| {
        element.fields.iter().find_map(|(name, value)| match value {
            InstanceField::ElementId { id, .. } if name == AGGREGATE_WHOLE_FIELD => Some(*id),
            _ => None,
        })
    };
    // Each stair's runs, by id with their shapes, and its landing count.
    type StairParts = (Vec<(u32, Option<crate::native_stairs::RunShape>)>, usize);
    let mut parts: BTreeMap<u32, StairParts> = BTreeMap::new();
    for element in products.iter() {
        let (Some(stair), Some(id)) = (whole(element), element.id) else {
            continue;
        };
        match element.class.as_str() {
            "StairsRun" => parts
                .entry(stair)
                .or_default()
                .0
                .push((id, shapes.get(&u64::from(id)).copied())),
            "StairsLanding" => parts.entry(stair).or_default().1 += 1,
            _ => {}
        }
    }
    for element in products
        .iter_mut()
        .filter(|element| element.class == "Stair")
    {
        let Some((runs, landings)) = element.id.and_then(|id| parts.get_mut(&id)) else {
            continue;
        };
        runs.sort_by_key(|(id, _)| *id);
        let Some(shapes) = runs
            .iter()
            .map(|(_, shape)| *shape)
            .collect::<Option<Vec<_>>>()
        else {
            continue;
        };
        if let Some(kind) = crate::native_stairs::stair_predefined_type(&shapes, *landings) {
            element
                .fields
                .push((STAIR_TYPE_FIELD.into(), InstanceField::String(kind.into())));
        }
    }
}

/// Give each monolithic stair landing the slab Revit draws (#824): its own
/// outline (`landing_outline`), from its top (`StairsLanding.m_baseElevation`)
/// down its type's thickness (`StairsLandingType.m_thickness`). The type is
/// the one `OST_StairsLandings` type definition its element record's
/// reference list names (RE-28), its stairs' landing type on all 22 of the
/// BIM4LCA office's. Revit's underside also follows the soffit of a flight
/// where it meets the landing, which is not drawn. A landing whose type is
/// not monolithic, whose lines do not make an outline in its record box, or
/// whose slab leaves its record box keeps its box.
pub(super) fn attach_landing_slabs(
    rf: &mut RevitFile,
    revit_version: u32,
    products: &mut [DecodedElement],
) {
    use crate::element_record_plan_profiles as erpp;
    use crate::partition_type_records as ptr;
    const BOX: f64 = SKETCH_BOX_TOLERANCE_FEET;
    if !crate::native_stairs::NATIVE_LANDING_REVIT_VERSIONS.contains(&revit_version) {
        return;
    }
    if !products
        .iter()
        .any(|element| element.class == "StairsLanding")
    {
        return;
    }
    let declared: BTreeSet<u32> = match crate::elem_table::parse_records(rf) {
        Ok(records) => crate::elem_table::declared_ids(&records),
        Err(_) => return,
    };
    let type_records = ptr::scan_type_records(
        rf,
        revit_version,
        crate::partition_element_records::OST_STAIRS_LANDINGS,
        &declared,
    )
    .unwrap_or_default();
    let landing_types = ptr::type_definition_ids(&type_records);
    let mut landings: BTreeMap<u64, u64> = BTreeMap::new();
    for element in products
        .iter()
        .filter(|element| element.class == "StairsLanding")
    {
        let (Some(id), Some((references, _))) = (element.id, record_references(rf, element)) else {
            continue;
        };
        if let Some(type_id) = ptr::unique_type_reference(&references, &landing_types) {
            landings.insert(u64::from(id), u64::from(type_id));
        }
    }
    let Ok((native, type_names)) =
        crate::native_stairs::native_landings_and_type_names(rf, &landings)
    else {
        return;
    };
    // Each landing's type and its name, as for a run (RE-28).
    for element in products
        .iter_mut()
        .filter(|element| element.class == "StairsLanding")
    {
        let Some(type_id) = element.id.and_then(|id| landings.get(&u64::from(id))) else {
            continue;
        };
        let (Some(name), Ok(type_id)) = (type_names.get(type_id), u32::try_from(*type_id)) else {
            continue;
        };
        if element
            .fields
            .iter()
            .any(|(field, _)| field == TYPE_NAME_FIELD)
        {
            continue;
        }
        element.fields.push((
            TYPE_ID_FIELD.into(),
            InstanceField::ElementId {
                tag: 0,
                id: type_id,
            },
        ));
        element
            .fields
            .push((TYPE_NAME_FIELD.into(), InstanceField::String(name.clone())));
    }
    let ids: BTreeSet<u32> = native
        .keys()
        .filter_map(|id| u32::try_from(*id).ok())
        .collect();
    let lines =
        crate::partition_stairs::scan_landing_lines(rf, revit_version, &ids).unwrap_or_default();
    let scalar = |value: f64| InstanceField::Float { value, size: 8 };
    for element in products
        .iter_mut()
        .filter(|element| element.class == "StairsLanding")
    {
        let Some(id) = element.id else {
            continue;
        };
        let (Some(landing), Some(lines), Some(bbox)) = (
            native.get(&u64::from(id)),
            lines.get(&id),
            element_record_bbox(element),
        ) else {
            continue;
        };
        if element
            .fields
            .iter()
            .any(|(name, _)| name == erpp::PLAN_PROFILE_OUTER_FIELD)
        {
            continue;
        }
        let bottom = landing.top_feet - landing.thickness_feet;
        if landing.top_feet > bbox[5] + BOX || bottom < bbox[2] - BOX {
            continue;
        }
        let Some(outline) = landing_outline(lines, landing.top_feet) else {
            continue;
        };
        if !outline.iter().all(|p| {
            p[0] >= bbox[0] - BOX
                && p[0] <= bbox[3] + BOX
                && p[1] >= bbox[1] - BOX
                && p[1] <= bbox[4] + BOX
        }) {
            continue;
        }
        let segments: Vec<[f64; 4]> = (0..outline.len())
            .map(|i| {
                let (a, b) = (outline[i], outline[(i + 1) % outline.len()]);
                [a[0], a[1], b[0], b[1]]
            })
            .collect();
        let Some(profile) = erpp::plan_profile_from_lines(&segments) else {
            continue;
        };
        element
            .fields
            .extend(profile.fields().into_iter().map(|(name, value)| {
                if name == erpp::PLAN_PROFILE_SOURCE_FIELD {
                    (name, InstanceField::String(LANDING_OUTLINE_SOURCE.into()))
                } else {
                    (name, value)
                }
            }));
        element
            .fields
            .push((LANDING_BODY_BOTTOM_FIELD.into(), scalar(bottom)));
        element
            .fields
            .push((LANDING_BODY_TOP_FIELD.into(), scalar(landing.top_feet)));
    }
    // #824: a landing that is not monolithic and takes its tread from its
    // runs is that tread, its outline arcs and all.
    let run_types = ptr::type_definition_ids(
        &ptr::scan_type_records(
            rf,
            revit_version,
            crate::partition_element_records::OST_STAIRS_RUNS,
            &declared,
        )
        .unwrap_or_default(),
    );
    let mut runs: BTreeMap<u64, u64> = BTreeMap::new();
    for element in products
        .iter()
        .filter(|element| element.class == "StairsRun")
    {
        let (Some(id), Some((references, _))) = (element.id, record_references(rf, element)) else {
            continue;
        };
        if let Some(type_id) = ptr::unique_type_reference(&references, &run_types) {
            runs.insert(u64::from(id), u64::from(type_id));
        }
    }
    let rest: BTreeMap<u64, u64> = landings
        .iter()
        .filter(|(id, _)| !native.contains_key(id))
        .map(|(id, type_id)| (*id, *type_id))
        .collect();
    let Ok(treads) = crate::native_stairs::native_tread_landings(rf, &rest, &runs) else {
        return;
    };
    for element in products
        .iter_mut()
        .filter(|element| element.class == "StairsLanding")
    {
        let (Some(landing), Some(bbox)) = (
            element.id.and_then(|id| treads.get(&u64::from(id))),
            element_record_bbox(element),
        ) else {
            continue;
        };
        let bottom = landing.top_feet - landing.thickness_feet;
        let inside = |p: [f64; 2]| {
            p[0] >= bbox[0] - BOX
                && p[0] <= bbox[3] + BOX
                && p[1] >= bbox[1] - BOX
                && p[1] <= bbox[4] + BOX
        };
        if landing.top_feet > bbox[5] + BOX
            || bottom < bbox[2] - BOX
            || !landing.outline.iter().all(|curve| match curve {
                crate::native_stairs::LandingCurve::Line(a, b) => inside(*a) && inside(*b),
                crate::native_stairs::LandingCurve::Arc(a, m, b) => {
                    inside(*a) && inside(*m) && inside(*b)
                }
            })
        {
            continue;
        }
        let float = |value: f64| InstanceField::Float { value, size: 8 };
        let curves = landing
            .outline
            .iter()
            .map(|curve| {
                let values: Vec<f64> = match curve {
                    crate::native_stairs::LandingCurve::Line(a, b) => {
                        vec![0.0, a[0], a[1], b[0], b[1]]
                    }
                    crate::native_stairs::LandingCurve::Arc(a, m, b) => {
                        vec![1.0, a[0], a[1], m[0], m[1], b[0], b[1]]
                    }
                };
                InstanceField::Vector(values.into_iter().map(float).collect())
            })
            .collect();
        element.fields.push((
            LANDING_TREAD_OUTLINE_FIELD.into(),
            InstanceField::Vector(curves),
        ));
        element
            .fields
            .push((LANDING_TREAD_BOTTOM_FIELD.into(), float(bottom)));
        element
            .fields
            .push((LANDING_TREAD_TOP_FIELD.into(), float(landing.top_feet)));
    }
}
