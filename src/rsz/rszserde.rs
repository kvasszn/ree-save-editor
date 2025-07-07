
/*
 * Deserializer/Serializer
 */

use std::{any::Any, collections::{HashMap, HashSet}, fmt::Debug, io::{Cursor, Read, Seek, SeekFrom, Write}, rc::Rc, str::FromStr};

use indexmap::IndexMap;
use rsz_macros::{DeRszFrom, DeRszInstance};
use serde::{Deserialize, Serialize};

use crate::reerr::{self, Result, RszError};
use crate::file_ext::*;
use super::{dump::{enum_map, get_enum_name, get_enum_val, RszDump, RszField}, Extern, Rsz, TypeDescriptor};

pub trait ReadSeek: Read + Seek {}
impl<'a, T: Read + Seek> ReadSeek for T {}
pub struct RszDeserializerCtx<'a> {
    data: Box<dyn ReadSeek + 'a>,
    type_descriptors: &'a Vec<TypeDescriptor>,
    roots: &'a Vec<u32>,
    cur_hash: Vec<u32>,
    extern_slots: &'a HashMap<u32, Extern>,
    field: Vec<&'a RszField>,
    registry: Rc<DeRszRegistry>
}

impl<'a> RszDeserializerCtx<'a> {
    fn get_hash(&self) -> Result<u32> {
        if let Some(hash) = self.cur_hash.last() {
            return Ok(*hash)
        } else { return Err(RszError::InvalidRszTypeHash(0).into()) };
    }
}

pub trait WriteSeek: Write + Seek {}
impl<'a, T: Write + Seek> WriteSeek for T {}
pub struct RszSerializerCtx<'a> {
    pub data: &'a mut dyn WriteSeek,
    pub base_addr: usize,
}

impl<'a> From<&'a Rsz> for RszDeserializerCtx<'a> {
    fn from(value: &'a Rsz) -> Self {
        let cursor = Cursor::new(&value.data);
        let boxed: Box<dyn ReadSeek + 'a> = Box::new(cursor);
        let mut registry = DeRszRegistry::new();
        //registry.register::<Nullable>("Nullable");
        registry.register::<u8>("U8");
        registry.register::<u16>("U16");
        registry.register::<u32>("U32");
        registry.register::<u64>("U64");
        registry.register::<i8>("S8");
        registry.register::<i16>("S16");
        registry.register::<i32>("S32");
        registry.register::<i64>("S64");
        registry.register::<u8>("F8");
        registry.register::<u16>("F16");
        registry.register::<f32>("F32");
        registry.register::<f64>("F64");
        registry.register::<String>("RuntimeType");
        registry.register::<StringU16>("String");
        registry.register::<StringU16>("Resource");
        registry.register::<bool>("Bool");
        registry.register::<UInt2>("Uint2");
        registry.register::<UInt3>("Uint3");
        registry.register::<UInt4>("Uint4");
        registry.register::<Int2>("Int2");
        registry.register::<Int3>("Int3");
        registry.register::<Int4>("Int4");
        registry.register::<Float2>("Float2");
        registry.register::<Float3>("Float3");
        registry.register::<Float4>("Float4");
        registry.register::<Vec2>("Vec2");
        registry.register::<Vec3>("Vec3");
        registry.register::<Vec4>("Vec4");
        registry.register::<Quaternion>("Quaternion");
        registry.register::<Sphere>("Sphere");
        registry.register::<Position>("Position");
        registry.register::<Color>("Color");
        registry.register::<Mat4x4>("Mat4");
        registry.register::<Guid>("Guid");
        registry.register::<Object>("Object");
        registry.register::<Object>("UserData");
        registry.register::<OBB>("OBB");
        registry.register::<AABB>("AABB");
        registry.register::<Data>("Data");
        registry.register::<Range>("Range");
        registry.register::<RangeI>("RangeI");
        registry.register::<Rect>("Rect");
        registry.register::<Struct>("Struct");
        registry.register::<Guid>("GameObjectRef");
        registry.register::<KeyFrame>("KeyFrame");
        registry.register::<u64>("Size");
        Self {
            data: boxed,
            cur_hash: Vec::new(),
            type_descriptors: &value.type_descriptors,
            roots: &value.roots,
            extern_slots: &value.extern_slots,
            field: Vec::new(),
            registry: Rc::new(registry)
        }
    }
}

pub type RszFieldsValue = (u32, Vec<Box<dyn DeRszInstance>>);

pub struct RszJsonSerializerCtx<'a> {
    root: Option<u32>,
    field: Option<&'a RszField>,
    objects: &'a Vec<RszFieldsValue>,
}

pub trait DeRszType<'a> {
    fn from_bytes(ctx: &'a mut RszDeserializerCtx) -> Result<Self> where Self: Sized;
}

pub trait DeRszInstance: Debug  {
    fn as_any(&self) -> &dyn Any;
    fn to_json(&self, ctx: &RszJsonSerializerCtx) -> serde_json::Value;
    fn to_bytes(&self, _ctx: &mut RszSerializerCtx) -> Result<()>;
}

pub struct RszJsonDeserializerCtx<'a> {
    hash: u32,
    field: Option<&'a RszField>,
    objects: &'a mut Vec<RszFieldsValue>,
    registry: Rc<DeRszRegistry>
}

pub trait RszFromJson {
    fn from_json(data: &serde_json::Value, ctx: &mut RszJsonDeserializerCtx) -> Result<Self> where Self: Sized;
}

/*
 * Default Implementations
 */
impl DeRszInstance for Vec<Box<dyn DeRszInstance>> {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn to_json(&self, ctx: &RszJsonSerializerCtx) -> serde_json::Value {
        serde_json::Value::Array(self.iter().map(|item| {
            let new_ctx = RszJsonSerializerCtx {
                root: None,
                field: ctx.field,
                objects: ctx.objects,
            };
            item.to_json(&new_ctx)
        }).collect())
    }
    fn to_bytes(&self, ctx: &mut RszSerializerCtx) -> Result<()> {
        ctx.data.write_all(&(self.len() as u32).to_le_bytes())?;
        for i in 0..self.len() {
            self[i].to_bytes(ctx)?;
        }
        Ok(())
    }
}

impl<T: 'static + DeRszInstance + Debug + Serialize> DeRszInstance for Vec<T> {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn to_json(&self, _ctx: &RszJsonSerializerCtx) -> serde_json::Value {
        serde_json::json!(self)
    }

    fn to_bytes(&self, ctx: &mut RszSerializerCtx) -> Result<()> {
        (self.len() as u32).to_bytes(ctx)?;
        for item in self {
            item.to_bytes(ctx)?;
        }
        Ok(())
    }
}


impl<'a, T> DeRszType<'a> for Vec<T>
where
    T: for<'b> DeRszType<'b>, 
{
    fn from_bytes(ctx: &'a mut RszDeserializerCtx) -> Result<Self> {
        let len = u32::from_bytes(ctx)? as usize;
        let mut items = Vec::with_capacity(len);
        for _ in 0..len {
            let item = T::from_bytes(ctx)?;
            items.push(item);
        }
        Ok(items)
    }
}

impl<T: RszFromJson> RszFromJson for Vec<T> {
    fn from_json(data: &serde_json::Value, ctx: &mut RszJsonDeserializerCtx) -> Result<Self> where Self: Sized {
        let data_vals = data.as_array().unwrap();
        let mut vals = Vec::new();
        for val in data_vals {
            let t = T::from_json(&val, ctx)?;
            vals.push(t);
        }
        Ok(vals)
    }
}



impl<T: 'static + DeRszInstance + Debug, const N: usize> DeRszInstance for [T; N] {
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn to_json(&self, ctx: &RszJsonSerializerCtx) -> serde_json::Value {
        let values = self.iter().map(|x| {
            x.to_json(ctx)
        }).collect::<Vec<serde_json::Value>>();
        serde_json::Value::Array(values)
    }
    fn to_bytes(&self, ctx: &mut RszSerializerCtx) -> Result<()> {
        for i in 0..self.len() {
            self[i].to_bytes(ctx)?;
        }
        Ok(())
    }
}

impl<'a, T, const N: usize> DeRszType<'a> for [T; N]
where
    T: for<'b> DeRszType<'b> + Debug,
{
    fn from_bytes(ctx: &'a mut RszDeserializerCtx) -> Result<Self> {
        let mut vec = Vec::with_capacity(N);
        for _ in 0..N {
            vec.push(T::from_bytes(ctx)?);
        }
        Ok(vec.try_into().unwrap())
    }
}

impl<T: RszFromJson + Debug, const N: usize> RszFromJson for [T; N] {
    fn from_json(data: &serde_json::Value, ctx: &mut RszJsonDeserializerCtx) -> Result<Self> where Self: Sized {
        let data_vals = data.as_array().unwrap();
        let mut vals = Vec::with_capacity(N);
        for val in data_vals {
            vals.push(T::from_json(&val, ctx)?);
        }
        Ok(vals.try_into().unwrap())
    }
}



#[derive(Debug)]
pub struct Object {
    hash: u32,
    idx: u32,
}

impl DeRszInstance for Object {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn to_json(&self, ctx: &RszJsonSerializerCtx) -> serde_json::Value {
        let res: Result<&RszFieldsValue> = ctx.objects.get(ctx.root.unwrap_or(self.idx) as usize)
            .ok_or(RszError::InvalidRszObjectIndex(self.idx, self.hash).into());
        let (hash, field_values) = match res {
            Ok(a) => a,
            Err(e) => {eprintln!("{:?}", e); return serde_json::Value::Null;}
        };
        let struct_desc = match RszDump::rsz_map().get(&hash) {
            Some(struct_desc) => struct_desc,
            None => return serde_json::Value::Null
        };
        if let Some(obj) = field_values.get(0) {
            if let Some(extern_obj) = obj.as_any().downcast_ref::<ExternObject>() {
                return extern_obj.to_json(ctx)
            }
        }
        let values = struct_desc.fields.iter().enumerate().map(|(i, field)| {
            let obj = &field_values[i];
            let new_ctx = RszJsonSerializerCtx {
                root: None,
                field: Some(&field),
                objects: ctx.objects
            };
            (field.name.clone(), obj.to_json(&new_ctx))
        }).collect::<IndexMap<String, serde_json::Value>>();
        serde_json::to_value(values).unwrap()
    }
    fn to_bytes(&self, ctx: &mut RszSerializerCtx) -> Result<()> {
        self.idx.to_bytes(ctx)
    }
}

impl<'a> DeRszType<'a> for Object {
    fn from_bytes(ctx: &'a mut RszDeserializerCtx) -> Result<Self> {
        let hash = ctx.get_hash()?;
        let _rsz_struct = RszDump::get_struct(hash)?;
        //println!("object: {struct_desc:?}");
        Ok(Self {
            hash,
            idx: ctx.data.read_u32()?
        })
    }
}

impl RszFromJson for Object {
    fn from_json(data: &serde_json::Value, ctx: &mut RszJsonDeserializerCtx) -> Result<Self> where Self: Sized {
        let parent_struct = RszDump::get_struct(ctx.hash)?;
        println!("{}: {:?}", parent_struct.name, data);
        let field_name = &ctx.field.unwrap().original_type;
        let hash = if field_name == "ace.user_data.ExcelUserData.cData" {
            let og_type = parent_struct.name.clone() + ".cData";
            let mapped_hash = RszDump::name_map().get(&og_type).unwrap();
            *mapped_hash
        } else {
            *ctx.field.unwrap().get_type_hash().unwrap()
        };
        let mut new_ctx = RszJsonDeserializerCtx {
            hash,
            field: None,
            objects: ctx.objects,
            registry: ctx.registry.clone(),
        };
        let r#struct: Box<dyn DeRszInstance> = Box::new(Struct::from_json(data, &mut new_ctx)?);
        //println!("object struct: {:#?}", r#struct);
        let obj = Object{hash: ctx.hash, idx: ctx.objects.len() as u32};
        ctx.objects.push((hash, vec![r#struct]));
        println!("got obj {:#?}", obj);
        return Ok(obj);
    }
}


#[derive(Debug)]
pub struct ExternObject {
    path: String,
    pub object: Object,
}

impl DeRszInstance for ExternObject {
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn to_json(&self, _ctx: &RszJsonSerializerCtx) -> serde_json::Value {
        serde_json::json!({
            "extern_path": self.path,
        })
    }
    fn to_bytes(&self, _ctx: &mut RszSerializerCtx) -> Result<()> {
        todo!()
    }
}

/*
 *
 *
 */

type DeserializerFn = fn(&mut RszDeserializerCtx) -> Result<Box<dyn DeRszInstance>>;
type JsonDeserializerFn = fn(&serde_json::Value, &mut RszJsonDeserializerCtx) -> Result<Box<dyn DeRszInstance>>;

pub struct DeRszRegistry {
    deserializers: HashMap<&'static str, DeserializerFn>,
    serializers: HashMap<&'static str, JsonDeserializerFn>,
}

impl DeRszRegistry {
    pub fn init(&mut self) {
        //registry.register::<Nullable>("Nullable");
        self.register::<u8>("U8");
        self.register::<u16>("U16");
        self.register::<u32>("U32");
        self.register::<u64>("U64");
        self.register::<i8>("S8");
        self.register::<i16>("S16");
        self.register::<i32>("S32");
        self.register::<i64>("S64");
        self.register::<u8>("F8");
        self.register::<u16>("F16");
        self.register::<f32>("F32");
        self.register::<f64>("F64");
        self.register::<String>("RuntimeType");
        self.register::<StringU16>("String");
        self.register::<StringU16>("Resource");
        self.register::<bool>("Bool");
        self.register::<UInt2>("Uint2");
        self.register::<UInt3>("Uint3");
        self.register::<UInt4>("Uint4");
        self.register::<Int2>("Int2");
        self.register::<Int3>("Int3");
        self.register::<Int4>("Int4");
        self.register::<Float2>("Float2");
        self.register::<Float3>("Float3");
        self.register::<Float4>("Float4");
        self.register::<Vec2>("Vec2");
        self.register::<Vec3>("Vec3");
        self.register::<Vec4>("Vec4");
        self.register::<Quaternion>("Quaternion");
        self.register::<Sphere>("Sphere");
        self.register::<Position>("Position");
        self.register::<Color>("Color");
        self.register::<Mat4x4>("Mat4");
        self.register::<Guid>("Guid");
        self.register::<Object>("Object");
        self.register::<Object>("UserData");
        self.register::<OBB>("OBB");
        self.register::<AABB>("AABB");
        self.register::<Data>("Data");
        self.register::<Range>("Range");
        self.register::<RangeI>("RangeI");
        self.register::<Rect>("Rect");
        self.register::<Struct>("Struct");
        self.register::<Guid>("GameObjectRef");
        self.register::<KeyFrame>("KeyFrame");
        self.register::<u64>("Size");
    }
    pub fn new() -> Self {
        Self {
            deserializers: HashMap::new(),
            serializers: HashMap::new(),
        }
    }
    fn register<T>(&mut self, type_id: &'static str)
    where
        T: for<'a> DeRszType<'a> + RszFromJson + DeRszInstance + 'static
    {
        self.deserializers.insert(type_id, |ctx| {
            Ok(Box::new(T::from_bytes(ctx)?))
        });
        self.serializers.insert(type_id, |data, ctx| {
            Ok(Box::new(T::from_json(data, ctx)?))
        });
    }
    fn get(&self, name: &'static str) -> Result<&DeserializerFn> {
        let des = self.deserializers.get(name);
        let desfn = des.ok_or(RszError::UnsetDeserializer(name));
        Ok(desfn?)
    }
    fn get_se(&self, name: &'static str) -> Result<&JsonDeserializerFn> {
        let se = self.serializers.get(name);
        let sefn = se.ok_or(RszError::UnsetDeserializer(name));
        Ok(sefn?)
    }
}

#[derive(Debug)]
pub struct DeRsz {
    pub offset: usize,
    pub roots: Vec<u32>,
    pub structs: Vec<RszFieldsValue>,
    pub extern_idxs: HashSet<u32>,
}


impl Serialize for DeRsz {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer {
            #[derive(Serialize)]
            struct Wrapped {
                offset: usize,
                roots: Vec<&'static str>,
                rsz: Vec<serde_json::Value>
            }
            let mut wrapped = Wrapped{offset: self.offset, roots: Vec::new(), rsz: Vec::new()};
            for root in &self.roots {
                let ctx = RszJsonSerializerCtx {root: Some(*root), field: None, objects: &self.structs};
                let hash = self.structs[*root as usize].0;
                let name = &RszDump::get_struct(hash).unwrap().name;
                wrapped.roots.push(name);
                let obj = Object {hash: 0, idx: *root as u32};
                //let data = ctx.objects[*root as usize].1.to_json(&ctx);
                let data = obj.to_json(&ctx);
                wrapped.rsz.push(data);
            }
            wrapped.serialize(serializer)
    }
}

impl<'a> DeRszType<'a> for DeRsz {
    fn from_bytes(ctx: &'a mut RszDeserializerCtx) -> Result<Self> {
        let mut structs: Vec<RszFieldsValue> = Vec::new();
        let mut extern_idxs: Vec<u32> = Vec::new();
        println!("{:?}", ctx.type_descriptors);
        for (i, &TypeDescriptor { hash, crc: _crc }) in ctx.type_descriptors.clone().iter().enumerate() {
            if let Some(_slot_extern) = ctx.extern_slots.get(&u32::try_from(i)?) {
                extern_idxs.push(i as u32);
                let eobj: Box<dyn DeRszInstance> = Box::new(ExternObject {
                    path: _slot_extern.path.clone(),
                    object: Object { hash: _slot_extern.hash, idx: i as u32 } 
                });
                structs.push((hash, vec![eobj]));
                continue;
            } else {
                let struct_type = match RszDump::rsz_map().get(&hash) {
                    Some(x) => Ok(x),
                    None => Err(Box::new(reerr::FileParseError::InvalidRszTypeHash(hash)))
                }?;
                #[cfg(debug_assertions)]
                log::debug!("\nDeserializing: {struct_type:?}");
                let mut field_values: RszFieldsValue = (hash, Vec::new());
                for field in &struct_type.fields {
                    //println!("{field:?}");
                    if let Some(field_hash) = field.get_type_hash() {
                        ctx.cur_hash.push(*field_hash);
                    } else {
                        log::warn!("Could not find type hash for {} where native = {}", field.name, field.native);
                    }
                    ctx.field.push(&field);
                    #[cfg(debug_assertions)]
                    log::debug!("\nDeserializing field: {field:?}");
                    if field.array {
                        ctx.data.seek_align_up(4)?;
                        let len = ctx.data.read_u32()?;
                        ctx.data.seek_align_up(field.align.into())?;
                        let mut vals = Vec::new();
                        for _ in 0..len {
                            let dersz_fn = ctx.registry.get(field.r#type.as_str())?;
                            let x: Box<dyn DeRszInstance> = dersz_fn(ctx)?;
                            vals.push(x);
                        }
                        field_values.1.push(Box::new(vals));

                    } else {
                        ctx.data.seek_align_up(field.align.into())?;
                        let dersz_fn = ctx.registry.get(field.r#type.as_str())?;
                        let x: Box<dyn DeRszInstance> = dersz_fn(ctx)?;
                        field_values.1.push(x);
                    }
                    //println!("{field_values:?}");
                }
                structs.push(field_values);
            }
        }

        //println!("{structs:#?}, {extern_idxs:#?}");

        Ok(Self { offset: 0, roots: ctx.roots.clone(), structs, extern_idxs: HashSet::new() })
    }
}

impl DeRsz {
    pub fn from_json(data: &serde_json::Value, registry: Rc<DeRszRegistry>) -> Result<Self> {
        let offset = data.get("offset").unwrap().as_u64().expect("offset should be an integer") as usize;
        let root_data = data.get("roots").unwrap();
        let roots_types: Vec<String> = serde_json::from_value(root_data.clone())?;
        let rszs_data = data.get("rsz").unwrap().as_array().expect("rszs should be in an array");
        
        let mut roots = Vec::new();
        let mut objects: Vec<RszFieldsValue> = Vec::new();
        objects.push((0, vec![]));
        for (rsz_data, root_type) in rszs_data.iter().zip(roots_types) {
            println!("{root_type}");
            let hash = *RszDump::name_map().get(&root_type).unwrap();
            let mut ctx = RszJsonDeserializerCtx {
                hash,
                objects: &mut objects,
                registry: registry.clone(),
                field: None,
            };
            let val = Struct::from_json(rsz_data, &mut ctx)?;
            ctx.objects.push((val.hash, val.values));
            roots.push(ctx.objects.len() as u32 - 1);
        }
        println!("{:#?}", objects);
        Ok(Self {
            offset,
            roots,
            structs: objects,
            extern_idxs: HashSet::new()
        })
    }
}

/*
 * types
 */

#[derive(Debug, Serialize, DeRszFrom, DeRszInstance)]
pub struct UInt2(u32, u32);
#[derive(Debug, Serialize, DeRszFrom, DeRszInstance)]
pub struct UInt3(u32, u32, u32);
#[derive(Debug, Serialize, DeRszFrom, DeRszInstance)]
pub struct UInt4(u32, u32, u32, u32);

#[derive(Debug, Serialize, DeRszFrom, DeRszInstance)]
pub struct Int2(i32, i32);
#[derive(Debug, Serialize, DeRszFrom, DeRszInstance)]
pub struct Int3(i32, i32, i32);
#[derive(Debug, Serialize, DeRszFrom, DeRszInstance)]
pub struct Int4(i32, i32, i32, i32);

#[derive(Debug, Serialize, DeRszFrom, DeRszInstance)]
pub struct Color(u8, u8, u8, u8);

#[derive(Debug, Serialize, DeRszFrom, DeRszInstance)]
pub struct Vec2(f32, f32, f32, f32);

#[derive(Debug, Serialize, DeRszFrom, DeRszInstance)]
pub struct Vec3(f32, f32, f32, f32);
#[derive(Debug, Serialize, DeRszFrom, DeRszInstance)]
pub struct Vec4(f32, f32, f32, f32);

#[derive(Debug, Serialize, DeRszFrom, DeRszInstance)]
pub struct Quaternion(f32, f32, f32, f32);
#[derive(Debug, Serialize, DeRszFrom, DeRszInstance)]
pub struct Sphere(f32, f32, f32, f32);
#[derive(Debug, Serialize, DeRszFrom, DeRszInstance)]
pub struct Position(f32, f32, f32);

#[derive(Debug, Serialize, DeRszFrom, DeRszInstance)]
pub struct Float2(f32, f32);
#[derive(Debug, Serialize, DeRszFrom, DeRszInstance)]
pub struct Float3(f32, f32, f32);
#[derive(Debug, Serialize, DeRszFrom, DeRszInstance)]
pub struct Float4(f32, f32, f32, f32);

#[derive(Debug, Serialize, DeRszFrom, DeRszInstance)]
pub struct Mat4x4([f32; 16]);



macro_rules! derive_dersz_instance {
    ( $t:ty ) => {
        #[allow(unused)]
        impl DeRszInstance for $t {
            fn as_any(&self) -> &dyn Any {
                self
            }
            fn to_json(&self, ctx: &RszJsonSerializerCtx) -> serde_json::Value {
                serde_json::json!(self)
            }
            fn to_bytes(&self, ctx: &mut RszSerializerCtx) -> Result<()> {
                Ok(ctx.data.write_all(&self.to_le_bytes())?)
            }
        }
    };
}

macro_rules! derive_dersz_type{
    ($rsz_type:ty) => {
        impl<'a> DeRszType<'a> for $rsz_type {
            fn from_bytes(ctx: &'a mut RszDeserializerCtx) -> Result<$rsz_type> {
                let mut buf = [0; size_of::<$rsz_type>()];
                ctx.data.read_exact(&mut buf)?;
                Ok(<$rsz_type>::from_le_bytes(buf))
            }
        }
    };
}

macro_rules! derive_dersz_from_json {
    ($rsz_type:ty) => {
        impl RszFromJson for $rsz_type {
            fn from_json(data: &serde_json::Value, _ctx: &mut RszJsonDeserializerCtx) -> Result<$rsz_type> {
                Ok(serde_json::from_value(data.clone())?)
            }
        }
    };
}

macro_rules! derive_dersz_full{
    ($rsz_type:ty) => {
        derive_dersz_instance!( $rsz_type );
        derive_dersz_type!( $rsz_type );
        derive_dersz_from_json! ( $rsz_type );
    };
}


pub fn capitalize(s: &str) -> String {
    let c: String = s.chars().map(|c| c.to_uppercase().to_string()).collect::<String>();
    c
}
macro_rules! derive_dersz_type_enum{
    ($rsz_type:ty) => {
        #[allow(unused)]
        impl DeRszInstance for $rsz_type {
            fn as_any(&self) -> &dyn Any {
                self
            }
            fn to_json(&self, ctx: &RszJsonSerializerCtx) -> serde_json::Value {
                match ctx.field {
                    Some(field) => {
                        let tmp = field.original_type.replace("[]", "");
                        let str_enum_name = |name: &str, val: $rsz_type| { 
                            match get_enum_name(name, &val.to_string()) {
                                //None => format!("{} // Could not find enum value in map {}", name, val.to_string()),
                                None => format!("NULL_BIT_ENUM_OR_COULD_NOT_FIND[{}]", val.to_string()),
                                Some(value) => value
                            }
                        };
                        if enum_map().get(&tmp).is_some() || tmp.contains("Serializable") {
                            return serde_json::json!(str_enum_name(&tmp, *self))
                        } 
                        serde_json::json!(self)
                    }
                    None => serde_json::json!(self)
                }
            }
            fn to_bytes(&self, ctx: &mut RszSerializerCtx) -> Result<()> {
                Ok(ctx.data.write_all(&self.to_le_bytes())?)
            }
        }
        impl RszFromJson for $rsz_type {
            fn from_json(data: &serde_json::Value, ctx: &mut RszJsonDeserializerCtx) -> Result<$rsz_type> {
                match ctx.field {
                    Some(field) => {
                        let tmp = field.original_type.replace("[]", "");
                        let enum_val_from_str = |name: &str, val: &str| { 
                            match get_enum_val(name, val) {
                                //None => format!("{} // Could not find enum value in map {}", name, val.to_string()),
                                None => 0,
                                Some(value) => value
                            }
                        };
                        if enum_map().get(&tmp).is_some() || tmp.contains("Serializable") {
                            let enum_str_val = data.as_str().unwrap();
                            let enum_val = enum_val_from_str(&tmp, &enum_str_val);
                            return Ok(enum_val as $rsz_type);
                        }
                        Ok(serde_json::from_value(data.clone())?)
                    }
                    None => Ok(serde_json::from_value(data.clone())?)
                }
            }
        }

        derive_dersz_type!( $rsz_type );
    };
}


derive_dersz_type_enum!(i32);
derive_dersz_type_enum!(u32);

derive_dersz_full!(u8);
derive_dersz_full!(u16);
derive_dersz_full!(u64);
derive_dersz_full!(i8);
derive_dersz_full!(i16);
derive_dersz_full!(i64);

pub type F8 = u8; // scuffed
pub type F16 = u16; // scuffed
derive_dersz_full!(f32);
derive_dersz_full!(f64);

impl DeRszInstance for bool {
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn to_json(&self, ctx: &RszJsonSerializerCtx) -> serde_json::Value {
        serde_json::json!(self)
    }
    fn to_bytes(&self, ctx: &mut RszSerializerCtx) -> Result<()> {
        Ok(ctx.data.write_all(&(*self as u8).to_le_bytes())?)
    }
}
derive_dersz_from_json!(bool);
impl<'a> DeRszType<'a> for bool {
    fn from_bytes(ctx: &'a mut RszDeserializerCtx) -> Result<Self> where Self: Sized {
        let v = ctx.data.read_u8()?;
        if v > 1 {
            return Err(Box::new(crate::reerr::FileParseError::InvalidBool(v)))
        }
        Ok(v != 0)
    }
}

#[derive(Debug, DeRszInstance)]
pub struct StringU16(Vec<u16>);

derive_dersz_from_json!(StringU16);

impl<'a> DeRszType<'a> for StringU16 {
    fn from_bytes(ctx: &'a mut RszDeserializerCtx) -> Result<Self> where Self: Sized {
        let s = Vec::<u16>::from_bytes(ctx)?;
        Ok(StringU16(s))
    }
}

impl Serialize for StringU16 {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer {
            let s = String::from_utf16_lossy(&self.0);
            s.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for StringU16 {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
        where
            D: serde::Deserializer<'de> {
            let s = String::deserialize(deserializer)?;
            let s: Vec<u16> = s.encode_utf16().collect();
            Ok(StringU16(s))
    }
}


impl DeRszInstance for Option<Box<dyn DeRszInstance>> {
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn to_json(&self, ctx: &RszJsonSerializerCtx) -> serde_json::Value {
        if let Some(val) = self { serde_json::json!(val.to_json(ctx)) }
        else { serde_json::Value::Null }
    }
    fn to_bytes(&self, _ctx: &mut RszSerializerCtx) -> Result<()> {
        todo!()
    }
}

#[derive(Debug)]
pub struct Nullable {
    has_value: bool,
    value: Option<Box<dyn DeRszInstance>>
}
impl DeRszInstance for Nullable {
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn to_json(&self, ctx: &RszJsonSerializerCtx) -> serde_json::Value {
        serde_json::json!({
            "has_value": self.has_value,
            "value": self.value.to_json(ctx)
        })
    }
    fn to_bytes(&self, _ctx: &mut RszSerializerCtx) -> Result<()> {
        todo!()
    }
}

impl<'a> DeRszType<'a> for Nullable {
    fn from_bytes(ctx: &'a mut RszDeserializerCtx) -> Result<Self> {
        let hash = *ctx.cur_hash.last().unwrap();
        let struct_desc = match RszDump::rsz_map().get(&hash) {
            Some(struct_desc) => struct_desc,
            None => return Err(Box::new(reerr::FileParseError::InvalidRszTypeHash(hash)))
        };
        let has_value = bool::from_bytes(ctx)?;
        if has_value {
            return Ok(Nullable { has_value, value: None })
        } else {
            let field = &struct_desc.fields[1];
            let name: &str = &field.r#type;
            let dersz_fn = ctx.registry.get(name)?;
            Ok(Nullable{has_value, value: Some(dersz_fn(ctx)?)})
        }
    }

}


impl DeRszInstance for String {
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn to_json(&self, ctx: &RszJsonSerializerCtx) -> serde_json::Value {
        serde_json::json!(self)
    }
    fn to_bytes(&self, ctx: &mut RszSerializerCtx) -> Result<()> {
        Ok(ctx.data.write_all(&self.as_bytes())?)
    }
}

derive_dersz_from_json!(String);

impl<'a> DeRszType<'a> for String {
    fn from_bytes(ctx: &'a mut RszDeserializerCtx) -> Result<Self> where Self: Sized {
        let s = Vec::<u8>::from_bytes(ctx)?;
        Ok(String::from_utf8(s)?)
    }
}



#[derive(Debug, Serialize, DeRszInstance, DeRszFrom)]
pub struct RuntimeType(String);

#[derive(Debug, Serialize, DeRszInstance, DeRszFrom)]
struct OBB {
    center: Vec3,
    half_extents: Vec3,
    orientation: [Vec3; 3], // local axes (right, up, forward)
}

#[derive(Debug, Serialize, DeRszInstance, DeRszFrom)]
struct AABB{
    a: Vec4,
    b: Vec4,
}


#[derive(Debug, Serialize, DeRszInstance, DeRszFrom)]
struct Rect{
    start: UInt2,
    end: UInt2,
}

#[derive(Debug, Serialize, DeRszInstance)]
struct Data {
    data: Vec<u8>
}

impl<'a> DeRszType<'a> for Data {
    fn from_bytes(ctx: &'a mut RszDeserializerCtx) -> Result<Self> where Self: Sized {
        let len = ctx.field.last()
            .ok_or(RszError::MissingFieldDescription(format!("{}, {}",file!(), line!())))?.size;
        let buf = ctx.data.read_u8_n(len as usize)?;
        Ok(Data {
            data: buf.to_vec()
        })
    }
}

impl RszFromJson for Data {
    fn from_json(data: &serde_json::Value, ctx: &mut RszJsonDeserializerCtx) -> Result<Self> where Self: Sized {
        Ok(Self {
            data: Vec::new()
        })
    }
}

#[derive(Debug, Serialize, DeRszInstance, DeRszFrom)]
struct Range{
    start: f32,
    end: f32,
}

#[derive(Debug, Serialize, DeRszInstance, DeRszFrom)]
struct RangeI{
    start: i32,
    end: i32,
}

#[derive(Debug, Serialize, DeRszFrom, DeRszInstance)]
struct KeyFrame{
    time: f32,
    val: Vec3,
}

#[derive(Debug)]
pub struct Struct {
    pub hash: u32,
    pub values: Vec<Box<dyn DeRszInstance>>
}

impl DeRszInstance for Struct {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn to_json(&self, ctx: &RszJsonSerializerCtx) -> serde_json::Value {
        let struct_desc = match RszDump::rsz_map().get(&self.hash) {
            Some(struct_desc) => struct_desc,
            None => return serde_json::Value::Null
        };
        let values = struct_desc.fields.iter().enumerate().map(|(i, field)| {
            let obj = &self.values[i];
            let new_ctx = RszJsonSerializerCtx {
                root: None,
                field: Some(&field),
                objects: ctx.objects
            };
            (field.name.clone(), obj.to_json(&new_ctx))
        }).collect::<IndexMap<String, serde_json::Value>>();
        serde_json::to_value(values).unwrap()
    }

    fn to_bytes(&self, ctx: &mut RszSerializerCtx) -> Result<()> {
        let r#struct = RszDump::get_struct(self.hash)?;
        //println!("struct {:#?}", r#struct);
        //println!("struct_field_values {:#?}", self.values);
        for (i, field) in r#struct.fields.iter().enumerate() {
            //println!("{i} field {_field:#?}");
            //println!("{:?}", self.values[i]);
            // probably align shit
            let align = field.align as usize;
            let len = get_writer_length(ctx.data)? as usize;

            if field.array && (len + ctx.base_addr) % 4 as usize != 0 {
                println!("normal offset\n");
                let buf = vec![0; 4 - (len + ctx.base_addr) % 4];
                ctx.data.write_all(&buf)?;
            }
            if (len + ctx.base_addr) % align != 0 {
                println!("normal offset\n");
                let buf = vec![0; align - (len + ctx.base_addr) % align];
                ctx.data.write_all(&buf)?;
            } 
            self.values[i].to_bytes(ctx)?;
        }
        Ok(())
    }
}


pub fn get_writer_length<T: WriteSeek + ?Sized>(writer: &mut T) -> std::io::Result<u64> {
    let current_pos = writer.seek(SeekFrom::Current(0))?;
    let end_pos = writer.seek(SeekFrom::End(0))?;
    writer.seek(SeekFrom::Start(current_pos))?;
    Ok(end_pos)
}

impl<'a> DeRszType<'a> for Struct {
    fn from_bytes(ctx: &'a mut RszDeserializerCtx) -> Result<Self> {
        let hash = ctx.get_hash()?;
        let struct_desc = RszDump::get_struct(hash)?;
        let mut values: Vec<Box<dyn DeRszInstance>> = Vec::new();
        for (_i, field) in struct_desc.fields.iter().enumerate() {
            //println!("struct field: {field:?}");
            if field.array {
                ctx.data.seek_align_up(4)?;
                let len = ctx.data.read_u32()?;
                ctx.data.seek_align_up(field.align.into())?;
                let mut vals = Vec::new();
                //println!("LEN {len}");
                for _ in 0..len {
                    let dersz_fn = ctx.registry.get(field.name.as_str())?;
                    let x: Box<dyn DeRszInstance> = dersz_fn(ctx)?;
                    vals.push(x);
                }
                values.push(Box::new(vals))
            } else {
                ctx.data.seek_align_up(field.align.into())?;
                let dersz_fn = ctx.registry.get(field.name.as_str())?;
                let x: Box<dyn DeRszInstance> = dersz_fn(ctx)?;
                values.push(x);
            }
        }
        Ok(Self {
            hash: hash,
            values
        })
    }
}

impl RszFromJson for Struct {
    fn from_json(data: &serde_json::Value, ctx: &mut RszJsonDeserializerCtx) -> Result<Self> where Self: Sized {
        let r#struct = RszDump::get_struct(ctx.hash)?;
        let mut field_values: Vec<Box<dyn DeRszInstance>> = Vec::new();
        println!("");
        println!("\nJson Deser Struct: {:?}", r#struct);
        for field in r#struct.fields.iter() {
            let mut ctx = RszJsonDeserializerCtx {
                hash: ctx.hash,
                objects: &mut ctx.objects,
                registry: ctx.registry.clone(),
                field: Some(&field),
            };
            println!("\n\tJson Deserializing: {field:?}");
            let field_data = data.get(&field.name).expect(format!("Could not find field in json data {:?}", field.name).as_str());
            println!("\n\tJson field: {field_data:?}");
            //log::debug!("\n\tData: {field_data:?}");
            if field.array {
                let mut vals = Vec::new();
                let values: Vec<serde_json::Value> = field_data.as_array().expect("field should be an array").to_vec();
                for value in values {
                    let sersz_fn = ctx.registry.get_se(field.r#type.as_str())?;
                    vals.push(sersz_fn(&value, &mut ctx)?)
                }
                //println!("array_vals: {vals:#?}");
                field_values.push(Box::new(vals));
            } else {
                let sersz_fn = ctx.registry.get_se(field.r#type.as_str())?;
                let x: Box<dyn DeRszInstance> = sersz_fn(&field_data, &mut ctx)?;
                field_values.push(x);
            }

        }
        println!("{:#?}", field_values);
        Ok(Struct {hash: ctx.hash, values: field_values})
    }
}

#[derive(Debug, DeRszInstance)]
pub struct Guid([u8; 16]);

derive_dersz_from_json!(Guid);

impl<'a> DeRszType<'a> for Guid {
    fn from_bytes(ctx: &'a mut RszDeserializerCtx) -> Result<Self> where Self: Sized {
        let b = <[u8; 16]>::from_bytes(ctx)?;
        Ok(Guid(b))
    }
}

impl Serialize for Guid {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer {
            serializer.serialize_str(&uuid::Uuid::from_bytes(self.0).to_string())
    }
}

impl<'de> Deserialize<'de> for Guid {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
        where
            D: serde::Deserializer<'de> {
            let s = String::deserialize(deserializer)?;
            let uuid = uuid::Uuid::from_str(&s).unwrap();
            Ok(Guid(uuid.into_bytes()))
        
    }
}
