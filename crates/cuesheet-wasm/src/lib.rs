//! Browser adapter: the same core pipeline the desktop app and CLI use,
//! exposed to JavaScript. Everything runs client-side; nothing is uploaded.

use std::io::Cursor;

use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct GeneratedCuesheet {
    playlist_name: String,
    suggested_filename: String,
    pdf: Vec<u8>,
}

#[wasm_bindgen]
impl GeneratedCuesheet {
    #[wasm_bindgen(getter, js_name = playlistName)]
    pub fn playlist_name(&self) -> String {
        self.playlist_name.clone()
    }

    #[wasm_bindgen(getter, js_name = suggestedFilename)]
    pub fn suggested_filename(&self) -> String {
        self.suggested_filename.clone()
    }

    /// The PDF bytes, ready for a Blob download.
    #[wasm_bindgen(getter)]
    pub fn pdf(&self) -> Vec<u8> {
        self.pdf.clone()
    }
}

/// Generates a cuesheet PDF from playlist file bytes (as read from a dropped
/// File in the browser). Errors are user-facing strings.
#[wasm_bindgen]
pub fn generate(playlist_bytes: &[u8]) -> Result<GeneratedCuesheet, JsError> {
    let opened = cuesheet_core::parse::open_playlist(Cursor::new(playlist_bytes.to_vec()))
        .map_err(|e| JsError::new(&e.to_string()))?;
    let generated = cuesheet_core::generate_from_opened(&opened, cuesheet_typst::shared_compiler())
        .map_err(|e| JsError::new(&e.to_string()))?;
    Ok(GeneratedCuesheet {
        suggested_filename: generated.suggested_filename(),
        playlist_name: generated.playlist_name,
        pdf: generated.pdf,
    })
}
