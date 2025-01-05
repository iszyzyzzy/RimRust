use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};


#[derive(Serialize, Deserialize, Clone)]
pub struct Mod {
    id: String,
    enabled: bool,
    package_id: String,
    name: String,
    description: String,
    dependencies: Vec<ModDependency>,
    order: Vec<ModOrder>,
}

#[derive(Serialize, Deserialize, Clone)]
pub enum ModOrder {
    Before(String),// target mod id
    After(String),
    First(String),
    Last(String),
}

#[derive(Serialize, Deserialize, Clone)]
pub enum ModDependency {
    Optional(String),
    Required(String),
}

#[derive(Serialize, Deserialize, Default)]
pub struct BaseList {
    mods: Vec<Arc<Mutex<Mod>>>,
    mods_groups: Vec<Arc<Mutex<ModsGroup>>>,
}

impl BaseList {
    pub fn recover_from_file(&mut self, app_data_path: String) {
        if !std::fs::exists(format!("{}/app_data.json", app_data_path)).unwrap() {
            return;
        }
        let base_list: BaseListForSave = serde_json::from_str(
            &std::fs::read_to_string(format!("{}/app_data.json", app_data_path)).unwrap(),
        )
        .unwrap();
        for mod_ in base_list.mods {
            self.mods.push(Arc::new(Mutex::new(mod_)));
        }
        let mod_map: std::collections::HashMap<String, Arc<Mutex<Mod>>> = self
            .mods
            .iter()
            .map(|mod_| (mod_.lock().unwrap().id.clone(), mod_.clone()))
            .collect();
        let mut group_map: std::collections::HashMap<String, Arc<Mutex<ModsGroup>>> =
            std::collections::HashMap::new();
        for mods_group in &base_list.mods_groups {
            let group = Arc::new(Mutex::new(ModsGroup {
                id: mods_group.id.clone(),
                name: mods_group.name.clone(),
                mods: Vec::new(),
            }));
            group_map.insert(group.lock().unwrap().id.clone(), group.clone());
            self.mods_groups.push(group);
        }

        for (i, mods_group) in base_list.mods_groups.iter().enumerate() {
            let group = self.mods_groups[i].clone();
            let mut group = group.lock().unwrap();

            for item in &mods_group.mods {
                match item {
                    ModsGroupItemForSave::Mod(id) => {
                        group
                            .mods
                            .push(ModsGroupItem::Mod(mod_map.get(id).unwrap().clone()));
                    }
                    ModsGroupItemForSave::ModsGroup(id) => {
                        group
                            .mods
                            .push(ModsGroupItem::ModsGroup(group_map.get(id).unwrap().clone()));
                    }
                }
            }
        }
    }
    pub fn to_save(&self) -> BaseListForSave {
        let base_list = BaseListForSave {
            mods: self
                .mods
                .iter()
                .map(|mod_| mod_.lock().unwrap().clone())
                .collect(),
            mods_groups: self
                .mods_groups
                .iter()
                .map(|group| {
                    let group = group.lock().unwrap();
                    ModsGroupForSave {
                        id: group.id.clone(),
                        name: group.name.clone(),
                        mods: group
                            .mods
                            .iter()
                            .map(|item| match item {
                                ModsGroupItem::Mod(mod_) => {
                                    ModsGroupItemForSave::Mod(mod_.lock().unwrap().id.clone())
                                }
                                ModsGroupItem::ModsGroup(group_) => {
                                    ModsGroupItemForSave::ModsGroup(group_.lock().unwrap().id.clone())
                                }
                            })
                            .collect(),
                    }
                })
                .collect(),
        };
        base_list
    }
    pub fn save(&self, app_data_path: String) {
        std::fs::write(
            format!("{}/app_data.json", app_data_path),
            serde_json::to_string(&self.to_save()).unwrap(),
        )
        .unwrap();
    }
    pub fn get_mod(&self, id: &str) -> Option<Arc<Mutex<Mod>>> {
        self.mods
            .iter()
            .find(|mod_| mod_.lock().unwrap().id == id)
            .map(|mod_| mod_.clone())
    }
    pub fn get_mod_group(&self, id: &str) -> Option<Arc<Mutex<ModsGroup>>> {
        self.mods_groups
            .iter()
            .find(|group| group.lock().unwrap().id == id)
            .map(|group| group.clone())
    }
    pub fn set_enable_mod(&self, mod_id: &str, enabled: bool) -> Result<(), String> {
        let mod_ = self.get_mod(mod_id).ok_or("Mod not found")?;
        mod_.lock().unwrap().enabled = enabled;
        Ok(())
    }
    pub fn set_enable_mod_group(&self, group_id: &str, enabled: bool) -> Result<(), String> {
        let group = self.get_mod_group(group_id).ok_or("Group not found")?;
        group.lock().unwrap().change_enable(enabled)?;
        Ok(())
    }
}

#[derive(Serialize, Deserialize)]
pub enum ModsGroupItem {
    Mod(Arc<Mutex<Mod>>),
    ModsGroup(Arc<Mutex<ModsGroup>>),
}

#[derive(Serialize, Deserialize)]
pub struct ModsGroup {
    id: String,
    name: String,
    mods: Vec<ModsGroupItem>,
}

impl ModsGroup {
    pub fn change_enable(&self, enabled: bool) -> Result<(), String> {
        for item in &self.mods {
            match item {
                ModsGroupItem::Mod(mod_) => {
                    mod_.lock().unwrap().enabled = enabled;
                }
                ModsGroupItem::ModsGroup(group_) => {
                    group_.lock().unwrap().change_enable(enabled)?;
                }
            }
        }
        Ok(())
    }
}

#[derive(Serialize, Deserialize)]
struct ModsGroupForSave {
    id: String,
    name: String,
    mods: Vec<ModsGroupItemForSave>,
}

#[derive(Serialize, Deserialize)]
enum ModsGroupItemForSave {
    Mod(String),
    ModsGroup(String),
}

#[derive(Serialize, Deserialize)]
pub struct BaseListForSave {
    mods: Vec<Mod>,
    mods_groups: Vec<ModsGroupForSave>,
}

#[tauri::command]
pub async fn get_base_list(
    base_list: tauri::State<'_, BaseList>,
) -> Result<BaseListForSave, String> {
    Ok(base_list.to_save())
}

#[tauri::command]
pub async fn set_enable_mod(
    base_list: tauri::State<'_, BaseList>,
    mod_id: Vec<String>,
    enabled: bool,
) -> Result<(), String> {
    for id in mod_id {
        base_list.set_enable_mod(&id, enabled)?;
    }
    Ok(())
}

#[tauri::command]
pub async fn set_enable_mod_group(
    base_list: tauri::State<'_, BaseList>,
    group_id: String,
    enabled: bool,
) -> Result<(), String> {
    base_list.set_enable_mod_group(&group_id, enabled)
}