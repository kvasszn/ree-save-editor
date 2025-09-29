pub mod rszserde;
pub mod object;
pub mod dump;
use byteorder::LittleEndian;
use byteorder::WriteBytesExt;
use dump::RszDump;
use rszserde::DeRsz;
use rszserde::DeRszType;
use rszserde::RszDeserializerCtx;
use rszserde::RszSerializerCtx;

use crate::file_ext::*;

use std::collections::HashMap;
use std::convert::TryFrom;
use std::fmt::Debug;
use std::io::Cursor;
use std::io::Write;
use std::io::{Read, Seek, SeekFrom};
use crate::reerr::*;

#[derive(Debug, Clone)]
pub struct Extern {
    pub hash: u32,
    pub path: String,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct TypeDescriptor {
    pub hash: u32,
    pub crc: u32,
}

// This is just the Rsz Header parsed, with the raw data of the rsz in bytes
// Can be turned into an intermediate format with DeRsz
#[derive(Debug)]
pub struct Rsz {
    version: u32,
    offset: usize,
    pub roots: Vec<u32>,
    pub extern_slots: HashMap<u32, Extern>,
    pub type_descriptors: Vec<TypeDescriptor>,
    pub data: Vec<u8>,
}

impl Rsz {
    pub fn new<F: Read + Seek>(file: &mut F, base: u64, cap: u64) -> Result<Rsz> {
        file.seek(SeekFrom::Start(base))?;
        let magic = file.read_magic()?;
        let ext = core::str::from_utf8(&magic)?;
        if ext != "RSZ\0" {
            return Err(Box::new(FileParseError::MagicError { 
                real_magic: String::from("RSZ"), 
                read_magic: ext.to_string()
            }))
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

        let type_descriptor_offset = file.read_u64()?;
        let data_offset = file.read_u64()?;
        let extern_offset = file.read_u64()?;

        let roots = (0..root_count)
            .map(|_| file.read_u32())
            .collect::<Result<Vec<_>>>()?;

        file.seek_noop(base + type_descriptor_offset)?;

        let type_descriptors = (0..type_descriptor_count)
            .map(|_| {
                let hash = file.read_u32()?;
                let crc = file.read_u32()?;
                Ok(TypeDescriptor { hash, crc })
            })
            .collect::<Result<Vec<_>>>()?;

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
            .collect::<Result<Vec<_>>>()?;

        let extern_slots = extern_slot_info
            .into_iter()
            .map(|(slot, hash, offset)| {
                file.seek_noop(base + offset)?;
                let path = file.read_u16str()?;
                if !path.ends_with(".user") {
                    return Err(format!("Non-USER slot string").into());
                }
                if hash != type_descriptors
                        .get(usize::try_from(slot)?)
                        .expect("slot out of bound")
                        .hash
                {
                    return Err(format!("slot hash mismatch").into())
                }
                Ok((slot, Extern { hash, path }))
            })
            .collect::<Result<HashMap<u32, Extern>>>()?;
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
        Ok(Rsz {
            version,
            offset: base as usize,
            roots,
            extern_slots,
            type_descriptors,
            data,
        })
    }

    // Deserializes the Rsz data into a DeRsz, consuming the Rsz
    pub fn deserialize_to_dersz(&self) -> Result<DeRsz>
    {
        let mut ctx = RszDeserializerCtx::from(self);
        let x = DeRsz::from_bytes(&mut ctx)?;

        /*let mut leftover = vec![];
        ctx.boxed.read_to_end(&mut leftover)?;
        if !leftover.is_empty() {
            //return Err(format!("Left over data {leftover:?}").into());
            //eprintln!("Left over data {leftover:?}");
        }*/

        Ok(x)
    }

    pub fn to_buf(&self, _start_addr: usize) -> Result<Vec<u8>> {
        let mut buf = vec![];
        buf.write_all(b"RSZ\0")?;
        buf.write_u32::<LittleEndian>(self.version)?;
        buf.write_u32::<LittleEndian>(self.roots.len() as u32)?;
        buf.write_u32::<LittleEndian>(self.type_descriptors.len() as u32)?;
        buf.write_u32::<LittleEndian>(self.extern_slots.len() as u32)?;
        buf.write_all(&[0; 4])?;
        let type_descriptor_offset = buf.len() + self.roots.len() * size_of::<u32>() + 3 * size_of::<u64>();

        let mut extern_offset = type_descriptor_offset + self.type_descriptors.len() * size_of::<u64>();
        if extern_offset % 16 != 0 { extern_offset += 16 - extern_offset % 16; }
        let mut data_offset = extern_offset + self.extern_slots.len() * size_of::<u32>();
        if data_offset % 16 != 0 { data_offset += 16 - data_offset % 16; }

        println!("{:x}, {:x}, {:x}, {:x}", type_descriptor_offset, extern_offset, data_offset, _start_addr);
        buf.write_u64::<LittleEndian>(type_descriptor_offset as u64)?;
        buf.write_u64::<LittleEndian>(data_offset as u64)?;
        buf.write_u64::<LittleEndian>(extern_offset as u64)?;
        for root in &self.roots {
            buf.write_u32::<LittleEndian>(*root)?;
        }
        if buf.len() != type_descriptor_offset {
            //println!("here\n");
            buf.extend(vec![0; buf.len() - type_descriptor_offset as usize]);
        }
        for descriptor in &self.type_descriptors {
            buf.write_u32::<LittleEndian>(descriptor.hash)?;
            buf.write_u32::<LittleEndian>(descriptor.crc)?;
        }
        println!("{}", buf.len());
        if buf.len() != extern_offset {
            buf.extend(vec![0; extern_offset as usize - buf.len()]);
        }
        println!("{}", size_of::<u32>());
        // Figure this out
        //for extern in &self.extern_slots {
        //    buf.write_u32::<LittleEndian>(descriptor.hash)?;
        //    buf.write_u32::<LittleEndian>(descriptor.crc)?;
        //}
        if buf.len() != data_offset {
            println!("{}, {}", buf.len(), data_offset);
            buf.extend(vec![0; data_offset as usize - buf.len()]);
        }
        buf.extend(&self.data);
        Ok(buf)
    }
}

impl From<DeRsz> for Result<Rsz> {
    fn from(dersz: DeRsz) -> Self {
        let type_descriptors = dersz.structs.iter().map(|(hash, _)| {
            let crc = RszDump::get_struct(*hash).unwrap().crc;
            TypeDescriptor{hash: *hash, crc}
        }).collect();
        let mut buffer = Vec::new();
        {
            let mut cursor = Cursor::new(&mut buffer);
            let mut ctx = RszSerializerCtx {
                data: &mut cursor,
                base_addr: 0
            };

            for (hash, val) in dersz.structs {
                if hash == 0 { continue; }
                let s = &val[0]; // dumb stupid
                ctx.base_addr = dersz.offset;// + rszserde::get_writer_length(ctx.data)? as usize;
                s.to_bytes(&mut ctx)?;
            }
        }
        Ok(Rsz{ version: 0x10, offset: dersz.offset, roots: dersz.roots, extern_slots: HashMap::new(), type_descriptors, data: buffer })

    }

}
