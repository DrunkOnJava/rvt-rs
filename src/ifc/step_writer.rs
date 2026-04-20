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
                random_guid_stub(),
                project_name,
                quoted_or_dollar(&project_desc),
            ),
        );

        // Future: emit `model.entities` as IFCBUILDINGELEMENT et al.
        // here, wired back to the project via IFCRELAGGREGATES. The
        // shape is well-defined once the walker surfaces typed
        // `BuildingElement` values.
        let _ = (project_id, model);

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

/// Placeholder GUID — IFC requires 22-char compressed base64.
/// For v1 we use a deterministic stub; future work: proper UUID4 →
/// IfcGloballyUniqueId encoding.
fn random_guid_stub() -> String {
    "0rvtrsgeneratedguidnullx".chars().take(22).collect()
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
}
