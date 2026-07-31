mod analysis;
mod api;
mod commands;
mod domain;
mod error;
mod scheduler;
mod secrets;
mod solar_time;
mod storage;
mod sync_service;

use std::{fs, path::PathBuf};

use storage::Repository;
use tauri::Manager;

pub struct AppState {
    repository: Repository,
}

fn application_directory(home: PathBuf) -> PathBuf {
    home.join(".comparingapp")
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let home = app.path().home_dir().map_err(|error| Box::<dyn std::error::Error>::from(error.to_string()))?;
            let directory = application_directory(home);
            fs::create_dir_all(&directory)?;
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                fs::set_permissions(&directory, fs::Permissions::from_mode(0o700))?;
            }
            let database_path = directory.join("comparison.db");
            let repository = Repository::new(&database_path)?;
            #[cfg(unix)]
            if database_path.exists() {
                use std::os::unix::fs::PermissionsExt;
                fs::set_permissions(&database_path, fs::Permissions::from_mode(0o600))?;
            }
            app.manage(AppState { repository });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_dashboard_snapshot,
            commands::save_api_configuration,
            commands::begin_oauth,
            commands::sync_now,
            commands::save_plant_schema,
        ])
        .run(tauri::generate_context!())
        .expect("Comparison App çalıştırılamadı");
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn data_directory_is_named_as_requested() {
        assert!(application_directory(PathBuf::from("/tmp/user")).ends_with(".comparingapp"));
    }
}
