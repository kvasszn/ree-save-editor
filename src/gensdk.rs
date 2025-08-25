use std::collections::{HashMap, HashSet, VecDeque};
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;

use proc_macro2::TokenStream;
use quote::quote;

use crate::rsz::dump::RszDump;
use crate::reerr::Result;

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
        //println!("path_created={path:?}");
        std::fs::create_dir_all(&path).unwrap();

        let submodules: Vec<_> = self.submodules.iter().map(|s| {
            let path: syn::Path = syn::parse_str(&s).unwrap();
            quote! {
                pub mod #path;
            }
        }).collect();

        let structs: Vec<_> = self.structs.iter().map(|(_, v)| &v.tokens).collect();
        let tokens = if self.name == "lib" {
            quote! {
                #![allow(unused, nonstandard_style)]
                #(#submodules)*
                #(#structs)*
            }
        } else {
            quote! {
                #![allow(unused, nonstandard_style)]
                use crate::*;
                #(#submodules)*
                #(#structs)*
            }
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
    pub fn get_parent(&self) -> &str {
        let tmp: Vec<_> = self.name.split(".").collect();
        tmp[tmp.len() - 2]
    }
    pub fn get_path(&self) -> String {
        let tmp: Vec<_> = self.name.split(".").collect();
        tmp[..tmp.len() - 2].join("/").to_lowercase()
    }
    pub fn get_file_name(&self) -> String {
        self.get_parent().to_string().to_lowercase().replace("crate", "crate_").replace("crate__", "crate_")
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
        if let Some(charthings) = RszDump::name_map().get(&"app.user_data.CharacterEditThumbnailTextureData".to_string()) {
            queue.push_back(*charthings);
        }
        if let Some(charthings) = RszDump::name_map().get(&"app.user_data.CharacterEditThumbnailTexturePairData".to_string()) {
            queue.push_back(*charthings);
        }
        while let Some(hash) = queue.pop_front() {
            // get the struct we're gonna look at
            let rsz_struct = match RszDump::rsz_map().get(&hash) {
                Some(x) => x,
                None => {eprintln!("Invalid hash {hash:x}"); continue},
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
            if let Some((deps, sdk_struct)) = rsz_struct.gen_struct() {
                for dep in deps {
                    queue.push_back(dep);
                }
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
            //println!("file: {}, {:?}, {:?}", file_name, path_buf_dir, struct_file_name);
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
            //println!("");
            sdk_file.add(sdk_struct);
        }
        let type_defs = quote! {
            pub type U8 = u8;
            pub type U16 = u16;
            pub type U32 = u32;
            pub type U64 = u64;
            pub type S8 = i8;
            pub type S16 = i16;
            pub type S32 = i32;
            pub type S64 = i64;
            pub type F8 = u8;
            pub type F16 = u16;
            pub type F32 = f32;
            pub type F64 = f64;
            pub type Size = u64;
            pub type Bool = bool;
            pub type Guid = String;
            pub type Float2 = [f32;2];
            pub type Float3 = [f32;3];
            pub type Float4 = [f32;4];
            pub type Vec4 = [f32;4];
            pub type Vec3 = [f32;3];
            pub type Vec2 = [f32;2];
            pub type Mat4 = [[f32;4];4];
            pub type Data = Vec<u8>;
            pub type Resource = String;
            pub type RuntimeType = ();
            pub type Quaternion = [[f32;4];4];
            pub type Range = std::ops::Range<u32>;
            #[derive(Debug, serde::Deserialize)]
            pub struct AABB(f32, f32, f32, f32);
            pub type OBB = ();
            pub type Color = (u8, u8, u8, u8);
            pub type KeyFrame = [f32;4];
        };
        let mut lib_tokens = HashMap::new();
        lib_tokens.insert("lib".to_string(), SdkComponent::new("lib".to_string(), type_defs, HashSet::new()));
        self.files.insert("lib.rs".to_string(), SdkFile {
            name: "lib".to_string(),
            path: "".to_string(),
            deps: HashSet::new(),
            submodules: HashSet::new(),
            structs: lib_tokens,
        });

        fn new_custom(struct_name: String, code: TokenStream) -> (String, String, String, SdkComponent) {
            let sdk_struct = SdkComponent::new(struct_name.to_string(), code, HashSet::new());
            let struct_file_name = sdk_struct.get_file_name();
            let path_name = sdk_struct.get_path();
            let mut path_buf_rs = PathBuf::from(&path_name);
            path_buf_rs.push(struct_file_name.clone() + ".rs");
            let mut path_buf_dir = PathBuf::from(&path_name);
            path_buf_dir.push(&struct_file_name);
            let file_name = path_buf_rs.to_string_lossy().to_string();
            (file_name, struct_file_name, path_name, sdk_struct)
        }
        let customs = [
            new_custom(
                "app.user_data.OptionGraphicsData.cSetting".to_string(),
                quote! {
                    #[derive(Debug, serde::Deserialize)]
                    pub struct cSetting<T, S> {
                        pub _Items: cItem<T, S>,
                    }
                }
            ),
            new_custom(
                "app.user_data.OptionDisplayData.cSetting".to_string(),
                quote! {
                    #[derive(Debug, serde::Deserialize)]
                    pub struct cSetting<T, S> {
                        pub _Items: cItem<T, S>,
                    }
                }
            ),
            new_custom(
                "app.user_data.OptionGraphicsData.cItem".to_string(),
                quote! {
                    #[derive(Debug, serde::Deserialize)]
                    pub struct cItem<T, S> {
                        pub _Option: T,
                        pub _Value: S,
                    }
                }
            ),
            new_custom(
                "app.user_data.OptionDisplayData.cItem".to_string(),
                quote! {
                    #[derive(Debug, serde::Deserialize)]
                    pub struct cItem<T, S> {
                        pub _Option: T,
                        pub _Value: S,
                    }
                }
            ),
            new_custom(
                "ace.Bitset".to_string(),
                quote! {
                    #[derive(Debug, serde::Deserialize)]
                    pub struct Bitset<T> {
                        pub _Value: Vec<T>,
                        pub _MaxElement: i32,
                    }
                },
            ),
            new_custom(
                "app.user_data.CharacterEditGenderedThumbnails.cEntry".to_string(),
                quote! {
                    #[derive(Debug, serde :: Deserialize)]
                    pub struct cEntry<T, S> {
                        _marker: std::marker::PhantomData<S>,
                        pub _Key: T,
                        pub _Value: app::user_data::CharacterEditThumbnailTexturePairData,
                    }
                }
            ),
            new_custom(
                "app.user_data.CharacterEditGenderedThumbnails.cEntry".to_string(),
                quote! {
                    #[derive(Debug, serde :: Deserialize)]
                    pub struct cEntry<T, S> {
                        _marker: std::marker::PhantomData<S>,
                        pub _Key: T,
                        pub _Value: app::user_data::CharacterEditThumbnailTexturePairData,
                    }
                }
            ),
            new_custom(
                "app.user_data.CharacterEditThumbnails.cEntry".to_string(),
                quote! {
                    #[derive(Debug, serde :: Deserialize)]
                    pub struct cEntry<T, S> {
                        _marker: std::marker::PhantomData<S>,
                        pub _Key: T,
                        pub _Value: app::user_data::CharacterEditThumbnailTextureData,
                    }
                }
            ),
            new_custom(
                "ace.btable.cEditFieldEnum".to_string(),
                quote! {
                    #[derive(Debug, serde :: Deserialize)]
                    pub struct cEditFieldEnum<T> {
                        pub _Value: T
                    }
                }
            ),
            new_custom(
                "ace.btable.cEditFieldDropBox".to_string(),
                quote! {
                    #[derive(Debug, serde :: Deserialize)]
                    pub struct cEditFieldDropBox<T> {
                        pub _Value: T
                    }
                }
            ),
            new_custom(
                "app.appactionutil.cActionParamEditableArray".to_string(),
                quote! {
                    #[derive(Debug, serde :: Deserialize)]
                    pub struct cActionParamEditableArray<T> {
                        pub _ElementList: Vec<T>
                    }
                }
            ),
            new_custom(
                "ace.cInstanceGuidArray".to_string(),
                quote! {
                    #[derive(Debug, serde :: Deserialize)]
                    pub struct cInstanceGuidArray<T> {
                        pub _DataArray: Vec<T>
                    }
                }
            ),
            new_custom(
                "app.InstanceGuidArray".to_string(),
                quote! {
                    #[derive(Debug, serde :: Deserialize)]
                    pub struct InstanceGuidArray<T> {
                        pub _DataArray: Vec<T>
                    }
                }
            ),
            new_custom(
                "ace.cUserDataArgumentHolder".to_string(),
                quote! {
                    #[derive(Debug, serde :: Deserialize)]
                    pub struct cUserDataArgumentHolder<T> {
                        pub _Argument: T
                    }
                }
            ),
            new_custom(
                "system.Nullable".to_string(),
                quote! {
                    pub type Nullable<T> = Option<T>;
                }
            ),
            new_custom(
                "system.collections.generic.Dictionary".to_string(),
                quote! {
                    pub type Dictionary<T, S> = std::collections::HashMap<T, S>;
                }
            ),
            new_custom(
                "app.cEnumerableParam".to_string(),
                quote! {
                    #[derive(Debug, serde :: Deserialize)]
                    pub struct cEnumerableParam<T, S> {
                        _marker: std::marker::PhantomData<T>,
                        pub _ParamList: Vec<S>
                    }
                }
            ),


        ];

        for (file, struct_name, path, comp) in customs {
            match self.files.get_mut(&file) {
                Some(file_store) => {
                    file_store
                },
                None => {
                    self.files.insert(file.to_string(), SdkFile {
                        name: struct_name.to_string(),
                        path: path.to_string(),
                        deps: HashSet::new(),
                        submodules: HashSet::new(),
                        structs: HashMap::new(),
                    });
                    self.files.get_mut(&file).unwrap()
                }
            }.add(comp);
        }

        let paths: Vec<_> = self.files.iter().map(|(k, v)| (k.clone(), v.path.clone())).collect();

        // the file must be added to the submodules of its parent, and its parent to its parent
        // parent, etc
        for (f, path) in paths {
            let path_parts: Vec<_> = path.split("/").collect();
            let base = path_parts[0].to_string();
            if f != "lib.rs" && base != "" {
                match self.files.get_mut("lib.rs") {
                    Some(module_file) => {
                        module_file.submodules.insert(base);
                    },
                    None => { }
                }
            }


            //println!("\nfile {:?}, parts{:?}", f, path_parts);
            for i in 0..path_parts.len() {
                let module_name = path_parts[0..i + 1].join("/").to_string();
                let module_file_path = module_name.clone() + ".rs";
                let file_path_parts = f.strip_suffix(".rs").unwrap().split("/").collect::<Vec<_>>();
                if file_path_parts.len() <= 1 {
                    continue;
                }
                let submod = file_path_parts[i + 1].to_string();
                match self.files.get_mut(&module_file_path) {
                    Some(module_file) => {
                        module_file.submodules.insert(submod.clone());
                    },
                    None => {
                        self.files.insert(module_file_path.clone(), SdkFile {
                            name: path_parts[i].to_string(),
                            path: path_parts[0..i].join("/"),
                            deps: HashSet::new(),
                            submodules: HashSet::from([submod.clone()]),
                            structs: HashMap::new(),
                        });
                    }
                }


            }
        }

        Ok(())
    }

    pub fn write_files(&self) {
        for (_path, file) in &self.files {
            //println!("{path}");
            file.write();
        }
        std::process::Command::new("rustfmt").arg(&"gen/sdk/src/lib.rs").status().unwrap();

    }
}




