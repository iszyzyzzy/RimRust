#![feature(stmt_expr_attributes)]

use std::fs;

use tokio::sync::Mutex;

use serde::{Deserialize, Serialize};

use tauri::{Emitter, Manager};

use tracing::{debug, error, info, warn};
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::{
    fmt::format::FmtSpan, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter, Layer,
};

mod background_task;
mod file;
mod func;
mod mods;
mod types;

use types::*;

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    info!(name = ?name,"test msg");
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // #[cfg(debug_assertions)]
    // let devtools = tauri_plugin_devtools::init();

    let builder = tauri::Builder::default().plugin(tauri_plugin_process::init());

    // #[cfg(debug_assertions)]
    // builder = builder.plugin(devtools);

    builder
        .plugin(tauri_plugin_single_instance::init(|_, _, _| {}))

        .plugin(tauri_plugin_dialog::init())
        // .plugin(tauri_plugin_log::Builder::new().build())
        .setup(|app| {
            //let handle = app.handle();
            std::panic::set_hook(Box::new(|info| {
                let backtrace = std::backtrace::Backtrace::force_capture();
                let payload = info.payload();
                let payload = if let Some(s) = payload.downcast_ref::<&str>() {
                    s
                } else if let Some(s) = payload.downcast_ref::<String>() {
                    &s[..]
                } else {
                    "Box<Any>"
                };
                error!("thread panic! \n {:#?} \n {:#?} \n {:#?}", payload, info, backtrace);
                //handle.emit("panic", format!("thread panic! \n {:#?} \n {:#?} \n {:#?}", payload, info, backtrace));
            }));

            // 初始化 tracing
            let app_data_path = app
                .path()
                .app_data_dir()
                .unwrap()
                .to_string_lossy()
                .to_string();

            if fs::exists(format!("{}/logs/rimrust.log", app_data_path)).unwrap() {
                if let Err(e) = fs::rename(
                    format!("{}/logs/rimrust.log", app_data_path),
                    format!("{}/logs/rimrust.log.1", app_data_path),
                ) {
                    warn!("无法重命名日志文件: {}", e);
                }
            };
            if fs::exists(format!("{}/logs/rimrust.warn.log", app_data_path)).unwrap() {
                if let Err(e) = fs::rename(
                    format!("{}/logs/rimrust.warn.log", app_data_path),
                    format!("{}/logs/rimrust.warn.log.1", app_data_path),
                ) {
                    warn!("无法重命名日志文件: {}", e);
                }
            };
            let file_appender = RollingFileAppender::builder()
                .rotation(Rotation::NEVER)
                .filename_suffix("rimrust.log")
                .build(
                    format!("{}/logs", app_data_path),
                )
                .expect("无法创建日志文件");
            let file_layer = tracing_subscriber::fmt::layer()
                .with_ansi(false)
                .with_file(true)
                .with_line_number(true)
                .with_thread_ids(true)
                .with_thread_names(true)
                .with_target(false)
                // 记录异步操作的开始和结束
                .with_span_events(FmtSpan::CLOSE)
                .with_writer(file_appender);
                // .with_filter(EnvFilter::from_default_env().add_directive(tracing::Level::INFO.into()));
                // .init();
            let file_warn_appender = RollingFileAppender::builder()
            .rotation(Rotation::NEVER)
            .filename_suffix("rimrust.warn.log")
            .build(
                format!("{}/logs", app_data_path),
            )
            .expect("无法创建日志文件");
            let file_warn_layer = tracing_subscriber::fmt::layer()
            .with_ansi(false)
            .with_file(true)
            .with_line_number(true)
            .with_thread_ids(true)
            .with_thread_names(true)
            .with_target(true)
            .with_writer(file_warn_appender);

            let console_layer = tracing_subscriber::fmt::layer()
                .with_ansi(true)
                .with_target(true)
                .with_thread_names(true)
                .pretty();
                // .with_filter(EnvFilter::from_default_env().add_directive(tracing::Level::INFO.into()));

            let file_layer = file_layer.with_filter(EnvFilter::new("rimrust_lib=debug"));
            //let file_layer = file_layer.with_filter(EnvFilter::from_default_env().add_directive(tracing::Level::INFO.into()));

            let console_layer = console_layer.with_filter(EnvFilter::new("rimrust_lib=info"));
            //let console_layer = console_layer.with_filter(EnvFilter::from_default_env().add_directive(tracing::Level::INFO.into()));

            // let file_warn_layer = file_warn_layer.with_filter(EnvFilter::from_default_env().add_directive(tracing::Level::WARN.into()));
            let file_warn_layer = file_warn_layer.with_filter(EnvFilter::new("rimrust_lib=warn"));

            let (file_layer, console_layer) = if cfg!(debug_assertions) {
                (Some(file_layer),Some(console_layer))
            } else {
                (None,None)
            };

            match tracing_subscriber::registry().with(console_layer).with(file_layer).with(file_warn_layer).try_init() {
                Ok(_) => info!("tracing 初始化成功"),
                Err(e) => info!("tracing 初始化失败: {}, 应该是tauri已经启动了tracing", e),
            }
            tauri::async_runtime::block_on(async {
                let handle = app.handle();
                handle.emit("start-loading", "读取配置文件").unwrap();
                info!("读取配置文件 app_data_path: {}", app_data_path);
                let app_data_path = app
                    .path()
                    .app_data_dir()
                    .unwrap()
                    .to_string_lossy()
                    .to_string();
                if !fs::exists(app_data_path.clone()).unwrap() {
                    fs::create_dir_all(app_data_path.clone()).unwrap();
                }
                let app_config = AppConfig::load(app_data_path.clone());
                let background_worker = app_config.backgroud_worker;
                app.manage(Mutex::new(app_config));
                let task_manager = Mutex::new(background_task::TaskManager::new(handle.clone(), background_worker));
                app.manage(task_manager);
                handle.emit("end-loading", ()).unwrap();

                Ok(())
            })
        })
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            greet,
            config_get,
            config_set,
            init_mission,
            func::base_list_get,
            func::mod_set_enable,
            func::mod_set_display_name,
            func::mod_change_order,
            func::mod_refresh_all,
            func::group_create,
            func::group_delete,
            func::group_add_object,
            func::group_remove_object,
            func::group_rename,
            func::group_set_enable,
            func::group_change_order,
            func::xml_load_file,
            func::xml_load_from_config,
            func::xml_save_file,
            func::xml_save_to_config,
            func::sort_mods,
            func::scan_err,
            func::scan_err_ignore_add,
            func::scan_err_ignore_remove,
            func::tran_unconfirmed_get,
            func::tran_comfirm,
            func::tran_match_get,
            func::tran_remove,
            func::tran_rematch_all,
            func::tran_user_ignore_add,
            func::tran_user_ignore_remove,
            func::tran_package_get,
            func::tran_package_add,
            func::tran_package_remove,
            func::tran_custom_calc,
            func::search_mod,
            func::steamdb_get_by_package_id,
            func::translate,
            func::save_mata_data_get,
            func::sort_set_user_custom_order,
            func::sort_remove_user_custom_order,
            func::sort_get_user_custom_order,
            func::find_preview_image,
        ])
        .build(tauri::generate_context!())
        .expect("error while running tauri application")
        .run(move |_app_handle, _event| {
            tauri::async_runtime::block_on(async move {
                match &_event {
                    tauri::RunEvent::ExitRequested { .. } => {
                        info!("窗口关闭，保存数据");
                        _app_handle.emit("start-loading", "正在保存").unwrap();
                        let app_config_state = _app_handle.state::<Mutex<AppConfig>>();
                        let app_config = app_config_state.lock().await;
                        app_config.save();
                        let base_list = _app_handle.state::<mods::BaseList>();
                        base_list.save(app_config.app_data_path.clone()).await;
                        info!("保存数据完成");
                        info!("Bye~");
                        _app_handle.emit("end-loading", ()).unwrap();
                    }
                    _ => {}
                }
            });
        });
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct AppConfig {
    app_data_path: String,
    game_config_path: String,
    steam_mods_path: String,
    game_path: String,
    game_version: Version,
    community_rules_update_path: String,
    steam_db_update_path: String,
    prefer_language: String,
    fallback_language: String,
    use_advance_search: bool,
    backgroud_worker: usize,
    main_worker: usize,
    proxy: Option<String>,
    data_init: bool,
}

impl AppConfig {
    fn new(app_data_path: String) -> Self {
        let handle = tokio::runtime::Handle::current();
        let main_worker = handle.metrics().num_workers();
        Self {
            app_data_path,
            game_config_path: "".to_string(),
            steam_mods_path: "".to_string(),
            game_path: "".to_string(),
            game_version: Version::default(),
            community_rules_update_path: "https://raw.githubusercontent.com/RimSort/Community-Rules-Database/main/communityRules.json".to_string(),
            steam_db_update_path: "https://raw.githubusercontent.com/RimSort/Steam-Workshop-Database/main/steamDB.json".to_string(),
            prefer_language: "zh".to_string(),
            fallback_language: "en".to_string(),
            use_advance_search: false,
            backgroud_worker: 4, 
            // 貌似量太大有概率卡死windows(特征是鼠标移动不到任务栏，shift键按下后无法监测到其他按键，恢复后会弹一个windows安全中心的消息)
            // 原本tokio的默认值是cpu核心数（我这32），不过我试下来4也不怎么会影响性能，属于是windows文件系统上限就在这了
            main_worker,
            proxy: None,
            data_init: false,
        }
    }
    fn from_load(
        app_config_load: AppConfigLoad,
        app_data_path: String,
        game_version: Version,
    ) -> Self {
        let handle = tokio::runtime::Handle::current();
        let main_worker = handle.metrics().num_workers();
        Self {
            app_data_path,
            game_config_path: app_config_load.game_config_path,
            steam_mods_path: app_config_load.steam_mods_path,
            game_path: app_config_load.game_path,
            game_version,
            community_rules_update_path: app_config_load.community_rules_update_path,
            steam_db_update_path: app_config_load.steam_db_update_path,
            prefer_language: app_config_load.prefer_language,
            fallback_language: app_config_load.fallback_language,
            use_advance_search: app_config_load.use_advance_search,
            backgroud_worker: main_worker,
            main_worker,
            data_init: false,
            proxy: if let Some(proxy) = app_config_load.proxy {
                if proxy == "" {
                    None
                } else {
                    Some(proxy)
                }
            } else {
                None
            },
        }
    }
    fn load(app_data_path: String) -> Self {
        if !fs::exists(format!("{}/app_config.json", app_data_path)).unwrap() {
            warn!("配置文件不存在，使用空配置");
            return Self::new(app_data_path);
        }
        let app_config: AppConfigLoad = serde_json::from_str(
            &fs::read_to_string(format!("{}/app_config.json", app_data_path)).unwrap(),
        )
        .unwrap();
        info!(path = ?format!("{}/app_config.json", app_data_path), "读取配置文件");
        let game_version = if app_config.game_path == "" {
            "*".to_string()
        } else {
            fs::read_to_string(format!("{}/version.txt", app_config.game_path))
                .unwrap()
                .trim()
                .to_string()
        };
        info!(game_version_raw = ?game_version, "读取游戏版本");
        Self::from_load(app_config, app_data_path, Version::new(game_version))
    }
    fn save(&self) {
        info!(path = ?format!("{}/app_config.json", self.app_data_path), "保存配置文件");
        debug!(config = ?self, "保存配置文件");
        fs::write(
            format!("{}/app_config.json", self.app_data_path),
            serde_json::to_string(self).unwrap(),
        )
        .unwrap();
    }
}

#[derive(Serialize, Deserialize, Clone)]
struct AppConfigLoad {
    #[serde(default = "String::new")]
    game_config_path: String,
    #[serde(default = "String::new")]
    steam_mods_path: String,
    #[serde(default = "String::new")]
    game_path: String,
    #[serde(default = "String::new")]
    community_rules_update_path: String,
    #[serde(default = "String::new")]
    steam_db_update_path: String,
    #[serde(default = "String::new")]
    prefer_language: String,
    #[serde(default = "String::new")]
    fallback_language: String,
    #[serde(default)]
    use_advance_search: bool,
    #[serde(default)]
    proxy: Option<String>,
    
}

#[tauri::command]
async fn config_get(state: tauri::State<'_, Mutex<AppConfig>>) -> Result<AppConfig, String> {
    let app_config = state.lock().await;
    Ok(app_config.clone())
}

#[tauri::command]
async fn config_set(
    state: tauri::State<'_, Mutex<AppConfig>>,
    app: tauri::AppHandle,
    app_config: AppConfigLoad,
) -> Result<(), String> {
    let mut app_config_managed = state.lock().await;
    let mut new_config = AppConfig::from_load(
        app_config,
        app_config_managed.app_data_path.clone(),
        app_config_managed.game_version.clone(),
    );
    let ver = if new_config.game_path == "" {
        "".to_string()
    } else {
        fs::read_to_string(format!("{}/version.txt", new_config.game_path))
            .unwrap()
            .trim()
            .to_string()
    };
    new_config.game_version = Version::new(ver);
    *app_config_managed = new_config;
    app_config_managed.save();
    info!("保存app_config，重启应用");
    app.restart();
}

// 前端启动完成后会来调用这个函数
// 主要是启动一些后台任务
#[tauri::command]
async fn init_mission(
    load_from_autosave: bool,
    task_manager: tauri::State<'_, Mutex<background_task::TaskManager>>,
    app_config: tauri::State<'_, Mutex<AppConfig>>,
    app: tauri::AppHandle,
) -> Result<(), String> {
    let init = app_config.lock().await.data_init;
    if !init { // 这个部分是只能执行一次的
        let task_manager_guard = task_manager.lock().await;
        let app_config_guard = app_config.lock().await;

        let load_app_data_path = if load_from_autosave {
            format!("{}/.auto_save", app_config_guard.app_data_path)
        } else {
            app_config_guard.app_data_path.clone()
        };
    
        info!("读取保存的数据&初始化");
        app.emit("start-loading", "读取保存的数据").unwrap();
        let mut base_list = mods::BaseList::default().with_app_handles(app.clone());
        debug!(app_data_path = ?app_config_guard.app_data_path, ?load_app_data_path, target_language_code = ?app_config_guard.prefer_language,version = ?app_config_guard.game_version);
        info!("初始化翻译数据");
        base_list
            .init_translation_mod_data(
                &app_config_guard.app_data_path,
                &app_config_guard.prefer_language,
                app_config_guard.game_version.clone(),
            )
            .await;
        info!("初始化搜索引擎");
        base_list
            .init_search_engine(app_config_guard.use_advance_search ,&app_config_guard.app_data_path, app_config_guard.game_version.clone(), app.clone())
            .await;
        info!("尝试从文件中恢复数据");
        base_list
            .recover_from_file(load_app_data_path.clone(), app_config_guard.game_version.clone())
            .await;
        app.manage(base_list);
        app.emit("end-loading", ()).unwrap();
    
        info!("初始化文件监听");
        drop(task_manager_guard);
        drop(app_config_guard);
        let mut file_watcher =
            file::watcher::FileWatcher::new(app.clone()).await;
        file_watcher.start().await.unwrap();
        app.manage(file_watcher);
    };

    info!("初始化启动任务");
    let task_manager_guard = task_manager.lock().await;
    let mut app_config_guard = app_config.lock().await;

    app_config_guard.data_init = true;

    if app_config_guard.game_path == ""
        || app_config_guard.steam_mods_path == ""
        || app_config_guard.game_config_path == ""
    {
        // 虽然我不确定这个地方会不会有问题，但是我还是加上了
        return Ok(());
    }
    let app_data_path = app_config_guard.app_data_path.clone();
    let community_rules_update_path = app_config_guard.community_rules_update_path.clone();
    let proxy = app_config_guard.proxy.clone();
    let comm_id = task_manager_guard
        .add_task(
            "更新社区规则数据库".to_string(),
            Box::new(move |_app, status| {
                Box::pin(async move {
                    let base_list = _app.state::<mods::BaseList>();
                    let mut status = status.lock().await;
                    base_list
                        .community_data
                        .update_community_mods_order(
                            app_data_path,
                            &mut status,
                            community_rules_update_path,
                            proxy,
                            Priority::LOW,
                        )
                        .await
                })
            }),
            None,
            None,
        )
        .await;
    let app_data_path = app_config_guard.app_data_path.clone();
    let app_data_path_clone = app_data_path.clone();
    let steam_db_update_path = app_config_guard.steam_db_update_path.clone();
    let proxy = app_config_guard.proxy.clone();
    let stdb_id = task_manager_guard
        .add_task(
            "更新创意工坊数据库".to_string(),
            Box::new(move |_app, status| {
                Box::pin(async move {
                    let base_list = _app.state::<mods::BaseList>();
                    let mut status = status.lock().await;
                    base_list
                        .community_data
                        .update_steam_db(
                            app_data_path_clone,
                            &mut status,
                            steam_db_update_path,
                            proxy,
                            Priority::LOW,
                        )
                        .await
                })
            }),
            None,
            None,
        )
        .await;
    crate::file::reader::generate_load_mission(
        &app_config_guard.steam_mods_path,
        &task_manager_guard,
        Some(vec![comm_id.clone(), stdb_id.clone()]),
    )
    .await?;

/*     for _ in 1..10 {
        crate::file::reader::generate_load_mission(
            &app_config_guard.steam_mods_path,
            &task_manager_guard,
            Some(vec![comm_id.clone(), stdb_id.clone()]),
        ).await?;
    } */

    crate::file::reader::generate_load_mission(
        &format!("{}\\Mods", app_config_guard.game_path),
        &task_manager_guard,
        Some(vec![comm_id.clone(), stdb_id.clone()]),
    )
    .await?;
    crate::file::reader::generate_load_mission(
        &format!("{}\\Data", app_config_guard.game_path),
        &task_manager_guard,
        Some(vec![comm_id, stdb_id]),
    )
    .await?;
    /*     task_manager
    .add_task(
        "强制刷新全部数据".to_string(),
        Box::new(move |_app, status| {
            Box::pin(async move {
                let base_list = _app.state::<mods::BaseList>();
                let app_config = _app.state::<Mutex<AppConfig>>();
                let app_config = app_config.lock().await;
                let mut status = status.lock().await;

                let res = base_list
                    .refresh_mods_data(&app_config, &mut status)
                    .await?;

            })
        }),
    )
    .await; */

    app.state::<mods::BaseList>().start_auto_save(app_data_path, &task_manager_guard).await;

    Ok(())
}

