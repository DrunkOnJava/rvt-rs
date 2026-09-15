//! Bounded structural coverage without retaining large per-owner JSON graphs.
use anyhow::{Result, ensure};
use rvt::{RevitFile, native_document};
use sha2::{Digest, Sha256};
use std::{
    fs::{File, OpenOptions},
    io::{Read, Write},
    path::Path,
};
fn main() -> Result<()> {
    let args: Vec<_> = std::env::args().collect();
    ensure!(args.len() == 3, "FILE NEW_SUMMARY");
    let mut out = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&args[2])?;
    let mut f = File::open(&args[1])?;
    let mut hash = Sha256::new();
    let mut bytes = [0u8; 65536];
    loop {
        let n = f.read(&mut bytes)?;
        if n == 0 {
            break;
        }
        hash.update(&bytes[..n]);
    }
    let mut file = RevitFile::open(Path::new(&args[1]))?;
    let result = native_document::extract(&mut file, &Default::default(), |_| Ok(()));
    let report = serde_json::json!({"source":args[1],"source_sha256":format!("{:x}",hash.finalize()),"summary":result.as_ref().ok(),"error":result.as_ref().err().map(|e|format!("{e:#}")),"scope":"current bounded graph coverage; not evaluated semantic parity"});
    serde_json::to_writer_pretty(&mut out, &report)?;
    out.write_all(b"\n")?;
    if let Err(e) = result {
        eprintln!("{e:#}");
        std::process::exit(2);
    }
    Ok(())
}
