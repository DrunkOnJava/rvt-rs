//! Q6 probe: parse the Global/Latest class-tag directory and test
//! whether the payload u16/u32 values make sense as:
//!   (a) file-offset pointers into Global/Latest where instance data lives
//!   (b) instance counts
//!   (c) tag-relative offsets into a separate Partitions chunk
//!
//! Strategy: enumerate directory entries, then look at what the payload
//! value lands on if interpreted as a file offset into the decompressed
//! Global/Latest stream.
#![allow(
    clippy::needless_range_loop,
    clippy::type_complexity,
    clippy::collapsible_if,
    clippy::collapsible_match
)]

use rvt::{RevitFile, compression, streams::GLOBAL_LATEST};

fn main() -> anyhow::Result<()> {
    let path = std::env::args()
        .nth(1)
        .expect("usage: directory_probe <file>");
    let mut rf = RevitFile::open(&path)?;
    let raw = rf.read_stream(GLOBAL_LATEST)?;
    let d = compression::inflate_at(&raw, 8)?;
    println!("Global/Latest: {} bytes decompressed", d.len());

    // Document upgrade history lives at the start of the stream.
    // The class-tag directory begins AFTER it. Scan for the first
    // u32 LE value that looks like a small tag (0x0000-0x4000 range)
    // preceded by something that looks like a u16 payload (< 0x4000).
    // Empirically (from record_framing.rs output) the directory begins
    // around offset 0x430 in the 2024 file; we'll scan from 0x400 to
    // find it robustly.
    let scan_start = 0x400;
    let scan_end = (scan_start + 0x2000).min(d.len());

    #[derive(Debug)]
    struct DirEntry {
        offset: usize,
        tag: u32,
        payload_u16: u16,
        entry_span: usize,
    }
    let mut entries: Vec<DirEntry> = Vec::new();

    // Walk forward. An entry is `[u32 tag][variable bytes until next u32 tag]`.
    let mut i = scan_start;
    while i + 8 < scan_end {
        let tag = u32::from_le_bytes([d[i], d[i + 1], d[i + 2], d[i + 3]]);
        if tag == 0 || tag > 0x4000 {
            i += 1;
            continue;
        }
        // Find the next candidate tag (u32 in same range)
        let mut next_at = None;
        let mut j = i + 4;
        while j + 4 <= scan_end {
            let t2 = u32::from_le_bytes([d[j], d[j + 1], d[j + 2], d[j + 3]]);
            if t2 > tag && t2 <= tag + 20 && t2 < 0x4000 {
                next_at = Some(j);
                break;
            }
            j += 2;
            if j - i > 40 {
                break;
            }
        }
        let span = next_at.map(|n| n - i).unwrap_or(6);
        // Payload: the first 2 bytes after the tag
        let payload = if i + 6 <= scan_end {
            u16::from_le_bytes([d[i + 4], d[i + 5]])
        } else {
            0
        };
        entries.push(DirEntry {
            offset: i,
            tag,
            payload_u16: payload,
            entry_span: span,
        });
        i += span.max(6);
    }

    println!("\nDirectory entries found: {}", entries.len());
    println!("First 30 (sequential tags ascending):");
    for e in entries.iter().take(30) {
        print!(
            "  @0x{:04x}  tag=0x{:04x} ({:4})  payload u16={:5} (0x{:04x})  span={}",
            e.offset, e.tag, e.tag, e.payload_u16, e.payload_u16, e.entry_span
        );
        // Interpret payload as offset: is d[payload..payload+16] structured?
        let off = e.payload_u16 as usize;
        if off > 0 && off + 8 < d.len() {
            let a = u32::from_le_bytes([d[off], d[off + 1], d[off + 2], d[off + 3]]);
            let b = u32::from_le_bytes([d[off + 4], d[off + 5], d[off + 6], d[off + 7]]);
            print!("  | payload-as-offset[{off}]: u32={a} u32={b}");
        }
        println!();
    }

    // Check monotonicity: if payload is an offset, it should be monotonic
    // (or at least all-in-range).
    let mut in_range_as_offset = 0;
    let mut beyond_stream = 0;
    for e in &entries {
        let off = e.payload_u16 as usize;
        if off < d.len() {
            in_range_as_offset += 1;
        } else {
            beyond_stream += 1;
        }
    }
    println!(
        "\nPayload-as-offset sanity: {} in range, {} beyond stream end ({} bytes)",
        in_range_as_offset,
        beyond_stream,
        d.len()
    );

    // Alt interpretation: payload is a COUNT of instances for this class
    // elsewhere. Total instances across all directory entries:
    let total: u32 = entries.iter().map(|e| e.payload_u16 as u32).sum();
    println!(
        "Payload-as-count sanity: sum across all {} entries = {}",
        entries.len(),
        total
    );

    Ok(())
}
