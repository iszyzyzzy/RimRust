use ahash::{HashMap, HashMapExt};
use ahash::RandomState;
use tracing::{debug, info, instrument};
use tracing_subscriber::field::debug;

use crate::{mods::ModInner, types::*};

use super::{SearchField, SearchResult, SearchResultItem};

struct InnerData {
    name: String,
    display_name: String,
    description: String,
    author: String,
    package_id: String,
    enabled: bool,
}

pub struct SearchDataInner {
    data: HashMap<Id, InnerData>,
    game_versions: Version,
    cache: lru::LruCache<(String, Vec<SearchField>), SearchResult, RandomState>,
}

impl SearchField {
    fn weight(&self) -> f32 {
        match self {
            SearchField::Id => 0.0,
            SearchField::Name => 5.0,
            SearchField::DisplayName => 4.0,
            SearchField::Description => 3.0,
            SearchField::Author => 2.0,
            SearchField::PackageId => 1.0,
        }
    }
    fn score(&self, count: f32) -> f32 { // 返回值是衰减过后的最终分数
        if count <= 1.0 {
            count * self.weight()
        } else {
            self.weight() * (1.0 + count).ln()
        }
    }
}

impl SearchDataInner {
const MAX_CACHE_SIZE: usize = 50;
    pub fn new(game_versions: Version) -> Self {
        Self {
            data: HashMap::new(),
            game_versions,
            cache: lru::LruCache::with_hasher(std::num::NonZeroUsize::new(Self::MAX_CACHE_SIZE).unwrap(), RandomState::new()),
        }
    }
    pub fn add(&mut self, mod_: &ModInner) {
        let inner_data = InnerData {
            name: mod_.name.clone().to_lowercase(),
            display_name: mod_.display_name.clone().to_lowercase(),
            description: mod_.description.get(&self.game_versions).cloned().unwrap_or("".to_string()).to_lowercase(),
            author: mod_.author.clone().to_lowercase(),
            package_id: mod_.package_id.to_string(),
            enabled: mod_.enabled,
        };
        self.data.insert(mod_.id.clone(), inner_data);
        self.cache.clear();
    }
    #[instrument(skip(self))]
    pub async fn search(&mut self, query_str: &str, field: Vec<SearchField>, enabled_only: bool) -> SearchResult {
        let query_str = query_str.to_lowercase();

        if let Some(result) = self.cache.get(&(query_str.clone(), field.clone())) {
            debug!("search cache hit");
            return result.clone();
        }

        let mut result = SearchResult {
            total: 0,
            mods: Vec::new(),
            highlight: vec![query_str.clone()],
        };

        for (id, data) in &self.data {
            let mut matched_fields = Vec::new();
            let mut score = 0.0;
            for f in &field {
                let field_str = match f {
                    SearchField::Id => &id.to_string(),
                    SearchField::Name => &data.name,
                    SearchField::DisplayName => &data.display_name,
                    SearchField::Description => &data.description,
                    SearchField::Author => &data.author,
                    SearchField::PackageId => &data.package_id,
                };
                let count = field_str.matches(&query_str).count();
                if count > 0 {
                    matched_fields.push(f.to_string());
                    score += f.score(count as f32);
                }
            }
            if !matched_fields.is_empty() {
                result.mods.push(SearchResultItem {
                    id: id.clone(),
                    matched_fields,
                    score
                });
            }
        }

        result.mods.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
        result.total = result.mods.len();

        self.cache.put((query_str, field), result.clone());
        
        debug!("search result: {:?}", result.mods);

        if enabled_only {
            result.mods.retain(|item| {
                if let Some(data) = self.data.get(&item.id) {
                    data.enabled
                } else {
                    false
                }
            });
            result.total = result.mods.len();
        }
        result
    }

    pub fn remove(&mut self, id: &Id) {
        self.data.remove(id);
        self.cache.clear();
    }

    pub fn update(&mut self, mod_: &ModInner) {
        self.add(mod_);
        self.cache.clear();
    }
}