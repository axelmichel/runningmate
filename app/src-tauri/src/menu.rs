//! Native menu, after `ui/main_menu.py`. App-specific items are sent to the frontend as `menu`
//! events carrying the item id; the standard items are handled by the OS. The menu is rebuilt
//! in the app language whenever the language setting changes.

use tauri::{
    App, AppHandle, Emitter, Manager, Runtime,
    menu::{Menu, MenuBuilder, MenuEvent, MenuItemBuilder, SubmenuBuilder},
};
use tauri_plugin_opener::OpenerExt;

use crate::settings;

pub const MENU_EVENT: &str = "menu";
const HELP_URL: &str = "https://axelmichel.github.io/runningmate/";

/// Menu texts in one language.
struct Labels {
    about: &'static str,
    settings: &'static str,
    user: &'static str,
    services: &'static str,
    hide: &'static str,
    hide_others: &'static str,
    show_all: &'static str,
    quit: &'static str,
    file: &'static str,
    sync: &'static str,
    close_window: &'static str,
    edit: &'static str,
    undo: &'static str,
    redo: &'static str,
    cut: &'static str,
    copy: &'static str,
    paste: &'static str,
    select_all: &'static str,
    view: &'static str,
    search: &'static str,
    health: &'static str,
    fullscreen: &'static str,
    window: &'static str,
    minimize: &'static str,
    zoom: &'static str,
    help: &'static str,
    help_item: &'static str,
}

const EN: Labels = Labels {
    about: "About RunningMate",
    settings: "Settings…",
    user: "User Profile…",
    services: "Services",
    hide: "Hide RunningMate",
    hide_others: "Hide Others",
    show_all: "Show All",
    quit: "Quit RunningMate",
    file: "File",
    sync: "Sync…",
    close_window: "Close Window",
    edit: "Edit",
    undo: "Undo",
    redo: "Redo",
    cut: "Cut",
    copy: "Copy",
    paste: "Paste",
    select_all: "Select All",
    view: "View",
    search: "Search",
    health: "Health Data",
    fullscreen: "Enter Full Screen",
    window: "Window",
    minimize: "Minimize",
    zoom: "Zoom",
    help: "Help",
    help_item: "RunningMate Help",
};

const DE: Labels = Labels {
    about: "Über RunningMate",
    settings: "Einstellungen…",
    user: "Benutzerprofil…",
    services: "Dienste",
    hide: "RunningMate ausblenden",
    hide_others: "Andere ausblenden",
    show_all: "Alle einblenden",
    quit: "RunningMate beenden",
    file: "Ablage",
    sync: "Synchronisieren…",
    close_window: "Fenster schließen",
    edit: "Bearbeiten",
    undo: "Widerrufen",
    redo: "Wiederholen",
    cut: "Ausschneiden",
    copy: "Kopieren",
    paste: "Einsetzen",
    select_all: "Alles auswählen",
    view: "Darstellung",
    search: "Suchen",
    health: "Gesundheitsdaten",
    fullscreen: "Vollbildmodus",
    window: "Fenster",
    minimize: "Im Dock ablegen",
    zoom: "Zoomen",
    help: "Hilfe",
    help_item: "RunningMate-Hilfe",
};

fn labels(language: &str) -> &'static Labels {
    if language == "de" { &DE } else { &EN }
}

fn build<R: Runtime, M: Manager<R>>(manager: &M, language: &str) -> tauri::Result<Menu<R>> {
    let l = labels(language);
    let item = |id: &str, text: &str, accelerator: Option<&str>| {
        let builder = MenuItemBuilder::with_id(id, text);
        match accelerator {
            Some(a) => builder.accelerator(a).build(manager),
            None => builder.build(manager),
        }
    };
    let about = item("about", l.about, None)?;
    let settings = item("settings", l.settings, Some("CmdOrCtrl+,"))?;
    let user = item("user", l.user, None)?;
    // One entry for every way of getting activities in or out (see the Connectors dialog).
    let sync = item("sync", l.sync, Some("CmdOrCtrl+O"))?;
    let search = item("search", l.search, Some("CmdOrCtrl+F"))?;
    let health = item("health", l.health, Some("CmdOrCtrl+Shift+H"))?;
    let help = item("help", l.help_item, Some("F1"))?;

    let app_menu = SubmenuBuilder::new(manager, "RunningMate")
        // Opens the app's own About dialog with the RunningMate logo.
        .item(&about)
        .separator()
        .item(&settings)
        .item(&user)
        .separator()
        .services_with_text(l.services)
        .separator()
        .hide_with_text(l.hide)
        .hide_others_with_text(l.hide_others)
        .show_all_with_text(l.show_all)
        .separator()
        .quit_with_text(l.quit)
        .build()?;
    let file = SubmenuBuilder::new(manager, l.file)
        .item(&sync)
        .separator()
        .close_window_with_text(l.close_window)
        .build()?;
    // Without an Edit menu, copy and paste stop working in text fields on macOS.
    let edit = SubmenuBuilder::new(manager, l.edit)
        .undo_with_text(l.undo)
        .redo_with_text(l.redo)
        .separator()
        .cut_with_text(l.cut)
        .copy_with_text(l.copy)
        .paste_with_text(l.paste)
        .select_all_with_text(l.select_all)
        .build()?;
    let view = SubmenuBuilder::new(manager, l.view)
        .item(&search)
        .item(&health)
        .separator()
        .fullscreen_with_text(l.fullscreen)
        .build()?;
    let window = SubmenuBuilder::new(manager, l.window)
        .minimize_with_text(l.minimize)
        .maximize_with_text(l.zoom)
        .build()?;
    let help_menu = SubmenuBuilder::new(manager, l.help).item(&help).build()?;

    MenuBuilder::new(manager)
        .items(&[&app_menu, &file, &edit, &view, &window, &help_menu])
        .build()
}

pub fn install<R: Runtime>(app: &App<R>) -> tauri::Result<()> {
    let language = settings::load().map(|s| s.language).unwrap_or_default();
    app.set_menu(build(app, &language)?)?;
    app.on_menu_event(handle);
    Ok(())
}

/// Rebuilds the menu after the language setting changed.
pub fn set_language<R: Runtime>(app: &AppHandle<R>, language: &str) -> tauri::Result<()> {
    app.set_menu(build(app, language)?)?;
    Ok(())
}

fn handle<R: Runtime>(app: &AppHandle<R>, event: MenuEvent) {
    match event.id().as_ref() {
        "help" => {
            let _ = app.opener().open_url(HELP_URL, None::<&str>);
        }
        id @ ("about" | "sync" | "search" | "health" | "settings" | "user") => {
            let _ = app.emit(MENU_EVENT, id);
        }
        _ => {}
    }
}
