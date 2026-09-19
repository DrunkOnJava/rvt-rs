//! Rust-only native geometry extraction for a conservatively identified 2027
//! schema profile. Unknown schema, owner, topology, or geometry is diagnosed.
//! This does not claim complete document geometry or enable the legacy walker.
mod cuts;
mod glb;
mod index;
pub(crate) mod layered_prism;
pub(crate) mod mesh;
use crate::native_segments;
use crate::{
    RevitFile, compression,
    geometry::native_2027::{self, WallCurveKind},
    native_index,
};
use anyhow::{Result, ensure};
use index::{u32_at, u64_at};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Mesh {
    pub vertices: Vec<[f64; 3]>,
    pub triangles: Vec<[u32; 3]>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Source {
    pub stream: String,
    /// Offset of the first SegmentMarker in the checksum-prepared stream.
    pub group_marker_offset: usize,
    /// Offset of the record in the reassembled, inflated continuation group.
    pub group_record_offset: usize,
    pub group_segment_count: usize,
    pub body_sha256: String,
    pub stored_revision: u32,
    pub other_revision: u32,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Element {
    pub id: u64,
    pub identity: native_index::Identity,
    pub class: String,
    pub status: String,
    pub diagnostics: Vec<String>,
    pub source: Source,
    pub curves: Vec<serde_json::Value>,
    pub geometry: Option<serde_json::Value>,
    pub meshes: Vec<Mesh>,
}
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Coverage {
    pub indexed_rows: usize,
    pub recognized_native_class_records: usize,
    pub recognized_native_owners: usize,
    pub recognized_geometry_owners: usize,
    pub mesh_owners: usize,
    pub recognized_geometry_owners_without_mesh: usize,
    pub indexed_rows_not_recognized: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Scene {
    pub schema_version: u32,
    pub revit_version: u32,
    pub units: String,
    pub status: String,
    pub complete_document_geometry: bool,
    pub schema_sha256: Option<String>,
    pub source_sha256: Option<String>,
    pub diagnostics: Vec<String>,
    pub indexed_element_count: usize,
    pub coverage: Coverage,
    pub index_diagnostic: Option<serde_json::Value>,
    pub elements: Vec<Element>,
}
impl Scene {
    pub fn to_glb(&self) -> Result<Vec<u8>> {
        glb::encode(self)
    }
}
#[derive(Clone)]
struct Record {
    stream: String,
    group_marker_offset: usize,
    group_segment_count: usize,
    outer: usize,
    bytes: Vec<u8>,
}
fn body(record: &Record) -> &[u8] {
    &record.bytes
}

/// Validate every record in a main-content continuation group, including
/// unselected channels and classes. Record offsets are group-local, never
/// offsets in one gzip member when continuation segments are present.
fn group_records(
    source: &native_segments::GroupSource,
    bytes: &[u8],
) -> Result<Vec<(u64, usize, usize, usize)>> {
    let header = match source.channel {
        101 => 12,
        102 | 103 => 16,
        _ => anyhow::bail!("unsupported native scene record channel {}", source.channel),
    };
    let mut pos = 0usize;
    let mut records = Vec::new();
    let mut body_bytes = 0u64;
    while pos < bytes.len() {
        ensure!(
            bytes.len() - pos >= header + 4,
            "truncated native scene record header"
        );
        let id = u64_at(bytes, pos)?;
        let length = u32_at(bytes, pos + header - 4)? as usize;
        let start = pos + header;
        let end = start
            .checked_add(length)
            .ok_or_else(|| anyhow::anyhow!("native scene record length overflow"))?;
        ensure!(
            end <= bytes.len().saturating_sub(4) && u32_at(bytes, end)? as usize == length,
            "native scene record dual lengths disagree"
        );
        records.push((id, pos, start, end));
        body_bytes += length as u64;
        pos = end + 4;
    }
    ensure!(
        records.len() as u64 == source.declared_objects && body_bytes == source.declared_body_bytes,
        "native scene group record counts/body sizes disagree"
    );
    Ok(records)
}
fn hash(b: &[u8]) -> String {
    format!("{:x}", Sha256::digest(b))
}
fn curve_value(c: &native_2027::NativeCurve) -> serde_json::Value {
    if let Some(radius) = c.radius {
        serde_json::json!({"kind":"arc","center":c.origin,"basis_x":c.basis_x,"basis_y":c.basis_y.unwrap(),"radius":radius,"start_angle":c.parameters[0],"end_angle":c.parameters[1]})
    } else {
        let start: [f64; 3] = std::array::from_fn(|j| c.origin[j] + c.parameters[0] * c.basis_x[j]);
        let end: [f64; 3] = std::array::from_fn(|j| c.origin[j] + c.parameters[1] * c.basis_x[j]);
        serde_json::json!({"kind":"line","start":start,"end":end})
    }
}
fn markers(body: &[u8]) -> Vec<usize> {
    body.windows(4)
        .enumerate()
        .filter_map(|(i, b)| (b == [4, 0, 8, 1]).then_some(i))
        .collect()
}
fn line(body: &[u8], offset: usize) -> Result<([f64; 3], [f64; 3], serde_json::Value)> {
    let bytes = body
        .get(offset..offset + 68)
        .ok_or_else(|| anyhow::anyhow!("truncated line"))?;
    let c = native_2027::decode_line_curve_span(bytes, 2027).map_err(anyhow::Error::msg)?;
    let start = std::array::from_fn(|j| c.origin[j] + c.parameters[0] * c.basis_x[j]);
    let end = std::array::from_fn(|j| c.origin[j] + c.parameters[1] * c.basis_x[j]);
    Ok((start, end, curve_value(&c)))
}

/// Extract a supported subset directly from a native RVT. No API snapshots,
/// Python subprocesses, label matching, or external converters are inputs.
/// Unsupported file profiles return a scene with diagnostics and no meshes.
pub fn extract(rf: &mut RevitFile) -> Result<Scene> {
    let version = rf.basic_file_info()?.version;
    let mut scene=Scene{schema_version:1,revit_version:version,units:"feet".into(),status:"partial_native_geometry".into(),complete_document_geometry:false,schema_sha256:None,source_sha256:None,index_diagnostic:None,diagnostics:vec!["Qualified wall surfaces, unioned rectangular wall cuts and horizontal polygonal floors only; other classes and unsupported features are not complete document geometry".into()],indexed_element_count:0,coverage:Coverage{indexed_rows:0,recognized_native_class_records:0,recognized_native_owners:0,recognized_geometry_owners:0,mesh_owners:0,recognized_geometry_owners_without_mesh:0,indexed_rows_not_recognized:0},elements:Vec::new()};
    if version != 2027 {
        scene.status = "unsupported_version".into();
        scene.diagnostics.push(format!(
            "Native scene decoder has no validated Revit {version} profile"
        ));
        return Ok(scene);
    }
    let schema = crate::native_document::read_single(rf, "Formats/Latest")?;
    let schema_hash = hash(&schema);
    scene.schema_sha256 = Some(schema_hash.clone());
    let registry = match crate::schema_registry::parse(&schema) {
        Ok(registry) => registry,
        Err(error) => {
            scene.status = "unsupported_schema_framing".into();
            scene.diagnostics.push(error.to_string());
            return Ok(scene);
        }
    };
    // The bundled profile supplies the semantic class vocabulary used by this
    // projection; it is version-scoped and does not expose source provenance.
    #[derive(Deserialize)]
    struct ClassProfile {
        name: String,
        tag: u16,
    }
    #[derive(Deserialize)]
    struct RegistryProfile {
        version: u32,
        classes: Vec<ClassProfile>,
    }
    let profile: RegistryProfile =
        serde_json::from_str(include_str!("registry-profile-2027.json"))?;
    if version != profile.version {
        scene.status = "unsupported_schema_profile".into();
        scene
            .diagnostics
            .push("document version differs from the supported named registry profile".into());
        return Ok(scene);
    }
    let classes: BTreeMap<u16, &str> = profile
        .classes
        .iter()
        .map(|c| {
            let definition = registry.named(&c.name).ok_or_else(|| {
                anyhow::anyhow!("geometry class absent from structural registry: {}", c.name)
            })?;
            ensure!(
                definition.tag == c.tag,
                "geometry profile/structural registry disagree for {}",
                c.name
            );
            Ok((definition.tag, c.name.as_str()))
        })
        .collect::<Result<_>>()?;
    let index_bytes = crate::native_document::read_single(rf, "Global/ElemTable")?;
    let episodes = native_index::creation_episodes(
        &crate::native_document::read_single(rf, "Global/History")?,
        &registry,
    )?;
    let entries = match native_index::parse(&index_bytes, &registry, &episodes) {
        Ok(index) => index.identities,
        Err(error) => {
            scene.status = "unsupported_index_layout".into();
            let declared_rows = index::u32_at(&index_bytes, 2).ok();
            scene.indexed_element_count = declared_rows.unwrap_or(0) as usize;
            scene.coverage.indexed_rows = scene.indexed_element_count;
            scene.coverage.indexed_rows_not_recognized = scene.indexed_element_count;
            scene.index_diagnostic = Some(
                serde_json::json!({"stream":"Global/ElemTable","inflated_bytes":index_bytes.len(),"inflated_sha256":hash(&index_bytes),"declared_rows":declared_rows,"reason":error.to_string()}),
            );
            scene.diagnostics.push(format!(
                "Element-index layout is not validated for native geometry: {error}"
            ));
            return Ok(scene);
        }
    };
    scene.indexed_element_count = entries.len();
    let increments = native_index::storage_increments(
        &crate::native_document::read_single(rf, "Global/DocumentIncrementTable")?,
        &registry,
    )?;
    let mut names: Vec<_> = rf
        .stream_names()
        .iter()
        .filter(|n| n.starts_with("Partitions/"))
        .cloned()
        .collect();
    names.sort();
    let present: BTreeSet<u32> = names
        .iter()
        .map(|name| name[11..].parse::<u32>())
        .collect::<std::result::Result<_, _>>()?;
    let mut records = BTreeMap::<(u64, String), Vec<Record>>::new();
    let mut skipped_embedded_groups = 0usize;
    let mut opaque_terminal_bytes = 0usize;
    for name in names {
        let partition = name[11..].parse::<u32>()?;
        let raw = rf.read_stream_with_limit(&name, 512 * 1024 * 1024)?;
        let prepared = compression::prepare_stream_for_inflate(&name, &raw);
        let statistics =
            native_segments::walk(&prepared, &registry, 256 * 1024 * 1024, |source, bytes| {
                if source.content_key.is_some() {
                    skipped_embedded_groups += 1;
                    return Ok(());
                }
                let frames = group_records(source, bytes)?;
                if source.channel != 102 {
                    return Ok(());
                }
                for (id, outer, start, end) in frames {
                    let Some(entry) = entries.get(&id) else {
                        continue;
                    };
                    if native_index::route_episode(entry.stored_revision, &increments, &present)?
                        != partition
                    {
                        continue;
                    }
                    let record_body = &bytes[start..end];
                    let Some(tag_bytes) = record_body.get(..2) else {
                        continue;
                    };
                    let tag = u16::from_le_bytes([tag_bytes[0], tag_bytes[1]]);
                    let Some(&class) = classes.get(&tag) else {
                        continue;
                    };
                    // Optional owning pointers include class tags only when present;
                    // their variable widths move the root identity field. Decode it
                    // from the schema rather than a fixture-specific byte offset.
                    ensure!(
                        u64::try_from(crate::native_metadata::identifier(
                            &crate::native_parameters::decode_object_fields(
                                record_body,
                                &registry,
                                2,
                                tag
                            )?
                            .fields["m_id"]
                        )?)? == id,
                        "native scene supported class repeated identity disagrees for {id}"
                    );
                    records.entry((id, class.into())).or_default().push(Record {
                        stream: name.clone(),
                        group_marker_offset: source.first_marker_offset,
                        group_segment_count: source.segment_count,
                        outer,
                        bytes: record_body.to_vec(),
                    });
                }
                Ok(())
            })?;
        opaque_terminal_bytes += statistics.opaque_terminal_bytes;
    }
    if skipped_embedded_groups > 0 {
        scene.diagnostics.push(format!("Skipped {skipped_embedded_groups} embedded-content groups; embedded family geometry is not exported"));
    }
    if opaque_terminal_bytes > 0 {
        scene.diagnostics.push(format!("Physical partition framing completed with {opaque_terminal_bytes} uninterpreted terminal bytes"));
    }
    let mut unique = BTreeMap::new();
    let mut ambiguous_host_geometry = false;
    for (key, list) in records {
        if list.len() != 1 {
            if ["Opening", "SWallRectOpening", "SWall", "ArcWall"].contains(&key.1.as_str()) {
                ambiguous_host_geometry = true;
            }
            scene.diagnostics.push(format!(
                "Ambiguous current {} record for {}; geometry withheld",
                key.1, key.0
            ));
            continue;
        }
        unique.insert(key, list.into_iter().next().unwrap());
    }
    let mut openings = BTreeMap::<u64, Vec<(u64, u64)>>::new();
    let mut unknown_opening = ambiguous_host_geometry;
    for ((id, class), r) in &unique {
        if class == "SWallRectOpening" {
            let b = body(r);
            if b.len() < 127 || u32_at(b, 123)? != 1 {
                unknown_opening = true;
                scene.diagnostics.push(format!(
                    "Unsupported rectangular opening {id}; host geometry withheld"
                ));
                continue;
            }
            openings.entry(u64_at(b, 115)?).or_default().push((*id, 0));
        } else if class == "Opening" {
            let b = body(r);
            if b.len() != 132 {
                unknown_opening = true;
                scene.diagnostics.push(format!(
                    "Unsupported Opening record {id}; host geometry withheld"
                ));
                continue;
            }
            openings
                .entry(u64_at(b, 105)?)
                .or_default()
                .push((*id, u64_at(b, 121)?));
        }
    }
    let rectangular_cuts = cuts::read(
        rf,
        unique
            .keys()
            .filter(|(_, class)| class == "SWallRectOpening")
            .map(|(id, _)| *id)
            .collect(),
    );
    let cut_host_proof = rectangular_cuts
        .as_ref()
        .map_err(|e| anyhow::anyhow!("cut records: {e:#}"))
        .and_then(|cuts| {
            let mut expected: BTreeMap<u64, BTreeSet<u64>> = unique
                .keys()
                .filter(|(_, class)| class == "SWall")
                .map(|(id, _)| (*id, BTreeSet::new()))
                .collect();
            for c in cuts.values() {
                expected.entry(c.host_id).or_default().insert(c.element_id);
            }
            cuts::validate_hosts(rf, &expected)
        });
    let wall_ids: BTreeSet<_> = unique
        .keys()
        .filter_map(|(id, class)| {
            ["SWall", "ArcWall"]
                .contains(&class.as_str())
                .then_some(*id)
        })
        .collect();
    let mut joined = BTreeSet::new();
    let evaluated_joins = crate::native_wall_joins::read(rf, wall_ids.clone());
    for ((id, class), r) in &unique {
        if !["SWall", "ArcWall"].contains(&class.as_str()) {
            continue;
        }
        let b = body(r);
        // Measured bounded topology reference: repeated peer identity, fixed
        // link discriminator, and a counted list of face references. This
        // establishes an unresolved join relationship, never a geometric trim.
        for p in 0..b.len().saturating_sub(27) {
            if b[p + 8..p + 15] != [1, 1, 0, 1, 0, 0, 0] || b[p..p + 8] != b[p + 15..p + 23] {
                continue;
            }
            let peer = u64_at(b, p)?;
            let n = u32_at(b, p + 23)? as usize;
            if peer != *id
                && wall_ids.contains(&peer)
                && n > 0
                && n <= b.len().saturating_sub(p + 27) / 4
            {
                joined.insert(*id);
                joined.insert(peer);
            }
        }
    }
    for ((id, class), r) in &unique {
        if !["SWall", "ArcWall", "Floor"].contains(&class.as_str()) {
            continue;
        }
        let e = &entries[id];
        let mut element = Element {
            id: *id,
            identity: e.clone(),
            class: if class == "Floor" { "Floor" } else { "Wall" }.into(),
            status: "unsupported_geometry".into(),
            diagnostics: Vec::new(),
            source: Source {
                stream: r.stream.clone(),
                group_marker_offset: r.group_marker_offset,
                group_record_offset: r.outer,
                group_segment_count: r.group_segment_count,
                body_sha256: hash(body(r)),
                stored_revision: e.stored_revision,
                other_revision: e.other_revision,
            },
            curves: Vec::new(),
            geometry: None,
            meshes: Vec::new(),
        };
        let result = (|| -> Result<()> {
            ensure!(
                !unknown_opening,
                "unresolved opening host prevents complete host geometry"
            );
            if class == "SWall" {
                let joins = evaluated_joins
                    .as_ref()
                    .map_err(|e| anyhow::anyhow!("native join proof unavailable: {e:#}"))?;
                if let Some(join) = joins.get(id) {
                    ensure!(
                        !openings.contains_key(id),
                        "combined wall joins and openings outside qualified scope"
                    );
                    let mesh = join.mesh.as_ref().ok_or_else(|| {
                        anyhow::anyhow!(
                            "native join outside qualified scope: {}",
                            join.diagnostic
                                .as_deref()
                                .unwrap_or("missing qualified mesh")
                        )
                    })?;
                    ensure!(
                        join.diagnostic.is_none(),
                        "qualified join contains diagnostic"
                    );
                    element.meshes.push(mesh.clone());
                    element.geometry = Some(join.geometry.clone());
                    element.status = "decoded_joined_wall".into();
                    return Ok(());
                }
            }
            let b = body(r);
            let offsets = markers(b);
            ensure!(!offsets.is_empty(), "no supported bounded curve sequence");
            if class != "Floor" {
                ensure!(
                    !joined.contains(id),
                    "native join topology references another wall; uncut mesh withheld"
                );
                ensure!(
                    class == "SWall" || !openings.contains_key(id),
                    "curved wall openings unsupported"
                );
                let kind = if class == "SWall" {
                    WallCurveKind::Line
                } else {
                    WallCurveKind::Arc
                };
                let length = if kind == WallCurveKind::Line {
                    488
                } else {
                    649
                };
                let span = b
                    .get(offsets[0]..offsets[0] + length)
                    .ok_or_else(|| anyhow::anyhow!("truncated wall surface sequence"))?;
                let geometry = native_2027::decode_wall_geometry_span(span, version, kind)
                    .map_err(anyhow::Error::msg)?;
                element.curves.push(curve_value(&geometry.curve));
                if class == "SWall" {
                    let proofs = cut_host_proof
                        .as_ref()
                        .map_err(|e| anyhow::anyhow!("wall host proof unavailable: {e:#}"))?;
                    let diagnostic = proofs
                        .get(id)
                        .ok_or_else(|| anyhow::anyhow!("current wall host proof missing"))?;
                    ensure!(
                        diagnostic.is_none(),
                        "wall host outside qualified scope: {}",
                        diagnostic.as_deref().unwrap_or("")
                    );
                }
                if let Some(host_cuts) = openings.get(id) {
                    let available = rectangular_cuts
                        .as_ref()
                        .map_err(|e| anyhow::anyhow!("opening ownership unavailable: {e:#}"))?;
                    let selected = host_cuts
                        .iter()
                        .map(|(opening, sketch)| {
                            ensure!(*sketch == 0, "nonrectangular wall opening unsupported");
                            let c = available
                                .get(opening)
                                .ok_or_else(|| anyhow::anyhow!("rectangular opening absent"))?;
                            ensure!(c.host_id == *id, "opening host differs from native record");
                            Ok(c)
                        })
                        .collect::<Result<Vec<_>>>()?;
                    element.meshes.push(cuts::wall(&geometry, &selected)?);
                    let mut value = serde_json::to_value(&geometry)?;
                    value["rectangular_cuts"] = serde_json::to_value(selected)?;
                    element.geometry = Some(value);
                    element.status = "decoded_wall_with_rectangular_cuts".into();
                } else {
                    element.meshes.push(mesh::wall(&geometry, kind)?);
                    element.geometry = Some(serde_json::to_value(geometry)?);
                    element.status = "decoded_uncut_wall_surface_model".into();
                }
            } else {
                let mut edges = Vec::new();
                for offset in offsets {
                    let (a, b, c) = line(b, offset)?;
                    edges.push((a, b));
                    element.curves.push(c);
                }
                for &(opening_id, sketch_id) in openings.get(id).into_iter().flatten() {
                    let sketch = unique
                        .get(&(sketch_id, "VarSketch".into()))
                        .ok_or_else(|| {
                            anyhow::anyhow!(
                                "Opening {opening_id} has unresolved sketch {sketch_id}"
                            )
                        })?;
                    let sb = body(sketch);
                    let marker_offsets = markers(sb);
                    ensure!(
                        !marker_offsets.is_empty(),
                        "opening sketch has no supported lines"
                    );
                    for offset in marker_offsets {
                        let (a, b, c) = line(sb, offset)?;
                        edges.push((a, b));
                        element.curves.push(c);
                    }
                }
                ensure!(b.len() >= 210, "short floor");
                let repeated = &b[b.len() - 105..];
                let matches: Vec<_> = b[..b.len() - 105]
                    .windows(105)
                    .enumerate()
                    .filter_map(|(i, v)| (v == repeated).then_some(i))
                    .collect();
                ensure!(matches.len() == 1, "ambiguous repeated floor surface");
                let pair = b
                    .get(matches[0]..matches[0] + 210)
                    .ok_or_else(|| anyhow::anyhow!("truncated floor planes"))?;
                let planes = native_2027::decode_floor_plane_pair(pair, repeated, version)
                    .map_err(anyhow::Error::msg)?;
                let rings = mesh::loops(edges)?;
                let top = planes[0].origin[2];
                let bottom = planes[1].origin[2];
                element.meshes.push(mesh::floor(&rings, bottom, top)?);
                element.geometry = Some(
                    serde_json::json!({"kind":"vertical_extrusion","profiles_xy":rings,"top_elevation":top,"bottom_elevation":bottom,"height":top-bottom}),
                );
                element.status = "decoded_horizontal_polygonal_floor".into();
            }
            Ok(())
        })();
        if let Err(error) = result {
            element.meshes.clear();
            element.diagnostics.push(error.to_string());
        }
        scene.elements.push(element);
    }
    // Multi-layer wall reference side planes can bound the core rather than
    // the complete wall. Never present that core prism as the finished wall.
    if scene
        .elements
        .iter()
        .any(|e| unique.contains_key(&(e.id, "SWall".into())) && !e.meshes.is_empty())
    {
        let mut materials = crate::native_materials::InventoryBuilder::default();
        let inventory =
            crate::native_document::extract(rf, &Default::default(), |r| materials.ingest(&r))
                .and_then(|_| materials.finish());
        for element in scene
            .elements
            .iter_mut()
            .filter(|e| unique.contains_key(&(e.id, "SWall".into())) && !e.meshes.is_empty())
        {
            let result = (|| -> Result<()> {
                let inventory = inventory
                    .as_ref()
                    .map_err(|e| anyhow::anyhow!("wall layer validation unavailable: {e}"))?;
                let refs: Vec<_> = inventory
                    .owner_type_references
                    .iter()
                    .filter(|r| r.source.owner.element_id == element.id)
                    .collect();
                ensure!(refs.len() == 1, "wall layer type reference unavailable");
                let ty = u64::try_from(refs[0].raw_target_id)?;
                let constructions: Vec<_> = inventory
                    .constructions
                    .iter()
                    .filter(|c| c.identity.element_id == ty)
                    .collect();
                ensure!(
                    constructions.len() == 1,
                    "wall construction width unavailable"
                );
                let width: f64 = constructions[0].layers.iter().map(|l| l.width).sum();
                let g = element
                    .geometry
                    .as_ref()
                    .ok_or_else(|| anyhow::anyhow!("wall reference geometry unavailable"))?;
                if element.status == "decoded_joined_wall" {
                    let evaluated_width = g["finished_width"]
                        .as_f64()
                        .filter(|v| v.is_finite() && *v > 0.0)
                        .ok_or_else(|| anyhow::anyhow!("qualified join finished width missing"))?;
                    ensure!(
                        width > 0.0 && (evaluated_width - width).abs() < 1e-7,
                        "joined wall profile/construction width mismatch"
                    );
                    return Ok(());
                }
                let a = &g["surfaces"][1]["origin"];
                let b = &g["surfaces"][2]["origin"];
                let mut square = 0.;
                for i in 0..3 {
                    let d = a[i]
                        .as_f64()
                        .ok_or_else(|| anyhow::anyhow!("wall plane origin"))?
                        - b[i]
                            .as_f64()
                            .ok_or_else(|| anyhow::anyhow!("wall plane origin"))?;
                    square += d * d;
                }
                ensure!(
                    width > 0. && (square.sqrt() - width).abs() < 1e-7,
                    "saved wall reference planes bound core rather than total construction width; finished mesh withheld"
                );
                Ok(())
            })();
            if let Err(e) = result {
                element.meshes.clear();
                element.status = "unsupported_finished_wall_extent".into();
                element.diagnostics.push(e.to_string());
            }
        }
    }
    scene.coverage.indexed_rows = entries.len();
    scene.coverage.recognized_native_class_records = unique.len();
    scene.coverage.recognized_native_owners = unique
        .keys()
        .map(|(id, _)| *id)
        .collect::<BTreeSet<_>>()
        .len();
    scene.coverage.recognized_geometry_owners = scene.elements.len();
    scene.coverage.mesh_owners = scene
        .elements
        .iter()
        .filter(|e| !e.meshes.is_empty())
        .count();
    scene.coverage.recognized_geometry_owners_without_mesh =
        scene.elements.len() - scene.coverage.mesh_owners;
    scene.coverage.indexed_rows_not_recognized =
        entries.len() - scene.coverage.recognized_native_owners;
    scene.elements.sort_by_key(|e| e.id);
    Ok(scene)
}

#[cfg(test)]
mod physical_tests {
    use super::*;
    use std::io::Write;

    fn frame(channel: u64) -> Vec<u8> {
        let mut bytes = 42u64.to_le_bytes().to_vec();
        if channel != 101 {
            bytes.extend(0u32.to_le_bytes());
        }
        bytes.extend(3u32.to_le_bytes());
        bytes.extend([7, 8, 9]);
        bytes.extend(3u32.to_le_bytes());
        bytes
    }

    #[test]
    fn all_main_channels_require_exact_dual_lengths_and_counts() {
        for channel in [101, 102, 103] {
            let source = native_segments::GroupSource {
                content_key: None,
                channel,
                first_marker_offset: 18,
                segment_count: 2,
                declared_objects: 1,
                declared_body_bytes: 3,
            };
            let bytes = frame(channel);
            let frames = group_records(&source, &bytes).unwrap();
            assert_eq!(frames.len(), 1);
            assert_eq!(frames[0].0, 42);
            assert_eq!(&bytes[frames[0].2..frames[0].3], &[7, 8, 9]);
            let mut bad = bytes.clone();
            *bad.last_mut().unwrap() = 1;
            assert!(group_records(&source, &bad).is_err());
            assert!(group_records(&source, &bytes[..bytes.len() - 1]).is_err());
            let mut wrong = source.clone();
            wrong.declared_objects = 2;
            assert!(group_records(&wrong, &bytes).is_err());
            wrong = source.clone();
            wrong.declared_body_bytes = 4;
            assert!(group_records(&wrong, &bytes).is_err());
        }
    }

    fn registry() -> crate::schema_registry::Registry {
        let classes: Vec<_> = [
            "ContentMarker",
            "ContentKey",
            "SegmentMarker",
            "SegmentCheckback",
            "SignatureMarker",
        ]
        .iter()
        .enumerate()
        .map(|(i, name)| {
            serde_json::json!({
                "tag":12+i,"name":name,"offset":0,
                "parent_reference":{"offset":0,"tag":0,"introduces_definition":false},
                "version_like_word":0,"fields":[],"opaque_16byte_entry_count":0,
                "opaque_entries_offset":0,"opaque_entries_sha256":"","end":0
            })
        })
        .collect();
        serde_json::from_value(serde_json::json!({"source_sha256":"","consumed_bytes":0,
            "reference_count":0,"terminator_offset":0,"classes":classes}))
        .unwrap()
    }
    fn segment(out: &mut Vec<u8>, flags: u32, count: u32, body_count: u32, bytes: &[u8]) {
        let mut encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
        encoder.write_all(bytes).unwrap();
        let gzip = encoder.finish().unwrap();
        let size = gzip.len() as u32 + 8;
        out.extend(14u16.to_le_bytes());
        for n in [flags, count, size, body_count] {
            out.extend(n.to_le_bytes());
        }
        out.extend(102u64.to_le_bytes());
        out.extend(gzip);
        out.extend(15u16.to_le_bytes());
        out.extend(size.to_le_bytes());
    }
    #[test]
    fn physical_walk_reassembles_record_split_inside_its_length_word() {
        let registry = registry();
        let bytes = frame(102);
        let mut prepared = 9u64.to_le_bytes().to_vec();
        prepared.extend(12u16.to_le_bytes());
        prepared.extend(0u32.to_le_bytes());
        prepared.extend(1u32.to_le_bytes());
        let first_marker = prepared.len();
        segment(&mut prepared, 6, 1, 3, &bytes[..14]);
        let second_marker = prepared.len();
        segment(&mut prepared, 5, 0, 0, &bytes[14..]);
        prepared.extend(12u16.to_le_bytes());
        prepared.extend(0u32.to_le_bytes());
        prepared.extend(u32::MAX.to_le_bytes());
        let mut calls = 0;
        native_segments::walk(&prepared, &registry, 1024, |source, group| {
            calls += 1;
            assert_eq!(source.first_marker_offset, first_marker);
            assert_eq!(source.segment_count, 2);
            assert_eq!(group, bytes);
            assert_eq!(group_records(source, group)?.len(), 1);
            Ok(())
        })
        .unwrap();
        assert_eq!(calls, 1);
        let mut bad = prepared.clone();
        bad[second_marker + 2..second_marker + 6].copy_from_slice(&4u32.to_le_bytes());
        assert!(native_segments::walk(&bad, &registry, 1024, |_, _| Ok(())).is_err());
        let mut bad = prepared.clone();
        // Last byte before the second marker belongs to the first checkback size.
        bad[second_marker - 1] ^= 1;
        assert!(native_segments::walk(&bad, &registry, 1024, |_, _| Ok(())).is_err());
        assert!(
            native_segments::walk(&prepared, &registry, bytes.len() - 1, |_, _| Ok(())).is_err()
        );
        assert!(
            native_segments::walk(&prepared[..second_marker - 7], &registry, 1024, |_, _| Ok(
                ()
            ))
            .is_err()
        );
    }
}
