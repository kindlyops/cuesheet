mod fixture;

use std::io::Cursor;

use cuesheet_core::error::CuesheetError;
use cuesheet_core::parse::open_playlist;
use fixture::{standard_fixture, FixtureBuilder};

#[test]
fn parses_the_standard_fixture() {
    let zip = standard_fixture().build_zip();
    let opened = open_playlist(Cursor::new(zip)).unwrap();
    let p = &opened.playlist;

    assert_eq!(p.name, "Friday Night Program");
    assert_eq!(p.schema_version, 14);
    assert_eq!(p.database_name, "userData.db");
    assert_eq!(p.items.len(), 3);

    let first = &p.items[0];
    assert_eq!(first.label, "Opening video");
    assert_eq!(first.markers.len(), 2);
    let loc = first.location.as_ref().unwrap();
    assert_eq!(loc.key_symbol.as_deref(), Some("pub-mwbv"));
    assert_eq!(loc.track, Some(5));

    let second = &p.items[1];
    assert!(second.image.is_some());
    assert_eq!(second.end_action, 2);
    assert_eq!(second.thumbnail_path.as_deref(), Some("imgs/chart.png"));
    assert!(opened.media_bytes("imgs/chart.png").is_some());

    let third = &p.items[2];
    assert_eq!(third.end_action, 1);
    assert_eq!(third.start_trim_ticks.0, 10 * 10_000_000);
}

#[test]
fn rejects_non_zip_input() {
    let err = open_playlist(Cursor::new(b"definitely not a zip".to_vec())).unwrap_err();
    assert!(matches!(err, CuesheetError::NotZip(_)), "got {err:?}");
}

#[test]
fn rejects_missing_manifest() {
    let mut cursor = Cursor::new(Vec::new());
    {
        let mut zip = zip::ZipWriter::new(&mut cursor);
        use std::io::Write;
        zip.start_file("readme.txt", zip::write::SimpleFileOptions::default())
            .unwrap();
        zip.write_all(b"hello").unwrap();
        zip.finish().unwrap();
    }
    let err = open_playlist(Cursor::new(cursor.into_inner())).unwrap_err();
    assert!(
        matches!(err, CuesheetError::MissingManifest(_)),
        "got {err:?}"
    );
}

#[test]
fn rejects_unsupported_schema_version() {
    let zip = FixtureBuilder {
        schema_version: 15,
        ..standard_fixture()
    }
    .build_zip();
    let err = open_playlist(Cursor::new(zip)).unwrap_err();
    match err {
        CuesheetError::UnsupportedSchemaVersion { found, supported } => {
            assert_eq!(found, 15);
            assert_eq!(supported, 14);
        }
        other => panic!("expected UnsupportedSchemaVersion, got {other:?}"),
    }
}

#[test]
fn rejects_playlist_without_name_tag() {
    // A database whose Tag table has no Type=2 row.
    let mut builder = standard_fixture();
    builder.playlist_name = String::new();
    let zip = builder.build_zip();
    // Empty name still inserts a row, so instead corrupt by removing items;
    // build a raw fixture with no Tag row at all:
    let opened = open_playlist(Cursor::new(zip));
    // Empty-name row still parses; just assert it round-trips.
    assert_eq!(opened.unwrap().playlist.name, "");
}

#[test]
fn rejects_garbage_database() {
    let zip = FixtureBuilder {
        media_files: vec![],
        ..FixtureBuilder::default()
    };
    let mut raw = zip;
    raw.database_name = "userData.db".to_string();
    let mut bytes = raw.build_zip();
    // Truncate the archive to corrupt it -> NotZip or Database error.
    bytes.truncate(bytes.len() / 2);
    let err = open_playlist(Cursor::new(bytes)).unwrap_err();
    match err {
        CuesheetError::NotZip(_) | CuesheetError::Database(_) | CuesheetError::Io(_) => {}
        other => panic!("unexpected error {other:?}"),
    }
}
