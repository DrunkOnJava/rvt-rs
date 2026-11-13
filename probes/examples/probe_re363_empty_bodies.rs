//! RE-363 (#1496): which family instances of any category have a saved body with
//! no visible surface, and are they in Revit's export.
//!
//! FACT under test (to be measured): RE1 Plumbing's water closet tank 442378 is
//! the one plumbing fixture whose saved graphics at the Fine detail level
//! (native level 3, what `GeometryUtil.GetIFCExportGeometryOptions` in revit-ifc
//! asks for, CLEANROOM.md source 5) decode cleanly to no surface at all, every
//! primitive lying under a branch flagged not visible
//! (`probe_re363_plumbing_fixtures`), and revit-ifc's `FamilyInstanceExporter`
//! writes nothing for a family instance that is not a door or a window and has no
//! solid or mesh. A rule that leaves such an instance out is safe only if no
//! family instance Revit's export holds, of any category, has that outcome. This
//! probe reads the saved graphics of every element rvt-rs writes whose native class
//! is `FamilyInstance`, as `probe_re329_saved_graphics_census` does, and prints
//! the outcome of each category, and the ids of the instances whose outcome is
//! `empty_none` (no surface, no diagnostic at all) or `empty_view_only` (no
//! surface, only the view-component or nested-detail-filter diagnostics).
//! `tools/re/empty_bodies_vs_ifc.py` scores those ids against the `Tag`s of
//! Revit's export.
//!
//! Output (one line each):
//!   release saved=<n> layout=<n>
//!   written <n> family_instances <n> classes <Class:n,..>
//!   progress <analysed> of <total>
//!   analysed <category> <name> <count> <ids>
//!   outcomes <category> <name> <outcome>=<n> ...
//!   empty <category> <name> <outcome> <count> <ids>
//!   emptyinfo id=<id> category=<c> outcome=<o> hidden=<n> visible=<n> vis_branches=<n>
//!       filters=<n> nonsurface=<n>
//!   truncated <analysed> of <total>   (only where the time budget ran out)
//!   timing export_ms=<n> entities=<n>          the default export, on this handle
//!   timing classes_ms=<n>                      the class of each written element
//!   timing scoped_pass_ms=<n> read_ms=<n> analyse_ms=<n> instances=<n> symbols=<n>
//!                                              the pass over the plumbing fixtures and
//!                                              structural columns alone, after the export
//!   timing pass_ms=<n> read_ms=<n> analyse_ms=<n> instances=<n> symbols=<n>
//!                                              the pass over every family instance
//!   memory vmhwm_kb after_export=<n> after_scoped=<n> after_pass=<n>   (Linux only)
//!   done
//!
//! ```bash
//! cargo run --profile ci -p rvt-probes --example probe_re363_empty_bodies -- model.rvt
//! ```

use rvt::RevitFile;
use rvt::ifc::RvtDocExporter;
use rvt::research::graph_value::Fv;
use rvt::research::native_document::{NATIVE_RELEASES, Options, Record, extract_graphs};
use rvt::research::native_graphics_traversal::{
    NESTED_DETAIL_FILTER_DIAGNOSTIC, VIEW_COMPONENT_DIAGNOSTIC,
};
use rvt::research::native_saved_mesh::{
    graphics_including_hidden, graphics_with_resolver_at_detail,
};
use rvt::research::partition_element_records::{self as per, PartitionElementRecord};
use rvt::research::partition_element_records_2023 as per32;
use std::collections::{BTreeMap, BTreeSet};
use std::time::{Duration, Instant};

/// How many instances are read and analysed at a time. The symbols they name
/// are kept between chunks, the instances' own graphs are not.
const CHUNK: usize = 1000;

/// The time after which no further chunk is started, so the lines already
/// printed stay a usable partial result under Measure's 20 minute limit.
const BUDGET: Duration = Duration::from_secs(14 * 60);

/// The categories of the scoped pass: where the two extras of #1496 sit (the
/// water closet tank and the permit office's column 1927742).
const SCOPED: [i64; 2] = [per::OST_PLUMBING_FIXTURES, per::OST_STRUCTURAL_COLUMNS];

type Ids = BTreeMap<i64, BTreeSet<u32>>;
type EmptyIds = BTreeMap<(i64, &'static str), BTreeSet<u32>>;

fn i64_at(buf: &[u8], at: usize) -> Option<i64> {
    let end = at.checked_add(8)?;
    buf.get(at..end)
        .map(|b| i64::from_le_bytes(b.try_into().expect("8 bytes")))
}

/// Every frame of the file's marker in the `BuiltInCategory` band, decoded
/// for any category (as `probe_re350_category_stages`).
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

fn ignorable(diagnostic: &str) -> bool {
    diagnostic.ends_with(NESTED_DETAIL_FILTER_DIAGNOSTIC)
        || diagnostic.ends_with(VIEW_COMPONENT_DIAGNOSTIC)
}

/// The process's peak resident set in kilobytes, where `/proc` gives it.
fn peak_kb() -> Option<u64> {
    let text = std::fs::read_to_string("/proc/self/status").ok()?;
    text.lines()
        .find_map(|line| line.strip_prefix("VmHWM:"))
        .and_then(|rest| rest.trim().trim_end_matches("kB").trim().parse().ok())
}

fn name_of(category: i64) -> String {
    per::builtin_category_name(category)
        .map(|name| name.replace(' ', "_"))
        .unwrap_or_else(|| "unnamed".to_string())
}

fn list(ids: &BTreeSet<u32>) -> String {
    ids.iter().map(u32::to_string).collect::<Vec<_>>().join(",")
}

/// Read the graphics-channel records of `ids` and, one round at a time, of the
/// symbols their `InstanceInfo` objects name, into `records`. `attempted` holds
/// every id already asked for, so a symbol is read once for the whole run.
fn load(
    rf: &mut RevitFile,
    records: &mut BTreeMap<u64, Record>,
    attempted: &mut BTreeSet<u64>,
    ids: BTreeSet<u64>,
) {
    let mut options = Options {
        channels: BTreeSet::from([103]),
        max_graph_values: 1_000_000,
        max_graph_objects: 1_000_000,
        releases: NATIVE_RELEASES,
        ..Options::default()
    };
    let mut pending: BTreeSet<u64> = ids.difference(attempted).copied().collect();
    for _ in 0..128 {
        if pending.is_empty() {
            break;
        }
        attempted.extend(&pending);
        options.selected_ids = std::mem::take(&mut pending);
        let mut fresh: Vec<Record> = Vec::new();
        let read = extract_graphs(rf, &options, |record| {
            fresh.push(record);
            Ok(())
        });
        if let Err(error) = read {
            println!("graphics refused: {error}");
            break;
        }
        for record in fresh {
            if let Some(graph) = &record.graph {
                for object in &graph.objects {
                    if object.class_name != "InstanceInfo" {
                        continue;
                    }
                    if let Some(id) = identifier(&object.fields["m_symbolId"])
                        .and_then(|id| u64::try_from(id).ok())
                        .filter(|id| *id > 0)
                    {
                        if !attempted.contains(&id) {
                            pending.insert(id);
                        }
                    }
                }
            }
            records.insert(record.identity.element_id, record);
        }
    }
}

/// The outcome of an instance's saved graphics at the Fine detail level, as
/// `probe_re329` reads it, and for an empty one its hidden and visible
/// primitive counts and the branches the walk excluded.
fn analyse(id: u64, records: &BTreeMap<u64, Record>) -> (&'static str, Option<String>) {
    let Some(record) = records.get(&id) else {
        return ("no_record", None);
    };
    let Some(graph) = record.graph.as_ref() else {
        return ("no_graph", None);
    };
    let resolver = |rid: u64| records.get(&rid).and_then(|r| r.graph.as_ref());
    let meshes = graphics_with_resolver_at_detail(graph, &resolver, 3);
    let blocking = meshes.diagnostics.iter().any(|d| !ignorable(d));
    let outcome = match (meshes.primitives.is_empty(), blocking) {
        (false, false) => "drawn",
        (false, true) => "box_partial",
        (true, true) => "box_no_surface",
        (true, false) if !meshes.diagnostics.is_empty() => "empty_view_only",
        (true, false) => "empty_none",
    };
    if !outcome.starts_with("empty") {
        return (outcome, None);
    }
    let all = graphics_including_hidden(graph, &resolver, 3);
    let hidden = all.primitives.iter().filter(|p| p.hidden).count();
    let visible = all.primitives.len() - hidden;
    let info = format!(
        "outcome={outcome} hidden={hidden} visible={visible} vis_branches={} filters={} nonsurface={}",
        meshes.excluded_visibility_branches,
        meshes.rejected_filters,
        meshes.excluded_non_surface_branches
    );
    (outcome, Some(info))
}

/// What one pass over a set of instances found.
#[derive(Debug, Default)]
struct Pass {
    analysed: Ids,
    outcomes: BTreeMap<(i64, &'static str), usize>,
    empty: EmptyIds,
    read: Duration,
    analyse: Duration,
    done: usize,
    symbols: usize,
}

/// Read the saved graphics of `ids` a chunk at a time and analyse each instance.
/// `quiet` leaves out the per-instance and progress lines.
fn run_pass(
    rf: &mut RevitFile,
    ids: &[u64],
    category_of: &BTreeMap<u32, i64>,
    started: Instant,
    quiet: bool,
) -> Pass {
    let mut pass = Pass::default();
    let mut graphics: BTreeMap<u64, Record> = BTreeMap::new();
    let mut attempted: BTreeSet<u64> = BTreeSet::new();
    for chunk in ids.chunks(CHUNK) {
        if started.elapsed() > BUDGET {
            println!("truncated {} of {}", pass.done, ids.len());
            break;
        }
        let timed = Instant::now();
        load(
            rf,
            &mut graphics,
            &mut attempted,
            chunk.iter().copied().collect(),
        );
        pass.read += timed.elapsed();
        let timed = Instant::now();
        for id in chunk {
            let Ok(short) = u32::try_from(*id) else {
                continue;
            };
            let Some(category) = category_of.get(&short).copied() else {
                continue;
            };
            let (outcome, info) = analyse(*id, &graphics);
            pass.analysed.entry(category).or_default().insert(short);
            *pass.outcomes.entry((category, outcome)).or_default() += 1;
            if let Some(info) = info {
                pass.empty
                    .entry((category, outcome))
                    .or_default()
                    .insert(short);
                if !quiet {
                    println!("emptyinfo id={id} category={category} {info}");
                }
            }
        }
        pass.analyse += timed.elapsed();
        for id in chunk {
            graphics.remove(id);
        }
        pass.done += chunk.len();
        if !quiet {
            println!("progress {} of {}", pass.done, ids.len());
        }
    }
    let own: BTreeSet<u64> = ids.iter().copied().collect();
    pass.symbols = attempted.difference(&own).count();
    pass
}

fn main() -> anyhow::Result<()> {
    let started = Instant::now();
    let path = std::env::args()
        .skip(1)
        .find(|arg| !arg.starts_with("--"))
        .ok_or_else(|| anyhow::anyhow!("usage: probe_re363_empty_bodies <file.rvt>"))?;
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
    let mut category_of: BTreeMap<u32, i64> = BTreeMap::new();
    for record in &records {
        if !record.is_exported_instance() || !record.has_volume() {
            continue;
        }
        if design.as_ref().is_some_and(|o| o.excludes(record))
            || phases.as_ref().is_some_and(|p| p.excludes(record))
        {
            continue;
        }
        category_of
            .entry(record.element_id)
            .or_insert(record.builtin_category);
    }
    let written: BTreeSet<u64> = category_of.keys().map(|id| u64::from(*id)).collect();

    // The default export on this handle, for what the pass below adds to it.
    let timed = Instant::now();
    match RvtDocExporter.export_with_diagnostics(&mut rf) {
        Ok(result) => println!(
            "timing export_ms={} entities={}",
            timed.elapsed().as_millis(),
            result.model.entities.len()
        ),
        Err(error) => println!("export refused: {error}"),
    }
    let after_export = peak_kb();

    // The native class of each written element, read without its graph.
    let timed = Instant::now();
    let mut instances: BTreeSet<u64> = BTreeSet::new();
    let mut classes: BTreeMap<String, usize> = BTreeMap::new();
    let base = Options {
        selected_ids: written.clone(),
        releases: NATIVE_RELEASES,
        graph_classes: Some(BTreeSet::new()),
        ..Options::default()
    };
    let read = extract_graphs(&mut rf, &base, |record| {
        let class = record.class_name.clone().unwrap_or_else(|| "-".to_string());
        if class == "FamilyInstance" {
            instances.insert(record.identity.element_id);
        }
        *classes.entry(class).or_default() += 1;
        Ok(())
    });
    if let Err(error) = read {
        println!("native refused: {error}");
    }
    println!("timing classes_ms={}", timed.elapsed().as_millis());
    let shown: Vec<String> = classes
        .iter()
        .take(12)
        .map(|(class, n)| format!("{class}:{n}"))
        .collect();
    println!(
        "written {} family_instances {} classes {}",
        written.len(),
        instances.len(),
        shown.join(",")
    );

    let ordered: Vec<u64> = instances.iter().copied().collect();

    // The scoped pass first, so its peak memory is not hidden by the full pass.
    let scoped: Vec<u64> = ordered
        .iter()
        .copied()
        .filter(|id| {
            u32::try_from(*id)
                .ok()
                .and_then(|short| category_of.get(&short))
                .is_some_and(|category| SCOPED.contains(category))
        })
        .collect();
    let timed = Instant::now();
    let narrow = run_pass(&mut rf, &scoped, &category_of, started, true);
    println!(
        "timing scoped_pass_ms={} read_ms={} analyse_ms={} instances={} symbols={}",
        timed.elapsed().as_millis(),
        narrow.read.as_millis(),
        narrow.analyse.as_millis(),
        narrow.done,
        narrow.symbols
    );
    let after_scoped = peak_kb();
    let scoped_empty: usize = narrow.empty.values().map(BTreeSet::len).sum();
    println!(
        "scoped empty_instances={scoped_empty} of {}",
        narrow.analysed.values().map(BTreeSet::len).sum::<usize>()
    );

    let timed = Instant::now();
    let full = run_pass(&mut rf, &ordered, &category_of, started, false);
    println!(
        "timing pass_ms={} read_ms={} analyse_ms={} instances={} symbols={}",
        timed.elapsed().as_millis(),
        full.read.as_millis(),
        full.analyse.as_millis(),
        full.done,
        full.symbols
    );
    println!(
        "memory vmhwm_kb after_export={} after_scoped={} after_pass={}",
        after_export.map_or_else(|| "-".to_string(), |kb| kb.to_string()),
        after_scoped.map_or_else(|| "-".to_string(), |kb| kb.to_string()),
        peak_kb().map_or_else(|| "-".to_string(), |kb| kb.to_string())
    );
    let (analysed, outcomes, empty) = (full.analysed, full.outcomes, full.empty);
    for (category, ids) in &analysed {
        println!(
            "analysed {category} {} {} {}",
            name_of(*category),
            ids.len(),
            list(ids)
        );
        let counts: Vec<String> = outcomes
            .iter()
            .filter(|((c, _), _)| c == category)
            .map(|((_, outcome), n)| format!("{outcome}={n}"))
            .collect();
        println!(
            "outcomes {category} {} {}",
            name_of(*category),
            counts.join(" ")
        );
    }
    for ((category, outcome), ids) in &empty {
        println!(
            "empty {category} {} {outcome} {} {}",
            name_of(*category),
            ids.len(),
            list(ids)
        );
    }
    println!("done");
    Ok(())
}
