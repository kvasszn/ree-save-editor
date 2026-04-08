use half::f16;

use crate::sdk::*;

#[derive(Debug, Clone)]
pub struct Instance {
    pub hash: u32,
    pub fields: Vec<Value>,
}

#[derive(Debug, Clone)]
pub struct Extern {
    pub index: u32,
    pub r#type: String,
    pub path: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Object(u32),
    Array(Vec<Value>),
    UserData(u32),
    Null,
    Bool(bool),
    U8(u8),
    U16(u16),
    U32(u32),
    U64(u64),
    S8(i8),
    S16(i16),
    S32(i32),
    S64(i64),
    F8(u8),
    F16(f16),
    F32(f32),
    F64(f64),
    Size(u64),
    RuntimeType(RuntimeType),
    String(StringU16),
    Resource(StringU16),
    UInt2(UInt2),
    UInt3(UInt3),
    UInt4(UInt4),
    Int2(Int2),
    Int3(Int3),
    Int4(Int4),
    Float2(Float2),
    Float3(Float3),
    Float4(Float4),
    Vec2(Vec2),
    Vec3(Vec3),
    Vec4(Vec4),
    Quaternion(Quaternion),
    Sphere(Sphere),
    Position(Position),
    Color(Color),
    Mat4x4(Box<Mat4x4>),
    Guid(Guid),
    OBB(Box<OBB>),
    AABB(Box<AABB>),
    Data(Data),
    Range(RangeF),
    RangeI(RangeI),
    Rect(Rect),
    GameObjectRef(GameObjectRef),
    KeyFrame(KeyFrame),
}

impl Value {
    pub fn as_i128(&self) -> Option<i128> {
        match self {
            Value::U8(v)  => Some(*v as i128),
            Value::U16(v) => Some(*v as i128),
            Value::U32(v) => Some(*v as i128),
            Value::U64(v) => Some(*v as i128),
            Value::S8(v)  => Some(*v as i128),
            Value::S16(v) => Some(*v as i128),
            Value::S32(v) => Some(*v as i128),
            Value::S64(v) => Some(*v as i128),
            _ => None,
        }
    }
}
