//! Phase B · Corpus-wide delta inference engine.
//!
//! Loads multiple Revit files (ideally same logical content across many
//! versions), decompresses every stream, and classifies each byte position
//! as one of:
//!
//! - **Invariant** — same byte across every version (magic / structural)
//! - **LowVariance** — 2..=5 distinct values across versions (type tag, enum)
//! - **SizeCorrelated** — monotonically non-decreasing across versions
//!   (likely a length field or count)
//! - **MonotonicInt** — strictly increasing integer sequence (IDs, timestamps)
//! - **Variable** — genuinely varies (payload data)
//!
//! Output: per-stream region maps that feed Phase D's object-graph inference.

use crate::{Result, compression, reader::RevitFile, streams::year_for_partition};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, path::Path};

/// One sample in the corpus.
pub struct Sample {
    pub label: String,
    pub year: Option<u32>,
    pub file: RevitFile,
}

impl Sample {
    /// Load a file and auto-infer its year from the BasicFileInfo stream.
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let mut file = RevitFile::open(path)?;
        let year = file.basic_file_info().map(|b| b.version).ok();
        let label = path
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_default();
        Ok(Self { label, year, file })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ByteClass {
    Invariant,
    LowVariance(u8), // 2..=5 distinct values
    SizeCorrelated,  // monotonic non-decreasing
    MonotonicInt,    // strictly increasing integer-like
    Variable,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamDeltaReport {
    /// Stream name, e.g. `Global/PartitionTable`.
    pub name: String,
    /// Number of samples contributing to this stream.
    pub samples: usize,
    /// Min / max / uniform raw-stream size across samples.
    pub raw_size_min: usize,
    pub raw_size_max: usize,
    /// Decompressed size stats, when we successfully decompressed for every sample.
    pub decomp_size_min: Option<usize>,
    pub decomp_size_max: Option<usize>,
    /// Byte-class histogram across the prefix we could align
    /// (min decomp length when comparing decompressed, else min raw length).
    pub counts: BTreeMap<String, usize>,
    /// Invariant byte runs longer than `invariant_run_threshold` (default 8).
    /// Each entry is `(offset, run_length, hex_preview)`.
    pub invariant_runs: Vec<InvariantRun>,
    /// Length of the prefix we aligned on.
    pub aligned_len: usize,
    /// Whether we used decompressed bytes (`true`) or raw (`false`) for alignment.
    pub used_decompressed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvariantRun {
    pub offset: usize,
    pub length: usize,
    pub hex_preview: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorpusReport {
    pub samples: Vec<String>,
    pub streams: Vec<StreamDeltaReport>,
    pub partition_mapping: BTreeMap<u32, Option<u32>>, // partition number -> year if known
}

pub fn analyze_corpus(samples: &mut [Sample]) -> Result<CorpusReport> {
    // Collect stream name universe
    let mut stream_universe: BTreeMap<String, ()> = BTreeMap::new();
    for s in samples.iter() {
        for name in s.file.stream_names() {
            stream_universe.insert(name, ());
        }
    }

    // Partition mapping
    let mut partition_mapping = BTreeMap::new();
    for s in samples.iter() {
        if let Some(pname) = s.file.partition_stream_name() {
            if let Some(num) = pname
                .strip_prefix("Partitions/")
                .and_then(|n| n.parse::<u32>().ok())
            {
                partition_mapping.insert(num, year_for_partition(num));
            }
        }
    }

    let mut stream_reports = Vec::new();

    for name in stream_universe.keys() {
        // Collect raw bytes per sample that HAS the stream.
        let mut raws: Vec<Vec<u8>> = Vec::new();
        for s in samples.iter_mut() {
            if let Ok(bytes) = s.file.read_stream(name) {
                raws.push(bytes);
            }
        }
        if raws.len() < 2 {
            continue;
        }

        let raw_size_min = raws.iter().map(|v| v.len()).min().unwrap_or(0);
        let raw_size_max = raws.iter().map(|v| v.len()).max().unwrap_or(0);

        // Attempt decompression of each sample's bytes. Streams are one of:
        //   - raw GZIP at offset 0 (Formats/Latest)
        //   - custom header then GZIP body (most Global/* streams, offset 8)
        //   - plain data (BasicFileInfo, TransmissionData)
        //   - PartAtom is text XML; skip decompression
        //   - Partitions/NN: multi-chunk, use find_gzip_offsets + first chunk
        let decompressed: Vec<Option<Vec<u8>>> =
            raws.iter().map(|data| try_decompress(data)).collect();

        let (bytes_for_alignment, used_decompressed, decomp_size_min, decomp_size_max) =
            if decompressed.iter().all(|d| d.is_some()) {
                let decomped: Vec<&Vec<u8>> =
                    decompressed.iter().map(|d| d.as_ref().unwrap()).collect();
                let min = decomped.iter().map(|v| v.len()).min().unwrap_or(0);
                let max = decomped.iter().map(|v| v.len()).max().unwrap_or(0);
                (
                    decomped.iter().map(|v| v.as_slice()).collect(),
                    true,
                    Some(min),
                    Some(max),
                )
            } else {
                let all: Vec<&[u8]> = raws.iter().map(|v| v.as_slice()).collect();
                (all, false, None, None)
            };

        // Align on the common prefix length.
        let aligned_len = bytes_for_alignment
            .iter()
            .map(|s| s.len())
            .min()
            .unwrap_or(0);

        // Per-byte classification.
        let mut counts: BTreeMap<String, usize> = BTreeMap::new();
        counts.insert("Invariant".into(), 0);
        counts.insert("LowVariance".into(), 0);
        counts.insert("SizeCorrelated".into(), 0);
        counts.insert("MonotonicInt".into(), 0);
        counts.insert("Variable".into(), 0);

        let mut classifications: Vec<ByteClass> = Vec::with_capacity(aligned_len);
        for i in 0..aligned_len {
            let col: Vec<u8> = bytes_for_alignment.iter().map(|s| s[i]).collect();
            classifications.push(classify_column(&col));
        }

        for c in &classifications {
            let bucket = match c {
                ByteClass::Invariant => "Invariant",
                ByteClass::LowVariance(_) => "LowVariance",
                ByteClass::SizeCorrelated => "SizeCorrelated",
                ByteClass::MonotonicInt => "MonotonicInt",
                ByteClass::Variable => "Variable",
            };
            *counts.entry(bucket.to_string()).or_insert(0) += 1;
        }

        // Invariant runs of length >= 8.
        const MIN_RUN: usize = 8;
        let mut invariant_runs = Vec::new();
        let mut run_start: Option<usize> = None;
        for (i, c) in classifications.iter().enumerate() {
            match (c, run_start) {
                (ByteClass::Invariant, None) => run_start = Some(i),
                (ByteClass::Invariant, Some(_)) => {}
                (_, Some(start)) => {
                    let len = i - start;
                    if len >= MIN_RUN {
                        let sample = bytes_for_alignment[0];
                        let hex = sample[start..start + len.min(32)]
                            .iter()
                            .map(|b| format!("{b:02x}"))
                            .collect::<Vec<_>>()
                            .join(" ");
                        invariant_runs.push(InvariantRun {
                            offset: start,
                            length: len,
                            hex_preview: hex,
                        });
                    }
                    run_start = None;
                }
                (_, None) => {}
            }
        }
        if let Some(start) = run_start {
            let len = classifications.len() - start;
            if len >= MIN_RUN {
                let sample = bytes_for_alignment[0];
                let hex = sample[start..start + len.min(32)]
                    .iter()
                    .map(|b| format!("{b:02x}"))
                    .collect::<Vec<_>>()
                    .join(" ");
                invariant_runs.push(InvariantRun {
                    offset: start,
                    length: len,
                    hex_preview: hex,
                });
            }
        }

        stream_reports.push(StreamDeltaReport {
            name: name.clone(),
            samples: raws.len(),
            raw_size_min,
            raw_size_max,
            decomp_size_min,
            decomp_size_max,
            counts,
            invariant_runs,
            aligned_len,
            used_decompressed,
        });
    }

    Ok(CorpusReport {
        samples: samples.iter().map(|s| s.label.clone()).collect(),
        streams: stream_reports,
        partition_mapping,
    })
}

fn classify_column(col: &[u8]) -> ByteClass {
    if col.iter().all(|&b| b == col[0]) {
        return ByteClass::Invariant;
    }
    // Monotonic patterns win over low-variance: a length-prefix byte that
    // happens to only take 2-3 distinct values is still structurally a
    // length prefix.
    let strictly_increasing = col.windows(2).all(|w| w[0] < w[1]);
    let non_decreasing = col.windows(2).all(|w| w[0] <= w[1]);
    if strictly_increasing {
        return ByteClass::MonotonicInt;
    }
    let distinct: std::collections::BTreeSet<u8> = col.iter().copied().collect();
    if non_decreasing && distinct.len() > 2 {
        return ByteClass::SizeCorrelated;
    }
    if distinct.len() <= 5 {
        return ByteClass::LowVariance(distinct.len() as u8);
    }
    if non_decreasing {
        return ByteClass::SizeCorrelated;
    }
    ByteClass::Variable
}

/// Attempt to decompress a stream's bytes. Tries offset 0, 8, and scanning
/// for gzip magic within the first 128 bytes. Returns `None` on total failure.
fn try_decompress(data: &[u8]) -> Option<Vec<u8>> {
    for off in [0usize, 4, 8, 16] {
        if compression::has_gzip_magic(data, off) {
            if let Ok(out) = compression::inflate_at(data, off) {
                return Some(out);
            }
        }
    }
    // Fallback: scan for gzip magic in first 128 bytes.
    for off in 0..data.len().min(128) {
        if compression::has_gzip_magic(data, off) {
            if let Ok(out) = compression::inflate_at(data, off) {
                return Some(out);
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify_invariant() {
        assert_eq!(classify_column(&[5, 5, 5, 5]), ByteClass::Invariant);
    }

    #[test]
    fn classify_low_variance() {
        assert!(matches!(
            classify_column(&[0, 1, 0, 1, 0]),
            ByteClass::LowVariance(2)
        ));
    }

    #[test]
    fn classify_monotonic() {
        assert_eq!(classify_column(&[1, 2, 3, 4, 5]), ByteClass::MonotonicInt);
    }

    #[test]
    fn classify_size_correlated() {
        assert_eq!(classify_column(&[1, 1, 2, 2, 3]), ByteClass::SizeCorrelated);
    }

    #[test]
    fn classify_variable() {
        // More than 5 distinct bytes, not monotonic
        assert_eq!(classify_column(&[9, 1, 7, 3, 200, 85]), ByteClass::Variable);
    }
}
