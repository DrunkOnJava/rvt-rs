//! Inspect a structurally registered global root without semantic normalization.
use rvt::{RevitFile, native_document, native_parameters, schema_registry};
fn main() -> anyhow::Result<()> {
    let args: Vec<_> = std::env::args().collect();
    anyhow::ensure!(args.len() == 3, "expected FILE STREAM");
    let mut file = RevitFile::open(&args[1])?;
    let registry =
        schema_registry::parse(&native_document::read_single(&mut file, "Formats/Latest")?)?;
    let bytes = native_document::read_single(&mut file, &args[2])?;
    anyhow::ensure!(bytes.len() >= 2, "truncated root");
    let tag = u16::from_le_bytes(bytes[..2].try_into()?);
    let root = native_parameters::decode_object_fields(&bytes, &registry, 2, tag)?;
    println!("{}", serde_json::to_string_pretty(&root)?);
    Ok(())
}
