use std::collections::{HashMap, HashSet, VecDeque};
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::PathBuf;

use quote::{format_ident, quote};

use crate::dersz::{get_generics, RszDump, RszField};
use crate::reerr::Result;

const VIA_TYPES: [&str; 4] = ["vec4", "vec3", "vec2", "mat4"];

pub struct Test<S, Y> {
    x: S,
    y: Y
}

pub fn gen_sdk(types: HashSet<u32>) -> Result<()> {
    let mut added: HashSet<u32> = HashSet::new();
    let mut queue: VecDeque<u32> = types.iter().map(|x| *x).collect();
    let base = "gen/sdk/src/";
    std::fs::create_dir_all(base)?;
    let mut sdk: HashMap<String, (String, HashSet<String>, String, HashSet<String>)> = HashMap::new();
    while let Some(hash) = queue.pop_front() {
        let rsz_struct = match RszDump::rsz_map().get(&hash) {
            Some(x) => x,
            None => return Err("Invalid hash".into())
        };
        if added.get(&hash).is_none() && rsz_struct.name != "" {
            added.insert(hash);
            /*
             *  via.rs
             *  via
             *      render
             *      render.rs
             *  app
             *
             */
            let (tmp, generics) = get_generics(&rsz_struct.name.replace("cRate", "cRate_"));

            //let tmp = rsz_struct.name.replace("cRate", "cRate_").replace("`2", "").replace("`1", ""); // lol
            let mut namespaces: Vec<_> = tmp.split(".").collect();
            let _struct_name = namespaces.pop().unwrap();
            let file_path = namespaces.clone().join("/");
            let module = namespaces.pop().unwrap();
            let path = namespaces.clone().join("/");
            let (deps, tokens, includes) = rsz_struct.gen_struct();
            deps.iter().for_each(|dep| if added.get(dep).is_none() { queue.push_back(*dep) });
            let code = tokens.to_string();
            if let Some((file, inc, _mod, _mods)) = sdk.get_mut(&file_path) {
                *file += &code;
                inc.extend(includes);
                //structs.push(struct_name.to_string());
                if *_mod == "" {
                    *_mod = module.to_string();
                }
                //(*uses).insert(struct_name.to_string());
            } else {
                sdk.insert(file_path.clone(), (code, includes, module.to_string(), HashSet::new()));
            }

            if let Some((_file, _inc, _, mods)) = sdk.get_mut(&path) {
                mods.insert(module.to_string());
            } else {
                let mut set = HashSet::new();
                set.insert(module.to_string());
                sdk.insert(path.clone(), ("".to_string(), HashSet::new(), "".to_string(), set));
            }
        }
    }
    println!("{:#?}", sdk);
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
    }

    let mut file = File::create("gen/sdk/src/natives.rs")?;
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
    file.write_all(tokens.to_string().as_bytes())?;
    std::process::Command::new("rustfmt").arg(&"gen/sdk/src/lib.rs").status()?;
    Ok(())
}

