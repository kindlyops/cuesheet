//! Cuesheet Tauri app: thin driving adapter over `cuesheet-core`.
//!
//! Commands receive a path (from the open dialog or a native drop event),
//! run the offline pipeline, hold the result in managed state, and write
//! the PDF wherever the save dialog pointed.

mod menu;

use std::path::Path;
use std::sync::Mutex;

use cuesheet_core::Generated;
use tauri::State;

/// The most recently generated cuesheet, kept until the user saves it
/// (or generates another).
pub struct AppState(pub Mutex<Option<Generated>>);

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GenerateResponse {
    pub playlist_name: String,
    pub suggested_filename: String,
    pub cue_count: usize,
}

fn generate_blocking(path: &Path) -> Result<(Generated, usize), String> {
    let opened = cuesheet_core::parse::open_playlist_file(path).map_err(|e| e.to_string())?;
    let cue_count = cuesheet_core::sheet::build_sheet(&opened).manifest.cues.len();
    let generated =
        cuesheet_core::generate_from_opened(&opened, cuesheet_typst::shared_compiler())
            .map_err(|e| e.to_string())?;
    Ok((generated, cue_count))
}

#[tauri::command]
async fn generate_cuesheet(
    path: String,
    state: State<'_, AppState>,
) -> Result<GenerateResponse, String> {
    let (generated, cue_count) =
        tauri::async_runtime::spawn_blocking(move || generate_blocking(Path::new(&path)))
            .await
            .map_err(|e| format!("internal error: {e}"))??;

    let response = GenerateResponse {
        playlist_name: generated.playlist_name.clone(),
        suggested_filename: generated.suggested_filename(),
        cue_count,
    };
    *state.0.lock().expect("state mutex poisoned") = Some(generated);
    Ok(response)
}

#[tauri::command]
async fn save_cuesheet(target_path: String, state: State<'_, AppState>) -> Result<(), String> {
    let pdf = {
        let guard = state.0.lock().expect("state mutex poisoned");
        match guard.as_ref() {
            Some(generated) => generated.pdf.clone(),
            None => return Err("No cuesheet has been generated yet.".to_string()),
        }
    };
    tauri::async_runtime::spawn_blocking(move || {
        std::fs::write(&target_path, &pdf)
            .map_err(|e| format!("Could not save the PDF to {target_path}: {e}"))
    })
    .await
    .map_err(|e| format!("internal error: {e}"))?
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_opener::init())
        // TODO: the updater `pubkey` in tauri.conf.json is an empty
        // placeholder until `tauri signer generate` keys are added as CI
        // secrets (docs/PLAN.md §6). An empty pubkey is harmless at startup;
        // "Check for Updates…" reports a friendly failure until it's set.
        .plugin(tauri_plugin_updater::Builder::new().build())
        .manage(AppState(Mutex::new(None)))
        .setup(|app| {
            menu::install(app.handle())?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![generate_cuesheet, save_cuesheet])
        .run(tauri::generate_context!())
        .expect("error while running Cuesheet");
}
