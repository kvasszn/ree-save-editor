use crate::rsz::rszserde::DeRsz;
use crate::rsz::{Rsz, rszserde::StringU16};
use crate::reerr::FileParseError::*;
use util::*;
use std::io::{Read, Seek};
use nalgebra_glm::Vec4;
use serde::Serialize;
use crate::file::{self, DefaultDump, StructRW};


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

impl StructRW for Pog {
    fn read<R: Read + Seek>(reader: &mut R, _ctx: &mut ()) -> file::Result<Self>
            where
                Self: Sized {
        let magic = reader.read_magic()?;
        let ext = core::str::from_utf8(&magic)?;
        if ext != "POG\0" {
            return Err(Box::new(MagicError { 
                real_magic: String::from("POG"), 
                read_magic: ext.to_string()
            }))
        }

        let version = reader.read_u32()?;

        let hash1 = reader.read_u64()?;
        let _num_unks = reader.read_u32()?;
        let num_nodes = reader.read_u32()?;

        let hash2 = reader.read_u64()?;
        let _unk_offset = reader.read_u64()?;
        let nodes_offset = reader.read_u64()?;
        let points_offset = reader.read_u64()?;
        let rsz_start1 = reader.read_u64()?;
        let rsz_end1 = reader.read_u64()?;
        let rsz_start2 = reader.read_u64()?;
        let rsz_end2 = reader.read_u64()?;

        //println!("{}, {:x}, {}, {:x}", version, hash, num_points2, struct_type_offset);
        /*if version == 10 { // there was some string stuff in version 10, which was used in OBT
            reader.seek(std::io::SeekFrom::Start(struct_type_offset))?;
            let struct_type = reader.read_u16str()?;
        }*/
        //println!("{}, {}", rsz_start1, rsz_start2);
        let mut nodes = Vec::with_capacity(num_nodes as usize);
        if nodes_offset != 0 {
            reader.seek(std::io::SeekFrom::Start(nodes_offset.into()))?;
            for _i in 0..num_nodes {
                let x = reader.read_u32()?;
                let y = reader.read_u32()?;
                let offset = reader.read_u64()?;

                if offset == 0 {
                    //reader.seek(std::io::SeekFrom::Current(0x10))?;
                    continue;
                }
                let tmp = reader.tell()?;
                reader.seek(std::io::SeekFrom::Start(offset.into()))?;
                let z = reader.read_u32()?;
                let w = reader.read_f32()?;
                reader.seek(std::io::SeekFrom::Start(tmp.into()))?;
                nodes.push(PogNode {
                    x,
                    y,
                    z,
                    w
                })
            }
        }

        let points = if points_offset != 0 {
            reader.seek(std::io::SeekFrom::Start(points_offset.into()))?;
            let num_points = reader.read_u64()?;
            let data_offset = reader.read_u64()?;
            reader.seek(std::io::SeekFrom::Start(data_offset.into()))?;

            let mut points = Vec::with_capacity(num_points as usize);
            for _i in 0..num_points {
                let point = PogPoint {
                    a: reader.read_f32vec4()?,
                    b: reader.read_f32vec4()?,
                    c: (reader.read_i32()?, reader.read_i32()?, reader.read_i32()?, reader.read_i32()?),
                };
                points.push(point);
            }
            points
        } else {
            vec![]
        };
        let mut rszs = vec![];
        if rsz_start1 != 0 && rsz_end1 > rsz_start1 {
            rszs.push(Rsz::new(reader, rsz_start1, rsz_end1)?);
        }
        if rsz_start2 != 0 && rsz_end2 > rsz_start2 {
            rszs.push(Rsz::new(reader, rsz_start2, rsz_end2)?);
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

impl Serialize for Pog {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
        where
            S: serde::Serializer {
            let mut nodes = Vec::new();
            for rsz in &self.rszs {
                if let Ok(node) = rsz.deserialize_to_dersz() {
                    nodes.push(node);
                }
            }
            #[derive(Serialize)]
            struct Wrapped<'a> {
                points: &'a Vec<PogPoint>,
                graph: &'a Vec<PogNode>,
                nodes: Vec<DeRsz>
            }
            
            let x = Wrapped {
                points: &self.points,
                graph: &self.nodes,
                nodes: nodes
            };
            x.serialize(serializer)
    }

}

impl DefaultDump for Pog {}

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
