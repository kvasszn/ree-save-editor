use std::io::{self, Read};
use nalgebra_glm::*;
use uuid::Uuid;

pub trait ReadExt: Read {
    fn read_bool(&mut self) -> io::Result<bool> {
        let mut buf = [0 ;1];
        self.read_exact(&mut buf)?;
        let b = buf[0];
        if b > 1 {
            return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "Boolean value has > 1"
            ));
        }
        Ok(b != 0)
    }

    fn read_u8_n(&mut self, n: usize) -> io::Result<Vec<u8>> {
        let mut buf = vec![0u8; n];
        self.read_exact(&mut buf)?;
        Ok(buf)
    }

     fn read_u8(&mut self) ->io::Result<u8> {
        let mut buf = [0; 1];
        self.read_exact(&mut buf)?;
        Ok(buf[0])
    }
    fn read_u16(&mut self) ->io::Result<u16> {
        let mut buf = [0; 2];
        self.read_exact(&mut buf)?;
        Ok(u16::from_le_bytes(buf))
    }
    fn read_u32(&mut self) ->io::Result<u32> {
        let mut buf = [0; 4];
        self.read_exact(&mut buf)?;
        Ok(u32::from_le_bytes(buf))
    }
    fn read_u64(&mut self) ->io::Result<u64> {
        let mut buf = [0; 8];
        self.read_exact(&mut buf)?;
        Ok(u64::from_le_bytes(buf))
    }
    fn read_i8(&mut self) ->io::Result<i8> {
        let mut buf = [0; 1];
        self.read_exact(&mut buf)?;
        Ok(buf[0] as i8)
    }
    fn read_i16(&mut self) ->io::Result<i16> {
        let mut buf = [0; 2];
        self.read_exact(&mut buf)?;
        Ok(i16::from_le_bytes(buf))
    }
    fn read_i32(&mut self) ->io::Result<i32> {
        let mut buf = [0; 4];
        self.read_exact(&mut buf)?;
        Ok(i32::from_le_bytes(buf))
    }
    fn read_i64(&mut self) ->io::Result<i64> {
        let mut buf = [0; 8];
        self.read_exact(&mut buf)?;
        Ok(i64::from_le_bytes(buf))
    }
    fn read_magic(&mut self) ->io::Result<[u8; 4]> {
        let mut buf = [0; 4];
        self.read_exact(&mut buf)?;
        Ok(buf)
    }

    fn read_utf16str(&mut self) ->io::Result<String> {
        let mut s = vec![];
        let n = self.read_u32()?;
        for _i in 0..n {
            let c = self.read_u16()?;
            s.push(c);
        }
        String::from_utf16(&s).map_err(|e| {
            io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("{e}")
            )
        })
    }

    fn read_u16str_null_terminated(&mut self) ->io::Result<String> {
        let mut u16str = vec![];
        loop {
            let c = self.read_u16()?;
            if c == 0 {
                break;
            }
            u16str.push(c);
        }
        String::from_utf16(&u16str).map_err(|e| {
            io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("{e}")
            )
        })
    }

    fn read_u16str(&mut self) ->io::Result<String> {
        let mut u16str = vec![];
        loop {
            let c = self.read_u16()?;
            if c == 0 {
                break;
            }
            u16str.push(c);
        }
        String::from_utf16(&u16str).map_err(|e| {
            io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("{e}")
            )
        })
    }

    fn read_u8str(&mut self) ->io::Result<String> {
        let mut s = vec![];
        loop {
            let c = self.read_u8()?;
            if c == 0 {
                break;
            }
            s.push(c);
        }
        String::from_utf8(s).map_err(|e| {
            io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("{e}")
            )
        })
    }
    fn read_f32(&mut self) ->io::Result<f32> {
        let mut buf = [0; 4];
        self.read_exact(&mut buf)?;
        Ok(f32::from_le_bytes(buf))
    }
    fn read_f64(&mut self) ->io::Result<f64> {
        let mut buf = [0; 8];
        self.read_exact(&mut buf)?;
        Ok(f64::from_le_bytes(buf))
    }
    fn read_f32vec4(&mut self) ->io::Result<Vec4> {
        Ok(vec4(
            self.read_f32()?,
            self.read_f32()?,
            self.read_f32()?,
            self.read_f32()?,
        ))
    }

    fn read_f32vec3(&mut self) ->io::Result<Vec3> {
        Ok(vec3(self.read_f32()?, self.read_f32()?, self.read_f32()?))
    }

    fn read_f32vec2(&mut self) ->io::Result<Vec2> {
        Ok(vec2(self.read_f32()?, self.read_f32()?))
    }

    fn read_f32m4x4(&mut self) ->io::Result<Mat4x4> {
        let data: Vec<f32> = std::iter::from_fn(|| Some(self.read_f32()))
            .take(16)
            .collect::<io::Result<_>>()?;
        Ok(make_mat4x4(&data))
    }
    fn read_guid(&mut self) ->io::Result<Uuid> {
        let mut buf = [0; 16];
        for i in 0..16 {
            buf[i] = self.read_u8()?;
        }
        Ok(Uuid::from_bytes_le(buf))
    }

    fn read_u8_arr<const N: usize>(&mut self) -> io::Result<[u8; N]> {
        let mut arr = [0; N];
        for i in 0..N { arr[i] = self.read_u8()?; }
        Ok(arr)
    }

    fn read_16_arr<const N: usize>(&mut self) -> io::Result<[u8; N]> {
        let mut arr = [0; N];
        for i in 0..N { arr[i] = self.read_u8()?; }
        Ok(arr)
    }

    fn read_u32_arr<const N: usize>(&mut self) -> io::Result<[u32; N]> {
        let mut arr = [0u32; N];
        for i in 0..N { arr[i] = self.read_u32()?; }
        Ok(arr)
    }

    fn read_f32_arr<const N: usize>(&mut self) -> io::Result<[f32; N]> {
        let mut arr = [0f32; N];
        for i in 0..N { arr[i] = self.read_f32()?; }
        Ok(arr)
    }
    
    fn read_i32_arr<const N: usize>(&mut self) -> io::Result<[i32; N]> {
        let mut arr = [0i32; N];
        for i in 0..N { arr[i] = self.read_i32()?; }
        Ok(arr)
    }
}

impl<T: Read + ?Sized> ReadExt for T {}
