mod commands;
mod error;
mod paths;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![commands::app::app_info])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod tests {
    use super::paths;

    #[test]
    fn data_dir_is_in_home() {
        let dir = paths::data_dir().unwrap();
        assert!(dir.ends_with("RunningData"));
        assert_eq!(dir.parent(), dirs::home_dir().as_deref());
    }
}
