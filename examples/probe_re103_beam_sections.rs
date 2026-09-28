//! RE-103: a steel framing type stores its I section, and its centroid
//! tells an I from a channel.
//!
//! FACT: on Autodesk's Snowdon Towers 2024 structural sample, the type of
//! each wide-flange beam stores its flange width, depth, web thickness and
//! flange thickness under BuiltInParameters -1005502, -1005503, -1005525
//! and -1005524 (`f64 feet · ff×8 · i64 parameter`, RE-93), and its
//! centroid under -1005508 (from the left) and -1005509 (from the bottom).
//! A wide flange's centroid is at its centre both ways; a channel stores
//! the same four dimensions with its centroid off centre across.
//!
//! Given a VIM export of the same model (`vimaec/vim-format`, MIT), the
//! probe checks, for each beam rvt-rs draws as an I, that Revit's mesh has
//! a vertex at each of the section's four outer corners and four inner
//! flange tips, where a solid box has none at the tips. Beams are scored
//! only where the VIM's edition holds them as the 2024 file does (RE-49).
//!
//! Usage:
//!   cargo run --profile ci --example probe_re103_beam_sections -- \
//!     MODEL.rvt [MODEL.vim "Document Title"]

use rvt::RevitFile;
use rvt::partition_beam_axes::{beam_body, i_section_body, scan_bounded_lines};
use rvt::partition_schema_mvp::{
    FAMILY_NAME_FIELD, TYPE_NAME_FIELD, beam_i_section_from_fields, element_record_bbox,
    recover_partition_schema_mvp,
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
    let lines = scan_bounded_lines(&mut rf, version, &ids)?;
    let mut census: BTreeMap<String, usize> = BTreeMap::new();
    let mut drawn = Vec::new();
    for beam in &beams {
        let section = beam_i_section_from_fields(&beam.fields);
        let line = beam.id.and_then(|id| lines.get(&id)).copied();
        let bbox = element_record_bbox(beam);
        let body = line
            .zip(bbox)
            .and_then(|(line, bbox)| beam_body(bbox, line.start(), line.end()));
        let key = match (section, body, line.zip(bbox)) {
            (None, _, _) => "type is not an I section",
            (Some(section), Some(body), Some((line, bbox))) => {
                match i_section_body(body, &section, bbox, line.start(), line.end()) {
                    Some(drawn_body) => {
                        drawn.push((*beam, section, drawn_body));
                        if drawn_body == body {
                            "drawn as its I section: body as deep as the section"
                        } else {
                            "drawn as its I section: set down from its line"
                        }
                    }
                    None => "I section, body does not hold it",
                }
            }
            _ => "I section, no body along its line",
        };
        let type_name = text(beam, TYPE_NAME_FIELD).unwrap_or_default();
        *census.entry(key.to_string()).or_default() += 1;
        *census.entry(format!("  {key}: {type_name}")).or_default() += 1;
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
    let wanted: BTreeSet<i64> = drawn
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
    for (beam, section, body) in &drawn {
        let id = i64::from(beam.id.expect("filtered"));
        let verdict = (|| {
            let Some(&element) = by_id.get(&id) else {
                return "not in the VIM".to_string();
            };
            let Some(mesh) = vim.points.get(&element).filter(|p| !p.is_empty()) else {
                return "no VIM mesh".to_string();
            };
            if let Some(line) = beam.id.and_then(|id| lines.get(&id)) {
                let (start, end) = (line.start(), line.end());
                let location = vim.locations[element];
                if (0..3).any(|axis| {
                    ((start[axis] + end[axis]) / 2.0 - location[axis]).abs() > MATCH_FEET
                }) {
                    return "moved in the VIM's edition".to_string();
                }
            }
            let ours = text(beam, FAMILY_NAME_FIELD).zip(text(beam, TYPE_NAME_FIELD));
            if ours.as_ref() != Some(&(vim.families[element].clone(), vim.names[element].clone())) {
                return "another type in the VIM's edition".to_string();
            }
            let d = body.direction;
            let plan = d[0].hypot(d[1]);
            let u = [-d[1] / plan, d[0] / plan, 0.0];
            let w = [-d[2] * d[0] / plan, -d[2] * d[1] / plan, plan];
            let (cu, cw) = (dot(body.centre, u), dot(body.centre, w));
            let (mu0, mu1) = extent(mesh, u);
            let (mw0, mw1) = extent(mesh, w);
            if ((mu0 + mu1) / 2.0 - cu).abs() > MATCH_FEET
                || ((mw0 + mw1) / 2.0 - cw).abs() > MATCH_FEET
            {
                return "moved in the VIM's edition".to_string();
            }
            let section_points: Vec<(f64, f64)> = mesh
                .iter()
                .map(|&p| (dot(p, u) - cu, dot(p, w) - cw))
                .collect();
            let has = |a: f64, b: f64| {
                section_points
                    .iter()
                    .any(|&(x, y)| (x - a).abs() <= MATCH_FEET && (y - b).abs() <= MATCH_FEET)
            };
            let (hw, hd) = (section.width_feet / 2.0, section.depth_feet / 2.0);
            let tip = hd - section.flange_feet;
            let outer = [(-hw, -hd), (hw, -hd), (-hw, hd), (hw, hd)]
                .iter()
                .all(|&(a, b)| has(a, b));
            let tips = [(-hw, -tip), (hw, -tip), (-hw, tip), (hw, tip)]
                .iter()
                .all(|&(a, b)| has(a, b));
            let web = [
                (-section.web_feet / 2.0, 0.0),
                (section.web_feet / 2.0, 0.0),
            ]
            .iter()
            .all(|&(a, _)| {
                section_points
                    .iter()
                    .any(|&(x, y)| (x - a).abs() <= MATCH_FEET && y.abs() < tip)
            });
            match (outer, tips, web) {
                (true, true, true) => {
                    "mesh is the I: outer corners, flange tips and web faces".to_string()
                }
                (true, true, false) => {
                    "mesh has the corners and flange tips, no web vertex found".to_string()
                }
                (true, false, _) => {
                    "mesh has the outer corners but not the flange tips".to_string()
                }
                _ => "mesh lacks outer corners of the section".to_string(),
            }
        })();
        *scores.entry(verdict).or_default() += 1;
    }
    println!("against {} in {}:", title, vim_path.display());
    for (what, count) in &scores {
        println!("  {count:>5}  {what}");
    }
    Ok(())
}
