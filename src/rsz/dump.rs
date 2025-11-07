use std::sync::OnceLock;
use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::reerr::{Result, RszError::*};

pub static RSZ_FILE: OnceLock<String> = OnceLock::new();
pub static ENUM_FILE: OnceLock<String> = OnceLock::new();

#[derive(Debug, Clone)]
pub struct RszStruct<T> {
    pub name: String,
    pub crc: u32,
    pub hash: u32,
    pub fields: Vec<T>,
}

impl<T> RszStruct<T> {
    pub fn hash(&self) -> Option<&u32> {
        RszDump::name_map().get(&self.name)
    }
}


#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RszField {
    pub align: u32,
    pub array: bool,
    pub name: String,
    pub native: bool,
    pub original_type: String,
    pub size: u32,
    pub r#type: String,
}



pub struct RszMap<T>(pub T);

pub type RszMapType = HashMap<String, RszStruct<RszField>>;
pub type RszNameMapType = HashMap<String, u32>;

impl<'de> Deserialize<'de> for RszStruct<RszField> {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de> {
            #[derive(Debug, Clone, Deserialize)]
            pub struct RszStructTemp<T> {
                name: String,
                crc: String,
                fields: Vec<T>,
            }
            let mut rsz_struct: RszStructTemp<RszField> = Deserialize::deserialize(deserializer)?;

            for field in &mut rsz_struct.fields {
                if field.original_type == "ace.user_data.ExcelUserData.cData[]" {
                    field.original_type = rsz_struct.name.clone() + ".cData[]"
                }
            }
            let rsz_struct: RszStruct<RszField> = RszStruct {
                name: rsz_struct.name,
                hash: 0,
                crc: u32::from_str_radix(&rsz_struct.crc, 16).unwrap(),
                fields: rsz_struct.fields
            };
            Ok(rsz_struct)
        }
}

impl RszMap<RszMapType> {
    pub fn get(&self, hash: &u32) -> Option<&RszStruct<RszField>> {
        let d = format!("{:x}", *hash);
        let x = self.0.get(&d);
        x
    }
}

impl RszMap<RszNameMapType> {
    pub fn get(&self, hash: &String) -> Option<&u32> {
        let x = self.0.get(&hash.to_string());
        x
    }
}

pub struct RszDump;


impl RszDump {
    pub fn get_struct<'a>(hash: u32) -> Result<&'a RszStruct<RszField>> {
        match RszDump::rsz_map().get(&hash) {
            Some(struct_desc) => Ok(struct_desc),
            None => Err(InvalidRszTypeHash(hash).into())
        }
    }

    pub fn rsz_map() -> &'static RszMap<RszMapType> {
        static HASHMAP: OnceLock<RszMap<RszMapType>> = OnceLock::new();
        HASHMAP.get_or_init(|| {
            RSZ_FILE.get_or_init(|| {
                "rszmhwilds.json".to_string()
            });
            let file = std::fs::read_to_string(RSZ_FILE.get().unwrap()).unwrap();
            let m: RszMapType = serde_json::from_str(&file).unwrap();
            RszMap(m)
        })
    }

    pub fn name_map() -> &'static RszMap<RszNameMapType> {
        static HASHMAP: OnceLock<RszMap<RszNameMapType>> = OnceLock::new();
        HASHMAP.get_or_init(|| {
            let temp = &Self::rsz_map().0;
            let mut m = HashMap::new();
            for (_key, rsz_struct) in temp {
                let hash = u32::from_str_radix(&_key, 16).unwrap();
                m.insert(rsz_struct.name.clone(), hash);
            }
            RszMap(m)
        })
    }
}

pub fn get_enum_val(name: &str, enum_str_name: &str) -> Option<i128> {
    let name_tmp = name.replace("[]", "").replace("_Serializable", "_Fixed");
    if let Some(map) = enum_map().get(&name_tmp) {
        if let Some(value) = map.get(enum_str_name){
            if let Ok(value) = value.parse::<i128>() {
                return Some(value)
            }
        }
    }
    let name_tmp = name_tmp.replace("_Fixed", "");
    if let Some(map) = enum_map().get(&name_tmp) {
        if !name_tmp.ends_with("Bit") {
            if let Some(value) = map.get(enum_str_name) {
                if let Ok(value) = value.parse::<i128>() {
                    return Some(value)
                }
            }
        }
    }

    let name = name.replace("_Serializable", "");
    if let Some(map) = enum_map().get(&name) {
        let enum_names: Vec<&str> = enum_str_name.split('|').collect();
        let mut enum_val = 0;
        for e in &enum_names {
            if let Some(value) = map.get(*e) {
                if let Ok(value) = value.parse::<i128>() {
                    enum_val += value;
                } else { // just dip if it doesnt work
                    break
                }
            }
        }
        return Some(enum_val)
    }
    None
}


type EnumMap = HashMap<String, HashMap<String, String>>;

pub fn enum_map() -> &'static EnumMap {
    static HASHMAP: OnceLock<EnumMap> = OnceLock::new();
    HASHMAP.get_or_init(|| {
        ENUM_FILE.get_or_init(|| {
            "enums.json".to_string()
        });
        let json_data = std::fs::read_to_string(ENUM_FILE.get().unwrap()).unwrap();
        let hashmap: EnumMap = serde_json::from_str(&json_data).unwrap();
        hashmap
    })
}

pub fn get_enum_name(name: &str, value: &str) -> Option<String> {
    let name_tmp = name.replace("[]", "").replace("_Serializable", "_Fixed");
    if let Some(map) = enum_map().get(&name_tmp) {
        if let Some(value) = map.get(value){
            return Some(value.to_string())
        }
    }
    let name_tmp = name_tmp.replace("_Fixed", "");
    if let Some(map) = enum_map().get(&name_tmp) {
        if !name_tmp.ends_with("Bit") {
            if let Some(value) = map.get(value){
                return Some(value.to_string())
            }
        }
    }

    let enum_val: u64 = value.parse().unwrap_or(0);
    let mut flag_enum_names = String::from("");
    let name = name.replace("_Serializable", "");
    if let Some(map) = enum_map().get(&name) {
        for i in 0..64 {
            let mask = 1 << i;
            let bit_val = enum_val & mask;
            if let Some(value) = map.get(&bit_val.to_string()){
                if !flag_enum_names.contains(value) {
                    if flag_enum_names != "" {
                        flag_enum_names += "|";
                    }
                    flag_enum_names += value;
                }
            }
        }
        if flag_enum_names != "" {
            return Some(flag_enum_names.to_string())
        }
    }
    None
}

pub fn get_enum_list(name: &str) -> Option<&HashMap<String, String>> {
    let name_tmp = name.replace("[]", "").replace("_Serializable", "_Fixed");
    if let Some(map) = enum_map().get(&name_tmp) {
        return Some(map);
    }
    let name_tmp = name_tmp.replace("_Fixed", "");
    if let Some(map) = enum_map().get(&name_tmp) {
        return Some(map);
    }

    let name = name.replace("_Serializable", "");
    if let Some(map) = enum_map().get(&name) {
        return Some(map);
    }
    None
}
