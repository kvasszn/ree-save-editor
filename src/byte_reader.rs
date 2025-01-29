use byteorder::{self, LittleEndian, ReadBytesExt};
use std::{fs::{self}, io::{Error, ErrorKind, Result}};

#[derive(Debug)]
pub struct BytesFile {
    pub data: Vec<u8>,
    pub index: usize,
}

impl BytesFile {
    pub fn new(file_name: String) -> Result<BytesFile> {
        let data = fs::read(file_name)?;
        Ok(BytesFile { data, index: 0 })
    }

    pub fn read<T: ReadBytesTyped>(&mut self) -> Result<T> {
        if self.index > self.data.len() {
            return Err(Error::new(ErrorKind::InvalidData, "Invalid file Index, greater than data length"))
        }
        T::read(self)
    }

    pub fn readn<T: ReadBytesTyped, const N: usize>(&mut self) -> Result<[T; N]> {
        if self.index > self.data.len() {
            return Err(Error::new(ErrorKind::InvalidData, "Invalid file Index, greater than data length"))
        }
        T::readn::<N>(self)
    }

    pub fn read_bytes_to_vec(&mut self, num: usize) -> Result<Vec<u8>> {
        if self.index > self.data.len() {
            return Err(Error::new(ErrorKind::InvalidData, "Invalid file Index, greater than data length"));
        }
        let mut data = vec![0; num];
        for i in 0..num {
            let byte = u8::read(self)?;
            data[i] = byte;
        }
        Ok(data)
    }

    pub fn read_utf16(&mut self, from: usize) -> Result<String> {
        let mut data: Vec<u16> = vec![];
        self.index = from;
        loop {
            if self.index >= self.data.len() {
                return Err(Error::new(ErrorKind::InvalidData, "Invalid file Index, greater than data length"));
            }
            let c = u16::read(self)?;
            if c == 0 {
                break;
            }
            data.push(c);
        }
        let string = String::from_utf16(&data).unwrap();
        Ok(string)
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn seek(&mut self, num: usize) {
        self.index = num;
    }
}

pub trait ReadBytesTyped: Sized {
    fn read(file: &mut BytesFile) -> Result<Self>;
    fn readn<const N: usize>(file: &mut BytesFile) -> Result<[Self; N]>;
}

impl ReadBytesTyped for u64 {
    fn read(file: &mut BytesFile) -> Result<u64> {
        let mut data = &file.data[file.index..file.index + 8];
        let res = data.read_u64::<LittleEndian>()?;
        file.seek(file.index + 8);
        Ok(res)
    }

    fn readn<const N: usize>(file: &mut BytesFile) -> Result<[u64; N]> {
        let mut data = [0u64; N];
        for i in 0..N {
            data[i] = file.read::<u64>()?;
        }
        Ok(data)
    }
}

impl ReadBytesTyped for i32 {
    fn read(file: &mut BytesFile) -> Result<i32> {
        let res = (&file.data[file.index..file.index + 4]).read_u32::<LittleEndian>()?;
        file.seek(file.index + 4);
        Ok(res as i32)
    }

    fn readn<const N: usize>(file: &mut BytesFile) -> Result<[i32; N]> {
        let mut data = [0i32; N];
        for i in 0..N {
            data[i] = file.read::<u32>()? as i32;
        }
        Ok(data)
    }
}
impl ReadBytesTyped for u32 {
    fn read(file: &mut BytesFile) -> Result<u32> {
        let res = (&file.data[file.index..file.index + 4]).read_u32::<LittleEndian>()?;
        file.seek(file.index + 4);
        Ok(res)
    }

    fn readn<const N: usize>(file: &mut BytesFile) -> Result<[u32; N]> {
        let mut data = [0u32; N];
        for i in 0..N {
            data[i] = file.read::<u32>()?;
        }
        Ok(data)
    }
}

impl ReadBytesTyped for u16 {
    fn read(file: &mut BytesFile) -> Result<u16> {
        let res = (&file.data[file.index..file.index + 2]).read_u16::<LittleEndian>()?;
        file.seek(file.index + 2);
        Ok(res)
    }

    fn readn<const N: usize>(file: &mut BytesFile) -> Result<[u16; N]> {
        let mut data = [0u16; N];
        for i in 0..N {
            data[i] = file.read::<u16>()?;
        }
        Ok(data)
    }
}

impl ReadBytesTyped for u8 {
    fn read(file: &mut BytesFile) -> Result<u8> {
        let res = file.data[file.index];
        file.seek(file.index + 1);
        Ok(res)
    }

    fn readn<const N: usize>(file: &mut BytesFile) -> Result<[u8; N]> {
        let mut data = [0u8; N];
        for i in 0..N {
            data[i] = file.read::<u8>()?;
        }
        Ok(data)
    }
}
