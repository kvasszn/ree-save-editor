use std::{collections::{HashMap, HashSet, VecDeque}, io::{Read, Seek}};

use fasthash::{FastHash, murmur3};
use indexmap::IndexMap;
use num_enum::TryFromPrimitive;

use crate::{align::seek_align_up, reerr::Result, rsz::{self, dump::{RszDump, RszField, enum_map}, rszserde::{DeRsz, DeRszInstance, Object, RszFieldsValue, RszSerializerCtx, StringU16, StructData}}};
use crate::file::*;

#[repr(i32)]
#[derive(Debug, Clone, Copy, TryFromPrimitive, PartialEq, Eq)]
pub enum ArrayType {
    Value = 0,
    Class = 1,
}
//
#[repr(i32)]
#[derive(Clone, Copy, Debug, TryFromPrimitive, PartialEq, Eq)]
pub enum FieldType {
    Array = -1,
    Enum = 0x1,
    Boolean = 0x2,
    S8 = 0x3,
    U8 = 0x4,
    S16 = 0x5,
    U16 = 0x6,
    S32 = 0x7,
    U32 = 0x8,
    S64 = 0x9,
    U64 = 0xa,
    F32 = 0xb,
    //F64 = 0xc, // this is a guess
    //C8 = 0xd, // guess, wtf even aer these lol, oh just char8 and char 16 probably, i dont think
    //they're used at all though
    //C16 = 0xe, // guess
    String = 0xf, // U16
    Struct = 0x10, // this might overlap with something else or just be wrong rip
    Class = 0x11,
}

impl<'a> TryFrom<&'a RszField> for FieldType {
    type Error = String;
    fn try_from(value: &'a RszField) -> std::result::Result<Self, Self::Error> {
        if value.array {
            return Ok(Self::Array)
        }
        if enum_map().get(&value.original_type).is_some() {
           return Ok(Self::Enum)
        }
        let s = value.r#type.clone();
        Ok(match value.r#type.as_str() {
            "Bool" => Self::Boolean,
            "S8" => Self::S8,
            "U8" => Self::U8,
            "S16" => Self::S16,
            "U16" => Self::U16,
            "S32" => Self::S32,
            "U32" => Self::U32,
            "S64" => Self::S64,
            "U64" => Self::U64,
            "F32" => Self::F32,
            //"F64" => Self::F64,
            "String" => Self::String,
            "Struct" | "Guid" => Self::Struct,
            //"Guid" => Self::Guid,
            "Class" | "Object" => Self::Class,
            _ => Self::Struct,
            //_ => return Err(format!("String value not in FieldType {s}"))
        })
    }
}

#[derive(Debug, Clone)]
pub struct Class {
    pub num_fields: u32,
    pub hash: u32,
    pub fields: IndexMap<u32, Box<dyn DeRszInstance>>
}

impl StructRW for Class {
    fn read<R: Read + Seek>(reader: &mut R, ctx: &mut ()) -> crate::file::Result<Self>
            where
                Self: Sized {
        //seek_align_up(reader, 4)?;
        let num_fields = u32::read(reader, ctx)?;
        let hash = u32::read(reader, ctx)?;
        let _type_info = RszDump::get_struct(hash)?;
        //println!("Class: {}, {num_fields}, {hash:08x}", _type_info.name);
        let fields = (0..num_fields).map(|_i| {
            read_field(reader)
        }).collect::<Result<IndexMap<u32, Box<dyn DeRszInstance>>>>()?;

        Ok(Class {
            num_fields,
            hash,
            fields,
        })
    }
}

pub fn write_value(value: &Box<dyn DeRszInstance>, field_size: Option<u32>, field_type: FieldType, ctx: &mut rsz::rszserde::RszSerializerCtx) -> Result<()> {
    if let Some(field_size) = field_size {
        if field_type != FieldType::String {
            seek_align_up(ctx.data, field_size as u64)?;
        }
    }
    // might need to change this?, not sure if i need to downcast for Class type or Structs (these
    // are probably converted into they're native types, unsure of alignment)
    match field_type {
        FieldType::String => {
            seek_align_up(ctx.data, 4)?;
            if let Some(s) = value.as_any().downcast_ref::<StringU16>() {
                (s.0.len() as u32).to_bytes(ctx)?;
                for item in &s.0 {
                    item.to_bytes(ctx)?;
                }
            } else {
                panic!("Expected String Type found {:?}", value);
            }
        },
        FieldType::Struct => {
            if let Some(sd) = value.as_any().downcast_ref::<StructData>() {
                seek_align_up(ctx.data, sd.0.len() as u64)?;
                ctx.data.write(&sd.0)?;
            } else {
                value.to_bytes(ctx)?;
            }
        },
        _ => {
            value.to_bytes(ctx)?
        }
    }
    Ok(())
}

pub fn write_field(field: (&u32, &Box<dyn DeRszInstance>), type_info: &RszField, ctx: &mut rsz::rszserde::RszSerializerCtx) -> Result<()> {
    let name_hash = field.0;
    let field_value = field.1;
    let field_type = FieldType::try_from(type_info)?;
    let mut field_size = match field_type {
        FieldType::Class | FieldType::Array | FieldType::String => None,
        _ => Some(type_info.size)
    };
    name_hash.to_bytes(ctx)?;
    (field_type as i32).to_bytes(ctx)?;
    // if its StructData, the rszdump is broken so i have to infer the size from the StructData
    // inner value
    if let Some(sd) = field.1.as_any().downcast_ref::<StructData>() {
        field_size = Some(sd.0.len() as u32)
    }
    if let Some(field_size) = field_size {
        field_size.to_bytes(ctx)?;
    }
    write_value(field_value, field_size, field_type, ctx)?;
    seek_align_up(ctx.data, 4)?;
    Ok(())
}

impl DeRszInstance for Class {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn to_json(&self, _ctx: &crate::rsz::rszserde::RszJsonSerializerCtx) -> serde_json::Value {
        todo!()
    }
    fn to_bytes(&self, ctx: &mut crate::rsz::rszserde::RszSerializerCtx) -> Result<()> {
        self.num_fields.to_bytes(ctx)?;
        self.hash.to_bytes(ctx)?;
        let type_info = RszDump::get_struct(self.hash)?;
        for field in &self.fields {
            let mut field_info = type_info.fields.iter()
                .find(|x| murmur3::hash32_with_seed(&x.name, 0xffffffff) == *field.0);
            if field_info.is_none() {
                if let Some(idx) = self.fields.get_index_of(field.0) {
                    field_info = type_info.fields.get(idx);
                    //println!("{:?}", field_info);
                }
            }
            if let Some(field_info) = field_info {
                write_field(field, field_info, ctx)?;
            } else {
                panic!("Skipped field {:?}, {:?}", field_info, field);
            }
        }
        seek_align_up(ctx.data, 4)?;
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct Array {
    pub field_type: FieldType,
    field_type_size: u32,
    pub array_type: ArrayType,
    pub values: Vec<Box<dyn DeRszInstance>>
}

impl StructRW for Array {
    fn read<R: Read + Seek>(reader: &mut R, ctx: &mut ()) -> crate::file::Result<Self>
            where
                Self: Sized {

        seek_align_up(reader, 4)?;
        let field_type = FieldType::try_from(i32::read(reader, ctx)?)?;
        let field_type_size = u32::read(reader, ctx)?;
        let len = u32::read(reader, ctx)?;
        let array_type = ArrayType::try_from(i32::read(reader, ctx)?)?;
        let mut values = Vec::new();
        //println!("Array: {field_type:?}, {field_type_size}, {len}, {array_type:?}");
        for _i in 0..len {
            let value = match array_type {
                ArrayType::Value => read_value(reader, field_type, Some(field_type_size))?,
                ArrayType::Class => Box::new(Class::read(reader, &mut ())?),
            };
            values.push(value);
        }
        seek_align_up(reader, 4)?;
        Ok(Array {
            field_type,
            field_type_size,
            array_type,
            values
        })
    }
}

impl DeRszInstance for Array {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn to_json(&self, _ctx: &crate::rsz::rszserde::RszJsonSerializerCtx) -> serde_json::Value {

        todo!()
    }
    fn to_bytes(&self, ctx: &mut crate::rsz::rszserde::RszSerializerCtx) -> Result<()> {
        seek_align_up(ctx.data, 4)?;
        (self.field_type as i32).to_bytes(ctx)?;
        self.field_type_size.to_bytes(ctx)?;
        (self.values.len() as u32).to_bytes(ctx)?;
        (self.array_type as i32).to_bytes(ctx)?;
        for item in &self.values {
            match self.array_type {
                ArrayType::Class => {
                    // this could a None?
                    item.to_bytes(ctx)?;
                }
                ArrayType::Value => {
                    write_value(item, Some(self.field_type_size), self.field_type, ctx)?;
                }
            }
        }
        seek_align_up(ctx.data, 4)?;
        Ok(())
    }
}



pub fn read_value<R: Read + Seek>(stream: &mut R, field_type: FieldType, field_size: Option<u32>) -> Result<Box<dyn DeRszInstance>> {
    if let Some(field_size) = field_size {
        if field_type != FieldType::String {
            seek_align_up(stream, field_size as u64)?;
        }
    }
    let value: Box<dyn DeRszInstance> = match field_type {
        FieldType::Boolean => Box::new(u8::read(stream, &mut ())? != 0),
        FieldType::U8 => Box::new(u8::read(stream, &mut ())?),
        FieldType::U16 => Box::new(u16::read(stream, &mut ())?),
        FieldType::U32 => Box::new(u32::read(stream, &mut ())?),
        FieldType::U64 => Box::new(u64::read(stream, &mut ())?),
        FieldType::S8 => Box::new(i8::read(stream, &mut ())?),
        FieldType::S16 => Box::new(i16::read(stream, &mut ())?),
        FieldType::S32 | FieldType::Enum => Box::new(i32::read(stream, &mut ())?),
        FieldType::S64 => Box::new(i64::read(stream, &mut ())?),
        FieldType::F32 => Box::new(f32::read(stream, &mut ())?),
        //FieldType::F64 => Box::new(i64::read(stream, &mut ())?),
        FieldType::String => {
            seek_align_up(stream, 4)?;
            let len = u32::read(stream, &mut ())? as usize;
            //println!("0x{:x}", ctx.data.tell()?);
            //println!("len: {len:x}");
            let mut items = Vec::with_capacity(len);
            for _i in 0..len {
                let item = u16::read(stream, &mut ())?;
                items.push(item);
            }
            Box::new(StringU16(items))
        },
        FieldType::Class => {
            //seek_align_up(stream, 4)?;
            Box::new(Class::read(stream, &mut ())?)
        },
        FieldType::Array => Box::new(Array::read(stream, &mut ())?),
        FieldType::Struct => Box::new(StructData(<Vec<u8>>::read(stream, &mut (field_size.expect("Struct Field Type requires field size") as usize))?)),
    };
    Ok(value)
}

pub fn read_field<R: Read + Seek>(stream: &mut R) -> Result<(u32, Box<dyn DeRszInstance>)> {
    let name_hash = u32::read(stream, &mut ())?;
    let field_type = FieldType::try_from(i32::read(stream, &mut ())?)?;
    //println!("{name_hash:x}, {field_type:?}");
    let field_size = match field_type {
        FieldType::Class | FieldType::Array | FieldType::String => None,
        _ => Some(u32::read(stream, &mut ())?)
    };
    let value = read_value(stream, field_type, field_size)?;
    seek_align_up(stream, 4)?;
    //println!("value={value:?}");
    Ok((name_hash, value))
}

pub fn to_dersz(object: Class) -> Result<DeRsz> {
    let offset = 0;
    let mut roots = vec![];
    let mut structs: Vec<RszFieldsValue> = vec![];
    let extern_idxs: HashSet<u32> = HashSet::new();
    use fasthash::murmur3::Hash32;
    let mut queue = VecDeque::new();
    queue.push_back(&object);
    let mut obj_counter = 0;
    roots.push(obj_counter);
    obj_counter += 1;
    while let Some(obj) = queue.pop_front() {
        let type_info = RszDump::get_struct(obj.hash)?;
        //println!("TypeInfo: {type_info:?}");
        //println!("{:?}", obj.fields.keys());
        let mut fields: Vec<Box<dyn DeRszInstance>> = Vec::with_capacity(type_info.fields.len());
        for field in type_info.fields.iter() {
            let name_hash = Hash32::hash_with_seed(field.name.as_bytes(), 0xffffffff);
            //println!("{name_hash:#x}, {}", field.name);
            match obj.fields.get(&name_hash) {
                Some(value) => {
                    if let Some(class) = value.as_any().downcast_ref::<Class>() {
                        fields.push(Box::new(Object {hash: class.hash, idx: obj_counter}));
                        obj_counter += 1;
                        queue.push_back(class);
                    } else if let Some(array) = value.as_any().downcast_ref::<Array>() {
                        let mut vals: Vec<Box<dyn DeRszInstance>> = Vec::new();
                        let array_type = array.array_type;
                        for arr_val in &array.values {
                            if array_type == ArrayType::Class {
                                if let Some(class) = arr_val.as_any().downcast_ref::<Class>() {
                                    vals.push(Box::new(Object {hash: class.hash, idx: obj_counter}));
                                    obj_counter += 1;
                                    queue.push_back(class);
                                }
                            } else if array_type == ArrayType::Value {
                                vals.push(arr_val.clone())
                            }
                        } 
                        fields.push(Box::new(vals));
                    } else {
                        fields.push(value.clone());
                    }
                },
                None => {
                    println!("No field found for {}, {:x}: {}", field.name, name_hash, type_info.name);
                    fields.push(Box::new("[[NO FIELD FOUND FOR THIS]]".to_string()));
                }
            }
        }
        //println!("{fields:?}");
        structs.push((obj.hash, fields));
        //panic!();
    }
    //println!("{structs:?}");
    Ok(DeRsz {
        offset,
        roots,
        structs,
        extern_idxs
    })
}
