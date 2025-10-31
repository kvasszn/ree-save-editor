#![allow(unused)]
use crate::file_ext::*;
use crate::rsz::Rsz;
use crate::reerr::{Result, FileParseError::*};
use std::io::{Read, Seek};
use nalgebra_glm::Vec4;
use serde::Serialize;
use uuid::Uuid;

pub struct Scn {
    game_objects: Vec<GameObject>,
    pub rsz: Rsz
}

pub struct GameObject {
    guid: Uuid,
    unk1: u32,
    unk2: u32,
    unk3: u32,
    unk4: u32,
}

pub struct Folder {
    unk1: u32,
    unk2: u32,
    unk3: u32,
    unk4: u32,
}

pub struct Child {
    unk1: u32,
    unk2: u32,
    unk3: u32,
    unk4: u32,
}

impl Scn {
    // thanks to mhrice for the structure
    pub fn new<F: Read + Seek>(mut file: F) -> Result<Scn> {
        let magic = file.read_magic()?;
        let ext = core::str::from_utf8(&magic)?;
        if &magic != b"SCN\0" {
            return Err(Box::new(MagicError { 
                real_magic: String::from("SCN"), 
                read_magic: ext.to_string()
            }))
        }

        let game_obj_count = file.read_u32()?;
        let resource_count = file.read_u32()?;
        let folder_count = file.read_u32()?;
        let prefab_count = file.read_u32()?;
        let child_count = file.read_u32()?;

        let folder_list_offset = file.read_u64()?;
        let resource_list_offset = file.read_u64()?;
        let prefab_list_offset = file.read_u64()?;
        let child_list_offset = file.read_u64()?;
        let rsz_offset = file.read_u64()?;
        
        let game_objects = (0..game_obj_count).map(|_| {
            let x = GameObject {
                guid: file.read_guid()?,
                unk1: file.read_u32()?,
                unk2: file.read_u32()?,
                unk3: file.read_u32()?,
                unk4: file.read_u32()?,
            };
            Ok(x)
        }).collect::<Result<Vec<_>>>()?; 
        
        file.seek(std::io::SeekFrom::Start(folder_list_offset))?;
        let folders = (0..folder_count).map(|_| {
            let x = Folder {
                unk1: file.read_u32()?,
                unk2: file.read_u32()?,
                unk3: file.read_u32()?,
                unk4: file.read_u32()?,
            };
            Ok(x)
        }).collect::<Result<Vec<_>>>()?;

        file.seek(std::io::SeekFrom::Start(resource_list_offset))?;
        let resource_offsets = (0..resource_count).map(|_| {
            let x = file.read_u64()?;
            Ok(x)
        }).collect::<Result<Vec<_>>>()?;

        file.seek(std::io::SeekFrom::Start(prefab_list_offset))?;
        let prefab_offsets = (0..prefab_count).map(|_| {
            let x = file.read_u64()?;
            Ok(x)
        }).collect::<Result<Vec<_>>>()?;

        file.seek(std::io::SeekFrom::Start(child_list_offset))?;
        let child_offsets = (0..child_count).map(|_| {
            let x = file.read_u64()?;
            Ok(x)
        }).collect::<Result<Vec<_>>>()?;

        // actually read the names and stuff for the other things
        

        // rsz
        file.seek(std::io::SeekFrom::Start(rsz_offset))?;
        let rsz = Rsz::new(&mut file, rsz_offset, 0)?;

        Ok(Scn {
            game_objects,
            rsz,
        })
    }
}

