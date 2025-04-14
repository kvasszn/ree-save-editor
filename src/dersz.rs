use core::str;
use std::{
    collections::{HashMap, HashSet}, i128, io::{Read, Seek, Write}, str::FromStr, sync::OnceLock
};

use crate::file_ext::*;

pub static RSZ_FILE: OnceLock<String> = OnceLock::new();
pub static ENUM_FILE: OnceLock<String> = OnceLock::new();

use nalgebra_glm::{Mat4x4, Vec2, Vec3, Vec4};
use serde::{ser::{SerializeSeq, SerializeStruct}, Deserialize, Serialize};
use uuid::Uuid;
use crate::rsz::TypeDescriptor;
use crate::reerr::{Result, FileParseError::*};

// enums to hold values in a lightweight Rsz Struct
#[derive(Debug, Clone)]
pub enum RszType {
    // Numbers
    Null,
    Extern(String),
    Int8(i8),
    Int16(i16),
    Int32(i32),
    Int64(i64),
    UInt8(u8),
    UInt16(u16),
    UInt32(u32),
    UInt64(u64),
    Size(u64),
    F8(u8),
    F16(u16),
    F32(f32),
    F64(f64),

    // Lin alg
    UInt2((u32, u32)),
    UInt3((u32, u32, u32)),
    UInt4((u32, u32, u32, u32)),
    Int2((i32, i32)),
    Int3((i32, i32, i32)),
    Int4((i32, i32, i32, i32)),
    Float2(Vec2),
    Float3(Vec3),
    Float4(Vec4),
    Mat4x4(Mat4x4),
    Vec2(Vec2), // might all be vec4
    Vec3(Vec3),
    Vec4(Vec4),
    Quaternion((f32, f32, f32, f32)),
    Position((f32, f32, f32)),
    Sphere((f32, f32, f32, f32)),
    GameObjectRef([u8; 16]),

    Range((f32, f32)),
    RangeI((i32, i32)),

    // Shapes
    AABB((f32, f32, f32, f32, f32, f32, f32, f32)),
    Capsule((Vec3, Vec3, Vec3)),
    // ...
    Rect((u32, u32, u32, u32)),
    Color((u8, u8, u8, u8)),
    Bool(bool),
    String(String),
    Guid([u8; 16]),
    Array(Vec<RszType>),
    Object(RszStruct<RszField>, u32),
    RuntimeType(String),
    Struct(RszStruct<RszType>),
    Enum(Box<RszType>, String),
    OBB,
    Data(Vec<u8>),

    Nullable(Box<Option<RszType>>, String, String, String, String),
}

impl RszType {
    
    pub fn write_to(&self, data: &mut Vec<u8>, field_info: &RszField, base_addr: usize, is_array: bool) -> Result<()>{
        let align = field_info.align as usize;
        if !is_array && (data.len() + base_addr) % align != 0 {
            data.extend(vec![0; align - (data.len() + base_addr) % align]);
        }
        match self {
            RszType::Bool(x) => {data.write(&(*x as u8).to_le_bytes())?;},
            RszType::Int8(x) => {data.write(&x.to_le_bytes())?;},
            RszType::Int16(x) => {data.write(&x.to_le_bytes())?;},
            RszType::Int32(x) => {data.write(&x.to_le_bytes())?;},
            RszType::Int64(x) => {data.write(&x.to_le_bytes())?;},
            RszType::UInt8(x) => {data.write(&x.to_le_bytes())?;},
            RszType::UInt16(x) => {data.write(&x.to_le_bytes())?;},
            RszType::UInt32(x) => {data.write(&x.to_le_bytes())?;},
            RszType::UInt64(x) => {data.write(&x.to_le_bytes())?;},
            RszType::Size(x) => {data.write(&x.to_le_bytes())?;},
            RszType::F8(x) => {data.write(&x.to_le_bytes())?;},
            RszType::F16(x) => {data.write(&x.to_le_bytes())?;},
            RszType::F32(x) => {data.write(&x.to_le_bytes())?;},
            RszType::F64(x) => {data.write(&x.to_le_bytes())?;},

            RszType::UInt2(x) => { 
                data.write(&x.0.to_le_bytes())?;
                data.write(&x.1.to_le_bytes())?;
            },
            RszType::UInt3(x) => { 
                data.write(&x.0.to_le_bytes())?;
                data.write(&x.1.to_le_bytes())?;
                data.write(&x.2.to_le_bytes())?;
            },
            RszType::UInt4(x) => { 
                data.write(&x.0.to_le_bytes())?;
                data.write(&x.1.to_le_bytes())?;
                data.write(&x.2.to_le_bytes())?;
                data.write(&x.3.to_le_bytes())?;
            },
            RszType::Int2(x) => { 
                data.write(&x.0.to_le_bytes())?;
                data.write(&x.1.to_le_bytes())?;
            },
            RszType::Int3(x) => { 
                data.write(&x.0.to_le_bytes())?;
                data.write(&x.1.to_le_bytes())?;
                data.write(&x.2.to_le_bytes())?;
            },
            RszType::Int4(x) => { 
                data.write(&x.0.to_le_bytes())?;
                data.write(&x.1.to_le_bytes())?;
                data.write(&x.2.to_le_bytes())?;
                data.write(&x.3.to_le_bytes())?;
            },
            RszType::Vec3(x) => {
                data.write(&x.x.to_le_bytes())?;
                data.write(&x.y.to_le_bytes())?;
                data.write(&x.z.to_le_bytes())?;
                if (data.len() + base_addr) % 16 != 0 {
                    data.extend(vec![0; 16 - (data.len() + base_addr) % 16]);
                }
            },
            RszType::Vec4(x) => {
                data.write(&x.x.to_le_bytes())?;
                data.write(&x.y.to_le_bytes())?;
                data.write(&x.z.to_le_bytes())?;
                data.write(&x.w.to_le_bytes())?;
                if (data.len() + base_addr) % 16 != 0 {
                    data.extend(vec![0; 16 - (data.len() + base_addr) % 16]);
                }
            },

            RszType::Color(x) => {
                data.write(&x.0.to_le_bytes())?;
                data.write(&x.1.to_le_bytes())?;
                data.write(&x.2.to_le_bytes())?;
                data.write(&x.3.to_le_bytes())?;
            },

            RszType::Guid(x) => {data.write(x)?;},
            RszType::Struct(x) => {
                if let Some(mapped_hash) = RszDump::name_map().get(&field_info.original_type) {
                    if let Some(r#struct) = RszDump::rsz_map().get(&mapped_hash) {
                        for (field, field_info) in x.fields.iter().zip(r#struct.fields.iter()) {
                            field.write_to(data, field_info, base_addr, field_info.array)?
                        }
                    } else {
                        return Err(format!("Name hash not in hash map {:X}", mapped_hash).into())
                    };
                }
            },

            RszType::String(x) => {
                let buf: Vec<u16> = x.encode_utf16().collect::<Vec<u16>>();
                let len: u32 = buf.len() as u32;
                data.write(&len.to_le_bytes())?;
                for c in buf {
                    data.write(&c.to_le_bytes())?;
                }
            }
            RszType::Object(_, x) => {data.write(&x.to_le_bytes())?;},
            RszType::Enum(x, _) => {
                x.write_to(data, field_info, base_addr, false)?;
            },
            RszType::Array(x) => {
                data.write(&(x.len() as u32).to_le_bytes())?;
                for v in x {
                    v.write_to(data, field_info, base_addr, false)?;
                }
            },
            _ => panic!("not implemented type serialization {:?}", self),
        };
        Ok(())
    }

    fn from_field<F: Read + Seek>(data: &mut F, field: &RszField) -> Result<RszType> {
        data.seek_align_up(field.align.into())?;
        let r#type = match field.r#type.as_str() {
            "S8" => RszType::Int8(data.read_i8()?),
            "S16" => RszType::Int16(data.read_i16()?),
            "S32" => RszType::Int32(data.read_i32()?),
            "S64" => RszType::Int64(data.read_i64()?),
            "U8" => RszType::UInt8(data.read_u8()?),
            "U16" => RszType::UInt16(data.read_u16()?),
            "U32" => RszType::UInt32(data.read_u32()?),
            "U64" => RszType::UInt64(data.read_u64()?),
            "F8" => RszType::F8(data.read_u8()?),
            "F16" => RszType::F16(data.read_u16()?),
            "F32" => RszType::F32(data.read_f32()?),
            "F64" => RszType::F64(data.read_f64()?),

            "Uint2" => RszType::UInt2((data.read_u32()?, data.read_u32()?)),
            "Uint3" => RszType::UInt3((data.read_u32()?, data.read_u32()?, data.read_u32()?)),
            "Uint4" => RszType::UInt4((data.read_u32()?, data.read_u32()?, data.read_u32()?, data.read_u32()?)),
            "Color" => RszType::Color((data.read_u8()?, data.read_u8()?, data.read_u8()?, data.read_u8()?)),
            "Int2" => RszType::Int2((data.read_i32()?, data.read_i32()?)),
            "Int3" => RszType::Int3((data.read_i32()?, data.read_i32()?, data.read_i32()?)),
            "Int4" => RszType::Int4((data.read_i32()?, data.read_i32()?, data.read_i32()?, data.read_i32()?)),
            "Vec2" => {
                let x = RszType::Vec2(data.read_f32vec2()?);
                data.seek_align_up(16)?;
                //data.seek(std::io::SeekFrom::Current(8))?;
                x
            },
            "Vec3" => {
                let x = RszType::Vec3(data.read_f32vec3()?);
                data.seek_align_up(16)?;
                //data.seek(std::io::SeekFrom::Current(4))?;
                x
            },
            "Vec4" => RszType::Vec4(data.read_f32vec4()?),
            "Quaternion" => RszType::Quaternion((data.read_f32()?, data.read_f32()?, data.read_f32()?, data.read_f32()?)),
            "Sphere" => RszType::Sphere((data.read_f32()?, data.read_f32()?, data.read_f32()?, data.read_f32()?)),
            "Position" => RszType::Position((data.read_f32()?, data.read_f32()?, data.read_f32()?)),
            "Float2" => {
                let x = data.read_f32vec2()?;
                //data.seek_align_up(16)?;
                RszType::Float2(x)
            },
            "Float3" => {
                let read_f32vec3 = data.read_f32vec3();
                //data.seek_align_up(16)?;
                RszType::Float3(read_f32vec3?)
            },
            "Float4" => RszType::Float4(data.read_f32vec4()?),
            "Mat4" => RszType::Mat4x4(data.read_f32m4x4()?),

            "Range" => RszType::Range((data.read_f32()?, data.read_f32()?)),
            "RangeI" => RszType::RangeI((data.read_i32()?, data.read_i32()?)),

            "Data" => {
                let mut v = Vec::new();
                //let n = data.read_u32();
                for _ in 0..field.size {
                    v.push(data.read_u8()?);
                }
                RszType::Data(v)
            },
            "AABB" => {
                RszType::AABB((data.read_f32()?, data.read_f32()?, data.read_f32()?, data.read_f32()?,
                data.read_f32()?, data.read_f32()?, data.read_f32()?, data.read_f32()?))
            },
            "Capsule" => {
                RszType::Capsule((data.read_f32vec3()?, data.read_f32vec3()?, data.read_f32vec3()?))
            },
            "Rect" => {
                RszType::Rect((data.read_u32()?, data.read_u32()?, data.read_u32()?, data.read_u32()?))
            },
            "OBB" => {
                data.seek_relative(field.size.into())?;
                RszType::OBB
            },
            "Guid" => {
                let mut buf = [0; 16];
                for i in 0..16 {
                    buf[i] = data.read_u8()?;
                }
                RszType::Guid(buf) // make it read ?????? idek what this comment means
            },
            "GameObjectRef" => {
                let mut buf = [0; 16];
                for i in 0..16 {
                    buf[i] = data.read_u8()?;
                }
                RszType::GameObjectRef(buf)
            },
            "Bool" => {
                //println!("{:?}, {}", field.r#type, field.name);
                RszType::Bool(data.read_bool().unwrap_or(false))
            }
            "String" | "Resource" => {
                //println!("{:x}", data.stream_position()?);
                RszType::String(data.read_utf16str()?)
            },
            "Struct" => {
                let x;
                let r#og_type = &field.original_type.replace("[]", "");
                if let Some(mapped_hash) = RszDump::name_map().get(r#og_type) {
                    if let Some(r#struct) = RszDump::rsz_map().get(&mapped_hash) {
                        let v = RszDump::parse_struct(data, TypeDescriptor{hash: *mapped_hash, crc: r#struct.crc})?;
                        x = RszType::Struct(v)
                    } else {
                        return Err(format!("Name hash not in hash map {:X}", mapped_hash).into())
                    };
                } else if r#og_type.contains("System.Nullable") {
                    //"System.Nullable`1[[via.vec3, System.Private.CoreLib, Version=1.0.0.0, Culture=neutral, PublicKeyToken=null]][];
                    // case 1 `1[[
                    let stripped = r#og_type.replace("System.Nullable`1[[", "").replace("]]", "");
                    let splitted: Vec<&str> = stripped.split(", ").collect();
                    let r#type = *splitted.get(0).unwrap();
                    let lib = *splitted.get(1).unwrap();
                    let version = *splitted.get(2).unwrap();
                    let culture = *splitted.get(3).unwrap();
                    let public_key_token = *splitted.get(4).unwrap();
                    println!("{} {} {} {} {}", r#type, lib, version, culture, public_key_token);
                    let _hash = RszDump::name_map().get(&r#type.to_string()).unwrap();
                    let is_null = data.read_u32()? != 0; // idk if this is actually in the right
                                                        // spot at all. could be a u32 or u8?
                    let rsz_value = match r#type {
                        "via.vec3" => {
                            let x = RszType::Vec3(data.read_f32vec3()?);
                            data.seek(std::io::SeekFrom::Current(4))?;
                            x
                        },
                        "via.Quaternion" => {
                            let x = RszType::Quaternion((data.read_f32()?, data.read_f32()?, data.read_f32()?, data.read_f32()?));
                            x
                        }
                        _ => RszType::Null
                    };
                    data.seek_align_up(16)?;
                    x = if !is_null {
                        RszType::Nullable(Box::new(Some(rsz_value)), lib.to_string(), version.to_string(), culture.to_string(), public_key_token.to_string())
                    } else {
                        RszType::Nullable(Box::new(Some(RszType::Null)), lib.to_string(), version.to_string(), culture.to_string(), public_key_token.to_string())
                    }
                } else {
                    return Err(format!("field original type {:?} not in dump map", field).into())
                };
                x
            },
            "Size" => {
                let v = data.read_u64()?;
                RszType::Size(v)
            },
            "RuntimeType" => {
                let size = data.read_u32()?;
                let mut buf = vec![];
                for _ in 0..size {
                    buf.push(data.read_u8()?);
                }
                let rtype = str::from_utf8(&buf)?.to_string();
                RszType::RuntimeType(rtype)
            },
            "Object" | "UserData" => {
                let x;
                if let Some(mapped_hash) = RszDump::name_map().get(&field.original_type) {
                    if let Some(r#struct) = RszDump::rsz_map().get(&mapped_hash) {
                        x = RszType::Object(r#struct.clone(), data.read_u32()?)
                    } else {
                        return Err(format!("Name crc not in hash map {:X}", mapped_hash).into())
                    };
                } else {
                    return Err(format!("field original type {:?} not in dump map", field).into())
                };
                x
            },
            _ => {
                return Err(format!("Type {:?} is not implemented", field.r#type).into())
            }
        };
        let enum_val = enum_map().get(&field.original_type.replace("[]", "")).is_some();
        if  enum_val || field.original_type.ends_with("Serializable") || field.original_type.ends_with("Fixed") 
            || field.original_type.ends_with("Serializable[]") || field.original_type.ends_with("Fixed[]")
            || field.original_type.ends_with("Bit") || field.original_type.ends_with("Bit[]") {
                Ok(RszType::Enum(Box::new(r#type), field.original_type.clone()))
        } else {
                Ok(r#type)
        }
    }

    fn from_json(data: &serde_json::Value, field: &RszField, parent: &RszStruct<RszField>, objects: &mut Vec<RszValue>) -> Result<RszType> {
        //println!("{:?}", field.name);
        let enum_val = enum_map().get(&field.original_type.replace("[]", "")).is_some();
        if enum_val || field.original_type.ends_with("Serializable") || field.original_type.ends_with("Fixed") 
            || field.original_type.ends_with("Serializable[]") || field.original_type.ends_with("Fixed[]")
            || field.original_type.ends_with("Bit") || field.original_type.ends_with("Bit[]") {
                //let enum_name: String = serde_json::from_value(data.clone())?;
                let enum_name: Option<String> = match &data {
                    serde_json::Value::String(s) => Some(s.clone()),
                    _ => None, // or Some("default".to_string()) if you want a fallback
                };
                if let Some(enum_name) = enum_name {
                    let enum_num = get_enum_val(&field.original_type, &enum_name).expect("Could not find enum name");
                    let rsz_type = match field.r#type.as_str() {
                        "S8" => RszType::Int8(enum_num as i8),
                        "S16" => RszType::Int16(enum_num as i16),
                        "S32" => RszType::Int32(enum_num as i32),
                        "S64" => RszType::Int64(enum_num as i64),
                        "U8" => RszType::UInt8(enum_num as u8),
                        "U16" => RszType::UInt16(enum_num as u16),
                        "U32" => RszType::UInt32(enum_num as u32),
                        "U64" => RszType::UInt64(enum_num as u64),
                        "Object" => {
                            if let Some(mapped_hash) = RszDump::name_map().get(&field.original_type) {
                                if let Some(r#struct) = RszDump::rsz_map().get(&mapped_hash) {
                                    // println!("{:#?}", r#struct);
                                    let mut field_values = vec![];
                                    // lazy but wahtever
                                    //println!("{:?}", r#struct.fields[0]);
                                    let rsz_type = match r#struct.fields[0].r#type.as_str() {
                                        "S8" => RszType::Int8(enum_num as i8),
                                        "S16" => RszType::Int16(enum_num as i16),
                                        "S32" => RszType::Int32(enum_num as i32),
                                        "S64" => RszType::Int64(enum_num as i64),
                                        "U8" => RszType::UInt8(enum_num as u8),
                                        "U16" => RszType::UInt16(enum_num as u16),
                                        "U32" => RszType::UInt32(enum_num as u32),
                                        "U64" => RszType::UInt64(enum_num as u64),
                                        _ => RszType::UInt64(0),
                                    };
                                    field_values.push(rsz_type);
                                    //println!("field_values: {:?}, {enum_name}, {enum_num}", field_values);
                                    let struct_value = RszValue {
                                        name: r#struct.name.clone(),
                                        crc: r#struct.crc,
                                        fields: field_values,
                                    };
                                    objects.push(struct_value);
                                    return Ok(RszType::Object(r#struct.clone(), objects.len() as u32));
                                } else {
                                    return Err(format!("Name crc not in hash map {:X}", mapped_hash).into())
                                };
                            } else {
                                return Err(format!("field original type {:?} not in dump map", field).into())
                            };
                        },
                        _ => {
                            return Err(format!("Type {:?} is not implemented", field.r#type).into())
                        }
                        y  => return Err(format!("Unknown Enum Type, {:?}, {:?}", y, enum_name).into())
                    };
                    return Ok(RszType::Enum(Box::new(rsz_type), field.original_type.clone()))
                }
            }
        let r#type = match field.r#type.as_str() {
            "S8" => RszType::Int8(read_json_int(data)? as i8),
            "S16" => RszType::Int16(read_json_int(data)? as i16),
            "S32" => RszType::Int32(data.as_i64().unwrap() as i32),
            "S64" => RszType::Int64(data.as_i64().unwrap() as i64),
            "U8" => RszType::UInt8(data.as_u64().unwrap() as u8),
            "U16" => RszType::UInt16(data.as_u64().unwrap() as u16),
            "U32" => RszType::UInt32(data.as_u64().unwrap() as u32),
            "U64" => RszType::UInt64(data.as_u64().unwrap() as u64),
            "F8" => RszType::F8(data.as_f64().unwrap() as u8),
            "F16" => RszType::F16(data.as_f64().unwrap() as u16),
            "F32" => RszType::F32(data.as_f64().unwrap() as f32),
            "F64" => RszType::F64(data.as_f64().unwrap() as f64),

            "Uint2" => RszType::UInt2(serde_json::from_value(data.clone())?),
            "Uint3" => RszType::UInt3(serde_json::from_value(data.clone())?),
            "Uint4" => RszType::UInt4(serde_json::from_value(data.clone())?),
            "Color" => RszType::Color(serde_json::from_value(data.clone())?),
            "Int2" => RszType::Int2(serde_json::from_value(data.clone())?),
            "Int3" => RszType::Int3(serde_json::from_value(data.clone())?),
            "Int4" => RszType::Int4(serde_json::from_value(data.clone())?),
            "Vec2" => RszType::Vec2(serde_json::from_value(data.clone())?),
            "Vec3" => RszType::Vec3(serde_json::from_value(data.clone())?),
            "Vec4" => RszType::Vec4(serde_json::from_value(data.clone())?),
            "Quaternion" => RszType::Quaternion(serde_json::from_value(data.clone())?),
            "Sphere" => RszType::Sphere(serde_json::from_value(data.clone())?),
            "Position" => RszType::Position(serde_json::from_value(data.clone())?),
            "Float2" => RszType::Float2(serde_json::from_value(data.clone())?),
            "Float3" => RszType::Float3(serde_json::from_value(data.clone())?),
            "Float4" => RszType::Float4(serde_json::from_value(data.clone())?),
            "Mat4" => RszType::Mat4x4(serde_json::from_value(data.clone())?),
            "Range" => RszType::Range(serde_json::from_value(data.clone())?),
            "RangeI" => RszType::RangeI(serde_json::from_value(data.clone())?),
            "Data" => RszType::Data(serde_json::from_value(data.clone())?),
            "AABB" => RszType::AABB(serde_json::from_value(data.clone())?),
            "Capsule" => RszType::Capsule(serde_json::from_value(data.clone())?),
            "Rect" => RszType::Rect(serde_json::from_value(data.clone())?),
            "OBB" => RszType::OBB,
            "Guid" => {
                let val: String = serde_json::from_value(data.clone())?;
                let uuid: Uuid = Uuid::from_str(&val)?;
                let buf = uuid.to_bytes_le();
                RszType::Guid(buf)
            },
            "GameObjectRef" => {
                let val: String = serde_json::from_value(data.clone())?;
                let uuid: Uuid = Uuid::from_str(&val)?;
                let buf = uuid.to_bytes_le();
                RszType::Guid(buf)
            },
            "Bool" => RszType::Bool(serde_json::from_value(data.clone())?),
            "String" | "Resource" => RszType::String(serde_json::from_value(data.clone())?),
            "Struct" => {
                let x;
                let r#og_type = &field.original_type.replace("[]", "");
                if let Some(mapped_hash) = RszDump::name_map().get(r#og_type) {
                    if let Some(r#struct) = RszDump::rsz_map().get(&mapped_hash) {
                        let v = RszDump::parse_struct_from_json(data, *mapped_hash, objects)?;
                        x = RszType::Struct(v)
                    } else {
                        return Err(format!("Name hash not in hash map {:X}", mapped_hash).into())
                    };
                } else {
                    return Err(format!("field original type {:?} not in dump map", field).into())
                };
                x
            },
            "Size" => RszType::Size(serde_json::from_value(data.clone())?),
            "RuntimeType" => RszType::RuntimeType(serde_json::from_value(data.clone())?),
            "Object" | "UserData" => {
                // I should make a vector of objects exist in the context that I append a new
                // object each time I encounter one
                // then i read the object based on the type information, and return an object at an index
                let og_type = if field.original_type == "ace.user_data.ExcelUserData.cData" {
                    //println!("HERE\n\n");
                    parent.name.clone() + ".cData"
                } else {
                    field.original_type.clone()
                };
                let x;
                if let Some(mapped_hash) = RszDump::name_map().get(&og_type) {
                    if let Some(r#struct) = RszDump::rsz_map().get(&mapped_hash) {
                        //println!("{:?}", r#struct);
                        let v = RszDump::parse_struct_from_json(data, *mapped_hash, objects)?;
                        objects.push(v);
                        x = RszType::Object(r#struct.clone(), objects.len() as u32); // make this zero an actual
                                                                                     // value, and read some other
                                                                                     // important info
                    } else {
                        return Err(format!("Name crc not in hash map {:X}", mapped_hash).into())
                    };
                } else {
                    return Err(format!("field original type {:?} not in dump map", field).into())
                };
                x
            },
            _ => {
                return Err(format!("Type {:?} is not implemented", field.r#type).into())
            }
        };
        Ok(r#type)
    }
}


impl TryInto<String> for RszType {
    type Error = &'static str;
    fn try_into(self) -> std::result::Result<String, Self::Error> {
        use RszType::*;
        match self {
            UInt8(v) => Ok(v.to_string()),
            UInt16(v) => Ok(v.to_string()),
            UInt32(v) => Ok(v.to_string()),
            UInt64(v) => Ok(v.to_string()),
            Int8(v) => Ok(v.to_string()),
            Int16(v) => Ok(v.to_string()),
            Int32(v) => Ok(v.to_string()),
            Int64(v) => Ok(v.to_string()),
            _ => {
                Err("Enum type cannot be converted to string")
            }
        }
    }
}

#[derive(Debug, Clone)]
struct RszSerializerContext<'a> {
    structs: &'a Vec<RszValue>,
    parent_ptr: u32,
}

pub struct RszTypeWithContext<'a>(&'a RszType, &'a RszSerializerContext<'a>);

impl<'a> Serialize for RszTypeWithContext<'a> {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer 
    {
        let rsz_type = self.0;
        let context = self.1;
        let parent_struct = &context.structs[context.parent_ptr as usize];
        let parent_name = &parent_struct.name;
        //println!("{rsz_type:?}");
        use RszType::*;
        return match rsz_type {
            Extern(path) => {
                serializer.serialize_str(path)
            },
            Int8(v) => serializer.serialize_i8(*v), 
            Int16(v) => serializer.serialize_i16(*v), 
            Int32(v) => serializer.serialize_i32(*v), 
            Int64(v) => serializer.serialize_i64(*v), 
            UInt8(v) => serializer.serialize_u8(*v), 
            UInt16(v) => serializer.serialize_u16(*v), 
            UInt32(v) => serializer.serialize_u32(*v), 
            UInt64(v) => serializer.serialize_u64(*v), 
            Size(v) => serializer.serialize_u64(*v), 
            Bool(v) => serializer.serialize_bool(*v),
            String(v) => serializer.serialize_str(v),
            F8(v) => serializer.serialize_u8(*v), 
            F16(v) => serializer.serialize_u16(*v), 
            F32(v) => serializer.serialize_f32(*v), 
            F64(v) => serializer.serialize_f64(*v),
            Vec2(v) => v.serialize(serializer),
            Vec3(v) => v.serialize(serializer),
            Vec4(v) => v.serialize(serializer),
            UInt2(v) => v.serialize(serializer),
            UInt3(v) => v.serialize(serializer),
            UInt4(v) => v.serialize(serializer),
            Int2(v) => v.serialize(serializer),
            Int3(v) => v.serialize(serializer),
            Int4(v) => v.serialize(serializer),
            Float2(v) => v.serialize(serializer),
            Float3(v) => v.serialize(serializer),
            Float4(v) => v.serialize(serializer),
            Mat4x4(v) => v.serialize(serializer),
            RszType::Color(v) => v.serialize(serializer),
            RszType::Quaternion(v) => v.serialize(serializer),
            RszType::Sphere(v) => v.serialize(serializer),
            RszType::Position(v) => v.serialize(serializer),

            Range(v) => v.serialize(serializer),
            RangeI(v) => v.serialize(serializer),
            AABB(v) => v.serialize(serializer),
            Capsule(v) => v.serialize(serializer),
            Rect(v) => v.serialize(serializer),
            Guid(id) => {
                let id = Uuid::from_bytes_le(*id);
                serializer.serialize_str(&id.to_string().as_str())
            },
            RszType::Struct(r#struct) => {
                let struct_info = RszDump::rsz_map().get(RszDump::name_map().get(&r#struct.name).unwrap()).expect("Could not find struct in dump");

                if let Some(RszType::Extern(path)) = r#struct.fields.get(0) {
                    let mut state = serializer.serialize_struct("RszValue", 1)?;
                    state.serialize_field(&struct_info.name, &path)?;
                    return state.end()
                }

                let mut state = serializer.serialize_struct("RszValue", r#struct.fields.len())?;
                for i in 0..r#struct.fields.len() {
                    let field_value = &r#struct.fields[i];
                    let field_info = &struct_info.fields[i];
                    let name = &field_info.name;
                    let ctx = RszSerializerContext {
                        structs: context.structs,
                        parent_ptr: context.parent_ptr,
                    };
                    let serialize_context = RszTypeWithContext(field_value, &ctx);
                    state.serialize_field(name, &serialize_context)?;
                }
                state.end()
            },
            RuntimeType(v) => {
                v.serialize(serializer)
            }
            Object(_struct_info, ptr) => {
                if *ptr == context.parent_ptr {
                    return Err(serde::ser::Error::custom("Detected Recursion in Objects, RSZ dump could be for an old version, or the RSZ data is corrupted"))
                } 
                match context.structs.get(*ptr as usize) {
                    Some(struct_derefed) => {
                        // why not just use passed on struct info???
                        let struct_info = RszDump::rsz_map().get(RszDump::name_map().get(&struct_derefed.name).unwrap()).expect("Could not find struct in dump");

                        if let Some(RszType::Extern(path)) = struct_derefed.fields.get(0) {
                            let mut state = serializer.serialize_struct("RszValue", 1)?;
                            state.serialize_field(&struct_info.name, &path)?;
                            return state.end();
                        }

                        // Handle bitset, ace.Bitset`1<>
                        if let Some(r#type) = struct_info.name.strip_prefix("ace.Bitset`1<") {
                            let mut r#type = r#type.strip_suffix(">").unwrap().to_string(); // should be there, if not idk
                            let is_bit = if enum_map().get(&(r#type.clone() + "Bit")).is_some() {
                                r#type = r#type + "Bit";
                                true
                            } else { false };

                            let values = &struct_derefed.fields[0];
                            if let Int32(max) = &struct_derefed.fields[1] {
                                if let Array(values) = values {
                                    let values = values.iter().enumerate().filter_map(|(i, x)| {
                                        if let UInt32(val) = x {
                                            return Some((0..32).filter_map(|j| {
                                                if *val & (1 << j) != 0 && (i*32 + j < *max as usize){ 
                                                    Some((i * 32 + j) as u32)
                                                } else { None }
                                            }).collect::<Vec<_>>());
                                        } else {None}
                                    }).flatten().collect::<Vec<_>>();
                                    let mut state = serializer.serialize_seq(Some(values.len()))?;

                                    for val in &values {
                                        let val = if is_bit {2u32.pow(*val)} else { *val };
                                        match get_enum_name(&r#type, &(val).to_string()) {
                                            Some(enum_name) => state.serialize_element(&enum_name)?,
                                            None => state.serialize_element(&val.to_string())?,
                                        }
                                    }
                                    //  state.serialize_element(format!("{max}").as_str())?;
                                    return state.end();
                                }
                            }
                        }

                        let ctx = RszSerializerContext {
                            structs: context.structs,
                            parent_ptr: *ptr,
                        };

                        //enumerable param app.cEnumerableParam`2<>
                        if let Some(r#type) = parent_name.strip_prefix("app.cEnumerableParam`2<") {
                            let r#type = r#type.strip_suffix(">").unwrap().split(",").collect::<Vec<&str>>();
                            let enum_type = r#type[0];
                            let mut state = serializer.serialize_struct("app.cEnumerableParam", struct_derefed.fields.len())?;
                            for i in 0..struct_derefed.fields.len() {
                                if struct_info.fields[i].name.contains("EnumValue") {
                                    let enum_val = &struct_derefed.fields[i];
                                    if let Int32(enum_val) = enum_val {
                                        match get_enum_name(&enum_type, &enum_val.to_string()) {
                                            Some(enum_name) => state.serialize_field(&struct_info.fields[i].name, &enum_name)?,
                                            None => state.serialize_field(&struct_info.fields[i].name, &enum_val.to_string())?,
                                        }
                                    }
                                } else {
                                    let field_value = &struct_derefed.fields[i];
                                    let serialize_context = RszTypeWithContext(&field_value, &ctx);
                                    state.serialize_field(&struct_info.fields[i].name, &serialize_context)?;
                                }
                            }
                            return state.end();
                        }


                        let mut state = serializer.serialize_struct("RszValue", struct_info.fields.len())?;
                        for i in 0..struct_info.fields.len() {
                            let field_value = &struct_derefed.fields[i];
                            let field_info = &struct_info.fields[i];
                            let _og_type = field_info.original_type.as_str();
                            let name = field_info.name.as_str();
                            let serialize_context = RszTypeWithContext(field_value, &ctx);
                            state.serialize_field(name, &serialize_context)?;
                        }
                        state.end()
                    }
                    None => {
                        eprintln!("{rsz_type:?}");
                        Err(serde::ser::Error::custom("Could not find Object pointer in data"))
                    }
                }
            },
            RszType::Data(val) => {
                val.serialize(serializer)
            },
            Enum(underlying, name) => {

                let str_enum_name = |name: &str, val: &dyn ToString| { 
                    match get_enum_name(name, &val.to_string()) {
                        //None => format!("{} // Could not find enum value in map {}", name, val.to_string()),
                        None => format!("NULL_BIT_ENUM_OR_COULD_NOT_FIND[{}]", val.to_string()),
                        Some(value) => value
                    }
                };
                let underlying = *underlying.clone();
                match underlying {
                    Object(_info, ptr) => {
                        let res = &context.structs.get(ptr as usize);
                        let struct_derefed = match res {
                            Some(struct_derefed) => {
                                struct_derefed
                            }
                            None => {
                                eprintln!("{rsz_type:?}");
                                return Err(serde::ser::Error::custom("Could not find Enum Object pointer in data"))
                            }
                        };

                        if struct_derefed.fields.len() == 0 {
                            return serializer.serialize_str(format!("{}, {:?}", ptr, struct_derefed).as_str());
                        }
                        let x = struct_derefed.fields[0].clone();
                        //serializer.serialize_str(format!("{x:?} name goes here").as_str());
                        let v = match &x {
                            RszType::UInt64(v) => Ok(v.to_string()),
                            RszType::UInt32(v) => Ok(v.to_string()),
                            RszType::UInt16(v) => Ok(v.to_string()),
                            RszType::UInt8(v) => Ok(v.to_string()),
                            RszType::Int64(v) => Ok(v.to_string()),
                            RszType::Int32(v) => Ok(v.to_string()),
                            RszType::Int16(v) => Ok(v.to_string()),
                            RszType::Int8(v) => Ok(v.to_string()),
                            RszType::Object(_info, _ptr) => {
                                if *_ptr == context.parent_ptr {
                                    return Err(serde::ser::Error::custom("Detected Recursion in Objects, RSZ dump could be for an old version, or the RSZ data is corrupted"))
                                }
                                let ctx = RszSerializerContext {
                                    structs: context.structs,
                                    parent_ptr: ptr, //has to be se tto the original
                                                     //underlying object of the enum
                                };
                                let serialize_context = RszTypeWithContext(&x, &ctx);
                                return serialize_context.serialize(serializer)
                            },
                            _ => {
                                eprintln!("{rsz_type:?}");
                                Err(serde::ser::Error::custom("Unknown underlying Enum type"))
                            }
                        }?;
                        match get_enum_name(name, &v) {
                            //None => serializer.serialize_str(format!("{v} // Could not find enum value in map {name}").as_str()),
                            None => serializer.serialize_str(format!("NULL_BIT_ENUM_OR_COULD_NOT_FIND[{}]", v.to_string()).as_str()),
                            Some(value) => serializer.serialize_str(&value)
                        }
                    },
                    Int8(_) | Int16(_) | Int32(_) | Int64(_) |
                        UInt8(_) | UInt16(_) | UInt32(_) | UInt64(_) => {
                            let val: std::string::String = underlying.try_into().unwrap();
                            serializer.serialize_str(str_enum_name(&name, &val).as_str())
                        },
                    _ => {
                        eprintln!("{rsz_type:?}");
                        Err(serde::ser::Error::custom("Unknown underlying Enum type"))
                    }
                }
            },
            Array(vec_of_types) => {
                //let struct_derefed = &structs.get(*ptr as usize).expect("Struct not in context");
                let mut state = serializer.serialize_seq(Some(vec_of_types.len()))?;
                for r#type in vec_of_types {
                    let serialize_context = RszTypeWithContext(&r#type, context);
                    state.serialize_element(&serialize_context)?;
                }
                state.end()
            }
            _ => serializer.serialize_str("NOT IMPLEMENTED")
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RszField {
    align: u32,
    array: bool,
    name: String,
    native: bool,
    original_type: String,
    size: u32,
    r#type: String,
}


#[derive(Debug, Clone)]
pub struct RszStruct<T> {
    pub name: String,
    pub crc: u32,
    pub fields: Vec<T>,
}

impl<T> RszStruct<T> {
    pub fn hash(&self) -> Option<&u32> {
        RszDump::name_map().get(&self.name)
    }
}

impl RszStruct<RszField> {
    pub fn to_value(&self, r#type: RszType) -> RszValue {
        RszStruct {
            name: self.name.clone(),
            crc: self.crc,
            fields: vec![r#type]
        }
    }
}

impl<'de> Deserialize<'de> for RszStruct<RszField> {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de> {
            #[derive(Debug, Clone, Deserialize)]
            pub struct RszStructTemp<T> {
                name: String,
                crc: String,
                fields: Vec<T>,
            }
            let mut rsz_struct: RszStructTemp<RszField> = Deserialize::deserialize(deserializer)?;

            for field in &mut rsz_struct.fields {
                if field.original_type == "ace.user_data.ExcelUserData.cData[]" {
                    field.original_type = rsz_struct.name.clone() + ".cData[]"
                }
            }
            let rsz_struct: RszStruct<RszField> = RszStruct {
                name: rsz_struct.name,
                crc: u32::from_str_radix(&rsz_struct.crc, 16).unwrap(),
                fields: rsz_struct.fields
            };
            Ok(rsz_struct)
        }
}

pub type RszValue = RszStruct<RszType>;

impl RszValue {
    pub fn to_buffer(&self, base_addr: usize) -> Result<Vec<u8>> {
        let mut data: Vec<u8> = vec![];
        let struct_info = RszDump::rsz_map().get(self.hash().unwrap()).unwrap();
        for (i, field) in self.fields.iter().enumerate() {
            let field_info = &struct_info.fields[i];
            if field_info.array {
                if (data.len() + base_addr) % 4 as usize != 0 {
                    data.extend(vec![0; 4 - (data.len() + base_addr) % 4 as usize]);
                }
            }
            field.write_to(&mut data, &field_info, base_addr, field_info.array)?;
        }
        Ok(data)
    }
}

pub struct RszValueWithContext<'a>(&'a RszValue, &'a RszSerializerContext<'a>);

impl<'a> Serialize for RszValueWithContext<'a> {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer {
            //println!("{:?}", &self.1);
            let r#struct = self.0;
            let context = self.1;
            let struct_info = RszDump::rsz_map().get(RszDump::name_map().get(&r#struct.name).unwrap()).expect("Could not find struct in dump");
            let mut names = vec![];
            for e in &struct_info.fields {
                names.push(e.name.clone());
            }

            if let Some(RszType::Extern(path)) = r#struct.fields.get(0) {
                let mut state = serializer.serialize_struct("RszValue", 1)?;
                state.serialize_field(&struct_info.name, &path)?;
                return state.end()
            }

            let mut state = serializer.serialize_struct("RszValue", r#struct.fields.len())?;
            for i in 0..struct_info.fields.len() {
                let field_value = &r#struct.fields[i];
                let field_info = &struct_info.fields[i];
                let _og_type = field_info.original_type.as_str();
                let name = field_info.name.as_str();

                let serialize_context = RszTypeWithContext(field_value, context);
                state.serialize_field(name, &serialize_context)?;
            }
            state.end()
        }
}


pub struct RszMap<T>(pub T);

pub type RszMapType = HashMap<String, RszStruct<RszField>>;
pub type RszNameMapType = HashMap<String, u32>;

impl RszMap<RszMapType> {
    pub fn get(&self, hash: &u32) -> Option<&RszStruct<RszField>> {
        let d = format!("{:x}", *hash);
        let x = self.0.get(&d);
        x
    }
}

impl RszMap<RszNameMapType> {
    pub fn get(&self, hash: &String) -> Option<&u32> {
        let x = self.0.get(&hash.to_string());
        x
    }
}

pub struct RszDump;


impl RszDump {
    pub fn parse_struct<'a, F: 'a + Read + Seek>(
        data: &mut F,
        type_descriptor: TypeDescriptor,
    ) -> Result<RszValue> {
        let struct_type = match RszDump::rsz_map().get(&type_descriptor.hash) {
            Some(x) => x,
            None => return Err(Box::new(InvalidRszTypeHash(type_descriptor.hash)))
        };

        //println!("{:?}", struct_type);
        let mut field_values = Vec::new();
        for field in &struct_type.fields {
            if field.array {
                data.seek_align_up(4)?;
                let count = data.read_u32()?;
                //println!("count: {}, {count}", field.name);
                let vals = (0..count).map(|_| {
                    RszType::from_field(data, field)
                }).collect::<Result<Vec<RszType>>>()?;
                field_values.push(RszType::Array(vals));
            } else {
                //println!("name: {}", field.name);
                let r#type = RszType::from_field(data, field)?;
                //println!("{:?}", r#type);
                field_values.push(r#type);
            }
        }
        //println!("{:?}, {:?}", struct_type, field_values);
        Ok(RszValue {
            name: struct_type.name.clone(),
            crc: struct_type.crc,
            fields: field_values,
        })
    }

    pub fn parse_struct_from_json(json_data: &serde_json::Value, type_hash: u32, objects: &mut Vec<RszValue>) -> Result<RszValue> {
        let struct_type = match RszDump::rsz_map().get(&type_hash) {
            Some(x) => x,
            None => return Err(Box::new(InvalidRszTypeHash(type_hash)))
        };
        //println!("{:?}", struct_type);
        let mut field_values: Vec<RszType> = Vec::new();
        for field in &struct_type.fields {
            let field_data = json_data.get(&field.name);
            //println!("{}", field.name);
            match field_data {
                None => return Err(format!("Could not find field in json {:?}", field).into()),
                Some(value) => {
                    if field.array {
                        let mut arr_vals = vec![];
                        let values: Vec<serde_json::Value> = value.as_array().expect("field should be an array").to_vec();
                        for value in &values {
                            arr_vals.push(RszType::from_json(value, &field, &struct_type, objects)?)
                        }
                        field_values.push(RszType::Array(arr_vals))
                    } else {
                        field_values.push(RszType::from_json(value, &field, &struct_type, objects)?)
                    }
                }
            }
        }
        //println!("{:?}, {:?}", struct_type, field_values);
        Ok(RszValue {
            name: struct_type.name.clone(),
            crc: struct_type.crc,
            fields: field_values,
        })
    }

    pub fn rsz_map() -> &'static RszMap<RszMapType> {
        static HASHMAP: OnceLock<RszMap<RszMapType>> = OnceLock::new();
        HASHMAP.get_or_init(|| {
            let file = std::fs::read_to_string(RSZ_FILE.get().unwrap()).unwrap();
            let m: RszMapType = serde_json::from_str(&file).unwrap();
            RszMap(m)
        })
    }

    pub fn name_map() -> &'static RszMap<RszNameMapType> {
        static HASHMAP: OnceLock<RszMap<RszNameMapType>> = OnceLock::new();
        HASHMAP.get_or_init(|| {
            let temp = &Self::rsz_map().0;
            let mut m = HashMap::new();
            for (_key, rsz_struct) in temp {
                let hash = u32::from_str_radix(&_key, 16).unwrap();
                m.insert(rsz_struct.name.clone(), hash);
            }
            RszMap(m)
        })
    }
}


#[derive(Debug, Clone)]
pub struct DeRsz {
    pub offset: usize,
    pub roots: Vec<u32>,
    pub structs: Vec<RszValue>,
    pub extern_idxs: HashSet<u32>,
}

impl<'a> Serialize for DeRsz {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer {
            let mut state = serializer.serialize_seq(Some(self.roots.len()))?;
            for i in 0..self.roots.len() {
                let ptr = self.roots[i];
                let r#struct = self.structs[ptr as usize].clone();
                use serde::ser::Error;
                let hash = r#struct.hash().ok_or(
                    Error::custom(format!("struct hash not found for {}", r#struct.name))
                )?;
                let x = RszDump::rsz_map().get(&hash);
                let name = match x {
                    Some(v) => &v.name,
                    None => "unknown struct?"
                };

                #[derive(Serialize)]
                struct Wrapped<'a> {
                    offset: usize,
                    r#type: &'a str,
                    rsz: &'a RszValueWithContext<'a>,
                }
                let ctx = RszSerializerContext {
                    structs: &self.structs,
                    parent_ptr: ptr,
                };
                let val_with_context = RszValueWithContext(&r#struct, &ctx);
                state.serialize_element(&Wrapped {
                    offset: self.offset,
                    r#type: name,
                    rsz: &val_with_context,
                })?;
            }
            state.end()
    }
}


pub fn enum_val_map() -> &'static EnumMap {
    static HASHMAP: OnceLock<EnumMap> = OnceLock::new();
    HASHMAP.get_or_init(|| {
        let json_data = std::fs::read_to_string("enumtoval.json").unwrap();
        let hashmap: EnumMap = serde_json::from_str(&json_data).unwrap();
        hashmap
    })
}

pub fn get_enum_val(name: &str, enum_str_name: &str) -> Option<i128> {
    let name_tmp = name.replace("[]", "").replace("_Serializable", "_Fixed");
    if let Some(map) = enum_map().get(&name_tmp) {
        if let Some(value) = map.get(enum_str_name){
            if let Ok(value) = value.parse::<i128>() {
                return Some(value)
            }
        }
    }
    let name_tmp = name_tmp.replace("_Fixed", "");
    if let Some(map) = enum_map().get(&name_tmp) {
        if !name_tmp.ends_with("Bit") {
            if let Some(value) = map.get(enum_str_name) {
                if let Ok(value) = value.parse::<i128>() {
                    return Some(value)
                }
            }
        }
    }

    let name = name.replace("_Serializable", "");
    if let Some(map) = enum_map().get(&name) {
        let enum_names: Vec<&str> = enum_str_name.split('|').collect();
        let mut enum_val = 0;
        for e in &enum_names {
            if let Some(value) = map.get(*e) {
                if let Ok(value) = value.parse::<i128>() {
                    enum_val += value;
                } else { // just dip if it doesnt work
                    break
                }
            }
        }
        return Some(enum_val)
    }
    None
}


type EnumMap = HashMap<String, HashMap<String, String>>;

pub fn enum_map() -> &'static EnumMap {
    static HASHMAP: OnceLock<EnumMap> = OnceLock::new();
    HASHMAP.get_or_init(|| {
        let json_data = std::fs::read_to_string(ENUM_FILE.get().unwrap()).unwrap();
        let hashmap: EnumMap = serde_json::from_str(&json_data).unwrap();
        hashmap
    })
}

pub fn get_enum_name(name: &str, value: &str) -> Option<String> {
    let name_tmp = name.replace("[]", "").replace("_Serializable", "_Fixed");
    if let Some(map) = enum_map().get(&name_tmp) {
        if let Some(value) = map.get(value){
            return Some(value.to_string())
        }
    }
    let name_tmp = name_tmp.replace("_Fixed", "");
    if let Some(map) = enum_map().get(&name_tmp) {
        if !name_tmp.ends_with("Bit") {
            if let Some(value) = map.get(value){
                return Some(value.to_string())
            }
        }
    }

    let enum_val: u64 = value.parse().unwrap_or(0);
    let mut flag_enum_names = String::from("");
    let name = name.replace("_Serializable", "");
    if let Some(map) = enum_map().get(&name) {
        for i in 0..64 {
            let mask = 1 << i;
            let bit_val = enum_val & mask;
            if let Some(value) = map.get(&bit_val.to_string()){
                if !flag_enum_names.contains(value) {
                    if flag_enum_names != "" {
                        flag_enum_names += "|";
                    }
                    flag_enum_names += value;
                }
            }
        }
        if flag_enum_names != "" {
            return Some(flag_enum_names.to_string())
        }
    }
    None
}

fn read_json_int(data: &serde_json::Value) -> Result<i64> {
    Ok(data.as_i64().ok_or_else(|| "Expected integer in json value")?)
}

fn read_json_uint(data: &serde_json::Value) -> Result<u64> {
    Ok(data.as_u64().ok_or_else(|| "Expected unsigned integer in json value")?)
}

fn read_json_float(data: &serde_json::Value) -> Result<f64> {
    Ok(data.as_f64().ok_or_else(|| "Expected float in json value")?)
}
