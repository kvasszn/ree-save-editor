use std::{collections::{HashMap, VecDeque}, io::{Cursor, Read, Seek}};

use num_enum::TryFromPrimitive;

use crate::{file_ext::{ReadExt, SeekExt}, reerr::Result, rsz::{dump::{RszDump, RszField}, rszserde::{DeRszInstance, DeRszType, Guid, RszDeserializerCtx, RszFieldsValue, StringU16}, Extern, TypeDescriptor}};

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
#[derive(Debug, TryFromPrimitive, PartialEq, Eq)]
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
    Guid = 0x10,
    Class = 0x11,
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
    pub fn read_class<R: ReadExt + SeekExt + Seek + Read> (stream: &mut R) -> Result<Self> {
        stream.seek_align_up(4)?;
        let num_fields = stream.read_u32()?;
        let hash = stream.read_u32()?;
        println!("read_class: num_fields={num_fields:08X}, hash:{hash:08X}");
        let type_info = RszDump::get_struct(hash)?;
        println!("Class: {}", type_info.name);
        let fields = (0..num_fields).map(|_i| {
            println!("\tfield: {:?}", type_info.fields[_i as usize]);
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
        println!("Array: {arr_field_type:?}, {arr_field_type_size:08X}, {len:08X}, {array_type:?}");
        let mut values = Vec::new();
        for i in 0..len {
            println!("read array member {i}");
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
        println!("{size:08X}");
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

        println!("value={value:?}");
        Ok(Field::Value {
            size,
            value
        })
    }

    pub fn from_stream<R: ReadExt + SeekExt + Seek + Read> (stream: &mut R) -> Result<(u32, Self)> {
        let unk = stream.read_u32()?;
        let field_type_i32 = stream.read_i32()?;
        println!("Read field: {unk:08X} {field_type_i32:08X} pos={:08X}", stream.tell()?);
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
}

#[derive(Debug)]
pub enum SaveNode {
    Object {
        from_array: bool,
        values: Vec<Box<dyn DeRszInstance>>,
    },
    Value {
        field: &'static RszField,
        value: Vec<Box<dyn DeRszInstance>>
    },
    Array(&'static RszField, Vec<Box<dyn DeRszInstance>>),
    Unk,
}

impl SaveFile {
    pub fn from_reader_v2<R: ReadExt + SeekExt + Seek + Read> (mut reader: R) -> Result<SaveFile> {
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

    pub fn from_reader<R: ReadExt + SeekExt + Seek + Read> (mut reader: R) -> Result<()> {
        let mut queue: VecDeque<SaveNode> = VecDeque::new();

        let fake_extern_slots = HashMap::<u32, Extern>::new();
        let fake_roots = Vec::<u32>::new();
        let fake_type_descriptors = Vec::new();
        let mut ctx = RszDeserializerCtx::new(&mut reader, &fake_type_descriptors, &fake_extern_slots, &fake_roots);

        let unk = ctx.data.read_u32()?;
        let num_fields = ctx.data.read_u32()?;
        let hash = ctx.data.read_u32()?;
        //let unk = ctx.data.read_u32()?;
        let type_info = RszDump::get_struct(hash)?;
        println!("Read data {unk:08X}, {num_fields:08X}, {hash:08X}, {unk:08X}, {}", type_info.name);
        for field in type_info.fields.iter().rev() {
            println!("{}, {}", field.name, field.original_type);
            if field.array {
                queue.push_back(SaveNode::Array(&field, Vec::new()));
            }
            else if field.r#type.as_str() == "Object" {
                queue.push_back(SaveNode::Object { from_array: false,  values: Vec::new() });
            }
            else {
                queue.push_back(SaveNode::Value { field: &field, value: Vec::new()});
            }
        }
        //let type_info = RszDump::get_struct(type_hash)?;
        while let Some(node) = queue.pop_back() {
            let unk = ctx.data.read_u32()?;
            println!("UNK={unk:08X}");
            println!("Popped {:?}", node);
            match node {
                SaveNode::Unk => {
                    //let unk = ctx.data.read_u32()?;
                },
                SaveNode::Object { from_array, values } => {

                    let type_val = if !from_array { ctx.data.read_u32()?} else {0};
                    let num_fields = ctx.data.read_u32()?;
                    let hash = ctx.data.read_u32()?;
                    //let unk = ctx.data.read_u32()?;
                    let type_info = RszDump::get_struct(hash)?;
                    println!("Read data {type_val:08X}, {num_fields:08X}, {hash:08X}, {unk:08X}, {}", type_info.name);
                    let mut obj_fields: Vec<RszFieldsValue> = Vec::new();

                    if !from_array {
                        //queue.push_back(SaveNode::Unk);
                    }
                    for field in type_info.fields.iter().rev() {
                        println!("{}, {}", field.name, field.original_type);
                        if field.array {
                            queue.push_back(SaveNode::Array(&field, Vec::new()));
                        }
                        else if field.r#type.as_str() == "Object" {
                            queue.push_back(SaveNode::Object { from_array: false,  values: Vec::new() });
                        }
                        else {
                            println!("Pushed Unk");
                            queue.push_back(SaveNode::Unk);
                            queue.push_back(SaveNode::Value { field: &field, value: Vec::new()});
                        }
                    }
                },
                SaveNode::Array(field, mut values) => {
                    let type_val = ctx.data.read_u32()?;
                    let idk1 = ctx.data.read_u32()?;
                    let idk2 = ctx.data.read_u32()?;
                    let count = ctx.data.read_u32()?;
                    let array_type = ctx.data.read_u32()?;
                    println!("Array {idk1:08X}, {idk2:08X}, {count:X}, {array_type:X}");
                    if array_type == 0 { // Value type array
                        for _i in 0..count {
                            println!("Pushed Value for array {_i}");
                            let dersz_fn = ctx.registry.get(field.r#type.as_str())?;
                            let val: Box<dyn DeRszInstance> = dersz_fn(&mut ctx)?;
                            println!("{val:?}");
                            values.push(val);
                        }
                        //let unk = ctx.data.read_u32()?;
                    } else {
                        for _i in 0..count {
                            println!("Pushed Object for array {_i}");
                            panic!();
                            queue.push_back(SaveNode::Object { from_array: true, values: Vec::new() });
                        }
                    }
                    /*for _i in 0..count {
                      }*/
                    println!("Array: unk={unk:08X}, count={count:08X}, vals={values:?}, {}, {}", field.name, field.original_type);
                }
                SaveNode::Value { field, value } => {
                    let type_val = ctx.data.read_u32()?;
                    let size = ctx.data.read_u32()?;
                    let dersz_fn = ctx.registry.get(field.r#type.as_str())?;
                    let x: Box<dyn DeRszInstance> = dersz_fn(&mut ctx)?;
                    ctx.data.seek_align_up(4)?;
                    //ctx.data.seek_align_up(field.align)?;
                    println!("Value type={type_val:08X}, size={size:x}, {}, {}, val={x:?}", field.name, field.original_type);
                }
            }
        }
        Ok(())

    }
}
