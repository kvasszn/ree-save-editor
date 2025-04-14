use byteorder::LittleEndian;
use byteorder::WriteBytesExt;
use serde::Deserialize;

use crate::file_ext::*;
use crate::rsz::*;
use crate::reerr::{Result, FileParseError::*};
use std::fs::File;
use std::io::Write;
use std::io::{Read, Seek};

#[allow(dead_code)]
#[derive(Debug)]
pub struct UserChild {
    pub hash: u32,
    pub name: String,
}

#[allow(dead_code)]
#[derive(Debug)]
pub struct User {
    pub resource_names: Vec<String>,
    pub children: Vec<UserChild>,
    pub rsz: Rsz,
}

impl User {
    pub fn new<F: Read + Seek>(mut file: F) -> Result<User> {
        let magic = file.read_magic()?;
        let ext = core::str::from_utf8(&magic)?;
        if ext != "USR\0" {
            return Err(Box::new(MagicError { 
                real_magic: String::from("USR"), 
                read_magic: ext.to_string()
            }))
        }

        let resource_count = file.read_u32()?;
        let child_count = file.read_u32()?;
        let padding = file.read_u32()?;
        if padding != 0 {
            return Err(format!("Unexpected non-zero padding A: {}", padding).into());
        }
        let resource_list_offset = file.read_u64()?;
        let child_list_offset = file.read_u64()?;
        let rsz_offset = file.read_u64()?;
        let rsz_offset_cap = file.read_u64()?;

        file.seek_assert_align_up(resource_list_offset, 16)?;
        let resource_name_offsets = (0..resource_count)
            .map(|_| file.read_u64())
            .collect::<Result<Vec<_>>>()?;

        file.seek_assert_align_up(child_list_offset, 16)?;
        let child_info = (0..child_count)
            .map(|_| {
                let hash = file.read_u32()?;
                let padding = file.read_u32()?;
                if padding != 0 {
                    return Err(format!("Child Unexpected non-zero padding A: {}", padding).into());
                }
                let name_offset = file.read_u64()?;
                Ok((hash, name_offset))
            })
            .collect::<Result<Vec<_>>>()?;

        let resource_names = resource_name_offsets
            .into_iter()
            .map(|resource_name_offset| {
                file.seek_noop(resource_name_offset)?;
                let name = file.read_u16str()?;
                if name.ends_with(".user") {
                    return Err("USER resource".into());
                }
                Ok(name)
            })
            .collect::<Result<Vec<_>>>()?;

        let children = child_info
            .into_iter()
            .map(|(hash, name_offset)| {
                file.seek_noop(name_offset)?;
                let name = file.read_u16str()?;
                if !name.ends_with(".user") {
                    return Err("Non-USER child".into());
                }
                Ok(UserChild { hash, name })
            })
            .collect::<Result<Vec<_>>>()?;


        let rsz = Rsz::new(&mut file, rsz_offset, rsz_offset_cap)?;

        Ok(User {
            resource_names,
            children,
            rsz,
        })
    }
    
    pub fn to_buf(&self) -> Result<Vec<u8>> {

        let mut buf = vec![];
        buf.write_all(b"USR\0")?;
        buf.write_u32::<LittleEndian>(self.resource_names.len() as u32)?;
        buf.write_u32::<LittleEndian>(self.children.len() as u32)?;
        buf.write_all(&[0; 4])?;
        let resource_list_offset = buf.len() + size_of::<u64>() * 4;
        // this has to be aligned by 16 likely
        let child_list_offset = resource_list_offset + self.resource_names.len() * size_of::<u64>();
        // this has to be aligned by 16 likely
        let rsz_offset = child_list_offset + self.children.len() * (size_of::<u32>() * 2 + size_of::<u64>());
        let rsz_buf = self.rsz.to_buf(rsz_offset)?;
        buf.write_u64::<LittleEndian>(resource_list_offset as u64)?;
        buf.write_u64::<LittleEndian>(child_list_offset as u64)?;
        buf.write_u64::<LittleEndian>(rsz_offset as u64)?;
        buf.write_u64::<LittleEndian>(0)?;
        //buf.write_u64::<LittleEndian>(rsz_offset_cap as u64)?;
        for child in &self.children {
            buf.write_u32::<LittleEndian>(child.hash)?;
            buf.write_all(&[0; 4])?;
            buf.write_u64::<LittleEndian>(0)?;
            // figure out where to put the string tables
        }

        buf.extend(rsz_buf);
        Ok(buf)
    }

    pub fn save_from_json(&self, file: &str) -> Result<()> {
        let data = self.to_buf()?;
        let mut file = File::create(file)?;
        file.write_all(&data)?;
        Ok(())
    }

    pub fn from_json_file(file: &str) -> Result<User> {
        let data = std::fs::read_to_string(file)?;
        let json_data: serde_json::Value = serde_json::from_str(&data).unwrap();
        let rsz: Rsz = Rsz::from_json(&json_data)?; 
        Ok(User {
            resource_names: vec![],
            children: vec![],
            rsz
        })
    }
}



