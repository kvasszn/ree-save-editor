use std::{collections::HashMap, io::{Cursor, Read, Seek, SeekFrom}};
use std::sync::OnceLock;

use serde::{Deserialize, Serialize};
use util::ReadExt;
use uuid::Uuid; 

use crate::sdk::Guid;
use crate::file::{StructRW, DefaultDump, Result};
use crate::sdk::type_map::ContentLanguage;

const KEY: [u8; 16] = [
    207, 206, 251, 248, 236, 10, 51, 102, 147, 169, 29, 147, 80, 57, 95, 9,
];

type Result2<T> = std::result::Result<T, Box<dyn std::error::Error>>;

pub struct MsgContext<'a, 'b> {
    pub data_offset: u64,
    pub lang_count: u32,
    pub attr_count: u32,
    pub cursor: &'a mut Cursor<&'b [u8]>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MsgAttribute {
    Int(i64),
    Float(f64),
    String(String),
    Unknown(u64),
}

impl<'a, 'b> StructRW<MsgContext<'a, 'b>> for MsgAttribute {
    fn read<R: Read + Seek>(reader: &mut R, _ctx: &mut MsgContext<'a, 'b>) -> Result2<Self> {
        let attr = reader.read_u64()?;
        Ok(Self::Int(attr as i64))
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MsgString(pub String);

impl<'a, 'b> StructRW<MsgContext<'a, 'b>> for MsgString {
    fn read<R: Read + Seek>(reader: &mut R, ctx: &mut MsgContext<'a, 'b>) -> Result2<Self> {
        let offset = reader.read_u64()?;
        ctx.cursor.set_position(offset - ctx.data_offset);
        
        let mut buf: Vec<u16> = vec![];
        loop {
            let c = ctx.cursor.read_u16()?;
            if c == 0 {
                break;
            }
            buf.push(c);
        }
        let string = String::from_utf16(&buf)?;
        Ok(Self(string))
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MsgEntry {
    pub guid: Guid,
    pub unk: u32,
    pub hash: u32,
    pub name: MsgString,
    pub attributes_offset: u64,
    pub content: Vec<MsgString>,
    pub attributes: Vec<MsgAttribute>,
}

impl<'a, 'b> StructRW<MsgContext<'a, 'b>> for MsgEntry {
    fn read<R: Read + Seek>(reader: &mut R, ctx: &mut MsgContext<'a, 'b>) -> Result2<Self> {
        let guid: Guid = Guid(reader.read_u8_arr()?);
        //let guid = <Guid>::read(reader, &mut ())?;
        let unk = reader.read_u32()?;
        let hash = reader.read_u32()?;
        let name = MsgString::read(reader, ctx)?;
        let attributes_offset = reader.read_u64()?;

        let mut content = Vec::with_capacity(ctx.lang_count as usize);
        for _ in 0..ctx.lang_count {
            content.push(MsgString::read(reader, ctx)?);
        }

        // Save position, jump to attributes, read them, and jump back
        let pos = reader.stream_position()?;
        reader.seek(SeekFrom::Start(attributes_offset))?;
        
        let mut attributes = Vec::with_capacity(ctx.attr_count as usize);
        for _ in 0..ctx.attr_count {
            attributes.push(MsgAttribute::read(reader, ctx)?);
        }
        
        reader.seek(SeekFrom::Start(pos))?;

        Ok(MsgEntry {
            guid,
            unk,
            hash,
            name,
            attributes_offset,
            content,
            attributes,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MsgEncryptedData {
    pub data: Vec<u8>,
}

impl<C> StructRW<C> for MsgEncryptedData {
    fn read<R: Read + Seek>(reader: &mut R, _ctx: &mut C) -> Result2<Self> {
        let mut data = vec![];
        reader.read_to_end(&mut data)?;
        
        let mut prev_byte = 0;
        for i in 0..data.len() {
            let cur = data[i];
            data[i] = prev_byte ^ cur ^ KEY[i & 0xf];
            prev_byte = cur;
        }
        Ok(Self { data })
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Msg {
    pub version: u32,
    pub magic: [u8; 4],
    pub header_offset: u64,
    pub entry_count: u32,
    pub attr_count: u32,
    pub lang_count: u32,
    pub null: u32,
    pub data_offset: u64,
    pub p_offset: u64,
    pub lang_offset: u64,
    pub attr_type_offset: u64,
    pub attr_type_name_offset: u64,
    pub entry_offsets: Vec<u64>,
    pub languages: Vec<u32>,
    pub p: u64,
    pub attr_types: Vec<i32>,
    pub data: MsgEncryptedData,
    pub attr_names: Vec<MsgString>,
    pub entries: Vec<MsgEntry>,
}

impl StructRW<()> for Msg {
    fn read<R: Read + Seek>(reader: &mut R, _ctx: &mut ()) -> Result2<Self> {
        let version = reader.read_u32()?;
        let mut magic = [0u8; 4];
        reader.read_exact(&mut magic)?;
        if &magic != b"GMSG" {
            return Err("Invalid magic, expected GMSG".into());
        }

        let header_offset = reader.read_u64()?;
        let entry_count = reader.read_u32()?;
        let attr_count = reader.read_u32()?;
        let lang_count = reader.read_u32()?;
        let null = reader.read_u32()?;
        let data_offset = reader.read_u64()?;
        let p_offset = reader.read_u64()?;
        let lang_offset = reader.read_u64()?;
        let attr_type_offset = reader.read_u64()?;
        let attr_type_name_offset = reader.read_u64()?;

        // 1. Read flat arrays immediately
        let mut entry_offsets = Vec::with_capacity(entry_count as usize);
        for _ in 0..entry_count {
            entry_offsets.push(reader.read_u64()?);
        }

        reader.seek(SeekFrom::Start(lang_offset))?;
        let mut languages = Vec::with_capacity(lang_count as usize);
        for _ in 0..lang_count {
            languages.push(reader.read_u32()?);
        }

        reader.seek(SeekFrom::Start(p_offset))?;
        let p = reader.read_u64()?;

        reader.seek(SeekFrom::Start(attr_type_offset))?;
        let mut attr_types = Vec::with_capacity(attr_count as usize);
        for _ in 0..attr_count {
            attr_types.push(reader.read_i32()?);
        }

        // 2. Read the entire encrypted block into memory
        reader.seek(SeekFrom::Start(data_offset))?;
        let data = MsgEncryptedData::read(reader, &mut ())?;

        // 3. Set up the context containing the unencrypted memory buffer
        let mut cursor = Cursor::new(data.data.as_slice());
        let mut ctx = MsgContext {
            data_offset,
            lang_count,
            attr_count,
            cursor: &mut cursor,
        };

        // 4. Map the complex attributes
        reader.seek(SeekFrom::Start(attr_type_name_offset))?;
        let mut attr_names = Vec::with_capacity(attr_count as usize);
        for _ in 0..attr_count {
            attr_names.push(MsgString::read(reader, &mut ctx)?);
        }

        // 5. Map the main entries
        let mut entries = Vec::with_capacity(entry_count as usize);
        for offset in &entry_offsets {
            reader.seek(SeekFrom::Start(*offset))?;
            entries.push(MsgEntry::read(reader, &mut ctx)?);
        }

        Ok(Msg {
            version, magic, header_offset, entry_count, attr_count, lang_count, null,
            data_offset, p_offset, lang_offset, attr_type_offset, attr_type_name_offset,
            entry_offsets, languages, p, attr_types, data, attr_names, entries,
        })
    }
}

impl Msg {
    pub fn get_entry<'a>(&'a self, guid: &'a Guid, language: ContentLanguage) -> Option<&'a str> {
        if let Some(entry) = self.entries.iter().find(|e| e.guid.0 == guid.0) {
            return entry.content.get(language as usize).map(|x| x.0.as_str())
        }
        println!("Could not find entry for guid {}, {language:?}", guid.to_string());
        None
    }
}

impl DefaultDump for Msg {}
