//! RE-363 (#1496): can the empty bodies be found from the type symbols' graphics
//! alone, and at what cost.
//!
//! FACT under test (to be measured): `probe_re363_empty_bodies` found that the
//! family instances whose saved graphics decode cleanly to no surface with some
//! branch excluded as not visible are, in every model with a Revit export, none
//! in Revit's export, and that reading every instance's graphics costs 31% to
//! 36% of an office export. An instance draws its type symbol's graphics, so a
//! test on the symbol's own graphics, read once per type, would find the same
//! instances at the cost of the symbols. This probe measures, for the two
//! extras of #1496 (RE1 Plumbing's water closet tank 442378 and the permit
//! office's column 1927742) and every other instance:
//!
//! - the cost of reading and analysing only the type symbols of the plumbing
//!   fixtures and structural columns, beside the default export on the same
//!   handle, with the memory the read added;
//! - for each symbol an instance names, the outcome of the symbol's own graphics
//!   (the same walk as an instance's, from the symbol's root) and the outcomes of
//!   the instances that name it, so a type whose instances differ is seen;
//! - which instances a type-level rule would drop (every symbol they name decodes
//!   clean to no surface, with a branch excluded as not visible), for
//!   `tools/re/empty_bodies_vs_ifc.py` to score against Revit's `Tag`s.
//!
//! Output (one line each):
//!   release saved=<n> layout=<n>
//!   written <n> family_instances <n> classes <Class:n,..>
//!   timing export_ms=<n> entities=<n>
//!   timing scoped_instance_read_ms=<n> instances=<n> symbols=<n>
//!       the instance graphs the symbols-only pass takes its symbol ids from; the
//!       decoded elements already carry them, so this is not part of its cost
//!   timing symbols_only_ms=<n> read_ms=<n> analyse_ms=<n> requested=<n> read=<n>
//!   memory symbols_only rss_before_kb=<n> hwm_after_kb=<n> reset=<ok|failed>
//!   scoped_typedrop <count> <ids>
//!   symbolonly id=<s> outcome=<o> hidden=<n> visible=<n> vis_branches=<n> ...
//!   progress <analysed> of <total>
//!   timing pass_ms=<n> read_ms=<n> analyse_ms=<n> symbol_analyse_ms=<n> instances=<n>
//!   symbol id=<s> outcome=<o> vis_branches=<n> instances=<n> by=<outcome:n,..>
//!   typedrop <category> <name> <count> <ids>
//!   analysed / outcomes / empty / emptyinfo   as `probe_re363_empty_bodies`
//!   done
//!
//! ```bash
//! cargo run --profile ci -p rvt-probes --example probe_re363_type_level -- model.rvt
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

/// The categories of the symbols-only pass: where the two extras of #1496 sit.
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

/// A figure of `/proc/self/status` in kilobytes (`VmHWM`, `VmRSS`), where the
/// platform gives it.
fn status_kb(key: &str) -> Option<u64> {
    let text = std::fs::read_to_string("/proc/self/status").ok()?;
    text.lines()
        .find_map(|line| line.strip_prefix(key))
        .and_then(|rest| rest.trim().trim_end_matches("kB").trim().parse().ok())
}

/// Bring the process's peak resident set down to its current one, where the
/// platform lets it (`/proc/self/clear_refs`, 5).
fn reset_peak() -> bool {
    std::fs::write("/proc/self/clear_refs", "5").is_ok()
}

fn shown(kb: Option<u64>) -> String {
    kb.map_or_else(|| "-".to_string(), |kb| kb.to_string())
}

fn name_of(category: i64) -> String {
    per::builtin_category_name(category)
        .map(|name| name.replace(' ', "_"))
        .unwrap_or_else(|| "unnamed".to_string())
}

fn list(ids: &BTreeSet<u32>) -> String {
    ids.iter().map(u32::to_string).collect::<Vec<_>>().join(",")
}

/// The ids of the symbols the `InstanceInfo` objects of a graph name.
fn named_symbols(record: &Record) -> Vec<u64> {
    let mut out = BTreeSet::new();
    if let Some(graph) = &record.graph {
        for object in &graph.objects {
            if object.class_name != "InstanceInfo" {
                continue;
            }
            if let Some(id) = identifier(&object.fields["m_symbolId"])
                .and_then(|id| u64::try_from(id).ok())
                .filter(|id| *id > 0)
            {
                out.insert(id);
            }
        }
    }
    out.into_iter().collect()
}

fn graph_options() -> Options {
    Options {
        channels: BTreeSet::from([103]),
        max_graph_values: 1_000_000,
        max_graph_objects: 1_000_000,
        releases: NATIVE_RELEASES,
        ..Options::default()
    }
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
    let mut options = graph_options();
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
            for id in named_symbols(&record) {
                if !attempted.contains(&id) {
                    pending.insert(id);
                }
            }
            records.insert(record.identity.element_id, record);
        }
    }
}

/// What an empty body's graphics walk left out or found hidden.
#[derive(Debug, Clone)]
struct Empty {
    hidden: usize,
    visible: usize,
    vis_branches: usize,
    filters: usize,
    nonsurface: usize,
}

impl Empty {
    fn text(&self, outcome: &str) -> String {
        format!(
            "outcome={outcome} hidden={} visible={} vis_branches={} filters={} nonsurface={}",
            self.hidden, self.visible, self.vis_branches, self.filters, self.nonsurface
        )
    }
}

type Reading = (&'static str, Option<Empty>);

/// The outcome of a record's saved graphics at the Fine detail level, an
/// instance's or a symbol's, as `probe_re329` reads an instance's, and for an
/// empty one what the walk left out.
fn analyse(id: u64, records: &BTreeMap<u64, Record>) -> Reading {
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
    let empty = Empty {
        hidden,
        visible: all.primitives.len() - hidden,
        vis_branches: meshes.excluded_visibility_branches,
        filters: meshes.rejected_filters,
        nonsurface: meshes.excluded_non_surface_branches,
    };
    (outcome, Some(empty))
}

/// Whether a reading is the rule's: a clean decode to no surface, with a branch
/// the walk left out as not visible.
fn clean_hidden(reading: &Reading) -> bool {
    reading.0 == "empty_none" && reading.1.as_ref().is_some_and(|e| e.vis_branches > 0)
}

/// What one pass over a set of instances found.
#[derive(Debug, Default)]
struct Pass {
    analysed: Ids,
    outcomes: BTreeMap<(i64, &'static str), usize>,
    empty: EmptyIds,
    /// The symbols each analysed instance names.
    symbols_of: BTreeMap<u32, Vec<u64>>,
    /// For each symbol, how many instances named it with each outcome.
    by_symbol: BTreeMap<u64, BTreeMap<&'static str, usize>>,
    /// The reading of each symbol's own graphics.
    symbol_reading: BTreeMap<u64, Reading>,
    read: Duration,
    analyse: Duration,
    symbol_analyse: Duration,
    done: usize,
}

/// Read the saved graphics of `ids` a chunk at a time and analyse each instance,
/// then each symbol they named, once.
fn run_pass(
    rf: &mut RevitFile,
    ids: &[u64],
    category_of: &BTreeMap<u32, i64>,
    started: Instant,
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
            let (outcome, empty) = analyse(*id, &graphics);
            let symbols = graphics.get(id).map(named_symbols).unwrap_or_default();
            for symbol in &symbols {
                *pass
                    .by_symbol
                    .entry(*symbol)
                    .or_default()
                    .entry(outcome)
                    .or_default() += 1;
            }
            pass.symbols_of.insert(short, symbols);
            pass.analysed.entry(category).or_default().insert(short);
            *pass.outcomes.entry((category, outcome)).or_default() += 1;
            if let Some(empty) = empty {
                pass.empty
                    .entry((category, outcome))
                    .or_default()
                    .insert(short);
                println!(
                    "emptyinfo id={id} category={category} {}",
                    empty.text(outcome)
                );
            }
        }
        pass.analyse += timed.elapsed();
        for id in chunk {
            graphics.remove(id);
        }
        pass.done += chunk.len();
        println!("progress {} of {}", pass.done, ids.len());
    }
    let timed = Instant::now();
    for symbol in pass.by_symbol.keys() {
        pass.symbol_reading
            .insert(*symbol, analyse(*symbol, &graphics));
    }
    pass.symbol_analyse = timed.elapsed();
    pass
}

fn main() -> anyhow::Result<()> {
    let started = Instant::now();
    let path = std::env::args()
        .skip(1)
        .find(|arg| !arg.starts_with("--"))
        .ok_or_else(|| anyhow::anyhow!("usage: probe_re363_type_level <file.rvt>"))?;
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

    // The default export on this handle, for what the passes below add to it.
    let timed = Instant::now();
    match RvtDocExporter.export_with_diagnostics(&mut rf) {
        Ok(result) => println!(
            "timing export_ms={} entities={}",
            timed.elapsed().as_millis(),
            result.model.entities.len()
        ),
        Err(error) => println!("export refused: {error}"),
    }

    // The native class of each written element, read without its graph.
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
    let listed: Vec<String> = classes
        .iter()
        .take(12)
        .map(|(class, n)| format!("{class}:{n}"))
        .collect();
    println!(
        "written {} family_instances {} classes {}",
        written.len(),
        instances.len(),
        listed.join(",")
    );
    let ordered: Vec<u64> = instances.iter().copied().collect();

    // The symbols of the plumbing fixtures and structural columns, from their
    // instances' graphs (a decoded element carries its type symbol already).
    let scoped: BTreeSet<u64> = ordered
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
    let mut scoped_symbols: BTreeMap<u32, Vec<u64>> = BTreeMap::new();
    let mut options = graph_options();
    options.selected_ids = scoped.clone();
    let read = extract_graphs(&mut rf, &options, |record| {
        if let Ok(short) = u32::try_from(record.identity.element_id) {
            scoped_symbols.insert(short, named_symbols(&record));
        }
        Ok(())
    });
    if let Err(error) = read {
        println!("scoped instances refused: {error}");
    }
    let wanted: BTreeSet<u64> = scoped_symbols.values().flatten().copied().collect();
    println!(
        "timing scoped_instance_read_ms={} instances={} symbols={}",
        timed.elapsed().as_millis(),
        scoped_symbols.len(),
        wanted.len()
    );

    // The symbols alone: read and analyse each once.
    let reset = reset_peak();
    let rss_before = status_kb("VmRSS:");
    let mut symbol_graphs: BTreeMap<u64, Record> = BTreeMap::new();
    let mut attempted: BTreeSet<u64> = BTreeSet::new();
    let timed = Instant::now();
    load(&mut rf, &mut symbol_graphs, &mut attempted, wanted.clone());
    let read_time = timed.elapsed();
    let timed = Instant::now();
    let mut readings: BTreeMap<u64, Reading> = BTreeMap::new();
    for symbol in &wanted {
        readings.insert(*symbol, analyse(*symbol, &symbol_graphs));
    }
    let analyse_time = timed.elapsed();
    println!(
        "timing symbols_only_ms={} read_ms={} analyse_ms={} requested={} read={}",
        (read_time + analyse_time).as_millis(),
        read_time.as_millis(),
        analyse_time.as_millis(),
        wanted.len(),
        attempted.len()
    );
    println!(
        "memory symbols_only rss_before_kb={} hwm_after_kb={} reset={}",
        shown(rss_before),
        shown(status_kb("VmHWM:")),
        if reset { "ok" } else { "failed" }
    );
    for (symbol, reading) in &readings {
        let detail = reading
            .1
            .as_ref()
            .map_or_else(String::new, |empty| format!(" {}", empty.text(reading.0)));
        println!("symbolonly id={symbol} outcome={}{detail}", reading.0);
    }
    let dropped: BTreeSet<u32> = scoped_symbols
        .iter()
        .filter(|(_, symbols)| {
            !symbols.is_empty()
                && symbols
                    .iter()
                    .all(|symbol| readings.get(symbol).is_some_and(clean_hidden))
        })
        .map(|(short, _)| *short)
        .collect();
    println!("scoped_typedrop {} {}", dropped.len(), list(&dropped));
    drop(symbol_graphs);

    // Every family instance, and the symbols they name.
    let timed = Instant::now();
    let full = run_pass(&mut rf, &ordered, &category_of, started);
    println!(
        "timing pass_ms={} read_ms={} analyse_ms={} symbol_analyse_ms={} instances={}",
        timed.elapsed().as_millis(),
        full.read.as_millis(),
        full.analyse.as_millis(),
        full.symbol_analyse.as_millis(),
        full.done
    );
    for (symbol, by) in &full.by_symbol {
        let reading = full.symbol_reading.get(symbol);
        let outcome = reading.map_or("-", |r| r.0);
        let vis = reading
            .and_then(|r| r.1.as_ref())
            .map_or_else(|| "-".to_string(), |e| e.vis_branches.to_string());
        let total: usize = by.values().sum();
        let split: Vec<String> = by.iter().map(|(o, n)| format!("{o}:{n}")).collect();
        println!(
            "symbol id={symbol} outcome={outcome} vis_branches={vis} instances={total} by={}",
            split.join(",")
        );
    }
    let mut type_dropped: Ids = Ids::new();
    for (short, symbols) in &full.symbols_of {
        let found = !symbols.is_empty()
            && symbols
                .iter()
                .all(|symbol| full.symbol_reading.get(symbol).is_some_and(clean_hidden));
        if let (true, Some(category)) = (found, category_of.get(short)) {
            type_dropped.entry(*category).or_default().insert(*short);
        }
    }
    for (category, ids) in &type_dropped {
        println!(
            "typedrop {category} {} {} {}",
            name_of(*category),
            ids.len(),
            list(ids)
        );
    }
    for (category, ids) in &full.analysed {
        println!(
            "analysed {category} {} {} {}",
            name_of(*category),
            ids.len(),
            list(ids)
        );
        let counts: Vec<String> = full
            .outcomes
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
    for ((category, outcome), ids) in &full.empty {
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
