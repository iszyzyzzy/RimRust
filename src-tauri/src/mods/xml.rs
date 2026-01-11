use super::base_list::*;
use crate::types::*;

use serde::{Deserialize, Serialize};
use ahash::{HashSet, HashSetExt};
use tracing::{debug, info, trace, warn};

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ModsConfigData {
    pub version: Version,
    #[serde(rename = "activeMods")]
    pub active_mods: Vec<Li<PackageId>>,
    #[serde(rename = "knownExpansions")]
    pub known_expansions: Vec<Li<PackageId>>,
}

impl BaseList {
    pub async fn load_from_xml(
        &self,
        xml_path: &str,
    ) -> Result<(), String> {
        info!(path = ?xml_path, "load_from_xml");
        let xml = match std::fs::read_to_string(xml_path) {
            Ok(xml) => xml,
            Err(e) => {
                warn!("读取文件失败: {}", e);
                return Err(format!("读取文件失败: {}", e));
            }
        };
        let mods_config_data: ModsConfigData = match quick_xml::de::from_str(&xml) {
            Ok(mods_config_data) => mods_config_data,
            Err(e) => {
                warn!("解析文件失败: {}", e);
                return Err(format!("解析文件失败: {}", e));
            }
        };
        let mut set: HashSet<PackageId> = HashSet::from_iter(Li::into_vec(mods_config_data.active_mods));
        set.extend(Li::into_vec(mods_config_data.known_expansions));
        debug!(set = ?set, "启用的package_id list");

        for item in self.mods_map.iter() {
            let mod_ = item.value();
            let mut mod_ = mod_.lock().await;
            if set.contains(&mod_.package_id) {
                mod_.enabled = true;
                set.remove(&mod_.package_id);
            } else {
                mod_.enabled = false;
            }
        }
        Ok(())
    }
    pub async fn save_to_xml(
        &self,
        xml_path: &str,
        game_version: Version,
        mods: Vec<Id>,
    ) -> Result<(), String> {
        info!(path = ?xml_path, "save_to_xml");
        let mut mods_config_data = ModsConfigData {
            version: game_version,
            active_mods: Vec::new(),
            known_expansions: Vec::new(),
        };
        for mod_ in mods {
            let mod_ = self.mods_map.get(&mod_).unwrap();
            let mod_ = mod_.lock().await;
            if mod_.enabled {
                mods_config_data.active_mods.push(Li::new(mod_.package_id.clone()));
                if DLC_LIST.contains_key(&mod_.package_id) {
                    mods_config_data
                        .known_expansions
                        .push(Li::new(mod_.package_id.clone()));
                }
            }
        }
        debug!("准备写入");
        trace!(mods_config_data = ?mods_config_data);
        let xml = match quick_xml::se::to_string(&mods_config_data) {
            Ok(xml) => xml,
            Err(e) => {
                warn!("生成文件失败: {}", e);
                return Err(format!("生成文件失败: {}", e));
            }
        };
        match std::fs::write(xml_path, xml) {
            Ok(_) => Ok(()),
            Err(e) => {
                warn!("写入文件失败: {}", e);
                return Err(format!("写入文件失败: {}", e));
            }
        }
    }
}
