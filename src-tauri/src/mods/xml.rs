use super::base_list::*;
use crate::types::*;
use crate::file::xml::{XmlParse, XmlWrite, ParseError, WriteError, parse, write};

use serde::{Deserialize, Serialize};
use ahash::{HashSet};
use tracing::{debug, info, trace, warn};
use quick_xml::{Reader, Writer, events::Event};
use std::io::BufRead;

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ModsConfigData {
    pub version: Version,
    #[serde(rename = "activeMods")]
    pub active_mods: Vec<PackageId>,
    #[serde(rename = "knownExpansions")]
    pub known_expansions: Vec<PackageId>,
}

impl XmlParse for Version {
    fn parse_from_reader<R: BufRead>(reader: &mut Reader<R>, end_tag: &[u8]) -> Result<Self, ParseError> {
        let content = String::parse_from_reader(reader, end_tag)?;
        Ok(Version::new(content))
    }
}

impl XmlWrite for Version {
    fn parse_to_writer<W: std::io::Write>(&self, writer: &mut quick_xml::Writer<W>, tag: &str) -> Result<(), WriteError> {
        write(writer, tag, &self.to_string())
    }
}

impl XmlParse for PackageId {
    fn parse_from_reader<R: BufRead>(reader: &mut Reader<R>, end_tag: &[u8]) -> Result<Self, ParseError> {
        let content = String::parse_from_reader(reader, end_tag)?;
        Ok(PackageId::new(content))
    }
}

impl XmlWrite for PackageId {
    fn parse_to_writer<W: std::io::Write>(&self, writer: &mut quick_xml::Writer<W>, tag: &str) -> Result<(), WriteError> {
        write(writer, tag, &self.to_string())
    }
}

// TODO
impl ModsConfigData {
    fn from_xml(xml: &str) -> Result<Self, ParseError> {
        let mut reader = Reader::from_str(xml);
        reader.config_mut().trim_text(true);

        let mut buf = Vec::new();
        let mut res = ModsConfigData {
            version: Version::default(),
            active_mods: Vec::new(),
            known_expansions: Vec::new(),
        };

        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Start(e)) => {
                    match e.name().as_ref() {
                        b"version" => res.version = parse(&mut reader, b"version")?,
                        b"activeMods" => res.active_mods = parse(&mut reader, b"activeMods")?,
                        b"knownExpansions" => res.known_expansions = parse(&mut reader, b"knownExpansions")?,
                        _ => {}
                    }   
                }
                Ok(Event::Eof) => break,
                Err(e) => return Err(ParseError::XmlError(e)),
                _ => {}
            }
        }
        Ok(res)
    }

    fn to_xml(&self) -> Result<String, WriteError> {
        use quick_xml::events::{BytesStart, BytesEnd};
        let mut writer = Writer::new(Vec::new());
        
        writer.write_event(Event::Start(BytesStart::new("ModsConfigData")))?;

        write(&mut writer, "version", &self.version)?;
        write(&mut writer, "activeMods", &self.active_mods)?;
        write(&mut writer, "knownExpansions", &self.known_expansions)?;

        writer.write_event(Event::End(BytesEnd::new("ModsConfigData")))?;

        let xml = String::from_utf8(writer.into_inner()).map_err(|e| WriteError::Custom(format!("{:?}", e)))?;
        Ok(xml)
    }
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
        let mods_config_data = match ModsConfigData::from_xml(&xml) {
            Ok(mods_config_data) => mods_config_data,
            Err(e) => {
                warn!("解析文件失败: {:?}", e);
                return Err(format!("解析文件失败: {:?}", e));
            }
        };
        let mut set: HashSet<PackageId> = HashSet::from_iter(mods_config_data.active_mods.into_iter());
        set.extend(mods_config_data.known_expansions.into_iter());
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
            mods_config_data.active_mods.push(mod_.package_id.clone());
            if DLC_LIST.contains_key(&mod_.package_id) {
                mods_config_data
                    .known_expansions
                    .push(mod_.package_id.clone());
            }
        }
        debug!("准备写入");
        trace!(mods_config_data = ?mods_config_data);
        let xml = match mods_config_data.to_xml() {
            Ok(xml) => xml,
            Err(e) => {
                warn!("生成文件失败: {:?}", e);
                return Err(format!("生成文件失败: {:?}", e));
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
