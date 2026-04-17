use std::{collections::HashMap, fs::File, io::{BufReader, Cursor}};

use serde::{Deserialize, Serialize};

use crate::{file::{Msg, Result, StructRW, User}, save::remap::Remap, sdk::{rsz::Rsz, type_map::{self, TypeMap}}};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Asset {
    Rsz(Box<Rsz>),
    Msg(Box<Msg>),
}

impl Asset {
    pub fn load(path: &str, type_map: &TypeMap) -> Result<Self> {
        let (_, t) = path.split_once('.').unwrap();
        let file = File::open(path)?;
        let mut reader = BufReader::new(file);
        let asset = match t {
            "user.3" => {
                //let x = User::new(reader)?;
                //println!("{:?}", &x.rsz.data[0..32]);
                let data = User::rsz_data(reader)?;
                let mut data = Cursor::new(data);
                let rsz = Rsz::from_data(&mut data, 0, type_map)?;
                Self::Rsz(Box::new(rsz))
            },
            "msg.23" => Self::Msg(Box::new(Msg::read(&mut reader, &mut ())?)),
            _ => {
                return Err("Could not load asset".into())
            }
        };
        Ok(asset)
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Assets {
    pub loaded: HashMap<String, Asset>
}

impl Assets {
    pub fn bake_to_file(&self, output_path: &str) -> Result<()> {
        let packed_bytes = bincode::serialize(self)?;
        std::fs::write(output_path, packed_bytes)?;
        Ok(())
    }

    pub fn load_baked(path: &str) -> Result<Self> {
        let bytes = std::fs::read(path)?;
        Self::load_baked_bytes(&bytes)
    }

    pub fn load_baked_bytes(bytes: &[u8]) -> Result<Self> {
        let assets: Assets = bincode::deserialize(&bytes)?;
        Ok(assets)
    }

    pub fn bake_to_bitcode(&self, output_path: &str) -> Result<()> {
        let packed_bytes: Vec<u8> = bitcode::serialize(self)
            .map_err(|e| format!("Bitcode serialization failed: {:?}", e))?;

        std::fs::write(output_path, packed_bytes)?;
        Ok(())
    }

    pub fn load_bitcode(path: &str) -> Result<Self> {
        let bytes = std::fs::read(path)?;

        let assets: Assets = bitcode::deserialize(&bytes)
            .map_err(|e| format!("Bitcode deserialization failed: {:?}", e))?;

        Ok(assets)
    }

    pub fn load_by_remaps(&mut self, remaps: &HashMap<String, Remap>, type_map: &TypeMap) -> Result<()> {
        for (_, v) in remaps {
            for (_, v) in &v.data {
                let path = &v.path;
                println!("[INFO] Loading Asset {path}");
                let asset = Asset::load(path, type_map)?;
                self.loaded.insert(path.to_string(), asset);
            }
        }
        Ok(())
    }

    pub fn get_rsz<'a>(&'a self, path: &'a str) -> Option<&'a Rsz> {
        if let Some(asset) = self.loaded.get(path) {
            if let Asset::Rsz(rsz) = asset {
                return Some(rsz)
            }
        }
        None
    }

    pub fn get_msg<'a>(&'a self, path: &'a str) -> Option<&'a Msg> {
        if let Some(asset) = self.loaded.get(path) {
            if let Asset::Msg(msg) = asset {
                return Some(msg)
            }
        }
        None
    }
}
