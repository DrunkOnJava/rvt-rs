//! #1865: which curtain walls do the curtain parts that lose their wall on one
//! save name, and which of them was the part's wall on the save before?
//!
//! FACT under test: a curtain mullion or panel takes a curtain wall as its
//! aggregate whole only through the one curtain grid its record names or, with
//! no grid, the one curtain wall its reference list names, or, naming none, the
//! one whose record box contains its own (RE-46, RE-62, RE-72). A part whose
//! list names two curtain walls has no whole and stands alone. On Autodesk's
//! `rac_advanced` saved by Revit 2027, 80 curtain parts (79 `L Mullion 1` corner
//! mullions and one curtain door) are parts of a curtain wall in the 2025 save
//! and of none in the 2027 save, and so lose the `IsExternal` they take from
//! their wall (#1865). RE-179 reads the cause as the lists naming both walls
//! of the corner on 2027 and one on 2025. This prints the lists.
//!
//! The probe:
//! - with one file, counts its curtain mullions and panels by how many curtain
//!   walls their lists name, and how many of each are parts of a wall: counts
//!   only, so it can be run on a held-out model;
//! - with two (Measure's `probe_pair_release`: a sample, then the same project
//!   saved by another release), prints that count for both and, for each part
//!   that is a part of a curtain wall in the first and of none in the second,
//!   the curtain walls each save's list names and which of them held the part
//!   in the first save.
//!
//! Output (the pair's lines name ElementIds; the samples are Autodesk's and
//! redistributable, the held-out models are run with one file only):
//!   curtain_parts side=<left|right> class=<mullion|panel> parts=<n> named0=<n> named1=<n> named2=<n>
//!     named3plus=<n> aggregated_named0=<n> aggregated_named1=<n> aggregated_named2=<n> aggregated_named3plus=<n>
//!   corner_part tag=<ElementId> type=<IFC entity> name=<name> left_wall=<ElementId>
//!     left_named=<ElementIds> right_named=<ElementId>:<in|out>,...
//!     (`in`: the wall's record box contains the part's within 0.05 ft, as RE-62)
//!   corner_parts_lost n=<n> right_named0=<n> right_named1=<n> right_named2=<n> right_named3plus=<n>
//!     host_in_right_named=<n> host_first=<n> host_last=<n> box_picks_host=<n> box_picks_other=<n>
//!     box_ties=<n> box_none=<n>
//!
//! Usage:
//!   cargo run --profile ci -p rvt-probes --example probe_1865_part_walls -- A.rvt [B.rvt]

use std::collections::{BTreeMap, BTreeSet, HashMap};

use rvt::RevitFile;
use rvt::ifc::entities::IfcEntity;
use rvt::ifc::{ExportOptions, run_export};
use rvt::research::partition_element_records as per;

/// How far past a curtain wall's record box a part's box may reach and still
/// lie inside it, feet (RE-62).
const TOLERANCE_FEET: f64 = 0.05;

/// One save: its curtain walls, which curtain wall each aggregated part is a
/// part of, what each element is, and the element records of the categories a
/// curtain part or wall belongs to.
struct Side {
    walls: BTreeSet<u32>,
    whole_of: HashMap<u32, u32>,
    names: HashMap<u32, (String, String)>,
    records: BTreeMap<u32, Vec<per::PartitionElementRecord>>,
}

fn load(path: &str) -> anyhow::Result<Side> {
    let mut rf = RevitFile::open_with_limits(path, rvt::cli::open_limits(None, None, false))?;
    let model = run_export(&mut rf, &ExportOptions::default())?.model;
    // Each entity: its IFC entity, its Tag as an ElementId and its name.
    let info: Vec<Option<(String, u32, String)>> = model
        .entities
        .iter()
        .map(|entity| match entity {
            IfcEntity::BuildingElement {
                ifc_type,
                name,
                type_guid,
                ..
            } => type_guid
                .as_deref()
                .and_then(|tag| tag.parse::<u32>().ok())
                .map(|tag| (ifc_type.clone(), tag, name.clone())),
            _ => None,
        })
        .collect();
    let mut walls = BTreeSet::new();
    let mut names = HashMap::new();
    for (ifc_type, tag, name) in info.iter().flatten() {
        names.insert(*tag, (ifc_type.clone(), name.clone()));
        if ifc_type == "IFCCURTAINWALL" {
            walls.insert(*tag);
        }
    }
    let mut whole_of = HashMap::new();
    for entity in &model.entities {
        let IfcEntity::Aggregate { whole, parts } = entity else {
            continue;
        };
        let Some(Some((ifc_type, wall, _))) = info.get(*whole) else {
            continue;
        };
        if ifc_type != "IFCCURTAINWALL" {
            continue;
        }
        for part in parts {
            if let Some(Some((_, tag, _))) = info.get(*part) {
                whole_of.entry(*tag).or_insert(*wall);
            }
        }
    }
    let version = rf.basic_file_info()?.version;
    let declared = rvt::elem_table::declared_ids(&rvt::elem_table::parse_records(&mut rf)?);
    let categories = [
        per::OST_WALLS,
        per::OST_CURTAIN_WALL_MULLIONS,
        per::OST_CURTAIN_WALL_PANELS,
        per::OST_DOORS,
        per::OST_WINDOWS,
    ];
    let mut records: BTreeMap<u32, Vec<per::PartitionElementRecord>> = BTreeMap::new();
    for record in per::scan_category_records_multi(&mut rf, version, &categories, &declared)? {
        records.entry(record.element_id).or_default().push(record);
    }
    Ok(Side {
        walls,
        whole_of,
        names,
        records,
    })
}

/// The distinct curtain walls `record`'s reference list names, in list order.
fn named_walls(record: &per::PartitionElementRecord, walls: &BTreeSet<u32>) -> Vec<u32> {
    let mut out = Vec::new();
    for slot in &record.references {
        if let Ok(id) = u32::try_from(*slot) {
            if id != record.element_id && walls.contains(&id) && !out.contains(&id) {
                out.push(id);
            }
        }
    }
    out
}

/// `inner` lies inside `outer` within [`TOLERANCE_FEET`], both `[min x, min y,
/// min z, max x, max y, max z]`.
fn contains(outer: &[f64; 6], inner: &[f64; 6]) -> bool {
    (0..3).all(|axis| {
        inner[axis] >= outer[axis] - TOLERANCE_FEET
            && inner[axis + 3] <= outer[axis + 3] + TOLERANCE_FEET
    })
}

/// The first element record `side` holds for `tag`.
fn first_record<'a>(side: &'a Side, tag: &u32) -> Option<&'a per::PartitionElementRecord> {
    side.records.get(tag).and_then(|records| records.first())
}

fn list(ids: &[u32]) -> String {
    if ids.is_empty() {
        "-".to_string()
    } else {
        ids.iter().map(u32::to_string).collect::<Vec<_>>().join(",")
    }
}

/// Count `side`'s curtain mullions and panels by how many curtain walls their
/// lists name, and how many of each are a part of a curtain wall.
fn census(label: &str, side: &Side) {
    for (class, category) in [
        ("mullion", per::OST_CURTAIN_WALL_MULLIONS),
        ("panel", per::OST_CURTAIN_WALL_PANELS),
    ] {
        let mut parts = 0usize;
        let mut named = [0usize; 4];
        let mut aggregated = [0usize; 4];
        for (id, records) in &side.records {
            let Some(record) = records.iter().find(|record| {
                record.builtin_category == category && record.is_exported_instance()
            }) else {
                continue;
            };
            parts += 1;
            let at = named_walls(record, &side.walls).len().min(3);
            named[at] += 1;
            aggregated[at] += usize::from(side.whole_of.contains_key(id));
        }
        println!(
            "curtain_parts side={label} class={class} parts={parts} named0={} named1={} named2={} named3plus={} aggregated_named0={} aggregated_named1={} aggregated_named2={} aggregated_named3plus={}",
            named[0],
            named[1],
            named[2],
            named[3],
            aggregated[0],
            aggregated[1],
            aggregated[2],
            aggregated[3]
        );
    }
}

/// Print, for each part that is a part of a curtain wall in `left` and of none
/// in `right`, the walls each list names, then how often the first save's wall
/// is among the second's, first or last of them, or the one whose box holds
/// the part.
fn lost(left: &Side, right: &Side) {
    let parts: BTreeSet<u32> = left
        .whole_of
        .keys()
        .filter(|id| !right.whole_of.contains_key(*id))
        .copied()
        .collect();
    let (mut by_named, mut host_in, mut first, mut last) = ([0usize; 4], 0usize, 0usize, 0usize);
    let (mut picks_host, mut picks_other, mut ties, mut none) = (0usize, 0usize, 0usize, 0usize);
    for tag in &parts {
        let host = left.whole_of[tag];
        let (ifc_type, name) = right
            .names
            .get(tag)
            .or_else(|| left.names.get(tag))
            .map_or(("?", "?"), |(ifc_type, name)| {
                (ifc_type.as_str(), name.as_str())
            });
        let left_named = first_record(left, tag)
            .map(|record| named_walls(record, &left.walls))
            .unwrap_or_default();
        let (right_named, in_box) = match first_record(right, tag) {
            Some(record) => {
                let named = named_walls(record, &right.walls);
                let inside: Vec<bool> = named
                    .iter()
                    .map(|wall| {
                        right
                            .records
                            .get(wall)
                            .and_then(|records| {
                                records
                                    .iter()
                                    .find(|r| r.builtin_category == per::OST_WALLS)
                            })
                            .is_some_and(|wall| contains(&wall.bbox_feet, &record.bbox_feet))
                    })
                    .collect();
                (named, inside)
            }
            None => (Vec::new(), Vec::new()),
        };
        println!(
            "corner_part tag={tag} type={ifc_type} name={name} left_wall={host} left_named={} right_named={}",
            list(&left_named),
            if right_named.is_empty() {
                "-".to_string()
            } else {
                right_named
                    .iter()
                    .zip(&in_box)
                    .map(|(wall, inside)| format!("{wall}:{}", if *inside { "in" } else { "out" }))
                    .collect::<Vec<_>>()
                    .join(",")
            }
        );
        by_named[right_named.len().min(3)] += 1;
        if right_named.contains(&host) {
            host_in += 1;
        }
        if right_named.len() >= 2 {
            first += usize::from(right_named.first() == Some(&host));
            last += usize::from(right_named.last() == Some(&host));
        }
        let holding: Vec<u32> = right_named
            .iter()
            .zip(&in_box)
            .filter(|(_, inside)| **inside)
            .map(|(wall, _)| *wall)
            .collect();
        match holding.as_slice() {
            [] => none += 1,
            [wall] if *wall == host => picks_host += 1,
            [_] => picks_other += 1,
            _ => ties += 1,
        }
    }
    println!(
        "corner_parts_lost n={} right_named0={} right_named1={} right_named2={} right_named3plus={} host_in_right_named={host_in} host_first={first} host_last={last} box_picks_host={picks_host} box_picks_other={picks_other} box_ties={ties} box_none={none}",
        parts.len(),
        by_named[0],
        by_named[1],
        by_named[2],
        by_named[3]
    );
}

fn main() -> anyhow::Result<()> {
    // Measure passes a single sample `--records` too.
    let files: Vec<String> = std::env::args()
        .skip(1)
        .filter(|arg| !arg.starts_with("--"))
        .collect();
    let Some(first) = files.first() else {
        anyhow::bail!("usage: probe_1865_part_walls A.rvt [B.rvt]");
    };
    let left = load(first)?;
    census("left", &left);
    if let Some(second) = files.get(1) {
        let right = load(second)?;
        census("right", &right);
        lost(&left, &right);
    }
    Ok(())
}
