//! #1912: what do the doors and windows that Revit's export aggregates into a
//! curtain wall, and rvt-rs does not, say about that wall?
//!
//! FACT under test: a door or window takes a curtain wall as its aggregate whole
//! through the one curtain grid its record names (RE-46, RE-72) or, with no grid
//! found, through the one curtain wall its reference list names when the element
//! data of an id in the list names that wall too: at least two copies of its
//! ElementId within 1,200 bytes past the id (RE-452, `scan_walls_in_element_data`).
//! On the four BIM4LCA offices that puts 184 of the 268 doors Revit's export
//! aggregates into a curtain wall in the same wall and leaves 84 (21 in each
//! office) without one (#1865, #1912). Which of them does the list name, which
//! does the element data name, which does the record box lie in, and is Revit's
//! wall one rvt-rs counts as a curtain wall at all?
//!
//! The probe prints, for every door and window with an exported instance record,
//! what its list, its grid, its element data and its record box say about walls.
//! It does not compare them with Revit's export: `tools/re/curtain_openings_vs_ifc.py`
//! does, and tests candidate rules on every opening of the model, those Revit
//! leaves standalone included. It prints ElementIds, so for a model fetched
//! without a licence to redistribute it the run's scrub drops its output
//! (`tools/ci/scrub-holdout-results.sh` keeps only the count lines it names).
//!
//! Output (one model per run):
//!   curtain_walls n=<n> ids=<ElementIds, or ->
//!     (the walls rvt-rs writes as `IfcCurtainWall`)
//!   walls n=<n> ids=<ElementIds, or ->
//!     (every wall with an exported instance record)
//!   opening tag=<ElementId> kind=<door|window> exported=<y|n> ours=<ElementId|->
//!     walls=<ElementIds> curtain=<ElementIds> grid=<id>><wall>,... data=<id>><wall>*<c>[/<c>...],...
//!     box=<ElementIds> ids=<id>:<category>/d<n>,...
//!     - `ours`: the curtain wall rvt-rs's export aggregates it into;
//!     - `walls`: the walls its list names (itself left out), `curtain` those of them
//!       that rvt-rs counts as curtain walls;
//!     - `grid`: an id of its list that is no wall and whose element data names a wall
//!       at both of RE-72's offsets (85 and 478), if every copy agrees;
//!     - `data`: an id of its list that is no wall and a wall whose ElementId lies in
//!       its element data (within 1,200 bytes past the id), with the number of
//!       places it lies at in each copy of the data, copies separated by `/`;
//!     - `box`: the walls whose record box lies around its own, within 0.05 ft;
//!     - `ids`: every id of its list that is no wall, with the category of its
//!       element record (`none` where it has none: a type, a family, a grid) and the
//!       number of copies of its element data (`d0` where it has none).
//!   openings kind=<door|window> n=<n> ours=<n>
//!
//! Usage:
//!   cargo run --profile ci -p rvt-probes --example probe_1912_curtain_openings -- model.rvt

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

use rvt::RevitFile;
use rvt::ifc::entities::IfcEntity;
use rvt::ifc::{ExportOptions, run_export};
use rvt::research::partition_element_records as per;

/// How far past an element's id the walls are looked for, bytes
/// (`ELEMENT_DATA_WALL_WINDOW`).
const WINDOW: usize = 1200;

/// Where a curtain grid's element data names its wall, past the id's 8 bytes
/// (`CURTAIN_GRID_WALL_OFFSETS`, RE-72), and the two must agree.
const RE72_OFFSETS: [usize; 2] = [85, 478];

/// How far past a wall's record box an opening's box may reach and still lie
/// inside it, feet (RE-62).
const TOLERANCE_FEET: f64 = 0.05;

/// What a scan of the element data finds for the ids asked for.
struct Scan {
    /// The wall both of RE-72's offsets name, per id, if every copy of its data
    /// agrees on one.
    grid: BTreeMap<u32, u32>,
    /// For each copy of an id's data, how many places each wall's ElementId lies
    /// at within [`WINDOW`].
    data: BTreeMap<u32, Vec<BTreeMap<u32, usize>>>,
}

fn u64_at(buf: &[u8], at: usize) -> Option<u64> {
    Some(u64::from_le_bytes(buf.get(at..at + 8)?.try_into().ok()?))
}

fn ids_text<'a>(ids: impl IntoIterator<Item = &'a u32>) -> String {
    let text: Vec<String> = ids.into_iter().map(u32::to_string).collect();
    if text.is_empty() {
        "-".to_string()
    } else {
        text.join(",")
    }
}

/// One pass over the partitions: for each id of `wanted`, the grid wall and the
/// walls its element data names.
fn scan(rf: &mut RevitFile, version: u32, wanted: &BTreeSet<u32>, walls: &BTreeSet<u32>) -> Scan {
    let mut grid: BTreeMap<u32, Option<u32>> = BTreeMap::new();
    let mut data: BTreeMap<u32, Vec<BTreeMap<u32, usize>>> = BTreeMap::new();
    let Some(header) = rvt::research::partition_names::element_data_header(version) else {
        return Scan {
            grid: BTreeMap::new(),
            data,
        };
    };
    for stream in rf.partition_stream_names() {
        let Ok(inflated) = rf.inflated_partition(&stream) else {
            continue;
        };
        let buf = inflated.bytes();
        for hit in memchr::memmem::find_iter(buf, &header) {
            let id_at = hit + header.len();
            let Some(id) = u64_at(buf, id_at)
                .and_then(|id| u32::try_from(id).ok())
                .filter(|id| wanted.contains(id))
            else {
                continue;
            };
            let [first, second] = RE72_OFFSETS.map(|offset| u64_at(buf, id_at + 8 + offset));
            if let (Some(first), Some(second)) = (first, second) {
                if first == second {
                    if let Some(wall) = u32::try_from(first).ok().filter(|w| walls.contains(w)) {
                        match grid.get_mut(&id) {
                            None => {
                                grid.insert(id, Some(wall));
                            }
                            Some(held) => {
                                if *held != Some(wall) {
                                    *held = None;
                                }
                            }
                        }
                    }
                }
            }
            let mut copies: BTreeMap<u32, usize> = BTreeMap::new();
            for offset in 0..WINDOW {
                let wall = u64_at(buf, id_at + 8 + offset)
                    .and_then(|value| u32::try_from(value).ok())
                    .filter(|wall| walls.contains(wall));
                if let Some(wall) = wall {
                    *copies.entry(wall).or_default() += 1;
                }
            }
            data.entry(id).or_default().push(copies);
        }
    }
    Scan {
        grid: grid
            .into_iter()
            .filter_map(|(id, wall)| Some((id, wall?)))
            .collect(),
        data,
    }
}

/// `inner` lies inside `outer` within [`TOLERANCE_FEET`], both `[min x, min y,
/// min z, max x, max y, max z]`.
fn contains(outer: &[f64; 6], inner: &[f64; 6]) -> bool {
    (0..3).all(|axis| {
        inner[axis] >= outer[axis] - TOLERANCE_FEET
            && inner[axis + 3] <= outer[axis + 3] + TOLERANCE_FEET
    })
}

fn main() -> anyhow::Result<()> {
    // Measure passes a single sample `--records` too.
    let files: Vec<String> = std::env::args()
        .skip(1)
        .filter(|arg| !arg.starts_with("--"))
        .collect();
    let Some(path) = files.first() else {
        anyhow::bail!("usage: probe_1912_curtain_openings model.rvt");
    };
    let mut rf = RevitFile::open_with_limits(path, rvt::cli::open_limits(None, None, false))?;
    let model = run_export(&mut rf, &ExportOptions::default())?.model;
    // Each entity: its IFC entity and its Tag as an ElementId.
    let info: Vec<Option<(String, u32)>> = model
        .entities
        .iter()
        .map(|entity| match entity {
            IfcEntity::BuildingElement {
                ifc_type,
                type_guid,
                ..
            } => type_guid
                .as_deref()
                .and_then(|tag| tag.parse::<u32>().ok())
                .map(|tag| (ifc_type.clone(), tag)),
            _ => None,
        })
        .collect();
    let mut curtain: BTreeSet<u32> = BTreeSet::new();
    let mut exported: HashSet<u32> = HashSet::new();
    for (ifc_type, tag) in info.iter().flatten() {
        exported.insert(*tag);
        if ifc_type == "IFCCURTAINWALL" {
            curtain.insert(*tag);
        }
    }
    let mut whole_of: HashMap<u32, u32> = HashMap::new();
    for entity in &model.entities {
        let IfcEntity::Aggregate { whole, parts } = entity else {
            continue;
        };
        let Some(Some((ifc_type, wall))) = info.get(*whole) else {
            continue;
        };
        if ifc_type != "IFCCURTAINWALL" {
            continue;
        }
        for part in parts {
            if let Some(Some((_, tag))) = info.get(*part) {
                whole_of.entry(*tag).or_insert(*wall);
            }
        }
    }
    let version = rf.basic_file_info()?.version;
    let declared = rvt::elem_table::declared_ids(&rvt::elem_table::parse_records(&mut rf)?);
    // Every category, so that the ids of an opening's list can be told by what
    // they are: a wall, a Level, a curtain panel or a grid-like element.
    let categories: Vec<i64> = per::BUILTIN_CATEGORY_NAMES
        .iter()
        .map(|(id, _)| *id)
        .chain(per::PRODUCT_RECORD_CATEGORIES.iter().map(|(id, _)| *id))
        .chain([per::OST_WALLS, per::OST_DOORS, per::OST_WINDOWS])
        .collect::<BTreeSet<i64>>()
        .into_iter()
        .collect();
    let mut records: BTreeMap<u32, Vec<per::PartitionElementRecord>> = BTreeMap::new();
    for record in per::scan_category_records_multi(&mut rf, version, &categories, &declared)? {
        records.entry(record.element_id).or_default().push(record);
    }
    let category_of: HashMap<u32, &'static str> = records
        .iter()
        .filter_map(|(id, records)| {
            let record = records.first()?;
            Some((
                *id,
                per::builtin_category_name(record.builtin_category).unwrap_or("unnamed"),
            ))
        })
        .collect();
    // The walls, with the box of each one's first wall record.
    let wall_boxes: BTreeMap<u32, [f64; 6]> = records
        .iter()
        .filter_map(|(id, records)| {
            let record = records
                .iter()
                .find(|r| r.builtin_category == per::OST_WALLS && r.is_exported_instance())?;
            Some((*id, record.bbox_feet))
        })
        .collect();
    let walls: BTreeSet<u32> = wall_boxes.keys().copied().collect();
    println!(
        "curtain_walls n={} ids={}",
        curtain.len(),
        ids_text(&curtain)
    );
    println!("walls n={} ids={}", walls.len(), ids_text(&walls));
    // The openings, and the ids of their lists that are no wall: the element data
    // of those is what names a wall.
    let mut openings: Vec<(u32, &'static str, &per::PartitionElementRecord)> = Vec::new();
    for (id, records) in &records {
        for (category, kind) in [(per::OST_DOORS, "door"), (per::OST_WINDOWS, "window")] {
            if let Some(record) = records
                .iter()
                .find(|r| r.builtin_category == category && r.is_exported_instance())
            {
                openings.push((*id, kind, record));
            }
        }
    }
    let mut wanted: BTreeSet<u32> = BTreeSet::new();
    for (id, _, record) in &openings {
        for slot in &record.references {
            if let Ok(other) = u32::try_from(*slot) {
                if other != *id && !walls.contains(&other) {
                    wanted.insert(other);
                }
            }
        }
    }
    let scanned = scan(&mut rf, version, &wanted, &walls);
    let mut counts: BTreeMap<&str, (usize, usize)> = BTreeMap::new();
    for (id, kind, record) in &openings {
        let mut named: Vec<u32> = Vec::new();
        let mut others: Vec<u32> = Vec::new();
        for slot in &record.references {
            let Ok(other) = u32::try_from(*slot) else {
                continue;
            };
            if other == *id {
                continue;
            }
            let into = if walls.contains(&other) {
                &mut named
            } else {
                &mut others
            };
            if !into.contains(&other) {
                into.push(other);
            }
        }
        let on_curtain: Vec<u32> = named
            .iter()
            .copied()
            .filter(|wall| curtain.contains(wall))
            .collect();
        let grids: Vec<String> = others
            .iter()
            .filter_map(|other| Some(format!("{other}>{}", scanned.grid.get(other)?)))
            .collect();
        let mut data: Vec<String> = Vec::new();
        for other in &others {
            let Some(copies) = scanned.data.get(other) else {
                continue;
            };
            let named_walls: BTreeSet<u32> =
                copies.iter().flat_map(|c| c.keys().copied()).collect();
            for wall in named_walls {
                let places: Vec<String> = copies
                    .iter()
                    .map(|copy| copy.get(&wall).copied().unwrap_or(0).to_string())
                    .collect();
                data.push(format!("{other}>{wall}*{}", places.join("/")));
            }
        }
        let inside: Vec<u32> = wall_boxes
            .iter()
            .filter(|(_, wall)| contains(wall, &record.bbox_feet))
            .map(|(wall, _)| *wall)
            .collect();
        // Each id of the list that is no wall: what it is, if it has an element
        // record, and how many places its element data lies at.
        let listed: Vec<String> = others
            .iter()
            .map(|other| {
                let category = category_of.get(other).copied().unwrap_or("none");
                let places = scanned.data.get(other).map_or(0, Vec::len);
                format!("{other}:{}/d{places}", category.replace(' ', "_"))
            })
            .collect();
        let ours = whole_of.get(id).copied();
        let entry = counts.entry(kind).or_default();
        entry.0 += 1;
        entry.1 += usize::from(ours.is_some());
        println!(
            "opening tag={id} kind={kind} exported={} ours={} walls={} curtain={} grid={} data={} box={} ids={}",
            if exported.contains(id) { "y" } else { "n" },
            ours.map_or("-".to_string(), |wall| wall.to_string()),
            ids_text(&named),
            ids_text(&on_curtain),
            if grids.is_empty() {
                "-".to_string()
            } else {
                grids.join(",")
            },
            if data.is_empty() {
                "-".to_string()
            } else {
                data.join(",")
            },
            ids_text(&inside),
            if listed.is_empty() {
                "-".to_string()
            } else {
                listed.join(",")
            },
        );
    }
    for (kind, (n, ours)) in &counts {
        println!("openings kind={kind} n={n} ours={ours}");
    }
    Ok(())
}
