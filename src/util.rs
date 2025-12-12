use std::io::Cursor;
use murmur3::murmur3_32;

pub fn murmur3(data: impl AsRef<[u8]>, seed: u32) -> u32 {
    murmur3_32(&mut Cursor::new(data), seed).unwrap()
}
