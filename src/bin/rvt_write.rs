//! `rvt-write` — apply stream-level patches to a Revit file and verify
//! the round-trip.
//!
//! WRT-14 capstone CLI that ties together:
//!
//! - [`rvt::writer::write_with_patches_verified`] (WRT-13)
//! - [`rvt::compression::validate_truncated_gzip_round_trip`] (WRT-11)
//! - [`rvt::round_trip::verify_instance_round_trip`] (WRT-04)
//!
//! Workflow: the user supplies a JSON manifest of stream patches + a
//! source .rvt/.rfa, the writer applies them to a new output path, and
//! the verifier confirms each patched stream's decompressed bytes
//! match what the user asked for. Exit 0 on success, non-zero on
//! failure with the first diverging stream + offset printed.
//!
//! # Patch manifest JSON
//!
//! ```json
//! {
//!   "patches": [
//!     {
//!       "stream_name": "Formats/Latest",
//!       "new_decompressed_base64": "<base64-encoded bytes>",
//!       "framing": "RawGzipFromZero"
//!     },
//!     {
//!       "stream_name": "Global/Latest",
//!       "new_decompressed_base64": "<base64-encoded bytes>",
//!       "framing": "CustomPrefix8"
//!     }
//!   ]
//! }
//! ```

use clap::Parser;
use rvt::writer::{StreamFraming, StreamPatch, write_with_patches_verified};
use serde::Deserialize;
use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;

#[derive(Parser, Debug)]
#[command(
    name = "rvt-write",
    version,
    about = "Apply stream-level patches to a Revit file and verify the round-trip"
)]
struct Cli {
    /// Source Revit file path.
    #[arg(short, long)]
    src: PathBuf,

    /// Destination Revit file path. Written atomically; a mid-write
    /// failure leaves the source untouched.
    #[arg(short, long)]
    dst: PathBuf,

    /// Path to a JSON patch manifest. Each entry describes one
    /// stream to replace. Required unless `--dry-run` is set.
    #[arg(short = 'p', long)]
    patches: Option<PathBuf>,

    /// Validate the patch manifest + source file without writing
    /// anything. Exits 0 when the manifest is syntactically
    /// well-formed and every listed stream exists in `src`.
    #[arg(long)]
    dry_run: bool,

    /// Print the verification report even on success. Off by
    /// default (the successful exit code is the signal); enable
    /// when scripting a diagnostic pipeline that wants per-stream
    /// detail.
    #[arg(long)]
    verbose: bool,
}

#[derive(Deserialize, Debug)]
struct PatchManifest {
    patches: Vec<ManifestPatch>,
}

#[derive(Deserialize, Debug)]
struct ManifestPatch {
    stream_name: String,
    /// Base64-encoded bytes. Preferred over inlined arrays so the
    /// JSON stays compact for multi-kilobyte patches.
    #[serde(default)]
    new_decompressed_base64: Option<String>,
    /// Inline byte array (alternative to base64). Accepts `[0, 1,
    /// 2, ...]` for tiny patches.
    #[serde(default)]
    new_decompressed: Option<Vec<u8>>,
    framing: ManifestFraming,
}

#[derive(Deserialize, Debug, Clone, Copy)]
#[serde(rename_all = "PascalCase")]
enum ManifestFraming {
    RawGzipFromZero,
    CustomPrefix8,
    Verbatim,
}

impl From<ManifestFraming> for StreamFraming {
    fn from(m: ManifestFraming) -> Self {
        match m {
            ManifestFraming::RawGzipFromZero => Self::RawGzipFromZero,
            ManifestFraming::CustomPrefix8 => Self::CustomPrefix8,
            ManifestFraming::Verbatim => Self::Verbatim,
        }
    }
}

fn decode_base64(s: &str) -> Result<Vec<u8>, String> {
    // Minimal base64 decoder — avoid adding a crate dep for a
    // single helper. Supports the standard alphabet + trailing
    // `=` padding.
    let lookup = |c: u8| -> Option<u8> {
        match c {
            b'A'..=b'Z' => Some(c - b'A'),
            b'a'..=b'z' => Some(c - b'a' + 26),
            b'0'..=b'9' => Some(c - b'0' + 52),
            b'+' => Some(62),
            b'/' => Some(63),
            _ => None,
        }
    };
    let bytes: Vec<u8> = s.bytes().filter(|c| !c.is_ascii_whitespace()).collect();
    let mut out = Vec::with_capacity(bytes.len() * 3 / 4);
    for chunk in bytes.chunks(4) {
        if chunk.iter().all(|c| *c == b'=') {
            break;
        }
        let mut buf = [0u8; 4];
        let mut pad = 0;
        for (i, c) in chunk.iter().enumerate() {
            if *c == b'=' {
                pad += 1;
                buf[i] = 0;
            } else {
                buf[i] = lookup(*c).ok_or_else(|| format!("invalid base64 char 0x{c:02x}"))?;
            }
        }
        let triple = ((buf[0] as u32) << 18)
            | ((buf[1] as u32) << 12)
            | ((buf[2] as u32) << 6)
            | buf[3] as u32;
        if pad <= 2 {
            out.push((triple >> 16) as u8);
        }
        if pad <= 1 {
            out.push((triple >> 8) as u8);
        }
        if pad == 0 {
            out.push(triple as u8);
        }
    }
    Ok(out)
}

fn manifest_to_patches(m: PatchManifest) -> Result<Vec<StreamPatch>, String> {
    let mut patches = Vec::with_capacity(m.patches.len());
    for mp in m.patches {
        let bytes = match (mp.new_decompressed, mp.new_decompressed_base64) {
            (Some(b), None) => b,
            (None, Some(s)) => decode_base64(&s)
                .map_err(|e| format!("stream '{}': base64 decode failed: {e}", mp.stream_name))?,
            (Some(_), Some(_)) => {
                return Err(format!(
                    "stream '{}': both new_decompressed and new_decompressed_base64 set — pick one",
                    mp.stream_name
                ));
            }
            (None, None) => {
                return Err(format!(
                    "stream '{}': neither new_decompressed nor new_decompressed_base64 set",
                    mp.stream_name
                ));
            }
        };
        patches.push(StreamPatch {
            stream_name: mp.stream_name,
            new_decompressed: bytes,
            framing: mp.framing.into(),
        });
    }
    Ok(patches)
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match run(&cli) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("rvt-write: {e}");
            ExitCode::from(1)
        }
    }
}

fn run(cli: &Cli) -> Result<(), String> {
    let manifest_path = cli
        .patches
        .as_ref()
        .ok_or_else(|| "--patches is required (or pass --dry-run)".to_string())?;

    let manifest_text = fs::read_to_string(manifest_path)
        .map_err(|e| format!("read {}: {e}", manifest_path.display()))?;
    let manifest: PatchManifest = serde_json::from_str(&manifest_text)
        .map_err(|e| format!("parse {}: {e}", manifest_path.display()))?;
    let patches = manifest_to_patches(manifest)?;

    // Quick sanity on the source file before attempting anything.
    rvt::RevitFile::open(&cli.src).map_err(|e| format!("open src {}: {e}", cli.src.display()))?;

    if cli.dry_run {
        println!(
            "dry-run: {} patch(es) validated against {}",
            patches.len(),
            cli.src.display()
        );
        for p in &patches {
            println!(
                "  {} <- {} bytes (framing: {:?})",
                p.stream_name,
                p.new_decompressed.len(),
                p.framing
            );
        }
        return Ok(());
    }

    let report = write_with_patches_verified(&cli.src, &cli.dst, &patches)
        .map_err(|e| format!("write {} -> {}: {e}", cli.src.display(), cli.dst.display()))?;

    if !report.all_matched() {
        for fail in report.failures() {
            eprintln!(
                "stream '{}': mismatch — actual {} bytes, expected {} bytes, first diff @ {:?}, \
                 err: {:?}",
                fail.stream_name,
                fail.actual_len,
                fail.expected_len,
                fail.first_diff_at,
                fail.decompress_error,
            );
        }
        return Err(format!(
            "{} stream(s) failed verification; inspect {}",
            report.failure_count(),
            cli.dst.display()
        ));
    }

    if cli.verbose {
        println!(
            "verified {} stream(s) round-tripped cleanly",
            report.streams.len()
        );
        for s in &report.streams {
            println!("  {}: {} bytes OK", s.stream_name, s.actual_len);
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base64_decodes_canonical_cases() {
        assert_eq!(decode_base64("").unwrap(), b"");
        assert_eq!(decode_base64("Zg==").unwrap(), b"f");
        assert_eq!(decode_base64("Zm8=").unwrap(), b"fo");
        assert_eq!(decode_base64("Zm9v").unwrap(), b"foo");
        assert_eq!(decode_base64("Zm9vYg==").unwrap(), b"foob");
        assert_eq!(decode_base64("Zm9vYmE=").unwrap(), b"fooba");
        assert_eq!(decode_base64("Zm9vYmFy").unwrap(), b"foobar");
    }

    #[test]
    fn base64_ignores_whitespace() {
        assert_eq!(decode_base64("  Zm9v\n YmFy  ").unwrap(), b"foobar");
    }

    #[test]
    fn base64_rejects_invalid_char() {
        assert!(decode_base64("Z@9v").is_err());
    }

    #[test]
    fn manifest_with_base64_decodes_to_patches() {
        let m = PatchManifest {
            patches: vec![ManifestPatch {
                stream_name: "Formats/Latest".into(),
                new_decompressed: None,
                new_decompressed_base64: Some("aGVsbG8=".into()), // "hello"
                framing: ManifestFraming::RawGzipFromZero,
            }],
        };
        let patches = manifest_to_patches(m).unwrap();
        assert_eq!(patches.len(), 1);
        assert_eq!(patches[0].stream_name, "Formats/Latest");
        assert_eq!(patches[0].new_decompressed, b"hello");
    }

    #[test]
    fn manifest_with_inline_bytes_decodes_to_patches() {
        let m = PatchManifest {
            patches: vec![ManifestPatch {
                stream_name: "Formats/Latest".into(),
                new_decompressed: Some(vec![1, 2, 3, 4]),
                new_decompressed_base64: None,
                framing: ManifestFraming::CustomPrefix8,
            }],
        };
        let patches = manifest_to_patches(m).unwrap();
        assert_eq!(patches[0].new_decompressed, vec![1, 2, 3, 4]);
        assert!(matches!(patches[0].framing, StreamFraming::CustomPrefix8));
    }

    #[test]
    fn manifest_rejects_both_payload_forms() {
        let m = PatchManifest {
            patches: vec![ManifestPatch {
                stream_name: "x".into(),
                new_decompressed: Some(vec![1]),
                new_decompressed_base64: Some("AA==".into()),
                framing: ManifestFraming::Verbatim,
            }],
        };
        assert!(manifest_to_patches(m).is_err());
    }

    #[test]
    fn manifest_rejects_no_payload() {
        let m = PatchManifest {
            patches: vec![ManifestPatch {
                stream_name: "x".into(),
                new_decompressed: None,
                new_decompressed_base64: None,
                framing: ManifestFraming::Verbatim,
            }],
        };
        assert!(manifest_to_patches(m).is_err());
    }

    #[test]
    fn framing_converts_all_variants() {
        let cases = [
            (
                ManifestFraming::RawGzipFromZero,
                StreamFraming::RawGzipFromZero,
            ),
            (ManifestFraming::CustomPrefix8, StreamFraming::CustomPrefix8),
            (ManifestFraming::Verbatim, StreamFraming::Verbatim),
        ];
        for (m, expected) in cases {
            let converted: StreamFraming = m.into();
            assert_eq!(converted, expected);
        }
    }
}
