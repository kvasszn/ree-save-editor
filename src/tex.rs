use libdeflater::TileStream;

use crate::bitfield::BitField;
use crate::reerr::FileParseError::{self, MagicError};
use crate::file_ext::*;
use crate::compression::{
    Bc1Unorm, Bc3Unorm, Bc4Unorm, Bc5Unorm, Bc7Unorm, CompressionType, R8G8B8A8Unorm, R8G8Unorm, R8Unorm, TexCodec
};

use std::error::Error;
use std::fmt;
use std::str;
use std::result::Result;
use std::io::{Cursor, Read, Seek, SeekFrom};

pub struct RGBAImage {
    pub data: Vec<u8>,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone)]
pub struct Tex {
    width: u32,
    height: u32,
    format: u32,
    layout: u32,
    tex_infos: Vec<TexInfo>,
    textures: Vec<Vec<u8>>,
    _tex_count: usize,
    mip_count: usize,
}

#[derive(Debug, Clone)]
struct TexInfo {
    offset: u64,
    compressed_size : u32,
    len: u32,
}

impl fmt::Display for TexInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "\tOffset: {:#010x}, compressed_size: {:#010x}, Len: {:#010x}",
            self.offset, self.compressed_size, self.len
        )
    }
}

impl Tex {
    pub fn new<F: Read + Seek>(mut file: F) -> std::result::Result<Tex, Box<dyn Error>> {
        let magic = file.read_magic()?;
        let ext = str::from_utf8(&magic)?;
        if ext != "TEX\0" {
            return Err(Box::new(MagicError { 
                real_magic: String::from("TEX"), 
                read_magic: ext.to_string()
            }))
        }
        let version = file.read_u32()?;

        let width = file.read_u16()?;
        let height = file.read_u16()?;
        let depth = file.read_u16()?;

        let counts = file.read_u16()?;
        let (tex_count, mipmap_count) = counts.bit_split((12, 4));

        let format = file.read_u32()?;
        let layout = file.read_u32()?;

        let _cubemap = file.read_u32()?;
        let _unkn1 = file.read_u8()?; // These are some weird bit flags
        let _unkn2 = file.read_u8()?;
        let _null1 = file.read_u16()?;

        // no idea where this is used, it's just zero?
        if version > 27 && version != 190820018 {
            let _swizzle_height_depth = file.read_u8()?;
            let _swizzle_width = file.read_u8()?;
            let _null2 = file.read_u16()?;
            let _seven = file.read_u16()?;
            let _one = file.read_u16()?;
        }

        /*println!("magic: {:?}", magic);
        println!("version: {:?}", version);
        println!("dims: {width}, {height}, {depth}");
        println!("format: {format:#010x}, layout: {layout:#010x}");
        println!("counts: {counts},texs: {tex_count}, mipmaps: {mipmap_count}");
        println!("{_unkn}, {_unkn1:08b}, {_unkn2:08b}, {_null1}");*/

        //println!("{_swizzle_height_depth:?}, {_swizzle_width:?}, {_null2:?}, {_seven:?}, {_one:?}");

        let mut tex_infos = Vec::new();

        let mut decompressed_size = 0;
        for _i in 0..tex_count {
            for _j in 0..mipmap_count {
                let offset = file.read_u64()?;
                let compressed_size = file.read_u32()?;
                let len = file.read_u32()?;
                decompressed_size += len * depth as u32;
                tex_infos.push(TexInfo { offset, compressed_size, len });
            }
        }

        //println!("{tex_infos:#?}");
        #[derive(Debug)]
        struct GDefSection {
            compressed_size: u32,
            offset: u32,
        }

        let mut total_size = 0;
        let gdef_sections = if version == 240701001{
            let sections = (0..mipmap_count * tex_count)
                .into_iter()
                .map(|_| {
                    let compressed_size = file.read_u32()?;
                    let offset = file.read_u32()?;
                    total_size += compressed_size;
                    Ok(GDefSection {
                        compressed_size,
                        offset,
                    })
                })
                .collect::<Result<Vec<_>, Box<dyn Error>>>()?;
            Some(sections)
        } else {
            None
        };

        //println!("{gdef_sections:?}");
        let base = tex_infos[0].offset + mipmap_count as u64 * tex_count as u64 * 8;
        let mut bytes_read = 0;
        let textures = tex_infos
            .iter()
            .enumerate()
            .map(|(i, tex_info)| {
                let in_size = match &gdef_sections {
                    Some(sections) => {
                        let in_size = sections[i].compressed_size;
                        file.seek_noop(base + sections[i].offset as u64).expect("Invalid file base");
                        in_size
                    },
                    None => tex_info.len
                };
                //println!("in_size {}, out_size {}", in_size, tex_info.len);
                let in_buf = file.read_u8_n(in_size as usize).unwrap();
                if tex_info.len == in_size {
                    bytes_read += in_size;
                    return Ok(in_buf);
                }

                let mut in_data = Cursor::new(&in_buf);
                let _header = TileStream::from(&mut in_data).unwrap();

                let out_size = u32::max(_header.get_uncompressed_size() as u32, tex_info.len * depth as u32);
                let mut out_buf: Vec<u8> = Vec::new();
                out_buf.resize(out_size as usize, 0);
                if in_size > out_size {
                    return Err(Box::new(FileParseError::TexReadError{source: format!("in_size {in_size} larger than out_size {out_size}")}))
                }
                match libdeflater::GDeflateDecompressor::gdeflate_decompress(&in_buf, &mut out_buf)
                {
                    Ok(x) => {
                        bytes_read += x as u32;
                        Ok(out_buf)
                    }
                    Err(e) => Err(Box::new(FileParseError::GDeflateError { source: e.to_string()})),
                }
            })
            .collect::<Result<Vec<_>, _>>()?;

        println!("read {bytes_read}, decompressed {decompressed_size}");
        if bytes_read > decompressed_size {
            return Err(Box::new(FileParseError::TexReadError{source: String::from("Decompressed not same as btyes read")}))
        }

        let tex = Tex {
            width: width as u32,
            height: height as u32,
            format,
            layout,
            tex_infos,
            textures,
            mip_count: mipmap_count as usize,
            _tex_count: tex_count as usize
        };
        Ok(tex)
    }

    pub fn to_rgba(&self, tex_idx: usize, mip_idx: usize) -> Result<RGBAImage, Box<dyn Error>> {
        let texture = &self.textures[tex_idx * self.mip_count + mip_idx];
        let tex_info = &self.tex_infos[tex_idx * self.mip_count + mip_idx];
        //println!("{tex_info:?}");
        let swizzle = "rgba";

        let bpps = CompressionType::get_bpps(self.format);
        if bpps == 0 {
                return Err(Box::new(FileParseError::Unsupported { source: format!("unsupported format {:02X}", self.format) }))
        }
        let texel_size = if bpps == 4 || bpps == 8 {
            4
        } else {
            bpps / 8
        };
        let bit_amount = self.width * bpps;
        //println!("{bit_amount}");
        let pad = 8 - if bpps < 8 { bit_amount } else { 0 }; 
        //println!("{pad}");
        let read_len = (bit_amount / 8 + pad) * texel_size - pad * 4;
        let (width, height) = (self.width as u32, self.height as u32);
        //println!("w{}, h{}, pad:{pad}", width, height);
        //println!("readlen: {read_len}");

        let mut data2 = vec![];//0; (self.width as usize) * (self.height as usize) * 4 ];
        let mut t = Cursor::new(&texture);
        for _i in 0..tex_info.len / tex_info.compressed_size {
            let x = t.read_u8_n(read_len as usize)?;
            t.seek(SeekFrom::Current((tex_info.compressed_size as u32 - read_len as u32) as i64))?;
            data2.extend(x);
        }
        let texture = data2;

        let mut data = vec![0; (width * height * 4) as usize];
        let writer = |x: usize, y: usize, v: [u8; 4]| {
            let i = (x + y * (width as usize)) * 4;
            let dest = &mut data[i..][..4];
            for (dest, &code) in dest.iter_mut().zip(swizzle.as_bytes()) {
                *dest = match code {
                    b'r' | b'x' => v[0],
                    b'g' | b'y' => v[1],
                    b'b' | b'z' => v[2],
                    b'a' | b'w' => v[3],
                    b'0' => 0,
                    b'1' => 255,
                    b'n' => 0,
                    _ => 0,
                }
            }

            if let Some(n) = swizzle.as_bytes().iter().position(|&c| c == b'n') {
                let mut l: f32 = dest
                    .iter()
                    .map(|&x| {
                        let x = x as f32 / 255.0 * 2.0 - 1.0;
                        x * x
                    })
                    .sum();
                if l > 1.0 {
                    l = 1.0
                }
                let z = (((1.0 - l).sqrt() + 1.0) / 2.0 * 255.0).round() as u8;
                dest[n] = z;
            }
        };

        // need to add 0xA and 0x5F
        match self.format {
            0x1C | 0x1D => R8G8B8A8Unorm::decode_image( &texture, width as usize, height as usize, self.layout, writer,),
            0x31 => R8G8Unorm::decode_image( &texture, width as usize, height as usize, self.layout, writer,),
            0x3D => R8Unorm::decode_image( &texture, width as usize, height as usize, self.layout, writer,),
            0x47 | 0x48 => Bc1Unorm::decode_image( &texture, width as usize, height as usize, self.layout, writer,),
            0x4D | 0x4E => Bc3Unorm::decode_image( &texture, width as usize, height as usize, self.layout, writer,),
            0x50 => Bc4Unorm::decode_image( &texture, width as usize, height as usize, self.layout, writer,),
            0x53 => Bc5Unorm::decode_image( &texture, width as usize, height as usize, self.layout, writer,),
            0x62 | 0x63  => Bc7Unorm::decode_image( &texture, width as usize, height as usize, self.layout, writer,),
            x => {
                eprintln!("unsupported format {:08X}", x);
                return Err(Box::new(FileParseError::Unsupported { source: format!("unsupported format {x:08X}") }))
            }
        };

        Ok(RGBAImage {
            data,
            width: width as u32,
            height: height as u32,
        })
    }
}
