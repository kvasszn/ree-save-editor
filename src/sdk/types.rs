use std::{fmt::{Debug, Display}, io::{self, Read}};

use serde::Serialize;
use half::f16;
use util::ReadExt;
use uuid::Uuid;

// Objects
#[derive(Debug, Clone, Copy)]
pub struct Object {
    pub hash: u32,
    pub index: u32,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct TypeDescriptor {
    pub hash: u32,
    pub crc: u32,
}

pub type UserData = Object;

#[derive(Clone)]
pub struct U16String<const NULL_TERM: bool>(pub Vec<u16>);

impl U16String<false> {
    pub fn read<R: Read>(r: &mut R) -> io::Result<Self> {
        let len = r.read_u32()?;
        let s: Vec<u16> = (0..len).map(|_| r.read_u16()).collect::<Result<_, _>>()?;
        Ok(Self(s))
    }
}
impl U16String<true> {
    pub fn read<R: Read>(r: &mut R) -> io::Result<Self> {
        let mut s = vec![];
        loop {
            let c = r.read_u16()?;
            if c == 0 {
                break;
            }
            s.push(c);
        }
        Ok(Self(s))
    }
}

impl<const N: bool> Display for U16String<N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = String::from_utf16_lossy(&self.0);
        write!(f, "{}", s.trim_matches(char::from(0))) 
    }
}

impl<const N: bool> Debug for U16String<N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = String::from_utf16_lossy(&self.0);
        write!(f, "U16String<{N}>{}", s.trim_matches(char::from(0))) 
    }
}

pub type StringU16 = U16String<false>;
impl StringU16 {
    pub fn new(data: Vec<u16>) -> Self {
        U16String::<false>(data)
    }
    pub fn from(data: &str) -> Self {
        let data = data.chars().map(|x| x as u16).collect();
        U16String::<false>(data)
    }
}
pub type StringU16C = U16String<true>;

#[derive(Debug, Clone, Copy)]
pub struct Guid(pub [u8; 16]);
impl Display for Guid {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let uuid = Uuid::from_bytes_le(self.0);
        write!(f, "{}", uuid)    }
}

impl Serialize for Guid {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer {
            serializer.serialize_str(&self.to_string())
    }
}


#[derive(Debug, Serialize, Clone, Copy)]
pub struct Range<T> {
    pub start: T,
    pub end: T,
}

pub type RangeF = Range<f32>;
impl RangeF {
    pub fn read<R: Read>(r: &mut R) -> io::Result<Self> {
        Ok(Self {
            start: r.read_f32()?,
            end: r.read_f32()?,
        })
    }
}

pub type RangeI = Range<i32>;
impl RangeI {
    pub fn read<R: Read>(r: &mut R) -> io::Result<Self> {
        Ok(Self {
            start: r.read_i32()?,
            end: r.read_i32()?,
        })
    }
}

pub type UInt2 = [u32; 2];
pub type UInt3 = [u32; 3];
pub type UInt4 = [u32; 4];
pub type Int2 = [i32; 2];
pub type Int3 = [i32; 3];
pub type Int4 = [i32; 4];
pub type Float2 = [f32; 2];
pub type Float3 = [f32; 3];
pub type Float4 = [f32; 4];

#[derive(Debug, Clone, Copy, Serialize)]
pub struct Vec2(f32, f32, f32, f32);
impl Vec2 {
    pub fn read<R: Read>(r: &mut R) -> io::Result<Self> {
        Ok(Self(r.read_f32()?,r.read_f32()?,r.read_f32()?,r.read_f32()?,))
    }
}

#[derive(Debug, Clone, Copy, Serialize)]
pub struct Vec3(f32, f32, f32, f32);
impl Vec3 {
    pub fn read<R: Read>(r: &mut R) -> io::Result<Self> {
        Ok(Self(r.read_f32()?,r.read_f32()?,r.read_f32()?,r.read_f32()?,))
    }
}

#[derive(Debug, Clone, Copy, Serialize)]
pub struct Vec4(f32, f32, f32, f32);
impl Vec4 {
    pub fn read<R: Read>(r: &mut R) -> io::Result<Self> {
        Ok(Self(r.read_f32()?,r.read_f32()?,r.read_f32()?,r.read_f32()?,))
    }
}

#[derive(Debug, Clone, Copy, Serialize)]
pub struct Quaternion(f32, f32, f32, f32);
impl Quaternion {
    pub fn read<R: Read>(r: &mut R) -> io::Result<Self> {
        Ok(Self(r.read_f32()?,r.read_f32()?,r.read_f32()?,r.read_f32()?,))
    }
}

#[derive(Debug, Clone, Copy, Serialize)]
pub struct Sphere(f32, f32, f32, f32);
impl Sphere {
    pub fn read<R: Read>(r: &mut R) -> io::Result<Self> {
        Ok(Self(r.read_f32()?,r.read_f32()?,r.read_f32()?,r.read_f32()?,))
    }
}

#[derive(Debug, Clone, Copy, Serialize)]
pub struct Position(f64, f64, f64);
impl Position {
    pub fn read<R: Read>(r: &mut R) -> io::Result<Self> {
        Ok(Self(r.read_f64()?,r.read_f64()?,r.read_f64()?))
    }
}

#[derive(Debug, Clone, Copy, Serialize)]
pub struct Color(u8, u8, u8, u8);
impl Color {
    pub fn read<R: Read>(r: &mut R) -> io::Result<Self> {
        Ok(Self(r.read_u8()?, r.read_u8()?, r.read_u8()?, r.read_u8()?))
    }
}

#[derive(Debug, Clone, Copy, Serialize)]
pub struct Mat4x4(pub [f32; 16]);

#[derive(Debug, Serialize, Clone)]
pub struct RuntimeType(pub String);

#[derive(Debug, Serialize, Clone)]
pub struct GameObjectRef(pub Guid);

#[derive(Debug, Serialize, Clone, Copy)]
pub struct OBB {
    center: Vec3,
    half_extents: Vec3,
    orientation: [Vec3; 3], // local axes (right, up, forward)
}

impl OBB {
    pub fn read<R: Read>(r: &mut R) -> io::Result<Self> {
        Ok(Self {
            center: Vec3::read(r)?,
            half_extents: Vec3::read(r)?,
            orientation: [Vec3::read(r)?, Vec3::read(r)?, Vec3::read(r)?],
        })
    }
}

#[derive(Debug, Serialize, Clone, Copy)]
pub struct AABB(pub Vec4, pub Vec4);
impl AABB {
    pub fn read<R: Read>(r: &mut R) -> io::Result<Self> {
        Ok(Self(Vec4::read(r)?, Vec4::read(r)?))
    }
}

#[derive(Debug, Serialize, Clone, Copy)]
pub struct Rect {
    start: UInt2,
    end: UInt2,
}

impl Rect {
    pub fn read<R: Read>(r: &mut R) -> io::Result<Self> {
        Ok(Self {
            start: r.read_u32_arr()?,
            end: r.read_u32_arr()?,
        })
    }
}

#[derive(Debug, Serialize, Clone)]
pub struct Data(pub Vec<u8>);
impl Data {
    pub fn read<R: Read>(r: &mut R) -> io::Result<Self> {
        let len = r.read_u32()?;
        let v: Vec<u8> = (0..len).map(|_| r.read_u8()).collect::<Result<_, _>>()?;
        Ok(Self(v))
    }
}

#[derive(Debug, Serialize, Clone, Copy)]
pub struct KeyFrame{
    time: f32,
    val: [f32; 3],
}

impl KeyFrame {
    pub fn read<R: Read>(r: &mut R) -> io::Result<Self> {
        Ok(Self {
            time: r.read_f32()?,
            val: r.read_f32_arr()?,
        })
    }
}

/*
 * Native Structs/Custom impls
 */

#[derive(Debug, Clone, Serialize)]
pub struct AnimationCurve3D {
    pub xkeys: Vec<AnimationCurveKey>,
    pub ykeys: Vec<AnimationCurveKey>,
    pub zkeys: Vec<AnimationCurveKey>,
    pub min_value: f32,
    pub max_value: f32,
    pub min_time: f32,
    pub max_time: f32,
    pub loop_count: u32,
    pub loop_wrap_no: u32,
}

#[derive(Debug, Clone, Copy, Serialize)]
pub struct AnimationCurveKey {
    pub value: f32,
    pub curve_type: u16,
    #[serde(serialize_with="f16::serialize_as_f32")]
    pub time: f16,
    #[serde(serialize_with="f16::serialize_as_f32")]
    pub in_normal_x: f16,
    #[serde(serialize_with="f16::serialize_as_f32")]
    pub in_normal_y: f16,
    #[serde(serialize_with="f16::serialize_as_f32")]
    pub out_normal_x: f16,
    #[serde(serialize_with="f16::serialize_as_f32")]
    pub out_normal_y: f16,
}

#[derive(Debug, Serialize, Clone)]
pub struct AnimationCurve {
    pub keys: Vec<AnimationCurveKey>,
    pub min_value: f32,
    pub max_value: f32,
    pub min_time: f32,
    pub max_time: f32,
    pub loop_count: u32,
    pub loop_wrap_no: u32,
}


#[derive(Debug, Serialize, Clone, Copy)]
pub struct Mandrake {
    pub v: i64,
    pub m: i64, // maybe change to NonZeroU64
}

impl Mandrake {
    pub fn to_buf(self) -> [u8; size_of::<Self>()] {
        let mut buf = [0u8; size_of::<Self>()]; 
        buf[0..8].copy_from_slice(&self.v.to_le_bytes());
        buf[8..16].copy_from_slice(&self.m.to_le_bytes());
        buf
    }

    pub fn set(&mut self, n: i64) {
        self.v = n * self.m 
    }

    pub fn get(&self) -> Option<i64> {
        if self.m == 0 {
            None
        } else {
            Some(self.v / self.m)
        }
    }
}
