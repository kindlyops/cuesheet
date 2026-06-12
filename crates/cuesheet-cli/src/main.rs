//! Headless cuesheet generation — drives the same core the GUI uses.
//!
//! Usage: cuesheet <playlist-file> [-o output.pdf] [--typ output.typ]

use std::path::PathBuf;
use std::process::ExitCode;

fn main() -> ExitCode {
    let mut args = std::env::args().skip(1);
    let mut input: Option<PathBuf> = None;
    let mut output: Option<PathBuf> = None;
    let mut typ_output: Option<PathBuf> = None;

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "-o" | "--output" => output = args.next().map(PathBuf::from),
            "--typ" => typ_output = args.next().map(PathBuf::from),
            "-h" | "--help" => {
                eprintln!("usage: cuesheet <playlist-file> [-o output.pdf] [--typ output.typ]");
                return ExitCode::SUCCESS;
            }
            other => input = Some(PathBuf::from(other)),
        }
    }

    let Some(input) = input else {
        eprintln!("usage: cuesheet <playlist-file> [-o output.pdf] [--typ output.typ]");
        return ExitCode::from(2);
    };

    let generated =
        match cuesheet_core::generate_from_path(&input, cuesheet_typst::shared_compiler()) {
            Ok(g) => g,
            Err(e) => {
                eprintln!("error: {e}");
                return ExitCode::FAILURE;
            }
        };

    if let Some(typ_path) = typ_output {
        if let Err(e) = std::fs::write(&typ_path, &generated.typ_source) {
            eprintln!("error writing {}: {e}", typ_path.display());
            return ExitCode::FAILURE;
        }
        eprintln!("wrote {}", typ_path.display());
    }

    let out_path = output.unwrap_or_else(|| PathBuf::from(generated.suggested_filename()));
    if let Err(e) = std::fs::write(&out_path, &generated.pdf) {
        eprintln!("error writing {}: {e}", out_path.display());
        return ExitCode::FAILURE;
    }
    eprintln!(
        "wrote {} ({} cues from \"{}\")",
        out_path.display(),
        generated
            .typ_source
            .matches("table.hline(stroke: 0.3pt")
            .count(),
        generated.playlist_name
    );
    ExitCode::SUCCESS
}
