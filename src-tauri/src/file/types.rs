use ahash::{HashMap, HashSet, HashMapExt, HashSetExt};
use std::{fmt::Debug, io::BufRead};
use tracing::{warn, debug, trace};
use quick_xml::{Reader, events::Event};

use crate::types::*;
use crate::mods::SteamDb;
use super::xml::*;


#[derive(Debug, Default)]
pub struct ModMetaData {
    name: Option<String>,
    author: Option<String>,
    authors: Option<Vec<String>>,
    package_id: Option<String>,
    url: Option<String>,
    supported_versions: Option<Vec<String>>,
    target_version: Option<String>,
    description: Option<String>,
    descriptions_by_version: Option<HashMap<String, String>>, // <version, description>
    mod_dependencies: Option<Vec<ModDependency>>,
    mod_dependencies_by_version: Option<HashMap<String, Vec<ModDependency>>>,
    load_before: Option<Vec<String>>,
    load_before_by_version: Option<HashMap<String, Vec<String>>>, // <version, loadBefore>
    force_load_before: Option<Vec<String>>,
    load_after: Option<Vec<String>>,
    load_after_by_version: Option<HashMap<String, Vec<String>>>, // <version, loadAfter>
    force_load_after: Option<Vec<String>>,
    incompatible_with: Option<Vec<String>>,
    incompatible_with_by_version: Option<HashMap<String, Vec<String>>>, // <version, incompatibleWith>
}

impl ModMetaData {
    pub fn from_xml(xml: &str) -> Result<Self, ParseError> {
        debug!("开始解析ModMetaData XML");
        let mut reader = Reader::from_str(xml);
        reader.config_mut().trim_text(true);
        
        let mut buf = Vec::new();
    
        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Start(e)) => {
                if e.name().as_ref() == b"ModMetaData" {
                    return ModMetaData::parse_from_reader(&mut reader, b"ModMetaData");
                } else if e.name().as_ref() == b"modMetaData" {
                    return ModMetaData::parse_from_reader(&mut reader, b"modMetaData");
                }}
                Ok(Event::Eof) => {
                    return Err(ParseError::MissingField("ModMetaData tag not found"));
                }
                Err(e) => return Err(ParseError::XmlError(e)),
                _ => {}
            }
        }
    }
}

fn try_get_steam_id(url: Option<String>) -> Option<crate::mods::SteamId> {
    if let Some(url) = url {
        if url.contains("steam://url/CommunityFilePage/") {
            Some(crate::mods::SteamId::WorkShopId(url.split("/").last().unwrap().to_string()))
        } else if url.contains("steamcommunity.com/sharedfiles/filedetails/?id=") {
            Some(crate::mods::SteamId::WorkShopId(url.split("=").last().unwrap().to_string()))
        } else {
            None
        }
    } else {
        None
    }
}

impl ModMetaData {
    pub async fn to_mod(
        mut self,
        path: String,
        steam_db: SteamDb,
        priority: Option<Priority>
    ) -> crate::mods::ModInner {
        debug!(path = ?path, "开始转换ModMetaData到Mod");
        let steam_db_data = if path.contains("workshop\\content\\294100") {
            debug!("检测到Steam Workshop路径");
            let steam_db = steam_db.lock(priority).await;
            steam_db.get_data(path.split("\\").last().unwrap())
        } else {
            None
        };
        let language_scan_result = self.language_scan(&path);
        trace!(languages = ?language_scan_result, "语言扫描结果");
        if self.package_id.is_none() {
            debug!("未找到package_id，尝试从Steam数据获取");
            if let Some(data) = &steam_db_data {
                if let Some(data_id) = data.get_package_id() {
                    self.package_id = Some(data_id);
                } else {
                    self.package_id = Some("missing.packageid".to_string());
                }
            } else {
                self.package_id = Some("missing.packageid".to_string());
            }
        }
        if self.authors.is_some() {
            if self.author.is_some() {
                self.authors.as_mut().unwrap().push(self.author.unwrap().clone());
            }
            self.author = Some(
                self.authors.clone().unwrap().join(", ")
            )
        }
        if self.author.is_none() {
            if let Some(data) = &steam_db_data {
                if let Some(data_author) = data.get_authors() {
                    self.author = Some(data_author);
                } else {
                    self.author = Some("Missing Author".to_string());
                }
            } else {
                self.author = Some("Missing Author".to_string());
            }
        }
        if self.is_core() {
            self.name = Some("Rimworld Core".to_string());
            // 抽象 + 1
            self.supported_versions = Some(vec!["*".to_string()]);
        } else if self.is_dlc() {
            self.name = Some(
                DLC_LIST
                    .get(&PackageId::from_str(&self.package_id.clone().unwrap()))
                    .unwrap()
                    .to_string(),
            );
        }
        // 不写name的也是神人了
        // 说实话我没有遇到过，不过作为预防措施还是写上了
        if self.name.is_none() {
            if let Some(data) = &steam_db_data {
                if let Some(data_name) = data.get_name() {
                    self.name = Some(data_name);
                } else {
                    self.name = Some("Missing Name".to_string());
                }
            } else {
                self.name = Some("Missing Name".to_string());
            }
        }
        let mut mod_ = crate::mods::ModInner::default();
        mod_.id = Id::new();
        mod_.enabled = false;
        mod_.visible = true;
        mod_.package_id = PackageId::from_str(self.package_id.unwrap());
        mod_.name = self.name.clone().unwrap();
        mod_.author = self.author.unwrap();
        mod_.display_name = self.name.clone().unwrap();
        if let Some(description) = &self.description {
            mod_.description.insert(Version::all(), description.clone());
        };
        if let Some(descriptions_by_version) = &self.descriptions_by_version {
            for (version, description) in descriptions_by_version.iter() {
                mod_.description
                    .insert(Version::new(version), description.clone());
            }
        };
        if let Some(mod_dependencies) = &self.mod_dependencies {
            let mut dependencies = HashSet::new();
            for dependency in mod_dependencies.iter() {
                let steam_workshop_url = if let Some(url) = dependency.steam_workshop_url.clone() {
                    Some(url)
                } else if let Some(url) = dependency.download_url.clone() {
                    Some(url)
                } else {
                    None
                };
                dependencies.insert(crate::mods::ModDependency::new(
                    dependency.package_id.clone(),
                    dependency.display_name.clone(),
                    steam_workshop_url.clone(),
                    try_get_steam_id(steam_workshop_url),
                    false,
                ));
            }
            mod_.dependencies.insert(Version::all(), dependencies);
        };
        if let Some(mod_dependencies_by_version) = &self.mod_dependencies_by_version {
            for (version, dependencies) in mod_dependencies_by_version.iter() {
                let mut dependencies_ = HashSet::new();
                for dependency in dependencies.iter() {
                    let steam_workshop_url = if let Some(url) = dependency.steam_workshop_url.clone() {
                        Some(url)
                    } else if let Some(url) = dependency.download_url.clone() {
                        Some(url)
                    } else {
                        None
                    };
                    dependencies_.insert(crate::mods::ModDependency::new(
                        dependency.package_id.clone(),
                        dependency.display_name.clone(),
                        steam_workshop_url.clone(),
                        try_get_steam_id(steam_workshop_url),
                        false,
                    ));
                }
                mod_.dependencies
                    .insert(Version::new(version), dependencies_);
            }
        };
        if self.supported_versions.is_none() {
            if self.target_version.is_some() {
                self.supported_versions = Some(vec![self.target_version.clone().unwrap()]);
            } else {
                if let Some(data) = &steam_db_data {
                    if let Some(data_game_versions) = data.get_game_versions() {
                        self.supported_versions = Some(data_game_versions);
                    } else {
                        self.supported_versions = Some(vec![]);
                    }
                } else {
                    self.supported_versions = Some(vec![]);
                }
            }
        }
        mod_.supported_version = self
            .supported_versions
            .unwrap()
            .iter()
            .map(|v| Version::new(v))
            .collect();
        let mut load_order: HashMap<Version, HashSet<crate::mods::ModOrder>> = HashMap::new();
        if let Some(load_before) = &self.load_before {
            let mut load_before_ = HashSet::new();
            for id in load_before.iter() {
                load_before_.insert(crate::mods::ModOrder::Before(PackageId::from_str(
                    id.clone(),
                )));
            }
            load_order.insert(Version::all(), load_before_);
        };
        if let Some(load_before_by_version) = &self.load_before_by_version {
            for (version, load_before) in load_before_by_version.iter() {
                let mut load_before_ = HashSet::new();
                for id in load_before.iter() {
                    load_before_.insert(crate::mods::ModOrder::Before(PackageId::from_str(
                        id.clone(),
                    )));
                }
                load_order.insert(Version::new(version), load_before_);
            }
        };
        if let Some(force_load_before) = &self.force_load_before {
            let mut force_load_before_ = HashSet::new();
            for id in force_load_before.iter() {
                force_load_before_.insert(crate::mods::ModOrder::Before(PackageId::from_str(
                    id.clone(),
                )));
            }
            load_order.insert(Version::all(), force_load_before_);
        };
        if let Some(load_after) = &self.load_after {
            let mut load_after_ = HashSet::new();
            for id in load_after.iter() {
                load_after_.insert(crate::mods::ModOrder::After(PackageId::from_str(
                    id.clone(),
                )));
            }
            load_order.insert(Version::all(), load_after_);
        };
        if let Some(load_after_by_version) = &self.load_after_by_version {
            for (version, load_after) in load_after_by_version.iter() {
                let mut load_after_ = HashSet::new();
                for id in load_after.iter() {
                    load_after_.insert(crate::mods::ModOrder::After(PackageId::from_str(
                        id.clone(),
                    )));
                }
                load_order.insert(Version::new(version), load_after_);
            }
        };
        if let Some(force_load_after) = &self.force_load_after {
            let mut force_load_after_ = HashSet::new();
            for id in force_load_after.iter() {
                force_load_after_.insert(crate::mods::ModOrder::After(PackageId::from_str(
                    id.clone(),
                )));
            }
            load_order.insert(Version::all(), force_load_after_);
        };
        mod_.load_order = VersionMap::from_map(load_order);
        if let Some(incompatible_with) = &self.incompatible_with {
            let mut incompatible_with_ = HashSet::new();
            for id in incompatible_with.iter() {
                incompatible_with_.insert(PackageId::from_str(id.clone()));
            }
            mod_.incompatible_with
                .insert(Version::all(), incompatible_with_);
        };
        if let Some(incompatible_with_by_version) = &self.incompatible_with_by_version {
            for (version, incompatible_with) in incompatible_with_by_version.iter() {
                let mut incompatible_with_ = HashSet::new();
                for id in incompatible_with.iter() {
                    incompatible_with_.insert(PackageId::from_str(id.clone()));
                }
                mod_.incompatible_with
                    .insert(Version::new(version), incompatible_with_);
            }
        };
        mod_.support_languages = language_scan_result;
        mod_.path = path;
        debug!(id = ?mod_.id, name = ?mod_.name, "Mod对象创建完成");
        mod_
    }
    fn is_core(&self) -> bool {
        self.package_id == Some("Ludeon.RimWorld".to_string())
    }
    fn is_dlc(&self) -> bool {
        if let Some(package_id) = self.package_id.as_ref() {
            DLC_LIST.contains_key(&PackageId::from_str(package_id))
        } else {
            false
        }
    }
    fn language_scan(&self, path: &str) -> VersionMap<HashSet<String>> {
        debug!(path = ?path, "开始扫描mod支持的语言");
        let mut language_scan_result = VersionMap::new();
        if self.is_core() || self.is_dlc() {
            trace!("检测到core/DLC，添加所有支持的语言");
            language_scan_result.insert(Version::all(), LANGUAGE_CODE_TO_NAME
            .keys()
            .map(|k| k.to_string())
            .collect());
        } else {
            if std::fs::exists(format!("{}\\Languages", path)).unwrap() {
                trace!("开始扫描Languages目录");
                let mut t = HashSet::new();
                for entry in std::fs::read_dir(format!("{}\\Languages", path)).unwrap() {
                    let entry = entry.unwrap();
                    let path = entry.path();
                    if path.is_dir() {
                        let path = path.to_str().unwrap().to_string();
                        debug!(path = ?path);
                        match LANGUAGE_NAME_TO_CODE.get(path.split("\\").last().unwrap()) {
                            Some(code) => {
                                t.insert(code.to_string());
                            }
                            None => {
                                warn!(path = ?path, "未知语言");
                            }
                        }
                    }
                }
                if !t.is_empty() {
                language_scan_result.insert(Version::all(), t);
                }
            }

            let entries = match std::fs::read_dir(path) {
                Ok(entries) => entries,
                Err(_) => return language_scan_result,
            };

            for entry in entries {
                let entry = match entry {
                    Ok(e) => e,
                    Err(_) => continue,
                };

                let version_path = entry.path();
                if !version_path.is_dir() {
                    continue;
                }

                let version = match version_path.file_name() {
                    Some(v) => v.to_str().unwrap().to_string(),
                    None => continue,
                };

                if !Version::is_version(&version) {
                    continue;
                }

                let languages_path = version_path.join("Languages");
                if !languages_path.exists() || !languages_path.is_dir() {
                    continue;
                }

                trace!("扫描版本目录{version}下的Languages文件夹");

                let mut t = HashSet::new();
                for entry in std::fs::read_dir(languages_path).unwrap() {
                    let entry = entry.unwrap();
                    let path = entry.path();
                    if path.is_dir() {
                        let path = path.to_str().unwrap().to_string();
                        debug!(path = ?path);
                        match LANGUAGE_NAME_TO_CODE.get(path.split("\\").last().unwrap()) {
                            Some(code) => {
                                t.insert(code.to_string());
                            }
                            None => {
                                warn!(path = ?path, "未知语言");
                            }
                        }
                    }
                }

                if !t.is_empty() {
                    language_scan_result.insert(Version::new(&version), t);
                }
            }
        }
        debug!(supported_languages = ?language_scan_result, "语言扫描完成");
        language_scan_result
    }
}


#[derive(Debug)]
pub struct ModDependency {
    package_id: String,
    display_name: Option<String>,
    steam_workshop_url: Option<String>,
    download_url: Option<String>,
}

impl XmlParse for ModDependency {
    fn parse_from_reader<R: BufRead>(reader: &mut Reader<R>, end_tag: &[u8]) -> Result<Self, ParseError> {
        let mut package_id = None;
        let mut display_name = None;
        let mut steam_workshop_url = None;
        let mut download_url = None;
        let mut buf = Vec::new();
        
        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Start(e)) => {
                    match e.name().as_ref() {
                        b"packageId" => package_id = parse(reader, b"packageId")?,
                        b"displayName" => display_name = parse(reader, b"displayName")?,
                        b"steamWorkshopUrl" => steam_workshop_url = parse(reader, b"steamWorkshopUrl")?,
                        b"downloadUrl" => download_url = parse(reader, b"downloadUrl")?,
                        _ => {}
                    }
                }
                Ok(Event::End(e)) if e.name().as_ref() == end_tag => {
                    break;
                }
                Ok(Event::Eof) => {
                    return Err(ParseError::MissingField("Unexpected EOF"));
                }
                Err(e) => return Err(ParseError::XmlError(e)),
                _ => {}
            }
        }
        
        Ok(ModDependency {
            package_id: package_id.ok_or(ParseError::MissingField("packageId"))?,
            display_name: display_name,
            steam_workshop_url,
            download_url,
        })
    }
}

// ModMetaData 的解析实现
impl XmlParse for ModMetaData {
    fn parse_from_reader<R: BufRead>(reader: &mut Reader<R>, end_tag: &[u8]) -> Result<Self, ParseError> {
        let mut meta = ModMetaData::default();
        let mut buf = Vec::new();
        
        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Start(e)) => {
                    match e.name().as_ref() {
                        b"name" => meta.name = parse(reader, b"name")?,
                        b"author" => meta.author = parse(reader, b"author")?,
                        b"authors" => meta.authors = parse(reader, b"authors")?,
                        b"packageId" => meta.package_id = parse(reader, b"packageId")?,
                        b"url" => meta.url = parse(reader, b"url")?,
                        b"supportedVersions" => meta.supported_versions = parse(reader, b"supportedVersions")?,
                        b"targetVersion" => meta.target_version = parse(reader, b"targetVersion")?,
                        b"description" => meta.description = parse(reader, b"description")?,
                        b"descriptionsByVersion" => meta.descriptions_by_version = parse(reader, b"descriptionsByVersion")?,
                        b"modDependencies" => meta.mod_dependencies = parse(reader, b"modDependencies")?,
                        b"modDependenciesByVersion" => meta.mod_dependencies_by_version = parse(reader, b"modDependenciesByVersion")?,
                        b"loadBefore" => meta.load_before = parse(reader, b"loadBefore")?,
                        b"loadBeforeByVersion" => meta.load_before_by_version = parse(reader, b"loadBeforeByVersion")?,
                        b"forceLoadBefore" => meta.force_load_before = parse(reader, b"forceLoadBefore")?,
                        b"loadAfter" => meta.load_after = parse(reader, b"loadAfter")?,
                        b"loadAfterByVersion" => meta.load_after_by_version = parse(reader, b"loadAfterByVersion")?,
                        b"forceLoadAfter" => meta.force_load_after = parse(reader, b"forceLoadAfter")?,
                        b"incompatibleWith" => meta.incompatible_with = parse(reader, b"incompatibleWith")?,
                        b"incompatibleWithByVersion" => meta.incompatible_with_by_version = parse(reader, b"incompatibleWithByVersion")?,
                        _ => {}
                    }
                }
                Ok(Event::End(e)) if e.name().as_ref() == end_tag => {
                    break;
                }
                Ok(Event::Eof) => {
                    return Err(ParseError::MissingField("Unexpected EOF"));
                }
                Err(e) => return Err(ParseError::XmlError(e)),
                _ => {}
            }
        }
        
        Ok(meta)
    }
}
