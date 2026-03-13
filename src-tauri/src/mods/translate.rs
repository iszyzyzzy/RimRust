mod identifier;
mod matcher;
mod index;
mod rules;
mod auto_translate;

use ahash::RandomState;
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use ahash::{HashMap, HashMapExt};
use tracing::{debug, info, warn};

use super::base_list::*;
use crate::types::*;

pub use auto_translate::{auto_translate, auto_translate_streaming, AutoTranslateResult, AutoTranslateEvent};
pub use index::ModIndex;
pub use rules::LanguagePackRules;

use identifier::Identifyer;
use matcher::Matcher;
use rules::LanguagePackRule;

struct CompiledRule {
    identify: Identifyer,
    matcher: Matcher,
}

impl CompiledRule {
    fn new(rule: &LanguagePackRule, version: Version) -> Self {
        Self {
            identify: Identifyer::new(&rule.identify, version.clone()),
            matcher: Matcher::new(&rule.matches),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, bincode::Decode, bincode::Encode)]
#[serde(tag = "type", content = "value")]
pub enum ModStatus {
    Translation,
    Matched(Id),
    UnconfirmedMatches(Vec<(Id, f64)>),
    Ignored,
    NoMatch,
}

pub struct TranslationModData {
    mod_index: ModIndex,
    rule: Option<CompiledRule>,
    mod_status: HashMap<Id, ModStatus>,
    have_rules: bool,
    game_version: Version,
    sync_tx: Option<tokio::sync::mpsc::Sender<SyncMessage>>,
    auto_translate_cache: Arc<Mutex<lru::LruCache<String, AutoTranslateResult, RandomState>>>,
    ongoing_auto_translate: Arc<Mutex<HashMap<String, Vec<tokio::sync::oneshot::Sender<Result<AutoTranslateResult,String>>>>>>,
    ongoing_auto_translate_streaming: Arc<Mutex<HashMap<String, tokio::sync::broadcast::Sender<AutoTranslateEvent>>>>,
}

impl Default for TranslationModData {
    fn default() -> Self {
        Self {
            mod_index: ModIndex::new(Version::default()),
            rule: None,
            mod_status: HashMap::with_hasher(RandomState::new()),
            have_rules: false,
            game_version: Version::default(),
            sync_tx: None,
            auto_translate_cache: Arc::new(Mutex::new(lru::LruCache::with_hasher(
                std::num::NonZeroUsize::new(Self::MAX_CACHE_SIZE).unwrap(),
                RandomState::new(),
            ))),
            ongoing_auto_translate: Arc::new(Mutex::new(HashMap::with_hasher(RandomState::new()))),
            ongoing_auto_translate_streaming: Arc::new(Mutex::new(HashMap::with_hasher(RandomState::new()))),
        }
    }
}

impl BaseList {
    pub async fn init_translation_mod_data(
        &self,
        app_data_path: &str,
        target_language_code: &str,
        version: Version,
    ) {
        let rules = LanguagePackRules::new(&format!(
            "{}/translation_mod_match_rules.toml",
            app_data_path
        ));
        
        let mut data = self.translation_mod_data.lock_h().await;
        *data = TranslationModData::new(Some(rules), target_language_code, version, self.sync_tx.clone());
    }
    
    pub async fn rematch_translation(&self, priority: Option<Priority>) -> HashMap<Id, Vec<(Id, f64)>> {
        let mut data = self.translation_mod_data.lock(priority).await;
        data.try_rematch(&self.mods_map).await
    }
}

impl TranslationModData {
    const MAX_CACHE_SIZE: usize = 200;
    
    fn new(
        rules: Option<LanguagePackRules>,
        target_language_code: &str,
        version: Version,
        sync_tx: Option<tokio::sync::mpsc::Sender<SyncMessage>>,
    ) -> Self {
        let rule = if let Some(rules) = rules {
            if let Some(rule) = rules.get_lan(target_language_code) {
                info!("预编译规则：{}", target_language_code);
                Some(CompiledRule::new(rule, version.clone()))
            } else {
                warn!("无处理 {} 的有效规则", target_language_code);
                None
            }
        } else {
            None
        };
        
        let have_rules = rule.is_some();
        Self {
            mod_index: ModIndex::new(version.clone()),
            rule,
            mod_status: HashMap::with_hasher(RandomState::new()),
            have_rules,
            game_version: version,
            sync_tx,
            auto_translate_cache: Arc::new(Mutex::new(lru::LruCache::with_hasher(
                std::num::NonZeroUsize::new(Self::MAX_CACHE_SIZE).unwrap(),
                RandomState::new(),
            ))),
            ongoing_auto_translate: Arc::new(Mutex::new(HashMap::with_hasher(RandomState::new()))),
            ongoing_auto_translate_streaming: Arc::new(Mutex::new(HashMap::with_hasher(RandomState::new()))),
        }
    }
    pub async fn recover(&mut self, mod_status:HashMap<Id, ModStatus>, index: ModIndex) {
        info!("恢复数据");
        self.mod_status = mod_status.into_iter().collect();
        self.mod_index = index;
        if let Some(tx) = &self.sync_tx {
            for (id, status) in &self.mod_status {
                match status {
                    ModStatus::Translation => {
                        let _ = tx.send(SyncMessage { 
                            id: next_sync_id(), 
                            operation: SyncOperation::TranslationSync(
                                TranslationSyncOperation::AddTranPack(id.clone())
                            ) 
                        });
                    }
                    ModStatus::Matched(language_pack_id) => {
                        let _ = tx.send(SyncMessage { 
                            id: next_sync_id(), 
                            operation: SyncOperation::TranslationSync(
                                TranslationSyncOperation::AddMatch((id.clone(), language_pack_id.clone()))
                            ) 
                        });
                    }
                    ModStatus::UnconfirmedMatches(matches) => {
                        let _ = tx.send(SyncMessage { 
                            id: next_sync_id(), 
                            operation: SyncOperation::TranslationSync(
                                TranslationSyncOperation::AddUnconfirmed((id.clone(), matches.clone()))
                            ) 
                        });
                    }
                    ModStatus::Ignored => {
                        let _ = tx.send(SyncMessage { 
                            id: next_sync_id(), 
                            operation: SyncOperation::TranslationSync(
                                TranslationSyncOperation::AddUserIgnore(id.clone())
                            ) 
                        });
                    }
                    ModStatus::NoMatch => {}
                }
            }
        }
    }
    pub fn overwrite_version(&mut self, version: Version) {
        self.game_version = version.clone();
        // ModIndex 中的 game_version 是私有的，我们需要重新创建
        // 保留现有数据
        let old_index = std::mem::replace(&mut self.mod_index, ModIndex::new(version));
        // 这里我们需要访问 ModIndex 的字段来迁移数据
        // 由于字段是公开的，我们可以直接访问并复制
        self.mod_index.name_to_id = old_index.name_to_id;
        self.mod_index.id_to_name = old_index.id_to_name;
        self.mod_index.names_by_length = old_index.names_by_length;
        self.mod_index.id_to_package_id = old_index.id_to_package_id;
        self.mod_index.id_to_author = old_index.id_to_author;
        self.mod_index.load_after_map = old_index.load_after_map;
    }
    pub fn save_data(&self) -> HashMap<Id, ModStatus> {
        self.mod_status.iter().map(|(k,v)| (k.clone(), v.clone())).collect()
    }
    pub fn save_index(&self) -> ModIndex {
        self.mod_index.clone()
    }
    pub fn save_auto_translate_cache(&self) -> Vec<(String, AutoTranslateResult)> {
        if let Ok(cache) = self.auto_translate_cache.lock() {
            cache.iter().map(|(k, v)| (k.clone(), v.clone())).collect()
        } else {
            Vec::new()
        }
    }
    pub fn recover_auto_translate_cache(&mut self, cache_data: Vec<(String, AutoTranslateResult)>) {
        if let Ok(mut cache) = self.auto_translate_cache.lock() {
            for (key, value) in cache_data {
                cache.put(key, value);
            }
        }
    }
    fn send_sync(&self, operation: SyncOperation) {
        if let Some(tx) = self.sync_tx.as_ref() {
            let _ = tx.send(SyncMessage { id: next_sync_id(), operation });
        }
    }
    pub fn identify_if_translation(&mut self, mod_: &ModInner) -> IdentifyResult {
        if !self.have_rules {
            return IdentifyResult::NoRules;
        }
        match self.mod_status.get(&mod_.id) {
            Some(ModStatus::Translation) => {
                debug!("Mod '{}' 已被判定为汉化包", mod_.name);
                IdentifyResult::Translation
            }
            Some(ModStatus::Ignored) => {
                debug!("Mod '{}' 在用户忽略列表中", mod_.name);
                IdentifyResult::CheckedNotTranslation
            }
            Some(ModStatus::Matched(_)) => {
                debug!("Mod '{}' 已经过匹配", mod_.name);
                IdentifyResult::CheckedNotTranslation
            }
            Some(ModStatus::UnconfirmedMatches(_)) => {
                debug!("Mod '{}' 已经过匹配", mod_.name);
                IdentifyResult::CheckedNotTranslation
            }
            Some(ModStatus::NoMatch) => {
                debug!("Mod '{}' 不是汉化包且未被检查过", mod_.name);
                IdentifyResult::NotTranslation
            }
            None => {
                info!("尝试为 '{}' 匹配汉化包", mod_.name);
                let identify_result = self.rule.as_ref().unwrap().identify.identify(mod_);
                if identify_result {
                    IdentifyResult::Translation
                } else {
                    IdentifyResult::NotTranslation
                }
            }
        }
    }
    
    pub async fn add(&mut self, mod_: &ModInner) {
        if !self.have_rules {
            return;
        }
        match self.identify_if_translation(mod_) {
            IdentifyResult::Translation => {
                info!("添加index Mod '{}'", mod_.name);
                self.send_sync(SyncOperation::TranslationSync(
                    TranslationSyncOperation::AddTranPack(mod_.id.clone())
                ));
                self.mod_index.add_mod(mod_);
                self.mod_status.insert(mod_.id.clone(), ModStatus::Translation);
            }
            IdentifyResult::CheckedNotTranslation => {}
            IdentifyResult::NotTranslation => {
                debug!("Mod '{}' 不是汉化包且未被检查过", mod_.name);
                self.match_(mod_).await;
            }
            IdentifyResult::NoRules => {
                warn!("无有效规则");
            }
        }
    }
    
    pub fn remove(&mut self, mod_: &ModInner) {
        if !self.have_rules {
            return;
        }
        info!("移除 mod index: '{}'", mod_.name);
        if self.mod_index.contains(mod_.id) {
            self.mod_index.remove_mod(mod_.id);
        }
        if let Some(status) = self.mod_status.get(&mod_.id) {
            match status {
                ModStatus::Matched(_) => {}
                _ => {
                    self.mod_status.remove(&mod_.id);
                }
            }
        }
    }
    pub fn remove_match(&mut self, mod_id: Id) {
        if !self.have_rules {
            return;
        }
        if let Some(status) = self.mod_status.get(&mod_id) {
            match status {
                ModStatus::Matched(_) => {
                    info!("移除匹配: {:?}", mod_id);
                    self.mod_status.remove(&mod_id);
                }
                _ => {}
            }
        }
    }
    pub async fn update(&mut self, mod_: &ModInner) {
        if !self.have_rules {
            //warn!("无有效规则");
            return;
        }
        debug!("更新 mod index '{}'", mod_.name);
        self.remove(mod_);
        self.add(mod_).await;
    }
    pub async fn match_(&mut self, mod_: &ModInner) {
        if !self.have_rules {
            return;
        }

        if let Some(status) = self.mod_status.get(&mod_.id) {
            match status {
                ModStatus::Translation
                | ModStatus::Ignored
                | ModStatus::Matched(_)
                | ModStatus::UnconfirmedMatches(_) => {
                    debug!("Mod '{}' 已处理，跳过匹配", mod_.name);
                    return;
                }
                ModStatus::NoMatch => {
                    debug!("Mod '{}' 不是汉化包且未被检查过", mod_.name);
                }
            }
        }
        
        if let Some(rule) = &self.rule {
            let matches = rule.matcher.match_mod(&self.mod_index, mod_);
            
            if !matches.is_empty() {
                let len = matches.len();
                self.mod_status
                    .insert(mod_.id.clone(), ModStatus::UnconfirmedMatches(matches.clone()));
                self.send_sync(SyncOperation::TranslationSync(
                    TranslationSyncOperation::AddUnconfirmed((mod_.id.clone(), matches)),
                ));
                debug!("匹配成功，等待用户确认: total={}", len);
                info!("匹配成功，等待用户确认");
            } else {
                info!("未匹配到汉化包");
            }
        }
    }
    pub fn confirm(&mut self, mod_id: Id, language_pack_id: Id) {
        // 这个操作其实不需要两个mod在unconfirmed里，所以也可以用于用户直接添加
        if !self.have_rules {
            return;
        }
        info!("确认匹配 {:?} -> {:?}", mod_id, language_pack_id);
        self.mod_status.insert(mod_id, ModStatus::Matched(language_pack_id));
    }
    pub fn get(&self, mod_id: &Id) -> Option<&Id> {
        match self.mod_status.get(mod_id) {
            Some(ModStatus::Matched(language_pack_id)) => Some(language_pack_id),
            _ => None,
        }
    }
    pub fn get_all_match(&self) -> HashMap<Id, Id> {
        self.mod_status.iter().filter_map(|(k, v)| match v {
            ModStatus::Matched(language_pack_id) => Some((k.clone(), language_pack_id.clone())),
            _ => None,
        }).collect()
    }
    pub fn get_all_unconfirmed(&self) -> HashMap<Id, Vec<(Id, f64)>> {
        self.mod_status.iter().filter_map(|(k, v)| match v {
            ModStatus::UnconfirmedMatches(matches) => Some((k.clone(), matches.clone())),
            _ => None,
        }).collect()
    }
    pub fn get_all_translation_pack(&self) -> Vec<Id> {
        self.mod_index.id_to_package_id.keys().cloned().collect()
    }
    pub async fn try_rematch(&mut self, mod_map: &DashMap<Id, Mod, RandomState>) -> HashMap<Id, Vec<(Id, f64)>> {
        if !self.have_rules {
            return HashMap::new();
        }
        if !self.mod_index.change {
            return HashMap::new();
        }
        let mut mods = Vec::new();
        for id in mod_map.iter().map(|item| item.key().clone()) {
            match self.mod_status.get(&id) {
                Some(ModStatus::Matched(_)) | Some(ModStatus::Translation) | Some(ModStatus::Ignored) => {
                    continue;
                }
                _ => {}
            }
            if let Some(mod_) = mod_map.get(&id) {
                mods.push(mod_.clone());
            }
        }
        self.mod_status = self.mod_status.iter().filter_map(|(k, v)| match v {
            ModStatus::Matched(_) | ModStatus::Translation | ModStatus::Ignored => Some((k.clone(), v.clone())),
            _ => None,
        }).collect();
        for mod_ in mods {
            let mod__ = mod_.lock().await;
            self.match_(&mod__).await;
        }
        self.mod_index.reset_change();
        self.get_all_unconfirmed()
    }
    pub fn add_user_ignore(&mut self, mod_: &ModInner) {
        info!("添加用户忽略: {}", mod_.name);
        self.remove(mod_);
        self.mod_status.insert(mod_.id.clone(), ModStatus::Ignored);
        self.send_sync(SyncOperation::TranslationSync(
            TranslationSyncOperation::AddUserIgnore(mod_.id.clone())
        ) );
    }
    pub fn remove_user_ignore(&mut self, mod_id: Id) {
        info!("移除用户忽略: {:?}", mod_id);
        if let Some(ModStatus::Ignored) = self.mod_status.get(&mod_id) {
            self.mod_status.remove(&mod_id);
            self.send_sync(SyncOperation::TranslationSync(
                TranslationSyncOperation::RemoveUserIgnore(mod_id)
            ) );
        }
    }
    pub fn add_translation_pack(&mut self, mod_: &ModInner) {
        info!("添加汉化包: {}", mod_.name);
        self.mod_status.insert(mod_.id.clone(), ModStatus::Translation);
        self.mod_index.add_mod(mod_);
    }
    
    pub fn remove_translation_pack(&mut self, mod_: &ModInner) {
        info!("移除汉化包: {}", mod_.name);
        self.mod_index.remove_mod(mod_.id);
        self.mod_status.insert(mod_.id.clone(), ModStatus::Ignored);
        self.send_sync(SyncOperation::TranslationSync(
            TranslationSyncOperation::RemoveTranPack(mod_.id.clone())
        ));
    }

    pub fn get_auto_translate_cache(&self) -> 
    (Arc<Mutex<lru::LruCache<String, AutoTranslateResult, RandomState>>>,
    Arc<Mutex<HashMap<String, Vec<tokio::sync::oneshot::Sender<Result<AutoTranslateResult,String>>>>>>,
    Arc<Mutex<HashMap<String, tokio::sync::broadcast::Sender<AutoTranslateEvent>>>>) {
        (
            self.auto_translate_cache.clone(),
            self.ongoing_auto_translate.clone(),
            self.ongoing_auto_translate_streaming.clone(),
        )
    }

    pub fn custom_calc(&self, source: &ModInner, target: &ModInner) -> CustomCalcResult {
        if !self.have_rules || self.rule.is_none() {
            return CustomCalcResult {
                overall_score: 0.0,
                threshold: 0.0,
                matcher_result: None,
                additional: HashMap::new(),
            };
        }

        let rule = self.rule.as_ref().unwrap();
        let matcher_result = rule.matcher.custom_calc(source, target);
        
        let mut additional = HashMap::new();
        
        // 检查 load_after
        let load_after_match = if let Some(ids) = self.mod_index.load_after_map.get(&source.package_id) {
            ids.contains(&target.id)
        } else {
            false
        };
        additional.insert("load_after_match".to_string(), load_after_match.to_string());
        
        let internal_status = self.mod_status.get(&source.id);
        let internal_status_str = match internal_status {
            Some(inner) => serde_json::to_string(inner).unwrap(),
            None => "None".to_string(),
        };
        additional.insert("internal_status".to_string(), internal_status_str);
        
        CustomCalcResult {
            overall_score: matcher_result.score,
            threshold: 0.0, // 可以从规则中获取
            matcher_result: Some(matcher_result),
            additional,
        }
    }
}

#[derive(Debug, serde::Serialize)]
pub struct CustomCalcResult {
    overall_score: f64,
    threshold: f64,
    matcher_result: Option<matcher::MatcherResult>,
    additional: HashMap<String, String>,
}

pub enum IdentifyResult {
    NoRules,
    Translation,
    CheckedNotTranslation,
    NotTranslation,
}
