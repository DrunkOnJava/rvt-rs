//! Truncated-gzip decompression for Revit streams.
//!
//! Revit writes standard gzip file headers (magic `1F 8B 08` + 10-byte
//! minimum header) but omits the trailing 8-byte CRC32 + ISIZE that
//! RFC 1952 requires. `flate2::read::GzDecoder` validates those trailing
//! bytes and refuses truncated streams, so we skip the gzip header manually
//! and pump the raw DEFLATE body through `flate2::read::DeflateDecoder`.

use crate::le_bytes;
use crate::{Error, Result};
use flate2::read::DeflateDecoder;
use std::io::{Read, Write};

pub const GZIP_MAGIC: [u8; 3] = [0x1F, 0x8B, 0x08];

/// Default maximum decompressed size per inflate call.
///
/// 256 MiB is comfortably above any real `Formats/Latest` or
/// `Global/Latest` stream we've observed across the 11-release
/// corpus (typical is 1–5 MiB; largest observed ~40 MiB on a
/// worksharing project RVT), and comfortably below the point where
/// a compressed-bomb attack becomes a credible DoS. Callers with
/// larger legitimate needs can override by passing a custom
/// [`InflateLimits`] to [`inflate_at_with_limits`].
pub const DEFAULT_MAX_INFLATE_BYTES: usize = 256 * 1024 * 1024;

/// Default maximum inflated bytes of one multi-member stream, all its members
/// together: what [`InflateLimits::max_stream_total_bytes`] gives at the
/// default per-member cap.
pub const DEFAULT_MAX_INFLATED_STREAM_BYTES: usize = 1024 * 1024 * 1024;

/// Upper bound on the capacity reserved before a DEFLATE member is
/// decoded. The reservation must not scale with the input that follows
/// the member: [`inflate_all_chunks_with_limits`] decodes every member
/// of a stream with the whole remainder as `data`, keeps each result
/// alive, and a 33 MB Revit 2024 project holds dozens of members, so a
/// `4 x remaining` hint reserved gigabytes. Native allocators back
/// untouched capacity lazily; wasm32 linear memory commits every grown
/// page and trapped at the 4 GiB ceiling in ~140 ms (viewer demo
/// core-interior-2024). Amortised `Vec` growth covers larger outputs.
const INITIAL_INFLATE_CAPACITY_BYTES: usize = 1024 * 1024;

/// Caps for bounded decompression. Passed explicitly to
/// [`inflate_at_with_limits`] or pulled from
/// [`InflateLimits::default`] by the back-compat [`inflate_at`]
/// wrapper.
///
/// # Why this exists
///
/// DEFLATE is an adversary-choice format: a small number of input
/// bytes can legitimately expand by 1000× or more (e.g. a 1 KB
/// block of zeros compresses to ~20 bytes; uncompressed is 1 KB;
/// a 1 MB block of zeros compresses to ~1 KB). The previous
/// `inflate_at` allocated `body.len() * 4` without a hard upper
/// bound and called `read_to_end` with no size limit, so a
/// hostile `.rvt` could trivially DoS any process that opened it.
///
/// An April 2026 security audit found it the most urgent fix the
/// project needed.
#[derive(Debug, Clone, Copy)]
pub struct InflateLimits {
    /// Maximum decompressed output bytes per call.
    pub max_output_bytes: usize,
}

impl Default for InflateLimits {
    fn default() -> Self {
        Self {
            max_output_bytes: DEFAULT_MAX_INFLATE_BYTES,
        }
    }
}

impl InflateLimits {
    /// The cap on one multi-member stream's inflated members together:
    /// [`DEFAULT_MAX_INFLATED_STREAM_BYTES`], or `max_output_bytes` when that
    /// is larger, so raising the per-member cap for a large model raises
    /// the stream's total with it (#1075).
    pub fn max_stream_total_bytes(&self) -> usize {
        DEFAULT_MAX_INFLATED_STREAM_BYTES.max(self.max_output_bytes)
    }
}

/// What a multi-member inflate left out because a limit stopped it (#1075).
/// Members that fail to inflate for any other reason are gzip-magic false
/// positives inside compressed data and are not counted.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct InflateShortfall {
    /// Members skipped because each would inflate past
    /// [`InflateLimits::max_output_bytes`].
    pub members_over_limit: usize,
    /// True when the members stopped at the stream's inflated total
    /// ([`InflateLimits::max_stream_total_bytes`]) and the rest were not
    /// inflated.
    pub stopped_at_total: bool,
}

impl InflateShortfall {}

/// Stored bytes in a full checksum page (`PagedStreamImplReader<..., 65249>`).
///
/// Independently documented by ahzs645/reviter's ODA loader analysis and
/// Discussion #112: each complete stored page is 64_896 payload bytes plus a
/// 353-byte checksum/ECC trailer.
pub const REVIT_STORED_PAGE_BYTES: usize = 65_249;

/// Original payload bytes recovered from a full checksum page.
pub const REVIT_PAGE_PAYLOAD_BYTES: usize = 64_896;

/// Trailer length cut from each complete stored page before inflate.
pub const REVIT_PAGE_CHECKSUM_BYTES: usize = REVIT_STORED_PAGE_BYTES - REVIT_PAGE_PAYLOAD_BYTES;

/// Normalize a CFB stream path for gate comparisons.
fn clean_stream_path(path: &str) -> &str {
    let clean = path
        .trim_start_matches('/')
        .trim_start_matches("Root Entry/");
    clean.trim_start_matches('/')
}

/// Whether production inflate wrappers strip checksum-page trailers.
///
/// `Partitions/*`, the listed `Global/*` streams and, since #410,
/// `Formats/Latest` (issue #151 / Discussion #112). `Formats/Latest` was left
/// out at first because stripping it lowered an opportunistic class-name
/// count; that count was drift garbage. With the strip, every file of one
/// Revit release inflates to the byte-identical schema, family or project,
/// and without it no two do (`probes/examples/probe_schema_page_strip.rs`).
///
/// `ProjectInformation`, `PartAtom`, `BasicFileInfo`, and preview streams are
/// never gated — they are not routed through the paged gzip reader.
///
/// Raw [`inflate_at`] / [`inflate_all_chunks`] never strip; only
/// [`prepare_stream_for_inflate`] / [`inflate_stream_at`] /
/// [`inflate_stream_auto`] / [`inflate_all_chunks_for_stream`] consult this gate.
pub fn is_checksum_paged_stream(path: &str) -> bool {
    let clean = clean_stream_path(path);
    if let Some(rest) = clean
        .strip_prefix("Partitions/")
        .or_else(|| clean.strip_prefix("partitions/"))
    {
        return !rest.is_empty() && !rest.contains('/');
    }
    matches!(
        clean,
        "Formats/Latest"
            | "Global/ContentDocuments"
            | "Global/DocumentIncrementTable"
            | "Global/ElemTable"
            | "Global/History"
            | "Global/Latest"
            | "Global/PartitionTable"
    )
}

/// Broader candidate set aligned with reviter's paged loader (includes
/// `Formats/Latest`) for probes and research — **not** the production strip gate.
///
/// Prefer [`is_checksum_paged_stream`] at decode call sites.
pub fn is_revit_paged_loader_candidate(path: &str) -> bool {
    let clean = clean_stream_path(path);
    if clean.eq_ignore_ascii_case("Formats/Latest") {
        return true;
    }
    is_checksum_paged_stream(path)
}

/// Remove each complete stored page's checksum trailer.
///
/// The last short page is retained in full (matching reviter's
/// `stripRevitPageChecksums`). Call this on **stored** CFB bytes for
/// [`is_checksum_paged_stream`] paths **before** [`inflate_at`] /
/// [`inflate_all_chunks`]. Do **not** run this on writer identity copies —
/// [`crate::reader::RevitFile::read_stream`] returns stored bytes unchanged.
pub fn strip_revit_page_checksums(data: &[u8]) -> Vec<u8> {
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

/// Return checksum-clean bytes ready for inflate when `stream_name` is gated.
///
/// Does **not** mutate writer identity copies: [`crate::reader::RevitFile::read_stream`]
/// still returns stored CFB bytes unchanged. Decode-path only.
pub fn prepare_stream_for_inflate<'a>(
    stream_name: &str,
    stored: &'a [u8],
) -> std::borrow::Cow<'a, [u8]> {
    if is_checksum_paged_stream(stream_name) {
        std::borrow::Cow::Owned(strip_revit_page_checksums(stored))
    } else {
        std::borrow::Cow::Borrowed(stored)
    }
}

/// Inflate a named stored stream, stripping page checksums when
/// [`is_checksum_paged_stream`] says so.
///
/// Prefer this over bare [`inflate_at`] whenever the CFB stream path is known.
/// Raw [`inflate_at`] remains a path-agnostic codec and **never** strips — so
/// Formats/Latest, tiny streams, and already-clean buffers stay untouched.
pub fn inflate_stream_at(stream_name: &str, stored: &[u8], offset: usize) -> Result<Vec<u8>> {
    inflate_stream_at_with_limits(stream_name, stored, offset, InflateLimits::default())
}

/// [`inflate_stream_at`] under `limits`.
pub fn inflate_stream_at_with_limits(
    stream_name: &str,
    stored: &[u8],
    offset: usize,
    limits: InflateLimits,
) -> Result<Vec<u8>> {
    let prepared = prepare_stream_for_inflate(stream_name, stored);
    inflate_at_with_limits(prepared.as_ref(), offset, limits)
}

/// Auto-offset inflate for a named stored stream (page-stripped when gated).
pub fn inflate_stream_auto(stream_name: &str, stored: &[u8]) -> Result<(usize, Vec<u8>)> {
    inflate_stream_auto_with_limits(stream_name, stored, InflateLimits::default())
}

/// [`inflate_stream_auto`] under `limits`.
pub fn inflate_stream_auto_with_limits(
    stream_name: &str,
    stored: &[u8],
    limits: InflateLimits,
) -> Result<(usize, Vec<u8>)> {
    let prepared = prepare_stream_for_inflate(stream_name, stored);
    inflate_at_auto_with_limits(prepared.as_ref(), limits)
}

/// Inflate every gzip member in a named stored stream (page-stripped when gated).
///
/// Primary production entry for multi-member `Partitions/*` recovery (#151).
pub fn inflate_all_chunks_for_stream(stream_name: &str, stored: &[u8]) -> Vec<Vec<u8>> {
    let prepared = prepare_stream_for_inflate(stream_name, stored);
    inflate_all_chunks(prepared.as_ref())
}

/// Every gzip member of one stored stream, inflated and concatenated.
///
/// Built once per stream by [`crate::reader::RevitFile::inflated_partition`]
/// and shared by every partition consumer. Holding the concatenation
/// (rather than the member vector) is what the scanners actually want:
/// each of them re-joined the members itself, and the byte-at-a-time
/// `chunks.into_iter().flatten().collect()` those sites used cost more
/// than the inflate on a project file.
#[derive(Debug, Default)]
pub struct InflatedStream {
    concat: Vec<u8>,
    chunk_ends: Vec<usize>,
    /// Each member's offset in the prepared stream and the bytes its gzip
    /// header and DEFLATE stream take there.
    // Tracker: B82.
    members: Vec<(usize, usize)>,
}

impl InflatedStream {
    /// Inflate every member of `stored` and concatenate them in order, at
    /// the default [`InflateLimits`].
    pub fn from_stored(stream_name: &str, stored: &[u8]) -> Self {
        Self::from_stored_with_limits(stream_name, stored, InflateLimits::default()).0
    }

    /// [`Self::from_stored`] under `limits`: each member is capped at
    /// `max_output_bytes` and the members together at
    /// [`InflateLimits::max_stream_total_bytes`], with what the caps left out.
    pub fn from_stored_with_limits(
        stream_name: &str,
        stored: &[u8],
        limits: InflateLimits,
    ) -> (Self, InflateShortfall) {
        let prepared = prepare_stream_for_inflate(stream_name, stored);
        let (chunks, shortfall) = inflate_all_members_with_limits(
            prepared.as_ref(),
            limits,
            limits.max_stream_total_bytes(),
        );
        let total: usize = chunks.iter().map(|(_, _, chunk)| chunk.len()).sum();
        let mut concat = Vec::with_capacity(total);
        let mut chunk_ends = Vec::with_capacity(chunks.len());
        let mut members = Vec::with_capacity(chunks.len());
        for (offset, consumed, chunk) in &chunks {
            concat.extend_from_slice(chunk);
            chunk_ends.push(concat.len());
            members.push((*offset, *consumed));
        }
        (
            Self {
                concat,
                chunk_ends,
                members,
            },
            shortfall,
        )
    }

    /// The inflated member whose gzip header starts at `offset` of the
    /// prepared stream ([`prepare_stream_for_inflate`]), with the bytes its
    /// header and DEFLATE stream take there.
    // Tracker: B82.
    pub fn member_at(&self, offset: usize) -> Option<(&[u8], usize)> {
        let i = self
            .members
            .binary_search_by_key(&offset, |&(start, _)| start)
            .ok()?;
        let start = i.checked_sub(1).map_or(0, |prev| self.chunk_ends[prev]);
        Some((&self.concat[start..self.chunk_ends[i]], self.members[i].1))
    }

    /// The inflated members, concatenated in stored order.
    pub fn bytes(&self) -> &[u8] {
        &self.concat
    }

    /// End offset of each member within [`Self::bytes`].
    pub fn chunk_ends(&self) -> &[usize] {
        &self.chunk_ends
    }

    /// Number of inflated members.
    pub fn chunk_count(&self) -> usize {
        self.chunk_ends.len()
    }

    /// Iterate the members as slices of [`Self::bytes`].
    pub fn chunks(&self) -> impl Iterator<Item = &[u8]> {
        let mut start = 0usize;
        self.chunk_ends.iter().map(move |end| {
            let slice = &self.concat[start..*end];
            start = *end;
            slice
        })
    }
}

/// Whether production inflate strips checksum-page trailers for a stream.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum ChecksumTailStripping {
    /// No longer produced for `Formats/Latest` (#410); kept for readers of
    /// older diagnostics.
    Disabled,
    Enabled,
}

/// Coarse integrity verdict for `Formats/Latest` after production inflate.
///
/// Multipage streams stay [`FormatsIntegrityStatus::Uncertain`] while strip
/// remains disabled — do **not** treat inflate success as completeness proof
/// (Discussion #112 / #151 residual).
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum FormatsIntegrityStatus {
    /// Stored length is under one full checksum page and inflate succeeded.
    Ok,
    /// At least one full page boundary is present; completeness unverified.
    Uncertain,
    /// Stream missing or production inflate failed.
    Incomplete,
}

/// Stable diagnostic code for Formats multipage integrity.
pub const RVT_FORMATS_MULTIPAGE_UNVERIFIED: &str = "RVT_FORMATS_MULTIPAGE_UNVERIFIED";

/// Machine-readable `Formats/Latest` integrity sidecar for inspect / export /
/// viewer File Status.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[non_exhaustive]
pub struct FormatsLatestIntegrity {
    pub stream: String,
    pub stored_bytes: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inflated_bytes: Option<usize>,
    pub page_boundary_detected: bool,
    pub checksum_tail_stripping: ChecksumTailStripping,
    pub integrity_status: FormatsIntegrityStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub diagnostic_code: Option<String>,
    /// `true` when the inflated stream is larger than
    /// [`crate::formats::SCHEMA_SCAN_LIMIT`], so schema parsing stops before
    /// the end of the stream. A "we stopped looking" signal — it does not
    /// claim the unscanned tail is parseable schema.
    #[serde(default)]
    pub schema_scan_truncated: bool,
    /// Bytes schema parsing would scan — `min(inflated_bytes,
    /// SCHEMA_SCAN_LIMIT)`, or `None` when inflate failed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub schema_scanned_bytes: Option<usize>,
}

impl FormatsLatestIntegrity {
    /// Short user-facing summary for CLI / File Status rows.
    pub fn summary_line(&self) -> String {
        let mut line = match self.integrity_status {
            FormatsIntegrityStatus::Ok => format!(
                "Formats/Latest · {} stored · inflate ok · single-page",
                self.stored_bytes
            ),
            FormatsIntegrityStatus::Uncertain => format!(
                "Formats/Latest · {} stored · multipage integrity uncertain (strip disabled)",
                self.stored_bytes
            ),
            FormatsIntegrityStatus::Incomplete => format!(
                "Formats/Latest · {} stored · inflate incomplete",
                self.stored_bytes
            ),
        };
        if self.schema_scan_truncated {
            let scanned = self
                .schema_scanned_bytes
                .unwrap_or(crate::formats::SCHEMA_SCAN_LIMIT);
            let total = self.inflated_bytes.unwrap_or(scanned);
            line.push_str(&format!(
                " · schema scan truncated ({scanned} of {total} inflated bytes scanned)"
            ));
        }
        line
    }
}

/// Diagnose `Formats/Latest`: its size, whether it spans checksum pages,
/// and whether it inflates once they are stripped (#410).
pub fn diagnose_formats_latest_integrity(stored: &[u8]) -> FormatsLatestIntegrity {
    let stored_bytes = stored.len();
    let page_boundary_detected = stored_bytes >= REVIT_STORED_PAGE_BYTES;
    let inflate = inflate_stream_at(crate::streams::FORMATS_LATEST, stored, 0);
    let inflated_bytes = inflate.as_ref().ok().map(|b| b.len());
    // The stream ends in a real gzip trailer: after the strip its CRC32 and
    // ISIZE verify on every file measured (#410), so a stream is `Ok` only
    // when they do. Accepting the strip on that condition is rosejn's (#255).
    let (integrity_status, diagnostic_code) = match &inflate {
        Err(_) => (FormatsIntegrityStatus::Incomplete, None),
        Ok(_)
            if gzip_trailer_verifies(&prepare_stream_for_inflate(
                crate::streams::FORMATS_LATEST,
                stored,
            )) =>
        {
            (FormatsIntegrityStatus::Ok, None)
        }
        Ok(_) => (
            FormatsIntegrityStatus::Uncertain,
            Some(RVT_FORMATS_MULTIPAGE_UNVERIFIED.to_string()),
        ),
    };
    let schema_scanned_bytes = inflated_bytes;
    let schema_scan_truncated = false;

    FormatsLatestIntegrity {
        stream: crate::streams::FORMATS_LATEST.to_string(),
        stored_bytes,
        inflated_bytes,
        page_boundary_detected,
        checksum_tail_stripping: ChecksumTailStripping::Enabled,
        integrity_status,
        diagnostic_code,
        schema_scan_truncated,
        schema_scanned_bytes,
    }
}

/// True when `data` is one gzip member whose CRC32 and ISIZE trailer match
/// what it inflates to, within [`DEFAULT_MAX_INFLATE_BYTES`].
fn gzip_trailer_verifies(data: &[u8]) -> bool {
    let mut decoder = flate2::read::GzDecoder::new(data);
    let mut buf = [0u8; 8192];
    let mut total = 0usize;
    loop {
        match Read::read(&mut decoder, &mut buf) {
            Ok(0) => return true,
            Ok(n) => {
                total = total.saturating_add(n);
                if total > DEFAULT_MAX_INFLATE_BYTES {
                    return false;
                }
            }
            Err(_) => return false,
        }
    }
}

/// Returns `true` iff `data` starts with the gzip magic at the given offset.
pub fn has_gzip_magic(data: &[u8], offset: usize) -> bool {
    offset
        .checked_add(GZIP_MAGIC.len())
        .and_then(|end| data.get(offset..end))
        == Some(GZIP_MAGIC.as_slice())
}

/// Length of the gzip header starting at `offset`, or `None` if no magic.
///
/// Standard gzip: magic(3) + method(1) + flags(1) + mtime(4) + xfl(1) + os(1) = 10 bytes.
/// Plus optional FEXTRA / FNAME / FCOMMENT / FHCRC fields when flags are set.
pub fn gzip_header_len(data: &[u8], offset: usize) -> Option<usize> {
    if !has_gzip_magic(data, offset) {
        return None;
    }
    let flags = *data.get(offset.checked_add(3)?)?;
    let mut pos = offset.checked_add(10)?;
    if flags & 0x04 != 0 {
        // FEXTRA: 2-byte LE length, then that many bytes
        let xlen_end = pos.checked_add(2)?;
        let xlen = le_bytes::u16_at(data, pos)? as usize;
        pos = xlen_end.checked_add(xlen)?;
    }
    if flags & 0x08 != 0 {
        // FNAME: null-terminated string. Bounds-check `pos` first —
        // the 10-byte base header might not fit if `offset + 10 >
        // data.len()`, in which case slicing `data[pos..]` would
        // panic. Discovered by libFuzzer fuzz_inflate_at_with_limits
        // 2026-04-21 on input `1f 8b 08 b9` (FNAME+FHCRC flag set,
        // 4-byte buffer).
        pos = data
            .get(pos..)?
            .iter()
            .position(|&b| b == 0)
            .and_then(|i| pos.checked_add(i)?.checked_add(1))?;
    }
    if flags & 0x10 != 0 {
        // FCOMMENT: null-terminated string. Same bounds-check
        // rationale as FNAME above.
        pos = data
            .get(pos..)?
            .iter()
            .position(|&b| b == 0)
            .and_then(|i| pos.checked_add(i)?.checked_add(1))?;
    }
    if flags & 0x02 != 0 {
        // FHCRC: 2-byte header CRC
        pos = pos.checked_add(2)?;
    }
    // The base 10-byte header is set unconditionally at `pos = offset
    // + 10` above, but none of the `data.get(..)` probes on the
    // optional flag fields fire when those flags are clear. Verify
    // the base header actually fits in the buffer before returning —
    // otherwise a caller using the returned length to slice past
    // `data.len()` will panic. Discovered by the Q-04 regression
    // harness on a 9-byte `[0x1f, 0x8b, 0x08, 0x00, ...]` input.
    if pos > data.len() {
        return None;
    }
    Some(pos - offset)
}

/// Inflate the DEFLATE stream that follows a gzip header starting at
/// `offset`, with an output-size ceiling enforced.
///
/// Returns the decompressed bytes. Unused tail (any garbage / next
/// chunk / missing CRC+ISIZE) is silently ignored — which is exactly
/// what we need for Revit's truncated-gzip streams.
///
/// The output size is capped by `limits.max_output_bytes`. If a
/// DEFLATE block would expand beyond that cap, the function returns
/// [`Error::DecompressLimitExceeded`] rather than allocating
/// unbounded memory. The initial allocation is also clamped — the
/// historic `body.len() * 4` hint is bounded to at most
/// `max_output_bytes` so an attacker can't force a multi-GB
/// allocation just by handing us a large compressed body.
///
/// ```
/// # use rvt::compression::{self, InflateLimits};
/// let empty_truncated_gzip = [
///     0x1f, 0x8b, 0x08, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xff,
///     0x03, 0x00,
/// ];
/// let decomp = compression::inflate_at_with_limits(
///     &empty_truncated_gzip, 0, InflateLimits::default()
/// ).unwrap();
/// assert_eq!(decomp, b"");
/// ```
pub fn inflate_at_with_limits(
    data: &[u8],
    offset: usize,
    limits: InflateLimits,
) -> Result<Vec<u8>> {
    inflate_member_at_with_limits(data, offset, limits).map(|(out, _)| out)
}

/// [`inflate_at_with_limits`], with the number of bytes of `data` the gzip
/// header and its DEFLATE stream take from `offset`.
// Tracker: B82.
pub fn inflate_member_at_with_limits(
    data: &[u8],
    offset: usize,
    limits: InflateLimits,
) -> Result<(Vec<u8>, usize)> {
    let header_len =
        gzip_header_len(data, offset).ok_or_else(|| Error::Decompress("no gzip header".into()))?;
    let body_start = offset
        .checked_add(header_len)
        .ok_or_else(|| Error::Decompress("gzip header offset overflow".into()))?;
    let body = data
        .get(body_start..)
        .ok_or_else(|| Error::Decompress("gzip header extends past input".into()))?;
    // Clamp initial capacity. `body.len() * 4` was the historical hint,
    // but `body` is the whole remainder of the stream, so it is bounded
    // by [`INITIAL_INFLATE_CAPACITY_BYTES`] (see that constant) as well
    // as by the caller's output limit.
    let cap = body
        .len()
        .saturating_mul(4)
        .min(INITIAL_INFLATE_CAPACITY_BYTES)
        .min(limits.max_output_bytes);
    let mut out = Vec::with_capacity(cap);
    // Chunked read loop so we can enforce the cap deterministically.
    let mut decoder = DeflateDecoder::new(body);
    let mut buf = [0u8; 8192];
    loop {
        let n = decoder
            .read(&mut buf)
            .map_err(|e| Error::Decompress(format!("DEFLATE at offset {offset}: {e}")))?;
        if n == 0 {
            break;
        }
        let next_len = out.len().checked_add(n).ok_or_else(|| {
            Error::DecompressLimitExceeded("DEFLATE output length overflow".into())
        })?;
        if next_len > limits.max_output_bytes {
            return Err(Error::DecompressLimitExceeded(format!(
                "DEFLATE at offset {offset} would exceed {} bytes",
                limits.max_output_bytes
            )));
        }
        out.extend_from_slice(&buf[..n]);
    }
    // Results are retained per member by the chunk collectors; trim the
    // amortised-growth slack so retained buffers cost what they hold.
    out.shrink_to_fit();
    Ok((out, header_len + decoder.total_in() as usize))
}

/// Inflate the DEFLATE stream that follows a gzip header starting at `offset`.
///
/// Backwards-compatible wrapper around [`inflate_at_with_limits`] with
/// the default [`InflateLimits`] (256 MiB per call). Existing callers
/// continue to work unchanged; the cap only kicks in on adversarial
/// input that would not have produced a useful result anyway.
///
/// ```
/// # use rvt::compression;
/// // Revit omits the trailing CRC+ISIZE, so standard gzip decoders refuse
/// // these streams. Our `inflate_at` returns the decompressed bytes
/// // regardless.
/// let empty_truncated_gzip = [
///     0x1f, 0x8b, 0x08, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xff, // 10-byte header
///     0x03, 0x00, // empty deflate block
/// ];
/// let decomp = compression::inflate_at(&empty_truncated_gzip, 0).unwrap();
/// assert_eq!(decomp, b"");
/// ```
pub fn inflate_at(data: &[u8], offset: usize) -> Result<Vec<u8>> {
    inflate_at_with_limits(data, offset, InflateLimits::default())
}

/// Inflate a Revit Global/* stream without requiring the caller to
/// know the custom-prefix length.
///
/// Family files (the phi-ag/rvt corpus through 2026) put an 8-byte
/// custom header in front of the gzip body on most Global/* streams
/// (`Global/History`, `Global/Latest`, `Global/PartitionTable`, etc).
/// Project files (`.rvt`) don't always — on at least some releases
/// (observed on a 2025-saved `.rvt`) `Global/History` starts with the
/// gzip magic at offset 0 and has no custom prefix at all.
///
/// This helper probes for the first gzip magic in `data` via
/// [`find_gzip_offsets`] and inflates from there. If no magic is
/// found, it falls back to the family-file heuristic of offset 8 so
/// callers still see a structured `Err(..)` for diagnostics rather
/// than a silent mismatch.
///
/// Returns the tuple `(prefix_len, decompressed_bytes)` so callers
/// that need to preserve the custom prefix on round-trip write can
/// read it from `data[..prefix_len]`.
pub fn inflate_at_auto(data: &[u8]) -> Result<(usize, Vec<u8>)> {
    inflate_at_auto_with_limits(data, InflateLimits::default())
}

/// [`inflate_at_auto`] under `limits`.
pub fn inflate_at_auto_with_limits(data: &[u8], limits: InflateLimits) -> Result<(usize, Vec<u8>)> {
    // Heuristic: the first gzip magic in the stream. Family files hit
    // at offset 8; project files sometimes hit at offset 0. We always
    // prefer the observed offset over a hard-coded constant because
    // the hard-coded value panicked on the first project-file Global/
    // History we probed.
    let offset = find_gzip_offsets(data).into_iter().next().unwrap_or(8);
    let out = inflate_at_with_limits(data, offset, limits)?;
    Ok((offset, out))
}

/// Find every gzip magic byte-triple in `data`.
/// Used for streams like `Partitions/NN` that pack multiple GZIP segments.
pub fn find_gzip_offsets(data: &[u8]) -> Vec<usize> {
    let mut hits = Vec::new();
    let mut i = 0;
    while i + 3 <= data.len() {
        if data[i..i + 3] == GZIP_MAGIC {
            hits.push(i);
            i += 3;
        } else {
            i += 1;
        }
    }
    hits
}

/// Inflate every GZIP chunk in `data`, concatenating the outputs,
/// with bounded per-chunk + aggregate output sizes.
///
/// `per_chunk` caps each chunk's inflated size (same semantics as
/// [`inflate_at_with_limits`]). `aggregate` caps the total bytes
/// returned across all chunks — useful when the caller wants to
/// refuse processing of streams that would expand to multi-gigabyte
/// totals even if each individual chunk is small.
///
/// False positives (byte triples that match the gzip magic inside
/// random compressed data) continue to be silently skipped via
/// `filter_map` — their error is "looks like gzip but isn't." A
/// chunk that IS gzip but would exceed `per_chunk` is also skipped.
/// A fully-materialised result that would exceed `aggregate` causes
/// iteration to stop early at the last chunk whose inclusion keeps
/// us under budget.
pub fn inflate_all_chunks_with_limits(
    data: &[u8],
    per_chunk: InflateLimits,
    aggregate: usize,
) -> Vec<Vec<u8>> {
    inflate_all_members_with_limits(data, per_chunk, aggregate)
        .0
        .into_iter()
        .map(|(_, _, chunk)| chunk)
        .collect()
}

/// [`inflate_all_chunks_with_limits`], with each chunk's offset in `data`
/// and the bytes its gzip header and DEFLATE stream take there, and what the
/// limits left out.
// Tracker: B82.
fn inflate_all_members_with_limits(
    data: &[u8],
    per_chunk: InflateLimits,
    aggregate: usize,
) -> (Vec<(usize, usize, Vec<u8>)>, InflateShortfall) {
    let mut total: usize = 0;
    let mut results = Vec::new();
    let mut shortfall = InflateShortfall::default();
    for off in find_gzip_offsets(data) {
        let (chunk, consumed) = match inflate_member_at_with_limits(data, off, per_chunk) {
            Ok(c) => c,
            Err(Error::DecompressLimitExceeded(_)) => {
                shortfall.members_over_limit += 1;
                continue;
            }
            Err(_) => continue,
        };
        if total.saturating_add(chunk.len()) > aggregate {
            shortfall.stopped_at_total = true;
            break;
        }
        total += chunk.len();
        results.push((off, consumed, chunk));
    }
    (results, shortfall)
}

/// Inflate every GZIP chunk in `data`, concatenating the outputs.
/// Silently skips chunks that fail to inflate (some trailing "magic-like"
/// byte triples in random compressed data are false positives).
///
/// Backwards-compatible wrapper around [`inflate_all_chunks_with_limits`].
/// Uses default per-chunk limits (256 MiB) and a 1 GiB aggregate cap.
pub fn inflate_all_chunks(data: &[u8]) -> Vec<Vec<u8>> {
    inflate_all_chunks_with_limits(
        data,
        InflateLimits::default(),
        1024 * 1024 * 1024, // 1 GiB aggregate
    )
}

/// Encode `bytes` as Revit's "truncated-gzip" stream format: a minimal
/// 10-byte gzip header (magic `1F 8B 08`, no FNAME / FCOMMENT / FHCRC),
/// followed by raw DEFLATE output, and **without** the conforming
/// trailing CRC32+ISIZE that a true gzip writer would append.
///
/// This is the inverse of `inflate_at(_, 0)` and is what Revit writes
/// for streams like `Formats/Latest`.
pub fn truncated_gzip_encode(bytes: &[u8]) -> Result<Vec<u8>> {
    use flate2::{Compression, write::DeflateEncoder};
    let mut out = Vec::with_capacity(bytes.len());
    // 10-byte minimal gzip header: magic, deflate method, no flags, 0
    // mtime, XFL=0 (unknown), OS=255 (unknown).
    out.extend_from_slice(&[0x1f, 0x8b, 0x08, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xff]);
    let mut enc = DeflateEncoder::new(&mut out, Compression::default());
    enc.write_all(bytes)
        .map_err(|e| Error::Decompress(format!("deflate write: {e}")))?;
    enc.finish()
        .map_err(|e| Error::Decompress(format!("deflate finish: {e}")))?;
    Ok(out)
}

/// Encode `bytes` with Revit's 8-byte custom prefix (used by
/// `Global/*` streams) followed by truncated gzip. The custom prefix
/// appears as `[u32 LE 0][u32 LE 0]` in every file we've inspected;
/// its semantic meaning is not yet reverse-engineered, but it's
/// byte-for-byte invariant, so we replay it verbatim.
pub fn truncated_gzip_encode_with_prefix8(bytes: &[u8]) -> Result<Vec<u8>> {
    let body = truncated_gzip_encode(bytes)?;
    let mut out = Vec::with_capacity(8 + body.len());
    out.extend_from_slice(&[0u8; 8]);
    out.extend_from_slice(&body);
    Ok(out)
}

/// Encode `bytes` as one complete gzip member: [`truncated_gzip_encode`]'s
/// header and DEFLATE data, then the CRC32 and ISIZE trailer. Revit's own
/// `Formats/Latest` and `Global/Latest` carry that trailer, and a conforming
/// reader such as `crate::native_document::read_single` checks it, so this
/// is what the writer stores.
// Tracker: B69.
pub fn gzip_member_encode(bytes: &[u8]) -> Result<Vec<u8>> {
    let mut out = truncated_gzip_encode(bytes)?;
    let mut crc = flate2::Crc::new();
    crc.update(bytes);
    out.extend_from_slice(&crc.sum().to_le_bytes());
    out.extend_from_slice(&(bytes.len() as u32).to_le_bytes());
    Ok(out)
}

/// Validate that [`truncated_gzip_encode`] and
/// [`inflate_at`] are round-trip-exact inverses for the given
/// payload. Encodes `bytes`, inflates the result, compares
/// byte-for-byte.
///
/// Returns an [`Error::Decompress`] when any step fails or the
/// round-trip produces a different byte sequence. The error
/// message includes the first divergence offset when the
/// re-decoded output differs.
///
/// Useful as a self-check before calling
/// [`crate::writer::write_with_patches`] — if this fails, the
/// writer would have produced a file that the reader couldn't
/// re-open.
// Tracker: WRT-11.
pub fn validate_truncated_gzip_round_trip(bytes: &[u8]) -> Result<()> {
    let encoded = truncated_gzip_encode(bytes)?;
    let decoded = inflate_at(&encoded, 0)?;
    if decoded != bytes {
        let diff = decoded
            .iter()
            .zip(bytes.iter())
            .position(|(a, b)| a != b)
            .unwrap_or(decoded.len().min(bytes.len()));
        return Err(Error::Decompress(format!(
            "truncated_gzip round-trip mismatch at offset {diff}: encoded {} bytes, inflated to \
             {} bytes, expected {} bytes",
            encoded.len(),
            decoded.len(),
            bytes.len()
        )));
    }
    Ok(())
}

/// Validate that [`truncated_gzip_encode_with_prefix8`]
/// is a round-trip-exact inverse for the given payload. Encodes
/// with the 8-byte custom prefix, inflates at offset 8, compares
/// byte-for-byte.
// Tracker: WRT-11.
pub fn validate_truncated_gzip_prefix8_round_trip(bytes: &[u8]) -> Result<()> {
    let encoded = truncated_gzip_encode_with_prefix8(bytes)?;
    // First 8 bytes are the custom prefix; caller-visible invariant
    // is that they're zero.
    if encoded.len() < 8 || encoded[..8] != [0u8; 8] {
        return Err(Error::Decompress(
            "prefix8 encode: first 8 bytes not zero — framing invariant broken".into(),
        ));
    }
    let decoded = inflate_at(&encoded, 8)?;
    if decoded != bytes {
        let diff = decoded
            .iter()
            .zip(bytes.iter())
            .position(|(a, b)| a != b)
            .unwrap_or(decoded.len().min(bytes.len()));
        return Err(Error::Decompress(format!(
            "prefix8 round-trip mismatch at offset {diff}: encoded {} bytes (incl. 8-byte \
             prefix), inflated to {} bytes, expected {} bytes",
            encoded.len(),
            decoded.len(),
            bytes.len()
        )));
    }
    Ok(())
}
