//! Compare native revision snapshots without an external parser.
use anyhow::{Context, Result, ensure};
use clap::Parser;
use rvt::native_revision::{Snapshot, compare};
use serde_json::Value;
use std::{
    fs::OpenOptions,
    io::{BufReader, BufWriter, Write},
    path::PathBuf,
};
#[derive(Parser)]
#[command(
    about = "Compare saved native identities, record bodies and projected parameter values",
    after_help = "Inputs may be revision snapshots or rvt-native-world-model reports containing revision_snapshot. Same saved UID does not prove document lineage or real-world entity identity. Missing input coverage is explicit. Output must not already exist."
)]
struct Args {
    before: PathBuf,
    after: PathBuf,
    #[arg(long)]
    json: PathBuf,
}
fn snapshot(value: Value) -> Result<Snapshot> {
    let value =
        if value.get("format").and_then(Value::as_str) == Some("rvt-native-revision-snapshot/v1") {
            value
        } else {
            value
                .get("revision_snapshot")
                .cloned()
                .ok_or_else(|| anyhow::anyhow!("input has no native revision snapshot"))?
        };
    ensure!(!value.is_null(), "input revision snapshot unavailable");
    Ok(serde_json::from_value(value)?)
}
fn run() -> Result<()> {
    let args = Args::parse();
    let before = snapshot(serde_json::from_reader(BufReader::new(
        std::fs::File::open(&args.before)?,
    ))?)
    .context("read before snapshot")?;
    let after = snapshot(serde_json::from_reader(BufReader::new(
        std::fs::File::open(&args.after)?,
    ))?)
    .context("read after snapshot")?;
    let delta = compare(&before, &after)?;
    let mut output = BufWriter::new(
        OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&args.json)?,
    );
    serde_json::to_writer_pretty(&mut output, &delta)?;
    writeln!(output)?;
    output.flush()?;
    Ok(())
}
fn main() {
    if let Err(error) = run() {
        eprintln!("{error:#}");
        std::process::exit(1);
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn unavailable_projection_is_not_an_empty_snapshot() {
        assert!(snapshot(serde_json::json!({"revision_snapshot":null})).is_err());
        assert!(snapshot(serde_json::json!({"format":"other"})).is_err());
    }
}
