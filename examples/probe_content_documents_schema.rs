//! RE-20 — validate the 40-byte ContentDocuments record schema on
//! 2024 Core Interior. Parse the entire 1.4 MB buffer as records,
//! check invariants, report violations.
//!
//! Hypothesis from RE-17:
//! ```
//! #[repr(C, packed)]
//! struct ContentDocRecord {
//!     id:         u64,   // element id (monotonic)
//!     count_a:    u32,   // unknown (19 observed)
//!     count_b:    u32,   // unknown, == count_a
//!     marker:     u32,   // 0xFFFFFFFF
//!     id_again:   u64,   // == id
//!     prev_id:    u64,   // previous id or 0xFFFFFFFFFFFFFFFF
//!     trailing:   u32,   // 0
//! }
//! ```
//!
//! Start scanning at first 0xFFFFFFFF marker after the header.

use rvt::{RevitFile, compression};

fn read_u32(b: &[u8], o: usize) -> u32 {
    u32::from_le_bytes([b[o], b[o + 1], b[o + 2], b[o + 3]])
}
fn read_u64(b: &[u8], o: usize) -> u64 {
    u64::from_le_bytes([
        b[o],
        b[o + 1],
        b[o + 2],
        b[o + 3],
        b[o + 4],
        b[o + 5],
        b[o + 6],
        b[o + 7],
    ])
}

fn main() {
    let project_dir = std::env::var("RVT_PROJECT_CORPUS_DIR")
        .unwrap_or_else(|_| "/private/tmp/rvt-corpus-probe/magnetar/Revit".into());
    let path = format!("{project_dir}/2024_Core_Interior.rvt");
    let mut rf = RevitFile::open(&path).unwrap();
    let raw = rf.read_stream("Global/ContentDocuments").unwrap();
    let (prefix, d) = compression::inflate_at_auto(&raw).unwrap();
    println!(
        "2024 ContentDocuments: raw={} B, decomp={} B, prefix={}",
        raw.len(),
        d.len(),
        prefix
    );

    // The first record observed at offset 0x85 based on RE-17 hex dump.
    // Let me find the first 0xFFFFFFFF marker followed by a 19/19 count
    // signature — that's the first record anchor.
    //
    // Actually from the RE-17 dump the record starts at 0x85, which is
    // after a variable-length header. Let's parse the header region
    // too, and find the first record anchor by looking for the
    // count_a=count_b=19 + marker=FFFFFFFF pattern.

    let mut anchor = None;
    let mut i = 0x10;
    while i + 40 <= d.len() {
        if read_u32(&d, i + 8) == 19
            && read_u32(&d, i + 12) == 19
            && read_u32(&d, i + 16) == 0xFFFFFFFF
        {
            anchor = Some(i);
            break;
        }
        i += 1;
    }
    let anchor = match anchor {
        Some(a) => {
            println!("First record anchor found at offset 0x{a:x}");
            a
        }
        None => {
            println!("No record anchor found — schema hypothesis wrong");
            return;
        }
    };

    // Parse records from anchor.
    let mut off = anchor;
    let mut prev_id_seen: u64 = 0xFFFFFFFFFFFFFFFF;
    let mut records_ok = 0usize;
    let mut violations = 0usize;
    let mut first_ids = Vec::new();
    let mut last_ids = Vec::new();
    let mut count_a_values = std::collections::BTreeMap::<u32, usize>::new();
    let mut count_b_values = std::collections::BTreeMap::<u32, usize>::new();

    while off + 40 <= d.len() {
        let id = read_u64(&d, off);
        let ca = read_u32(&d, off + 8);
        let cb = read_u32(&d, off + 12);
        let marker = read_u32(&d, off + 16);
        let id_again = read_u64(&d, off + 20);
        let prev_id = read_u64(&d, off + 28);
        let trailing = read_u32(&d, off + 36);

        let id_ok = id == id_again;
        let marker_ok = marker == 0xFFFFFFFF;
        let trailing_ok = trailing == 0;
        let _prev_ok = prev_id == prev_id_seen || prev_id == 0xFFFFFFFFFFFFFFFF;

        *count_a_values.entry(ca).or_insert(0) += 1;
        *count_b_values.entry(cb).or_insert(0) += 1;

        if id_ok && marker_ok && trailing_ok {
            records_ok += 1;
            if first_ids.len() < 5 {
                first_ids.push(id);
            }
            last_ids.push(id);
            if last_ids.len() > 5 {
                last_ids.remove(0);
            }
            prev_id_seen = id;
            off += 40;
        } else {
            violations += 1;
            if violations <= 5 {
                println!(
                    "  VIOLATION @ 0x{off:x}: id={id:#x} id_again={id_again:#x} marker={marker:#x} trailing={trailing:#x}"
                );
            }
            // Try skipping 8 bytes and resyncing
            off += 8;
        }
    }

    println!("\nParse summary:");
    println!("  Records parsed OK: {records_ok}");
    println!("  Violations (resync skips): {violations}");
    println!("  First 5 ids: {first_ids:?}");
    println!("  Last 5 ids: {last_ids:?}");
    println!(
        "  count_a distribution: {:?}",
        count_a_values.iter().take(10).collect::<Vec<_>>()
    );
    println!(
        "  count_b distribution: {:?}",
        count_b_values.iter().take(10).collect::<Vec<_>>()
    );
    println!(
        "  count_a == count_b on all records? {}",
        count_a_values == count_b_values
    );
}
