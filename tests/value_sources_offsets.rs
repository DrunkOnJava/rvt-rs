//! #1360: every body drawn from a record other than the element's own frame
//! names the copy it was drawn from, and the bytes at that copy decode to what
//! the reader read.
//!
//! The export's provenance table lists, for each element, the copy each reader
//! took a value of its body from (`value_sources`: the reader, the rule that
//! chose the copy, its stream and offset). This test exports each model with
//! the table on, then, without the reader that noted the choice:
//!
//! - looks up every element's `BodySource` in `COVERAGE`, which says which
//!   reader and rule a body must have a value source from, or that no reader
//!   notes its copy yet. A body that is in neither, on any model, fails the
//!   test: nothing is excluded by being left out of the table;
//! - reads the bytes at the offset of each element-data copy (the readers of
//!   a wall's or beam's line and of a family instance's transform): the
//!   element-data header, then the element's own id, then a record that
//!   decodes to the value the reader returns for that element when it is
//!   asked again;
//! - reads the bytes at the offset of each native record: the ElementId is the
//!   first bytes there;
//! - reads the bytes at each record in a source's `parts`, by what the record
//!   is: a sketch line's frame or a type symbol's frame has the element's id at
//!   its first bytes, or, on a second-prologue frame (no id there), the id of
//!   the record that encloses the frame, which this test finds from the bytes
//!   (`partition_record_chain`); a sketch line's data (`sketch_line_data`) is an
//!   element-data header followed by the line's own id; a room's solid is the
//!   room's id and the solid's tag, and read again it gives an outline of the
//!   vertex count the room holds; a type's parameter entry is an `f64`, `ff` x 8
//!   and an `i64`, in the value block the type owns (the nearest block mark
//!   before it whose owner is declared), and a section's parts are distinct
//!   entries of the expected parameters (an I section's six, a hollow
//!   section's `b`, `Ht`, `t` and `kr` by their captions); a pipe's connector
//!   entry is the keyed element's `u64`, an index of at most 255, the `u32` 1
//!   and a point, the two parts' points making a cylinder (`pipe_body`) with
//!   the pipe's record box (the frame decoded here); a run type is an id
//!   object with the type's id; and a Sketch's curve list is an element-data
//!   header followed, at a fixed place, by the sketched element's id. Frames of
//!   a 32-bit release that carry no id at their first bytes are counted as not
//!   read;
//! - requires an element whose profile is solved from sketch lines, or is a
//!   type symbol's box, to have a source from the reader of that profile;
//! - checks that a `selected` source names the frame the element's entry
//!   names (that the frame's own bytes hold the element is the #1325 test's,
//!   `element_provenance_offsets.rs`, and is not read again here), that an
//!   element whose profile is read from another record has `selected_frame_box`
//!   and no other `selected_frame`, that `differs_from_frame` says whether the
//!   value is held in another partition than the frame, and that
//!   `value_source_counts` equals the count made here;
//! - on the models whose family instances are drawn from their families' own
//!   extrusions, exported with `--family-extrusions` as well, reads the
//!   family's own document again and requires an instance's parts to be all
//!   the records its visible extrusions are made of and no others: each
//!   extrusion, its sketch plane and its sketch's curves, every one read from
//!   its bytes by its role (the record's id, then a body of the same length at
//!   both ends that opens with the tag of an `ExtrusionElem`, a `SketchPlane`
//!   or a `CurveElem` respectively);
//! - on puzzbobb's Revit 2025 models (`RVT_R2025_DIR`, where the roofs drawn
//!   from their slopes and from an extrusion are), requires the models
//!   together to have sources from `R2025_READERS`, and reads a roof's slope
//!   blocks and sketch lines again: each decodes to a slope, an arrow or a
//!   curve;
//! - requires each model's `MIN_READERS` to have sources, so a table with no
//!   sources cannot pass.
//!
//! What it does not do: say that the copy a reader took is the right one. For
//! the `agreeing` rule every copy holds the same value, so any of them is; for
//! `current` the choice is `native_document::current_partitions`, whose
//! correctness this test does not measure. The bodies marked `NotYet` are
//! counted and printed, with how many of their elements have a source anyway.
//!
//! Per model, the counts by body source and reader go to
//! `value-sources/<model>.json` and `summary.txt` under `CARGO_TARGET_TMPDIR`,
//! which CI prints and uploads.
//!
//! Runs on every `.rvt` under `RVT_PROJECT_CORPUS_DIR` (Core Interior and the
//! RE1 models in CI) and on the BIM4LCA as-built office under
//! `RVT_NORDIC_LCA_DIR`; each is skipped when its variable is unset.

use crate::element_parameters::nordic_dir;
use rvt::ifc::entities::{IfcEntity, PropertyValue};
use rvt::ifc::{ExportOptions, IfcModel, run_export};
use rvt::research::element_record_plan_profiles::SKETCH_OWNER_OFFSET;
use rvt::research::native_document::{Options, extract_graphs};
use rvt::research::native_family_forms::family_records_in;
use rvt::research::native_form_geometry::FormDocument;
use rvt::research::partition_beam_axes::{
    BEAM_DATA_WINDOW, BoundedArc, BoundedLine, first_bounded_arc, first_bounded_line,
    first_bounded_spline, first_circle, first_sketch_curve, scan_first_bounded_arcs,
    scan_first_bounded_lines,
};
use rvt::research::partition_compound_structure::{
    LAYER_WINDOW, find_layers, layer_frame_tag, layer_layout,
};
use rvt::research::partition_element_records::{
    PartitionElementRecord, PartitionRecordSpan, carries_no_element_id, decode_at_with_marker,
    decode_frame_as, enclosing_record, file_bbox_marker, partition_record_chain,
};
use rvt::research::partition_instance_transforms::{
    INSTANCE_TRANSFORM_WINDOW, InstanceTransform, instance_transform, scan_instance_transforms,
};
use rvt::research::partition_names::{ElementDataLayout, element_data_layout};
use rvt::research::partition_pipe_axes::pipe_body;
use rvt::research::partition_roof_slopes::{
    EDGE_SLOPE_WINDOW, arrow_heights, edge_slope_in, fascia_depth, scan_rafter_cuts,
};
use rvt::research::partition_room_boundaries::{
    ROOM_OUTLINE_SOURCE, ROOM_OUTLINE_SOURCE_APPROXIMATE, ROOM_OUTLINE_SOURCE_FACES,
    ROOM_SOLID_TAG, ResolvedKind, is_room_outline_source, resolved_outline_of_solids,
    room_outline_at,
};
use rvt::research::partition_type_parameters::{
    HOLLOW_SECTION_PARAMETER_NAMES, I_SECTION_PARAMETERS, VALUE_BLOCK_MARK,
};
use rvt::research::partition_type_records::{OST_MATERIALS, scan_type_records};
use rvt::{RevitFile, elem_table};
use serde_json::json;
use std::collections::{BTreeMap, BTreeSet};
use std::io::Write;
use std::path::{Path, PathBuf};

/// How a `BodySource` is covered.
enum Coverage {
    /// Every element with this body has a value source from one of `readers`
    /// by one of `rules`.
    Recorded {
        readers: &'static [&'static str],
        rules: &'static [&'static str],
    },
    /// No reader notes its copy yet: counted and printed, not asserted.
    NotYet,
}

use Coverage::{NotYet, Recorded};

/// The record's box, or the box trimmed or cut by joins: read from the
/// selected frame. `selected_frame_box` is the same where the profile is read
/// from another record (sketch lines, the type's section), which no reader
/// notes yet.
const FRAME: Coverage = Recorded {
    readers: &["selected_frame", "selected_frame_box"],
    rules: &["selected"],
};

/// A wall's or beam's location line: every copy of its element data agrees.
const LINE: Coverage = Recorded {
    readers: &["bounded_line"],
    rules: &["agreeing"],
};

/// Every `BodySource` on the models below, and how its value sources are
/// covered. A row is disputed here, not left out.
const COVERAGE: &[(&str, Coverage)] = &[
    ("partition_element_record_bbox", FRAME),
    ("partition_element_record_join_trimmed", FRAME),
    ("partition_element_record_join_cut", FRAME),
    ("partition_element_record_column_cut", FRAME),
    ("partition_wall_centreline", LINE),
    ("partition_wall_centreline_layer_joins", LINE),
    ("partition_wall_tapered_section", LINE),
    (
        "partition_wall_arc",
        Recorded {
            readers: &["bounded_arc"],
            rules: &["agreeing"],
        },
    ),
    ("partition_beam_axis", LINE),
    ("partition_beam_box_along_axis", LINE),
    (
        "partition_family_instance_turned_box",
        Recorded {
            readers: &["instance_transform"],
            rules: &["agreeing", "current"],
        },
    ),
    (
        "partition_family_instance_oriented_box",
        Recorded {
            readers: &["instance_transform"],
            rules: &["agreeing", "current"],
        },
    ),
    (
        "saved_graphics_mesh",
        Recorded {
            readers: &["saved_graphics"],
            rules: &["native"],
        },
    ),
    (
        "native_spiral_run_treads",
        Recorded {
            readers: &["native_run"],
            rules: &["native"],
        },
    ),
    (
        "native_landing_tread",
        Recorded {
            readers: &["native_landing"],
            rules: &["native"],
        },
    ),
    (
        "partition_stair_run_sketch",
        Recorded {
            readers: &[
                "stair_run_lines",
                "stair_run_ends",
                "stair_run_type",
                "stair_run_base_cuts",
                "native_run",
                "native_run_type",
            ],
            rules: &["agreeing", "native"],
        },
    ),
    (
        "partition_landing_slab",
        Recorded {
            readers: &["landing_lines", "native_landing", "native_landing_type"],
            rules: &["agreeing", "native"],
        },
    ),
    ("partition_area_boundary_lines", NotYet),
    (
        "family_type_hollow_section",
        Recorded {
            readers: &["type_hollow_section"],
            rules: &["agreeing"],
        },
    ),
    (
        "family_type_i_section",
        Recorded {
            readers: &["type_i_section"],
            rules: &["agreeing"],
        },
    ),
    (
        "partition_pipe_axis",
        Recorded {
            readers: &["pipe_connectors"],
            rules: &["agreeing"],
        },
    ),
    (
        "partition_roof_slope",
        Recorded {
            readers: &["roof_slope_edges"],
            rules: &["agreeing"],
        },
    ),
    (
        "partition_roof_extrusion",
        Recorded {
            readers: &["roof_line_data"],
            rules: &["agreeing", "current"],
        },
    ),
    (
        "family_extrusions",
        Recorded {
            readers: &["family_forms"],
            rules: &["frame"],
        },
    ),
];

/// The readers that at least one model of the Revit 2025 set
/// (`RVT_R2025_DIR`, puzzbobb's models) must have sources from: the roofs drawn
/// from their slopes and from their extrusion, and a straight run.
const R2025_READERS: &[&str] = &[
    "sketch_lines",
    "roof_slope_edges",
    "roof_line_data",
    "roof_type_layers",
    "roof_fascia_depth",
    "roof_rafter_cut",
    "native_run",
];

/// The readers that source what a box alone does not (a profile, a section, a
/// radius): an element with one is `selected_frame`, not `selected_frame_box`.
const PROFILE_FRAME_READERS: &[&str] = &[
    "sketch_lines",
    "type_section",
    "room_solid_faces",
    "type_hollow_section",
    "type_i_section",
    "pipe_connectors",
    "family_forms",
];

/// The models whose family instances are drawn from their families' own
/// extrusions (#1372), and the readers each must have sources from when
/// exported with `--family-extrusions` as well as the table: the form records
/// the body was rebuilt from, and the transform it was placed with.
const FAMILY_MIN_READERS: &[(&str, &[&str])] = &[
    (
        "nordic-office-as-built",
        &["selected_frame", "family_forms", "instance_transform"],
    ),
    (
        "RE1-Architecture",
        &["selected_frame", "family_forms", "instance_transform"],
    ),
    (
        "RE1-Electrical",
        &["selected_frame", "family_forms", "instance_transform"],
    ),
];

/// The readers each model's table must have at least one value source from.
const MIN_READERS: &[(&str, &[&str])] = &[
    (
        "2024_Core_Interior",
        &[
            "selected_frame",
            "bounded_line",
            "saved_graphics",
            "sketch_lines",
            "type_section",
            "room_solid_faces",
        ],
    ),
    (
        "RE1-Architecture",
        &["selected_frame", "bounded_line", "saved_graphics"],
    ),
    // puzzbobb's Revit 2025 roofs: a shed by its edge, a shed by its slope
    // arrow and a roof by extrusion, each drawn from the sources pinned here
    // (RE-56, RE-199, RE-211).
    (
        "02_1Slope",
        &[
            "selected_frame",
            "roof_slope_edges",
            "roof_line_data",
            "roof_type_layers",
        ],
    ),
    (
        "05_1Slope_ArrowDefined",
        &[
            "selected_frame",
            "roof_slope_edges",
            "roof_line_data",
            "roof_type_layers",
        ],
    ),
    (
        "11_RoofByExtrusion",
        &[
            "selected_frame",
            "sketch_lines",
            "roof_line_data",
            "roof_type_layers",
        ],
    ),
    ("RE1-Mechanical", &["selected_frame", "pipe_connectors"]),
    ("RE1-Plumbing", &["selected_frame", "pipe_connectors"]),
    ("Revit_IFC5_Einhoven", &["selected_frame", "saved_graphics"]),
    (
        "nordic-office-as-built",
        &[
            "selected_frame",
            "bounded_line",
            "saved_graphics",
            "instance_transform",
            "native_run",
            "native_landing",
            "stair_run_lines",
            "landing_lines",
            "room_solid_faces",
            "type_hollow_section",
            "pipe_connectors",
        ],
    ),
];

/// The ElementId an `Identifier` field holds: `m_id.m_id64` from Revit 2024,
/// `m_id.m_id` before.
fn identifier_of(value: &serde_json::Value) -> Option<i64> {
    let identifier = value.get("m_id")?;
    identifier
        .get("m_id64")
        .or_else(|| identifier.get("m_id"))
        .and_then(serde_json::Value::as_i64)
}

/// The class `class` records' root ElementId field `field`, by record.
fn root_ids(
    rf: &mut RevitFile,
    ids: &BTreeSet<u64>,
    class: &str,
    field: &str,
) -> BTreeMap<u64, u64> {
    let mut out = BTreeMap::new();
    if ids.is_empty() {
        return out;
    }
    let options = Options {
        selected_ids: ids.clone(),
        ..Options::default()
    };
    extract_graphs(rf, &options, |record| {
        let target = record
            .graph
            .as_ref()
            .and_then(|graph| graph.objects.first())
            .filter(|root| root.class_name == class)
            .and_then(|root| root.fields.get(field))
            .and_then(identifier_of)
            .and_then(|n| u64::try_from(n).ok());
        if let Some(target) = target {
            out.insert(record.identity.element_id, target);
        }
        Ok(())
    })
    .expect("read the family instances' records");
    out
}

/// Of the family instances `instances`, the records of its family's document
/// that its body is rebuilt from, with the class each must have: its visible
/// extrusions (`ExtrusionElem`), their sketch planes (`SketchPlane`) and their
/// sketches' curves (`CurveElem`). Read from the whole document, not from the
/// parts a source lists. An instance whose family has a void, or an extrusion
/// the reader cannot read, is not rebuilt and is left out.
fn family_form_roles(
    rf: &mut RevitFile,
    instances: &BTreeSet<u64>,
) -> BTreeMap<u64, BTreeMap<u64, &'static str>> {
    let symbol_of = root_ids(rf, instances, "FamilyInstance", "m_masterSymbolId");
    let symbols: BTreeSet<u64> = symbol_of.values().copied().collect();
    let family_of_symbol = root_ids(rf, &symbols, "FamilySymbol", "m_familyId");
    let families: BTreeSet<u64> = family_of_symbol.values().copied().collect();
    let classes: BTreeSet<String> = ["ExtrusionElem", "CurveElem", "SketchPlane"]
        .iter()
        .map(|class| class.to_string())
        .collect();
    let records = family_records_in(rf, &families, &classes, &Options::default().releases, None)
        .expect("read the families' documents");
    let mut by_family: BTreeMap<u64, BTreeMap<u64, &'static str>> = BTreeMap::new();
    for (family, items) in records {
        let mut document = FormDocument::default();
        for (id, graph) in items {
            document.insert(id, graph);
        }
        let mut roles: BTreeMap<u64, &'static str> = BTreeMap::new();
        let mut rebuilt = true;
        for id in document.extrusion_ids() {
            let Ok(extrusion) = document.extrusion(id) else {
                rebuilt = false;
                break;
            };
            if extrusion.cutting {
                rebuilt = false;
                break;
            }
            if extrusion.visible == Some(0) {
                continue;
            }
            roles.insert(extrusion.id, "ExtrusionElem");
            roles.insert(extrusion.plane_id, "SketchPlane");
            for curve in &extrusion.curve_ids {
                roles.insert(*curve, "CurveElem");
            }
        }
        if rebuilt && !roles.is_empty() {
            by_family.insert(family, roles);
        }
    }
    instances
        .iter()
        .filter_map(|instance| {
            let family = family_of_symbol.get(symbol_of.get(instance)?)?;
            Some((*instance, by_family.get(family)?.clone()))
        })
        .collect()
}

fn project_models() -> Vec<(String, PathBuf)> {
    let Some(dir) = std::env::var_os("RVT_PROJECT_CORPUS_DIR") else {
        eprintln!("skipping the project corpus: RVT_PROJECT_CORPUS_DIR is not set");
        return Vec::new();
    };
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut models: Vec<(String, PathBuf)> = entries
        .filter_map(|entry| entry.ok().map(|e| e.path()))
        .filter(|path| path.extension().is_some_and(|ext| ext == "rvt"))
        .filter_map(|path| {
            let name = path.file_stem()?.to_string_lossy().into_owned();
            Some((name, path))
        })
        .collect();
    models.sort();
    models
}

/// puzzbobb's Revit 2025 models (`RVT_R2025_DIR`, #356): the only models of
/// the gate whose roofs are drawn from their slopes and from an extrusion.
fn r2025_models() -> Vec<(String, PathBuf)> {
    let Some(dir) = std::env::var_os("RVT_R2025_DIR") else {
        eprintln!("skipping the Revit 2025 models: RVT_R2025_DIR is not set");
        return Vec::new();
    };
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut models: Vec<(String, PathBuf)> = entries
        .filter_map(|entry| entry.ok().map(|e| e.path()))
        .filter(|path| path.extension().is_some_and(|ext| ext == "rvt"))
        .filter_map(|path| {
            let name = path.file_stem()?.to_string_lossy().into_owned();
            Some((name, path))
        })
        .collect();
    models.sort();
    models
}

fn office_model() -> Vec<(String, PathBuf)> {
    let Some(dir) = nordic_dir() else {
        return Vec::new();
    };
    let path = dir.join("ARK_NordicLCA_Office_Concrete_As-Built.rvt");
    if !path.is_file() {
        eprintln!("skipping the office: the as-built model is absent");
        return Vec::new();
    }
    vec![("nordic-office-as-built".to_string(), path)]
}

/// Each element's ElementId and `BodySource`, with whether its profile is read
/// from another record than its body's (a `ProfileSource` that is not the
/// `BodySource`, with the profile resolved).
type Bodies = BTreeMap<(u32, String), BodyInfo>;

/// What re-reading one record of a source's `parts` found.
enum Part {
    Confirmed,
    /// The release carries the element's id where this test does not read it.
    Unverified,
    Failed,
}

/// What the model says of how an element's profile was read.
struct BodyInfo {
    /// The profile is read from another record than the body's.
    profile_elsewhere: bool,
    /// The `ProfileSource` property.
    profile_source: Option<String>,
    /// The `ProfileVertexCount` property.
    profile_vertices: Option<i64>,
}

/// The elements of `model` by ElementId and `BodySource`, and how many entities
/// they are: entities that share a Tag (the pieces of a floor) are one element.
fn element_bodies(model: &IfcModel) -> (Bodies, usize) {
    let mut bodies = Bodies::new();
    let mut entities = 0usize;
    for entity in &model.entities {
        let IfcEntity::BuildingElement {
            type_guid,
            property_set,
            ..
        } = entity
        else {
            continue;
        };
        let Some(id) = type_guid.as_deref().and_then(|tag| tag.parse::<u32>().ok()) else {
            continue;
        };
        let mut body = None;
        let mut profile_source = None;
        let mut profile_resolved = false;
        let mut profile_vertices = None;
        for property in property_set.iter().flat_map(|set| &set.properties) {
            match (property.name.as_str(), &property.value) {
                ("BodySource", PropertyValue::Text(text)) => body = Some(text.clone()),
                ("ProfileSource", PropertyValue::Text(text)) => profile_source = Some(text.clone()),
                ("ProfileResolved", PropertyValue::Boolean(true)) => profile_resolved = true,
                ("ProfileVertexCount", PropertyValue::Integer(count)) => {
                    profile_vertices = Some(*count)
                }
                _ => {}
            }
        }
        if let Some(body) = body {
            entities += 1;
            let elsewhere = profile_resolved && profile_source.as_ref() != Some(&body);
            bodies.entry((id, body)).or_insert(BodyInfo {
                profile_elsewhere: elsewhere,
                profile_source,
                profile_vertices,
            });
        }
    }
    (bodies, entities)
}

fn f64_at(buf: &[u8], at: usize) -> Option<f64> {
    buf.get(at..at.checked_add(8)?)
        .map(|b| f64::from_le_bytes(b.try_into().expect("8 bytes")))
}

fn i64_at(buf: &[u8], at: usize) -> Option<i64> {
    buf.get(at..at.checked_add(8)?)
        .map(|b| i64::from_le_bytes(b.try_into().expect("8 bytes")))
}

/// The ElementId of the value block that the entry at `at` is in: the nearest
/// block mark before it, not preceded by a further `ff`, whose owner is
/// declared or is 0, as the reader's blocks are. A mark with 0 before it ends
/// the block before it and is owned by none.
fn value_block_owner(buf: &[u8], at: usize, declared: &BTreeSet<u32>) -> Option<u32> {
    let mark = &VALUE_BLOCK_MARK[..];
    let mut start = at.checked_sub(mark.len())?;
    loop {
        if start >= 8 && buf.get(start..start + mark.len())? == mark && buf[start - 1] != 0xff {
            match u32::try_from(u64_at(buf, start - 8)?) {
                Ok(0) if !declared.contains(&0) => return None,
                Ok(id) if declared.contains(&id) => return Some(id),
                _ => {}
            }
        }
        start = start.checked_sub(1)?;
    }
}

/// The partition element record of the frame an entry names, read again.
fn frame_record(
    rf: &mut RevitFile,
    frame: Option<(&str, usize)>,
    id: u32,
    marker: Option<[u8; 8]>,
    declared: &BTreeSet<u32>,
) -> Option<PartitionElementRecord> {
    let ((stream, offset), marker) = frame.zip(marker)?;
    let inflated = rf.inflated_partition(stream).ok()?;
    decode_at_with_marker(stream, inflated.bytes(), offset, declared, &marker)
        .or_else(|| decode_frame_as(stream, inflated.bytes(), offset, id, &marker))
}

fn u16_at(buf: &[u8], at: usize) -> Option<u16> {
    buf.get(at..at.checked_add(2)?)
        .map(|b| u16::from_le_bytes(b.try_into().expect("2 bytes")))
}

fn u32_at(buf: &[u8], at: usize) -> Option<u32> {
    buf.get(at..at.checked_add(4)?)
        .map(|b| u32::from_le_bytes(b.try_into().expect("4 bytes")))
}

fn u64_at(buf: &[u8], at: usize) -> Option<u64> {
    buf.get(at..at.checked_add(8)?)
        .map(|b| u64::from_le_bytes(b.try_into().expect("8 bytes")))
}

/// The element data of the copy that starts at `offset` in `buf`, as the
/// readers see it: after the header, the element's id and eight bytes, up to
/// the next header or `window` bytes (from the copy's start for a line, from
/// the data for a transform). `None` when the bytes there are not the header
/// followed by `id`.
fn element_data_at<'a>(
    buf: &'a [u8],
    layout: &ElementDataLayout,
    offset: usize,
    id: u32,
    window: usize,
    window_from_data: bool,
) -> Option<&'a [u8]> {
    let header = &layout.header[..];
    if buf.get(offset..offset.checked_add(header.len())?)? != header {
        return None;
    }
    let id_at = offset + header.len();
    if layout.id_at(buf, id_at)? != id {
        return None;
    }
    let start = id_at + 8;
    let next = buf
        .get(id_at..)?
        .windows(header.len())
        .position(|bytes| bytes == header)
        .map(|at| id_at + at);
    let limit = if window_from_data {
        start.saturating_add(window)
    } else {
        offset.saturating_add(window)
    };
    let end = next.unwrap_or(buf.len()).min(limit).min(buf.len());
    buf.get(start..end)
}

#[derive(Default)]
struct BodyStats {
    elements: usize,
    unlisted: usize,
    with_source: usize,
    box_only: usize,
    profiled: usize,
    profile_sourced: usize,
    profile_missing: Vec<u32>,
    missing: Vec<u32>,
    by_reader: BTreeMap<String, usize>,
}

fn check_model(name: &str, path: &Path) -> BTreeSet<String> {
    check_model_with(name, path, false)
}

/// Check a model exported with the table, and with `--family-extrusions` as
/// well where `family_extrusions` is set: its report is then
/// `<model>+family-extrusions`, and its readers are pinned by
/// `FAMILY_MIN_READERS`.
fn check_model_with(base: &str, path: &Path, family_extrusions: bool) -> BTreeSet<String> {
    let label = if family_extrusions {
        format!("{base}+family-extrusions")
    } else {
        base.to_string()
    };
    let name = label.as_str();
    let mut rf = RevitFile::open(path).expect("open the model");
    let run = run_export(
        &mut rf,
        &ExportOptions::default()
            .with_element_provenance(true)
            .with_family_extrusions(family_extrusions),
    )
    .expect("export the model with the table");
    let table = run
        .diagnostics
        .decoded
        .element_provenance
        .as_ref()
        .expect("the table when asked for");
    let version = rf.layout_release().expect("layout release");
    let layout = element_data_layout(version);
    let marker = file_bbox_marker(&mut rf, version);
    let classes = rf.schema_classes().ok();
    let declared: BTreeSet<u32> =
        elem_table::declared_ids(&elem_table::parse_records(&mut rf).expect("ElemTable"));
    let mut chains: BTreeMap<String, Vec<PartitionRecordSpan>> = BTreeMap::new();
    let mut problems: Vec<String> = Vec::new();

    // Coverage by body source: every element, listed in the table or not.
    let mut by_body: BTreeMap<String, BodyStats> = BTreeMap::new();
    let (listed_bodies, entities) = element_bodies(&run.model);
    let distinct_ids = listed_bodies
        .keys()
        .map(|(id, _)| *id)
        .collect::<BTreeSet<u32>>()
        .len();
    for ((id, body), info) in &listed_bodies {
        let (id, profile_elsewhere) = (*id, info.profile_elsewhere);
        let stats = by_body.entry(body.clone()).or_default();
        stats.elements += 1;
        let coverage = COVERAGE.iter().find(|(source, _)| source == body);
        if coverage.is_none() {
            problems.push(format!("{id}: BodySource {body} is not in COVERAGE"));
        }
        let Some(entry) = table.get(id) else {
            stats.unlisted += 1;
            continue;
        };
        match coverage {
            None => {}
            Some((_, Recorded { readers, rules })) => {
                let found = entry.value_sources.iter().find(|source| {
                    readers.contains(&source.reader.as_str())
                        && rules.contains(&source.rule.as_str())
                });
                // The frame an element's box is held to: `selected_frame`
                // where a reader sourced its profile, else `selected_frame_box`.
                if let Some(frame) = entry
                    .value_sources
                    .iter()
                    .find(|held| held.rule.as_str() == "selected")
                {
                    let sourced = entry
                        .value_sources
                        .iter()
                        .any(|held| PROFILE_FRAME_READERS.contains(&held.reader.as_str()));
                    let expected = if profile_elsewhere && !sourced {
                        "selected_frame_box"
                    } else {
                        "selected_frame"
                    };
                    if frame.reader != expected {
                        problems.push(format!(
                            "{id}: {body} has {}, and its profile {} another record",
                            frame.reader,
                            if profile_elsewhere {
                                "is read from"
                            } else {
                                "is not read from"
                            }
                        ));
                    }
                }
                match found {
                    Some(source) => {
                        stats.with_source += 1;
                        stats.box_only += usize::from(source.reader == "selected_frame_box");
                        *stats.by_reader.entry(source.reader.clone()).or_default() += 1;
                        if source.rule.as_str() == "selected" {
                            // The profile's own readers, by the profile's source.
                            let profile_readers: &[&str] = match info.profile_source.as_deref() {
                                Some("partition_element_record_sketch_lines") => &["sketch_lines"],
                                Some("family_type_symbol_envelope") => &["type_section"],
                                Some(source) if is_room_outline_source(source) => {
                                    &["room_solid_faces"]
                                }
                                _ => &[],
                            };
                            if !profile_readers.is_empty() {
                                stats.profiled += 1;
                                let sourced = entry
                                    .value_sources
                                    .iter()
                                    .any(|held| profile_readers.contains(&held.reader.as_str()));
                                if sourced {
                                    stats.profile_sourced += 1;
                                } else if stats.profile_missing.len() < 10 {
                                    stats.profile_missing.push(id);
                                }
                            }
                        }
                    }
                    None if stats.missing.len() < 10 => stats.missing.push(id),
                    None => {}
                }
            }
            Some((_, NotYet)) => {
                if let Some(source) = entry.value_sources.first() {
                    stats.with_source += 1;
                    *stats.by_reader.entry(source.reader.clone()).or_default() += 1;
                }
            }
        }
    }
    for (body, stats) in &by_body {
        if stats.profiled > stats.profile_sourced {
            problems.push(format!(
                "{body}: {} of {} profiles read from sketch lines or a type's section have no value source for them, such as {:?}",
                stats.profiled - stats.profile_sourced,
                stats.profiled,
                stats.profile_missing
            ));
        }
        let recorded = COVERAGE
            .iter()
            .any(|(source, coverage)| source == body && matches!(coverage, Recorded { .. }));
        if recorded && !stats.missing.is_empty() {
            problems.push(format!(
                "{body}: {} of {} elements have no value source from its readers, such as {:?}",
                stats.elements - stats.unlisted - stats.with_source,
                stats.elements - stats.unlisted,
                stats.missing
            ));
        }
    }

    // The bytes at each copy a source names.
    let mut checked = 0usize;
    let mut confirmed = 0usize;
    let mut decoded_lines: BTreeMap<u32, Option<BoundedLine>> = BTreeMap::new();
    let mut decoded_arcs: BTreeMap<u32, Option<BoundedArc>> = BTreeMap::new();
    let mut decoded_transforms: BTreeMap<u32, Option<InstanceTransform>> = BTreeMap::new();
    // Each room's `ProfileSource` and `ProfileVertexCount`, to read its solids
    // again; the hollow sections' parameters, to name them below.
    let rooms: BTreeMap<u32, (Option<String>, Option<i64>)> = listed_bodies
        .iter()
        .map(|((id, _), info)| (*id, (info.profile_source.clone(), info.profile_vertices)))
        .collect();
    let mut hollow_checks: Vec<(u32, Vec<i64>)> = Vec::new();
    // The roofs whose rafter cut is sourced, read again below; the materials the
    // layers reader needs to read a type's layers again.
    let mut rafter_roofs: BTreeSet<u32> = BTreeSet::new();
    let layer_materials: BTreeSet<u32> = if table.elements.values().any(|entry| {
        entry
            .value_sources
            .iter()
            .any(|source| source.reader == "roof_type_layers")
    }) {
        scan_type_records(&mut rf, version, OST_MATERIALS, &declared)
            .unwrap_or_default()
            .iter()
            .map(|record| record.element_id)
            .collect()
    } else {
        BTreeSet::new()
    };
    let mut parts_checked = 0usize;
    let mut parts_confirmed = 0usize;
    let mut parts_by_record = 0usize;
    let mut parts_unverified = 0usize;
    let mut counts: BTreeMap<String, (usize, usize)> = BTreeMap::new();
    let mut by_rule: BTreeMap<String, usize> = BTreeMap::new();
    let width = usize::from(table.record_id_bytes.unwrap_or(8));
    // The records each family instance drawn from its family's extrusions must
    // list, read from the family's whole document.
    let family_instances: BTreeSet<u64> = table
        .elements
        .iter()
        .filter(|(_, entry)| {
            entry
                .value_sources
                .iter()
                .any(|source| source.reader == "family_forms")
        })
        .map(|(id, _)| u64::from(*id))
        .collect();
    let family_roles = family_form_roles(&mut rf, &family_instances);
    for (&id, entry) in &table.elements {
        // A roof by extrusion reads the data of every sketch line whose frame it
        // listed: the same lines, by ElementId (all of them where no more than
        // the listing's cap are held; the count where the lists are cut).
        let framed = entry
            .value_sources
            .iter()
            .find(|source| source.reader == "sketch_lines" && source.rule.as_str() == "frame");
        let data = entry
            .value_sources
            .iter()
            .find(|source| source.reader == "roof_line_data");
        if let (Some(framed), Some(data)) = (framed, data) {
            let framed_total = framed.parts.len() as u32 + framed.parts_left_out;
            let data_total = data.parts.len() as u32 + data.parts_left_out;
            let mut framed_lines: Vec<u32> =
                framed.parts.iter().map(|part| part.element_id).collect();
            let mut data_lines: Vec<u32> = data.parts.iter().map(|part| part.element_id).collect();
            framed_lines.sort_unstable();
            data_lines.sort_unstable();
            let listed_whole = framed.parts_left_out == 0 && data.parts_left_out == 0;
            if framed_total != data_total || (listed_whole && framed_lines != data_lines) {
                problems.push(format!(
                    "{id}: roof_line_data has {data_total} parts ({data_lines:?}) for {framed_total} sketch lines ({framed_lines:?})"
                ));
            }
        }
        // A family instance lists every record its family's visible extrusions
        // are made of, and no other.
        if let Some(forms) = entry
            .value_sources
            .iter()
            .find(|source| source.reader == "family_forms")
        {
            match family_roles.get(&u64::from(id)) {
                None => problems.push(format!(
                    "{id}: family_forms lists parts, but the instance's family has no visible extrusions the reader rebuilds"
                )),
                Some(roles) => {
                    let kept: BTreeSet<u64> = forms
                        .parts
                        .iter()
                        .map(|part| u64::from(part.element_id))
                        .collect();
                    let listed = forms.parts.len() + forms.parts_left_out as usize;
                    let complete = kept.len() == forms.parts.len()
                        && listed == roles.len()
                        && if forms.parts_left_out == 0 {
                            kept.iter().eq(roles.keys())
                        } else {
                            kept.iter().all(|record| roles.contains_key(record))
                        };
                    if !complete {
                        problems.push(format!(
                            "{id}: family_forms lists {listed} records ({kept:?}), the instance's visible extrusions are made of {} ({:?})",
                            roles.len(),
                            roles.keys().collect::<Vec<_>>()
                        ));
                    }
                }
            }
        }
        for source in &entry.value_sources {
            let count = counts.entry(source.reader.clone()).or_default();
            count.0 += 1;
            count.1 += usize::from(source.differs_from_frame);
            *by_rule
                .entry(format!("{}:{}", source.reader, source.rule.as_str()))
                .or_default() += 1;
            let frame_partition = entry
                .stream
                .as_deref()
                .and_then(|stream| stream.strip_prefix("Partitions/"))
                .and_then(|number| number.parse::<u32>().ok());
            let has_place = source.stream.is_some() || !source.parts.is_empty();
            // A profile's sketch lines are held in the frame's partition too
            // when the Sketch's curve list has a copy there.
            let list_in_frame = entry.stream.as_ref().is_some_and(|frame| {
                entry.value_sources.iter().any(|other| {
                    other.reader == "sketch_list"
                        && (other.stream.as_ref() == Some(frame)
                            || frame_partition.is_some_and(|n| other.partitions.contains(&n)))
                })
            });
            let held = source.stream == entry.stream
                || entry
                    .stream
                    .as_ref()
                    .is_some_and(|frame| source.parts.iter().any(|part| &part.stream == frame))
                || frame_partition.is_some_and(|n| source.partitions.contains(&n))
                || (list_in_frame
                    && matches!(source.reader.as_str(), "sketch_lines" | "sketch_line_data"));
            let differs = entry.stream.is_some() && has_place && !held;
            if differs != source.differs_from_frame {
                problems.push(format!(
                    "{id}: {} differs_from_frame is {}, the streams say {differs}",
                    source.reader, source.differs_from_frame
                ));
            }
            checked += 1;
            let holds = if !source.parts.is_empty() {
                let mut all = true;
                // What the section and pipe parts say, checked as a set below.
                let mut parameters: Vec<i64> = Vec::new();
                let mut points: Vec<[f64; 3]> = Vec::new();
                let frame_box = if matches!(
                    source.reader.as_str(),
                    "room_solid_faces" | "pipe_connectors"
                ) {
                    frame_record(
                        &mut rf,
                        entry.stream.as_deref().zip(entry.offset),
                        id,
                        marker,
                        &declared,
                    )
                    .map(|record| record.bbox_feet)
                } else {
                    None
                };
                for part in &source.parts {
                    parts_checked += 1;
                    let inflated = rf
                        .inflated_partition(&part.stream)
                        .unwrap_or_else(|error| panic!("{name}: {}: {error}", part.stream));
                    let bytes = inflated.bytes();
                    let wanted = u64::from(part.element_id);
                    let verdict = match source.reader.as_str() {
                        // The line's element-data header, then the line's own
                        // id: the part's offset is a header, not a frame.
                        // A roof's slope block or sketch line is read again:
                        // it decodes to a slope, an arrow or a curve. A roof
                        // type's layers are an element-data header and the
                        // type's id.
                        "sketch_line_data" | "roof_slope_edges" | "roof_line_data"
                        | "roof_type_layers" => match layout.as_ref() {
                            Some(layout) => {
                                let window = match source.reader.as_str() {
                                    "roof_slope_edges" => EDGE_SLOPE_WINDOW,
                                    "roof_type_layers" => LAYER_WINDOW,
                                    _ => BEAM_DATA_WINDOW,
                                };
                                let read = element_data_at(
                                    bytes,
                                    layout,
                                    part.offset,
                                    part.element_id,
                                    window,
                                    false,
                                );
                                let decodes = match (source.reader.as_str(), read) {
                                    (_, None) => false,
                                    ("roof_slope_edges", Some(data)) => {
                                        edge_slope_in(data, version).is_some()
                                            || arrow_heights(data).is_some()
                                    }
                                    ("roof_line_data", Some(data)) => {
                                        first_bounded_line(data).is_some()
                                            || first_bounded_arc(data).is_some()
                                            || first_bounded_spline(data).is_some()
                                            || first_circle(data).is_some()
                                            || first_sketch_curve(data).is_some()
                                    }
                                    // The type's layers are read again by the layers
                                    // reader, with the reader's window and anchors.
                                    ("roof_type_layers", Some(data)) => {
                                        match (layer_frame_tag(version), layer_layout(version)) {
                                            (Some(tag), Some(layers)) => find_layers(
                                                data,
                                                tag,
                                                layers,
                                                &layer_materials,
                                                &declared,
                                            )
                                            .is_some(),
                                            _ => false,
                                        }
                                    }
                                    (_, Some(_)) => true,
                                };
                                if decodes {
                                    Part::Confirmed
                                } else {
                                    Part::Failed
                                }
                            }
                            None => Part::Unverified,
                        },
                        // A room's solid: its room's id, then the solid's tag
                        // directly or after `ff ff ff ff`; read again, the
                        // solid gives the outline the room holds.
                        "room_solid_faces" => {
                            let tag_at = part.offset + 8;
                            let tagged = bytes.get(tag_at..tag_at + 4) == Some(&ROOM_SOLID_TAG[..])
                                || (bytes.get(tag_at..tag_at + 4) == Some(&[0xff; 4][..])
                                    && bytes.get(tag_at + 4..tag_at + 8)
                                        == Some(&ROOM_SOLID_TAG[..]));
                            let held = rooms.get(&id);
                            match (
                                u64_at(bytes, part.offset) == Some(wanted) && tagged,
                                frame_box,
                            ) {
                                (false, _) => Part::Failed,
                                (true, Some(bbox)) => {
                                    let again = match held {
                                        Some((Some(profile), Some(vertices))) => {
                                            let outline = if profile == ROOM_OUTLINE_SOURCE {
                                                room_outline_at(bytes, part.offset, bbox)
                                            } else {
                                                resolved_outline_of_solids(
                                                    bytes,
                                                    &[part.offset],
                                                    bbox,
                                                )
                                                .filter(|(_, kind)| {
                                                    matches!(
                                                        (profile.as_str(), kind),
                                                        (
                                                            ROOM_OUTLINE_SOURCE_FACES,
                                                            ResolvedKind::Faces
                                                        ) | (
                                                            ROOM_OUTLINE_SOURCE_APPROXIMATE,
                                                            ResolvedKind::Approximate
                                                        )
                                                    )
                                                })
                                                .map(|(outline, _)| outline)
                                            };
                                            outline.is_some_and(|outline| {
                                                i64::try_from(outline.vertex_count())
                                                    == Ok(*vertices)
                                            })
                                        }
                                        _ => false,
                                    };
                                    if again { Part::Confirmed } else { Part::Failed }
                                }
                                (true, None) => Part::Unverified,
                            }
                        }
                        // A type's parameter entry, `f64 · ff × 8 · i64`, in
                        // the value block the type owns: the nearest block
                        // mark before it has the type's id before it.
                        "type_hollow_section" | "type_i_section" => {
                            let parameter = i64_at(bytes, part.offset + 16);
                            let shaped = bytes.get(part.offset + 8..part.offset + 16)
                                == Some(&[0xff; 8][..])
                                && parameter.is_some()
                                && f64_at(bytes, part.offset).is_some_and(f64::is_finite);
                            if let Some(parameter) = parameter {
                                parameters.push(parameter);
                            }
                            if shaped
                                && value_block_owner(bytes, part.offset, &declared)
                                    == Some(part.element_id)
                            {
                                Part::Confirmed
                            } else {
                                Part::Failed
                            }
                        }
                        // A pipe's connector entry, `u64 element · u32 index ·
                        // u32 1 · f64 × 3`: the element is the one at that end
                        // (the pipe or a neighbour), the point is the end.
                        "pipe_connectors" => {
                            let point: Option<Vec<f64>> = (0..3)
                                .map(|axis| f64_at(bytes, part.offset + 16 + 8 * axis))
                                .collect();
                            let shaped = u64_at(bytes, part.offset) == Some(wanted)
                                && u32_at(bytes, part.offset + 8).is_some_and(|index| index <= 255)
                                && u32_at(bytes, part.offset + 12) == Some(1);
                            match (shaped, point) {
                                (true, Some(point)) if point.len() == 3 => {
                                    points.push([point[0], point[1], point[2]]);
                                    Part::Confirmed
                                }
                                _ => Part::Failed,
                            }
                        }
                        // A record of a family's own document, as the native
                        // framing lays it out: the record's id, a length (after
                        // four bytes more on the channels that carry them), a
                        // body that opens with its class tag and the same
                        // length again. Its class is the one its role in the
                        // family's visible extrusions asks for: an extrusion,
                        // its sketch plane or a curve of its sketch.
                        "family_forms" => {
                            let first = if width == 8 {
                                u64_at(bytes, part.offset)
                            } else {
                                u32_at(bytes, part.offset).map(u64::from)
                            };
                            let body = [width + 4, width + 8].into_iter().find_map(|header| {
                                let length =
                                    usize::try_from(u32_at(bytes, part.offset + header - 4)?)
                                        .ok()?;
                                let start = part.offset + header;
                                let echo = u32_at(bytes, start.checked_add(length)?)?;
                                (usize::try_from(echo).ok()? == length).then_some(start)
                            });
                            let kind = body
                                .and_then(|start| u16_at(bytes, start))
                                .and_then(|tag| classes.as_ref()?.by_tag(tag))
                                .map(|class| class.name.as_str());
                            let role = family_roles
                                .get(&u64::from(id))
                                .and_then(|roles| roles.get(&wanted))
                                .copied();
                            if first == Some(wanted) && role.is_some() && kind == role {
                                Part::Confirmed
                            } else {
                                Part::Failed
                            }
                        }
                        // A run type's id object: its id is the u64 there.
                        "stair_run_type" => {
                            if u64_at(bytes, part.offset) == Some(wanted) {
                                Part::Confirmed
                            } else {
                                Part::Failed
                            }
                        }
                        // A frame: the element's id at its first bytes, or on
                        // a second-prologue frame the no-id marker there and
                        // the element's id at the head of the record that
                        // encloses the frame (RE-35), found from the bytes.
                        _ => {
                            let first = if width == 8 {
                                u64_at(bytes, part.offset)
                            } else {
                                u32_at(bytes, part.offset).map(u64::from)
                            };
                            if first == Some(wanted) {
                                Part::Confirmed
                            } else if width == 8 && first.is_some_and(carries_no_element_id) {
                                match marker {
                                    Some(marker) => {
                                        let chain =
                                            chains.entry(part.stream.clone()).or_insert_with(
                                                || partition_record_chain(bytes, &marker),
                                            );
                                        if enclosing_record(chain, part.offset)
                                            .is_some_and(|span| span.element_id == wanted)
                                        {
                                            parts_by_record += 1;
                                            Part::Confirmed
                                        } else {
                                            Part::Failed
                                        }
                                    }
                                    None => Part::Unverified,
                                }
                            } else if width == 4 {
                                // A second-prologue frame of a 32-bit release
                                // carries its id elsewhere; not read here.
                                Part::Unverified
                            } else {
                                Part::Failed
                            }
                        }
                    };
                    match verdict {
                        Part::Confirmed => parts_confirmed += 1,
                        Part::Unverified => parts_unverified += 1,
                        Part::Failed => {
                            all = false;
                            problems.push(format!(
                                "{id}: {} part {} at {}:{} does not hold the element",
                                source.reader, part.element_id, part.stream, part.offset
                            ));
                        }
                    }
                }
                // The parts of one source are distinct records, and a section
                // or a pipe has the ones it is made of, not a sibling entry.
                let places: BTreeSet<(&str, usize)> = source
                    .parts
                    .iter()
                    .map(|part| (part.stream.as_str(), part.offset))
                    .collect();
                let mut sound = places.len() == source.parts.len();
                match source.reader.as_str() {
                    "type_i_section" => {
                        let mut found = parameters.clone();
                        found.sort_unstable();
                        let mut wanted = I_SECTION_PARAMETERS.to_vec();
                        wanted.sort_unstable();
                        sound &= found == wanted;
                    }
                    "type_hollow_section" => {
                        sound &= parameters.len() == HOLLOW_SECTION_PARAMETER_NAMES.len();
                        hollow_checks.push((id, parameters.clone()));
                    }
                    "pipe_connectors" => {
                        sound &= match (points.as_slice(), frame_box) {
                            ([start, end], Some(bbox)) => pipe_body(bbox, *start, *end).is_some(),
                            ([_, _], None) => {
                                parts_unverified += 1;
                                true
                            }
                            _ => false,
                        };
                    }
                    _ => {}
                }
                if !sound {
                    all = false;
                    problems.push(format!(
                        "{id}: {} is not made of the records it should be",
                        source.reader
                    ));
                }
                all
            } else {
                match source.rule.as_str() {
                    "selected" => source.stream == entry.stream && source.offset == entry.offset,
                    "agreeing" | "current" => {
                        let (Some(stream), Some(offset), Some(layout)) =
                            (source.stream.as_deref(), source.offset, layout.as_ref())
                        else {
                            problems.push(format!("{id}: {} has no place", source.reader));
                            continue;
                        };
                        let inflated = rf
                            .inflated_partition(stream)
                            .unwrap_or_else(|error| panic!("{name}: {stream}: {error}"));
                        let from_data = source.reader == "instance_transform";
                        let window = if from_data {
                            INSTANCE_TRANSFORM_WINDOW
                        } else if source.reader == "roof_fascia_depth" {
                            EDGE_SLOPE_WINDOW
                        } else {
                            BEAM_DATA_WINDOW
                        };
                        if source.reader == "sketch_list" {
                            // The Sketch's element-data header; the sketched
                            // element's id is at a fixed place after it.
                            let bytes = inflated.bytes();
                            bytes.get(offset..offset + layout.header.len())
                                == Some(&layout.header[..])
                                && u64_at(bytes, offset + SKETCH_OWNER_OFFSET)
                                    == Some(u64::from(id))
                        } else {
                            match element_data_at(
                                inflated.bytes(),
                                layout,
                                offset,
                                id,
                                window,
                                from_data,
                            ) {
                                Some(data) => {
                                    let mut decodes = true;
                                    match source.reader.as_str() {
                                        // The roof's fascia depth, read again.
                                        "roof_fascia_depth" => {
                                            decodes = fascia_depth(data).is_some();
                                        }
                                        "bounded_line" => {
                                            decoded_lines.insert(id, first_bounded_line(data));
                                        }
                                        "bounded_arc" => {
                                            decoded_arcs.insert(id, first_bounded_arc(data));
                                        }
                                        "instance_transform" => {
                                            decoded_transforms.insert(id, instance_transform(data));
                                        }
                                        _ => {}
                                    }
                                    decodes
                                }
                                None => false,
                            }
                        }
                    }
                    "native" => {
                        let (Some(stream), Some(offset)) =
                            (source.stream.as_deref(), source.offset)
                        else {
                            problems.push(format!("{id}: {} has no place", source.reader));
                            continue;
                        };
                        let inflated = rf
                            .inflated_partition(stream)
                            .unwrap_or_else(|error| panic!("{name}: {stream}: {error}"));
                        let first = if width == 8 {
                            u64_at(inflated.bytes(), offset)
                        } else {
                            u32_at(inflated.bytes(), offset).map(u64::from)
                        };
                        if source.reader == "roof_rafter_cut" {
                            rafter_roofs.insert(id);
                        }
                        first == Some(u64::from(id))
                    }
                    _ => false,
                }
            };
            if holds {
                confirmed += 1;
            } else if problems.len() < 40 {
                problems.push(format!(
                    "{id}: {} ({}) at {:?}:{:?} does not hold the element",
                    source.reader,
                    source.rule.as_str(),
                    source.stream,
                    source.offset
                ));
            }
        }
    }

    // A roof's rafter cut is read again by the reader, from its own record.
    if !rafter_roofs.is_empty() {
        let cuts = scan_rafter_cuts(&mut rf, version, &rafter_roofs);
        for roof in &rafter_roofs {
            if !cuts.contains_key(roof) {
                problems.push(format!(
                    "{roof}: roof_rafter_cut: the reader gives no rafter cut from the roof's record"
                ));
            }
        }
    }

    // The four parameters of a hollow section are the ones captioned b, Ht, t
    // and kr, by the ParamElemFamily elements the parameter ids name.
    let hollow_ids: BTreeSet<u64> = hollow_checks
        .iter()
        .flat_map(|(_, parameters)| parameters.iter())
        .filter_map(|parameter| u64::try_from(*parameter).ok())
        .collect();
    if !hollow_ids.is_empty() {
        let mut captions: BTreeMap<i64, String> = BTreeMap::new();
        let options = Options {
            selected_ids: hollow_ids,
            ..Options::default()
        };
        extract_graphs(&mut rf, &options, |record| {
            let (Some("ParamElemFamily"), Some(graph)) =
                (record.class_name.as_deref(), record.graph.as_ref())
            else {
                return Ok(());
            };
            let caption = graph
                .objects
                .iter()
                .find(|object| object.class_name == "ParamDefValue")
                .and_then(|object| object.fields.get("m_caption")?.as_str());
            if let (Some(caption), Ok(id)) = (caption, i64::try_from(record.identity.element_id)) {
                captions.insert(id, caption.to_string());
            }
            Ok(())
        })
        .expect("read the parameter captions");
        let mut wanted: Vec<&str> = HOLLOW_SECTION_PARAMETER_NAMES.to_vec();
        wanted.sort_unstable();
        for (id, parameters) in &hollow_checks {
            let mut found: Vec<&str> = parameters
                .iter()
                .filter_map(|parameter| captions.get(parameter).map(String::as_str))
                .collect();
            found.sort_unstable();
            if found != wanted {
                problems.push(format!(
                    "{id}: type_hollow_section is made of the parameters {found:?}, not {wanted:?}"
                ));
            }
        }
    }

    // What the readers return when asked again for the same elements.
    let line_ids: BTreeSet<u32> = decoded_lines.keys().copied().collect();
    let lines = scan_first_bounded_lines(&mut rf, version, &line_ids).expect("scan the lines");
    for (id, at) in &decoded_lines {
        if at.as_ref() != lines.get(id) {
            problems.push(format!("{id}: the line at the offset is not the reader's"));
        }
    }
    let arc_ids: BTreeSet<u32> = decoded_arcs.keys().copied().collect();
    let arcs = scan_first_bounded_arcs(&mut rf, version, &arc_ids).expect("scan the arcs");
    for (id, at) in &decoded_arcs {
        if at.as_ref() != arcs.get(id) {
            problems.push(format!("{id}: the arc at the offset is not the reader's"));
        }
    }
    let transform_ids: BTreeSet<u32> = decoded_transforms.keys().copied().collect();
    let transforms = scan_instance_transforms(&mut rf, version, &transform_ids)
        .expect("scan the instance transforms");
    for (id, at) in &decoded_transforms {
        if at.as_ref() != transforms.get(id) {
            problems.push(format!(
                "{id}: the transform at the offset is not the reader's"
            ));
        }
    }

    // The table's counts are the ones made here.
    for (reader, count) in &table.value_source_counts {
        if counts.get(reader) != Some(&(count.elements, count.differ_from_frame)) {
            problems.push(format!("{reader}: the table's count is not the one made"));
        }
    }
    if counts.len() != table.value_source_counts.len() {
        problems.push("value_source_counts lacks a reader that has sources".to_string());
    }
    let pins = if family_extrusions {
        FAMILY_MIN_READERS
    } else {
        MIN_READERS
    };
    for (model, readers) in pins {
        if *model != base {
            continue;
        }
        for reader in *readers {
            let held = counts.get(*reader).map_or(0, |count| count.0)
                + if *reader == "selected_frame" {
                    counts.get("selected_frame_box").map_or(0, |count| count.0)
                } else {
                    0
                };
            if held == 0 {
                problems.push(format!("{name}: no element has a source from {reader}"));
            }
        }
    }

    // The report.
    let mut line = format!(
        "{name}: Revit {version}, {entities} entities with a BodySource, {} elements ({} entities share an ElementId with another), {} value sources on {} elements, {confirmed} of {checked} re-read and confirmed\n",
        listed_bodies.len(),
        entities - listed_bodies.len(),
        counts.values().map(|count| count.0).sum::<usize>(),
        table
            .elements
            .values()
            .filter(|entry| !entry.value_sources.is_empty())
            .count()
    );
    line.push_str(&format!(
        "  source parts: {parts_confirmed} of {parts_checked} records re-read and confirmed ({parts_by_record} second-prologue frames, by the record that encloses them), {parts_unverified} not read\n"
    ));
    for (reader, (elements, differ)) in &counts {
        line.push_str(&format!(
            "  {reader}: {elements} elements, {differ} in another partition than their frame\n"
        ));
    }
    let mut bodies = serde_json::Map::new();
    for (body, stats) in &by_body {
        let covered = COVERAGE.iter().find(|(source, _)| source == body).map_or(
            "not in COVERAGE",
            |(_, coverage)| match coverage {
                Recorded { .. } => "recorded",
                NotYet => "not yet",
            },
        );
        line.push_str(&format!(
            "  {body} ({covered}): {} elements, {} with a source ({} box only; {} of {} profiles sourced), {} not in the table\n",
            stats.elements,
            stats.with_source,
            stats.box_only,
            stats.profile_sourced,
            stats.profiled,
            stats.unlisted
        ));
        bodies.insert(
            body.clone(),
            json!({
                "coverage": covered,
                "elements": stats.elements,
                "not_in_table": stats.unlisted,
                "with_source": stats.with_source,
                "box_only": stats.box_only,
                "by_reader": stats.by_reader,
            }),
        );
    }
    let report = json!({
        "model": name,
        "layout_release": version,
        "entities_with_a_body_source": entities,
        "elements_with_a_body_source": listed_bodies.len(),
        "distinct_element_ids": distinct_ids,
        "source_parts_checked": parts_checked,
        "source_parts_confirmed": parts_confirmed,
        "source_parts_by_enclosing_record": parts_by_record,
        "source_parts_unverified": parts_unverified,
        "value_sources_checked": checked,
        "value_sources_confirmed": confirmed,
        "by_reader": counts
            .iter()
            .map(|(reader, (elements, differ))| {
                (reader.clone(), json!({ "elements": elements, "differ_from_frame": differ }))
            })
            .collect::<serde_json::Map<_, _>>(),
        "by_reader_and_rule": by_rule,
        "by_body_source": bodies,
        "problems": problems,
    });
    let out = Path::new(env!("CARGO_TARGET_TMPDIR")).join("value-sources");
    std::fs::create_dir_all(&out).expect("report folder");
    std::fs::write(
        out.join(format!("{name}.json")),
        serde_json::to_vec_pretty(&report).expect("report json"),
    )
    .expect("write the report");
    eprint!("{line}");
    std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(out.join("summary.txt"))
        .and_then(|mut summary| summary.write_all(line.as_bytes()))
        .expect("write the summary");

    assert!(problems.is_empty(), "{name}: {problems:#?}");
    counts.keys().cloned().collect()
}

/// Check each model. With `dir_variable` set, a model of `must_hold` that the
/// corpus lacks fails the test, whether or not any other model is there.
fn check_all(dir_variable: &str, models: Vec<(String, PathBuf)>, must_hold: &[&str]) {
    if std::env::var_os(dir_variable).is_some() {
        for model in must_hold {
            assert!(
                models.iter().any(|(name, _)| name == model),
                "{model} is not in the corpus, and {dir_variable} is set"
            );
        }
    }
    if models.is_empty() {
        eprintln!("skipping: no model to read");
        return;
    }
    for (name, path) in &models {
        check_model(name, path);
    }
}

#[test]
fn project_corpus_value_sources_name_the_copy_the_reader_took() {
    check_all(
        "RVT_PROJECT_CORPUS_DIR",
        project_models(),
        &[
            "2024_Core_Interior",
            "RE1-Architecture",
            "Revit_IFC5_Einhoven",
        ],
    );
}

#[test]
fn nordic_office_value_sources_name_the_copy_the_reader_took() {
    check_all(
        "RVT_NORDIC_LCA_DIR",
        office_model(),
        &["nordic-office-as-built"],
    );
}

#[test]
fn r2025_value_sources_name_the_copy_the_reader_took() {
    let models = r2025_models();
    // Skipped only where the variable is unset; set, an empty or missing
    // directory fails below, as the pinned models are absent from it.
    if std::env::var_os("RVT_R2025_DIR").is_none() {
        return;
    }
    // The models MIN_READERS pins are in the set, or those pins read nothing.
    for pinned in ["02_1Slope", "05_1Slope_ArrowDefined", "11_RoofByExtrusion"] {
        assert!(
            models.iter().any(|(name, _)| name == pinned),
            "{pinned} is not among the Revit 2025 models, and RVT_R2025_DIR is set"
        );
    }
    let mut held: BTreeSet<String> = BTreeSet::new();
    for (name, path) in &models {
        held.extend(check_model(name, path));
    }
    for reader in R2025_READERS {
        assert!(
            held.contains(*reader),
            "no Revit 2025 model has a source from {reader}"
        );
    }
}

#[test]
fn family_extrusion_value_sources_name_the_forms_the_body_was_rebuilt_from() {
    let mut models = project_models();
    let office = office_model();
    // A pinned model that the corpus lacks fails the test where its directory
    // variable is set, as `check_all` does; unset, that directory is skipped.
    for (model, _) in FAMILY_MIN_READERS {
        let (variable, held) = if *model == "nordic-office-as-built" {
            ("RVT_NORDIC_LCA_DIR", &office)
        } else {
            ("RVT_PROJECT_CORPUS_DIR", &models)
        };
        if std::env::var_os(variable).is_some() {
            assert!(
                held.iter().any(|(name, _)| name == model),
                "{model} is not in the corpus, and {variable} is set"
            );
        }
    }
    models.retain(|(name, _)| FAMILY_MIN_READERS.iter().any(|(model, _)| model == name));
    models.extend(office);
    if models.is_empty() {
        eprintln!("skipping: no model to read");
        return;
    }
    for (name, path) in &models {
        check_model_with(name, path, true);
    }
}
