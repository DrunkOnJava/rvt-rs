//! Export independently inflated stream members for local byte observations.
//!
//! Usage: `cargo run --release --example probe_stream_payload -- FILE STREAM OUTPUT_DIR [GZIP_OFFSET]`.
//! Optional offset is in the checksum-normalized stored stream; selecting one
//! member avoids exporting every member and the complete stored stream.
//! This is a diagnostic export, not a claim of element decoding. Run against
//! each of the eleven family corpus releases to compare member boundaries.
use rvt::{RevitFile, compression};
fn main() -> anyhow::Result<()> {
    let args: Vec<_> = std::env::args().collect();
    anyhow::ensure!(
        (4..=5).contains(&args.len()),
        "expected FILE STREAM OUTPUT_DIR [GZIP_OFFSET]"
    );
    let selected_offset = args.get(4).map(|s| s.parse::<usize>()).transpose()?;
    // Explicit local research input: permit a stream up to the file's size,
    // retaining the library's file-size and per-member inflate caps.
    let limits = rvt::reader::OpenLimits {
        max_stream_bytes: std::fs::metadata(&args[1])?.len(),
        ..Default::default()
    };
    let mut rf = RevitFile::open_with_limits(&args[1], limits)?;
    let raw = rf.read_stream(&args[2])?;
    let root = std::path::Path::new(&args[3]);
    std::fs::create_dir_all(root)?;
    if selected_offset.is_none() {
        std::fs::write(root.join("stored.bin"), &raw)?;
    }
    let prepared = compression::prepare_stream_for_inflate(&args[2], &raw);
    let mut selected_exported = false;
    for (index, offset) in compression::find_gzip_offsets(&prepared)
        .into_iter()
        .enumerate()
    {
        if selected_offset.is_some_and(|wanted| wanted != offset) {
            continue;
        }
        match compression::inflate_at(&prepared, offset) {
            Ok(bytes) => {
                selected_exported = true;
                println!("member {index} offset {offset} bytes {}", bytes.len());
                std::fs::write(root.join(format!("member-{index}.bin")), bytes)?;
            }
            Err(error) => eprintln!("member {index} offset {offset}: {error}"),
        }
    }
    anyhow::ensure!(
        selected_offset.is_none() || selected_exported,
        "selected gzip offset was not found or could not be inflated"
    );
    Ok(())
}
