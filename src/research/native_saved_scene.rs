//! Current native saved-graphics extraction with explicit shared-symbol closure.
use crate::{
    RevitFile,
    control::{Stage, WalkerControl},
    native_document::{self, Options, Record, Summary},
    native_saved_mesh::{self, GraphicsMeshes},
};
use anyhow::{Result, ensure};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};
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
pub fn extract(file: &mut RevitFile, options: &Options) -> Result<SavedScene> {
    extract_at_detail(file, options, 3)
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
    ensure!(
        (1..=3).contains(&detail_level),
        "detail level must be 1, 2 or 3"
    );
    control.check()?;
    control.report(Stage::SavedGraphics, 0, None);
    let mut opts = options.clone();
    opts.channels = BTreeSet::from([103]);
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
            if let Some(g) = &r.graph {
                for o in &g.objects {
                    if o.class_name == "InstanceInfo" {
                        if let Some(id) =
                            crate::native_metadata::identifier(&o.fields["m_symbolId"])
                                .ok()
                                .and_then(|id| u64::try_from(id).ok())
                                .filter(|id| *id > 0)
                        {
                            if !attempted.contains(&id) {
                                wanted.insert(id);
                            }
                        }
                    }
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
    // Elements mesh independently: spread them over the cores, keeping order.
    // The browser build (wasm32-unknown-unknown) has no threads, so there, as
    // on a single core, they mesh one after the other.
    let roots: Vec<u64> = roots.into_iter().collect();
    let total = roots.len() as u64;
    control.report(Stage::SavedMeshes, 0, Some(total));
    let mesh = |id: u64| -> Result<SavedElement> {
        let r = &records[&id];
        let mut meshes = match &r.graph {
            Some(g) => native_saved_mesh::graphics_with_resolver_at_detail(
                g,
                &|id| records.get(&id).and_then(|r| r.graph.as_ref()),
                detail_level,
            ),
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
            .map(|id| Ok((id, serde_json::to_value(&records[&id].source)?)))
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
    // Counted under a lock so the reports stay in order across workers.
    let done = std::sync::Mutex::new(0u64);
    let build = |id: u64| -> Result<SavedElement> {
        control.check()?;
        let element = mesh(id);
        let mut done = done.lock().unwrap_or_else(|e| e.into_inner());
        *done += 1;
        control.report(Stage::SavedMeshes, *done, Some(total));
        element
    };
    let workers = if cfg!(all(target_arch = "wasm32", target_os = "unknown")) {
        1
    } else {
        std::thread::available_parallelism().map_or(1, |n| n.get())
    };
    let elements = if workers == 1 {
        roots
            .iter()
            .map(|&id| build(id))
            .collect::<Result<Vec<_>>>()?
    } else {
        let next = std::sync::atomic::AtomicUsize::new(0);
        let mut built: Vec<(usize, Result<SavedElement>)> = std::thread::scope(|scope| {
            let handles: Vec<_> = (0..workers)
                .map(|_| {
                    scope.spawn(|| {
                        let mut out = Vec::new();
                        loop {
                            let i = next.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                            let Some(&id) = roots.get(i) else { break };
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
            .collect::<Result<Vec<_>>>()?
    };
    // Freeing the decoded graphs takes seconds on large models: do it aside
    // where there are threads.
    if workers > 1 {
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
