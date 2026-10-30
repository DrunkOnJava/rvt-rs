//! W6: does a wall's native record give what its partition data gives it?
//!
//! FACT under test: where the partition data gives a wall its line and its
//! orientation (RE-53, RE-54), the wall's native `SWall` record gives the
//! same: its `GLine` is that line, `m_isFlipped` is the orientation's flip
//! and `m_wallCrossSection` its word. And the walls the partition data gives
//! no line (on the BIM4LCA office, the US-1 exterior walls) have a native
//! curve.
//!
//! Output:
//!   walls <n> partition line <n> orientation <n> native curve <n>
//!   flip agree <n> disagree <n>; word==crossSection <n> differ <n>
//!   line vs native curve: max end distance <ft>, walls over 0.001 ft <n>
//!   no line <id> orientation <yes|no> native <yes|no> flip <bool> section <n>
//!
//! Usage:
//!   cargo run --profile ci -p rvt-probes --example probe_wall_native_orientation -- MODEL.rvt

use std::collections::{BTreeMap, BTreeSet};

use rvt::RevitFile;
use rvt::research::native_document::{Options, extract_graphs};
use rvt::research::partition_compound_structure as pcs;
use rvt::walker::iter_elements;
use serde_json::Value;

fn triple(value: Option<&Value>) -> Option<[f64; 3]> {
    let items = value?.as_array()?;
    Some([
        items.first()?.as_f64()?,
        items.get(1)?.as_f64()?,
        items.get(2)?.as_f64()?,
    ])
}

fn main() -> anyhow::Result<()> {
    let path = std::env::args().nth(1).expect("usage: MODEL.rvt");
    let mut rf = RevitFile::open(&path)?;
    let version = rf.basic_file_info()?.version;
    let walls: BTreeSet<u32> = iter_elements(&mut rf)?
        .filter(|e| e.class == "Wall")
        .filter_map(|e| e.id)
        .collect();
    let lines = pcs::scan_wall_lines(&mut rf, version, &walls)?;
    let orientations = pcs::scan_wall_orientations(&mut rf, version, &walls)?;
    // id -> (start, end, flipped, cross section)
    type Native = ([f64; 2], [f64; 2], Option<bool>, Option<i64>);
    let mut native: BTreeMap<u32, Native> = BTreeMap::new();
    let options = Options {
        selected_ids: walls.iter().map(|&id| u64::from(id)).collect(),
        ..Options::default()
    };
    extract_graphs(&mut rf, &options, |record| {
        let (Some("SWall"), Some(graph)) = (record.class_name.as_deref(), record.graph.as_ref())
        else {
            return Ok(());
        };
        let Ok(id) = u32::try_from(record.identity.element_id) else {
            return Ok(());
        };
        let Some(root) = graph.objects.first() else {
            return Ok(());
        };
        let curves: Vec<_> = graph
            .objects
            .iter()
            .filter(|o| matches!(o.class_name.as_str(), "GLine" | "GArc"))
            .collect();
        let [curve] = curves.as_slice() else {
            return Ok(());
        };
        if curve.class_name != "GLine" {
            return Ok(());
        }
        let (Some(o), Some(d), Some(params)) = (
            triple(curve.fields.get("m_origin")),
            triple(curve.fields.get("m_dirVec")),
            curve.fields.get("m_endParams").and_then(Value::as_array),
        ) else {
            return Ok(());
        };
        let (Some(a), Some(b)) = (
            params.first().and_then(Value::as_f64),
            params.get(1).and_then(Value::as_f64),
        ) else {
            return Ok(());
        };
        native.insert(
            id,
            (
                [o[0] + d[0] * a, o[1] + d[1] * a],
                [o[0] + d[0] * b, o[1] + d[1] * b],
                root.fields.get("m_isFlipped").and_then(Value::as_bool),
                root.fields
                    .get("m_wallCrossSection")
                    .and_then(Value::as_i64),
            ),
        );
        Ok(())
    })?;
    println!(
        "walls {} partition line {} orientation {} native curve {}",
        walls.len(),
        lines.len(),
        orientations.len(),
        native.len()
    );
    let (mut flip_same, mut flip_diff, mut word_same, mut word_diff) = (0, 0, 0, 0);
    let mut words: BTreeMap<(u32, Option<i64>), usize> = BTreeMap::new();
    for (id, orientation) in &orientations {
        let Some((_, _, flipped, section)) = native.get(id) else {
            continue;
        };
        if *flipped == Some(orientation.flip) {
            flip_same += 1;
        } else {
            flip_diff += 1;
        }
        if *section == Some(i64::from(orientation.word)) {
            word_same += 1;
        } else {
            word_diff += 1;
        }
        *words.entry((orientation.word, *section)).or_default() += 1;
    }
    println!(
        "flip agree {flip_same} disagree {flip_diff}; word==crossSection {word_same} differ {word_diff}"
    );
    for ((word, section), n) in &words {
        println!("  word {word} crossSection {section:?}: {n}");
    }
    let (mut worst, mut over) = (0.0f64, 0);
    for (id, line) in &lines {
        let Some((start, end, _, _)) = native.get(id) else {
            continue;
        };
        let (s, e) = (line.start(), line.end());
        let far = (s[0] - start[0])
            .hypot(s[1] - start[1])
            .max((e[0] - end[0]).hypot(e[1] - end[1]));
        worst = worst.max(far);
        if far > 0.001 {
            over += 1;
            println!("  line {id} differs from its native curve by {far:.4} ft");
        }
    }
    println!("line vs native curve: max end distance {worst:.6} ft, walls over 0.001 ft {over}");
    for id in walls.iter().filter(|id| !lines.contains_key(id)) {
        let n = native.get(id);
        println!(
            "no line {id} orientation {} native {} flip {:?} section {:?}",
            if orientations.contains_key(id) {
                "yes"
            } else {
                "no"
            },
            if n.is_some() { "yes" } else { "no" },
            n.and_then(|n| n.2),
            n.and_then(|n| n.3),
        );
    }
    Ok(())
}
