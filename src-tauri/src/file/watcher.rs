use notify_debouncer_full::notify::{
    recommended_watcher, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher,
};
use std::path::Path;
use tauri::{AppHandle, Manager};
use tokio::sync::{broadcast, Mutex};
use tracing::{debug, info, warn};

use crate::types::*;

pub struct FileWatcher {
    app: AppHandle,
    shutdown_tx: broadcast::Sender<()>,
    steam_watcher: Option<RecommendedWatcher>,
    local_watcher: Option<RecommendedWatcher>,
}

impl FileWatcher {
    pub async fn new(app: AppHandle) -> Self {
        let (shutdown_tx, _) = broadcast::channel(1);

        Self {
            app,
            shutdown_tx,
            steam_watcher: None,
            local_watcher: None,
        }
    }
    async fn handle_about_xml_event(app: &AppHandle, event: Event, source: &str) {
        debug!(?event, "handle_about_xml_event");
        if !event
            .paths
            .iter()
            .any(|p| p.file_name().map_or(false, |name| name == "About.xml"))
        {
            return;
        }

        match event.kind {
            EventKind::Create(_) | EventKind::Modify(_) => {
                
                let mod_path = event
                    .paths
                    .iter()
                    .filter_map(|p| p.parent().and_then(|p| p.parent()))
                    .next()
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_default();
                if mod_path == "" {
                    warn!("About.xml的父目录识别异常");
                    return;
                }
                let mod_folder_name = mod_path.split('\\').last().unwrap();

                info!(?mod_path, "检测到{} mod变化", source);
                debug!(?event, ?mod_path, ?mod_folder_name);

                let task_manager = app.state::<Mutex<crate::background_task::TaskManager>>();
                let task_manager = task_manager.lock().await;

                task_manager
                    .add_task(
                        format!("加载{} mod metadata: {}", source, mod_folder_name),
                        Box::new(move |_app, status| {
                            Box::pin(async move {
                                let mut status = status.lock().await;
                                let base_list = _app.state::<crate::mods::BaseList>();
                                let steam_db = base_list.community_data.get_steam_db();
                                let mod_ =
                                    crate::file::reader::load_mod_from_path(&mod_path, &mut status, steam_db, Priority::LOW).await
                                        .unwrap();
                                base_list.check_and_add_mod(mod_, Priority::LOW).await;
                                Ok(())
                            })
                        }),
                        Some(10),
                        None,
                    )
                    .await;
            }
            _ => {}
        }
    }
    pub async fn start(&mut self) -> Result<(), String> {
        let app_ = self.app.clone();
        let app_config = app_.state::<Mutex<crate::AppConfig>>();
        let app_config = app_config.lock().await;
        if app_config.steam_mods_path == "" || app_config.game_path == "" {
            warn!("未设置游戏路径或steam mod路径，跳过监听文件变化");
            return Ok(());
        }
        let steam_mod_path = app_config.steam_mods_path.clone();
        let local_mod_path = format!("{}\\Mods", app_config.game_path);
        drop(app_config);
        drop(app_);
        info!(path1 = ?steam_mod_path,path2 = ?local_mod_path,"开始监听文件变化");

        let (tx_steam, mut rx_steam) = tokio::sync::mpsc::channel(100);
        let mut watcher_steam = recommended_watcher(move |res| {
            let _ = tx_steam.blocking_send(res);
        })
        .unwrap();
        watcher_steam
            .watch(Path::new(&steam_mod_path), RecursiveMode::Recursive)
            .unwrap();
        self.steam_watcher = Some(watcher_steam);

        let (tx_local, mut rx_local) = tokio::sync::mpsc::channel(100);
        let mut watcher_local = recommended_watcher(move |res| {
            let _ = tx_local.blocking_send(res);
        })
        .unwrap();
        watcher_local
            .watch(Path::new(&local_mod_path), RecursiveMode::Recursive)
            .unwrap();
        self.local_watcher = Some(watcher_local);

        // Steam mod watcher
        let app_s = self.app.clone();
        let mut shutdown_rx = self.shutdown_tx.subscribe();
        tauri::async_runtime::spawn(async move {
            loop {
                tokio::select! {
                    _ = shutdown_rx.recv() => {
                        break;
                    }
                    result = rx_steam.recv() => {
                        match result {
                            Some(Ok(event)) => {
                                Self::handle_about_xml_event(&app_s, event, "Steam").await;
                            }
                            Some(Err(_)) => {},
                            None => break,
                        }
                    }
                }
            }
        });

        // Local mod watcher
        let mut shutdown_rx = self.shutdown_tx.subscribe();
        let app_l = self.app.clone();
        tauri::async_runtime::spawn(async move {
            loop {
                tokio::select! {
                    _ = shutdown_rx.recv() => {
                        break;
                    }
                    result = rx_local.recv() => {
                        match result {
                            Some(Ok(event)) => {
                                Self::handle_about_xml_event(&app_l, event, "Local").await;
                            }
                            Some(Err(_)) => {},
                            None => break,
                        }
                    }
                }
            }
        });

        Ok(())
    }
}
