use std::collections::{HashMap, HashSet};

use proc_macro2::{Literal, TokenStream};
use quote::{format_ident, quote, TokenStreamExt};
use crate::{gensdk::SdkComponent, rsz::dump::{enum_map, RszDump}};

use super::dump::{RszField, RszStruct};


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

impl RszStruct<RszField> {
    pub fn gen_enum(&self, enum_type: &HashMap<String, String>) -> Vec<TokenStream> {
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
