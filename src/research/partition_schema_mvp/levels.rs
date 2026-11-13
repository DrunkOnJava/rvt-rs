//! Level parameters, base and top Level resolution, and Level elements.

use super::*;
use crate::partition_arc_walls::{self, PartitionArcWall};

/// The `Level` ElementIds the #219 / RE-27 reference-list join tests
/// against, or an empty set on a release / file with no proven Level
/// framing (the join then resolves nothing and every element keeps the
/// elevation join it already had).
pub fn level_element_ids(rf: &mut RevitFile, revit_version: u32) -> Result<BTreeSet<u32>> {
    if !crate::partition_level_records::supports_revit_version(revit_version) {
        return Ok(BTreeSet::new());
    }
    let declared: BTreeSet<u32> = match crate::elem_table::parse_records(rf) {
        Ok(records) => crate::elem_table::declared_ids(&records),
        Err(_) => return Ok(BTreeSet::new()),
    };
    crate::partition_level_records::scan_partition_level_ids(rf, revit_version, &declared)
}

/// Give `element` its level fields (RE-172, RE-174, RE-175, RE-176).
pub(super) fn attach_level_parameters(
    rf: &mut RevitFile,
    element: &mut DecodedElement,
    parameters: &BTreeMap<u32, crate::partition_level_parameters::LevelParameters>,
    names: &BTreeMap<u32, String>,
    elevations: &BTreeMap<u32, f64>,
) {
    use crate::partition_level_parameters as lp;
    // A duct's own record named its reference level (RE-371).
    if element
        .fields
        .iter()
        .any(|(name, _)| name == lp::LEVEL_PARAMETER_FIELD)
    {
        return;
    }
    let found = element
        .id
        .and_then(|id| parameters.get(&id))
        // A stair's record is read as a stair's only for a stair.
        .filter(|p| p.stair.is_none() || element.class == "Stair");
    let push = |element: &mut DecodedElement, name: &str, value: String| {
        element
            .fields
            .push((name.into(), InstanceField::String(value)));
    };
    if let Some(found) = found {
        if let Some(level) = &found.level {
            push(element, lp::LEVEL_PARAMETER_FIELD, level.clone());
        }
        if let Some(top) = &found.top {
            push(element, lp::TOP_LEVEL_PARAMETER_FIELD, top.clone());
        }
        if let Some((_, top)) = &found.stair {
            push(element, lp::TOP_LEVEL_PARAMETER_FIELD, top.clone());
        }
        return;
    }
    // The Level an element names in its record's id lists (RE-174): the
    // second list's if it names any, else the first's, if exactly one
    // recovered Level is named. A beam or brace naming more than one also
    // lists the Level(s) below its own: its Reference Level is the highest
    // (RE-175), kept apart as it is proven for those classes only.
    let (Some(stream), Some(offset)) = (
        element.fields.iter().find_map(|(n, v)| match v {
            InstanceField::String(s) if n == "m_source_stream" => Some(s.clone()),
            _ => None,
        }),
        element.fields.iter().find_map(|(n, v)| match v {
            InstanceField::Integer { value, .. } if n == "m_source_offset" => {
                usize::try_from(*value).ok()
            }
            _ => None,
        }),
    ) else {
        return;
    };
    let Ok(inflated) = rf.inflated_partition(&stream) else {
        return;
    };
    let buf = inflated.bytes();
    let at = offset + crate::partition_element_records::REFERENCE_LIST_OFFSET;
    let Some(first) = crate::partition_element_records::decode_reference_list(buf, at) else {
        return;
    };
    let second = at
        .checked_add(4 + first.len() * 8)
        .and_then(|at| crate::partition_element_records::decode_reference_list(buf, at))
        .unwrap_or_default();
    let ids: BTreeSet<u32> = names.keys().copied().collect();
    let mut named = crate::element_record_level_refs::named_levels(&second, &ids);
    if named.is_empty() {
        named = crate::element_record_level_refs::named_levels(&first, &ids);
    }
    if named.len() > 1 && element.class == "Stair" {
        // A stair whose association record gives no Base and Top Level
        // lists them, and at most the Level above its top (RE-176).
        if let Some((base, top)) = stair_levels_from_list(&named, elevations) {
            push(element, lp::LEVEL_PARAMETER_FIELD, names[&base].clone());
            push(element, lp::TOP_LEVEL_PARAMETER_FIELD, names[&top].clone());
            element
                .fields
                .push((LEVEL_FROM_LIST_FIELD.into(), InstanceField::Bool(true)));
        }
        return;
    }
    if named.len() > 1 {
        if let Some(level) = highest_level(&named, elevations) {
            push(
                element,
                REFERENCE_LEVEL_FROM_LIST_FIELD,
                names[&level].clone(),
            );
        }
        return;
    }
    if let [only] = named[..] {
        push(element, lp::LEVEL_PARAMETER_FIELD, names[&only].clone());
        element
            .fields
            .push((LEVEL_FROM_LIST_FIELD.into(), InstanceField::Bool(true)));
    }
}

/// The fields of a record's graph [`attach_levels_32`] reads, by the
/// record's class: a wall's Top Constraint (`m_upToLevelId`), a roof's Base
/// Level (`m_baseLevelId`), a stair's Base and Top Level (`m_baseLevelId` and
/// `m_topLevelId`; `m_bottomId` and `m_topId` on a sketched `Stairs`), a
/// family instance's Schedule Level (`m_scheduleOnlyLevelId`) and a beam's or
/// brace's Reference Level (`FamInstCurveDriver`'s `m_refLevelId`), and a
/// duct's or pipe's (`m_idLevelStart`). Each list holds `m_id`, which the
/// check of a record against its identity reads; no other field of the graph
/// is built (#1335), and a record of another class builds none but `m_id`.
fn level_read_fields(class: &str) -> Option<&'static [&'static str]> {
    Some(match class {
        "SWall" | "ArcWall" => &["m_id", "m_upToLevelId"],
        "Roof" => &["m_id", "m_baseLevelId"],
        "StairsElement" => &["m_id", "m_baseLevelId", "m_topLevelId"],
        "Stairs" => &["m_id", "m_bottomId", "m_topId"],
        "FamilyInstance" => &["m_id", "m_scheduleOnlyLevelId", "m_refLevelId"],
        "RbsDuctCurve" | "RbsPipeCurve" => &["m_id", "m_idLevelStart"],
        _ => &["m_id"],
    })
}

/// What one record names: its Element base's Level, and from its graph a
/// roof's or stair's base Level, a wall's or stair's top Level, a Schedule
/// Level, an MEP curve's start Level and a framing member's reference Level.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct NamedLevels {
    base: Option<u32>,
    own_base: Option<u32>,
    top: Option<u32>,
    schedule: Option<u32>,
    curve_start: Option<u32>,
    reference: Option<u32>,
}

/// Keep `found` for `id`, or nothing where two copies of the record disagree.
fn agree_levels<T: PartialEq + Copy>(map: &mut BTreeMap<u32, Option<T>>, id: u32, found: T) {
    let slot = map.entry(id).or_insert(Some(found));
    if *slot != Some(found) {
        *slot = None;
    }
}

/// Give each element on Revit 2015 to 2023 the Level parameters its own
/// native record names (RE-369), as 2024's association record gives them
/// (RE-172): `m_assocLevelId` of its Element base, which the export writes as
/// `Level`, `Base Constraint` or `Base Level` by class; a wall's
/// `m_upToLevelId` (`Top Constraint`) and a stair's top Level (`Top Level`);
/// where the base names none, a roof's or stair's base Level (`Base Level`),
/// a family instance's `m_scheduleOnlyLevelId` (`Level`), a duct's or pipe's
/// `m_idLevelStart` and a beam's or brace's `FamInstCurveDriver.m_refLevelId`
/// (`Reference Level`). Each is written only where it is one of `levels`, the
/// Levels [`crate::partition_level_records`] recovers, and every copy of the
/// record names the same one. The graphs are read only for walls and for
/// elements whose base names no Level, projected to the fields
/// `level_read_fields` lists.
pub(super) fn attach_levels_32(
    rf: &mut RevitFile,
    revit_version: u32,
    levels: &[crate::partition_level_records::PartitionLevel],
    groups: [&mut Vec<DecodedElement>; 6],
) {
    use crate::native_document::{self, Options};
    use crate::native_json::identifier_i64;
    use crate::partition_level_parameters as lp;
    if !LEVEL_32BIT_REVIT_VERSIONS.contains(&revit_version) {
        return;
    }
    let mut groups = groups;
    let ids = |pick: &dyn Fn(&DecodedElement) -> bool, groups: &[&mut Vec<DecodedElement>; 6]| {
        groups
            .iter()
            .flat_map(|group| group.iter())
            .filter(|element| pick(element))
            .filter_map(|element| element.id.map(u64::from))
            .collect::<BTreeSet<u64>>()
    };
    let selected = ids(&|_| true, &groups);
    if selected.is_empty() {
        return;
    }
    let names: BTreeMap<u32, &str> = levels
        .iter()
        .map(|level| (level.element_id, level.name.as_str()))
        .collect();
    if names.is_empty() {
        return;
    }
    let level = |value: i64| {
        u32::try_from(value)
            .ok()
            .filter(|id| names.contains_key(id))
    };
    // Every element's Element base, its graph left undecoded: the id the base
    // names, as stored, so that one naming a Level that was not recovered is
    // told from one naming none.
    let mut bases: BTreeMap<u32, Option<i64>> = BTreeMap::new();
    let base_options = Options {
        selected_ids: selected,
        graph_classes: Some(BTreeSet::new()),
        releases: native_document::NATIVE_RELEASES,
        ..Options::default()
    };
    if native_document::extract_graphs(rf, &base_options, |record| {
        if let (Some(base), Ok(id)) = (&record.base, u32::try_from(record.identity.element_id)) {
            agree_levels(&mut bases, id, base.associated_level_id);
        }
        Ok(())
    })
    .is_err()
    {
        return;
    }
    // The graphs of walls and of the elements whose base names no Level.
    let graphed = ids(
        &|element| {
            element.class == "Wall"
                || element.id.is_some_and(|id| match bases.get(&id) {
                    Some(Some(raw)) => *raw <= 0,
                    // Copies that disagree, or a base that was not read, may name a
                    // Level: no other field stands in for it.
                    Some(None) | None => false,
                })
        },
        &groups,
    );
    let mut graphs: BTreeMap<u32, Option<NamedLevels>> = BTreeMap::new();
    if !graphed.is_empty() {
        let graph_options = Options {
            selected_ids: graphed,
            graph_fields: Some(level_read_fields),
            releases: native_document::NATIVE_RELEASES,
            ..Options::default()
        };
        let read = native_document::extract_graphs(rf, &graph_options, |record| {
            let (Some(graph), Ok(id)) = (
                record.graph.as_ref(),
                u32::try_from(record.identity.element_id),
            ) else {
                return Ok(());
            };
            let Some(root) = graph.objects.first() else {
                return Ok(());
            };
            let field = |object: &crate::native_parameters::GraphObject, name: &str| {
                object
                    .fields
                    .get(name)
                    .and_then(identifier_i64)
                    .and_then(level)
            };
            let mut drivers = graph
                .objects
                .iter()
                .filter(|object| object.class_name == "FamInstCurveDriver");
            let reference = match (drivers.next(), drivers.next()) {
                (Some(driver), None) => field(driver, "m_refLevelId"),
                _ => None,
            };
            agree_levels(
                &mut graphs,
                id,
                NamedLevels {
                    base: None,
                    own_base: field(root, "m_baseLevelId").or_else(|| field(root, "m_bottomId")),
                    top: field(root, "m_upToLevelId")
                        .or_else(|| field(root, "m_topLevelId"))
                        .or_else(|| field(root, "m_topId")),
                    schedule: field(root, "m_scheduleOnlyLevelId"),
                    curve_start: field(root, "m_idLevelStart"),
                    reference,
                },
            );
            Ok(())
        });
        if read.is_err() {
            graphs.clear();
        }
    }
    for element in groups.iter_mut().flat_map(|group| group.iter_mut()) {
        let Some(id) = element.id else {
            continue;
        };
        let raw = bases.get(&id).copied().flatten();
        let named = NamedLevels {
            base: raw.and_then(level),
            ..graphs.get(&id).copied().flatten().unwrap_or_default()
        };
        // A base naming a Level that was not recovered says the element has
        // one, so no other field stands in for it.
        let base_unrecovered = raw.is_some_and(|raw| raw > 0 && level(raw).is_none());
        // A beam's or brace's Reference Level is its own field: its Schedule
        // Level is another parameter, and never stands in for it.
        let schedule = if element.class == "StructuralFraming" {
            None
        } else {
            named.schedule
        };
        let mut push = |field: &str, level: u32| {
            if let Some(name) = names.get(&level) {
                // A duct's own record named its reference level (RE-371): the
                // same name is not pushed a second time.
                let held = element.fields.iter().any(|(key, value)| {
                    key == field && matches!(value, InstanceField::String(held) if held == *name)
                });
                if !held {
                    element
                        .fields
                        .push((field.into(), InstanceField::String((*name).into())));
                }
            }
        };
        if !base_unrecovered {
            if let Some(level) = named
                .base
                .or(named.own_base)
                .or(schedule)
                .or(named.curve_start)
            {
                push(lp::LEVEL_PARAMETER_FIELD, level);
            } else if let Some(level) = named.reference {
                push(REFERENCE_LEVEL_FIELD, level);
            }
        }
        if let Some(level) = named.top {
            push(lp::TOP_LEVEL_PARAMETER_FIELD, level);
        }
    }
}

/// A stair's Base and Top Level from the Levels its record lists: the
/// lowest and the next above it (RE-176). `None` unless every named Level
/// has an elevation and the lowest three are at distinct elevations.
pub(super) fn stair_levels_from_list(
    named: &[u32],
    elevations: &BTreeMap<u32, f64>,
) -> Option<(u32, u32)> {
    let mut by_height: Vec<(f64, u32)> = named
        .iter()
        .map(|id| Some((*elevations.get(id)?, *id)))
        .collect::<Option<_>>()?;
    by_height.sort_by(|a, b| a.0.total_cmp(&b.0));
    let above = |low: f64, high: f64| high - low > 1e-6;
    match by_height[..] {
        [(b, base), (t, top)] if above(b, t) => Some((base, top)),
        [(b, base), (t, top), (n, _), ..] if above(b, t) && above(t, n) => Some((base, top)),
        _ => None,
    }
}

/// The Level of `named` with the highest elevation; `None` unless every
/// named Level has an elevation and the highest is unique (RE-175).
pub(super) fn highest_level(named: &[u32], elevations: &BTreeMap<u32, f64>) -> Option<u32> {
    let mut by_height: Vec<(f64, u32)> = named
        .iter()
        .map(|id| Some((*elevations.get(id)?, *id)))
        .collect::<Option<_>>()?;
    by_height.sort_by(|a, b| b.0.total_cmp(&a.0));
    match by_height[..] {
        [(top, id), (next, _), ..] if top - next > 1e-6 => Some(id),
        _ => None,
    }
}

/// RE-59: give each record-backed element whose record names two Levels
/// its base constraint ([`crate::element_record_level_refs::base_constraint_level`])
/// as its host Level, from the Levels' own elevations (RE-51). Nothing
/// changes where the Levels' elevations are not recovered.
///
/// A beam or brace takes the highest Level its record names instead, whatever
/// their number: its Reference Level (RE-175, [`highest_level`]), which is the
/// storey Revit's export puts it in. On the two BIM4LCA timber office files the
/// 47 beams that name two Levels are in the highest's storey and in the base
/// constraint's in none (RE-454).
pub(super) fn resolve_base_constraint_levels(
    elevations: &BTreeMap<u32, f64>,
    elements: &mut [&mut Vec<DecodedElement>],
) {
    use crate::element_record_level_refs as refs;
    if elevations.is_empty() {
        return;
    }
    for element in elements.iter_mut().flat_map(|list| list.iter_mut()) {
        let mut named = Vec::new();
        let mut base = None;
        for (name, value) in &element.fields {
            match (name.as_str(), value) {
                (refs::CONSTRAINT_LEVELS_FIELD, InstanceField::Vector(ids)) => {
                    named = ids
                        .iter()
                        .filter_map(|id| match id {
                            InstanceField::ElementId { id, .. } => Some(*id),
                            _ => None,
                        })
                        .collect();
                }
                ("m_locationZ", InstanceField::Float { value, .. }) => base = Some(*value),
                _ => {}
            }
        }
        let (level, source) = if element.class == "StructuralFraming" {
            (
                highest_level(&named, elevations),
                refs::FRAMING_REFERENCE_LEVEL_SOURCE,
            )
        } else {
            (
                base.and_then(|z| refs::base_constraint_level(&named, elevations, z)),
                refs::BASE_CONSTRAINT_SOURCE,
            )
        };
        let Some(level) = level else {
            continue;
        };
        bind_record_level(element, level, source);
    }
}

/// Bind each Revit 2023 structural framing element to the one Level at its
/// record box's top, in place of the Level its record names (RE-119). A
/// beam sits under its reference Level: on a 2023 project Revit's export
/// puts four beams spanning 3.2 to 4.0 m on the 4.0 m Level, and four
/// foundation beams spanning -0.8 to 0.0 m on the 0.0 m Level, where their
/// records name the 0.0 m Level and none. Nothing changes where no Level, or
/// more than one, is at the top.
pub(super) fn resolve_framing_top_levels(
    elevations: &BTreeMap<u32, f64>,
    elements: &mut [&mut Vec<DecodedElement>],
) {
    use crate::element_record_level_refs as refs;
    const TOLERANCE_FEET: f64 = 1e-3;
    for element in elements.iter_mut().flat_map(|list| list.iter_mut()) {
        if element.class != "StructuralFraming" {
            continue;
        }
        let float = |wanted: &str| {
            element.fields.iter().find_map(|(name, value)| match value {
                InstanceField::Float { value, .. } if name == wanted => Some(*value),
                _ => None,
            })
        };
        let (Some(base), Some(height)) = (float("m_locationZ"), float("m_bboxHeight")) else {
            continue;
        };
        let top = base + height;
        let mut at_top = elevations
            .iter()
            .filter(|(_, elevation)| (**elevation - top).abs() <= TOLERANCE_FEET)
            .map(|(id, _)| *id);
        let (Some(level), None) = (at_top.next(), at_top.next()) else {
            continue;
        };
        element.fields.retain(|(name, _)| {
            name != refs::LEVEL_REFERENCE_FIELD && name != refs::LEVEL_BIND_SOURCE_FIELD
        });
        bind_record_level(element, level, FRAMING_TOP_LEVEL_SOURCE);
    }
}

pub(super) fn bind_record_level(element: &mut DecodedElement, level: u32, source: &str) {
    use crate::element_record_level_refs as refs;
    for (name, value) in element.fields.iter_mut() {
        if name == "m_level_bound" {
            *value = InstanceField::Bool(true);
        }
    }
    element.fields.push((
        refs::LEVEL_REFERENCE_FIELD.into(),
        InstanceField::ElementId { tag: 0, id: level },
    ));
    element.fields.push((
        refs::LEVEL_BIND_SOURCE_FIELD.into(),
        InstanceField::String(source.into()),
    ));
}

pub(super) fn element_ids_field(element: &DecodedElement, field: &str) -> Vec<u32> {
    element
        .fields
        .iter()
        .find_map(|(name, value)| match value {
            InstanceField::Vector(ids) if name == field => Some(
                ids.iter()
                    .filter_map(|id| match id {
                        InstanceField::ElementId { id, .. } => Some(*id),
                        _ => None,
                    })
                    .collect(),
            ),
            _ => None,
        })
        .unwrap_or_default()
}

pub(super) fn record_level(element: &DecodedElement) -> Option<u32> {
    element.fields.iter().find_map(|(name, value)| match value {
        InstanceField::ElementId { id, .. }
            if name == crate::element_record_level_refs::LEVEL_REFERENCE_FIELD =>
        {
            Some(*id)
        }
        _ => None,
    })
}

/// RE-60: a Level for record-backed elements whose record names none.
///
/// - A railing takes the Level of the one stair or ramp its record names,
///   where that has one.
/// - A plumbing fixture takes the Level the objects its record names carry
///   ([`crate::partition_level_records::scan_level_objects`]), where they
///   carry one (RE-68 §6). On Snowdon Towers that is Revit's storey for all
///   4 such fixtures, where their base elevation gives another.
/// - Any other element takes that Level only where it is also the highest
///   Level at or below the element's base
///   ([`crate::element_record_level_refs::level_at_or_below`]). For light
///   fixtures, slab edges and wall sweeps the objects alone name a Level
///   other than Revit's on 73 of the 214 that name one.
pub(super) fn resolve_hosted_levels(
    rf: &mut RevitFile,
    elevations: &BTreeMap<u32, f64>,
    elements: &mut [&mut Vec<DecodedElement>],
) {
    use crate::element_record_level_refs as refs;
    if elevations.is_empty() {
        return;
    }
    let pending = elements.iter().flat_map(|list| list.iter()).any(|element| {
        record_level(element).is_none()
            && !element_ids_field(element, refs::REFERENCES_FIELD).is_empty()
    });
    if !pending {
        return;
    }
    let hosts: BTreeMap<u32, Option<u32>> = elements
        .iter()
        .flat_map(|list| list.iter())
        .filter(|element| matches!(element.class.as_str(), "Stair" | "Ramp"))
        .filter_map(|element| Some((element.id?, record_level(element))))
        .collect();
    let declared = match crate::elem_table::parse_records(rf) {
        Ok(records) => crate::elem_table::declared_ids(&records),
        Err(_) => return,
    };
    let level_ids: BTreeSet<u32> = elevations.keys().copied().collect();
    let level_objects =
        crate::partition_level_records::scan_level_objects(rf, &declared, &level_ids);
    for element in elements.iter_mut().flat_map(|list| list.iter_mut()) {
        if record_level(element).is_some() {
            continue;
        }
        let references = element_ids_field(element, refs::REFERENCES_FIELD);
        if references.is_empty() {
            continue;
        }
        if element.class == "Railing" {
            let named: Vec<&Option<u32>> =
                references.iter().filter_map(|id| hosts.get(id)).collect();
            if let [Some(level)] = named.as_slice() {
                bind_record_level(element, *level, refs::HOST_SOURCE);
            }
            continue;
        }
        let carried: BTreeSet<u32> = references
            .iter()
            .filter_map(|id| level_objects.get(id).copied())
            .collect();
        let base = element.fields.iter().find_map(|(name, value)| match value {
            InstanceField::Float { value, .. } if name == "m_locationZ" => Some(*value),
            _ => None,
        });
        let (Some(&level), 1) = (carried.first(), carried.len()) else {
            continue;
        };
        if element.class == "PlumbingFixture" {
            bind_record_level(element, level, refs::PLUMBING_LEVEL_OBJECT_SOURCE);
            continue;
        }
        let Some(base) = base else {
            continue;
        };
        if refs::level_at_or_below(elevations, base) == Some(level) {
            bind_record_level(element, level, refs::LEVEL_OBJECT_SOURCE);
        }
    }
}

/// A record-backed element's base elevation, feet.
pub(super) fn record_base_feet(element: &DecodedElement) -> Option<f64> {
    element.fields.iter().find_map(|(name, value)| match value {
        InstanceField::Float { value, .. } if name == "m_locationZ" => Some(*value),
        _ => None,
    })
}

/// RE-68: a stair or ramp that names no Level, or several (a multistory
/// stair names three), takes the one Level its base sits exactly at, so
/// its railings can follow it (RE-60). On Snowdon Towers these are the
/// three multistory stairs, whose base is L3's elevation; Revit writes
/// each one on L3 and again on L4.
pub(super) fn resolve_base_at_level(
    elevations: &BTreeMap<u32, f64>,
    elements: &mut [&mut Vec<DecodedElement>],
) {
    use crate::element_record_level_refs as refs;
    for element in elements.iter_mut().flat_map(|list| list.iter_mut()) {
        if record_level(element).is_some() || !matches!(element.class.as_str(), "Stair" | "Ramp") {
            continue;
        }
        let Some(base) = record_base_feet(element) else {
            continue;
        };
        let mut at = elevations
            .iter()
            .filter(|(_, at)| (**at - base).abs() <= 1e-3);
        if let (Some((&level, _)), None) = (at.next(), at.next()) {
            bind_record_level(element, level, refs::BASE_AT_LEVEL_SOURCE);
        }
    }
}

/// RE-68: a Level for record-backed elements still without one after RE-59
/// and RE-60.
///
/// - An element takes the Level of the one element of its own class its
///   record names, where that has one: the parts of a nested light-fixture
///   family follow the fixture that holds them.
/// - Otherwise an element of [`ELEVATION_PLACED_CLASSES`] takes the Level
///   its base elevation gives, within a fail-closed band
///   ([`crate::element_record_level_refs::elevation_band_level`]).
///
/// Measured on Snowdon Towers against every copy Revit writes of each
/// element (a multistory stair and its railings appear once per storey).
pub(super) fn resolve_remaining_levels(
    elevations: &BTreeMap<u32, f64>,
    elements: &mut [&mut Vec<DecodedElement>],
) {
    use crate::element_record_level_refs as refs;
    if elevations.is_empty() {
        return;
    }
    let leveled: BTreeMap<u32, (String, u32)> = elements
        .iter()
        .flat_map(|list| list.iter())
        .filter_map(|element| Some((element.id?, (element.class.clone(), record_level(element)?))))
        .collect();
    for element in elements.iter_mut().flat_map(|list| list.iter_mut()) {
        if record_level(element).is_some() {
            continue;
        }
        let references = element_ids_field(element, refs::REFERENCES_FIELD);
        let same_class: BTreeSet<u32> = references
            .iter()
            .filter_map(|id| leveled.get(id))
            .filter(|(class, _)| *class == element.class)
            .map(|(_, level)| *level)
            .collect();
        let hosts = references
            .iter()
            .filter(|id| {
                leveled
                    .get(id)
                    .is_some_and(|(class, _)| *class == element.class)
            })
            .count();
        if let (1, Some(&level)) = (hosts, same_class.first()) {
            bind_record_level(element, level, refs::SAME_CLASS_HOST_SOURCE);
            continue;
        }
        if !ELEVATION_PLACED_CLASSES.contains(&element.class.as_str()) {
            continue;
        }
        if let Some(level) =
            record_base_feet(element).and_then(|base| refs::elevation_band_level(elevations, base))
        {
            bind_record_level(element, level, refs::BASE_ELEVATION_SOURCE);
        }
    }
}

pub(super) fn levels_from_storeys_and_names(
    walls: &[PartitionArcWall],
    level_names: &[String],
) -> Vec<DecodedElement> {
    let recovery = partition_arc_walls::recover_storeys_from_arc_walls(walls, level_names);
    // A name that no Level's elevation confirms is not a storey: the names
    // are candidates from partition strings, and on a file whose Levels are
    // not read they are often not the model's (a Revit 2026 house gave
    // `Ground floor`, `Level 1` and `Roof`, none of its four Levels, and a
    // family with no Levels gave `Level 1` and `Roof`), each written at
    // elevation 0 (RE-124). Such a file has no storeys (fail closed).
    if recovery.storeys.is_empty() {
        return Vec::new();
    }
    recovery
        .storeys
        .into_iter()
        .enumerate()
        .map(|(i, s)| level_decoded(&s.name, Some(s.elevation_feet), i))
        .collect()
}

pub(super) fn level_decoded(name: &str, elevation: Option<f64>, index: usize) -> DecodedElement {
    let mut fields = vec![
        ("m_name".into(), InstanceField::String(name.to_string())),
        ("m_isBuildingStory".into(), InstanceField::Bool(true)),
        (
            "m_source".into(),
            InstanceField::String("partition_schema_mvp".into()),
        ),
    ];
    if let Some(elev) = elevation {
        fields.push((
            "m_elevation".into(),
            InstanceField::Float {
                value: elev,
                size: 8,
            },
        ));
    }
    let confidence = if elevation.is_some() { 0.75 } else { 0.55 };
    DecodedElement {
        id: None,
        class: "Level".into(),
        fields,
        byte_range: index..index,
        provenance: ElementProvenance::partition(
            "partition",
            index,
            "partition_schema_mvp",
            "partition_schema_mvp::level",
            confidence,
            if elevation.is_none() {
                Some("elevation_unknown")
            } else {
                None
            },
        ),
    }
}
