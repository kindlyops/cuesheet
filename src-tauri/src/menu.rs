//! Native application menu: About Cuesheet, Check for Updates…, Quit.
//!
//! On macOS this becomes the app menu; on Windows/Linux it renders as a
//! regular window menu bar.

use tauri::menu::{AboutMetadataBuilder, Menu, MenuItem, PredefinedMenuItem, Submenu};
use tauri::{AppHandle, Runtime};
use tauri_plugin_dialog::{DialogExt, MessageDialogKind};

const CHECK_FOR_UPDATES_ID: &str = "check-for-updates";

pub fn install<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    let about = PredefinedMenuItem::about(
        app,
        Some("About Cuesheet"),
        Some(
            AboutMetadataBuilder::new()
                .name(Some("Cuesheet"))
                .version(Some(app.package_info().version.to_string()))
                .copyright(Some("© Kindly Ops, LLC. Apache-2.0."))
                .comments(Some(
                    "Generate a PDF cuesheet from a purple playlist export.",
                ))
                .build(),
        ),
    )?;
    let check_updates = MenuItem::with_id(
        app,
        CHECK_FOR_UPDATES_ID,
        "Check for Updates…",
        true,
        None::<&str>,
    )?;
    let quit = PredefinedMenuItem::quit(app, Some("Quit Cuesheet"))?;

    let app_menu = Submenu::with_items(
        app,
        "Cuesheet",
        true,
        &[
            &about,
            &PredefinedMenuItem::separator(app)?,
            &check_updates,
            &PredefinedMenuItem::separator(app)?,
            &quit,
        ],
    )?;

    let menu = Menu::with_items(app, &[&app_menu])?;
    app.set_menu(menu)?;

    app.on_menu_event(|app, event| {
        if event.id() == CHECK_FOR_UPDATES_ID {
            let app = app.clone();
            tauri::async_runtime::spawn(async move {
                check_for_updates(app).await;
            });
        }
    });

    Ok(())
}

async fn check_for_updates<R: Runtime>(app: AppHandle<R>) {
    use tauri_plugin_updater::UpdaterExt;

    let updater = match app.updater() {
        Ok(updater) => updater,
        Err(e) => {
            show_update_message(
                &app,
                MessageDialogKind::Warning,
                &format!("Updates aren't available in this build.\n\n{e}"),
            );
            return;
        }
    };

    match updater.check().await {
        Ok(Some(update)) => {
            let version = update.version.clone();
            let confirmed = app
                .dialog()
                .message(format!(
                    "Cuesheet {version} is available. Download and install it now?"
                ))
                .title("Update Available")
                .blocking_show();
            if !confirmed {
                return;
            }
            match update.download_and_install(|_, _| {}, || {}).await {
                Ok(()) => {
                    show_update_message(
                        &app,
                        MessageDialogKind::Info,
                        "The update is installed. Cuesheet will now restart.",
                    );
                    app.restart();
                }
                Err(e) => show_update_message(
                    &app,
                    MessageDialogKind::Error,
                    &format!("The update could not be installed.\n\n{e}"),
                ),
            }
        }
        Ok(None) => show_update_message(
            &app,
            MessageDialogKind::Info,
            "You're running the latest version of Cuesheet.",
        ),
        Err(e) => show_update_message(
            &app,
            MessageDialogKind::Warning,
            &format!("Could not check for updates.\n\n{e}"),
        ),
    }
}

fn show_update_message<R: Runtime>(app: &AppHandle<R>, kind: MessageDialogKind, message: &str) {
    app.dialog()
        .message(message)
        .kind(kind)
        .title("Cuesheet Updates")
        .show(|_| {});
}
