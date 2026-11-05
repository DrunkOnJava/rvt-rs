//! The elements and rooms Revit's export leaves out by phase (#328, RE-184,
//! RE-326).
//!
//! A project's phases are listed in order by its `AllProjectPhases` record
//! (`m_phaseIds`); a family document's own `AllProjectPhases` (`m_famId` set)
//! is not the project's. Every element body carries `m_createdPhaseId` and
//! `m_demolishedPhaseId`. Revit's IFC export shows one phase, the last unless
//! the export setup names another, and an element is exported when it exists
//! there: created in it or before it, and not demolished in it or before it.
//!
//! A room is left out by a different rule. Revit's exporter applies that
//! status rule to non-spatial elements only (revit-ifc,
//! `ElementFilteringUtil.GetExportFilter`), and skips a room by
//! `ElementFilteringUtil.IsRoomInInvalidPhase`, as `Exporter.ExportSpatialElements`
//! calls it: a room whose own `ROOM_PHASE` parameter is set and is not the
//! export phase. An area or an MEP space gets no phase check at all. A room's
//! phase is the `m_phaseId` field of its `RoomElem` record, not its
//! `m_createdPhaseId`, which is unset on every room (RE-326):
//! [`PhaseFilter::room_phases`]. A room whose phase is unset, is not one of
//! the project's phases, was not read, or whose copies disagree is written.
//!
//! The export phase is [`PhaseFilter::export_phase`]: the last phase, or the
//! one `RevitFile::set_export_phase` chose (`rvt-ifc --phase`), where
//! revit-ifc takes it from the export setup's active phase or the
//! `VIEW_PHASE` of the view the export is limited to
//! (`ExportOptionsCache.Create`). Both rules read it from there.
//!
//! Rooms alone have a fallback (Griffin, 2026-11-05, #1359): where no phase
//! is named and the last phase holds no room, the rooms are judged against
//! the latest phase that holds any ([`PhaseFilter::room_fallback`]), since
//! rooms all in one phase cannot overlap and the only Revit export of such a
//! project (Autodesk's `rac_basic_sample_project`) holds them. Only placed
//! rooms count, both for "the last phase holds a room" and for "the latest
//! phase that holds any": the records the export writes rooms from (placed
//! instances of the rooms category, not in a non-primary design option), which
//! `scan_phase_excluded_instances` counts the left-out rooms from by the same
//! test. An Area and an unplaced record (RE-326 §2) hold no room, so an
//! unplaced record in the last phase does not stop the fallback and one in a
//! later phase than the placed rooms does not move it. A placed room whose
//! phase is unset, unread or not one of the project's is written as before and
//! does not count either. Where a release's records cannot be scanned for
//! placement, or the scan fails, there is no fallback and the export is what
//! it was. Naming a phase, the last included, is exact. Every other element
//! follows the export phase as before.
//!
//! Measured (RE-184): `01_Walls_Phases` keeps its wall created in the last
//! of three phases, and `01_Walls_Phases_Demo` leaves out its wall demolished
//! in it. On Snowdon Towers, whose phases run Legends, Existing, New
//! Construction, the 6 slabs created and demolished in Legends are the ones
//! Revit's export leaves out. Measured (RE-326, ref `0d12ba0e`, Measure run
//! 41776212669): all 621 rooms of the 12 models with a Revit export are in the
//! last phase and in Revit's export. Autodesk's `rac_basic_sample_project` has
//! 14 rooms in the second of its three phases, and its last holds none: the
//! rooms fall back to Working Drawings by default (#1359), are left out when
//! the last phase is named, and `--phase "Working Drawings"` writes the 14
//! rooms a public Revit 2018 export of it holds, with no other entity changed
//! (Measure runs 41784415396 and 41784419457, ref `4532731d`).
// Tracker: B46.

use crate::native_json::{agree, identifier_i64};
use crate::partition_element_records::{OST_ROOMS, PartitionElementRecord};
use crate::{RevitFile, native_document};
use std::collections::{BTreeMap, BTreeSet};

/// A project's phases, the export phase, and what does not exist in it.
#[derive(Debug, Clone, Default)]
pub struct PhaseFilter {
    /// The project's phase ids, in order.
    pub order: Vec<i64>,
    /// The phases' names by ElementId (`ProjectPhase.m_name`).
    pub names: BTreeMap<i64, String>,
    /// The phase the export shows: the last of [`Self::order`] until
    /// `RevitFile::set_export_phase` chooses another.
    pub(crate) export: Option<i64>,
    /// Whether the export phase was named (`RevitFile::set_export_phase`)
    /// rather than defaulted to the last: a named phase is exact, and the
    /// room fallback does not apply to it.
    pub(crate) export_named: bool,
    /// Every phased element's ElementId with its created and demolished
    /// phase, which [`Self::excluded`] is computed from for any export phase.
    pub(crate) phased: Vec<(u64, i64, i64)>,
    /// Phased elements that do not exist in the export phase, by ElementId,
    /// with the reason. [`Self::excludes`] applies it to no room.
    pub excluded: BTreeMap<u64, PhaseExclusion>,
    /// The phase each `RoomElem` record names in its own `m_phaseId`, by
    /// ElementId: Revit's `ROOM_PHASE` (RE-326). A record whose phase is
    /// unset (-1), whose graph was not read, or whose copies disagree is not
    /// here. An Area has no such field.
    pub room_phases: BTreeMap<u64, i64>,
    /// The phase whose rooms the default export writes in place of the last
    /// phase's, which holds none: the latest phase of [`Self::order`] that
    /// holds a placed room (`room_fallback_phase`). `None` where the last
    /// phase holds a placed room, no phase does, the release's records cannot
    /// be scanned for placement, or no record names an earlier phase. It
    /// applies only while the export phase is the default
    /// ([`Self::room_fallback_in_effect`]).
    pub room_fallback: Option<i64>,
    /// The placed rooms in [`Self::room_fallback`]'s phase, each counted once:
    /// the rooms the fallback brings in. Zero when there is no fallback.
    pub room_fallback_rooms: usize,
    /// Why the phases could not be read, where a read of a release the
    /// native record path reads failed. The filter is empty then, so the
    /// export writes every element, and the export diagnostics say so.
    pub read_error: Option<String>,
}

/// Why an element does not exist in the export phase.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PhaseExclusion {
    /// Created in a phase after the export phase.
    Future,
    /// Demolished in the export phase or an earlier one.
    Demolished,
}

impl PhaseFilter {
    /// The phase the export shows: the last of [`Self::order`], or the one
    /// chosen with `RevitFile::set_export_phase`.
    pub fn export_phase(&self) -> Option<i64> {
        self.export
    }

    /// A phase's name, or `ElementId <n>` where the file names none.
    pub fn phase_label(&self, id: i64) -> String {
        self.names
            .get(&id)
            .cloned()
            .unwrap_or_else(|| format!("ElementId {id}"))
    }

    /// The project's phases in order, named, for a message.
    pub fn phases_listing(&self) -> String {
        self.order
            .iter()
            .map(|id| match self.names.get(id) {
                Some(name) => format!("\"{name}\" (ElementId {id})"),
                None => format!("ElementId {id}"),
            })
            .collect::<Vec<_>>()
            .join(", ")
    }

    /// The phase `request` names: a phase's name exactly, or else its
    /// ElementId. Only a phase of [`Self::order`] is found.
    pub fn find_phase(&self, request: &str) -> Option<i64> {
        self.order
            .iter()
            .copied()
            .find(|id| self.names.get(id).is_some_and(|name| name == request))
            .or_else(|| {
                request
                    .parse::<i64>()
                    .ok()
                    .filter(|id| self.order.contains(id))
            })
    }

    /// This filter for another export phase of [`Self::order`], its
    /// exclusions recomputed from the phased elements.
    pub fn with_export_phase(&self, export: i64) -> PhaseFilter {
        let mut filter = self.clone();
        filter.export = Some(export);
        filter.export_named = true;
        filter.excluded = exclusions(&self.order, export, &self.phased);
        filter
    }

    /// This filter at the default export phase, the project's last, with the
    /// room fallback ([`Self::room_fallback`]): as it is before any phase is
    /// named, whatever phase was named since.
    pub fn with_default_export_phase(&self) -> PhaseFilter {
        let mut filter = self.clone();
        let export = self.order.last().copied();
        filter.export = export;
        filter.export_named = false;
        filter.excluded = export.map_or_else(BTreeMap::new, |last| {
            exclusions(&self.order, last, &self.phased)
        });
        filter
    }

    /// Whether `other` exports anything other than this filter does: another
    /// export phase, or rooms judged against another. The memoised production
    /// walk holds what the phase leaves out, so it is dropped when this is
    /// true of the filter that replaces it.
    pub fn differs_in_effect_from(&self, other: &PhaseFilter) -> bool {
        self.export != other.export || self.room_export_phase() != other.room_export_phase()
    }

    /// The room fallback phase while it applies: the export phase is the
    /// default, not one named. A named phase, the last included, is exact.
    pub fn room_fallback_in_effect(&self) -> Option<i64> {
        if self.export_named {
            None
        } else {
            self.room_fallback
        }
    }

    /// The phase rooms are judged against: the fallback phase while it
    /// applies, else the export phase.
    pub fn room_export_phase(&self) -> Option<i64> {
        self.room_fallback_in_effect().or(self.export)
    }

    /// The phase of room `id` when it is read, is one of the project's
    /// phases and is not the phase rooms are judged against
    /// ([`Self::room_export_phase`]): Revit's exporter leaves such a room out
    /// (`IsRoomInInvalidPhase`, RE-326). A room whose phase is unset, unread
    /// or unknown is written.
    pub fn room_outside_export_phase(&self, id: u64) -> Option<i64> {
        let export = self.room_export_phase()?;
        self.room_phases
            .get(&id)
            .copied()
            .filter(|phase| *phase != export && self.order.contains(phase))
    }

    /// Whether some room is left out for its phase, against the phase rooms
    /// are judged against ([`Self::room_export_phase`]).
    pub fn leaves_out_rooms(&self) -> bool {
        self.room_phases
            .keys()
            .any(|id| self.room_outside_export_phase(*id).is_some())
    }

    /// Whether `record` is left out of Revit's export for its phase. A room
    /// is by its own phase ([`Self::room_outside_export_phase`]); any other
    /// element when it does not exist in the export phase: created after
    /// it, or demolished in it or before it (RE-184).
    pub fn excludes(&self, record: &PartitionElementRecord) -> bool {
        let id = u64::from(record.element_id);
        if record.builtin_category == OST_ROOMS {
            self.room_outside_export_phase(id).is_some()
        } else {
            self.excluded.contains_key(&id)
        }
    }
}

/// The elements of `phased` (ElementId, created phase, demolished phase) that
/// do not exist in the phase `export` of `order`.
fn exclusions(
    order: &[i64],
    export: i64,
    phased: &[(u64, i64, i64)],
) -> BTreeMap<u64, PhaseExclusion> {
    let Some(export) = order.iter().position(|p| *p == export) else {
        return BTreeMap::new();
    };
    phased
        .iter()
        .filter_map(|(id, created, demolished)| {
            Some((*id, exclusion(order, export, *created, *demolished)?))
        })
        .collect()
}

/// Whether `record` is a placed instance the export can write, not in a
/// non-primary design option: the one test the room fallback and the counts of
/// what the phase rule leaves out share.
fn is_placed(
    record: &PartitionElementRecord,
    options: &crate::partition_design_options::DesignOptions,
) -> bool {
    record.is_exported_instance() && !options.excludes(record)
}

/// The ElementIds of the placed rooms of the file, by the scan of its release
/// (Revit 2016 to 2023 as `scan_phase_excluded_instances` scans them, and the
/// releases whose element records are supported). `None` where the release's
/// records cannot be scanned for placement or the scan fails: the room
/// fallback then does not apply, and the export is what it was.
fn placed_room_ids(rf: &mut RevitFile, revit_version: u32) -> Option<BTreeSet<u32>> {
    use crate::partition_element_records::{scan_category_records_multi, supports_revit_version};
    let filtered_32bit =
        crate::partition_element_records_2023::applies_export_filters(revit_version);
    if !filtered_32bit && !supports_revit_version(revit_version) {
        return None;
    }
    let options = rf.memoised_design_options();
    let records = if filtered_32bit {
        crate::partition_element_records_2023::scan_records(rf, revit_version)
    } else {
        let table = crate::elem_table::parse_records(rf).ok()?;
        let declared: BTreeSet<u32> = crate::elem_table::declared_ids(&table);
        scan_category_records_multi(rf, revit_version, &[OST_ROOMS], &declared).ok()?
    };
    Some(
        records
            .iter()
            .filter(|r| r.builtin_category == OST_ROOMS && is_placed(r, &options))
            .map(|r| r.element_id)
            .collect(),
    )
}

/// The room fallback of a project (#1359) and the placed rooms it brings in.
/// `room_phases` holds the phase of every `RoomElem` record, placed or not, so
/// a project whose records name no phase before the last has no fallback and
/// is not scanned. Otherwise the placed rooms are found
/// ([`placed_room_ids`]) and [`room_fallback_phase`] decides from their phases.
fn placed_room_fallback(
    rf: &mut RevitFile,
    order: &[i64],
    room_phases: &BTreeMap<u64, i64>,
) -> (Option<i64>, usize) {
    let Some((_, earlier)) = order.split_last() else {
        return (None, 0);
    };
    if !room_phases.values().any(|held| earlier.contains(held)) {
        return (None, 0);
    }
    let Some(placed) = rf
        .layout_release()
        .ok()
        .and_then(|release| placed_room_ids(rf, release))
    else {
        return (None, 0);
    };
    let placed_phases: BTreeMap<u64, i64> = room_phases
        .iter()
        .filter(|(id, _)| u32::try_from(**id).is_ok_and(|id| placed.contains(&id)))
        .map(|(id, phase)| (*id, *phase))
        .collect();
    let fallback = room_fallback_phase(order, &placed_phases);
    let rooms = fallback.map_or(0, |phase| {
        placed_phases
            .values()
            .filter(|held| **held == phase)
            .count()
    });
    (fallback, rooms)
}

/// The phase whose rooms the default export writes when no phase is named and
/// the last phase of `order` holds no room (#1359): the latest phase of `order`
/// that holds one, by the phases `room_phases` reads, which the caller gives
/// for the placed rooms only. `None` where the last phase holds a room (nothing
/// changes) or no phase of `order` holds any. A room whose phase is unset,
/// unread or not in `order` is not in `room_phases` and does not count.
fn room_fallback_phase(order: &[i64], room_phases: &BTreeMap<u64, i64>) -> Option<i64> {
    let last = *order.last()?;
    let holds = |phase: i64| room_phases.values().any(|held| *held == phase);
    if holds(last) {
        return None;
    }
    order.iter().rev().copied().find(|phase| holds(*phase))
}

/// An element's status in the phase at `export` of `order`, from its created
/// and demolished phase ids (-1: none). An element whose phases are not in
/// `order` is kept.
pub fn exclusion(
    order: &[i64],
    export: usize,
    created: i64,
    demolished: i64,
) -> Option<PhaseExclusion> {
    let index = |id: i64| order.iter().position(|p| *p == id);
    let created = index(created)?;
    if created > export {
        return Some(PhaseExclusion::Future);
    }
    let demolished = if demolished == -1 {
        None
    } else {
        Some(index(demolished)?)
    };
    demolished
        .is_some_and(|d| d <= export)
        .then_some(PhaseExclusion::Demolished)
}

/// The fields of a `RoomElem` record that [`compute_phase_filter`] takes: the
/// phases it was created and demolished in, its own phase, and `m_id`, which
/// the record's identity is checked against. The rest of a room's graph is
/// read and checked but not built (#1335). The two records of the project's
/// phases are read in full.
fn room_phase_fields(class: &str) -> Option<&'static [&'static str]> {
    match class {
        "RoomElem" => Some(&[
            "m_id",
            "m_createdPhaseId",
            "m_demolishedPhaseId",
            "m_phaseId",
        ]),
        _ => None,
    }
}

/// The project's phase order and names, every element not in its last phase
/// and every room's own phase, in one pass over the native records (graphs
/// only for `AllProjectPhases`, `ProjectPhase` and `RoomElem`). Empty where
/// the native record path does not read the release (Revit 2015 to 2027 are
/// read) or the file lists no phases; an error where it reads the release
/// and the read fails. The export phase is the last.
pub fn compute_phase_filter(rf: &mut RevitFile) -> anyhow::Result<PhaseFilter> {
    if !rf
        .layout_release()
        .is_ok_and(|release| native_document::NATIVE_RELEASES.contains(&release))
    {
        return Ok(PhaseFilter::default());
    }
    let options = native_document::Options {
        graph_classes: Some(BTreeSet::from([
            "AllProjectPhases".to_string(),
            "ProjectPhase".to_string(),
            "RoomElem".to_string(),
        ])),
        releases: native_document::NATIVE_RELEASES,
        graph_fields: Some(room_phase_fields),
        ..native_document::Options::default()
    };
    let mut order: Option<Vec<i64>> = None;
    let mut names: BTreeMap<i64, String> = BTreeMap::new();
    let mut phases: Vec<(u64, i64, i64)> = Vec::new();
    let mut rooms: BTreeMap<u64, Option<i64>> = BTreeMap::new();
    native_document::extract_graphs(rf, &options, |record| {
        if let Some(base) = &record.base {
            if base.created_phase_id != -1 {
                phases.push((
                    record.identity.element_id,
                    base.created_phase_id,
                    base.demolished_phase_id,
                ));
            }
            return Ok(());
        }
        let Some(root) = record.graph.as_ref().and_then(|g| g.objects.first()) else {
            return Ok(());
        };
        if root.class_name == "ProjectPhase" {
            // An embedded family's own phases are not the project's: skip a
            // record that names a family, as the phase order does.
            if root
                .fields
                .get("m_famId")
                .and_then(identifier_i64)
                .is_some_and(|family| family != -1)
            {
                return Ok(());
            }
            if let (Ok(id), Some(name)) = (
                i64::try_from(record.identity.element_id),
                root.fields
                    .get("m_name")
                    .and_then(serde_json::Value::as_str),
            ) {
                names.insert(id, name.to_string());
            }
            return Ok(());
        }
        if root.class_name == "RoomElem" {
            let id = record.identity.element_id;
            let phase = |field: &str| root.fields.get(field).and_then(identifier_i64);
            if let Some(created) = phase("m_createdPhaseId").filter(|p| *p != -1) {
                phases.push((id, created, phase("m_demolishedPhaseId").unwrap_or(-1)));
            }
            agree(&mut rooms, id, phase("m_phaseId").filter(|p| *p != -1));
            return Ok(());
        }
        let fields = serde_json::to_value(&root.fields)?;
        if fields.get("m_famId").and_then(identifier_i64) != Some(-1) {
            return Ok(());
        }
        let ids: Vec<i64> = fields["m_phaseIds"]
            .as_array()
            .map(|a| a.iter().filter_map(identifier_i64).collect())
            .unwrap_or_default();
        anyhow::ensure!(
            order.is_none() || order.as_ref() == Some(&ids),
            "two project phase lists disagree"
        );
        order = Some(ids);
        Ok(())
    })?;
    let order = order.unwrap_or_default();
    let export = order.last().copied();
    let excluded = export.map_or_else(BTreeMap::new, |export| exclusions(&order, export, &phases));
    let room_phases: BTreeMap<u64, i64> = rooms
        .into_iter()
        .filter_map(|(id, phase)| Some((id, phase?)))
        .collect();
    let (room_fallback, room_fallback_rooms) = placed_room_fallback(rf, &order, &room_phases);
    Ok(PhaseFilter {
        order,
        names,
        export,
        export_named: false,
        phased: phases,
        excluded,
        room_phases,
        room_fallback,
        room_fallback_rooms,
        read_error: None,
    })
}

/// What the phase rules leave out of an export, for the diagnostics.
#[derive(Debug, Clone, Default)]
pub struct PhaseExcluded {
    /// Placed instances of the recovered categories that do not exist in the
    /// export phase, one per ElementId, in ElementId order (RE-184), each
    /// with its class and the frame it was read from. Rooms are in
    /// [`Self::rooms`].
    pub elements: Vec<crate::partition_element_records::LeftOutElement>,
    /// Placed rooms whose own phase is not the export phase, in ElementId
    /// order, each with that phase's ElementId (RE-326).
    pub rooms: Vec<(i64, crate::partition_element_records::LeftOutElement)>,
}

/// Placed instances of the recovered categories
/// ([`crate::partition_element_records::RECOVERED_CATEGORIES`]) that are not
/// in the export phase, one per ElementId, and the rooms whose own phase is
/// not, each with the frame it was read from. The export leaves them out, as
/// Revit's does; the diagnostics count them so they can say so. An element
/// also in a non-primary design option is counted there instead.
pub fn scan_phase_excluded_instances(
    rf: &mut RevitFile,
    revit_version: u32,
) -> crate::Result<PhaseExcluded> {
    use crate::partition_element_records::{
        LeftOutElement, RECOVERED_CATEGORIES, recovered_class, scan_category_records_multi,
        supports_revit_version,
    };
    let mut counted = PhaseExcluded::default();
    let filtered_32bit =
        crate::partition_element_records_2023::applies_export_filters(revit_version);
    if !filtered_32bit && !supports_revit_version(revit_version) {
        return Ok(counted);
    }
    let phases = rf.memoised_phase_filter();
    if phases.excluded.is_empty() && !phases.leaves_out_rooms() {
        return Ok(counted);
    }
    let options = rf.memoised_design_options();
    let records = if filtered_32bit {
        crate::partition_element_records_2023::scan_records(rf, revit_version)
    } else {
        let declared: BTreeSet<u32> = match crate::elem_table::parse_records(rf) {
            Ok(records) => crate::elem_table::declared_ids(&records),
            Err(_) => return Ok(counted),
        };
        let categories: Vec<i64> = RECOVERED_CATEGORIES.iter().map(|(c, _)| *c).collect();
        scan_category_records_multi(rf, revit_version, &categories, &declared)?
    };
    let mut excluded: BTreeMap<u32, &PartitionElementRecord> = BTreeMap::new();
    let mut rooms: BTreeMap<u32, (i64, &PartitionElementRecord)> = BTreeMap::new();
    for record in records
        .iter()
        .filter(|r| is_placed(r, &options) && phases.excludes(r))
    {
        if record.builtin_category == OST_ROOMS {
            if let Some(phase) = phases.room_outside_export_phase(u64::from(record.element_id)) {
                rooms.insert(record.element_id, (phase, record));
            }
        } else {
            excluded.insert(record.element_id, record);
        }
    }
    for record in excluded.into_values() {
        if let Some(class) = recovered_class(record.builtin_category) {
            counted
                .elements
                .push(LeftOutElement::from_record(record, class));
        }
    }
    for (phase, record) in rooms.into_values() {
        counted
            .rooms
            .push((phase, LeftOutElement::from_record(record, "Room")));
    }
    Ok(counted)
}

#[cfg(test)]
mod tests {
    use super::{PhaseExclusion, PhaseFilter, exclusions, room_fallback_phase};
    use std::collections::BTreeMap;

    // Existing, Working Drawings, Learning Content: the order of Autodesk's
    // `rac_basic_sample_project`.
    const ORDER: [i64; 3] = [12589, 86961, 961604];

    fn rooms(phases: &[i64]) -> BTreeMap<u64, i64> {
        phases
            .iter()
            .enumerate()
            .map(|(index, phase)| (index as u64 + 1, *phase))
            .collect()
    }

    // Rooms 1 and 2 in Working Drawings and room 3 in Existing, one element
    // (500) created in the last phase, and the fallback the caller gives.
    fn project(fallback: Option<i64>) -> PhaseFilter {
        let phased = vec![(500, 961604, -1)];
        PhaseFilter {
            order: ORDER.to_vec(),
            export: Some(961604),
            excluded: exclusions(&ORDER, 961604, &phased),
            phased,
            room_phases: rooms(&[86961, 86961, 12589]),
            room_fallback: fallback,
            room_fallback_rooms: if fallback.is_some() { 2 } else { 0 },
            ..PhaseFilter::default()
        }
    }

    #[test]
    fn a_project_with_no_phases_has_no_fallback() {
        assert_eq!(room_fallback_phase(&[], &rooms(&[86961])), None);
    }

    #[test]
    fn a_project_with_no_room_has_no_fallback() {
        assert_eq!(room_fallback_phase(&ORDER, &BTreeMap::new()), None);
    }

    #[test]
    fn a_room_in_the_last_phase_keeps_the_rule_as_it_is() {
        assert_eq!(room_fallback_phase(&ORDER, &rooms(&[86961, 961604])), None);
        assert_eq!(room_fallback_phase(&ORDER, &rooms(&[961604])), None);
    }

    #[test]
    fn a_room_whose_phase_is_not_the_projects_does_not_hold_the_last_phase() {
        assert_eq!(room_fallback_phase(&ORDER, &rooms(&[777, 888])), None);
        assert_eq!(
            room_fallback_phase(&ORDER, &rooms(&[777, 86961])),
            Some(86961)
        );
    }

    #[test]
    fn rooms_all_in_an_earlier_phase_fall_back_to_it() {
        assert_eq!(
            room_fallback_phase(&ORDER, &rooms(&[86961; 14])),
            Some(86961)
        );
    }

    #[test]
    fn rooms_in_two_earlier_phases_fall_back_to_the_latest() {
        assert_eq!(
            room_fallback_phase(&ORDER, &rooms(&[12589, 86961, 12589])),
            Some(86961)
        );
        assert_eq!(
            room_fallback_phase(&ORDER, &rooms(&[86961, 12589])),
            Some(86961)
        );
    }

    // The latest phase is the last in the project's order, not the greatest
    // ElementId: here Working Drawings (86961) is listed first.
    #[test]
    fn the_latest_phase_is_by_the_projects_order_not_by_the_greatest_id() {
        let order = [86961, 12589, 961604];
        assert_eq!(
            room_fallback_phase(&order, &rooms(&[86961, 12589])),
            Some(12589)
        );
        assert_eq!(
            room_fallback_phase(&order, &rooms(&[12589, 86961])),
            Some(12589)
        );
        assert_eq!(room_fallback_phase(&order, &rooms(&[86961])), Some(86961));
    }

    #[test]
    fn a_project_with_one_phase_has_no_other_to_fall_back_to() {
        assert_eq!(room_fallback_phase(&[12589], &rooms(&[777])), None);
        assert_eq!(room_fallback_phase(&[12589], &rooms(&[12589])), None);
    }

    #[test]
    fn the_default_judges_rooms_against_the_fallback_phase() {
        let filter = project(Some(86961));
        assert_eq!(filter.room_fallback_in_effect(), Some(86961));
        assert_eq!(filter.room_export_phase(), Some(86961));
        assert_eq!(filter.room_outside_export_phase(1), None);
        assert_eq!(filter.room_outside_export_phase(3), Some(12589));
        // Every other element is still judged against the last phase.
        assert_eq!(filter.export_phase(), Some(961604));
        assert!(filter.excluded.is_empty());
    }

    #[test]
    fn a_project_without_a_fallback_judges_rooms_against_the_last_phase() {
        let filter = project(None);
        assert_eq!(filter.room_export_phase(), Some(961604));
        assert_eq!(filter.room_outside_export_phase(1), Some(86961));
        assert_eq!(filter.room_outside_export_phase(3), Some(12589));
    }

    #[test]
    fn naming_the_last_phase_is_exact() {
        let named = project(Some(86961)).with_export_phase(961604);
        assert!(named.export_named);
        assert_eq!(named.room_fallback_in_effect(), None);
        assert_eq!(named.room_export_phase(), Some(961604));
        assert_eq!(named.room_outside_export_phase(1), Some(86961));
    }

    #[test]
    fn going_back_to_the_default_restores_the_fallback_and_the_elements() {
        let named = project(Some(86961)).with_export_phase(86961);
        assert_eq!(named.excluded.get(&500), Some(&PhaseExclusion::Future));
        let default = named.with_default_export_phase();
        assert!(!default.export_named);
        assert_eq!(default.export_phase(), Some(961604));
        assert_eq!(default.room_fallback_in_effect(), Some(86961));
        assert!(default.excluded.is_empty());
    }

    #[test]
    fn a_filter_with_no_phases_has_no_default_phase() {
        let default = PhaseFilter::default().with_default_export_phase();
        assert_eq!(default.export_phase(), None);
        assert_eq!(default.room_export_phase(), None);
    }

    // The walk is dropped exactly when the replacing filter exports otherwise.
    #[test]
    fn the_walk_is_dropped_when_the_rooms_are_judged_against_another_phase() {
        let default = project(Some(86961));
        // The last phase named after a default that fell back: same export
        // phase, other rooms.
        assert!(default.differs_in_effect_from(&default.with_export_phase(961604)));
        // And back again.
        let named_last = default.with_export_phase(961604);
        assert!(named_last.differs_in_effect_from(&default.with_default_export_phase()));
        // Another phase named.
        assert!(default.differs_in_effect_from(&default.with_export_phase(12589)));
    }

    #[test]
    fn the_walk_is_kept_when_nothing_exported_changes() {
        let no_fallback = project(None);
        assert!(!no_fallback.differs_in_effect_from(&no_fallback.with_export_phase(961604)));
        let named = project(Some(86961)).with_export_phase(86961);
        assert!(!named.differs_in_effect_from(&named.with_export_phase(86961)));
        let default = project(Some(86961));
        assert!(!default.differs_in_effect_from(&default.with_default_export_phase()));
    }
}
