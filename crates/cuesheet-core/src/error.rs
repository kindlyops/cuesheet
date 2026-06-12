use thiserror::Error;

/// Failure classes surfaced to the UI in plain language; the variant message
/// carries the underlying detail for the "details" disclosure.
#[derive(Debug, Error)]
pub enum CuesheetError {
    #[error("could not read the file: {0}")]
    Io(#[from] std::io::Error),

    #[error("this file is not a playlist archive (not a ZIP): {0}")]
    NotZip(String),

    #[error("the playlist is missing its manifest.json: {0}")]
    MissingManifest(String),

    #[error("the playlist manifest is invalid: {0}")]
    InvalidManifest(String),

    #[error("unsupported playlist schema version {found} (this app supports version {supported})")]
    UnsupportedSchemaVersion { found: i64, supported: i64 },

    #[error("the playlist database could not be read: {0}")]
    Database(String),

    #[error("the playlist has no name tag")]
    MissingPlaylistName,

    #[error("PDF compilation failed: {0}")]
    Compile(String),
}

pub type Result<T> = std::result::Result<T, CuesheetError>;
