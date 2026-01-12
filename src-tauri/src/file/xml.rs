#![allow(dead_code)]

use std::{io::BufRead, str::FromStr};
use ahash::{HashMap, HashMapExt};
use quick_xml::{Reader, events::Event};

#[derive(Debug)]
pub enum DecodeError {
    UnsupportEncoding(String),
    UnknowCharsets(String),
    Custom(String)
}

pub fn try_decode(data: Vec<u8>) -> Result<String, DecodeError> {
    match String::from_utf8(data.to_vec()) {
        Ok(content) => Ok(content),
        Err(_) => {
            match charset_normalizer_rs::from_bytes(&data, None) {
                Ok(possible_charsets) => {
                    if let Some(encoding) = possible_charsets.get_best() {
                        if let Some(encoder) = encoding_rs::Encoding::for_label(encoding.encoding().as_bytes()) {
                            let (decoded, _, _) = encoder.decode(&data);
                            Ok(decoded.to_string())
                        } else {
                            Err(DecodeError::UnsupportEncoding(encoding.encoding().to_string()))
                        }
                    } else {
                        Err(DecodeError::UnknowCharsets("字符集检测无结果".to_string()))
                    }
                }
                Err(e) => Err(DecodeError::UnknowCharsets(format!("尝试检测字符集失败: {}", e))),
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

#[derive(Debug)]
pub enum WriteError {
    XmlError(quick_xml::Error),
    IoError(std::io::Error),
    Custom(String),
}

impl From<quick_xml::Error> for WriteError {
    fn from(err: quick_xml::Error) -> Self {
        WriteError::XmlError(err)
    }
}

impl From<std::io::Error> for WriteError {
    fn from(err: std::io::Error) -> Self {
        WriteError::IoError(err)
    }
}

pub trait XmlParse: Sized {
    fn parse_from_reader<R: BufRead>(reader: &mut Reader<R>, end_tag: &[u8]) -> Result<Self, ParseError>;
    //fn parse_to_writer<W: std::io::Write>(&self, writer: &mut quick_xml::Writer<W>, tag: &str) -> Result<(), WriteError>;
}

pub trait XmlWrite: Sized {
    fn parse_to_writer<W: std::io::Write>(&self, writer: &mut quick_xml::Writer<W>, tag: &str) -> Result<(), WriteError>;
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

impl XmlWrite for String  {
    fn parse_to_writer<W: std::io::Write>(&self, writer: &mut quick_xml::Writer<W>, tag: &str) -> Result<(), WriteError> {
        use quick_xml::events::{BytesStart, BytesText, BytesEnd};
        
        writer.write_event(Event::Start(BytesStart::new(tag)))?;
        if !self.is_empty() {
            writer.write_event(Event::Text(BytesText::new(self)))?;
        }
        writer.write_event(Event::End(BytesEnd::new(tag)))?;
        Ok(())
    }
}

impl<T: XmlParse> XmlParse for Option<T> {
    fn parse_from_reader<R: BufRead>(reader: &mut Reader<R>, end_tag: &[u8]) -> Result<Self, ParseError> {
        Ok(Some(T::parse_from_reader(reader, end_tag)?))
    }
}

impl<T: XmlWrite> XmlWrite for Option<T> {
    fn parse_to_writer<W: std::io::Write>(&self, writer: &mut quick_xml::Writer<W>, tag: &str) -> Result<(), WriteError> {
        if let Some(value) = self {
            value.parse_to_writer(writer, tag)?;
        }
        Ok(())
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

impl <T: XmlWrite> XmlWrite for Vec<T> {
    fn parse_to_writer<W: std::io::Write>(&self, writer: &mut quick_xml::Writer<W>, tag: &str) -> Result<(), WriteError> {
        use quick_xml::events::{BytesStart, BytesEnd};
        
        writer.write_event(Event::Start(BytesStart::new(tag)))?;
        for item in self {
            item.parse_to_writer(writer, "li")?;
        }
        writer.write_event(Event::End(BytesEnd::new(tag)))?;
        Ok(())
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

impl<K: FromStr + Eq + std::hash::Hash + std::fmt::Display, V: XmlWrite> XmlWrite for HashMap<K, V> {
    fn parse_to_writer<W: std::io::Write>(&self, writer: &mut quick_xml::Writer<W>, tag: &str) -> Result<(), WriteError> {
        use quick_xml::events::{BytesStart, BytesEnd};
        
        writer.write_event(Event::Start(BytesStart::new(tag)))?;
        for (key, value) in self {
            let key_str = key.to_string();
            value.parse_to_writer(writer, &key_str)?;
        }
        writer.write_event(Event::End(BytesEnd::new(tag)))?;
        Ok(())
    }
}

pub fn parse<R: BufRead, T: XmlParse>(reader: &mut Reader<R>, end_tag: &[u8]) -> Result<T, ParseError> {
    T::parse_from_reader(reader, end_tag)
}

pub fn write<W: std::io::Write, T: XmlWrite>(writer: &mut quick_xml::Writer<W>, tag: &str, value: &T) -> Result<(), WriteError> {
    value.parse_to_writer(writer, tag)
}