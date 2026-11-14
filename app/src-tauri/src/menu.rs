//! Native menu, after `ui/main_menu.py`. App-specific items are sent to the frontend as `menu`
//! events carrying the item id; the standard items are handled by the OS.

use tauri::{
    App, AppHandle, Emitter, Runtime,
    menu::{MenuBuilder, MenuEvent, MenuItemBuilder, SubmenuBuilder},
};
use tauri_plugin_opener::OpenerExt;

pub const MENU_EVENT: &str = "menu";
const HELP_URL: &str = "https://axelmichel.github.io/runningmate/";

pub fn install<R: Runtime>(app: &App<R>) -> tauri::Result<()> {
    let import = MenuItemBuilder::with_id("import", "Import TCX…")
        .accelerator("CmdOrCtrl+O")
        .build(app)?;
    let folder_sync = MenuItemBuilder::with_id("folder-sync", "Sync Folder…").build(app)?;
    let search = MenuItemBuilder::with_id("search", "Search")
        .accelerator("CmdOrCtrl+F")
        .build(app)?;
    let settings = MenuItemBuilder::with_id("settings", "Settings…")
        .accelerator("CmdOrCtrl+,")
        .build(app)?;
    let user = MenuItemBuilder::with_id("user", "User Profile…").build(app)?;
    let help = MenuItemBuilder::with_id("help", "RunningMate Help")
        .accelerator("F1")
        .build(app)?;

    let app_menu = SubmenuBuilder::new(app, "RunningMate")
        .about(None)
        .separator()
        .item(&settings)
        .item(&user)
        .separator()
        .services()
        .separator()
        .hide()
        .hide_others()
        .show_all()
        .separator()
        .quit()
        .build()?;
    let file = SubmenuBuilder::new(app, "File")
        .item(&import)
        .item(&folder_sync)
        .separator()
        .close_window()
        .build()?;
    // Without an Edit menu, copy and paste stop working in text fields on macOS.
    let edit = SubmenuBuilder::new(app, "Edit")
        .undo()
        .redo()
        .separator()
        .cut()
        .copy()
        .paste()
        .select_all()
        .build()?;
    let view = SubmenuBuilder::new(app, "View")
        .item(&search)
        .separator()
        .fullscreen()
        .build()?;
    let window = SubmenuBuilder::new(app, "Window")
        .minimize()
        .maximize()
        .build()?;
    let help_menu = SubmenuBuilder::new(app, "Help").item(&help).build()?;

    let menu = MenuBuilder::new(app)
        .items(&[&app_menu, &file, &edit, &view, &window, &help_menu])
        .build()?;
    app.set_menu(menu)?;
    app.on_menu_event(handle);
    Ok(())
}

fn handle<R: Runtime>(app: &AppHandle<R>, event: MenuEvent) {
    match event.id().as_ref() {
        "help" => {
            let _ = app.opener().open_url(HELP_URL, None::<&str>);
        }
        id @ ("import" | "folder-sync" | "search" | "settings" | "user") => {
            let _ = app.emit(MENU_EVENT, id);
        }
        _ => {}
    }
}
