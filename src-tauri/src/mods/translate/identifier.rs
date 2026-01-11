use regex::RegexSet;
use ahash::HashSet;
use tracing::debug;

use crate::{types::*, mods::ModInner};
use super::rules::{IdentifyRule,  IdentifyPattern, IdentifyPatternType};

pub struct Identifyer {
    threshold: f64,
    name_patterns: Patterns,
    package_id_patterns: Patterns,
    author_patterns: Patterns,
    support_languages: (HashSet<String>, f64),
    game_version: Version,
}

struct Patterns {
    regex: RegexSet,
    regex_weights: Vec<f64>,
    plain: Vec<String>,
    plain_weights: Vec<f64>,
}

impl Patterns {
    fn new(rules: Vec<IdentifyPattern>) -> Self {
        let mut regex = Vec::new();
        let mut regex_weights = Vec::new();
        let mut plain = Vec::new();
        let mut plain_weights = Vec::new();

        for rule in rules {
            if rule.is_regex {
                regex.push(rule.pattern);
                regex_weights.push(rule.weight);
            } else {
                plain.push(rule.pattern);
                plain_weights.push(rule.weight);
            }
        }

        Self {
            regex: RegexSet::new(regex).unwrap(),
            regex_weights,
            plain,
            plain_weights,
        }
    }
    fn scoring(&self, text: &str) -> f64 {
        let mut score = 0.0;

        let matches = self.regex.matches(text);
        for idx in matches.into_iter() {
            score += self.regex_weights[idx];
        }

        for (pattern, weight) in self.plain.iter().zip(&self.plain_weights) {
            if text.contains(pattern) {
                score += weight;
            }
        }

        score
    }
}

impl Identifyer {
    pub fn new(rule: &IdentifyRule, game_version: Version) -> Self {
        let mut name_patterns = Vec::new();
        let mut package_patterns = Vec::new();
        let mut author_patterns = Vec::new();

        for pattern in &rule.patterns {
            match pattern.pattern_type {
                IdentifyPatternType::Name => {
                    name_patterns.push(pattern.clone());
                }
                IdentifyPatternType::PackageId => {
                    package_patterns.push(pattern.clone());
                }
                IdentifyPatternType::Author => {
                    author_patterns.push(pattern.clone());
                }
            }
        }

        Self {
            threshold: rule.threshold,
            name_patterns: Patterns::new(name_patterns),
            package_id_patterns: Patterns::new(package_patterns),
            author_patterns: Patterns::new(author_patterns),
            support_languages: (
                rule.support_languages.support_languages.iter().cloned().collect(),
                rule.support_languages.weight,
            ),
            game_version
        }
    }

    pub fn scoring(&self, mod_: &ModInner) -> f64 {
        let mut score = 0.0;

        score += self.name_patterns.scoring(&mod_.name);

        score += self.package_id_patterns.scoring(&mod_.package_id.to_string());

        score += self.author_patterns.scoring(&mod_.author);

        if let Some(support_languages) = mod_.support_languages.get(&self.game_version) {
            if support_languages.iter().any(|v| support_languages.contains(&v.to_string())) {
                score += self.support_languages.1;
            }
        }

        score
    }

    pub fn identify(&self, mod_: &ModInner) -> bool {
        let score = self.scoring(mod_);
        debug!(id = ?mod_.id, "Mod '{}' 语言包判断得分 {}", mod_.name, score);
        score >= self.threshold
    }
}