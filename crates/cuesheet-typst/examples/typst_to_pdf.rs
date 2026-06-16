//! Compile an arbitrary Typst file to PDF with Cuesheet's bundled compiler.
//!
//! Handy for generating project docs (e.g. docs/signing-setup.typ) with the
//! same in-process Typst engine the app ships, without the Typst CLI.
//!
//! Usage: cargo run -p cuesheet-typst --example typst_to_pdf -- in.typ out.pdf

use std::collections::BTreeMap;

use cuesheet_core::ports::PdfCompiler;

fn main() -> std::process::ExitCode {
    let mut args = std::env::args().skip(1);
    let (Some(input), Some(output)) = (args.next(), args.next()) else {
        eprintln!("usage: typst_to_pdf <input.typ> <output.pdf>");
        return std::process::ExitCode::from(2);
    };

    let source = match std::fs::read_to_string(&input) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error reading {input}: {e}");
            return std::process::ExitCode::FAILURE;
        }
    };

    match cuesheet_typst::shared_compiler().compile(&source, &BTreeMap::new()) {
        Ok(pdf) => {
            if let Err(e) = std::fs::write(&output, pdf) {
                eprintln!("error writing {output}: {e}");
                return std::process::ExitCode::FAILURE;
            }
            eprintln!("wrote {output}");
            std::process::ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("compile failed: {e}");
            std::process::ExitCode::FAILURE
        }
    }
}
