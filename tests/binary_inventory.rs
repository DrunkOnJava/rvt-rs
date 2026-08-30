//! The README's "N CLIs ship (...)" sentence must match the `[[bin]]` targets
//! declared in Cargo.toml and the files under src/bin/. Issue #186 found the
//! three disagreeing (comment said seven, README seventeen, reality eighteen).

use std::path::Path;

fn root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

fn declared_bins() -> Vec<String> {
    let manifest = std::fs::read_to_string(root().join("Cargo.toml")).expect("read Cargo.toml");
    let mut bins = Vec::new();
    let mut in_bin = false;
    for line in manifest.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            in_bin = line == "[[bin]]";
            continue;
        }
        if !in_bin {
            continue;
        }
        if let Some(rest) = line.strip_prefix("name") {
            let name = rest
                .trim_start()
                .trim_start_matches('=')
                .trim()
                .trim_matches('"');
            bins.push(name.to_string());
            in_bin = false;
        }
    }
    bins
}

fn number_word(n: usize) -> &'static str {
    match n {
        10 => "Ten",
        11 => "Eleven",
        12 => "Twelve",
        13 => "Thirteen",
        14 => "Fourteen",
        15 => "Fifteen",
        16 => "Sixteen",
        17 => "Seventeen",
        18 => "Eighteen",
        19 => "Nineteen",
        20 => "Twenty",
        21 => "Twenty-one",
        22 => "Twenty-two",
        23 => "Twenty-three",
        24 => "Twenty-four",
        25 => "Twenty-five",
        other => panic!("extend number_word() for {other} binaries"),
    }
}

#[test]
fn cargo_bins_match_src_bin_files() {
    let bins = declared_bins();
    let mut files: Vec<String> = std::fs::read_dir(root().join("src/bin"))
        .expect("list src/bin")
        .map(|e| {
            e.expect("dir entry")
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .filter(|f| f.ends_with(".rs"))
        .map(|f| f.trim_end_matches(".rs").replace('_', "-"))
        .collect();
    files.sort();
    let mut declared = bins.clone();
    declared.sort();
    assert_eq!(
        declared, files,
        "[[bin]] targets in Cargo.toml and files in src/bin/ disagree"
    );
}

#[test]
fn readme_cli_sentence_matches_cargo_bins() {
    let bins = declared_bins();
    let readme = std::fs::read_to_string(root().join("README.md")).expect("read README.md");
    let expected = format!("**{} CLIs ship**", number_word(bins.len()));
    assert!(
        readme.contains(&expected),
        "README.md must say `{expected}` — Cargo.toml declares {} [[bin]] targets",
        bins.len()
    );
    let sentence_start = readme.find(&expected).unwrap();
    let sentence =
        &readme[sentence_start..readme[sentence_start..].find(')').unwrap() + sentence_start];
    for bin in &bins {
        assert!(
            sentence.contains(&format!("`{bin}`")),
            "README CLI list is missing `{bin}` (declared in Cargo.toml)"
        );
    }
}
