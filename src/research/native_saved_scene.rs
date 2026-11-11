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
/// The fields of a graphics record's objects that meshing reads, which are the
/// only ones this pass builds ([`Options::graph_fields`], #1694). The dhat run
/// of the pass on Autodesk's samples of Revit 2023 put 77% to 87% of its peak in
/// the decoded nodes, each a map entry with its own key, and the readers below
/// take a small part of what a graph holds. The names are the `m_` string
/// literals of every module that reads a graph object on the way from a record
/// to a mesh (`native_saved_mesh`, `native_graphics_traversal`,
/// `native_parametric_mesh`, `native_parametric_surface`, `native_empty_faces`,
/// `partition_instance_transforms` and this one), keys inside a field's value
/// included, and `m_id`. A decode that keeps a field reads it, and checks it, as
/// the full decode does, so a graph that is refused is refused for the same
/// reason; what is left out reads as absent. A reader added to those modules
/// that takes another field of these graphs must add its name here: Measure's
/// `ifc.sha256` of the 126 exports moves when it does not.
pub const SAVED_GRAPHICS_FIELDS: &[&str] = &[
    "m_3x3",
    "m_Envelope",
    "m_GInfo",
    "m_GRepId",
    "m_Trf",
    "m_bHasScale",
    "m_bIsNestedDetailFamily",
    "m_cda",
    "m_center",
    "m_comp",
    "m_corners",
    "m_dir",
    "m_dirVec",
    "m_elementId",
    "m_endParams",
    "m_faceFlags_v9",
    "m_faceRegions",
    "m_facetsArr",
    "m_firstAndLastEdgePnts",
    "m_flags",
    "m_forbiddenTarget",
    "m_halfAngle",
    "m_id",
    "m_instanceInfo",
    "m_interiorEdgePnts",
    "m_interiorGStyleID",
    "m_materialID",
    "m_negateDirCondition",
    "m_next",
    "m_nextLoop",
    "m_normalsArr",
    "m_normalsFlag",
    "m_oBackgroundFilling",
    "m_oConditions",
    "m_oEmbeddedSymbolGRep",
    "m_open",
    "m_or",
    "m_orientFlag",
    "m_origin",
    "m_other",
    "m_pDoc",
    "m_pFace",
    "m_pFaces",
    "m_pFacetedTopology",
    "m_pFirstLoop",
    "m_pGFilling",
    "m_pProfileCurve",
    "m_pProfileCurve1",
    "m_pProfileCurve2",
    "m_pSurf",
    "m_param",
    "m_pointsArr",
    "m_prev",
    "m_radius",
    "m_renderStyleId",
    "m_resolveSymInView",
    "m_subNodes",
    "m_symbolId",
    "m_tag",
    "m_targets",
    "m_value",
    "m_xLen",
    "m_xVec",
    "m_yLen",
    "m_yVec",
    "m_zVec",
];
/// [`SAVED_GRAPHICS_FIELDS`] for a record of any class.
fn saved_graphics_fields(_class: &str) -> Option<&'static [&'static str]> {
    Some(SAVED_GRAPHICS_FIELDS)
}
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
    /// Whether the file is of Revit 2015 to 2022, whose saved graphics no
    /// export reads: only then are they meshed by
    /// [`native_saved_mesh::graphics_pre2023`] (RE-342).
    pre2023: bool,
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
            Some(g) if run.pre2023 => native_saved_mesh::graphics_pre2023(
                g,
                &resolver,
                run.detail_level,
                run.include_hidden || run.include_filtered,
                run.include_filtered,
            ),
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
    opts.graph_fields = Some(saved_graphics_fields);
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
        pre2023: file.layout_release()? < 2023,
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
/// The bytes of record body one batch of [`extract_in_batches_with_control`]
/// may read from the shared symbols its owners name: the 20,000 decoded graph
/// objects, about 95 MiB on the BIM4LCA office (4.8 KiB an object), that a body
/// of 80 to 160 bytes an object comes to (#1694). A symbol's bytes are known
/// before it is decoded and its objects are not, and the symbols sort by id
/// with the largest last: a batch sized from the one before it, in symbols, held
/// 3 to 7 times those objects on Autodesk's samples of Revit 2023 and 2024.
const BATCH_BODY_BUDGET: usize = 2_000_000;
/// The graph objects of the owners that name nothing that are held at once
/// while they are meshed: the buffer is meshed and let go when it holds this
/// many (the same 95 MiB or so as a batch of symbols).
const OWNER_OBJECT_BUDGET: usize = 20_000;
/// An owner as a batch is planned: the first shared symbol it names (the
/// order owners naming the same one together), its id, and the shared symbols
/// it names itself.
type Named = (Option<u64>, u64, BTreeSet<u64>);
/// The bytes of the body of each record of `ids`, the shared symbols the
/// batches of [`extract_in_batches_with_control`] read, found without building
/// a graph: with [`Options::graph_classes`] empty each selected record comes
/// back with its Element base alone. An id with no record has no size.
fn body_sizes(
    file: &mut RevitFile,
    options: &Options,
    ids: BTreeSet<u64>,
) -> Result<BTreeMap<u64, usize>> {
    let mut sizes = BTreeMap::new();
    // No ids would select every record.
    if ids.is_empty() {
        return Ok(sizes);
    }
    let mut opts = options.clone();
    opts.selected_ids = ids;
    opts.graph_classes = Some(BTreeSet::new());
    native_document::extract_graphs(file, &opts, |record| {
        sizes.insert(record.identity.element_id, record.source.body_bytes);
        Ok(())
    })?;
    Ok(sizes)
}
/// Where the batch that starts at `named[start]` ends: it takes the owners after
/// it for as long as the record bodies of the symbols they name, each counted
/// once however many owners name it, stay within `budget` bytes. A batch holds
/// at least one owner whatever its symbols weigh, so that the plan always
/// advances, and the owners that name no symbol, which sort first, are a batch
/// of their own that reads none. A symbol with no size weighs nothing.
fn batch_end(named: &[Named], start: usize, sizes: &BTreeMap<u64, usize>, budget: usize) -> usize {
    let mut symbols = BTreeSet::<u64>::new();
    let mut bytes = 0usize;
    let mut end = start;
    while let Some((_, _, own)) = named.get(end) {
        let mut added = 0usize;
        for symbol in own {
            if !symbols.contains(symbol) {
                added = added.saturating_add(sizes.get(symbol).copied().unwrap_or(0));
            }
        }
        // The owners that name no symbol sort first and make a batch that
        // reads none, so that their graphs are gone before any symbol is.
        let ends_symbol_free = symbols.is_empty() && !own.is_empty();
        if end > start && (ends_symbol_free || bytes.saturating_add(added) > budget) {
            break;
        }
        symbols.extend(own);
        bytes = bytes.saturating_add(added);
        end += 1;
    }
    end
}
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
/// Mesh the owners of `buffer`, give each element to `visit` and let the graphs
/// go.
fn mesh_buffer(
    buffer: &mut BTreeMap<u64, Record>,
    run: &MeshRun<'_>,
    visit: &mut impl FnMut(SavedElement) -> Result<()>,
) -> Result<()> {
    if buffer.is_empty() {
        return Ok(());
    }
    let ids: Vec<u64> = buffer.keys().copied().collect();
    let no_shared = BTreeMap::new();
    let elements = mesh_elements(
        &ids,
        Records {
            owners: &*buffer,
            shared: &no_shared,
        },
        run,
    )?;
    for element in elements {
        visit(element)?;
    }
    buffer.clear();
    Ok(())
}
/// [`extract_at_detail_with_control`] for a caller that takes each element as
/// it is meshed and keeps none of the graphs: `visit` receives every element,
/// in no set order, and the decoded graphs of the shared symbols are freed
/// before the next ones are read.
///
/// The owners are read first for what their graphs name. Those that name
/// nothing (a wall's graph is its own geometry, about 1.6 MiB on the office's
/// walls that embed a curtain wall, #1429, and a file has hundreds) and that no
/// owner names are let go, and read again in one further pass that meshes
/// them a buffer at a time (`OWNER_OBJECT_BUDGET`), first. The others are kept,
/// as every owner was. Their symbols,
/// which are shared and which hold the geometry (on the BIM4LCA office 105
/// symbols hold 99 % of the graph objects of 538 doors and windows), are
/// decoded a batch at a time, with the owners that name them meshed against
/// the batch. A batch holds the symbols of owners next to each other by
/// symbol, as many as hold about `BATCH_BODY_BUDGET` bytes of record body,
/// which one walk reads for all of them before the first batch is cut (it
/// builds no graph). The owners that name only other owners are meshed first
/// among them, in a
/// batch that reads no symbol of its own but one that an owner it names
/// names. An owner that names nothing and that another owner names is read
/// with the batch that needs it, and kept as the others named are, though its
/// graph is not counted in the batch.
///
/// An owner's graph is freed with its batch, unless another owner names that
/// owner: its graph is then kept for the mesh of the one that names it. The
/// one-shot read has every owner in hand, so a shared symbol that names an
/// owner already freed has that owner read again with it, as one of its
/// shared records, and the mesh finds it as it finds it in the one-shot read.
/// Each element is the one [`extract_at_detail_with_control`] makes; only the
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
    opts.graph_fields = Some(saved_graphics_fields);
    // Everything each owner's graph names. An owner can name another owner,
    // whose own symbols its mesh then reaches, as the one-shot read finds them
    // by reading every record's references.
    let mut owner_references = BTreeMap::<u64, Vec<u64>>::new();
    let mut owners = BTreeMap::<u64, Record>::new();
    native_document::extract_graphs(file, &opts, |r| {
        let id = r.identity.element_id;
        let references = instance_symbols(&r);
        let names_something = !references.is_empty();
        ensure!(
            owner_references.insert(id, references).is_none(),
            "duplicate current graphics owner"
        );
        // An owner that names nothing is not kept: its graph is its own
        // geometry (a wall's is about 1.6 MiB, #1429, and a file has hundreds),
        // and it is read again below and meshed a buffer of them at a time. One
        // that names a symbol or an owner is kept, as every owner was, for the
        // batch that meshes it with what it names.
        if names_something {
            owners.insert(id, r);
        }
        Ok(())
    })?;
    // The owners another owner names: their graphs stay for the mesh of the one
    // that names them. Every other owner's graph is freed once it is meshed.
    let named_owners: BTreeSet<u64> = owner_references
        .values()
        .flatten()
        .copied()
        .filter(|symbol| owner_references.contains_key(symbol))
        .collect();
    // The owners that name nothing, and that no owner names: meshed in one
    // further pass, before the batches.
    let streamed: BTreeSet<u64> = owner_references
        .iter()
        .filter(|(id, references)| references.is_empty() && !named_owners.contains(*id))
        .map(|(&id, _)| id)
        .collect();
    // Each of the other owners with the symbols it names that are not owners
    // themselves, ordered so that owners naming the same symbols are next to
    // each other.
    let mut named: Vec<Named> = owner_references
        .iter()
        .filter(|(id, _)| !streamed.contains(*id))
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
    let total = owner_references.len() as u64;
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
        pre2023: file.layout_release()? < 2023,
        control,
        done: &done,
        total,
    };
    // The owners that name nothing are read in one pass and meshed a buffer of
    // about 20,000 graph objects at a time, as they are read.
    if !streamed.is_empty() {
        let mut buffer = BTreeMap::<u64, Record>::new();
        let mut held = 0usize;
        opts.selected_ids = streamed;
        native_document::extract_graphs(file, &opts, |r| {
            held += r.graph.as_ref().map_or(0, |graph| graph.objects.len());
            ensure!(
                buffer.insert(r.identity.element_id, r).is_none(),
                "duplicate current graphics owner"
            );
            if held >= OWNER_OBJECT_BUDGET {
                mesh_buffer(&mut buffer, &run, &mut visit)?;
                held = 0;
            }
            Ok(())
        })?;
        mesh_buffer(&mut buffer, &run, &mut visit)?;
    }
    // What each shared symbol the batches read weighs in record body, found
    // before any is decoded: the batches are cut by it (#1694).
    control.check()?;
    let sizes = body_sizes(
        file,
        &opts,
        named
            .iter()
            .flat_map(|(_, _, own)| own.iter().copied())
            .collect(),
    )?;
    let mut next = 0;
    let mut freeing: Option<std::thread::JoinHandle<()>> = None;
    while next < named.len() {
        control.check()?;
        let end = batch_end(&named, next, &sizes, BATCH_BODY_BUDGET);
        let ids: Vec<u64> = named[next..end].iter().map(|(_, id, _)| *id).collect();
        next = end;
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
        // An owner the batch's owners name that names nothing was not kept (an
        // owner that names something was): it is read now, and kept as it is
        // named. An owner a symbol names is read with the symbols below.
        let missing: BTreeSet<u64> = expanded
            .iter()
            .copied()
            .filter(|id| !owners.contains_key(id))
            .collect();
        if !missing.is_empty() {
            control.check()?;
            opts.selected_ids = missing;
            native_document::extract_graphs(file, &opts, |r| {
                ensure!(
                    owners.insert(r.identity.element_id, r).is_none(),
                    "duplicate current graphics owner"
                );
                Ok(())
            })?;
        }
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
            // A symbol that names an owner whose graph was freed with an
            // earlier batch (no owner names it, so it was not kept): the mesh
            // looks it up among the shared records, so it is read again.
            wanted.extend(found.iter().copied().filter(|id| {
                owner_references.contains_key(id)
                    && !owners.contains_key(id)
                    && !attempted.contains(id)
            }));
            add_references(
                found,
                &owner_references,
                &mut expanded,
                &attempted,
                &mut wanted,
            );
        }
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
        // The batch's owners are meshed: their graphs go with its symbols,
        // those another owner names excepted.
        let meshed: Vec<Record> = ids
            .iter()
            .filter(|id| !named_owners.contains(*id))
            .filter_map(|id| owners.remove(id))
            .collect();
        // Freeing the decoded graphs takes seconds on large models: do it
        // aside where there are threads.
        if workers() > 1 {
            freeing = Some(std::thread::spawn(move || {
                drop(shared);
                drop(meshed);
            }));
        }
    }
    Ok(())
}
// The batch plan is a pure function of the owners, the symbols' sizes and the
// budget: the ways it can fail are written first (#1694).
#[cfg(test)]
mod tests {
    use super::*;

    fn owner(id: u64, symbols: &[u64]) -> Named {
        (
            symbols.first().copied(),
            id,
            symbols.iter().copied().collect(),
        )
    }

    fn sizes(entries: &[(u64, usize)]) -> BTreeMap<u64, usize> {
        entries.iter().copied().collect()
    }

    #[test]
    fn an_owner_over_the_budget_is_a_batch_of_its_own() {
        let named = [owner(10, &[1]), owner(11, &[2])];
        let weights = sizes(&[(1, 5_000), (2, 5_000)]);
        assert_eq!(batch_end(&named, 0, &weights, 100), 1);
        assert_eq!(batch_end(&named, 1, &weights, 100), 2);
    }

    #[test]
    fn a_zero_budget_still_advances() {
        let named = [owner(1, &[1]), owner(2, &[2])];
        let weights = sizes(&[(1, 1), (2, 1)]);
        assert_eq!(batch_end(&named, 0, &weights, 0), 1);
    }

    #[test]
    fn a_symbol_two_owners_name_weighs_once() {
        let named = [owner(1, &[7]), owner(2, &[7])];
        let weights = sizes(&[(7, 600)]);
        assert_eq!(batch_end(&named, 0, &weights, 700), 2);
    }

    #[test]
    fn a_symbol_with_no_size_weighs_nothing() {
        let named = [owner(1, &[7]), owner(2, &[8]), owner(3, &[9])];
        assert_eq!(batch_end(&named, 0, &BTreeMap::new(), 0), 3);
    }

    #[test]
    fn sizes_that_overflow_do_not_wrap() {
        let named = [owner(1, &[1]), owner(2, &[2])];
        let weights = sizes(&[(1, usize::MAX), (2, usize::MAX)]);
        assert_eq!(batch_end(&named, 0, &weights, usize::MAX - 1), 1);
    }

    #[test]
    fn owners_naming_no_symbol_are_a_batch_of_their_own() {
        let named = [owner(1, &[]), owner(2, &[]), owner(3, &[5])];
        let weights = sizes(&[(5, 1)]);
        assert_eq!(batch_end(&named, 0, &weights, 100), 2);
        assert_eq!(batch_end(&named, 2, &weights, 100), 3);
    }

    #[test]
    fn the_budget_is_a_ceiling_the_batch_may_reach() {
        let named = [owner(1, &[1]), owner(2, &[2]), owner(3, &[3])];
        let weights = sizes(&[(1, 50), (2, 50), (3, 50)]);
        assert_eq!(batch_end(&named, 0, &weights, 100), 2);
        assert_eq!(batch_end(&named, 0, &weights, 99), 1);
    }
}
