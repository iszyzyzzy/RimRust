mod identifier;
mod matcher;
mod index;
mod rules;

use ahash::RandomState;
use dashmap::DashMap;
use regex::{Regex, RegexSet};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap, sync::{Arc, Mutex}, vec
};
use ahash::{HashMap, HashSet, HashMapExt, HashSetExt};
use tracing::{debug, info, warn};

use super::base_list::*;
use crate::types::*;

// 示例规则文件 trancslation_mod_match_rules.toml
static EXAMPLE: &'static str = r#"
version = '1.0'

[[rules]]
language_code = 'zh'

[rules.identify]
threshold = 1.0

  [[rules.identify.patterns]]
  pattern_type = 'name'
  pattern = '(?i)(chinese|中文|汉化|_zh|zh-pack|zh|cn)'
  weight = 0.5
  is_regex = true

  [[rules.identify.patterns]]
  pattern_type = 'package_id'
  pattern = '(?i)\b\.?(zh|cn|chinesepack)\.?\b'
  weight = 0.5
  is_regex = true

  [[rules.identify.patterns]]
  pattern_type = 'package_id'
  pattern = '(?i)^rwzh\.'
  weight = 1.0
  is_regex = true

  [rules.identify.support_languages]
  support_languages = ['ChineseSimplified']
  weight = 0.4

  [rules.identify.author_patterns]
    known_authors = ['leafzxg']
    weight = 1.0

[rules.matches]
load_after_match = true
threshold = 0.6

  [rules.matches.candidates]
  filter = true
  

  [rules.matches.name]
  cleanup = true
  direct_match = true
  similarity = true
  similarity_weight = 1.0

    [[rules.matches.name.cleanup_patterns]]
    pattern = '(?i)(chinese|中文|汉化|_zh|zh-pack|zh|cn)'
    replace = ''

    [[rules.matches.name.cleanup_patterns]]
    pattern = '_'
    replace = ' '

  [rules.matches.package_id]
  cleanup = true
  direct_match = true
  direct_match_threshold = 2
  similarity = true
  similarity_weight = 1.0

    [[rules.matches.package_id.cleanup_patterns]]
    pattern = '(?i)\b\.?(zh|cn|chinesepack)\.?\b'
    replace = ''

    [[rules.matches.package_id.cleanup_patterns]]
    pattern = '(?i)^rwzh\.'
    replace = ''
"#;

#[derive(Serialize, Deserialize, Clone)]
pub struct LanguagePackRules {
    pub version: String,
    pub rules: Vec<LanguagePackRule>,
}

impl Default for LanguagePackRules {
    fn default() -> Self {
        toml::from_str(EXAMPLE).unwrap()
    }
}

impl LanguagePackRules {
    fn get_lan(&self, language_code: &str) -> Option<&LanguagePackRule> {
        let t = self
            .rules
            .iter()
            .find(|rule| rule.language_code == language_code);
        if t.is_none() {
            warn!("无有效规则, code: '{}'", language_code);
        }
        t
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct LanguagePackRule {
    pub language_code: String,
    pub identify: IdentifyRule,
    pub matches: MatchRule,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct IdentifyRule {
    pub threshold: f64,
    pub patterns: Vec<IdentifyPattern>,
    pub support_languages: IdentifyFileStructureRule,
    pub author_patterns: IdentifyAuthorPatternRule,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct IdentifyPattern {
    pub pattern_type: IdentifyPatternType,
    pub pattern: String,
    pub weight: f64,
    #[serde(default)]
    pub is_regex: bool,
}

#[derive(Serialize, Deserialize, Clone)]
pub enum IdentifyPatternType {
    #[serde(rename = "name")]
    Name,
    #[serde(rename = "package_id")]
    PackageId,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct IdentifyFileStructureRule {
    pub support_languages: Vec<String>,
    pub weight: f64,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct IdentifyAuthorPatternRule {
    pub known_authors: Vec<String>,
    pub weight: f64,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct MatchRule {
    pub load_after_match: bool,
    pub threshold: f64,
    pub name: MatchRuleName,
    pub package_id: MatchRulePackageId,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct MatchRuleName {
    pub cleanup: bool,
    pub direct_match: bool,
    pub similarity: bool,
    pub similarity_weight: f64,
    pub cleanup_patterns: Vec<CleanupPattern>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct CleanupPattern {
    pub pattern: String,
    pub replace: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct MatchRulePackageId {
    pub similarity: bool,
    pub similarity_weight: f64,
    pub direct_match: bool,
    pub direct_match_threshold: usize,
    pub cleanup: bool,
    pub cleanup_patterns: Vec<CleanupPattern>,
}

struct PreCompiledRule {
    identify: PreCompiledIdentifyer,
    matches: PreCompiledMatcher,
}

impl PreCompiledRule {
    fn new(rule: &LanguagePackRule, version: Version) -> Self {
        Self {
            identify: PreCompiledIdentifyer::new(&rule.identify, version.clone()),
            matches: PreCompiledMatcher::new(&rule.matches, version),
        }
    }
}

struct PreCompiledIdentifyer {
    threshold: f64,
    name_patterns: (RegexSet, Vec<f64>),
    package_id_patterns: (RegexSet, Vec<f64>),
    support_languages: (HashSet<String>, f64),
    author_patterns: (Vec<String>, f64),
    game_version: Version,
}

impl PreCompiledIdentifyer {
    fn new(rule: &IdentifyRule, game_version: Version) -> Self {
        let mut name_patterns = Vec::new();
        let mut name_weights = Vec::new();
        let mut package_patterns = Vec::new();
        let mut package_weights = Vec::new();

        for pattern in &rule.patterns {
            match pattern.pattern_type {
                IdentifyPatternType::Name => {
                    name_patterns.push(pattern.pattern.clone());
                    name_weights.push(pattern.weight);
                }
                IdentifyPatternType::PackageId => {
                    package_patterns.push(pattern.pattern.clone());
                    package_weights.push(pattern.weight);
                }
            }
        }

        Self {
            threshold: rule.threshold,
            name_patterns: (RegexSet::new(name_patterns).unwrap(), name_weights),
            package_id_patterns: (RegexSet::new(package_patterns).unwrap(), package_weights),
            support_languages: (
                rule.support_languages.support_languages.iter().cloned().collect(),
                rule.support_languages.weight,
            ),
            author_patterns: (
                rule.author_patterns.known_authors.clone(),
                rule.author_patterns.weight,
            ),
            game_version
        }
    }

    fn scoring(&self, mod_: &ModInner) -> f64 {
        let mut score = 0.0;

        // 使用 RegexSet 一次性匹配 name
        let name_matches = self.name_patterns.0.matches(&mod_.name);
        for idx in name_matches.into_iter() {
            score += self.name_patterns.1[idx];
        }

        // 使用 RegexSet 一次性匹配 package_id
        let package_matches = self
            .package_id_patterns
            .0
            .matches(&mod_.package_id.to_string());
        for idx in package_matches.into_iter() {
            score += self.package_id_patterns.1[idx];
        }

        if let Some(support_languages) = mod_.support_languages.get(&self.game_version) {
            if support_languages.iter().any(|v| support_languages.contains(&v.to_string())) {
                score += self.support_languages.1;
            }
        }

        if self
            .author_patterns
            .0
            .iter()
            .any(|author| mod_.author.contains(author))
        {
            score += self.author_patterns.1;
        }

        score
    }

    fn identify(&self, mod_: &ModInner) -> bool {
        let score = self.scoring(mod_);
        debug!(id = ?mod_.id, "Mod '{}' 语言包判断得分 {}", mod_.name, score);
        score >= self.threshold
    }
}

struct PreCompiledMatcher {
    load_after_match: bool,
    threshold: f64,
    name: PreCompiledMatcherName,
    package_id: PreCompiledMatcherPackageId,
    game_version: Version,
}

impl PreCompiledMatcher {
    fn new(rule: &MatchRule, version: Version) -> Self {
        Self {
            load_after_match: rule.load_after_match,
            threshold: rule.threshold,
            name: PreCompiledMatcherName::new(&rule.name),
            package_id: PreCompiledMatcherPackageId::new(&rule.package_id),
            game_version: version,
        }
    }
    fn matches(
        &self,
        mod_: &ModInner,
        name_candidates: Vec<(Id, String)>,
        id_to_packageid_map: &HashMap<Id, PackageId>,
        load_after_map: &HashMap<PackageId, Id>,
    ) -> (Option<(Id, f64)>, Option<(Id, f64)>) {
        debug!(mod_ = ?mod_, candidates = ?name_candidates, "Matching mod '{}' with {} candidates", mod_.name, name_candidates.len());
        if self.load_after_match {
            if let Some(id) = load_after_map.get(&mod_.package_id) {
                return (Some((id.clone(), 1.0)), None);
            }
        }
        let name_match_result = self.name.matches(&mod_.name, name_candidates);
        // 如果 name_match已经超过阈值，直接返回
        if let Some((_id, score)) = &name_match_result {
            debug!("Name match found for '{}' with score {}", mod_.name, score);
            if score >= &self.threshold {
                return (name_match_result, None);
            }
        }
        let package_id_match_result = self
            .package_id
            .matches(&mod_.package_id.to_string(), id_to_packageid_map);
        if let Some((_id, score)) = &package_id_match_result {
            debug!(
                "Package ID match found for '{:?}' with score {}",
                mod_.package_id, score
            );
        }
        (name_match_result, package_id_match_result)
    }
}

struct PreCompiledMatcherName {
    cleanup: bool,
    cleanup_patterns: Vec<(Regex, String)>,
    direct_match: bool,
    similarity: bool,
    similarity_weight: f64,
}

impl PreCompiledMatcherName {
    fn new(rule: &MatchRuleName) -> Self {
        let mut cleanup_patterns = Vec::new();
        for pattern in &rule.cleanup_patterns {
            cleanup_patterns.push((
                Regex::new(&pattern.pattern).unwrap(),
                pattern.replace.clone(),
            ));
        }
        Self {
            cleanup: rule.cleanup,
            direct_match: rule.direct_match,
            similarity: rule.similarity,
            similarity_weight: rule.similarity_weight,
            cleanup_patterns,
        }
    }
    fn similarity_find(&self, name: String, candidates: Vec<(Id, String)>) -> Option<(Id, f64)> {
        let mut best_score = 0.0;
        let mut best_id = None;
        for (id, candidate) in candidates {
            let score = strsim::jaro_winkler(&name, &candidate);
            if score > best_score {
                best_score = score;
                best_id = Some((id, score));
            }
        }
        //best_id * self.similarity_weight
        if let Some((id, score)) = best_id {
            if score >= self.similarity_weight {
                return Some((id, score))
            }
        }
        None
    }
    fn preprocess_name(&self, name: &str) -> String {
        let mut name = name.to_string();
        if self.cleanup {
            for (pattern, replace) in &self.cleanup_patterns {
                name = pattern.replace_all(&name, replace).to_string();
            }
        }
        name
    }
    fn matches(&self, name: &str, candidates: Vec<(Id, String)>) -> Option<(Id, f64)> {
        let name = self.preprocess_name(name);
        if self.direct_match {
            for (id, candidate) in &candidates {
                if &name == candidate {
                    return Some((id.clone(), 1.0));
                }
            }
        }
        if self.similarity {
            return self.similarity_find(name, candidates);
        }
        None
    }
}

struct PreCompiledMatcherPackageId {
    similarity: bool,
    similarity_weight: f64,
    direct_match: bool,
    direct_match_threshold: usize,
    cleanup: bool,
    cleanup_patterns: Vec<(Regex, String)>,
}

impl PreCompiledMatcherPackageId {
    fn new(rule: &MatchRulePackageId) -> Self {
        Self {
            similarity: rule.similarity,
            similarity_weight: rule.similarity_weight,
            direct_match: rule.direct_match,
            direct_match_threshold: rule.direct_match_threshold,
            cleanup: rule.cleanup,
            cleanup_patterns: rule
                .cleanup_patterns
                .iter()
                .map(|pattern| {
                    (
                        Regex::new(&pattern.pattern).unwrap(),
                        pattern.replace.clone(),
                    )
                })
                .collect(),
        }
    }
    fn preprocess_package_id(&self, package_id: &str) -> String {
        let mut package_id = package_id.to_string();
        if self.cleanup {
            for (pattern, replace) in &self.cleanup_patterns {
                package_id = pattern.replace_all(&package_id, replace).to_string();
            }
        }
        package_id
    }
    fn similarity_find(
        &self,
        package_id: String,
        candidates: Vec<(Id, PackageId)>,
    ) -> Option<(Id, f64)> {
        let mut best_score = 0.0;
        let mut best_id = None;
        for (id, candidate) in candidates {
            let score = strsim::jaro_winkler(&package_id, &candidate.to_string());
            if score > best_score {
                best_score = score;
                best_id = Some((id, score));
            }
        }
        if let Some((id, score)) = best_id {
            if score >= self.similarity_weight {
                return Some((id, score));
            }
        }
        None
    }
    fn direct_match(&self, package_id: &str, candidates: Vec<(Id, PackageId)>) -> Option<Id> {
        let package_id_set = package_id.split('.').collect::<Vec<_>>();
        for (id, candidate) in &candidates {
            let candidate_string = candidate.to_string();
            let candidate_set = candidate_string.split('.').collect::<Vec<_>>();
            if (candidate_set
                .iter()
                .zip(package_id_set.iter())
                .take_while(|(a, b)| a == b)
                .count())
                >= self.direct_match_threshold
            {
                return Some(id.clone());
            }
        }
        None
    }
    fn matches(
        &self,
        package_id: &str,
        packageid_map: &HashMap<Id, PackageId>,
    ) -> Option<(Id, f64)> {
        let candidates = packageid_map
            .iter()
            .map(|(id, package_id)| (id.clone(), package_id.clone()))
            .collect::<Vec<_>>();
        let package_id = self.preprocess_package_id(package_id);
        if self.direct_match {
            if let Some(id) = self.direct_match(&package_id, candidates.clone()) {
                return Some((id, 1.0));
            }
        }
        if self.similarity {
            return self.similarity_find(package_id, candidates);
        }
        None
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
    rule: Option<PreCompiledRule>,
    mod_status: HashMap<Id, ModStatus>,
    //pub matches: HashMap<Id, Id, RandomState>, // (mod_id, language_pack_id)
    //pub unconfirmed: HashMap<Id, Vec<(Id, f64)>, RandomState>,
    //checked: HashSet<Id, RandomState>,
    //confirmed: HashSet<Id, RandomState>,
    // confirmed 属于 checked， 二者与 found_langugage_pack 无交集
    have_rules: bool,
    //user_ignore: HashSet<Id, RandomState>,
    game_version: Version,
    sync_tx: Option<tokio::sync::mpsc::Sender<SyncMessage>>,
    auto_translate_cache: Arc<Mutex<lru::LruCache<String, AutoTranslateResult, RandomState>>>,
    ongoing_auto_translate: Arc<Mutex<HashMap<String, Vec<tokio::sync::oneshot::Sender<Result<AutoTranslateResult,String>>>>>>,
}

impl Default for TranslationModData {
    fn default() -> Self {
        Self {
            mod_index: ModIndex::default(),
            rule: None,
            mod_status: HashMap::with_hasher(RandomState::new()),
            have_rules: false,
            game_version: Version::default(),
            sync_tx: None,
            auto_translate_cache: Arc::new(Mutex::new(lru::LruCache::with_hasher(std::num::NonZeroUsize::new(Self::MAX_CACHE_SIZE).unwrap(), RandomState::new()))),
            ongoing_auto_translate: Arc::new(Mutex::new(HashMap::with_hasher(RandomState::new()))),
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
        let rules = match std::fs::read_to_string(format!(
            "{}/translation_mod_match_rules.toml",
            app_data_path
        )) {
            Ok(rules) => Some(toml::from_str::<LanguagePackRules>(&rules).unwrap()),
            Err(_) => {
                warn!("无有效规则文件/读取失败");
                None
            }
        };
        let mut data = self.translation_mod_data.lock_h().await;
        *data = TranslationModData::new(rules, target_language_code, version, self.sync_tx.clone());
    }
    pub async fn rematch_translation(&self, priority: Option<Priority>) -> HashMap<Id, Vec<(Id, f64)>> {
        let mut data = self.translation_mod_data.lock(priority).await;
        data.try_rematch(&self.mods_map).await
    }
}

impl TranslationModData {
    const MAX_CACHE_SIZE: usize = 200;
    fn new(rules: Option<LanguagePackRules>, target_language_code: &str, version: Version, sync_tx: Option<tokio::sync::mpsc::Sender<SyncMessage>>) -> Self {
        let rule = if rules.is_some() {
            let rules = rules.unwrap();
            let rule = rules.get_lan(target_language_code);
            if rule.is_some() {
                info!("预编译规则：{}",target_language_code);
                Some(PreCompiledRule::new(rule.unwrap(), version.clone()))
            } else {
                warn!("无处理 {} 的有效规则", target_language_code);
                None
            }
        } else {
            warn!("无有效规则文件, 使用内置示例规则");
            let rules = LanguagePackRules::default();
            let rule = rules.get_lan(target_language_code);
            if rule.is_some() {
                info!("预编译规则：{}",target_language_code);
                Some(PreCompiledRule::new(rule.unwrap(), version.clone()))
            } else {
                warn!("无处理 {} 的有效规则", target_language_code);
                None
            }
        };
        let have_rules = rule.is_some();
        Self {
            mod_index: ModIndex::new(version.clone()),
            rule,
            mod_status: HashMap::with_hasher(RandomState::new()),
            have_rules: have_rules,
            game_version: version,
            sync_tx,
            auto_translate_cache: Arc::new(Mutex::new(lru::LruCache::with_hasher(std::num::NonZeroUsize::new(Self::MAX_CACHE_SIZE).unwrap(), RandomState::new()))),
            ongoing_auto_translate: Arc::new(Mutex::new(HashMap::with_hasher(RandomState::new()))),
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
        self.mod_index.game_version = version;
    }
    pub fn save_data(&self) -> HashMap<Id, ModStatus> {
        self.mod_status.iter().map(|(k,v)| (k.clone(), v.clone())).collect()
    }
    pub fn save_index(&self) -> ModIndex {
        self.mod_index.clone()
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
        //info!("尝试add mod '{}'", mod_.name);
        match self.identify_if_translation(mod_) {
            IdentifyResult::Translation => {
                info!("添加index Mod '{}'", mod_.name);
                self.send_sync(SyncOperation::TranslationSync(
                    TranslationSyncOperation::AddTranPack(mod_.id.clone())
                ) );
                self.mod_index.build_index(mod_);
                self.mod_status.insert(mod_.id.clone(), ModStatus::Translation);
            }
            IdentifyResult::CheckedNotTranslation => {}
            IdentifyResult::NotTranslation => {
                debug!("Mod '{}' 不是汉化包且未被检查过", mod_.name);
                //info!("尝试为 '{}' 匹配汉化包", mod_.name);
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
        if self.mod_index.contains(&mod_.id) {
            self.mod_index.remove_index(mod_);
        }
        if let Some(status) = self.mod_status.get(&mod_.id){
            match status {
                ModStatus::Matched(_) => {},
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
        // 这个是系统内部调用，在mod add或者update的时候调用，拿到match result后塞到unconfirmed里等待用户确认
        if !self.have_rules {
            return;
        }

        if let Some(status) = self.mod_status.get(&mod_.id) {
            match status {
                ModStatus::Translation => {
                    debug!("Mod '{}' 已被判定为汉化包", mod_.name);
                    return;
                }
                ModStatus::Ignored => {
                    debug!("Mod '{}' 在用户忽略列表中", mod_.name);
                    return;
                }
                ModStatus::Matched(_) => {
                    debug!("Mod '{}' 已经过匹配", mod_.name);
                    return;
                }
                ModStatus::UnconfirmedMatches(_) => {
                    debug!("Mod '{}' 已经过匹配", mod_.name);
                    return;
                }
                ModStatus::NoMatch => {
                    debug!("Mod '{}' 不是汉化包且未被检查过", mod_.name);
                }
            }
        }
        if let Some(rule) = &self.rule {
            let name_candidates = self.mod_index.get_name_candidates(&mod_.name);
            let match_result = rule.matches.matches(
                mod_,
                name_candidates,
                &self.mod_index.id_to_package_id,
                &self.mod_index.load_after_map,
            );

            let ls = match match_result {
                (Some((id, score)), None) => {
                    vec![(id, score)]
                }
                (None, Some((id, score))) => {
                    vec![(id, score)]
                }
                (Some((id1, score1)), Some((id2, score2))) => {
                    vec![(id1, score1), (id2, score2)]
                }
                _ => vec![],
            };
            debug!(result = ?ls);
            if !ls.is_empty() {
                let len = ls.len();
                self.mod_status.insert(mod_.id.clone(), ModStatus::UnconfirmedMatches(ls.clone()));
                self.send_sync(SyncOperation::TranslationSync(
                    TranslationSyncOperation::AddUnconfirmed((mod_.id.clone(), ls))
                ) );
                debug!("匹配成功，等待用户确认: ls: {:?} total={}", len, self.mod_status.iter().filter(|(_, v)| matches!(v, ModStatus::UnconfirmedMatches(_))).count());
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
        self.mod_index.build_index(mod_);
    }
    pub fn remove_translation_pack(&mut self, mod_: &ModInner) {
        info!("移除汉化包: {}", mod_.name);
        self.mod_index.remove_index(mod_);

        self.mod_status.insert(mod_.id.clone(), ModStatus::Ignored);

        self.send_sync(SyncOperation::TranslationSync(
            TranslationSyncOperation::RemoveTranPack(mod_.id.clone())
        ) );
    }

    pub fn get_auto_translate_cache(&self) -> 
    (Arc<Mutex<lru::LruCache<String, AutoTranslateResult, RandomState>>>,
    Arc<Mutex<HashMap<String, Vec<tokio::sync::oneshot::Sender<Result<AutoTranslateResult,String>>>>>>) {
        (self.auto_translate_cache.clone(), self.ongoing_auto_translate.clone())
    }

    // 计算给定的source和target的得分及其详情, 给前端展示用
    pub fn custom_calc(&self, source: &ModInner, target: &ModInner) -> CustomCalcResult {
        if !self.have_rules || self.rule.is_none() {
            return CustomCalcResult {
                overall_score: 0.0,
                theshold: 0.0,
                name: CustomCalcNameResult {
                    score: 0.0,
                    details: HashMap::new(),
                },
                package_id: CustomCalcPackageIdResult {
                    score: 0.0,
                    details: HashMap::new(),
                },
                additional: HashMap::new(),
            };
        }

        let rule = self.rule.as_ref().unwrap();
        
        // 名称匹配详情
        let mut name_details = HashMap::new();
        let source_name = source.name.clone();
        let target_name = target.name.clone();
        
        name_details.insert("source_name".to_string(), source_name.clone());
        name_details.insert("target_name".to_string(), target_name.clone());
        
        // 预处理名称并计算相似度
        let processed_source_name = rule.matches.name.preprocess_name(&source_name);
        let processed_target_name = rule.matches.name.preprocess_name(&target_name);
        name_details.insert("processed_source_name".to_string(), processed_source_name.clone());
        name_details.insert("processed_target_name".to_string(), processed_target_name.clone());
        
        let name_similarity = strsim::jaro_winkler(&processed_source_name, &processed_target_name);
        let name_score = if name_similarity >= rule.matches.name.similarity_weight {
            name_similarity
        } else {
            0.0
        };
        name_details.insert("similarity".to_string(), name_similarity.to_string());
        name_details.insert("similarity_threshold".to_string(), rule.matches.name.similarity_weight.to_string());
        
        // 包ID匹配详情
        let mut package_id_details = HashMap::new();
        let source_package_id = source.package_id.to_string();
        let target_package_id = target.package_id.to_string();
        
        package_id_details.insert("source_package_id".to_string(), source_package_id.clone());
        package_id_details.insert("target_package_id".to_string(), target_package_id.clone());
        
        // 预处理包ID并计算相似度
        let processed_source_package_id = rule.matches.package_id.preprocess_package_id(&source_package_id);
        let processed_target_package_id = rule.matches.package_id.preprocess_package_id(&target_package_id);
        package_id_details.insert("processed_source_package_id".to_string(), processed_source_package_id.clone());
        package_id_details.insert("processed_target_package_id".to_string(), processed_target_package_id.clone());
        
        let package_id_similarity = strsim::jaro_winkler(&processed_source_package_id, &processed_target_package_id);
        let package_id_score = if package_id_similarity >= rule.matches.package_id.similarity_weight {
            package_id_similarity
        } else {
            0.0
        };
        package_id_details.insert("similarity".to_string(), package_id_similarity.to_string());
        package_id_details.insert("similarity_threshold".to_string(), rule.matches.package_id.similarity_weight.to_string());
        
        // 直接匹配 package_id 分段
        let source_package_id_parts = processed_source_package_id.split('.').collect::<Vec<_>>();
        let target_package_id_parts = processed_target_package_id.split('.').collect::<Vec<_>>();
        let common_parts_count = source_package_id_parts
            .iter()
            .zip(target_package_id_parts.iter())
            .take_while(|(a, b)| a == b)
            .count();
        package_id_details.insert("common_parts_count".to_string(), common_parts_count.to_string());
        package_id_details.insert("direct_match_threshold".to_string(), 
            rule.matches.package_id.direct_match_threshold.to_string());
        
        // 直接匹配判定
        let direct_match = common_parts_count >= rule.matches.package_id.direct_match_threshold;
        package_id_details.insert("direct_match".to_string(), direct_match.to_string());
        
        // 添加额外信息
        let mut additional = HashMap::new();
        
        // 检查 load_after
        let load_after_match = source.load_order
            .get(&self.game_version)
            .map(|orders| {
                orders.iter().any(|order| {
                    if let ModOrder::After(pid) = order {
                        pid == &target.package_id
                    } else {
                        false
                    }
                })
            })
            .unwrap_or(false);
        additional.insert("load_after_match".to_string(), load_after_match.to_string());
        
        let internal_status = self.mod_status.get(&source.id);
        let internal_status_str = match internal_status {
            Some(inner) => {
                serde_json::to_string(inner).unwrap()
            },
            None => {
                "None".to_string()
            }
        };
        additional.insert("internal_status".to_string(), internal_status_str);
        
        // 计算综合得分（基于规则中的阈值）
        let final_name_score = if rule.matches.name.direct_match && processed_source_name == processed_target_name {
            name_score + 1.0
        } else if rule.matches.name.similarity {
            name_score
        } else {
            0.0
        };
        
        let final_package_id_score = if rule.matches.package_id.direct_match && direct_match {
            package_id_score + 1.0
        } else if rule.matches.package_id.similarity {
            package_id_score
        } else {
            0.0
        };
        
        additional.insert("final_name_score".to_string(), final_name_score.to_string());
        additional.insert("final_package_id_score".to_string(), final_package_id_score.to_string());
        
        // 综合得分：最高的分数
        let overall_score = final_name_score.max(final_package_id_score);
        additional.insert("match_result".to_string(), (overall_score >= rule.matches.threshold).to_string());
        
        CustomCalcResult {
            overall_score,
            theshold: rule.matches.threshold,
            name: CustomCalcNameResult {
                score: final_name_score,
                details: name_details,
            },
            package_id: CustomCalcPackageIdResult {
                score: final_package_id_score,
                details: package_id_details,
            },
            additional,
        }
    }
}

#[derive(Debug, serde::Serialize)]
pub struct CustomCalcResult {
    overall_score: f64,
    theshold: f64,
    name: CustomCalcNameResult,
    package_id: CustomCalcPackageIdResult,
    additional: HashMap<String, String>
}

#[derive(Debug, serde::Serialize)]
pub struct CustomCalcNameResult {
    score: f64,
    details: HashMap<String, String>,
}

#[derive(Debug, serde::Serialize)]
pub struct CustomCalcPackageIdResult {
    score: f64,
    details: HashMap<String, String>,
}

pub enum IdentifyResult {
    NoRules,
    Translation,
    CheckedNotTranslation,
    NotTranslation,
}

#[derive(Default, Clone, bincode::Decode, bincode::Encode)]
pub struct ModIndex {
    pub name_to_id: HashMap<String, Id>, // 就一点汉化包的name应该不会有重复吧
    pub names_by_length: BTreeMap<usize, Vec<String>>,
    pub id_to_package_id: HashMap<Id, PackageId>,
    pub load_after_map: HashMap<PackageId, Id>,
    change: bool,
    game_version: Version,
}

impl ModIndex {
    pub fn new(version: Version) -> Self {
        Self {
            name_to_id: HashMap::new(),
            names_by_length: BTreeMap::new(),
            id_to_package_id: HashMap::new(),
            load_after_map: HashMap::new(),
            change: false,
            game_version: version,
        }
    }
    fn build_index(&mut self, mod_: &ModInner) {
        self.name_to_id.insert(mod_.name.clone(), mod_.id.clone());
        self.names_by_length
            .entry(mod_.name.len())
            .or_insert_with(Vec::new)
            .push(mod_.name.clone());
        self.id_to_package_id
            .insert(mod_.id.clone(), mod_.package_id.clone());
        for id in mod_.load_order.get(&self.game_version).unwrap_or(&HashSet::new()) {
            if let ModOrder::After(packageid) = id {
                self.load_after_map
                    .insert(packageid.clone(), mod_.id.clone());
            }
        }
        self.change = true;
        debug!("Mod '{}' 添加到索引, 现长度 {}", mod_.name, self.id_to_package_id.len());
    }
    fn remove_index(&mut self, mod_: &ModInner) {
        self.name_to_id.remove(&mod_.name);
        match self.names_by_length
            .get_mut(&mod_.name.len())
            {
                Some(names) => {
                    names.retain(|name| name != &mod_.name);
                    if names.is_empty() {
                        self.names_by_length.remove(&mod_.name.len());
                    }
                }
                None => {}
            }
        self.id_to_package_id.remove(&mod_.id);
        self.load_after_map.retain(|_packageid, id| id != &mod_.id);
        self.change = true;
    }

    fn get_name_candidates(&self, name: &str) -> Vec<(Id, String)> {
        let mut candidates = Vec::new();
        let name_len = name.len();
        // 范围+-20%
        for (_len, names) in self
            .names_by_length
            .range((name_len - name_len / 5)..=(name_len + name_len / 5))
        {
            for candidate in names {
                if candidate.starts_with(name) {
                    if let Some(id) = self.name_to_id.get(candidate) {
                        candidates.push((id.clone(), candidate.clone()));
                    }
                }
            }
        }
        candidates
    }

    fn reset_change(&mut self) {
        self.change = false;
    }

    fn contains(&self, id: &Id) -> bool {
        self.id_to_package_id.contains_key(id)
    }
}


#[derive(Debug, serde::Serialize, Clone)]
pub struct AutoTranslateResult {
    pub code: i32,
    pub message: Option<String>,
    pub data: String,
    pub source: String,
    pub target: String,
}

pub async fn auto_translate(text: String, from: String, to: String, proxy: Option<String>,
    cache: Arc<Mutex<lru::LruCache<String, AutoTranslateResult, RandomState>>>,
    ongoing_auto_translate: Arc<Mutex<HashMap<String, Vec<tokio::sync::oneshot::Sender<Result<AutoTranslateResult,String>>>>>>
) -> Result<AutoTranslateResult, String> {

    if cache.lock().unwrap().contains(&text) {
        debug!("翻译缓存命中");
        return Ok(cache.lock().unwrap().get(&text).unwrap().clone());
    }

    let rx = {
        let mut ongoing = ongoing_auto_translate.lock().unwrap();
        if ongoing.contains_key(&text) {
            debug!("翻译正在进行中");
            let (tx, rx) = tokio::sync::oneshot::channel();
            ongoing.get_mut(&text).unwrap().push(tx);
            Some(rx)
        } else {
            ongoing.insert(text.clone(), vec![]);
            None
        }
    };

    if let Some(rx) = rx {
        return match rx.await {
            Ok(res) => res,
            Err(e) => Err(e.to_string()),
        };
    }

    ongoing_auto_translate.lock().unwrap().insert(text.clone(), vec![]);

    let proxy = if let Some(proxy) = proxy {
        if proxy == "" {
            None
        } else {
            Some(proxy)
        }
    } else {
        None
    };
    let deeplx = deeplx::DeepLX::new(deeplx::Config {
        proxy,
        ..deeplx::Config::default()
    });
    
    let res = deeplx.translate(&from, &to, &text, None, None).await
            .map_err(|e| e.to_string())
            .map(|res| {
                let result = AutoTranslateResult {
                    code: res.code,
                    message: res.message,
                    data: res.data,
                    source: res.source_lang,
                    target: res.target_lang,
                };
                if res.code == 200 {
                    cache.lock().unwrap().put(text.clone(), result.clone());
                }
                debug!(?result, "翻译结果");
                result
            });
    
    let txs = ongoing_auto_translate.lock().unwrap().remove(&text).unwrap();
    for tx in txs {
        let _ = tx.send(res.clone());
    }

    res
}