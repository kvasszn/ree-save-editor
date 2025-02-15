use crate::reerr::{FileParseError, Result};
use std::io::{Read, Seek};
use crate::file_ext::*;

pub const TDB_ANCHOR: &[u8] = b"TDB\0\x4A\0\0\0";

pub struct Tdb {
}

impl Tdb {
    pub fn new<F: Read + Seek>(mut f: F, offset: usize) -> Result<Tdb> {
        f.seek(std::io::SeekFrom::Start(offset as u64))?;
        let magic = f.read_magic()?;
        let version = f.read_u32()?;
        println!("{magic:?}, {version:010x}");
        let ext = core::str::from_utf8(&magic)?;
        if magic != *b"TDB\0" {
            return Err(Box::new(FileParseError::MagicError { 
                real_magic: String::from("TDB"), 
                read_magic: ext.to_string(),
            }))
        }
        let tdb = Tdb {

        };
        Ok(tdb)
    }
}
