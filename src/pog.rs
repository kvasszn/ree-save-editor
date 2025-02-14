use crate::file_ext::*;
use crate::rsz::*;
use crate::reerr::{Result, FileParseError::*};
use std::io::{Read, Seek};
use nalgebra_glm::Vec4;
use serde::Serialize;


#[allow(dead_code)]
#[derive(Debug)]
pub struct Pog {
    magic: [u8; 4],
    version: u32,
    hash: u64,
    num_points: u32,
    entry_offset: u64,
    pub points: Vec<PogPoint>,
    pub rszs: Vec<Rsz>,
    // other rsz stuff
}

#[derive(Debug, Serialize)]
pub struct PogPoint {
    //a: (u32, u32, u32, u32),
    a: Vec4,
    b: Vec4, //(u32, u32, u32, u32),
    c: (i32, i32, i32, i32),
}

impl Pog {
    pub fn new<F: Read + Seek>(mut file: F) -> Result<Pog> {
        let magic = file.read_magic()?;
        let ext = core::str::from_utf8(&magic)?;
        if ext != "POG\0" {
            return Err(Box::new(MagicError { 
                real_magic: String::from("POG"), 
                read_magic: ext.to_string()
            }))
        }

        let version = file.read_u32()?;
        let hash = file.read_u64()?;
        let _ = file.read_u32()?;
        let num_points = file.read_u32()?;
        let struct_type_offset = file.read_u64()?;
        let _ = file.read_u64()?;
        let entry_offset = file.read_u64()?;
        let _ = file.read_u64()?;
        println!("{:x}, {:x}, {:x}, {:x}", version, hash, num_points, struct_type_offset);

        let mut rsz_offsets = vec![];
        for _i in 0..2 {
            rsz_offsets.push((file.read_u64()?, file.read_u64()?));
        }
        println!("{rsz_offsets:?}");
        if version == 10 {
            file.seek(std::io::SeekFrom::Start(struct_type_offset))?;
            let struct_type = file.read_u16str()?;
        }

        file.seek(std::io::SeekFrom::Start(entry_offset.into()))?;
        let mut points: Vec<PogPoint> = Vec::with_capacity(num_points as usize);
        for _i in 0..num_points {
            let _idx = file.read_u64()?;
            let _ = file.read_u64()?;
            //points.push(idx);
        }
        if version >= 12 {
            let _ = file.read_u64()?;
            let points_start = file.read_u64()?;
            let _ = file.read_u64()?;
            file.seek(std::io::SeekFrom::Start(points_start.into()))?;
            for _i in 0..num_points {
                //let a = (file.read_u32()?, file.read_u32()?, file.read_u32()?, file.read_u32()?);
                let a = file.read_f32vec4()?;
                let b = file.read_f32vec4()?;
                //let b = (file.read_u32()?, file.read_u32()?, file.read_u32()?, file.read_u32()?);
                //let c = (file.read_u32()?, file.read_u32()?, file.read_u32()?, file.read_u32()?);
                let c = (file.read_i32()?, file.read_i32()?, file.read_i32()?, file.read_i32()?);
                points.push(PogPoint { a, b, c });
            }

        }

        //assert_eq!(file.tell()?, rsz_offset1);
        let mut rszs = vec![];
        for (off, cap) in rsz_offsets {
            if off != 0 {
                rszs.push(Rsz::new(&mut file, off, cap)?);
            }
        }

        Ok(Pog {
            magic,
            version,
            hash,
            num_points,
            entry_offset,
            points,
            rszs,
        })
    }
}

#[derive(Debug, Serialize)]
pub struct PogList {
    paths: Vec<String>,
}

impl PogList {
    pub fn new<F: Read + Seek>(mut file: F) -> Result<PogList> {
        let magic = file.read_magic()?;
        let ext = core::str::from_utf8(&magic)?;
        if ext != "PGL\0" {
            return Err(Box::new(MagicError { 
                real_magic: String::from("PGL"), 
                read_magic: ext.to_string()
            }))
        }
        let _version = file.read_u32()?;
        let count = file.read_u32()?;
        let _ = file.read_u32()?;
        let entry_offset = file.read_u64()?;
        file.seek(std::io::SeekFrom::Start(entry_offset.into()))?;

        let paths = (0..count).map(|_| {
            file.read_u64()
        }).collect::<Result<Vec<_>>>()?;
        let paths = paths.iter().map(|offset| {
            file.seek(std::io::SeekFrom::Start(*offset))?;
            file.read_u16str()
        }).collect::<Result<Vec<_>>>()?;

        Ok(PogList {
            paths
        })
    }
}
