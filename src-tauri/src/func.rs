use ahash::{HashMap, HashSet, HashMapExt, HashSetExt};
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::info;

use crate::mods::{self, BaseList, BaseListForSave, ModOrder, SearchField};
use crate::{background_task, types::*, AppConfig};

#[tauri::command]
pub async fn base_list_get(
    base_list: tauri::State<'_, BaseList>,
) -> Result<BaseListForSave, String> {
    info!("前端请求全量数据");
    Ok(base_list.to_save(Priority::HIGH).await)
}

#[tauri::command]
pub async fn mod_set_enable(
    base_list: tauri::State<'_, BaseList>,
    mod_id: Vec<String>,
    enabled: bool,
) -> Result<(), String> {
    info!(
        "前端请求设置mod启用状态, mod_id: {:?}, enabled: {}",
        mod_id, enabled
    );
    for id in mod_id {
        base_list
            .set_enable_mod(&Id::from_str(id), enabled, Priority::HIGH)
            .await?;
    }
    Ok(())
}

#[tauri::command]
pub async fn group_set_enable(
    base_list: tauri::State<'_, BaseList>,
    group_id: String,
    enabled: bool,
) -> Result<(), String> {
    info!(
        "前端请求设置mod组启用状态, group_id: {}, enabled: {}",
        group_id, enabled
    );
    base_list
        .set_enable_mod_group(&Id::from_str(group_id), enabled, Priority::HIGH)
        .await
}

#[tauri::command]
pub async fn mod_change_order(
    base_list: tauri::State<'_, BaseList>,
    from_index: usize,
    to_index: usize,
) -> Result<(), String> {
    info!(
        "前端请求设置mod显示顺序, from_index: {}, to_index: {}",
        from_index, to_index
    );
    Ok(base_list
        .change_mod_display_order(from_index, to_index)
        .await)
}

#[tauri::command]
pub async fn group_create(
    base_list: tauri::State<'_, BaseList>,
    group: mods::ModsGroupForSave,
) -> Result<(), String> {
    info!("前端请求添加mod组, group: {:?}", group);
    Ok(base_list.add_mod_group(group).await)
}

#[tauri::command]
pub async fn group_delete(
    base_list: tauri::State<'_, BaseList>,
    group_id: String,
) -> Result<(), String> {
    info!("前端请求删除mod组, group_id: {}", group_id);
    Ok(base_list
        .remove_mod_group(&Id::from_str(group_id))
        .await)
}

#[tauri::command]
pub async fn group_add_object(
    base_list: tauri::State<'_, BaseList>,
    group_id: String,
    mod_id: String,
) -> Result<(), String> {
    info!(
        "前端请求添加对象到组, group_id: {}, object_id: {}",
        group_id, mod_id
    );
    Ok(base_list
        .add_object_to_group(
            &Id::from_str(group_id),
            &Id::from_str(mod_id)
        )
        .await)
}

#[tauri::command]
pub async fn group_remove_object(
    base_list: tauri::State<'_, BaseList>,
    group_id: String,
    mod_id: String,
) -> Result<(), String> {
    info!(
        "前端请求从组中移除对象, group_id: {}, object_id: {}",
        group_id, mod_id
    );
    Ok(base_list
        .remove_object_from_group(
            &Id::from_str(group_id),
            &Id::from_str(mod_id),
        )
        .await)
}

#[tauri::command]
pub async fn group_rename(
    base_list: tauri::State<'_, BaseList>,
    group_id: String,
    new_name: String,
) -> Result<(), String> {
    info!(
        "前端请求重命名mod组, group_id: {}, new_name: {}",
        group_id, new_name
    );
    Ok(base_list
        .rename_mod_group(&Id::from_str(group_id), &new_name)
        .await)
}

#[tauri::command]
pub async fn group_change_order(
    base_list: tauri::State<'_, BaseList>,
    from_index: usize,
    to_index: usize,
) -> Result<(), String> {
    info!(
        "前端请求设置mod组显示顺序, from_index: {}, to_index: {}",
        from_index, to_index
    );
    Ok(base_list
        .change_mods_group_display_order(from_index, to_index)
        .await)
}



#[tauri::command]
pub async fn mod_set_display_name(
    base_list: tauri::State<'_, BaseList>,
    mod_id: String,
    new_name: String,
) -> Result<(), String> {
    info!(
        "前端请求设置mod显示名称, mod_id: {}, new_name: {}",
        mod_id, new_name
    );
    Ok(base_list
        .set_mod_display_name(&Id::from_str(mod_id), &new_name, Priority::HIGH)
        .await)
}

#[tauri::command]
pub async fn sort_mods(
    base_list: tauri::State<'_, BaseList>,
    app_config: tauri::State<'_, Mutex<AppConfig>>,
) -> Result<crate::mods::SortResult, String> {
    info!("前端请求排序mod列表");
    let app_config = app_config.lock().await;
    let version = app_config.game_version.clone();
    drop(app_config);
    base_list
        .sort(version.to_short_ver(), Priority::HIGH)
        .await
}

use tauri_plugin_dialog::DialogExt;

#[tauri::command]
pub async fn xml_load_file(
    app: tauri::AppHandle,
    base_list: tauri::State<'_, BaseList>,
) -> Result<(), String> {
    let file_path = app
        .dialog()
        .file()
        .add_filter("XML", &["xml"])
        .blocking_pick_file();
    if let Some(file_path) = file_path {
        info!("前端请求加载mods列表, file_path: {:?}", file_path);
        base_list
            .load_from_xml(
                &file_path
                    .into_path()
                    .unwrap()
                    .into_os_string()
                    .into_string()
                    .unwrap()
            )
            .await
    } else {
        Err("未选择文件".to_string())
    }
}

#[tauri::command]
pub async fn xml_load_from_config(
    app_config: tauri::State<'_, Mutex<AppConfig>>,
    base_list: tauri::State<'_, BaseList>,
) -> Result<(), String> {
    info!("前端请求从游戏设置加载mods列表");
    let file_path = app_config.lock().await.game_config_path.clone();
    base_list
        .load_from_xml(
            &format!("{}/ModsConfig.xml", file_path)
        )
        .await
}

#[tauri::command]
pub async fn xml_save_file(
    mods: Vec<Id>,
    app: tauri::AppHandle,
    app_config: tauri::State<'_, Mutex<AppConfig>>,
    base_list: tauri::State<'_, BaseList>,
) -> Result<(), String> {
    let game_version = app_config.lock().await.game_version.clone();
    let file_path = app
        .dialog()
        .file()
        .add_filter("XML", &["xml"])
        .blocking_save_file();
    if let Some(file_path) = file_path {
        info!("前端请求保存mods列表到文件, file_path: {:?}", file_path);
        base_list
            .save_to_xml(
                &file_path
                    .into_path()
                    .unwrap()
                    .into_os_string()
                    .into_string()
                    .unwrap(),
                game_version,
                mods
            )
            .await
    } else {
        Err("未选择文件".to_string())
    }
}

#[tauri::command]
pub async fn xml_save_to_config(
    mods: Vec<Id>,
    app_config: tauri::State<'_, Mutex<AppConfig>>,
    base_list: tauri::State<'_, BaseList>,
) -> Result<(), String> {
    info!("前端请求保存mods列表到游戏设置");
    let file_path = app_config.lock().await.game_config_path.clone();
    let game_version = app_config.lock().await.game_version.clone();
    base_list
        .save_to_xml(
            &format!("{}/ModsConfig.xml", file_path),
            game_version,
            mods
        )
        .await
}

#[tauri::command]
pub async fn scan_err(
    base_list: tauri::State<'_, BaseList>,
    app_config: tauri::State<'_, Mutex<AppConfig>>,
) -> Result<mods::ScanResult, String> {
    info!("前端请求错误扫描");
    let game_version = app_config.lock().await.game_version.clone();
    Ok(base_list.scan(game_version).await)
}

#[tauri::command]
pub async fn scan_err_ignore_add(
    base_list: tauri::State<'_, BaseList>,
    mod_id: String,
    info: mods::InfoType,
) -> Result<(), String> {
    info!(
        "前端请求添加忽略的错误信息, mod_id: {}, info: {:?}",
        mod_id, info
    );
    base_list
        .add_ignore_info(Id::from_str(mod_id), info)
        .await;
    Ok(())
}

#[tauri::command]
pub async fn scan_err_ignore_remove(
    base_list: tauri::State<'_, BaseList>,
    mod_id: String,
    info: mods::InfoType,
) -> Result<(), String> {
    info!(
        "前端请求移除忽略的错误信息, mod_id: {}, info: {:?}",
        mod_id, info
    );
    base_list
        .remove_ignore_info(Id::from_str(mod_id), info)
        .await;
    Ok(())
}

#[tauri::command]
pub async fn tran_unconfirmed_get(
    base_list: tauri::State<'_, BaseList>,
) -> Result<HashMap<Id, Vec<(Id, f64)>>, String> {
    info!("前端请求翻译数据");
    let tr = base_list
        .translation_mod_data
        .lock(Priority::HIGH)
        .await;
    Ok(tr.get_all_unconfirmed())
}

#[tauri::command]
pub async fn tran_comfirm(
    base_list: tauri::State<'_, BaseList>,
    mod_id: String,
    translation_id: String,
) -> Result<(), String> {
    info!(
        "前端请求确认翻译数据, mod_id: {}, translation_id: {}",
        mod_id, translation_id
    );
    base_list
        .translation_mod_data
        .lock(Priority::HIGH)
        .await
        .confirm(Id::from_str(mod_id), Id::from_str(translation_id));
    Ok(())
}


#[tauri::command]
pub async fn tran_match_get(
    base_list: tauri::State<'_, BaseList>,
) -> Result<HashMap<Id, Id>, String> {
    info!("前端请求翻译匹配数据");
    let tr = base_list
        .translation_mod_data
        .lock(Priority::HIGH)
        .await;
    Ok(tr.get_all_match())
}

#[tauri::command]
pub async fn tran_remove(
    base_list: tauri::State<'_, BaseList>,
    mod_id: String,
) -> Result<(), String> {
    info!("前端请求移除翻译数据, mod_id: {}", mod_id);
    let data = base_list.get_mod(&Id::from_str(mod_id));
    if let Some(data) = data {
        let data = data.lock().await;
        base_list
            .translation_mod_data
            .lock(Priority::HIGH)
            .await
            .remove(&data);
    }
    Ok(())
}


#[tauri::command]
pub async fn tran_rematch_all(
    base_list: tauri::State<'_, BaseList>,
) -> Result<HashMap<Id, Vec<(Id, f64)>>, String> {
    info!("前端请求重新匹配翻译数据");
    Ok(base_list
        .rematch_translation(Priority::HIGH)
        .await)
}

#[tauri::command]
pub async fn tran_package_get(
    base_list: tauri::State<'_, BaseList>,
) -> Result<Vec<Id>, String> {
    info!("前端请求获取所有翻译包");
    Ok(base_list
        .translation_mod_data
        .lock(Priority::HIGH)
        .await
        .get_all_translation_pack()
    )
}

#[tauri::command]
pub async fn tran_user_ignore_add(
    base_list: tauri::State<'_, BaseList>,
    mod_id: String,
) -> Result<(), String> {
    info!(
        "前端请求添加用户忽略的翻译匹配, mod_id: {}",
        mod_id
    );
    let guard = base_list.get_mod(&Id::from_str(mod_id));
    if guard.is_none() {
        return Ok(());
    }
    let guard = guard.unwrap();
    let guard = guard.lock().await;
    base_list
        .translation_mod_data
        .lock(Priority::HIGH)
        .await
        .add_user_ignore(&guard);
    Ok(())
}

#[tauri::command]
pub async fn tran_user_ignore_remove(
    base_list: tauri::State<'_, BaseList>,
    mod_id: String,
) -> Result<(), String> {
    info!(
        "前端请求移除用户忽略的翻译匹配, mod_id: {}",
        mod_id
    );
    base_list
        .translation_mod_data
        .lock(Priority::HIGH)
        .await
        .remove_user_ignore(Id::from_str(mod_id));
    Ok(())
}

#[tauri::command]
pub async fn tran_package_add(
    base_list: tauri::State<'_, BaseList>,
    mod_id: String,
) -> Result<(), String> {
    info!(
        "前端请求添加翻译包, mod_id: {}",
        mod_id
    );
    let guard = base_list.get_mod(&Id::from_str(mod_id));
    if guard.is_none() {
        return Ok(());
    }
    let guard = guard.unwrap();
    let guard = guard.lock().await;
    base_list
        .translation_mod_data
        .lock(Priority::HIGH)
        .await
        .add_translation_pack(&guard);
    Ok(())
}

#[tauri::command]
pub async fn tran_package_remove(
    base_list: tauri::State<'_, BaseList>,
    mod_id: String,
) -> Result<(), String> {
    info!(
        "前端请求移除翻译包, mod_id: {}",
        mod_id
    );
    let guard = base_list.get_mod(&Id::from_str(mod_id));
    if guard.is_none() {
        return Ok(());
    }
    let guard = guard.unwrap();
    let guard = guard.lock().await;
    base_list
        .translation_mod_data
        .lock(Priority::HIGH)
        .await
        .remove_translation_pack(&guard);
    Ok(())
}

#[tauri::command]
pub async fn tran_custom_calc(
    source_id: String,
    target_id: String,
    base_list: tauri::State<'_, BaseList>,
) -> Result<mods::CustomCalcResult, String> {
    info!(
        "前端请求自定义翻译匹配计算, source_id: {}, target_id: {}",
        source_id, target_id
    );
    let source = base_list.mods_map.get(&Id::from_str(source_id)).unwrap();
    let source = source.lock().await;
    let target = base_list.mods_map.get(&Id::from_str(target_id)).unwrap();
    let target = target.lock().await;
    Ok(base_list
        .translation_mod_data
        .lock(Priority::HIGH)
        .await
        .custom_calc(&source, &target)
    )
}

#[tauri::command]
pub async fn search_mod(
    base_list: tauri::State<'_, BaseList>,
    search_text: String,
    search_field: Vec<String>,
    enabled_only: bool,
) -> Result<mods::SearchResult, String> {
    info!(
        "前端请求搜索mod, search_text: {}, search_field: {:?}, enabled_only: {}",
        search_text, search_field, enabled_only
    );
    Ok(base_list
        .search_data
        .lock(Priority::HIGH)
        .await
        .search(&search_text, SearchField::from_str_vec(search_field), enabled_only)
        .await)
}

#[tauri::command]
pub async fn mod_refresh_all(
    base_list: tauri::State<'_, BaseList>,
    app_config: tauri::State<'_, Mutex<AppConfig>>,
    task_manager: tauri::State<'_, Mutex<background_task::TaskManager>>,
    app: tauri::AppHandle,
) -> Result<(Vec<Id>,Vec<Id>,HashMap<Id, Vec<(Id, f64)>>), String> {
    info!("前端请求强制刷新mod数据");
    let app_config = app_config.lock().await;
    let task_status = background_task::TaskStatusAdd::new(
        Arc::new(app),
        background_task::TaskStatus {
            id: uuid::Uuid::new_v4().to_string(),
            name: "强制刷新ing".to_string(),
            status: "运行中".to_string(),
            info: "".to_string(),
            progress: 0.0,
        },
        task_manager.lock().await.get_status_tx(),
    ).await;
    let mut task_status = task_status.lock().await;
    base_list
        .refresh_mods_data(&app_config, &mut task_status)
        .await
}

#[tauri::command]
pub async fn steamdb_get_by_package_id(
    base_list: tauri::State<'_, BaseList>,
    package_id: String,
) -> Result<Vec<mods::SteamDbData>, String> {
    info!(
        package_id = ?package_id,
        "前端请求使用package_id搜索steam_db",
    );
    Ok(base_list
        .community_data
        .get_steam_db()
        .lock_h().await
        .get_data_by_package_id(&package_id)
    )
}

#[tauri::command]
pub async fn translate(
    text: String,
    from: String,
    to: String,
    app_config: tauri::State<'_, Mutex<AppConfig>>,
    base_list: tauri::State<'_, BaseList>
) -> Result<crate::mods::AutoTranslateResult, String> {
    info!(
        text = ?text,
        from = ?from,
        to = ?to,
        "前端请求翻译",
    );
    let proxy = app_config.lock().await.proxy.clone();
    let (cache, ongoing) = base_list
        .translation_mod_data
        .lock(Priority::HIGH)
        .await
        .get_auto_translate_cache();

    crate::mods::auto_translate(text, from, to, proxy, cache, ongoing).await
}

#[tauri::command]
pub async fn save_mata_data_get(
    app_config: tauri::State<'_, Mutex<AppConfig>>
) -> Result<(Option<crate::mods::SaveMetaData>, Option<crate::mods::SaveMetaData>), String> {
    let app_data_path = app_config.lock().await.app_data_path.clone();

    Ok(crate::mods::load_save_meta_data(app_data_path))
}

#[tauri::command]
pub async fn sort_set_user_custom_order(
    package_id: PackageId,
    order: ModOrder,
    base_list: tauri::State<'_, BaseList>,
) -> Result<(), String> {
    info!(
        "前端请求设置用户自定义排序, order: {:?}",
        order
    );
    let mut entry = base_list
        .user_custom_mods_order
        .entry(package_id)
        .or_insert_with(HashSet::new);
    entry.insert(order);
    Ok(())
}

#[tauri::command]
pub async fn sort_remove_user_custom_order(
    package_id: PackageId,
    order: ModOrder,
    base_list: tauri::State<'_, BaseList>,
) -> Result<(), String> {
    info!(
        "前端请求移除用户自定义排序, order: {:?}",
        order
    );
    if let Some(mut entry) = base_list.user_custom_mods_order.get_mut(&package_id) {
        entry.retain(|x| x != &order);
    }
    Ok(())
}

#[tauri::command]
pub async fn sort_get_user_custom_order(
    base_list: tauri::State<'_, BaseList>,
) -> Result<HashMap<PackageId, HashSet<ModOrder>>, String> {
    info!(
        "前端请求获取用户自定义排序"
    );
    Ok(base_list
        .user_custom_mods_order
        .iter()
        .map(|r| (r.key().clone(), r.value().clone()))
        .collect()
    )
}

use std::path::Path;
use once_cell::sync::Lazy;

static POSSIBLE_FILENAMES: Lazy<Vec<String>> = Lazy::new(|| {
    let possible_names = ["preview", "Preview"];
    let possible_extensions = ["jpg", "png", "jpeg", "bmp", "gif"];
    possible_names.iter().flat_map(|filename| {
        possible_extensions.iter().map(move |ext| {
            format!("{}.{}", filename, ext)
        })
    }).collect()
});

#[tauri::command]
pub fn find_preview_image(mod_path: &str) -> Option<String> {
    if mod_path.is_empty() {
        return None;
    }

    let base_path = Path::new(mod_path);
    
    for filename in POSSIBLE_FILENAMES.iter() {
        let file_path = base_path.join(filename);
        if file_path.exists() {
            return file_path.to_str().map(|s| s.to_string());
        }
    }
    
    None
}