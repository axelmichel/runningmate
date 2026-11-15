//! Thin `#[tauri::command]` wrappers. Logic lives in the `db` modules and friends.
//! Commands use `async` so database work runs off the main thread.

pub mod activities;
pub mod ai;
pub mod analysis;
pub mod app;
pub mod garmin;
pub mod goals;
pub mod import;
pub mod segments;
pub mod settings;
pub mod training;
pub mod users;
