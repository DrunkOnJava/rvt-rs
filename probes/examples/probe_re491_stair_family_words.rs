//! RE-491: does a Revit file hold the family words Revit's export gives a
//! stair's parts?
//!
//! FACT under test: Revit's export names a stair, a run, a landing and a
//! support after a system family (`Assembled Stair`, `Monolithic Run`,
//! `Monolithic Landing`, `Stringer`), and the file does not hold those words.
//! On Snowdon Towers Architectural (Revit 2024, local only), whose export names
//! the stair of 20 landings `Assembled Stair`, that word is in no stream, as
//! UTF-16 or as ASCII, while the names of the file's stair and landing types
//! are. So the word is the exporting Revit's, in its own language, and a reader
//! of the file can only write one language's.
//!
//! For each word (the eight family words below when none is given), prints
//! the streams it is found in, in the stream as stored and in each gzip member
//! of it, with the count and the string each hit sits in (80 characters at
//! most, the first 12 distinct strings of a word), then the word's total. A
//! word found nowhere has the total 0. `Monolithic Landing` is also counted
//! inside `Non-Monolithic Landing`, and `Monolithic Run` inside
//! `Non-Monolithic Run`. Words and counts only: no coordinate of the model is
//! read.
//!
//! Usage:
//!   cargo run --profile ci -p rvt-probes --example probe_re491_stair_family_words -- MODEL.rvt [WORD...]

use std::collections::BTreeMap;
use std::path::PathBuf;

use anyhow::{Context, Result, ensure};
use memchr::memmem;
use rvt::RevitFile;
use rvt::compression::inflate_all_chunks_for_stream;

/// The longest string printed around a hit, in characters.
const CLIP: usize = 80;
/// How many distinct strings of a word are printed.
const SHOWN: usize = 12;
/// The family words Revit's export gives a stair and its parts (RE-65, RE-491).
const FAMILY_WORDS: [&str; 8] = [
    "Assembled Stair",
    "Cast-In-Place Stair",
    "Monolithic Run",
    "Non-Monolithic Run",
    "Monolithic Landing",
    "Non-Monolithic Landing",
    "Stringer",
    "Carriage",
];

fn utf16(word: &str) -> Vec<u8> {
    word.encode_utf16().flat_map(u16::to_le_bytes).collect()
}

/// The UTF-16 string a hit at `at` (`len` bytes) sits in: the code units on
/// either side of it up to a control character or a surrogate.
fn utf16_around(bytes: &[u8], at: usize, len: usize) -> String {
    let unit = |i: usize| u16::from_le_bytes([bytes[i], bytes[i + 1]]);
    let text = |u: u16| u >= 0x20 && !(0xd800..=0xdfff).contains(&u) && u != 0xffff;
    let mut start = at;
    while start >= 2 && at - start < 2 * CLIP && text(unit(start - 2)) {
        start -= 2;
    }
    let mut end = at + len;
    while end + 2 <= bytes.len() && end - at < 2 * CLIP && text(unit(end)) {
        end += 2;
    }
    let units: Vec<u16> = (start..end).step_by(2).map(unit).collect();
    String::from_utf16_lossy(&units)
        .chars()
        .take(CLIP)
        .collect()
}

/// The ASCII string a hit sits in: the printable bytes on either side of it.
fn ascii_around(bytes: &[u8], at: usize, len: usize) -> String {
    let text = |b: u8| (0x20..0x7f).contains(&b);
    let mut start = at;
    while start >= 1 && at - start < CLIP && text(bytes[start - 1]) {
        start -= 1;
    }
    let mut end = at + len;
    while end < bytes.len() && end - at < CLIP && text(bytes[end]) {
        end += 1;
    }
    String::from_utf8_lossy(&bytes[start..end])
        .chars()
        .take(CLIP)
        .collect()
}

fn main() -> Result<()> {
    let mut args = std::env::args().skip(1);
    let path = PathBuf::from(args.next().context("usage: MODEL.rvt [WORD...]")?);
    let mut words: Vec<String> = args.collect();
    if words.is_empty() {
        words = FAMILY_WORDS.iter().map(|word| word.to_string()).collect();
    }
    ensure!(
        words.iter().all(|word| !word.is_empty()),
        "an empty word matches everywhere"
    );
    let mut rf = RevitFile::open(&path).with_context(|| format!("open {}", path.display()))?;

    // (word, stream, encoding, string) -> hits.
    type Hit = (String, String, &'static str, String);
    let mut found: BTreeMap<Hit, usize> = BTreeMap::new();
    let mut totals: BTreeMap<String, usize> = words.iter().map(|w| (w.clone(), 0)).collect();
    let (mut streams, mut members) = (0usize, 0usize);
    for name in rf.stream_names() {
        let Ok(stored) = rf.read_stream(&name) else {
            println!("unread {name}");
            continue;
        };
        streams += 1;
        let mut buffers = inflate_all_chunks_for_stream(&name, &stored);
        members += buffers.len();
        buffers.push(stored);
        for bytes in &buffers {
            for word in &words {
                let wide = utf16(word);
                for at in memmem::find_iter(bytes, &wide) {
                    let around = utf16_around(bytes, at, wide.len());
                    *found
                        .entry((word.clone(), name.clone(), "utf16", around))
                        .or_default() += 1;
                    *totals.entry(word.clone()).or_default() += 1;
                }
                for at in memmem::find_iter(bytes, word.as_bytes()) {
                    let around = ascii_around(bytes, at, word.len());
                    *found
                        .entry((word.clone(), name.clone(), "ascii", around))
                        .or_default() += 1;
                    *totals.entry(word.clone()).or_default() += 1;
                }
            }
        }
    }
    println!("streams {streams} gzip_members {members}");
    let mut shown: BTreeMap<&str, usize> = BTreeMap::new();
    for ((word, stream, encoding, around), hits) in &found {
        let count = shown.entry(word.as_str()).or_default();
        *count += 1;
        if *count <= SHOWN {
            println!("hit {hits} word {word:?} stream {stream} {encoding} in {around:?}");
        }
    }
    for (word, count) in &shown {
        if *count > SHOWN {
            println!(
                "more {} word {word:?} (distinct strings not printed)",
                count - SHOWN
            );
        }
    }
    for (word, hits) in &totals {
        println!("total {hits} word {word:?}");
    }
    Ok(())
}
