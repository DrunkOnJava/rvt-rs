//! RE-342 (#1158, #1190): whether the saved-graphics reader reads Revit 2015
//! to 2022 files, and what a Revit 2020 family that holds import geometry
//! only needs of it (STE1200, discussion #112).
//!
//! FACT under test: `native_saved_mesh` and `native_graphics_traversal` were
//! measured on Revit 2023 to 2027. They refuse a `GPolyMesh` whose topology is
//! not `FacetedTopology0` or whose `m_normalsFlag` is not 0, an
//! `InstanceInfo` whose `m_GRepId` is not 0 (Revit 2020's schema has no such
//! field), and a `GInstance` whose `m_bHasScale` is set; and
//! `native_index::creation_episodes` refuses a `Global/History` whose
//! `m_episodeList.m_oEpisodes` is not empty. Whether any of the four is met
//! in a file CI can fetch, and whether reading it changes a mesh, is what
//! this probe counts. Since RE-342 the readers read an `InstanceInfo` of
//! exactly `m_Trf`, `m_symbolId` and `m_cda`, a set `m_bHasScale` on such an
//! `InstanceInfo` with a rotation matrix, and, on this probe's research route
//! only, deferred history episodes (`Options::deferred_history_episodes`, set
//! below) and, in the graphics of a file of Revit 2015 to 2022, a
//! `FacetedTopology9` under binding 2 whose stored normals all follow the
//! winding (`native_saved_mesh::graphics_pre2023`, chosen by the file's
//! release). No export reader takes that route. The census says how many
//! objects each rule meets and, for the normals, how many it refuses; the
//! `history default reading` line says what every other reader answers the
//! same history.
//!
//! One file, or two (`OLDER.rvt NEWER.rvt`, the sample first and the same
//! project saved by a newer release second, as the Measure harness calls a
//! probe with `probe_pair_release`). A trailing `--records`, which the harness
//! adds for a sample, leaves the scene out of a one-file run: the pair run
//! reads it.
//!
//! One file:
//! - `release`, `kind` (project or family);
//! - `schema <class>`: the fields of `InstanceInfo`, `GInstance`,
//!   `GPolyMesh` and every `FacetedTopology*` class the file's own
//!   `Formats/Latest` declares;
//! - `history`: the number of deferred episode pointers in the root of
//!   `Global/History` (`m_oEpisodes`) and whether
//!   `native_index::creation_episodes` reads it;
//! - `census`: over every graph of every channel-103 record, the
//!   `FacetedTopology*` objects by class and `m_normalsFlag` (objects, points,
//!   facets, those with one normal per facet, those in a graph that also holds
//!   a `GImposter`, RE-300), for those with one normal per facet how many lie
//!   within 0.1 degree and 1 degree of the facet's winding normal and whether
//!   `native_saved_mesh::normals_follow_winding` keeps or refuses the object, the
//!   `InstanceInfo` objects by `m_GRepId` (absent, zero, nonzero) and by the
//!   class of their `m_Trf` (identity, rotation, mirror, uniform scale,
//!   orthogonal, general), and the `GInstance` objects by `m_bHasScale`, the
//!   `m_GRepId` and the matrix class of the `InstanceInfo` they name;
//! - `scene`, on files of Revit 2023 and earlier: the elements of class
//!   `FamilyInstance`, `ImportInstance` and `DirectShape`, meshed as
//!   `saved_meshes::attach` meshes a body
//!   (`native_saved_scene::extract_in_batches_with_control`, detail level 3,
//!   `Options::releases` widened to `NATIVE_RELEASES`): their status, the
//!   histogram of what stops the partial ones, and their triangles.
//!
//! Two files: the `scene` of each and, for every ElementId both read whole,
//! the triangle count, the world box (feet) and the surface area of the older
//! file's mesh against the newer file's.
//!
//! Output is `saved graphics pre2023:` lines of counts, schema class and field
//! names and normalised error text, in the restricted alphabet
//! `tools/ci/scrub-holdout-results.sh` keeps for a model whose per-element
//! results are not uploaded. No ElementId, name or coordinate is printed.
//!
//! ```text
//! cargo run --profile ci -p rvt-probes --example probe_re342_saved_graphics_pre2023 -- FILE.rvt
//! cargo run --profile ci -p rvt-probes --example probe_re342_saved_graphics_pre2023 -- OLDER.rvt NEWER.rvt
//! ```

use anyhow::{Result, bail};
use rvt::RevitFile;
use rvt::control::WalkerControl;
use rvt::research::native_document::{self, NATIVE_RELEASES, Options, extract_graphs};
use rvt::research::native_index;
use rvt::research::native_parameters::{self, GraphObject, ObjectGraph};
use rvt::research::native_saved_mesh::{normals_follow_winding, pointer};
use rvt::research::native_saved_scene::extract_in_batches_with_control;
use rvt::research::schema_registry::{self, Class, Field, Registry};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

/// The element classes a project's scene is meshed for: the ones that hold
/// `GInstance`s of shared symbols (families placed in the model) and the ones
/// that hold imported geometry.
const SCENE_CLASSES: [&str; 3] = ["FamilyInstance", "ImportInstance", "DirectShape"];

const TOLERANCE_FEET: f64 = 1e-4;

fn line(text: &str) {
    let scrubbed: String = text
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || " _:.,;()=#<>/+-".contains(c) {
                c
            } else {
                '_'
            }
        })
        .collect();
    println!("saved graphics pre2023: {scrubbed}");
}

/// An error message with its numbers and hex words folded, so that one cause
/// is one histogram row.
fn norm_msg(text: &str) -> String {
    let mut words = Vec::new();
    for word in text.split_whitespace() {
        let core = word.trim_matches(|c: char| !c.is_ascii_alphanumeric());
        let hex_like = core.len() >= 8
            && core.chars().all(|c| c.is_ascii_hexdigit() || c == '-')
            && core.chars().any(|c| c.is_ascii_digit());
        words.push(if hex_like { "<hex>" } else { word });
    }
    let joined = words.join(" ");
    let mut folded = String::new();
    let mut in_digits = false;
    for c in joined.chars() {
        if c.is_ascii_digit() {
            if !in_digits {
                folded.push('#');
            }
            in_digits = true;
        } else {
            in_digits = false;
            folded.push(c);
        }
    }
    folded.chars().take(110).collect()
}

#[derive(Default)]
struct Tally(BTreeMap<String, u64>);

impl Tally {
    fn add(&mut self, key: impl Into<String>, n: u64) {
        *self.0.entry(key.into()).or_insert(0) += n;
    }

    fn print(&self) {
        for (key, n) in &self.0 {
            line(&format!("{key} {n}"));
        }
    }

    fn print_with(&self, prefix: &str) {
        for (key, n) in &self.0 {
            line(&format!("{prefix} {key} {n}"));
        }
    }
}

fn describe_field(registry: &Registry, field: &Field) -> String {
    let mut text = format!(
        "{}:{}.{}.{}",
        field.name, field.base, field.modifier, field.uninterpreted_flags
    );
    for reference in &field.references {
        text.push('>');
        text.push_str(
            registry
                .class(reference.tag)
                .map_or("?", |c| c.name.as_str()),
        );
    }
    text
}

fn describe_class(registry: &Registry, class: &Class) -> String {
    let fields: Vec<String> = class
        .fields
        .iter()
        .map(|f| describe_field(registry, f))
        .collect();
    let parent = if class.parent_reference.tag >= 12 {
        registry
            .class(class.parent_reference.tag)
            .map_or("?", |c| c.name.as_str())
    } else {
        "none"
    };
    format!(
        "parent={parent} fields={}: {}",
        class.fields.len(),
        fields.join(",")
    )
}

fn schema_lines(registry: &Registry) {
    for class in &registry.classes {
        let wanted = matches!(
            class.name.as_str(),
            "InstanceInfo" | "InstInfoBase" | "GInstance" | "GPolyMesh"
        ) || class.name.starts_with("FacetedTopology");
        if wanted {
            line(&format!(
                "schema {} {}",
                class.name,
                describe_class(registry, class)
            ));
        }
    }
}

fn history(rf: &mut RevitFile, registry: &Registry, tally: &mut Tally) {
    let bytes = match native_document::read_single(rf, "Global/History") {
        Ok(bytes) => bytes,
        Err(error) => {
            tally.add(
                format!("history unreadable {}", norm_msg(&format!("{error:#}"))),
                1,
            );
            return;
        }
    };
    let tag = bytes
        .get(0..2)
        .and_then(|b| <[u8; 2]>::try_from(b).ok())
        .map(u16::from_le_bytes);
    if let Some(tag) = tag.filter(|t| {
        registry
            .class(*t)
            .is_some_and(|c| c.name == "DocumentHistory")
    }) {
        match native_parameters::decode_object_fields(&bytes, registry, 2, tag) {
            Ok(root) => {
                let pointers = root.fields["m_episodeList"]["m_oEpisodes"]
                    .as_array()
                    .map(Vec::len);
                match pointers {
                    Some(n) => tally.add(format!("history deferred episode pointers {n}"), 1),
                    None => tally.add("history deferred episode pointers absent", 1),
                }
            }
            Err(error) => tally.add(
                format!("history root {}", norm_msg(&format!("{error:#}"))),
                1,
            ),
        }
    }
    match native_index::creation_episodes_with_deferred(&bytes, registry) {
        Ok((episodes, _)) => tally.add(format!("history episodes read {}", episodes.len()), 1),
        Err(error) => tally.add(
            format!(
                "history episodes refused {}",
                norm_msg(&format!("{error:#}"))
            ),
            1,
        ),
    }
    // What every reader but this probe's route answers the same history.
    match native_index::creation_episodes(&bytes, registry) {
        Ok(episodes) => tally.add(
            format!("history default reading read {}", episodes.len()),
            1,
        ),
        Err(error) => tally.add(
            format!(
                "history default reading refused {}",
                norm_msg(&format!("{error:#}"))
            ),
            1,
        ),
    }
}

fn numbers<const N: usize>(value: &Value) -> Option<[f64; N]> {
    let items = value.as_array().filter(|a| a.len() == N)?;
    let mut out = [0.0; N];
    for (slot, item) in out.iter_mut().zip(items) {
        *slot = item.as_f64().filter(|x| x.is_finite())?;
    }
    Some(out)
}

fn rows<const N: usize>(value: &Value) -> Vec<[f64; N]> {
    value
        .as_array()
        .map(|a| a.iter().filter_map(numbers::<N>).collect())
        .unwrap_or_default()
}

/// The rows of `value` as the reader takes them (`point` in
/// `native_saved_mesh`): every one an array of `N` finite numbers, or `None`,
/// where `rows` drops the malformed ones.
fn strict_rows<const N: usize>(value: &Value) -> Option<Vec<[f64; N]>> {
    value.as_array()?.iter().map(numbers::<N>).collect()
}

/// The facets of `value` as the reader takes them: every one an array of three
/// unsigned integers that fit 32 bits and index one of `points` points, or
/// `None`, where `facets` drops the malformed ones and saturates the indices.
fn strict_facets(value: &Value, points: usize) -> Option<Vec<[u32; 3]>> {
    value
        .as_array()?
        .iter()
        .map(|facet| {
            let items = facet.as_array().filter(|a| a.len() == 3)?;
            let mut corners = [0u32; 3];
            for (slot, item) in corners.iter_mut().zip(items) {
                *slot = u32::try_from(item.as_u64()?).ok()?;
                if *slot as usize >= points {
                    return None;
                }
            }
            Some(corners)
        })
        .collect()
}

fn facets(value: &Value) -> Vec<[usize; 3]> {
    value
        .as_array()
        .map(|a| {
            a.iter()
                .filter_map(|facet| {
                    let t = numbers::<3>(facet)?;
                    Some([t[0] as usize, t[1] as usize, t[2] as usize])
                })
                .collect()
        })
        .unwrap_or_default()
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

fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn length(a: [f64; 3]) -> f64 {
    dot(a, a).sqrt()
}

/// One `FacetedTopology*` object: its sizes and, where it holds one normal per
/// facet, how far each stored normal is from the facet's winding normal.
fn topology(object: &GraphObject, imposter: bool, tally: &mut Tally) {
    let f = &object.fields;
    let flag = f["m_normalsFlag"]
        .as_u64()
        .map_or_else(|| "none".to_string(), |v| v.to_string());
    let key = format!("topology {} normals_flag {flag}", object.class_name);
    let points = rows::<3>(&f["m_pointsArr"]);
    let normals = rows::<3>(&f["m_normalsArr"]);
    let facets = facets(&f["m_facetsArr"]);
    tally.add(format!("{key} objects"), 1);
    tally.add(format!("{key} points"), points.len() as u64);
    tally.add(format!("{key} facets"), facets.len() as u64);
    if imposter {
        tally.add(format!("{key} objects in a graph with a GImposter"), 1);
    }
    if f.get("m_edgeVisFlagsArr").is_some() {
        let per_facet = f["m_edgeVisFlagsArr"]
            .as_array()
            .is_some_and(|a| a.len() == facets.len());
        tally.add(
            format!("{key} edge visibility flags one per facet {per_facet}"),
            1,
        );
    }
    // The winding rule is the reader's own (`normals_follow_winding`), counted
    // on every object under binding 2, whatever its normals, and on the arrays
    // as the reader takes them: one with no normals, with a count that is not
    // its facets', or with a row that is malformed, an index that is not an
    // unsigned 32-bit integer or is out of range, is refused as a whole.
    if flag == "2" {
        let kept = match (
            strict_rows::<3>(&f["m_pointsArr"]),
            strict_rows::<3>(&f["m_normalsArr"]),
        ) {
            (Some(points), Some(normals)) => strict_facets(&f["m_facetsArr"], points.len())
                .is_some_and(|facets| normals_follow_winding(&points, &facets, &normals)),
            _ => false,
        };
        let rule = if kept { "keeps" } else { "refuses" };
        tally.add(format!("{key} objects the winding rule {rule}"), 1);
    }
    if normals.is_empty() {
        return;
    }
    if normals.len() != facets.len() {
        tally.add(format!("{key} objects with other normal counts"), 1);
        return;
    }
    tally.add(format!("{key} objects with one normal per facet"), 1);
    let (mut fine, mut coarse, mut opposite, mut degenerate, mut stored_zero) =
        (0u64, 0u64, 0u64, 0u64, 0u64);
    for (tri, stored) in facets.iter().zip(&normals) {
        let (Some(&a), Some(&b), Some(&c)) =
            (points.get(tri[0]), points.get(tri[1]), points.get(tri[2]))
        else {
            degenerate += 1;
            continue;
        };
        let winding = cross(sub(b, a), sub(c, a));
        let (wl, sl) = (length(winding), length(*stored));
        if wl < 1e-12 {
            degenerate += 1;
        } else if sl < 1e-9 {
            stored_zero += 1;
        } else {
            let cosine = (dot(winding, *stored) / (wl * sl)).clamp(-1.0, 1.0);
            let degrees = cosine.acos().to_degrees();
            if degrees <= 0.1 {
                fine += 1;
            } else if degrees <= 1.0 {
                coarse += 1;
            } else {
                opposite += 1;
            }
        }
    }
    tally.add(format!("{key} facets within 0.1 degree of winding"), fine);
    tally.add(
        format!("{key} facets between 0.1 and 1 degree of winding"),
        coarse,
    );
    tally.add(
        format!("{key} facets more than 1 degree from winding"),
        opposite,
    );
    tally.add(format!("{key} facets degenerate"), degenerate);
    tally.add(
        format!("{key} facets with a zero stored normal"),
        stored_zero,
    );
}

fn grep_id(fields: &Value) -> &'static str {
    match fields.get("m_GRepId") {
        None => "absent",
        Some(v) => match v.as_u64() {
            Some(0) => "zero",
            Some(_) => "nonzero",
            None => "other",
        },
    }
}

/// The class of an `InstanceInfo.m_Trf`: how its 3x3 rows relate.
fn matrix_class(value: &Value) -> &'static str {
    let Some(rows) = value["m_3x3"].as_array().filter(|r| r.len() == 3) else {
        return "unreadable";
    };
    let mut m = [[0.0f64; 3]; 3];
    for (slot, row) in m.iter_mut().zip(rows) {
        let Some(row) = numbers::<3>(row) else {
            return "unreadable";
        };
        *slot = row;
    }
    let gram: [[f64; 3]; 3] = std::array::from_fn(|a| std::array::from_fn(|b| dot(m[a], m[b])));
    let off_diagonal = [gram[0][1], gram[0][2], gram[1][2]]
        .iter()
        .all(|x| x.abs() < 1e-6);
    if !off_diagonal {
        return "general";
    }
    let identity =
        (0..3).all(|a| (0..3).all(|b| (m[a][b] - f64::from(u8::from(a == b))).abs() < 1e-9));
    if identity {
        return "identity";
    }
    let unit = (0..3).all(|a| (gram[a][a] - 1.0).abs() < 1e-6);
    if unit {
        return if dot(cross(m[0], m[1]), m[2]) > 0.0 {
            "rotation"
        } else {
            "mirror"
        };
    }
    let equal = (gram[0][0] - gram[1][1]).abs() < 1e-6 && (gram[1][1] - gram[2][2]).abs() < 1e-6;
    if equal { "uniform scale" } else { "orthogonal" }
}

fn census_graph(graph: &ObjectGraph, tally: &mut Tally) {
    let imposter = graph.objects.iter().any(|o| o.class_name == "GImposter");
    for (index, object) in graph.objects.iter().enumerate() {
        let name = object.class_name.as_str();
        if name.starts_with("FacetedTopology") {
            topology(object, imposter, tally);
        } else if name == "GPolyMesh" {
            tally.add("polymesh objects", 1);
        } else if name == "InstanceInfo" {
            tally.add(
                format!(
                    "instance_info m_GRepId {} m_Trf {}",
                    grep_id(&object.fields),
                    matrix_class(&object.fields["m_Trf"])
                ),
                1,
            );
        } else if name == "GInstance" {
            let scale = match object.fields["m_bHasScale"].as_bool() {
                Some(true) => "true",
                Some(false) => "false",
                None => "absent",
            };
            let info = pointer(graph, index, &object.fields["m_instanceInfo"])
                .ok()
                .flatten()
                .and_then(|i| graph.objects.get(i))
                .filter(|o| o.class_name == "InstanceInfo");
            let (rep, matrix) = match info {
                Some(info) => (grep_id(&info.fields), matrix_class(&info.fields["m_Trf"])),
                None => ("unresolved", "unresolved"),
            };
            tally.add(
                format!("ginstance m_bHasScale {scale} m_GRepId {rep} matrix {matrix}"),
                1,
            );
        }
    }
}

fn census(rf: &mut RevitFile, tally: &mut Tally) {
    let options = Options {
        channels: BTreeSet::from([103]),
        releases: NATIVE_RELEASES,
        deferred_history_episodes: true,
        max_graph_values: 1_000_000,
        max_graph_objects: 1_000_000,
        ..Options::default()
    };
    let result = extract_graphs(rf, &options, |record| {
        tally.add("graphics records", 1);
        match &record.graph {
            Some(graph) => {
                tally.add("graphics records with a graph", 1);
                census_graph(graph, tally);
            }
            None => tally.add(
                format!(
                    "graphics records without a graph {}",
                    norm_msg(record.diagnostic.as_deref().unwrap_or("no diagnostic"))
                ),
                1,
            ),
        }
        Ok(())
    });
    match result {
        Ok(summary) => {
            tally.add("census indexed elements", summary.indexed_elements as u64);
            for (class, n) in &summary.refused_graph_classes {
                tally.add(format!("census refused graph class {class}"), *n as u64);
            }
        }
        Err(error) => tally.add(
            format!("census read refused {}", norm_msg(&format!("{error:#}"))),
            1,
        ),
    }
}

/// What a pair compares of an element's mesh.
struct Mesh {
    whole: bool,
    triangles: usize,
    low: [f64; 3],
    high: [f64; 3],
    area: f64,
}

fn diagnostic_kind(diagnostic: &str) -> String {
    let kind = diagnostic
        .strip_prefix("object ")
        .and_then(|rest| rest.split_once(": "))
        .map_or(diagnostic, |(_, kind)| kind);
    norm_msg(kind)
}

/// The ElementIds of the elements of `SCENE_CLASSES`. A channel-103 record's
/// body opens with its graphics class, not the element's, so the class is
/// read from the channel-102 record, without decoding its graph.
fn scene_ids(rf: &mut RevitFile, tally: &mut Tally) -> BTreeSet<u64> {
    let options = Options {
        releases: NATIVE_RELEASES,
        deferred_history_episodes: true,
        graph_classes: Some(BTreeSet::new()),
        ..Options::default()
    };
    let mut ids = BTreeSet::new();
    let result = extract_graphs(rf, &options, |record| {
        if let Some(class) = record
            .class_name
            .as_deref()
            .filter(|c| SCENE_CLASSES.contains(c))
        {
            tally.add(format!("scene elements of class {class}"), 1);
            ids.insert(record.identity.element_id);
        }
        Ok(())
    });
    if let Err(error) = result {
        tally.add(
            format!("scene classes refused {}", norm_msg(&format!("{error:#}"))),
            1,
        );
    }
    ids
}

fn scene(rf: &mut RevitFile, tally: &mut Tally) -> BTreeMap<u64, Mesh> {
    let ids = scene_ids(rf, tally);
    let mut meshes = BTreeMap::new();
    if ids.is_empty() {
        tally.add("scene has no element of the classes", 1);
        return meshes;
    }
    let options = Options {
        selected_ids: ids,
        releases: NATIVE_RELEASES,
        deferred_history_episodes: true,
        max_graph_values: 1_000_000,
        max_graph_objects: 1_000_000,
        ..Options::default()
    };
    let result =
        extract_in_batches_with_control(rf, &options, 3, &WalkerControl::default(), |element| {
            tally.add("scene owners", 1);
            tally.add(format!("scene status {}", element.status), 1);
            let primitives = &element.meshes.primitives;
            if primitives.is_empty() {
                tally.add("scene owners with no primitive", 1);
            }
            let kinds: BTreeSet<String> = element
                .meshes
                .diagnostics
                .iter()
                .map(|d| diagnostic_kind(d))
                .collect();
            for kind in kinds {
                tally.add(format!("scene owners stopped by {kind}"), 1);
            }
            let whole = element.status == "decoded_saved_graphics" && !primitives.is_empty();
            let mut mesh = Mesh {
                whole,
                triangles: 0,
                low: [f64::MAX; 3],
                high: [f64::MIN; 3],
                area: 0.0,
            };
            for primitive in primitives {
                mesh.triangles += primitive.triangles.len();
                for v in &primitive.vertices {
                    for ((low, high), x) in mesh.low.iter_mut().zip(mesh.high.iter_mut()).zip(v) {
                        *low = low.min(*x);
                        *high = high.max(*x);
                    }
                }
                for t in &primitive.triangles {
                    let corner = |i: u32| primitive.vertices.get(i as usize).copied();
                    if let (Some(a), Some(b), Some(c)) = (corner(t[0]), corner(t[1]), corner(t[2]))
                    {
                        mesh.area += 0.5 * length(cross(sub(b, a), sub(c, a)));
                    }
                }
            }
            if whole {
                tally.add("scene owners drawn whole", 1);
                tally.add("scene triangles of whole owners", mesh.triangles as u64);
            }
            meshes.insert(element.id, mesh);
            Ok(())
        });
    if let Err(error) = result {
        tally.add(
            format!("scene read refused {}", norm_msg(&format!("{error:#}"))),
            1,
        );
    }
    meshes
}

fn bucket(value: f64) -> &'static str {
    if value == 0.0 {
        "exactly 0"
    } else if value <= 1e-6 {
        "<=1e-6"
    } else if value <= 1e-4 {
        "<=1e-4"
    } else if value <= 1e-2 {
        "<=1e-2"
    } else if value <= 1.0 {
        "<=1"
    } else {
        ">1"
    }
}

fn compare(older: &BTreeMap<u64, Mesh>, newer: &BTreeMap<u64, Mesh>, tally: &mut Tally) {
    for (id, a) in older {
        let Some(b) = newer.get(id) else {
            tally.add("pair elements only in the older file", 1);
            continue;
        };
        match (a.whole, b.whole) {
            (true, true) => {
                tally.add("pair elements whole in both", 1);
                let same_triangles = a.triangles == b.triangles;
                tally.add(
                    format!("pair whole in both: same triangle count {same_triangles}"),
                    1,
                );
                let box_difference = (0..3)
                    .map(|k| {
                        (a.low[k] - b.low[k])
                            .abs()
                            .max((a.high[k] - b.high[k]).abs())
                    })
                    .fold(0.0, f64::max);
                let area_difference = (a.area - b.area).abs() / b.area.abs().max(1e-12);
                tally.add(
                    format!(
                        "pair whole in both: world box differs by {} ft",
                        bucket(box_difference)
                    ),
                    1,
                );
                tally.add(
                    format!(
                        "pair whole in both: surface area differs by {} of the newer",
                        bucket(area_difference)
                    ),
                    1,
                );
                let equal = same_triangles
                    && box_difference <= TOLERANCE_FEET
                    && area_difference <= TOLERANCE_FEET;
                tally.add(
                    format!(
                        "pair whole in both: equal (same triangles, box and area within {TOLERANCE_FEET})  {equal}"
                    ),
                    1,
                );
            }
            (true, false) => tally.add("pair elements whole in the older file only", 1),
            (false, true) => tally.add("pair elements whole in the newer file only", 1),
            (false, false) => tally.add("pair elements whole in neither", 1),
        }
    }
    let only_newer = newer.keys().filter(|id| !older.contains_key(id)).count();
    tally.add("pair elements only in the newer file", only_newer as u64);
}

fn open(path: &str) -> Result<RevitFile> {
    Ok(RevitFile::open_with_limits(
        path,
        rvt::cli::open_limits(None, None, true),
    )?)
}

fn header(rf: &mut RevitFile, path: &str) -> Result<u32> {
    let saved = rf.basic_file_info()?.version;
    let layout = rf.layout_release()?;
    let family = path.to_ascii_lowercase().ends_with(".rfa");
    line(&format!(
        "release saved {saved} layout {layout} kind {}",
        if family { "family" } else { "project" }
    ));
    Ok(layout)
}

fn single(path: &str, census_only: bool) -> Result<()> {
    let mut rf = open(path)?;
    let release = header(&mut rf, path)?;
    let mut tally = Tally::default();
    match native_document::read_single(&mut rf, "Formats/Latest")
        .and_then(|bytes| schema_registry::parse(&bytes))
    {
        Ok(registry) => {
            schema_lines(&registry);
            history(&mut rf, &registry, &mut tally);
        }
        Err(error) => tally.add(
            format!("schema unreadable {}", norm_msg(&format!("{error:#}"))),
            1,
        ),
    }
    census(&mut rf, &mut tally);
    tally.print();
    let mut scene_tally = Tally::default();
    if census_only {
        scene_tally.add("scene not run: --records", 1);
    } else if release <= 2023 {
        scene(&mut rf, &mut scene_tally);
    } else {
        scene_tally.add(
            "scene not run: the release is read by the default export",
            1,
        );
    }
    scene_tally.print();
    Ok(())
}

fn pair(older_path: &str, newer_path: &str) -> Result<()> {
    let mut older_file = open(older_path)?;
    let mut newer_file = open(newer_path)?;
    header(&mut older_file, older_path)?;
    header(&mut newer_file, newer_path)?;
    let mut older_tally = Tally::default();
    let older = scene(&mut older_file, &mut older_tally);
    older_tally.print_with("older");
    let mut newer_tally = Tally::default();
    let newer = scene(&mut newer_file, &mut newer_tally);
    newer_tally.print_with("newer");
    let mut tally = Tally::default();
    compare(&older, &newer, &mut tally);
    tally.print();
    Ok(())
}

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let census_only = args.iter().any(|a| a == "--records");
    let files: Vec<&str> = args
        .iter()
        .map(String::as_str)
        .filter(|a| !a.starts_with("--"))
        .collect();
    match files.as_slice() {
        [one] => single(one, census_only),
        [older, newer] => pair(older, newer),
        _ => bail!("usage: probe_re342_saved_graphics_pre2023 FILE.rvt | OLDER.rvt NEWER.rvt"),
    }
}
