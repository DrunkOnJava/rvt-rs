//! Strict bounded dynamic-entity research from a file-derived catalog.
use anyhow::{Result, ensure};
fn main() -> Result<()> {
    let a: Vec<_> = std::env::args().collect();
    ensure!(a.len() == 5, "BODY SCHEMA CATALOG OUTPUT");
    let body = std::fs::read(&a[1])?;
    let registry = rvt::schema_registry::parse(&std::fs::read(&a[2])?)?;
    let catalog: rvt::native_extensible_storage::Catalog =
        serde_json::from_slice(&std::fs::read(&a[3])?)?;
    let result = rvt::native_parameters::decode_graph_with_catalog(
        &body,
        &registry,
        &Default::default(),
        Some(&catalog),
    );
    std::fs::write(
        &a[4],
        serde_json::to_vec_pretty(
            &serde_json::json!({"graph":result.as_ref().ok(),"error":result.as_ref().err().map(|e|format!("{e:#}"))}),
        )?,
    )?;
    Ok(())
}
