use crate::background_task;

use futures::StreamExt;
use serde::{Deserialize, Serialize};
use ahash::{HashMap, HashMapExt};
use tracing::{info, trace, warn};

#[derive(Serialize, Deserialize, Debug, Default, Clone, bincode::Decode, bincode::Encode)]
pub struct SteamDb {
    data: SteamDbData,
    package_id_to_appid: HashMap<String, Vec<String>>,
    updated_at: u64,
}

impl SteamDb {
    //#[instrument(skip(self))]
    pub async fn update_steam_db(
        &mut self,
        app_data_path: String,
        status: &mut background_task::TaskStatusAdd,
        steam_db_update_path: String,
        proxy: Option<String>,
    ) -> Result<(), String> {
        //let span = span!(tracing::Level::INFO, "update_steam_db");
        //let _enter = span.enter();
        info!(update_at = ?self.updated_at, now = ?chrono::Utc::now().timestamp(),"检查SteamDb更新");
        if !(chrono::Utc::now().timestamp() - self.updated_at as i64 > 60 * 60 * 24 * 30) {
            info!("无须更新");
            status.update_info("无须更新");
            status.update_progress(100.0);
            return Ok(());
        }
        self.data = SteamDbData::default();
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
        let res = client.get(steam_db_update_path)
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
        status.update_info("反序列化...");
        status.update_progress(30.0);
        let response: SteamDbData = serde_json::from_slice(&buffer)
        .map_err(|e| {
            warn!(error = ?e, "反序列化失败");
            status.update_info("反序列化失败");
            e.to_string()
        })?;

        trace!(response = ?response, "raw");

        let total = response.database.len();
        info!(len = ?total, "反序列化完成");

        status.update_info("解析&构建映射...");
        status.update_progress(50.0);
        info!("解析&构建映射...");

        self.data = response;
        self.updated_at = chrono::Utc::now().timestamp() as u64;

        // 构建一个package_id -> appid的映射
        let mut package_id_to_appid: HashMap<String, Vec<String>> = HashMap::new();
        let max = self.data.database.len();
        for (index, (appid, data)) in self.data.database.iter().enumerate() {
            if let Some(package_id) = data.get_package_id() {
                let appid = appid.clone();
                let entry = package_id_to_appid.entry(package_id).or_insert_with(Vec::new);
                entry.push(appid);
            }
            if index % 500 == 0 {
                status.update_progress(50.0 + 40.0 * index as f64 / max as f64);
            }
        }

        self.package_id_to_appid = package_id_to_appid;

        status.update_info("正在保存缓存...");
        status.update_progress(90.0);

        let file = std::fs::File::options()
        .write(true)
        .create(true)
        .open(format!("{}/steam_db_cache.bin", app_data_path));
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
        info!(path = ?format!("{}/steam_db_cache.bin", app_data_path), update_at = ?self.updated_at, "保存完成");
        status.update_info("已完成");
        status.update_progress(100.0);
        Ok(())
    } 
    pub fn _get_steam_db(&self) -> &HashMap<String, Database> {
        &self.data.database
    }
    pub fn get_data(&self, appid: &str) -> Option<Database> {
        self.data.database.get(appid).cloned()
    }
    /// !全库遍历，很慢
    pub fn get_data_by_package_id(&self, package_id: &str) -> Vec<Database> {
        let package_id = package_id.to_lowercase();
        self.data
            .database
            .values()
            .filter(|data| {
                if let Some(data) = data.get_package_id() {
                    data.to_lowercase() == package_id
                } else {
                    false
                }
            })
            .cloned()
            .collect()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, bincode::Decode, bincode::Encode)]
pub struct SteamDbData {
    version: i64,
    database: HashMap<String, Database>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, bincode::Decode, bincode::Encode)]
#[serde(rename_all = "camelCase")]
pub struct Database {
    appid: Option<bool>,
    url: Option<String>,
    package_id: Option<String>,
    packageid: Option<String>, // 兼容泰南的神奇代码
    name: Option<String>,
    authors: Option<String>,
    game_versions: Option<Vec<Option<String>>>,
    steam_name: Option<String>,
    dependencies: Option<HashMap<String, Vec<String>>>,
    unpublished: Option<bool>,
    #[serde(rename = "external_time_created")]
    external_time_created: Option<i64>,
    #[serde(rename = "external_time_updated")]
    external_time_updated: Option<i64>,
    blacklist: Option<Blacklist>,
}

impl Database {
    pub fn get_name(&self) -> Option<String> {
        if let Some(data) = self.name.clone() {
            Some(data)
        } else if let Some(data) = self.steam_name.clone() {
            Some(data)
        } else {
            None
        }
    }
    pub fn get_package_id(&self) -> Option<String> {
        if let Some(data) = self.package_id.clone() {
            Some(data)
        } else if let Some(data) = self.packageid.clone() {
            Some(data)
        } else {
            None
        }
    }
    pub fn get_authors(&self) -> Option<String> {
        if let Some(data) = self.authors.clone() {
            Some(data)
        } else {
            None
        }
    }
    pub fn get_game_versions(&self) -> Option<Vec<String>> {
        if let Some(data) = self.game_versions.clone() {
            Some(data.into_iter().filter_map(|x| x).collect())
        } else {
            None
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, bincode::Decode, bincode::Encode)]
pub struct Blacklist {
    value: bool,
    comment: String,
}