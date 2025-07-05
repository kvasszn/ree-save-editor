#![allow(dead_code)]
use byteorder::LittleEndian;
use byteorder::WriteBytesExt;

use crate::dersz::*;

use crate::file_ext::*;
use std::collections::HashMap;
use std::collections::HashSet;
use std::convert::TryFrom;
use std::fmt::Debug;
use std::fs::File;
use std::io::Write;
use std::io::{Cursor, Read, Seek, SeekFrom};
use crate::reerr::*;

#[derive(Debug, Clone)]
pub struct Extern {
    pub hash: u32,
    pub path: String,
}

#[derive(Debug, PartialEq, Eq)]
pub struct TypeDescriptor {
    pub hash: u32,
    pub crc: u32,
}

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


    pub fn deserialize(&self) -> Result<DeRsz> {
        let mut cursor = Cursor::new(&self.data);
        let mut structs: Vec<RszValue> = Vec::new();
        let mut extern_idxs: HashSet<u32> = HashSet::new();
        for (i, &TypeDescriptor { hash, crc }) in self.type_descriptors.iter().enumerate() {
            if let Some(slot_extern) = self.extern_slots.get(&u32::try_from(i)?) {
                let x = RszType::Extern(slot_extern.path.clone());
                let struct_type = match RszDump::rsz_map().get(&hash) {
                    Some(x) => x,
                    None => return Err(Box::new(FileParseError::InvalidRszTypeHash(hash)))
                };
                let x = struct_type.to_value(vec![x]);
                structs.push(x);
                extern_idxs.insert(i as u32);
                continue;
            } else {
                // check for object index and return that too
                let something = RszDump::parse_struct(&mut cursor, TypeDescriptor{hash, crc})?;
                //println!("{something:?}");
                structs.push(something);
            }
        }
        //println!("{structs:#?}");
        let mut roots = Vec::new();
        for root in &self.roots {
            match structs.get(*root as usize) {
                None => eprintln!("Could not find root {}", root),
                Some(obj) => {
                    roots.push(obj.clone());
                }
            }
        }
        //let results = self.roots.iter().map(|root| {
        //    structs.get(*root as usize).unwrap()
        //}).collect::<Vec<RszStruct<RszType>>>();
        let mut leftover = vec![];
        cursor.read_to_end(&mut leftover)?;
        if !leftover.is_empty() {
            //return Err(format!("Left over data {leftover:?}").into());
            //eprintln!("Left over data {leftover:?}");
        }

        Ok(DeRsz {
            offset: self.offset,
            roots: self.roots.clone(),
            structs,
            extern_idxs,
        })
    }

    pub fn save_from_json(&self, file: &str) -> Result<()> {
        let data = self.to_buf(0)?;
        let mut file = File::create(file)?;
        file.write_all(&data)?;
        Ok(())
    }

    pub fn from_json_file(file: &str) -> Result<Rsz> {
        let data = std::fs::read_to_string(file)?;
        let json_data: serde_json::Value = serde_json::from_str(&data).unwrap();
        let rsz: Rsz = Rsz::from_json(&json_data)?; 
        Ok(rsz)
    }

    pub fn from_json(value: &serde_json::Value) -> Result<Rsz> {
        assert!(value.is_array(), "Value should be array");
        //let mut root_structs = vec![];
        let version = match value.get("version") {
            None => 0x10,
            Some(value) => value.as_u64().unwrap_or_else(|| 0x10),
        } as u32;
        let base_addr = match value.get("offset") {
            None => 0x0,
            Some(value) => value.as_u64().unwrap_or_else(|| 0x0),
        } as usize;
        let mut objects: Vec<RszValue> = vec![];
        let mut type_descriptors: Vec<TypeDescriptor> = vec![];
        type_descriptors.push(TypeDescriptor{hash: 0, crc: 0});
        let mut roots: Vec<u32> = vec![];
        for root in value.as_array().unwrap() {
            let r#type = root.get("type");
            match r#type {
                None => return Err(format!("Missing type information").into()),
                Some(type_info) => {
                    println!("{}", type_info.as_str().unwrap());
                    let type_name = type_info.as_str().unwrap().to_string();
                    let hash = RszDump::name_map().get(&type_name).unwrap();
                    let rsz_type = RszDump::rsz_map().get(hash).unwrap();
                    println!("Found Root with type {:?}", rsz_type);
                    let rsz_json = root.get("rsz");
                    match rsz_json {
                        None => return Err(format!("Missing rsz json").into()),
                        Some(rsz_json) => {
                            let x = RszDump::parse_struct_from_json(rsz_json, *hash, &mut objects)?;
                            objects.push(x);
                            roots.push(objects.len() as u32);
                        }
                    }

                }
            }
        }
        let mut data = vec![];
        for object in &objects {
            let hash = object.hash().unwrap();
            type_descriptors.push(TypeDescriptor{hash: *hash, crc: object.crc});
            data.extend(object.to_buffer(base_addr + data.len())?);
        }
        //println!("{:#?}", objects);
        Ok(Rsz {
            version,
            offset: base_addr,
            roots,
            extern_slots: HashMap::new(), // FIGURE THIS OUT IG, MAYBE JUST IF ERROR TRY EXTERN
                                          // THINGY, OR MAKE IT SO THAT IF IT'S ORGINALLY EXTERN,
                                          // HAS TO STAY EXTERN
            type_descriptors,
            data,
        })
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


/*
 * Deserializer/Serializer
 */

pub trait ReadSeek: Read + Seek {}
pub struct RszDeserializerCtx {
    data: Box<dyn ReadSeek>
}

pub trait DeRszType<'a> where Self: Sized {
    fn from_bytes(ctx: &'a mut RszDeserializerCtx) -> Result<Self>;
}

impl<'a, T> DeRszType<'a> for Vec<T>
where
    T: for<'b> DeRszType<'b>, 
{
    fn from_bytes(ctx: &'a mut RszDeserializerCtx) -> Result<Self> {
        let len = u32::from_bytes(ctx)? as usize;
        let mut items = Vec::with_capacity(len);
        for _ in 0..len {
            let item = T::from_bytes(ctx)?;
            items.push(item);
        }
        Ok(items)
    }
}

impl<'a, T, const N: usize> DeRszType<'a> for [T; N]
where
    T: for<'b> DeRszType<'b> + Debug,
{
    fn from_bytes(ctx: &'a mut RszDeserializerCtx) -> Result<Self> {
        let mut vec = Vec::with_capacity(N);
        for _ in 0..N {
            vec.push(T::from_bytes(ctx)?);
        }
        
        Ok(vec.try_into().unwrap())
    }
}


macro_rules! derive_dersztype{
    ($rsz_type:ty) => {
        impl<'a> DeRszType<'a> for $rsz_type {
            fn from_bytes(ctx: &'a mut RszDeserializerCtx) -> Result<$rsz_type> {
                let mut buf = [0; size_of::<$rsz_type>()];
                ctx.data.read_exact(&mut buf)?;
                Ok(<$rsz_type>::from_le_bytes(buf))
            }
        }
    };
    ($rsz_type:ty, $func:ident) => {
        impl<'a> DeRszType<'a> for $rsz_type {
            fn from_bytes(ctx: &'a mut RszDeserializerCtx) -> Result<$rsz_type> {
                ctx.data.$func()
            }
        }
    };
    ($rsz_type:ty, $func:expr) => {
        impl<'a> DeRszType<'a> for $rsz_type {
            fn from_bytes(ctx: &'a mut RszDeserializerCtx) -> Result<$rsz_type> {
                $func(ctx)
            }
        }
    };

}


derive_dersztype!(u8);
derive_dersztype!(u16);
derive_dersztype!(u32);
derive_dersztype!(u64);
derive_dersztype!(i8);
derive_dersztype!(i16);
derive_dersztype!(i32);
derive_dersztype!(i64);

pub type F8 = u8; // scuffed
pub type F16 = u16; // scuffed
derive_dersztype!(f32);
derive_dersztype!(f64);

derive_dersztype!(bool, |ctx: &'a mut RszDeserializerCtx| -> Result<bool> {
    let v = ctx.data.read_u8()?;
    if v > 1 {
        return Err(Box::new(FileParseError::InvalidBool(v)))
    }
    Ok(v != 0)
});

derive_dersztype!(uuid::Uuid, |ctx: &'a mut RszDeserializerCtx| -> Result<Self> {
    let mut buf = [0; 16];
    for i in 0..16 {
        buf[i] = ctx.data.read_u8()?;
    }
    Ok(uuid::Uuid::from_bytes_le(buf))
});

derive_dersztype!(String, |ctx: &'a mut RszDeserializerCtx| -> Result<Self> {
    let mut s = vec![];
    let n = ctx.data.read_u32()?;
    for _i in 0..n {
        let c = ctx.data.read_u16()?;
        s.push(c);
    }
    Ok(String::from_utf16(&s)?)
});


use rsz_macros::DeRszType;

#[derive(DeRszType)]
pub struct UInt2(u32, u32);
#[derive(DeRszType)]
pub struct UInt3(u32, u32, u32);
#[derive(DeRszType)]
pub struct UInt4(u32, u32, u32, u32);

#[derive(DeRszType)]
pub struct Int2(i32, i32);
#[derive(DeRszType)]
pub struct Int3(i32, i32, i32);
#[derive(DeRszType)]
pub struct Int4(i32, i32, i32, i32);

#[derive(DeRszType)]
pub struct Color(u8, u8, u8, u8);

#[derive(DeRszType)]
#[rsz(align = 16)]
pub struct Vec2(f32, f32);

#[derive(DeRszType)]
#[rsz(align = 16)]
pub struct Vec3(f32, f32, f32);
#[derive(DeRszType)]
pub struct Vec4(f32, f32, f32, f32);

#[derive(DeRszType)]
pub struct Quaternion(f32, f32, f32, f32);
#[derive(DeRszType)]
pub struct Sphere(f32, f32, f32, f32);
#[derive(DeRszType)]
pub struct Position(f32, f32, f32);

#[derive(DeRszType)]
pub struct Float2(f32, f32);
#[derive(DeRszType)]
pub struct Float3(f32, f32, f32);
#[derive(DeRszType)]
pub struct Float4(f32, f32, f32, f32);

#[derive(DeRszType)]
pub struct Mat4x4([f32; 16]);

