//! RE-104: a steel column's I section runs its flanges along the column's
//! own X axis.
//!
//! FACT: on Autodesk's Snowdon Towers 2024 structural sample, a structural
//! column whose type stores an I section (RE-103) has, in Revit's own mesh,
//! the section's four outer corners and four inner flange tips with the
//! flange width along the X axis of the column's instance transform (RE-87)
//! and the depth along its Y axis, centred on the column's record box.
//!
//! The probe scores every column rvt-rs draws as its I section (the
//! section, turned to that axis, fills the record box in plan within 0.01
//! ft) against the VIM export of the sample's 2027 edition, where the VIM
//! holds it as the 2024 file does: same family and type, centred on the
//! same place.
//!
//! Usage:
//!   cargo run --profile ci --example probe_re104_column_sections -- \
//!     MODEL.rvt MODEL.vim "Document Title"

use rvt::RevitFile;
use rvt::partition_schema_mvp::{
    COLUMN_I_AXIS_FIELDS, FAMILY_NAME_FIELD, TYPE_NAME_FIELD, beam_i_section_from_fields,
    element_record_bbox, recover_partition_schema_mvp,
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
    let path = PathBuf::from(args.next().expect("usage: MODEL.rvt MODEL.vim TITLE"));
    let vim_path = PathBuf::from(args.next().expect("MODEL.vim"));
    let title = args.next().expect("TITLE");
    let mut rf = RevitFile::open(&path)?;
    let version = rf.basic_file_info()?.version;
    let mvp = recover_partition_schema_mvp(&mut rf, version, WalkerLimits::default())?;
    let columns: Vec<&DecodedElement> = mvp
        .products
        .iter()
        .filter(|element| element.class == "StructuralColumn" && element.id.is_some())
        .collect();
    let float = |element: &DecodedElement, wanted: &str| {
        element.fields.iter().find_map(|(name, value)| match value {
            InstanceField::Float { value, .. } if name == wanted => Some(*value),
            _ => None,
        })
    };
    let wanted: BTreeSet<i64> = columns.iter().filter_map(|c| c.id.map(i64::from)).collect();
    let vim = Vim::open(&vim_path, &wanted).expect("readable VIM");
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
    for column in &columns {
        let name = text(column, TYPE_NAME_FIELD).unwrap_or_default();
        let key = (|| {
            let Some(section) = beam_i_section_from_fields(&column.fields) else {
                return "type is not an I section".to_string();
            };
            let (Some(ax), Some(ay)) = (
                float(column, COLUMN_I_AXIS_FIELDS[0]),
                float(column, COLUMN_I_AXIS_FIELDS[1]),
            ) else {
                return "I section, no upright transform".to_string();
            };
            let Some(bbox) = element_record_bbox(column) else {
                return "no record box".to_string();
            };
            let length = ax.hypot(ay);
            let (ux, uy) = (ax / length, ay / length);
            let (w, d) = (section.width_feet, section.depth_feet);
            let (dx, dy) = (bbox[3] - bbox[0], bbox[4] - bbox[1]);
            if (w * ux.abs() + d * uy.abs() - dx).abs() > MATCH_FEET
                || (w * uy.abs() + d * ux.abs() - dy).abs() > MATCH_FEET
            {
                return "I section, does not fill its record box".to_string();
            }
            let Some(&element) = column.id.and_then(|id| by_id.get(&i64::from(id))) else {
                return "drawn; not in the VIM".to_string();
            };
            let Some(mesh) = vim.points.get(&element).filter(|p| !p.is_empty()) else {
                return "drawn; no VIM mesh".to_string();
            };
            let ours = text(column, FAMILY_NAME_FIELD).zip(text(column, TYPE_NAME_FIELD));
            if ours.as_ref() != Some(&(vim.families[element].clone(), vim.names[element].clone())) {
                return "drawn; another type in the VIM's edition".to_string();
            }
            let (cx, cy) = ((bbox[0] + bbox[3]) / 2.0, (bbox[1] + bbox[4]) / 2.0);
            let location = vim.locations[element];
            let (mx0, mx1) = extent(mesh, [1.0, 0.0, 0.0]);
            let (my0, my1) = extent(mesh, [0.0, 1.0, 0.0]);
            if ((mx0 + mx1) / 2.0 - cx).abs() > MATCH_FEET
                || ((my0 + my1) / 2.0 - cy).abs() > MATCH_FEET
                || (location[0] - cx).abs() > MATCH_FEET
                || (location[1] - cy).abs() > MATCH_FEET
            {
                return "drawn; moved in the VIM's edition".to_string();
            }
            let local: Vec<(f64, f64)> = mesh
                .iter()
                .map(|p| {
                    let (x, y) = (p[0] - cx, p[1] - cy);
                    (x * ux + y * uy, -x * uy + y * ux)
                })
                .collect();
            let has = |a: f64, b: f64| {
                local
                    .iter()
                    .any(|&(x, y)| (x - a).abs() <= MATCH_FEET && (y - b).abs() <= MATCH_FEET)
            };
            let (hw, hd, tip) = (w / 2.0, d / 2.0, d / 2.0 - section.flange_feet);
            let signs = [(-1.0, -1.0), (1.0, -1.0), (-1.0, 1.0), (1.0, 1.0)];
            let the_i = signs
                .iter()
                .all(|&(s, t)| has(s * hw, t * hd) && has(s * hw, t * tip));
            let turned = ux.abs() > 1e-6 && uy.abs() > 1e-6;
            format!(
                "drawn; the mesh is {}the I{}",
                if the_i { "" } else { "not " },
                if turned { " (turned)" } else { "" }
            )
        })();
        *scores.entry(key.clone()).or_default() += 1;
        *scores.entry(format!("  {key}: {name}")).or_default() += 1;
    }
    println!("release {version}: {} structural columns", columns.len());
    for (what, count) in &scores {
        println!("  {count:>5}  {what}");
    }
    Ok(())
}
