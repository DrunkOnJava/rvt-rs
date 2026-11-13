//! #1865: where does a curtain grid's element data name its curtain wall on
//! Revit 2027, against 2025?
//!
//! FACT under test: a curtain part names the curtain grid it lies on in its
//! reference list, and the grid's own element data names its curtain wall at
//! two offsets past the grid's ElementId, `CURTAIN_GRID_WALL_OFFSETS` = 85 and
//! 478 (RE-72); a part with such a grid takes that wall as its whole before
//! anything else, and a curtain door or window takes no other (RE-46). On
//! Autodesk's `rac_advanced` saved by Revit 2027, 80 curtain parts (79 `L Mullion 1`
//! corner mullions that name two curtain walls and a curtain door that names
//! one) are parts of a curtain wall in the 2025 save and of none in the 2027
//! save (#1865, probe_1865_part_walls). If the 2027 grids hold their wall at
//! other offsets, the grid lookup finds nothing on 2027, the corner mullions
//! fall to RE-46's exactly-one rule and the door has no other way in: one cause
//! for all 80. This prints, for each of the 80, every ElementId its list names
//! that is no curtain wall, and where in that element's data (first
//! occurrence of the release's element-data header followed by the id) a
//! curtain wall's ElementId lies, as offsets past the id.
//!
//! The probe takes two files (Measure's `probe_pair_release`: a sample, then
//! the same project saved by another release); it needs both exports to find
//! the 80, so it prints names and ElementIds and is for Autodesk's samples.
//!
//! Output:
//!   lost_part tag=<ElementId> type=<IFC entity> left_wall=<ElementId>
//!   grid_data side=<left|right> part=<ElementId> id=<ElementId> copies=<n>
//!     walls_at=<offset>:<ElementId>,... (offsets past the id's 8 bytes)
//!     (one line per id the part's list names that is no curtain wall, has
//!     element data, and has a curtain wall's id within 1,200 bytes of it)
//!   grid_offsets side=<left|right> offsets=<offset,...> parts=<n>
//!     (the offsets at which a curtain wall of the part's own list lies, as a
//!     set, over the parts that have any; RE-72's are 85,478)
//!   lookup side=<left|right> part=<ElementId> named=<ElementIds> on_grid=<ElementIds>
//!     (the two sets `attach_curtain_walls` matches on: the curtain walls the
//!     list names, and the walls the list's other ids give as grids at RE-72's
//!     offsets 85 and 478, where the two agree and are a wall)
//!   lookup_arms side=<left|right> (on_grid,named)=<n> ...
//!     (how many parts have each pair of set sizes: (1,*) takes the grid's wall,
//!     (0,1) the named one, (0,0) the box container, any other pair none)
//!
//! Usage:
//!   cargo run --profile ci -p rvt-probes --example probe_1865_grid_walls -- A.rvt B.rvt

use std::collections::{BTreeMap, BTreeSet, HashMap};

use rvt::RevitFile;
use rvt::ifc::entities::IfcEntity;
use rvt::ifc::{ExportOptions, run_export};
use rvt::research::partition_element_records as per;

/// How far past an element's id the walls are looked for, bytes.
const WINDOW: usize = 1200;

/// Where a curtain grid's element data names its wall, past the id's 8 bytes
/// (`CURTAIN_GRID_WALL_OFFSETS`, RE-72), and the two must agree.
const RE72_OFFSETS: [usize; 2] = [85, 478];

/// One save: its curtain walls, which curtain wall each aggregated part is a
/// part of, what each element is, and the records of the categories a curtain
/// part belongs to.
struct Side {
    rf: RevitFile,
    version: u32,
    walls: BTreeSet<u32>,
    all_walls: BTreeSet<u32>,
    whole_of: HashMap<u32, u32>,
    types: HashMap<u32, String>,
    records: BTreeMap<u32, Vec<per::PartitionElementRecord>>,
}

fn load(path: &str) -> anyhow::Result<Side> {
    let mut rf = RevitFile::open_with_limits(path, rvt::cli::open_limits(None, None, false))?;
    let model = run_export(&mut rf, &ExportOptions::default())?.model;
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
    let mut walls = BTreeSet::new();
    let mut types = HashMap::new();
    for (ifc_type, tag) in info.iter().flatten() {
        types.insert(*tag, ifc_type.clone());
        if ifc_type == "IFCCURTAINWALL" {
            walls.insert(*tag);
        }
    }
    let mut whole_of = HashMap::new();
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
    let all_walls: BTreeSet<u32> = records
        .iter()
        .filter(|(_, records)| {
            records
                .iter()
                .any(|r| r.builtin_category == per::OST_WALLS && r.is_exported_instance())
        })
        .map(|(id, _)| *id)
        .collect();
    Ok(Side {
        rf,
        version,
        walls,
        all_walls,
        whole_of,
        types,
        records,
    })
}

/// The ElementIds `part`'s first record's list names, in list order, split
/// into the curtain walls and the rest.
fn list_ids(side: &Side, part: u32) -> (Vec<u32>, Vec<u32>) {
    let (mut named, mut others) = (Vec::new(), Vec::new());
    let Some(record) = side.records.get(&part).and_then(|records| records.first()) else {
        return (named, others);
    };
    for slot in &record.references {
        let Ok(id) = u32::try_from(*slot) else {
            continue;
        };
        if id == part {
            continue;
        }
        let into = if side.walls.contains(&id) {
            &mut named
        } else {
            &mut others
        };
        if !into.contains(&id) {
            into.push(id);
        }
    }
    (named, others)
}

fn u64_at(buf: &[u8], at: usize) -> Option<u64> {
    Some(u64::from_le_bytes(buf.get(at..at + 8)?.try_into().ok()?))
}

/// For each id of `wanted`, every place the release's element-data header is
/// followed by it, with the offsets (past the id) at which a curtain wall lies
/// within [`WINDOW`] bytes.
fn data_of(side: &mut Side, wanted: &BTreeSet<u32>) -> BTreeMap<u32, Vec<Vec<(usize, u32)>>> {
    let mut out: BTreeMap<u32, Vec<Vec<(usize, u32)>>> = BTreeMap::new();
    let Some(header) = rvt::research::partition_names::element_data_header(side.version) else {
        return out;
    };
    for stream in side.rf.partition_stream_names() {
        let Ok(inflated) = side.rf.inflated_partition(&stream) else {
            continue;
        };
        let buf = inflated.bytes();
        for hit in memchr::memmem::find_iter(buf, &header) {
            let id_at = hit + header.len();
            let Some(id) = u64_at(buf, id_at).and_then(|id| u32::try_from(id).ok()) else {
                continue;
            };
            if !wanted.contains(&id) {
                continue;
            }
            let mut found = Vec::new();
            for offset in 0..WINDOW {
                if let Some(value) = u64_at(buf, id_at + 8 + offset) {
                    if let Ok(wall) = u32::try_from(value) {
                        if side.walls.contains(&wall) {
                            found.push((offset, wall));
                        }
                    }
                }
            }
            out.entry(id).or_default().push(found);
        }
    }
    out
}

/// What `scan_curtain_grid_walls` gives for `wanted` on this save: for each id,
/// the wall its element data names at both of RE-72's offsets where the two
/// agree and are a wall, if every such place agrees on one.
fn grid_walls(side: &mut Side, wanted: &BTreeSet<u32>) -> BTreeMap<u32, u32> {
    let mut found: BTreeMap<u32, Option<u32>> = BTreeMap::new();
    let Some(header) = rvt::research::partition_names::element_data_header(side.version) else {
        return BTreeMap::new();
    };
    for stream in side.rf.partition_stream_names() {
        let Ok(inflated) = side.rf.inflated_partition(&stream) else {
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
            let wall = match (first, second) {
                (Some(first), Some(second)) if first == second => u32::try_from(first)
                    .ok()
                    .filter(|wall| side.all_walls.contains(wall)),
                _ => None,
            };
            let Some(wall) = wall else {
                continue;
            };
            match found.get_mut(&id) {
                None => {
                    found.insert(id, Some(wall));
                }
                Some(held) => {
                    if *held != Some(wall) {
                        *held = None;
                    }
                }
            }
        }
    }
    found
        .into_iter()
        .filter_map(|(id, wall)| Some((id, wall?)))
        .collect()
}

/// The two sets `attach_curtain_walls` matches on, for each lost part.
fn lookup(label: &str, side: &mut Side, lost: &BTreeSet<u32>) {
    let mut wanted = BTreeSet::new();
    let mut per_part: BTreeMap<u32, (Vec<u32>, Vec<u32>)> = BTreeMap::new();
    for part in lost {
        let (named, others) = list_ids(side, *part);
        wanted.extend(others.iter().copied());
        per_part.insert(*part, (named, others));
    }
    let grids = grid_walls(side, &wanted);
    let mut arms: BTreeMap<(usize, usize), usize> = BTreeMap::new();
    for (part, (named, others)) in &per_part {
        let on_grid: BTreeSet<u32> = others
            .iter()
            .filter_map(|id| grids.get(id).copied())
            .collect();
        println!(
            "lookup side={label} part={part} named={} on_grid={}",
            ids_text(named),
            ids_text(&on_grid.iter().copied().collect::<Vec<_>>())
        );
        *arms.entry((on_grid.len(), named.len())).or_default() += 1;
    }
    println!(
        "lookup_arms side={label} {}",
        arms.iter()
            .map(|((grid, named), parts)| format!("({grid},{named})={parts}"))
            .collect::<Vec<_>>()
            .join(" ")
    );
}

fn ids_text(ids: &[u32]) -> String {
    if ids.is_empty() {
        "-".to_string()
    } else {
        ids.iter().map(u32::to_string).collect::<Vec<_>>().join(",")
    }
}

fn report(label: &str, side: &mut Side, lost: &BTreeSet<u32>) {
    let mut per_part: BTreeMap<u32, Vec<u32>> = BTreeMap::new();
    let mut wanted = BTreeSet::new();
    for part in lost {
        let (_, others) = list_ids(side, *part);
        wanted.extend(others.iter().copied());
        per_part.insert(*part, others);
    }
    let data = data_of(side, &wanted);
    let mut offsets_seen: BTreeMap<Vec<usize>, usize> = BTreeMap::new();
    for (part, others) in &per_part {
        let (named, _) = list_ids(side, *part);
        let mut part_offsets: BTreeSet<usize> = BTreeSet::new();
        for id in others {
            let Some(copies) = data.get(id) else {
                continue;
            };
            let at: Vec<String> = copies
                .iter()
                .flatten()
                .filter(|(_, wall)| named.contains(wall))
                .map(|(offset, wall)| {
                    part_offsets.insert(*offset);
                    format!("{offset}:{wall}")
                })
                .collect();
            if !at.is_empty() {
                println!(
                    "grid_data side={label} part={part} id={id} copies={} walls_at={}",
                    copies.len(),
                    at.join(",")
                );
            }
        }
        if !part_offsets.is_empty() {
            *offsets_seen
                .entry(part_offsets.into_iter().collect())
                .or_default() += 1;
        }
    }
    for (offsets, parts) in &offsets_seen {
        println!(
            "grid_offsets side={label} offsets={} parts={parts}",
            offsets
                .iter()
                .map(usize::to_string)
                .collect::<Vec<_>>()
                .join(",")
        );
    }
    if offsets_seen.is_empty() {
        println!("grid_offsets side={label} offsets=- parts=0");
    }
}

fn main() -> anyhow::Result<()> {
    // Measure passes a single sample `--records` too.
    let files: Vec<String> = std::env::args()
        .skip(1)
        .filter(|arg| !arg.starts_with("--"))
        .collect();
    let [first, second, ..] = files.as_slice() else {
        anyhow::bail!("usage: probe_1865_grid_walls A.rvt B.rvt");
    };
    let mut left = load(first)?;
    let mut right = load(second)?;
    let lost: BTreeSet<u32> = left
        .whole_of
        .keys()
        .filter(|id| !right.whole_of.contains_key(*id))
        .copied()
        .collect();
    for tag in &lost {
        println!(
            "lost_part tag={tag} type={} left_wall={}",
            right.types.get(tag).map_or("?", String::as_str),
            left.whole_of[tag]
        );
    }
    report("left", &mut left, &lost);
    report("right", &mut right, &lost);
    lookup("left", &mut left, &lost);
    lookup("right", &mut right, &lost);
    Ok(())
}
