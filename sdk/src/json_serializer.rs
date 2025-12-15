use std::fmt::Display;

use serde::{Serialize, Serializer, ser::{SerializeMap, SerializeSeq}};

use crate::{rsz::Rsz, type_map::{self, FieldInfo, TypeMap}, value::{Instance, Value}};

pub struct RszWithCtx<'a> {
    pub rsz: &'a Rsz,
    pub type_map: &'a TypeMap,
}


impl<'a> Serialize for RszWithCtx<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer {
        let mut state = serializer.serialize_seq(Some(self.rsz.roots.len()))?;
        for root in &self.rsz.roots {
            if let Some(instance) = self.rsz.instances.get(*root as usize) {
                let wrapped = InstanceWithCtx {
                    instance,
                    instances: &self.rsz.instances,
                    type_map: self.type_map,
                };
                state.serialize_element(&wrapped)?;
            }
        }
        state.end()
    }
}

struct InstanceWithCtx<'a> {
    instance: &'a Instance,
    instances: &'a Vec<Instance>,
    type_map: &'a TypeMap,
}

impl<'a> Serialize for InstanceWithCtx<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer {
        let type_map = self.type_map;
        let instance = self.instance;
        let instances = self.instances;
        let mut state = serializer.serialize_map(Some(instance.fields.len()))?;
        let type_info = type_map.get_by_hash(instance.hash).unwrap(); // TODO should throw error if not
                                                                  // found
        for (i, value) in instance.fields.iter().enumerate() {
            if let Some(field_info) = type_info.get_by_index(i) {
                let wrapped = ValueWithCtx {value, instances, type_map, field_info};
                state.serialize_entry(&field_info.name, &wrapped)?;
            } else {
                //state.serialize_entry(format!("v{i}").as_str(), &wrapped)?;
            }
        }

        state.end()
    }
}

struct ValueWithCtx<'a> {
    value: &'a Value,
    instances: &'a Vec<Instance>,
    field_info: &'a FieldInfo,
    type_map: &'a TypeMap,
}

pub fn serialize_enum<S: Serializer, T: Display + Serialize>(n: &T, original_type: &str, type_map: &TypeMap, serializer: S) -> Result<S::Ok, S::Error> {
    let base_name = if let Some(idx) = original_type.find('[') {
        &original_type[..idx]
    } else {original_type};
    if let Some(x) = type_map.get_enum_str(n, base_name) {
        x.serialize(serializer)
    } else {
        n.serialize(serializer)
    }
}

impl<'a> Serialize for ValueWithCtx<'a> {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let value = self.value;
        let instances = self.instances;
        let type_map = self.type_map;
        match value {
            // TODO: make enums faster by embedding enum = True or false in the field info somewhere
            // otherwise this is very slow
            Value::S32(v) => {
                serialize_enum(v, &self.field_info.original_type, type_map, serializer)
            },
            Value::U32(v) => {
                serialize_enum(v, &self.field_info.original_type, type_map, serializer)
            },
            Value::U64(v) => {
                serialize_enum(v, &self.field_info.original_type, type_map, serializer)
            }
            Value::Bool(v) => v.serialize(serializer),
            Value::U8(v) => v.serialize(serializer),
            Value::U16(v) => v.serialize(serializer),
            Value::S8(v) => v.serialize(serializer),
            Value::S16(v) => v.serialize(serializer),
            Value::S64(v) => v.serialize(serializer),
            Value::F8(v) => v.serialize(serializer),
            Value::F16(v) => v.serialize(serializer),
            Value::F32(v) => v.serialize(serializer),
            Value::F64(v) => v.serialize(serializer),
            Value::Size(v) => v.serialize(serializer),
            Value::RuntimeType(v) => v.serialize(serializer),
            Value::String(v) => v.to_string().serialize(serializer),
            Value::Resource(v) => v.to_string().serialize(serializer),
            Value::UInt2(v) => v.serialize(serializer),
            Value::UInt3(v) => v.serialize(serializer),
            Value::UInt4(v) => v.serialize(serializer),
            Value::Int2(v) => v.serialize(serializer),
            Value::Int3(v) => v.serialize(serializer),
            Value::Int4(v) => v.serialize(serializer),
            Value::Float2(v) => v.serialize(serializer),
            Value::Float3(v) => v.serialize(serializer),
            Value::Float4(v) => v.serialize(serializer),
            Value::Vec2(v) => v.serialize(serializer),
            Value::Vec3(v) => v.serialize(serializer),
            Value::Vec4(v) => v.serialize(serializer),
            Value::Quaternion(v) => v.serialize(serializer),
            Value::Sphere(v) => v.serialize(serializer),
            Value::Position(v) => v.serialize(serializer),
            Value::Color(v) => v.serialize(serializer),
            Value::Mat4x4(v) => v.serialize(serializer),
            Value::Guid(v) => v.serialize(serializer),
            Value::OBB(v) => v.serialize(serializer),
            Value::AABB(v) => v.serialize(serializer),
            Value::Data(v) => v.serialize(serializer),
            Value::Range(v) => v.serialize(serializer),
            Value::RangeI(v) => v.serialize(serializer),
            Value::Rect(v) => v.serialize(serializer),
            Value::GameObjectRef(v) => v.0.serialize(serializer),
            Value::KeyFrame(v) => v.serialize(serializer),
            Value::Null => serializer.serialize_none(),
            Value::Object(index) | Value::UserData(index) => {
                if let Some(instance) = instances.get(*index as usize) {
                    let wrapped = InstanceWithCtx {
                        instance,
                        instances,
                        type_map
                    };
                    wrapped.serialize(serializer)
                } else {
                    serializer.serialize_none()
                }
            },
            Value::Array(arr) => {
                let mut state = serializer.serialize_seq(Some(arr.len()))?;
                for value in arr {
                    let wrapped = ValueWithCtx {
                        value,
                        instances,
                        field_info: &self.field_info,
                        type_map
                    };
                    state.serialize_element(&wrapped)?;
                }
                state.end()
            },
        }
    }
}
