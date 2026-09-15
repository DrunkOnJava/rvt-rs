use rvt::{
    native_parameters::ObjectGraph, native_saved_material_quantities::compute,
    native_saved_mesh::graphics_with_resolver_at_detail,
};
use serde_json::json;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let a: Vec<_> = std::env::args().collect();
    let g: ObjectGraph = serde_json::from_reader(std::fs::File::open(&a[1])?)?;
    let mut rows = vec![];
    for detail in 1..=3 {
        let meshes = graphics_with_resolver_at_detail(&g, &|_| None, detail);
        let metrics = compute(&meshes, &|_, p| Some(p.render_style_id));
        rows.push(json!({"detail":detail,"metrics":metrics,"selection":{
   "diagnostics":meshes.diagnostics,"rejected_filters":meshes.rejected_filters,
   "excluded_visibility":meshes.excluded_visibility_branches,"primitives":meshes.primitives.len()}}));
    }
    serde_json::to_writer_pretty(
        std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&a[2])?,
        &rows,
    )?;
    Ok(())
}
