use crate::file_ext::*;
use crate::rsz::rszserde::StringU16;
use crate::rsz::*;
use crate::reerr::{Result, FileParseError::*};
use std::io::{Read, Seek};
use file_macros::StructRW;
use nalgebra_glm::Vec4;
use serde::Serialize;
use crate::file::{DefaultDump, StructRW};


#[allow(dead_code)]
#[derive(Debug)]
pub struct Pog {
    version: u32,
    hash1: u64,
    hash2: u64,
    pub points: Vec<PogPoint>,
    pub nodes: Vec<PogNode>,
    pub rszs: Vec<Rsz>,
    // other rsz stuff
}

#[derive(Debug, Serialize)]
pub struct PogPoint {
    a: Vec4,
    b: Vec4,
    c: (i32, i32, i32, i32),
}

#[derive(Debug, Serialize)]
pub struct PogNode {
    x: u32,
    y: u32,
    z: u32,
    w: f32,
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

        let hash1 = file.read_u64()?;
        let _num_unks = file.read_u32()?;
        let num_nodes = file.read_u32()?;

        let hash2 = file.read_u64()?;
        let _unk_offset = file.read_u64()?;
        let nodes_offset = file.read_u64()?;
        let points_offset = file.read_u64()?;
        let rsz_start1 = file.read_u64()?;
        let rsz_end1 = file.read_u64()?;
        let rsz_start2 = file.read_u64()?;
        let rsz_end2 = file.read_u64()?;

        //println!("{}, {:x}, {}, {:x}", version, hash, num_points2, struct_type_offset);
        /*if version == 10 { // there was some string stuff in version 10, which was used in OBT
            file.seek(std::io::SeekFrom::Start(struct_type_offset))?;
            let struct_type = file.read_u16str()?;
        }*/
        //println!("{}, {}", rsz_start1, rsz_start2);
        let mut nodes = Vec::with_capacity(num_nodes as usize);
        if nodes_offset != 0 {
            file.seek(std::io::SeekFrom::Start(nodes_offset.into()))?;
            for _i in 0..num_nodes {
                let x = file.read_u32()?;
                let y = file.read_u32()?;
                let offset = file.read_u64()?;

                if offset == 0 {
                    //file.seek(std::io::SeekFrom::Current(0x10))?;
                    continue;
                }
                let tmp = file.tell()?;
                file.seek(std::io::SeekFrom::Start(offset.into()))?;
                let z = file.read_u32()?;
                let w = file.read_f32()?;
                file.seek(std::io::SeekFrom::Start(tmp.into()))?;
                nodes.push(PogNode {
                    x,
                    y,
                    z,
                    w
                })
            }
        }

        let points = if points_offset != 0 {
            file.seek(std::io::SeekFrom::Start(points_offset.into()))?;
            let num_points = file.read_u64()?;
            let data_offset = file.read_u64()?;
            file.seek(std::io::SeekFrom::Start(data_offset.into()))?;

            let mut points = Vec::with_capacity(num_points as usize);
            for _i in 0..num_points {
                let point = PogPoint {
                    a: file.read_f32vec4()?,
                    b: file.read_f32vec4()?,
                    c: (file.read_i32()?, file.read_i32()?, file.read_i32()?, file.read_i32()?),
                };
                points.push(point);
            }
            points
        } else {
            vec![]
        };
        let mut rszs = vec![];
        if rsz_start1 != 0 && rsz_end1 > rsz_start1 {
            rszs.push(Rsz::new(&mut file, rsz_start1, rsz_end1)?);
        }
        if rsz_start2 != 0 && rsz_end2 > rsz_start2 {
            rszs.push(Rsz::new(&mut file, rsz_start2, rsz_end2)?);
        }


        Ok(Pog {
            version,
            hash1,
            hash2,
            points,
            nodes,
            rszs,
        })
    }
}

impl DefaultDump for PogList {}

#[derive(Debug, file_macros::StructRW, Serialize)]
pub struct PogList {
    #[magic = b"PGL\0"]
    magic: [u8; 4],
    version: u32,
    count: u32,
    unk: u32,
    paths_offset: u64,
    #[varlist(ty = u64, count = count, offset = paths_offset)]
    path_offsets: Vec<u64>,
    #[varlist(ty = StringU16, count = count, offsets = path_offsets)]
    paths: Vec<StringU16>
}
