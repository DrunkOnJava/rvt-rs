//! RE-208 (#878): where a Revit 2015 file keeps each element's editing
//! episode, the half of Revit's GlobalId that RE-48 reads from
//! `Global/History` and `Global/ElemTable`.
//!
//! FACT under test: RE-48's layout (episode count at `+0x0e`, a `u32`-counted
//! table at `+0x66`, then the count again and 17-byte entries, a GUID and
//! `0x28`; the episode number at `+8` of a 28-byte ElemTable record, RE-108)
//! gives no GlobalId on BIMcollab's Revit 2015 structural example. This probe
//! prints the raw facts; `tools/re/episodes_vs_ifc.py` recovers each
//! element's episode GUID from Revit's own GlobalIds and finds where the file
//! keeps it.
//!
//! Output:
//!   history <bytes> count0e=<u32> table66=<u32> parse=<ok N | none>
//!   head <hex of the first 0x100 bytes>
//!   run <offset> <entries> before=<u32 at offset-4>     each run of >= 2 entries
//!   guid <run offset> <index> <32 hex, stored order>
//!   elemtable <bytes> records=<n | error> len=<record length>
//!   rec <id_primary> <id_secondary> <raw hex>
//!   gid <stream> <offset> <utf16|ascii> <string> <hex of the 32 bytes before>
//!     every 22-character string of IFC's GlobalId alphabet whose first
//!     character is 0 to 3, in each partition and `Global/Latest`: a GlobalId
//!     Revit's exporter took from a stored IfcGUID parameter would be one.
//!
//! Usage:
//!   cargo run --profile ci --example probe_re208_episodes_2015 -- FILE.rvt

use rvt::research::revit_global_ids::{EPISODE_ENTRY_LEN, EPISODE_TERMINATOR, EpisodeHistory};
use rvt::{RevitFile, compression, elem_table, streams};

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn u32_at(buf: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(
        buf.get(at..at.checked_add(4)?)?.try_into().ok()?,
    ))
}

fn inflate(rf: &mut RevitFile, name: &str) -> Result<Vec<u8>, String> {
    let stored = rf.read_stream(name).map_err(|e| e.to_string())?;
    let prepared = compression::prepare_stream_for_inflate(name, &stored);
    let data = prepared.as_ref();
    for offset in [0, 4, 8, 16] {
        if compression::has_gzip_magic(data, offset) {
            if let Ok(out) = compression::inflate_at(data, offset) {
                return Ok(out);
            }
        }
    }
    compression::inflate_at_auto(data)
        .map(|(_, out)| out)
        .map_err(|e| e.to_string())
}

const GID_ALPHABET: &[u8] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz_$";

fn scan_global_ids(stream: &str, data: &[u8]) {
    let is_gid = |b: u8| GID_ALPHABET.contains(&b);
    let before = |at: usize| hex(&data[at.saturating_sub(32)..at]);
    // ASCII: exactly 22 alphabet bytes, not part of a longer run.
    let mut i = 0;
    while i < data.len() {
        if !is_gid(data[i]) {
            i += 1;
            continue;
        }
        let start = i;
        while i < data.len() && is_gid(data[i]) {
            i += 1;
        }
        if i - start == 22 && (b'0'..=b'3').contains(&data[start]) {
            let text = String::from_utf8_lossy(&data[start..i]);
            println!("gid {stream} {start} ascii {text} {}", before(start));
        }
    }
    // UTF-16LE: 22 alphabet code units, each followed by a zero byte.
    for parity in 0..2 {
        let mut i = parity;
        while i + 1 < data.len() {
            if !(is_gid(data[i]) && data[i + 1] == 0) {
                i += 2;
                continue;
            }
            let start = i;
            while i + 1 < data.len() && is_gid(data[i]) && data[i + 1] == 0 {
                i += 2;
            }
            if (i - start) / 2 == 22 && (b'0'..=b'3').contains(&data[start]) {
                let text: String = data[start..i]
                    .iter()
                    .step_by(2)
                    .map(|&b| char::from(b))
                    .collect();
                println!("gid {stream} {start} utf16 {text} {}", before(start));
            }
        }
    }
}

fn main() {
    let path = std::env::args_os()
        .nth(1)
        .expect("usage: probe_re208_episodes_2015 FILE.rvt");
    let mut rf = RevitFile::open(&path).expect("open");

    match inflate(&mut rf, streams::GLOBAL_HISTORY) {
        Ok(history) => {
            let parsed = EpisodeHistory::parse(&history)
                .map_or("none".to_string(), |h| format!("ok {}", h.len()));
            println!(
                "history {} count0e={:?} table66={:?} parse={parsed}",
                history.len(),
                u32_at(&history, 0x0e),
                u32_at(&history, 0x66)
            );
            println!("head {}", hex(&history[..history.len().min(0x100)]));
            // Every run of at least two consecutive entries closed by the terminator.
            let mut start = 0;
            while start + EPISODE_ENTRY_LEN <= history.len() {
                let mut n = 0;
                while start + (n + 1) * EPISODE_ENTRY_LEN <= history.len()
                    && history[start + n * EPISODE_ENTRY_LEN + 16] == EPISODE_TERMINATOR
                {
                    n += 1;
                }
                if n >= 2 {
                    let before = start.checked_sub(4).and_then(|at| u32_at(&history, at));
                    println!("run {start} {n} before={before:?}");
                    for i in 0..n {
                        let at = start + i * EPISODE_ENTRY_LEN;
                        println!("guid {start} {i} {}", hex(&history[at..at + 16]));
                    }
                    start += n * EPISODE_ENTRY_LEN;
                } else {
                    start += 1;
                }
            }
        }
        Err(error) => println!("history error {error}"),
    }

    let table_bytes = inflate(&mut rf, streams::GLOBAL_ELEM_TABLE)
        .map(|t| t.len())
        .unwrap_or(0);
    match elem_table::parse_records(&mut rf) {
        Ok(records) => {
            println!(
                "elemtable {table_bytes} records={} len={:?}",
                records.len(),
                records.first().map(|r| r.raw.len())
            );
            for r in &records {
                println!("rec {} {} {}", r.id_primary, r.id_secondary, hex(&r.raw));
            }
        }
        Err(error) => println!("elemtable {table_bytes} records=error {error}"),
    }

    let mut names: Vec<String> = rf
        .stream_names()
        .into_iter()
        .filter(|n| n.starts_with("Partitions/"))
        .collect();
    names.push(streams::GLOBAL_LATEST.to_string());
    for name in names {
        match rf.inflated_partition(&name) {
            Ok(inflated) => scan_global_ids(&name, inflated.bytes()),
            Err(_) => {
                if let Ok(bytes) = inflate(&mut rf, &name) {
                    scan_global_ids(&name, &bytes);
                }
            }
        }
    }
}
