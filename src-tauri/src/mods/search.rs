use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use tauri::AppHandle;
use ahash::RandomState;

use crate::{
    types::*,
    mods::{Mod, ModInner, BaseList}
};

mod advance;
mod basic;

use advance::SearchDataInner as SearchDataInnerAdvance;
use basic::SearchDataInner as SearchDataInnerBasic;

impl BaseList {
    pub async fn init_search_engine(
        &self,
        use_advance: bool,
        app_data_path: &str,
        game_version: Version,
        app: tauri::AppHandle,
    ) {
        let mut guard = self.search_data.lock_h().await;
        guard.init(use_advance, app_data_path, game_version);
        guard.start_auto_commit(app).await;
    }
}

pub struct SearchData {
    pub search_data_advance: Option<SearchDataInnerAdvance>,
    pub search_data_basic: Option<SearchDataInnerBasic>,
    pub use_advance: bool,
}

impl Default for SearchData {
    fn default() -> Self {
        Self {
            search_data_advance: None,
            search_data_basic: None,
            use_advance: false,
        }
    }
}

impl SearchData {
    pub fn init(
        &mut self,
        use_advance: bool,
        app_data_path: &str,
        game_version: Version,
    ) {
        if use_advance {
            self.search_data_advance = Some(SearchDataInnerAdvance::new(app_data_path, game_version));
        } else {
            self.search_data_basic = Some(SearchDataInnerBasic::new(game_version));
        }
    }
    pub async fn add(&mut self, mod_: &ModInner) {
        if self.use_advance {
            if let Some(engine) = self.search_data_advance.as_ref() {
                engine.add(mod_).await;
            }
        } else {
            if let Some(engine) = self.search_data_basic.as_mut() {
                engine.add(mod_);
            }
        }
    }
    pub async fn search(&mut self, query: &str, field: Vec<SearchField>, enabled_only: bool) -> SearchResult {
        let field = if field.is_empty() {
            SearchField::all()
        } else {
            field
        };
        if self.use_advance {
            if let Some(engine) = self.search_data_advance.as_ref() {
                return engine.search(query, field, enabled_only).await
            }
        } else {
            if let Some(engine) = self.search_data_basic.as_mut() {
                return engine.search(query, field, enabled_only).await
            }
        }
        SearchResult {
            total: 0,
            mods: Vec::new(),
            highlight: Vec::new(),
        }
    }
    pub async fn remove(&mut self, id: &Id) {
        if self.use_advance {
            if let Some(engine) = self.search_data_advance.as_ref() {
                engine.remove(id).await;
            }
        } else {
            if let Some(engine) = self.search_data_basic.as_mut() {
                engine.remove(id);
            }
        }
    }
    pub async fn update(&mut self, mod_: &ModInner) {
        if self.use_advance {
            if let Some(engine) = self.search_data_advance.as_ref() {
                engine.update(mod_).await;
            }
        } else {
            if let Some(engine) = self.search_data_basic.as_mut() {
                engine.update(mod_);
            }
        }
    }
    pub async fn start_auto_commit(&mut self, app: AppHandle) {
        if self.use_advance {
            if let Some(engine) = self.search_data_advance.as_mut() {
                engine.start_auto_commit(app).await;
            }
        }
    }
    pub async fn _stop_auto_commit(&self) {
        if self.use_advance {
            if let Some(engine) = self.search_data_advance.as_ref() {
                engine.stop_auto_commit().await;
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum SearchField {
    Id,
    Name,
    DisplayName,
    Description,
    Author,
    PackageId,
}

impl SearchField {
    pub fn to_str(&self) -> &str {
        match self {
            Self::Id => "id",
            Self::Name => "name",
            Self::DisplayName => "display_name",
            Self::Description => "description",
            Self::Author => "author",
            Self::PackageId => "packageId",
        }
    }
    pub fn to_string(&self) -> String {
        self.to_str().to_string()
    }
    pub fn to_str_vec(fields: &Vec<Self>) -> Vec<String> {
        fields.iter().map(|f| f.to_str().to_string()).collect()
    }
    pub fn all() -> Vec<Self> { // 没有id是刻意的
        vec![Self::Name, Self::DisplayName, Self::Description, Self::Author, Self::PackageId]
    }
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "name" => Some(Self::Name),
            "description" => Some(Self::Description),
            "author" => Some(Self::Author),
            "packageId" => Some(Self::PackageId),
            _ => None,
        }
    }
    pub fn from_str_vec(fields: Vec<String>) -> Vec<Self> {
        fields.iter().filter_map(|f| Self::from_str(f)).collect()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SearchResult {
    pub total: usize,
    pub highlight: Vec<String>,
    pub mods: Vec<SearchResultItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResultItem {
    pub id: Id,
    pub matched_fields: Vec<String>,
    pub score: f32,
}