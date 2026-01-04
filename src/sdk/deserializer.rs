use std::{collections::HashMap, error::Error, io::{Cursor, Read, Seek}};

use half::f16;
use log::debug;
use util::{ReadExt, SeekExt};

use crate::sdk::{rsz::{Rsz, RszHeader}, type_map::{FieldInfo, TypeInfo, TypeMap}, types::*, value::{Extern, Instance, Value}};

pub struct RszDeserializer<'a, R: Read + Seek> {
    data: R,
    type_map: &'a TypeMap,
    type_descriptors: &'a Vec<TypeDescriptor>,
    roots: &'a Vec<u32>,
    externs: &'a HashMap<u32, String>,
}

impl<'a> RszDeserializer<'a, Cursor<&'a [u8]>> {
    pub fn from_rsz_header(rsz_header: &'a RszHeader, type_map: &'a TypeMap) -> Self {
        let data = Cursor::new(rsz_header.data.as_slice());
        Self {
            data,
            type_map,
            type_descriptors: &rsz_header.type_descriptors,
            roots: &rsz_header.roots,
            externs: &rsz_header.extern_slots,
        }
    }
}

impl<'a, R: Read + Seek> RszDeserializer<'a, R> {
    pub fn new(data: R, roots: &'a Vec<u32>, type_descriptors: &'a Vec<TypeDescriptor>, type_map: &'a TypeMap, externs: &'a HashMap<u32, String>) -> Self {
        Self {
            data,
            type_map,
            type_descriptors,
            externs,
            roots,
        }
    }

    pub fn deserialize(&mut self) -> Result<Rsz, Box<dyn Error>> {
        let mut instances: Vec<Instance> = Vec::new();
        let mut externs = HashMap::new();
        for (i, TypeDescriptor {hash, ..}) in self.type_descriptors.iter().enumerate() {
            let type_info = self.type_map.get_by_hash(*hash).ok_or("hash not found in type map")?;
            debug!("class: {}", type_info.name);
            
            if let Some(extern_slot) = self.externs.get(&(i as u32)) {
                externs.insert(i as u32, Extern {
                    index: i as u32,
                    path: extern_slot.clone(),
                    r#type: type_info.name.clone()
                });
            }

            let mut fields = Vec::new();
            for (_hash, field) in &type_info.fields {
                debug!("field: {}", field.name);
                let value = self.deserialize_field(field, type_info)?;
                debug!("value: {:?}", value);
                fields.push(value);
            }
            instances.push(Instance {hash: *hash, fields});
        }
        Ok(Rsz {
            roots: self.roots.clone(),
            instances,
            externs,
        })
    }


    fn deserialize_field(&mut self, field: &FieldInfo, parent: &TypeInfo) -> Result<Value, Box<dyn Error>> {
        let value = if field.array {
            self.data.seek_align_up(4)?;
            let len = self.data.read_u32()?;
            let mut arr_vals = Vec::new();
            for _ in 0..len {
                self.data.seek_align_up(field.align as u64)?;
                let value = self.deserialize_field_single(field, parent)?;
                arr_vals.push(value);
            }
            Value::Array(arr_vals)
        } else {
            self.data.seek_align_up(field.align as u64)?;
            let value = self.deserialize_field_single(field, parent)?;
            value
        };
        Ok(value)
    }

    fn deserialize_field_single(&mut self, field: &FieldInfo, _parent: &TypeInfo) -> Result<Value, Box<dyn Error>> {
        let value = match field.r#type.as_str() {
            "Bool" =>  Value::Bool(self.data.read_bool()?),
            "U8" =>  Value::U8(self.data.read_u8()?),
            "U16" => Value::U16(self.data.read_u16()?),
            "U32" => Value::U32(self.data.read_u32()?),
            "U64" => Value::U64(self.data.read_u64()?),
            "S8" =>  Value::S8(self.data.read_i8()?),
            "S16" => Value::S16(self.data.read_i16()?),
            "S32" => Value::S32(self.data.read_i32()?),
            "S64" => Value::S64(self.data.read_i64()?),
            "F8" =>  Value::F8(self.data.read_u8()?),
            "F16" => Value::F16(f16::from_bits(self.data.read_u16()?)),
            "F32" => Value::F32(self.data.read_f32()?),
            "F64" => Value::F64(self.data.read_f64()?),
            "Size" => Value::Size(self.data.read_u64()?),
            "Object" => Value::Object(self.data.read_u32()?),
            "UserData" => Value::Object(self.data.read_u32()?),
            "RuntimeType" => Value::RuntimeType(RuntimeType(self.data.read_u8str()?)),
            "String" => Value::String(StringU16::read(&mut self.data)?),
            "Resource" => Value::String(StringU16::read(&mut self.data)?),
            "UInt2" => Value::UInt2(self.data.read_u32_arr()?),
            "UInt3" => Value::UInt3(self.data.read_u32_arr()?),
            "UInt4" => Value::UInt4(self.data.read_u32_arr()?),
            "Int2" => Value::Int2(self.data.read_i32_arr()?),
            "Int3" => Value::Int3(self.data.read_i32_arr()?),
            "Int4" => Value::Int4(self.data.read_i32_arr()?),
            "Float2" => Value::Float2(self.data.read_f32_arr()?),
            "Float3" => Value::Float3(self.data.read_f32_arr()?),
            "Float4" => Value::Float4(self.data.read_f32_arr()?),
            "Vec2" => Value::Vec2(Vec2::read(&mut self.data)?),
            "Vec3" => Value::Vec3(Vec3::read(&mut self.data)?),
            "Vec4" => Value::Vec4(Vec4::read(&mut self.data)?),
            "Quaternion" => Value::Quaternion(Quaternion::read(&mut self.data)?),
            "Sphere" => Value::Sphere(Sphere::read(&mut self.data)?),
            "Position" => Value::Position(Position::read(&mut self.data)?),
            "Color" => Value::Color(Color::read(&mut self.data)?),
            "Mat4x4" => Value::Mat4x4(Box::new(Mat4x4(self.data.read_f32_arr()?))),
            "Guid" => Value::Guid(Guid(self.data.read_u8_arr()?)),
            "OBB" => Value::OBB(Box::new(OBB::read(&mut self.data)?)),
            "AABB" => Value::AABB(Box::new(AABB::read(&mut self.data)?)),
            "Data" => Value::Data(Data::read(&mut self.data)?),
            "Range" => Value::Range(RangeF::read(&mut self.data)?),
            "RangeI" => Value::RangeI(RangeI::read(&mut self.data)?),
            "Rect" => Value::Rect(Rect::read(&mut self.data)?),
            "GameObjectRef" => Value::GameObjectRef(GameObjectRef(Guid(self.data.read_u8_arr()?))),
            "KeyFrame" => Value::KeyFrame(KeyFrame::read(&mut self.data)?),
            _ => {
                return Err("ahhhhh".into())
            }
        };
        Ok(value)
    }
}
