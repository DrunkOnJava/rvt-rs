//! Structurally decode one inflated Formats/Latest member (no native-value claim).
fn main() -> anyhow::Result<()> {
    let args: Vec<_> = std::env::args_os().collect();
    anyhow::ensure!(args.len() == 3, "INFLATED_SCHEMA OUTPUT.json");
    let registry = rvt::schema_registry::parse(&std::fs::read(&args[1])?)?;
    std::fs::write(&args[2], serde_json::to_vec_pretty(&registry)?)?;
    println!(
        "{} definitions, {} references, {} bytes consumed",
        registry.classes.len(),
        registry.reference_count,
        registry.consumed_bytes
    );
    Ok(())
}
