//! Shared partition ArcWall iteration (RE-15 / D20).
//!
//! The production walker (`iter_elements`) only scans `Global/Latest`
//! and therefore cannot see ArcWall instance records, which live in
//! `Partitions/*`. The IFC exporter previously inlined its own
//! partition scan; this module is the shared path for IFC export,
//! diagnostics, CLI counts, and ElemTable linkage.

use crate::arc_wall_record::{
    ArcWallRecord, ArcWallScanStatus, ArcWallTrailer, STANDARD_RECORD_MIN_SIZE,
};
use crate::compression;
use crate::ifc::Storey;
use crate::{Result, RevitFile};
use std::collections::BTreeMap;

/// Stable reference to a decoded ArcWall record inside a partition
/// stream. Suitable as an ElemTable → partition index value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PartitionArcWallRef {
    /// Stream name, e.g. `"Partitions/5"`.
    pub partition: String,
    /// Byte offset of the ArcWall tag inside the decompressed
    /// partition buffer.
    pub offset: usize,
}

/// One standard ArcWall recovered from a partition stream, including
/// optional trailer fields when the singleton trailer region is
/// present and validates.
#[derive(Debug, Clone, PartialEq)]
pub struct PartitionArcWall {
    pub partition: String,
    pub offset: usize,
    pub record: ArcWallRecord,
    pub trailer: Option<ArcWallTrailer>,
}

impl PartitionArcWall {
    /// ElemTable-linkable ElementId when the trailer validated.
    pub fn element_id(&self) -> Option<u32> {
        self.trailer.as_ref().and_then(|t| t.element_id)
    }

    /// Shared type-symbol candidate from the trailer (`+0xfe`).
    pub fn type_id(&self) -> Option<u32> {
        self.trailer.as_ref().and_then(|t| t.type_id)
    }

    /// Base elevation in feet (trailer `+0xf6`, else core start Z).
    pub fn base_elevation_feet(&self) -> Option<f64> {
        self.trailer
            .as_ref()
            .and_then(|t| t.base_elevation_feet)
            .or_else(|| {
                let z = self.record.start_point().2;
                z.is_finite().then_some(z)
            })
    }

    /// Unconnected height from the core Z delta when present.
    pub fn height_feet(&self) -> Option<f64> {
        self.record.height_feet()
    }

    /// Thickness is not present in the 2023 singleton ArcWall trailer
    /// (RE-15). Always `None` until a WallType / HostObjAttr width
    /// join lands.
    pub fn thickness_feet(&self) -> Option<f64> {
        None
    }

    pub fn partition_ref(&self) -> PartitionArcWallRef {
        PartitionArcWallRef {
            partition: self.partition.clone(),
            offset: self.offset,
        }
    }
}

/// Version-scoped scan report across every `Partitions/*` stream.
#[derive(Debug, Clone, PartialEq)]
pub struct PartitionArcWallScan {
    pub status: ArcWallScanStatus,
    pub walls: Vec<PartitionArcWall>,
}

/// Scan every `Partitions/*` stream for version-gated standard
/// ArcWall records. Unsupported Revit releases return an empty wall
/// list with an `UnsupportedVersion` status (no 2023 pattern applied).
pub fn scan_partition_arc_walls(
    rf: &mut RevitFile,
    revit_version: u32,
) -> Result<PartitionArcWallScan> {
    let status = ArcWallRecord::standard_decoder_status(revit_version);
    if !status.is_supported() {
        return Ok(PartitionArcWallScan {
            status,
            walls: Vec::new(),
        });
    }

    let partition_streams: Vec<String> = rf
        .stream_names()
        .into_iter()
        .filter(|s| s.starts_with("Partitions/"))
        .collect();

    let mut walls = Vec::new();
    for partition in partition_streams {
        let Ok(raw) = rf.read_stream(&partition) else {
            continue;
        };
        let chunks = compression::inflate_all_chunks(&raw);
        let concat: Vec<u8> = chunks.into_iter().flatten().collect();
        if concat.len() < STANDARD_RECORD_MIN_SIZE {
            continue;
        }
        let report = ArcWallRecord::scan_standard_for_revit_version(revit_version, &concat);
        for off in report.offsets {
            let Ok(record) = ArcWallRecord::decode_standard(&concat, off) else {
                continue;
            };
            let trailer = ArcWallRecord::decode_trailer(&concat, off);
            walls.push(PartitionArcWall {
                partition: partition.clone(),
                offset: off,
                record,
                trailer,
            });
        }
    }

    Ok(PartitionArcWallScan { status, walls })
}

/// Convenience wrapper: read BasicFileInfo version, then scan.
pub fn iter_partition_arc_walls(rf: &mut RevitFile) -> Result<PartitionArcWallScan> {
    let version = rf.basic_file_info()?.version;
    scan_partition_arc_walls(rf, version)
}

/// Build ElementId → partition-record index from decoded ArcWalls.
///
/// Only walls whose trailer ElementId validated are indexed. Duplicate
/// ElementIds keep the first occurrence (stable scan order).
pub fn element_id_partition_index(
    walls: &[PartitionArcWall],
) -> BTreeMap<u32, PartitionArcWallRef> {
    let mut map = BTreeMap::new();
    for wall in walls {
        if let Some(id) = wall.element_id() {
            map.entry(id).or_insert_with(|| wall.partition_ref());
        }
    }
    map
}

/// Result of deriving storeys from ArcWall elevations, optionally
/// labelled with partition Level-like display names (RE-15 / #86).
#[derive(Debug, Clone, PartialEq)]
pub struct ArcWallStoreyRecovery {
    pub storeys: Vec<Storey>,
    /// Storeys whose name came from a partition Level-like string.
    pub named_from_partition: usize,
    /// Storeys that still use the `Elevation … ft` fallback label.
    pub elevation_fallback: usize,
    /// Count of building-storey name candidates considered.
    pub level_name_candidates: usize,
}

/// Derive IFC building storeys from distinct ArcWall base elevations.
///
/// When `level_names` contains confident building-storey labels
/// (see [`crate::partition_name_candidates::is_building_storey_name`]),
/// names are applied by ordinal (`Level 1` → lowest elevation, `Roof`
/// → highest) or by exact count zip. Unmatched storeys keep an
/// elevation fallback label — never invent a `"Level 1"` placeholder
/// for the whole model.
pub fn storeys_from_arc_wall_base_elevations(walls: &[PartitionArcWall]) -> Vec<Storey> {
    recover_storeys_from_arc_walls(walls, &[]).storeys
}

/// Like [`storeys_from_arc_wall_base_elevations`], but applies partition
/// Level-like display names when the match is confident enough.
pub fn recover_storeys_from_arc_walls(
    walls: &[PartitionArcWall],
    level_names: &[String],
) -> ArcWallStoreyRecovery {
    let mut elevations: Vec<f64> = walls
        .iter()
        .filter_map(PartitionArcWall::base_elevation_feet)
        .filter(|z| z.is_finite())
        .collect();
    elevations.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    elevations.dedup_by(|a, b| (*a - *b).abs() < 1e-6);

    let filtered: Vec<String> = level_names
        .iter()
        .filter(|n| crate::partition_name_candidates::is_building_storey_name(n))
        .cloned()
        .collect();

    let mut storeys: Vec<Storey> = elevations
        .iter()
        .map(|&elevation_feet| Storey {
            name: elevation_fallback_name(elevation_feet),
            elevation_feet,
        })
        .collect();

    if storeys.is_empty() {
        return ArcWallStoreyRecovery {
            storeys,
            named_from_partition: 0,
            elevation_fallback: 0,
            level_name_candidates: filtered.len(),
        };
    }

    if filtered.len() == storeys.len() {
        let ordered = order_building_storey_names(&filtered);
        for (storey, name) in storeys.iter_mut().zip(ordered) {
            storey.name = name;
        }
    } else {
        apply_pattern_storey_names(&mut storeys, &filtered);
    }

    let named_from_partition = storeys
        .iter()
        .filter(|s| !s.name.starts_with("Elevation "))
        .count();
    let elevation_fallback = storeys.len() - named_from_partition;
    ArcWallStoreyRecovery {
        storeys,
        named_from_partition,
        elevation_fallback,
        level_name_candidates: filtered.len(),
    }
}

fn elevation_fallback_name(elevation_feet: f64) -> String {
    format!("Elevation {elevation_feet:.3} ft")
}

fn parse_level_number(name: &str) -> Option<u32> {
    let lower = name.trim().to_ascii_lowercase();
    let rest = lower.strip_prefix("level ")?;
    let digits: String = rest.chars().filter(|c| c.is_ascii_digit()).collect();
    if digits.is_empty() || digits.len() != rest.chars().filter(|c| !c.is_whitespace()).count() {
        return None;
    }
    digits.parse().ok()
}

fn order_building_storey_names(names: &[String]) -> Vec<String> {
    let mut indexed: Vec<(i32, String)> = names
        .iter()
        .map(|name| {
            let lower = name.to_ascii_lowercase();
            let rank = if lower == "basement" {
                -2
            } else if lower == "ground floor" || lower == "groundfloor" {
                -1
            } else if lower == "roof" {
                10_000
            } else if let Some(n) = parse_level_number(name) {
                n as i32
            } else if lower == "first floor" {
                1
            } else if lower == "second floor" {
                2
            } else if lower == "third floor" {
                3
            } else if lower == "mezzanine" {
                50
            } else if lower == "podium" {
                0
            } else {
                100
            };
            (rank, name.clone())
        })
        .collect();
    indexed.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.cmp(&b.1)));
    indexed.into_iter().map(|(_, name)| name).collect()
}

fn apply_pattern_storey_names(storeys: &mut [Storey], names: &[String]) {
    for name in names {
        let lower = name.to_ascii_lowercase();
        if lower == "roof" {
            if let Some(last) = storeys.last_mut() {
                if last.name.starts_with("Elevation ") {
                    last.name = name.clone();
                }
            }
            continue;
        }
        if lower == "ground floor"
            || lower == "groundfloor"
            || lower == "basement"
            || lower == "podium"
        {
            if let Some(first) = storeys.first_mut() {
                if first.name.starts_with("Elevation ") {
                    first.name = name.clone();
                }
            }
            continue;
        }
        if let Some(n) = parse_level_number(name) {
            let idx = (n as usize).saturating_sub(1);
            if let Some(storey) = storeys.get_mut(idx) {
                if storey.name.starts_with("Elevation ") {
                    storey.name = name.clone();
                }
            }
        }
    }
}

/// Match a wall's base elevation to a storey index (nearest within
/// 1e-3 ft). Returns `None` when no storeys match.
pub fn storey_index_for_elevation(storeys: &[Storey], elevation_feet: f64) -> Option<usize> {
    storeys
        .iter()
        .enumerate()
        .find(|(_, storey)| (storey.elevation_feet - elevation_feet).abs() < 1e-3)
        .map(|(idx, _)| idx)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::arc_wall_record::{
        ARC_WALL_TAG, ARC_WALL_VARIANT_STANDARD, RECORD_TRAILER, SCHEMA_FAMILY_MARKER,
        STANDARD_RECORD_SINGLETON_STRIDE,
    };

    fn fixture_record_bytes_with_trailer(element_id: u32, base_z: f64) -> Vec<u8> {
        // Core from RECORD_4_HEX in arc_wall_record tests, then a
        // minimal singleton trailer with validated ElementId slots.
        let mut buf = vec![0u8; STANDARD_RECORD_SINGLETON_STRIDE];
        buf[0] = (ARC_WALL_TAG & 0xff) as u8;
        buf[1] = (ARC_WALL_TAG >> 8) as u8;
        buf[4..8].copy_from_slice(&SCHEMA_FAMILY_MARKER.to_le_bytes());
        buf[8..12].copy_from_slice(&1u32.to_le_bytes());
        buf[12..16].copy_from_slice(&3u32.to_le_bytes());
        buf[0x10..0x12].copy_from_slice(&ARC_WALL_VARIANT_STANDARD.to_le_bytes());
        // start (0,0,base_z) end (10,0,base_z+10)
        for (i, v) in [0.0, 0.0, base_z, 10.0, 0.0, base_z + 10.0]
            .into_iter()
            .enumerate()
        {
            let p = 0x12 + i * 8;
            buf[p..p + 8].copy_from_slice(&v.to_le_bytes());
            let q = 0x42 + i * 8;
            buf[q..q + 8].copy_from_slice(&v.to_le_bytes());
        }
        buf[0x72] = RECORD_TRAILER;

        // Trailer: type id + base elevation + element id pair.
        buf[0xfe..0x102].copy_from_slice(&0x217au32.to_le_bytes());
        buf[0xf6..0xfe].copy_from_slice(&base_z.to_le_bytes());
        buf[0x10e..0x112].copy_from_slice(&element_id.to_le_bytes());
        buf[0x11c..0x120].copy_from_slice(&element_id.to_le_bytes());
        buf
    }

    #[test]
    fn element_id_index_maps_validated_trailer_ids() {
        let bytes = fixture_record_bytes_with_trailer(0x1c79, 0.0);
        let record = ArcWallRecord::decode_standard(&bytes, 0).unwrap();
        let trailer = ArcWallRecord::decode_trailer(&bytes, 0);
        let wall = PartitionArcWall {
            partition: "Partitions/5".into(),
            offset: 100,
            record,
            trailer,
        };
        assert_eq!(wall.element_id(), Some(0x1c79));
        assert_eq!(wall.type_id(), Some(0x217a));
        assert_eq!(wall.height_feet(), Some(10.0));
        assert!(wall.thickness_feet().is_none());

        let index = element_id_partition_index(std::slice::from_ref(&wall));
        assert_eq!(
            index.get(&0x1c79),
            Some(&PartitionArcWallRef {
                partition: "Partitions/5".into(),
                offset: 100,
            })
        );
    }

    #[test]
    fn storeys_dedup_base_elevations() {
        let walls: Vec<_> = [0.0, 0.0, 6.5617, 6.5617, 13.1234]
            .into_iter()
            .enumerate()
            .map(|(i, z)| {
                let bytes = fixture_record_bytes_with_trailer(1000 + i as u32, z);
                PartitionArcWall {
                    partition: "Partitions/5".into(),
                    offset: i * STANDARD_RECORD_SINGLETON_STRIDE,
                    record: ArcWallRecord::decode_standard(&bytes, 0).unwrap(),
                    trailer: ArcWallRecord::decode_trailer(&bytes, 0),
                }
            })
            .collect();
        let storeys = storeys_from_arc_wall_base_elevations(&walls);
        assert_eq!(storeys.len(), 3);
        assert!((storeys[0].elevation_feet - 0.0).abs() < 1e-9);
        assert!((storeys[1].elevation_feet - 6.5617).abs() < 1e-6);
        assert_eq!(storey_index_for_elevation(&storeys, 6.5617), Some(1));
    }

    #[test]
    fn storey_names_apply_level_and_roof_patterns() {
        let walls: Vec<_> = [0.0, 6.5617, 13.1234, 19.685]
            .into_iter()
            .enumerate()
            .map(|(i, z)| {
                let bytes = fixture_record_bytes_with_trailer(2000 + i as u32, z);
                PartitionArcWall {
                    partition: "Partitions/5".into(),
                    offset: i * STANDARD_RECORD_SINGLETON_STRIDE,
                    record: ArcWallRecord::decode_standard(&bytes, 0).unwrap(),
                    trailer: ArcWallRecord::decode_trailer(&bytes, 0),
                }
            })
            .collect();
        let names = vec![
            "Level Head - Upgrade".into(),
            "Level 1".into(),
            "Roof".into(),
        ];
        let recovery = recover_storeys_from_arc_walls(&walls, &names);
        assert_eq!(recovery.level_name_candidates, 2); // Head filtered out
        assert_eq!(recovery.storeys[0].name, "Level 1");
        assert!(recovery.storeys[1].name.starts_with("Elevation "));
        assert!(recovery.storeys[2].name.starts_with("Elevation "));
        assert_eq!(recovery.storeys[3].name, "Roof");
        assert_eq!(recovery.named_from_partition, 2);
        assert_eq!(recovery.elevation_fallback, 2);
    }

    #[test]
    fn storey_names_zip_when_counts_match() {
        let walls: Vec<_> = [0.0, 10.0]
            .into_iter()
            .enumerate()
            .map(|(i, z)| {
                let bytes = fixture_record_bytes_with_trailer(3000 + i as u32, z);
                PartitionArcWall {
                    partition: "Partitions/5".into(),
                    offset: i * STANDARD_RECORD_SINGLETON_STRIDE,
                    record: ArcWallRecord::decode_standard(&bytes, 0).unwrap(),
                    trailer: ArcWallRecord::decode_trailer(&bytes, 0),
                }
            })
            .collect();
        let recovery =
            recover_storeys_from_arc_walls(&walls, &["Roof".into(), "Ground floor".into()]);
        assert_eq!(recovery.storeys[0].name, "Ground floor");
        assert_eq!(recovery.storeys[1].name, "Roof");
        assert_eq!(recovery.elevation_fallback, 0);
    }
}
