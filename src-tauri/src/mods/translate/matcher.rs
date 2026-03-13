use ahash::{HashSet, HashSetExt};
use regex::Regex;
use serde::Serialize;

use crate::mods::ModInner;
use crate::types::*;

use super::rules::{CleanupPattern, MatchRule, MatchRuleCandidates, MatchRuleCandidatesFilter, MatchRuleInner};

struct CleanupPatterns {
    patterns: Vec<(Regex, String)>,
}

impl CleanupPatterns {
    fn new(rules: Vec<CleanupPattern>) -> Self {
        let mut patterns = Vec::new();

        for rule in rules {
            patterns.push((Regex::new(&rule.pattern).unwrap(), rule.replace));
        }

        Self { patterns }
    }

    fn empty() -> Self {
        Self {
            patterns: Vec::new(),
        }
    }

    fn cleanup(&self, text: &str) -> String {
        let mut cleaned = text.to_string();

        for (pattern, replace) in &self.patterns {
            cleaned = pattern.replace_all(&cleaned, replace).to_string();
        }

        cleaned
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct PartMatcheResult {
    pub score: f64,
    pub detail: Vec<(String, String)>,
}

#[derive(Debug, Clone, Serialize)]
pub struct MatcherResult {
    pub score: f64,
    pub detail: Vec<(String, PartMatcheResult)>,
}

pub struct Matcher {
    threshold: f64,
    load_after_match: bool,
    name_matcher: NameMatcher,
    package_id_matcher: PackageIdMatcher,
    candidates_maker: CandidatesFilter,
}

impl Matcher {
    pub fn new(rules: &MatchRule) -> Self {
        // 创建默认的候选过滤器（不过滤）
        let default_candidates = MatchRuleCandidates {
            use_filter: false,
            filter: MatchRuleCandidatesFilter {
                load_after: false,
                name_length_diff: 0.0,
            },
        };

        Self {
            threshold: rules.threshold,
            load_after_match: rules.load_after_match,
            name_matcher: NameMatcher::new(&rules.name),
            package_id_matcher: PackageIdMatcher::new(&rules.package_id),
            candidates_maker: CandidatesFilter::new(&default_candidates),
        }
    }

    pub fn match_mod(&self, index: &ModIndex, target: &ModInner) -> Vec<(Id, f64)> {
        // 先检查 load_after
        if self.load_after_match {
            if let Some(ids) = index.load_after_map.get(&target.package_id) {
                if !ids.is_empty() {
                    return ids.iter().map(|id| (id.clone(), 1.0)).collect();
                }
            }
        }

        let candidates = self.candidates_maker.filter(index, target);
        let mut results = Vec::new();

        for candidate in &candidates.0 {
            let name_score = self.name_matcher.scoring(&target.name, &candidate.name);
            let package_id_score = self.package_id_matcher.scoring(
                &target.package_id.to_string(),
                &candidate.package_id.to_string(),
            );

            let score = name_score.max(package_id_score);
            if score >= self.threshold {
                results.push((candidate.id.clone(), score));
            }
        }

        // 按分数降序排序
        results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        results
    }

    pub fn custom_calc(&self, source: &ModInner, target: &ModInner) -> MatcherResult {
        let name_result = self.name_matcher.scoring_detail(&source.name, &target.name);
        let package_id_result = self.package_id_matcher.scoring_detail(
            &source.package_id.to_string(),
            &target.package_id.to_string(),
        );

        let score = name_result.score.max(package_id_result.score);
        let mut detail = Vec::new();
        detail.push(("name".to_string(), name_result));
        detail.push(("package_id".to_string(), package_id_result));

        MatcherResult { score, detail }
    }
}

struct NameMatcher {
    cleanup_patterns: CleanupPatterns,
    use_direct_match: bool,
    direct_match_threshold: usize,
    direct_match_weight: f64,
    use_similarity: bool,
    similarity_weight: f64,
}

impl NameMatcher {
    fn new(rules: &MatchRuleInner) -> Self {
        let cleanup_patterns = if rules.cleanup {
            CleanupPatterns::new(rules.cleanup_patterns.clone())
        } else {
            CleanupPatterns::empty()
        };

        Self {
            cleanup_patterns,
            use_direct_match: rules.direct_match,
            direct_match_threshold: rules.direct_match_threshold,
            direct_match_weight: rules.direct_match_weight,
            use_similarity: rules.similarity,
            similarity_weight: rules.similarity_weight,
        }
    }

    fn direct_match(&self, source: &str, target: &str) -> bool {
        if self.use_direct_match {
            let diff = strsim::levenshtein(source, target);
            if diff <= self.direct_match_threshold {
                return true;
            }
        }

        false
    }

    fn similarity(&self, source: &str, target: &str) -> f64 {
        if self.use_similarity {
            let similarity = strsim::jaro_winkler(source, target);
            return similarity * self.similarity_weight;
        }

        0.0
    }

    fn scoring(&self, source: &str, target: &str) -> f64 {
        let mut score = 0.0;

        let source = self.cleanup_patterns.cleanup(source);
        let target = self.cleanup_patterns.cleanup(target);

        if self.direct_match(&source, &target) {
            score += self.direct_match_weight;
        }

        score += self.similarity(&source, &target);

        score
    }

    fn scoring_detail(&self, source: &str, target: &str) -> PartMatcheResult {
        let mut detail = Vec::new();
        let mut score = 0.0;

        let source = self.cleanup_patterns.cleanup(source);
        let target = self.cleanup_patterns.cleanup(target);

        if self.use_direct_match {
            if self.direct_match(&source, &target) {
                score += self.direct_match_weight;
                detail.push((
                    "direct_match".to_string(),
                    self.direct_match_weight.to_string(),
                ));
            } else {
                detail.push(("direct_match".to_string(), "0.0".to_string()));
            }
        }

        if self.use_similarity {
            let similarity = self.similarity(&source, &target);
            detail.push(("similarity".to_string(), similarity.to_string()));
        }

        PartMatcheResult { score, detail }
    }
}

struct PackageIdMatcher {
    cleanup_patterns: CleanupPatterns,
    use_direct_match: bool,
    direct_match_threshold: usize,
    direct_match_weight: f64,
    use_similarity: bool,
    similarity_weight: f64,
}

impl PackageIdMatcher {
    fn new(rules: &MatchRuleInner) -> Self {
        let cleanup_patterns = if rules.cleanup {
            CleanupPatterns::new(rules.cleanup_patterns.clone())
        } else {
            CleanupPatterns::empty()
        };

        Self {
            cleanup_patterns,
            use_direct_match: rules.direct_match,
            direct_match_threshold: rules.direct_match_threshold,
            direct_match_weight: rules.direct_match_weight,
            use_similarity: rules.similarity,
            similarity_weight: rules.similarity_weight,
        }
    }

    fn direct_match(&self, source: &str, target: &str) -> bool {
        if self.use_direct_match {
            let source_set: Vec<_> = source.split('.').collect();
            let target_set: Vec<_> = target.split('.').collect();

            if target_set
                .iter()
                .zip(source_set.iter())
                .filter(|(a, b)| a == b)
                .count()
                >= self.direct_match_threshold
            {
                return true;
            }
        }

        false
    }

    fn similarity(&self, source: &str, target: &str) -> f64 {
        if self.use_similarity {
            let similarity = strsim::jaro_winkler(source, target);
            return similarity * self.similarity_weight;
        }

        0.0
    }

    fn scoring(&self, source: &str, target: &str) -> f64 {
        let mut score = 0.0;

        let source = self.cleanup_patterns.cleanup(source);
        let target = self.cleanup_patterns.cleanup(target);

        if self.direct_match(&source, &target) {
            score += self.direct_match_weight;
        }

        score += self.similarity(&source, &target);

        score
    }

    fn scoring_detail(&self, source: &str, target: &str) -> PartMatcheResult {
        let mut detail = Vec::new();
        let mut score = 0.0;

        let source = self.cleanup_patterns.cleanup(source);
        let target = self.cleanup_patterns.cleanup(target);

        if self.use_direct_match {
            if self.direct_match(&source, &target) {
                score += self.direct_match_weight;
                detail.push((
                    "direct_match".to_string(),
                    self.direct_match_weight.to_string(),
                ));
            } else {
                detail.push(("direct_match".to_string(), "0.0".to_string()));
            }
        }

        if self.use_similarity {
            let similarity = self.similarity(&source, &target);
            detail.push(("similarity".to_string(), similarity.to_string()));
        }

        PartMatcheResult { score, detail }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Candidate {
    pub id: Id,
    pub name: String,
    pub package_id: PackageId,
    pub author: String,
}

pub struct Candidates(Vec<Candidate>);

pub struct CandidatesFilter {
    enabled: bool,
    load_after: bool,
    name_length_threshold: f64,
}

use super::index::ModIndex;

impl CandidatesFilter {
    fn new(rules: &MatchRuleCandidates) -> Self {
        Self {
            enabled: rules.use_filter,
            load_after: rules.filter.load_after,
            name_length_threshold: rules.filter.name_length_diff,
        }
    }

    fn filter(&self, index: &ModIndex, target: &ModInner) -> Candidates {
        if !self.enabled {
            let mut result = Vec::with_capacity(index.id_to_name.len());
            for (id, name) in &index.id_to_name {
                result.push(Candidate {
                    id: id.clone(),
                    name: name.clone(),
                    package_id: index.id_to_package_id.get(&id).unwrap().clone(),
                    author: index.id_to_author.get(&id).unwrap().to_string(),
                })
            }
            return Candidates(result);
        };
        let mut result = HashSet::new();

        let target_name_len = target.name.len() as f64;

        for (_len, ids) in index.names_by_length.range(
            (target_name_len * (1.0 - self.name_length_threshold)) as usize
                ..=(target_name_len * (1.0 + self.name_length_threshold)) as usize,
        ) {
            for candidate in ids {
                result.insert(Candidate {
                    id: candidate.clone(),
                    name: index.id_to_name.get(candidate).unwrap().to_string(),
                    package_id: index.id_to_package_id.get(candidate).unwrap().clone(),
                    author: index.id_to_author.get(candidate).unwrap().to_string(),
                });
            }
        }

        if let Some(ids) = index.load_after_map.get(&target.package_id) {
            for candidate in ids {
                result.insert(Candidate {
                    id: candidate.clone(),
                    name: index.id_to_name.get(candidate).unwrap().to_string(),
                    package_id: index.id_to_package_id.get(candidate).unwrap().clone(),
                    author: index.id_to_author.get(candidate).unwrap().to_string(),
                });
            }
        };

        Candidates(result.into_iter().collect())
    }
}

