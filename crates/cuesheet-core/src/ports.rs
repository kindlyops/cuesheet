//! Hexagonal ports: the seams between the pure domain and the outside world.

use std::collections::BTreeMap;
use std::path::Path;

use crate::error::Result;
use crate::parse::OpenedPlaylist;

/// Driven port: something that can open a playlist archive.
/// Production adapter: the ZIP/SQLite reader in `parse`. Tests substitute
/// in-memory fixtures.
pub trait PlaylistSource {
    fn load(&self, path: &Path) -> Result<OpenedPlaylist>;
}

/// Default adapter: read the playlist ZIP from the filesystem.
pub struct FilePlaylistSource;

impl PlaylistSource for FilePlaylistSource {
    fn load(&self, path: &Path) -> Result<OpenedPlaylist> {
        crate::parse::open_playlist_file(path)
    }
}

/// Driven port: something that can compile Typst source (plus referenced
/// asset files keyed by virtual path) into a PDF. Production adapter lives in
/// the `cuesheet-typst` crate; template tests skip compilation entirely.
pub trait PdfCompiler {
    fn compile(
        &self,
        main_source: &str,
        assets: &BTreeMap<String, Vec<u8>>,
    ) -> std::result::Result<Vec<u8>, String>;
}
