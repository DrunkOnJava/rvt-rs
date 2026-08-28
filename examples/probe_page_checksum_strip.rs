//! Probe Discussion #112 / reviter `stripRevitPageChecksums`.
//!
//! Compare inflate outcomes on checksum-paged streams with and without
//! stripping each full 65_249-byte stored page's 353-byte trailer.
//!
//! Usage:
//!   cargo run --release --example probe_page_checksum_strip -- path.rvt Formats/Latest

use std::env;
use std::io::Read;
use std::path::PathBuf;

use rvt::compression::{self, strip_revit_page_checksums};

fn main() {
    let mut args = env::args().skip(1);
    let path = PathBuf::from(args.next().expect("usage: PATH [STREAM]"));
    let stream = args.next().unwrap_or_else(|| "Formats/Latest".to_string());

    let mut file = cfb::open(&path).expect("open cfb");
    let mut raw = Vec::new();
    file.open_stream(stream.as_str())
        .unwrap_or_else(|e| panic!("open stream {stream}: {e}"))
        .read_to_end(&mut raw)
        .expect("read stream");

    let stripped = strip_revit_page_checksums(&raw);
    println!(
        "file={} stream={} raw={} stripped={} removed={}",
        path.display(),
        stream,
        raw.len(),
        stripped.len(),
        raw.len() - stripped.len()
    );

    for (label, bytes) in [("raw", raw.as_slice()), ("stripped", stripped.as_slice())] {
        match compression::inflate_at(bytes, 0) {
            Ok(out) => println!("{label}: inflate_at(0) ok len={}", out.len()),
            Err(e) => println!("{label}: inflate_at(0) err={e}"),
        }
        match compression::inflate_at_auto(bytes) {
            Ok((off, out)) => println!("{label}: inflate_at_auto off={off} len={}", out.len()),
            Err(e) => println!("{label}: inflate_at_auto err={e}"),
        }
        let chunks = compression::inflate_all_chunks(bytes);
        let total: usize = chunks.iter().map(|c| c.len()).sum();
        println!(
            "{label}: inflate_all_chunks count={} total_bytes={}",
            chunks.len(),
            total
        );
    }
}
