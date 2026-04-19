// Dump the raw 32 bytes immediately around each class-name occurrence in
// Formats/Latest so we can learn the tag encoding.
use rvt::{compression, streams::FORMATS_LATEST, RevitFile};

fn main() -> anyhow::Result<()> {
    let path = std::env::args().nth(1).expect("path");
    let mut rf = RevitFile::open(&path)?;
    let raw = rf.read_stream(FORMATS_LATEST)?;
    let decomp = compression::inflate_at(&raw, 0)?;
    for name in ["ADocument", "DBView", "Symbol", "HostObj", "Category"] {
        let bytes = name.as_bytes();
        if let Some(pos) = decomp.windows(bytes.len()).position(|w| w == bytes) {
            let start = pos.saturating_sub(4);
            let end = (pos + bytes.len() + 16).min(decomp.len());
            println!("\n{name} @ {pos} (0x{pos:x})");
            print!("  pre: ");
            for b in &decomp[start..pos] { print!("{:02x} ", b); }
            print!("| name: ");
            for b in &decomp[pos..pos+bytes.len()] { print!("{:02x} ", b); }
            print!("| post: ");
            for b in &decomp[pos+bytes.len()..end] { print!("{:02x} ", b); }
            println!();
        } else {
            println!("\n{name}: NOT FOUND");
        }
    }
    Ok(())
}
