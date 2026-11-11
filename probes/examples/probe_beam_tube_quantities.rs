//! #1693, #1698: what the saved mesh of a beam drawn as a hollow tube gives of
//! the six base quantities Revit writes.
//!
//! Revit's export writes `Qto_BeamBaseQuantities`' `CrossSectionArea`,
//! `OuterSurfaceArea`, `GrossSurfaceArea`, `NetSurfaceArea`, `GrossVolume` and
//! `NetVolume` on each of the BIM4LCA office's 8 beams, which it draws as an
//! `IfcExtrudedAreaSolid` over an `IfcArbitraryProfileDefWithVoids`: a 200 mm
//! by 300 mm tube with a 4 mm wall, its outer corners rounded 8 mm and its
//! inner ones 4 mm. rvt-rs writes none of the six on any of them: the office's
//! 8 are drawn as their record's box (`partition_beam_axis`, #1698), and
//! `saved_meshes::beam_mesh_quantities` reads a beam's saved mesh as a prism
//! only where the mesh is one closed solid with no hole through it (RE-423),
//! while a tube's surface is two, an outer and an inner, so its Euler
//! characteristic is 0 where that rule's test asks for 2. RE-423 section 4
//! leaves the 8 without for that reason and says why the values are not
//! established.
//!
//! The probe prints two lines for every structural-framing element whose saved
//! graphics decode whole and whose element data holds a bounded line: what its
//! mesh gives along that line (whether the triangles close, in how many pieces,
//! the Euler characteristic, how many planes are square to the line and their
//! areas, the area of the faces along the line split by whether their outward
//! normal points away from the line, the outer surface, or towards it, the
//! inner one, the extent along the line and the closed volume), and the six
//! quantities those make, in square and cubic metres. A rule for a tube can be
//! written only where those are Revit's values on every beam it covers: pair a
//! line with Revit's export of the same beam by its ElementId, the IFC `Tag`,
//! and compare it with the six values the export writes there.
//!
//! Counts and values only: no name and no coordinate is printed.
//!
//! ```text
//! cargo run --profile ci -p rvt-probes --example probe_beam_tube_quantities -- model.rvt
//! ```

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

use rvt::RevitFile;
use rvt::research::native_document::Options;
use rvt::research::native_saved_scene::extract_at_detail;
use rvt::research::partition_beam_axes::scan_bounded_lines;
use rvt::research::partition_schema_mvp::recover_partition_schema_mvp;
use rvt::walker::{DecodedElement, WalkerLimits};

/// Points this close, feet, are one vertex.
const WELD_FEET: f64 = 1e-6;

/// How close, as one minus the cosine, two normals are one plane's.
const PLANE_COSINE: f64 = 1e-5;

/// How far apart, feet, two planes of one normal are one plane.
const PLANE_FEET: f64 = 7e-4;

/// A face's normal is along the line, which makes the face an end, where its
/// cosine with it is over this (0.06 degrees from square).
const PARALLEL_COSINE: f64 = 1e-3;

/// An end is square to the line where its normal's cosine with it is over this
/// (0.8 degrees from square).
const SQUARE_COSINE: f64 = 0.9999;

/// How far, as a fraction of the larger, the areas of the two ends may differ
/// and still be the two of one section (the tolerance `mesh_volume` reads them
/// with).
const END_AREA_TOLERANCE: f64 = 1e-4;

/// How far, as a fraction of the volume, the volume may be from the end area
/// times the extent and the solid still be that prism.
const PRISM_VOLUME_TOLERANCE: f64 = 1e-4;

/// A foot in metres, whose square and cube convert the values the model holds
/// to the ones Revit's export writes.
const FEET_METRES: f64 = 0.3048;

/// The saved mesh of one element, its triangles indexing its own vertices.
struct Mesh {
    vertices: Vec<[f64; 3]>,
    triangles: Vec<[u32; 3]>,
    partial: bool,
}

/// One plane of a mesh: its unit normal, its offset along it, and the area of
/// the triangles on it.
struct Plane {
    normal: [f64; 3],
    offset: f64,
    area: f64,
}

/// What a beam's mesh gives along the beam's line, feet, square and cubic feet.
struct Found {
    closed: bool,
    pieces: usize,
    euler: i64,
    two_vertices_less_faces: i64,
    planes: usize,
    ends: [f64; 2],
    ends_square: bool,
    lateral_out: f64,
    lateral_in: f64,
    extent: f64,
    volume: f64,
    wound_out: bool,
}

fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

/// The root of `vertex` in the union-find forest `parent`.
fn root(parent: &mut [u32], mut vertex: u32) -> u32 {
    while parent[vertex as usize] != vertex {
        let up = parent[parent[vertex as usize] as usize];
        parent[vertex as usize] = up;
        vertex = up;
    }
    vertex
}

/// Join the pieces holding `a` and `b`.
fn join(parent: &mut [u32], a: u32, b: u32) {
    let (a, b) = (root(parent, a), root(parent, b));
    if a != b {
        parent[b as usize] = a;
    }
}

/// What `mesh` is along the line through `start` in direction `axis`: none
/// where a vertex is not there or not finite, a triangle names no vertex, or no
/// triangle is left or they hold no volume.
fn read_mesh(mesh: &Mesh, start: [f64; 3], axis: [f64; 3]) -> Option<Found> {
    let mut welded: HashMap<[i64; 3], u32> = HashMap::new();
    let mut ids: Vec<u32> = Vec::with_capacity(mesh.vertices.len());
    for point in &mesh.vertices {
        if !point.iter().all(|coordinate| coordinate.is_finite()) {
            return None;
        }
        let key = (*point).map(|coordinate| (coordinate / WELD_FEET).round() as i64);
        let next = u32::try_from(welded.len()).ok()?;
        ids.push(*welded.entry(key).or_insert(next));
    }
    // The tetrahedra are taken from a vertex of the mesh rather than from the
    // origin, which the model's coordinates are far from.
    let origin = *mesh.vertices.first()?;
    let mut parent: Vec<u32> = (0..u32::try_from(welded.len()).ok()?).collect();
    let mut edges: HashMap<(u32, u32), u32> = HashMap::new();
    let mut used: HashSet<u32> = HashSet::new();
    let mut planes: Vec<Plane> = Vec::new();
    let mut lateral_out = 0.0;
    let mut lateral_in = 0.0;
    let mut low = f64::INFINITY;
    let mut high = f64::NEG_INFINITY;
    let mut sum = 0.0;
    let mut counted = 0usize;
    for triangle in &mesh.triangles {
        let (Some(&ia), Some(&ib), Some(&ic)) = (
            ids.get(triangle[0] as usize),
            ids.get(triangle[1] as usize),
            ids.get(triangle[2] as usize),
        ) else {
            return None;
        };
        if ia == ib || ib == ic || ic == ia {
            continue;
        }
        let (Some(&p), Some(&q), Some(&r)) = (
            mesh.vertices.get(triangle[0] as usize),
            mesh.vertices.get(triangle[1] as usize),
            mesh.vertices.get(triangle[2] as usize),
        ) else {
            return None;
        };
        used.extend([ia, ib, ic]);
        join(&mut parent, ia, ib);
        join(&mut parent, ib, ic);
        counted += 1;
        for edge in [(ia, ib), (ib, ic), (ic, ia)] {
            *edges.entry(edge).or_insert(0) += 1;
        }
        sum += dot(sub(p, origin), cross(sub(q, origin), sub(r, origin)));
        for point in [p, q, r] {
            let along = dot(sub(point, start), axis);
            low = low.min(along);
            high = high.max(along);
        }
        let twice = cross(sub(q, p), sub(r, p));
        let length = dot(twice, twice).sqrt();
        if !(length.is_finite() && length > 0.0) {
            continue;
        }
        let normal = twice.map(|component| component / length);
        let area = length / 2.0;
        if dot(normal, axis).abs() > PARALLEL_COSINE {
            let offset = dot(normal, p);
            let known = planes.iter().position(|plane| {
                let cosine = dot(normal, plane.normal);
                cosine.abs() > 1.0 - PLANE_COSINE
                    && (offset - cosine.signum() * plane.offset).abs() < PLANE_FEET
            });
            match known {
                Some(index) => planes[index].area += area,
                None => planes.push(Plane {
                    normal,
                    offset,
                    area,
                }),
            }
        } else if dot(normal, sub(p, start)) > 0.0 {
            lateral_out += area;
        } else {
            lateral_in += area;
        }
    }
    if edges.is_empty() {
        return None;
    }
    // A surface wound inwards has every normal the other way, so the faces
    // away from the line are the inner ones.
    let wound_out = sum > 0.0;
    let (lateral_out, lateral_in) = if wound_out {
        (lateral_out, lateral_in)
    } else {
        (lateral_in, lateral_out)
    };
    let volume = sum.abs() / 6.0;
    if !(volume.is_finite() && volume > 0.0) {
        return None;
    }
    let closed = edges
        .iter()
        .all(|(&(from, to), &seen)| seen == 1 && edges.get(&(to, from)) == Some(&1));
    let pieces = used
        .iter()
        .map(|vertex| root(&mut parent, *vertex))
        .collect::<HashSet<u32>>()
        .len();
    let undirected = edges.len() / 2;
    let euler = used.len() as i64 - undirected as i64 + counted as i64;
    let two_vertices_less_faces = 2 * used.len() as i64 - counted as i64;
    let ends_square = planes.len() == 2
        && planes
            .iter()
            .all(|plane| dot(plane.normal, axis).abs() > SQUARE_COSINE);
    let ends = [
        planes.first().map_or(0.0, |plane| plane.area),
        planes.get(1).map_or(0.0, |plane| plane.area),
    ];
    Some(Found {
        closed,
        pieces,
        euler,
        two_vertices_less_faces,
        planes: planes.len(),
        ends,
        ends_square,
        lateral_out,
        lateral_in,
        extent: high - low,
        volume,
        wound_out,
    })
}

fn main() -> rvt::Result<()> {
    let Some(path) = std::env::args().nth(1) else {
        eprintln!("usage: probe_beam_tube_quantities MODEL.rvt");
        return Ok(());
    };
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
    let mut meshes: BTreeMap<u64, Mesh> = BTreeMap::new();
    if !ids.is_empty() {
        let options = Options {
            selected_ids: ids.iter().map(|id| u64::from(*id)).collect(),
            max_graph_values: 1_000_000,
            max_graph_objects: 1_000_000,
            ..Default::default()
        };
        let scene = extract_at_detail(&mut rf, &options, 3)?;
        for element in &scene.elements {
            let primitives = &element.meshes.primitives;
            if primitives.is_empty() {
                continue;
            }
            let partial =
                !element.meshes.diagnostics.is_empty() || element.meshes.unbounded_faces > 0;
            let mut vertices: Vec<[f64; 3]> = Vec::new();
            let mut triangles: Vec<[u32; 3]> = Vec::new();
            for primitive in primitives {
                let Ok(base) = u32::try_from(vertices.len()) else {
                    break;
                };
                vertices.extend_from_slice(&primitive.vertices);
                for triangle in &primitive.triangles {
                    let (Some(a), Some(b), Some(c)) = (
                        base.checked_add(triangle[0]),
                        base.checked_add(triangle[1]),
                        base.checked_add(triangle[2]),
                    ) else {
                        continue;
                    };
                    triangles.push([a, b, c]);
                }
            }
            meshes.insert(
                element.identity.element_id,
                Mesh {
                    vertices,
                    triangles,
                    partial,
                },
            );
        }
    }
    let mut read = 0usize;
    let mut no_line = 0usize;
    let mut no_mesh = 0usize;
    let mut partial = 0usize;
    let mut unread = 0usize;
    for beam in &beams {
        let Some(id) = beam.id else {
            continue;
        };
        let Some(line) = lines.get(&id) else {
            no_line += 1;
            continue;
        };
        let Some(mesh) = meshes.get(&u64::from(id)) else {
            no_mesh += 1;
            continue;
        };
        if mesh.partial {
            partial += 1;
            continue;
        }
        let Some(found) = read_mesh(mesh, line.start(), line.direction) else {
            unread += 1;
            continue;
        };
        read += 1;
        let [a, b] = found.ends;
        let end_area = (a + b) / 2.0;
        let square_metres = FEET_METRES * FEET_METRES;
        let cubic_metres = square_metres * FEET_METRES;
        let same = (a - b).abs() <= END_AREA_TOLERANCE * a.max(b);
        let prism =
            (found.volume - end_area * found.extent).abs() <= PRISM_VOLUME_TOLERANCE * found.volume;
        println!(
            "tag {id} closed {} pieces {} euler {} two_vertices_less_faces {} planes {} ends {a:.9} {b:.9} ends_same {} ends_square {} extent {:.9} lateral_out {:.9} lateral_in {:.9} volume {:.9} prism {} wound_out {}",
            u8::from(found.closed),
            found.pieces,
            found.euler,
            found.two_vertices_less_faces,
            found.planes,
            u8::from(same),
            u8::from(found.ends_square),
            found.extent,
            found.lateral_out,
            found.lateral_in,
            found.volume,
            u8::from(prism),
            u8::from(found.wound_out),
        );
        println!(
            "tag {id} CrossSectionArea {:.9} OuterSurfaceArea {:.9} GrossSurfaceArea {:.9} NetSurfaceArea {:.9} GrossVolume {:.9} NetVolume {:.9}",
            end_area * square_metres,
            found.lateral_out * square_metres,
            (found.lateral_out + a + b) * square_metres,
            (found.lateral_out + found.lateral_in + a + b) * square_metres,
            found.volume * cubic_metres,
            found.volume * cubic_metres,
        );
    }
    println!(
        "beams {} meshed {} read {} no_line {} no_mesh {} partial {} unread {}",
        beams.len(),
        meshes.len(),
        read,
        no_line,
        no_mesh,
        partial,
        unread
    );
    Ok(())
}
