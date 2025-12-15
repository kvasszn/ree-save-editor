use std::{collections::HashMap, error::Error, io::{self, Read, Seek, SeekFrom}};

use serde::Serialize;
use util::{ReadExt, SeekExt};

use crate::{deserializer::RszDeserializer, rsz, type_map::{self, TypeMap}, types::TypeDescriptor, value::{Extern, Instance, Value}};


#[derive(Debug, Clone)]
pub struct Rsz {
    pub roots: Vec<u32>,
    pub instances: Vec<Instance>,
    pub externs: HashMap<u32, Extern>,
}

impl Rsz {
    pub fn from_data<R: Read + Seek>(data: &mut R, base: u64, type_map: &TypeMap) -> Result<Self, Box<dyn Error>> {
        let header = RszHeader::new(data, base, 0)?;
        let mut deserializer = RszDeserializer::from_rsz_header(&header, type_map);
        deserializer.deserialize()
    }
}

#[derive(Debug, Clone)]
pub struct RszHeader {
    version: u32,
    pub roots: Vec<u32>,
    pub extern_slots: HashMap<u32, String>,
    pub type_descriptors: Vec<TypeDescriptor>,
    pub data: Vec<u8>,
}

impl RszHeader {
    pub fn new<F: Read + Seek>(file: &mut F, base: u64, cap: u64) -> Result<RszHeader, Box<dyn Error>> {
        file.seek(SeekFrom::Start(base))?;
        let magic: [u8; 4] = file.read_magic()?;
        if &magic != b"RSZ\0" {
            return Err("Wrong Magic".into())
        }

        let version = file.read_u32()?;
        if version != 0x10 {
            return Err(format!("Unexpected RSZ version {}", version).into());
        }

        let root_count = file.read_u32()?;
        let type_descriptor_count = file.read_u32()?;
        let extern_count = file.read_u32()?;
        let padding = file.read_u32()?;
        if padding != 0 {
            return Err(format!("Unexpected non-zero padding in RSZ: {}", padding).into());
        }

        let _type_descriptor_offset = file.read_u64()?;
        let data_offset = file.read_u64()?;
        let extern_offset = file.read_u64()?;

        let roots = (0..root_count)
            .map(|_| file.read_u32())
            .collect::<io::Result<Vec<_>>>()?;

        //file.seek_noop(base + type_descriptor_offset)?;

        let type_descriptors: Vec<_> = (0..type_descriptor_count)
            .map(|_| {
                let hash = file.read_u32()?;
                let crc = file.read_u32()?;
                Ok(TypeDescriptor { hash, crc })
            })
            .collect::<io::Result<_>>()?;

        if type_descriptors.first() != Some(&TypeDescriptor { hash: 0, crc: 0 }) {
            return Err(format!("The first type descriptor should be 0").into())
        }

        //file.seek_assert_align_up(base + extern_offset, 16)?;
        file.seek(SeekFrom::Start(base + extern_offset))?;
        file.seek_align_up(16)?;

        let extern_slot_info = (0..extern_count)
            .map(|_| {
                let slot = file.read_u32()?;
                let hash = file.read_u32()?;
                let offset = file.read_u64()?;
                Ok((slot, hash, offset))
            })
            .collect::<io::Result<Vec<_>>>()?;

        let extern_slots = extern_slot_info
            .into_iter()
            .map(|(slot, hash, _offset)| {
                //file.seek_noop(base + offset)?;
                let path = file.read_u16str()?;
                if hash != type_descriptors
                        .get(slot as usize)
                        .expect("slot out of bound")
                        .hash
                {
                    return Err("slot hash mismatch".into())
                }
                Ok((slot, path))
            })
            .collect::<Result<HashMap<u32, String>, Box<dyn Error>>>()?;
        //println!("{extern_slots:?}");
        //file.seek(SeekFrom::Start(base + data_offset))?;
        //file.seek_assert_align_up(base + data_offset, 16)?;
        file.seek(SeekFrom::Start(base + data_offset))?;
        file.seek_align_up(16)?;
        let mut data: Vec<u8> = vec![];
        if cap != 0 {
            let len = (cap) as usize - (base + data_offset) as usize;
            let current_pos = file.seek(SeekFrom::Current(0))?;
            let total_size = file.seek(SeekFrom::End(0))?;
            file.seek(SeekFrom::Start(base + data_offset))?;
            let remaining = (total_size - current_pos) as usize;
            if len as u64 > remaining as u64 {
                println!("[WARNING] adding extra bytes to end of RSZ");
                data = vec![0u8; remaining];
            }
            else {
                data = vec![0u8; len];
            }
            file.read_exact(&mut data)?;

            // add some extra bytes in case
            if len as u64 > remaining as u64 {
                data.extend(vec![0; len - remaining]);
            }
        } else {
            file.read_to_end(&mut data)?;
            data.extend(vec![0; 128]);
        };
        Ok(RszHeader {
            version,
            roots,
            extern_slots,
            type_descriptors,
            data,
        })
    }
}

