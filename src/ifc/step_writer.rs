//! Minimal STEP / ISO-10303-21 serializer for `IfcModel` → valid IFC4.
//!
//! This is the serialization half of Layer 5. Given an `IfcModel` in
//! memory, produce a .ifc text file that a spec-compliant reader
//! (IfcOpenShell, BlenderBIM, buildingSMART validator) accepts.
//!
//! The output is intentionally minimal but **structurally valid**: a
//! well-formed IFC4 schema header, the required framework entities
//! (`IfcPerson` / `IfcOrganization` / `IfcApplication` /
//! `IfcOwnerHistory` / `IfcSIUnit` / `IfcUnitAssignment` /
//! `IfcGeometricRepresentationContext`), and an `IfcProject` populated
//! from the model's metadata. As the walker grows, `BuildingElement`
//! and family entities will land too; those extensions plug in here
//! without touching the header-level plumbing.
//!
//! Design principle: string-based emission, no external IFC library
//! dependency, fully `#![deny(unsafe_code)]`-clean.

use super::IfcModel;

/// Serialize an `IfcModel` into an IFC4 STEP text stream. The output
/// includes the ISO-10303-21 envelope and a minimal but spec-valid
/// data section centred on `IfcProject`.
pub fn write_step(model: &IfcModel) -> String {
    let mut w = StepWriter::new();
    w.emit_header(model);
    w.emit_data(model);
    w.finish()
}

struct StepWriter {
    out: String,
    next_id: usize,
}

impl StepWriter {
    fn new() -> Self {
        Self {
            out: String::new(),
            next_id: 1,
        }
    }

    fn id(&mut self) -> usize {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    fn emit_line<S: AsRef<str>>(&mut self, line: S) {
        self.out.push_str(line.as_ref());
        self.out.push('\n');
    }

    fn emit_entity<S: AsRef<str>>(&mut self, id: usize, body: S) {
        self.out.push_str(&format!("#{id}={};\n", body.as_ref()));
    }

    fn emit_header(&mut self, model: &IfcModel) {
        let project = escape(model.project_name.as_deref().unwrap_or("Untitled"));
        let desc = escape(model.description.as_deref().unwrap_or(
            "Produced by rvt-rs (https://github.com/DrunkOnJava/rvt-rs) — \
                 clean-room Apache-2 Revit reader.",
        ));
        self.emit_line("ISO-10303-21;");
        self.emit_line("HEADER;");
        self.emit_line("FILE_DESCRIPTION(('ViewDefinition [CoordinationView]'),'2;1');");
        self.emit_line(format!(
            "FILE_NAME('{project}.ifc','{}',('rvt-rs'),('DrunkOnJava/rvt-rs'),'rvt-rs 0.1.x','rvt-rs STEP writer','');",
            iso_timestamp()
        ));
        self.emit_line("FILE_SCHEMA(('IFC4'));");
        self.emit_line("ENDSEC;");
        self.emit_line(format!("/* {desc} */"));
    }

    fn emit_data(&mut self, model: &IfcModel) {
        self.emit_line("DATA;");

        // Required framework entities (buildingSMART minimum viable).
        let person = self.id();
        self.emit_entity(person, "IFCPERSON($,$,'rvt-rs',$,$,$,$,$)");
        let org = self.id();
        self.emit_entity(
            org,
            "IFCORGANIZATION($,'rvt-rs','Clean-room Apache-2 Revit reader',$,$)",
        );
        let person_and_org = self.id();
        self.emit_entity(
            person_and_org,
            format!("IFCPERSONANDORGANIZATION(#{person},#{org},$)"),
        );
        let application = self.id();
        self.emit_entity(
            application,
            format!("IFCAPPLICATION(#{org},'0.1.x','rvt-rs','{}')", "rvt_rs"),
        );
        let owner_hist = self.id();
        self.emit_entity(
            owner_hist,
            format!(
                "IFCOWNERHISTORY(#{person_and_org},#{application},$,.ADDED.,$,#{person_and_org},#{application},{})",
                unix_seconds()
            ),
        );

        // Unit assignment — fixed to SI millimetres + square metres +
        // cubic metres for v1. Future: wire from model.units (Forge
        // unit identifiers → IfcSIUnit mapping).
        let u_length = self.id();
        self.emit_entity(u_length, "IFCSIUNIT(*,.LENGTHUNIT.,.MILLI.,.METRE.)");
        let u_area = self.id();
        self.emit_entity(u_area, "IFCSIUNIT(*,.AREAUNIT.,$,.SQUARE_METRE.)");
        let u_volume = self.id();
        self.emit_entity(u_volume, "IFCSIUNIT(*,.VOLUMEUNIT.,$,.CUBIC_METRE.)");
        let u_plane_angle = self.id();
        self.emit_entity(u_plane_angle, "IFCSIUNIT(*,.PLANEANGLEUNIT.,$,.RADIAN.)");
        let unit_assignment = self.id();
        self.emit_entity(
            unit_assignment,
            format!("IFCUNITASSIGNMENT((#{u_length},#{u_area},#{u_volume},#{u_plane_angle}))"),
        );

        // Representation context — needs IfcAxis2Placement3D +
        // IfcDirection + IfcCartesianPoint (origin, X, Z axes).
        let origin = self.id();
        self.emit_entity(origin, "IFCCARTESIANPOINT((0.,0.,0.))");
        let z_axis = self.id();
        self.emit_entity(z_axis, "IFCDIRECTION((0.,0.,1.))");
        let x_axis = self.id();
        self.emit_entity(x_axis, "IFCDIRECTION((1.,0.,0.))");
        let axis_placement = self.id();
        self.emit_entity(
            axis_placement,
            format!("IFCAXIS2PLACEMENT3D(#{origin},#{z_axis},#{x_axis})"),
        );
        let geom_ctx = self.id();
        self.emit_entity(
            geom_ctx,
            format!("IFCGEOMETRICREPRESENTATIONCONTEXT($,'Model',3,1.E-5,#{axis_placement},$)"),
        );

        // Root project.
        let project_name = escape(model.project_name.as_deref().unwrap_or("Untitled"));
        let project_desc = escape(model.description.as_deref().unwrap_or("Exported by rvt-rs"));
        let project_id = self.id();
        self.emit_entity(
            project_id,
            format!(
                "IFCPROJECT('{}',#{owner_hist},'{}',{},$,$,$,(#{geom_ctx}),#{unit_assignment})",
                make_guid(project_id),
                project_name,
                quoted_or_dollar(&project_desc),
            ),
        );

        // Spatial containment hierarchy — required by IFC4 for any
        // project with building content. We emit a minimal but valid
        // IfcSite → IfcBuilding → IfcBuildingStorey chain with
        // identity placements so downstream viewers (BlenderBIM,
        // IfcOpenShell-based tools, buildingSMART validator) render
        // the file directly without needing to synthesise a host
        // structure. Names default to "Default {Site,Building,Level
        // 1}"; once the walker surfaces site/level instances they'll
        // flow in here.
        //
        // Every IfcSpatialStructureElement needs its own
        // IfcLocalPlacement — we share the `axis_placement` across
        // the three (they're all identity), then chain the
        // placements via `PlacementRelTo` so the coordinate frames
        // compose correctly.
        let site_placement = self.id();
        self.emit_entity(
            site_placement,
            format!("IFCLOCALPLACEMENT($,#{axis_placement})"),
        );
        let site_id = self.id();
        self.emit_entity(
            site_id,
            format!(
                "IFCSITE('{}',#{owner_hist},'Default Site',$,$,#{site_placement},$,'Default Site',.ELEMENT.,$,$,$,$,$)",
                make_guid(site_id),
            ),
        );

        let building_placement = self.id();
        self.emit_entity(
            building_placement,
            format!("IFCLOCALPLACEMENT(#{site_placement},#{axis_placement})"),
        );
        let building_id = self.id();
        self.emit_entity(
            building_id,
            format!(
                "IFCBUILDING('{}',#{owner_hist},'Default Building',$,$,#{building_placement},$,'Default Building',.ELEMENT.,$,$,$)",
                make_guid(building_id),
            ),
        );

        let storey_placement = self.id();
        self.emit_entity(
            storey_placement,
            format!("IFCLOCALPLACEMENT(#{building_placement},#{axis_placement})"),
        );
        let storey_id = self.id();
        self.emit_entity(
            storey_id,
            format!(
                "IFCBUILDINGSTOREY('{}',#{owner_hist},'Level 1',$,$,#{storey_placement},$,'Level 1',.ELEMENT.,0.)",
                make_guid(storey_id),
            ),
        );

        // Aggregation relationships — IfcRelAggregates is how the
        // spatial hierarchy binds in IFC4. Each level of the chain
        // gets one relationship pointing from parent to child.
        let rel_proj_site = self.id();
        self.emit_entity(
            rel_proj_site,
            format!(
                "IFCRELAGGREGATES('{}',#{owner_hist},$,$,#{project_id},(#{site_id}))",
                make_guid(rel_proj_site),
            ),
        );
        let rel_site_building = self.id();
        self.emit_entity(
            rel_site_building,
            format!(
                "IFCRELAGGREGATES('{}',#{owner_hist},$,$,#{site_id},(#{building_id}))",
                make_guid(rel_site_building),
            ),
        );
        let rel_building_storey = self.id();
        self.emit_entity(
            rel_building_storey,
            format!(
                "IFCRELAGGREGATES('{}',#{owner_hist},$,$,#{building_id},(#{storey_id}))",
                make_guid(rel_building_storey),
            ),
        );

        // Classifications — one IfcClassification per source
        // (OmniClass, Uniformat, …), with one IfcClassificationReference
        // per coded item. Each classification gets its own
        // IfcRelAssociatesClassification tying its references back to
        // the project, which is how IFC4 consumers (BlenderBIM,
        // IfcOpenShell's classification viewer) discover code refs.
        //
        // RvtDocExporter populates `model.classifications` from
        // PartAtom's `<category term="...">` blocks. Previously those
        // codes were collected but never emitted; this wires them
        // through the STEP writer so downstream consumers can see
        // them directly.
        for classification in &model.classifications {
            let source_name = match &classification.source {
                super::entities::ClassificationSource::OmniClass => "OmniClass",
                super::entities::ClassificationSource::Uniformat => "Uniformat",
                super::entities::ClassificationSource::Other(s) => s.as_str(),
            };
            let source_name_escaped = escape(source_name);
            let edition = classification
                .edition
                .as_deref()
                .map(escape)
                .map(|e| format!("'{e}'"))
                .unwrap_or_else(|| "$".into());

            let classification_id = self.id();
            self.emit_entity(
                classification_id,
                format!("IFCCLASSIFICATION($,{edition},$,'{source_name_escaped}',$,$,$)"),
            );

            // One IfcClassificationReference per item; collect their
            // ids so we can bundle them into the IfcRelAssociatesClassification.
            let mut ref_ids: Vec<usize> = Vec::with_capacity(classification.items.len());
            for item in &classification.items {
                let code_escaped = escape(&item.code);
                let name_str = item
                    .name
                    .as_deref()
                    .map(escape)
                    .map(|n| format!("'{n}'"))
                    .unwrap_or_else(|| "$".into());
                let ref_id = self.id();
                self.emit_entity(
                    ref_id,
                    format!(
                        "IFCCLASSIFICATIONREFERENCE($,'{code_escaped}',{name_str},#{classification_id},$)"
                    ),
                );
                ref_ids.push(ref_id);
            }

            if !ref_ids.is_empty() {
                let refs_list = ref_ids
                    .iter()
                    .map(|id| format!("#{id}"))
                    .collect::<Vec<_>>()
                    .join(",");
                // IfcRelAssociatesClassification binds a set of objects
                // to one classification reference. IFC4's schema
                // requires the RelatingClassification to be a single
                // IfcClassificationReferenceSelect; we pick the last
                // reference as the relating one and treat the rest as
                // project associations. If the project only has one
                // reference this is exact; when there are multiple,
                // each gets its own association relationship.
                for ref_id in &ref_ids {
                    let rel_id = self.id();
                    self.emit_entity(
                        rel_id,
                        format!(
                            "IFCRELASSOCIATESCLASSIFICATION('{}',#{owner_hist},$,$,(#{project_id}),#{ref_id})",
                            make_guid(rel_id),
                        ),
                    );
                }
                // Silence the warning about an unused local when the
                // outer `for` loop only iterates once.
                let _ = refs_list;
            }
        }

        // Future: emit `model.entities` as IFCBUILDINGELEMENT et al.
        // here, each wired to `storey_id` via `IfcRelContainedInSpatialStructure`.
        // The shape is well-defined once the walker surfaces typed
        // `BuildingElement` values.

        self.emit_line("ENDSEC;");
    }

    fn finish(self) -> String {
        let mut out = self.out;
        out.push_str("END-ISO-10303-21;\n");
        out
    }
}

/// STEP-style string escape. IFC uses single-quoted strings with `''`
/// for literal apostrophes and `\S\\` / `\X\` escapes for non-ASCII.
/// For our minimal emitter we handle the apostrophe case and
/// pass-through ASCII; non-ASCII strings get their bytes stripped to
/// safe replacements (conservative, avoids invalid STEP output).
fn escape(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for c in input.chars() {
        if c == '\'' {
            out.push_str("''");
        } else if c.is_ascii() && !c.is_control() {
            out.push(c);
        } else {
            out.push('_');
        }
    }
    out
}

fn quoted_or_dollar(s: &str) -> String {
    if s.is_empty() {
        "$".into()
    } else {
        format!("'{s}'")
    }
}

fn iso_timestamp() -> String {
    // Minimal implementation: format the current Unix epoch as ISO
    // 8601. Avoids chrono to stay dep-lean.
    let secs = unix_seconds();
    let (y, m, d, hh, mm, ss) = epoch_to_ymdhms(secs);
    format!("{y:04}-{m:02}-{d:02}T{hh:02}:{mm:02}:{ss:02}")
}

fn unix_seconds() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// Gregorian breakdown without chrono. Good from 1970 through 2400.
fn epoch_to_ymdhms(secs: i64) -> (i32, u32, u32, u32, u32, u32) {
    let days = secs.div_euclid(86_400);
    let remainder = secs.rem_euclid(86_400) as u32;
    let hh = remainder / 3600;
    let mm = (remainder % 3600) / 60;
    let ss = remainder % 60;

    // Days since 1970-01-01 → Gregorian date. Algorithm from Howard
    // Hinnant's date.h (public domain).
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = (yoe as i64) + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = (if mp < 10 { mp + 3 } else { mp - 9 }) as u32;
    let y = (y + if m <= 2 { 1 } else { 0 }) as i32;
    (y, m, d, hh, mm, ss)
}

/// IFC4 globally-unique-ID. Format: 22 chars from the IFC-GUID
/// alphabet (`0-9A-Za-z_$`, 64 symbols). The spec requires these be
/// unique per file but does not mandate a specific encoding —
/// `IfcOpenShell` and `buildingSMART` validators accept any 22-char
/// string in the alphabet.
///
/// v1 encoding is deterministic per `index`: a fixed 6-char `"0rvtrs"`
/// prefix followed by the base-64 big-endian encoding of `index` into
/// 16 chars. Gives a bijection between `index` and GUID for the first
/// 64^16 ≈ 7.9 × 10^28 entities — trivially enough. Stable across
/// runs (same input → same output), which makes STEP text diffs
/// tractable.
///
/// Future: once the walker surfaces real per-element GUIDs from the
/// Revit file, we'll prefer those (they're already in the correct
/// format) and fall back to this for entities without a native GUID.
fn make_guid(index: usize) -> String {
    const ALPHABET: &[u8; 64] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz_$";
    let mut guid = String::with_capacity(22);
    guid.push_str("0rvtrs");
    let mut suffix = [b'0'; 16];
    let mut n = index;
    for slot in suffix.iter_mut().rev() {
        *slot = ALPHABET[n & 63];
        n >>= 6;
    }
    for b in &suffix {
        guid.push(*b as char);
    }
    guid
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn step_emits_iso_envelope() {
        let model = IfcModel {
            project_name: Some("Demo".into()),
            description: None,
            entities: Vec::new(),
            classifications: Vec::new(),
            units: Vec::new(),
        };
        let s = write_step(&model);
        assert!(s.starts_with("ISO-10303-21;\n"));
        assert!(s.contains("FILE_SCHEMA(('IFC4'));"));
        assert!(s.contains("DATA;"));
        assert!(s.contains("IFCPROJECT"));
        assert!(s.ends_with("END-ISO-10303-21;\n"));
    }

    #[test]
    fn step_escapes_apostrophes_in_project_name() {
        let model = IfcModel {
            project_name: Some("Griffin's Building".into()),
            description: None,
            entities: Vec::new(),
            classifications: Vec::new(),
            units: Vec::new(),
        };
        let s = write_step(&model);
        assert!(s.contains("Griffin''s Building"));
    }

    #[test]
    fn step_includes_required_framework_entities() {
        let model = IfcModel::default();
        let s = write_step(&model);
        for required in [
            "IFCPERSON",
            "IFCORGANIZATION",
            "IFCAPPLICATION",
            "IFCOWNERHISTORY",
            "IFCSIUNIT",
            "IFCUNITASSIGNMENT",
            "IFCGEOMETRICREPRESENTATIONCONTEXT",
            "IFCPROJECT",
        ] {
            assert!(s.contains(required), "missing required entity: {required}");
        }
    }

    #[test]
    fn epoch_to_ymdhms_known_dates() {
        // 1970-01-01 00:00:00 UTC
        assert_eq!(epoch_to_ymdhms(0), (1970, 1, 1, 0, 0, 0));
        // 2024-04-01 00:00:00 UTC = 1711929600
        assert_eq!(epoch_to_ymdhms(1_711_929_600), (2024, 4, 1, 0, 0, 0));
    }

    #[test]
    fn step_emits_spatial_hierarchy() {
        // IFC4 viewers expect a Project → Site → Building → Storey
        // spine before any building elements. Every IfcSpatialStructureElement
        // in the chain needs its own IfcLocalPlacement and must be
        // bound to its parent via IfcRelAggregates.
        let model = IfcModel::default();
        let s = write_step(&model);
        for required in [
            "IFCSITE(",
            "IFCBUILDING(",
            "IFCBUILDINGSTOREY(",
            "IFCLOCALPLACEMENT(",
            "IFCRELAGGREGATES(",
        ] {
            assert!(
                s.contains(required),
                "spatial hierarchy missing required entity: {required}\n\nOutput:\n{s}"
            );
        }
    }

    #[test]
    fn step_hierarchy_count_is_stable() {
        // The hierarchy adds exactly:
        //   3 IfcLocalPlacement (one per spatial container)
        //   1 IfcSite
        //   1 IfcBuilding
        //   1 IfcBuildingStorey
        //   3 IfcRelAggregates (project-site, site-building, building-storey)
        // Pinning the counts prevents silent regressions if the writer
        // grows extra placeholder entities.
        let model = IfcModel::default();
        let s = write_step(&model);
        assert_eq!(s.matches("IFCSITE(").count(), 1);
        assert_eq!(s.matches("IFCBUILDING(").count(), 1);
        assert_eq!(s.matches("IFCBUILDINGSTOREY(").count(), 1);
        assert_eq!(s.matches("IFCLOCALPLACEMENT(").count(), 3);
        assert_eq!(s.matches("IFCRELAGGREGATES(").count(), 3);
    }

    #[test]
    fn make_guid_is_22_chars_in_alphabet() {
        let g = make_guid(0);
        assert_eq!(g.len(), 22, "IFC GUIDs must be exactly 22 characters");
        const ALPHABET: &str = "0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz_$";
        for c in g.chars() {
            assert!(
                ALPHABET.contains(c),
                "character {c:?} not in IFC GUID alphabet"
            );
        }
    }

    #[test]
    fn make_guid_is_deterministic_and_distinct() {
        // Same input → same output (stable diffs across runs).
        assert_eq!(make_guid(42), make_guid(42));
        // Different inputs → different outputs (uniqueness).
        let g1 = make_guid(1);
        let g2 = make_guid(2);
        let g100 = make_guid(100);
        assert_ne!(g1, g2);
        assert_ne!(g1, g100);
        assert_ne!(g2, g100);
    }

    #[test]
    fn step_emits_omniclass_classification_when_present() {
        use super::super::entities::{Classification, ClassificationItem, ClassificationSource};
        let model = IfcModel {
            project_name: Some("ClassifiedDemo".into()),
            description: None,
            entities: Vec::new(),
            classifications: vec![Classification {
                source: ClassificationSource::OmniClass,
                edition: Some("2012".into()),
                items: vec![
                    ClassificationItem {
                        code: "23.45.12.34".into(),
                        name: Some("Example Product".into()),
                    },
                    ClassificationItem {
                        code: "23.45.12.35".into(),
                        name: None,
                    },
                ],
            }],
            units: Vec::new(),
        };
        let s = write_step(&model);
        assert!(
            s.contains("IFCCLASSIFICATION("),
            "classification entity missing"
        );
        assert!(s.contains("'OmniClass'"), "OmniClass source missing");
        assert!(s.contains("'2012'"), "edition 2012 missing");
        assert!(
            s.matches("IFCCLASSIFICATIONREFERENCE(").count() == 2,
            "expected two classification references (one per item)"
        );
        assert!(s.contains("'23.45.12.34'"), "first code missing");
        assert!(s.contains("'23.45.12.35'"), "second code missing");
        assert!(s.contains("'Example Product'"), "item name missing");
        assert!(
            s.matches("IFCRELASSOCIATESCLASSIFICATION(").count() == 2,
            "expected one association rel per reference"
        );
    }

    #[test]
    fn step_omits_classification_entities_when_empty() {
        // Model with no classifications must NOT emit classification
        // entities. Guards against a regression where the writer
        // emits empty IfcClassification / IfcRelAssociates entities.
        let model = IfcModel::default();
        let s = write_step(&model);
        assert!(
            !s.contains("IFCCLASSIFICATION("),
            "should not emit IfcClassification when model.classifications is empty"
        );
        assert!(
            !s.contains("IFCCLASSIFICATIONREFERENCE("),
            "should not emit IfcClassificationReference when model has no classifications"
        );
        assert!(
            !s.contains("IFCRELASSOCIATESCLASSIFICATION("),
            "should not emit IfcRelAssociatesClassification when model has no classifications"
        );
    }

    #[test]
    fn step_guids_are_unique_across_entities() {
        // The writer assigns each entity a unique GUID by index; the
        // STEP output should therefore contain no duplicate GUIDs.
        // We grep for '0rvtrs' (our prefix) and check uniqueness.
        let model = IfcModel::default();
        let s = write_step(&model);
        let guids: Vec<_> = s
            .split("'0rvtrs")
            .skip(1)
            .filter_map(|chunk| chunk.split('\'').next())
            .collect();
        let mut seen = std::collections::HashSet::new();
        for g in &guids {
            assert!(seen.insert(*g), "duplicate IFC GUID in output: 0rvtrs{g}");
        }
        assert!(
            guids.len() >= 7,
            "expected ≥7 GUIDs (project+site+building+storey+3 rel-aggregates), got {}",
            guids.len()
        );
    }
}
