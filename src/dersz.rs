use core::str;
use std::{
    collections::{HashMap, HashSet}, io::{BufReader, Read, Seek}, sync::OnceLock, time::Instant
};

use crate::file_ext::*;

pub static RSZ_FILE: OnceLock<String> = OnceLock::new();
pub static ENUM_FILE: OnceLock<String> = OnceLock::new();

use nalgebra_glm::{Mat4x4, Vec2, Vec3, Vec4};
use serde::{ser::{SerializeSeq, SerializeStruct}, Deserialize, Deserializer, Serialize};
use uuid::Uuid;
use crate::rsz::TypeDescriptor;
use crate::reerr::{Result, FileParseError::*};


#[derive(Debug, Clone)]
pub enum ObjectType {
    None,
    EnumerableParam(String),
}

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
    Object(RszStruct<RszField>, u32, ObjectType),
    RuntimeType(String),
    Struct(RszStruct<RszType>),
    Enum(Box<RszType>, String),
    OBB,
    Data(Vec<u8>),

    Nullable(Box<Option<RszType>>, String, String, String, String)
}

impl RszType {
    fn from_field<F: Read + Seek>(data: &mut F, field: &RszField, parent_object: ObjectType) -> Result<RszType> {
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
            "Bool" => RszType::Bool(data.read_bool()?),
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
                let obj_type = if field.original_type.starts_with("app.cEnumerableParam") {
                    let x = field.original_type.strip_prefix("app.cEnumerableParam`2<").unwrap().to_string();
                    let enum_name = x.split(",").collect::<Vec<_>>()[0];
                    ObjectType::EnumerableParam(enum_name.to_string())
                } else if let ObjectType::EnumerableParam(_) = parent_object {
                    parent_object
                }
                else {
                    ObjectType::None
                };

                let x;
                if let Some(mapped_hash) = RszDump::name_map().get(&field.original_type) {
                    if let Some(r#struct) = RszDump::rsz_map().get(&mapped_hash) {
                        x = RszType::Object(r#struct.clone(), data.read_u32()?, obj_type)
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
            || field.original_type.ends_with("Serializable[]") || field.original_type.ends_with("Fixed[]"){
                Ok(RszType::Enum(Box::new(r#type), field.original_type.clone()))
            } else {
                Ok(r#type)
        }
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

pub struct RszTypeWithInfo<'a>(&'a RszType, &'a Vec<RszValue>, Option<&'a RszType>);

impl<'a> Serialize for RszTypeWithInfo<'a> {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer 
    {
        let rsz_type = self.0;
        let structs = self.1;
        use RszType::*;
        return match rsz_type {
            Extern(path) => serializer.serialize_str(path),
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
                let parent_info = match self.2 {
                    Some(v) => v,
                    None => rsz_type,
                };
                let val = RszValueWithInfo(r#struct, structs, Some(parent_info));
                val.serialize(serializer)
            },
            RuntimeType(v) => v.serialize(serializer),
            Object(_info, ptr, _obj_type) => {
                match &structs.get(*ptr as usize) {
                    Some(struct_derefed) => {
                        let parent_info = match self.2 {
                            Some(v) => v,
                            None => rsz_type,
                        };
                        let val = RszValueWithInfo(struct_derefed, structs, Some(parent_info));
                        val.serialize(serializer)
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
                    match get_enum_name(name.to_string(), val.to_string()) {
                            None => format!("{} // Could not find enum value in map {}", name, val.to_string()),
                            Some(value) => value
                        }
                };
                let underlying = *underlying.clone();
                match underlying {
                    Object(_info, ptr, _obj_type) => {
                        let res = &structs.get(ptr as usize);
                        let struct_derefed = match res {
                            Some(struct_derefed) => {
                                struct_derefed
                            }
                            None => {
                                eprintln!("{rsz_type:?}");
                                return Err(serde::ser::Error::custom("Could not find Enum Object pointer in data"))
                            }
                        };

                        /*if struct_derefed.fields.len() == 0 {
                          return serializer.serialize_str(format!("{}, {:?}", ptr, struct_derefed).as_str());
                          }*/
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
                            RszType::Object(_info, ptr, _obj_type) => {
                                match &structs.get(*ptr as usize) {
                                    Some(struct_derefed) => {
                                        let parent_info = match self.2 {
                                            Some(v) => v,
                                            None => rsz_type,
                                        };
                                        let val = RszValueWithInfo(struct_derefed, structs, Some(parent_info));
                                        return val.serialize(serializer)
                                    }
                                    None => {
                                        eprintln!("{rsz_type:?}");
                                        Err(serde::ser::Error::custom("Could not find Object pointer in data"))
                                    }
                                }
                            },
                            _ => {
                                eprintln!("{rsz_type:?}");
                                Err(serde::ser::Error::custom("Unknown underlying Enum type"))
                            }
                        }?;
                        match get_enum_name(name.to_string(), v.clone()){
                            None => serializer.serialize_str(format!("{v} // Could not find enum value in map {name}").as_str()),
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
                    let type_with_context = RszTypeWithInfo(r#type, structs, self.2);
                    state.serialize_element(&type_with_context)?;
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
    native: bool,          // almost always false, except for some via types
    original_type: String, //should also be used to index other structs
    size: u32,
    r#type: String, //basic type of the struct
}


#[derive(Debug, Clone)]
pub struct RszStruct<T> {
    name: String,
    crc: u32,
    fields: Vec<T>,
}

impl RszStruct<RszField> {
    fn hash(&self) -> Option<&u32> {
        RszDump::name_map().get(&self.name)
    }
    pub fn to_value(&self, r#type: RszType) -> RszValue {
        RszStruct {
            name: self.name.clone(),
            crc: self.crc,
            fields: vec![r#type]
        }
    }
}

impl RszValue {
    fn hash(&self) -> Option<&u32> {
        RszDump::name_map().get(&self.name)
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

pub struct RszValueWithInfo<'a>(&'a RszValue, &'a Vec<RszValue>, Option<&'a RszType>);

impl<'a> Serialize for RszValueWithInfo<'a> {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer {
            //println!("{:?}", &self.1);
            let r#struct = self.0;
            let context = self.1;
            let struct_info = RszDump::rsz_map().get(RszDump::name_map().get(&r#struct.name).unwrap()).expect("Could not find struct in dump");
            let mut state = serializer.serialize_struct("RszValue", r#struct.fields.len())?;
            let mut names = vec![];
            for e in &struct_info.fields {
                names.push(e.name.clone());
            }
            if let Some(RszType::Extern(path)) = r#struct.fields.get(0) {
                state.serialize_field(&struct_info.name, &path)?;
                return state.end()
            }
            match self.2 {
                Some(RszType::Object(_, _, ot)) => {
                    for i in 0..struct_info.fields.len() {
                        let field_value = &r#struct.fields[i];
                        let field_info = &struct_info.fields[i];
                        let _og_type = field_info.original_type.as_str();
                        let name = field_info.name.as_str();
                        match ot {
                            ObjectType::EnumerableParam(v) => {
                                if name.contains("EnumValue") {
                                    let val = match field_value {
                                        RszType::UInt32(v) => v.to_string(),
                                        RszType::Int32(v) => v.to_string(),
                                        RszType::UInt64(v) => v.to_string(),
                                        RszType::Int64(v) => v.to_string(),
                                        _ =>  {
                                            let serialize_context = RszTypeWithInfo(field_value, context, self.2);
                                            state.serialize_field(name, &serialize_context)?;
                                            continue;
                                        },
                                    };
                                    let enum_name = get_enum_name(v.to_string(), val);
                                    state.serialize_field(name, &enum_name)?;
                                } else {
                                    let serialize_context = RszTypeWithInfo(&field_value, context, self.2);
                                    state.serialize_field(name, &serialize_context)?;
                                }
                            },
                            ObjectType::None => {
                                let serialize_context = RszTypeWithInfo(&field_value, context, self.2);
                                state.serialize_field(name, &serialize_context)?;
                            }
                        }
                    }
                },
                None | Some(_) => {
                    for i in 0..struct_info.fields.len() {
                        let field_value = &r#struct.fields[i];
                        let field_info = &struct_info.fields[i];
                        let name = &field_info.name;
                        let serialize_context = RszTypeWithInfo(field_value, context, self.2);
                        state.serialize_field(name, &serialize_context)?;
                    }
                }
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
                    RszType::from_field(data, field, ObjectType::None)
                }).collect::<Result<Vec<RszType>>>()?;
                field_values.push(RszType::Array(vals));
            } else {
                //println!("name: {}", field.name);
                let r#type = RszType::from_field(data, field, ObjectType::None)?;
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

    pub fn rsz_map() -> &'static RszMap<RszMapType> {
        static HASHMAP: OnceLock<RszMap<RszMapType>> = OnceLock::new();
        HASHMAP.get_or_init(|| {
            let file = std::fs::File::open(RSZ_FILE.get().unwrap()).unwrap();
            let reader = BufReader::new(file);
            let m: RszMapType = serde_json::from_reader(reader).unwrap();
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
    pub roots: Vec<RszValue>,
    pub structs: Vec<RszValue>,
    pub extern_idxs: HashSet<u32>,
}

impl<'a> Serialize for DeRsz {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer {
            let mut state = serializer.serialize_struct("Rsz", self.roots.len())?;
            let context = self.structs.clone();
            for i in 0..self.roots.len() {
                let r#struct = self.roots[i].clone();
                use serde::ser::Error;
                let hash = r#struct.hash().ok_or(
                    Error::custom(format!("struct hash not found for {}", r#struct.name))
                )?;
                let x = RszDump::rsz_map().get(&hash);
                let name = match x {
                    Some(v) => &v.name,
                    None => "unknown struct?"
                };
                /*println!("{i}: {:?}", self.extern_idxs);
                if self.extern_idxs.contains(&(i as u32)) {
                    for field in &r#struct.fields {
                        match field {
                            RszType::Extern(path) => {
                                state.serialize_field(name, &path)?;
                            },
                            _ => {

                            }
                        }
                    }
                    continue;
                }*/
                let val_with_context = RszValueWithInfo(&r#struct, &context, None);
                //println!("{}", r#struct.name);
                state.serialize_field(name, &val_with_context)?;
            }
            state.end()
    }
}


pub fn enum_map() -> &'static HashMap<String, HashMap<String, String>> {
    static HASHMAP: OnceLock<HashMap<String, HashMap<String, String>>> = OnceLock::new();
    HASHMAP.get_or_init(|| {
        let json_data = std::fs::read_to_string(ENUM_FILE.get().unwrap()).unwrap();
        let hashmap: HashMap<String, HashMap<String, String>> = serde_json::from_str(&json_data).unwrap();
        hashmap
    })
}

pub fn get_enum_name(name: String, value: String) -> Option<String> {
    let name = name.replace("[]", "").replace("_Serializable", "_Fixed");
    if let Some(map) = enum_map().get(&name) {
        if let Some(value) = map.get(&value){
            return Some(value.to_string())
        }
    }
    let name = name.replace("_Fixed", "");
    if let Some(map) = enum_map().get(&name) {
        if let Some(value) = map.get(&value){
            return Some(value.to_string())
        }
    }
    None
}

