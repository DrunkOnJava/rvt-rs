//! Current native saved-graphics extraction with explicit shared-symbol closure.
use crate::{
    RevitFile,
    control::{Stage, WalkerControl},
    native_document::{self, Options, Record, Summary},
    native_parameters::ObjectGraph,
    native_saved_mesh::{self, GraphicsMeshes},
};
use anyhow::{Result, ensure};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Mutex;
#[derive(Debug, Serialize)]
pub struct SavedElement {
    pub id: u64,
    pub identity: crate::native_index::Identity,
    pub source: serde_json::Value,
    pub referenced_graph_sources: BTreeMap<u64, serde_json::Value>,
    pub status: String,
    pub meshes: GraphicsMeshes,
}
#[derive(Debug, Serialize)]
pub struct SavedScene {
    pub source_sha256: Option<String>,
    pub format: &'static str,
    pub units: &'static str,
    pub coordinate_frame: &'static str,
    pub complete_document_geometry: bool,
    pub detail_level: i64,
    pub graphics_profile: &'static str,
    pub summaries: Vec<Summary>,
    pub elements: Vec<SavedElement>,
}
fn saved_element_status(meshes: &GraphicsMeshes) -> &'static str {
    // Selection exclusions are intentional profile decisions and stay visible
    // as counters without tainting an otherwise valid mesh. An unbounded face,
    // or a decoder diagnostic, means the owner was not fully tessellated.
    if meshes.diagnostics.is_empty() && meshes.unbounded_faces == 0 {
        "decoded_saved_graphics"
    } else {
        "partial_saved_graphics"
    }
}
/// The symbols the `InstanceInfo` objects of a record's graph name.
fn instance_symbols(record: &Record) -> Vec<u64> {
    let Some(graph) = &record.graph else {
        return Vec::new();
    };
    graph
        .objects
        .iter()
        .filter(|object| object.class_name == "InstanceInfo")
        .filter_map(|object| {
            crate::native_metadata::identifier(&object.fields["m_symbolId"])
                .ok()
                .and_then(|id| u64::try_from(id).ok())
                .filter(|id| *id > 0)
        })
        .collect()
}
/// The decoded records meshing looks graphs up in: the owners being meshed
/// and, beside them, the shared symbols decoded for them.
#[derive(Clone, Copy)]
struct Records<'a> {
    owners: &'a BTreeMap<u64, Record>,
    shared: &'a BTreeMap<u64, Record>,
}
impl<'a> Records<'a> {
    fn get(&self, id: u64) -> Option<&'a Record> {
        self.owners.get(&id).or_else(|| self.shared.get(&id))
    }
    fn graph(&self, id: u64) -> Option<&'a ObjectGraph> {
        self.get(id).and_then(|record| record.graph.as_ref())
    }
}
/// What meshing an owner takes besides its records.
struct MeshRun<'a> {
    detail_level: i64,
    include_hidden: bool,
    include_filtered: bool,
    control: &'a WalkerControl,
    /// Owners meshed so far, counted under a lock so the reports stay in
    /// order across workers.
    done: &'a Mutex<u64>,
    total: u64,
}
/// Mesh each of `ids`, in turn or spread over the cores, and return the
/// elements in the order of `ids`.
fn mesh_elements(
    ids: &[u64],
    records: Records<'_>,
    run: &MeshRun<'_>,
) -> Result<Vec<SavedElement>> {
    let mesh = |id: u64| -> Result<SavedElement> {
        let r = records
            .get(id)
            .ok_or_else(|| anyhow::anyhow!("graphics owner {id} was not decoded"))?;
        let resolver = |id: u64| records.graph(id);
        let mut meshes = match &r.graph {
            Some(g) if run.include_filtered => {
                native_saved_mesh::graphics_including_filtered(g, &resolver, run.detail_level)
            }
            Some(g) if run.include_hidden => {
                native_saved_mesh::graphics_including_hidden(g, &resolver, run.detail_level)
            }
            Some(g) => {
                native_saved_mesh::graphics_with_resolver_at_detail(g, &resolver, run.detail_level)
            }
            None => GraphicsMeshes::default(),
        };
        if r.graph.is_none() {
            meshes.diagnostics.push(
                r.diagnostic
                    .clone()
                    .unwrap_or_else(|| "graphics graph unavailable".into()),
            )
        }
        let refs: BTreeSet<_> = meshes
            .primitives
            .iter()
            .filter_map(|p| p.source_owner_id)
            .collect();
        let referenced_graph_sources = refs
            .into_iter()
            .map(|id| {
                let source = records
                    .get(id)
                    .ok_or_else(|| anyhow::anyhow!("graphics owner {id} was not decoded"))?;
                Ok((id, serde_json::to_value(&source.source)?))
            })
            .collect::<Result<_>>()?;
        Ok(SavedElement {
            id,
            identity: r.identity.clone(),
            source: serde_json::to_value(&r.source)?,
            referenced_graph_sources,
            status: saved_element_status(&meshes).into(),
            meshes,
        })
    };
    let build = |id: u64| -> Result<SavedElement> {
        run.control.check()?;
        let element = mesh(id);
        let mut done = run.done.lock().unwrap_or_else(|e| e.into_inner());
        *done += 1;
        run.control
            .report(Stage::SavedMeshes, *done, Some(run.total));
        element
    };
    if workers() == 1 {
        return ids.iter().map(|&id| build(id)).collect::<Result<Vec<_>>>();
    }
    let next = std::sync::atomic::AtomicUsize::new(0);
    let mut built: Vec<(usize, Result<SavedElement>)> = std::thread::scope(|scope| {
        let handles: Vec<_> = (0..workers())
            .map(|_| {
                scope.spawn(|| {
                    let mut out = Vec::new();
                    loop {
                        let i = next.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                        let Some(&id) = ids.get(i) else { break };
                        out.push((i, build(id)));
                    }
                    out
                })
            })
            .collect();
        handles
            .into_iter()
            .flat_map(|h| h.join().unwrap_or_else(|e| std::panic::resume_unwind(e)))
            .collect()
    });
    built.sort_by_key(|(i, _)| *i);
    built
        .into_iter()
        .map(|(_, e)| e)
        .collect::<Result<Vec<_>>>()
}
/// Elements mesh independently: spread them over the cores, keeping order.
/// The browser build (wasm32-unknown-unknown) has no threads, so there, as
/// on a single core, they mesh one after the other.
fn workers() -> usize {
    if cfg!(all(target_arch = "wasm32", target_os = "unknown")) {
        1
    } else {
        std::thread::available_parallelism().map_or(1, |n| n.get())
    }
}
pub fn extract(file: &mut RevitFile, options: &Options) -> Result<SavedScene> {
    extract_at_detail(file, options, 3)
}
/// [`extract`], also decoding the branches whose visibility flags hide them;
/// each primitive one lies on the way to is marked hidden (RE-318).
pub fn extract_including_hidden(file: &mut RevitFile, options: &Options) -> Result<SavedScene> {
    extract_impl(file, options, 3, &WalkerControl::default(), true, false)
}
/// [`extract_including_hidden`], also decoding the children of a filter whose
/// conditions reject the context or are not read; each primitive one lies on
/// the way to is marked filtered (RE-318).
pub fn extract_including_filtered(file: &mut RevitFile, options: &Options) -> Result<SavedScene> {
    extract_impl(file, options, 3, &WalkerControl::default(), true, true)
}
pub fn extract_at_detail(
    file: &mut RevitFile,
    options: &Options,
    detail_level: i64,
) -> Result<SavedScene> {
    extract_at_detail_with_control(file, options, detail_level, &WalkerControl::default())
}
/// Same as [`extract_at_detail`], reporting [`Stage::SavedGraphics`] and
/// [`Stage::SavedMeshes`] through `control` and stopping with
/// [`crate::Error::Cancelled`] when its token is cancelled.
pub fn extract_at_detail_with_control(
    file: &mut RevitFile,
    options: &Options,
    detail_level: i64,
    control: &WalkerControl,
) -> Result<SavedScene> {
    extract_impl(file, options, detail_level, control, false, false)
}
fn extract_impl(
    file: &mut RevitFile,
    options: &Options,
    detail_level: i64,
    control: &WalkerControl,
    include_hidden: bool,
    include_filtered: bool,
) -> Result<SavedScene> {
    ensure!(
        (1..=3).contains(&detail_level),
        "detail level must be 1, 2 or 3"
    );
    control.check()?;
    control.report(Stage::SavedGraphics, 0, None);
    let mut opts = options.clone();
    opts.channels = BTreeSet::from([103]);
    // Each element's `source` below carries its record's body digest.
    opts.record_extras = true;
    let mut records = BTreeMap::<u64, Record>::new();
    let first = native_document::extract_graphs(file, &opts, |r| {
        ensure!(
            records.insert(r.identity.element_id, r).is_none(),
            "duplicate current graphics owner"
        );
        Ok(())
    })?;
    let roots: BTreeSet<_> = records.keys().copied().collect();
    let mut summaries = vec![first];
    let mut attempted = roots.clone();
    for depth in 0..=128 {
        let mut wanted = BTreeSet::new();
        for r in records.values() {
            for id in instance_symbols(r) {
                if !attempted.contains(&id) {
                    wanted.insert(id);
                }
            }
        }
        if wanted.is_empty() {
            break;
        }
        ensure!(depth < 128, "shared graphics closure depth budget");
        control.check()?;
        attempted.extend(&wanted);
        opts.selected_ids = wanted;
        summaries.push(native_document::extract_graphs(file, &opts, |r| {
            ensure!(
                records.insert(r.identity.element_id, r).is_none(),
                "duplicate shared graphics owner"
            );
            Ok(())
        })?);
    }
    control.check()?;
    control.report(Stage::SavedGraphics, 1, Some(1));
    let roots: Vec<u64> = roots.into_iter().collect();
    let total = roots.len() as u64;
    control.report(Stage::SavedMeshes, 0, Some(total));
    let done = Mutex::new(0u64);
    let run = MeshRun {
        detail_level,
        include_hidden,
        include_filtered,
        control,
        done: &done,
        total,
    };
    let no_shared = BTreeMap::new();
    let elements = mesh_elements(
        &roots,
        Records {
            owners: &records,
            shared: &no_shared,
        },
        &run,
    )?;
    // Freeing the decoded graphs takes seconds on large models: do it aside
    // where there are threads.
    if workers() > 1 {
        std::thread::spawn(move || drop(records));
    }
    Ok(SavedScene {
        source_sha256: None,
        format: "rvt-native-saved-scene-v1",
        units: "feet",
        coordinate_frame: "document Z-up",
        complete_document_geometry: false,
        detail_level,
        graphics_profile: "default 3D viewport (qualified supported subset)",
        summaries,
        elements,
    })
}
/// The decoded graph objects one batch of [`extract_in_batches_with_control`]
/// may hold, about 95 MiB on the BIM4LCA office (4.8 KiB an object), which
/// reads its 105 symbols in about eight passes.
const BATCH_OBJECT_BUDGET: usize = 20_000;
/// The shared symbols of the first batch; the next ones are sized from the
/// graph objects the one before decoded for each.
const FIRST_BATCH_SYMBOLS: usize = 8;
const MIN_BATCH_SYMBOLS: usize = 4;
const MAX_BATCH_SYMBOLS: usize = 1024;
/// An owner as a batch is planned: the first shared symbol it names (the
/// order owners naming the same one together), its id, and the shared symbols
/// it names itself.
type Named = (Option<u64>, u64, BTreeSet<u64>);
/// Adds to `wanted` the shared symbols named by `ids` that no read has asked
/// for yet. An id that is an owner is not read again: the references of its
/// own graph (`owner_references`) are followed instead, once, and so on through
/// the owners they name.
fn add_references(
    ids: impl IntoIterator<Item = u64>,
    owner_references: &BTreeMap<u64, Vec<u64>>,
    expanded: &mut BTreeSet<u64>,
    attempted: &BTreeSet<u64>,
    wanted: &mut BTreeSet<u64>,
) {
    let mut pending: Vec<u64> = ids.into_iter().collect();
    while let Some(id) = pending.pop() {
        match owner_references.get(&id) {
            Some(references) => {
                if expanded.insert(id) {
                    pending.extend(references.iter().copied());
                }
            }
            None => {
                if !attempted.contains(&id) {
                    wanted.insert(id);
                }
            }
        }
    }
}
/// [`extract_at_detail_with_control`] for a caller that takes each element as
/// it is meshed and keeps none of the graphs: `visit` receives every element,
/// in no set order, and the decoded graphs of the shared symbols are freed
/// before the next ones are read.
///
/// The owners (a door's or window's own graph is a few objects) are decoded
/// first and kept. Their symbols, which are shared and which hold the
/// geometry (on the BIM4LCA office 105 symbols hold 99 % of the graph
/// objects of 538 owners), are decoded a batch at a time, with the owners
/// that name them meshed against the batch. A batch holds the symbols of
/// owners next to each other by symbol, as many as hold about 20,000 graph
/// objects (`BATCH_OBJECT_BUDGET`) by the size of the batch before. Each
/// element is the one [`extract_at_detail_with_control`] makes; only the
/// summaries of the passes are not returned. The progress reported to
/// `control` is its too: [`Stage::SavedGraphics`] at 0 and then at 1 of 1
/// once the owners are read, then [`Stage::SavedMeshes`] at 0 and once for each
/// owner meshed, of the number of owners.
pub fn extract_in_batches_with_control(
    file: &mut RevitFile,
    options: &Options,
    detail_level: i64,
    control: &WalkerControl,
    mut visit: impl FnMut(SavedElement) -> Result<()>,
) -> Result<()> {
    ensure!(
        (1..=3).contains(&detail_level),
        "detail level must be 1, 2 or 3"
    );
    control.check()?;
    control.report(Stage::SavedGraphics, 0, None);
    let mut opts = options.clone();
    opts.channels = BTreeSet::from([103]);
    opts.record_extras = true;
    let mut owners = BTreeMap::<u64, Record>::new();
    native_document::extract_graphs(file, &opts, |r| {
        ensure!(
            owners.insert(r.identity.element_id, r).is_none(),
            "duplicate current graphics owner"
        );
        Ok(())
    })?;
    // Everything each owner's graph names. An owner can name another owner,
    // whose own symbols its mesh then reaches, as the one-shot read finds them
    // by reading every record's references.
    let owner_references: BTreeMap<u64, Vec<u64>> = owners
        .iter()
        .map(|(&id, record)| (id, instance_symbols(record)))
        .collect();
    // Each owner with the symbols it names that are not owners themselves,
    // ordered so that owners naming the same symbols are next to each other.
    let mut named: Vec<Named> = owner_references
        .iter()
        .map(|(&id, references)| {
            let symbols: BTreeSet<u64> = references
                .iter()
                .copied()
                .filter(|symbol| !owner_references.contains_key(symbol))
                .collect();
            (symbols.iter().next().copied(), id, symbols)
        })
        .collect();
    named.sort();
    let total = named.len() as u64;
    // The progress is the one-shot read's, whatever the batches: the graphics
    // are reported read once the owners are, and each owner is reported meshed
    // once, in order, against the same total.
    control.check()?;
    control.report(Stage::SavedGraphics, 1, Some(1));
    control.report(Stage::SavedMeshes, 0, Some(total));
    let done = Mutex::new(0u64);
    let run = MeshRun {
        detail_level,
        include_hidden: false,
        include_filtered: false,
        control,
        done: &done,
        total,
    };
    let mut limit = FIRST_BATCH_SYMBOLS;
    let mut next = 0;
    let mut freeing: Option<std::thread::JoinHandle<()>> = None;
    while next < named.len() {
        control.check()?;
        let mut ids = Vec::new();
        let mut symbols = BTreeSet::<u64>::new();
        while let Some((_, id, own)) = named.get(next) {
            if !ids.is_empty() && symbols.union(own).count() > limit {
                break;
            }
            ids.push(*id);
            symbols.extend(own);
            next += 1;
        }
        // The last batch's graphs are gone before these are read.
        if let Some(handle) = freeing.take() {
            let _ = handle.join();
        }
        // The closure of the batch's owners: the symbols they name, those the
        // symbols name, and, through any owner either names, the symbols that
        // owner names. It is the one the one-shot read makes for these owners.
        let mut shared = BTreeMap::<u64, Record>::new();
        let mut attempted = BTreeSet::<u64>::new();
        let mut expanded = BTreeSet::<u64>::new();
        let mut wanted = BTreeSet::<u64>::new();
        add_references(
            ids.iter().copied(),
            &owner_references,
            &mut expanded,
            &attempted,
            &mut wanted,
        );
        for depth in 0..=128 {
            if wanted.is_empty() {
                break;
            }
            ensure!(depth < 128, "shared graphics closure depth budget");
            control.check()?;
            attempted.extend(&wanted);
            opts.selected_ids = std::mem::take(&mut wanted);
            let mut found = Vec::new();
            native_document::extract_graphs(file, &opts, |r| {
                found.extend(instance_symbols(&r));
                ensure!(
                    shared.insert(r.identity.element_id, r).is_none(),
                    "duplicate shared graphics owner"
                );
                Ok(())
            })?;
            add_references(
                found,
                &owner_references,
                &mut expanded,
                &attempted,
                &mut wanted,
            );
        }
        let objects: usize = shared
            .values()
            .filter_map(|record| record.graph.as_ref())
            .map(|graph| graph.objects.len())
            .sum();
        let elements = mesh_elements(
            &ids,
            Records {
                owners: &owners,
                shared: &shared,
            },
            &run,
        )?;
        for element in elements {
            visit(element)?;
        }
        limit = (BATCH_OBJECT_BUDGET.saturating_mul(shared.len().max(1)) / objects.max(1))
            .clamp(MIN_BATCH_SYMBOLS, MAX_BATCH_SYMBOLS);
        // Freeing the decoded graphs takes seconds on large models: do it
        // aside where there are threads.
        if workers() > 1 {
            freeing = Some(std::thread::spawn(move || drop(shared)));
        }
    }
    Ok(())
}
