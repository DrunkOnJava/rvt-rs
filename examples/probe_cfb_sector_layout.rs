//! WRT-10.1 — probe OLE CFB sector layout across the 11-release
//! Revit family corpus.
//!
//! Reads the CFB header (first 512 bytes) from each corpus file
//! and reports: minor/major version, byte order, sector shift,
//! number of FAT / mini-FAT / DIFAT sectors, first directory
//! sector, first mini-FAT / DIFAT sector. This is the baseline
//! we'll diff against `cfb::CompoundFile::create` output to find
//! where the default layout diverges from Revit's exact layout
//! (WRT-10.3 downstream).
//!
//! Resolves paths via `RVT_SAMPLES_DIR` (default
//! `../../samples`).

use std::fs::File;
use std::io::Read;

const HEADER_MAGIC: [u8; 8] = [0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1];

#[derive(Debug)]
struct CfbHeader {
    minor_version: u16,
    major_version: u16,
    byte_order: u16,
    sector_shift: u16,
    mini_sector_shift: u16,
    dir_sector_count: u32,
    fat_sector_count: u32,
    first_dir_sector: u32,
    mini_stream_cutoff: u32,
    first_mini_fat: u32,
    mini_fat_sector_count: u32,
    first_difat: u32,
    difat_sector_count: u32,
}

fn parse_header(buf: &[u8]) -> Option<CfbHeader> {
    if buf.len() < 512 || buf[0..8] != HEADER_MAGIC {
        return None;
    }
    let u16_at = |o: usize| u16::from_le_bytes([buf[o], buf[o + 1]]);
    let u32_at = |o: usize| u32::from_le_bytes([buf[o], buf[o + 1], buf[o + 2], buf[o + 3]]);
    Some(CfbHeader {
        minor_version: u16_at(0x18),
        major_version: u16_at(0x1A),
        byte_order: u16_at(0x1C),
        sector_shift: u16_at(0x1E),
        mini_sector_shift: u16_at(0x20),
        dir_sector_count: u32_at(0x28),
        fat_sector_count: u32_at(0x2C),
        first_dir_sector: u32_at(0x30),
        mini_stream_cutoff: u32_at(0x38),
        first_mini_fat: u32_at(0x3C),
        mini_fat_sector_count: u32_at(0x40),
        first_difat: u32_at(0x44),
        difat_sector_count: u32_at(0x48),
    })
}

fn main() {
    let dir = std::env::var("RVT_SAMPLES_DIR").unwrap_or_else(|_| "../../samples".into());
    let mut files: Vec<std::path::PathBuf> = std::fs::read_dir(&dir)
        .map(|it| {
            it.flatten()
                .map(|e| e.path())
                .filter(|p| p.extension().is_some_and(|x| x == "rfa" || x == "rvt"))
                .collect()
        })
        .unwrap_or_default();
    files.sort();
    if files.is_empty() {
        eprintln!("no corpus files in {dir}");
        return;
    }

    println!(
        "{:<42} {:>8} {:>6} {:>5} {:>4} {:>4} {:>6} {:>6} {:>6} {:>6} {:>4}",
        "file", "ver", "sec_sh", "msh", "dir#", "fat#", "1stDir", "1stMF", "mf#", "1stDI", "di#"
    );
    println!("{}", "-".repeat(120));

    for path in &files {
        let Ok(mut f) = File::open(path) else {
            continue;
        };
        let mut buf = [0u8; 512];
        if f.read_exact(&mut buf).is_err() {
            continue;
        }
        let Some(h) = parse_header(&buf) else {
            continue;
        };
        let name = path.file_name().unwrap().to_string_lossy();
        println!(
            "{:<42} {:>3}.{:<4} {:>6} {:>5} {:>4} {:>4} {:>6} {:>6} {:>6} {:>6} {:>4}",
            name,
            h.major_version,
            h.minor_version,
            h.sector_shift,
            h.mini_sector_shift,
            h.dir_sector_count,
            h.fat_sector_count,
            h.first_dir_sector,
            h.first_mini_fat,
            h.mini_fat_sector_count,
            h.first_difat,
            h.difat_sector_count,
        );
        let _ = (h.byte_order, h.mini_stream_cutoff);
    }
}
