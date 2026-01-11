#![allow(dead_code)]

use std::{io::BufRead, str::FromStr};
use ahash::{HashMap, HashMapExt};
use quick_xml::{Reader, events::Event};

#[derive(Debug)]
pub enum DecodeError {
    UnsupportEncoding(String),
    UnknowEncoding,
    Custom(String)
}

pub fn try_decode(data: Vec<u8>) -> Result<String, DecodeError> {
    match String::from_utf8(data.to_vec()) {
        Ok(content) => Ok(content),
        Err(_) => {
            if let Some(result) = charset_normalizer_rs::from_bytes(&data,None).get_best() {
                let encoding_name = result.encoding();
                if let Some(encoder) = encoding_rs::Encoding::for_label(encoding_name.as_bytes()) {
                    let (decoded, _, _) = encoder.decode(&data);
                    Ok(decoded.to_string())
                } else {
                    Err(DecodeError::UnsupportEncoding(encoding_name.to_string()))
                }
            } else {
                Err(DecodeError::UnknowEncoding)
            }
        }
    }
}

#[derive(Debug)]
pub enum ParseError {
    XmlError(quick_xml::Error),
    MissingField(&'static str),
    InvalidFormat(&'static str),
    Custom(String),
}

pub trait XmlParse: Sized {
    fn parse_from_reader<R: BufRead>(reader: &mut Reader<R>, end_tag: &[u8]) -> Result<Self, ParseError>;
}

impl XmlParse for String {
    fn parse_from_reader<R: BufRead>(reader: &mut Reader<R>, end_tag: &[u8]) -> Result<Self, ParseError> {
        let mut buf = Vec::new();
        let mut content = String::new();
        
        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Text(e)) => {
                    content = e.unescape().map_err(ParseError::XmlError)?.to_string();
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
        
        Ok(content.trim().to_string())
    }
}

impl<T: XmlParse> XmlParse for Option<T> {
    fn parse_from_reader<R: BufRead>(reader: &mut Reader<R>, end_tag: &[u8]) -> Result<Self, ParseError> {
        Ok(Some(T::parse_from_reader(reader, end_tag)?))
    }
}

impl<T: XmlParse> XmlParse for Vec<T> {
    fn parse_from_reader<R: BufRead>(reader: &mut Reader<R>, end_tag: &[u8]) -> Result<Self, ParseError> {
        let mut items = Vec::new();
        let mut buf = Vec::new();
        
        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Start(e)) if e.name().as_ref() == b"li" => {
                    items.push(T::parse_from_reader(reader, b"li")?);
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
        
        Ok(items)
    }
}

impl<K: FromStr + Eq + std::hash::Hash, V: XmlParse> XmlParse for HashMap<K, V> {
    fn parse_from_reader<R: BufRead>(reader: &mut Reader<R>, end_tag: &[u8]) -> Result<Self, ParseError> {
        let mut map = HashMap::new();
        let mut buf = Vec::new();
        
        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Start(e)) => {
                    let name = e.name();
                    let key = String::from_utf8_lossy(name.as_ref());
                    let key = key.parse().map_err(|_| ParseError::InvalidFormat("Invalid key format"))?;
                    let value = V::parse_from_reader(reader, e.name().as_ref())?;
                    map.insert(key, value);
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
        
        Ok(map)
    }
}

pub fn parse<R: BufRead, T: XmlParse>(reader: &mut Reader<R>, end_tag: &[u8]) -> Result<T, ParseError> {
    T::parse_from_reader(reader, end_tag)
}