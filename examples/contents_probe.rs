//! Probe the `Contents` stream. Memory note says it starts with the same
//! 4-byte magic as RevitPreview4.0 (`62 19 22 05`), followed by some
//! unknown structure before the real payload. Let's see what's inside.
#![allow(
    clippy::needless_range_loop,
    clippy::type_complexity,
    clippy::collapsible_if,
    clippy::collapsible_match
)]

use rvt::{compression, streams::CONTENTS};

fn main() -> anyhow::Result<()> {
    let path = std::env::args()
        .nth(1)
        .expect("usage: contents_probe <file>");
    let mut rf = rvt::RevitFile::open(&path)?;
    let raw = rf.read_stream(CONTENTS)?;
    println!("Contents stream: {} raw bytes", raw.len());

    // First 32 bytes
    println!("\nFirst 64 bytes:");
    for chunk_start in (0..raw.len().min(64)).step_by(16) {
        let chunk_end = (chunk_start + 16).min(raw.len().min(64));
        print!("0x{chunk_start:04x}  ");
        for i in chunk_start..chunk_end {
            print!("{:02x} ", raw[i]);
        }
        print!(" |");
        for i in chunk_start..chunk_end {
            let b = raw[i];
            print!(
                "{}",
                if (0x20..0x7f).contains(&b) {
                    b as char
                } else {
                    '.'
                }
            );
        }
        println!("|");
    }

    // Look for gzip magic within the stream
    println!("\nGzip magic (1F 8B 08) positions in Contents:");
    let mut positions = Vec::new();
    for i in 0..raw.len().saturating_sub(3) {
        if raw[i] == 0x1f && raw[i + 1] == 0x8b && raw[i + 2] == 0x08 {
            positions.push(i);
        }
    }
    for p in positions.iter().take(10) {
        println!("  offset 0x{p:04x} ({p})");
    }
    println!("  total: {} gzip chunks", positions.len());

    // Try to decompress from each gzip offset
    for (i, offset) in positions.iter().take(3).enumerate() {
        match compression::inflate_at(&raw, *offset) {
            Ok(d) => {
                println!(
                    "\nGzip #{i} @ 0x{offset:04x}: {} bytes decompressed",
                    d.len()
                );
                // First 200 bytes as ASCII-ish
                let preview_len = d.len().min(200);
                print!("  preview: ");
                for &b in &d[..preview_len] {
                    if (0x20..0x7f).contains(&b) {
                        print!("{}", b as char);
                    } else if b == 0 {
                        print!(".");
                    } else {
                        print!("?");
                    }
                }
                println!();

                // Check for UTF-16LE-ish content
                let zeros_in_odd = d
                    .iter()
                    .skip(1)
                    .step_by(2)
                    .take(100)
                    .filter(|&&b| b == 0)
                    .count();
                if zeros_in_odd > 40 {
                    println!(
                        "  (looks like UTF-16LE — {}% zeros in odd positions)",
                        zeros_in_odd
                    );
                }
            }
            Err(e) => println!("\nGzip #{i} @ 0x{offset:04x}: decompress failed: {e}"),
        }
    }

    Ok(())
}
