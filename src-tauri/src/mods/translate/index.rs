use std::collections::BTreeMap;
use ahash::{HashMap, HashMapExt, HashSet, HashSetExt};

use crate::{mods::{ModInner, ModOrder}, types::*};

#[derive(Clone, bincode::Encode, bincode::Decode)]
pub struct ModIndex {
    pub name_to_id: HashMap<String, Id>,
    pub id_to_name: HashMap<Id, String>,
    pub names_by_length: BTreeMap<usize, Vec<Id>>,
    pub id_to_package_id: HashMap<Id, PackageId>,
    pub id_to_author: HashMap<Id, String>,
    pub load_after_map: HashMap<PackageId, Vec<Id>>,
    pub(super) change: bool,
    game_version: Version,
}

impl ModIndex {
    pub fn new(version: Version) -> Self {
        Self {
            name_to_id: HashMap::new(),
            id_to_name: HashMap::new(),
            names_by_length: BTreeMap::new(),
            id_to_package_id: HashMap::new(),
            id_to_author: HashMap::new(),
            load_after_map: HashMap::new(),
            change: false,
            game_version: version,
        }
    }

    pub fn add_mod(&mut self, mod_: &ModInner) {
        let id = mod_.id;
        let name = mod_.name.clone();
        let package_id = mod_.package_id.clone();

        self.name_to_id.insert(name.clone(), id);
        self.id_to_name.insert(id, name.clone());
        self.names_by_length
            .entry(name.len())
            .or_insert_with(Vec::new)
            .push(id);
        self.id_to_package_id.insert(id, package_id.clone());
        self.id_to_author.insert(id, mod_.author.clone());
        for order in mod_.load_order.get(&self.game_version).unwrap_or(&HashSet::new()) {
            if let ModOrder::After(package_id) = order {
                self.load_after_map
                    .entry(package_id.clone())
                    .or_insert_with(Vec::new)
                    .push(id.clone());
            }
        }
        self.change = true;
    }

    pub fn remove_mod(&mut self, id: Id) {
        if let Some(name) = self.id_to_name.remove(&id) {
            self.name_to_id.remove(&name);
            if let Some(length) = self.names_by_length.get_mut(&name.len()) {
                length.retain(|&x| x != id);
            }
        }
        self.id_to_author.remove(&id);
        if let Some(package_id) = self.id_to_package_id.remove(&id) {
            self.load_after_map.remove(&package_id);
        }
        self.change = true;
    }

    pub fn update_mod(&mut self, mod_: &ModInner) {
        if let Some(old_name) = self.id_to_name.get(&mod_.id).cloned() {
            if old_name != mod_.name {
                self.remove_mod(mod_.id);
                self.add_mod(mod_);
            }
        } else {
            self.add_mod(mod_);
        }
    }

    pub fn contains(&self, id: Id) -> bool {
        self.id_to_name.contains_key(&id)
    }
    
    pub fn reset_change(&mut self) {
        self.change = false;
    }
}