//! Integration test for the Layer 5 document-level IFC exporter.
//! Runs `RvtDocExporter` + `write_step` against every release in the
//! 11-version corpus and validates the STEP output structurally:
//!
//! 1. Contains the ISO-10303-21 header and terminator.
//! 2. Declares IFC4 schema.
//! 3. Includes every required framework entity.
//! 4. Emits exactly one IfcProject entity.
//! 5. Project name (if present) matches what `rvt-info` surfaces via
//!    the PartAtom path — cross-check against the same oracle that
//!    every other layer uses.

mod common;

use common::{ALL_YEARS, sample_for_year, samples_dir};
use rvt::ifc::{Exporter, RvtDocExporter, write_step};
use rvt::{Result, RevitFile};

fn corpus_available() -> bool {
    ALL_YEARS.iter().all(|y| sample_for_year(*y).exists())
}

#[test]
fn ifc_export_is_structurally_valid_on_every_release() -> Result<()> {
    if !corpus_available() {
        eprintln!("skipping: corpus missing at {}", samples_dir().display());
        return Ok(());
    }
    for year in ALL_YEARS {
        let path = sample_for_year(year);
        let mut rf = RevitFile::open(&path)?;
        let model = RvtDocExporter.export(&mut rf)?;
        let step = write_step(&model);

        // 1. envelope
        assert!(
            step.starts_with("ISO-10303-21;\n"),
            "{year}: missing ISO-10303-21 header"
        );
        assert!(
            step.ends_with("END-ISO-10303-21;\n"),
            "{year}: missing ISO-10303-21 terminator"
        );

        // 2. schema
        assert!(
            step.contains("FILE_SCHEMA(('IFC4'));"),
            "{year}: wrong FILE_SCHEMA line"
        );

        // 3. required framework entities — each must appear at
        // least once in the DATA section.
        for required in [
            "IFCPERSON",
            "IFCORGANIZATION",
            "IFCPERSONANDORGANIZATION",
            "IFCAPPLICATION",
            "IFCOWNERHISTORY",
            "IFCSIUNIT",
            "IFCUNITASSIGNMENT",
            "IFCCARTESIANPOINT",
            "IFCDIRECTION",
            "IFCAXIS2PLACEMENT3D",
            "IFCGEOMETRICREPRESENTATIONCONTEXT",
            "IFCPROJECT",
        ] {
            assert!(
                step.contains(required),
                "{year}: IFC output missing required entity {required}"
            );
        }

        // 4. exactly one IFCPROJECT entity.
        let proj_count = step.matches("IFCPROJECT(").count();
        assert_eq!(
            proj_count, 1,
            "{year}: expected exactly one IFCPROJECT entity, found {proj_count}"
        );

        // 5. cross-check project name against the PartAtom oracle.
        let title = rf.part_atom().ok().and_then(|pa| pa.title);
        if let Some(t) = &title {
            let escaped = t.replace('\'', "''");
            assert!(
                step.contains(&escaped),
                "{year}: project title {t:?} not present in IFC output"
            );
        }
    }
    Ok(())
}

#[test]
fn ifc_export_has_no_unescaped_apostrophes() -> Result<()> {
    if !corpus_available() {
        return Ok(());
    }
    for year in ALL_YEARS {
        let path = sample_for_year(year);
        let mut rf = RevitFile::open(&path)?;
        let model = RvtDocExporter.export(&mut rf)?;
        let step = write_step(&model);

        // STEP strings use `''` as the literal apostrophe. An isolated
        // `'` inside a quoted string would prematurely terminate it.
        // We check by counting apostrophes in each quoted span — each
        // span must have its count be even after collapsing `''`.
        for line in step.lines() {
            if !line.starts_with('#') {
                continue;
            }
            let apostrophes = line.matches('\'').count();
            // After collapsing `''` pairs, we should have an even
            // number of remaining apostrophes (start+end of each
            // string literal).
            let doubled_pairs = line.matches("''").count();
            let remaining_singles = apostrophes - 2 * doubled_pairs;
            assert!(
                remaining_singles.is_multiple_of(2),
                "{year}: entity has an odd number of unescaped apostrophes — {line}"
            );
        }
    }
    Ok(())
}
