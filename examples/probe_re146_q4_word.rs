//! RE-146 (probe): what the two tag words `formats::parse_schema` keeps on
//! each class are (#154).
//!
//! puzzbobb reported (#154) that, read against `formats::schema_classes`
//! (every class, its definition-ordinal tag and its base, RE-133):
//!
//! 1. `ClassEntry::tag` is the tag of the class's **base**, never the
//!    class's own;
//! 2. `ClassEntry::ancestor_tag` (the word the Q4 addendum read as an
//!    ancestor) is the tag of the **base's base**.
//!
//! This counts both on every file given: classes `parse_schema` tags,
//! those whose tag is the base's and those whose tag is their own, classes
//! with an `ancestor_tag`, and those whose `ancestor_tag` is the
//! grandparent's, with up to five counterexamples of each.
//!
//! Usage:
//!   cargo run --profile ci --example probe_re146_q4_word -- FILE.rvt ...

use rvt::RevitFile;
use std::collections::HashMap;

const EXAMPLES: usize = 5;

fn probe(path: &str) -> rvt::Result<String> {
    let file = std::path::Path::new(path)
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let mut rf = RevitFile::open(path)?;
    let revit = rf
        .basic_file_info()
        .map_or_else(|_| "null".to_string(), |info| info.version.to_string());
    let classes = rf.schema_classes()?;
    let by_tag: HashMap<u16, (&str, Option<u16>)> = classes
        .classes
        .iter()
        .map(|c| (c.tag, (c.name.as_str(), c.base)))
        .collect();
    let tag_of: HashMap<&str, u16> = classes
        .classes
        .iter()
        .map(|c| (c.name.as_str(), c.tag))
        .collect();
    let old = rf.schema()?;
    let (mut tagged, mut tag_is_base, mut tag_is_own) = (0usize, 0usize, 0usize);
    let (mut with_ancestor, mut ancestor_is_grandparent) = (0usize, 0usize);
    let (mut not_base, mut not_grandparent) = (Vec::new(), Vec::new());
    for entry in &old.classes {
        let (Some(tag), Some(&own)) = (entry.tag, tag_of.get(entry.name.as_str())) else {
            continue;
        };
        let base = by_tag.get(&own).and_then(|(_, base)| *base);
        let grandparent = base.and_then(|b| by_tag.get(&b)).and_then(|(_, gp)| *gp);
        tagged += 1;
        tag_is_own += usize::from(tag == own);
        if Some(tag) == base {
            tag_is_base += 1;
        } else if not_base.len() < EXAMPLES {
            not_base.push(format!(
                "{{\"class\":{:?},\"own\":{own},\"base\":{base:?},\"tag\":{tag}}}",
                entry.name
            ));
        }
        if let Some(ancestor) = entry.ancestor_tag {
            with_ancestor += 1;
            if Some(ancestor) == grandparent {
                ancestor_is_grandparent += 1;
            } else if not_grandparent.len() < EXAMPLES {
                not_grandparent.push(format!(
                    "{{\"class\":{:?},\"base\":{base:?},\"grandparent\":{grandparent:?},\"ancestor_tag\":{ancestor}}}",
                    entry.name
                ));
            }
        }
    }
    Ok(format!(
        "{{\"file\":{file:?},\"revit\":{revit},\"schema_classes\":{},\"schema_classes_stopped\":{},\
         \"parse_schema_tagged\":{tagged},\"tag_is_base\":{tag_is_base},\"tag_is_own\":{tag_is_own},\
         \"with_ancestor_tag\":{with_ancestor},\"ancestor_is_grandparent\":{ancestor_is_grandparent},\
         \"tag_not_base\":[{}],\"ancestor_not_grandparent\":[{}]}}",
        classes.classes.len(),
        classes.stopped.is_some(),
        not_base.join(","),
        not_grandparent.join(","),
    ))
}

fn main() {
    let paths: Vec<String> = std::env::args().skip(1).collect();
    if paths.is_empty() {
        eprintln!("usage: probe_re146_q4_word FILE.rvt ...");
        std::process::exit(2);
    }
    for path in &paths {
        match probe(path) {
            Ok(line) => println!("{line}"),
            Err(error) => println!("{{\"file\":{path:?},\"error\":{:?}}}", error.to_string()),
        }
    }
}
