//! Purple playlist export parsing: ZIP -> manifest.json -> SQLite.
//!
//! Mirrors the extraction performed by vbs cmd/plt_parse.go (same tables and
//! fields), implemented over the pure-Rust SQLite reader in [`crate::sqlite`]
//! so the whole core compiles to wasm32.

use std::collections::HashMap;
use std::io::{Read, Seek};
use std::path::Path;

use serde::Deserialize;

use crate::error::{CuesheetError, Result};
use crate::model::{EmbeddedImage, Item, Location, Marker, Playlist, Ticks};
use crate::sqlite::{SqliteFile, Table, Value};

/// Lowest playlist schema version this port understands. vbs originally
/// targeted version 14.
pub const MIN_SCHEMA_VERSION: i64 = 14;

/// Highest playlist schema version this port understands. Newer purple
/// playlist exports (15, 16) only add tables and columns the parser ignores —
/// the column subset read here is unchanged from 14 — so they parse identically.
pub const MAX_SCHEMA_VERSION: i64 = 16;

#[derive(Deserialize)]
struct ManifestDoc {
    #[serde(rename = "userDataBackup")]
    user_data_backup: UserDataBackup,
}

#[derive(Deserialize)]
struct UserDataBackup {
    #[serde(rename = "schemaVersion")]
    schema_version: serde_json::Number,
    #[serde(rename = "databaseName", default)]
    database_name: String,
}

/// A playlist archive opened for reading: the parsed database plus access to
/// the embedded media files (thumbnails) by their FilePath.
#[derive(Debug)]
pub struct OpenedPlaylist {
    pub playlist: Playlist,
    media: HashMap<String, Vec<u8>>,
}

impl OpenedPlaylist {
    pub fn media_bytes(&self, file_path: &str) -> Option<&[u8]> {
        self.media.get(file_path).map(|v| v.as_slice())
    }
}

pub fn open_playlist_file(path: &Path) -> Result<OpenedPlaylist> {
    let file = std::fs::File::open(path)?;
    open_playlist(file)
}

/// Parses a playlist from any seekable byte source (file, memory buffer —
/// the latter is how the wasm build feeds browser-dropped files in).
pub fn open_playlist<R: Read + Seek>(reader: R) -> Result<OpenedPlaylist> {
    let mut archive =
        zip::ZipArchive::new(reader).map_err(|e| CuesheetError::NotZip(e.to_string()))?;

    let manifest_raw = read_entry(&mut archive, "manifest.json")
        .map_err(|e| CuesheetError::MissingManifest(e.to_string()))?;
    let manifest: ManifestDoc = serde_json::from_slice(&manifest_raw)
        .map_err(|e| CuesheetError::InvalidManifest(e.to_string()))?;

    if manifest.user_data_backup.database_name.is_empty() {
        return Err(CuesheetError::InvalidManifest(
            "manifest missing userDataBackup.databaseName".to_string(),
        ));
    }
    let schema_version = manifest
        .user_data_backup
        .schema_version
        .as_i64()
        .ok_or_else(|| {
            CuesheetError::InvalidManifest(
                "manifest userDataBackup.schemaVersion is not an integer".to_string(),
            )
        })?;
    if !(MIN_SCHEMA_VERSION..=MAX_SCHEMA_VERSION).contains(&schema_version) {
        return Err(CuesheetError::UnsupportedSchemaVersion {
            found: schema_version,
            min_supported: MIN_SCHEMA_VERSION,
            max_supported: MAX_SCHEMA_VERSION,
        });
    }

    let db_bytes = read_entry(&mut archive, &manifest.user_data_backup.database_name)
        .map_err(|e| CuesheetError::Database(e.to_string()))?;

    let mut playlist = parse_database(&db_bytes)?;
    playlist.schema_version = schema_version;
    playlist.database_name = manifest.user_data_backup.database_name;

    // Pull every other archive entry into the media map so thumbnails can be
    // resolved by their IndependentMedia.FilePath / ThumbnailFilePath.
    let mut media = HashMap::new();
    for i in 0..archive.len() {
        let mut entry = archive
            .by_index(i)
            .map_err(|e| CuesheetError::NotZip(e.to_string()))?;
        let name = entry.name().to_string();
        if name == "manifest.json" || name == playlist.database_name || name.ends_with('/') {
            continue;
        }
        let mut buf = Vec::with_capacity(entry.size() as usize);
        entry.read_to_end(&mut buf)?;
        media.insert(name, buf);
    }

    Ok(OpenedPlaylist { playlist, media })
}

fn read_entry<R: Read + Seek>(
    archive: &mut zip::ZipArchive<R>,
    name: &str,
) -> std::result::Result<Vec<u8>, Box<dyn std::error::Error>> {
    let mut entry = archive.by_name(name)?;
    let mut buf = Vec::with_capacity(entry.size() as usize);
    entry.read_to_end(&mut buf)?;
    Ok(buf)
}

fn get_i64(table: &Table, row: &[Value], column: &str) -> i64 {
    table.get(row, column).and_then(Value::as_i64).unwrap_or(0)
}

fn get_opt_i64(table: &Table, row: &[Value], column: &str) -> Option<i64> {
    table.get(row, column).and_then(Value::as_i64)
}

fn get_string(table: &Table, row: &[Value], column: &str) -> String {
    table
        .get(row, column)
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string()
}

fn get_opt_string(table: &Table, row: &[Value], column: &str) -> Option<String> {
    table
        .get(row, column)
        .and_then(Value::as_str)
        .map(str::to_string)
}

fn parse_database(db_bytes: &[u8]) -> Result<Playlist> {
    let file = SqliteFile::parse(db_bytes).map_err(CuesheetError::Database)?;

    let tags = file.read_table("Tag").map_err(CuesheetError::Database)?;
    let playlist_tag = tags
        .rows
        .iter()
        .find(|row| get_i64(&tags, row, "Type") == 2)
        .ok_or(CuesheetError::MissingPlaylistName)?;
    let name = get_string(&tags, playlist_tag, "Name");
    let playlist_tag_id = get_i64(&tags, playlist_tag, "TagId");

    // TagMap rows for the playlist tag, in Position order -> ordered item ids.
    let tag_map = file.read_table("TagMap").map_err(CuesheetError::Database)?;
    let mut ordered: Vec<(i64, i64)> = tag_map
        .rows
        .iter()
        .filter(|row| get_i64(&tag_map, row, "TagId") == playlist_tag_id)
        .map(|row| {
            (
                get_i64(&tag_map, row, "Position"),
                get_i64(&tag_map, row, "PlaylistItemId"),
            )
        })
        .collect();
    ordered.sort_by_key(|(position, _)| *position);

    let playlist_items = file
        .read_table("PlaylistItem")
        .map_err(CuesheetError::Database)?;
    let mut items: Vec<Item> = Vec::with_capacity(ordered.len());
    let mut by_id: HashMap<i64, usize> = HashMap::new();
    for (position, item_id) in ordered {
        let Some(row) = playlist_items
            .rows
            .iter()
            .find(|row| get_i64(&playlist_items, row, "PlaylistItemId") == item_id)
        else {
            continue;
        };
        by_id.insert(item_id, items.len());
        items.push(Item {
            position,
            playlist_item_id: item_id,
            label: get_string(&playlist_items, row, "Label"),
            start_trim_ticks: Ticks(get_i64(&playlist_items, row, "StartTrimOffsetTicks")),
            end_trim_ticks: Ticks(get_i64(&playlist_items, row, "EndTrimOffsetTicks")),
            end_action: get_i64(&playlist_items, row, "EndAction"),
            thumbnail_path: get_opt_string(&playlist_items, row, "ThumbnailFilePath"),
            ..Item::default()
        });
    }

    // Locations joined through PlaylistItemLocationMap.
    let locations = file
        .read_table("Location")
        .map_err(CuesheetError::Database)?;
    let location_by_id: HashMap<i64, &Vec<Value>> = locations
        .rows
        .iter()
        .map(|row| (get_i64(&locations, row, "LocationId"), row))
        .collect();
    let loc_map = file
        .read_table("PlaylistItemLocationMap")
        .map_err(CuesheetError::Database)?;
    for row in &loc_map.rows {
        let item_id = get_i64(&loc_map, row, "PlaylistItemId");
        let location_id = get_i64(&loc_map, row, "LocationId");
        let (Some(&idx), Some(loc_row)) = (by_id.get(&item_id), location_by_id.get(&location_id))
        else {
            continue;
        };
        items[idx].location = Some(Location {
            major_multimedia_type: get_i64(&loc_map, row, "MajorMultimediaType"),
            base_duration_ticks: Ticks(get_i64(&loc_map, row, "BaseDurationTicks")),
            book_number: get_opt_i64(&locations, loc_row, "BookNumber"),
            chapter_number: get_opt_i64(&locations, loc_row, "ChapterNumber"),
            document_id: get_opt_i64(&locations, loc_row, "DocumentId"),
            track: get_opt_i64(&locations, loc_row, "Track"),
            key_symbol: get_opt_string(&locations, loc_row, "KeySymbol"),
            meps_language: get_opt_i64(&locations, loc_row, "MepsLanguage"),
            location_type: get_i64(&locations, loc_row, "Type"),
        });
    }

    // Embedded images joined through PlaylistItemIndependentMediaMap.
    let independent = file
        .read_table("IndependentMedia")
        .map_err(CuesheetError::Database)?;
    let independent_by_id: HashMap<i64, &Vec<Value>> = independent
        .rows
        .iter()
        .map(|row| (get_i64(&independent, row, "IndependentMediaId"), row))
        .collect();
    let media_map = file
        .read_table("PlaylistItemIndependentMediaMap")
        .map_err(CuesheetError::Database)?;
    for row in &media_map.rows {
        let item_id = get_i64(&media_map, row, "PlaylistItemId");
        let media_id = get_i64(&media_map, row, "IndependentMediaId");
        let (Some(&idx), Some(media_row)) = (by_id.get(&item_id), independent_by_id.get(&media_id))
        else {
            continue;
        };
        items[idx].image = Some(EmbeddedImage {
            duration_ticks: Ticks(get_i64(&media_map, row, "DurationTicks")),
            original_filename: get_string(&independent, media_row, "OriginalFilename"),
            file_path: get_string(&independent, media_row, "FilePath"),
            mime_type: get_string(&independent, media_row, "MimeType"),
            hash: get_string(&independent, media_row, "Hash"),
        });
    }

    // Markers, ordered by (PlaylistItemId, StartTimeTicks).
    let markers = file
        .read_table("PlaylistItemMarker")
        .map_err(CuesheetError::Database)?;
    let mut marker_rows: Vec<&Vec<Value>> = markers.rows.iter().collect();
    marker_rows.sort_by_key(|row| {
        (
            get_i64(&markers, row, "PlaylistItemId"),
            get_i64(&markers, row, "StartTimeTicks"),
        )
    });
    for row in marker_rows {
        let item_id = get_i64(&markers, row, "PlaylistItemId");
        let Some(&idx) = by_id.get(&item_id) else {
            continue;
        };
        items[idx].markers.push(Marker {
            label: get_string(&markers, row, "Label"),
            start_time_ticks: Ticks(get_i64(&markers, row, "StartTimeTicks")),
            duration_ticks: Ticks(get_i64(&markers, row, "DurationTicks")),
            end_transition_duration_ticks: Ticks(get_i64(
                &markers,
                row,
                "EndTransitionDurationTicks",
            )),
        });
    }

    Ok(Playlist {
        name,
        schema_version: 0,
        database_name: String::new(),
        items,
    })
}
