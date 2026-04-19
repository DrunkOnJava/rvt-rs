//! Show the 2 bytes of Global/PartitionTable that DO differ across
//! releases and report what each release writes there.
use rvt::{RevitFile, compression, streams::GLOBAL_PARTITION_TABLE};
use std::path::PathBuf;

fn main() -> anyhow::Result<()> {
    let sample_dir = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "samples/_phiag/examples/Autodesk".to_string());

    println!("Release | bytes[0..2] hex | interpreted u16 LE");
    for v in 2016..=2026 {
        for filename in [
            format!("racbasicsamplefamily-{v}.rfa"),
            format!("rac_basic_sample_family-{v}.rfa"),
        ] {
            let path = PathBuf::from(&sample_dir).join(&filename);
            if !path.exists() {
                continue;
            }
            let mut rf = RevitFile::open(&path)?;
            let raw = rf.read_stream(GLOBAL_PARTITION_TABLE)?;
            let d = compression::inflate_at(&raw, 8)?;
            if d.len() >= 2 {
                let b0 = d[0];
                let b1 = d[1];
                let u16v = u16::from_le_bytes([b0, b1]);
                println!(" {v}   |   {b0:02x} {b1:02x}      | {u16v}");
            }
            break;
        }
    }
    Ok(())
}
