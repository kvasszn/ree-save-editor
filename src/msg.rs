use core::str;
use std::{collections::HashMap, io::{Error, ErrorKind, Result, Write}, sync::OnceLock};

use indexmap::IndexMap;
use serde::{ser::SerializeSeq, Serialize};
use serde_json::json;
use uuid::Uuid;

use crate::{byte_reader::BytesFile, dersz::get_enum_name};

const KEY: [u8; 16] = [207, 206, 251, 248, 236, 10, 51, 102, 147, 169, 29, 147, 80, 57, 95, 9];

#[derive(Debug)]
#[allow(unused)]
struct Entry {
    unkn: u32,
    guid: [u8; 16],
    hash: u32,
    name: String,
    attributes: Vec<MsgAttribute>,
    content: Vec<String>,
}

#[derive(Debug)]
pub struct MsgAttributeHeader {
    pub ty: i32,
    pub name: String,
}

impl Serialize for MsgAttributeHeader {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer {
            if &self.name != "" {
                serializer.serialize_str(&self.name)
            } else {
                serializer.serialize_none()
            }
    }
}


#[derive(Debug, Clone)]
pub enum MsgAttribute {
    Int(i64),
    Float(f64),
    String(String),
    Unknown(u64),
}

impl Serialize for MsgAttribute {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
        where
            S: serde::Serializer {
                match self {
                    Self::Int(v) => serializer.serialize_i64(*v),
                    Self::Float(v) => serializer.serialize_f64(*v),
                    Self::String(v) => serializer.serialize_str(v),
                    Self::Unknown(v) => serializer.serialize_u64(*v),
                }
    }
}

#[derive(Debug, Default)]
pub struct Msg {
    attribute_headers: Vec<MsgAttributeHeader>,
    entries: Vec<Entry>,
}

#[derive(Default)]
#[allow(unused)]
pub struct MsgHeader {
    version: u32,
    magic: [u8; 4],
    header_offset: u64,
    entry_count: u32,
    type_count: u32,
    lang_count: u32,
    data1_offset: u64,
    p_offset: u64,
    lang_offset: u64,
    type_offset: u64,
    type_name_offset: u64
}

impl Msg {
    pub fn new(file_name: String) -> Result<Msg> {
        let mut file = BytesFile::new(file_name)?;
        let _version = file.read::<u32>()?;
        let magic = file.readn::<u8, 4>()?;
        let magic = str::from_utf8(&magic).unwrap();
        if magic != "GMSG" {
            return Err(Error::new(ErrorKind::Other, format!("Invalid Magic {magic}, {_version}")))
        }

        let _header_offset = file.read::<u64>()?;
        let entry_count = file.read::<u32>()?;
        let attr_count = file.read::<u32>()?;
        let lang_count = file.read::<u32>()?;
        file.read::<u32>()?; // null
        let data_offset = file.read::<u64>()?;
        let p_offset = file.read::<u64>()?;
        let lang_offset = file.read::<u64>()?;
        let attr_type_offset = file.read::<u64>()?;
        let attr_type_name_offset = file.read::<u64>()?;
        let base_entry_offset = file.index;
        //println!("{entry_count}, {attr_count}, {lang_count}");

        // Read Data
        file.index = data_offset as usize;
        let mut data = file.read_bytes_to_vec(file.len() - data_offset as usize)?;
        let mut b = 0;
        let mut num = 0;
        let mut num2 = 0;
        while num < data.len() {
            let b2 = b;
            b = data[num2];
            let num3 = num & 0xf;
            num += 1;
            data[num2] = b2 ^ b ^ KEY[num3];
            num2 = num;
        }

        let mut data = BytesFile {
            data,
            index: 0,
        };
        
        // PUT A CHECK HERE FOR IF ITS A VALID FILE OR NOT

        file.index = lang_offset as usize;
        let _languages = (0..lang_count).map(|_| file.read::<u32>()).collect::<Result<Vec<_>>>()?;

        file.index = p_offset as usize;
        let _p = file.read::<u64>()?; // idk what this does

        file.index = attr_type_offset as usize;
        let attr_types = (0..attr_count).map(
            |_| file.read::<i32>()
        ).collect::<Result<Vec<i32>>>()?;

        file.index = attr_type_name_offset as usize;
        let attr_names = (0..attr_count)
            .map(|_| file.read::<u64>())
            .collect::<Result<Vec<u64>>>()?;

        //println!("{:?}", attr_types);
        //println!("{:?}", attr_names);

        let mut entries: Vec<Entry> = Vec::new();
        for i in 0..entry_count as usize {

            file.index = base_entry_offset + i * 8;
            let entry_offset = file.read::<u64>()?;
            file.index = entry_offset as usize;

            let guid: [u8; 16] = file.readn()?;

            let _unkn = file.read::<u32>()?;
            let hash = file.read::<u32>()?;
            let name = file.read::<u64>()?;
            let attributes_offset = file.read::<u64>()?;

            let name = data.read_utf16(name as usize - data_offset as usize)?;

            let content = (0..lang_count).map(|_| {
                let offset = file.read::<u64>()? as usize;
                Ok(data.read_utf16(offset - data_offset as usize).unwrap_or("".to_string()))
            }).collect::<Result<Vec<_>>>()?;

            file.index = attributes_offset as usize;
            let attributes = (0..attr_count).into_iter().zip(&attr_types)
                .map(|(_, &attr_type)| {
                    let attr = file.read::<u64>()?;
                    match attr_type {
                        0 => Ok(MsgAttribute::Int(attr as i64)),
                        1 => Ok(MsgAttribute::Float(f64::from_bits(attr))),
                        2 => {
                            let x = data.read_utf16((attr -data_offset) as usize)?;
                            Ok(MsgAttribute::String(x))
                        },
                        -1 => Ok(MsgAttribute::Unknown(attr)),
                        _ => Err(Error::new(ErrorKind::Other, "Unknown attribute type")),
                    }
                }).collect::<Result<Vec<_>>>()?;
            entries.push(Entry { name, guid, unkn: _unkn, hash, attributes, content });
        }

        //println!("{:#?}", entries);

        let attribute_headers = attr_types
            .iter()
            .zip(&attr_names)
            .map(|(&ty, name)| {
                let name = data.read_utf16((name - data_offset) as usize)?;
                Ok(MsgAttributeHeader{ty, name})
            })
        .collect::<Result<Vec<_>>>()?;

        Ok(Msg {
            entries,
            attribute_headers,
        })
    }

    pub fn lang_map() -> &'static HashMap<String, String> {
        static HASHMAP: OnceLock<HashMap<String, String>> = OnceLock::new();
        HASHMAP.get_or_init(|| {
            let json_data = r#"{
                    "0": "Japanese",
                    "1": "English",
                    "2": "French",
                    "3": "Italian",
                    "4": "German",
                    "5": "Spanish",
                    "6": "Russian",
                    "7": "Polish",
                    "8": "Dutch",
                    "9": "Portuguese",
                    "10": "PortugueseBr",
                    "11": "Korean",
                    "12": "TransitionalChinese",
                    "13": "SimplelifiedChinese",
                    "14": "Finnish",
                    "15": "Swedish",
                    "16": "Danish",
                    "17": "Norwegian",
                    "18": "Czech",
                    "19": "Hungarian",
                    "20": "Slovak",
                    "21": "Arabic",
                    "22": "Turkish",
                    "23": "Bulgarian",
                    "24": "Greek",
                    "25": "Romanian",
                    "26": "Thai",
                    "27": "Ukrainian",
                    "28": "Vietnamese",
                    "29": "Indonesian",
                    "30": "Fiction",
                    "31": "Hindi",
                    "32": "LatinAmericanSpanish",
                    "33": "Unknown"
            }"#;
            let hashmap: HashMap<String, String> = serde_json::from_str(&json_data).unwrap();
            hashmap
        })
    }
    pub fn save(&self, writer: &mut dyn Write) {
        #[derive(Debug, Serialize)]
        struct EntryInfo<'a> {
            name: &'a str,
            hash: u32,
            attributes: &'a Vec<MsgAttribute>,
            content: IndexMap<&'a str, String>,
        }
        let name_to_uuid_map: IndexMap<_, _> = self.entries.iter()
            .map(|entry| {
                let uuid = Uuid::from_bytes_le(entry.guid).to_string();
                (&entry.name, uuid)
            }).collect();
        let msgs: IndexMap<_, EntryInfo> = self.entries.iter()
            .map(|entry| {
                let uuid = Uuid::from_bytes_le(entry.guid).to_string();
                //println!("{:?}", entry.content);
                let content = entry.content.iter().enumerate()
                        .map(|(i, c)| {
                            let enum_name = Msg::lang_map().get(&i.to_string()).unwrap();
                            //println!("{}, {}", &enum_name, &c);
                            //c.to_string()
                            (enum_name.as_str(), c.clone())
                        })
                        .collect();
                ( uuid, EntryInfo {
                    name: &entry.name,
                    hash: entry.hash,
                    attributes: &entry.attributes,
                    content
                })
            }).collect();
        //println!("{:#?}", msgs.get_index(66));
        #[derive(Serialize)]
        struct Data<'a> {
            msgs: IndexMap<String, EntryInfo<'a>>,
            attributes: &'a Vec<MsgAttributeHeader>,
            name_to_uuid: IndexMap<&'a String, String>,
        }
        serde_json::to_writer_pretty(writer, 
            &Data {
                msgs,
                attributes: &self.attribute_headers,
                name_to_uuid: name_to_uuid_map,
            }
        ).unwrap();
        //serde_json::to_writer_pretty(writer, &json_map).unwrap();
    }
}
