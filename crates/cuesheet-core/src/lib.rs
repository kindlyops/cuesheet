//! cuesheet-core: pure domain logic for turning a purple playlist export
//! into a Typst cuesheet and PDF.
//!
//! This crate is the hexagon: no GUI, no dialogs, no Tauri. I/O happens
//! through the ports in [`ports`]; adapters live at the edges
//! (`cuesheet-typst` for PDF compilation, the Tauri app for the GUI).

pub mod error;
pub mod model;
pub mod parse;
pub mod ports;
pub mod sheet;
pub mod sqlite;
pub mod template;

use std::path::Path;

pub use error::{CuesheetError, Result};
use ports::PdfCompiler;

/// Everything produced from one playlist: name (for the save-dialog default
/// filename), the Typst source, the thumbnail assets it references, and the
/// compiled PDF.
pub struct Generated {
    pub playlist_name: String,
    pub typ_source: String,
    pub assets: std::collections::BTreeMap<String, Vec<u8>>,
    pub pdf: Vec<u8>,
}

impl Generated {
    /// Default filename offered in the save dialog.
    pub fn suggested_filename(&self) -> String {
        let slug: String = self
            .playlist_name
            .chars()
            .map(|c| {
                if c.is_alphanumeric() || c == '-' || c == '_' {
                    c
                } else {
                    '-'
                }
            })
            .collect::<String>()
            .split('-')
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join("-");
        let slug = if slug.is_empty() {
            "playlist".to_string()
        } else {
            slug
        };
        format!("{slug}-cuesheet.pdf")
    }
}

/// The whole offline pipeline: parse -> build -> render -> compile.
pub fn generate_from_path(path: &Path, compiler: &dyn PdfCompiler) -> Result<Generated> {
    let opened = parse::open_playlist_file(path)?;
    generate_from_opened(&opened, compiler)
}

pub fn generate_from_opened(
    opened: &parse::OpenedPlaylist,
    compiler: &dyn PdfCompiler,
) -> Result<Generated> {
    let built = sheet::build_sheet(opened);
    let typ_source = template::render_cuesheet(&built.manifest);
    let pdf = compiler
        .compile(&typ_source, &built.assets)
        .map_err(CuesheetError::Compile)?;
    Ok(Generated {
        playlist_name: built.manifest.name,
        typ_source,
        assets: built.assets,
        pdf,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn suggested_filename_slugs_the_playlist_name() {
        let g = Generated {
            playlist_name: "My Great  Playlist! (2026)".to_string(),
            typ_source: String::new(),
            assets: Default::default(),
            pdf: Vec::new(),
        };
        assert_eq!(
            g.suggested_filename(),
            "My-Great-Playlist-2026-cuesheet.pdf"
        );
    }

    #[test]
    fn suggested_filename_handles_empty_name() {
        let g = Generated {
            playlist_name: "!!!".to_string(),
            typ_source: String::new(),
            assets: Default::default(),
            pdf: Vec::new(),
        };
        assert_eq!(g.suggested_filename(), "playlist-cuesheet.pdf");
    }
}
