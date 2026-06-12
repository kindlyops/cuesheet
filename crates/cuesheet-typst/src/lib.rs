//! In-process Typst compilation over an in-memory file map — the production
//! adapter for the `PdfCompiler` port. No typst CLI, no filesystem root.

use std::collections::{BTreeMap, HashMap};
use std::sync::OnceLock;

use cuesheet_core::ports::PdfCompiler;
use typst::diag::{FileError, FileResult};
use typst::foundations::{Bytes, Datetime};
use typst::syntax::{FileId, Source, VirtualPath};
use typst::text::{Font, FontBook};
use typst::utils::LazyHash;
use typst::{Library, LibraryExt as _, World};

/// Compiles Typst source to PDF using the fonts embedded in typst-assets.
/// The template's Helvetica/Arial stack falls back to the embedded defaults
/// when those system fonts are unavailable, keeping output deterministic.
pub struct TypstCompiler {
    library: LazyHash<Library>,
    book: LazyHash<FontBook>,
    fonts: Vec<Font>,
}

impl Default for TypstCompiler {
    fn default() -> Self {
        Self::new()
    }
}

impl TypstCompiler {
    pub fn new() -> Self {
        let fonts: Vec<Font> = typst_assets::fonts()
            .flat_map(|data| {
                let bytes = Bytes::new(data);
                let count = ttf_face_count(data);
                (0..count)
                    .filter_map(move |i| Font::new(bytes.clone(), i))
                    .collect::<Vec<_>>()
            })
            .collect();
        let book = FontBook::from_fonts(&fonts);
        TypstCompiler {
            library: LazyHash::new(Library::default()),
            book: LazyHash::new(book),
            fonts,
        }
    }
}

/// Shared singleton: font parsing is the expensive part and is immutable.
pub fn shared_compiler() -> &'static TypstCompiler {
    static COMPILER: OnceLock<TypstCompiler> = OnceLock::new();
    COMPILER.get_or_init(TypstCompiler::new)
}

fn ttf_face_count(_data: &[u8]) -> u32 {
    // typst-assets ships single-face files.
    1
}

impl PdfCompiler for TypstCompiler {
    fn compile(
        &self,
        main_source: &str,
        assets: &BTreeMap<String, Vec<u8>>,
    ) -> Result<Vec<u8>, String> {
        let main_id = FileId::new(None, VirtualPath::new("/cuesheet.typ"));
        let mut files: HashMap<FileId, Bytes> = HashMap::new();
        for (path, bytes) in assets {
            let id = FileId::new(None, VirtualPath::new(format!("/{path}")));
            files.insert(id, Bytes::new(bytes.clone()));
        }

        let world = SheetWorld {
            compiler: self,
            main: Source::new(main_id, main_source.to_string()),
            files,
        };

        let result = typst::compile::<typst::layout::PagedDocument>(&world);
        let document = result.output.map_err(|errors| {
            errors
                .iter()
                .map(|e| e.message.to_string())
                .collect::<Vec<_>>()
                .join("; ")
        })?;

        typst_pdf::pdf(&document, &typst_pdf::PdfOptions::default()).map_err(|errors| {
            errors
                .iter()
                .map(|e| e.message.to_string())
                .collect::<Vec<_>>()
                .join("; ")
        })
    }
}

struct SheetWorld<'a> {
    compiler: &'a TypstCompiler,
    main: Source,
    files: HashMap<FileId, Bytes>,
}

impl World for SheetWorld<'_> {
    fn library(&self) -> &LazyHash<Library> {
        &self.compiler.library
    }

    fn book(&self) -> &LazyHash<FontBook> {
        &self.compiler.book
    }

    fn main(&self) -> FileId {
        self.main.id()
    }

    fn source(&self, id: FileId) -> FileResult<Source> {
        if id == self.main.id() {
            Ok(self.main.clone())
        } else {
            Err(FileError::NotFound(
                id.vpath().as_rootless_path().to_path_buf(),
            ))
        }
    }

    fn file(&self, id: FileId) -> FileResult<Bytes> {
        self.files
            .get(&id)
            .cloned()
            .ok_or_else(|| FileError::NotFound(id.vpath().as_rootless_path().to_path_buf()))
    }

    fn font(&self, index: usize) -> Option<Font> {
        self.compiler.fonts.get(index).cloned()
    }

    fn today(&self, _offset: Option<i64>) -> Option<Datetime> {
        // The cuesheet template never reads the date; a fixed value keeps
        // compilation deterministic.
        Datetime::from_ymd(2000, 1, 1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compiles_minimal_document() {
        let pdf = shared_compiler()
            .compile("Hello, cuesheet!", &BTreeMap::new())
            .unwrap();
        assert!(pdf.starts_with(b"%PDF-"));
    }

    #[test]
    fn reports_compile_errors() {
        let err = shared_compiler()
            .compile("#assert(false)", &BTreeMap::new())
            .unwrap_err();
        assert!(!err.is_empty());
    }
}
