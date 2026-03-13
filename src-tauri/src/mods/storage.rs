use ahash::RandomState;
use dashmap::DashMap;
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use tauri::Manager;
use std::{io::Read, sync::Arc};
use ahash::{HashMap, HashSet};
use tokio::sync::Mutex;
use tracing::{info, instrument, warn, debug};

use super::base_list::*;
use crate::types::*;

#[derive(Serialize, Deserialize, Debug, Clone, bincode::Decode, bincode::Encode)]
#[serde(rename_all = "camelCase")]
pub struct ModsGroupForSave {
    pub id: String,
    pub name: String,
    pub enabled: bool,
    pub mods: Vec<ModsGroupItemForSave>,
}

#[derive(Serialize, Deserialize, Debug, Clone, bincode::Decode, bincode::Encode)]
#[serde(rename_all = "camelCase")]
pub enum ModsGroupItemForSave {
    Mod(String),
    ModsGroup(String),
}

#[derive(Serialize, Deserialize, Debug, Clone, bincode::Decode, bincode::Encode)]
#[serde(rename_all = "camelCase")]
pub struct BaseListForSave {
    pub mods: Vec<ModInner>,
    pub mods_order: Vec<Id>,
    pub mods_groups: Vec<ModsGroupForSave>,
    pub mods_groups_order: Vec<Id>,
    pub user_custom_mods_order: HashMap<PackageId, HashSet<ModOrder>>,
    pub user_ignore_info: HashMap<Id, HashSet<super::scan::InfoType>>,
    pub translation_mod_data: HashMap<Id, super::TranslateModStatus>,
    pub auto_translate_cache: Vec<(String, super::translate::AutoTranslateResult)>,
}

impl BaseList {
    pub async fn recover_from_file(&mut self, app_data_path: String, version: Version) {
        // 直接假定了运行前结构体内没有数据，应该问题不大
        info!("base_list_recover_from_file");
        let start = std::time::Instant::now();

        let span_recover_community = tracing::info_span!("recover_community_data");
        let span_load_saved = tracing::info_span!("load_saved_data");
        {
            let _enter = span_recover_community.enter();
            self.community_data.recover_from_file(app_data_path.clone()).await;
        }
        let saved = {
            let _enter = span_load_saved.enter();
            if !std::fs::exists(format!("{}/app_data.bin", app_data_path)).unwrap() {
                warn!("app_data.bin不存在, 跳过恢复");
                return ();
            };
            let mut buffer = Vec::new();
            let mut file = std::fs::File::open(format!("{}/app_data.bin", app_data_path)).unwrap();
            file.read_to_end(&mut buffer).unwrap();
            let res = match bincode::decode_from_slice::<BaseListForSave, _>(
                &buffer,
                bincode::config::standard(),
            ) {
                Ok(saved) => Some(saved.0),
                Err(e) => {
                    warn!("app_data.bin解析失败: {:?}", e);
                    None
                }
            };
            res
        };

        let Some(saved) = saved else { return };

        let span_translate = tracing::info_span!("recover_translate_data");
        let span_user_data = tracing::info_span!("recover_user_data");
        {
            let _enter = span_translate.enter();
            let mut translate = self.translation_mod_data.lock_h().await;
            let index = if std::fs::exists(format!("{}/translation_mod_data_index.bin", app_data_path)).unwrap() {
                let mut buffer = Vec::new();
                let mut file = std::fs::File::open(format!("{}/translation_mod_data_index.bin", app_data_path)).unwrap();
                file.read_to_end(&mut buffer).unwrap();
                bincode::decode_from_slice(
                    &buffer,
                    bincode::config::standard(),
                ).unwrap().0
            } else {
                super::translate::ModIndex::new(Version::default())
            };
            translate.recover(
                saved.translation_mod_data,
                index,
            ).await;
            translate.recover_auto_translate_cache(saved.auto_translate_cache);
            translate.overwrite_version(version);
        }
        {
            let _enter = span_user_data.enter();
            self.user_custom_mods_order = saved.user_custom_mods_order.into_iter().collect();
            self.user_ignore_info = saved.user_ignore_info.into_iter()
            .map(|(k,v)| {
                (k, v.into_iter().collect())
            })
            .collect();
        }

        let span_mods = tracing::info_span!("recover_mods");
        {
            let _enter = span_mods.enter();
            let mods_to_add: Vec<_> = futures::stream::iter(saved.mods.iter().cloned())
                .map(|mod_| async {
                    Arc::new(ModWrapper::new(mod_, self.sync_tx.clone().unwrap()).await)
                })
                .buffer_unordered(16)
                .collect()
                .await;
            self.add_mods_batch(mods_to_add, Priority::HIGH).await;
        }

        // 为了支持循环引用，先创建空壳mods_group
        let span_group_shell = tracing::info_span!("create_group_shells");
        let group_map: DashMap<Id, Arc<Mutex<ModsGroup>>, RandomState> = DashMap::with_hasher(RandomState::default());
        {
            let _enter = span_group_shell.enter();
            for mods_group in &saved.mods_groups {
                let group = Arc::new(Mutex::new(ModsGroup {
                    id: Id::from_str(mods_group.id.clone()),
                    name: mods_group.name.clone(),
                    enabled: false,
                    mods: Vec::new(),
                }));
                group_map.insert(group.lock().await.id.clone(), group.clone());
            }
        }

        let span_group_fill = tracing::info_span!("fill_group_items");
        {
            let _enter = span_group_fill.enter();
            for (_, mods_group) in saved.mods_groups.iter().enumerate() {
                let group = group_map.get(&Id::from_str(mods_group.id.clone())).unwrap();
                let mut group = group.lock().await;

                for item in &mods_group.mods {
                    match item {
                        ModsGroupItemForSave::Mod(id) => {
                            group.mods.push(ModsGroupItem::Mod(
                                self.mods_map.get(&Id::from_str(id)).unwrap().value().clone(),
                            ));
                        }
                        ModsGroupItemForSave::ModsGroup(id) => {
                            group.mods.push(ModsGroupItem::ModsGroup(
                                group_map.get(&Id::from_str(id)).unwrap().clone(),
                            ));
                        }
                    }
                }
            }
        }
        self.mods_groups_map = group_map;


        self.mods_order = Arc::new(Mutex::new(saved.mods_order));
        self.mods_groups_order = Arc::new(Mutex::new(saved.mods_groups_order));
        
        info!("恢复完成, 耗时: {:?}", start.elapsed());
    }
    
    pub async fn to_save(&self, priority: Option<Priority>) -> BaseListForSave {
        let mods = futures::stream::iter(self.mods_map.iter())
                    .then(|item| async move { item.value().lock().await.clone() })
                    .collect()
                    .await;
        let mods_order = self.mods_order.lock().await.clone();
        let mods_groups = futures::stream::iter(self.mods_groups_map.iter())
                .then(|item| async move {
                    let group = item.value().lock().await;
                    ModsGroupForSave {
                        id: group.id.to_string(),
                        name: group.name.clone(),
                        enabled: group.enabled,
                        mods: futures::stream::iter(group.mods.iter())
                            .then(|item| async move {
                                match item {
                                    ModsGroupItem::Mod(mod_) => {
                                        ModsGroupItemForSave::Mod(mod_.lock().await.id.to_string())
                                    }
                                    ModsGroupItem::ModsGroup(group_) => {
                                        ModsGroupItemForSave::ModsGroup(
                                            group_.lock().await.id.to_string(),
                                        )
                                    }
                                }
                            })
                            .collect()
                            .await,
                    }
                })
                .collect()
                .await;
        let mods_groups_order = self.mods_groups_order.lock().await.clone();
        let user_custom_mods_order = self.user_custom_mods_order.iter().map(|item| (item.key().clone(),item.value().clone())).collect();
        let user_ignore_info = self.user_ignore_info.iter().map(|item| (item.key().clone(),item.value().clone())).map(|(k,v)| (k,v.into_iter().collect())).collect();
        let translate = self.translation_mod_data.lock(priority).await;
        let translation_mod_data = translate.save_data();
        let auto_translate_cache = translate.save_auto_translate_cache();
        drop(translate);
        BaseListForSave {
            mods,
            mods_order,
            mods_groups,
            mods_groups_order,
            user_custom_mods_order,
            user_ignore_info,
            translation_mod_data,
            auto_translate_cache,
        }
    }
    pub async fn save(&self, app_data_path: String) {
        info!("save");
        let base_list = self.to_save(Priority::HIGH).await;
        let save_meta_data = SaveMetaData {
            save_time: chrono::Local::now(),
            mods_count: base_list.mods.len(),
            mods_groups_count: base_list.mods_groups.len(),
        };
        info!("保存主要数据");
        bincode::encode_into_std_write(base_list, 
            &mut std::fs::File::options().write(true).create(true).open(format!("{}/app_data.bin", app_data_path)).unwrap(), 
            bincode::config::standard()).unwrap();
        self.community_data.save_to_file(app_data_path.clone()).await;

        info!("保存翻译数据");
        let translation_index = self.translation_mod_data.lock_h().await.save_index();
        bincode::encode_into_std_write(translation_index, 
            &mut std::fs::File::options().write(true).create(true).open(format!("{}/translation_index.bin", app_data_path)).unwrap(), 
            bincode::config::standard()).unwrap();

        std::fs::write(format!("{}/save_meta_data.json", app_data_path), serde_json::to_string(&save_meta_data).unwrap()).unwrap();
    }
    pub async fn start_auto_save(&self, app_data_path: String, task_manager: &crate::background_task::TaskManager) {

        let app_data_path = format!("{}/.auto_save", app_data_path);
        if !std::fs::exists(&app_data_path).unwrap() {
            std::fs::create_dir(&app_data_path).unwrap();
        }

        if let Some((tx, handle)) = self.auto_save_handle.lock().await.take() {
            info!("停止已有的自动保存");
            tx.send(()).unwrap();
            let _ = tokio::try_join!(handle);
        }

        let mut interval = tokio::time::interval(std::time::Duration::from_secs(60));
        let (tx, mut rx) = tokio::sync::oneshot::channel();
        let app = Arc::new(self.app_handles.clone().unwrap());
        let status_tx = task_manager.get_status_tx();
        

        let task_status = crate::background_task::TaskStatusAdd::new(
            app.clone(),
            crate::background_task::TaskStatus {
                id: uuid::Uuid::new_v4().to_string(),
                name: "自动保存".to_string(),
                status: "运行中".to_string(),
                info: "".to_string(),
                progress: 0.0,
            },
            status_tx,
        ).await;

        let handle = tokio::spawn(async move {
            let mut task_status = task_status.lock().await;
            loop {
                tokio::select! {
                    _ = interval.tick() => {
                        info!("自动保存...");
                        task_status.update_info("自动保存中");
                        task_status.update_status("运行中");
                        task_status.update_progress(10.0);

                        let base_list = app.state::<BaseList>();
                        base_list.save(app_data_path.clone()).await;

                        task_status.update_info("自动保存完成");
                        task_status.update_progress(100.0);
                        task_status.update_status("休眠中");
                        info!("自动保存完成");
                    },
                    _ = &mut rx => {
                        task_status.update_status("已结束");
                        break;
                    }
                }
            }
        });
        let mut guard = self.auto_save_handle.lock().await;
        *guard = Some((tx, handle));
    }

    pub async fn start_auto_refresh_mods_data(&self, task_manager: &crate::background_task::TaskManager) {
        if let Some((tx, handle)) = self.auto_refresh_handle.lock().await.take() {
            info!("停止已有的自动刷新mod任务");
            tx.send(()).unwrap();
            let _ = tokio::try_join!(handle);
        }

        let mut interval = tokio::time::interval(std::time::Duration::from_secs(300));
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

        let (tx, mut rx) = tokio::sync::oneshot::channel();
        let app = Arc::new(self.app_handles.clone().unwrap());
        let status_tx = task_manager.get_status_tx();

        let task_status = crate::background_task::TaskStatusAdd::new(
            app.clone(),
            crate::background_task::TaskStatus {
                id: uuid::Uuid::new_v4().to_string(),
                name: "自动刷新mod数据".to_string(),
                status: "运行中".to_string(),
                info: "".to_string(),
                progress: 0.0,
            },
            status_tx,
        ).await;

        let handle = tokio::spawn(async move {
            let mut task_status = task_status.lock().await;
            task_status.update_info("休眠中");
            task_status.update_status("休眠中");
            task_status.update_progress(100.0);
            interval.tick().await; // 跳过第一次执行避免影响初次加载
            loop {
                tokio::select! {
                    _ = interval.tick() => {
                        let base_list = app.state::<BaseList>();
                        let app_config = app.state::<Mutex<crate::AppConfig>>();
                        let app_config = app_config.lock().await;

                        if app_config.steam_mods_path.is_empty() || app_config.game_path.is_empty() {
                            task_status.update_info("路径未配置，跳过自动刷新");
                            task_status.update_status("休眠中");
                            continue;
                        }

                        task_status.update_info("自动刷新mod数据中");
                        task_status.update_status("运行中");
                        task_status.update_progress(5.0);

                        match base_list.refresh_mods_data(&app_config, &mut task_status).await {
                            Ok(_) => {
                                info!("自动刷新mod数据完成");
                                task_status.update_info("自动刷新mod数据完成");
                            }
                            Err(e) => {
                                warn!(error = ?e, "自动刷新mod数据失败");
                                task_status.update_info(format!("自动刷新失败: {}", e));
                            }
                        }

                        info!("自动刷新mod数据完成");
                        task_status.update_status("休眠中");
                    },
                    _ = &mut rx => {
                        task_status.update_status("已结束");
                        break;
                    }
                }
            }
        });

        let mut guard = self.auto_refresh_handle.lock().await;
        *guard = Some((tx, handle));
    }
}

pub fn load_save_meta_data(app_data_path: String) -> (Option<SaveMetaData>, Option<SaveMetaData>) {
    let main = if std::fs::exists(format!("{}/save_meta_data.json", app_data_path)).unwrap() {
        let save_meta_data = serde_json::from_str::<SaveMetaData>(&std::fs::read_to_string(format!("{}/save_meta_data.json", app_data_path)).unwrap()).unwrap();
        Some(save_meta_data)
    } else {
        None
    };
    let auto = if std::fs::exists(format!("{}/.auto_save/save_meta_data.json", app_data_path)).unwrap() {
        let save_meta_data = serde_json::from_str::<SaveMetaData>(&std::fs::read_to_string(format!("{}/.auto_save/save_meta_data.json", app_data_path)).unwrap()).unwrap();
        Some(save_meta_data)
    } else {
        None
    };
    (main, auto)
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SaveMetaData {
    save_time: chrono::DateTime<chrono::Local>,
    mods_count: usize,
    mods_groups_count: usize,
}