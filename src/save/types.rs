use std::io::{Read, Seek};
use std::error::Error;

use eframe::egui::{CollapsingHeader, Ui};
use indexmap::IndexMap;
use num_enum::TryFromPrimitive;
use sdk::type_map;
use sdk::types::StringU16;

use util::*;

use crate::edit::{EditContext, EditResponse, Editable, RszEditCtx};
#[allow(unused_imports)]
use crate::rsz::dump::RszDump;

// Some of this stuff comes from via.reflection.TypeKind
#[repr(i32)]
#[derive(Clone, Copy, Debug, TryFromPrimitive, PartialEq, Eq)]
pub enum FieldType {
    Array = -1, // This is hidden in any enums that are similar to this
    Unknown = 0,
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
    F64 = 0xc,
    C8 = 0xd,
    C16 = 0xe,
    String = 0xf, // U16String
    Struct = 0x10,
    Class = 0x11,
}

#[derive(Debug, Clone)]
pub enum FieldValue {
    Array(Box<Array>),
    Unknown,
    Enum(i32),
    Boolean(bool),
    S8(i8),
    U8(u8),
    S16(i16),
    U16(u16),
    S32(i32),
    U32(u32),
    S64(i64),
    U64(u64),
    F32(f32),
    F64(f64),
    C8(u8),
    C16(u16),
    String(Box<StringU16>),
    Struct(Box<Struct>),
    Class(Box<Class>)
}

impl FieldValue {
    pub fn read<R: Read + Seek>(reader: &mut R, field_type: FieldType) -> Result<Self, Box<dyn Error>> {
        let value = match field_type {
            FieldType::Unknown => { panic!("Unknown Field Type found") }
            FieldType::Array => { 
                FieldValue::Array(Box::new(Array::read(reader)?))
            }
            FieldType::Class=> { 
                FieldValue::Class(Box::new(Class::read(reader)?))
            }
            FieldType::String => { 
                reader.seek_align_up(4)?;
                let size = reader.read_u32()?;
                let data = (0..size).map(|_| {
                    Ok(reader.read_u16()?)
                }).collect::<Result<Vec<u16>, Box<dyn Error>>>()?;
                FieldValue::String(Box::new(StringU16::new(data)))
            }
            // These values actually need a size/len
            _ => {
                reader.seek_align_up(4)?;
                let size = reader.read_u32()?;
                FieldValue::read_sized(reader, field_type, size)?
            }
        };
        Ok(value)
    }

    // When this is run for the Array struct, it should never be able to read an Object, I'm not
    // sure about array though
    pub fn read_sized<R: Read + Seek>(reader: &mut R, field_type: FieldType, size: u32) -> Result<Self, Box<dyn Error>> {
        if field_type != FieldType::String {
            reader.seek_align_up(size as u64)?;
        }
        let value = match field_type {
            FieldType::Enum => { FieldValue::Enum(reader.read_i32()?) }
            FieldType::Boolean => { FieldValue::Boolean(reader.read_bool()?) }
            FieldType::S8 => { FieldValue::S8(reader.read_i8()?) }
            FieldType::U8 => { FieldValue::U8(reader.read_u8()?) }
            FieldType::S16 => { FieldValue::S16(reader.read_i16()?) }
            FieldType::U16 => { FieldValue::U16(reader.read_u16()?) }
            FieldType::S32 => { FieldValue::S32(reader.read_i32()?) }
            FieldType::U32 => { FieldValue::U32(reader.read_u32()?) }
            FieldType::S64 => { FieldValue::S64(reader.read_i64()?) }
            FieldType::U64 => { FieldValue::U64(reader.read_u64()?) }
            FieldType::F32 => { FieldValue::F32(reader.read_f32()?) }
            FieldType::F64 => { FieldValue::F64(reader.read_f64()?) }
            FieldType::C8 => { FieldValue::C8(reader.read_u8()?) }
            FieldType::C16 => { FieldValue::C16(reader.read_u16()?) }
            FieldType::Array => { FieldValue::Array(Box::new(Array::read(reader)?))}
            FieldType::Struct => { 
                let mut data = vec![0u8; size as usize];
                reader.read_exact(&mut data)?;
                FieldValue::Struct(Box::new(Struct{ data }))
            }
            _ => return Err(format!("Unexpected sized read of {:?} for {:?}", size, field_type).into())
        };
        return Ok(value)
    }
}

impl Editable for FieldValue {
    fn edit(&mut self, ui: &mut Ui, ctx: &EditContext) -> EditResponse {
        match self {
            FieldValue::Enum(v) => {
                v.edit(ui, ctx)
            },
            FieldValue::S8(v) => v.edit(ui, ctx),
            FieldValue::U8(v) => v.edit(ui, ctx),
            FieldValue::S16(v) => v.edit(ui, ctx),
            FieldValue::U16(v) => v.edit(ui, ctx),
            FieldValue::S32(v) => v.edit(ui, ctx),
            FieldValue::U32(v) => v.edit(ui, ctx),
            FieldValue::S64(v) => v.edit(ui, ctx),
            FieldValue::U64(v) => v.edit(ui, ctx),
            FieldValue::F32(v) => v.edit(ui, ctx),
            FieldValue::F64(v) => v.edit(ui, ctx),
            FieldValue::Class(c) => c.edit(ui, ctx),
            FieldValue::Array(a) => a.edit(ui, ctx),
            FieldValue::String(v) => {
                ui.label(format!("{}", v));
                EditResponse::default()
            },
            _ => { 
                ui.label(format!("{:?}", self));
                EditResponse::default()
            }
        }
    }
}

#[repr(i32)]
#[derive(Debug, Clone, Copy, TryFromPrimitive, PartialEq, Eq)]
pub enum ArrayType {
    Value = 0,
    Class = 1,
}

#[derive(Debug, Clone)]
pub struct Array {
    pub member_type: FieldType,
    pub member_size: u32,
    pub array_type: ArrayType,
    pub values: Vec<FieldValue>
}

impl Array {
    pub fn read<R: Read + Seek>(reader: &mut R) -> Result<Self, Box<dyn Error>> {
        reader.seek_align_up(4)?;
        let member_type = FieldType::try_from(reader.read_i32()?)?;
        let member_size = reader.read_u32()?;
        let len = reader.read_u32()?;
        let array_type = ArrayType::try_from(reader.read_i32()?)?;
        let mut values = Vec::with_capacity(len as usize);
        for _i in 0..len {
            let value = match array_type {
                ArrayType::Value => {
                    if member_type == FieldType::String { 
                        reader.seek_align_up(4)?;
                        let size = reader.read_u32()?;
                        let data = (0..size).map(|_| {
                            Ok(reader.read_u16()?)
                        }).collect::<Result<Vec<u16>, Box<dyn Error>>>()?;
                        FieldValue::String(Box::new(StringU16::new(data)))
                    } else {
                        FieldValue::read_sized(reader, member_type, member_size)?
                    }
                },
                ArrayType::Class => FieldValue::Class(Box::new(Class::read(reader)?)),
            };
            values.push(value);
        }
        reader.seek_align_up(4)?;
        Ok(Self {
            member_type,
            member_size,
            array_type,
            values
        })
    }
}

impl Editable for Array {
    fn edit(&mut self, ui: &mut Ui, ctx: &EditContext) -> EditResponse {
        ui.push_id(ctx.id, |ui| {
            for (i, member) in self.values.iter_mut().enumerate() {
                let child_id = ui.make_persistent_id(i);
                let child_ctx = EditContext {
                    id: child_id.value(),
                    ..*ctx
                };
                // need to make a get member name helper for TypeInfo
                member.edit(ui, &child_ctx);
            }
            EditResponse::default()
        }).inner
    }
}

#[derive(Debug, Clone)]
pub struct Field {
    pub hash: u32,
    pub field_type: FieldType,
    pub value: FieldValue,
}

impl Field {
    pub fn read<R: Read + Seek>(reader: &mut R) -> Result<Self, Box<dyn Error>> {
        let hash = reader.read_u32()?;
        // This migth treat parse Array here, would make sense with the enum not being in
        // reflection::TypeKind
        let field_type = FieldType::try_from(reader.read_i32()?)?;
        let value = FieldValue::read(reader, field_type)?;
        seek_align_up(reader, 4)?;
        Ok(Self {
            hash,
            field_type,
            value
        })
    }
}

impl Editable for Field {
    fn edit(&mut self, ui: &mut Ui, ctx: &EditContext) -> EditResponse {
        //let type_info = ctx.type_map.get_by_hash(self.hash);
        let field_info = ctx.parent_type.and_then(|x| x.get_by_hash(self.hash));
        let name: String = field_info.map(|x| x.name.clone())
            .unwrap_or(format!("{:08x}", self.hash));
        let og_type = field_info.map(|f| f.original_type.clone())
            .unwrap_or(format!("{:?}", self.field_type));
        let name_contained = format!("{name}: {og_type}");

        match self.field_type {
            FieldType::Array | FieldType::Class | FieldType::Struct => {
                CollapsingHeader::new(name_contained)
                    .id_salt(self.hash)
                    .default_open(!ctx.search_term.is_empty())
                    .show(ui, |ui| {
                        let child_resp = self.value.edit(ui, ctx);
                    });
            }
            _ => {
                ui.horizontal(|ui| {
                    ui.label(name);
                    let child_resp = self.value.edit(ui, ctx);
                    ui.label(format!(" :{og_type}"));
                    //response.unite(child_resp);
                });
            }
        }
        EditResponse::default()
    }
}

#[derive(Debug, Clone)]
pub struct Struct {
    pub data: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct Class {
    pub num_fields: u32,
    pub hash: u32,
    pub fields: IndexMap<u32, Field>
}

impl Class {
    pub fn read<R: Read + Seek>(reader: &mut R) -> Result<Self, Box<dyn Error>> {
        let num_fields = reader.read_u32()?;
        let hash = reader.read_u32()?;
        //let name = RszDump::get_struct(hash).and_then(|x|Ok(x.name.clone())).unwrap_or("none".to_string());
        let fields = (0..num_fields).map(|_| {
            let field = Field::read(reader).unwrap();
            Ok((field.hash, field))
        }).collect::<Result<IndexMap<u32, Field>, Box<dyn Error>>>()?;
        Ok(Self {
            num_fields,
            hash,
            fields
        })
    }
}

impl Editable for Class {
    fn edit(&mut self, ui: &mut Ui, ctx: &EditContext) -> EditResponse {
        let type_info = ctx.type_map.get_by_hash(self.hash);
        ui.push_id(ctx.id, |ui| {
            for (field_hash, field) in &mut self.fields {
                let child_id = ui.make_persistent_id(field_hash);
                let child_ctx = EditContext {
                    id: child_id.value(),
                    parent_type: type_info,
                    ..*ctx
                };
                // need to make a get field name helper for TypeInfo
                field.edit(ui, &child_ctx);
            }

            EditResponse::default()
        }).inner
    }
}

