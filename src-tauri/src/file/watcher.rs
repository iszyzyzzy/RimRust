use notify_debouncer_full::notify::{
    recommended_watcher, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher,
};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Manager};
use tokio::sync::{broadcast, Mutex};
use tracing::{debug, info, warn};

use crate::types::*;

pub struct FileWatcher {
    app: AppHandle,
    shutdown_tx: broadcast::Sender<()>,
    steam_watcher: Option<RecommendedWatcher>,
    local_watcher: Option<RecommendedWatcher>,
    debounce_map: Arc<Mutex<HashMap<String, Instant>>>,
}

impl FileWatcher {
    pub async fn new(app: AppHandle) -> Self {
        let (shutdown_tx, _) = broadcast::channel(1);

        Self {
            app,
            shutdown_tx,
            steam_watcher: None,
            local_watcher: None,
            debounce_map: Arc::new(Mutex::new(HashMap::new())),
        }
    }
    fn extract_mod_path(event: &Event) -> Option<String> {
        event
            .paths
            .iter()
            .find(|p| p.file_name().map_or(false, |name| name == "About.xml"))
            .and_then(|p| p.parent().and_then(|p| p.parent()))
            .map(|p| p.to_string_lossy().to_string())
    }

    async fn is_debounced(
        debounce_map: &Arc<Mutex<HashMap<String, Instant>>>,
        key: &str,
        debounce_window: Duration,
    ) -> bool {
        let mut guard = debounce_map.lock().await;
        let now = Instant::now();

        if let Some(last) = guard.get(key) {
            if now.duration_since(*last) < debounce_window {
                return true;
            }
        }

        guard.insert(key.to_string(), now);
        if guard.len() > 2048 {
            guard.retain(|_, t| now.duration_since(*t) < Duration::from_secs(30));
        }
        false
    }

    async fn handle_about_xml_event(
        app: &AppHandle,
        event: Event,
        source: &str,
        debounce_map: Arc<Mutex<HashMap<String, Instant>>>,
    ) {
        debug!(?event, "handle_about_xml_event");

        let is_target_event = matches!(
            event.kind,
            EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_)
        );
        if !is_target_event {
            return;
        }

        let Some(mod_path) = Self::extract_mod_path(&event) else {
            return;
        };

        if mod_path.is_empty() {
            warn!("About.xml的父目录识别异常");
            return;
        }

        let debounce_key = format!("{}:{}:{:?}", source, mod_path, event.kind);
        if Self::is_debounced(&debounce_map, &debounce_key, Duration::from_millis(1200)).await {
            debug!(key = ?debounce_key, "忽略抖动事件");
            return;
        }

        let mod_folder_name = mod_path.split('\\').last().unwrap_or_default().to_string();
        info!(?mod_path, "检测到{} mod变化", source);
        debug!(?event, ?mod_path, ?mod_folder_name);

        match event.kind {
            EventKind::Create(_) | EventKind::Modify(_) => {
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
                                let mod_ = match crate::file::reader::load_mod_from_path(
                                    &mod_path,
                                    &mut status,
                                    steam_db,
                                    Priority::LOW,
                                ).await {
                                    Ok(mod_) => mod_,
                                    Err(e) => {
                                        warn!(path = ?mod_path, error = ?e, "监听到文件更新但重新读取失败");
                                        return Err(e);
                                    }
                                };
                                base_list.check_and_add_mod(mod_, Priority::LOW).await;
                                Ok(())
                            })
                        }),
                        Some(10),
                        None,
                    )
                    .await;
            }
            EventKind::Remove(_) => {
                let task_manager = app.state::<Mutex<crate::background_task::TaskManager>>();
                let task_manager = task_manager.lock().await;

                task_manager
                    .add_task(
                        format!("清理{} 已删除mod: {}", source, mod_folder_name),
                        Box::new(move |_app, _status| {
                            Box::pin(async move {
                                let base_list = _app.state::<crate::mods::BaseList>();
                                let removed = base_list
                                    .remove_mod_by_path(&mod_path, Priority::LOW)
                                    .await;
                                info!(path = ?mod_path, removed = removed, "已清理删除的mod");
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
        let debounce_map = self.debounce_map.clone();
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
                                Self::handle_about_xml_event(&app_s, event, "Steam", debounce_map.clone()).await;
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
        let debounce_map = self.debounce_map.clone();
        tauri::async_runtime::spawn(async move {
            loop {
                tokio::select! {
                    _ = shutdown_rx.recv() => {
                        break;
                    }
                    result = rx_local.recv() => {
                        match result {
                            Some(Ok(event)) => {
                                Self::handle_about_xml_event(&app_l, event, "Local", debounce_map.clone()).await;
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
