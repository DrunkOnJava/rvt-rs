//! Stream-evidence harness for Discussion #112 / issue #151.
//!
//! Reports machine-readable JSON for stored CFB streams: page layout,
//! control (raw) vs experimental (page-stripped) inflate outcomes,
//! gzip-trailer probes, parser summaries, and file provenance.
//!
//! Production inflate paths now call `rvt::compression::prepare_stream_for_inflate`
//! / `inflate_stream_*` (issue #151). This harness still keeps an independent
//! experimental strip so control vs experimental A/B remains meaningful even
//! after production wiring lands.
//!
//! Credit: finding reported by [@STE1200](https://github.com/STE1200)
//! (Steffen) in Discussion #112.

#![forbid(unsafe_code)]

use flate2::read::DeflateDecoder;
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::io::Read;
use std::path::{Path, PathBuf};

/// Stored bytes in a full checksum page (Discussion #112 / reviter).
pub const REVIT_STORED_PAGE_BYTES: usize = 65_249;
/// Payload bytes kept from each full page after stripping the trailer.
pub const REVIT_PAGE_PAYLOAD_BYTES: usize = 64_896;
/// Checksum/ECC trailer length cut from each full stored page.
pub const REVIT_PAGE_CHECKSUM_BYTES: usize = REVIT_STORED_PAGE_BYTES - REVIT_PAGE_PAYLOAD_BYTES;

pub const GZIP_MAGIC: [u8; 3] = [0x1F, 0x8B, 0x08];

/// Evidence schema version for consumers / probes.
pub const EVIDENCE_SCHEMA_VERSION: u32 = 1;

/// Hypothesis-side page strip (experimental arm). Independent copy of the
/// Discussion #112 layout so probes do not need production wiring.
pub fn experimental_strip_page_checksums(data: &[u8]) -> Vec<u8> {
    let full_pages = data.len() / REVIT_STORED_PAGE_BYTES;
    if full_pages == 0 {
        return data.to_vec();
    }
    let remainder = data.len() - full_pages * REVIT_STORED_PAGE_BYTES;
    let mut out = Vec::with_capacity(full_pages * REVIT_PAGE_PAYLOAD_BYTES + remainder);
    for page in 0..full_pages {
        let start = page * REVIT_STORED_PAGE_BYTES;
        out.extend_from_slice(&data[start..start + REVIT_PAGE_PAYLOAD_BYTES]);
    }
    out.extend_from_slice(&data[full_pages * REVIT_STORED_PAGE_BYTES..]);
    out
}

/// Paths suspected to use checksum-paged storage (issue #151).
pub fn is_suspected_checksum_paged_stream(path: &str) -> bool {
    let clean = path
        .trim_start_matches('/')
        .trim_start_matches("Root Entry/");
    let clean = clean.trim_start_matches('/');
    if clean.eq_ignore_ascii_case("Formats/Latest") {
        return true;
    }
    if let Some(rest) = clean
        .strip_prefix("Partitions/")
        .or_else(|| clean.strip_prefix("partitions/"))
    {
        return !rest.is_empty() && !rest.contains('/');
    }
    matches!(
        clean,
        "Global/ContentDocuments"
            | "Global/DocumentIncrementTable"
            | "Global/ElemTable"
            | "Global/History"
            | "Global/Latest"
            | "Global/PartitionTable"
    )
}

/// Describe stored-page boundaries for a raw CFB stream body.
pub fn analyze_page_layout(stored: &[u8]) -> PageLayoutEvidence {
    let full_page_count = stored.len() / REVIT_STORED_PAGE_BYTES;
    let remainder_bytes = stored.len() % REVIT_STORED_PAGE_BYTES;
    let mut boundaries = Vec::with_capacity(full_page_count + 1);
    let mut pages = Vec::with_capacity(full_page_count + usize::from(remainder_bytes > 0));
    for page in 0..full_page_count {
        let start = page * REVIT_STORED_PAGE_BYTES;
        boundaries.push(start);
        pages.push(PageEvidence {
            index: page,
            start_offset: start,
            stored_length: REVIT_STORED_PAGE_BYTES,
            is_full_page: true,
            payload_length: REVIT_PAGE_PAYLOAD_BYTES,
            tail_length: REVIT_PAGE_CHECKSUM_BYTES,
        });
    }
    if remainder_bytes > 0 || full_page_count == 0 {
        let start = full_page_count * REVIT_STORED_PAGE_BYTES;
        if full_page_count == 0 {
            boundaries.push(0);
        } else {
            boundaries.push(start);
        }
        pages.push(PageEvidence {
            index: full_page_count,
            start_offset: start,
            stored_length: if full_page_count == 0 {
                stored.len()
            } else {
                remainder_bytes
            },
            is_full_page: false,
            payload_length: if full_page_count == 0 {
                stored.len()
            } else {
                remainder_bytes
            },
            tail_length: 0,
        });
    }
    let stripped_len = experimental_strip_page_checksums(stored).len();
    PageLayoutEvidence {
        stored_length: stored.len(),
        full_page_count,
        remainder_bytes,
        suspected_page_count: pages.len(),
        suspected_boundaries: boundaries,
        pages,
        experimental_stripped_length: stripped_len,
        bytes_removed_by_strip: stored.len().saturating_sub(stripped_len),
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct PageEvidence {
    pub index: usize,
    pub start_offset: usize,
    pub stored_length: usize,
    pub is_full_page: bool,
    pub payload_length: usize,
    pub tail_length: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct PageLayoutEvidence {
    pub stored_length: usize,
    pub full_page_count: usize,
    pub remainder_bytes: usize,
    pub suspected_page_count: usize,
    pub suspected_boundaries: Vec<usize>,
    pub pages: Vec<PageEvidence>,
    pub experimental_stripped_length: usize,
    pub bytes_removed_by_strip: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct GzipTrailerEvidence {
    /// Absolute offset of the putative CRC32+ISIZE trailer, when known.
    pub trailer_offset: Option<usize>,
    pub present: bool,
    pub isize_matches_output: Option<bool>,
    pub isize_le: Option<u32>,
    pub crc32_le: Option<u32>,
    pub note: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct MemberInflateEvidence {
    pub gzip_offset: usize,
    pub ok: bool,
    pub header_length: Option<usize>,
    pub decompressed_length: Option<usize>,
    /// DEFLATE body bytes consumed (excludes gzip header).
    pub deflate_consumed_bytes: Option<usize>,
    /// Header + DEFLATE consumed from the prepared buffer.
    pub total_consumed_bytes: Option<usize>,
    pub failure_offset: Option<usize>,
    pub error: Option<String>,
    pub trailer: GzipTrailerEvidence,
}

#[derive(Debug, Clone, Serialize)]
pub struct ParserEvidence {
    pub kind: String,
    pub ok: bool,
    pub class_count: Option<usize>,
    pub field_count: Option<usize>,
    pub skipped_records: Option<usize>,
    pub class_name_count: Option<usize>,
    pub error: Option<String>,
    pub failure_offset: Option<usize>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ArmEvidence {
    /// `"control"` = raw stored bytes; `"experimental"` = page-stripped.
    pub arm: String,
    pub prepared_length: usize,
    pub gzip_member_offsets: Vec<usize>,
    pub members: Vec<MemberInflateEvidence>,
    pub first_member_decompressed_length: Option<usize>,
    pub inflate_all_chunks_count: usize,
    pub inflate_all_chunks_total_bytes: usize,
    pub first_failure_offset: Option<usize>,
    pub parser: Option<ParserEvidence>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ComparisonEvidence {
    pub outputs_equal: bool,
    pub control_decompressed_length: Option<usize>,
    pub experimental_decompressed_length: Option<usize>,
    pub length_delta: Option<i64>,
    pub first_divergence_offset: Option<usize>,
    pub note: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct StreamEvidence {
    pub stream_name: String,
    pub suspected_checksum_paged: bool,
    pub page_layout: PageLayoutEvidence,
    pub control: ArmEvidence,
    pub experimental: ArmEvidence,
    pub comparison: ComparisonEvidence,
    /// Whether production `rvt::compression::strip_revit_page_checksums`
    /// matches this harness's experimental strip (byte-identical).
    pub production_strip_matches_experimental: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct FileProvenance {
    pub path: String,
    pub file_name: String,
    pub file_type: String,
    pub file_size_bytes: u64,
    pub sample_hash_sha256: String,
    pub release: Option<u32>,
    pub build: Option<String>,
    pub guid: Option<String>,
    pub provenance: String,
    pub credit: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct EvidenceReport {
    pub schema_version: u32,
    pub harness: String,
    pub discussion: String,
    pub issue: String,
    pub file: FileProvenance,
    pub streams: Vec<StreamEvidence>,
}

fn gzip_header_len(data: &[u8], offset: usize) -> Option<usize> {
    rvt::compression::gzip_header_len(data, offset)
}

fn find_gzip_offsets(data: &[u8]) -> Vec<usize> {
    rvt::compression::find_gzip_offsets(data)
}

/// Inflate one gzip member and record consumed bytes + trailer probe.
pub fn inflate_member_evidence(data: &[u8], offset: usize) -> MemberInflateEvidence {
    let Some(header_len) = gzip_header_len(data, offset) else {
        return MemberInflateEvidence {
            gzip_offset: offset,
            ok: false,
            header_length: None,
            decompressed_length: None,
            deflate_consumed_bytes: None,
            total_consumed_bytes: None,
            failure_offset: Some(offset),
            error: Some("no gzip header".into()),
            trailer: GzipTrailerEvidence {
                trailer_offset: None,
                present: false,
                isize_matches_output: None,
                isize_le: None,
                crc32_le: None,
                note: "no gzip header; trailer not probed".into(),
            },
        };
    };
    let body_start = match offset.checked_add(header_len) {
        Some(v) => v,
        None => {
            return MemberInflateEvidence {
                gzip_offset: offset,
                ok: false,
                header_length: Some(header_len),
                decompressed_length: None,
                deflate_consumed_bytes: None,
                total_consumed_bytes: None,
                failure_offset: Some(offset),
                error: Some("gzip header offset overflow".into()),
                trailer: GzipTrailerEvidence {
                    trailer_offset: None,
                    present: false,
                    isize_matches_output: None,
                    isize_le: None,
                    crc32_le: None,
                    note: "header overflow".into(),
                },
            };
        }
    };
    let body = match data.get(body_start..) {
        Some(b) => b,
        None => {
            return MemberInflateEvidence {
                gzip_offset: offset,
                ok: false,
                header_length: Some(header_len),
                decompressed_length: None,
                deflate_consumed_bytes: None,
                total_consumed_bytes: None,
                failure_offset: Some(body_start),
                error: Some("gzip header extends past input".into()),
                trailer: GzipTrailerEvidence {
                    trailer_offset: None,
                    present: false,
                    isize_matches_output: None,
                    isize_le: None,
                    crc32_le: None,
                    note: "header past input".into(),
                },
            };
        }
    };

    let limits = rvt::compression::InflateLimits::default();
    let mut out = Vec::with_capacity(body.len().saturating_mul(4).min(limits.max_output_bytes));
    let mut decoder = DeflateDecoder::new(body);
    let mut buf = [0u8; 8192];
    loop {
        match decoder.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => {
                let next = out.len().saturating_add(n);
                if next > limits.max_output_bytes {
                    let consumed = decoder.total_in() as usize;
                    return MemberInflateEvidence {
                        gzip_offset: offset,
                        ok: false,
                        header_length: Some(header_len),
                        decompressed_length: Some(out.len()),
                        deflate_consumed_bytes: Some(consumed),
                        total_consumed_bytes: Some(header_len + consumed),
                        failure_offset: Some(body_start + consumed),
                        error: Some(format!(
                            "DEFLATE would exceed {} bytes",
                            limits.max_output_bytes
                        )),
                        trailer: GzipTrailerEvidence {
                            trailer_offset: None,
                            present: false,
                            isize_matches_output: None,
                            isize_le: None,
                            crc32_le: None,
                            note: "inflate aborted on output limit".into(),
                        },
                    };
                }
                out.extend_from_slice(&buf[..n]);
            }
            Err(e) => {
                let consumed = decoder.total_in() as usize;
                return MemberInflateEvidence {
                    gzip_offset: offset,
                    ok: false,
                    header_length: Some(header_len),
                    decompressed_length: Some(out.len()),
                    deflate_consumed_bytes: Some(consumed),
                    total_consumed_bytes: Some(header_len + consumed),
                    failure_offset: Some(body_start + consumed),
                    error: Some(format!("DEFLATE at offset {offset}: {e}")),
                    trailer: GzipTrailerEvidence {
                        trailer_offset: None,
                        present: false,
                        isize_matches_output: None,
                        isize_le: None,
                        crc32_le: None,
                        note: "inflate failed before trailer probe".into(),
                    },
                };
            }
        }
    }

    let deflate_consumed = decoder.total_in() as usize;
    let total_consumed = header_len + deflate_consumed;
    let trailer_offset = body_start + deflate_consumed;
    let trailer = probe_gzip_trailer(data, trailer_offset, out.len());

    MemberInflateEvidence {
        gzip_offset: offset,
        ok: true,
        header_length: Some(header_len),
        decompressed_length: Some(out.len()),
        deflate_consumed_bytes: Some(deflate_consumed),
        total_consumed_bytes: Some(total_consumed),
        failure_offset: None,
        error: None,
        trailer,
    }
}

fn probe_gzip_trailer(
    data: &[u8],
    trailer_offset: usize,
    decompressed_len: usize,
) -> GzipTrailerEvidence {
    if trailer_offset.saturating_add(8) > data.len() {
        return GzipTrailerEvidence {
            trailer_offset: Some(trailer_offset),
            present: false,
            isize_matches_output: None,
            isize_le: None,
            crc32_le: None,
            note: "fewer than 8 bytes remain after DEFLATE (typical Revit truncated-gzip)".into(),
        };
    }
    let t = &data[trailer_offset..trailer_offset + 8];
    let crc32_le = u32::from_le_bytes([t[0], t[1], t[2], t[3]]);
    let isize_le = u32::from_le_bytes([t[4], t[5], t[6], t[7]]);
    let expected_isize = decompressed_len as u32;
    let isize_matches = isize_le == expected_isize;
    GzipTrailerEvidence {
        trailer_offset: Some(trailer_offset),
        present: true,
        isize_matches_output: Some(isize_matches),
        isize_le: Some(isize_le),
        crc32_le: Some(crc32_le),
        note: if isize_matches {
            "8-byte RFC 1952 trailer present; ISIZE matches decompressed length".into()
        } else {
            "8 bytes present after DEFLATE but ISIZE does not match output (may be next member / noise)".into()
        },
    }
}

fn first_member_bytes(data: &[u8]) -> Option<Vec<u8>> {
    let offset = find_gzip_offsets(data).into_iter().next().unwrap_or(0);
    rvt::compression::inflate_at(data, offset).ok()
}

fn run_arm(arm: &str, prepared: &[u8], stream_name: &str) -> ArmEvidence {
    let offsets = find_gzip_offsets(prepared);
    // Detailed member evidence for the first gzip magic only. Full scans
    // of every magic hit are expensive on multi-MB streams and include
    // false positives inside compressed payloads; `inflate_all_chunks`
    // already covers multi-member aggregation.
    let probe_offsets: Vec<usize> = if offsets.is_empty() {
        vec![0]
    } else {
        vec![offsets[0]]
    };
    let members: Vec<_> = probe_offsets
        .iter()
        .copied()
        .map(|off| inflate_member_evidence(prepared, off))
        .collect();
    let first_failure_offset = members.iter().find_map(|m| m.failure_offset);
    let first_member_decompressed_length = members.first().and_then(|m| m.decompressed_length);

    let chunks = rvt::compression::inflate_all_chunks(prepared);
    let inflate_all_chunks_total_bytes: usize = chunks.iter().map(|c| c.len()).sum();

    let parser = if stream_name.eq_ignore_ascii_case("Formats/Latest") {
        Some(parse_formats_latest(prepared))
    } else {
        None
    };

    ArmEvidence {
        arm: arm.to_string(),
        prepared_length: prepared.len(),
        gzip_member_offsets: offsets,
        members,
        first_member_decompressed_length,
        inflate_all_chunks_count: chunks.len(),
        inflate_all_chunks_total_bytes,
        first_failure_offset,
        parser,
    }
}

fn parse_formats_latest(prepared: &[u8]) -> ParserEvidence {
    let decompressed = match first_member_bytes(prepared) {
        Some(d) => d,
        None => {
            return ParserEvidence {
                kind: "formats_schema".into(),
                ok: false,
                class_count: None,
                field_count: None,
                skipped_records: None,
                class_name_count: None,
                error: Some("inflate failed before schema parse".into()),
                failure_offset: Some(0),
            };
        }
    };
    let class_name_count = rvt::class_index::extract_class_names(&decompressed)
        .ok()
        .map(|n| n.len());
    match rvt::formats::parse_schema(&decompressed) {
        Ok(schema) => {
            let field_count: usize = schema.classes.iter().map(|c| c.fields.len()).sum();
            ParserEvidence {
                kind: "formats_schema".into(),
                ok: true,
                class_count: Some(schema.classes.len()),
                field_count: Some(field_count),
                skipped_records: Some(schema.skipped_records),
                class_name_count,
                error: None,
                failure_offset: None,
            }
        }
        Err(e) => ParserEvidence {
            kind: "formats_schema".into(),
            ok: false,
            class_count: None,
            field_count: None,
            skipped_records: None,
            class_name_count,
            error: Some(e.to_string()),
            failure_offset: None,
        },
    }
}

fn compare_arms(control_prepared: &[u8], experimental_prepared: &[u8]) -> ComparisonEvidence {
    let control = first_member_bytes(control_prepared);
    let experimental = first_member_bytes(experimental_prepared);
    match (control, experimental) {
        (Some(c), Some(e)) => {
            let first_divergence_offset = if c == e {
                None
            } else {
                Some(
                    c.iter()
                        .zip(e.iter())
                        .position(|(a, b)| a != b)
                        .unwrap_or(c.len().min(e.len())),
                )
            };
            ComparisonEvidence {
                outputs_equal: c == e,
                control_decompressed_length: Some(c.len()),
                experimental_decompressed_length: Some(e.len()),
                length_delta: Some(e.len() as i64 - c.len() as i64),
                first_divergence_offset,
                note: if c == e {
                    "control and experimental first-member outputs are byte-identical".into()
                } else {
                    "control and experimental first-member outputs diverge (silent corruption hypothesis when full pages present)".into()
                },
            }
        }
        (c, e) => ComparisonEvidence {
            outputs_equal: false,
            control_decompressed_length: c.as_ref().map(|v| v.len()),
            experimental_decompressed_length: e.as_ref().map(|v| v.len()),
            length_delta: match (&c, &e) {
                (Some(a), Some(b)) => Some(b.len() as i64 - a.len() as i64),
                _ => None,
            },
            first_divergence_offset: None,
            note: "one or both arms failed to inflate the first gzip member".into(),
        },
    }
}

/// Analyze one stored stream body under control vs experimental strip.
pub fn analyze_stored_stream(stream_name: &str, stored: &[u8]) -> StreamEvidence {
    let page_layout = analyze_page_layout(stored);
    let experimental_prepared = experimental_strip_page_checksums(stored);
    let production_strip = rvt::compression::strip_revit_page_checksums(stored);
    let production_strip_matches_experimental = production_strip == experimental_prepared;

    let control = run_arm("control", stored, stream_name);
    let experimental = run_arm("experimental", &experimental_prepared, stream_name);
    let comparison = compare_arms(stored, &experimental_prepared);

    StreamEvidence {
        stream_name: stream_name.to_string(),
        suspected_checksum_paged: is_suspected_checksum_paged_stream(stream_name),
        page_layout,
        control,
        experimental,
        comparison,
        production_strip_matches_experimental,
    }
}

fn file_type_of(path: &Path) -> String {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_ascii_lowercase())
        .filter(|e| matches!(e.as_str(), "rvt" | "rfa" | "rte" | "rft"))
        .unwrap_or_else(|| "unknown".into())
}

fn sha256_file(path: &Path) -> anyhow::Result<(u64, String)> {
    let bytes = std::fs::read(path)?;
    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    Ok((bytes.len() as u64, hex::encode(hasher.finalize())))
}

fn provenance_note(path: &Path) -> String {
    let s = path.to_string_lossy();
    if s.contains("corpus/tier1") {
        "synthetic license-free tier1 fixture (gen-fixture)".into()
    } else if s.contains("_project_corpus") || s.contains("revit-test-datasets") {
        "external magnetar-io/revit-test-datasets corpus (not redistributed; local probe only)"
            .into()
    } else if s.contains("_corpus") || s.contains("phi-ag") {
        "external phi-ag/rvt sample corpus (not redistributed; local probe only)".into()
    } else if s.contains("gen-fixture") || s.contains("rvt-demo") || s.contains("/tmp/") {
        "synthetic gen-fixture output".into()
    } else {
        "local file path supplied to stream-evidence harness".into()
    }
}

fn list_cfb_streams(path: &Path) -> anyhow::Result<Vec<(String, Vec<u8>)>> {
    let mut rf = rvt::RevitFile::open(path)?;
    // Evidence probes may need multi-MB partition streams; raise the
    // per-stream cap without touching production defaults for other callers.
    let limit = 512 * 1024 * 1024;
    let names = rf.stream_names();
    let mut out = Vec::with_capacity(names.len());
    for name in names {
        match rf.read_stream_with_limit(&name, limit) {
            Ok(bytes) => out.push((name, bytes)),
            Err(e) => {
                // Skip unreadable streams but keep going for evidence.
                eprintln!("stream-evidence: skip {name}: {e}");
            }
        }
    }
    Ok(out)
}

/// Build a full evidence report for selected streams in a Revit file.
pub fn analyze_file(
    path: &Path,
    stream_filter: Option<&[String]>,
    all_paged: bool,
    include_empty: bool,
) -> anyhow::Result<EvidenceReport> {
    let (file_size_bytes, sample_hash_sha256) = sha256_file(path)?;
    let mut release = None;
    let mut build = None;
    let mut guid = None;
    if let Ok(mut rf) = rvt::RevitFile::open(path) {
        if let Ok(info) = rf.basic_file_info() {
            release = Some(info.version);
            build = info.build;
            guid = info.guid;
        }
    }

    let streams_raw = list_cfb_streams(path)?;
    let selected: Vec<(String, Vec<u8>)> = streams_raw
        .into_iter()
        .filter(|(name, bytes)| {
            if !include_empty && bytes.is_empty() {
                return false;
            }
            if let Some(filter) = stream_filter {
                return filter.iter().any(|f| f.eq_ignore_ascii_case(name));
            }
            if all_paged {
                return is_suspected_checksum_paged_stream(name);
            }
            // Default: Formats/Latest only.
            name.eq_ignore_ascii_case("Formats/Latest")
        })
        .collect();

    let selected = if selected.is_empty() {
        if stream_filter.is_none() && !all_paged {
            // Fall back: first non-empty stream for smoke tests on tiny fixtures.
            list_cfb_streams(path)?
                .into_iter()
                .filter(|(_, b)| !b.is_empty())
                .take(1)
                .collect()
        } else if let Some(filter) = stream_filter {
            let requested = filter.join(", ");
            anyhow::bail!("no stream matched the requested filter: {requested}");
        } else {
            anyhow::bail!("no suspected checksum-paged non-empty streams found");
        }
    } else {
        selected
    };

    if selected.is_empty() {
        anyhow::bail!("no non-empty streams found in {}", path.display());
    }

    let streams: Vec<StreamEvidence> = selected
        .iter()
        .map(|(name, bytes)| analyze_stored_stream(name, bytes))
        .collect();

    Ok(EvidenceReport {
        schema_version: EVIDENCE_SCHEMA_VERSION,
        harness: "stream-evidence".into(),
        discussion: "https://github.com/DrunkOnJava/rvt-rs/discussions/112".into(),
        issue: "https://github.com/DrunkOnJava/rvt-rs/issues/151".into(),
        file: FileProvenance {
            path: path.display().to_string(),
            file_name: path
                .file_name()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_default(),
            file_type: file_type_of(path),
            file_size_bytes,
            sample_hash_sha256,
            release,
            build,
            guid,
            provenance: provenance_note(path),
            credit: "@STE1200 (Steffen) — Discussion #112 Finding 1; harness by Wave 1 Worker B"
                .into(),
        },
        streams,
    })
}

/// Convenience: write pretty JSON to a path.
pub fn write_report(report: &EvidenceReport, out: &Path) -> anyhow::Result<()> {
    if let Some(parent) = out.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let json = serde_json::to_string_pretty(report)?;
    std::fs::write(out, json)?;
    Ok(())
}

/// Resolve stream name arguments into owned strings.
pub fn stream_names_from_args(raw: &[String]) -> Option<Vec<String>> {
    if raw.is_empty() {
        None
    } else {
        Some(raw.to_vec())
    }
}

#[allow(dead_code)]
pub fn default_output_path(input: &Path) -> PathBuf {
    let stem = input
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "stream_evidence".into());
    PathBuf::from(format!("{stem}.stream_evidence.json"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use rvt::compression::truncated_gzip_encode;

    #[test]
    fn experimental_strip_removes_full_page_trailers() {
        let mut stored = vec![0xABu8; REVIT_PAGE_PAYLOAD_BYTES];
        stored.extend(vec![0xCDu8; REVIT_PAGE_CHECKSUM_BYTES]);
        stored.extend(vec![0xEFu8; 100]);
        let clean = experimental_strip_page_checksums(&stored);
        assert_eq!(clean.len(), REVIT_PAGE_PAYLOAD_BYTES + 100);
        assert!(!clean.contains(&0xCD));
    }

    #[test]
    fn page_layout_reports_boundaries() {
        let stored = vec![0u8; REVIT_STORED_PAGE_BYTES + 10];
        let layout = analyze_page_layout(&stored);
        assert_eq!(layout.full_page_count, 1);
        assert_eq!(layout.remainder_bytes, 10);
        assert_eq!(
            layout.suspected_boundaries,
            vec![0, REVIT_STORED_PAGE_BYTES]
        );
        assert_eq!(layout.pages[0].tail_length, REVIT_PAGE_CHECKSUM_BYTES);
        assert_eq!(layout.pages[1].tail_length, 0);
    }

    #[test]
    fn control_vs_experimental_diverge_when_checksum_embedded() {
        // Highly incompressible payload so truncated-gzip stays large enough
        // to span more than one 64_896-byte stored page after framing.
        let mut payload = Vec::with_capacity(200_000);
        let mut state = 0xC0FFEE_u32;
        for _ in 0..200_000 {
            state = state.wrapping_mul(1103515245).wrapping_add(12345);
            payload.push((state >> 16) as u8);
        }
        let gzip = truncated_gzip_encode(&payload).expect("encode");
        assert!(
            gzip.len() > REVIT_PAGE_PAYLOAD_BYTES,
            "encoded gzip too small to cross a page: {}",
            gzip.len()
        );
        // Reshape into fake paged storage: 64896 payload + 353 junk per page.
        let mut stored = Vec::new();
        let mut rest = gzip.as_slice();
        while rest.len() >= REVIT_PAGE_PAYLOAD_BYTES {
            stored.extend_from_slice(&rest[..REVIT_PAGE_PAYLOAD_BYTES]);
            stored.extend(vec![0x5Au8; REVIT_PAGE_CHECKSUM_BYTES]);
            rest = &rest[REVIT_PAGE_PAYLOAD_BYTES..];
        }
        stored.extend_from_slice(rest);
        assert!(stored.len() > REVIT_STORED_PAGE_BYTES);

        let report = analyze_stored_stream("Formats/Latest", &stored);
        assert!(report.page_layout.full_page_count >= 1);
        assert!(report.production_strip_matches_experimental);
        // Stripped should restore the original gzip and inflate cleanly.
        assert_eq!(
            report.experimental.first_member_decompressed_length,
            Some(payload.len())
        );
        // Control leaves checksum junk in the bitstream — typically shorter
        // or divergent output (silent success).
        assert!(!report.comparison.outputs_equal);
    }

    #[test]
    fn suspected_paths() {
        assert!(is_suspected_checksum_paged_stream("Formats/Latest"));
        assert!(is_suspected_checksum_paged_stream("Global/ElemTable"));
        assert!(is_suspected_checksum_paged_stream("Partitions/67"));
        assert!(!is_suspected_checksum_paged_stream("BasicFileInfo"));
    }
}
