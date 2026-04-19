//! Full hex dump of the 167-byte Global/PartitionTable from the 2024 file
//! with byte-offset annotations so the format-identifier GUID is readable.
use rvt::{compression, streams::GLOBAL_PARTITION_TABLE};

fn main() -> anyhow::Result<()> {
    let path = std::env::args().nth(1).expect("usage: partition_full <file>");
    let mut rf = rvt::RevitFile::open(&path)?;
    let raw = rf.read_stream(GLOBAL_PARTITION_TABLE)?;
    let d = compression::inflate_at(&raw, 8)?;
    println!("Global/PartitionTable: {} bytes decompressed\n", d.len());

    // Print 16 bytes per line with offset, hex, ASCII
    for chunk_start in (0..d.len()).step_by(16) {
        let chunk_end = (chunk_start + 16).min(d.len());
        print!("0x{:04x}  ", chunk_start);
        for i in chunk_start..chunk_end {
            print!("{:02x} ", d[i]);
        }
        for _ in chunk_end..chunk_start + 16 {
            print!("   ");
        }
        print!(" |");
        for i in chunk_start..chunk_end {
            let b = d[i];
            print!("{}", if (0x20..0x7f).contains(&b) { b as char } else { '.' });
        }
        println!("|");
    }

    // Structured decode
    if d.len() >= 26 {
        println!("\nStructured decode:");
        println!("  bytes[0..2]   = u16 LE {}  (format version counter, varies per release)",
            u16::from_le_bytes([d[0], d[1]]));
        let u32_at = |i: usize| -> u32 {
            u32::from_le_bytes([d[i], d[i+1], d[i+2], d[i+3]])
        };
        println!("  bytes[2..6]   = u32 LE {}  (constant 1)", u32_at(2));
        println!("  bytes[6..10]  = u32 LE {}  (constant 1)", u32_at(6));
        let g = &d[10..26];
        let guid = format!(
            "{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
            g[3], g[2], g[1], g[0],
            g[5], g[4], g[7], g[6],
            g[8], g[9], g[10], g[11],
            g[12], g[13], g[14], g[15],
        );
        println!("  bytes[10..26] = 16-byte UUIDv1 (Windows GUID format):");
        println!("    GUID: {{{guid}}}");
        let node = format!(
            "{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
            g[10], g[11], g[12], g[13], g[14], g[15]
        );
        println!("    MAC suffix (node): {node}");
        let version_bits = (g[7] & 0xf0) >> 4;
        println!("    UUID version: {version_bits} ({})",
            match version_bits { 1 => "time-based (UUIDv1) — encodes a timestamp and MAC", 4 => "random", _ => "other" });
        if d.len() >= 30 {
            println!("  bytes[26..30] = u32 LE {}", u32_at(26));
        }
        if d.len() >= 34 {
            println!("  bytes[30..34] = u32 LE {}  (likely a length)", u32_at(30));
        }
    }
    Ok(())
}
