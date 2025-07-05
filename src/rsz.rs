#![allow(dead_code)]
use rsz_macros::DeRszType;
use byteorder::LittleEndian;
use byteorder::WriteBytesExt;
use indexmap::IndexMap;
use serde::de;
use serde::Serialize;
use serde_json::Value;
use syn::token::Struct;

use crate::dersz::*;

use crate::file_ext::*;
use crate::reerr;
use std::any::Any;
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

#[derive(Debug, PartialEq, Eq, Clone)]
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


    pub fn deserialize<'a>(&'a self) -> Result<DeRsz>
    {
        let mut cursor = Cursor::new(&self.data);
        let boxed: Box<dyn ReadSeek + 'a> = Box::new(cursor.clone());
        let mut ctx = RszDeserializerCtx {  
            data: boxed,
            objects: Vec::new(),
            cur_hash: 0,
            type_descriptors: self.type_descriptors.clone(),
            roots: self.roots.clone(),
            extern_slots: self.extern_slots.clone()
        };
        let x = DeRsz::from_bytes(&mut ctx)?;

        let mut leftover = vec![];
        cursor.read_to_end(&mut leftover)?;
        if !leftover.is_empty() {
            //return Err(format!("Left over data {leftover:?}").into());
            //eprintln!("Left over data {leftover:?}");
        }

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


/*
 * Deserializer/Serializer
 */

pub trait ReadSeek: Read + Seek {}
impl<'a, T: Read + Seek> ReadSeek for T {}
pub struct RszDeserializerCtx<'a> {
    data: Box<dyn ReadSeek + 'a>,
    objects: Vec<u32>,
    type_descriptors: Vec<TypeDescriptor>,
    roots: Vec<u32>,
    cur_hash: u32,
    extern_slots: HashMap<u32, Extern>
}


pub type RszFieldsValue = (u32, Vec<Box<dyn DeRszInstance>>);

pub struct RszJsonSerializerCtx<'a> {
    root: Option<u32>,
    field: Option<&'a RszField>,
    objects: &'a Vec<RszFieldsValue>,
}

pub trait DeRszType<'a> {
    fn from_bytes(ctx: &'a mut RszDeserializerCtx) -> Result<Self> where Self: Sized;
}

pub trait DeRszInstance: Debug {
    fn as_any(&self) -> &dyn Any;
    fn to_json(&self, ctx: &RszJsonSerializerCtx) -> serde_json::Value;
}

/*
 * Default Implementations
 */

impl DeRszInstance for Vec<Box<dyn DeRszInstance>> {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn to_json(&self, ctx: &RszJsonSerializerCtx) -> serde_json::Value {
        serde_json::Value::Array(self.iter().map(|item| {
            let new_ctx = RszJsonSerializerCtx {
                root: None,
                field: ctx.field,
                objects: ctx.objects,
            };
            item.to_json(&new_ctx)
        }).collect())
    }
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

impl<T: 'static + DeRszInstance + Debug, const N: usize> DeRszInstance for [T; N] {
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn to_json(&self, ctx: &RszJsonSerializerCtx) -> serde_json::Value {
        let values = self.iter().map(|x| {
            x.to_json(ctx)
        }).collect::<Vec<serde_json::Value>>();
        Value::Array(values)
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

#[derive(Debug, Serialize)]
pub struct Object {
    hash: u32,
    idx: u32,
}

impl DeRszInstance for Object {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn to_json(&self, ctx: &RszJsonSerializerCtx) -> serde_json::Value {
        let (hash, field_values) = &ctx.objects[ctx.root.unwrap_or(self.idx) as usize];
        let struct_desc = match RszDump::rsz_map().get(&hash) {
            Some(struct_desc) => struct_desc,
            None => return serde_json::Value::Null
        };
        let values = struct_desc.fields.iter().enumerate().map(|(i, field)| {
            let obj = &field_values[i];
            let new_ctx = RszJsonSerializerCtx {
                root: None,
                field: Some(&field),
                objects: ctx.objects
            };
            (field.name.clone(), obj.to_json(&new_ctx))
        }).collect::<IndexMap<String, serde_json::Value>>();
        serde_json::to_value(values).unwrap()

    }
}

impl<'a> DeRszType<'a> for Object {
    fn from_bytes(ctx: &'a mut RszDeserializerCtx) -> Result<Self> {
        Ok(Self {
            hash: ctx.cur_hash,
            idx: ctx.data.read_u32()?
        })
    }
}


/*
 *
 *
 */

type DeserializerFn = fn(&mut RszDeserializerCtx) -> Result<Box<dyn DeRszInstance>>;

struct DeRszRegistry {
    deserializers: HashMap<&'static str, DeserializerFn>,
}

impl DeRszRegistry {
    fn new() -> Self {
        Self {
            deserializers: HashMap::new(),
        }
    }

    fn register<T>(&mut self, type_id: &'static str)
    where
        T: for<'a> DeRszType<'a> + DeRszInstance + 'static
    {
        self.deserializers.insert(type_id, |ctx| {
            Ok(Box::new(T::from_bytes(ctx)?))
        });
    }
}

pub struct DeRsz {
    pub offset: usize,
    pub roots: Vec<u32>,
    pub structs: Vec<RszFieldsValue>,
    pub extern_idxs: HashSet<u32>,
}

impl Serialize for DeRsz {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
        where
            S: serde::Serializer {
        #[derive(Serialize)]
        struct Wrapped {
            offset: usize,
            rsz: Vec<serde_json::Value>
        }
        let mut wrapped = Wrapped{offset: self.offset, rsz: Vec::new()};
        for root in &self.roots {
            let ctx = RszJsonSerializerCtx {root: Some(*root), field: None, objects: &self.structs};
            let data = ctx.objects[*root as usize].1.to_json(&ctx);
            wrapped.rsz.push(data);
        }
        println!("{}", serde_json::to_string_pretty(&wrapped).unwrap());
        wrapped.serialize(serializer)
    }
}


impl<'a> DeRszType<'a> for DeRsz {
    fn from_bytes(ctx: &'a mut RszDeserializerCtx) -> Result<Self> {
        let mut structs: Vec<RszFieldsValue> = Vec::new();
        let mut extern_idxs: Vec<u32> = Vec::new();

        let mut deserializers = DeRszRegistry::new();
        deserializers.register::<u8>("U8");
        deserializers.register::<u16>("U16");
        deserializers.register::<u32>("U32");
        deserializers.register::<u64>("U64");
        deserializers.register::<i8>("S8");
        deserializers.register::<i16>("S16");
        deserializers.register::<i32>("S32");
        deserializers.register::<i64>("S64");
        deserializers.register::<u8>("F8");
        deserializers.register::<u16>("F16");
        deserializers.register::<f32>("F32");
        deserializers.register::<f64>("F64");
        deserializers.register::<String>("String");
        deserializers.register::<String>("Resource");
        deserializers.register::<bool>("Bool");
        deserializers.register::<UInt2>("Uint2");
        deserializers.register::<UInt3>("Uint3");
        deserializers.register::<UInt4>("Uint4");
        deserializers.register::<Float2>("Float2");
        deserializers.register::<Float3>("Float3");
        deserializers.register::<Float4>("Float4");
        deserializers.register::<Vec2>("Vec2");
        deserializers.register::<Vec3>("Vec3");
        deserializers.register::<Vec4>("Vec4");
        deserializers.register::<Quaternion>("Quaternion");
        deserializers.register::<Sphere>("Sphere");
        deserializers.register::<Position>("Position");
        deserializers.register::<Color>("Color");
        deserializers.register::<Mat4x4>("Mat4");
        deserializers.register::<Guid>("Guid");
        deserializers.register::<Object>("Object");
        deserializers.register::<Object>("UserData");
        //deserializers.register::<Range>("Range");
        //deserializers.register::<RangeI>("RangeI");
        for (i, &TypeDescriptor { hash, crc }) in ctx.type_descriptors.clone().iter().enumerate() {
            if let Some(slot_extern) = ctx.extern_slots.get(&u32::try_from(i)?) {
                continue;
            } else {
                println!("{hash}");
                let struct_type = match RszDump::rsz_map().get(&hash) {
                    Some(x) => x,
                    None => return Err(Box::new(reerr::FileParseError::InvalidRszTypeHash(hash)))
                };
                let mut field_values: RszFieldsValue = (hash, Vec::new());
                for field in &struct_type.fields {
                    ctx.cur_hash = *field.get_type_hash().unwrap();
                    if field.array {
                        ctx.data.seek_align_up(4)?;
                        let len = ctx.data.read_u32()?;
                        ctx.data.seek_align_up(field.align.into())?;
                        let dersz_fn = deserializers.deserializers.get(field.r#type.as_str()).unwrap();
                        let mut vals = Vec::new();
                        for _ in 0..len {
                            let x: Box<dyn DeRszInstance> = dersz_fn(ctx)?;
                            vals.push(x);
                        }
                        field_values.1.push(Box::new(vals));

                    } else {
                        ctx.data.seek_align_up(field.align.into())?;
                        let dersz_fn = deserializers.deserializers.get(field.r#type.as_str()).expect(&format!("Deserializer for {} is not set", field.r#type));
                        let x: Box<dyn DeRszInstance> = dersz_fn(ctx)?;
                        field_values.1.push(x);
                    }
                }
                println!("{:?}", field_values);
                structs.push(field_values);
            }
        }

        Ok(Self { offset: 0, roots: ctx.roots.clone(), structs, extern_idxs: HashSet::new() })
    }
}


/*
 * types
 */

#[derive(Debug, Serialize, DeRszType)]
pub struct UInt2(u32, u32);
#[derive(Debug, Serialize, DeRszType)]
pub struct UInt3(u32, u32, u32);
#[derive(Debug, Serialize, DeRszType)]
pub struct UInt4(u32, u32, u32, u32);

#[derive(Debug, Serialize, DeRszType)]
pub struct Int2(i32, i32);
#[derive(Debug, Serialize, DeRszType)]
pub struct Int3(i32, i32, i32);
#[derive(Debug, Serialize, DeRszType)]
pub struct Int4(i32, i32, i32, i32);

#[derive(Debug, Serialize, DeRszType)]
pub struct Color(u8, u8, u8, u8);

#[derive(Debug, Serialize, DeRszType)]
#[rsz(align = 16)]
pub struct Vec2(f32, f32);

#[derive(Debug, Serialize, DeRszType)]
#[rsz(align = 16)]
pub struct Vec3(f32, f32, f32);
#[derive(Debug, Serialize, DeRszType)]
pub struct Vec4(f32, f32, f32, f32);

#[derive(Debug, Serialize, DeRszType)]
pub struct Quaternion(f32, f32, f32, f32);
#[derive(Debug, Serialize, DeRszType)]
pub struct Sphere(f32, f32, f32, f32);
#[derive(Debug, Serialize, DeRszType)]
pub struct Position(f32, f32, f32);

#[derive(Debug, Serialize, DeRszType)]
pub struct Float2(f32, f32);
#[derive(Debug, Serialize, DeRszType)]
pub struct Float3(f32, f32, f32);
#[derive(Debug, Serialize, DeRszType)]
pub struct Float4(f32, f32, f32, f32);

#[derive(Debug, Serialize, DeRszType)]
pub struct Mat4x4([f32; 16]);

#[derive(Debug, DeRszType)]
pub struct Guid([u8; 16]);

impl Serialize for Guid {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
        where
            S: serde::Serializer {
            serializer.serialize_str(&uuid::Uuid::from_bytes(self.0).to_string())
    }
}

macro_rules! impl_dersz_instance {
    ( $t:ty ) => {
        #[allow(unused)]
        impl DeRszInstance for $t {
            fn as_any(&self) -> &dyn Any {
                self
            }
            fn to_json(&self, ctx: &RszJsonSerializerCtx) -> serde_json::Value {
                serde_json::json!(self)
            }
        }
    };
}


macro_rules! derive_dersztype_full{
    ($rsz_type:ty) => {
        impl_dersz_instance!( $rsz_type );
        impl<'a> DeRszType<'a> for $rsz_type {
            fn from_bytes(ctx: &'a mut RszDeserializerCtx) -> Result<$rsz_type> {
                let mut buf = [0; size_of::<$rsz_type>()];
                ctx.data.read_exact(&mut buf)?;
                Ok(<$rsz_type>::from_le_bytes(buf))
            }
        }
    };
    ($rsz_type:ty, $func:expr) => {
        impl_dersz_instance!( $rsz_type );
        impl<'a> DeRszType<'a> for $rsz_type {
            fn from_bytes(ctx: &'a mut RszDeserializerCtx) -> Result<$rsz_type> {
                $func(ctx)
            }
        }
    };
}

pub fn capitalize(s: &str) -> String {
    let c: String = s.chars().map(|c| c.to_uppercase().to_string()).collect::<String>();
    c
}
macro_rules! derive_dersztype_enum{
    ($rsz_type:ty) => {
        #[allow(unused)]
        impl DeRszInstance for $rsz_type {
            fn as_any(&self) -> &dyn Any {
                self
            }
            fn to_json(&self, ctx: &RszJsonSerializerCtx) -> serde_json::Value {
                match ctx.field {
                    Some(field) => {
                        let tmp = field.original_type.replace("[]", "");
                        let str_enum_name = |name: &str, val: $rsz_type| { 
                            match get_enum_name(name, &val.to_string()) {
                                //None => format!("{} // Could not find enum value in map {}", name, val.to_string()),
                                None => format!("NULL_BIT_ENUM_OR_COULD_NOT_FIND[{}]", val.to_string()),
                                Some(value) => value
                            }
                        };
                        if enum_map().get(&tmp).is_some() || tmp.contains("Serializable") {
                            return serde_json::json!(str_enum_name(&tmp, *self))
                        } 
                        serde_json::json!(self)
                    }
                    None => serde_json::json!(self)
                }
            }
        }
        impl<'a> DeRszType<'a> for $rsz_type {
            fn from_bytes(ctx: &'a mut RszDeserializerCtx) -> Result<$rsz_type> {
                let mut buf = [0; size_of::<$rsz_type>()];
                ctx.data.read_exact(&mut buf)?;
                Ok(<$rsz_type>::from_le_bytes(buf))
            }
        }
    };
}

derive_dersztype_enum!(i32);
derive_dersztype_enum!(u32);

derive_dersztype_full!(u8);
derive_dersztype_full!(u16);
derive_dersztype_full!(u64);
derive_dersztype_full!(i8);
derive_dersztype_full!(i16);
derive_dersztype_full!(i64);

pub type F8 = u8; // scuffed
pub type F16 = u16; // scuffed
derive_dersztype_full!(f32);
derive_dersztype_full!(f64);

derive_dersztype_full!(bool, |ctx: &'a mut RszDeserializerCtx| -> Result<bool> {
    let v = ctx.data.read_u8()?;
    if v > 1 {
        return Err(Box::new(FileParseError::InvalidBool(v)))
    }
    Ok(v != 0)
});

derive_dersztype_full!(String, |ctx: &'a mut RszDeserializerCtx| -> Result<Self> {
    let mut s = vec![];
    let n = ctx.data.read_u32()?;
    for _i in 0..n {
        let c = ctx.data.read_u16()?;
        s.push(c);
    }
    Ok(String::from_utf16(&s)?)
});
