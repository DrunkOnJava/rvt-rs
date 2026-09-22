//! `rvt-schedule` — element and room schedules of a Revit file as CSV,
//! ready for Excel, Google Sheets or LibreOffice.
//!
//! Runs the document exporter and writes one row per decoded element (or
//! per room) with the columns documented in `rvt::ifc::schedule_csv`. Only
//! decoded values are written; unknowns are empty cells.
//!
//! ```bash
//! rvt-schedule model.rvt                       # writes model.elements.csv
//! rvt-schedule model.rvt --schedule rooms      # writes model.rooms.csv
//! rvt-schedule model.rvt --metric --excel -o schedule.csv
//! rvt-schedule model.rvt -o -                  # CSV to stdout
//! ```

use clap::{Parser, ValueEnum};
use rvt::RevitFile;
use rvt::ifc::schedule_csv::{CsvOptions, LengthUnit, elements_csv, rooms_csv};
use rvt::ifc::{Exporter, RvtDocExporter};
use std::io::Write;
use std::path::PathBuf;
use std::process::ExitCode;

#[derive(Parser, Debug)]
#[command(
    name = "rvt-schedule",
    version,
    about = "Export element and room schedules of a Revit file as CSV (Excel, Sheets, LibreOffice)",
    after_help = "Examples:\n  \
        rvt-schedule model.rvt                     writes model.elements.csv next to the input\n  \
        rvt-schedule model.rvt --schedule rooms    writes model.rooms.csv\n  \
        rvt-schedule model.rvt --metric --excel -o schedule.csv\n  \
        rvt-schedule model.rvt -o -                CSV to stdout"
)]
struct Cli {
    /// Revit file (.rvt / .rfa / .rte / .rft).
    #[arg(value_name = "INPUT")]
    input: PathBuf,

    /// Which schedule to write.
    #[arg(long, value_enum, default_value = "elements")]
    schedule: Kind,

    /// Output path, or `-` for stdout. Default: `<input>.elements.csv` /
    /// `<input>.rooms.csv` next to the input.
    #[arg(short = 'o', long)]
    output: Option<PathBuf>,

    /// Lengths in metres instead of feet (columns end in `_m`).
    #[arg(long)]
    metric: bool,

    /// Start the file with a UTF-8 byte-order mark so Excel on Windows reads
    /// non-ASCII names correctly.
    #[arg(long)]
    excel: bool,
}

#[derive(ValueEnum, Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    /// Every decoded building element: type, level, material, placement,
    /// size, host.
    Elements,
    /// Every room: number, name, level.
    Rooms,
}

impl Kind {
    fn as_str(self) -> &'static str {
        match self {
            Kind::Elements => "elements",
            Kind::Rooms => "rooms",
        }
    }

    fn noun(self) -> &'static str {
        match self {
            Kind::Elements => "element",
            Kind::Rooms => "room",
        }
    }
}

fn main() -> ExitCode {
    rvt::cli::exit_quietly_on_broken_pipe();
    match run(Cli::parse()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::from(1)
        }
    }
}

fn run(cli: Cli) -> Result<(), String> {
    // Open errors from the library already name the file.
    let mut rf = RevitFile::open(&cli.input).map_err(|e| e.to_string())?;
    let model = RvtDocExporter
        .export(&mut rf)
        .map_err(|e| format!("export {}: {e}", cli.input.display()))?;
    let options = CsvOptions {
        unit: if cli.metric {
            LengthUnit::Metres
        } else {
            LengthUnit::Feet
        },
        excel_bom: cli.excel,
    };
    let csv = match cli.schedule {
        Kind::Elements => elements_csv(&model, &options),
        Kind::Rooms => rooms_csv(&model, &options),
    };
    let rows = csv.matches("\r\n").count().saturating_sub(1);

    let output = cli.output.unwrap_or_else(|| {
        cli.input
            .with_extension(format!("{}.csv", cli.schedule.as_str()))
    });
    if output.as_os_str() == "-" {
        std::io::stdout()
            .write_all(csv.as_bytes())
            .map_err(|e| format!("write stdout: {e}"))?;
        return Ok(());
    }
    if output == cli.input {
        return Err(format!(
            "refusing to overwrite the input {}; pass -o <path>",
            cli.input.display()
        ));
    }
    std::fs::write(&output, csv.as_bytes())
        .map_err(|e| format!("write {}: {e}", output.display()))?;
    eprintln!(
        "rvt-schedule: wrote {rows} {} row{} to {}",
        cli.schedule.noun(),
        if rows == 1 { "" } else { "s" },
        output.display()
    );
    if rows == 0 {
        eprintln!(
            "rvt-schedule: no {} decoded from this file — `rvt-inspect {}` explains what was recovered",
            cli.schedule.as_str(),
            cli.input.display()
        );
    }
    Ok(())
}
