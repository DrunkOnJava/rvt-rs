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
//! Measured (RE-184): `01_Walls_Phases` keeps its wall created in the last
//! of three phases, and `01_Walls_Phases_Demo` leaves out its wall demolished
//! in it. On Snowdon Towers, whose phases run Legends, Existing, New
//! Construction, the 6 slabs created and demolished in Legends are the ones
//! Revit's export leaves out. Measured (RE-326, ref `0d12ba0e`, Measure run
//! 41776212669): all 621 rooms of the 12 models with a Revit export are in the
//! last phase and in Revit's export. Autodesk's `rac_basic_sample_project` has
//! 14 rooms in the second of its three phases; they are left out by default
//! and come back with `--phase "Working Drawings"`, which writes the 14 rooms
//! a public Revit 2018 export of it holds, with no other entity changed
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
        filter.excluded = exclusions(&self.order, export, &self.phased);
        filter
    }

    /// The phase of room `id` when it is read, is one of the project's
    /// phases and is not the export phase: Revit's exporter leaves such a
    /// room out (`IsRoomInInvalidPhase`, RE-326). A room whose phase is
    /// unset, unread or unknown is written.
    pub fn room_outside_export_phase(&self, id: u64) -> Option<i64> {
        let export = self.export?;
        self.room_phases
            .get(&id)
            .copied()
            .filter(|phase| *phase != export && self.order.contains(phase))
    }

    /// Whether some room is left out for its phase.
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
    let room_phases = rooms
        .into_iter()
        .filter_map(|(id, phase)| Some((id, phase?)))
        .collect();
    Ok(PhaseFilter {
        order,
        names,
        export,
        phased: phases,
        excluded,
        room_phases,
        read_error: None,
    })
}

/// What the phase rules leave out of an export, counted for the diagnostics.
#[derive(Debug, Clone, Default)]
pub struct PhaseExcluded {
    /// Placed instances of the recovered categories that do not exist in the
    /// export phase, one per ElementId, by class name (RE-184). Rooms are
    /// counted in [`Self::rooms_by_phase`].
    pub by_class: BTreeMap<String, usize>,
    /// Placed rooms whose own phase is not the export phase, by that phase's
    /// ElementId (RE-326).
    pub rooms_by_phase: BTreeMap<i64, usize>,
}

/// Placed instances of the recovered categories
/// ([`crate::partition_element_records::RECOVERED_CATEGORIES`]) that are not
/// in the export phase, one per ElementId, keyed by class name, and the
/// rooms whose own phase is not, by that phase. The export leaves them out,
/// as Revit's does; this counts them so the diagnostics can say so. An
/// element also in a non-primary design option is counted there instead.
pub fn scan_phase_excluded_instances(
    rf: &mut RevitFile,
    revit_version: u32,
) -> crate::Result<PhaseExcluded> {
    use crate::partition_element_records::{
        RECOVERED_CATEGORIES, scan_category_records_multi, supports_revit_version,
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
    let mut excluded: BTreeMap<u32, i64> = BTreeMap::new();
    let mut rooms: BTreeMap<u32, i64> = BTreeMap::new();
    for record in records
        .iter()
        .filter(|r| r.is_exported_instance() && !options.excludes(r) && phases.excludes(r))
    {
        if record.builtin_category == OST_ROOMS {
            if let Some(phase) = phases.room_outside_export_phase(u64::from(record.element_id)) {
                rooms.insert(record.element_id, phase);
            }
        } else {
            excluded.insert(record.element_id, record.builtin_category);
        }
    }
    for category in excluded.into_values() {
        if let Some((_, class)) = RECOVERED_CATEGORIES.iter().find(|(c, _)| *c == category) {
            *counted.by_class.entry((*class).to_string()).or_insert(0) += 1;
        }
    }
    for phase in rooms.into_values() {
        *counted.rooms_by_phase.entry(phase).or_insert(0) += 1;
    }
    Ok(counted)
}
