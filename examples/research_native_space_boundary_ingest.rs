//! Bounded end-to-end research replay through native extraction and the
//! production spatial InventoryBuilder. Emits a saved 2D carrier observation.
use anyhow::{Context, Result, ensure};
use rvt::{
    RevitFile,
    native_document::{self, Options},
    native_spatial_boundaries::InventoryBuilder,
};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::{collections::BTreeSet, env, fs};

fn main() -> Result<()> {
    let a: Vec<_> = env::args().collect();
    ensure!(
        a.len() == 9,
        "usage: research_native_space_boundary_ingest <model.rvt> <space-id> <topology-id> <circuit-id> <level-id> <carrier-ids-csv> <expected-area-ft2> <output.json>"
    );
    let mut file = RevitFile::open(&a[1])?;
    let space_id: u64 = a[2].parse().context("space-id must be an integer")?;
    let topology_id: u64 = a[3].parse().context("topology-id must be an integer")?;
    let circuit_id: i64 = a[4].parse().context("circuit-id must be an integer")?;
    let level_id: u64 = a[5].parse().context("level-id must be an integer")?;
    let carrier_ids: Vec<u64> = a[6]
        .split(',')
        .map(|id| {
            id.parse()
                .context("carrier IDs must be comma-separated integers")
        })
        .collect::<Result<_>>()?;
    ensure!(
        !carrier_ids.is_empty(),
        "at least one carrier ID is required"
    );
    let expected_area: f64 = a[7].parse().context("expected-area-ft2 must be a number")?;
    ensure!(
        expected_area.is_finite() && expected_area > 0.,
        "expected area must be positive"
    );
    let ids: BTreeSet<u64> = std::iter::once(space_id)
        .chain(std::iter::once(topology_id))
        .chain(std::iter::once(level_id))
        .chain(carrier_ids.iter().copied())
        .chain(u64::try_from(circuit_id).ok())
        .collect();
    let options = Options {
        selected_ids: ids.clone(),
        channels: BTreeSet::from([102, 103]),
        max_stream_bytes: 512 * 1024 * 1024,
        max_group_bytes: 256 * 1024 * 1024,
        max_graph_values: 1_000_000,
        max_graph_objects: 1_000_000,
    };
    let mut builder = InventoryBuilder::default();
    let mut emitted = 0usize;
    let coverage = native_document::extract(&mut file, &options, |record| {
        builder.ingest(&record)?;
        emitted += 1;
        Ok(())
    })?;
    let inventory = builder.finish()?;
    let room = inventory
        .rooms
        .iter()
        .find(|r| r.owner.element_id == space_id)
        .with_context(|| format!("space {space_id} missing"))?;
    ensure!(
        room.carrier_loops.len() == 1 && room.carrier_loops[0].len() == 6,
        "expected one six-segment carrier loop"
    );
    let points: Vec<_> = room.carrier_loops[0].iter().map(|s| s.points[0]).collect();
    let area = points
        .iter()
        .enumerate()
        .map(|(i, a)| {
            let b = points[(i + 1) % points.len()];
            a[0] * b[1] - b[0] * a[1]
        })
        .sum::<f64>()
        .abs()
        / 2.;
    ensure!(
        (area - expected_area).abs() < 1e-9,
        "unexpected carrier area {area}"
    );
    ensure!(
        room.status == "resolved_saved_carrier_topology_not_evaluated_boundary",
        "unexpected status {}",
        room.status
    );
    let report = json!({"format":"rvt-native-space-boundary-research/v2", "status":"saved_carrier_topology_2d_only", "source":a[1], "source_sha256":format!("{:x}", Sha256::digest(fs::read(&a[1])?)), "selected_ids":ids, "emitted_records":emitted, "coverage":coverage, "space_id":space_id, "topology_id":topology_id, "circuit_id":circuit_id, "level_id":level_id, "carrier_segments":carrier_ids.len(), "signed_area_square_feet":area, "coordinate_space":"native document horizontal carrier coordinates", "length_unit":"feet", "three_dimensional_boundary":false, "inventory":inventory});
    let bytes = serde_json::to_vec_pretty(&report)?;
    fs::write(&a[8], &bytes)?;
    println!(
        "{} sha256={:x} area_ft2={area:.15} status={}",
        a[8],
        Sha256::digest(&bytes),
        room.status
    );
    Ok(())
}
