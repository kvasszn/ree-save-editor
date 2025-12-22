use core::fmt;
use std::{borrow::Cow, collections::{HashMap, HashSet, VecDeque}, error::Error, fmt::Display, fs::File, io::{BufWriter, Cursor, Read, Seek, Write}, result::Result};

use bitcode;
use indexmap::IndexMap;
use serde::{Deserialize, Deserializer, Serialize, de::{MapAccess, Visitor}};

use murmur3::murmur3_32;

pub fn murmur3(data: impl AsRef<[u8]>, seed: u32) -> u32 {
    murmur3_32(&mut Cursor::new(data), seed).unwrap()
}


#[derive(Debug, Serialize, Deserialize)]
pub struct TypeMap {
    pub types: TypesWrapper,
    pub enums: EnumMap,
    pub msgs: MsgCombined,
    // enum_type -> (enum_str -> guid)
    pub enum_mappings: HashMap<String, HashMap<String, String>>
}

impl TypeMap {
    // Takes in something like "FooClass, _Foo._Bar[0]._Baz"
    // Should implement this for loaded values
    pub fn get_field_from_str<'a>(&'a self, start_type: &'a str,  path: &'a str) -> Option<&'a FieldInfo> {
        let start_type = self.get_by_name(start_type)?;
        self.get_field_from_type(start_type, path)
    }

    pub fn get_field_from_type<'a>(&'a self, start_type: &'a TypeInfo,  path: &'a str) -> Option<&'a FieldInfo> {
        let mut current_type = Some(start_type);
        let mut cur_field = None;
        for part in path.split('.') {
            let part = part.find(']').map(|x| &part[..x]).unwrap_or(part);
            cur_field = current_type.and_then(|x| x.get_by_name(part));
            current_type = cur_field.and_then(|f| self.get_by_name(&f.original_type));
        }
        cur_field
    }

    pub fn search(&self, start_type: &TypeInfo, search_term: &str, max_depth: usize) -> HashSet<(u32, u32)> {
        let search_lower = search_term.to_lowercase();

        let mut fields_to_expand: HashSet<(u32, u32)> = HashSet::new();

        let mut q = VecDeque::new();
        q.push_back((start_type, Vec::<(u32, u32)>::new()));

        while let Some((curr_type, path)) = q.pop_front() {
            if path.len() >= max_depth {
                continue;
            }

            for (field_hash, field) in &curr_type.fields {
                if field.name.to_lowercase().contains(&search_lower) {
                    fields_to_expand.insert((field.type_hash, *field_hash));
                    for ancestor in &path {
                        fields_to_expand.insert(ancestor.clone());
                    }
                }

                if let Some(next_type) = field.get_original_type(self) {
                    if next_type.name.to_lowercase().contains(&search_lower) {
                        fields_to_expand.insert((field.type_hash, *field_hash));
                        for ancestor in &path {
                            fields_to_expand.insert(ancestor.clone());
                        }
                    }

                    let mut new_path = path.clone();
                    new_path.push((field.type_hash, *field_hash));
                    q.push_back((next_type, new_path));
                }
            }
        }

        fields_to_expand
    }

    // TODO: add something that returns a HashSet of leaf nodes as well, this could be in the first
    // res ig since its kinda useless rn lol
    // then add a found_search in the EditContext to know to exit search_mode after that point
    pub fn searchv2(&self, start_type: &TypeInfo, search_term: &str, max_depth: usize) -> (HashSet<String>, HashSet<(String, String)>) {
        let search_lower = search_term.to_lowercase();

        let mut matching_types = HashSet::new();   // Types that matched directly
        let mut fields_to_expand: HashSet<(String, String)> = HashSet::new(); // Fields that are part of a matching chain

        let mut q = VecDeque::new();
        q.push_back((start_type, Vec::<(String, String)>::new()));

        while let Some((curr_type, path)) = q.pop_front() {
            if path.len() >= max_depth {
                continue;
            }

            for (_, field) in &curr_type.fields {
                if field.name.to_lowercase().contains(&search_lower) {
                    fields_to_expand.insert((field.original_type.clone(), field.name.clone()));
                    for ancestor in &path {
                        fields_to_expand.insert(ancestor.clone());
                    }
                }

                if let Some(next_type) = field.get_original_type(self) {
                    if next_type.name.to_lowercase().contains(&search_lower) {
                        matching_types.insert(next_type.name.clone());
                        fields_to_expand.insert((field.original_type.clone(), field.name.clone()));
                        for ancestor in &path {
                            fields_to_expand.insert(ancestor.clone());
                        }
                    }

                    let mut new_path = path.clone();
                    new_path.push((field.original_type.clone(), field.name.clone())); // Add this field to history
                    q.push_back((next_type, new_path));
                }
            }
        }

        (matching_types, fields_to_expand)
    }

    pub fn load_with_msgs(rsz_path: &str, enum_path: &str, msg_path: &str, mappings_path: &str) -> std::result::Result<Self, Box<dyn Error>> {
        let mut type_map =  TypeMap::load_from_file(rsz_path, enum_path)?;
        let mut file = File::open(msg_path)?;
        let mut data = Vec::new();
        file.read_to_end(&mut data)?;
        type_map.msgs = serde_json::from_slice(&mut data)?;
        let mut file = File::open(mappings_path)?;
        let mut data = Vec::new();
        file.read_to_end(&mut data)?;
        type_map.enum_mappings = serde_json::from_slice(&mut data)?;
        Ok(type_map)
    }

    pub fn load_from_file(rsz_path: &str, enum_path: &str) -> std::result::Result<Self, Box<dyn Error>> {
        let mut file = File::open(rsz_path)?;
        let mut data = Vec::new();
        file.read_to_end(&mut data)?;
        let types = serde_json::from_slice(&mut data)?;
        let mut file = File::open(enum_path)?;
        let mut data = Vec::new();
        file.read_to_end(&mut data)?;
        let enums = serde_json::from_slice(&mut data)?;
        Ok(TypeMap{types, enums, msgs: MsgCombined::default(), enum_mappings: HashMap::default()})
    }

    pub fn from_reader<R: Read + Seek>(rsz_reader: R, enum_reader: R) -> std::result::Result<Self, Box<dyn Error>> {
        let types: TypesWrapper = simd_json::from_reader(rsz_reader)?;
        let enums: HashMap<String, HashMap<String, String>> = simd_json::from_reader(enum_reader)?;
        Ok(TypeMap{types, enums, msgs: MsgCombined::default(), enum_mappings: HashMap::new()})
    }

    pub fn parse(type_data: &mut [u8], enum_data: &mut [u8]) -> std::result::Result<Self, Box<dyn Error>> {
        let types = simd_json::from_slice(type_data)?;
        let enums = simd_json::from_slice(enum_data)?;
        Ok(TypeMap{types, enums, msgs: MsgCombined::default(), enum_mappings: HashMap::new()})
    }

    pub fn parse_str(type_data: &str, enum_data: &str) -> std::result::Result<Self, Box<dyn Error>> {
        let types = serde_json::from_str(type_data)?;
        let enums = serde_json::from_str(enum_data)?;
        Ok(TypeMap{types, enums, msgs: MsgCombined::default(), enum_mappings: HashMap::new()})
    }
    pub fn parse_str_compressed(type_data: &str, enum_data: &str) -> std::result::Result<Self, Box<dyn Error>> {
        let types = serde_json::from_str(type_data)?;
        let enums = serde_json::from_str(enum_data)?;
        Ok(TypeMap{types, enums, msgs: MsgCombined::default(), enum_mappings: HashMap::new()})
    }

    pub fn load_msg(mut self, msg_data: &str, enum_mappings: &str) -> Self {
        self.msgs = serde_json::from_str(msg_data).unwrap_or_default();
        self.enum_mappings = serde_json::from_str(enum_mappings).unwrap_or_default();
        self
    }

    pub fn to_bincode(&self, file_path: &str) -> std::result::Result<(), Box<dyn Error>> {
        let data = bitcode::serialize(self)?;// ::serde::encode_to_vec(self, bincode::config::standard().with_variable_int_encoding())?;
        let file = File::create(file_path)?;
        let mut writer = BufWriter::new(file);
        //let mut encoder = lz4_flex::frame::FrameEncoder::new(writer);
        //let mut encoder = zstd::stream::write::Encoder::new(writer, 21)?;
        //encoder.write_all(&data)?;
        //encoder.finish()?;
        writer.write_all(&data)?;
        Ok(())
    }
    pub fn parse_bincode(data: &[u8]) -> std::result::Result<Self, Box<dyn Error>> {
        //let mut decoder = FrameDecoder::new(data);
        //let mut decoder = zstd::stream::read::Decoder::new(data)?;
        //let mut decompressed = Vec::with_capacity(data.len() * 4);
        //decoder.read_to_end(&mut decompressed)?;
        let type_map: TypeMap = bitcode::deserialize(&data)?;
        Ok(type_map)
    }

    pub fn get_by_hash(&self, hash: u32) -> Option<&TypeInfo> {
        self.types.0.get(&hash)
    }

    pub fn get_by_name(&self, name: &str) -> Option<&TypeInfo> {
        let hash = murmur3(name, 0xffffffff);
        self.types.0.get(&hash)
    }

    pub fn get_hash(original_type: &str) -> u32 {
        let hash = murmur3(original_type, 0xffffffff);
        hash
    }

    // get the corresponding string value for an enum
    // TODO
    // the T: Display is kinda a hack for now since my enums.json format is dumb
    pub fn get_enum_str<T: Display>(&self, n: T, original_type: &str) -> Option<&String> {
        self.enums.get(original_type).and_then(|x| {
            x.get(n.to_string().as_str())
        }) 
    }

    /*pub fn does_type_contain_string(&self, hash: u32, query: &str) -> bool {
      let mut current_type = self.get_by_hash(hash);
      let mut cur_field = None;
      for part in path.split('.') {
      let part = part.find(']').map(|x| &part[..x]).unwrap_or(part);
      cur_field = current_type.and_then(|x| x.get_by_name(part));
      current_type = cur_field.and_then(|f| self.get_by_name(&f.original_type));
      }
      cur_field
      }*/

    pub fn get_enum_text(&self, enum_str: &str, original_type: &str, language: ContentLanguage) -> Option<String> {
        let enum_guid_map = self.enum_mappings.get(original_type);
        let guid = enum_guid_map.and_then(|x| {
            x.get(enum_str)
        });
        let res = guid.and_then(|guid| {
            self.msgs.get_content(guid, language)
        });
        //println!("{enum_str:?} {original_type:?} {guid:?} {res:?}");
        res
    }
}

#[derive(Debug, Serialize)]
pub struct TypesWrapper(pub HashMap<u32, TypeInfo>);

impl<'de> Deserialize<'de> for TypesWrapper {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        if !deserializer.is_human_readable() {
            let types: HashMap<u32, TypeInfo> = Deserialize::deserialize(deserializer)?;
            return Ok( TypesWrapper(types) );
        }
        struct HexKeyMapVisitor;
        impl<'de> Visitor<'de> for HexKeyMapVisitor {
            type Value = TypesWrapper;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a map with hex-string keys")
            }

            fn visit_map<M>(self, mut access: M) -> Result<Self::Value, M::Error>
            where
                M: MapAccess<'de>,
            {
                let mut types = HashMap::with_capacity(access.size_hint().unwrap_or(0));
                while let Some(key_str) = access.next_key::<Cow<str>>()? {
                    let key_u32 = if key_str.starts_with("0x") {
                        u32::from_str_radix(&key_str[2..], 16)
                    } else {
                        u32::from_str_radix(&key_str, 16)
                    }
                    .map_err(|_| serde::de::Error::custom(format!("Invalid hex key: {}", key_str)))?;

                    let value: TypeInfo = access.next_value()?;
                    types.insert(key_u32, value);
                }
                Ok(TypesWrapper(types))
            }
        }
        deserializer.deserialize_map(HexKeyMapVisitor)
    }
}


pub type EnumMap = HashMap<String, HashMap<String, String>>;

// having functions for getting things like generics could be really useful
#[derive(Debug, Serialize)]
pub struct TypeInfo {
    pub crc: u32,
    pub name: String,
    pub fields: IndexMap<u32, FieldInfo>
}

impl TypeInfo {
    pub fn get_by_hash(&self, hash: u32) -> Option<&FieldInfo> {
        self.fields.get(&hash)
    }

    pub fn get_by_name(&self, name: &str) -> Option<&FieldInfo> {
        let hash = murmur3(name, 0xffffffff);
        self.fields.get(&hash)
    }

    pub fn get_by_index(&self, index: usize) -> Option<&FieldInfo> {
        self.fields.get_index(index).map(|(_hash, field)| field)
    }

    pub fn get_hash_at_index(&self, index: usize) -> Option<u32> {
        self.fields.get_index(index).map(|(hash, _field)| *hash)
    }

    pub fn get_generic_args(&self) -> Vec<&str> {
        if let Some(start) = self.name.find('<') {
            if let Some(end) = self.name.rfind('>') {
                let content = &self.name[start + 1..end];
                return content.split(',').map(|s| s.trim()).collect();
            }
        }
        Vec::new()
    }

    pub fn get_base_type_name(&self) -> &str {
        if let Some(idx) = self.name.find('<') {
            &self.name[..idx]
        } else {
            &self.name
        }
    }
}

fn convert_to_map(fields: Vec<FieldInfo>) -> IndexMap<u32, FieldInfo> {
    fields
        .into_iter()
        .map(|field| {
            let hash = murmur3(field.name.as_str(), 0xffffffff);
            (hash, field)
        })
    .collect()
}

impl<'de> Deserialize<'de> for TypeInfo {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de> {

            if !deserializer.is_human_readable() {
                #[derive(Deserialize)]
                struct TypeInfoBin {
                    crc: u32,
                    name: String,
                    fields: IndexMap<u32, FieldInfo>
                }
                let bin = TypeInfoBin::deserialize(deserializer)?;
                return Ok(TypeInfo {
                    crc: bin.crc,
                    name: bin.name,
                    fields: bin.fields,
                });
            }

            #[derive(Debug, Deserialize)]
            struct RawFieldInfo {
                align: u16,
                array: bool,
                name: String,
                native: bool,
                original_type: String,
                size: u16,
                r#type: String,
            }

            #[derive(Debug, Deserialize)]
            pub struct RawTypeInfo<'a> {
                name: Cow<'a, str>,
                crc: &'a str,
                fields: Vec<RawFieldInfo>,
            }
            let type_info: RawTypeInfo = Deserialize::deserialize(deserializer)?;
            let mut fields_map = IndexMap::with_capacity(type_info.fields.len());
            let type_name_string = type_info.name.to_string();

            for raw_field in type_info.fields {
                let og_type = if raw_field.original_type == "ace.user_data.ExcelUserData.cData[]" {
                    type_name_string.clone() + ".cData[]"
                } else {raw_field.original_type.clone()};
                let name_hash = murmur3(raw_field.name.as_bytes(), 0xffffffff);
                let type_hash = murmur3(raw_field.original_type.as_bytes(), 0xffffffff);

                let field_info = FieldInfo {
                    align: raw_field.align,
                    array: raw_field.array,
                    name: raw_field.name,
                    hash: name_hash,
                    native: raw_field.native,
                    original_type: og_type,
                    size: raw_field.size,
                    r#type: raw_field.r#type,
                    type_hash,
                };

                fields_map.insert(name_hash, field_info);
            }
            let crc = u32::from_str_radix(&type_info.crc, 16).unwrap_or(0);
            let type_info = TypeInfo {
                name: type_info.name.to_string(),
                crc,
                fields: fields_map,
            };
            Ok(type_info)
        }
}


#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FieldInfo {
    pub align: u16,
    pub array: bool,
    pub name: String,
    pub hash: u32,
    pub native: bool,
    pub original_type: String,
    pub size: u16,
    pub r#type: String,
    pub type_hash: u32,
}

impl FieldInfo {
    pub fn hash(&self) -> u32 {
        self.hash
    }

    pub fn get_original_type<'a>(&'a self, map: &'a TypeMap) -> Option<&'a TypeInfo> {
        /*let base_name = match self.original_type.find('<') {
          Some(idx) => &self.original_type[..idx],
          None => &self.original_type
          };*/
        map.get_by_hash(self.type_hash)
            //map.get_by_name(&self.original_type)
    }
}

#[repr(usize)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContentLanguage {
    Japanese = 0,
    English = 1,
    French = 2,
    Italian = 3,
    German = 4,
    Spanish = 5,
    Russian = 6,
    Polish = 7,
    Dutch = 8,
    Portuguese = 9,
    PortugueseBr = 10,
    Korean = 11,
    TransitionalChinese = 12,
    SimplelifiedChinese = 13,
    Finnish = 14,
    Swedish = 15,
    Danish = 16,
    Norwegian = 17,
    Czech = 18,
    Hungarian = 19,
    Slovak = 20,
    Arabic = 21,
    Turkish = 22,
    Bulgarian = 23,
    Greek = 24,
    Romanian = 25,
    Thai = 26,
    Ukrainian = 27,
    Vietnamese = 28,
    Indonesian = 29,
    Fiction = 30,
    Hindi = 31,
    LatinAmericanSpanish = 32,
    Unknown = 33,
}


#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct MsgCombined {
    msgs: HashMap<String, MsgEntry>,
    name_to_guid: HashMap<String, String>
}

impl MsgCombined {
    pub fn get_content(&self, guid: &str, language: ContentLanguage) -> Option<String> {
        self.msgs.get(guid).map(|e| e.content[language as usize].clone())
    }
    pub fn get_from_enum(&self, name: &str, language: ContentLanguage) -> Option<String> {
        None
    }
}

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
struct MsgEntry {
    guid: String,
    name: String,
    content: Vec<String>
}
