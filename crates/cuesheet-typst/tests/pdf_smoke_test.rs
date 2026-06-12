//! End-to-end smoke test: fixture playlist -> typ -> real PDF bytes.
//! PDF output isn't byte-stable across typst versions, so assertions are
//! structural.

#[path = "../../cuesheet-core/tests/fixture/mod.rs"]
mod fixture;

use std::io::Cursor;

use cuesheet_core::parse::open_playlist;
use cuesheet_typst::shared_compiler;
use fixture::standard_fixture;

#[test]
fn generates_a_pdf_from_the_standard_fixture() {
    let zip = standard_fixture().build_zip();
    let opened = open_playlist(Cursor::new(zip)).unwrap();
    let generated = cuesheet_core::generate_from_opened(&opened, shared_compiler()).unwrap();

    assert!(generated.pdf.starts_with(b"%PDF-"), "not a PDF");
    assert!(generated.pdf.len() > 1_000, "suspiciously small PDF");
    assert_eq!(generated.playlist_name, "Friday Night Program");
    assert_eq!(
        generated.suggested_filename(),
        "Friday-Night-Program-cuesheet.pdf"
    );
}
