//! RE-49: a beam's location line is in its element data, and with the
//! beam's record box it gives the beam's solid.
//!
//! FACT: on Autodesk's Snowdon Towers 2024 structural sample, the element
//! data of every one of the 942 structural-framing elements rvt-rs exports
//! carries a bounded line (`04 00 08 01 · f64 start · f64 end · f64×3
//! origin · f64×3 unit direction`). On 940 both ends lie inside the record
//! box, and on 923 a box `length × width × depth` along the line reproduces
//! the record box to 0.01 ft (the section solve of
//! `rvt::partition_beam_axes::beam_body`).
//!
//! Given a VIM export of the same model (`vimaec/vim-format`, MIT), the
//! probe measures each solved beam against the mesh the VIM holds for its
//! ElementId: Revit's own tessellated geometry. A VIM made from a later
//! edition can hold a different state of a beam, so only beams whose VIM
//! `Location` is the line's midpoint, whose VIM type is the one rvt-rs
//! names, and whose VIM mesh holds its own `Location` across the line are
//! scored. For each, the section across the line (width, depth and centre)
//! is compared with the mesh's, and so are the ends along it.
//!
//! Usage:
//!   cargo run --profile ci --example probe_re49_beam_axes -- \
//!     MODEL.rvt [MODEL.vim "Document Title"]

use rvt::RevitFile;
use rvt::partition_beam_axes::{END_TOLERANCE_FEET, beam_body, scan_beam_axes};
use rvt::partition_schema_mvp::{
    FAMILY_NAME_FIELD, TYPE_NAME_FIELD, element_record_bbox, recover_partition_schema_mvp,
};
use rvt::walker::{DecodedElement, InstanceField, WalkerLimits};
use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

/// Agreement, feet, for the VIM comparison.
const MATCH_FEET: f64 = 0.01;

fn u64_at(buf: &[u8], at: usize) -> Option<u64> {
    buf.get(at..at.checked_add(8)?)
        .map(|b| u64::from_le_bytes(b.try_into().expect("8 bytes")))
}

/// Named buffers of the BFAST container starting at `base`.
fn bfast(buf: &[u8], base: usize) -> Option<BTreeMap<String, (usize, usize)>> {
    let count = usize::try_from(u64_at(buf, base + 24)?).ok()?;
    let range = |index: usize| -> Option<(usize, usize)> {
        let begin = usize::try_from(u64_at(buf, base + 32 + 16 * index)?).ok()?;
        let end = usize::try_from(u64_at(buf, base + 40 + 16 * index)?).ok()?;
        Some((base + begin, base + end))
    };
    let (begin, end) = range(0)?;
    let names: Vec<&[u8]> = buf.get(begin..end)?.split(|&c| c == 0).collect();
    let mut out = BTreeMap::new();
    for index in 1..count {
        let name = String::from_utf8_lossy(names.get(index - 1)?).into_owned();
        out.insert(name, range(index)?);
    }
    Some(out)
}

fn words<const N: usize>(bytes: &[u8]) -> impl Iterator<Item = [u8; N]> + '_ {
    bytes
        .chunks_exact(N)
        .map(|c| <[u8; N]>::try_from(c).expect("N bytes"))
}

/// The parts of a VIM file this probe reads: per element its id, document,
/// family and type names and location; per element the mesh vertices of
/// every geometry instance it owns, in model feet.
struct Vim {
    ids: Vec<i64>,
    documents: Vec<i32>,
    titles: Vec<String>,
    families: Vec<String>,
    names: Vec<String>,
    locations: Vec<[f64; 3]>,
    points: BTreeMap<usize, Vec<[f64; 3]>>,
}

impl Vim {
    fn open(path: &PathBuf, wanted: &BTreeSet<i64>) -> Option<Self> {
        let data = std::fs::read(path).ok()?;
        let top = bfast(&data, 0)?;
        let entities = bfast(&data, top.get("entities")?.0)?;
        let (begin, end) = *top.get("strings")?;
        let strings: Vec<String> = data
            .get(begin..end)?
            .split(|&c| c == 0)
            .map(|s| String::from_utf8_lossy(s).into_owned())
            .collect();
        let raw = |table: &str, column: &str| -> Option<&[u8]> {
            let columns = bfast(&data, entities.get(table)?.0)?;
            let (begin, end) = *columns.get(column)?;
            data.get(begin..end)
        };
        let i32s = |table: &str, column: &str| -> Option<Vec<i32>> {
            Some(
                words::<4>(raw(table, column)?)
                    .map(i32::from_le_bytes)
                    .collect(),
            )
        };
        let f32s = |table: &str, column: &str| -> Option<Vec<f64>> {
            Some(
                words::<4>(raw(table, column)?)
                    .map(|w| f64::from(f32::from_le_bytes(w)))
                    .collect(),
            )
        };
        let texts = |table: &str, column: &str| -> Option<Vec<String>> {
            Some(
                i32s(table, column)?
                    .into_iter()
                    .map(|i| {
                        usize::try_from(i)
                            .ok()
                            .and_then(|i| strings.get(i))
                            .cloned()
                            .unwrap_or_default()
                    })
                    .collect(),
            )
        };
        let ids: Vec<i64> = words::<8>(raw("Vim.Element", "long:Id")?)
            .map(i64::from_le_bytes)
            .collect();
        let documents = i32s("Vim.Element", "index:Vim.BimDocument:BimDocument")?;
        let titles = texts("Vim.BimDocument", "string:Title")?;
        let families = texts("Vim.Element", "string:FamilyName")?;
        let names = texts("Vim.Element", "string:Name")?;
        let (lx, ly, lz) = (
            f32s("Vim.Element", "float:Location.X")?,
            f32s("Vim.Element", "float:Location.Y")?,
            f32s("Vim.Element", "float:Location.Z")?,
        );
        let locations = (0..ids.len()).map(|i| [lx[i], ly[i], lz[i]]).collect();
        let node_elements = i32s("Vim.Node", "index:Vim.Element:Element")?;

        let geometry = bfast(&data, top.get("geometry")?.0)?;
        let buffer = |name: &str| -> Option<&[u8]> {
            let (begin, end) = *geometry.get(name)?;
            data.get(begin..end)
        };
        let positions: Vec<f32> = words::<4>(buffer("g3d:vertex:position:0:float32:3")?)
            .map(f32::from_le_bytes)
            .collect();
        let corners: Vec<i32> = words::<4>(buffer("g3d:corner:index:0:int32:1")?)
            .map(i32::from_le_bytes)
            .collect();
        let mesh_submeshes: Vec<i32> = words::<4>(buffer("g3d:mesh:submeshoffset:0:int32:1")?)
            .map(i32::from_le_bytes)
            .collect();
        let submesh_corners: Vec<i32> = words::<4>(buffer("g3d:submesh:indexoffset:0:int32:1")?)
            .map(i32::from_le_bytes)
            .collect();
        let transforms: Vec<f32> = words::<4>(buffer("g3d:instance:transform:0:float32:16")?)
            .map(f32::from_le_bytes)
            .collect();
        let instance_meshes: Vec<i32> = words::<4>(buffer("g3d:instance:mesh:0:int32:1")?)
            .map(i32::from_le_bytes)
            .collect();
        let corner_range = |mesh: usize| -> Option<(usize, usize)> {
            let first = usize::try_from(*mesh_submeshes.get(mesh)?).ok()?;
            let last = mesh_submeshes
                .get(mesh + 1)
                .map_or(Some(submesh_corners.len()), |&s| usize::try_from(s).ok())?;
            let begin = usize::try_from(*submesh_corners.get(first)?).ok()?;
            let end = submesh_corners
                .get(last)
                .map_or(Some(corners.len()), |&c| usize::try_from(c).ok())?;
            Some((begin, end))
        };
        let mut points: BTreeMap<usize, Vec<[f64; 3]>> = BTreeMap::new();
        for (instance, &mesh) in instance_meshes.iter().enumerate() {
            let Some(element) = node_elements
                .get(instance)
                .and_then(|&e| usize::try_from(e).ok())
            else {
                continue;
            };
            if !ids.get(element).is_some_and(|id| wanted.contains(id)) {
                continue;
            }
            let Some((begin, end)) = usize::try_from(mesh).ok().and_then(corner_range) else {
                continue;
            };
            let Some(m) = transforms.get(16 * instance..16 * instance + 16) else {
                continue;
            };
            let m: Vec<f64> = m.iter().map(|&v| f64::from(v)).collect();
            let vertices: BTreeSet<usize> = corners[begin..end.min(corners.len())]
                .iter()
                .filter_map(|&c| usize::try_from(c).ok())
                .collect();
            let out = points.entry(element).or_default();
            for vertex in vertices {
                let Some(p) = positions.get(3 * vertex..3 * vertex + 3) else {
                    continue;
                };
                let (x, y, z) = (f64::from(p[0]), f64::from(p[1]), f64::from(p[2]));
                // Row vectors: the translation is the last row.
                out.push([
                    x * m[0] + y * m[4] + z * m[8] + m[12],
                    x * m[1] + y * m[5] + z * m[9] + m[13],
                    x * m[2] + y * m[6] + z * m[10] + m[14],
                ]);
            }
        }
        Some(Vim {
            ids,
            documents,
            titles,
            families,
            names,
            locations,
            points,
        })
    }
}

fn text(element: &DecodedElement, wanted: &str) -> Option<String> {
    element.fields.iter().find_map(|(name, value)| match value {
        InstanceField::String(v) if name == wanted => Some(v.clone()),
        _ => None,
    })
}

fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn extent(points: &[[f64; 3]], axis: [f64; 3]) -> (f64, f64) {
    points
        .iter()
        .fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), p| {
            let v = dot(*p, axis);
            (lo.min(v), hi.max(v))
        })
}

fn main() -> rvt::Result<()> {
    let mut args = std::env::args().skip(1);
    let path = PathBuf::from(args.next().expect("usage: MODEL.rvt [MODEL.vim TITLE]"));
    let vim_path = args.next().map(PathBuf::from);
    let title = args.next();
    let mut rf = RevitFile::open(&path)?;
    let version = rf.basic_file_info()?.version;
    let mvp = recover_partition_schema_mvp(&mut rf, version, WalkerLimits::default())?;
    let beams: Vec<&DecodedElement> = mvp
        .products
        .iter()
        .filter(|element| element.class == "StructuralFraming" && element.id.is_some())
        .collect();
    let ids: BTreeSet<u32> = beams.iter().filter_map(|beam| beam.id).collect();
    let lines = scan_beam_axes(&mut rf, version, &ids)?;

    let mut census: BTreeMap<&str, usize> = BTreeMap::new();
    let mut solved = Vec::new();
    for beam in &beams {
        let id = beam.id.expect("filtered");
        let (Some(line), Some(bbox)) = (lines.get(&id), element_record_bbox(beam)) else {
            *census.entry("no line in its data").or_default() += 1;
            continue;
        };
        let inside = [line.start(), line.end()].iter().all(|p| {
            (0..3).all(|axis| {
                p[axis] >= bbox[axis] - END_TOLERANCE_FEET
                    && p[axis] <= bbox[axis + 3] + END_TOLERANCE_FEET
            })
        });
        if !inside {
            *census.entry("line leaves the record box").or_default() += 1;
            continue;
        }
        match beam_body(bbox, line.start(), line.end()) {
            None => *census.entry("no box along the line closes").or_default() += 1,
            Some(body) => {
                let kind = if !body.is_horizontal() {
                    "solved: sloped"
                } else if body.direction[0].abs() < 1e-9 || body.direction[1].abs() < 1e-9 {
                    "solved: level, on a model axis"
                } else {
                    "solved: level, rotated in plan"
                };
                *census.entry(kind).or_default() += 1;
                solved.push((*beam, *line, body));
            }
        }
    }
    println!(
        "release {version}: {} placed structural-framing elements",
        beams.len()
    );
    for (what, count) in &census {
        println!("  {count:>5}  {what}");
    }

    let (Some(vim_path), Some(title)) = (vim_path, title) else {
        return Ok(());
    };
    let wanted: BTreeSet<i64> = solved
        .iter()
        .filter_map(|(beam, _, _)| beam.id.map(i64::from))
        .collect();
    let Some(vim) = Vim::open(&vim_path, &wanted) else {
        eprintln!("could not read {}", vim_path.display());
        return Ok(());
    };
    let by_id: BTreeMap<i64, usize> = (0..vim.ids.len())
        .filter(|&i| {
            usize::try_from(vim.documents[i])
                .ok()
                .and_then(|d| vim.titles.get(d))
                .is_some_and(|t| *t == title)
        })
        .map(|i| (vim.ids[i], i))
        .collect();
    let mut scores: BTreeMap<String, usize> = BTreeMap::new();
    let mut score = |key: String| *scores.entry(key).or_default() += 1;
    let mut trims: Vec<f64> = Vec::new();
    for (beam, line, body) in &solved {
        let id = i64::from(beam.id.expect("filtered"));
        let Some(&element) = by_id.get(&id) else {
            score("not in the VIM".into());
            continue;
        };
        let Some(mesh) = vim.points.get(&element).filter(|p| !p.is_empty()) else {
            score("no VIM mesh".into());
            continue;
        };
        let (start, end) = (line.start(), line.end());
        let midpoint = [
            (start[0] + end[0]) / 2.0,
            (start[1] + end[1]) / 2.0,
            (start[2] + end[2]) / 2.0,
        ];
        let location = vim.locations[element];
        if (0..3).any(|axis| (midpoint[axis] - location[axis]).abs() > MATCH_FEET) {
            score("moved in the VIM's edition".into());
            continue;
        }
        let ours = text(beam, FAMILY_NAME_FIELD).zip(text(beam, TYPE_NAME_FIELD));
        let theirs = (vim.families[element].clone(), vim.names[element].clone());
        if ours.as_ref() != Some(&theirs) {
            score("another type in the VIM's edition".into());
            continue;
        }
        let d = body.direction;
        let plan = d[0].hypot(d[1]);
        let u = [-d[1] / plan, d[0] / plan, 0.0];
        let w = [-d[2] * d[0] / plan, -d[2] * d[1] / plan, plan];
        let (mu0, mu1) = extent(mesh, u);
        if !(mu0 - MATCH_FEET..=mu1 + MATCH_FEET).contains(&dot(location, u)) {
            score("VIM mesh does not hold its own Location".into());
            continue;
        }
        let kind = if !body.is_horizontal() {
            "sloped"
        } else if d[0].abs() < 1e-9 || d[1].abs() < 1e-9 {
            "level, on a model axis"
        } else {
            "level, rotated in plan"
        };
        let (mw0, mw1) = extent(mesh, w);
        let (md0, md1) = extent(mesh, d);
        let (cu, cw, cd) = (
            dot(body.centre, u),
            dot(body.centre, w),
            dot(body.centre, d),
        );
        let width_gap = body.width_feet - (mu1 - mu0);
        let depth_gap = body.depth_feet - (mw1 - mw0);
        let across_gap = cu - (mu0 + mu1) / 2.0;
        let up_gap = cw - (mw0 + mw1) / 2.0;
        let section = if [width_gap, depth_gap, across_gap, up_gap]
            .iter()
            .all(|g| g.abs() < MATCH_FEET)
        {
            "section equals the mesh"
        } else if width_gap.abs() < MATCH_FEET
            && across_gap.abs() < MATCH_FEET
            && depth_gap > MATCH_FEET
            && (up_gap - depth_gap / 2.0).abs() < MATCH_FEET
        {
            "mesh is the section less its top (a floor join cut it)"
        } else {
            "section differs from the mesh"
        };
        score(format!("{kind}: {section}"));
        let (low, high) = (cd - body.length_feet / 2.0, cd + body.length_feet / 2.0);
        let ends = if (md0 - low).abs() < MATCH_FEET && (high - md1).abs() < MATCH_FEET {
            "ends equal the mesh"
        } else if md0 > low - MATCH_FEET && md1 < high + MATCH_FEET {
            trims.push((md0 - low).max(high - md1));
            "mesh is trimmed inside the line"
        } else {
            "mesh runs past an end of the line"
        };
        score(format!("{kind}: {ends}"));
    }
    println!("against {} in {}:", title, vim_path.display());
    for (what, count) in &scores {
        println!("  {count:>5}  {what}");
    }
    trims.sort_by(f64::total_cmp);
    if let (Some(first), Some(last)) = (trims.first(), trims.last()) {
        println!(
            "  trims: {} beams, largest end trim {first:.3} to {last:.3} ft, median {:.3} ft",
            trims.len(),
            trims[trims.len() / 2]
        );
    }
    Ok(())
}
