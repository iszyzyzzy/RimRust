use futures::stream::StreamExt;
use serde::{Deserialize, Serialize};
use std::{
    collections::VecDeque,
    fmt::Debug,
    sync::{Arc, Weak, atomic::AtomicUsize},
};
use tauri::Emitter;
use tokio::sync::Mutex;
use tracing::{debug, info, warn, instrument};
use dashmap::{DashMap, DashSet};
use ahash::{HashMap, HashSet, HashMapExt, HashSetExt, RandomState};

use async_recursion::async_recursion;

use super::storage::{ModsGroupForSave, ModsGroupItemForSave};
use crate::background_task;
use crate::types::*;

pub type Mod = Arc<ModWrapper>;

#[derive(Default, Debug)]
pub struct ModWrapper {
    inner: Mutex<ModInner>,
    tx: Option<tokio::sync::mpsc::Sender<SyncMessage>>,
}

impl ModWrapper {
    pub fn new_without_app_handles(inner: ModInner) -> Self {
        Self {
            inner: Mutex::new(inner),
            tx: None,
        }
    }
    pub fn _with_tx(mut self, tx: tokio::sync::mpsc::Sender<SyncMessage>) -> Self {
        self.tx = Some(tx);
        self
    }
    pub async fn new(inner: ModInner, tx: tokio::sync::mpsc::Sender<SyncMessage>) -> Self {
        let t = inner.clone();
        tx
            .send(
                SyncMessage {
                    id: next_sync_id(),
                    operation: SyncOperation::ModSync(
                        (t.id.clone(), ModSyncOperation::Add(t)),
                    ),
                },
            )
            .await
            .unwrap();
        Self {
            inner: Mutex::new(inner),
            tx: Some(tx),
        }
    }
    pub async fn lock(self: &Arc<Self>) -> ModGuard<'_> {
        if self.tx.is_none() {
            warn!("ModWrapper没有tx");
        }
        ModGuard {
            inner: self.inner.lock().await,
            changes: Vec::new(),
            tx: self.tx.clone(),
            sync_message_id: next_sync_id(),
        }
    }
}

impl Serialize for ModWrapper {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::ser::Serializer,
    {
        self.inner.blocking_lock().serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for ModWrapper {
    fn deserialize<D>(deserializer: D) -> Result<ModWrapper, D::Error>
    where
        D: serde::de::Deserializer<'de>,
    {
        ModInner::deserialize(deserializer).map(|inner| ModWrapper::new_without_app_handles(inner))
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub enum ModChange {
    Name(String),
    Author(String),
    Description(VersionMap<String>),
    Dependencies(VersionMap<HashSet<ModDependency>>),
    SupportedVersion(VersionMap<()>),
    LoadOrder(VersionMap<HashSet<ModOrder>>),
    IncompatibleWith(VersionMap<HashSet<PackageId>>),
    SupportLanguages(VersionMap<HashSet<String>>),
    Enabled(bool),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SyncMessage {
    pub id: usize,
    pub operation: SyncOperation,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum SyncOperation {
    ModSync((Id, ModSyncOperation)),
    TranslationSync(TranslationSyncOperation),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum ModSyncOperation {
    Add(ModInner),
    Update(Vec<ModChange>),
    Remove(Id),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
// 前端invoke的都是乐观更新不用发sync，可以绕过这个直接修改
pub enum TranslationSyncOperation {
    AddUnconfirmed((Id, Vec<(Id, f64)>)),
    RemoveUnconfirmed(Id),
    AddMatch((Id, Id)),
    RemoveMatch(Id),
    AddUserIgnore(Id),
    RemoveUserIgnore(Id),
    AddTranPack(Id),
    RemoveTranPack(Id),
    Remove(Id),
}

pub struct ModGuard<'a> {
    inner: tokio::sync::MutexGuard<'a, ModInner>,
    changes: Vec<ModChange>,
    tx: Option<tokio::sync::mpsc::Sender<SyncMessage>>,
    sync_message_id: usize,
}

impl<'a> ModGuard<'a> {
    pub fn change(&mut self, change: ModChange) {
        self.changes.push(change);
    }
    pub fn change_mult(&mut self, changes: Vec<ModChange>) {
        self.changes.extend(changes);
    }
}

impl<'a> Drop for ModGuard<'a> {
    fn drop(&mut self) {
        if !self.changes.is_empty() {
            for change in &self.changes {
                self.inner.apply_change(change.clone());
            }
            let sync_message = SyncMessage {
                id: self.sync_message_id,
                operation: SyncOperation::ModSync((
                    self.inner.id.clone(),
                    ModSyncOperation::Update(self.changes.clone()),
                )),
                //operation: SyncOperation::Update(self.changes.clone()),
            };
            if let Some(tx) = &self.tx {
                //tx.blocking_send(sync_message).unwrap();
                let tx = tx.clone();
                let sync_message = sync_message;
                tokio::task::spawn_blocking(move || {
                    tx.blocking_send(sync_message).unwrap();
                });
            }
        }
    }
}

use std::ops::{Deref, DerefMut};
impl Deref for ModGuard<'_> {
    type Target = ModInner;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}
impl DerefMut for ModGuard<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

#[derive(Serialize, Deserialize, Clone, Default, Debug, bincode::Decode, bincode::Encode)]
#[serde(rename_all = "camelCase")]
pub struct ModInner {
    pub id: Id,
    pub enabled: bool,
    pub visible: bool,
    pub package_id: PackageId,
    pub name: String,
    pub author: String,
    pub display_name: String,
    pub description: VersionMap<String>, // <version, description>
    pub dependencies: VersionMap<HashSet<ModDependency>>, // <version, dependencies>
    pub supported_version: VersionMap<()>,
    pub path: String,
    pub load_order: VersionMap<HashSet<ModOrder>>, // <version, order>
    pub incompatible_with: VersionMap<HashSet<PackageId>>, // <version, package_id>
    // 这几个带version的匹配顺序应该是: 当前版本 > * > 过时版本
    // 在VersionMap实现啦
    pub support_languages: VersionMap<HashSet<String>>,
}

impl ModInner {
    pub fn apply_change(&mut self, change: ModChange) {
        match change {
            ModChange::Name(name) => 
                self.name = name,
            ModChange::Author(author) => 
                self.author = author,
            ModChange::Description(description) => 
                self.description = description,
            ModChange::Dependencies(dependencies) => 
                self.dependencies = dependencies,
            ModChange::SupportedVersion(supported_version) => 
                self.supported_version = supported_version,
            ModChange::LoadOrder(load_order) => 
                self.load_order = load_order,
            ModChange::IncompatibleWith(incompatible_with) => 
                self.incompatible_with = incompatible_with,
            ModChange::SupportLanguages(support_languages) => 
                self.support_languages = support_languages,
            ModChange::Enabled(enabled) =>
                self.enabled = enabled,
        }
    }
    pub fn generate_diff(&self, other: &ModInner) -> Vec<ModChange> {
        let mut changes = Vec::new();
        if self.name != other.name {
            changes.push(ModChange::Name(self.name.clone()));
        }
        if self.author != other.author {
            changes.push(ModChange::Author(self.author.clone()));
        }
        if self.description != other.description {
            changes.push(ModChange::Description(self.description.clone()));
        }
        if self.dependencies != other.dependencies {
            changes.push(ModChange::Dependencies(self.dependencies.clone()));
        }
        if self.supported_version != other.supported_version {
            changes.push(ModChange::SupportedVersion(self.supported_version.clone()));
        }
        if self.load_order != other.load_order {
            changes.push(ModChange::LoadOrder(self.load_order.clone()));
        }
        if self.incompatible_with != other.incompatible_with {
            changes.push(ModChange::IncompatibleWith(self.incompatible_with.clone()));
        }
        if self.support_languages != other.support_languages {
            changes.push(ModChange::SupportLanguages(self.support_languages.clone()));
        }
        // diff的时候忽略display_order
/*         if self.display_order != other.display_order {
            changes.push(ModChange::DisplayOrder(self.display_order));
        } */
        changes
    }
}

#[derive(
    Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Hash, bincode::Decode, bincode::Encode,
)]
#[serde(tag = "type", content = "value")]
pub enum ModOrder {
    Before(PackageId), // target mod package_id
    After(PackageId),
    First(),
    Last(),
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Hash, bincode::Decode, bincode::Encode)]
pub struct ModDependency {
    pub package_id: PackageId,
    pub display_name: Option<String>,
    pub url: Option<String>,
    pub steam_id: Option<SteamId>,
    pub optional: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Hash, bincode::Decode, bincode::Encode)]
pub enum SteamId {
    Appid(String),
    WorkShopId(String)
}

impl ModDependency {
    pub fn new(package_id: String, display_name: Option<String>, url: Option<String>, steam_id: Option<SteamId>, optional: bool) -> Self {
        Self {
            package_id: PackageId::from_str(package_id),
            display_name,
            url,
            steam_id,
            optional,
        }
    }
}

use super::translate::TranslationModData;

#[derive(Default)]
pub struct BaseList {
    pub app_handles: Option<tauri::AppHandle>,
    pub mods_map: Arc<DashMap<Id, Mod, RandomState>>,
    pub mods_order: Arc<Mutex<Vec<Id>>>,
    pub mods_groups_map: DashMap<Id, Arc<Mutex<ModsGroup>>, RandomState>, // <id, group>
    pub mods_groups_order: Arc<Mutex<Vec<Id>>>,
    pub user_custom_mods_order: DashMap<PackageId, HashSet<ModOrder>, RandomState>, // <package_id, order>
    pub community_data: super::community_data::CommunityData,
    pub path_set: DashSet<String, RandomState>,
    pub package_id_to_mod: DashMap<PackageId, Vec<Mod>, RandomState>,
    pub user_ignore_info: DashMap<Id, DashSet<super::scan::InfoType>, RandomState>,
    pub translation_mod_data: PriorityMutex<TranslationModData>,
    pub search_data: PriorityMutex<super::search::SearchData>,
    //pub trans_track: DashMap<Id, usize, RandomState>,
    pub trans_track: PriorityMutex<HashMap<Id, usize>>,
    pub sync_tx: Option<tokio::sync::mpsc::Sender<SyncMessage>>,
    pub auto_save_handle: Arc<Mutex<Option<(tokio::sync::oneshot::Sender<()>, tokio::task::JoinHandle<()>)>>>,
    pub auto_refresh_handle: Arc<Mutex<Option<(tokio::sync::oneshot::Sender<()>, tokio::task::JoinHandle<()>)>>>,
}

//pub type BaseList = Arc<Mutex<BaseListInner>>;
//pub type BaseList = Mutex<BaseListInner>; // TODO 把这个大锁往里拆

impl BaseList {
    pub fn with_app_handles(mut self, app_handles: tauri::AppHandle) -> Self {
        self.app_handles = Some(app_handles);
        // 有了app_handles之后初始化一个用于发送sync消息的channel和一个更新聚合循环
        let (tx, mut rx) = tokio::sync::mpsc::channel(10000);
        let app_handles = self.app_handles.clone().unwrap();
        tokio::spawn(async move {
            let mut updates = VecDeque::new();
            let mut interval = tokio::time::interval(std::time::Duration::from_millis(100));
            interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
            loop {
                interval.tick().await;

                while let Ok(update) = rx.try_recv() {
                    debug!(update = ?update, "收到更新数据");
                    updates.push_back(update);
                }
                
                if !updates.is_empty() {
                    info!("发送更新数据 len={}", updates.len());
                    debug!(updates = ?updates, "发送更新数据");
                    app_handles.emit("sync_many", updates.clone()).unwrap();
                    updates.clear();
                }

            }
        });

        self.sync_tx = Some(tx);
        self
    }
    pub fn get_mod(&self, id: &Id) -> Option<Mod> {
        debug!(id = ?id, "获取mod");
        self.mods_map.get(id).map(|mod_| mod_.value().clone())
    }
    pub fn get_mod_group(
        &self,
        id: &Id,
    ) -> Option<Arc<Mutex<ModsGroup>>> {
        debug!(id = ?id, "获取mod组");
        self.mods_groups_map.get(id).map(|group_| group_.value().clone())
    }
    async fn add_mod(&self, mod_: ModInner, priority: Option<Priority>) {
        info!(id = ?mod_.id, "添加mod");
        debug!(new_mod = ?mod_);
        let mod_ = Arc::new(ModWrapper::new(mod_, self.sync_tx.clone().unwrap()).await);
        let guard = mod_.lock().await;
        self.mods_map
            .insert(guard.id.clone(), mod_.clone());
        self.mods_order.lock().await.push(guard.id.clone());
        self.path_set
            .insert(guard.path.clone());
        self.package_id_to_mod
            .entry(guard.package_id.clone())
            .or_insert_with(Vec::new)
            .push(mod_.clone());
        self.translation_mod_data.lock(priority).await.add(&guard).await;
        self.search_data.lock(priority).await.add(&guard).await;
    }
    async fn update_mod(&self, mod_: ModInner, target_mod_id: &Id, priority: Option<Priority>) {
        debug!(new_mod = ?mod_);
        let old_mod = self.get_mod(target_mod_id).unwrap();
        let mut old_mod_guard = old_mod.lock().await;

        let changes = old_mod_guard.generate_diff(&mod_);
        if changes.is_empty() {
            debug!(id = ?mod_.id, "无变化");
            return;
        }
        //debug!(id = ?mod_.id,changes = ?changes, "有变化");
        info!(id = ?mod_.id,changes = ?changes, "有变化");
        old_mod_guard.change_mult(changes);
        drop(old_mod_guard); // 偷懒.jpg
        let old_mod_guard = old_mod.lock().await;
        info!(id = ?mod_.id, "更新mod");

        self.translation_mod_data.lock(priority).await.update(&old_mod_guard).await;
        self.search_data.lock(priority).await.update(&old_mod_guard).await;
    }
    pub async fn check_and_add_mod(&self, new_mod: ModInner, priority: Option<Priority>) {
        let existing_ids = self.package_id_to_mod
            .get(&new_mod.package_id)
            .map(|inner| inner.value().clone());

        if existing_ids.is_none() {
            debug!(new_mod = ?new_mod, "package_id_set不包含，直接添加");
            self.add_mod(new_mod.clone(), priority).await;
            return;
        }

        let mut target_id = None;
        let existing_mods = existing_ids.unwrap();
        for existing_mod in existing_mods {
            let existing_mod = existing_mod.lock().await;
            if existing_mod.path == new_mod.path {
                target_id = Some(existing_mod.id.clone());
            }
        }

        if target_id.is_none() {
            debug!(?new_mod, "package_id_set包含，但path不同，添加");
            self.add_mod(new_mod, priority).await;
        } else {
            debug!(?new_mod, ?target_id, "package_id_set包含，path相同，更新");
            self.update_mod(new_mod, &target_id.unwrap(), priority).await;
        }
    }
    pub async fn add_mods_batch(&self, mods: Vec<Mod>, priority: Option<Priority>) {
        info!(count = mods.len(), "批量添加mod");
        let mut translation_data = self.translation_mod_data.lock(priority).await;
        let mut search_data = self.search_data.lock(priority).await;

        for mod_ in mods.clone() {
            let guard = mod_.lock().await;
            self.mods_map.insert(guard.id.clone(), mod_.clone());
            self.path_set.insert(guard.path.clone());
            self.package_id_to_mod
                .entry(guard.package_id.clone())
                .or_insert_with(Vec::new)
                .push(mod_.clone());
            translation_data.add(&guard).await;
            search_data.add(&guard).await;
        }
    }
    pub async fn add_mod_group(&self, group: ModsGroupForSave) {
        info!(id = ?group.id, "添加mod组");
        debug!(new_group = ?group);
        let mut new_group = ModsGroup {
            id: Id::from_str(group.id.clone()),
            name: group.name.clone(),
            enabled: false,
            mods: Vec::new(),
        };
        for item in group.mods {
            match item {
                ModsGroupItemForSave::Mod(id) => {
                    new_group.mods.push(ModsGroupItem::Mod(
                        self.mods_map
                            .get(&Id::from_str(id))
                            .unwrap()
                            .value()
                            .clone(),
                    ));
                }
                ModsGroupItemForSave::ModsGroup(id) => {
                    new_group.mods.push(ModsGroupItem::ModsGroup(
                        self.mods_groups_map
                            .get(&Id::from_str(id))
                            .unwrap()
                            .value()
                            .clone(),
                    ));
                }
            }
        }
        let new_group = Arc::new(Mutex::new(new_group));
        let id = Id::from_str(group.id);
        self.mods_groups_map
            .insert(id, new_group.clone());
        self.mods_groups_order.lock().await.push(id);
    }
    async fn add_mod_to_group(&self, group_id: &Id, mod_: Mod) {
        // 这两个fn的log移到下面的add_object_to_group里了，因为在这里拿一次id还得再拿一次锁
        let group = self
            .mods_groups_map
            .get(group_id)
            .unwrap()
            .value()
            .clone();
        group.lock().await.mods.push(ModsGroupItem::Mod(mod_));
    }
    async fn add_group_to_group(
        &self,
        group_id: &Id,
        group_: Arc<Mutex<ModsGroup>>,
    ) {
        let group = self
            .mods_groups_map
            .get(group_id)
            .unwrap()
            .value()
            .clone();
        group
            .lock()
            .await
            .mods
            .push(ModsGroupItem::ModsGroup(group_));
    }
    pub async fn add_object_to_group(
        &self,
        group_id: &Id,
        object_id: &Id,
    ) {
        if self.mods_map.contains_key(object_id) {
            info!(group_id = ?group_id, object_id = ?object_id, "添加mod到组");
            let mod_ = self
                .mods_map
                .get(object_id)
                .unwrap()
                .value()
                .clone();
            self.add_mod_to_group(group_id, mod_).await;
        } else if self
            .mods_groups_map
            .contains_key(object_id)
        {
            info!(group_id = ?group_id, object_id = ?object_id, "添加组到组");
            let group_ = self
                .mods_groups_map
                .get(object_id)
                .unwrap()
                .value()
                .clone();
            self.add_group_to_group(group_id, group_).await;
        } else {
            warn!(group_id = ?group_id, object_id = ?object_id, "未找到对象");
        }
    }
    async fn remove_mod_from_group(&self, group_id: &Id, mod_id: &Id) {
        info!(group_id = ?group_id, mod_id = ?mod_id, "从组中移除mod");
        let group = self
            .mods_groups_map
            .get(group_id)
            .unwrap()
            .value()
            .clone();
        let mods = group.lock().await;
        for (i, item) in mods.mods.iter().enumerate() {
            match item {
                ModsGroupItem::Mod(mod_) => {
                    if mod_.lock().await.id == mod_id {
                        drop(mods); // Release the lock before modification
                        group.lock().await.mods.remove(i);
                        return;
                    }
                }
                _ => continue,
            }
        }
    }
    async fn remove_group_from_group(
        &self,
        group_id: &Id,
        group_id_to_remove: &Id,
    ) {
        info!(group_id = ?group_id, group_id_to_remove = ?group_id_to_remove, "从组中移除组");
        let group = self
            .mods_groups_map
            .get(group_id)
            .unwrap()
            .value()
            .clone();
        let mods = group.lock().await;
        for (i, item) in mods.mods.iter().enumerate() {
            match item {
                ModsGroupItem::ModsGroup(group_) => {
                    if group_.lock().await.id == group_id_to_remove {
                        drop(mods); // Release the lock before modification
                        group.lock().await.mods.remove(i);
                        return;
                    }
                }
                _ => continue,
            }
        }
    }
    pub async fn remove_object_from_group(
        &self,
        group_id: &Id,
        object_id: &Id,
    ) {
        if self.mods_map.contains_key(object_id) {
            self.remove_mod_from_group(group_id, object_id)
                .await;
        } else if self
            .mods_groups_map
            .contains_key(object_id)
        {
            self.remove_group_from_group(group_id, object_id)
                .await;
        } else {
            warn!(group_id = ?group_id, object_id = ?object_id, "未找到对象");
        }
    }
    /*     async fn remove_mod(&mut self, mod_id: &Id, priority: Option<Priority>) {
        let package_id = self
            .mods_map
            .lock(priority).await
            .get(mod_id)
            .unwrap()
            .lock()
            .await
            .package_id
            .clone();
        let mut package_id_count = 0;
        for mod_ in self.mods.lock(priority).await.iter() {
            if mod_.lock().await.package_id == package_id {
                package_id_count += 1;
            }
        }
        let path = self.mods_map.lock(priority).await.get(mod_id).unwrap().lock().await.path.clone();
        let mut path_count = 0;
        for mod_ in self.mods.lock(priority).await.iter() {
            if mod_.lock().await.path == path {
                path_count += 1;
            }
        }
        for (i, mod_) in self.mods.lock(priority).await.iter().enumerate() {
            if mod_.lock().await.id == mod_id {
                self.mods.lock(priority).await.remove(i);
                break;
            }
        }
        self.mods_map.lock(priority).await.remove(mod_id);
        let group_ids: Vec<Id> = futures::stream::iter(self.mods_groups.lock(priority).await.iter())
            .then(|group| async move { group.lock().await.id.clone() })
            .collect()
            .await;
        for group_id in group_ids {
            self.remove_mod_from_group(&group_id, mod_id, priority).await;
        }
        if package_id_count == 1 {
            self.package_id_set.lock(priority).await.remove(&package_id);
        }
        if path_count == 1 {
            self.path_set.lock(priority).await.remove(&path);
        }
    } */
    async fn remove_mod(&self, mod_id: &Id, priority: Option<Priority>) {
        info!(mod_id = ?mod_id, "移除mod");
        // 先把它disable了防止错误的跟踪数据
        if let Err(e) = self.set_enable_mod(mod_id, false, priority).await {
            warn!(mod_id = ?mod_id, error = ?e, "移除mod前禁用失败，继续执行移除流程");
        }
        // 1. 获取必要的初始数据
        let mod_arc = {
            let mod_entry = self.mods_map.get(mod_id).ok_or_else(|| {
                format!("mod不存在，无法移除: {:?}", mod_id)
            });
            match mod_entry {
                Ok(entry) => entry.value().clone(),
                Err(e) => {
                    warn!(error = ?e);
                    return;
                }
            }
        };
        let mod_info = mod_arc.lock().await;
        let package_id = mod_info.package_id.clone();
        let path = mod_info.path.clone();

        // add. 从翻译数据和搜索数据中移除
        self.translation_mod_data
            .lock(priority)
            .await
            .remove(&mod_info);
        {
            let mut search_data = self.search_data.lock(priority).await;
            if tokio::time::timeout(
                std::time::Duration::from_secs(15),
                search_data.remove(&mod_info.id.clone()),
            )
            .await
            .is_err()
            {
                warn!(id = ?mod_info.id, "移除搜索索引超时，跳过并继续清理流程");
            }
        }
        drop(mod_info);
        info!(mod_id = ?mod_id, "已从翻译数据和搜索数据中移除");

        // 3. 移除mod
        self.mods_map.remove(mod_id);
        info!(mod_id = ?mod_id, "已从mods_map中移除");

        // 4. 从组中移除
        let group_ids: Vec<Id> = self
            .mods_groups_map
            .iter()
            .map(|item| item.key().clone())
            .collect();

        for group_id in group_ids {
            self.remove_mod_from_group(&group_id, mod_id)
                .await;
        }
        info!(mod_id = ?mod_id, "已从所有mod组中移除");

        // 5. 更新 package_id_to_mod 和 path_set
        let mods_same_package = self
            .package_id_to_mod
            .get(&package_id)
            .map(|entry| entry.value().clone())
            .unwrap_or_default();

        let mut retained = Vec::with_capacity(mods_same_package.len());
        for mod_ in mods_same_package {
            if mod_.lock().await.id != *mod_id {
                retained.push(mod_);
            }
        }

        if retained.is_empty() {
            self.package_id_to_mod.remove(&package_id);
        } else {
            self.package_id_to_mod.insert(package_id.clone(), retained);
        }

        self.path_set.remove(&path);
        info!(mod_id = ?mod_id, "从set中移除");
    }
    pub async fn remove_mod_by_path(&self, mod_path: &str, priority: Option<Priority>) -> usize {
        let entries: Vec<(Id, Mod)> = self
            .mods_map
            .iter()
            .map(|item| (item.key().clone(), item.value().clone()))
            .collect();

        let mut target_ids = Vec::new();
        for (id, mod_) in entries {
            if mod_.lock().await.path == mod_path {
                target_ids.push(id);
            }
        }

        let removed_count = target_ids.len();
        for mod_id in target_ids {
            self.remove_mod(&mod_id, priority).await;
        }

        removed_count
    }
    pub async fn remove_mod_group(&self, group_id: &Id) {
        warn!(group_id = ?group_id, "移除mod组");
        self.mods_groups_map.remove(group_id);
        let group_ids: Vec<Id> = futures::stream::iter(self.mods_groups_map.iter())
            .then(|item| async move { item.value().lock().await.id.clone() })
            .collect()
            .await;
        for group_id in group_ids {
            self.remove_group_from_group(&group_id, &group_id)
                .await;
        }
        self.mods_groups_map.remove(group_id);
    }
    pub async fn rename_mod_group(
        &self,
        group_id: &Id,
        new_name: &str,
    ) {
        info!(group_id = ?group_id, new_name = ?new_name, "重命名mod组");
        let group = self
            .mods_groups_map
            .get(group_id)
            .unwrap()
            .value()
            .clone();
        group.lock().await.name = new_name.to_string();
    }
    pub async fn set_mod_display_name(
        &self,
        mod_id: &Id,
        new_name: &str,
        priority: Option<Priority>,
    ) {
        info!(mod_id = ?mod_id, new_name = ?new_name, "设置mod显示名称");
        let mod_ = self.get_mod(mod_id).unwrap();
        mod_.lock().await.display_name = new_name.to_string();
        let t = mod_.lock().await;
        self.search_data.lock(priority).await.update(&t).await;
    }
    #[async_recursion]
    pub async fn set_enable_mod(
        &self,
        mod_id: &Id,
        enabled: bool,
        priority: Option<Priority>,
    ) -> Result<(), String> {
        info!(mod_id = ?mod_id, enabled, "设置mod启用状态");
        let mod_ = self
            .get_mod(mod_id)
            .ok_or("Mod not found")?;
        let language_pack = self
            .translation_mod_data
            .lock(priority)
            .await
            .get(mod_id)
            .cloned();

        if let Some(language_pack) = language_pack {
            let mut should_propagate = false;
            {
                let mut trans_track = self.trans_track.lock(priority).await;
                let entry = trans_track.entry(language_pack.clone()).or_insert(0);
                if enabled {
                    *entry += 1;
                    should_propagate = true;
                } else {
                    if *entry > 0 {
                        *entry -= 1;
                    }
                    if *entry == 0 {
                        trans_track.remove(&language_pack);
                        should_propagate = true;
                    }
                }
            }

            if should_propagate {
                self.set_enable_mod(&language_pack, enabled, priority)
                    .await?;
            }
        }
        mod_.lock().await.change(ModChange::Enabled(enabled));
        Ok(())
    }
    pub async fn set_enable_mod_group(
        &self,
        group_id: &Id,
        enabled: bool,
        priority: Option<Priority>,
    ) -> Result<(), String> {
        info!(group_id = ?group_id, enabled, "设置mod组启用状态");
        let trans_data = self.translation_mod_data.lock(priority).await;
        let group = self
            .get_mod_group(group_id)
            .ok_or("Group not found")?;
        let mut trans_track = self.trans_track.lock(priority).await;
        group
            .lock()
            .await
            .change_enable(enabled, &self.mods_map, &trans_data, &mut trans_track)
            .await?;
        Ok(())
    }
    pub async fn change_mod_display_order(
        &self,
        from_index: usize,
        to_index: usize,
    ) {
        info!(from_index, to_index, "设置mod显示顺序");
        let mut mods_order = self.mods_order.lock().await;
        if from_index >= mods_order.len() || to_index >= mods_order.len() {
            warn!("索引超出范围");
            return;
        }
        let mod_id = mods_order.remove(from_index);
        mods_order.insert(to_index, mod_id);
    }
    /// 清理mod显示顺序中的无效mod id
    pub async fn clean_mods_display_order(&self) {
        info!("清理mod显示顺序中的无效mod id");
        let mut mods_order = self.mods_order.lock().await;
        mods_order.retain(|mod_id| self.mods_map.contains_key(mod_id));
    }
    pub async fn get_mods_display_order(&self) -> Vec<Id> {
        self.mods_order.lock().await.clone()
    }
    pub async fn change_mods_group_display_order(
        &self,
        from_index: usize,
        to_index: usize,
    ) {
        info!(from_index, to_index, "设置mod组显示顺序");
        let mut mods_groups_order = self.mods_groups_order.lock().await;
        if from_index >= mods_groups_order.len() || to_index >= mods_groups_order.len() {
            warn!("索引超出范围");
            return;
        }
        let group_id = mods_groups_order.remove(from_index);
        mods_groups_order.insert(to_index, group_id);
    }
    pub async fn change_mods_group_inner_display_order(
        &self,
        group_id: &Id,
        from_index: usize,
        to_index: usize,
    ) -> Result<(), String> {
        info!(group_id = ?group_id, from_index, to_index, "设置mod组内显示顺序");
        let group = self
            .get_mod_group(group_id)
            .ok_or("Group not found")?;
        let mut group = group.lock().await;
        if from_index >= group.mods.len() || to_index >= group.mods.len() {
            warn!("索引超出范围");
            return Err("Index out of bounds".to_string());
        }
        let item = group.mods.remove(from_index);
        group.mods.insert(to_index, item);
        Ok(())
    }
    pub async fn clean_mods_group_inner(
        &self,
        group_id: &Id,
    ) -> Result<(), String> {
        info!(group_id = ?group_id, "清理mod组内无效对象");
        let group = self
            .get_mod_group(group_id)
            .ok_or("Group not found")?;
        let mut group = group.lock().await;
        let mut valid_items = Vec::new();
        for item in group.mods.drain(..) {
            match &item {
                ModsGroupItem::Mod(mod_) => {
                    let mod_guard = mod_.lock().await;
                    if self.mods_map.contains_key(&mod_guard.id) {
                        valid_items.push(item.clone());
                    }
                }
                ModsGroupItem::ModsGroup(group_) => {
                    let group_guard = group_.lock().await;
                    if self.mods_groups_map.contains_key(&group_guard.id) {
                        valid_items.push(item.clone());
                    }
                }
            }
        }
        group.mods = valid_items;
        Ok(())
    }
    pub async fn clean_mods_groups_inner(&self) {
        info!("清理mod组内无效对象");
        let group_ids: Vec<Id> = self.mods_groups_map.iter().map(|item| item.key().clone()).collect();
        for group_id in group_ids {
            self.clean_mods_group_inner(&group_id).await.unwrap();
        }
    }
    pub async fn get_mods_groups_display_order(&self) -> Vec<Id> {
        self.mods_groups_order.lock().await.clone()
    }

    #[instrument(skip(self, app_config, status), fields(task_id = %status.get_task_id()))]
    pub async fn refresh_mods_data(
        &self,
        app_config: &crate::AppConfig,
        status: &mut background_task::TaskStatusAdd,
    ) -> Result<(Vec<Id>, Vec<Id>, HashMap<Id, Vec<(Id, f64)>>), String> {
        let priority = Priority::HIGH;
        let steam_db = self.community_data.get_steam_db();
        info!("开始刷新mod数据");

        status.update_info("扫描现有mod");
        status.update_progress(20.0);

        // 0. 检查现有mod
        let mut invalid_mods = Vec::new();
        let mut mods_to_update = Vec::new();
        {
            for mod_ in self.mods_map.iter().map(|item| item.value().clone()) {
                let mod_guard = mod_.lock().await;
                let path = &mod_guard.path;

                if !std::fs::exists(path).unwrap() {
                    invalid_mods.push(mod_guard.id.clone());
                    continue;
                }

                // 重新读取mod元数据
                if let Ok(new_mod) = crate::file::reader::load_mod_from_path(
                    path,
                    status,
                    steam_db.clone(),
                    priority,
                ).await {
                    mods_to_update.push(new_mod);
                }
            }
        }
        debug!(invalid_mods = ?invalid_mods, mods_to_update = ?mods_to_update, "raw");

        // 1. 更新mod
        status.update_info("更新mod");
        status.update_progress(40.0);
        for new_mod in mods_to_update {
            self.check_and_add_mod(new_mod, priority).await;
        }

        // 2. 移除无效mod
        status.update_info("清理无效mod");
        status.update_progress(60.0);
        for mod_id in invalid_mods {
            self.remove_mod(&mod_id, priority).await;
        }

        // 3. 扫描新mod
        status.update_info("扫描新mod");
        status.update_progress(80.0);

        let paths = vec![
            format!("{}/Mods", app_config.game_path),
            format!("{}/Data", app_config.game_path),
            app_config.steam_mods_path.clone(),
        ];

        for path in paths {
            if !std::fs::exists(&path).unwrap() {
                continue;
            }

            for entry in std::fs::read_dir(path).unwrap() {
                let entry = entry.unwrap();
                let path = entry.path();
                if path.is_dir() {
                    let path_str = path.to_str().unwrap();
                    // 只处理未记录的新mod
                    if !self.path_set.contains(path_str) {
                        if let Ok(new_mod) = crate::file::reader::load_mod_from_path(
                            path_str,
                            status,
                            steam_db.clone(),
                            priority,
                        ).await {
                            self.check_and_add_mod(new_mod, priority).await;
                        }
                    }
                }
            }
        }

        status.update_info("刷新相关数据");
        status.update_progress(90.0);

        self.clean_mods_display_order().await;
        self.clean_mods_groups_inner().await;
        let mod_order = self.get_mods_display_order().await;
        let group_order = self.get_mods_groups_display_order().await;
        let matches = self
            .translation_mod_data
            .lock(priority)
            .await
            .try_rematch(&self.mods_map)
            .await;

        status.update_info("刷新完成");
        status.update_progress(100.0);

        info!("mod数据刷新完成");
        Ok((mod_order, group_order, matches))
    }
    pub async fn reorder_mods_by_name(&self) -> Vec<Id> {
        info!("按名称重新排序mod");
        let mut cache = Vec::new();
        for mod_ in self.mods_map.iter().map(|item| item.value().clone()) {
            let mod_guard = mod_.lock().await;
            cache.push((mod_guard.id.clone(), mod_guard.name.clone()));
        }
        cache.sort_by(|a, b| a.1.to_lowercase().cmp(&b.1.to_lowercase()));
        let mut mods_order = self.mods_order.lock().await;
        mods_order.clear();
        for (id, _) in cache {
            mods_order.push(id);
        }
        mods_order.clone()
    }
}

pub enum ModsGroupItem {
    Mod(Mod),
    ModsGroup(Arc<Mutex<ModsGroup>>),
}

impl Clone for ModsGroupItem {
    fn clone(&self) -> Self {
        match self {
            ModsGroupItem::Mod(mod_) => ModsGroupItem::Mod(mod_.clone()),
            ModsGroupItem::ModsGroup(group_) => ModsGroupItem::ModsGroup(group_.clone()),
        }
    }
}


pub struct ModsGroup {
    pub id: Id,
    pub name: String,
    pub enabled: bool,
    pub mods: Vec<ModsGroupItem>,// 这个就是显示顺序
}

impl ModsGroup {
    #[async_recursion]
    pub async fn change_enable(
        &self,
        enabled: bool,
        mods_map: &DashMap<Id, Mod, RandomState>,
        trans_data: &TranslationModData,
        trans_track: &mut HashMap<Id, usize>,
    ) -> Result<(), String> {
        debug!(self = ?self.id, enabled = ?enabled, "设置mod组启用状态");
        if self.enabled == enabled {
            return Ok(());
        }
        for item in &self.mods {
            match item {
                ModsGroupItem::Mod(mod_) => {
                    let mut mod_guard = mod_.lock().await;
                    mod_guard.change(ModChange::Enabled(enabled));
                    let id = mod_guard.id.clone();
                    drop(mod_guard);
                    if let Some(language_pack) = trans_data.get(&id) {
                        let entry = trans_track.entry(language_pack.clone()).or_insert(0);
                        if enabled {
                            *entry += 1;
                            mods_map
                                .get(&language_pack)
                                .unwrap()
                                .lock()
                                .await
                                .change(ModChange::Enabled(enabled));
                        } else {
                            *entry -= 1;
                            if entry == &0 {
                                trans_track.remove(&language_pack);
                                mods_map
                                    .get(&language_pack)
                                    .unwrap()
                                    .lock()
                                    .await
                                    .change(ModChange::Enabled(enabled));
                            }
                        }
                    }
                }
                ModsGroupItem::ModsGroup(group_) => {
                    group_
                        .lock()
                        .await
                        .change_enable(enabled, mods_map, trans_data, trans_track)
                        .await?;
                }
            }
        }
        Ok(())
    }
}
