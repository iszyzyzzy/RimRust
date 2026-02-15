use tracing::{debug, info, trace, warn};

use crate::types::*;
use super::types::*;
use super::xml::try_decode;
use crate::mods::SteamDb;

//#[instrument(skip(path, status, steam_db, priority),
//    fields(task_id = %status.get_task_id())
//)]
pub async fn load_mod_from_path(
    path: &str,
    status: &mut crate::background_task::TaskStatusAdd,
    steam_db: SteamDb,
    priority: Option<Priority>
) -> Result<crate::mods::ModInner, String> {
    debug!(path = ?path, "开始加载mod");
    status.update_info("读取中");
    status.update_progress(33.3);
    if !(tokio::fs::try_exists(path).await.unwrap_or(false)
        && tokio::fs::try_exists(format!("{}/About/About.xml", path)).await.unwrap_or(false))
    {
        warn!("Mod目录不存在: {}", path);
        return Err(format!("Mod目录不存在: {}", path));
    };
    let file_path = format!("{}/About/About.xml", path);
    debug!(file_path = ?file_path, "读取About.xml");
    let data = tokio::fs::read(&file_path).await.unwrap();
    
    let content = match try_decode(data) {
        Ok(content) => {
            debug!("成功解码About.xml");
            trace!(content = ?content);
            content
        },
        Err(e) => {
            warn!(path = ?file_path, "About.xml解码失败: {:?}", e);
            return Err(format!("About.xml解码失败: {:?}", e));
        }
    };

    status.update_info("解析中");
    status.update_progress(66.6);
    info!(path = ?path, "加载mod metadata");
    
    let meta_data = match ModMetaData::from_xml(&content) {
        Ok(meta_data) => meta_data,
        Err(e) => {
            warn!(path = ?path, "About.xml解析失败: {:?}", e);
            return Err(format!("About.xml解析失败: {:?}", e));
        }
    };
    debug!(meta_data = ?meta_data);

    Ok(meta_data.to_mod(path.to_string(), steam_db, priority).await)
}

use tauri::Manager;

pub async fn generate_load_mission(
    path: &str,
    task_manager: &tokio::sync::MutexGuard<'_, crate::background_task::TaskManager>,
    task_deps: Option<Vec<String>>,
) -> Result<(), String> {
    debug!(path = ?path, "开始生成加载任务");
    let mut entries = tokio::fs::read_dir(path).await.map_err(|e| e.to_string())?;
    while let Ok(Some(entry)) = entries.next_entry().await {
        debug!(?entry);
        let path = entry.path();
        if path.is_dir() {
            let path = path.to_str().unwrap().to_string();
            debug!(dir_path = ?path, "发现mod目录");
            task_manager
                .add_task(
                    format!("加载mod: {}", path.split("\\").last().unwrap()),
                    Box::new(move |_app, status| {
                        Box::pin(async move {
                            let mut status = status.lock().await;
                            let base_list = _app.state::<crate::mods::BaseList>();
                            let steam_db = base_list.community_data.get_steam_db();
                            let mod_ = match load_mod_from_path(&path, 
                                &mut status, 
                                steam_db, 
                                Priority::LOW
                            ).await {
                                Ok(mod_) => mod_,
                                Err(e) => {
                                    return Err(e);
                                }
                            };
                            base_list.check_and_add_mod(mod_, Priority::LOW).await;
                            Ok(())
                        })
                    }),
                    Some(15),
                    task_deps.clone(),
                )
                .await;
        };
    }
    Ok(())
}