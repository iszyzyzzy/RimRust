use std::{fs, sync::Mutex};

use serde::{Deserialize, Serialize};

use tauri::{Emitter, Manager};

mod mods;

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let handle = app.handle();
            handle.emit("start-loading", "读取配置文件").unwrap();
            let app_data_path = app
                .path()
                .app_data_dir()
                .unwrap()
                .to_string_lossy()
                .to_string();
            if !fs::exists(app_data_path.clone()).unwrap() {
                fs::create_dir_all(app_data_path.clone()).unwrap();
            }
            if !fs::exists(format!("{}/app_config.json", app_data_path)).unwrap() {
                let app_config = AppConfig::new(app_data_path.clone());
                fs::write(
                    format!("{}/app_config.json", app_data_path),
                    serde_json::to_string(&app_config).unwrap(),
                )
                .unwrap();
            }
            let app_config: AppConfigLoad = serde_json::from_str(
                &fs::read_to_string(format!("{}/app_config.json", app_data_path)).unwrap(),
            )
            .unwrap();
            let app_config = AppConfig::from_load(app_config, app_data_path.clone());
            app.manage(Mutex::new(app_config));
            handle.emit("end-loading", ()).unwrap();
            handle.emit("start-loading", "读取保存的mod数据").unwrap();
            let mut base_list = mods::BaseList::default();
            base_list.recover_from_file(app_data_path.clone());
            app.manage(base_list);
            handle.emit("end-loading", ()).unwrap();
            Ok(())
        })
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![greet])
        .invoke_handler(tauri::generate_handler![get_appconfig, set_appconfig])
        .invoke_handler(tauri::generate_handler![mods::get_base_list, mods::set_enable_mod, mods::set_enable_mod_group])
        .build(tauri::generate_context!())
        .expect("error while running tauri application")
        .run(move |_app_handle, _event| match &_event {
            tauri::RunEvent::ExitRequested { .. } => {
                _app_handle.emit("start-loading", "正在保存").unwrap();
                let app_config_state = _app_handle.state::<Mutex<AppConfig>>();
                let app_config = app_config_state.lock().unwrap();
                app_config.save();
                let base_list_state = _app_handle.state::<mods::BaseList>();
                let base_list = base_list_state;
                base_list.save(app_config.app_data_path.clone());
                _app_handle.emit("end-loading", ()).unwrap();
            }
            _ => {}
        });
}

#[derive(Serialize, Deserialize)]
struct AppConfig {
    app_data_path: String,
    game_config_path: String,
    steam_mods_path: String,
    local_mods_path: String,
}

impl AppConfig {
    fn new(app_data_path: String) -> Self {
        Self {
            app_data_path,
            game_config_path: "".to_string(),
            steam_mods_path: "".to_string(),
            local_mods_path: "".to_string(),
        }
    }
    fn from_load(app_config_load: AppConfigLoad, app_data_path: String) -> Self {
        Self {
            app_data_path,
            game_config_path: app_config_load.game_config_path,
            steam_mods_path: app_config_load.steam_mods_path,
            local_mods_path: app_config_load.local_mods_path,
        }
    }
    fn save(&self) {
        fs::write(
            format!("{}/app_config.json", self.app_data_path),
            serde_json::to_string(self).unwrap(),
        )
        .unwrap();
    }
}

#[derive(Serialize, Deserialize)]
struct AppConfigLoad {
    game_config_path: String,
    steam_mods_path: String,
    local_mods_path: String,
}

#[tauri::command]
async fn get_appconfig(state: tauri::State<'_, Mutex<AppConfig>>) -> Result<AppConfigLoad, String> {
    let app_config = state.lock().unwrap();
    Ok(AppConfigLoad {
        game_config_path: app_config.game_config_path.clone(),
        steam_mods_path: app_config.steam_mods_path.clone(),
        local_mods_path: app_config.local_mods_path.clone(),
    })
}

#[tauri::command]
async fn set_appconfig(
    state: tauri::State<'_, Mutex<AppConfig>>,
    app_config: AppConfigLoad,
) -> Result<(), String> {
    let mut app_config_managed = state.lock().unwrap();
    app_config_managed.game_config_path = app_config.game_config_path.clone();
    app_config_managed.steam_mods_path = app_config.steam_mods_path.clone();
    app_config_managed.local_mods_path = app_config.local_mods_path.clone();
    Ok(())
}
