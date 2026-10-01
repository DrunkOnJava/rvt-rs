//! RE-137 (probe): the nine classes of the reconnaissance document's Q4
//! addendum, read with the page-stripped schema grammar (#154).
//!
//! The addendum (2026-04-19) called the `u16` between a class's name and its
//! field count an "ancestor class reference", distinct from the class's parent,
//! and listed nine classes whose reference resolved to another class. That
//! read predates the checksum-page strip of `Formats/Latest` (#410) and the
//! grammar of STE1200's report, in which the same `u16` is the class's base
//! reference. This prints, for each of the nine, the base class the grammar
//! gives, and whether it is the class the addendum named; and for every class
//! of the file, whether a base reference resolves.
//!
//! Usage:
//!   cargo run --profile ci --example probe_re137_q4_base_classes -- MODEL.rvt

use rvt::RevitFile;

/// The addendum's table: a class and the class its reference resolved to.
const Q4: &[(&str, &str)] = &[
    ("APIVSTAMacroElemTracking", "ADocWarnings"),
    ("HostObjAttr", "APIVSTAMacroElem"),
    ("AnalyticalLineAutoConnectData", "ConnectorPositionModifier"),
    ("AnalyticalPanelPatternHelper", "ATFProvenanceBaseCell"),
    ("ReferencePointGridNetTrackerCell", "ATFProvenanceBaseCell"),
    ("AnalyticalSlabAdjustmentGStep", "AbsCurveGStep"),
    ("AppearanceAssetElemGroupHelper", "ATFProvenanceBaseCell"),
    ("ArcWallRectOpeningGStep", "AbsCurveGStep"),
    ("AreaMeasureCurveData", "Cell"),
];

fn main() -> rvt::Result<()> {
    let path = std::env::args().nth(1).expect("usage: MODEL.rvt");
    let mut rf = RevitFile::open(&path)?;
    let version = rf.basic_file_info()?.version;
    let classes = rf.schema_classes()?;
    let (mut present, mut agree) = (0usize, 0usize);
    for (name, named) in Q4 {
        let Some(class) = classes.classes.iter().find(|class| class.name == *name) else {
            println!("{{\"class\":{name:?},\"present\":false}}");
            continue;
        };
        present += 1;
        let base = class
            .base
            .and_then(|tag| classes.by_tag(tag))
            .map(|base| base.name.as_str());
        agree += usize::from(base == Some(*named));
        println!(
            "{{\"class\":{name:?},\"tag\":{},\"base_tag\":{},\"base\":{},\"addendum_named\":{named:?},\"same\":{}}}",
            class.tag,
            class
                .base
                .map_or_else(|| "null".to_string(), |tag| tag.to_string()),
            base.map_or_else(|| "null".to_string(), |base| format!("{base:?}")),
            base == Some(*named)
        );
    }
    let with_base = classes
        .classes
        .iter()
        .filter(|class| class.base.is_some())
        .count();
    let resolving = classes
        .classes
        .iter()
        .filter(|class| class.base.and_then(|tag| classes.by_tag(tag)).is_some())
        .count();
    println!(
        "{{\"revit\":{version},\"classes\":{},\"stopped\":{},\"with_base\":{with_base},\"base_resolves\":{resolving},\"q4_present\":{present},\"q4_same_as_base\":{agree}}}",
        classes.classes.len(),
        classes
            .stopped
            .as_ref()
            .map_or_else(|| "null".to_string(), |why| format!("{why:?}")),
    );
    eprintln!(
        "Revit {version}: {agree} of {present} Q4 classes have the base the addendum named; {resolving} of {with_base} base references resolve"
    );
    Ok(())
}
