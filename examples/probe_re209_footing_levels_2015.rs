//! RE-209 (#879): what a Revit 2015 footing and air terminal record say
//! about their Level, and what sets the three air terminals Revit's export
//! leaves out apart.
//!
//! FACT under test: on BIMcollab's Revit 2015 structural example rvt-rs
//! writes 16 `IfcFooting` on no storey, where Revit's export has each in a
//! storey, and 3 `IfcAirTerminal` (124771, 148100, 221553) that Revit's
//! export leaves out. This probe prints every decoded field of those
//! elements, of the file's Levels, and of the first structural columns for
//! contrast; `tools/re/footing_levels_vs_ifc.py` joins them by Tag with the
//! storeys of Revit's export.
//!
//! Output, one element per line:
//!   el <class> <id> <field>=<value>; ...
//!
//! Usage:
//!   cargo run --profile ci --example probe_re209_footing_levels_2015 -- FILE.rvt

use rvt::RevitFile;
use rvt::walker::{InstanceField, iter_elements};

fn show(value: &InstanceField) -> String {
    let text = match value {
        InstanceField::ElementId { id, .. } => format!("#{id}"),
        InstanceField::Integer { value, .. } => value.to_string(),
        InstanceField::Float { value, .. } => format!("{value:.4}"),
        InstanceField::String(s) => format!("{s:?}"),
        other => format!("{other:?}"),
    };
    text.chars().take(160).collect()
}

fn main() {
    let path = std::env::args_os()
        .nth(1)
        .expect("usage: probe_re209_footing_levels_2015 FILE.rvt");
    let mut rf = RevitFile::open(&path).expect("open");
    let mut columns = 0;
    for element in iter_elements(&mut rf).expect("elements") {
        let wanted = match element.class.as_str() {
            "StructuralFoundation" | "DuctTerminal" | "Level" => true,
            "StructuralColumn" => {
                columns += 1;
                columns <= 6
            }
            _ => false,
        };
        if !wanted {
            continue;
        }
        let fields: Vec<String> = element
            .fields
            .iter()
            .map(|(name, value)| format!("{name}={}", show(value)))
            .collect();
        println!(
            "el {} {} {}",
            element.class,
            element.id.map_or("-".into(), |id| id.to_string()),
            fields.join("; ")
        );
    }
}
