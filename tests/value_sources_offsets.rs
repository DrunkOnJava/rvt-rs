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
//!   element-data header followed by the line's own id; a run type is an id
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
use rvt::RevitFile;
use rvt::ifc::entities::{IfcEntity, PropertyValue};
use rvt::ifc::{ExportOptions, IfcModel, run_export};
use rvt::research::element_record_plan_profiles::SKETCH_OWNER_OFFSET;
use rvt::research::partition_beam_axes::{
    BEAM_DATA_WINDOW, BoundedArc, BoundedLine, first_bounded_arc, first_bounded_line,
    scan_first_bounded_arcs, scan_first_bounded_lines,
};
use rvt::research::partition_element_records::{
    PartitionRecordSpan, carries_no_element_id, enclosing_record, file_bbox_marker,
    partition_record_chain,
};
use rvt::research::partition_instance_transforms::{
    INSTANCE_TRANSFORM_WINDOW, InstanceTransform, instance_transform, scan_instance_transforms,
};
use rvt::research::partition_names::{ElementDataLayout, element_data_layout};
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
    ("family_type_hollow_section", NotYet),
    ("family_type_i_section", NotYet),
    ("partition_pipe_axis", NotYet),
    ("partition_roof_slope", NotYet),
    ("partition_roof_extrusion", NotYet),
    ("family_extrusions", NotYet),
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
        ],
    ),
    (
        "RE1-Architecture",
        &["selected_frame", "bounded_line", "saved_graphics"],
    ),
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
        ],
    ),
];

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
        for property in property_set.iter().flat_map(|set| &set.properties) {
            match (property.name.as_str(), &property.value) {
                ("BodySource", PropertyValue::Text(text)) => body = Some(text.clone()),
                ("ProfileSource", PropertyValue::Text(text)) => profile_source = Some(text.clone()),
                ("ProfileResolved", PropertyValue::Boolean(true)) => profile_resolved = true,
                _ => {}
            }
        }
        if let Some(body) = body {
            entities += 1;
            let elsewhere = profile_resolved && profile_source.as_ref() != Some(&body);
            bodies.entry((id, body)).or_insert(BodyInfo {
                profile_elsewhere: elsewhere,
                profile_source,
            });
        }
    }
    (bodies, entities)
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

fn check_model(name: &str, path: &Path) {
    let mut rf = RevitFile::open(path).expect("open the model");
    let run = run_export(
        &mut rf,
        &ExportOptions::default().with_element_provenance(true),
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
                                _ => &[],
                            };
                            let profile_sourced = entry.value_sources.iter().any(|held| {
                                matches!(held.reader.as_str(), "sketch_lines" | "type_section")
                            });
                            let expected = if profile_elsewhere && !profile_sourced {
                                "selected_frame_box"
                            } else {
                                "selected_frame"
                            };
                            if source.reader != expected {
                                problems.push(format!(
                                    "{id}: {body} has {}, and its profile {} another record",
                                    source.reader,
                                    if profile_elsewhere {
                                        "is read from"
                                    } else {
                                        "is not read from"
                                    }
                                ));
                            }
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
    let mut parts_checked = 0usize;
    let mut parts_confirmed = 0usize;
    let mut parts_by_record = 0usize;
    let mut parts_unverified = 0usize;
    let mut counts: BTreeMap<String, (usize, usize)> = BTreeMap::new();
    let mut by_rule: BTreeMap<String, usize> = BTreeMap::new();
    let width = usize::from(table.record_id_bytes.unwrap_or(8));
    for (&id, entry) in &table.elements {
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
                        "sketch_line_data" => match layout.as_ref() {
                            Some(layout) => {
                                let read = element_data_at(
                                    bytes,
                                    layout,
                                    part.offset,
                                    part.element_id,
                                    BEAM_DATA_WINDOW,
                                    false,
                                );
                                if read.is_some() {
                                    Part::Confirmed
                                } else {
                                    Part::Failed
                                }
                            }
                            None => Part::Unverified,
                        },
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
                                    match source.reader.as_str() {
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
                                    true
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
    for (model, readers) in MIN_READERS {
        if *model != name {
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
