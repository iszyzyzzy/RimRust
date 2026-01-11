use ahash::{HashMap, HashSet, HashSetExt, HashMapExt};

use super::base_list::*;
use crate::types::*;

use dashmap::DashSet;
use serde::{Deserialize, Serialize};
use tracing::{debug, info, instrument, warn};

#[derive(Serialize, Deserialize, Debug)]
pub struct ScanResult {
    pub info: HashMap<Id, Vec<InfoType>>,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Hash, Clone, bincode::Decode, bincode::Encode)]
pub enum InfoType {
    Warning(WarningType),
    Error(ErrorType),
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Hash, Clone, bincode::Decode, bincode::Encode)]
pub enum WarningType {
    VersionMismatch,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Hash, Clone, bincode::Decode, bincode::Encode)]
pub enum ErrorType {
    DuplicatePackageId(Id),
    MissingDependency(ModDependency),
}

// 实时错误扫描
impl BaseList {
    //#[instrument(skip(self))]
    pub async fn scan(&self, game_version: Version) -> ScanResult {
        info!("开始扫描");
        let mut result = ScanResult {
            info: HashMap::new(),
        };
        let mut checked_mods: HashSet<PackageId> = HashSet::with_capacity(self.mods_map.len());
        let mut enable_mods: HashSet<PackageId> = HashSet::with_capacity(self.mods_map.len() / 2);
        info!("第一次遍历");
        for mod_ in self.mods_map.iter().map(|item| item.value().clone()) {
            let mod_ = mod_.lock().await;
            if mod_.enabled {
                enable_mods.insert(mod_.package_id.clone());
            }
            if checked_mods.contains(&mod_.package_id) {
                result.info.insert(
                    mod_.id.clone(),
                    vec![InfoType::Error(ErrorType::DuplicatePackageId(
                        mod_.id.clone(),
                    ))],
                );
            }
            checked_mods.insert(mod_.package_id.clone());
        }
        info!(
            "共统计 {} 个mod, {} 已启用",
            checked_mods.len(),
            enable_mods.len()
        );
        debug!(checked_mods = ?checked_mods, enable_mods = ?enable_mods, "raw");
        info!("第二次遍历");
        for mod_ in self.mods_map.iter().map(|item| item.value().clone()) {
            let mod_ = mod_.lock().await;
            if !mod_.enabled {
                continue;
            }
            let mut info = Vec::new();
            if let Some(dependencies) = mod_.dependencies.get(&game_version) {
                for dependency in dependencies {
                    if !checked_mods.contains(&dependency.package_id) {
                        info.push(InfoType::Error(ErrorType::MissingDependency(
                            dependency.clone(),
                        )));
                    }
                }
            }
            if !info.is_empty() {
                result.info.insert(mod_.id.clone(), info);
            }
        }

        let error_count = result
            .info
            .values()
            .map(|infos| {
                infos
                    .iter()
                    .filter(|info| matches!(info, InfoType::Error(_)))
                    .count()
            })
            .sum::<usize>();

        let warning_count = result
            .info
            .values()
            .map(|infos| {
                infos
                    .iter()
                    .filter(|info| matches!(info, InfoType::Warning(_)))
                    .count()
            })
            .sum::<usize>();
        info!(errors = ?error_count, warnings = ?warning_count,"扫描完成,原始结果");
        debug!(result = ?result, "raw");
        let mut remove_list = Vec::new();
        for id in result.info.keys() {
            if let Some(ignore_info) = self.user_ignore_info.get(id).map(|item| item.value().clone()) {
                for info in result.info.get(id).unwrap() {
                    if ignore_info.contains(info) {
                        remove_list.push((id.clone(), info.clone()));
                    }
                }
            }
        }
        for (id, info) in remove_list {
            result.info.get_mut(&id).unwrap().retain(|x| x != &info);
        }
        let error_count = result
            .info
            .values()
            .map(|infos| {
                infos
                    .iter()
                    .filter(|info| matches!(info, InfoType::Error(_)))
                    .count()
            })
            .sum::<usize>();

        let warning_count = result
            .info
            .values()
            .map(|infos| {
                infos
                    .iter()
                    .filter(|info| matches!(info, InfoType::Warning(_)))
                    .count()
            })
            .sum::<usize>();
        info!(
            errors = error_count,
            warnings = warning_count,
            "排除完成,最终结果"
        );
        debug!(result = ?result, "raw");
        result
    }
    pub async fn add_ignore_info(&self, mod_id: Id, info: InfoType) {
        info!(id = ?mod_id,info = ?info, "添加忽略信息");
        self.user_ignore_info
            .entry(mod_id)
            .or_insert_with(DashSet::new)
            .insert(info);
    }
    pub async fn remove_ignore_info(&self, mod_id: Id, info: InfoType) {
        info!(id = ?mod_id,info = ?info, "移除忽略信息");
        if let Some(set) = self.user_ignore_info.get_mut(&mod_id) {
            set.value().remove(&info);
        } else {
            warn!(id = ?mod_id,info = ?info, "未找到忽略信息");
        }
    }
}
