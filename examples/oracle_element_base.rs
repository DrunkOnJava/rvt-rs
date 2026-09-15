//! Decode a bounded native body against a structurally parsed registry.
fn main() -> anyhow::Result<()> {
    let args: Vec<_> = std::env::args_os().collect();
    anyhow::ensure!(
        args.len() == 3 || args.len() == 4,
        "INFLATED_SCHEMA BOUNDED_BODY [--parameters]"
    );
    let registry = rvt::schema_registry::parse(&std::fs::read(&args[1])?)?;
    if args.len() == 4 {
        if args[3] == "--graph" {
            let graph = rvt::native_parameters::decode_graph(&std::fs::read(&args[2])?, &registry)?;
            println!("{}", serde_json::to_string_pretty(&graph)?);
            return Ok(());
        }
        if let Some(arg) = args[3].to_str().and_then(|s| s.strip_prefix("--object=")) {
            let (tag, offset) = arg
                .split_once('@')
                .ok_or_else(|| anyhow::anyhow!("--object=CLASS_TAG@BODY_OFFSET"))?;
            let values = rvt::native_parameters::decode_object_fields(
                &std::fs::read(&args[2])?,
                &registry,
                offset.parse()?,
                tag.parse()?,
            )?;
            println!("{}", serde_json::to_string_pretty(&values)?);
            return Ok(());
        }
        anyhow::ensure!(args[3] == "--parameters", "unknown option");
        let values = rvt::native_parameters::decode(&std::fs::read(&args[2])?, &registry)?;
        println!("{}", serde_json::to_string_pretty(&values)?);
        return Ok(());
    }
    let base = rvt::native_element::decode_base(&std::fs::read(&args[2])?, &registry)?;
    println!("{}", serde_json::to_string_pretty(&base)?);
    Ok(())
}
