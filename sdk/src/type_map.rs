use core::fmt;
use std::{borrow::Cow, collections::HashMap, error::Error, fmt::Display, fs::File, io::{BufWriter, Cursor, Read, Seek, Write}, result::Result, time::Instant};

use bitcode;
use indexmap::IndexMap;
use serde::{Deserialize, Deserializer, Serialize, de::{MapAccess, Visitor}};

use murmur3::murmur3_32;

pub fn murmur3(data: impl AsRef<[u8]>, seed: u32) -> u32 {
    murmur3_32(&mut Cursor::new(data), seed).unwrap()
}


#[derive(Debug, Serialize, Deserialize)]
pub struct TypeMap {
    types: TypesWrapper,
    enums: EnumMap,
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

    pub fn load_from_file(rsz_path: &str, enum_path: &str) -> std::result::Result<Self, Box<dyn Error>> {
        let mut file = File::open(rsz_path)?;
        let mut data = Vec::new();
        file.read_to_end(&mut data)?;
        let types = serde_json::from_slice(&mut data)?;
        let mut file = File::open(enum_path)?;
        let mut data = Vec::new();
        file.read_to_end(&mut data)?;
        let enums = serde_json::from_slice(&mut data)?;
        Ok(TypeMap{types, enums})
    }

    pub fn from_reader<R: Read + Seek>(rsz_reader: R, enum_reader: R) -> std::result::Result<Self, Box<dyn Error>> {
        let types: TypesWrapper = simd_json::from_reader(rsz_reader)?;
        let enums: HashMap<String, HashMap<String, String>> = simd_json::from_reader(enum_reader)?;
        Ok(TypeMap{types, enums})
    }

    pub fn parse(type_data: &mut [u8], enum_data: &mut [u8]) -> std::result::Result<Self, Box<dyn Error>> {
        let types = simd_json::from_slice(type_data)?;
        let enums = simd_json::from_slice(enum_data)?;
        Ok(TypeMap{types, enums})
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
            pub struct RawTypeInfo<'a> {
                name: Cow<'a, str>,
                crc: &'a str,
                fields: Vec<FieldInfo>,
            }
            let mut type_info: RawTypeInfo = Deserialize::deserialize(deserializer)?;

            // it's probably possible to just figure this out in the deserializer?
            for field in &mut type_info.fields {
                if field.original_type == "ace.user_data.ExcelUserData.cData[]" {
                    field.original_type = type_info.name.to_string() + ".cData[]"
                }
            }
            let crc = u32::from_str_radix(&type_info.crc, 16).unwrap_or(0);
            let type_info = TypeInfo {
                name: type_info.name.to_string(),
                crc,
                fields: convert_to_map(type_info.fields)
            };
            Ok(type_info)
        }
}


#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FieldInfo {
    pub align: u16,
    pub array: bool,
    pub name: String,
    pub native: bool,
    pub original_type: String,
    pub size: u16,
    pub r#type: String,
}

impl FieldInfo {
    pub fn hash(&self) -> u32 {
        murmur3(self.name.as_str(), 0xffffffff)
    }

    pub fn get_original_type<'a>(&'a self, map: &'a TypeMap) -> Option<&'a TypeInfo> {
        let base_name = match self.r#type.find('<') {
            Some(idx) => &self.r#type[..idx],
            None => &self.r#type
        };
        map.get_by_name(base_name)
    }
}
