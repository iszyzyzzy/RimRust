// use super::*;

use ahash::{HashMap, HashMapExt};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use tracing::warn;

#[derive(
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Serialize,
    Deserialize,
    Default,
    Debug
)]
#[serde(transparent)]
pub struct Id(uuid::Uuid);
impl Id {
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4())
    }
    pub fn from_str(id: impl Into<String>) -> Self {
        match uuid::Uuid::try_parse(&id.into()) {
            Ok(uuid) => Self(uuid),
            Err(_) => Self::default(),
        }
    }
    pub fn try_from_str(id: impl Into<String>) -> Result<Self, uuid::Error> {
        let uuid = uuid::Uuid::try_parse(&id.into())?;
        Ok(Self(uuid))
    }
    pub fn to_string(&self) -> String {
        self.0.to_string()
    }
    pub fn enpty() -> Self {
        Self(uuid::Uuid::nil())
    }
}
impl Into<String> for Id {
    fn into(self) -> String {
        self.0.to_string()
    }
}
impl From<String> for Id {
    fn from(s: String) -> Self {
        Self::from_str(s)
    }
}
impl From<&str> for Id {
    fn from(s: &str) -> Self {
        Self::from_str(s)
    }
}
impl PartialEq<&Id> for Id {
    fn eq(&self, other: &&Id) -> bool {
        self.0 == other.0
    }
}
impl PartialEq<Id> for &Id {
    fn eq(&self, other: &Id) -> bool {
        self.0 == other.0
    }
}
impl bincode::Encode for Id {
    fn encode<E: bincode::enc::Encoder>(&self, encoder: &mut E) -> Result<(), bincode::error::EncodeError> {
        bincode::Encode::encode(&self.0.as_bytes(), encoder)?;
        Ok(())
    }
}
impl<Context> bincode::Decode<Context> for Id {
    fn decode<D: bincode::de::Decoder<Context = Context>>(
        decoder: &mut D,
    ) -> Result<Self, bincode::error::DecodeError> {
        let bytes: [u8; 16] = bincode::Decode::decode(decoder)?;
        let uuid = uuid::Uuid::from_slice(&bytes).map_err(|_| bincode::error::DecodeError::Other("Invalid UUID"))?;
        Ok(Self(uuid))
    }
}
impl<'de, Context> bincode::BorrowDecode<'de, Context> for Id {
    fn borrow_decode<D: bincode::de::BorrowDecoder<'de, Context = Context>>(
        decoder: &mut D,
    ) -> Result<Self, bincode::error::DecodeError> {
        let bytes: [u8; 16] = bincode::BorrowDecode::borrow_decode(decoder)?;
        let uuid = uuid::Uuid::from_slice(&bytes).map_err(|_| bincode::error::DecodeError::Other("Invalid UUID"))?;
        Ok(Self(uuid))
    }
}


#[derive(Clone, Serialize, Deserialize, Debug, bincode::Decode, bincode::Encode)]
#[serde(transparent)]
pub struct PackageId(String);
impl PartialEq for PackageId {
    // packageId应全部小写，但是你猜怎么着官方dlc的packageId有大写但是读取modconfig的时候是小写
    fn eq(&self, other: &Self) -> bool {
        self.0.to_lowercase() == other.0.to_lowercase()
    }
}
impl Eq for PackageId {}
impl std::hash::Hash for PackageId {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.to_lowercase().hash(state)
    }
}
impl Default for PackageId {
    fn default() -> Self {
        Self("".to_string())
    }
}
impl PackageId {
    pub fn new(package_id: impl Into<String>) -> Self {
        Self(package_id.into())
    }
    pub fn _to_lowercase(&self) -> Self {
        Self(self.0.to_lowercase())
    }
    pub fn from_str(package_id: impl Into<String>) -> Self {
        Self(package_id.into())
    }
    pub fn to_string(&self) -> String {
        self.0.to_lowercase().clone()
    }
}

#[derive(
    Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, bincode::Decode, bincode::Encode,
)]
#[serde(transparent)]
pub struct Version(String);
// eg 1.5.4297 rev1078
impl Default for Version {
    fn default() -> Self {
        Self("*".to_string())
    }
}
impl Version {
    pub fn new(version: impl Into<String>) -> Self {
        let t = version.into();
        // 去掉可能的开头v
        let t = if t.starts_with('v') { &t[1..] } else { &t };
        if Self::is_version(t) {
            Self(t.to_string())
        } else {
            warn!("不合法的版本号: {}", t);
            Self("*".to_string())
        }
    }
    pub fn to_short(&self) -> String {
        // eg 1.5.4297 rev1078 -> 1.5
        if self.0 == "*" {
            return "*".to_string();
        } else {
            self.0
                .split(' ')
                .next()
                .unwrap()
                .split('.')
                .collect::<Vec<&str>>()[0..=1]
                .join(".")
        }
    }
    pub fn to_short_ver(&self) -> Self {
        Self(self.to_short())
    }
    pub fn all() -> Self {
        Self("*".to_string())
    }
    pub fn _matches(&self, other: &Self) -> bool {
        self.0 == "*" || other.0 == "*" || self.to_short() == other.to_short()
    }
    pub fn proximity(&self, other: &Self) -> f64 {
        if self.to_short() == other.to_short() {
            return 0.0;
        }
        if self.0 == "*" || other.0 == "*" {
            return 0.5;
        }
        let parse_version = |v: &str| -> f64 {
            v.split('.')
                .take(2)
                .enumerate()
                .map(|(i, num)| num.parse::<f64>().unwrap_or(0.0) * if i == 0 { 1.0 } else { 0.1 })
                .sum()
        };

        let v1 = parse_version(&self.to_short());
        let v2 = parse_version(&other.to_short());

        (v1 - v2).abs() + 1.0
    }
    pub fn is_version(string: impl Into<String>) -> bool {
        // 可以接受
        // 1.5.4297 rev1078
        // 1.5.4297
        // 1.5
        // 上面所有的加上v
        // *

        let string = string.into();
    
        // 如果是通配符"*"，直接返回true
        if string == "*" {
            return true;
        }
        
        // 去掉可能的开头v
        let version_str = if string.starts_with('v') {
            &string[1..]
        } else {
            &string
        };
        
        // 匹配主要版本号格式: 1.5 或 1.5.4297 等
        let mut parts = version_str.split(' ');
        let version_parts = parts.next().unwrap_or("");
        
        // 检查版本号部分是否合法（例如1.5或1.5.4297）
        let version_valid = version_parts
            .split('.')
            .all(|part| !part.is_empty() && part.chars().all(|c| c.is_ascii_digit()));
        
        // 必须至少有一个点号分隔符
        if !version_parts.contains('.') || !version_valid {
            return false;
        }
        
        // 剩下的不管啦
        true
    }

    pub fn to_string(&self) -> String {
        self.0.clone()
    }
}

#[derive(
    Clone, Debug, Serialize, Deserialize, bincode::Decode, bincode::Encode, Default, PartialEq, Eq,
)]
#[serde(transparent)]
pub struct VersionMap<T>(HashMap<Version, T>);

impl<T> VersionMap<T> {
    pub fn new() -> Self {
        Self(HashMap::new())
    }
    pub fn insert(&mut self, version: Version, value: T) {
        self.0.insert(version, value);
    }
    pub fn get(&self, version: &Version) -> Option<&T> {
        if let Some(data) = self.0.get(version) {
            return Some(data);
        }
        self.get_closest(version)
    }
    pub fn get_closest(&self, version: &Version) -> Option<&T> {
        self.0
            .iter()
            .min_by(|(v1, _), (v2, _)| {
                version
                    .proximity(v1)
                    .partial_cmp(&version.proximity(v2))
                    .unwrap()
            })
            .map(|(_, v)| v)
    }
    pub fn from_map(map: HashMap<Version, T>) -> Self {
        Self(map)
    }
}

impl<T> IntoIterator for VersionMap<T> {
    type Item = (Version, T);
    type IntoIter = std::collections::hash_map::IntoIter<Version, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<'a, T> IntoIterator for &'a VersionMap<T> {
    type Item = (&'a Version, &'a T);
    type IntoIter = std::collections::hash_map::Iter<'a, Version, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

impl<'a, T> IntoIterator for &'a mut VersionMap<T> {
    type Item = (&'a Version, &'a mut T);
    type IntoIter = std::collections::hash_map::IterMut<'a, Version, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter_mut()
    }
}

impl<T> VersionMap<T> {
    pub fn iter(&self) -> std::collections::hash_map::Iter<'_, Version, T> {
        self.0.iter()
    }

    pub fn iter_mut(&mut self) -> std::collections::hash_map::IterMut<'_, Version, T> {
        self.0.iter_mut()
    }
}

use std::sync::atomic::{AtomicUsize, Ordering};

pub static SYNC_MESSAGE_ID: Lazy<AtomicUsize> = Lazy::new(|| AtomicUsize::new(1));

pub fn next_sync_id() -> usize {
    SYNC_MESSAGE_ID.fetch_add(1, Ordering::Relaxed)
}


// /// 用于序列化xml的<li>标签的辅助结构
// #[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
// #[serde(rename = "li", transparent)]
// pub struct Li<T>(pub T);

// impl<T> Li<T> {
//     pub fn new(value: T) -> Self {
//         Self(value)
//     }
//     pub fn into_inner(self) -> T {
//         self.0
//     }
//     pub fn into_vec(vec: Vec<Self>) -> Vec<T> {
//         vec.into_iter().map(|li| li.0).collect()
//     }
// }

// impl<T> From<T> for Li<T> {
//     fn from(value: T) -> Self {
//         Self(value)
//     }
// }