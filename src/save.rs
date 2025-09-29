use std::{collections::{HashMap, HashSet, VecDeque}, io::{Cursor, Read, Seek}, path::Path};

use mhtame::rsz::dump::enum_map;
use num_enum::TryFromPrimitive;

use crate::{crypt::Mandarin, file_ext::{ReadExt, SeekExt}, reerr::Result, rsz::{dump::{RszDump, RszField}, rszserde::{DeRsz, DeRszInstance, DeRszType, Guid, Object, RszDeserializerCtx, RszFieldsValue, StringU16}, Extern}};

#[derive(Debug)]
pub struct SaveFile {
    pub data: Field,
    pub detail: Field
}

/*
 *  This shit lwk all wrong almost
 * The format for serialized save stuff is weird,
 * It's all inlined, different from the RSZ stuff
 * struct {
 *  hash: u32,
 *  somedumbshit: u32, // this could possibly a versioning thing, or just some consistency checker
 *  to mark the end of fields
 *  type: u32,
 *  unk: u32, // number of fields to read (this is to be safe from version updates)
 *
 * }
 *
 * to read:
 * first read the initial type hash, 
 * for each field in the type, 
 *
 *
 *
 */

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
    //C8 = 0xd, // guess
    //C16 = 0xe, // guess
    String = 0xf, // U16
    Guid = 0x10, // this might overlap with something else or just be wrong rip
    Class = 0x11,
}

impl<'a> TryFrom<&'a RszField> for FieldType {
    type Error = &'static str;
    fn try_from(value: &'a RszField) -> std::result::Result<Self, Self::Error> {
        if value.array {
            return Ok(Self::Array)
        }
        if enum_map().get(&value.original_type).is_some() {
            return Ok(Self::Enum)
        }
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
            "String" => Self::String,
            "Guid" => Self::Guid,
            "Class" => Self::Class,
            _ => return Err("String value not in FieldType")
        })
    }
}

#[derive(Debug)]
pub enum Field {
    Array {
        arr_field_type: FieldType,
        arr_field_type_size: u32,
        len: u32,
        array_type: ArrayType,
        values: Vec<Field>
    },
    Value {
        size: u32,
        value: Box<dyn DeRszInstance>
    },
    Class {
        num_fields: u32,
        hash: u32,
        fields: Vec<(u32, Field)>
    }
}

impl Field {
    pub fn read_class_info<R: ReadExt + SeekExt + Seek + Read> (stream: &mut R) -> Result<Self> {
        stream.seek_align_up(4)?;
        let num_fields = stream.read_u32()?;
        let hash = stream.read_u32()?;
        Ok(Field::Class { num_fields, hash, fields: Vec::new() })
    }
    pub fn read_class<R: ReadExt + SeekExt + Seek + Read> (stream: &mut R) -> Result<Self> {
        stream.seek_align_up(4)?;
        let num_fields = stream.read_u32()?;
        let hash = stream.read_u32()?;
        //println!("read_class: num_fields={num_fields:08X}, hash:{hash:08X}");
        let _type_info = RszDump::get_struct(hash)?;
        //println!("Class: {}", type_info.name);
        let fields = (0..num_fields).map(|_i| {
            //println!("\tfield: {:?}", type_info.fields[_i as usize]);
            let x = Field::from_stream(stream)?;
            Ok(x)
        }).collect::<Result<Vec<(u32, Field)>>>()?;
        Ok(Field::Class {
            num_fields,
            hash,
            fields,
        })
    }

    pub fn read_array<R: ReadExt + SeekExt + Seek + Read> (stream: &mut R) -> Result<Self> {
        stream.seek_align_up(4)?;
        let arr_field_type_i32 = stream.read_i32()?;
        let arr_field_type = FieldType::try_from(arr_field_type_i32)?;
        let arr_field_type_size = stream.read_u32()?;
        let len = stream.read_u32()?;
        let array_type = stream.read_i32()?;
        let array_type = ArrayType::try_from(array_type)?;
        //println!("Array: {arr_field_type:?}, {arr_field_type_size:08X}, {len:08X}, {array_type:?}");
        let mut values = Vec::new();
        for _i in 0..len {
            //println!("read array member {i}");
            let value = match array_type {
                ArrayType::Class => {
                    Field::read_class(stream)
                }
                ArrayType::Value => {
                    Field::read_value(stream, arr_field_type_size, arr_field_type)
                }
            }?;
            values.push(value);
        }
        stream.seek_align_up(4)?;
        Ok(Field::Array {
            arr_field_type,
            arr_field_type_size,
            len,
            array_type,
            values,
        })
    }

    pub fn read_value<R: ReadExt + SeekExt + Seek + Read> (stream: &mut R, size:u32, field_type: FieldType) -> Result<Self> {
        let fake_extern_slots = HashMap::<u32, Extern>::new();
        let fake_roots = Vec::<u32>::new();
        let fake_type_descriptors = Vec::new();
        if field_type != FieldType::String {
            stream.seek_align_up(size as u64)?;
        }
        //println!("{size:08X}");
        let mut ctx = RszDeserializerCtx::new(stream, &fake_type_descriptors, &fake_extern_slots, &fake_roots);
        //let value = stream.read_u8_n(size as usize)?;
        let value: Box<dyn DeRszInstance> = match field_type {
            FieldType::String => {
                ctx.data.seek_align_up(4)?;
                Box::new(StringU16::from_bytes(&mut ctx)?)
            },
            FieldType::Guid => Box::new(Guid::from_bytes(&mut ctx)?),
            FieldType::U64 => Box::new(u64::from_bytes(&mut ctx)?),
            FieldType::U32 => Box::new(u32::from_bytes(&mut ctx)?),
            FieldType::U16 => Box::new(u16::from_bytes(&mut ctx)?),
            FieldType::U8 => Box::new(u8::from_bytes(&mut ctx)?),
            FieldType::S64 => Box::new(i64::from_bytes(&mut ctx)?),
            FieldType::S32 | FieldType::Enum => Box::new(i32::from_bytes(&mut ctx)?),
            FieldType::S16 => Box::new(i16::from_bytes(&mut ctx)?),
            FieldType::S8 => Box::new(i8::from_bytes(&mut ctx)?),
            FieldType::F32 => Box::new(f32::from_bytes(&mut ctx)?),
            FieldType::Boolean => Box::new(bool::from_bytes(&mut ctx)?),
            _ => panic!("bad field type for value")
        };

        //println!("value={value:?}");
        Ok(Field::Value {
            size,
            value
        })
    }

    pub fn from_stream<R: ReadExt + SeekExt + Seek + Read> (stream: &mut R) -> Result<(u32, Self)> {
        let unk = stream.read_u32()?;
        let field_type_i32 = stream.read_i32()?;
        //println!("Read field: {unk:08X} {field_type_i32:08X} pos={:08X}", stream.tell()?);
        let field_type: FieldType = FieldType::try_from(field_type_i32).expect(&format!("No known type field type for {field_type_i32}"));
        let field = match field_type {
            FieldType::Class => {
                Self::read_class(stream)?
            },
            FieldType::Array => {
                Self::read_array(stream)?
            },
            FieldType::String => {
                let fake_extern_slots = HashMap::<u32, Extern>::new();
                let fake_roots = Vec::<u32>::new();
                let fake_type_descriptors = Vec::new();
                let mut ctx = RszDeserializerCtx::new(&mut *stream, &fake_type_descriptors, &fake_extern_slots, &fake_roots);
                let x = StringU16::from_bytes(&mut ctx)?;
                Field::Value {
                    size: x.0.len() as u32,
                    value: Box::new(x)
                }
            },
            _ => {
                let size = stream.read_u32()?;
                Self::read_value(stream, size, field_type)?
            },
        };
        stream.seek_align_up(4)?;
        Ok((unk, field))
    }


    pub fn make_value(field: &RszField, value: Box<dyn DeRszInstance>) -> Field {
        Field::Value { size: field.size, value }
    }

    pub fn make_array(field: &RszField, values: Vec<Box<dyn DeRszInstance>>) -> Field {
        let arr_field_type = FieldType::try_from(field).unwrap();
        let arr_field_type_size = field.size;
        let len = values.len() as u32;
        let array_type = if field.r#type == "Object" {
            ArrayType::Class
        } else {ArrayType::Value};
        let values: Vec<Field> = match array_type {
            ArrayType::Value => {
                values.into_iter().map(|v| {
                    Self::make_value(field, v)
                }).collect()
            },
            ArrayType::Class => {
                values.into_iter().map(|v| {
                    //Self::make_class(field.get_type_hash().unwrap(), v)
                    Self::make_value(field, v)
                }).collect()
            }
        };
        Field::Array { 
            arr_field_type,
            arr_field_type_size,
            len,
            array_type,
            values
        }
    }
}

impl From<SaveFile> for DeRsz {
    fn from(value: SaveFile) -> Self {
        let fields = vec![value.data, value.detail];
        Self::from(fields)
    }
}

#[allow(unused_variables)]
impl From<Vec<Field>> for DeRsz {
    // top level should be a Class type always
    fn from(fields: Vec<Field>) -> Self {
        let mut structs: Vec<RszFieldsValue> = Vec::new();
        let mut roots = Vec::new();
        let mut object_counter = 0;
        for field in fields {
            roots.push(object_counter);
            object_counter += 1;
            let mut queue = VecDeque::new();
            queue.push_back(field);
            while let Some(field) = queue.pop_front() {
                match field {
                    Field::Class { num_fields: _, hash, fields } => {
                        let mut field_vals: Vec<Box<dyn DeRszInstance>> = Vec::new();
                        for (_unk, field) in fields {
                            match field {
                                Field::Class { num_fields: _, hash, ref fields } => {
                                    field_vals.push(Box::new(Object {hash: hash, idx: object_counter}));
                                    object_counter += 1;
                                    queue.push_back(field);
                                },
                                Field::Value { size: _, value } => {
                                    field_vals.push(value);
                                },
                                Field::Array { arr_field_type: _, arr_field_type_size: _, len: _, array_type, values } => {
                                    let mut array_vals: Vec<Box<dyn DeRszInstance>> = Vec::new();
                                    for value in values {
                                        if array_type == ArrayType::Class {
                                            if let Field::Class{ num_fields: _, hash, ref fields } = value {
                                                array_vals.push(Box::new(Object {hash: hash, idx: object_counter}));
                                                object_counter += 1;
                                                queue.push_back(value);
                                            }
                                        } else if array_type == ArrayType::Value {
                                            if let Field::Value { size: _, value } = value {
                                                array_vals.push(value); 
                                            }
                                        }
                                    }
                                    field_vals.push(Box::new(array_vals));
                                },
                            }
                        }
                        structs.push((hash, field_vals));
                    }
                    _ => {
                        panic!("Top level Field must be Class")
                    }
                }
            }
        }
        Self {
            roots,
            offset: 0,
            structs,
            extern_idxs: HashSet::new()
        }
    }
}

impl SaveFile {
    pub fn from_file(file: &Path) -> Result<SaveFile> {
        let mandarin = Mandarin::init();
        let key: u64 = 0x011000011168AFC6;
        //let key: u64 = 76561198053503919;
        //let key: u64 = 76561198372695648;
        let decrypted = mandarin.decrypt_file(file, key)?;
        mandarin.uninit();
        Self::from_decrypted(decrypted)
    }
    pub fn from_decrypted(buffer: Vec<u8>) -> Result<SaveFile> {
        let mut buffer = Cursor::new(buffer);
        let _compressed_size = buffer.read_u64()?;
        let _unk = buffer.read_u32()?; // this mighte be compression level
        let _compressed_size_sub0x10 = buffer.read_u32()?;
        let decompressed_size = buffer.read_u64()?;

        let compressed = &buffer.get_ref()[0x18..];
        let mut decompressed = vec![0u8; decompressed_size as usize];
        let mut decompressor = libdeflater::Decompressor::new();
        let _decompression_result = decompressor.deflate_decompress(&compressed, &mut decompressed)?;
        std::fs::write("save.bin", &decompressed)?;

        let buffer = Cursor::new(decompressed);
        Self::from_reader(buffer)
    }

    pub fn from_reader<R: ReadExt + SeekExt + Seek + Read> (mut reader: R) -> Result<SaveFile> {
        let mut unks: Vec<u32> = Vec::new();
        let unk = reader.read_u32()?;
        unks.push(unk);
        let data = Field::read_class(&mut reader)?;
        let unk = reader.read_u32()?;
        unks.push(unk);
        let detail = Field::read_class(&mut reader)?;
        Ok(SaveFile {
            data,
            detail
        })
    }
}
