use serde::ser::SerializeStruct;
use serde::Deserialize;
use serde::Serialize;

use util::*;
use crate::rsz::{Rsz, rszserde::{DeRsz, DeRszRegistry}};
use crate::reerr::{Result, FileParseError::*};
use std::io;
use std::io::Write;
use std::io::{Read, Seek};

#[allow(dead_code)]
#[derive(Debug, Serialize, Deserialize)]
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
        let ext = core::str::from_utf8(&magic)?; if ext != "USR\0" {
            return Err(Box::new(MagicError { 
                real_magic: String::from("USR\0"), 
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
            .collect::<io::Result<Vec<_>>>()?;

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
            .map(|_resource_name_offset| {
                let name = file.read_u16str()?;
                if name.ends_with(".user") {
                    return Err("Found prohibited .user resource".into())
                }
                Ok(name)
            })
            .collect::<Result<Vec<_>>>()?;

        let children = child_info
            .into_iter()
            .map(|(hash, _name_offset)| {
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
    
    pub fn rsz_data<F: Read + Seek>(mut file: F) -> Result<Vec<u8>> {
        let magic = file.read_magic()?;
        let ext = core::str::from_utf8(&magic)?; if ext != "USR\0" {
            return Err(Box::new(MagicError { 
                real_magic: String::from("USR\0"), 
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
            .collect::<io::Result<Vec<_>>>()?;

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
            .map(|_resource_name_offset| {
                let name = file.read_u16str()?;
                if name.ends_with(".user") {
                    return Err("Found prohibited .user resource".into())
                }
                Ok(name)
            })
            .collect::<Result<Vec<_>>>()?;

        let children = child_info
            .into_iter()
            .map(|(hash, _name_offset)| {
                let name = file.read_u16str()?;
                if !name.ends_with(".user") {
                    return Err("Non-USER child".into());
                }
                Ok(UserChild { hash, name })
            })
            .collect::<Result<Vec<_>>>()?;

        file.seek(io::SeekFrom::Start(rsz_offset))?;
        let mut rsz_data = vec![];
        file.read_to_end(&mut rsz_data)?;
        Ok(rsz_data)
    }

    pub fn to_buf(&self) -> Result<Vec<u8>> {

        let mut buf = vec![];
        buf.write_all(b"USR\0")?;
        buf.write(&(self.resource_names.len() as u32).to_le_bytes())?;
        buf.write(&(self.children.len() as u32).to_le_bytes())?;
        buf.write_all(&[0; 4])?;
        let resource_list_offset = buf.len() + size_of::<u64>() * 4;
        // this has to be aligned by 16 likely
        let child_list_offset = resource_list_offset + self.resource_names.len() * size_of::<u64>();
        // this has to be aligned by 16 likely
        let rsz_offset = child_list_offset + self.children.len() * (size_of::<u32>() * 2 + size_of::<u64>());
        let rsz_buf = self.rsz.to_buf(rsz_offset)?;
        buf.write(&resource_list_offset.to_le_bytes())?;
        buf.write(&child_list_offset.to_le_bytes())?;
        buf.write(&rsz_offset.to_le_bytes())?;
        buf.write(&0u64.to_le_bytes())?;
        //buf.write(rsz_offset_cap as u64)?;
        for child in &self.children {
            buf.write(&child.hash.to_le_bytes())?;
            buf.write_all(&[0; 4])?;
            buf.write(&0u64.to_le_bytes())?;
            // figure out where to put the string tables
        }

        buf.extend(rsz_buf);
        Ok(buf)
    }
    pub fn from_json_file(file: &str) -> Result<User> {
        let data = std::fs::read_to_string(file)?;
        let json_data: serde_json::Value = serde_json::from_str(&data).unwrap();
        let rsz_json = json_data.get("rsz").unwrap();
        let mut registry = DeRszRegistry::new();
        registry.init();
        let dersz: DeRsz = DeRsz::from_json(rsz_json, registry.into())?; 
        let rsz = Result::<Rsz>::from(dersz)?;
        Ok(User {
            resource_names: vec![],
            children: vec![],
            rsz
        })
    }
}

impl Serialize for User {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer {
            let mut state = serializer.serialize_struct("user", 3)?;
            state.serialize_field("resource_names", &self.resource_names)?;
            state.serialize_field("children", &self.children)?;
            let dersz = match self.rsz.deserialize_to_dersz() {
                Ok(dersz) => dersz,
                Err(e) => return Err(serde::ser::Error::custom(format!("User serialization failed, {:?}", e).to_string()))
            };
            state.serialize_field("rsz", &dersz)?;
            state.end()
    }
}

