use std::io::{Read, Seek};
use crate::file_ext::*; 
use crate::reerr::{Result, FileParseError::*};

pub struct Oft {
    pub data: Vec<u8>,
}

impl Oft {
    pub fn new<F: Read + Seek>(mut file: F) -> Result<Oft> {
        let magic = file.read_magic()?;
        let ext = core::str::from_utf8(&magic)?;
        if ext != "FBFO" {
            return Err(Box::new(MagicError { 
                real_magic: String::from("FBFO"), 
                read_magic: ext.to_string()
            }))
        }
        let mut data = vec![];
        file.read_to_end(&mut data)?;

        // decrypt
        let mut seed = 1u64;
        let delta = 0xAE6E39B58A355F45u64;
        let size = data.len() & 0x3F;
        if size > 0 {
            for _ in 0..size {
                seed = 2 * seed + 1;
            }
        }

        let key = (delta >> size) | (seed & delta)  << (64 - size);
        let key_bytes = key.to_le_bytes();
        if data.len() > 0 {
            for i in 0..data.len() {
                data[i] ^= key_bytes[i % 8];
            }
        }

        return Ok(Oft {
            data
        })
    }
}
