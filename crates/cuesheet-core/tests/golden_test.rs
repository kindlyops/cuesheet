//! Golden test: the rendered cuesheet.typ for the standard fixture must stay
//! byte-identical. Regenerate intentionally with:
//!
//! ```sh
//! BLESS=1 cargo test -p cuesheet-core --test golden_test
//! ```

mod fixture;

use std::io::Cursor;
use std::path::Path;

use cuesheet_core::parse::open_playlist;
use cuesheet_core::sheet::build_sheet;
use cuesheet_core::template::render_cuesheet;
use fixture::standard_fixture;

#[test]
fn standard_fixture_matches_golden_typ() {
    let zip = standard_fixture().build_zip();
    let opened = open_playlist(Cursor::new(zip)).unwrap();
    let built = build_sheet(&opened);
    let actual = render_cuesheet(&built.manifest);

    let golden_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/golden/standard.typ");
    if std::env::var("BLESS").is_ok() {
        std::fs::create_dir_all(golden_path.parent().unwrap()).unwrap();
        std::fs::write(&golden_path, &actual).unwrap();
        return;
    }

    let expected = std::fs::read_to_string(&golden_path)
        .expect("golden file missing — run with BLESS=1 to create it");
    assert_eq!(
        actual, expected,
        "rendered cuesheet.typ diverged from the golden file; \
         if the change is intentional re-bless with BLESS=1"
    );
}

#[test]
fn thumbnails_are_extracted_as_assets() {
    let zip = standard_fixture().build_zip();
    let opened = open_playlist(Cursor::new(zip)).unwrap();
    let built = build_sheet(&opened);

    assert_eq!(built.assets.len(), 1);
    assert!(built.assets.contains_key("thumbs/02.png"));
    assert!(built.manifest.cues[0].thumbnail.is_empty());
    assert_eq!(built.manifest.cues[1].thumbnail, "thumbs/02.png");
    // The template must reference the extracted asset path.
    let typ = render_cuesheet(&built.manifest);
    assert!(typ.contains("#image(\"thumbs/02.png\", width: 2cm)"));
}
