//! Thin `#[tauri::command]` wrappers. Logic lives in the `db` modules and friends.
//! Commands use `async` so database work runs off the main thread.

pub mod activities;
pub mod app;
pub mod settings;
pub mod users;
