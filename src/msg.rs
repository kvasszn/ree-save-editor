use std::{collections::HashMap, io::Cursor};
use std::sync::OnceLock;

use crate::{file::*, rsz::rszserde::Guid, file_ext::*};
use crate::file::StructRW;
use serde::{ser::SerializeStruct, Serialize};

const KEY: [u8; 16] = [
    207, 206, 251, 248, 236, 10, 51, 102, 147, 169, 29, 147, 80, 57, 95, 9,
];

#[derive(Debug, Clone, Serialize)]
pub enum MsgAttribute {
    Int(i64),
    Float(f64),
    String(String),
    Unknown(u64),
}

#[derive(Debug)]
pub struct MsgEncryptedData {
    data: Vec<u8>,
}

type Result2<T> = std::result::Result<T, Box<dyn std::error::Error>>;
impl<C> StructRW<C> for MsgEncryptedData {
    fn read<R: std::io::Read + std::io::Seek>(reader: &mut R, _ctx: &mut C) -> Result2<Self>
    where
        Self: Sized,
    {
        let mut data = vec![];
        reader.read_to_end(&mut data)?;
        let mut prev_byte = 0;
        for i in 0..data.len() {
            let cur = data[i];
            data[i] = prev_byte ^ data[i] ^ KEY[i & 0xf];
            prev_byte = cur;
        }
        Ok(Self { data: data })
    }
}


#[allow(unused)]
#[derive(Debug, file_macros::StructRW, Serialize)]
#[depends_on(
    data_offset: u64,
    data: &'a MsgEncryptedData,
    lang_count: u32,
    attr_count: u32,
    cursor: Cursor<&'a [u8]>
    )
]
pub struct MsgEntry {
    guid: Guid,
    unk: u32,
    hash: u32,
    name: MsgString,
    attributes_offset: u64,
    #[varlist(ty = MsgString, count = ctx.lang_count)]
    content: Vec<MsgString>,
    #[varlist(ty = MsgAttribute, count = ctx.attr_count, offset = attributes_offset)]
    attributes: Vec<MsgAttribute>,
}


impl<'a> StructRW<MsgEntryContext<'a>> for MsgAttribute {
    fn read<R: std::io::Read + std::io::Seek>(reader: &mut R, _ctx: &mut MsgEntryContext<'a>) -> Result2<Self>
            where
                Self: Sized {
                    let attr = u64::read(reader, &mut ())?;
                    Ok(Self::Int(attr as i64))
    }
}


#[derive(Debug, Serialize)]
pub struct MsgString(pub String);

impl<'a> StructRW<MsgEntryContext<'a>> for MsgString {
    fn read<R: std::io::Read + std::io::Seek>(reader: &mut R, ctx: &mut MsgEntryContext<'a>) -> Result2<Self>
    where
        Self: Sized {
            let offset = <u64>::read(reader, &mut ())?;
            ctx.cursor.set_position(offset - ctx.data_offset);
            let mut buf: Vec<u16> = vec![];
            loop {
                let c = <u16>::read(&mut ctx.cursor, &mut ())?;
                if c == 0 {
                    break;
                }
                buf.push(c);
            }
            let string = String::from_utf16(&buf).unwrap();
            Ok(Self(string))
        }
}


#[derive(Debug, file_macros::StructRW)]
#[allow(unused)]
pub struct Msg {
    version: u32,
    #[magic = b"GMSG"]
    magic: [u8; 4],
    header_offset: u64,
    entry_count: u32,
    attr_count: u32,
    lang_count: u32,
    null: u32,
    data_offset: u64,
    p_offset: u64,
    lang_offset: u64,
    attr_type_offset: u64,
    attr_type_name_offset: u64,
    #[varlist(ty = u64, count = entry_count)]
    entry_offsets: Vec<u64>,
    #[varlist(ty = u32, count = lang_count, offset = lang_offset)]
    languages: Vec<u32>,
    #[var(u64, p_offset)]
    p: u64,
    #[varlist(ty = i32, count = attr_count, offset = attr_type_offset)]
    attr_types: Vec<i32>,
    #[var(MsgEncryptedData, data_offset)]
    data: MsgEncryptedData,
    #[varlist(ty = MsgString, count = attr_count, offset = attr_type_name_offset)]
    #[context(MsgEntry, data_offset: data_offset, data: &data, lang_count: lang_count, attr_count: attr_count, cursor: Cursor::new(&data.data))]
    attr_names: Vec<MsgString>,
    #[varlist(ty = MsgEntry, count = entry_count, offsets = entry_offsets)]
    #[context(MsgEntry, data_offset: data_offset, data: &data, lang_count: lang_count, attr_count: attr_count, cursor: Cursor::new(&data.data))]
    entries: Vec<MsgEntry>,
}

impl DefaultDump for Msg {}

impl Serialize for Msg {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
        where
            S: serde::Serializer {
        let mut state = serializer.serialize_struct("Msg", 3)?;
        state.serialize_field("entries", &self.entries)?;
        state.end()
    }
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

