use crate::file_ext::*;
use crate::rsz::*;
use crate::reerr::{Result, FileParseError::*};
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
}


