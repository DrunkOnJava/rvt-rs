use anyhow::Result;
use std::io::Write;
fn main() -> Result<()> {
    let args: Vec<_> = std::env::args().collect();
    let mut file = rvt::RevitFile::open(&args[1])?;
    let mut out = std::io::BufWriter::new(std::io::stdout().lock());
    rvt::native_embedded::scan_current(&mut file, 2_000_000, |record| {
        serde_json::to_writer(&mut out, &record)?;
        writeln!(&mut out)?;
        Ok(())
    })?;
    Ok(())
}
