//! Domain model mirroring the structures parsed by `vbs plt`
//! (kindlyops/vbs cmd/plt_parse.go).

/// Purple playlist databases store time as 100ns ticks.
pub const TICKS_PER_SECOND: i64 = 10_000_000;

/// A duration or position expressed in 100ns ticks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct Ticks(pub i64);

impl Ticks {
    pub fn as_secs_f64(self) -> f64 {
        self.0 as f64 / TICKS_PER_SECOND as f64
    }

    pub fn from_secs_f64(secs: f64) -> Self {
        Ticks((secs * TICKS_PER_SECOND as f64).round() as i64)
    }
}

/// What the player does when a cue finishes (PlaylistItem.EndAction).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EndAction {
    Continue,
    Stop,
    Freeze,
    Other(i64),
}

impl EndAction {
    pub fn from_code(code: i64) -> Self {
        match code {
            0 => EndAction::Continue,
            1 => EndAction::Stop,
            2 => EndAction::Freeze,
            other => EndAction::Other(other),
        }
    }

    pub fn code(self) -> i64 {
        match self {
            EndAction::Continue => 0,
            EndAction::Stop => 1,
            EndAction::Freeze => 2,
            EndAction::Other(c) => c,
        }
    }

    /// Label matching vbs `endActionLabel`.
    pub fn label(self) -> String {
        match self {
            EndAction::Continue => "continue".to_string(),
            EndAction::Stop => "stop".to_string(),
            EndAction::Freeze => "freeze".to_string(),
            EndAction::Other(c) => format!("code {c}"),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct Playlist {
    pub name: String,
    pub schema_version: i64,
    pub database_name: String,
    pub items: Vec<Item>,
}

#[derive(Debug, Clone, Default)]
pub struct Item {
    pub position: i64,
    pub playlist_item_id: i64,
    pub label: String,
    pub start_trim_ticks: Ticks,
    pub end_trim_ticks: Ticks,
    pub end_action: i64,
    pub thumbnail_path: Option<String>,
    pub location: Option<Location>,
    pub image: Option<EmbeddedImage>,
    pub markers: Vec<Marker>,
}

#[derive(Debug, Clone, Default)]
pub struct Location {
    pub major_multimedia_type: i64,
    pub base_duration_ticks: Ticks,
    pub book_number: Option<i64>,
    pub chapter_number: Option<i64>,
    pub document_id: Option<i64>,
    pub track: Option<i64>,
    pub key_symbol: Option<String>,
    pub meps_language: Option<i64>,
    pub location_type: i64,
}

#[derive(Debug, Clone, Default)]
pub struct EmbeddedImage {
    pub duration_ticks: Ticks,
    pub original_filename: String,
    pub file_path: String,
    pub mime_type: String,
    pub hash: String,
}

#[derive(Debug, Clone, Default)]
pub struct Marker {
    pub label: String,
    pub start_time_ticks: Ticks,
    pub duration_ticks: Ticks,
    pub end_transition_duration_ticks: Ticks,
}

/// The flattened data the Typst template consumes — the offline analog of the
/// vbs `buildManifest`.
#[derive(Debug, Clone, Default)]
pub struct SheetManifest {
    pub name: String,
    pub language_code: String,
    pub language_id: i64,
    pub resolution: String,
    pub cues: Vec<Cue>,
}

/// One table row — the offline analog of the vbs `cue`.
#[derive(Debug, Clone, Default)]
pub struct Cue {
    pub index: usize,
    pub label: String,
    /// Source identifier shown in the raw/monospace slot where vbs shows the
    /// cut clip filename (offline we have no cut clips).
    pub clip: String,
    /// Virtual path of the extracted thumbnail (key into the assets map),
    /// empty when the item has none.
    pub thumbnail: String,
    pub duration_sec: f64,
    pub end_action_raw: i64,
}
