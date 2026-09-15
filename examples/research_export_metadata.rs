//! Export native metadata without converter sidecars or caption/value witnesses.
use anyhow::{Result, ensure};
use serde_json::json;
use sha2::{Digest, Sha256};
fn main() -> Result<()> {
    let args = std::env::args().collect::<Vec<_>>();
    ensure!(
        args.len() == 3,
        "usage: research_export_metadata RVT NEW_OUTPUT_JSON"
    );
    let mut file = rvt::RevitFile::open(&args[1])?;
    let registry = rvt::schema_registry::parse(&rvt::native_document::read_single(
        &mut file,
        "Formats/Latest",
    )?)?;
    let history = rvt::native_document::read_single(&mut file, "Global/History")?;
    let episodes = rvt::native_index::creation_episodes(&history, &registry)?;
    let mut builder = rvt::native_export_metadata::Builder::with_history(
        episodes,
        format!("{:x}", Sha256::digest(&history)),
    );
    builder.set_partition_table(
        &rvt::native_document::read_single(&mut file, "Global/PartitionTable")?,
        &registry,
    )?;
    builder.set_global_catalog(
        &rvt::native_document::read_single(&mut file, "Global/Latest")?,
        &registry,
    )?;
    let mut phase_catalog = None;
    let mut phases = std::collections::BTreeMap::new();
    let options = rvt::native_document::Options {
        channels: std::collections::BTreeSet::from([101, 102, 103]),
        max_graph_values: 10_000_000,
        max_graph_objects: 1_000_000,
        ..Default::default()
    };
    let mut material_resolver = rvt::native_saved_materials::Resolver::ddc_viewport_profile();
    let mut categories = std::collections::BTreeMap::new();
    let mut category_sources = std::collections::BTreeMap::new();
    let mut owners = std::collections::BTreeMap::new();
    let mut owner_sources = std::collections::BTreeMap::new();
    let mut graphics = std::collections::BTreeMap::new();
    let mut graphics_sources = std::collections::BTreeMap::new();
    let coverage = rvt::native_document::extract(&mut file, &options, |record| {
        builder.ingest(&record)?;
        material_resolver.ingest(&record)?;
        if record.channel == 101 {
            if let Some(root) = record.graph.as_ref().and_then(|g| g.objects.first()) {
                if let Some(v) = root
                    .fields
                    .get("m_categroryId")
                    .and_then(|v| rvt::native_export_metadata::native_reference_id(v).ok())
                {
                    categories.insert(record.identity.element_id, v);
                    category_sources.insert(record.identity.element_id,json!({"owner_id":record.identity.element_id,"body_sha256":record.source.body_sha256,"stream":record.source.stream,"channel":101,"source_field":"ElementHeader.m_categroryId"}));
                }
            }
        }
        if record.channel == 102 {
            if let Some(root) = record.graph.as_ref().and_then(|g| g.objects.first()) {
                if root.class_name == "ProjectPhase" {
                    phases.insert(record.identity.element_id,json!({"owner_id":record.identity.element_id,"body_sha256":record.source.body_sha256}));
                }
                if root.class_name == "AllProjectPhases" {
                    ensure!(phase_catalog.is_none(), "duplicate phase catalog");
                    let ids = root
                        .fields
                        .get("m_phaseIds")
                        .and_then(serde_json::Value::as_array)
                        .ok_or_else(|| anyhow::anyhow!("phase list absent"))?
                        .iter()
                        .map(rvt::native_export_metadata::native_reference_id)
                        .collect::<Result<Vec<_>>>()?;
                    ensure!(
                        ids.iter().all(|id| *id > 0)
                            && ids.iter().collect::<std::collections::BTreeSet<_>>().len()
                                == ids.len(),
                        "invalid phase catalog"
                    );
                    phase_catalog = Some((
                        ids,
                        json!({"owner_id":record.identity.element_id,"body_sha256":record.source.body_sha256,"source_field":"AllProjectPhases.m_phaseIds last entry"}),
                    ));
                }
            }
            if let Some(graph) = record.graph.as_ref().filter(|g| {
                g.objects.first().is_some_and(|r| {
                    matches!(
                        r.class_name.as_str(),
                        "SWall"
                            | "Floor"
                            | "FamilyInstance"
                            | "FamilySymbol"
                            | "Family"
                            | "DirectShapeType"
                    )
                })
            }) {
                owner_sources.insert(record.identity.element_id,json!({"owner_id":record.identity.element_id,"body_sha256":record.source.body_sha256,"stream":record.source.stream,"channel":102}));
                owners.insert(record.identity.element_id, graph.clone());
            }
        }
        if record.channel == 103 {
            if let Some(graph) = record.graph {
                graphics_sources.insert(record.identity.element_id,json!({"owner_id":record.identity.element_id,"body_sha256":record.source.body_sha256,"stream":record.source.stream,"channel":103}));
                graphics.insert(record.identity.element_id, graph);
            }
        }
        Ok(())
    })?;
    if let Some((ids, source)) = &phase_catalog {
        ensure!(
            ids.iter().all(|id| phases.contains_key(&(*id as u64))),
            "phase catalog references missing current phase"
        );
        if let Some(&phase) = ids.last() {
            for (&id, owner) in &owners {
                if let Ok((true, visibility)) =
                    rvt::native_standard_view::evaluate_created_standard_3d(owner, phase)
                {
                    builder.add_standard_view_positive(id,json!({"visibility":visibility,"phase_catalog":source,"phase":phases.get(&(phase as u64)),"owner":owner_sources.get(&id)}))?;
                }
            }
        }
    }
    for (&id, graph) in &graphics {
        let meshes = rvt::native_saved_mesh::graphics_with_resolver_at_detail(
            graph,
            &|id| graphics.get(&id),
            1,
        );
        let mut metrics = rvt::native_saved_metrics::compute(&meshes);
        metrics.sources.push(json!({"source_field":"qualified_native_metric_detail_profile","detail_level":1,"interpretation":"Area forces coarse; positive coarse volume precedes finer levels; no absent-to-zero conversion"}));
        if metrics.volume.is_none()
            && owners
                .get(&id)
                .is_some_and(|g| g.objects.first().is_some_and(|o| o.class_name == "Floor"))
        {
            metrics.volume = metrics.closed_chain_volume;
        }
        if !meshes.default_profile_witnesses.is_empty() {
            metrics.volume = None;
        }
        let area = owners.get(&id).and_then(|owner| {
            if owner.objects[0].class_name == "Floor" {
                rvt::native_saved_metrics::floor_area(owner, graph, &meshes).ok()
            } else if owner.objects[0].class_name == "SWall" {
                if owner.objects.iter().any(|o|o.class_name=="CWallGridFaceGStep") {rvt::native_saved_metrics::curtain_wall_area(owner,graph,&meshes).ok()} else {rvt::native_saved_metrics::straight_wall_area(owner, graph, &meshes).ok()}
            } else if owner.objects[0].class_name == "FamilyInstance" {
                let symbol_id = u64::try_from(
                    rvt::native_export_metadata::native_reference_id(
                        owner.objects[0].fields.get("m_masterSymbolId")?,
                    )
                    .ok()?,
                )
                .ok()?;
                let symbol = owners.get(&symbol_id)?;
                let family_id = u64::try_from(
                    rvt::native_export_metadata::native_reference_id(
                        symbol.objects[0].fields.get("m_familyId")?,
                    )
                    .ok()?,
                )
                .ok()?;
                let family = owners.get(&family_id)?;
                rvt::native_saved_metrics::ordinary_family_area(
                    symbol,
                    family,
                    *categories.get(&symbol_id)?,
                    graph,
                    &|id| graphics.get(&id),
                    &meshes,
                )
                .ok().map(|(value,source)|(value,json!({"measurement":source,"symbol":owner_sources.get(&symbol_id),"family":owner_sources.get(&family_id),"symbol_category":category_sources.get(&symbol_id)})))
            } else {
                None
            }
        });
        if metrics.volume.is_some() || area.is_some() {
            if let Some((_, source)) = &area {
                metrics.sources.push(source.clone());
                if let Some(source) = owner_sources.get(&id) {
                    metrics.sources.push(source.clone());
                }
            }
            metrics.sources.push(graphics_sources[&id].clone());
            for dependency in meshes
                .primitives
                .iter()
                .filter_map(|p| p.source_owner_id)
                .collect::<std::collections::BTreeSet<_>>()
            {
                if let Some(source) = graphics_sources.get(&dependency) {
                    metrics.sources.push(source.clone());
                }
            }
            builder.add_derived_metrics(
                id,
                metrics.volume,
                area.as_ref().map(|v| v.0),
                metrics.sources,
            )?;
        }
        if let Some(owner) = owners
            .get(&id)
            .filter(|g| g.objects[0].class_name == "DirectShapeType")
        {
            if let Ok((areas, source)) =
                rvt::native_saved_material_quantities::direct_shape_type_areas(
                    owner, graph, &meshes,
                )
            {
                for (material, area) in areas {
                    builder.add_material_area_only(
                        id,
                        material,
                        area,
                        vec![
                            source.clone(),
                            graphics_sources[&id].clone(),
                            owner_sources[&id].clone(),
                        ],
                    )?;
                }
            }
            continue;
        }
        if let (Some(owner), Some(layers)) = (
            owners
                .get(&id)
                .filter(|g| g.objects[0].class_name == "Floor"),
            builder.material_layer_widths(id),
        ) {
            if layers.len() > 1 {
                if let Ok((quantities, source)) =
                    rvt::native_saved_material_quantities::constant_layers_floor_report(
                        owner, graph, &meshes, &layers,
                    )
                {
                    for (material, (area, volume)) in quantities {
                        builder.add_material_quantities(
                            id,
                            material,
                            area,
                            volume,
                            vec![
                                source.clone(),
                                graphics_sources[&id].clone(),
                                owner_sources[&id].clone(),
                            ],
                        )?;
                    }
                }
                continue;
            }
        }
        if let Some(category) = categories.get(&id) {
            let materials = meshes
                .primitives
                .iter()
                .map(|p| {
                    material_resolver.resolve_quantity_material(
                        id,
                        p.source_owner_id,
                        p.face_tag,
                        p.render_style_id,
                        p.material_id,
                        *category,
                    )
                })
                .collect::<Vec<_>>();
            let quantities = rvt::native_saved_material_quantities::compute(&meshes, &|i, _| {
                materials[i].as_ref().map(|m| m.material_id)
            });
            if owners
                .get(&id)
                .is_some_and(|g| g.objects[0].class_name == "FamilySymbol")
            {
                eprintln!(
                    "{}",
                    json!({"quantity_owner":id,"category":category,"primitive_count":meshes.primitives.len(),"unresolved_primitives":quantities.unresolved_primitives,"diagnostics":quantities.diagnostics,"materials":quantities.materials})
                );
            }
            let host = owners
                .get(&id)
                .is_some_and(|o| matches!(o.objects[0].class_name.as_str(), "SWall" | "Floor"));
            for (&material_id, q) in &quantities.materials {
                let mut material_area = if host && quantities.materials.len() == 1 {
                    area.as_ref().map(|v| v.0)
                } else if !host {
                    q.surface_area
                } else {
                    None
                };
                let mut material_volume = q.closed_chain_volume.or(q.volume);
                let mut sampled_source = None;
                if quantities.materials.len() == 1 {
                    if let (Some(owner), Some((width, width_source))) =
                        (owners.get(&id), builder.single_layer_width(id))
                    {
                        if owner.objects[0].class_name == "SWall" {
                            if let Ok((a, v, source)) =
                                rvt::native_saved_material_quantities::single_layer_wall_report(
                                    owner,
                                    graph,
                                    &meshes,
                                    width,
                                    width_source,
                                )
                            {
                                material_area = Some(a);
                                material_volume = Some(v);
                                sampled_source = Some(source);
                            } else {
                                material_area = None;
                                material_volume = None;
                            }
                        }
                    }
                }
                if let (Some(a), Some(v)) = (material_area, material_volume) {
                    let mut sources = q.sources.clone();
                    if let Some(source) = sampled_source {
                        sources.push(source);
                    }
                    sources.extend(quantities.sources.clone());
                    sources.push(graphics_sources[&id].clone());
                    sources.extend(
                        materials
                            .iter()
                            .flatten()
                            .filter(|m| m.material_id == material_id)
                            .flat_map(|m| m.provenance.clone()),
                    );
                    if let Some((_, source)) = area.as_ref().filter(|_| host) {
                        sources.push(source.clone());
                    }
                    builder.add_material_quantities(id, material_id, a, v, sources)?;
                }
            }
        }
    }
    ensure!(
        coverage.revit_version == 2024,
        "exporter label qualification currently Revit2024 only"
    );
    let source_sha256 = format!("{:x}", Sha256::digest(std::fs::read(&args[1])?));
    let report =
        json!({"source_sha256":source_sha256,"inventory":builder.finish(),"coverage":coverage});
    serde_json::to_writer(
        std::fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&args[2])?,
        &report,
    )?;
    Ok(())
}
