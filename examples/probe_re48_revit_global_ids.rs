//! RE-48: the GlobalId Revit's exporter gives an element is in the file.
//!
//! FACT: `Global/History` lists every editing episode's GUID, newest first,
//! and a 40-byte `Global/ElemTable` record holds its element's episode
//! number at `+0x18`. The element's GlobalId is that episode's GUID with the
//! record's first ElementId XORed into its last 32 bits, in IFC's base-64
//! form. Every element rvt-rs exports that Revit's own export also holds
//! gets the same GlobalId there: Core Interior 854 of 854, the RE1 models
//! 281 of 281, Snowdon Towers 5,945 of 5,945.
//!
//! The probe prints the episode count and every element's rebuilt GlobalId.
//! Given Revit's own IFC export as a second argument, it compares them with
//! the GlobalId of the same `Tag` there.
//!
//! Usage:
//!   cargo run --profile ci --example probe_re48_revit_global_ids -- FILE.rvt [REVIT.ifc]

use rvt::RevitFile;
use rvt::revit_global_ids::revit_global_ids;
use std::collections::BTreeMap;
use std::path::PathBuf;

/// `Tag -> GlobalId` of every entity in a STEP file whose eighth attribute
/// is a numeric `Tag` (types and openings excluded).
fn reference_global_ids(step: &str) -> BTreeMap<u32, String> {
    let mut out = BTreeMap::new();
    for line in step.lines() {
        let Some((_, body)) = line.split_once('=') else {
            continue;
        };
        let Some((entity, args)) = body.split_once('(') else {
            continue;
        };
        let entity = entity.trim();
        if entity.ends_with("TYPE") || entity == "IFCOPENINGELEMENT" {
            continue;
        }
        let mut fields = Vec::new();
        let mut current = String::new();
        let (mut depth, mut quoted) = (0i32, false);
        for ch in args.chars() {
            match ch {
                '\'' => quoted = !quoted,
                '(' if !quoted => depth += 1,
                ')' if !quoted => depth -= 1,
                ',' if !quoted && depth == 0 => {
                    fields.push(std::mem::take(&mut current));
                    continue;
                }
                _ => {}
            }
            current.push(ch);
        }
        fields.push(current);
        let (Some(global_id), Some(tag)) = (fields.first(), fields.get(7)) else {
            continue;
        };
        if let Ok(tag) = tag.trim_matches('\'').parse() {
            out.entry(tag)
                .or_insert_with(|| global_id.trim_matches('\'').to_string());
        }
    }
    out
}

fn main() -> rvt::Result<()> {
    let mut args = std::env::args().skip(1);
    let path = PathBuf::from(args.next().expect("usage: FILE.rvt [REVIT.ifc]"));
    let reference = args.next().map(PathBuf::from);
    let mut rf = RevitFile::open(&path)?;
    let ids = revit_global_ids(&mut rf)?;
    println!("{} ElementIds with a rebuilt GlobalId", ids.len());
    let Some(reference) = reference else {
        for (id, global_id) in ids.iter().take(20) {
            println!("  {id:>9}  {global_id}");
        }
        return Ok(());
    };
    let theirs = reference_global_ids(&std::fs::read_to_string(reference)?);
    let (mut same, mut different, mut missing) = (0usize, Vec::new(), 0usize);
    for (tag, revit) in &theirs {
        match ids.get(tag) {
            Some(ours) if ours == revit => same += 1,
            Some(ours) => different.push((*tag, ours.clone(), revit.clone())),
            None => missing += 1,
        }
    }
    println!(
        "Revit's export: {} Tags; {same} with the same GlobalId, {} different, {missing} not declared",
        theirs.len(),
        different.len()
    );
    for (tag, ours, revit) in different.iter().take(10) {
        println!("  {tag:>9}  ours {ours}  Revit {revit}");
    }
    Ok(())
}
