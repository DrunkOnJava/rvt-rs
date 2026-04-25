//! Truncated-gzip decompression for Revit streams.
//!
//! Revit writes standard gzip file headers (magic `1F 8B 08` + 10-byte
//! minimum header) but omits the trailing 8-byte CRC32 + ISIZE that
//! RFC 1952 requires. `flate2::read::GzDecoder` validates those trailing
//! bytes and refuses truncated streams, so we skip the gzip header manually
//! and pump the raw DEFLATE body through `flate2::read::DeflateDecoder`.

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
/// The audit (AUDIT-2026-04-19.md P0 item 4) flagged this as the
/// single most urgent security fix before promoting the repo.
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
        let xlen_bytes = data.get(pos..xlen_end)?;
        let xlen = u16::from_le_bytes([xlen_bytes[0], xlen_bytes[1]]) as usize;
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
    let header_len =
        gzip_header_len(data, offset).ok_or_else(|| Error::Decompress("no gzip header".into()))?;
    let body_start = offset
        .checked_add(header_len)
        .ok_or_else(|| Error::Decompress("gzip header offset overflow".into()))?;
    let body = data
        .get(body_start..)
        .ok_or_else(|| Error::Decompress("gzip header extends past input".into()))?;
    // Clamp initial capacity. `body.len() * 4` was the historical hint
    // but without an upper bound it's a memory-amplification vector.
    let cap = body.len().saturating_mul(4).min(limits.max_output_bytes);
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
    Ok(out)
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
    // Heuristic: the first gzip magic in the stream. Family files hit
    // at offset 8; project files sometimes hit at offset 0. We always
    // prefer the observed offset over a hard-coded constant because
    // the hard-coded value panicked on the first project-file Global/
    // History we probed.
    let offset = find_gzip_offsets(data).into_iter().next().unwrap_or(8);
    let out = inflate_at_with_limits(data, offset, InflateLimits::default())?;
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
    let mut total: usize = 0;
    let mut results = Vec::new();
    for off in find_gzip_offsets(data) {
        let chunk = match inflate_at_with_limits(data, off, per_chunk) {
            Ok(c) => c,
            Err(_) => continue,
        };
        if total.saturating_add(chunk.len()) > aggregate {
            break;
        }
        total += chunk.len();
        results.push(chunk);
    }
    results
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

/// Validate (WRT-11) that [`truncated_gzip_encode`] and
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

/// Validate (WRT-11) that [`truncated_gzip_encode_with_prefix8`]
/// is a round-trip-exact inverse for the given payload. Encodes
/// with the 8-byte custom prefix, inflates at offset 8, compares
/// byte-for-byte.
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_magic() {
        assert!(has_gzip_magic(&[0x1F, 0x8B, 0x08, 0, 0, 0], 0));
        assert!(!has_gzip_magic(&[0x1F, 0x8B, 0x07], 0));
        assert!(has_gzip_magic(&[0xFF, 0x1F, 0x8B, 0x08], 1));
    }

    #[test]
    fn minimal_gzip_header_len() {
        // magic(3) + method(1) + flags=0(1) + mtime(4) + xfl(1) + os(1) = 10
        let hdr = [0x1F, 0x8B, 0x08, 0x00, 0, 0, 0, 0, 0, 0x0B];
        assert_eq!(gzip_header_len(&hdr, 0), Some(10));
    }

    #[test]
    fn gzip_header_with_fname() {
        // flags = 0x08 (FNAME), followed by "foo\0" after 10-byte base
        let hdr = [
            0x1F, 0x8B, 0x08, 0x08, 0, 0, 0, 0, 0, 0x0B, b'f', b'o', b'o', 0,
        ];
        assert_eq!(gzip_header_len(&hdr, 0), Some(14));
    }

    #[test]
    fn compressed_bomb_rejected_by_inflate_limits() {
        // Build a minimal compressed bomb: truncated-gzip header plus a
        // DEFLATE body that inflates 1 MB of zeros. Then set the inflate
        // cap below 1 MB and assert the call refuses rather than
        // allocating past the limit.
        use flate2::{Compression, write::DeflateEncoder};
        use std::io::Write;
        let payload = vec![0u8; 1024 * 1024]; // 1 MiB of zeros
        let mut bomb = Vec::new();
        bomb.extend_from_slice(&[0x1f, 0x8b, 0x08, 0x00, 0, 0, 0, 0, 0, 0xff]);
        let mut enc = DeflateEncoder::new(&mut bomb, Compression::default());
        enc.write_all(&payload).unwrap();
        enc.finish().unwrap();

        // Bomb ratio is ~1000:1 — a few KB expands to 1 MB. Cap at
        // 64 KB to force rejection; a legit Formats/Latest stream is
        // nowhere near this small-compared-to-header.
        let tight = InflateLimits {
            max_output_bytes: 64 * 1024,
        };
        let result = inflate_at_with_limits(&bomb, 0, tight);
        match result {
            Err(Error::DecompressLimitExceeded(msg)) => {
                assert!(msg.contains("65536"), "error should name the cap: {msg}");
            }
            other => panic!("expected DecompressLimitExceeded, got {other:?}"),
        }
    }

    #[test]
    fn legitimate_decompression_under_default_limit_still_works() {
        // Sanity: ordinary small payloads continue to inflate with the
        // default cap. Previous regressions have come from over-zealous
        // cap application.
        let payload = b"hello, world";
        let compressed = truncated_gzip_encode(payload).unwrap();
        let out = inflate_at_with_limits(&compressed, 0, InflateLimits::default()).unwrap();
        assert_eq!(&out[..], payload);
    }

    // ---- WRT-11: truncated-gzip encoder validation ----

    #[test]
    fn round_trip_empty_payload_is_ok() {
        assert!(validate_truncated_gzip_round_trip(b"").is_ok());
        assert!(validate_truncated_gzip_prefix8_round_trip(b"").is_ok());
    }

    #[test]
    fn round_trip_single_byte_is_ok() {
        for b in 0..=255_u8 {
            let payload = [b];
            validate_truncated_gzip_round_trip(&payload)
                .unwrap_or_else(|e| panic!("round-trip failed for 0x{b:02x}: {e}"));
        }
    }

    #[test]
    fn round_trip_typical_schema_payload_is_ok() {
        // Size + entropy pattern similar to a Formats/Latest chunk:
        // ~8 KB of mixed ASCII + binary noise.
        let mut payload = Vec::with_capacity(8 * 1024);
        for i in 0..8 * 1024 {
            payload.push(((i * 17 + 3) % 256) as u8);
        }
        validate_truncated_gzip_round_trip(&payload).unwrap();
    }

    #[test]
    fn round_trip_large_payload_crosses_32k_deflate_window() {
        // 128 KB — larger than the 32 KB DEFLATE sliding window so
        // the encoder hits multi-block encoding.
        let payload: Vec<u8> = (0..128 * 1024).map(|i| (i as u8).wrapping_mul(7)).collect();
        validate_truncated_gzip_round_trip(&payload).unwrap();
    }

    #[test]
    fn round_trip_highly_compressible_payload() {
        // All zeros — DEFLATE compresses this to near-zero bytes.
        // Previous regressions have come from zero-length DEFLATE
        // blocks; keep this case pinned.
        let payload = vec![0u8; 32 * 1024];
        validate_truncated_gzip_round_trip(&payload).unwrap();
    }

    #[test]
    fn round_trip_highly_incompressible_payload() {
        // Deterministic "random" payload — DEFLATE output is larger
        // than the input. Ensures the encoder handles pathological
        // expansion without overflowing buffers.
        let mut payload = Vec::with_capacity(1024);
        let mut state = 0x12345_u32;
        for _ in 0..1024 {
            state = state.wrapping_mul(1103515245).wrapping_add(12345);
            payload.push((state >> 16) as u8);
        }
        validate_truncated_gzip_round_trip(&payload).unwrap();
    }

    #[test]
    fn prefix8_encoder_zero_fills_first_eight_bytes() {
        let encoded = truncated_gzip_encode_with_prefix8(b"test").unwrap();
        assert!(encoded.len() > 8);
        assert_eq!(&encoded[..8], &[0u8; 8]);
        // And byte 8 is the gzip magic.
        assert_eq!(encoded[8], 0x1f);
        assert_eq!(encoded[9], 0x8b);
    }

    #[test]
    fn prefix8_round_trip_includes_framing_invariant_check() {
        for payload in [
            &b""[..],
            &b"a"[..],
            &b"Global/Latest typical chunk"[..],
            &vec![0xFFu8; 4096][..],
        ] {
            validate_truncated_gzip_prefix8_round_trip(payload).unwrap();
        }
    }

    #[test]
    fn encoded_header_is_canonical_ten_bytes() {
        // The encoder always emits the minimal 10-byte header:
        //   1F 8B 08 00 00 00 00 00 00 FF
        // so every round-trip starts from the same place.
        let encoded = truncated_gzip_encode(b"x").unwrap();
        assert!(encoded.len() > 10);
        assert_eq!(
            &encoded[..10],
            &[0x1f, 0x8b, 0x08, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xff]
        );
    }
}
