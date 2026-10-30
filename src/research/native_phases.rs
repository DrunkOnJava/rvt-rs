//! The elements Revit's export leaves out by phase (#328, RE-184).
//!
//! A project's phases are listed in order by its `AllProjectPhases` record
//! (`m_phaseIds`); a family document's own `AllProjectPhases` (`m_famId` set)
//! is not the project's. Every element body carries `m_createdPhaseId` and
//! `m_demolishedPhaseId`. With no phase chosen, Revit's IFC export
//! shows the last phase, and an element is exported when it exists there:
//! created in it or before it, and not demolished in it or before it.
//!
//! Rooms are not checked. Revit's exporter applies that status rule to
//! non-spatial elements only (revit-ifc, `ElementFilteringUtil.GetExportFilter`).
//! It skips a room by a different rule, `ElementFilteringUtil.IsRoomInInvalidPhase`
//! as `Exporter.ExportSpatialElements` calls it: a room whose own `ROOM_PHASE`
//! parameter is set and is not the export phase. An area or an MEP space gets
//! no phase check at all. No model here measures whether a room's
//! `ROOM_PHASE` is its `m_createdPhaseId`, so [`PhaseFilter::excludes`] never
//! excludes a room, and rvt-rs may write a room that Revit's export leaves out
//! for its phase.
//!
//! Measured (RE-184): `01_Walls_Phases` keeps its wall created in the last
//! of three phases, and `01_Walls_Phases_Demo` leaves out its wall demolished
//! in it. On Snowdon Towers, whose phases run Legends, Existing, New
//! Construction, the 6 slabs created and demolished in Legends are the ones
//! Revit's export leaves out.
// Tracker: B46.

use crate::native_json::element_id_i64;
use crate::partition_element_records::{OST_ROOMS, PartitionElementRecord};
use crate::{RevitFile, native_document};
use std::collections::{BTreeMap, BTreeSet};

/// A project's phases and the elements that do not exist in the last one.
#[derive(Debug, Clone, Default)]
pub struct PhaseFilter {
    /// The project's phase ids, in order.
    pub order: Vec<i64>,
    /// Phased elements that do not exist in the last phase, by ElementId,
    /// with the reason. It holds rooms too; [`Self::excludes`] never excludes
    /// one.
    pub excluded: BTreeMap<u64, PhaseExclusion>,
    /// Why the phases could not be read, where a read of a release the
    /// native record path reads failed. The filter is empty then, so the
    /// export writes every element, and the export diagnostics say so.
    pub read_error: Option<String>,
}

/// Why an element does not exist in the export phase.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PhaseExclusion {
    /// Created in a phase after the export phase. Unreachable while the
    /// export phase is the last, kept so the rule reads whole.
    Future,
    /// Demolished in the export phase or an earlier one.
    Demolished,
}

impl PhaseFilter {
    /// The phase the export shows: the last of [`Self::order`].
    pub fn export_phase(&self) -> Option<i64> {
        self.order.last().copied()
    }

    /// Whether `record` does not exist in the export phase, which Revit's
    /// export leaves out. A room is never excluded: Revit's exporter checks
    /// a room's own phase parameter instead of its status, and that
    /// parameter is not measured to be the room's created phase.
    pub fn excludes(&self, record: &PartitionElementRecord) -> bool {
        record.builtin_category != OST_ROOMS
            && self.excluded.contains_key(&u64::from(record.element_id))
    }
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

/// The project's phase order and every element not in its last phase, in
/// one pass over the native records (graphs only for `AllProjectPhases`).
/// Empty where the native record path does not read the release (Revit 2023
/// to 2027 are read) or the file lists no phases; an error where it reads the
/// release and the read fails.
pub fn compute_phase_filter(rf: &mut RevitFile) -> anyhow::Result<PhaseFilter> {
    if !matches!(rf.layout_release(), Ok(2023..=2027)) {
        return Ok(PhaseFilter::default());
    }
    let options = native_document::Options {
        graph_classes: Some(BTreeSet::from(["AllProjectPhases".to_string()])),
        ..native_document::Options::default()
    };
    let mut order: Option<Vec<i64>> = None;
    let mut phases: Vec<(u64, i64, i64)> = Vec::new();
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
        let fields = serde_json::to_value(&root.fields)?;
        if fields.get("m_famId").and_then(element_id_i64) != Some(-1) {
            return Ok(());
        }
        let ids: Vec<i64> = fields["m_phaseIds"]
            .as_array()
            .map(|a| a.iter().filter_map(element_id_i64).collect())
            .unwrap_or_default();
        anyhow::ensure!(
            order.is_none() || order.as_ref() == Some(&ids),
            "two project phase lists disagree"
        );
        order = Some(ids);
        Ok(())
    })?;
    let order = order.unwrap_or_default();
    let mut excluded = BTreeMap::new();
    if let Some(export) = order.len().checked_sub(1) {
        for (id, created, demolished) in phases {
            if let Some(reason) = exclusion(&order, export, created, demolished) {
                excluded.insert(id, reason);
            }
        }
    }
    Ok(PhaseFilter {
        order,
        excluded,
        read_error: None,
    })
}

/// Placed instances of the recovered categories
/// ([`crate::partition_element_records::RECOVERED_CATEGORIES`]) that are not
/// in the export phase, one per ElementId, keyed by class name. The export
/// leaves them out, as Revit's does; this counts them so the diagnostics can
/// say so. An element also in a non-primary design option is counted there
/// instead. Rooms are never counted ([`PhaseFilter::excludes`]).
pub fn scan_phase_excluded_instances(
    rf: &mut RevitFile,
    revit_version: u32,
) -> crate::Result<BTreeMap<String, usize>> {
    use crate::partition_element_records::{
        RECOVERED_CATEGORIES, scan_category_records_multi, supports_revit_version,
    };
    let mut by_class = BTreeMap::new();
    if !supports_revit_version(revit_version) {
        return Ok(by_class);
    }
    let phases = rf.memoised_phase_filter();
    if phases.excluded.is_empty() {
        return Ok(by_class);
    }
    let declared: BTreeSet<u32> = match crate::elem_table::parse_records(rf) {
        Ok(records) => crate::elem_table::declared_ids(&records),
        Err(_) => return Ok(by_class),
    };
    let options = rf.memoised_design_options();
    let categories: Vec<i64> = RECOVERED_CATEGORIES.iter().map(|(c, _)| *c).collect();
    let records = scan_category_records_multi(rf, revit_version, &categories, &declared)?;
    let mut excluded: BTreeMap<u32, i64> = BTreeMap::new();
    for record in records
        .iter()
        .filter(|r| r.is_exported_instance() && !options.excludes(r) && phases.excludes(r))
    {
        excluded.insert(record.element_id, record.builtin_category);
    }
    for category in excluded.into_values() {
        if let Some((_, class)) = RECOVERED_CATEGORIES.iter().find(|(c, _)| *c == category) {
            *by_class.entry((*class).to_string()).or_insert(0) += 1;
        }
    }
    Ok(by_class)
}
