//! Beam bodies on Revit 2023: a beam stops at the face of the column its
//! record names (RE-122).
//!
//! A Revit 2023 structural-framing record's box runs the beam's location
//! line to its ends, which sit inside the columns it frames into, at their
//! centres. Revit's export stops the beam at the column's face. The column
//! is named in the beam's counted reference list, as a joined wall is in a
//! wall's (RE-29), and the name is what separates it from a column that
//! only shares the plan position: modelo_bim's foundation beams name the
//! columns below their Level and not the ones standing on it.
//!
//! [`beam_column_trims`] takes, for each end of a beam's longer plan axis,
//! the one column that is named in the beam's list, holds that end in plan
//! and overlaps the beam in height, and cuts the end back to the column's
//! near face. A beam with an end that two named columns hold is declined
//! and keeps its box.
//!
//! # Honesty
//!
//! - Measured on the eight beams of one Revit 2023 project, modelo_bim,
//!   against Revit's own IFC export: 16 of 16 ends exact.
//! - Only Revit 2023 records are trimmed. On Snowdon Towers' 2024
//!   structural model, most steel beams stop a stored join cutback short
//!   of the named column's face (1/2" or 1-1/2"), concrete beams and bar
//!   joists at the face, and some ends not at all; when Revit applies the
//!   cutback is not decoded (`reports/element-framing/RE-122-beam-column-cuts.md`).
//! - Only axis-parallel beams are resolved: the cut is computed on boxes.

use crate::partition_element_records::PartitionElementRecord;
use std::collections::BTreeMap;

/// Field carrying which body a recovered beam is emitting.
pub const BEAM_BODY_SOURCE_FIELD: &str = "m_beam_body_source";
/// Value of [`BEAM_BODY_SOURCE_FIELD`] when a column cut an end.
pub const BEAM_BODY_COLUMN_CUT: &str = "partition_element_record_column_cut";

/// Where a beam's box is cut back, as new bounds on one plan axis.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BeamColumnTrim {
    /// The plan axis the beam runs along: 0 for x, 1 for y.
    pub axis: usize,
    /// The beam's new low bound on `axis`, feet.
    pub low_feet: f64,
    /// The beam's new high bound on `axis`, feet.
    pub high_feet: f64,
}

/// The column-cut bounds of each beam that a named column cuts at one end
/// or both, by ElementId.
pub fn beam_column_trims(
    beams: &[PartitionElementRecord],
    columns: &[PartitionElementRecord],
) -> BTreeMap<u32, BeamColumnTrim> {
    let mut out = BTreeMap::new();
    for beam in beams {
        let b = beam.bbox_feet;
        let (along_x, along_y) = (b[3] - b[0], b[4] - b[1]);
        if along_x == along_y {
            continue;
        }
        let axis = usize::from(along_y > along_x);
        let across = 1 - axis;
        let middle = (b[across] + b[across + 3]) / 2.0;
        let holding = |end: f64| -> Vec<&PartitionElementRecord> {
            columns
                .iter()
                .filter(|column| {
                    let c = column.bbox_feet;
                    beam.references.contains(&u64::from(column.element_id))
                        && c[axis] <= end
                        && end <= c[axis + 3]
                        && c[across] <= middle
                        && middle <= c[across + 3]
                        && c[2] < b[5]
                        && b[2] < c[5]
                })
                .collect()
        };
        let (low_columns, high_columns) = (holding(b[axis]), holding(b[axis + 3]));
        if low_columns.len() > 1 || high_columns.len() > 1 {
            continue;
        }
        let low = low_columns
            .first()
            .map_or(b[axis], |column| column.bbox_feet[axis + 3]);
        let high = high_columns
            .first()
            .map_or(b[axis + 3], |column| column.bbox_feet[axis]);
        if (low_columns.is_empty() && high_columns.is_empty()) || low >= high {
            continue;
        }
        out.insert(
            beam.element_id,
            BeamColumnTrim {
                axis,
                low_feet: low,
                high_feet: high,
            },
        );
    }
    out
}
