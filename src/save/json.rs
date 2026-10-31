use ree_lib::{context::EngineContext, types::*};
use serde::{Serialize, Serializer, ser::{SerializeMap, SerializeSeq, SerializeStruct}};

use crate::{save::{SaveFile, types::{Array, Class, EnumValue, FieldValue, Struct}}};

pub struct ClassView<'a> {
    pub class: &'a Class,
    pub ctx: &'a EngineContext<'a>,
}

pub struct FieldValueView<'a> {
    pub value: &'a FieldValue,
    pub ctx: &'a EngineContext<'a>,
    pub parent_class_hash: Option<u32>,
    pub parent_field_hash: Option<u32>,
}

pub struct ArrayView<'a> {
    pub array: &'a Array,
    pub ctx: &'a EngineContext<'a>,
    pub parent_class_hash: Option<u32>,
    pub parent_field_hash: Option<u32>,
}

impl<'a> Serialize for ClassView<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: serde::Serializer {
        let mut map = serializer.serialize_map(Some(self.class.fields.len()))?;
        let type_info = self.ctx.rsz_map.get_by_hash(self.class.hash);
        for field in &self.class.fields {
            let field_name = type_info.and_then(|t| t.get_field_by_hash(field.hash))
                .map(|x| x.name.clone())
                .unwrap_or_else(|| format!("{:08x}", field.hash));

            let value_view = FieldValueView {
                value: &field.value,
                ctx: self.ctx,
                parent_class_hash: Some(self.class.hash),
                parent_field_hash: Some(field.hash),
            };

            map.serialize_entry(&field_name, &value_view)?;
        }
        map.end()
    }
}

macro_rules! try_serialize_as {
    ($data:expr, $serializer:expr, $ty:ty) => {
        if let Ok(v) = bytemuck::try_from_bytes::<$ty>($data) {
            return v.serialize($serializer);
        }
    };
    ($name:expr, $data:expr, $serializer:expr, { $( $pat:literal => $ty:ty ),* $(,)? }) => {
        match $name {
            $( $pat => try_serialize_as!($data, $serializer, $ty), )*
            _ => {}
        }
    };
}

impl<'a> Serialize for FieldValueView<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: serde::Serializer {
        match self.value {
            FieldValue::Boolean(v) => v.serialize(serializer),
            FieldValue::S8(v)  => v.serialize(serializer),
            FieldValue::U8(v)  => v.serialize(serializer),
            FieldValue::S16(v) => v.serialize(serializer),
            FieldValue::U16(v) => v.serialize(serializer),
            FieldValue::S32(v) => v.serialize(serializer),
            FieldValue::U32(v) => v.serialize(serializer),
            FieldValue::S64(v) => v.serialize(serializer),
            FieldValue::U64(v) => v.serialize(serializer),
            FieldValue::F32(v) => v.serialize(serializer),
            FieldValue::F64(v) => v.serialize(serializer),
            FieldValue::C8(v)  => v.serialize(serializer),
            FieldValue::C16(v) => v.serialize(serializer),
            FieldValue::Enum(v)   => v.serialize(serializer),
            FieldValue::Unknown   => serializer.serialize_none(),
            FieldValue::Array(v)  => {
                let arr_view = ArrayView {
                    array: v,
                    ctx: self.ctx,
                    parent_class_hash: self.parent_class_hash,
                    parent_field_hash: self.parent_field_hash,
                };
                arr_view.serialize(serializer)
            },
            FieldValue::String(v) => v.serialize(serializer),
            FieldValue::Struct(v) => {
                let struct_type_name: Option<String> = self.parent_class_hash
                    .and_then(|h| self.ctx.rsz_map.get_by_hash(h))
                    .and_then(|type_info| {
                        let field_info = type_info.get_field_by_hash(self.parent_field_hash?)?;
                        Some(field_info.original_type.clone())
                    });

                if let Some(name) = struct_type_name.as_deref() {
                    try_serialize_as!(name, &v.data, serializer, {
                        "via.vec2" => Vec2,
                        "via.vec3" => Vec3,
                        "via.vec4" => Vec4,
                        "via.rds.Mandrake" => Mandrake,
                    });
                }

                let mut state = serializer.serialize_struct("Struct", 2)?;
                if let Some(name) = struct_type_name {
                    state.serialize_field("inferred_type", &name)?;
                } else {
                    state.serialize_field("inferred_type", "Unknown Struct")?;
                }
                state.serialize_field("raw_data", &v.data)?;
                state.end()
            },
            FieldValue::Class(v)  => {
                let view = ClassView {
                    class: v,
                    ctx: self.ctx,
                };
                view.serialize(serializer)
            },
        }
    }
}

impl<'a> Serialize for ArrayView<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: serde::Serializer {
        let mut state = serializer.serialize_seq(Some(self.array.values.len()))?;
        for value in &self.array.values {
            let value_view = FieldValueView {
                value,
                ctx: self.ctx,
                parent_class_hash: self.parent_class_hash,
                parent_field_hash: self.parent_field_hash,
            };
            state.serialize_element(&value_view)?;
        }
        state.end()
    }
}

impl Serialize for EnumValue {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: serde::Serializer {
        match self {
            Self::E1(v) => v.serialize(serializer),
            Self::E2(v) => v.serialize(serializer),
            Self::E4(v) => v.serialize(serializer),
            Self::E8(v) => v.serialize(serializer),
        }
    }
}

impl Serialize for Struct {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: serde::Serializer {
        // TODO: add a raw_data and an inferred type serialization
        self.data.serialize(serializer)
    }
}

struct ObjectsMapWrapper<'a>(&'a [(u32, Class)], &'a EngineContext<'a>);

impl<'a> Serialize for ObjectsMapWrapper<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut map = serializer.serialize_map(Some(self.0.len()))?;
        for (hash, class) in self.0 {
            let view = ClassView {
                class,
                ctx: self.1
            };
            map.serialize_entry(&format!("{:08x}", hash), &view)?;
        }
        map.end()
    }
}

pub struct SaveFileView<'a> {
    pub file: &'a SaveFile,
    pub ctx: &'a EngineContext<'a>,
}

impl<'a> Serialize for SaveFileView<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer {
        let mut state = serializer.serialize_struct("SaveFile", 4)?;
        state.serialize_field("game", &self.file.game)?;
        state.serialize_field("flags", &self.file.flags)?;
        state.serialize_field("blowfish_options", &self.file.blowfish_options)?;
        state.serialize_field("data", &ObjectsMapWrapper(&self.file.fields, self.ctx))?;
        state.end()
    }
}
