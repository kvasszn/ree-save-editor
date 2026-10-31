use ree_lib::rsz::{FieldInfo, TypeInfo, RszMap};
use ree_lib::types::{StringU16};
use ree_lib::rsz::Value;
use num_enum::TryFromPrimitive;
use serde::Deserialize;
use std::error::Error;
use std::io::{Read, Seek, Write};
use strum::Display;

use ree_lib::util::*;
use util::{WriteAlign, ReadExt, SeekExt};

#[derive(Debug, Clone)]
pub enum Ref {
    Index(usize),
    Field(String),
}

// Some of this stuff comes from via.reflection.TypeKind
#[repr(i32)]
#[derive(Clone, Copy, Debug, Deserialize, TryFromPrimitive, PartialEq, Eq, Display)]
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

impl FieldType {
    pub fn from_field_info(info: &FieldInfo) -> Self {
        if info.array {
            return Self::Array;
        }

        // enums
        if info.original_type.starts_with("System.Enum") {
            return Self::Enum;
        }
        match info.r#type.as_str() {
            "S16" | "S32" | "S64" | "U32" | "U64" => {
                if !info.original_type.starts_with("System.") {
                    return Self::Enum;
                }
            }
            _ => (),
        }

        match info.r#type.as_str() {
            "Bool" => return Self::Boolean,
            "S8" => return Self::S8,
            "S16" => return Self::S16,
            "S32" => return Self::S32,
            "S64" => return Self::S64,
            "U8" => return Self::U8,
            "U16" => return Self::U16,
            "U32" => return Self::U32,
            "U64" => return Self::U64,
            "C8" => return Self::C8,
            "C16" => return Self::C16,
            "F32" => return Self::F32,
            "F64" => return Self::F64,
            "Object" | "UserData" => return Self::Class,
            "Struct" => return Self::Struct,
            "String" => return Self::String,
            _ => return Self::Struct, // probably?
        }
        //return Self::Unknown
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum EnumValue {
    E1(i8),
    E2(i16),
    E4(i32),
    E8(i64),
}

impl EnumValue {
    pub fn as_i64(&self) -> i64 {
        match self {
            Self::E1(v) => *v as i64,
            Self::E2(v) => *v as i64,
            Self::E4(v) => *v as i64,
            Self::E8(v) => *v,
        }
    }
    pub fn as_u64(&self) -> u64 {
        match self {
            Self::E1(v) => *v as u64,
            Self::E2(v) => *v as u64,
            Self::E4(v) => *v as u64,
            Self::E8(v) => *v as u64,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum FieldValue {
    Array(Box<Array>),
    Unknown,
    Enum(EnumValue),
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
    Class(Box<Class>),
}

use std::fmt;

impl fmt::Display for FieldValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Boolean(v) => write!(f, "{}", v),
            Self::S8(v)  => write!(f, "{}", v),
            Self::U8(v)  => write!(f, "{}", v),
            Self::S16(v) => write!(f, "{}", v),
            Self::U16(v) => write!(f, "{}", v),
            Self::S32(v) => write!(f, "{}", v),
            Self::U32(v) => write!(f, "{}", v),
            Self::S64(v) => write!(f, "{}", v),
            Self::U64(v) => write!(f, "{}", v),
            Self::F32(v) => write!(f, "{}", v),
            Self::F64(v) => write!(f, "{}", v),
            Self::C8(v)  => write!(f, "{}", *v as char),
            Self::C16(v) => write!(f, "{}", v),
            Self::Enum(v)   => write!(f, "{:?}", v), 
            Self::Unknown   => write!(f, "Unknown"),
            Self::Array(a)  => write!(f, "<Array len={}>", a.values.len()),
            Self::String(s) => write!(f, "{}", s),
            Self::Struct(v) => write!(f, "<Struct size={}>", v.data.len()),
            Self::Class(v)  => write!(f, "<Class hash={}>", v.hash),
        }
    }
}

impl From<&FieldValue> for Value {
    fn from(value: &FieldValue) -> Self {
        match value {
            FieldValue::S8(v) => Value::S8(*v),
            FieldValue::S16(v) => Value::S16(*v),
            FieldValue::S32(v) => Value::S32(*v),
            FieldValue::S64(v) => Value::S64(*v),
            FieldValue::U8(v) => Value::U8(*v),
            FieldValue::U16(v) => Value::U16(*v),
            FieldValue::U32(v) => Value::U32(*v),
            FieldValue::U64(v) => Value::U64(*v),
            FieldValue::Enum(EnumValue::E1(v)) => Value::S8(*v),
            FieldValue::Enum(EnumValue::E2(v)) => Value::S16(*v),
            FieldValue::Enum(EnumValue::E4(v)) => Value::S32(*v),
            FieldValue::Enum(EnumValue::E8(v)) => Value::S64(*v),
            FieldValue::F32(v) => Value::F32(*v),
            FieldValue::F64(v) => Value::F64(*v),
            FieldValue::Boolean(v) => Value::Bool(*v),
            FieldValue::String(v) => Value::String(*v.clone()),
            _ => Value::Null,
        }
    }
}

impl FieldValue {
    pub fn read<R: Read + Seek>(
        reader: &mut R,
        field_type: FieldType,
    ) -> Result<Self, Box<dyn Error>> {
        let value = match field_type {
            FieldType::Unknown => {
                /*println!("Unknown Field Type found");*/
                return Err("Unknown Field Type".into());
            }
            FieldType::Array => FieldValue::Array(Array::read(reader)?.into()),
            FieldType::Class => FieldValue::Class(Class::read(reader)?.into()),
            FieldType::String => {
                reader.seek_align_up(4)?;
                let size = reader.read_u32()?;
                let data = (0..size)
                    .map(|_| Ok(reader.read_u16()?))
                    .collect::<Result<Vec<u16>, Box<dyn Error>>>()?;
                FieldValue::String(Box::new(StringU16(data)))
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
    pub fn read_sized<R: Read + Seek>(
        reader: &mut R,
        field_type: FieldType,
        size: u32,
    ) -> Result<Self, Box<dyn Error>> {
        if size != 1 {
            let size = size as u64;
            let pos = reader.stream_position()?;
            let new_offset = (pos + size - 1) & !(size - 1);
            reader.seek(std::io::SeekFrom::Start(new_offset))?;
        }
        let value = match field_type {
            FieldType::Enum => {
                let enum_val = match size {
                    1 => EnumValue::E1(reader.read_i8()?),
                    2 => EnumValue::E2(reader.read_i16()?),
                    4 => EnumValue::E4(reader.read_i32()?),
                    8 => EnumValue::E8(reader.read_i64()?),
                    _ => return Err(format!("Invalid Enum size: {}", size).into()),
                };
                FieldValue::Enum(enum_val)
            }
            FieldType::Boolean => FieldValue::Boolean(reader.read_bool()?),
            FieldType::S8 => FieldValue::S8(reader.read_i8()?),
            FieldType::U8 => FieldValue::U8(reader.read_u8()?),
            FieldType::S16 => FieldValue::S16(reader.read_i16()?),
            FieldType::U16 => FieldValue::U16(reader.read_u16()?),
            FieldType::S32 => FieldValue::S32(reader.read_i32()?),
            FieldType::U32 => FieldValue::U32(reader.read_u32()?),
            FieldType::S64 => FieldValue::S64(reader.read_i64()?),
            FieldType::U64 => FieldValue::U64(reader.read_u64()?),
            FieldType::F32 => FieldValue::F32(reader.read_f32()?),
            FieldType::F64 => FieldValue::F64(reader.read_f64()?),
            FieldType::C8 => FieldValue::C8(reader.read_u8()?),
            FieldType::C16 => FieldValue::C16(reader.read_u16()?),
            FieldType::Array => FieldValue::Array(Array::read(reader)?.into()),
            FieldType::Struct => {
                let mut data = vec![0u8; size as usize];
                reader.read_exact(&mut data)?;
                FieldValue::Struct(Box::new(Struct { data }))
            }
            _ => {
                return Err(
                    format!("Unexpected sized read of {:?} for {:?}", size, field_type).into(),
                );
            }
        };
        Ok(value)
    }

    pub fn write<W: Write + Seek>(&self, w: &mut W) -> Result<(), Box<dyn Error>> {
        match self {
            FieldValue::Unknown => {
                panic!("Unknown Field Type found")
            }
            FieldValue::Array(v) => v.write(w),
            FieldValue::Class(v) => v.write(w),
            FieldValue::String(v) => {
                w.write_align_up(4)?;
                w.write_all(&(v.0.len() as u32).to_le_bytes())?;
                for e in &v.0 {
                    w.write_all(&e.to_le_bytes())?;
                }
                Ok(())
            }
            // These values actually need a size/len
            _ => {
                w.write_align_up(4)?;
                let size: u32 = self.get_size();
                w.write_all(&size.to_le_bytes())?;
                self.write_sized(w)?;
                Ok(())
            }
        }
    }

    pub fn write_sized<W: Write + Seek>(&self, w: &mut W) -> Result<(), Box<dyn Error>> {
        let size = self.get_size();
        if size != 1 {
            let size = size as u64;
            let pos = w.stream_position()?;
            let new_offset = (pos + size - 1) & !(size - 1);
            w.write_all(&vec![0u8; (new_offset - pos) as usize])?;
        }
        let _ = match self {
            FieldValue::Enum(v) => match v {
                EnumValue::E1(e) => w.write(&e.to_le_bytes())?,
                EnumValue::E2(e) => w.write(&e.to_le_bytes())?,
                EnumValue::E4(e) => w.write(&e.to_le_bytes())?,
                EnumValue::E8(e) => w.write(&e.to_le_bytes())?,
            },
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
            FieldValue::Array(v) => {
                v.write(w)?;
                0
            }
            FieldValue::Struct(v) => {
                for e in &v.data {
                    w.write_all(&e.to_le_bytes())?;
                }
                v.data.len()
            }
            _ => 0,
        };
        Ok(())
    }

    pub fn get_size(&self) -> u32 {
        match self {
            FieldValue::Enum(v) => match v {
                EnumValue::E1(_) => 1,
                EnumValue::E2(_) => 2,
                EnumValue::E4(_) => 4,
                EnumValue::E8(_) => 8,
            },
            FieldValue::Boolean(_) | FieldValue::U8(_) | FieldValue::S8(_) | FieldValue::C8(_) => 1,
            FieldValue::U16(_) | FieldValue::S16(_) | FieldValue::C16(_) => 2,
            FieldValue::U32(_) | FieldValue::S32(_) | FieldValue::F32(_) => 4,
            FieldValue::U64(_) | FieldValue::S64(_) | FieldValue::F64(_) => 8,
            FieldValue::Struct(v) => v.data.len() as u32,
            _ => 0,
        }
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

    pub fn as_any_u64(&self) -> Option<u64> {
        let val = match self {
            FieldValue::Enum(v) => match v {
                EnumValue::E1(v) => *v as u64,
                EnumValue::E2(v) => *v as u64,
                EnumValue::E4(v) => *v as u64,
                EnumValue::E8(v) => *v as u64,
            },
            FieldValue::S8(v) => *v as u64,
            FieldValue::U8(v) => *v as u64,
            FieldValue::C8(v) => *v as u64,
            FieldValue::S16(v) => *v as u64,
            FieldValue::U16(v) => *v as u64,
            FieldValue::C16(v) => *v as u64,
            FieldValue::S32(v) => *v as u64,
            FieldValue::U32(v) => *v as u64,
            FieldValue::F32(v) => *v as u64,
            FieldValue::S64(v) => *v as u64,
            FieldValue::U64(v) => *v as u64,
            _ => return None,
        };
        Some(val)
    }

    pub fn is_class_same(&self, other: &Self) -> bool {
        let class_hash_same = match (&self, &other) {
            (FieldValue::Class(a), FieldValue::Class(b)) => {
                a.hash == b.hash
            }
            _ => false
        };
        class_hash_same
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
impl_generic_primitive!(i8, S8);
impl_generic_primitive!(u8, U8);
impl_generic_primitive!(i16, S16);
impl_generic_primitive!(u16, U16);
impl_generic_primitive!(i32, S32);
impl_generic_primitive!(u32, U32);
impl_generic_primitive!(i64, S64);
impl_generic_primitive!(u64, U64);
impl_generic_primitive!(f32, F32);
impl_generic_primitive!(f64, F64);

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

impl<'a> TryFromValue<'a> for Vec<u8> {
    fn try_from_value(value: &'a FieldValue) -> Option<Self> {
        match value {
            FieldValue::Array(arr) if arr.member_type == FieldType::U8 => {
                Some(arr.values.iter().filter_map(|v| v.as_u8()).collect())
            }
            FieldValue::Struct(s) => Some(s.data.clone()),
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

#[derive(Debug, Clone, PartialEq)]
pub struct Array {
    pub member_type: FieldType,
    pub member_size: u32,
    pub array_type: ArrayType,
    pub values: Vec<FieldValue>,
    pub hashes: Option<Vec<u32>>
}

impl Array {
    pub fn read<R: Read + Seek>(reader: &mut R) -> Result<Self, Box<dyn Error>> {
        reader.seek_align_up(4)?;
        let member_type = FieldType::try_from(reader.read_i32()?)?;
        let member_size = reader.read_u32()?;
        let len = reader.read_u32()?;
        let array_type = ArrayType::try_from(reader.read_i32()?)?;
        let mut values: Vec<FieldValue> = Vec::with_capacity(len as usize);
        let mut hashes = None;

        if array_type == ArrayType::Class {
            let marker = reader.read_u32()?;
            if marker == 0xffeeffee {
                //log::info!("marker found for {len} classes");
                let mut class_hashes = Vec::new();
                for _i in 0..len {
                    class_hashes.push(reader.read_u32()?);
                }
                hashes = Some(class_hashes);
            } else {
                reader.seek_relative(-4)?;
            }
        }

        for _i in 0..len {
            let value = match array_type {
                ArrayType::Value => {
                    if member_type == FieldType::String {
                        reader.seek_align_up(4)?;
                        let size = reader.read_u32()?;
                        let data = (0..size)
                            .map(|_| Ok(reader.read_u16()?)).collect::<Result<Vec<u16>, Box<dyn Error>>>()?;
                        FieldValue::String(Box::new(StringU16(data)))
                    } else {
                        FieldValue::read_sized(reader, member_type, member_size)?
                    }
                }
                ArrayType::Class => {
                    let class = Class::read(reader)?;
                    FieldValue::Class(class.into())
                }
            };

            values.push(value);
        }
        reader.seek_align_up(4)?;
        Ok(Self {
            member_type,
            member_size,
            array_type,
            values,
            hashes
        })
    }

    pub fn write<W: Write + Seek>(&self, w: &mut W) -> Result<(), Box<dyn Error>> {
        w.write_align_up(4)?;
        w.write_all(&(self.member_type as i32).to_le_bytes())?;
        w.write_all(&self.member_size.to_le_bytes())?;
        w.write_all(&(self.values.len() as u32).to_le_bytes())?;
        w.write_all(&(self.array_type as i32).to_le_bytes())?;

        if self.array_type == ArrayType::Class && let Some(hashes) = &self.hashes {
            w.write_all(&0xffeeffeeu32.to_le_bytes())?;
            for hash in hashes {
                w.write_all(&hash.to_le_bytes())?;
            }
        }

        for e in &self.values {
            match self.array_type {
                ArrayType::Value => {
                    if self.member_type == FieldType::String {
                        if let FieldValue::String(s) = e {
                            w.write_align_up(4)?;
                            let size = s.0.len() as u32;
                            w.write_all(&size.to_le_bytes())?;
                            for v in &s.0 {
                                w.write_all(&v.to_le_bytes())?;
                            }
                        } else {
                            return Err("Expected Array of Strings i think hopefully".into());
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
        w.write_align_up(4)?;
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

#[derive(Debug, Clone, PartialEq)]
pub struct Field {
    pub hash: u32,
    pub field_type: FieldType,
    pub value: FieldValue,
}

impl Field {
    pub fn read<R: Read + Seek>(reader: &mut R) -> Result<Self, Box<dyn Error>> {
        let hash = reader.read_u32()?;
        // I definitely wrote this but kinda forget what it means?
        // maybe that Array should not be checked here? since its not a TypeKind?
        // This migth treat parse Array here, would make sense with the enum not being in
        // reflection::TypeKind

        //let pos = reader.stream_position()?;
        let field_type = FieldType::try_from(reader.read_i32()?)?;
        let value = FieldValue::read(reader, field_type)?;
        /*if hash == 0xEF10B158 && pos == 0x190c {
          println!("hash={hash:x}, {value:?}, ft={field_type:?}");
          }*/
        //println!("v={value:?}");
        seek_align_up(reader, 4)?;
        Ok(Self {
            hash,
            field_type,
            value,
        })
    }

    pub fn write<W: Write + Seek>(&self, w: &mut W) -> Result<(), Box<dyn Error>> {
        w.write_all(&self.hash.to_le_bytes())?;
        w.write_all(&(self.field_type as i32).to_le_bytes())?;
        self.value.write(w)?;
        w.write_align_up(4)?;
        Ok(())
    }

    pub fn get<'a, T: TryFromValue<'a>>(&'a self) -> Option<T> {
        self.value.get::<T>()
    }

    pub fn get_mut<'a, T: TryFromValueMut<'a>>(&'a mut self) -> Option<T> {
        self.value.get_mut::<T>()
    }

    pub fn is_class_same(&self, other: &Self) -> bool {
        self.field_type == other.field_type && self.is_class_same(other)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Struct {
    pub data: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Class {
    pub num_fields: u32,
    pub hash: u32,
    pub fields: Vec<Field>,
}

impl Class {
    pub fn read<R: Read + Seek>(reader: &mut R) -> Result<Self, Box<dyn Error>> {
        let num_fields = reader.read_u32()?;
        let hash = reader.read_u32()?;
        //println!("hash: {hash:0x}, num_fields: {num_fields}");
        let mut fields = Vec::<Field>::new();
        for _ in 0..num_fields {
            let field = Field::read(reader)?;
            //println!("{field:?}");
            fields.push(field);
        }
        Ok(Self {
            num_fields,
            hash,
            fields,
        })
    }

    pub fn write<W: Write + Seek>(&self, w: &mut W) -> Result<(), Box<dyn Error>> {
        w.write_all(&self.num_fields.to_le_bytes())?;
        w.write_all(&self.hash.to_le_bytes())?;
        for field in &self.fields {
            field.write(w)?;
        }
        Ok(())
    }

    pub fn get_type_info<'a>(&'a self, type_map: &'a RszMap) -> Option<&'a TypeInfo> {
        type_map.get_by_hash(self.hash)
    }

    pub fn get<'a, T: TryFromValue<'a>>(&'a self, name: &'a str) -> Option<T> {
        self.get_field(name).and_then(|x| x.get::<T>())
    }

    pub fn get_mut<'a, T: TryFromValueMut<'a>>(&'a mut self, name: &'a str) -> Option<T> {
        self.get_field_mut(name).and_then(|x| x.get_mut::<T>())
    }

    pub fn find<'a>(&'a self, name: &'a str) -> Option<usize> {
        let hash = murmur3(name);
        self.fields.iter().enumerate().find_map(
            |(i, f)| {
                if f.hash == hash { Some(i) } else { None }
            },
        )
    }
    pub fn get_field<'a>(&'a self, name: &'a str) -> Option<&'a Field> {
        let hash = murmur3(name);
        self.fields.iter().find(|f| f.hash == hash)
    }

    pub fn get_field_mut<'a>(&'a mut self, name: &'a str) -> Option<&'a mut Field> {
        let hash = murmur3(name);
        self.fields.iter_mut().find(|f| f.hash == hash)
    }

    pub fn get_value<'a>(&'a self, name: &'a str) -> Option<&'a FieldValue> {
        self.get_field(name).map(|f| &f.value)
    }

    pub fn get_value_mut<'a>(&'a mut self, name: &'a str) -> Option<&'a mut FieldValue> {
        self.get_field_mut(name).map(|f| &mut f.value)
    }

    pub fn get_index<'a>(&'a self, index: usize) -> Option<&'a Field> {
        self.fields.get(index).map(|f| f)
    }

    pub fn get_index_mut<'a>(&'a mut self, index: usize) -> Option<&'a mut Field> {
        self.fields.get_mut(index).map(|f| f)
    }

    pub fn get_index_value<'a>(&'a self, index: usize) -> Option<&'a FieldValue> {
        self.fields.get(index).map(|f| &f.value)
    }

    pub fn get_index_value_mut<'a>(&'a mut self, index: usize) -> Option<&'a mut FieldValue> {
        self.fields.get_mut(index).map(|f| &mut f.value)
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
