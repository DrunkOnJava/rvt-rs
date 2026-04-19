use rvt::writer;
use rvt::RevitFile;
use std::path::PathBuf;

fn main() -> anyhow::Result<()> {
    let src = PathBuf::from("../../samples/_phiag/examples/Autodesk/racbasicsamplefamily-2024.rfa");
    let dst = std::env::temp_dir().join("rvt-rs-roundtrip.rfa");
    writer::copy_file(&src, &dst)?;

    let mut a = RevitFile::open(&src)?;
    let mut b = RevitFile::open(&dst)?;
    let mut ok = 0;
    let mut mismatch = 0;
    for name in a.stream_names() {
        let ba = a.read_stream(&name)?;
        let bb = b.read_stream(&name)?;
        if ba == bb { ok += 1; } else { mismatch += 1; println!("DIFF: {name} ({} vs {} bytes)", ba.len(), bb.len()); }
    }
    println!("round-trip check: {ok} streams identical, {mismatch} mismatches");
    std::fs::remove_file(dst)?;
    Ok(())
}
