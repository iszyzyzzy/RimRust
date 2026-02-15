mod community_rules;
mod steam_db;

use tracing::{info, warn};
use std::io::Read;
use std::sync::Arc;

use crate::types::*;

pub use steam_db::Database as SteamDbData;
// pub use community_rules::CommunityModsOrder;
pub use steam_db::SteamDb;

#[derive(Default)]
pub struct CommunityData {
    community_rules: Arc<PriorityMutex<community_rules::CommunityModsOrder>>,
    steam_db: Arc<PriorityMutex<steam_db::SteamDb>>,
}

impl CommunityData {
    pub fn get_community_rules(&self) -> Arc<PriorityMutex<community_rules::CommunityModsOrder>> {
        self.community_rules.clone()
    }
    pub fn get_steam_db(&self) -> Arc<PriorityMutex<steam_db::SteamDb>> {
        self.steam_db.clone()
    }
    pub async fn update_community_mods_order(
        &self,
        app_data_path: String,
        status: &mut crate::background_task::TaskStatusAdd,
        community_rules_update_path: String,
        proxy: Option<String>,
        priority: Option<Priority>,
    ) -> Result<(), String> {
        self.community_rules
            .lock(priority)
            .await
            .update_community_mods_order(app_data_path, status, community_rules_update_path, proxy)
            .await
    }
    pub async fn update_steam_db(
        &self,
        app_data_path: String,
        status: &mut crate::background_task::TaskStatusAdd,
        steam_db_update_path: String,
        proxy: Option<String>,
        priority: Option<Priority>,
    ) -> Result<(), String> {
        self.steam_db
            .lock(priority)
            .await
            .update_steam_db(app_data_path, status, steam_db_update_path, proxy)
            .await
    }
    pub async fn recover_from_file(&self, app_data_path: String) {
        info!(app_data_path = ?app_data_path, "从文件恢复社区数据");
        if !std::fs::exists(format!("{}\\community_mods_order_cache.bin", app_data_path)).unwrap() {
            let mut community_rules = self.community_rules.lock(Priority::HIGH).await;
            *community_rules = community_rules::CommunityModsOrder::default();
            warn!("{}\\community_mods_order_cache.bin 不存在，设置空数据", app_data_path)
        } else {
            let mut community_rules = self.community_rules.lock(Priority::HIGH).await;
            let mut buffer = Vec::new();
            let mut file = std::fs::File::open(format!(
                "{}\\community_mods_order_cache.bin",
                app_data_path
            )).unwrap();
            file.read_to_end(&mut buffer).unwrap();
            (*community_rules, _) = match bincode::decode_from_slice(
                &buffer,
                bincode::config::standard(),
            )
            {
                Ok(community_rules) => community_rules,
                Err(e) => {
                    warn!(error = ?e, "community_mods_order_cache.bin 解析失败，设置空数据");
                    (community_rules::CommunityModsOrder::default(), 0)
                }
            };
        }
        if !std::fs::exists(format!("{}\\steam_db_cache.bin", app_data_path)).unwrap() {
            let mut steam_db = self.steam_db.lock(Priority::HIGH).await;
            *steam_db = steam_db::SteamDb::default();
            warn!("{}\\steam_db_cache.bin 不存在，设置空数据", app_data_path)
        } else {
            let mut steam_db = self.steam_db.lock(Priority::HIGH).await;
            let mut buffer = Vec::new();
            let mut file = std::fs::File::open(format!(
                "{}\\steam_db_cache.bin",
                app_data_path
            )).unwrap();
            file.read_to_end(&mut buffer).unwrap();
            (*steam_db,_) = match bincode::decode_from_slice(
                &buffer,
                bincode::config::standard(),
            )
            {
                Ok(steam_db) => steam_db,
                Err(e) => {
                    warn!(error = ?e, "steam_db_cache.bin 解析失败，设置空数据");
                    (steam_db::SteamDb::default(), 0)
                }
            };
        }
    }
    pub async fn save_to_file(&self, app_data_path: String) {
        info!("社区数据保存到文件");
        let community_rules = self.community_rules.lock(Priority::HIGH).await;
        let steam_db = self.steam_db.lock(Priority::HIGH).await;
        let file = std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .open(format!("{}\\community_mods_order_cache.bin", app_data_path));
        let mut file = match file {
            Ok(file) => file,
            Err(e) => {
                warn!(error = ?e, "创建/打开文件失败");
                return;
            }
        };
        bincode::encode_into_std_write(
            community_rules.clone(),
            &mut file,
            bincode::config::standard(),
        )
        .unwrap();
        let file = std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .open(format!("{}\\steam_db_cache.bin", app_data_path));
        let mut file = match file {
            Ok(file) => file,
            Err(e) => {
                warn!(error = ?e, "创建/打开文件失败");
                return;
            }
        };
        bincode::encode_into_std_write(
            steam_db.clone(),
            &mut file,
            bincode::config::standard(),
        )
        .unwrap();
    if cfg!(debug_assertions) {
        let debug_data = serde_json::json!({
            "community_rules": serde_json::to_value(community_rules.clone()).unwrap_or_default(),
        });

        let debug_file = std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .open(format!("{}\\debug_data.json", app_data_path));

        if let Ok(mut file) = debug_file {
            if let Err(e) = serde_json::to_writer_pretty(&mut file, &debug_data) {
                warn!(error = ?e, "写入debug json失败");
            }
        }
    }
    }
}