use std::io::{Cursor, Read, Seek, Write};
use std::error::Error;

use indexmap::IndexMap;
use num_enum::TryFromPrimitive;
use crate::sdk::type_map::{TypeInfo, TypeMap};
use crate::sdk::{types::*};

use util::*;

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
            // TODO: Add Struct weird shit handling
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
        /*if field_type == FieldType::Struct {
            let pos = reader.stream_position()?;
            if size == 24 {
                if pos % 32 > 16 {
                    let aligned = 32 - (pos % (16)); 
                    reader.seek(std::io::SeekFrom::Current(aligned as i64))?;
                } else {
                    reader.seek_align_up(16)?;
                }
            } else if size == 32 {
                let aligned = 32 - (pos % (16)); 
                reader.seek(std::io::SeekFrom::Current(aligned as i64))?;
            } else {
                reader.seek_align_up(size.into())?;
            }
        }
        else*/
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

    pub fn write<W: Write + Seek>(&self, w: &mut W) -> Result<(), Box<dyn Error>> {
        match self {
            FieldValue::Unknown => { panic!("Unknown Field Type found") }
            FieldValue::Array(v) => v.write(w),
            FieldValue::Class(v) => v.write(w),
            FieldValue::String(v) => { 
                w.seek_align_up(4)?;
                w.write(&(v.0.len() as u32).to_le_bytes())?;
                for e in &v.0 {
                    w.write(&e.to_le_bytes())?;
                }
                Ok(())
            }
            // These values actually need a size/len
            _ => {
                w.seek_align_up(4)?;
                let size = self.get_size();
                w.write(&size.to_le_bytes())?;
                self.write_sized(w)?;
                Ok(())
            }
        }
    }


    pub fn write_sized<W: Write + Seek>(&self, w: &mut W) -> Result<(), Box<dyn Error>> {
        w.seek_align_up(self.get_size() as u64)?;
        let _ = match self {
            FieldValue::Enum(v) => w.write(&v.to_le_bytes())?,
            FieldValue::Boolean(v) => w.write(&[*v as u8])?,
            FieldValue::S8(v) => w.write(&[*v as u8])?,
            FieldValue::U8(v) => w.write(&[*v])?,
            FieldValue::C8(v) => w.write(&[*v])?,
            FieldValue::S16(v) => w.write(&v.to_le_bytes())?,
            FieldValue::U16(v) => w.write(&v.to_le_bytes())?,
            FieldValue::C16(v) => w.write(&v.to_le_bytes())?,
            FieldValue::S32(v) => w.write(&v.to_le_bytes())?,
            FieldValue::U32(v) => w.write(&v.to_le_bytes())?,
            FieldValue::F32(v) => w.write(&v.to_le_bytes())?,
            FieldValue::S64(v) => w.write(&v.to_le_bytes())?,
            FieldValue::U64(v) => w.write(&v.to_le_bytes())?,
            FieldValue::F64(v) => w.write(&v.to_le_bytes())?,
            FieldValue::Array(v) => {v.write(w)?; 0},
            FieldValue::Struct(v) => {
                for e in &v.data {
                    w.write(&e.to_le_bytes())?;
                }
                v.data.len()
            }
            _ => 0
        };
        Ok(())
    }

    pub fn get_size(&self) -> u32 {
        match self {
            FieldValue::Boolean(_) | FieldValue::U8(_) | FieldValue::S8(_) | FieldValue::C8(_) => 1,
            FieldValue::U16(_) | FieldValue::S16(_) | FieldValue::C16(_)  => 2,
            FieldValue::Enum(_) | FieldValue::U32(_) | FieldValue::S32(_) | FieldValue::F32(_) => 4,
            FieldValue::U64(_) | FieldValue::S64(_) | FieldValue::F64(_) => 8,
            FieldValue::Struct(v) => v.data.len() as u32,
            _ => 0
        }

    }

}

impl FieldValue {
    pub fn get_enum_str(&self) -> Option<String> {
        None
    }

    pub fn as_class(&self) -> Option<&Class> {
        match self {
            FieldValue::Class(b) => Some(b),
            _ => None,
        }
    }

    pub fn as_class_mut(&mut self) -> Option<&mut Class> {
        match self {
            FieldValue::Class(b) => Some(b),
            _ => None,
        }
    }

    pub fn as_array(&self) -> Option<&Array> {
        match self {
            FieldValue::Array(b) => Some(b),
            _ => None,
        }
    }

    pub fn as_array_mut(&mut self) -> Option<&mut Array> {
        match self {
            FieldValue::Array(b) => Some(b),
            _ => None,
        }
    }

    pub fn as_struct(&self) -> Option<&Struct> {
        match self {
            FieldValue::Struct(b) => Some(b),
            _ => None,
        }
    }

    pub fn as_struct_mut(&mut self) -> Option<&mut Struct> {
        match self {
            FieldValue::Struct(b) => Some(b),
            _ => None,
        }
    }

    pub fn as_string_utf8(&self) -> Option<String> {
        match self {
            FieldValue::String(s) => Some(s.to_string()),
            _ => None,
        }
    }

    pub fn as_string_u16(&self) -> Option<&StringU16> {
        match self {
            FieldValue::String(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_string_u16_mut(&mut self) -> Option<&mut StringU16> {
        match self {
            FieldValue::String(s) => Some(s),
            _ => None,
        }
    }
}

macro_rules! impl_primitive_getter {
    ($fn_name:ident, $enum_variant:ident, $ret_type:ty) => {
        impl FieldValue {
            pub fn $fn_name(&self) -> Option<$ret_type> {
                match self {
                    FieldValue::$enum_variant(v) => Some(*v),
                    _ => None,
                }
            }
        }
    };
}

macro_rules! impl_primitive_getter_mut {
    ($fn_name:ident, $enum_variant:ident, $ret_type:ty) => {
        impl FieldValue {
            pub fn $fn_name(&mut self) -> Option<&mut $ret_type> {
                match self {
                    FieldValue::$enum_variant(v) => Some(v),
                    _ => None,
                }
            }
        }
    };
}

impl_primitive_getter!(as_bool, Boolean, bool);
impl_primitive_getter!(as_u8, U8, u8);
impl_primitive_getter!(as_i8, S8, i8);
impl_primitive_getter!(as_c8, C8, u8);
impl_primitive_getter!(as_c16, C16, u16);
impl_primitive_getter!(as_u16, U16, u16);
impl_primitive_getter!(as_i16, S16, i16);
impl_primitive_getter!(as_u32, U32, u32);
impl_primitive_getter!(as_i32, S32, i32);
impl_primitive_getter!(as_u64, U64, u64);
impl_primitive_getter!(as_i64, S64, i64);
impl_primitive_getter!(as_f32, F32, f32);
impl_primitive_getter!(as_f64, F64, f64);
impl_primitive_getter_mut!(as_bool_mut, Boolean, bool);
impl_primitive_getter_mut!(as_m8_mut, U8, u8);
impl_primitive_getter_mut!(as_i8_mut, S8, i8);
impl_primitive_getter_mut!(as_c8_mut, C8, u8);
impl_primitive_getter_mut!(as_c16_mut, C16, u16);
impl_primitive_getter_mut!(as_u16_mut, U16, u16);
impl_primitive_getter_mut!(as_i16_mut, S16, i16);
impl_primitive_getter_mut!(as_u32_mut, U32, u32);
impl_primitive_getter_mut!(as_i32_mut, S32, i32);
impl_primitive_getter_mut!(as_u64_mut, U64, u64);
impl_primitive_getter_mut!(as_i64_mut, S64, i64);
impl_primitive_getter_mut!(as_f32_mut, F32, f32);
impl_primitive_getter_mut!(as_f64_mut, F64, f64);

pub trait TryFromValue<'a>: Sized {
    fn try_from_value(value: &'a FieldValue) -> Option<Self>;
}

pub trait TryFromValueMut<'a>: Sized {
    fn try_from_value_mut(value: &'a mut FieldValue) -> Option<Self>;
}

impl FieldValue {
    pub fn get<'a, T: TryFromValue<'a>>(&'a self) -> Option<T> {
        T::try_from_value(self)
    }

    // Usage: val.get_mut::<&mut u32>() or val.get_mut::<&mut Class>()
    pub fn get_mut<'a, T: TryFromValueMut<'a>>(&'a mut self) -> Option<T> {
        T::try_from_value_mut(self)
    }
}

macro_rules! impl_generic_primitive {
    ($target_type:ty, $enum_variant:ident) => {
        impl<'a> TryFromValue<'a> for $target_type {
            fn try_from_value(value: &'a FieldValue) -> Option<Self> {
                match value {
                    FieldValue::$enum_variant(v) => Some(*v),
                    _ => None,
                }
            }
        }

        impl<'a> TryFromValueMut<'a> for &'a mut $target_type {
            fn try_from_value_mut(value: &'a mut FieldValue) -> Option<Self> {
                match value {
                    FieldValue::$enum_variant(v) => Some(v),
                    _ => None,
                }
            }
        }
    };
}

impl_generic_primitive!(bool, Boolean);
impl_generic_primitive!(i8,   S8);
impl_generic_primitive!(u8,   U8);
impl_generic_primitive!(i16,  S16);
impl_generic_primitive!(u16,  U16);
impl_generic_primitive!(i32,  S32);
impl_generic_primitive!(u32,  U32);
impl_generic_primitive!(i64,  S64);
impl_generic_primitive!(u64,  U64);
impl_generic_primitive!(f32,  F32);
impl_generic_primitive!(f64,  F64);

impl<'a> TryFromValue<'a> for &'a Class {
    fn try_from_value(value: &'a FieldValue) -> Option<Self> {
        match value {
            FieldValue::Class(v) => Some(v),
            _ => None,
        }
    }
}

impl<'a> TryFromValueMut<'a> for &'a mut Class {
    fn try_from_value_mut(value: &'a mut FieldValue) -> Option<Self> {
        match value {
            FieldValue::Class(v) => Some(v),
            _ => None,
        }
    }
}

impl<'a> TryFromValue<'a> for &'a Array {
    fn try_from_value(value: &'a FieldValue) -> Option<Self> {
        match value {
            FieldValue::Array(v) => Some(v),
            _ => None,
        }
    }
}

impl<'a> TryFromValueMut<'a> for &'a mut Array {
    fn try_from_value_mut(value: &'a mut FieldValue) -> Option<Self> {
        match value {
            FieldValue::Array(v) => Some(v),
            _ => None,
        }
    }
}

impl<'a> TryFromValue<'a> for &'a Vec<u16> {
    fn try_from_value(value: &'a FieldValue) -> Option<Self> {
        match value {
            FieldValue::String(v) => Some(&v.0),
            _ => None,
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

    pub fn write<W: Write + Seek>(&self, w: &mut W) -> Result<(), Box<dyn Error>> {
        w.seek_align_up(4)?;
        w.write(&(self.member_type as i32).to_le_bytes())?;
        w.write(&self.member_size.to_le_bytes())?;
        w.write(&(self.values.len() as u32).to_le_bytes())?;
        w.write(&(self.array_type as i32).to_le_bytes())?;
        for e in &self.values {
            match self.array_type {
                ArrayType::Value => {
                    if self.member_type == FieldType::String {
                        if let FieldValue::String(s) = e {
                            w.seek_align_up(4)?;
                            let size = s.0.len() as u32;
                            w.write(&size.to_le_bytes())?;
                            for v in &s.0 {
                                w.write(&v.to_le_bytes())?;
                            }
                        } else {
                            return Err("Expected Array of Strings i think hopefully".into())
                        }
                    } else {
                        e.write_sized(w)?;
                    }
                }
                ArrayType::Class => {
                    e.write(w)?;
                }
            }
        }
        w.seek_align_up(4)?;
        Ok(())
    }

    pub fn get<'a, T: TryFromValue<'a>>(&'a self, index: usize) -> Option<T> {
        self.values.get(index).and_then(|x| x.get::<T>())
    }

    pub fn get_mut<'a, T: TryFromValueMut<'a>>(&'a mut self, index: usize) -> Option<T> {
        self.values.get_mut(index).and_then(|x| x.get_mut::<T>())
    }

    pub fn get_value(&self, index: usize) -> Option<&FieldValue> {
        self.values.get(index)
    }

    pub fn get_value_mut(&mut self, index: usize) -> Option<&mut FieldValue> {
        self.values.get_mut(index)
    }

    pub fn get_as_class(&self, index: usize) -> Option<&Class> {
        self.values.get(index)?.as_class()
    }

    pub fn get_as_class_mut(&mut self, index: usize) -> Option<&mut Class> {
        self.values.get_mut(index)?.as_class_mut()
    }

    pub fn iter(&self) -> impl Iterator<Item = &FieldValue> {
        self.values.iter()
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut FieldValue> {
        self.values.iter_mut()
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

    pub fn write<W: Write + Seek>(&self, w: &mut W) -> Result<(), Box<dyn Error>> {
        w.write(&self.hash.to_le_bytes())?;
        w.write(&(self.field_type as i32).to_le_bytes())?;
        self.value.write(w)?;
        w.seek_align_up(4)?;
        Ok(())
    }

    pub fn get<'a, T: TryFromValue<'a>>(&'a self) -> Option<T> {
        self.value.get::<T>()
    }

    pub fn get_mut<'a, T: TryFromValueMut<'a>>(&'a mut self) -> Option<T> {
        self.value.get_mut::<T>()
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

    pub fn write<W: Write + Seek>(&self, w: &mut W) -> Result<(), Box<dyn Error>> {
        w.write(&self.num_fields.to_le_bytes())?;
        w.write(&self.hash.to_le_bytes())?;
        for (_, field) in &self.fields {
            field.write(w)?;
        }
        Ok(())
    }

    pub fn get_type_info<'a>(&'a self, type_map: &'a TypeMap) -> Option<&'a TypeInfo> {
        type_map.get_by_hash(self.hash)
    }

    pub fn get<'a, T: TryFromValue<'a>>(&'a self, name: &'a str) -> Option<T> {
        self.get_field(name).and_then(|x| x.get::<T>())
    }

    pub fn get_mut<'a, T: TryFromValueMut<'a>>(&'a mut self, name: &'a str) -> Option<T> {
        self.get_field_mut(name).and_then(|x| x.get_mut::<T>())
    }

    pub fn get_field<'a>(&'a self, name: &'a str) -> Option<&'a Field> {
        let hash = murmur3(name, 0xffffffff);
        self.fields.get(&hash)
    }

    pub fn get_field_mut<'a>(&'a mut self, name: &'a str) -> Option<&'a mut Field> {
        let hash = murmur3(name, 0xffffffff);
        self.fields.get_mut(&hash)
    }

    pub fn get_value<'a>(&'a self, name: &'a str) -> Option<&'a FieldValue> {
        self.get_field(name).map(|f| &f.value)
    }

    pub fn get_value_mut<'a>(&'a mut self, name: &'a str) -> Option<&'a mut FieldValue> {
        self.get_field_mut(name).map(|f| &mut f.value)
    }

    pub fn get_subclass<'a>(&'a self, name: &'a str) -> Option<&'a Class> {
        self.get_value(name)?.as_class()
    }

    pub fn get_subclass_mut<'a>(&'a mut self, name: &'a str) -> Option<&'a mut Class> {
        self.get_value_mut(name)?.as_class_mut()
    }

    pub fn get_array<'a>(&'a self, name: &'a str) -> Option<&'a Array> {
        self.get_value(name)?.as_array()
    }

    pub fn get_array_mut<'a>(&'a mut self, name: &'a str) -> Option<&'a mut Array> {
        self.get_value_mut(name)?.as_array_mut()
    }
}

impl TryFrom<&Struct> for Mandrake {
    type Error = Box<dyn Error>;
    fn try_from(value: &Struct) -> Result<Self, Self::Error> {
        if value.data.len() > 16 {
            return Err("Data Length > 16 for Mandrake".into())
        }
        let mut d = Cursor::new(&value.data);
        let v = d.read_i64()?;
        let m = d.read_i64()?;
        Ok(Mandrake {
            v, m
        })
    }
}
