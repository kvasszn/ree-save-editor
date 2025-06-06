use std::collections::{HashMap, HashSet, VecDeque};
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::PathBuf;

use proc_macro2::TokenStream;
use quote::{format_ident, quote};

use crate::dersz::{get_generics, RszDump, RszField};
use crate::reerr::Result;

const VIA_TYPES: [&str; 4] = ["vec4", "vec3", "vec2", "mat4"];
const SDK_BASE: &str = "gen/sdk/src/";

#[derive(Debug)]
pub struct Sdk {
    types: HashSet<u32>,
    files: HashMap<String, SdkFile>,
}

#[derive(Debug)]
pub struct SdkFile {
    name: String,
    path: String,
    deps: HashSet<String>,
    submodules: HashSet<String>,
    // name -> Tokens for the struct, these can also be enums or types i think, maybe not actually,
    // could just have a seperate map later
    structs: HashMap<String, SdkComponent>
}

impl SdkFile {
    pub fn add(&mut self, component: SdkComponent) {
        self.deps = self.deps.union(&component.deps)
            .into_iter().map(|s| s.clone()).collect();
        self.structs.insert(component.name.clone(), component);
    }

    pub fn write(&self) {
        let mut path = PathBuf::from(SDK_BASE);
        path.push(&self.path);
        std::fs::create_dir_all(&path).unwrap();
        let includes: Vec<_> = self.deps.iter().map(|s| {
            let s = s.replace(".", "::").to_lowercase();
            let path: syn::Path = syn::parse_str(&s).unwrap();
            quote! {
                use crate::#path;
            }
        }).collect();
        let structs: Vec<_> = self.structs.iter().map(|(k, v)| &v.tokens).collect();
        let tokens = quote! {
            #(#includes)*
            #(#structs)*
        };
        path.push(format!("{}.rs", self.name));
        let mut file = File::create(&path).unwrap();
        file.write_all(tokens.to_string().as_bytes()).unwrap();
    }
}

// same thing but whatever 
// // dependancies names get cut off by the struct name 
#[derive(Debug)]
pub struct SdkComponent {
    name: String,
    //parent: String,
    //path: String,
    deps: HashSet<String>,
    // name -> Tokens for the struct, these can also be enums or types i think, maybe not actually,
    // could just have a seperate map later
    tokens: TokenStream
}

impl SdkComponent {
    pub fn new(name: String, tokens: TokenStream, deps: HashSet<String>) -> Self {
        let deps: HashSet<String> = deps.into_iter().filter(|s| *s != name).collect();
        Self {
            name,
            deps,
            tokens
        }
    }
    pub fn get_parent(&self) {

    }
    pub fn get_path(&self) -> String {
        let tmp: Vec<_> = self.name.split(".").collect();
        tmp[..tmp.len() - 1].join("/").to_lowercase()
    }
    pub fn get_file_name(&self) -> String {
        let tmp: Vec<_> = self.name.split(".").collect();
        tmp.last().unwrap().to_lowercase()
    }
    
}

impl Sdk {
    pub fn new() -> Sdk {
        Self {
            types: HashSet::new(),
            files: HashMap::new(),
        }
    }
    pub fn add_types(&mut self, types: HashSet<u32>) -> Result<()> {
        let mut structs = Vec::new();
        
        let mut queue: VecDeque<u32> = types.into_iter().collect();
        while let Some(hash) = queue.pop_front() {
            // get the struct we're gonna look at
            let rsz_struct = match RszDump::rsz_map().get(&hash) {
                Some(x) => x,
                None => return Err("Invalid hash".into())
            };

            // if it's something we've seen already or null, skip
            if rsz_struct.name == "" || self.types.get(&hash).is_some() {
                continue;
            }

            // we've now seen it, so don't look at it again
            self.types.insert(hash);
            // example:
            // struct = app.user_data.OptionDisplayData.cItem`2<app.OptionParamDef.PL_TRANSPARENT,via.Color>
            //          ^^^ name                                ^^^generics, list, be careful,
            //          there technically can be generics within each part of the thing, handle
            //          that in get generics
            if let Some(sdk_struct) = rsz_struct.gen_struct() {
                structs.push(sdk_struct);
            }
        }

        //println!("{:#?}", structs);
        for sdk_struct in structs {
            let struct_file_name = sdk_struct.get_file_name();
            let path_name = sdk_struct.get_path();
            let mut path_buf_rs = PathBuf::from(&path_name);
            path_buf_rs.push(struct_file_name.clone() + ".rs");
            let mut path_buf_dir = PathBuf::from(&path_name);
            path_buf_dir.push(&struct_file_name);
            let file_name = path_buf_rs.to_string_lossy().to_string();
            let sdk_file = match self.files.get_mut(&file_name) {
         Some(sdk_file) => sdk_file,
                None => {
                    self.files.insert(file_name.clone(), SdkFile {
                        name: struct_file_name,
                        path: path_name,
                        deps: HashSet::new(),
                        submodules: HashSet::new(),
                        structs: HashMap::new(),
                    });
                    self.files.get_mut(&file_name).unwrap()
                }
            };
            sdk_file.add(sdk_struct);
        }

        println!("{:#?}", self.files);
        
        /*println!("{:#?}", sdk);
        for (path, (code, includes, _module, mods)) in sdk {
            let mut path_buf = PathBuf::new();
            path_buf.push(base);
            path_buf.push(&path);
            std::fs::create_dir_all(&path_buf.to_string_lossy().to_lowercase())?;
            path_buf.set_extension("rs");
            //println!("{path}");
            let mods: Vec<_> = mods.iter().map(|f| {
                let file: Vec<&str> = f.split(".").collect();
                let mut _mod = file[0].to_lowercase();
                //println!("{:?}", _mod);
                let file: syn::Path = syn::parse_str(&_mod).unwrap();
                quote! {
                    pub mod #file;
                }
            }).collect();

            let includes: Vec<_> = includes.into_iter().map(|s| {
                //println!("{s}");
                if s == "" {
                    return quote!{}
                }
                let path: syn::Path = syn::parse_str(&s).unwrap();
                quote! {
                    use crate::#path;
                }
            }).collect();
            let includes = quote!{
                #(#includes)*
            }.to_string();

            let mods = quote! { #(#mods)* };
            let mut file = File::create(&path_buf.to_string_lossy().to_lowercase())?;
            file.write_all(format!("// {path}\n// auto generated using mhtame\n").as_bytes())?;
            file.write_all("#![allow(non_snake_case, non_camel_case_types, unused_variables, dead_code, unused_imports)]\nuse crate::natives::*;\n".as_bytes())?;
            file.write_all(mods.to_string().as_bytes())?;
            file.write_all(includes.as_bytes())?;
            file.write_all(code.as_bytes())?;
        }*/

        /*let mut file = File::create("gen/sdk/src/natives.rs")?;
        let type_defs = quote! {
            use std::marker::PhantomData;
            pub type U8 = u8;
            pub type U16 = u16;
            pub type U32 = u32;
            pub type U64 = u64;
            pub type I8 = i8;
            pub type I16 = i16;
            pub type I32 = i32;
            pub type I64 = i64;
            pub type Vec4 = [f32;4];
            pub type Vec3 = [f32;3];
            pub type Vec2 = [f32;2];
            pub type Mat4 = [[f32;4];4];
            pub type Data = Vec<u8>;
            pub struct Bitset<T> {
                _marker: PhantomData<T>,
                _Value: Vec<u32>,
                _MaxElement: i32,
            }
            pub struct cItem<T, S> {
                _Option: T,
                _Value: S,
            }
            pub struct cEntry<T, S> {
                _Key: T,
                _Value: S,
            }
            pub struct cSetting<T, S> {
                _Items: cItem<T, S>,
            }
        };
        let tokens = type_defs.to_string();
        file.write_all(tokens.to_string().as_bytes())?;*/
        //std::process::Command::new("rustfmt").arg(&"gen/sdk/src/lib.rs").status()?;
        Ok(())
    }

    pub fn write_files(&self) {
        for (path, file) in &self.files {
            println!("{path}");
            file.write();
        }
        std::process::Command::new("rustfmt").arg(&"gen/sdk/src/lib.rs").status().unwrap();

    }
}




