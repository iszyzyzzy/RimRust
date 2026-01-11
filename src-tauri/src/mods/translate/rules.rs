use serde::{Deserialize, Serialize};

// 示例规则文件 trancslation_mod_match_rules.toml
static EXAMPLE: &'static str = r#"
version = '0.1'
# 我为大部分可能不用写的字段都加上了#[serde(default)]
# 但是我不确定某些类型的默认值是什么，所以还是建议都写上

[[rules]]
language_code = 'zh'

[rules.identify]
threshold = 1.0

  [[rules.identify.patterns]]
  pattern_type = 'name' # name, package_id, author
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

  [[rules.identify.patterns]]
  pattern_type = 'author'
  pattern = 'leafzxg'
  weight = 1.0
  # is_regex省略，默认为false

  [rules.identify.support_languages]
  # 无论匹配到了几个都只加一次weight
  support_languages = ['zh']
  weight = 0.4

[rules.matches]
threshold = 1.0

  [rules.matches.candidates]
  # 不使用filter时，candidates为所有mod
  # 其实不用性能也足够高了
  use_filter = false

    [rules.matches.candidates.filter]
    # 这两个条件是或的关系，主要是因为与的话太强了筛完啥也不剩了
    load_after = true 
    name_length_diff = 0.3 # 百分比

  [rules.matches.name]
  cleanup = true
  direct_match = true
  direct_match_threshold = 3 # 可以容忍的编辑距离
  direct_match_weight = 1.0
  similarity = true
  similarity_weight = 1.0 # 相似度的结果再乘以这个值

    [[rules.matches.name.cleanup_patterns]]
    pattern = '(?i)(chinese|中文|汉化|_zh|zh-pack|zh|cn)'
    replace = ''

    [[rules.matches.name.cleanup_patterns]]
    pattern = '_'
    replace = ' '

  [rules.matches.package_id]
  cleanup = true # cleanup的时候是整个按字符串处理的
  direct_match = true
  direct_match_threshold = 2 # 以点分隔的最小匹配数量
  direct_match_weight = 1.0
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
    pub fn from_path(path: &str) -> Option<Self> {
        let content = match std::fs::read_to_string(path) {
            Ok(content) => content,
            Err(_) => return None,
        };
        match toml::from_str(&content) {
            Ok(rules) => Some(rules),
            Err(_) => None,
        }
    }
    pub fn new(path: &str) -> Self {
        match Self::from_path(&path) {
            Some(rules) => rules,
            None => Self::default(),
        }
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
    #[serde(default)]
    pub patterns: Vec<IdentifyPattern>,
    pub support_languages: IdentifyLanguageSupportRule,
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
    #[serde(rename = "author")]
    Author,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct IdentifyLanguageSupportRule {
    #[serde(default)]
    pub support_languages: Vec<String>,
    #[serde(default)]
    pub weight: f64,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct MatchRule {
    pub load_after_match: bool,
    pub threshold: f64,
    pub name: MatchRuleInner,
    pub package_id: MatchRuleInner,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct MatchRuleCandidates {
    pub use_filter: bool,
    #[serde(default)]
    pub filter: MatchRuleCandidatesFilter,
}

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct MatchRuleCandidatesFilter {
    pub load_after: bool,
    pub name_length_diff: f64,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct CleanupPattern {
    pub pattern: String,
    pub replace: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct MatchRuleInner {
    pub cleanup: bool,
    #[serde(default)]
    pub cleanup_patterns: Vec<CleanupPattern>,
    pub similarity: bool,
    #[serde(default)]
    pub similarity_weight: f64,
    pub direct_match: bool,
    #[serde(default)]
    pub direct_match_threshold: usize,
    #[serde(default)]
    pub direct_match_weight: f64,
}