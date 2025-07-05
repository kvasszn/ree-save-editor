use core::str;
use std::{
    collections::{HashMap, HashSet}, i128, sync::OnceLock
};

use crate::gensdk::SdkComponent;

pub static RSZ_FILE: OnceLock<String> = OnceLock::new();
pub static ENUM_FILE: OnceLock<String> = OnceLock::new();

use proc_macro2::{Literal, TokenStream};
use quote::{format_ident, quote, TokenStreamExt};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub struct RszStruct<T> {
    pub name: String,
    pub crc: u32,
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

impl RszField {
    pub fn get_type(&self) -> Option<&RszStruct<RszField>> {
        match RszDump::name_map().get(&self.original_type) {
            Some(x) => RszDump::rsz_map().get(x),
            None => None
        }
    }
    pub fn get_type_hash(&self) -> Option<&u32> {
        RszDump::name_map().get(&self.original_type)
    }
}

pub fn convert_type_to_prim(r#type: &str) -> Option<&str>{
    match r#type {
        "System.UInt32" => Some("U32"),
        "System.UInt64" => Some("U64"),
        "System.Int32" => Some("S32"),
        "System.Int64" => Some("S64"),
        "System.Boolean" => Some("Bool"),
        "System.Guid" => Some("Guid"),
        _ => None
    }
}

pub fn get_generics(s: &str) -> (String, Vec<String>) {
    let s = s.replace("cRate", "cRate_");
    if s.contains("[[") {
        let x: Vec<&str> = s.split('[').collect();
        return (x[0].to_string(), vec![])
    }
    if let Some((name, generics_raw)) = s.split_once("<") {
        let generics_raw = generics_raw.strip_suffix(">").unwrap();
        let generics: Vec<String> = generics_raw.split(",").map(|s| {
            match convert_type_to_prim(s) {
                Some(s) => s,
                None => s
            }.to_string()
        }).collect();
        return (name.replace("`2", "").replace("`1", ""), generics)
    }
    return (s.to_string(), vec![])
}

impl RszField {
    pub fn gen_field(&self, is_last: bool, parent: &RszStruct<RszField>, enum_name: Option<String>, generic_symbol: Option<String>) -> (HashSet<u32>, TokenStream, String) {
        let use_original_type_types = vec!["Object", "Struct", "GameObjectRef", "UserData"];
        let full_field_type = if self.original_type == "ace.user_data.ExcelUserData.cData"{
            parent.name.clone() + ".cData"
        } else if let Some(enum_name) = enum_name.clone() {
            enum_name
        } else {
            self.original_type.clone()
        };

        let (mut field_type, generics, dep) = if !use_original_type_types.contains(&self.r#type.as_str()) && enum_name.is_none() {
            (self.r#type.to_string(), vec![], None)
        } else {
            let (field_type, generics) = get_generics(&full_field_type);
            let field_type: Vec<_> = field_type.split(".").collect();
            let namespace = field_type[..field_type.len() - 1].join("::").to_lowercase();
            let just_the_type = &field_type[field_type.len() - 1];
            let mut field_type = namespace;
            if field_type != "" {
                field_type.push_str("::");
            }
            field_type.push_str(&just_the_type);
            let hash = *RszDump::name_map().get(&full_field_type).unwrap();
            (field_type, generics, Some(hash))
        };
        // add generics to the type??

        if generics.len() > 0 {
            let generics = generics.iter().map(|g| {
                let field_type: Vec<_> = g.split(".").collect();
                let namespace = field_type[..field_type.len() - 1].join("::").to_lowercase();
                let just_the_type = &field_type[field_type.len() - 1];
                let mut field_type = namespace;
                if field_type != "" {
                    field_type.push_str("::");
                }
                field_type.push_str(&just_the_type);
                field_type
            }).collect::<Vec<String>>();
            let ext = generics.join(",").replace(".", "::");
            field_type = format!("{}<{}>", field_type, ext).replace(".", "_");
        }

        if self.array {
            if field_type.contains("cEntry") {
                println!("{self:?}, {}, {:?}", field_type, generics);
            }
            field_type = format!("Vec<{}>", field_type);
        }
        let new_name = if self.name == "type" {
            "r#type"
        } else {
            &self.name
        }.replace("crate", "r#crate");
        let new_name = format_ident!("{}", new_name);

        //println!("{}, {}, {}", self.original_type, self.name,field_type);
        if let Some(generic_symbol) = generic_symbol {
            field_type = generic_symbol;
        }
        let field_type: syn::Path = match syn::parse_str(&field_type) {
            Ok(x) => x,
            Err(_) => return (HashSet::new(), quote!{}, "".to_string())
        };
        let mut tokens = if self.r#type == "Object" || self.r#type == "UserData" {
            quote! { pub #new_name: Box < #field_type >}
        } else {
            quote! { pub #new_name: #field_type}
        };
        if !is_last {
            tokens.append_all(quote!{,});
        }

        let mut deps = HashSet::new();
        if let Some(dep) = dep {
            deps.insert(dep);
        }

        for generic in generics {
            if let Some(hash) = RszDump::name_map().get(&generic) {
                deps.insert(*hash);
            }
        }
        (deps, tokens, "".to_string())
    }
}

impl RszStruct<RszField> {
    pub fn gen_enum(&self, enum_type: &HashMap<String, String>) -> Vec<TokenStream> {
        struct EnumEntry<'a> {
            name: &'a str,
            val: &'a str,
        }
        let mut found: HashSet<i64> = HashSet::new();
        let enums: Vec<TokenStream> = enum_type.iter().flat_map(|(k, v)| {
            if let Ok(_v) = k.parse::<i64>() {
                return None
            }
            let name = format_ident!("{k}");
            let v = v.parse::<i64>().unwrap();
            let val = Literal::i64_unsuffixed(v);
            if found.contains(&v) {
                let val = Literal::i64_unsuffixed(v + 123213);
                let name = format_ident!("{k}_DUPLICATEENUM");
                found.insert(v);
                Some(quote!{ #name = #val, })
            } else {
                found.insert(v);
                Some(quote!{ #name = #val, })
            }
        }).collect();
        enums
    }

    pub fn gen_struct(&self) -> Option<(HashSet<u32>, SdkComponent)> {
        if self.name == "" {
            return None
        }
        println!("\nstruct: {}", self.name);
        // clean this shit up probably maybe?? ye nah
        let tokens = match self.name.as_str() {
            "via.vec4" => Some(quote!{ pub type vec4 = Vec4; }),
            "via.vec3" => Some(quote!{ pub type vec3 = Vec3; }),
            "via.vec2" => Some(quote!{ pub type vec2 = Vec2; }),
            "via.mat4" => Some(quote!{ pub type mat4 = Mat4; }),
            _ => None
        };

        // have to replace the name "cRate" -> to cRate_ because it collides with the crate
        // keyword, maybe add something that filters out keywords, yes probably good idea
        // probably dont actually replace here, do it when generating things
        let (struct_name, generics) = get_generics(&self.name);
        let struct_name = struct_name;

        let mut deps = HashSet::<u32>::new(); // the dependancies here are basically the same as
                                              // includes, just hashes instead of full names
                                              // might not be needed?
        let mut includes = HashSet::<String>::new();
        for generic in &generics {
            includes.insert(generic.clone());
        }

        if let Some(tokens) = tokens {
            return Some((HashSet::new(), SdkComponent::new(struct_name, tokens, includes)))
        }

        // if you're secretly an enum, deal with that shit
        if let Some(enum_type) = enum_map().get(&self.name) {
            println!("enum_name={}", self.name);
            let name: syn::Path = syn::parse_str(&struct_name.split(".").last().unwrap()).unwrap();
            let enums = self.gen_enum(enum_type);
            let tokens = quote!{
                #[repr(i32)]
                #[derive(Debug, serde::Deserialize)]
                pub enum #name {
                    #(#enums)*
                }
            };
            return Some((HashSet::new(), SdkComponent::new(struct_name, tokens, includes)))
        }

        // if the enum is _Serializable, make the Fixed one a dependancy
        // THIUS SHIT SODWONOST MAKE ANY SENSE
        let enum_name = self.name.replace("_Serializable", "_Fixed");
        let enum_type = enum_map().get(&enum_name);
        if let Some(dep) = RszDump::name_map().get(&enum_name) {
            deps.insert(*dep);
            includes.insert(enum_name.clone());
        }
        if enum_type.is_some() {
            println!("enum_name={}", self.name);
            let name: syn::Path = syn::parse_str(&struct_name.split(".").last().unwrap()).unwrap();
            let field_type: Vec<_> = enum_name.split(".").collect();
            let namespace = field_type[..field_type.len() - 1].join("::").to_lowercase();
            let just_the_type = &field_type[field_type.len() - 1];
            let enum_name: syn::Path = syn::parse_str(&format!("{namespace}::{just_the_type}")).unwrap();
            let tokens = quote!{ #[derive(Debug, serde::Deserialize)] pub struct #name (#enum_name);};
            return Some((deps, SdkComponent::new(struct_name, tokens, includes)))
        }

        let mut generic_counter = 0;
        let generic_symbols = ["T", "S", "T1", "S2", "T2", "T3", "T4", "T5"];
        let mut used_gens = vec![];
        let fields: Vec<_> = self.fields.iter().enumerate().map(|(i, field)| {
            let enum_name = if enum_type.is_some() {
                Some(enum_name.clone())
            } else if enum_map().get(&field.original_type).is_some() {
                Some(field.original_type.clone())
            } else {None};

            let mut tmp_generics = generics.clone();
            let gen_sym = if let Some(pos) = tmp_generics.iter().position(|x| *x == field.original_type) {
                let x = Some(generic_symbols[generic_counter].to_string());
                let res = tmp_generics.remove(pos);
                used_gens.push((generic_symbols[generic_counter], res));
                generic_counter += 1;
                x
            } else {None};

            let (field_deps, tokens, inc) = field.gen_field(i+1 == self.fields.len(), self, enum_name, gen_sym);
            for dep in field_deps {
                deps.insert(dep);
            }
            if inc != "" {
                //println!("\t{}", inc);
                includes.insert(inc);
            }
            tokens
        }).collect();
        // if there are generics, deal with them
        let (generics_tokens, _ext) = if used_gens.len() > 0 {
            let gens_tokens: Vec<_> = used_gens.iter().enumerate().map(|(i, (sym, g))| {
                println!("{g:?}");

                let field_type = g;
                let field_type: Vec<_> = field_type.split(".").collect();
                let namespace = field_type[..field_type.len() - 1].join("::").to_lowercase();
                let just_the_type = &field_type[field_type.len() - 1];
                let mut field_type = namespace;
                if field_type != "" {
                    field_type.push_str("::");
                }
                field_type.push_str(&just_the_type);

                if let Some(hash) = RszDump::name_map().get(&g.replace("::", ".")) {
                    deps.insert(*hash);
                }
                let g: syn::Path = syn::parse_str(&field_type).unwrap();
                let sym: syn::Path = syn::parse_str(&sym).unwrap();

                if i + 1 == used_gens.len() {
                    quote!{#sym = #g}
                } else {
                    quote!{#sym = #g,}
                }
            }).collect();
            let ext = generics.join(",").replace("::", "");
            (gens_tokens, ext)
        } else {
            (vec![], "".to_string())
        };
        let tokens = if !generics.is_empty() {
            let name = struct_name.split(".").last().unwrap().replace("cRate", "cRate_");
            let name: syn::Path = syn::parse_str(&name).unwrap();

            quote!{
                #[derive(Debug, serde::Deserialize)]
                pub struct #name < #(#generics_tokens)* > {
                    #(#fields)*
                }
            };
            quote!{}
        } else if struct_name != "" {
            //println!("{struct_name}");
            let tmp: Vec<_> = self.name.split(".").collect();
            let name: syn::Path = syn::parse_str(&tmp.last().unwrap().replace("cRate", "cRate_")).unwrap();
            quote!{
                #[derive(Debug, serde::Deserialize)]
                pub struct #name {
                    #(#fields)*
                }
            }
        } else {
            quote!{}
        };
        return Some((deps, SdkComponent::new(struct_name, tokens, includes)));
    }
}


/*
impl RszValue {
    pub fn to_buffer(&self, base_addr: usize) -> Result<Vec<u8>> {
        let mut data: Vec<u8> = vec![];
        let struct_info = RszDump::rsz_map().get(self.hash().unwrap()).unwrap();
        for (i, field) in self.fields.iter().enumerate() {
            let field_info = &struct_info.fields[i];
            if field_info.array {
                if (data.len() + base_addr) % 4 as usize != 0 {
                    data.extend(vec![0; 4 - (data.len() + base_addr) % 4 as usize]);
                }
            }
            field.write_to(&mut data, &field_info, base_addr, field_info.array)?;
        }
        Ok(data)
    }
}*/

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
    pub fn rsz_map() -> &'static RszMap<RszMapType> {
        static HASHMAP: OnceLock<RszMap<RszMapType>> = OnceLock::new();
        HASHMAP.get_or_init(|| {
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

pub fn enum_val_map() -> &'static EnumMap {
    static HASHMAP: OnceLock<EnumMap> = OnceLock::new();
    HASHMAP.get_or_init(|| {
        let json_data = std::fs::read_to_string("enumtoval.json").unwrap();
        let hashmap: EnumMap = serde_json::from_str(&json_data).unwrap();
        hashmap
    })
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
