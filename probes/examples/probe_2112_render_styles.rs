//! #2112: does a saved part's render style say which part of a door, window or lamp is its
//! glass?
//!
//! FACT under test: each face of a saved mesh holds `m_renderStyleId`
//! (`native_saved_mesh::Primitive::render_style_id`). #2112 found that the parts name no
//! material of their own on the reader's face path, and that their geometry tags are not keys
//! of their type's material map. If a part's render style is a material's ElementId, an
//! element whose parts hold two or more styles can be drawn part by part: its glass
//! see-through and its frame solid. This counts, for the doors, windows and lighting fixtures
//! of a model whose saved graphics decode, how many have one style across their visible parts
//! and how many two or more; which records the style ids are (their class, or whether the
//! file declares them at all); and, where a style is a material, its name.
//!
//! Output, one model per run:
//!   file revit=<release> elements=<n>
//!   styles category=<door|window|light> elements=<n> one_style=<n> several_styles=<n> parts=<n> parts_without_style=<n>
//!   style class=<record class | none(<id>) | declared, no record | undeclared> ids=<n> parts=<n>
//!   style material name=<name> parts=<n>   (the 40 with the most parts)
//!   element tag=<ElementId> category=<door|window|light> parts=<n> materials=<name>|<name>...
//!     the names of the materials its visible parts' styles are, `?<id>` for a style the file
//!     names no material for; `tools/re/render_styles_vs_ifc.py` sets them beside the surface
//!     styles Revit's export gives the element (Revit names a surface style after its material).
//!   area<TAB><ElementId><TAB><material name><TAB><square feet>
//!     for each material an element's visible parts name, the summed area of those parts'
//!     triangles, so that the see-through area of an element can be set beside the area
//!     Revit's export draws with a see-through style (`tools/re/render_styles_vs_ifc.py`).
//!
//! Usage:
//!   cargo run --profile ci -p rvt-probes --example probe_2112_render_styles -- MODEL.rvt

use std::collections::{BTreeMap, BTreeSet};

use anyhow::Result;
use rvt::RevitFile;
use rvt::research::native_document::{Options, extract_graphs};
use rvt::research::native_saved_scene::extract_at_detail;
use rvt::research::partition_element_records as per;

fn main() -> Result<()> {
    let path = std::env::args().nth(1).expect("usage: MODEL.rvt");
    let mut rf = RevitFile::open(&path)?;
    let release = rf.basic_file_info()?.version;
    let declared = rvt::elem_table::declared_ids(&rvt::elem_table::parse_records(&mut rf)?);
    let categories = [per::OST_DOORS, per::OST_WINDOWS, per::OST_LIGHTING_FIXTURES];
    let records = per::scan_category_records_multi(&mut rf, release, &categories, &declared)?;
    let mut category: BTreeMap<u64, &str> = BTreeMap::new();
    for record in records.iter().filter(|r| r.is_exported_instance()) {
        let name = match record.builtin_category {
            per::OST_DOORS => "door",
            per::OST_WINDOWS => "window",
            _ => "light",
        };
        category.entry(u64::from(record.element_id)).or_insert(name);
    }
    let names =
        rvt::research::partition_materials::scan_material_names(&mut rf, release, &declared)
            .unwrap_or_default();
    println!("file revit={release} elements={}", category.len());
    if category.is_empty() {
        return Ok(());
    }
    let options = Options {
        selected_ids: category.keys().copied().collect(),
        max_graph_values: 1_000_000,
        max_graph_objects: 1_000_000,
        ..Options::default()
    };
    let scene = match extract_at_detail(&mut rf, &options, 3) {
        Ok(scene) => scene,
        Err(error) => {
            println!("saved graphics: {error}");
            return Ok(());
        }
    };
    // Per category: elements with visible parts, one style, two or more, parts, parts with
    // no style (an id of 0 or below).
    let mut tally: BTreeMap<&str, [usize; 5]> = BTreeMap::new();
    let mut style_parts: BTreeMap<i64, usize> = BTreeMap::new();
    for element in &scene.elements {
        let Some(cat) = category.get(&element.identity.element_id) else {
            continue;
        };
        let parts: Vec<_> = element
            .meshes
            .primitives
            .iter()
            .filter(|p| !p.hidden && !p.filtered)
            .collect();
        if parts.is_empty() {
            continue;
        }
        let styles: BTreeSet<i64> = parts.iter().map(|p| p.render_style_id).collect();
        let entry = tally.entry(cat).or_default();
        entry[0] += 1;
        entry[if styles.len() == 1 { 1 } else { 2 }] += 1;
        entry[3] += parts.len();
        entry[4] += parts.iter().filter(|p| p.render_style_id <= 0).count();
        for part in &parts {
            *style_parts.entry(part.render_style_id).or_default() += 1;
        }
        let materials: Vec<String> = styles
            .iter()
            .map(|id| {
                u32::try_from(*id)
                    .ok()
                    .and_then(|id| names.get(&id))
                    .cloned()
                    .unwrap_or_else(|| format!("?{id}"))
            })
            .collect();
        println!(
            "element tag={} category={cat} parts={} materials={}",
            element.identity.element_id,
            parts.len(),
            materials.join("|")
        );
        let mut areas: BTreeMap<String, f64> = BTreeMap::new();
        for part in &parts {
            let name = u32::try_from(part.render_style_id)
                .ok()
                .and_then(|id| names.get(&id))
                .cloned()
                .unwrap_or_else(|| format!("?{}", part.render_style_id));
            let corner = |i: u32| part.vertices.get(i as usize).copied();
            let area: f64 = part
                .triangles
                .iter()
                .filter_map(|t| Some((corner(t[0])?, corner(t[1])?, corner(t[2])?)))
                .map(|(a, b, c)| {
                    let (u, v) = (
                        [b[0] - a[0], b[1] - a[1], b[2] - a[2]],
                        [c[0] - a[0], c[1] - a[1], c[2] - a[2]],
                    );
                    let n = [
                        u[1] * v[2] - u[2] * v[1],
                        u[2] * v[0] - u[0] * v[2],
                        u[0] * v[1] - u[1] * v[0],
                    ];
                    (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt() / 2.0
                })
                .sum();
            *areas.entry(name).or_default() += area;
        }
        for (name, area) in &areas {
            println!("area\t{}\t{name}\t{area:.6}", element.identity.element_id);
        }
    }
    for (cat, [elements, one, several, parts, unstyled]) in &tally {
        println!(
            "styles category={cat} elements={elements} one_style={one} several_styles={several} parts={parts} parts_without_style={unstyled}"
        );
    }
    let wanted: BTreeSet<u64> = style_parts
        .keys()
        .filter_map(|id| u64::try_from(*id).ok())
        .filter(|id| *id > 0)
        .collect();
    let mut classes: BTreeMap<u64, String> = BTreeMap::new();
    if !wanted.is_empty() {
        extract_graphs(
            &mut rf,
            &Options {
                selected_ids: wanted,
                ..Options::default()
            },
            |record| {
                classes.insert(
                    record.identity.element_id,
                    record.class_name.clone().unwrap_or_default(),
                );
                Ok(())
            },
        )?;
    }
    let mut by_class: BTreeMap<String, (usize, usize)> = BTreeMap::new();
    let mut by_name: BTreeMap<&str, usize> = BTreeMap::new();
    for (id, parts) in &style_parts {
        let class = match u64::try_from(*id).ok().filter(|id| *id > 0) {
            None => format!("none({id})"),
            Some(id) => match classes.get(&id) {
                Some(class) => class.clone(),
                None if u32::try_from(id).is_ok_and(|id| declared.contains(&id)) => {
                    "declared, no record".to_string()
                }
                None => "undeclared".to_string(),
            },
        };
        let entry = by_class.entry(class).or_default();
        entry.0 += 1;
        entry.1 += parts;
        if let Some(name) = u32::try_from(*id).ok().and_then(|id| names.get(&id)) {
            *by_name.entry(name.as_str()).or_default() += parts;
        }
    }
    for (class, (ids, parts)) in &by_class {
        println!("style class={class} ids={ids} parts={parts}");
    }
    let mut named: Vec<(&str, usize)> = by_name.into_iter().collect();
    named.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(b.0)));
    for (name, parts) in named.into_iter().take(40) {
        println!("style material name={name:?} parts={parts}");
    }
    Ok(())
}
