use super::super::base_list::*;
use crate::background_task;
use crate::types::*;

use futures::StreamExt;
use serde::{Deserialize, Serialize};
use tracing::trace;
use ahash::HashMap;
use tracing::{info, warn};

#[derive(Debug, Default, Clone, bincode::Decode,bincode::Encode, Serialize, Deserialize)]
pub struct CommunityModsOrder {
    data: HashMap<PackageId, Vec<ModOrder>>,
    updated_at: i64,
}

impl CommunityModsOrder {
    //#[instrument(skip(self))]
    pub async fn update_community_mods_order(
        &mut self,
        app_data_path: String,
        status: &mut background_task::TaskStatusAdd,
        community_rules_update_path: String,
        proxy: Option<String>,
    ) -> Result<(), String> {
        //let span = span!(tracing::Level::INFO, "update_community_mods_order");
        //let _enter = span.enter();
        info!(update_at = ?self.updated_at, now = ?chrono::Utc::now().timestamp(),"检查社区规则更新");
        if !(chrono::Utc::now().timestamp() - self.updated_at > 60 * 60 * 24 * 30) // 30天
        {
            info!("无须更新");
            status.update_info("无须更新");
            status.update_progress(100.0);
            return Ok(());
        }
        self.data.clear();
        info!("开始下载");
        status.update_info("正在下载...");
        status.update_progress(5.0);

        let client = if let Some(proxy) = proxy {
            if proxy == "" {
                reqwest::Client::new()
            } else {
                reqwest::Client::builder()
                .proxy(reqwest::Proxy::all(proxy).map_err(|e| e.to_string())?)
                .build()
                .map_err(|e| e.to_string())?
            }
        } else {
            reqwest::Client::new()
        }; 
        let res = client.get(community_rules_update_path)
        .send()
        .await
        .map_err(|e| e.to_string())?;

        let total_size = res.content_length().unwrap_or(0);
        let mut downloaded = 0;

        let mut buffer = Vec::new();

        let mut stream = res.bytes_stream();
        let mut count = 0;
        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(|e| {
                warn!(error = ?e, "下载失败");
                e.to_string()
            })?;
            buffer.extend_from_slice(&chunk);
            downloaded += chunk.len() as u64;
            
            // 计算下载进度
            count += 1;
            if total_size > 0 && count % 100 == 0 {
                status.update_progress(5.0 + 25.0 * downloaded as f64 / total_size as f64);
            }
        }


        info!(size = ?buffer.len(), "下载完成");
        if cfg!(debug_assertions) {
            std::fs::write(
                format!("{}/community_mods_order_debug.json", app_data_path),
                &buffer,
            )
            .ok();
        }

        status.update_info("反序列化...");
        status.update_progress(30.0);
        let response: StructOfCommunityModsOrder = serde_json::from_slice(&buffer)
        .map_err(|e| {
            warn!(error = ?e, "反序列化失败");
            status.update_info("反序列化失败");
            e.to_string()
        })?;

        trace!(response = ?response, "raw");

        let total = response.rules.len();
        info!(len = ?total, "反序列化完成");
        if cfg!(debug_assertions) {
            std::fs::write(
                format!("{}/community_mods_order_debug_parsed.json", app_data_path),
                serde_json::to_string_pretty(&response).unwrap(),
            )
            .ok();
        }


        status.update_info("解析...");
        status.update_progress(50.0);

        response
            .rules
            .iter()
            .enumerate()
            .for_each(|(index, (mod_id, rule))| {
                trace!(index = ?index, mod_package_id = ?mod_id, rule = ?rule, "解析");
                let mut order = Vec::new();
                if let Some(load_before) = &rule.load_before {
                    for (mod_id, _) in load_before {
                        order.push(ModOrder::Before(PackageId::from_str(mod_id.clone())));
                    }
                }
                if let Some(load_after) = &rule.load_after {
                    for (mod_id, _) in load_after {
                        order.push(ModOrder::After(PackageId::from_str(mod_id.clone())));
                    }
                }
                if let Some(_) = &rule.load_first {
                    order.push(ModOrder::First());
                }
                if let Some(_) = &rule.load_bottom {
                    order.push(ModOrder::Last());
                }
                self.data.insert(PackageId::from_str(mod_id.clone()), order);

                // 更新进度
                if index % 500 == 0 {
                    status.update_progress(50.0 + 40.0 * (index as f64 + 1.0) / total as f64);
                }
            });
        info!("解析完成");
        trace!(data = ?self.data, "raw");
        info!("开始保存");
        status.update_info("正在保存缓存...");
        status.update_progress(90.0);
        self.updated_at = chrono::Utc::now().timestamp();
        let file = std::fs::File::options()
            .write(true)
            .create(true)
            .open(format!("{}/community_mods_order_cache.bin", app_data_path));
        let mut file = match file {
            Ok(file) => file,
            Err(e) => {
                warn!(error = ?e, "创建/打开文件失败");
                status.update_info("创建/打开文件失败");
                return Err("创建/打开文件失败".to_string());
            }
        };
        bincode::encode_into_std_write(
            self.clone(),
            &mut file,
            bincode::config::standard(),
        )
        .unwrap();
        info!(path = ?format!("{}/community_mods_order_cache.bin", app_data_path), update_at = ?self.updated_at,"保存完成");
        /*             std::fs::write(
            format!("{}/community_mods_order_cache.bin", app_data_path),
            bincode::serialize(&self.community_mods_order).unwrap(),
        )
        .unwrap(); */
        status.update_info("已完成");
        status.update_progress(100.0);
        Ok(())
    }
    pub fn get(&self, package_id: &PackageId) -> Option<&Vec<ModOrder>> {
        self.data.get(package_id)
    }
    pub fn get_updated_at(&self) -> i64 {
        self.updated_at
    }
}

#[derive(Serialize, Deserialize, Debug)]
struct StructOfCommunityModsOrder {
    timestamp: u64,
    rules: HashMap<String, CommunityModsOrderRule>,
}

#[derive(Serialize, Deserialize, Debug)]
struct CommunityModsOrderRule {
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        rename = "loadBefore"
    )]
    load_before: Option<HashMap<String, CommunityModsOrderRuleModReference>>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        rename = "loadAfter"
    )]
    load_after: Option<HashMap<String, CommunityModsOrderRuleModReference>>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        rename = "loadFirst"
    )]
    load_first: Option<CommunityModsOrderRuleModReferenceSP>,
    #[serde(default, skip_serializing_if = "Option::is_none", rename = "loadLast")]
    load_bottom: Option<CommunityModsOrderRuleModReferenceSP>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(untagged)]
pub enum StringValue {
    Single(String),
    Multiple(Vec<String>),
}

#[derive(Serialize, Deserialize, Debug)]
pub struct CommunityModsOrderRuleModReference {
    name: StringValue,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    comment: Option<StringValue>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct CommunityModsOrderRuleModReferenceSP {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    comment: Option<StringValue>,
    value: bool,
}

