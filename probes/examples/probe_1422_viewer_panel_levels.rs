//! #1422: the viewer's Schedule panel, element info panel and scene tree take
//! each element's storey, and which elements they list, from the IFC writer's
//! placement decision.
//!
//! FACT under test: `scene_graph::build_schedule`, `build_scene_graph` and
//! `element_info_panel` put each building element on the storey
//! `step_writer::storey_of` gives it (the one it is contained in, that of its
//! space, its whole or its host) and list no element `step_writer::placements`
//! folds into another element's body, where they used to put it on its own
//! `storey_index` and list every building element. A drawn piece that the
//! export folds has the panel of the element that owns it.
//!
//! The probe builds the model the viewer's wasm binding builds (`rvt-ifc` with
//! no switch: `ExportOptions::default()`, which the viewer's default quality
//! mode and saved-mesh choice match), reproduces main's rules inline (every
//! building element; its own `storey_index`; the tree's bucket being that of the
//! top of its chain of hosts) and compares them with what the three functions
//! return now: the schedule's rows, the tree's nodes and a sample of panels
//! (every element whose storey moves up to a cap, every folded piece up to a
//! cap, and an even spread of the rest). Counts and IFC types only: no name, id
//! or coordinate is printed.
//!
//! Output, one line per item:
//!   elements <n> folded <n> written <n>
//!   schedule rows before <n> after <n> removed <n>
//!   schedule removed <IFC type> <n>                  the rows left out, by type
//!   schedule moved <n> none-to-storey <n> storey-to-none <n> storey-to-storey <n>
//!   schedule moved-by-type <IFC type> <n>            and by why, as moved-by-placement
//!   tree nodes before <n> after <n> removed <n> added <n>
//!   tree unassigned before <n> after <n>
//!   tree moved ...                                   as for the schedule
//!   panel checked <n> moved <n> (of <n> that move) folded <n>
//!   panel moved ...                                  as for the schedule
//!   check <name> ok | FAIL <detail>
//!
//! Usage:
//!   cargo run --profile ci -p rvt-probes --example probe_1422_viewer_panel_levels -- MODEL.rvt

use std::collections::{BTreeMap, BTreeSet};

use anyhow::{Context, Result, bail};
use rvt::RevitFile;
use rvt::ifc::entities::IfcEntity;
use rvt::ifc::scene_graph::{SceneNode, build_scene_graph, build_schedule, element_info_panel};
use rvt::ifc::step_writer::{Placement, placements, storey_of};
use rvt::ifc::{ExportOptions, IfcModel, run_export};

const PANEL_MOVED_CAP: usize = 2000;
const PANEL_FOLDED_CAP: usize = 300;
const PANEL_SPREAD: usize = 300;

type Buckets = BTreeMap<usize, Option<usize>>;

#[derive(Default)]
struct Moves {
    none_to_storey: usize,
    storey_to_none: usize,
    storey_to_storey: usize,
    by_type: BTreeMap<String, usize>,
    by_placement: BTreeMap<String, usize>,
}

impl Moves {
    fn record(&mut self, old: Option<usize>, new: Option<usize>, ifc_type: &str, why: &str) {
        if old == new {
            return;
        }
        match (old, new) {
            (None, Some(_)) => self.none_to_storey += 1,
            (Some(_), None) => self.storey_to_none += 1,
            _ => self.storey_to_storey += 1,
        }
        *self.by_type.entry(ifc_type.to_string()).or_insert(0) += 1;
        *self.by_placement.entry(why.to_string()).or_insert(0) += 1;
    }

    fn total(&self) -> usize {
        self.none_to_storey + self.storey_to_none + self.storey_to_storey
    }

    fn print(&self, label: &str) {
        println!(
            "{label} moved {} none-to-storey {} storey-to-none {} storey-to-storey {}",
            self.total(),
            self.none_to_storey,
            self.storey_to_none,
            self.storey_to_storey
        );
        print_counts(&format!("{label} moved-by-type"), &self.by_type);
        print_counts(&format!("{label} moved-by-placement"), &self.by_placement);
    }
}

fn print_counts(label: &str, counts: &BTreeMap<String, usize>) {
    let mut sorted: Vec<_> = counts.iter().collect();
    sorted.sort_by(|a, b| b.1.cmp(a.1).then_with(|| a.0.cmp(b.0)));
    for (name, count) in sorted {
        println!("{label} {name} {count}");
    }
}

fn how(placement: Option<Placement>) -> &'static str {
    match placement {
        Some(Placement::Folded) => "folded",
        Some(Placement::Opening { .. }) => "opening of its host",
        Some(Placement::InSpace(_)) => "in a space",
        Some(Placement::PartOf(_)) => "part of a whole",
        Some(Placement::OnStorey(_)) => "on a storey",
        Some(Placement::InBuilding) => "in the building",
        _ => "other",
    }
}

/// Where main's tree puts the element at `index`: the storey of the top of its
/// chain of hosts (`Some(None)` is its "Unassigned" node), or `None` where it
/// has no node, as when a host is not a building element or the chain loops.
fn old_tree_bucket(model: &IfcModel, index: usize) -> Option<Option<usize>> {
    let mut at = index;
    for _ in 0..16 {
        let IfcEntity::BuildingElement {
            storey_index,
            host_element_index,
            ..
        } = model.entities.get(at)?
        else {
            return None;
        };
        match host_element_index {
            Some(host) => at = *host,
            None => return Some(storey_index.filter(|i| *i < model.building_storeys.len())),
        }
    }
    None
}

fn collect(node: &SceneNode, bucket: Option<usize>, out: &mut Buckets, duplicates: &mut usize) {
    for child in &node.children {
        let twice = child
            .entity_index
            .is_some_and(|index| out.insert(index, bucket).is_some());
        if twice {
            *duplicates += 1;
        }
        collect(child, bucket, out, duplicates);
    }
}

fn main() -> Result<()> {
    let path = std::env::args()
        .nth(1)
        .context("usage: probe_1422_viewer_panel_levels MODEL.rvt")?;
    let mut rf = RevitFile::open(&path)?;
    let model = run_export(&mut rf, &ExportOptions::default())?.model;
    let placed = placements(&model);
    let storeys = model.building_storeys.len();
    let own = |storey_index: &Option<usize>| storey_index.filter(|i| *i < storeys);

    let mut elements = 0usize;
    let mut written: Vec<usize> = Vec::new();
    let mut folded_pieces: Vec<usize> = Vec::new();
    let mut moved: Vec<usize> = Vec::new();
    let mut removed_by_type: BTreeMap<String, usize> = BTreeMap::new();
    let mut schedule_moves = Moves::default();
    let mut old_tree: Buckets = BTreeMap::new();
    for (index, entity) in model.entities.iter().enumerate() {
        let IfcEntity::BuildingElement {
            ifc_type,
            storey_index,
            ..
        } = entity
        else {
            continue;
        };
        elements += 1;
        if let Some(bucket) = old_tree_bucket(&model, index) {
            old_tree.insert(index, bucket);
        }
        let placement = placed.get(index).copied().flatten();
        if matches!(placement, Some(Placement::Folded)) {
            folded_pieces.push(index);
            *removed_by_type.entry(ifc_type.clone()).or_insert(0) += 1;
            continue;
        }
        written.push(index);
        let old = own(storey_index);
        let new = storey_of(&placed, index);
        schedule_moves.record(old, new, ifc_type, how(placement));
        if old != new {
            moved.push(index);
        }
    }
    println!(
        "elements {elements} folded {} written {}",
        folded_pieces.len(),
        written.len()
    );

    let schedule = build_schedule(&model);
    println!(
        "schedule rows before {elements} after {} removed {}",
        schedule.rows.len(),
        folded_pieces.len()
    );
    print_counts("schedule removed", &removed_by_type);
    schedule_moves.print("schedule");

    let scene = build_scene_graph(&model);
    let mut new_tree: Buckets = BTreeMap::new();
    let mut tree_duplicates = 0usize;
    for storey in &scene.children {
        collect(
            storey,
            storey.storey_index,
            &mut new_tree,
            &mut tree_duplicates,
        );
    }
    let removed = old_tree
        .keys()
        .filter(|index| !new_tree.contains_key(*index))
        .count();
    let added = new_tree
        .keys()
        .filter(|index| !old_tree.contains_key(*index))
        .count();
    println!(
        "tree nodes before {} after {} removed {removed} added {added}",
        old_tree.len(),
        new_tree.len()
    );
    println!(
        "tree unassigned before {} after {}",
        old_tree.values().filter(|b| b.is_none()).count(),
        new_tree.values().filter(|b| b.is_none()).count()
    );
    let mut tree_moves = Moves::default();
    for (index, new) in &new_tree {
        let Some(old) = old_tree.get(index) else {
            continue;
        };
        if let Some(IfcEntity::BuildingElement { ifc_type, .. }) = model.entities.get(*index) {
            let placement = placed.get(*index).copied().flatten();
            tree_moves.record(*old, *new, ifc_type, how(placement));
        }
    }
    tree_moves.print("tree");

    let mut sample: Vec<usize> = moved.iter().copied().take(PANEL_MOVED_CAP).collect();
    sample.extend(folded_pieces.iter().copied().take(PANEL_FOLDED_CAP));
    let stride = (written.len() / PANEL_SPREAD).max(1);
    sample.extend(written.iter().copied().step_by(stride).take(PANEL_SPREAD));
    sample.sort_unstable();
    sample.dedup();
    let owner_of = |index: usize| {
        model
            .body_pieces
            .iter()
            .find(|(_, pieces)| pieces.contains(&index))
            .map_or(index, |(owner, _)| *owner)
    };
    let mut panel_moves = Moves::default();
    let mut panel_wrong = 0usize;
    let mut panel_missing = 0usize;
    let mut folded_checked = 0usize;
    let mut folded_wrong = 0usize;
    for &index in &sample {
        let Some(IfcEntity::BuildingElement {
            ifc_type,
            storey_index,
            ..
        }) = model.entities.get(index)
        else {
            continue;
        };
        let Some(panel) = element_info_panel(&model, index) else {
            panel_missing += 1;
            continue;
        };
        let is_folded = matches!(placed.get(index), Some(Some(Placement::Folded)));
        let target = if is_folded { owner_of(index) } else { index };
        let expected = storey_of(&placed, target);
        let got = panel.storey.as_ref().map(|s| s.index);
        panel_moves.record(
            own(storey_index),
            got,
            ifc_type,
            how(placed.get(index).copied().flatten()),
        );
        let consistent = panel.storey_name.as_deref()
            == got
                .and_then(|i| model.building_storeys.get(i))
                .map(|s| s.name.as_str())
            && panel.storey_elevation_feet
                == got
                    .and_then(|i| model.building_storeys.get(i))
                    .map(|s| s.elevation_feet);
        if got != expected || !consistent {
            panel_wrong += 1;
        }
        if is_folded {
            folded_checked += 1;
            let owner_name = match model.entities.get(target) {
                Some(IfcEntity::BuildingElement { name, .. }) => Some(name),
                _ => None,
            };
            if target == index || owner_name != Some(&panel.name) {
                folded_wrong += 1;
            }
        }
    }
    println!(
        "panel checked {} moved {} (of {} that move) folded {folded_checked}",
        sample.len() - panel_missing,
        panel_moves.total(),
        moved.len()
    );
    panel_moves.print("panel");

    let mut failed = false;
    let mut check = |name: &str, detail: Option<String>| match detail {
        None => println!("check {name} ok"),
        Some(detail) => {
            failed = true;
            println!("check {name} FAIL {detail}");
        }
    };
    let written_set: BTreeSet<usize> = written.iter().copied().collect();
    let row_set: BTreeSet<usize> = schedule.rows.iter().map(|r| r.entity_index).collect();
    check(
        "schedule-rows",
        (row_set != written_set || row_set.len() != schedule.rows.len()).then(|| {
            format!(
                "{} rows, {} distinct, {} written elements",
                schedule.rows.len(),
                row_set.len(),
                written_set.len()
            )
        }),
    );
    let name_of = |storey: Option<usize>| {
        storey
            .and_then(|i| model.building_storeys.get(i))
            .map(|s| s.name.as_str())
            .unwrap_or_default()
    };
    let wrong_level = schedule
        .rows
        .iter()
        .filter(|row| row.storey != name_of(storey_of(&placed, row.entity_index)))
        .count();
    check(
        "schedule-levels",
        (wrong_level > 0).then(|| format!("{wrong_level} rows off the placements' storey")),
    );
    let elevations: Vec<f64> = schedule
        .rows
        .iter()
        .map(|row| {
            storey_of(&placed, row.entity_index)
                .and_then(|i| model.building_storeys.get(i))
                .map_or(f64::INFINITY, |s| s.elevation_feet)
        })
        .collect();
    let unsorted = elevations.windows(2).filter(|w| w[0] > w[1]).count();
    check(
        "schedule-order",
        (unsorted > 0).then(|| format!("{unsorted} rows out of level-elevation order")),
    );
    let grouped: usize = schedule.groups.iter().map(|g| g.count).sum();
    let indexed: usize = schedule.groups.iter().map(|g| g.entity_indices.len()).sum();
    check(
        "schedule-groups",
        (grouped != schedule.rows.len() || indexed != schedule.rows.len()).then(|| {
            format!(
                "{grouped} counted, {indexed} indexed, {} rows",
                schedule.rows.len()
            )
        }),
    );
    let tree_set: BTreeSet<usize> = new_tree.keys().copied().collect();
    check(
        "tree-members",
        (tree_set != written_set || tree_duplicates > 0).then(|| {
            format!(
                "{} nodes, {tree_duplicates} twice, {} written elements",
                tree_set.len(),
                written_set.len()
            )
        }),
    );
    let wrong_bucket = written
        .iter()
        .filter(|index| new_tree.get(*index) != Some(&storey_of(&placed, **index)))
        .count();
    check(
        "tree-levels",
        (wrong_bucket > 0).then(|| format!("{wrong_bucket} nodes off the placements' storey")),
    );
    check(
        "panel-levels",
        (panel_wrong > 0 || panel_missing > 0).then(|| {
            format!("{panel_wrong} panels off the placements' storey, {panel_missing} missing")
        }),
    );
    check(
        "panel-folded",
        (folded_wrong > 0)
            .then(|| format!("{folded_wrong} folded pieces without their owner's panel")),
    );
    if failed {
        bail!("a panel or the tree does not follow the placements");
    }
    Ok(())
}
