//! Builds in-memory purple playlist ZIP fixtures: manifest.json + SQLite
//! database + embedded thumbnail files, matching the schema vbs parses.

use std::io::{Cursor, Write};

use rusqlite::Connection;
use zip::write::SimpleFileOptions;

/// Builds a valid 1x1 fox-orange PNG (correct CRCs and zlib framing) so
/// typst's image decoder accepts fixture thumbnails.
pub fn tiny_png() -> Vec<u8> {
    fn crc32(data: &[u8]) -> u32 {
        let mut crc = 0xFFFF_FFFFu32;
        for &b in data {
            crc ^= b as u32;
            for _ in 0..8 {
                crc = if crc & 1 != 0 {
                    (crc >> 1) ^ 0xEDB8_8320
                } else {
                    crc >> 1
                };
            }
        }
        !crc
    }

    fn chunk(out: &mut Vec<u8>, kind: &[u8; 4], body: &[u8]) {
        out.extend_from_slice(&(body.len() as u32).to_be_bytes());
        out.extend_from_slice(kind);
        out.extend_from_slice(body);
        let mut crc_input = kind.to_vec();
        crc_input.extend_from_slice(body);
        out.extend_from_slice(&crc32(&crc_input).to_be_bytes());
    }

    // Raw scanline: filter byte 0 + RGB pixel (fox orange #C95B0C).
    let raw = [0u8, 0xC9, 0x5B, 0x0C];
    // zlib stream with one stored (uncompressed) deflate block + adler32.
    let mut idat = vec![0x78, 0x01, 0x01];
    idat.extend_from_slice(&(raw.len() as u16).to_le_bytes());
    idat.extend_from_slice(&(!(raw.len() as u16)).to_le_bytes());
    idat.extend_from_slice(&raw);
    let (mut a, mut b) = (1u32, 0u32);
    for &byte in &raw {
        a = (a + byte as u32) % 65521;
        b = (b + a) % 65521;
    }
    idat.extend_from_slice(&((b << 16) | a).to_be_bytes());

    let mut png = vec![0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
    let ihdr = [
        0, 0, 0, 1, // width 1
        0, 0, 0, 1, // height 1
        8, 2, 0, 0, 0, // bit depth 8, color type 2 (RGB)
    ];
    chunk(&mut png, b"IHDR", &ihdr);
    chunk(&mut png, b"IDAT", &idat);
    chunk(&mut png, b"IEND", &[]);
    png
}

pub struct FixtureItem {
    pub label: String,
    pub start_trim_ticks: i64,
    pub end_trim_ticks: i64,
    pub end_action: i64,
    pub thumbnail_path: Option<String>,
    /// (key_symbol, track, base_duration_ticks) -> Location row
    pub location: Option<(String, i64, i64)>,
    /// (original_filename, file_path, duration_ticks) -> IndependentMedia row
    pub image: Option<(String, String, i64)>,
    /// (start_ticks, duration_ticks)
    pub markers: Vec<(i64, i64)>,
}

impl Default for FixtureItem {
    fn default() -> Self {
        FixtureItem {
            label: "Item".to_string(),
            start_trim_ticks: 0,
            end_trim_ticks: 0,
            end_action: 0,
            thumbnail_path: None,
            location: None,
            image: None,
            markers: Vec::new(),
        }
    }
}

pub struct FixtureBuilder {
    pub playlist_name: String,
    pub schema_version: i64,
    pub database_name: String,
    pub items: Vec<FixtureItem>,
    /// Extra files placed in the archive (path, bytes).
    pub media_files: Vec<(String, Vec<u8>)>,
}

impl Default for FixtureBuilder {
    fn default() -> Self {
        FixtureBuilder {
            playlist_name: "Test Playlist".to_string(),
            schema_version: 14,
            database_name: "userData.db".to_string(),
            items: Vec::new(),
            media_files: Vec::new(),
        }
    }
}

impl FixtureBuilder {
    pub fn build_zip(&self) -> Vec<u8> {
        let db = self.build_database();
        let mut cursor = Cursor::new(Vec::new());
        {
            let mut zip = zip::ZipWriter::new(&mut cursor);
            let opts = SimpleFileOptions::default();

            zip.start_file("manifest.json", opts).unwrap();
            zip.write_all(
                format!(
                    r#"{{"userDataBackup":{{"schemaVersion":{},"databaseName":"{}"}}}}"#,
                    self.schema_version, self.database_name
                )
                .as_bytes(),
            )
            .unwrap();

            zip.start_file(&self.database_name, opts).unwrap();
            zip.write_all(&db).unwrap();

            for (path, bytes) in &self.media_files {
                zip.start_file(path, opts).unwrap();
                zip.write_all(bytes).unwrap();
            }

            zip.finish().unwrap();
        }
        cursor.into_inner()
    }

    fn build_database(&self) -> Vec<u8> {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        {
            let conn = Connection::open(tmp.path()).unwrap();
            conn.execute_batch(
                "CREATE TABLE Tag (TagId INTEGER PRIMARY KEY, Type INTEGER, Name TEXT);
                 CREATE TABLE TagMap (TagMapId INTEGER PRIMARY KEY, TagId INTEGER,
                     PlaylistItemId INTEGER, Position INTEGER);
                 CREATE TABLE PlaylistItem (PlaylistItemId INTEGER PRIMARY KEY, Label TEXT,
                     StartTrimOffsetTicks INTEGER, EndTrimOffsetTicks INTEGER,
                     EndAction INTEGER, ThumbnailFilePath TEXT);
                 CREATE TABLE Location (LocationId INTEGER PRIMARY KEY, BookNumber INTEGER,
                     ChapterNumber INTEGER, DocumentId INTEGER, Track INTEGER,
                     KeySymbol TEXT, MepsLanguage INTEGER, Type INTEGER);
                 CREATE TABLE PlaylistItemLocationMap (PlaylistItemId INTEGER,
                     LocationId INTEGER, MajorMultimediaType INTEGER,
                     BaseDurationTicks INTEGER);
                 CREATE TABLE IndependentMedia (IndependentMediaId INTEGER PRIMARY KEY,
                     OriginalFilename TEXT, FilePath TEXT, MimeType TEXT, Hash TEXT);
                 CREATE TABLE PlaylistItemIndependentMediaMap (PlaylistItemId INTEGER,
                     IndependentMediaId INTEGER, DurationTicks INTEGER);
                 CREATE TABLE PlaylistItemMarker (PlaylistItemMarkerId INTEGER PRIMARY KEY,
                     PlaylistItemId INTEGER, Label TEXT, StartTimeTicks INTEGER,
                     DurationTicks INTEGER, EndTransitionDurationTicks INTEGER);",
            )
            .unwrap();

            conn.execute(
                "INSERT INTO Tag (TagId, Type, Name) VALUES (1, 2, ?1)",
                [&self.playlist_name],
            )
            .unwrap();

            for (i, item) in self.items.iter().enumerate() {
                let item_id = (i + 1) as i64;
                conn.execute(
                    "INSERT INTO PlaylistItem (PlaylistItemId, Label, StartTrimOffsetTicks,
                         EndTrimOffsetTicks, EndAction, ThumbnailFilePath)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                    rusqlite::params![
                        item_id,
                        item.label,
                        item.start_trim_ticks,
                        item.end_trim_ticks,
                        item.end_action,
                        item.thumbnail_path,
                    ],
                )
                .unwrap();
                conn.execute(
                    "INSERT INTO TagMap (TagId, PlaylistItemId, Position) VALUES (1, ?1, ?2)",
                    rusqlite::params![item_id, i as i64],
                )
                .unwrap();

                if let Some((key_symbol, track, base_ticks)) = &item.location {
                    conn.execute(
                        "INSERT INTO Location (LocationId, KeySymbol, Track, MepsLanguage, Type)
                         VALUES (?1, ?2, ?3, 0, 2)",
                        rusqlite::params![item_id, key_symbol, track],
                    )
                    .unwrap();
                    conn.execute(
                        "INSERT INTO PlaylistItemLocationMap
                             (PlaylistItemId, LocationId, MajorMultimediaType, BaseDurationTicks)
                         VALUES (?1, ?1, 2, ?2)",
                        rusqlite::params![item_id, base_ticks],
                    )
                    .unwrap();
                }

                if let Some((original, file_path, duration_ticks)) = &item.image {
                    conn.execute(
                        "INSERT INTO IndependentMedia
                             (IndependentMediaId, OriginalFilename, FilePath, MimeType, Hash)
                         VALUES (?1, ?2, ?3, 'image/png', 'hash')",
                        rusqlite::params![item_id, original, file_path],
                    )
                    .unwrap();
                    conn.execute(
                        "INSERT INTO PlaylistItemIndependentMediaMap
                             (PlaylistItemId, IndependentMediaId, DurationTicks)
                         VALUES (?1, ?1, ?2)",
                        rusqlite::params![item_id, duration_ticks],
                    )
                    .unwrap();
                }

                for (start, duration) in &item.markers {
                    conn.execute(
                        "INSERT INTO PlaylistItemMarker
                             (PlaylistItemId, Label, StartTimeTicks, DurationTicks,
                              EndTransitionDurationTicks)
                         VALUES (?1, '', ?2, ?3, 0)",
                        rusqlite::params![item_id, start, duration],
                    )
                    .unwrap();
                }
            }
        }
        std::fs::read(tmp.path()).unwrap()
    }
}

/// The standard fixture used by the golden tests: three cues exercising a
/// published video with markers, an embedded image with thumbnail, and a
/// trimmed video with a non-default end action.
pub fn standard_fixture() -> FixtureBuilder {
    const SEC: i64 = 10_000_000;
    FixtureBuilder {
        playlist_name: "Friday Night Program".to_string(),
        items: vec![
            FixtureItem {
                label: "Opening video".to_string(),
                location: Some(("pub-mwbv".to_string(), 5, 300 * SEC)),
                markers: vec![(0, 90 * SEC), (90 * SEC + SEC / 4, 30 * SEC)],
                ..FixtureItem::default()
            },
            FixtureItem {
                label: "Chart: [Section 2] *important*".to_string(),
                end_action: 2,
                thumbnail_path: Some("imgs/chart.png".to_string()),
                image: Some((
                    "chart.png".to_string(),
                    "imgs/chart.png".to_string(),
                    8 * SEC,
                )),
                ..FixtureItem::default()
            },
            FixtureItem {
                label: "Closing song".to_string(),
                start_trim_ticks: 10 * SEC,
                end_trim_ticks: 190 * SEC,
                end_action: 1,
                location: Some(("sjjm".to_string(), 151, 240 * SEC)),
                ..FixtureItem::default()
            },
        ],
        media_files: vec![("imgs/chart.png".to_string(), tiny_png())],
        ..FixtureBuilder::default()
    }
}
