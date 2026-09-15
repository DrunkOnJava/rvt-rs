use anyhow::{Result, ensure};
fn main() -> Result<()> {
    let a: Vec<_> = std::env::args().collect();
    ensure!(a.len() == 3, "FILE STREAM");
    let mut f = rvt::RevitFile::open(&a[1])?;
    let reg = rvt::schema_registry::parse(&rvt::native_document::read_single(
        &mut f,
        "Formats/Latest",
    )?)?;
    let b = rvt::native_document::read_single(&mut f, &a[2])?;
    let tail = if a[2] == "Global/Latest" { 4 } else { 0 };
    ensure!(
        b.len() >= tail + 2,
        "global stream shorter than root and suffix"
    );
    if tail != 0 {
        ensure!(
            b[b.len() - tail..] == [0, 0, 0, 0],
            "unqualified Global/Latest suffix"
        );
    }
    let body = &b[..b.len() - tail];
    let graph = rvt::native_parameters::decode_graph_with_limits(
        body,
        &reg,
        &rvt::native_parameters::GraphLimits {
            max_values: 1_000_000,
            max_objects: 100_000,
            max_depth: 128,
        },
    )?;
    println!(
        "{}",
        serde_json::to_string(
            &serde_json::json!({"graph":graph,"opaque_uninterpreted_suffix":&b[b.len()-tail..]})
        )?
    );
    Ok(())
}
