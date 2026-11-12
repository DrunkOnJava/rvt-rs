//! RE-363 (#1496): what separates RE1 Plumbing's water closet tank 442378, which
//! rvt-rs writes and Revit's export does not hold, from the plumbing fixtures
//! Revit's export does hold.
//!
//! FACT under test (to be measured): Revit's exporter writes a family instance
//! that is not a door or a window only when its 3D body has a solid or a mesh
//! (`FamilyInstanceExporter.ExportFamilyInstanceAsMappedItem` in revit-ifc,
//! CLEANROOM.md source 5, returns with nothing written when it has none, and
//! `ExportFamilyInstanceElement` returns on a null geometry or an invisible
//! instance; `GeometryUtil.GetIFCExportGeometryOptions` asks for the Fine detail
//! level, native level 3). rvt-rs writes a fixture whenever its record box has a
//! volume (RE-21). This probe prints, for every `OST_PlumbingFixtures` element a
//! file holds, the readings that candidate rules would use, so
//! `tools/re/tank_rules_vs_ifc.py` can say how many of Revit's own fixtures each
//! rule would drop:
//!
//! - the record layer: stage (the RE-350 stages, `written` where rvt-rs writes
//!   the element), container slot, placement kind, design option, box extents,
//!   and the mutual-reference parents of RE-79's nested-component rule (same
//!   category and any category);
//! - the native record of each written fixture, of its type symbol and of the
//!   symbol's family: every scalar of the root object (so a super-instance, host,
//!   work plane or visibility field is read where the schema names it) and the
//!   class census of the graph;
//! - the family document's records (class census) and the model forms of the
//!   family's own document with the RE-312 verdict
//!   (`native_family_forms::instances_without_body`);
//! - the saved graphics at the exporter's detail level, as RE-329 reads them:
//!   outcome, primitive counts at levels 1 to 3, how many are hidden, and the
//!   render styles and materials of the primitives.
//!
//! Output (one line each; no count is a verdict, the scorer joins them with
//! Revit's Tags):
//!   release saved=<n> layout=<n>
//!   fixture id=<id> category=<c> stage=<stage> frames=<n> container=<..> kind=<..>
//!       option=<..> extents=<dx,dy,dz> enclosing=<0|1> refs=<n> refs_list=<id:cat,..>
//!       mutual_same=<ids|-> mutual_any=<id:cat,..|->
//!   native <scope> <id> <class> objects <n> | native <scope> <id> <class> no_graph <why>
//!   objects <scope> <id> <Class:n,..>
//!   field <scope> <id> <key> <value>
//!   forms id=<id> symbol=<s> family=<f> doc=<found|missing> forms=<n> cutting=<n>
//!       invisible=<n> bound=<n> incomplete=<0|1> verdict=<..>
//!   famdoc family=<f> records=<n> classes=<Class:n,..>
//!   graphics id=<id> outcome=<..> prim=<l1>/<l2>/<l3> hidden=<n> visible=<n> diag=<n>
//!       blocking=<n> kinds=<..> vis_branches=<n> filters=<n> nonsurface=<n>
//!       unbounded=<n> styles=<style:n,..> materials=<material:n,..>
//! where `<scope>` is `instance`, `symbol` or `family`.
//!
//! ```bash
//! cargo run --profile ci -p rvt-probes --example probe_re363_plumbing_fixtures -- model.rvt
//! ```

use rvt::RevitFile;
use rvt::research::graph_value::Fv;
use rvt::research::native_document::{NATIVE_RELEASES, Options, Record, extract_graphs};
use rvt::research::native_family_forms::{family_records, family_solids, instances_without_body};
use rvt::research::native_graphics_traversal::{
    NESTED_DETAIL_FILTER_DIAGNOSTIC, VIEW_COMPONENT_DIAGNOSTIC,
};
use rvt::research::native_parameters::ObjectGraph;
use rvt::research::native_saved_mesh::{
    graphics_including_hidden, graphics_with_resolver_at_detail,
};
use rvt::research::partition_element_records::{self as per, PartitionElementRecord};
use rvt::research::partition_element_records_2023 as per32;
use std::collections::{BTreeMap, BTreeSet};

/// The most family documents whose records are counted by class: a document is
/// read whole for it.
const MAX_FAMILY_DOCUMENTS: usize = 60;

/// The most references of a record printed with their categories.
const MAX_REFERENCES: usize = 24;

fn i64_at(buf: &[u8], at: usize) -> Option<i64> {
    let end = at.checked_add(8)?;
    buf.get(at..end)
        .map(|b| i64::from_le_bytes(b.try_into().expect("8 bytes")))
}

/// Every frame of the file's marker in the `BuiltInCategory` band, decoded
/// for any category: a 2024 to 2027 file's frames with a declared id or an
/// enclosing record's (as `probe_re350_category_stages`).
fn records_2024(rf: &mut RevitFile, release: u32) -> Vec<PartitionElementRecord> {
    let Some(marker) = per::file_bbox_marker(rf, release) else {
        println!("the release gives no record marker");
        return Vec::new();
    };
    let Ok(table) = rvt::elem_table::parse_records(rf) else {
        return Vec::new();
    };
    let declared = rvt::elem_table::declared_ids(&table);
    let assigned = per::compute_second_prologue_ids(rf);
    let none = BTreeMap::new();
    let mut out = Vec::new();
    for stream in rf.partition_stream_names() {
        let Ok(inflated) = rf.inflated_partition(&stream) else {
            continue;
        };
        let buf = inflated.bytes();
        let mut categories: BTreeSet<i64> = BTreeSet::new();
        for hit in memchr::memmem::find_iter(buf, &marker) {
            let Some(start) = hit.checked_sub(per::BBOX_MARKER_OFFSET) else {
                continue;
            };
            if let Some(category) = i64_at(buf, start + per::CATEGORY_OFFSET) {
                if (per::BUILTIN_CATEGORY_MIN..=per::BUILTIN_CATEGORY_MAX).contains(&category) {
                    categories.insert(category);
                }
            }
        }
        let list: Vec<i64> = categories.into_iter().collect();
        for records in per::find_categories_records_assigned(
            &stream,
            buf,
            &list,
            &declared,
            &marker,
            assigned.get(&stream).unwrap_or(&none),
        ) {
            out.extend(records);
        }
    }
    out
}

/// Why a placed instance has no 3D body: how many of its three extents are not
/// above the volume epsilon, or that one is not a number.
fn no_body_reason(record: &PartitionElementRecord) -> &'static str {
    let (dx, dy, dz) = record.extents_feet();
    let extents = [dx, dy, dz];
    if extents.iter().any(|extent| !extent.is_finite()) {
        return "no_body_not_finite";
    }
    let flat = extents
        .iter()
        .filter(|extent| **extent <= per::VOLUME_EPSILON_FEET)
        .count();
    match flat {
        1 => "no_body_flat",
        2 => "no_body_line",
        _ => "no_body_point",
    }
}

/// `{"m_id":{"m_id64":n}}`, or a bare integer, as `n`.
fn identifier(value: &Fv) -> Option<i64> {
    if let Some(n) = value.as_i64() {
        return Some(n);
    }
    for key in ["m_id", "m_id64"] {
        if let Some(inner) = value.get(key) {
            return identifier(inner);
        }
    }
    None
}

/// The element a root field names, where it names one.
fn root_id(graph: &ObjectGraph, field: &str) -> Option<u64> {
    let root = graph.objects.first()?;
    let id = identifier(root.fields.get(field)?)?;
    u64::try_from(id).ok().filter(|id| *id > 0)
}

/// `text` as one word of at most 60 characters.
fn tidy(text: &str) -> String {
    text.chars()
        .take(60)
        .map(|c| if c.is_whitespace() { '_' } else { c })
        .collect()
}

/// A message with its numbers and digits made `#`, so equal kinds compare equal.
fn message_kind(message: &str) -> String {
    let text = match message.strip_prefix("object ") {
        Some(rest) => rest.split_once(": ").map_or(rest, |(_, kind)| kind),
        None => message,
    };
    let words: Vec<String> = text
        .split_whitespace()
        .map(|word| {
            word.chars()
                .map(|c| if c.is_ascii_digit() { '#' } else { c })
                .collect()
        })
        .collect();
    tidy(&words.join("_"))
}

fn ignorable(diagnostic: &str) -> bool {
    diagnostic.ends_with(NESTED_DETAIL_FILTER_DIAGNOSTIC)
        || diagnostic.ends_with(VIEW_COMPONENT_DIAGNOSTIC)
}

/// Every scalar under `value`, as (path, text): a pointer as `ptrnull` or
/// `ptrset`, an array as its length and, short, its items.
fn flatten(prefix: &str, value: &Fv, depth: usize, out: &mut Vec<(String, String)>) {
    match value {
        Fv::Null => {}
        Fv::Bool(b) => out.push((prefix.to_string(), b.to_string())),
        Fv::Number(n) => out.push((prefix.to_string(), n.to_string())),
        Fv::String(text) => out.push((prefix.to_string(), tidy(text))),
        Fv::Array(items) => {
            out.push((format!("{prefix}[]"), items.len().to_string()));
            if depth < 3 && items.len() <= 8 {
                for (index, item) in items.iter().enumerate() {
                    flatten(&format!("{prefix}[{index}]"), item, depth + 1, out);
                }
            }
        }
        Fv::Object(members) => {
            if let Some(token) = members.get("pointer_token") {
                let state = if token.as_u64() == Some(0) {
                    "ptrnull"
                } else {
                    "ptrset"
                };
                out.push((prefix.to_string(), state.to_string()));
                return;
            }
            if depth >= 4 {
                return;
            }
            for (name, member) in members {
                let key = if prefix.is_empty() {
                    name.clone()
                } else {
                    format!("{prefix}.{name}")
                };
                flatten(&key, member, depth + 1, out);
            }
        }
    }
}

/// `Class:n` of the objects of a graph, by class name.
fn census(graph: &ObjectGraph) -> String {
    let mut counts: BTreeMap<&str, usize> = BTreeMap::new();
    for object in &graph.objects {
        *counts.entry(object.class_name.as_str()).or_default() += 1;
    }
    counts
        .iter()
        .map(|(class, n)| format!("{class}:{n}"))
        .collect::<Vec<_>>()
        .join(",")
}

fn print_native(scope: &str, record: &Record) {
    let id = record.identity.element_id;
    let class = record.class_name.as_deref().unwrap_or("-");
    let Some(graph) = record.graph.as_ref() else {
        println!(
            "native {scope} {id} {class} no_graph {}",
            tidy(record.diagnostic.as_deref().unwrap_or("-"))
        );
        return;
    };
    println!(
        "native {scope} {id} {class} objects {}",
        graph.objects.len()
    );
    println!("objects {scope} {id} {}", census(graph));
    if let Some(root) = graph.objects.first() {
        let mut flat = Vec::new();
        flatten("", &root.fields, 0, &mut flat);
        for (key, value) in flat {
            println!("field {scope} {id} {key} {value}");
        }
    }
}

fn native_options(ids: BTreeSet<u64>) -> Options {
    Options {
        selected_ids: ids,
        releases: NATIVE_RELEASES,
        ..Options::default()
    }
}

/// The first eight entries of `counts` as `key:count`, joined, or `-`.
fn counted<K: std::fmt::Display>(counts: &BTreeMap<K, usize>) -> String {
    if counts.is_empty() {
        return "-".to_string();
    }
    counts
        .iter()
        .take(8)
        .map(|(key, n)| format!("{key}:{n}"))
        .collect::<Vec<_>>()
        .join(",")
}

/// The saved graphics of `id` at the Fine detail level, as `probe_re329`.
fn graphics_line(id: u64, records: &BTreeMap<u64, Record>) -> String {
    let Some(record) = records.get(&id) else {
        return format!("graphics id={id} outcome=no_record");
    };
    let Some(graph) = record.graph.as_ref() else {
        return format!(
            "graphics id={id} outcome=no_graph why={}",
            message_kind(
                record
                    .diagnostic
                    .as_deref()
                    .unwrap_or("graphics graph unavailable")
            )
        );
    };
    let resolver = |rid: u64| records.get(&rid).and_then(|r| r.graph.as_ref());
    let levels: Vec<usize> = (1..=3)
        .map(|level| {
            graphics_with_resolver_at_detail(graph, &resolver, level)
                .primitives
                .len()
        })
        .collect();
    let meshes = graphics_with_resolver_at_detail(graph, &resolver, 3);
    let all = graphics_including_hidden(graph, &resolver, 3);
    let hidden = all.primitives.iter().filter(|p| p.hidden).count();
    let visible = all.primitives.len() - hidden;
    let blocking = meshes.diagnostics.iter().any(|d| !ignorable(d));
    let outcome = match (meshes.primitives.is_empty(), blocking) {
        (false, false) => "drawn",
        (false, true) => "box_partial",
        (true, true) => "box_no_surface",
        (true, false) if !meshes.diagnostics.is_empty() => "empty_view_only",
        (true, false) => "empty_none",
    };
    let kinds: BTreeSet<String> = meshes
        .diagnostics
        .iter()
        .filter(|d| !ignorable(d))
        .map(|d| message_kind(d))
        .collect();
    let mut styles: BTreeMap<i64, usize> = BTreeMap::new();
    let mut materials: BTreeMap<String, usize> = BTreeMap::new();
    for primitive in &meshes.primitives {
        *styles.entry(primitive.render_style_id).or_default() += 1;
        let material = primitive
            .material_id
            .map_or_else(|| "none".to_string(), |m| m.to_string());
        *materials.entry(material).or_default() += 1;
    }
    format!(
        "graphics id={id} outcome={outcome} prim={}/{}/{} hidden={hidden} visible={visible} diag={} blocking={} kinds={} vis_branches={} filters={} nonsurface={} unbounded={} styles={} materials={}",
        levels[0],
        levels[1],
        levels[2],
        meshes.diagnostics.len(),
        meshes.diagnostics.iter().filter(|d| !ignorable(d)).count(),
        if kinds.is_empty() {
            "-".to_string()
        } else {
            kinds.into_iter().take(4).collect::<Vec<_>>().join(";")
        },
        meshes.excluded_visibility_branches,
        meshes.rejected_filters,
        meshes.excluded_non_surface_branches,
        meshes.unbounded_faces,
        counted(&styles),
        counted(&materials),
    )
}

fn main() -> anyhow::Result<()> {
    let path = std::env::args()
        .skip(1)
        .find(|arg| !arg.starts_with("--"))
        .ok_or_else(|| anyhow::anyhow!("usage: probe_re363_plumbing_fixtures <file.rvt>"))?;
    let mut rf = RevitFile::open_with_limits(&path, rvt::cli::open_limits(None, None, true))?;
    let saved = rf.basic_file_info()?.version;
    let release = rf.layout_release()?;
    println!("release saved={saved} layout={release}");
    let records: Vec<PartitionElementRecord> = if per32::is_32bit_release(release) {
        per32::scan_records(&mut rf, release)
    } else if per::supports_revit_version(release) {
        records_2024(&mut rf, release)
    } else {
        println!("a release whose records are not read: nothing to measure");
        return Ok(());
    };
    let filtered = per32::applies_export_filters(release) || per::supports_revit_version(release);
    let (phases, design) = if filtered {
        (Some(rf.phase_filter()), Some(rf.design_options()))
    } else {
        (None, None)
    };
    let stage_of = |record: &PartitionElementRecord| -> &'static str {
        if !record.is_exported_instance() {
            if record.is_container_member() {
                "container"
            } else if record.is_type_symbol() {
                "symbol"
            } else {
                "other_kind"
            }
        } else if !record.has_volume() {
            no_body_reason(record)
        } else if design.as_ref().is_some_and(|o| o.excludes(record)) {
            "design_option"
        } else if phases.as_ref().is_some_and(|p| p.excludes(record)) {
            "phase"
        } else {
            "written"
        }
    };

    let mut refs_of: BTreeMap<u32, BTreeSet<u64>> = BTreeMap::new();
    let mut category_of: BTreeMap<u32, i64> = BTreeMap::new();
    for record in &records {
        refs_of
            .entry(record.element_id)
            .or_default()
            .extend(record.references.iter().copied());
        category_of
            .entry(record.element_id)
            .or_insert(record.builtin_category);
    }
    let mut fixtures: BTreeMap<u32, Vec<&PartitionElementRecord>> = BTreeMap::new();
    for record in &records {
        if record.builtin_category == per::OST_PLUMBING_FIXTURES {
            fixtures.entry(record.element_id).or_default().push(record);
        }
    }
    println!("fixtures {}", fixtures.len());

    let mut written: BTreeSet<u64> = BTreeSet::new();
    for (id, frames) in &fixtures {
        let stages: BTreeSet<&'static str> = frames.iter().map(|r| stage_of(r)).collect();
        let is_written = stages.contains("written");
        if is_written {
            written.insert(u64::from(*id));
        }
        let stage_text = if is_written {
            "written".to_string()
        } else {
            stages.iter().copied().collect::<Vec<_>>().join("+")
        };
        let shown = frames
            .iter()
            .find(|r| stage_of(r) == "written")
            .or_else(|| frames.first())
            .copied()
            .expect("a fixture has a frame");
        let containers: BTreeSet<String> = frames
            .iter()
            .map(|r| {
                if r.is_container_member() {
                    r.container.to_string()
                } else {
                    "none".to_string()
                }
            })
            .collect();
        let kinds: BTreeSet<String> = frames
            .iter()
            .map(|r| format!("{:#x}", r.placement_kind))
            .collect();
        let designs: BTreeSet<String> = frames
            .iter()
            .map(|r| {
                r.design_option
                    .map_or_else(|| "none".to_string(), |o| o.to_string())
            })
            .collect();
        let (dx, dy, dz) = shown.extents_feet();
        let own_refs = refs_of.get(id).cloned().unwrap_or_default();
        let listed: Vec<String> = own_refs
            .iter()
            .take(MAX_REFERENCES)
            .map(|r| {
                let category = u32::try_from(*r)
                    .ok()
                    .and_then(|r| category_of.get(&r))
                    .map_or_else(|| "?".to_string(), i64::to_string);
                format!("{r}:{category}")
            })
            .collect();
        let parents: Vec<u32> = own_refs
            .iter()
            .filter_map(|r| u32::try_from(*r).ok())
            .filter(|p| p < id)
            .filter(|p| {
                refs_of
                    .get(p)
                    .is_some_and(|set| set.contains(&u64::from(*id)))
            })
            .collect();
        let same: Vec<String> = parents
            .iter()
            .filter(|p| category_of.get(*p) == Some(&per::OST_PLUMBING_FIXTURES))
            .map(u32::to_string)
            .collect();
        let any: Vec<String> = parents
            .iter()
            .map(|p| {
                format!(
                    "{p}:{}",
                    category_of
                        .get(p)
                        .map_or_else(|| "?".to_string(), i64::to_string)
                )
            })
            .collect();
        println!(
            "fixture id={id} category={} stage={stage_text} frames={} container={} kind={} option={} extents={dx:.3},{dy:.3},{dz:.3} enclosing={} refs={} refs_list={} mutual_same={} mutual_any={}",
            per::OST_PLUMBING_FIXTURES,
            frames.len(),
            containers.into_iter().collect::<Vec<_>>().join("+"),
            kinds.into_iter().collect::<Vec<_>>().join("+"),
            designs.into_iter().collect::<Vec<_>>().join("+"),
            u8::from(shown.id_from_enclosing_record),
            own_refs.len(),
            if listed.is_empty() {
                "-".to_string()
            } else {
                listed.join(",")
            },
            if same.is_empty() {
                "-".to_string()
            } else {
                same.join(",")
            },
            if any.is_empty() {
                "-".to_string()
            } else {
                any.join(",")
            },
        );
    }
    if written.is_empty() {
        println!("done: no written fixture");
        return Ok(());
    }

    // The native records of the written fixtures, their type symbols and the
    // symbols' families.
    let mut symbol_of: BTreeMap<u64, u64> = BTreeMap::new();
    let read = extract_graphs(&mut rf, &native_options(written.clone()), |record| {
        print_native("instance", &record);
        if let Some(symbol) = record
            .graph
            .as_ref()
            .and_then(|graph| root_id(graph, "m_masterSymbolId"))
        {
            symbol_of.insert(record.identity.element_id, symbol);
        }
        Ok(())
    });
    if let Err(error) = read {
        println!("native refused: {}", tidy(&error.to_string()));
    }
    let symbols: BTreeSet<u64> = symbol_of.values().copied().collect();
    let mut family_of: BTreeMap<u64, u64> = BTreeMap::new();
    if !symbols.is_empty() {
        let read = extract_graphs(&mut rf, &native_options(symbols.clone()), |record| {
            print_native("symbol", &record);
            if let Some(family) = record
                .graph
                .as_ref()
                .and_then(|graph| root_id(graph, "m_familyId"))
            {
                family_of.insert(record.identity.element_id, family);
            }
            Ok(())
        });
        if let Err(error) = read {
            println!("native symbols refused: {}", tidy(&error.to_string()));
        }
    }
    let families: BTreeSet<u64> = family_of.values().copied().collect();
    if !families.is_empty() {
        let read = extract_graphs(&mut rf, &native_options(families.clone()), |record| {
            print_native("family", &record);
            Ok(())
        });
        if let Err(error) = read {
            println!("native families refused: {}", tidy(&error.to_string()));
        }
    }

    // The model forms of the families' own documents, and the RE-312 verdict.
    let bodiless = instances_without_body(&mut rf, &written);
    let solids = family_solids(&mut rf, &families);
    for id in &written {
        let symbol = symbol_of.get(id).copied();
        let family = symbol.and_then(|s| family_of.get(&s).copied());
        let solid = family.and_then(|f| solids.as_ref().ok().and_then(|all| all.get(&f)));
        let (doc, forms, cutting, invisible, bound, incomplete) = match solid {
            Some(solid) => (
                "found",
                solid.forms.len(),
                solid.forms.iter().filter(|f| f.cutting).count(),
                solid.forms.iter().filter(|f| f.visible == Some(0)).count(),
                solid.forms.iter().filter(|f| f.bound).count(),
                u8::from(solid.incomplete),
            ),
            None => ("missing", 0, 0, 0, 0, 0),
        };
        let verdict = match &bodiless {
            Err(_) => "refused",
            Ok(set) if set.contains(id) => "bodiless",
            Ok(_) if solid.is_none() => "undecided_no_document",
            Ok(_) if incomplete == 1 => "undecided_incomplete",
            Ok(_) if forms == 0 => "undecided_no_forms",
            Ok(_) => "drawn_or_unresolved",
        };
        println!(
            "forms id={id} symbol={} family={} doc={doc} forms={forms} cutting={cutting} invisible={invisible} bound={bound} incomplete={incomplete} verdict={verdict}",
            symbol.map_or_else(|| "-".to_string(), |s| s.to_string()),
            family.map_or_else(|| "-".to_string(), |f| f.to_string()),
        );
    }
    if let Err(error) = &bodiless {
        println!("forms refused: {}", tidy(&error.to_string()));
    }
    if let Err(error) = &solids {
        println!("family solids refused: {}", tidy(&error.to_string()));
    }
    if !families.is_empty() && families.len() <= MAX_FAMILY_DOCUMENTS {
        match family_records(&mut rf, &families, &BTreeSet::new()) {
            Ok(documents) => {
                for (family, members) in &documents {
                    let mut classes: BTreeMap<&str, usize> = BTreeMap::new();
                    for (_, graph) in members {
                        if let Some(root) = graph.objects.first() {
                            *classes.entry(root.class_name.as_str()).or_default() += 1;
                        }
                    }
                    println!(
                        "famdoc family={family} records={} classes={}",
                        members.len(),
                        counted(&classes)
                    );
                }
            }
            Err(error) => println!("family records refused: {}", tidy(&error.to_string())),
        }
    }

    // The saved graphics of each written fixture: its record on channel 103 and
    // the symbols its InstanceInfo objects name, as `probe_re329`.
    let mut options = Options {
        selected_ids: written.clone(),
        channels: BTreeSet::from([103]),
        max_graph_values: 1_000_000,
        max_graph_objects: 1_000_000,
        releases: NATIVE_RELEASES,
        ..Options::default()
    };
    let mut graphics: BTreeMap<u64, Record> = BTreeMap::new();
    let first = extract_graphs(&mut rf, &options, |record| {
        graphics.insert(record.identity.element_id, record);
        Ok(())
    });
    if let Err(error) = first {
        println!("graphics refused: {}", tidy(&error.to_string()));
    }
    let mut attempted: BTreeSet<u64> = graphics.keys().copied().collect();
    for _ in 0..128 {
        let mut wanted = BTreeSet::new();
        for record in graphics.values() {
            let Some(graph) = &record.graph else { continue };
            for object in &graph.objects {
                if object.class_name != "InstanceInfo" {
                    continue;
                }
                if let Some(id) = identifier(&object.fields["m_symbolId"])
                    .and_then(|id| u64::try_from(id).ok())
                    .filter(|id| *id > 0)
                {
                    if !attempted.contains(&id) {
                        wanted.insert(id);
                    }
                }
            }
        }
        if wanted.is_empty() {
            break;
        }
        attempted.extend(&wanted);
        options.selected_ids = wanted;
        let more = extract_graphs(&mut rf, &options, |record| {
            graphics.insert(record.identity.element_id, record);
            Ok(())
        });
        if let Err(error) = more {
            println!("graphics symbols refused: {}", tidy(&error.to_string()));
            break;
        }
    }
    for id in &written {
        println!("{}", graphics_line(*id, &graphics));
    }
    println!("done");
    Ok(())
}
